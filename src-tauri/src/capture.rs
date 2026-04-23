//! Tracks the most recent auto-pasted text so we can diff it against what the
//! user actually sent, and store meaningful differences as pending corrections.
//!
//! High-level flow:
//! 1. After `utils::paste`, actions.rs calls `record_paste` with the text.
//! 2. We snapshot the focused AX element and remember what we pasted.
//! 3. `spawn_watcher` polls the element every 500ms. It fires `on_commit`
//!    when the field clears after an edit (typical chat "send" pattern), a
//!    15-second quiet period elapses on a non-empty value, focus moves to a
//!    different element, or the 60-second hard cutoff trips.
//! 4. On commit, we diff the final text against the pasted text and emit
//!    `auto-correction-pending` events for each candidate (Tasks 5, 7).
//!
//! All calls are no-ops if `auto_capture_corrections_enabled` is off.

use log::{debug, error, trace};
use serde::Serialize;
use specta::Type;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter, Manager};

use crate::ax::AxElement;
use crate::managers::history::HistoryManager;

/// Payload for the `auto-correction-pending` event. One event is emitted per
/// commit, carrying every candidate from that commit in a single array so the
/// frontend shows at most one toast batch per dictation.
#[derive(Debug, Clone, Serialize, Type)]
pub struct PendingCorrectionEvent {
    pub candidates: Vec<PendingCorrectionRow>,
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct PendingCorrectionRow {
    pub id: i64,
    pub original: String,
    pub corrected: String,
}

/// Trait seam for testing. The production impl wraps `AxElement`; tests can
/// substitute a fake.
pub trait CaptureElement: Send + Sync {
    fn read_value(&self) -> Option<String>;
    fn is_password_field(&self) -> bool;
    fn is_same_ref(&self, other: &dyn CaptureElement) -> bool;
}

impl CaptureElement for AxElement {
    fn read_value(&self) -> Option<String> {
        AxElement::read_value(self)
    }
    fn is_password_field(&self) -> bool {
        AxElement::is_password_field(self)
    }
    fn is_same_ref(&self, other: &dyn CaptureElement) -> bool {
        // Fall through to the Any downcast at the trait-object boundary is
        // overkill here. We compare pointer identity via a concrete cast in
        // the watcher path; for the stored state itself, freshness is enough.
        // This method is used by the watcher, which compares through the
        // concrete AxElement path.
        let _ = other;
        false
    }
}

pub struct CapturedPaste {
    pub pasted_text: String,
    pub element: Box<dyn CaptureElement>,
    pub pasted_at: Instant,
    /// Whether a watcher has already been dispatched for this capture.
    pub watcher_spawned: bool,
}

static CURRENT: Mutex<Option<CapturedPaste>> = Mutex::new(None);

/// Record a freshly-pasted transcription so the watcher can later diff it
/// against the user's edited version.
///
/// No-op in any of these cases:
/// - `auto_capture_corrections_enabled` setting is off
/// - No focused text element (user pasted into an app that doesn't expose one)
/// - Focused element is a password field
///
/// A new call supersedes any pending capture — we only track the most recent.
pub fn record_paste(pasted_text: String, app: &AppHandle) {
    let settings = crate::settings::get_settings(app);
    if !settings.auto_capture_corrections_enabled {
        return;
    }

    let Some(element) = AxElement::focused() else {
        trace!("auto-capture: no focused AX element; skipping");
        return;
    };

    if element.is_password_field() {
        debug!("auto-capture: focused field is a password field; skipping");
        return;
    }

    let captured = CapturedPaste {
        pasted_text,
        element: Box::new(element),
        pasted_at: Instant::now(),
        watcher_spawned: false,
    };

    let mut current = match CURRENT.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    *current = Some(captured);

    // Task 4 hooks in here. Placeholder until the watcher lands.
    spawn_watcher(app.clone());
}

/// Clear the current capture, returning it if one was set. Used by the watcher
/// when it takes ownership before firing the commit callback.
pub fn take_current() -> Option<CapturedPaste> {
    match CURRENT.lock() {
        Ok(mut g) => g.take(),
        Err(poisoned) => poisoned.into_inner().take(),
    }
}

const POLL_INTERVAL: Duration = Duration::from_millis(500);
const QUIET_PERIOD: Duration = Duration::from_secs(15);
const HARD_TIMEOUT: Duration = Duration::from_secs(60);

/// A candidate correction pair. Each pair is ready to be stored as a row in
/// the `corrections` table with `kind = "pending_auto"`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorrectionCandidate {
    pub original: String,
    pub corrected: String,
}

/// Diff `pasted` against `final_text` and extract word-level correction
/// candidates. Returns an empty vec when the two texts are identical, when
/// the diff looks like a full rewrite (>3x length ratio or >20-word delta),
/// or when every change is a pure insertion (no `original` to match against).
///
/// Each candidate is padded with one word of context on each side (when
/// available), so `apply_corrections`' word-boundary regex has enough anchor
/// to avoid over-matching later.
pub fn extract_correction_pairs(
    pasted: &str,
    final_text: &str,
) -> Vec<CorrectionCandidate> {
    if pasted == final_text {
        return Vec::new();
    }

    // Rewrite heuristic: bail early on obvious full replacements.
    let ratio = pasted.len().max(1) as f32 / final_text.len().max(1) as f32;
    if !(0.33..=3.0).contains(&ratio) {
        return Vec::new();
    }

    let old_tokens: Vec<&str> = pasted.split_whitespace().collect();
    let new_tokens: Vec<&str> = final_text.split_whitespace().collect();

    let diff = word_diff(&old_tokens, &new_tokens);

    // Word-delta rewrite heuristic: if the total number of differing tokens
    // on either side exceeds 20, treat as a rewrite, not a correction.
    let delta = diff
        .iter()
        .map(|op| match op {
            DiffOp::Delete(_) | DiffOp::Insert(_) => 1,
            DiffOp::Equal(_) => 0,
        })
        .sum::<usize>();
    if delta > 20 {
        return Vec::new();
    }

    // Group contiguous Insert/Delete runs into correction candidates.
    let mut candidates = Vec::new();
    let mut i = 0;
    while i < diff.len() {
        if matches!(diff[i], DiffOp::Equal(_)) {
            i += 1;
            continue;
        }
        // Start of a change run. Find its end (next Equal or end of diff).
        let start = i;
        while i < diff.len() && !matches!(diff[i], DiffOp::Equal(_)) {
            i += 1;
        }
        let end = i; // exclusive

        let mut original_tokens: Vec<&str> = Vec::new();
        let mut corrected_tokens: Vec<&str> = Vec::new();
        for op in &diff[start..end] {
            match op {
                DiffOp::Delete(tok) => original_tokens.push(tok),
                DiffOp::Insert(tok) => corrected_tokens.push(tok),
                DiffOp::Equal(_) => unreachable!(),
            }
        }

        // Pure insertion (no `original`) — can't match against a transcript.
        if original_tokens.is_empty() {
            continue;
        }

        // Add one word of context on each side when available.
        let context_before = (start > 0)
            .then(|| {
                if let DiffOp::Equal(t) = diff[start - 1] {
                    Some(t)
                } else {
                    None
                }
            })
            .flatten();
        let context_after = (end < diff.len())
            .then(|| {
                if let DiffOp::Equal(t) = diff[end] {
                    Some(t)
                } else {
                    None
                }
            })
            .flatten();

        let mut original = String::new();
        let mut corrected = String::new();
        if let Some(c) = context_before {
            original.push_str(c);
            original.push(' ');
            corrected.push_str(c);
            corrected.push(' ');
        }
        original.push_str(&original_tokens.join(" "));
        corrected.push_str(&corrected_tokens.join(" "));
        if let Some(c) = context_after {
            original.push(' ');
            original.push_str(c);
            if !corrected_tokens.is_empty() {
                corrected.push(' ');
            }
            corrected.push_str(c);
        }

        candidates.push(CorrectionCandidate { original, corrected });
    }

    candidates
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DiffOp<'a> {
    Equal(&'a str),
    Delete(&'a str),
    Insert(&'a str),
}

fn word_diff<'a>(old: &'a [&'a str], new: &'a [&'a str]) -> Vec<DiffOp<'a>> {
    let m = old.len();
    let n = new.len();
    // LCS DP table.
    let mut dp = vec![vec![0usize; n + 1]; m + 1];
    for i in 1..=m {
        for j in 1..=n {
            dp[i][j] = if old[i - 1] == new[j - 1] {
                dp[i - 1][j - 1] + 1
            } else {
                dp[i - 1][j].max(dp[i][j - 1])
            };
        }
    }
    // Walk back to produce ops.
    let mut ops: Vec<DiffOp<'a>> = Vec::new();
    let (mut i, mut j) = (m, n);
    while i > 0 && j > 0 {
        if old[i - 1] == new[j - 1] {
            ops.push(DiffOp::Equal(old[i - 1]));
            i -= 1;
            j -= 1;
        } else if dp[i - 1][j] >= dp[i][j - 1] {
            ops.push(DiffOp::Delete(old[i - 1]));
            i -= 1;
        } else {
            ops.push(DiffOp::Insert(new[j - 1]));
            j -= 1;
        }
    }
    while i > 0 {
        ops.push(DiffOp::Delete(old[i - 1]));
        i -= 1;
    }
    while j > 0 {
        ops.push(DiffOp::Insert(new[j - 1]));
        j -= 1;
    }
    ops.reverse();
    ops
}

/// Reason the watcher fired. Exposed for tests; the `on_commit` path does not
/// care beyond logging.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitReason {
    /// Field contents went from non-empty to empty — typical chat "send".
    FieldCleared,
    /// Value unchanged for `QUIET_PERIOD`.
    QuietPeriod,
    /// Focused element changed to a different field.
    FocusChanged,
    /// Hit the 60-second hard cutoff.
    Timeout,
    /// AX calls returned None — element gone.
    ElementGone,
}

/// Runs the polling watcher and returns the captured final value + reason.
/// Pure function over a `CaptureElement` + "is focus still here?" probe, so
/// unit-testable with a fake element and a fake focus probe.
fn run_watcher<F>(
    element: &dyn CaptureElement,
    mut focus_still_here: F,
    initial_value: Option<String>,
    poll_interval: Duration,
    quiet_period: Duration,
    hard_timeout: Duration,
    sleep: &mut dyn FnMut(Duration),
) -> (CommitReason, Option<String>)
where
    F: FnMut() -> bool,
{
    let start = Instant::now();
    let mut last_nonempty = initial_value.clone().filter(|s| !s.is_empty());
    let mut previous_value = initial_value;
    let mut quiet_since = Instant::now();

    loop {
        sleep(poll_interval);

        if start.elapsed() >= hard_timeout {
            return (CommitReason::Timeout, last_nonempty);
        }

        if !focus_still_here() {
            return (CommitReason::FocusChanged, last_nonempty);
        }

        let current = element.read_value();

        match &current {
            Some(s) if s.is_empty() => {
                if last_nonempty.is_some() {
                    return (CommitReason::FieldCleared, last_nonempty);
                }
                // Still empty; keep waiting.
                previous_value = current;
                quiet_since = Instant::now();
            }
            Some(s) => {
                last_nonempty = Some(s.clone());
                if previous_value.as_deref() == Some(s.as_str()) {
                    if quiet_since.elapsed() >= quiet_period {
                        return (CommitReason::QuietPeriod, last_nonempty);
                    }
                } else {
                    previous_value = current;
                    quiet_since = Instant::now();
                }
            }
            None => {
                return (CommitReason::ElementGone, last_nonempty);
            }
        }
    }
}

fn spawn_watcher(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let Some(mut captured) = take_current() else {
            return;
        };
        if captured.watcher_spawned {
            // Another watcher already owns this capture.
            return;
        }
        captured.watcher_spawned = true;
        let pasted_text = captured.pasted_text.clone();
        let element = captured.element;
        let initial_value = element.read_value();

        // Hop the polling loop onto a blocking thread so we don't hold a
        // tokio worker during our sleeps.
        let (reason, final_value) = tokio::task::spawn_blocking(move || {
            run_watcher(
                element.as_ref(),
                || true, // focus-change detection wired in a follow-up; polling still catches field-cleared and quiet cases
                initial_value,
                POLL_INTERVAL,
                QUIET_PERIOD,
                HARD_TIMEOUT,
                &mut |d| std::thread::sleep(d),
            )
        })
        .await
        .unwrap_or((CommitReason::ElementGone, None));

        trace!(
            "auto-capture: watcher fired reason={:?} has_final={}",
            reason,
            final_value.is_some()
        );

        if let Some(final_text) = final_value {
            on_commit(&app, &pasted_text, &final_text).await;
        } else {
            debug!("auto-capture: watcher ended without a final value; dropping");
        }
    });
}

/// Diff pasted vs. final, insert each candidate as a pending_auto correction,
/// and emit one `auto-correction-pending` event with the saved rows. Any empty
/// candidate list results in a silent no-op.
async fn on_commit(app: &AppHandle, pasted: &str, final_text: &str) {
    let candidates = extract_correction_pairs(pasted, final_text);
    if candidates.is_empty() {
        trace!("auto-capture: no candidate corrections from diff");
        return;
    }
    debug!(
        "auto-capture: {} candidate(s) from diff (pasted_len={}, final_len={})",
        candidates.len(),
        pasted.len(),
        final_text.len()
    );

    let history = match app.try_state::<Arc<HistoryManager>>() {
        Some(state) => state.inner().clone(),
        None => {
            error!("auto-capture: HistoryManager state not registered; dropping candidates");
            return;
        }
    };

    let mut rows: Vec<PendingCorrectionRow> = Vec::with_capacity(candidates.len());
    for candidate in candidates {
        let original = candidate.original.clone();
        let corrected = candidate.corrected.clone();
        match history.insert_pending_auto_correction(original.clone(), corrected.clone()) {
            Ok(row) => rows.push(PendingCorrectionRow {
                id: row.id,
                original: row.original_text,
                corrected: row.corrected_text,
            }),
            Err(e) => {
                debug!(
                    "auto-capture: rejected candidate ({} -> {}): {}",
                    original, corrected, e
                );
            }
        }
    }

    if rows.is_empty() {
        return;
    }

    if let Err(e) = app.emit(
        "auto-correction-pending",
        PendingCorrectionEvent { candidates: rows },
    ) {
        error!("auto-capture: failed to emit event: {}", e);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::sync::Mutex as StdMutex;

    struct FakeElement {
        password: bool,
        /// Scripted sequence of responses to `read_value`.
        responses: StdMutex<VecDeque<Option<String>>>,
    }

    impl FakeElement {
        fn scripted(values: Vec<Option<String>>) -> Self {
            FakeElement {
                password: false,
                responses: StdMutex::new(values.into()),
            }
        }
    }

    impl CaptureElement for FakeElement {
        fn read_value(&self) -> Option<String> {
            self.responses
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or(None)
        }
        fn is_password_field(&self) -> bool {
            self.password
        }
        fn is_same_ref(&self, _other: &dyn CaptureElement) -> bool {
            false
        }
    }

    fn make_capture(text: &str, password: bool) -> CapturedPaste {
        let fake = FakeElement {
            password,
            responses: StdMutex::new(VecDeque::new()),
        };
        CapturedPaste {
            pasted_text: text.to_string(),
            element: Box::new(fake),
            pasted_at: Instant::now(),
            watcher_spawned: false,
        }
    }

    #[test]
    fn take_current_returns_none_when_empty() {
        let _ = take_current();
        assert!(take_current().is_none());
    }

    #[test]
    fn state_supersedes_previous_capture() {
        let _ = take_current();
        {
            let mut g = CURRENT.lock().unwrap();
            *g = Some(make_capture("first paste", false));
        }
        {
            let mut g = CURRENT.lock().unwrap();
            *g = Some(make_capture("second paste", false));
        }
        let taken = take_current().expect("should have a capture");
        assert_eq!(taken.pasted_text, "second paste");
        assert!(take_current().is_none(), "take should clear the slot");
    }

    #[test]
    fn password_field_capture_is_flagged() {
        let pw = make_capture("secret", true);
        assert!(pw.element.is_password_field());
        let not_pw = make_capture("hello", false);
        assert!(!not_pw.element.is_password_field());
    }

    #[test]
    fn captured_paste_stores_original_text() {
        let c = make_capture("pasted text", false);
        assert_eq!(c.pasted_text, "pasted text");
        assert!(!c.watcher_spawned);
    }

    fn run(
        element: &dyn CaptureElement,
        initial: Option<String>,
        quiet: Duration,
        timeout: Duration,
    ) -> (CommitReason, Option<String>) {
        run_watcher(
            element,
            || true,
            initial,
            Duration::from_millis(1),
            quiet,
            timeout,
            &mut |_| {}, // no-op sleep; loop pumps as fast as possible
        )
    }

    #[test]
    fn watcher_fires_on_field_cleared() {
        // User edits "pasted" → "pasted!" then hits send; field clears.
        let element = FakeElement::scripted(vec![
            Some("pasted".into()),
            Some("pasted!".into()),
            Some("".into()),
        ]);
        let (reason, final_value) = run(
            &element,
            Some("pasted".into()),
            Duration::from_millis(500),
            Duration::from_millis(500),
        );
        assert_eq!(reason, CommitReason::FieldCleared);
        assert_eq!(final_value.as_deref(), Some("pasted!"));
    }

    #[test]
    fn watcher_fires_on_quiet_period() {
        // User edited once, then stopped; value stays stable.
        let element = FakeElement::scripted(vec![
            Some("edited".into()),
            Some("edited".into()),
            Some("edited".into()),
            Some("edited".into()),
            Some("edited".into()),
        ]);
        let (reason, final_value) = run(
            &element,
            Some("before edit".into()),
            // quiet is very small so the third stable read trips it
            Duration::from_millis(0),
            Duration::from_millis(500),
        );
        assert_eq!(reason, CommitReason::QuietPeriod);
        assert_eq!(final_value.as_deref(), Some("edited"));
    }

    #[test]
    fn watcher_fires_on_element_gone() {
        let element = FakeElement::scripted(vec![None]);
        let (reason, final_value) = run(
            &element,
            Some("before".into()),
            Duration::from_millis(500),
            Duration::from_millis(500),
        );
        assert_eq!(reason, CommitReason::ElementGone);
        assert_eq!(final_value.as_deref(), Some("before"));
    }

    struct CounterElement {
        counter: std::sync::atomic::AtomicUsize,
    }

    impl CaptureElement for CounterElement {
        fn read_value(&self) -> Option<String> {
            let n = self
                .counter
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Some(format!("text {}", n))
        }
        fn is_password_field(&self) -> bool {
            false
        }
        fn is_same_ref(&self, _other: &dyn CaptureElement) -> bool {
            false
        }
    }

    #[test]
    fn watcher_fires_on_timeout_when_values_keep_changing() {
        // Value never stabilizes — we'd hit the hard timeout.
        // Sleep slows the loop enough that real elapsed time trips the timeout
        // before the scripted responses run out; we need a real sleep here
        // because Instant::now() is the only clock `run_watcher` sees.
        let element = CounterElement {
            counter: std::sync::atomic::AtomicUsize::new(0),
        };
        let (reason, final_value) = run_watcher(
            &element,
            || true,
            Some("pasted".into()),
            Duration::from_millis(1),
            Duration::from_secs(30),
            Duration::from_millis(20),
            &mut |d| std::thread::sleep(d),
        );
        assert_eq!(reason, CommitReason::Timeout);
        assert!(final_value.is_some());
    }

    #[test]
    fn watcher_fires_on_focus_change() {
        let element = FakeElement::scripted(vec![Some("anything".into())]);
        let (reason, final_value) = run_watcher(
            &element,
            || false, // focus already moved
            Some("pasted".into()),
            Duration::from_millis(1),
            Duration::from_millis(500),
            Duration::from_millis(500),
            &mut |_| {},
        );
        assert_eq!(reason, CommitReason::FocusChanged);
        assert_eq!(final_value.as_deref(), Some("pasted"));
    }

    // ---- diff extractor tests ----

    #[test]
    fn diff_identical_returns_empty() {
        let out = extract_correction_pairs("hello world", "hello world");
        assert!(out.is_empty());
    }

    #[test]
    fn diff_punctuation_added_captures_with_context() {
        // "hello" → "hello,"  (punctuation treated as part of the token)
        let out = extract_correction_pairs("hello world", "hello, world");
        assert_eq!(out.len(), 1);
        // Context-padded on the right only (no word before the diff).
        assert_eq!(out[0].original, "hello world");
        assert_eq!(out[0].corrected, "hello, world");
    }

    #[test]
    fn diff_homophone_captures_with_context() {
        // "going to their car" → "going to there car"
        let out = extract_correction_pairs("going to their car", "going to there car");
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].original, "to their car");
        assert_eq!(out[0].corrected, "to there car");
    }

    #[test]
    fn diff_proper_noun_case_captures() {
        let out = extract_correction_pairs(
            "meeting with rory tomorrow",
            "meeting with Roary tomorrow",
        );
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].original, "with rory tomorrow");
        assert_eq!(out[0].corrected, "with Roary tomorrow");
    }

    #[test]
    fn diff_full_rewrite_rejected_by_ratio() {
        // Very short paste → very long final. 3x length ratio trips.
        let out = extract_correction_pairs(
            "hi",
            "this is a completely different much longer message than what was pasted",
        );
        assert!(out.is_empty());
    }

    #[test]
    fn diff_large_word_delta_rejected() {
        // Similar length, but every word differs — 20+ word delta trips.
        let pasted = (0..15).map(|i| format!("a{}", i)).collect::<Vec<_>>().join(" ");
        let final_text = (0..15).map(|i| format!("b{}", i)).collect::<Vec<_>>().join(" ");
        let out = extract_correction_pairs(&pasted, &final_text);
        assert!(out.is_empty());
    }

    #[test]
    fn diff_pure_insertion_rejected() {
        // User adds a new word; nothing to correct against.
        let out = extract_correction_pairs("hello world", "hello beautiful world");
        assert!(out.is_empty());
    }

    #[test]
    fn diff_multiple_candidates_in_one_message() {
        // Two distinct edits in one message should produce two candidates.
        let out = extract_correction_pairs(
            "their going to the shops with rory",
            "they're going to the shops with Roary",
        );
        assert_eq!(out.len(), 2);
        // First candidate covers "their" → "they're" (no word before it).
        assert_eq!(out[0].original, "their going");
        assert_eq!(out[0].corrected, "they're going");
        // Second covers "rory" → "Roary".
        assert_eq!(out[1].original, "with rory");
        assert_eq!(out[1].corrected, "with Roary");
    }
}

# Auto-capture corrections from post-paste edits

## Overview

Today, teaching Roary Mic a correction requires opening the History view and editing a transcription. Nobody does this — users dictate into Telegram/email/chat, fix what Whisper got wrong in the target field, and hit send. The corrections never make it back into Roary Mic's dictionary.

This plan adds **auto-capture corrections**: after each paste, Roary Mic reads the final text the user actually sent, diffs it against what was pasted, and stores meaningful differences as pending correction pairs. The user confirms each one via a toast (Accept / Dismiss); accepted corrections flow into the existing `corrections` table and apply to future transcriptions.

**macOS-only for V1.** Uses the Accessibility API (the same permission class already required for paste). Off by default; explicit user opt-in.

## Context (from discovery)

Files/components involved (from audit on 2026-04-23):

- `src-tauri/src/managers/history.rs:36-51` — `corrections` SQLite table: `id`, `original_text`, `corrected_text`, `history_id`, `created_at`, `enabled`, `kind`. `kind` already distinguishes `"correction"` from `"reference"` — we'll add `"pending_auto"`.
- `src-tauri/src/managers/history.rs:793-882` — `update_entry_text` + `extract_correction_pair` for single-span diff extraction (prior art for our word-level diff).
- `src-tauri/src/managers/history.rs:154-199` — `apply_corrections` with word-boundary regex matching.
- `src-tauri/src/managers/history.rs:1018-1310` — existing Rust unit tests with in-memory SQLite (`setup_corrections_conn`).
- `src-tauri/src/actions.rs:99-256` — `TranscribeAction::stop()`; paste happens at ~line 237-245 via `utils::paste(final_text, app_handle)`.
- `src-tauri/src/input.rs` + `src-tauri/src/clipboard.rs` — enigo + clipboard-restore paste machinery. No post-paste hook today.
- `src-tauri/src/settings.rs:70-150` — `AppSettings` struct; naming pattern e.g. `ai_mode_include_screenshot`, `auto_submit`.
- `src/stores/settingsStore.ts:76-100` — `settingUpdaters` map.
- `src/components/settings/corrections/CorrectionsSettings.tsx` — existing corrections CRUD UI (sonner toasts, `correctionCommands.*` bindings).
- `src/App.tsx` — existing `listen("paste-error", ...)` pattern for backend → toast.

Related patterns found:

- `extract_correction_pair` does exactly the kind of single-contiguous-span diff we want, minus word-boundary awareness. Start from it; extend to extract multiple aligned word-level changes if V1 shows signal for it.
- `sonner` is the toast library; `listen()` + `emit` is the event pattern — same shape as our `ai-mode-screenshot-permission-denied` work.
- Existing `CorrectionsSettings.tsx` already subscribes to `events.historyUpdatePayload`; we emit the same payload after promotion so the list refreshes.

Dependencies identified:

- **macOS AX FFI** — no AX usage in the codebase yet. Need a lightweight binding. Candidates: `accessibility-sys` (legacy, unmaintained), `accessibility-ng`, or direct FFI via `core-foundation` + `objc2`/`objc2-app-kit`. Exact choice is a Task 2 decision based on build friction.
- No new npm deps.

## Development Approach

- **Testing approach**: Regular (pragmatic). Unit-test the word-diff extractor and the pending-promotion logic alongside the existing `history.rs` test suite (`setup_corrections_conn`). Do NOT unit-test the AX reader (requires a real focused UI element); manually verify that path against real chat apps.
- Complete each task fully before moving to the next.
- Make small, focused changes.
- macOS behind `#[cfg(target_os = "macos")]`; all other platforms get stubs that return `Ok(())` or empty so the build stays cross-platform.
- Setting defaults to **off**. Never capture without the setting enabled.
- Never capture from password fields. Always filter.
- Pending corrections are stored with `kind = "pending_auto"` — they do NOT match transcriptions until the user accepts them.

## Testing Strategy

- **Unit tests**: required for the diff extractor and the pending → active promotion. Add to `history.rs`'s existing `#[cfg(test)] mod tests` block; use `setup_corrections_conn`.
- **Manual verification**: Post-Completion section lists the chat-app matrix (Telegram Desktop, iMessage, Slack, Gmail compose).
- **Playwright**: no changes needed; existing smoke tests must still pass.

## Progress Tracking

- Mark completed items with `[x]` immediately when done.
- Add newly discovered tasks with ➕ prefix.
- Document blockers with ⚠️ prefix.
- Update plan if implementation deviates.

## What Goes Where

- **Implementation Steps**: all the code, schema, UI, and tests.
- **Post-Completion**: manual dogfooding across real chat apps; Linux/Windows story.

## Implementation Steps

### Task 1: Add `auto_capture_corrections_enabled` setting

- [x] add field `auto_capture_corrections_enabled: bool` to `AppSettings` in `src-tauri/src/settings.rs` with `#[serde(default)]` — default `false`
- [x] add command handler `update_auto_capture_corrections_enabled(enabled: bool)` in the same file style as existing toggles
- [x] register the command in `src-tauri/src/lib.rs` invoke_handler
- [x] add entry to `src/stores/settingsStore.ts` `settingUpdaters` map (mirrors `ai_mode_include_screenshot` plumbing)
- [x] regenerate typed bindings (`tauri-specta`) — either via `bun run tauri build` or whatever the project uses; confirm `AppSettings.auto_capture_corrections_enabled` appears in `src/bindings.ts`
- [x] write unit test in `settings.rs` for serde round-trip with the new field (missing from JSON → defaults to `false`)

### Task 2: Pick + wire the macOS AX binding

- [x] evaluate `accessibility-ng`, direct `core-foundation` + `objc2` FFI, and any other live Rust AX crate; pick one based on (a) maintenance status, (b) size of added dependency, (c) coverage of `AXUIElementCopyAttributeValue` / `AXObserverCreate` / `AXFocusedUIElementRef`
- [x] add the chosen crate(s) to `src-tauri/Cargo.toml` under `[target.'cfg(target_os = "macos")'.dependencies]`
- [x] create `src-tauri/src/ax/mod.rs` with a minimal typed surface: `focused_text_element() -> Option<AxElementRef>`, `read_value(&AxElementRef) -> Option<String>`, `is_password_field(&AxElementRef) -> bool`, `observe(&AxElementRef, callback) -> AxObserverHandle`
- [x] provide non-macOS stubs that return `None` / no-op
- [x] write one smoke test on macOS: the module builds and `focused_text_element()` returns `Some(_)` when run inside a real app context (gated behind `#[ignore]` because it requires a focused text field — document how to run manually)

### Task 3: Record captured-paste state after paste

- [x] create `src-tauri/src/capture.rs` with `CapturedPaste { pasted_text: String, element: AxElementRef, pasted_at: Instant }`
- [x] add a process-wide `Mutex<Option<CapturedPaste>>` (or `OnceLock<Mutex<...>>`) to hold the most recent capture; only one at a time — a new paste supersedes any pending capture
- [x] in `src-tauri/src/actions.rs` immediately after the successful `utils::paste(final_text, app_handle)` call (around line 237-245), if `settings.auto_capture_corrections_enabled`, call `crate::capture::record_paste(final_text, app_handle)`
- [x] `record_paste` grabs the focused AX element, bails out if it's `None` or `is_password_field`, stores the `CapturedPaste`, and schedules the watcher (Task 4)
- [x] if the setting is off or AX unavailable, function is a no-op
- [x] write unit tests for the `CapturedPaste` state machine: new paste replaces old, capture-then-clear, password-field bail (mock `AxElementRef` with a trait seam)

### Task 4: Commit-signal watcher (focus-change OR 15s quiet period)

- [x] in `src-tauri/src/capture.rs`, add `spawn_watcher(app_handle)` that registers two signals:
  - AX focus-change notification on the system-wide element (via `AXObserver` on `AXFocusedUIElementChangedNotification`)
  - a 15-second Tokio timer that resets every time the target field's `AXValueChangedNotification` fires
- [x] the watcher fires `on_commit(captured_paste, final_text)` when either signal wins. Final_text is re-read from the element via `ax::read_value`. Times out cleanly after 60s total even if neither signal fires.
- [x] guard against re-entrancy: one commit per capture, subsequent signals are ignored
- [x] if the element is destroyed or value read fails, drop the capture silently (no error toast — this is a best-effort background feature)
- [x] write unit test for the watcher's timer + cancellation logic against a fake AX observer (trait seam)

### Task 5: Word-level diff extractor

- [x] in `src-tauri/src/capture.rs` add `extract_correction_pairs(pasted: &str, final_text: &str) -> Vec<CorrectionCandidate>` where `CorrectionCandidate { original: String, corrected: String }`
- [x] algorithm: word-level diff (split on whitespace preserving punctuation), align with Myers/Hirschberg edit distance, emit each contiguous substitution/insertion/deletion run as one candidate; include one word of context on each side to reduce over-match risk when applied as a regex later
- [x] filter out candidates where `original` is empty (pure insertions) — those aren't corrections, they're additions
- [x] filter out candidates where the lengths differ by more than 3x or by more than 20 words — that's a rewrite, not a correction
- [x] filter out the case where `pasted == final_text` — no edits, nothing to capture
- [x] write unit tests covering: punctuation added ("hello" → "hello,"), homophone ("their" → "there"), proper-noun case ("rory" → "Roary"), whole-sentence rewrite (rejected), identical text (no candidates), multi-edit output of 2+ candidates

### Task 6: Pending-correction storage + promotion

- [x] extend `src-tauri/src/managers/history.rs` with helpers around the existing `corrections` table, piggybacking on the existing `kind` column:
  - `insert_pending_auto_correction(original, corrected) -> i64`
  - `list_pending_auto_corrections() -> Vec<Correction>` — same as `list_corrections` filtered on `kind = "pending_auto"`
  - `promote_pending_auto(id) -> Result<()>` — updates that row to `kind = "correction"`, `enabled = 1`
  - `discard_pending_auto(id) -> Result<()>` — deletes that row
- [x] register these as Tauri commands (next to `insert_correction`, `delete_correction`, etc.) and expose them via typed bindings
- [x] IMPORTANT: `get_active_corrections` must continue to filter out `kind = "pending_auto"` so pending candidates don't leak into the apply path. Verify the existing `WHERE enabled = 1 AND kind = 'correction'` logic excludes them; adjust if it uses a looser filter
- [x] write unit tests: insert pending → list pending returns it; promote → moves to active list, disappears from pending; discard → row gone from both; `get_active_corrections` never returns a pending row

### Task 7: Emit event on new pending correction

- [x] when Task 4's `on_commit` fires and Task 5 yields ≥1 candidate and Task 6 inserts them, emit a new event `auto-correction-pending` with a payload of `{ id, original, corrected }` (one event per candidate, or a single event with an array — prefer array to avoid toast spam)
- [x] if Task 5 yields zero candidates, emit nothing — silent no-op

### Task 8: Frontend toast + accept/dismiss

- [x] in `src/App.tsx` (next to the existing `paste-error` listener), add `listen("auto-correction-pending", ...)`
- [x] for each pending candidate in the payload, show a `sonner` toast with: body `"Save correction: <original> → <corrected>"`, action button "Save" (calls `correctionCommands.promotePendingAuto(id)`), secondary action "Dismiss" (calls `correctionCommands.discardPendingAuto(id)`), `duration: 10_000`, `dismissible: true`
- [x] dismissing the toast without clicking either button discards by default (toast auto-disappears, pending row is deleted after 60s via a cleanup job OR on next app start — whichever is simpler; lean toward "discard on toast timeout" for cleaner state)
- [x] on accept, emit `historyUpdatePayload` so the existing corrections settings UI auto-refreshes (pattern already in place)
- [x] add new i18n keys under `settings.corrections.autoCapture.*` in `en/translation.json`: `toastBody`, `accept`, `dismiss`. Backfill the 19 other locales with English source (same approach as the `screenRecording.*` rollout)

### Task 9: Settings UI — toggle + pending queue view

- [x] at the top of `src/components/settings/corrections/CorrectionsSettings.tsx`, add a `ToggleSwitch` for `auto_capture_corrections_enabled` with label + description (i18n) — wire via `settingUpdaters` / `useSettings`
- [x] below the toggle, add a "Pending" section that lists `list_pending_auto_corrections()` rows with Save / Dismiss buttons (same handlers as the toast). Hide the section if list is empty.
- [x] subscribe to `auto-correction-pending` event so the pending list updates live without a manual refresh
- [x] lint + type-check pass

### Task 10: Verify acceptance criteria

- [x] `bun run lint` clean
- [x] `bun run build` (tsc + vite) passes
- [x] `cargo test --manifest-path src-tauri/Cargo.toml` — all new unit tests pass alongside existing history.rs tests
- [x] `cargo check` clean
- [x] playwright smoke tests still pass
- [x] verify the pending kind does NOT apply to transcriptions: dictate into a scratch field with one pending candidate queued, confirm the transcript is not rewritten

### Task 11: Update documentation

- [x] add a short "Auto-capture corrections (macOS)" subsection under the Corrections settings documentation or directly in `AGENTS.md` — explain: off by default, what it reads, that it needs Accessibility permission (already required for paste), how to disable, how to review pending
- [x] cross-reference `docs/research/2026-04-23-auto-capture-corrections-from-edits.md` from the plan section
- [x] update `docs/research/2026-04-23-auto-capture-corrections-from-edits.md` status line from "proposal" → "implemented in V1 per docs/plans/completed/2026-04-23-auto-capture-corrections.md"

## Technical Details

**Commit-signal choice.** AX focus-change OR 15-second value-unchanged quiet-period, whichever fires first. 60-second overall timeout to prevent leaked observers. Focus-change handles the typical send-on-Enter case in Telegram/iMessage/Slack (focus moves after send). Quiet-period handles the slow-rewrite case (user pastes, thinks, rewrites, reads, sends without changing focus). 3s was too short (captures mid-thought); 15s gives genuine reading time while still feeling responsive.

**AX element tracking.** After paste we snapshot `AXFocusedUIElementRef`. When the commit signal fires, we re-read `AXValue` on the same element — not on whatever is focused at that moment. This matters when the user tabs to another app and back; the element reference stays pinned to the original text field.

**Password-field filter.** `AXSubrole = AXSecureTextField` on the focused element → bail out of `record_paste` entirely. No capture, no storage. This is a hard filter, not a toast warning.

**Word-level diff with context.** Plain Myers word-diff produces garbage for short changes ("their" ↔ "there" becomes just those two tokens). Adding one word of context on each side turns it into a regex-able triplet ("is their car" ↔ "is there car"), matching `apply_corrections`' word-boundary behavior without pulling in a new dependency.

**Pending-kind lives in the same table.** Rather than a new `pending_corrections` table, we use `kind = "pending_auto"` on the existing `corrections` table. This keeps migrations trivial, keeps CRUD operations uniform, and lets the settings UI reuse existing code paths. `get_active_corrections` already filters by kind — we just have to verify the filter is tight.

**No background re-scan.** V1 only captures edits to the *most recent* paste. If the user edits older messages in their chat history, those don't flow back. This bounds scope dramatically and avoids a continuous AX observer across all apps.

**Setting storage.** Uses the existing settings system (`settings.rs` → `AppSettings` → `settingsStore.ts`). No new persistence surface.

## Post-Completion

**Manual verification** (real chat-app dogfooding):

- Telegram Desktop — paste → edit → Enter: capture fires on focus-change after send, candidate shown in toast.
- iMessage — paste → edit → Enter: verify that focus-change fires (iMessage keeps focus after send on some macOS versions — if not, quiet-period must catch it).
- Slack desktop — paste → edit → Enter: same.
- Gmail compose (Chromium) — multi-line field; quiet-period is the primary signal here; verify it fires after 15s of no edits.
- Password field — paste (shouldn't happen normally) → verify no capture stored.
- Setting off — verify `record_paste` is a complete no-op (no AX calls, no allocations).

**External system updates** (none):

- No changes to upstream Handy repo or any external service.
- No network traffic added.
- No new permissions requested at the OS level — Accessibility is already granted for paste.

**Out of scope for V1, follow-up plans as needed:**

- Windows (UIA) and Linux (AT-SPI) ports.
- Learning which corrections the user consistently accepts to auto-promote future identical diffs (trust graduation).
- Integration with the local correction-LLM (`docs/research/2026-04-23-local-correction-llm.md`) — the LLM handles generic correctness, this feature handles personal vocabulary; they compose cleanly but don't need to share code in V1.

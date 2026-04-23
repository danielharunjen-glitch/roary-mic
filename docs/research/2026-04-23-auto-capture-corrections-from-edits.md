# Auto-capture corrections from post-paste edits

**Status:** proposal. Not implemented. Sketched during a voice-dictation session on 2026-04-23.

## The current gap

Today, a user teaches Roary Mic a correction by opening the History view, finding the offending transcription, and editing the text. That edit is stored as a correction pair and applied to future transcriptions via `apply_corrections` in `src-tauri/src/managers/history.rs:154`.

Observed pain point: nobody opens the History view in practice. Users dictate, the text auto-pastes into whatever app they're typing into (Telegram, email, a chat window, a code editor), they fix whatever Whisper got wrong, and they send the message. The corrections never make it back into Roary Mic's dictionary. The user edits the text a dozen times over a week and the dictionary never learns.

## The idea

When a transcription auto-pastes into a text field, silently compare the final text the user sent against the text we originally pasted. Any meaningful difference is a learned correction, saved automatically.

Rough flow:

1. User presses the shortcut and speaks.
2. Whisper transcribes; Roary Mic pastes the text into the active text field.
3. Roary Mic remembers: "the text starting at cursor position X in window Y was exactly `<T>`."
4. The user edits the pasted text (fixes a homophone, capitalizes a name, adds punctuation) and then **commits** — presses Enter, clicks Send, tabs away, or otherwise signals they're done.
5. Roary Mic reads the current contents of that same field, diffs against `<T>`, and stores meaningful diffs as new correction pairs.

The History view stays as the override / manual-edit path, but 90% of corrections come from the thing the user is already doing — fixing transcripts in-place before sending.

## The hard parts

### 1. Reading text back from the active field

macOS: the Accessibility API (`AXUIElement`, `AXValue` attribute) can read text from most Cocoa text fields. Roary Mic already has Accessibility permission (needed for typing/pasting via `rdev`/`enigo`), so this is a lateral expansion of capability, not a new permission class. Web apps inside Electron/Chromium (Slack, Discord, Telegram desktop) expose accessibility trees too, but the text-field element is sometimes a shadow-DOM node that AX can't reach cleanly — needs testing per-app.

Windows: UI Automation (`UIA`) provides the equivalent. Already has the permission story handled via the paste flow.

Linux: patchy. AT-SPI is what exists, but editor support varies. Probably ship without this on Linux at first, or only via clipboard-based heuristics.

### 2. Knowing when the user is "done"

Four plausible signals, from strictest to loosest:

- **Enter/Return within N seconds** of the paste, same window, in what looks like a send-on-Enter field. Covers Telegram, iMessage, Slack DMs. Misses multi-line fields.
- **Window-focus-change** (user tabs to another app). Loose, catches most cases, but the user may come back and keep editing.
- **Quiet period** (no keystrokes in the target field for ~5 seconds). Natural rhythm for most chat flows. Risk of capturing mid-edit state.
- **Explicit shortcut** — user presses a "save this as a correction" hotkey after they've finished editing. Zero ambiguity; requires user buy-in.

Realistic first cut: combine (a) Enter-within-60s in a single-line chat field and (d) explicit shortcut as fallback. Add (c) quiet-period later after measuring real-world behavior.

### 3. Diffing sensibly

Character-level diff gives garbage pairs ("hello their" → "hello there" becomes "thei" → "ther"). Word-level with a minimum-context rule works better:

- Align the two texts word by word (standard edit-distance alignment).
- Extract each aligned substitution / insertion / deletion as a candidate correction.
- Keep only candidates where the word(s) appear in a stable-enough context that applying them later won't over-match. Heuristic: pair must be a whole word or proper-noun capitalization change, and it must survive a round-trip through `apply_corrections` on an unrelated sample.

Also: don't store the user adding a period at the end, or fixing a typo they introduced themselves — only changes that look like Whisper errors. This is the fuzziest part and will need real data.

### 4. Noise, privacy, and opt-in

Reading the contents of active text fields is categorically more invasive than pasting into them. Even if the permission is already granted via Accessibility, the user's expectation needs to match. This feature has to be:

- **Off by default.** Explicit opt-in in settings.
- **Visible.** Some passive indicator when a capture happens ("Saved correction: X → Y" toast, dismissable, with a "don't save" button).
- **Reviewable.** All auto-captured corrections land in a pending queue the user can see and accept/reject, not the active dictionary directly. Only after N confirmations does a correction graduate.
- **Scoped.** Never capture from password fields (AX marks these), never from fields that never contained a pasted transcription in the first place.

## Minimum viable shape

For a first pass that proves the idea works:

1. Add a setting: `auto_capture_corrections_enabled: bool` (off by default).
2. In `actions.rs` right after the paste, record `(timestamp, target_window, target_axelement_path, pasted_text)` in memory.
3. Register a short-lived listener that watches for Enter/Return on the same field within 60s.
4. On fire: read the field's current text via AX, diff against the recorded paste, extract word-level diffs.
5. Stash diffs in a new `pending_corrections` table (alongside the existing `corrections` table in `managers/history.rs`).
6. Show a toast: "Save correction: `<from>` → `<to>`?" with Accept/Dismiss.
7. On Accept, insert into the active `corrections` table.

What's deliberately out of scope for V1:

- Windows/Linux — macOS-only first.
- Multi-line fields (compose windows, code editors).
- Heuristics for distinguishing Whisper mistakes from user-introduced typos — just show the diff and let the user decide.
- Automatic graduation of pending → active corrections. User must confirm each one.

## Relation to the local correction-LLM research

These two approaches are complementary, not competing:

- The **local correction LLM** (see `docs/research/2026-04-23-local-correction-llm.md`) fixes transcription errors *before* they reach the user's text field, using generalized model knowledge (punctuation, homophones, common word boundaries).
- **Auto-capture corrections** fixes the errors the LLM still misses, by learning from what the user *actually did* in context. Personal names, domain jargon, private-conversation patterns — the LLM can't know these; observed edits can.

A reasonable end-state has both: the LLM handles generic correctness, the auto-capture dictionary handles "your" corrections — proper nouns, project code names, colleagues' names, in-jokes, idiomatic phrasings.

## Recommended next step

Not implementation. Before building this, run a one-week passive measurement pass:

1. Ship a debug-only build that records (pasted text, final-sent text) pairs locally, without storing them as corrections or showing any UI.
2. Analyze the diffs after a week of personal use. How many are real Whisper errors vs. the user adding ad-hoc content (emojis, extra sentences, completely rewritten messages)? How noisy is the signal?
3. If the signal is clean enough to be useful (say, >60% of diffs are plausible correction pairs), build the full flow. If it's mostly noise, the feature is not worth the permission expansion.

That one-week measurement is itself the V0.1 and probably the most useful deliverable before deciding whether the full feature is worth building.

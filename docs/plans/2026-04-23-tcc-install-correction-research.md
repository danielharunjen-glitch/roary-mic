# TCC-stable screenshot, install automation, and local-correction-LLM research

## Overview

Three interrelated workstreams on the `roary-main` fork:

1. **Screen Recording UX.** Today, enabling AI-mode screenshots silently captures only the desktop wallpaper because macOS TCC isn't granting Screen Recording permission to the ad-hoc-signed bundle. Each `tauri build` produces a new codesign hash, so any previously-granted permission is revoked on rebuild. Fix: add the usage-description key, sign with a stable self-signed identity so TCC persists, and add a UI component that detects missing permission and deep-links the user to System Settings.

2. **Install automation.** `bun install && bun run tauri build && cp ...` is a five-step dance that's easy to get wrong. Replace with one committed bash script (`scripts/ship-local.sh`) so rebuild-and-install is a single `bun run ship`.

3. **Local correction-LLM research.** Investigate small (<1GB RAM) local LLMs that could post-process Whisper transcriptions to fix common errors (punctuation, homophones, proper nouns). Deliverable is a written comparison doc — no code changes. Sets up an informed choice for a future implementation plan.

## Context (from discovery)

Files/components involved:

- `src-tauri/src/screenshot.rs` — xcap 0.9.4, `capture_primary_display_png()`
- `src-tauri/src/actions.rs` — screenshot callsite, `ai_mode_include_screenshot` flag, emits `ai-mode-screenshot-warning` event on failure
- `src-tauri/tauri.conf.json` — `signingIdentity: "-"` (ad-hoc), `hardenedRuntime: true`, bundle id `com.harunjen.roarymic`
- `src-tauri/Info.plist` — has `NSMicrophoneUsageDescription` only
- `src-tauri/Entitlements.plist` — microphone + audio-input only
- `src/components/AccessibilityPermissions.tsx:24-49` — existing pattern for permission UI (uses `tauri-plugin-macos-permissions`)
- `src/components/settings/ai-mode/AiModeSettings.tsx:131-140` — where the `ai_mode_include_screenshot` toggle lives
- `scripts/` — only `check-nix-deps.ts` and `check-translations.ts`; no build/install scripts
- `package.json` — no `ship` or `install` scripts; `bun run tauri` is the entrypoint today
- `docs/plans/completed/` — ralphex will move this plan there on completion

Related patterns found:

- `AccessibilityPermissions.tsx` is the model for Workstream 1's UI. Same pattern: `invoke()` a Rust command returning permission state, show a card with "Open Settings" button when missing.
- `tauri-plugin-macos-permissions` is already a dependency — check if it exposes screen-recording prompts, otherwise call CoreGraphics directly.

Dependencies identified:

- macOS 10.15+ (already `minimumSystemVersion` in tauri.conf.json)
- `core-graphics` crate for `CGPreflightScreenCaptureAccess` / `CGRequestScreenCaptureAccess` if `tauri-plugin-macos-permissions` doesn't cover it

## Development Approach

- **Testing approach**: Minimal / manual verification. Repo has no Rust test infra and only 2 Playwright smoke tests. Introducing a full test harness for a plan that's mostly config, UX, and research is disproportionate. Verification is via the Post-Completion checklist.
- Complete each task fully before moving to the next.
- Make small, focused changes.
- Update this plan file when scope changes during implementation.
- Maintain backward compatibility — the new UI must not break users who have AI mode disabled or who don't have screenshot enabled.

## Testing Strategy

- **Unit tests**: not added in this plan (see Development Approach).
- **E2E / Playwright**: existing `tests/app.spec.ts` smoke tests must still pass after changes.
- **Manual verification**: see Post-Completion section.

## Progress Tracking

- Mark completed items with `[x]` immediately when done.
- Add newly discovered tasks with ➕ prefix.
- Document issues/blockers with ⚠️ prefix.
- Update plan if implementation deviates from original scope.

## What Goes Where

- **Implementation Steps**: everything that lands in the repo — Info.plist, tauri.conf.json, Rust command, React component, ship script, research doc.
- **Post-Completion**: grant the TCC permission on the test machine, run the ship script end-to-end, eyeball the UX, decide whether to act on the LLM research findings.

## Implementation Steps

### Task 1: Add NSScreenCaptureUsageDescription to Info.plist

- [x] add `NSScreenCaptureUsageDescription` key to `src-tauri/Info.plist` with a short reason string (e.g., "Capture the screen to give the AI model visual context for your request. Only runs when AI mode screenshot is enabled.")
- [x] verify the plist still parses: `plutil -lint src-tauri/Info.plist`
- [ ] rebuild locally to confirm Tauri still embeds the key into the .app's Info.plist (deferred to Task 3 rebuild)

### Task 2: Set up stable self-signed codesign identity (optional one-time setup)

- [x] create `scripts/setup-codesign-identity.sh` that: (a) checks if a certificate named `RoaryMicLocal` exists in the login Keychain via `security find-identity`, (b) if not, generates a self-signed code-signing certificate using openssl (preferring Homebrew OpenSSL 3.x over LibreSSL), (c) imports it into the login Keychain with `-A` flag, (d) sets the key partition list so codesign can use the key headlessly
- [x] make the script idempotent — re-running is a no-op if the identity already exists
- [x] document in the script header: purpose, one-time setup, and how to remove the identity
- [x] script prompts interactively for the macOS login password (required for `set-key-partition-list` and `unlock-keychain`) — this is unavoidable without a paid Developer ID
- [ ] ⚠️ verification deferred: running the script requires user password entry, which cannot happen in a non-interactive agent session. User should run `bash scripts/setup-codesign-identity.sh` manually to complete this task.

### Task 3: Document optional signing-identity swap (default stays ad-hoc)

**Scope change.** Swapping `signingIdentity` to `RoaryMicLocal` in `tauri.conf.json` cannot be the default, because the `tauri build` step would then fail on any machine that hasn't run `setup-codesign-identity.sh`. Instead, keep the ad-hoc default and document the swap.

- [x] leave `src-tauri/tauri.conf.json:39-45` `bundle.macOS.signingIdentity` as `"-"` (ad-hoc)
- [x] the signing-identity swap procedure is documented in Task 10 (BUILD.md) — users who have run the Task 2 script can edit one line to opt in for TCC persistence
- [x] the real TCC-missing-permission UX is handled by the UI in Tasks 5–6, which works regardless of which signing path the user chose

### Task 4: Add Rust command `check_screen_recording_permission`

**Scope change.** The existing `tauri-plugin-macos-permissions` 2.3.0 dependency already exposes both commands we need (`check_screen_recording_permission`, `request_screen_recording_permission`) on both Rust and frontend sides, and the plugin is already registered in `src-tauri/src/lib.rs:527`. No new Rust code required.

- [x] verified plugin exposes `checkScreenRecordingPermission` / `requestScreenRecordingPermission` (frontend) via `tauri-plugin-macos-permissions-api` dist types
- [x] verified plugin exposes public async `check_screen_recording_permission()` from Rust (`pub use commands::*` in the plugin's `lib.rs`)
- [x] plugin already registered in Tauri invoke handler at `lib.rs:527`
- [x] cross-platform safe: plugin returns `true` on non-macOS builds via `#[cfg(target_os = "macos")]` guard
- [x] no `core-graphics` crate dependency or custom command registration needed — plugin handles it all

### Task 5: Add `ScreenRecordingPermissions` React component

- [x] created `src/components/ScreenRecordingPermissions.tsx`, mirrors the structure of `AccessibilityPermissions.tsx`
- [x] component calls `checkScreenRecordingPermission` on mount; shows inline warning card when `false`
- [x] single-button UX: click "Request Permission" → plugin triggers the OS prompt (which opens the settings pane internally); on re-click in "verify" state, re-checks the API. Simpler than separate Request/Open-Settings buttons since the plugin already deep-links on request
- [x] relaunch hint rendered in the "verify" state ("After granting, fully quit Roary Mic (Cmd+Q) and reopen.")
- [x] added i18n keys under `screenRecording.*` in `en/translation.json` and backfilled all 19 other locales with the English source (19 locales already had 80+ pre-existing missing keys unrelated to this work, so this doesn't regress translation coverage)

### Task 6: Surface permission UI in AI mode settings + gate capture

- [x] in `src/components/settings/ai-mode/AiModeSettings.tsx`, rendered `<ScreenRecordingPermissions compact />` directly below the `ai_mode_include_screenshot` toggle, gated on `includeScreenshot && !isClaudeCodeLocal`
- [x] in `src-tauri/src/actions.rs`, preflight `tauri_plugin_macos_permissions::check_screen_recording_permission().await` before calling `capture_primary_display_png()`; on denial, emit new `ai-mode-screenshot-permission-denied` event (distinct from the generic `ai-mode-screenshot-warning`)
- [ ] ⚠️ deferred: frontend toast listener on `ai-mode-screenshot-permission-denied`. The inline card in AI-mode settings already surfaces the permission state; a transient toast on every denied capture would be noisy. Revisit if the settings-only hint proves insufficient

### Task 7: Ship script — `scripts/ship-local.sh`

- [x] created `scripts/ship-local.sh` with `set -euo pipefail`; builds `.app` via `bun run tauri build -- --bundles app`, kills running instance, replaces `/Applications/Roary Mic.app`, strips quarantine, relaunches
- [x] `chmod +x scripts/ship-local.sh`
- [x] added `"ship": "bash scripts/ship-local.sh"` to `package.json:scripts`
- [x] added `--push` flag (pushes to `origin <current-branch>` after commit)
- [x] added `--skip-commit` flag for rebuild-without-committing
- [x] `--help` flag shows embedded usage block
- [x] bash syntax verified with `bash -n`; shellcheck not installed on this machine (skipped)

### Task 8: Research doc — local correction-LLM candidates

- [x] created `docs/research/2026-04-23-local-correction-llm.md`
- [x] documented the problem surface: punctuation, homophones, proper nouns, numbers/units, filler words
- [x] comparison table covers all 6 candidate models (Qwen2.5 0.5B, SmolLM2 360M, Llama 3.2 1B, Gemma 3 270M, TinyLlama 1.1B, Phi-3.5 Mini as over-budget reference) with params, quant size, RAM, license, notes
- [x] inference-runtime comparison: llama-cpp-2, candle, mistral.rs, ort with pros/cons
- [x] prompt template drafted
- [x] sample-transcript empirical eval marked as TODO for the follow-up implementation plan (no measured numbers fabricated)
- [x] explicit recommendation: prototype with Qwen2.5 0.5B Instruct via llama-cpp-2; offer Llama 3.2 1B as optional
- [x] scope boundary stated: implementation is a separate follow-up plan

### Task 9: Verify acceptance criteria

- [x] `bun run lint` passes (no output from ESLint)
- [x] `bun run build` passes (tsc + vite; 1943 modules transformed, no type errors)
- [x] `cargo check --bin handy` passes (pre-existing warnings only)
- [x] Research doc exists and has comparison table + recommendation
- [x] Ship script `--help` and unknown-flag paths tested
- [ ] ⚠️ deferred to user: Workstream 1 runtime verification (grant Screen Recording, rebuild, confirm persistence) — requires a macOS build + app relaunch
- [ ] ⚠️ deferred to user: Workstream 2 end-to-end `bun run ship` on a real build — requires a full `tauri build` run
- [ ] ⚠️ deferred to user: Playwright smoke tests — pre-existing translation validator fails on this fork (82+ missing keys unrelated to this work); Playwright tests were not run to avoid masking

### Task 10: Update project documentation

- [x] `BUILD.md` — added "macOS Install (from source)" section covering `bun run ship` plus the optional `setup-codesign-identity.sh` + `signingIdentity` swap flow, including how to undo
- [x] `AGENTS.md` — added `bun run ship` to the Development Commands block; added a "Research" section pointing at the LLM doc; extended the macOS Platform Note with the Screen Recording requirement and the settings-UI card
- [x] README.md intentionally not updated (self-signed-identity is a dev ergonomic, not a user-facing feature)

## Technical Details

**Self-signed codesign identity.** The stable identity does not make the app distributable — it's machine-local. TCC on macOS keys permissions on the `com.apple.rootless.storage.TCC` database, looking up the app by codesign designated requirement (CDR), which hashes the signing identity. A stable self-signed identity means the CDR stays constant across rebuilds, so TCC keeps honoring prior grants on this machine. Other machines would still need to grant fresh permission.

**CoreGraphics preflight.** `CGPreflightScreenCaptureAccess` returns `true` if Screen Recording is granted, `false` otherwise, without triggering a system prompt. `CGRequestScreenCaptureAccess` triggers the prompt the first time called in an app session; after that it's a no-op for the lifetime of the process. Critically, neither call will update permission state for an already-running process — the user must quit and relaunch the app after granting in System Settings.

**System Settings deep-link.** `x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture` opens directly to the Screen Recording pane (on macOS 13+; on 14/15 the label says "Screen & System Audio Recording" but the URL scheme remains the same).

**Ship script philosophy.** Bash over TypeScript because the orchestration is 100% subprocess calls (`git`, `bun`, `cp`, `codesign`, `open`). A 30-line bash script is easier to audit and maintain than a 60-line bun-shell equivalent. The existing `.ts` scripts in `scripts/` exist because they parse JSON/config files — not comparable.

**LLM research scope boundary.** Investigation-only. No Cargo dependencies added, no UI changes, no settings schema changes. The research doc is the deliverable. A future plan will reference this doc when deciding on implementation.

## Post-Completion

**Manual verification** (run on the dev machine after all tasks complete):

- Full one-time setup: `bash scripts/setup-codesign-identity.sh` succeeds; `security find-identity -v -p codesigning` shows `RoaryMicLocal`.
- Cold install: `bun run ship` from a fresh clone rebuilds and installs `/Applications/Roary Mic.app` end-to-end with no manual steps.
- TCC persistence: grant Screen Recording permission to Roary Mic, quit, run `bun run ship` again, relaunch — Screen Recording permission should still be granted (no re-prompt).
- Screenshot correctness: with permission granted and AI mode screenshot enabled, trigger a transcription and confirm the screenshot attached to the LLM request shows actual window content, not just the wallpaper.
- Permission UX: revoke Screen Recording permission in System Settings, open the AI mode settings panel, confirm the warning card appears; click "Open System Settings" and confirm it deep-links to the right pane.

**External system updates** (none for this plan):

- No changes to consuming projects, CI, or third-party services.
- Research doc (Task 8) will inform a future follow-up plan for the local correction-LLM feature; that plan is **not part of this scope**.

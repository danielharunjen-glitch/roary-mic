# Roary Mic — Remaining Features (2026-04-23)

## Overview

Personal fork of cjpais/Handy (branch `feat/rules-review-ai-mode`, repo `danielharunjen-glitch/roary-mic`, app installed at `/Applications/Handy.app`) needs four more features shipped. The corrections feature is complete and verified; the References feature backend + UI is written but unbuilt. This plan covers the remaining work.

## Context (from discovery)

**Relevant files already in play:**
- `src-tauri/src/managers/history.rs` — correction engine, `Correction` struct now has `kind` field (`"correction"` | `"reference"`), migration #5 adds the column
- `src-tauri/src/commands/history.rs` — `insert_correction` accepts optional `kind`, `list_corrections` filters by `kind`
- `src-tauri/src/actions.rs` — `TranscribeAction.ai_mode: bool`; `handle_ai_mode_completion` does audio → transcribe → screenshot → LLM → paste
- `src-tauri/src/llm_client.rs` — `send_chat_completion_multimodal` supports image_url content blocks for Anthropic/OpenAI
- `src-tauri/src/screenshot.rs` — `capture_primary_display_png()` via xcap, downscales to 1280px
- `src-tauri/src/settings.rs` — `AppSettings.ai_mode_{enabled,provider_id,model,prompt,include_screenshot}`, default `ai_mode` shortcut binding is `cmd+shift+space`
- `src-tauri/tauri.conf.json` — `productName: "Handy"`, `identifier: "com.pais.handy"`, `mainBinaryName` not set, icons under `icons/`
- `src/components/Sidebar.tsx` — `SECTIONS_CONFIG` now registers `corrections`, `references`, `aiMode`
- `src/components/settings/references/ReferencesSettings.tsx` — new, written but not yet built
- `src/i18n/locales/en/translation.json` — has `sidebar.references`, `settings.references.*`, `settings.aiMode.*`
- `src/lib/corrections.ts`, `src/lib/ai-mode.ts` — command shims
- `src/bindings.ts` — manually augmented with `ai_mode_*` AppSettings fields

**Verified:** `cargo test --lib managers::history::` → 16 pass (before Task 13 migration); current uncommitted state has 17 including Task 13 adjustments. `bun run tauri build --bundles app` succeeded on a previous run (output at `src-tauri/target/release/bundle/macos/Handy.app`).

**Uncommitted files** (from `git status`): Task 13 (References) backend + UI + sidebar + i18n changes. Must be committed as part of this plan.

## Development Approach

- **Testing approach**: Regular (code, then tests) — mirrors the existing TranscribeAction test patterns in this repo
- Each task ends with its own verification before the next begins
- Never skip the test checkbox
- Run `cargo test --lib` and `bun x tsc --noEmit && bun run lint` after backend or frontend changes respectively
- **Rebuild + reinstall** (`bun run tauri build --bundles app` then copy to `/Applications/Handy.app`) is deferred to the final verification task — do it ONCE after everything is stable

## Testing Strategy

- **Unit tests**: Rust unit tests live inside `src-tauri/src/managers/history.rs` and similar `#[cfg(test)]` modules. Add tests co-located with new logic.
- **Integration**: No e2e harness for UI in Handy; rely on type-checking (`tsc --noEmit`) and ESLint as the frontend safety net.
- **Manual smoke test** in final verification: quit + relaunch `/Applications/Handy.app`, exercise new features.

## Progress Tracking

- Mark completed items with `[x]` immediately when done
- Add newly discovered tasks with ➕ prefix
- Document blockers with ⚠️ prefix
- Update plan if scope shifts

## Implementation Steps

### Task 1: Finish + ship the References feature (Task 13 in TaskList)

The code is already written but unbuilt. Seal it off before moving on.

- [x] run `cargo test --lib managers::history::` — must pass (the `kind` column migration + updated helpers should already be green)
- [x] run `bun x tsc --noEmit` — verify `ReferencesSettings.tsx` compiles against the current shim types
- [x] run `bun run lint` — ESLint clean
- [x] ensure `correctionCommands.listCorrections` in `src/lib/corrections.ts` correctly passes `kind: null` when omitted (check Tauri command signature match with backend `Option<String>`)
- [x] manual-verify plan: after rebuild, launch app → sidebar should show a new "References" entry with a 📖 icon between Corrections and AI Mode
- [x] write tests: an integration-style Rust test in `managers/history.rs#tests` that inserts a correction with `kind="reference"`, calls `list_corrections(10, Some("reference"))`, and asserts the single row comes back (the existing in-memory `setup_conn()` doesn't create the `corrections` table yet — extend it or add a new `setup_corrections_conn()` fixture)
- [x] run tests

### Task 2: Rebrand Handy → Roary Mic

User-visible strings, not the Rust crate name.

- [x] `src-tauri/tauri.conf.json`: change `productName` `"Handy"` → `"Roary Mic"`; change `identifier` `"com.pais.handy"` → `"com.harunjen.roarymic"`; leave `mainBinaryName` unset (binary stays `handy`)
- [x] `src-tauri/tauri.conf.json`: remove Windows `signCommand` (references `CJ-Signing` cert we don't have); set `bundle.windows.signCommand` to `null` or delete
- [x] `package.json`: update `productName` if present; leave Rust `package.name` in `Cargo.toml` alone (many files `use handy::*`) — no `productName` field in `package.json`, left untouched; `name` stays `handy-app`
- [x] `src/components/icons/HandyTextLogo.tsx`: inspect — the SVG likely contains the literal "Handy" text. Swap text to "Roary Mic" while keeping the same logo styling. If the SVG uses a font-based text node, update the string; if it's pre-rasterized paths, add a small text overlay as an interim measure. — replaced pre-rasterized `<path>` logo with a font-based `<text>` node rendering `Roary Mic`, keeping the existing `logo-primary` class so theme colors still apply.
- [x] grep `src/` for literal `"Handy"` in JSX/TSX (not imports) — replace user-facing ones with `"Roary Mic"`. Skip comments and imports like `HandyHand`, `HandyTextLogo` (internal names). — no user-facing JSX literals required changing; the only remaining `"Handy"` label is `KeyboardImplementationSelector.tsx`'s `"Handy Keys"` which is an internal implementation name paired with the `handy_keys.rs` Rust module (left intact, matches guidance).
- [x] grep `src/i18n/locales/en/translation.json` for `"Handy"` in any string values, replace with `"Roary Mic"`
- [x] update `src-tauri/src/tray.rs` / `tray_i18n.rs` tooltip + menu strings that say "Handy" — only `tray.rs::version_label()` hardcoded "Handy"; `tray_i18n.rs` is auto-generated from the locale files so updating `en/translation.json` was sufficient. Also updated `cli.rs` `--help` text and `lib.rs` WebviewWindow title.
- [x] **record which files changed** so the same rebrand can be applied to the other 19 locales later (out of scope for this task) — files touched this iteration: `src-tauri/tauri.conf.json`, `src-tauri/src/tray.rs`, `src-tauri/src/cli.rs`, `src-tauri/src/lib.rs`, `src/components/icons/HandyTextLogo.tsx`, `src/i18n/locales/en/translation.json`. Locale keys that contained "Handy" and now say "Roary Mic": `settings.permissions.description`, `settings.general.shortcut.title`, `settings.general.autostart.description`, `settings.general.showTrayIcon.description`, `settings.corrections.description`, `settings.corrections.empty`, `settings.general.updateChecks.description`, `settings.about.version.description`, `settings.about.appDataDirectory.description`, `settings.about.supportDevelopment.description`, `settings.about.acknowledgments.whisper.details`, `accessibility.permissionsDescription`, `appLanguage.description` — mirror these across the other 19 locale files in a follow-up.
- [x] run `cargo fmt --check && bun run lint && bun x tsc --noEmit` — all clean
- [x] add a test: a Rust test that reads `tauri.conf.json` and asserts `productName == "Roary Mic"` (defensive, catches accidental revert) — `rebrand_tests::tauri_conf_product_name_is_roary_mic` in `src-tauri/src/lib.rs` also asserts the new identifier.
- [x] ⚠️ Document in plan that the bundle identifier change will invalidate TCC (macOS) permissions — user must re-grant Microphone, Accessibility, Screen Recording on first relaunch after installing the rebuilt app (see Post-Completion section below).

### Task 3: AI mode output options window (paste or speak via ElevenLabs)

When the AI-mode hotkey fires and the LLM replies, instead of auto-pasting, open a small always-on-top window showing the text with two buttons: **Paste** and **Speak**. "Speak" sends the text to ElevenLabs TTS and plays the MP3 through the default audio output.

- [ ] add `src-tauri/src/tts.rs`: `fn speak_via_elevenlabs(api_key: &str, voice_id: &str, model_id: &str, text: &str) -> Result<Vec<u8>>` — POST to `https://api.elevenlabs.io/v1/text-to-speech/{voice_id}`, header `xi-api-key`, body `{"text": text, "model_id": model_id}`, return MP3 bytes. Add tests for header construction and error-body formatting using a mock (consider `wiremock` or just split URL building into a pure helper).
- [ ] add playback: existing `rodio` dep — create a `Decoder::new_mp3` source from the bytes and play on default sink
- [ ] settings.rs: add `ai_mode_output_mode: String` (`"auto_paste"` | `"prompt_window"`, default `"prompt_window"`), `elevenlabs_api_key: String` (store in `SecretMap`-style, not plaintext), `elevenlabs_voice_id: String` (default `"21m00Tcm4TlvDq8ikWAM"` — Rachel), `elevenlabs_model_id: String` (default `"eleven_turbo_v2_5"`)
- [ ] `tauri.conf.json`: register a second window config `ai_reply` with label `ai-reply`, initially hidden, frameless, always-on-top, 400×280, non-resizable, center-screen
- [ ] `actions.rs::handle_ai_mode_completion`: when `ai_mode_output_mode == "prompt_window"`, instead of calling `utils::paste`, emit event `ai-mode-reply-ready` with payload `{ text: String }` and call `show_ai_reply_window(app)`
- [ ] add commands: `ai_reply_paste(text: String)` — pastes the text and hides the window; `ai_reply_speak(text: String)` — calls `tts.rs`, plays, hides the window; `ai_reply_cancel()` — just hides
- [ ] create `src/ai-reply/` entry: `index.html`, `main.tsx`, `AiReplyWindow.tsx` — listens for `ai-mode-reply-ready`, shows the text in a read-only textarea, three buttons: Paste (default, Enter), Speak, Cancel (Escape). Auto-focus on Paste button.
- [ ] update `vite.config.ts` to build `ai-reply/index.html` as a second entry
- [ ] `i18n/locales/en/translation.json`: new block `settings.aiMode.outputMode`, `settings.aiMode.elevenlabs*`, `aiReplyWindow.*`
- [ ] `AiModeSettings.tsx`: add output-mode dropdown (Auto-paste / Ask me), ElevenLabs API key field (password), voice ID text field with a help link to https://elevenlabs.io/voices
- [ ] tests: unit test for `tts.rs` URL + header building; unit test for settings default values
- [ ] run `cargo test && cargo fmt --check && bun run lint && bun x tsc --noEmit`

### Task 4: AI mode plugin — Claude Code subprocess provider

Give users a subscription-backed option that shells out to the locally-installed `claude` CLI (Paperclip-style). Zero API key, uses their `claude login` credentials. Only offered when `claude` is on PATH.

- [ ] add detection helper in `actions.rs` or a new `src-tauri/src/claude_code.rs`: `fn claude_cli_available() -> Option<PathBuf>` — uses `which::which("claude")` (add `which = "6"` to Cargo.toml if not present)
- [ ] new module `src-tauri/src/claude_code.rs`: `async fn run_claude_code(prompt: &str, image_png: Option<&[u8]>, system_prompt: Option<&str>) -> Result<String>` — spawns `claude -p <stdin> --output-format stream-json --verbose --dangerously-skip-permissions`, parses the newline-delimited JSON messages, extracts the assistant text. For image attachment, save PNG to a temp file and include `--image /path/to.png` (verify the CLI flag via `claude --help`; if not present, inline as base64 in the prompt text with a note "there is an image at the top of this message" and rely on the default behavior — document the limitation).
- [ ] register a synthetic provider in `settings.rs::default_post_process_providers()`: `id: "claude_code_local"`, `label: "Claude Code (local subscription)"`, `base_url: "claude-code-local://"`, `supports_structured_output: false` — the fake URL tags it as a non-HTTP provider
- [ ] route switcher in `llm_client.rs::send_chat_completion_multimodal`: if `provider.id == "claude_code_local"`, bypass `reqwest` and delegate to `claude_code::run_claude_code`
- [ ] in `AiModeSettings.tsx`: when the provider selector shows `claude_code_local`, hide the API key field (not needed), show a note: "Requires Claude Code CLI (`claude login`). Routes traffic through your Claude subscription. See Anthropic ToS." Link to https://docs.claude.com/en/docs/claude-code/overview
- [ ] add a health-check command `check_claude_code_available() -> bool`; `AiModeSettings.tsx` calls it on mount and disables the `claude_code_local` option with an explanatory tooltip if false
- [ ] tests: unit test that parses a sample stream-json from Claude Code into the final assistant text; unit test for the provider-routing switch in `send_chat_completion_multimodal`
- [ ] run `cargo test && cargo fmt --check && bun run lint && bun x tsc --noEmit`

### Task 5: Verify acceptance criteria

- [ ] verify all requirements from Overview are implemented
- [ ] run full test suite: `cargo test --lib`
- [ ] run full lint: `bun run lint` + `bun x tsc --noEmit`
- [ ] run format check: `cargo fmt --check` + `bun x prettier --check 'src/**/*.{ts,tsx}'`
- [ ] build release: `CMAKE_POLICY_VERSION_MINIMUM=3.5 bun run tauri build --bundles app` — exit 0 (Tauri updater signing error about `TAURI_SIGNING_PRIVATE_KEY` is expected and ignorable)

### Task 6: Install & commit

- [ ] replace `/Applications/Handy.app` (will be `/Applications/Roary Mic.app` after rebrand) with the freshly built bundle — `rm -rf /Applications/Handy.app /Applications/Roary\ Mic.app` (don't fail if missing) then `cp -R src-tauri/target/release/bundle/macos/<Name>.app /Applications/`
- [ ] `git add` each changed file explicitly (NEVER `git add -A`)
- [ ] commit with a Conventional Commit header summarizing the four features; include a note about the TCC reset caused by the bundle-id change
- [ ] push to `origin` (roary-mic remote) — existing branch `roary-main`

## Technical Details

**References kind column** — already added in Task 13's uncommitted work:
```sql
ALTER TABLE corrections ADD COLUMN kind TEXT NOT NULL DEFAULT 'correction';
```
Values: `"correction"` (Whisper edits + manual corrections) | `"reference"` (phrase expansions).

**AI mode output-mode state machine**:
```
hotkey pressed → record → transcribe → screenshot → LLM call → reply ready
  ├─ output_mode == "auto_paste"     → paste directly (current behavior)
  └─ output_mode == "prompt_window"  → emit event, show ai-reply window
                                         ├─ user picks Paste  → commands.ai_reply_paste
                                         ├─ user picks Speak  → commands.ai_reply_speak → ElevenLabs MP3 → rodio
                                         └─ user picks Cancel → close
```

**Claude Code subprocess protocol** (Paperclip reference):
```
echo "$PROMPT" | claude -p - --output-format stream-json --verbose --dangerously-skip-permissions
```
Output is newline-delimited JSON; filter for `type == "assistant"` messages and concatenate their `content` text parts.

## Post-Completion

**Manual verification** (user must do):
- Quit Handy (tray → Quit)
- First launch of `/Applications/Roary Mic.app` will prompt for Microphone, Accessibility, and Screen Recording permissions again (new bundle ID = fresh TCC state). Grant all three.
- Settings → References: add `my email` → `daniel.harunjen@fintech-farm.com`. Dictate "Please email me at my email" — expect the expansion to land in the target app.
- Settings → AI Mode: enable, pick "Claude Code (local subscription)" if `claude` CLI is logged in; otherwise pick Anthropic + paste API key. Choose output mode "Ask me". Press `⌘⇧Space` and say "summarize what's on screen". The AI Reply window should appear with two buttons.
- If ElevenLabs key configured, click Speak and confirm audio plays.

**External system updates**:
- None. This is a personal fork; no upstream PR.

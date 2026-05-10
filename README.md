# Roary Mic 🦁

**Speech-to-text on your Mac. Press a shortcut, speak, type appears. No internet needed.**

[![Download for Mac](https://img.shields.io/badge/Download_for_Mac-Apple_Silicon-000?style=for-the-badge&logo=apple&logoColor=white)](https://github.com/danielharunjen-glitch/roary-mic/releases/latest)

> Currently builds for **Apple Silicon Macs only** (M1/M2/M3/M4). The upstream [Handy](https://github.com/cjpais/handy) project this fork is based on supports Windows and Linux.

## Install (60 seconds, no terminal needed)

1. Click the **Download** button above. You'll land on the latest release page — click the file ending in `.dmg`.
2. Open the downloaded `.dmg` file.
3. In the window that opens, **drag the 🦁 Roary Mic icon onto the Applications folder**.
4. Open your **Applications** folder, **right-click** Roary Mic, and choose **Open**.
   On the popup that warns about an unidentified developer, click **Open** again.

   > **Why right-click?** macOS shows a one-time warning for apps not bought through the App Store. Right-click → Open is the bypass. After this first launch, double-click works normally.
5. When asked, grant **Microphone** and **Accessibility** permissions.
6. Press your shortcut and start talking.

That's it. No installer wizard, no terminal, no account.

### "macOS says the app is damaged"

If macOS blocks the app with an "is damaged" message instead of the right-click → Open flow, run this once in Terminal:

```bash
xattr -dr com.apple.quarantine "/Applications/Roary Mic.app"
```

Then double-click the app normally.

---

## What is Roary Mic?

A free, open-source, privacy-first dictation app for macOS. Transcription runs **entirely on your Mac** — your voice never leaves the device.

- **Free** — no paywall, no subscription
- **Open source** — fork it, change it, ship it
- **Private** — no cloud calls, no telemetry
- **Simple** — one tool, one job

Roary Mic is a fork of [`cjpais/handy`](https://github.com/cjpais/handy), customized for our preferred UX and shipping.

## How it works

1. **Press** a configurable keyboard shortcut to start/stop recording (or push-to-talk).
2. **Speak** while the shortcut is active.
3. **Release** — Roary Mic transcribes locally using Whisper or Parakeet V3.
4. **Get** the text pasted into whatever app you're using.

Silence is filtered with [Silero VAD](https://github.com/snakers4/silero-vad). Models supported:

- **Whisper** (Small / Medium / Turbo / Large) — Apple Metal accelerated
- **Parakeet V3** — CPU-optimized with automatic language detection

## CLI parameters

The app binary contains a space, so quote the path when invoking it:

```bash
"/Applications/Roary Mic.app/Contents/MacOS/Roary Mic" --toggle-transcription
```

**Remote control flags** (sent to the running instance):

| Flag                     | Action                                       |
| ------------------------ | -------------------------------------------- |
| `--toggle-transcription` | Toggle recording on/off                      |
| `--toggle-post-process`  | Toggle recording with post-processing on/off |
| `--cancel`               | Cancel the current operation                 |

**Startup flags:**

| Flag             | Action                                      |
| ---------------- | ------------------------------------------- |
| `--start-hidden` | Launch without showing the main window      |
| `--no-tray`      | Launch without the system tray icon         |
| `--debug`        | Verbose (Trace) logging                     |
| `--help`         | Print all flags                             |

## Manual model install (restricted networks)

If your network blocks the model CDN, install models by hand:

1. Open Roary Mic → Settings → **About** → copy the *App Data Directory* path. It's usually:

   ```
   ~/Library/Application Support/com.harunjen.roarymic/
   ```

2. Inside that folder, create a `models/` directory if it doesn't exist:

   ```bash
   mkdir -p ~/Library/Application\ Support/com.harunjen.roarymic/models
   ```

3. Download the models you want (these are upstream Handy's CDN):

   **Whisper (single `.bin` files)**
   - Small (487 MB) — `https://blob.handy.computer/ggml-small.bin`
   - Medium (492 MB) — `https://blob.handy.computer/whisper-medium-q4_1.bin`
   - Turbo (1.6 GB) — `https://blob.handy.computer/ggml-large-v3-turbo.bin`
   - Large (1.1 GB) — `https://blob.handy.computer/ggml-large-v3-q5_0.bin`

   **Parakeet (`.tar.gz` archives)**
   - V2 (473 MB) — `https://blob.handy.computer/parakeet-v2-int8.tar.gz`
   - V3 (478 MB) — `https://blob.handy.computer/parakeet-v3-int8.tar.gz`

4. **Whisper:** drop the `.bin` file into `models/` as-is.

   **Parakeet:** extract the archive and place the resulting directory in `models/`. The directory name must be exactly `parakeet-tdt-0.6b-v2-int8` or `parakeet-tdt-0.6b-v3-int8`.

5. Restart Roary Mic. The models appear under Settings → Models as "Downloaded."

### Custom Whisper models

Drop any Whisper GGML `.bin` file into `models/`. Roary Mic auto-discovers it and lists it under Custom Models. Source: [Hugging Face](https://huggingface.co/models?search=whisper%20ggml).

## Troubleshooting

**App doesn't open / Gatekeeper blocks it.** See the install steps above — right-click → Open the first time, or run the `xattr` command if it says "damaged."

**Mic permission stuck.** System Settings → Privacy & Security → Microphone → toggle Roary Mic off and back on. Quit the app fully (Cmd+Q) and reopen.

**Accessibility permission stuck.** Same path under Privacy & Security → Accessibility. Permissions are tied to the code-signing hash, so a fresh build invalidates them — you'll need to re-grant.

**Auto-capture corrections.** Off by default. Settings → Corrections → "Save correction?" toggle. Roary Mic watches the field you just dictated into and offers to learn from edits as future corrections.

**Debug menu.** `Cmd+Shift+D` opens the debug panel.

## Development

For build setup, see [BUILD.md](BUILD.md). Quick version:

```bash
bun install
bun run tauri dev
```

To rebuild and reinstall to `/Applications/Roary Mic.app` in one step:

```bash
bun run ship                 # commit, build, install, relaunch
bun run ship --skip-commit   # build + install only
```

## Architecture

Tauri 2.x app — Rust backend, React/TypeScript frontend.

- `src-tauri/` — Rust managers (audio, model, transcription, history), commands, CLI, shortcuts
- `src/` — React UI, settings, onboarding, recording overlay
- Audio pipeline: **device → VAD (Silero) → Whisper/Parakeet → clipboard/paste**

Key crates: `whisper-rs`, `transcribe-rs`, `cpal`, `vad-rs`, `rdev`, `rubato`.

## Credits

Roary Mic is a fork of [**Handy**](https://github.com/cjpais/handy) by [@cjpais](https://github.com/cjpais). The upstream project does the heavy lifting — the audio pipeline, the model integration, the cross-platform Tauri app. This fork is mostly editorial and UX polish on top.

If you want speech-to-text on Windows or Linux, or want to contribute to the core engine, go upstream.

## License

MIT — see [LICENSE](LICENSE).

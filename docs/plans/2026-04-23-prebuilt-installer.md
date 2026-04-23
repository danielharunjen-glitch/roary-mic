# Prebuilt installer for Roary Mic (macOS DMG)

## Overview

Today, installing Roary Mic requires cloning the fork, running `bun install`, `bun run tauri build`, and copying `.app` into `/Applications` — or running `bun run ship` once that script is on disk. This is fine for the maintainer but prohibitive for anyone else (and annoying even for the maintainer on a fresh machine).

This plan adds a prebuilt, downloadable installer for macOS: a signed, notarized `.dmg` published as a GitHub release so the app can be installed in two clicks. The release pipeline runs on CI so the maintainer doesn't need to sign locally.

**macOS only for V1.** Windows and Linux ports can follow in separate plans if there's demand.

## Context (from audit of existing state)

- Repo: `danielharunjen-glitch/roary-mic`, branch `roary-main`.
- Bundle: `tauri.conf.json` → `bundle.targets: "all"` already produces `.dmg` alongside `.app`. Verified locally.
- Current signing: `signingIdentity: "-"` (ad-hoc). Ad-hoc bundles are not notarizable and will throw "app is damaged" on fresh machines → must be replaced for distribution.
- Local install script: `scripts/ship-local.sh` exists for maintainer. This plan does NOT touch it.
- No `.github/workflows/` directory in the fork today.
- Upstream Handy has a release workflow — we may or may not fork it. Check before reinventing.

## Dependencies (external)

- **Paid Apple Developer ID** — required. Developer ID Application certificate + Developer ID Installer certificate (even for .dmg-only, notarization requires both to be present in CI). ~$99/yr.
- **App-specific password for notarization** — generated at appleid.apple.com; stored as a GitHub secret.
- **Team ID** — 10-char alphanumeric from developer.apple.com; stored as a GitHub secret.
- **GitHub Actions** (free tier is enough — one macOS build per release).

## Development Approach

- **Testing approach**: Manual verification only. CI release flows are inherently end-to-end; unit-testing bits of the release script adds little value over running the workflow and inspecting the artifact.
- One clean release cycle end-to-end before merging (tag a test version like `v0.8.2-ci-test`, run workflow, download DMG, install on a second Mac, verify no Gatekeeper warning).
- Keep the workflow minimal — no matrix, no complex caching, no Windows/Linux until someone asks.

## Testing Strategy

- No new unit tests.
- Manual E2E: see Post-Completion.

## Progress Tracking

- Mark completed items with `[x]` when done.
- Add newly discovered tasks with ➕ prefix.

## What Goes Where

- **Implementation Steps**: the workflow file, tauri.conf.json update, README install section, and secrets-setup doc.
- **Post-Completion**: actually signing up for the Developer Program, generating certs, running the first release.

## Implementation Steps

### Task 1: Decide on reusing upstream Handy's release workflow

- [ ] inspect `cjpais/Handy` on GitHub for an existing `.github/workflows/release.yml` or similar
- [ ] if upstream has a working workflow: document what we'd need to change (mostly: bundle identifier, app name, signing secrets). Otherwise plan to write ours from scratch.
- [ ] decision record: inline, one paragraph, in this plan under Technical Details

### Task 2: Add secrets + certificates to CI

- [ ] generate a Developer ID Application certificate on the Apple Developer portal and export as a .p12 (with a strong password)
- [ ] generate an app-specific password at appleid.apple.com (for `notarytool`)
- [ ] add GitHub repo secrets: `APPLE_CERTIFICATE` (base64-encoded .p12), `APPLE_CERTIFICATE_PASSWORD`, `APPLE_ID` (the developer account email), `APPLE_TEAM_ID` (10-char ID), `APPLE_APP_SPECIFIC_PASSWORD`
- [ ] document the secret names + rotation procedure in `docs/release-secrets.md` (gitignored entries only — never commit the actual secrets)

### Task 3: Write `.github/workflows/release.yml`

- [ ] trigger: push of any tag matching `v*`
- [ ] single job on `macos-14` (Apple Silicon)
- [ ] steps: checkout → install bun → install rust stable with aarch64-apple-darwin + x86_64-apple-darwin targets → import signing cert into a temporary keychain → `bun install` → `bun run tauri build -- --target universal-apple-darwin --bundles dmg` → notarize the resulting DMG via `notarytool submit --wait` → staple → upload as a GitHub release asset
- [ ] use `tauri-apps/tauri-action` if it covers this cleanly (it supports signing + notarization via env vars); fall back to hand-rolled steps if the action's assumptions don't match

### Task 4: Update `tauri.conf.json` for release builds

- [ ] leave the local-dev default as `signingIdentity: "-"` (so `bun run ship` still works without the paid cert)
- [ ] the CI workflow overrides `TAURI_SIGNING_IDENTITY` via env var for release builds — document that the JSON value is a fallback, not the source of truth for CI
- [ ] add `"minimumSystemVersion": "10.15"` confirmation (already present; just verify)
- [ ] confirm `"createUpdaterArtifacts": false` stays that way (Tauri updater is a separate concern, out of scope)

### Task 5: Update README + BUILD.md with install instructions

- [ ] add an "Install (macOS)" section at the top of README.md: "Download the latest `.dmg` from [Releases](link), drag to Applications. First launch: right-click → Open to bypass Gatekeeper on older macOS."
- [ ] `BUILD.md` keeps the from-source instructions; add a note at the top: "For most users, download the prebuilt DMG from Releases. Follow this doc only if you want to modify the source."
- [ ] cross-reference this plan from `AGENTS.md` under a "Release process" heading

### Task 6: Dry-run the workflow once

- [ ] tag a throwaway version (e.g., `v0.8.2-ci-test`) and push it
- [ ] confirm the workflow runs end-to-end without human intervention
- [ ] confirm the DMG downloads, mounts, installs cleanly, and launches without Gatekeeper warnings on a second Mac
- [ ] delete the test tag + test release once the process is known to work

### Task 7: Document how to cut a real release

- [ ] write a short `docs/releasing.md`: `git tag v0.8.3`, `git push origin v0.8.3`, watch the Actions tab, release notes appear as a draft — edit and publish
- [ ] include: how to bump the version in `package.json` and `src-tauri/Cargo.toml` / `tauri.conf.json` first (or document the single-source-of-truth)

## Technical Details

**Why notarization, not just signing.** On macOS 10.15+, Gatekeeper requires both a valid Developer ID signature AND a notarization ticket for apps distributed outside the Mac App Store. Without notarization, first-launch users see "app is damaged and can't be opened" — even with a valid signature. The first-launch-from-Applications UX depends on both.

**Universal binary vs. Apple Silicon only.** Target `universal-apple-darwin` (builds both arches into one .app). Roughly doubles build time (~20 min vs 10) and binary size. Worth it because many users still run Intel Macs, and releasing two separate DMGs is more annoying than one universal build.

**Certificate expiry.** Developer ID certificates expire every 5 years; app-specific passwords don't expire but can be revoked. Note the renewal date somewhere (the secrets doc in Task 2) so a release doesn't silently break in 2031.

**Coexistence with `ship-local.sh`.** The local ship script uses ad-hoc signing (or the optional self-signed identity from `setup-codesign-identity.sh`). The CI workflow overrides with the real Developer ID via environment variable. Both paths work — local ship stays fast, CI produces signed artifacts. No conflict.

**Version bumping.** Version must match across `package.json:version`, `src-tauri/Cargo.toml:version`, and `src-tauri/tauri.conf.json:version`. Currently manual. Task 7 docs note this. A `scripts/bump-version.sh` helper could automate it in a follow-up plan.

## Post-Completion

**Manual, one-time account setup (not automatable):**

- Enroll in the Apple Developer Program at developer.apple.com ($99/yr).
- Generate a Developer ID Application certificate; export as .p12.
- Generate an app-specific password at appleid.apple.com.
- Look up the team ID in the Developer portal membership page.

**Manual, first-release verification:**

- Tag `v<first-real-version>` and push.
- After the workflow completes, download the DMG from the GitHub release.
- On a second Mac that's never had Roary Mic installed:
  - Mount the DMG, drag to Applications, launch — confirm no Gatekeeper warnings.
  - `codesign -dv /Applications/Roary\ Mic.app` — confirm `Authority=Developer ID Application: <you>`.
  - `spctl --assess --verbose /Applications/Roary\ Mic.app` — confirm `accepted source=Notarized Developer ID`.
- If any of the above fails, the release is broken; debug before publishing.

**External system updates:**

- Update the roary-mic repo's README with a prominent "Download" link to the latest release.
- Consider adding the project to Homebrew (`brew install --cask roary-mic`) as a follow-up — requires a cask submission to homebrew-cask; out of scope for this plan.

**Out of scope for this plan (follow-ups if demand):**

- Windows installer (MSI via WiX or NSIS; needs Authenticode cert from Sectigo/Comodo).
- Linux packages (DEB / RPM / AppImage via `cargo-deb` / `cargo-rpm`; no signing story needed beyond checksum).
- Auto-update (Tauri's updater plugin; needs a signing key for updates distinct from the code-signing cert).
- Homebrew cask submission.
- Single-source-of-truth version file.

**Cross-references:**

- `docs/plans/completed/2026-04-23-tcc-install-correction-research.md` — local dev install tooling (`ship-local.sh`, `setup-codesign-identity.sh`).
- `docs/plans/2026-04-23-auto-capture-corrections.md` — current in-flight feature; doesn't depend on this plan and vice versa.

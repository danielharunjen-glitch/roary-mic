#!/usr/bin/env bash
#
# ship-local.sh — one-command rebuild + reinstall for Roary Mic on macOS.
#
# Runs from the repo root. Commits any uncommitted work (with confirmation),
# builds the .app bundle, replaces /Applications/Roary Mic.app, and relaunches.
#
# Flags:
#   --skip-commit   skip the git add/commit step; just build and install
#   --push          also `git push origin <current-branch>` after committing
#   --help          show this message
#
# Usage: bash scripts/ship-local.sh [--skip-commit] [--push]
#        bun run ship [--skip-commit] [--push]

set -euo pipefail

SKIP_COMMIT=0
PUSH=0

for arg in "$@"; do
    case "$arg" in
        --skip-commit) SKIP_COMMIT=1 ;;
        --push)        PUSH=1 ;;
        --help|-h)
            sed -n '2,14p' "$0" | sed 's/^# \{0,1\}//'
            exit 0
            ;;
        *)
            echo "Unknown flag: $arg" >&2
            echo "Run 'bash scripts/ship-local.sh --help' for usage." >&2
            exit 2
            ;;
    esac
done

if [[ "$(uname -s)" != "Darwin" ]]; then
    echo "ship-local.sh targets macOS only." >&2
    exit 1
fi

# Resolve repo root relative to this script so it works regardless of cwd.
REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

APP_NAME="Roary Mic"
BUNDLE_PATH="src-tauri/target/release/bundle/macos/$APP_NAME.app"
INSTALL_PATH="/Applications/$APP_NAME.app"

step() { printf "\n\033[1;34m==>\033[0m %s\n" "$*"; }

if [[ $SKIP_COMMIT -eq 0 ]]; then
    if [[ -n "$(git status --porcelain)" ]]; then
        step "Uncommitted changes detected:"
        git status --short
        read -r -p "Commit with auto message? [y/N] " reply
        if [[ "$reply" =~ ^[Yy]$ ]]; then
            git add -A
            git commit -m "build: $(date +%F-%H%M)"
        else
            echo "Aborting. Use --skip-commit to build without committing." >&2
            exit 1
        fi
    fi

    if [[ $PUSH -eq 1 ]]; then
        step "Pushing to origin"
        git push origin "$(git branch --show-current)"
    fi
else
    step "Skipping commit step (--skip-commit)"
fi

step "bun install"
bun install

step "Building .app bundle (this takes a while)"
# --bundles app skips dmg/updater artifacts for faster iteration.
bun run tauri build -- --bundles app

if [[ ! -d "$BUNDLE_PATH" ]]; then
    echo "Build succeeded but bundle not found at: $BUNDLE_PATH" >&2
    exit 1
fi

step "Stopping running instance (if any)"
pkill -x "$APP_NAME" 2>/dev/null || true

step "Replacing $INSTALL_PATH"
rm -rf "$INSTALL_PATH"
cp -R "$BUNDLE_PATH" "$INSTALL_PATH"

# Strip quarantine xattr so the app launches without Gatekeeper warning the
# first time. Harmless if no xattr is present.
xattr -cr "$INSTALL_PATH" 2>/dev/null || true

step "Launching"
open "$INSTALL_PATH"

echo
echo "Done. If Screen Recording permission was revoked by the rebuild,"
echo "toggle 'Roary Mic' in System Settings → Privacy & Security →"
echo "Screen Recording, then fully quit (Cmd+Q) and reopen."

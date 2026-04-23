#!/usr/bin/env bash
#
# setup-codesign-identity.sh
#
# Creates a stable self-signed code-signing identity named "RoaryMicLocal"
# in the login Keychain. This identity is used by `bun run tauri build`
# to sign the Roary Mic .app bundle with a stable designated requirement
# hash, so macOS TCC permissions (Screen Recording, Accessibility, etc.)
# persist across local rebuilds of the same bundle ID.
#
# One-time setup. Re-running is idempotent (no-op if the identity exists).
#
# You will be asked for your macOS login password ONCE — this is needed
# to set the key's partition list so that codesign can use the private
# key without GUI prompts on each subsequent build.
#
# To remove later:
#   security delete-identity -c "RoaryMicLocal" ~/Library/Keychains/login.keychain-db
#
# This cert is local-only. It does NOT distribute the app to other users.
# For distribution, use a paid Apple Developer ID instead.

set -euo pipefail

IDENTITY_NAME="RoaryMicLocal"
LOGIN_KEYCHAIN="$HOME/Library/Keychains/login.keychain-db"

if [[ "$(uname -s)" != "Darwin" ]]; then
    echo "This script only applies to macOS." >&2
    exit 1
fi

# Prefer Homebrew OpenSSL 3.x — LibreSSL on macOS sometimes produces p12
# files that the keychain accepts but can't link cert↔key into an identity.
if [[ -x /opt/homebrew/bin/openssl ]]; then
    OPENSSL=/opt/homebrew/bin/openssl
elif [[ -x /usr/local/bin/openssl ]]; then
    OPENSSL=/usr/local/bin/openssl
else
    OPENSSL=openssl
fi

if security find-identity -p codesigning "$LOGIN_KEYCHAIN" 2>/dev/null \
        | grep -Fq "\"$IDENTITY_NAME\""; then
    echo "Code-signing identity \"$IDENTITY_NAME\" already exists. Nothing to do."
    exit 0
fi

echo "Creating self-signed code-signing identity \"$IDENTITY_NAME\"..."
echo "You will be prompted for your macOS login password to unlock the keychain."
echo

# Read keychain password up-front so the rest runs without further prompts.
read -s -p "macOS login password: " KCPASS
echo
if [[ -z "$KCPASS" ]]; then
    echo "No password entered. Aborting." >&2
    exit 1
fi

# Verify password before doing any work.
if ! security unlock-keychain -p "$KCPASS" "$LOGIN_KEYCHAIN" 2>/dev/null; then
    echo "Password rejected. Aborting." >&2
    exit 1
fi

TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT

CONF="$TMPDIR/openssl.cnf"
cat > "$CONF" <<'EOF'
[ req ]
distinguished_name = dn
prompt             = no
x509_extensions    = v3

[ dn ]
CN = RoaryMicLocal

[ v3 ]
basicConstraints      = critical, CA:FALSE
keyUsage              = critical, digitalSignature
extendedKeyUsage      = critical, codeSigning
subjectKeyIdentifier  = hash
EOF

KEY="$TMPDIR/key.pem"
CERT="$TMPDIR/cert.pem"
P12="$TMPDIR/id.p12"
P12_PASS="roarymic-local-$RANDOM"

"$OPENSSL" req -x509 -new -nodes -newkey rsa:2048 -sha256 \
    -keyout "$KEY" -out "$CERT" -days 3650 \
    -config "$CONF" -extensions v3 >/dev/null 2>&1

# -legacy keeps the p12 format compatible with macOS security(1).
# If OpenSSL doesn't support -legacy (e.g. LibreSSL), retry without it.
if ! "$OPENSSL" pkcs12 -export -legacy \
        -inkey "$KEY" -in "$CERT" -out "$P12" \
        -name "$IDENTITY_NAME" -passout "pass:$P12_PASS" >/dev/null 2>&1; then
    "$OPENSSL" pkcs12 -export \
        -inkey "$KEY" -in "$CERT" -out "$P12" \
        -name "$IDENTITY_NAME" -passout "pass:$P12_PASS" >/dev/null 2>&1
fi

# Import with -A so any app can use the key without per-app ACL restrictions.
# The partition list is set separately below.
security import "$P12" -k "$LOGIN_KEYCHAIN" -P "$P12_PASS" -A >/dev/null

# Add the cert to the user's trust store for code signing. This does not
# require sudo when targeting the user domain (policy domain defaults).
security add-trusted-cert -r trustAsRoot -p codeSign \
    -k "$LOGIN_KEYCHAIN" "$CERT" 2>/dev/null || {
    echo "Note: could not mark the cert as trusted. Codesign may prompt once on first use." >&2
}

# Set the key's partition list so codesign can use the private key without
# triggering a GUI "allow access" dialog on every build. This requires the
# keychain password.
security set-key-partition-list -S apple-tool:,apple:,codesign: \
    -s -k "$KCPASS" "$LOGIN_KEYCHAIN" >/dev/null 2>&1 || {
    echo "Warning: could not set the key's partition list." >&2
    echo "You may see a keychain GUI prompt the first time you build." >&2
    echo "Click 'Always Allow' and future builds will work without prompting." >&2
}

echo
echo "Verifying..."
if security find-identity -p codesigning "$LOGIN_KEYCHAIN" \
        | grep -Fq "\"$IDENTITY_NAME\""; then
    echo "Success. Identity \"$IDENTITY_NAME\" is installed."
    echo
    echo "Next steps:"
    echo "  1. Set \"signingIdentity\": \"$IDENTITY_NAME\" in src-tauri/tauri.conf.json"
    echo "  2. Run: bun run tauri build"
    echo "  3. If you see a keychain GUI prompt on first build, click 'Always Allow'."
else
    echo "ERROR: identity \"$IDENTITY_NAME\" not found after import." >&2
    exit 1
fi

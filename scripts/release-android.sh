#!/usr/bin/env bash
# Builds and signs the Android package and attaches it to a published release.
# The signing key never leaves this machine, so the release workflow cannot do it.
#
#   ./scripts/release-android.sh v0.2.0
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TAG="${1:?usage: release-android.sh <tag>}"
cd "$HERE"

VERSION="$(node -p "require('./src-tauri/tauri.conf.json').version")"
[[ "$TAG" == "v$VERSION" ]] || { echo "Tag $TAG does not match version $VERSION." >&2; exit 1; }
[[ -f src-tauri/gen/android/keystore.properties ]] || { echo "No src-tauri/gen/android/keystore.properties: a phone will not install an unsigned APK." >&2; exit 2; }
gh release view "$TAG" >/dev/null || { echo "Release $TAG is not published yet; wait for the release workflow." >&2; exit 1; }

./scripts/build-android.sh
./scripts/check-apk.sh

NAME="grt-bible-$VERSION-android-arm64.apk"
OUT="$(mktemp -d)"
trap 'rm -rf "$OUT"' EXIT
cp src-tauri/gen/android/app/build/outputs/apk/universal/release/app-universal-release.apk "$OUT/$NAME"
gh release upload "$TAG" "$OUT/$NAME" --clobber

# The release notes promise SHA256SUMS covers every file, the APK included.
gh release download "$TAG" --pattern SHA256SUMS --dir "$OUT"
grep -v "  $NAME\$" "$OUT/SHA256SUMS" > "$OUT/sums" || true
(cd "$OUT" && sha256sum "$NAME" >> sums && mv sums SHA256SUMS && sha256sum -c SHA256SUMS --ignore-missing)
gh release upload "$TAG" "$OUT/SHA256SUMS" --clobber
echo "Attached $NAME to $TAG."

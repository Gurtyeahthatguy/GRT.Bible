#!/usr/bin/env bash
# Lists what an APK asks of the phone, then reads its library the way check-build.sh reads the desktop binary.
#
#   ./scripts/check-apk.sh src-tauri/gen/android/app/build/outputs/apk/universal/release/app-universal-release.apk
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
APK="${1:-$HERE/../src-tauri/gen/android/app/build/outputs/apk/universal/release/app-universal-release.apk}"
SDK="${ANDROID_HOME:-$HOME/Android/Sdk}"
AAPT="$(ls "$SDK"/build-tools/*/aapt2 2>/dev/null | tail -1)"

if [[ ! -f "$APK" ]]; then
  echo "No APK at $APK" >&2
  exit 2
fi
if [[ -z "$AAPT" ]]; then
  echo "No aapt2 in $SDK/build-tools; set ANDROID_HOME" >&2
  exit 2
fi

echo "Checking: $APK ($(du -h "$APK" | cut -f1))"
FAILED=0

echo "[1/2] Permissions"
PERMISSIONS=$("$AAPT" dump permissions "$APK" | grep '^uses-permission' | sed "s/.*name='//;s/'.*//" | grep -v 'DYNAMIC_RECEIVER_NOT_EXPORTED_PERMISSION' | sort -u)
if [[ -n "$PERMISSIONS" ]]; then
  echo "$PERMISSIONS" | sed 's/^/  ASKS FOR: /'
  FAILED=1
else
  echo "  OK: none, so the program cannot open a connection"
fi

echo "[2/2] Library"
STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT
unzip -q -o "$APK" 'lib/*/libgrt_bible_lib.so' -d "$STAGE"
LIBS=("$STAGE"/lib/*/libgrt_bible_lib.so)
if [[ ! -e "${LIBS[0]}" ]]; then
  echo "  FOUND: no library in the package"
  exit 1
fi
for lib in "${LIBS[@]}"; do
  "$HERE/check-build.sh" "$lib" | sed 's/^/  /'
  [[ ${PIPESTATUS[0]} -eq 0 ]] || FAILED=1
done

[[ $FAILED -eq 0 ]] && echo "Clean." || echo "Something to fix before distributing."
exit $FAILED

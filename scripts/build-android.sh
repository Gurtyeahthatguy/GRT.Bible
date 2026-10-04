#!/usr/bin/env bash
# Builds the Android package with the tools this project keeps under ~/Android.
#
#   ./scripts/build-android.sh                    an APK for phones
#   ./scripts/build-android.sh --target x86_64    an APK for the emulator
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
export JAVA_HOME="${JAVA_HOME:-$HOME/Android/jdk-17}"
export ANDROID_HOME="${ANDROID_HOME:-$HOME/Android/Sdk}"
export NDK_HOME="${NDK_HOME:-$(ls -d "$ANDROID_HOME"/ndk/* 2>/dev/null | tail -1)}"
# Cargo ignores the remapping in .cargo/config.toml once the CLI sets its own flags.
export RUSTFLAGS="${RUSTFLAGS:---remap-path-prefix=/home=/redacted --remap-path-prefix=/Users=/redacted}"

for tool in "$JAVA_HOME/bin/java" "$ANDROID_HOME/platform-tools/adb" "$NDK_HOME/source.properties"; do
  [[ -e "$tool" ]] || { echo "Missing $tool; see the Android section of the README." >&2; exit 2; }
done

cd "$HERE"
if [[ $# -eq 0 ]]; then
  set -- --target aarch64
fi
npx tauri android build --apk "$@"

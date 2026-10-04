#!/usr/bin/env bash
# Looks for build paths, network addresses, telemetry and symbols in a release binary.
#
#   ./scripts/check-build.sh target/release/grt-bible
set -uo pipefail

BINARY="${1:-}"
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ALLOWED_FILE="$HERE/allowed-strings.txt"

if [[ -z "$BINARY" || ! -f "$BINARY" ]]; then
  echo "Usage: $0 <binary-path>" >&2
  exit 2
fi
command -v strings >/dev/null 2>&1 || { echo "'strings' is not available (binutils)" >&2; exit 2; }

echo "Checking: $BINARY ($(du -h "$BINARY" | cut -f1))"
FAILED=0

echo "[1/4] Absolute paths"
PATHS=$(strings "$BINARY" | grep -E '(/home/[a-zA-Z0-9._-]+/|/Users/[a-zA-Z0-9._-]+/|[A-Z]:\\Users\\)' | sort -u)
if [[ -n "$PATHS" ]]; then
  echo "$PATHS" | head -20 | sed 's/^/  FOUND: /'
  FAILED=1
else
  echo "  OK"
fi

echo "[2/4] Network addresses"
TLDS='(com|org|net|io|dev|app|edu|gov|info|biz|xyz|tv|me|ai|co|uk|de|fr|it|es|nl|eu|us|ca|au|ru|cn|jp|in|br|be)'
ALLOWED=$(grep -vE '^\s*(#|$)' "$ALLOWED_FILE" | sed 's/[.[\*^$()+?{|]/\\&/g' | paste -sd '|')
URLS=$(strings "$BINARY" | grep -Eo 'https?://(localhost|[a-zA-Z0-9-]+(\.[a-zA-Z0-9-]+)+)(:[0-9]+)?(/[a-zA-Z0-9./_?=-]*)?' | sort -u)
[[ -n "$ALLOWED" ]] && URLS=$(echo "$URLS" | grep -Ev "^($ALLOWED)" | grep -v '^$')
REAL=$(echo "$URLS" | grep -Ei "^https?://([a-zA-Z0-9-]+\.)*$TLDS(:[0-9]+)?(/|$)")
if [[ -n "$REAL" ]]; then
  echo "$REAL" | head -20 | sed 's/^/  NOT ALLOWED: /'
  FAILED=1
else
  echo "  OK"
fi

echo "[3/4] Telemetry"
TELEMETRY=$(strings "$BINARY" | grep -Eio '(sentry[-_](core|types|panic)|getsentry|bugsnag|datadoghq|mixpanel|amplitude\.com|google-analytics|posthog|crashlytics)' | sort -u)
if [[ -n "$TELEMETRY" ]]; then
  echo "$TELEMETRY" | sed 's/^/  FOUND: /'
  FAILED=1
else
  echo "  OK"
fi

echo "[4/4] Debug symbols"
if command -v file >/dev/null 2>&1 && file "$BINARY" | grep -q 'not stripped'; then
  echo "  FOUND: symbols present"
  FAILED=1
else
  echo "  OK"
fi

[[ $FAILED -eq 0 ]] && echo "Clean." || echo "Something to fix before distributing."
exit $FAILED

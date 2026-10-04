#!/usr/bin/env bash
# Lists every address the running program opens a socket to.
#
#   ./scripts/check-network.sh target/release/grt-bible
set -uo pipefail

BINARY="${1:-}"
if [[ -z "$BINARY" || ! -x "$BINARY" ]]; then
  echo "Usage: $0 <path-to-binary>" >&2
  exit 2
fi
command -v strace >/dev/null 2>&1 || { echo "strace is not installed" >&2; exit 2; }

LOG="$(mktemp -t grt-bible-network-XXXXXX.log)"
echo "Tracing $BINARY. Use it, then close it."
strace -f -qq -e trace=connect -o "$LOG" "$BINARY" >/dev/null 2>&1

HITS=$(grep -E 'connect\(.*(sin_addr|sin6_addr)' "$LOG" \
  | grep -v '127\.0\.0\.1' | grep -v 'inet6_addr("::1")' \
  | sed 's/.*inet_addr("\([^"]*\)").*/\1/;s/.*inet6_addr("\([^"]*\)").*/\1/' | sort -u)

if [[ -z "$HITS" ]]; then
  echo "No outbound connection was made."
else
  echo "Connections were made to:"
  echo "$HITS" | sed 's/^/  /'
fi
echo "Trace: $LOG"

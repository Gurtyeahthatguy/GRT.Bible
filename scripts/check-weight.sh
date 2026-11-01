#!/usr/bin/env bash
# Checks the program, its packages and the study packs against scripts/weight-budget.txt.
# Packages not built here are skipped, and say so.
#
#   ./scripts/check-weight.sh
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
FAILED=0

over() {
  local name="$1" value="$2" limit="$3" unit="$4"
  if [[ "$value" -gt "$limit" ]]; then
    printf '  OVER  %-44s %12s %s, budget %s\n' "$name" "$value" "$unit" "$limit"
    FAILED=1
  else
    printf '  ok    %-44s %12s %s, budget %s\n' "$name" "$value" "$unit" "$limit"
  fi
}

echo "[1/2] Files"
while read -r kind pattern limit; do
  [[ -z "$kind" || "$kind" == \#* ]] && continue
  [[ "$kind" == open_ms || "$kind" == read_ms || "$kind" == peak_kb ]] && continue
  # shellcheck disable=SC2086
  file=$(ls -1 $pattern 2>/dev/null | head -1)
  if [[ -z "$file" ]]; then
    printf '  --    %-44s not built here\n' "$pattern"
    continue
  fi
  over "$file" "$(stat -c %s "$file")" "$limit" bytes
done < <(grep -vE '^\s*#' scripts/weight-budget.txt | awk '{print $1, $2, $NF}')

echo "[2/2] Opening and reading every module"
cargo build --release -q -p grtb-convert
target/release/grtb-convert measure modules --packs packs >/dev/null
while read -r key value; do
  limit=$(awk -v k="$key" '$1 == k {print $NF}' scripts/weight-budget.txt)
  [[ -n "$limit" ]] && over "$key" "$value" "$limit" "${key##*_}"
done < <(target/release/grtb-convert measure modules --packs packs | grep -E '^(open_ms|read_ms|peak_kb) ')

[[ $FAILED -eq 0 ]] && echo "Within budget." || echo "Over budget."
exit $FAILED

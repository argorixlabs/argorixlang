#!/usr/bin/env bash
# Measures bootstrap/measure/stdlib_sample.argx at several sizes (ESP-009).
# Usage, from the repository root on Linux with gcc and GNU time:
#   bootstrap/measure/run.sh <argorixc> [sizes...]
# For each size it reports the result, the wall time and the peak resident
# memory of the program built with the default limits, and again with the
# step limit lifted, so the report shows both what fits the default budget
# and what the work costs. One JSON object per line on stdout.
set -euo pipefail
argorixc=${1:?argorixc path}
shift
sizes=${*:-100 1000 4000}
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
for members in $sizes; do
  sed "s/^const MEMBERS: u64 = [0-9]*u64;/const MEMBERS: u64 = ${members}u64;/" \
    bootstrap/measure/stdlib_sample.argx > "$work/sample.argx"
  "$argorixc" --stdlib stdlib core-emit-c "$work/sample.argx" > "$work/sample.c"
  for budget in default lifted; do
    flags=()
    if [ "$budget" = lifted ]; then
      flags=(-DARGORIX_STEP_LIMIT=18446744073709551615ULL)
    fi
    gcc -std=c11 -O2 "${flags[@]}" -Ibootstrap/c "$work/sample.c" \
      bootstrap/c/argorix_core_runtime.c -o "$work/sample"
    set +e
    /usr/bin/time -f '%e %M' -o "$work/time" "$work/sample" > "$work/out" 2> "$work/err"
    status=$?
    set -e
    read -r seconds rss < <(tail -n 1 "$work/time")
    outcome=$(cat "$work/out" "$work/err" | tr -d '\n')
    printf '{"members":%s,"budget":"%s","exit":%s,"outcome":"%s","seconds":%s,"max_rss_kb":%s}\n' \
      "$members" "$budget" "$status" "$outcome" "$seconds" "$rss"
  done
done

#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

echo "=== Layer 4: Generated Code Contract ==="

types_file="crates/types/src/clippy_toml.rs"

# Count pub fields inside the ClippyToml struct only.
field_count=$(awk '
  /^pub struct ClippyToml \{/ { in_struct = 1; next }
  in_struct && /^\}/ { in_struct = 0 }
  in_struct && /^    pub [a-z_]+:/ { count++ }
  END { print count + 0 }
' "$types_file")

if (( field_count < 90 )); then
  echo "FAIL: ClippyToml has $field_count fields (< 90)"
  exit 1
fi
if (( field_count > 200 )); then
  echo "FAIL: ClippyToml has $field_count fields (> 200)"
  exit 1
fi
echo "ok  ClippyToml field count = $field_count"

required_defaults=(
  "cognitive_complexity_threshold: 25"
  "too_many_arguments_threshold: 7"
  "too_many_lines_threshold: 100"
)
for line in "${required_defaults[@]}"; do
  if ! grep -qF "$line" "$types_file"; then
    echo "FAIL: missing default sample: $line"
    exit 1
  fi
  echo "ok  default: $line"
done

required_fields=(
  "pub msrv:"
  "pub disallowed_methods:"
  "pub disallowed_types:"
)
for line in "${required_fields[@]}"; do
  if ! grep -qF "$line" "$types_file"; then
    echo "FAIL: missing field declaration: $line"
    exit 1
  fi
  echo "ok  field: $line"
done

if ! grep -qF "pub type Msrv" "$types_file"; then
  echo "FAIL: missing Msrv type alias"
  exit 1
fi
echo "ok  Msrv type alias present"

echo "PASS: Layer 4"

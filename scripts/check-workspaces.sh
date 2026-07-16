#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
tool="${root}/tools/aqc-requirement-architecture"

if reverse_names="$(rg --line-number 'shackles|shakrs|shakts' \
    --glob 'Cargo.toml' --glob 'deny.toml' --glob '*.rs' \
    "${root}/packages" "${root}/tools")" && [[ -n "$reverse_names" ]]; then
    printf '%s\n' "AQC implementation names downstream product vocabulary:" "$reverse_names" >&2
    exit 1
fi

(cd "$tool" && cargo deny check)
cargo clippy --manifest-path "$tool/Cargo.toml" --all-targets --locked -- -D warnings
specular lint "${root}/specs/explicit-setting-membership.spec.json"
specular verify "${root}/specs/explicit-setting-membership.spec.json"

#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "${repo_root}/scripts/check-workspaces.sh"

workspace="$(mktemp -d)"
trap 'rm -rf "$workspace"' EXIT
touch "$workspace/Cargo.toml" "$workspace/deny.toml" "$workspace/clippy.toml"

cargo_home_for() {
    printf '%s\n' "$workspace/cargo-home"
}

cargo_target_for() {
    printf '%s\n' "$workspace/target"
}

cargo() {
    local stage="$1"
    if [[ "$stage" == "deny" ]]; then
        stage="deny"
    fi
    printf '%s\n' "$stage" >>"$workspace/calls"
    if [[ "$stage" == "$failure_stage" ]]; then
        return 7
    fi
    return 0
}

for failure_stage in metadata deny clippy test; do
    : >"$workspace/calls"
    if run_rust_workspace "$workspace"; then
        printf 'workspace gate accepted injected %s failure\n' "$failure_stage" >&2
        exit 1
    fi
    if [[ "$(tail -n 1 "$workspace/calls")" != "$failure_stage" ]]; then
        printf 'workspace gate continued after injected %s failure\n' "$failure_stage" >&2
        exit 1
    fi
done

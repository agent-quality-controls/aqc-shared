#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
gate_root="${AQC_GATE_CACHE_DIR:-${repo_root}/.cargo-target/gate}"
config_scope="${AQC_GATE_CONFIG_SCOPE:-working-tree}"
run_scope="${AQC_GATE_RUN_SCOPE:-${config_scope}-$$}"
shared_cargo_cache="${gate_root}/cargo-cache"
logs="${gate_root}/runs/${run_scope}/logs"

mkdir -p "$logs"
rm -f "${logs}"/*.log

find_workspaces() {
    local root="$1"
    local relative_root="${root#${repo_root}/}"
    find "$root" -name Cargo.toml \
        -not -path '*/target/*' \
        -not -path "${repo_root}/tools/aqc-requirement-architecture/tests/fixtures/*" \
        -exec dirname {} \;
    git -C "$repo_root" ls-files --cached -- ":(glob)${relative_root}/**/Cargo.toml" \
        | while IFS= read -r manifest; do
            case "$manifest" in
                tools/aqc-requirement-architecture/tests/fixtures/*) continue ;;
                */target/*) dirname "${repo_root}/${manifest}" ;;
            esac
        done
}

identity_for() {
    local value="${1#${repo_root}/}"
    printf '%s' "$value" | shasum -a 256 | cut -c1-16
}

cargo_home_for() {
    local manifest="$1"
    local identity
    local home

    identity="$(identity_for "$manifest")"
    home="${gate_root}/cargo-homes/${config_scope}/${identity}"
    mkdir -p "$home" "${shared_cargo_cache}/registry" "${shared_cargo_cache}/git"
    ln -sfn "${shared_cargo_cache}/registry" "$home/registry"
    ln -sfn "${shared_cargo_cache}/git" "$home/git"
    python3 "${repo_root}/scripts/local_cargo_source.py" \
        --root "$repo_root" \
        --config "$home/config.toml" \
        --manifest "$manifest"
    printf '%s\n' "$home"
}

cargo_target_for() {
    printf '%s\n' "${gate_root}/targets/$(identity_for "$1")"
}

run_rust_workspace() {
    set -euo pipefail

    local workspace="$1"
    local manifest="${workspace}/Cargo.toml"
    local cargo_home
    local cargo_target
    local clippy_config

    cargo_home="$(cargo_home_for "$manifest")" || return $?
    cargo_target="$(cargo_target_for "$manifest")" || return $?
    CARGO_HOME="$cargo_home" cargo metadata \
        --manifest-path "$manifest" \
        --locked \
        --format-version 1 \
        --no-deps \
        >/dev/null || return $?
    if [[ -f "${workspace}/deny.toml" ]]; then
        (cd "$workspace" && CARGO_HOME="$cargo_home" cargo deny check) || return $?
    fi
    clippy_config="$workspace"
    if [[ ! -f "${workspace}/clippy.toml" ]]; then
        clippy_config="$repo_root"
    fi
    CARGO_HOME="$cargo_home" CARGO_TARGET_DIR="$cargo_target" CLIPPY_CONF_DIR="$clippy_config" cargo clippy \
        --manifest-path "$manifest" \
        --all-targets \
        --locked \
        -- \
        -D warnings || return $?
    CARGO_HOME="$cargo_home" CARGO_TARGET_DIR="$cargo_target" cargo test \
        --manifest-path "$manifest" \
        --locked || return $?
}

run_rust_workspace_logged() {
    local workspace="$1"
    local log="${logs}/workspace-$(identity_for "$workspace").log"

    if run_rust_workspace "$workspace" >"$log" 2>&1; then
        printf 'pass: %s\n' "${workspace#${repo_root}/}"
        return 0
    fi
    printf 'fail: %s\n' "${workspace#${repo_root}/}" >&2
    cat "$log" >&2
    return 1
}

run_fixture_suite_logged() {
    local suite="$1"
    local log="${logs}/fixture-${suite}.log"

    if (cd "$repo_root" && env -u CARGO_TARGET_DIR fixture3 check --suite "$suite") >"$log" 2>&1; then
        printf 'pass: fixture3/%s\n' "$suite"
        return 0
    fi
    printf 'fail: fixture3/%s\n' "$suite" >&2
    cat "$log" >&2
    return 1
}

gate_jobs() {
    local processors
    processors="$(sysctl -n hw.logicalcpu 2>/dev/null || getconf _NPROCESSORS_ONLN 2>/dev/null || printf '2')"
    if ((processors > 4)); then
        printf '4\n'
    elif ((processors < 1)); then
        printf '1\n'
    else
        printf '%s\n' "$processors"
    fi
}

main() {
    if reverse_names="$(rg --line-number 'shackles|shakrs|shakts' \
        --glob 'Cargo.toml' --glob 'deny.toml' --glob '*.rs' \
        "${repo_root}/packages" "${repo_root}/tools")" && [[ -n "$reverse_names" ]]; then
        printf '%s\n' "AQC implementation names downstream product vocabulary:" "$reverse_names" >&2
        exit 1
    fi

    "${repo_root}/scripts/test-check-workspaces-failures.sh"
    python3 "${repo_root}/scripts/test-local-cargo-source.py"

    if specular lint "${repo_root}/specs/explicit-setting-membership.spec.json" \
        >"${logs}/specular-lint.log" 2>&1; then
        printf 'pass: specular/lint\n'
    else
        cat "${logs}/specular-lint.log" >&2
        exit 1
    fi
    if specular verify "${repo_root}/specs/explicit-setting-membership.spec.json" \
        >"${logs}/specular-verify.log" 2>&1; then
        printf 'pass: specular/verify\n'
    else
        cat "${logs}/specular-verify.log" >&2
        exit 1
    fi

    export repo_root gate_root config_scope run_scope shared_cargo_cache logs
    export -f identity_for cargo_home_for cargo_target_for run_rust_workspace run_rust_workspace_logged run_fixture_suite_logged
    jobs="${AQC_GATE_JOBS:-$(gate_jobs)}"
    (
        find_workspaces "${repo_root}/packages"
        find_workspaces "${repo_root}/tools"
    ) | sort -u | tr '\n' '\0' \
        | xargs -0 -n 1 -P "$jobs" bash -c 'run_rust_workspace_logged "$1"' _

    (cd "$repo_root" && fixture3 doctor)
    (cd "$repo_root" && fixture3 status --all --json) \
        | python3 -c 'import json, sys; [print(item["suite"]) for item in json.load(sys.stdin)["suites"]]' \
        | tr '\n' '\0' \
        | xargs -0 -n 1 -P "$jobs" bash -c 'run_fixture_suite_logged "$1"' _
}

if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
    main "$@"
fi

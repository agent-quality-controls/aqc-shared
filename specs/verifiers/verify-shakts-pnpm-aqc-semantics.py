#!/usr/bin/env python3
import json
import re
import shutil
import subprocess
import sys
import tempfile
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path


SPEC = Path(sys.argv[1]).resolve()
ENTRY = json.loads(SPEC.read_text())["requirements"]["custom"][int(sys.argv[3])]
ROOT = SPEC.parent.parent

LOCAL_PATCHES = {
    "aqc-file-engine-core": [],
    "aqc-json-engine-core": ["packages/aqc-file-engine-core"],
    "aqc-package-json-engine": [
        "packages/aqc-file-engine-core",
        "packages/file-types/json/aqc-json-engine-core",
    ],
    "aqc-yaml-engine-core": ["packages/aqc-file-engine-core"],
    "aqc-pnpm-workspace-yaml-engine": [
        "packages/aqc-file-engine-core",
        "packages/file-types/yaml/aqc-yaml-engine-core",
    ],
    "aqc-toml-engine-core": ["packages/aqc-file-engine-core"],
    "aqc-text-file-engine": ["packages/aqc-file-engine-core"],
    "aqc-cargo-toml-engine": [
        "packages/aqc-file-engine-core",
        "packages/file-types/toml/aqc-toml-engine-core",
    ],
    "aqc-clippy-toml-engine": [
        "packages/aqc-file-engine-core",
        "packages/file-types/toml/aqc-toml-engine-core",
    ],
    "aqc-deny-toml-engine": [
        "packages/aqc-file-engine-core",
        "packages/file-types/toml/aqc-toml-engine-core",
    ],
    "aqc-rust-toolchain-toml-engine": [
        "packages/aqc-file-engine-core",
        "packages/file-types/toml/aqc-toml-engine-core",
    ],
    "aqc-rustfmt-toml-engine": [
        "packages/aqc-file-engine-core",
        "packages/file-types/toml/aqc-toml-engine-core",
    ],
}


def emit(errors: list[str]) -> None:
    evidence = {"check": ENTRY["check"], "status": "fail" if errors else "pass"}
    if errors:
        evidence["message"] = "; ".join(errors)
    print(json.dumps(evidence))


def test_text(package: Path) -> str:
    tests = package / "tests"
    if not tests.is_dir():
        return ""
    return "\n".join(path.read_text() for path in sorted(tests.glob("**/*.rs"))).lower()


def require_concepts(errors: list[str], label: str, text: str, groups: list[tuple[str, ...]]) -> None:
    if not text:
        errors.append(f"{label}: contract tests are missing")
        return
    for alternatives in groups:
        if not any(term.lower() in text for term in alternatives):
            errors.append(f"{label}: tests do not cover {'/'.join(alternatives)}")


def run_tests(package: Path) -> str | None:
    manifest = package / "Cargo.toml"
    lock = package / "Cargo.lock"
    if not manifest.is_file() or not lock.is_file():
        return f"{package.relative_to(ROOT)}: manifest or lockfile is missing"
    try:
        patches = LOCAL_PATCHES.get(package.name, [])
        if not patches:
            result = subprocess.run(
                ["cargo", "test", "--manifest-path", str(manifest), "--locked", "--quiet"],
                cwd=ROOT,
                text=True,
                capture_output=True,
                timeout=50,
                check=False,
            )
            return test_failure(package, result)
        with tempfile.TemporaryDirectory(prefix=f"{package.name}-") as temporary:
            temporary_package = Path(temporary) / package.name
            shutil.copytree(package, temporary_package, ignore=shutil.ignore_patterns("target"))
            command = [
                "cargo",
                "test",
                "--manifest-path",
                str(temporary_package / "Cargo.toml"),
                "--offline",
                "--quiet",
            ]
            for relative in patches:
                patch = ROOT / relative
                command.extend(
                    [
                        "--config",
                        f"patch.crates-io.{patch.name}.path={json.dumps(str(patch))}",
                    ]
                )
            command.extend(["--", "--include-ignored"])
            result = subprocess.run(
                command,
                cwd=ROOT,
                text=True,
                capture_output=True,
                timeout=50,
                check=False,
            )
    except subprocess.TimeoutExpired:
        return f"{package.relative_to(ROOT)}: cargo test timed out"

    return test_failure(package, result)


def test_failure(package: Path, result: subprocess.CompletedProcess[str]) -> str | None:
    if result.returncode:
        tail = "\n".join((result.stdout + result.stderr).splitlines()[-6:])
        return f"{package.relative_to(ROOT)}: cargo test failed ({result.returncode}): {tail}"
    return None


def run_packages(errors: list[str], packages: list[Path]) -> None:
    with ThreadPoolExecutor(max_workers=len(packages)) as executor:
        for failure in executor.map(run_tests, packages):
            if failure:
                errors.append(failure)


def json_contract(errors: list[str]) -> None:
    json_core = ROOT / "packages/file-types/json/aqc-json-engine-core"
    package_json = ROOT / "packages/file-types/json/aqc-package-json-engine"
    combined = test_text(json_core) + "\n" + test_text(package_json)
    merge_source = (package_json / "src/runtime/merge.rs").read_text(errors="replace")
    if "resolve_maybe" not in merge_source or "resolve_optional" in merge_source:
        errors.append("Package JSON optional scalar merge does not delegate to core resolve_maybe")
    require_concepts(
        errors,
        "JSON reconciliation",
        combined,
        [
            ("duplicate",),
            ("root object", "non_object", "non-object"),
            ("wrong shape", "wrong_shape", "non_scalar"),
            ("provenance", "attribution"),
            ("missing", "none"),
            ("unchanged", "no_op", "no-op", "preserve"),
            ("package_manager", "packagemanager"),
            ("dev_engines", "devengines"),
            ("conflict", "merge"),
        ],
    )
    run_packages(errors, [json_core, package_json])


def yaml_contract(errors: list[str]) -> None:
    yaml_core = ROOT / "packages/file-types/yaml/aqc-yaml-engine-core"
    text = test_text(yaml_core)
    runtime = (yaml_core / "src/runtime/decode.rs").read_text(errors="replace")
    for marker in ("effective_mapping_value", "effective_mapping_entries", "decode_mapping_key", "build_anchor_registry"):
        if marker not in runtime:
            errors.append(f"YAML reconciliation omits unified effective mapping behavior: {marker}")
    require_concepts(
        errors,
        "YAML reconciliation",
        text,
        [
            ("duplicate",),
            ("root mapping", "non_mapping", "non-mapping"),
            ("alias",),
            ("merge", "mergedmapping", "merged_mapping"),
            ("precedence", "override"),
            ("direct", "underlying"),
            ("unknown tag", "unknown_tag"),
            ("!!bool", "boolean"),
            ("yes", "no", "on", "off"),
            ("unchanged", "no_op", "no-op", "preserve"),
            ("missing", "none"),
        ],
    )
    for test_name in (
        "mapping_valued_merge_sources_are_effective",
        "nested_mappings_resolve_merge_sources",
        "quoted_merge_key_never_injects_effective_fields",
        "tagged_and_aliased_string_mapping_keys_decode",
        "empty_collection_writes_round_trip",
        "cyclic_merge_alias_is_an_invalid_merge_source",
    ):
        if f"fn {test_name}" not in text:
            errors.append(f"YAML reconciliation omits exact test {test_name}")
    run_packages(errors, [yaml_core])


def existing_engine_behavior_contract(errors: list[str]) -> None:
    run_packages(
        errors,
        [
            ROOT / "packages/file-types/text/aqc-text-file-engine",
            ROOT / "packages/file-types/toml/aqc-cargo-toml-engine",
            ROOT / "packages/file-types/toml/aqc-clippy-toml-engine",
            ROOT / "packages/file-types/toml/aqc-deny-toml-engine",
            ROOT / "packages/file-types/toml/aqc-rust-toolchain-toml-engine",
            ROOT / "packages/file-types/toml/aqc-rustfmt-toml-engine",
        ],
    )


def pnpm_contract(errors: list[str]) -> None:
    pnpm = ROOT / "packages/file-types/yaml/aqc-pnpm-workspace-yaml-engine"
    text = test_text(pnpm)
    require_concepts(
        errors,
        "pnpm engine",
        text,
        [
            ("scalar",),
            ("conflict",),
            ("selector",),
            ("glob",),
            ("minimum_release_age", "minimumreleaseage"),
            ("trust_policy", "trustpolicy"),
            ("allow_builds", "allowbuilds"),
            ("exclude",),
            ("root_keys", "root keys"),
            ("invalid glob", "invalid_glob"),
            ("determin", "policy order"),
            ("expected_bytes", "initializ", "missing"),
        ],
    )
    for test_name in (
        "reversing_agreeing_policy_order_preserves_generated_bytes",
        "scalar_disagreement_preserves_key_reason_and_contributors",
        "exact_selector_conflict_preserves_key_reason_and_contributors",
        "exact_true_allow_build_conflict_preserves_key_reason_and_contributors",
    ):
        if f"fn {test_name}" not in text:
            errors.append(f"pnpm engine omits exact test {test_name}")
    run_packages(errors, [pnpm])


def toml_scalar_contract(errors: list[str]) -> None:
    toml_core = ROOT / "packages/file-types/toml/aqc-toml-engine-core"
    source = (toml_core / "src/scalars.rs").read_text(errors="replace")
    tests = test_text(toml_core)
    for required in ("scalar_assertion_matches", "scalar_assertion_writable_value"):
        if required not in source:
            errors.append(f"TOML scalar reconciliation does not use core {required}")
    for forbidden in ("ScalarAssertion::AtLeast(..) | ScalarAssertion::AtMost",):
        if forbidden in source:
            errors.append("TOML scalar reconciliation retains an ad hoc assertion evaluator")
    if "fn present_requires_a_typed_scalar_and_absent_removes_any_shape" not in tests:
        errors.append("TOML scalar reconciliation omits typed-present/shape-absent proof")
    run_packages(errors, [toml_core])


def mismatch_bodies(source: str) -> list[str]:
    bodies = []
    marker = "Finding::Mismatch"
    offset = 0
    while True:
        index = source.find(marker, offset)
        if index < 0:
            return bodies
        brace = source.find("{", index + len(marker))
        if brace < 0:
            return bodies
        depth = 1
        for end in range(brace + 1, len(source)):
            if source[end] == "{":
                depth += 1
            elif source[end] == "}":
                depth -= 1
                if depth == 0:
                    bodies.append(source[brace + 1:end])
                    offset = end + 1
                    break
        else:
            return bodies


def selector_contract(errors: list[str]) -> None:
    core = ROOT / "packages/aqc-file-engine-core"
    finding = core / "src/finding.rs"
    public_contract = core / "tests/public_contract.rs"
    if not finding.is_file() or "selector: Option<String>" not in finding.read_text():
        errors.append("Finding::Mismatch does not declare selector: Option<String>")
    contract_text = public_contract.read_text() if public_contract.is_file() else ""
    if "selector" not in contract_text or "Some(" not in contract_text:
        errors.append("aqc-file-engine-core public contract does not prove selector preservation")

    existing = [
        ROOT / "packages/file-types/text/aqc-text-file-engine/src",
        ROOT / "packages/file-types/toml/aqc-toml-engine-core/src",
        ROOT / "packages/file-types/toml/aqc-cargo-toml-engine/src",
        ROOT / "packages/file-types/toml/aqc-clippy-toml-engine/src",
        ROOT / "packages/file-types/toml/aqc-deny-toml-engine/src",
        ROOT / "packages/file-types/toml/aqc-rust-toolchain-toml-engine/src",
        ROOT / "packages/file-types/toml/aqc-rustfmt-toml-engine/src",
    ]
    for source_dir in existing:
        if not source_dir.is_dir():
            errors.append(f"{source_dir.relative_to(ROOT)} is missing")
            continue
        source = "\n".join(path.read_text() for path in sorted(source_dir.glob("**/*.rs")))
        bodies = mismatch_bodies(source)
        for body in bodies:
            normalized = re.sub(r"\s+", "", body)
            if "key:" in normalized and "selector:None" not in normalized:
                errors.append(f"{source_dir.relative_to(ROOT)} has a pre-existing mismatch without selector: None")
                break
    run_packages(errors, [core])


errors: list[str] = []
check = ENTRY["check"]
if check == "json-core-and-package-json-reconciliation":
    json_contract(errors)
elif check == "yaml-core-reconciliation":
    yaml_contract(errors)
elif check == "pnpm-engine-resolution-and-findings":
    pnpm_contract(errors)
elif check == "adversarial-reconciliation-contract":
    toml_scalar_contract(errors)
    yaml_contract(errors)
    pnpm_contract(errors)
    selector_contract(errors)
    existing_engine_behavior_contract(errors)
elif check == "finding-selector-contract":
    selector_contract(errors)
else:
    errors.append(f"unsupported semantic check {check}")
emit(errors)

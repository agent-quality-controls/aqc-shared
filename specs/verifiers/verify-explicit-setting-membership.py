#!/usr/bin/env python3
import json
import os
import subprocess
import sys
import tempfile
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path


SPEC = Path(sys.argv[1]).resolve()
ENTRY = json.loads(SPEC.read_text())["requirements"]["custom"][int(sys.argv[3])]
ROOT = SPEC.parent.parent

PACKAGES = {
    "core": ROOT / "packages/aqc-file-engine-core",
    "json_core": ROOT / "packages/file-types/json/aqc-json-engine-core",
    "json": ROOT / "packages/file-types/json/aqc-json-file-engine",
    "toml": ROOT / "packages/file-types/toml/aqc-toml-engine-core",
    "cargo": ROOT / "packages/file-types/toml/aqc-cargo-toml-engine",
    "deny": ROOT / "packages/file-types/toml/aqc-deny-toml-engine",
    "toolchain": ROOT / "packages/file-types/toml/aqc-rust-toolchain-toml-engine",
    "rustfmt": ROOT / "packages/file-types/toml/aqc-rustfmt-toml-engine",
    "yaml": ROOT / "packages/file-types/yaml/aqc-yaml-engine-core",
    "pnpm": ROOT / "packages/file-types/yaml/aqc-pnpm-workspace-yaml-engine",
    "architecture": ROOT / "tools/aqc-requirement-architecture",
}

LOCAL_PATCHES = {
    "json": ["core", "json_core"],
    "toml": ["core"],
    "cargo": ["core", "toml"],
    "deny": ["core", "toml"],
    "toolchain": ["core", "toml"],
    "rustfmt": ["core", "toml"],
    "yaml": ["core"],
    "pnpm": ["core", "yaml"],
}


def emit(errors: list[str], executed: list[str]) -> None:
    evidence = {
        "check": ENTRY["check"],
        "status": "fail" if errors else "pass",
        "executed": executed,
    }
    if errors:
        evidence["message"] = "; ".join(errors)
    print(json.dumps(evidence))


def files_text(base: Path, patterns: tuple[str, ...]) -> str:
    if not base.exists():
        return ""
    paths: set[Path] = set()
    for pattern in patterns:
        paths.update(path for path in base.glob(pattern) if path.is_file())
    return "\n".join(path.read_text(errors="replace") for path in sorted(paths))


def source_text(package: Path) -> str:
    return files_text(package, ("src/**/*.rs",))


def test_text(package: Path) -> str:
    return files_text(package, ("tests/**/*.rs", "src/**/*.rs"))


def require_terms(errors: list[str], label: str, text: str, groups: list[tuple[str, ...]]) -> None:
    lowered = text.lower()
    if not lowered:
        errors.append(f"{label}: source or tests are missing")
        return
    for alternatives in groups:
        if not any(term.lower() in lowered for term in alternatives):
            errors.append(f"{label}: missing evidence for {'/'.join(alternatives)}")


def require_declared_cases(errors: list[str], label: str, text: str) -> None:
    for test in ENTRY.get("requiredTests", []):
        if f"fn {test}(" not in text:
            errors.append(f"{label}: declared test {test} is not implemented")


def cargo_test(name: str) -> str | None:
    package = PACKAGES[name]
    manifest = package / "Cargo.toml"
    if not manifest.is_file():
        return f"{package.relative_to(ROOT)}: Cargo.toml is missing"
    try:
        with tempfile.TemporaryDirectory(prefix=f"explicit-membership-{name}-") as temporary:
            command = [
                "cargo",
                "test",
                "--manifest-path",
                str(manifest),
                "--locked",
                "--quiet",
            ]
            for dependency in LOCAL_PATCHES.get(name, []):
                dependency_path = PACKAGES[dependency]
                command.extend(
                    [
                        "--config",
                        f"patch.crates-io.{dependency_path.name}.path={json.dumps(str(dependency_path))}",
                    ]
                )
            result = subprocess.run(
                command,
                cwd=ROOT,
                env={**os.environ, "CARGO_TARGET_DIR": str(Path(temporary) / "target")},
                text=True,
                capture_output=True,
                timeout=45,
                check=False,
            )
    except subprocess.TimeoutExpired:
        return f"{package.relative_to(ROOT)}: cargo test timed out"
    if result.returncode:
        tail = "\n".join((result.stdout + result.stderr).splitlines()[-8:])
        return f"{package.relative_to(ROOT)}: cargo test failed ({result.returncode}): {tail}"
    return None


def run_tests(errors: list[str], executed: list[str], names: list[str]) -> None:
    with ThreadPoolExecutor(max_workers=len(names)) as executor:
        results = list(executor.map(cargo_test, names))
    for name, failure in zip(names, results):
        executed.append(f"cargo test {PACKAGES[name].relative_to(ROOT)}")
        if failure:
            errors.append(failure)


def core_semantics(errors: list[str], executed: list[str]) -> None:
    source = source_text(PACKAGES["core"])
    tests = test_text(PACKAGES["core"])
    require_terms(
        errors,
        "core source",
        source,
        [
            ("pub struct ItemPresenceDifference",),
            ("pub fn item_presence_difference",),
            ("pub fn map",),
            ("RequiredItemResolution",),
            ("ForbiddenItemResolution",),
            ("FileKeyRequirement",),
            ("resolve_key_membership",),
        ],
    )
    require_declared_cases(errors, "core", source + tests)
    require_terms(
        errors,
        "core tests",
        tests,
        [
            ("missing",),
            ("forbidden",),
            ("unexpected", "extra"),
            ("exact_empty", "exact empty"),
            ("duplicate",),
            ("compatible", "agreeing"),
            ("incompatible", "conflict"),
            ("required_outside", "required outside"),
            ("forbidden_inside", "forbidden inside"),
            ("provenance", "attribution", "contributors"),
            ("map_preserves", "map preserves", "mapped"),
            ("derived_key_constraints", "derived key constraints"),
            ("derived_key_constraints_are_checked_against_explicit_membership",),
            ("one_rejection_per_identity", "one rejection per identity"),
        ],
    )
    if not errors:
        run_tests(errors, executed, ["core"])


def format_reconciliation(errors: list[str], executed: list[str]) -> None:
    sources = {
        "JSON": source_text(PACKAGES["json"]),
        "Cargo": source_text(PACKAGES["cargo"]),
        "TOML": source_text(PACKAGES["toml"]),
        "YAML": source_text(PACKAGES["yaml"]),
    }
    for label, source in sources.items():
        require_terms(errors, f"{label} source", source, [("item_presence_difference",)])
    require_terms(
        errors,
        "format tests",
        "\n".join(test_text(PACKAGES[name]) for name in ("json", "cargo", "toml", "yaml")),
        [
            ("object", "json"),
            ("lint", "cargo"),
            ("table_key", "table key"),
            ("effective", "merge key", "merge_key"),
            ("inherited", "anchor"),
            ("unwritable", "cannot construct", "not construct"),
            ("parent_removal_precedes_child_reconciliation",),
        ],
    )
    require_declared_cases(errors, "format reconciliation", "\n".join(sources.values()) + "\n" + "\n".join(test_text(PACKAGES[name]) for name in ("json", "cargo", "toml", "yaml")))
    if not errors:
        run_tests(errors, executed, ["json", "cargo", "toml", "yaml"])


def migrated_engines(errors: list[str], executed: list[str]) -> None:
    surfaces = {
        "rustfmt": "setting_keys",
        "toolchain": "toolchain_keys",
        "pnpm": "root_keys",
        "deny": "table_keys",
    }
    for name, field in surfaces.items():
        source = source_text(PACKAGES[name])
        require_terms(
            errors,
            f"{name} source",
            source,
            [(field,), ("ItemRequirements",), ("ResolvedItemRequirements",)],
        )
        if "exact_settings" in source or "closed_settings" in source:
            errors.append(f"{name} source retains a semantic closure marker")
        require_terms(
            errors,
            f"{name} tests",
            test_text(PACKAGES[name]),
            [
                ("missing",),
                ("forbidden",),
                ("extra", "unexpected", "unknown"),
                ("absent",),
                ("conflict",),
                ("init", "initialize"),
                ("second", "idempotent"),
                ("provenance", "attribution", "contributors"),
                ("cannot_exclude_a_constructive", "value and membership requirements must conflict"),
            ],
        )
    require_terms(
        errors,
        "Rustfmt regressions",
        test_text(PACKAGES["rustfmt"]),
        [("nightly",), ("ignore",), ("absent",)],
    )
    require_declared_cases(
        errors,
        "migrated engines",
        "\n".join(
            source_text(PACKAGES[name]) + test_text(PACKAGES[name])
            for name in ("deny", "rustfmt", "toolchain", "pnpm")
        ),
    )
    require_terms(
        errors,
        "toolchain regressions",
        test_text(PACKAGES["toolchain"]),
        [
            ("path",),
            ("targets",),
            ("empty", "no_targets", "no targets"),
            ("exact_empty_toolchain_membership_fails_merge",),
        ],
    )
    require_terms(
        errors,
        "pnpm regressions",
        test_text(PACKAGES["pnpm"]),
        [
            ("merge", "effective"),
            ("direct",),
            ("inherited", "anchor"),
            ("glob",),
            ("absence", "absent"),
            ("rejected_root_key_has_one_membership_finding",),
            ("inherited_rejected_root_key_has_one_membership_finding",),
            ("invalid_merge_source_stops_child_reconciliation",),
            ("rejected_anchor_owner_is_not_removed_or_followed_by_a_child_finding",),
        ],
    )
    require_terms(
        errors,
        "Deny table coverage",
        test_text(PACKAGES["deny"]),
        [
            ("graph",),
            ("output",),
            ("advisories",),
            ("licenses",),
            ("private",),
            ("workspace_dependencies", "workspace-dependencies"),
            ("build",),
            ("sources",),
            ("allow_org", "allow-org"),
        ],
    )
    if not errors:
        run_tests(errors, executed, ["deny", "rustfmt", "toolchain", "pnpm"])


def architecture_checker(errors: list[str], executed: list[str]) -> None:
    package = PACKAGES["architecture"]
    source = source_text(package)
    fixtures_and_tests = files_text(package, ("tests/**/*", "fixtures/**/*", "src/**/*.rs"))
    require_terms(
        errors,
        "architecture checker source",
        source,
        [
            ("EngineRequirement",),
            ("AdapterRequirement",),
            ("ItemRequirements",),
            ("AdapterMembershipConstruction",),
            ("NonCanonicalRequirementRoot",),
            ("resolve_trait_alias",),
            ("visit_macro",),
            ("visit_expr_method_call",),
            ("visit_expr_reference",),
            ("visit_pat_struct",),
            ("cargo_metadata",),
            ("syn",),
            ("inventory", "RequirementRoot", "roots"),
        ],
    )
    require_declared_cases(errors, "architecture checker", source + fixtures_and_tests)
    require_terms(
        errors,
        "architecture adversarial tests",
        fixtures_and_tests,
        [
            ("exact_settings",),
            ("closed_settings",),
            ("alias",),
            ("wrapped",),
            ("tuplerequirementroot", "aliasedrequirementroot"),
            ("hiddenadapter", "requirementcontract"),
            ("inferred_required",),
            ("inferred_membership",),
            ("inferred_by_extend",),
            ("inferred_by_mutable_reference",),
            ("inferred_by_destructuring",),
            ("default_membership_replacement",),
            ("noncanonical_membership_field", "NoncanonicalMembershipField"),
            ("local_macro_alias",),
            ("renamed_local_membership_mutation",),
            ("hidden_default_construction",),
            ("rewrite_membership_parameter",),
            ("PrivateClosureField",),
            ("ReimplementedCoreVocabulary",),
            ("UninspectableRequirementMacro",),
            ("imported_membership_alias",),
            ("unrelated_required_field_mutation",),
            ("unrelated_nested_required_field_mutation",),
            ("unrelated_external_macro",),
            ("The adversarial case", "must produce its own violation"),
            ("policy",),
            (".map", "map("),
            ("inventory",),
            ("../shackles", "repository root", "repository_root"),
        ],
    )
    if not errors:
        run_tests(errors, executed, ["architecture"])
    if errors:
        return
    repository_roots = [(ROOT / relative).resolve() for relative in ENTRY["repositoryRoots"]]
    try:
        result = subprocess.run(
            [
                "cargo",
                "run",
                "--quiet",
                "--locked",
                "--manifest-path",
                str(package / "Cargo.toml"),
                "--",
                *(str(root) for root in repository_roots),
            ],
            cwd=ROOT,
            text=True,
            capture_output=True,
            timeout=120,
            check=False,
        )
    except subprocess.TimeoutExpired:
        errors.append("architecture checker repository scan timed out")
        return
    executed.append(
        "aqc-requirement-architecture "
        + " ".join(str(root.relative_to(ROOT.parent)) for root in repository_roots)
    )
    if result.returncode:
        tail = "\n".join((result.stdout + result.stderr).splitlines()[-12:])
        errors.append(f"architecture checker repository scan failed ({result.returncode}): {tail}")


CHECKS = {
    "core-item-presence-semantics": core_semantics,
    "format-membership-reconciliation": format_reconciliation,
    "migrated-engine-behavior": migrated_engines,
    "architecture-checker-semantics": architecture_checker,
}


errors: list[str] = []
executed: list[str] = []
handler = CHECKS.get(ENTRY.get("check"))
if handler is None:
    errors.append(f"unsupported check: {ENTRY.get('check')}")
else:
    handler(errors, executed)
emit(errors, executed)

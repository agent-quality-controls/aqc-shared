#!/usr/bin/env python3
import json
import hashlib
import os
import re
import subprocess
import sys
import tempfile
import tomllib
from pathlib import Path


SPEC = Path(sys.argv[1]).resolve()
CATEGORY = sys.argv[2]
INDEX = int(sys.argv[3])
DATA = json.loads(SPEC.read_text())
ENTRY = DATA["requirements"][CATEGORY][INDEX]
ROOT = SPEC.parent.parent
SHACKLES_ROOT = ROOT.parent / "shackles"
PACKAGES = {
    "aqc-json-engine-core": ROOT / "packages/file-types/json/aqc-json-engine-core",
    "aqc-package-json-engine": ROOT / "packages/file-types/json/aqc-package-json-engine",
    "aqc-tsconfig-json-engine": ROOT / "packages/file-types/jsonc/aqc-tsconfig-json-engine",
}


def facade_names(package: Path) -> set[str]:
    path = package / "src/lib.rs"
    text = path.read_text() if path.is_file() else ""
    names = set(re.findall(r"\bpub\s+(?:const|static|struct|enum|trait|type|fn)\s+([A-Za-z_][A-Za-z0-9_]*)", text))
    for group in re.findall(r"pub\s+use\s+[^;]*?\{([^}]*)\}", text, re.S):
        for item in group.split(","):
            name = item.strip().split(" as ")[-1].split("::")[-1].strip()
            if name and name != "self":
                names.add(name)
    names.update(re.findall(r"pub\s+use\s+[^;{}]+::([A-Za-z_][A-Za-z0-9_]*)\s*;", text))
    names.update(re.findall(r"\bpub\s+mod\s+([A-Za-z_][A-Za-z0-9_]*)\s*;", text))
    return names


def source_tree(package: Path) -> str:
    return "\n".join(path.read_text() for path in sorted((package / "src").glob("**/*.rs")))


def emit_exports() -> None:
    names = facade_names(PACKAGES[ENTRY["package"]])
    for polarity in ("required", "exists", "forbidden"):
        for item in ENTRY.get(polarity, []):
            present = item in names
            passed = present if polarity != "forbidden" else not present
            evidence = {"item": item, "status": "pass" if passed else "fail"}
            if not passed:
                evidence["message"] = f"public name {item!r} has wrong presence for {polarity}"
            print(json.dumps(evidence))


def exact_api_errors() -> list[str]:
    core = PACKAGES["aqc-json-engine-core"]
    expected = {
        "ConfigScalar", "Finding", "JsonObject", "JsonParseOptions", "NonObjectParentAction", "Provenance",
        "ResolvedRequirement", "ScalarAssertion", "ScalarValue",
        "parse_object_or_report", "reconcile_scalar_assertion",
    }
    errors = []
    found = facade_names(core)
    if found != expected:
        errors.append(f"core facade expected {sorted(expected)}, found {sorted(found)}")
    concrete_expected = {
        "aqc-package-json-engine": {
            "ConflictEntry", "DevEnginePackageManagerRequirements", "ENGINE_ID", "PackageJsonEngine",
            "PackageJsonRequirements", "PackageManagerOnFail", "Provenance", "ResolvedDevEnginePackageManagerRequirements",
            "ResolvedMap", "ResolvedPackageJsonRequirements", "ResolvedRequirement", "ScalarAssertion",
        },
        "aqc-tsconfig-json-engine": {
            "ConflictEntry", "ENGINE_ID", "Provenance", "ResolvedMap", "ResolvedRequirement",
            "ResolvedTsconfigJsonRequirements", "ScalarAssertion", "TsconfigBooleanCompilerOption",
            "TsconfigJsonEngine", "TsconfigJsonRequirements",
        },
    }
    for package, expected_names in concrete_expected.items():
        concrete_found = facade_names(PACKAGES[package])
        if concrete_found != expected_names:
            errors.append(
                f"{package} facade expected {sorted(expected_names)}, found {sorted(concrete_found)}"
            )
    text = source_tree(core)
    required = [
        "pub fn scalar(&self", "pub fn value_exists(&self", "pub fn object_exists(&self",
        "pub fn set_scalar(", "pub fn remove_value(&mut self",
        "parent_action: NonObjectParentAction",
        "pub fn rendered_value(&self", "pub fn render(&self", "selector: Option<String>",
        "pub allow_comments: bool", "pub allow_loose_object_property_names: bool",
        "pub allow_trailing_commas: bool", "pub allow_missing_commas: bool",
        "pub allow_single_quoted_strings: bool", "pub allow_hexadecimal_numbers: bool",
        "pub allow_unary_plus_numbers: bool", "pub allow_extended_json_numbers: bool",
        "pub allow_extended_string_escapes: bool",
        "pub allow_extended_whitespace: bool",
        "pub allow_utf8_bom: bool",
    ]
    errors.extend(f"missing API fragment {item}" for item in required if item not in text)
    public_signatures = re.findall(
        r"\bpub(?:\([^)]*\))?\s+(?:const|static|struct|enum|trait|type|fn)\b[^;{]*(?:;|\{)",
        text,
        re.S,
    )
    public_fields = re.findall(r"\bpub(?:\([^)]*\))?\s+[A-Za-z_][A-Za-z0-9_]*\s*:[^,\n}]+", text)
    for declaration in [*public_signatures, *public_fields]:
        if any(parser in declaration for parser in ("jsonc_parser", "serde_json", "tree_sitter")):
            errors.append(f"public API leaks parser implementation type: {declaration.strip()}")
    forbidden_purity = [
        re.compile(r"\bstd\s*::\s*(?:fs|path|process|env|net)\b"),
        re.compile(r"\buse\s+std\s*::\s*\{[^}]*\b(?:fs|path|process|env|net)\b", re.S),
        re.compile(r"\b(?:tokio|async_std)\s*::\s*(?:fs|net|process)\b"),
        re.compile(r"\b(?:reqwest|ureq|hyper)\s*::"),
        re.compile(r"\bCommand\s*::\s*new\b"),
    ]
    for package in PACKAGES.values():
        package_source = source_tree(package)
        for forbidden in forbidden_purity:
            if forbidden.search(package_source):
                errors.append(
                    f"{package.name} violates pure byte-transform boundary with {forbidden.pattern}"
                )
    exact_dependencies = {
        "aqc-json-engine-core": {
            "aqc-file-engine-core", "jsonc-parser", "serde_json", "tree-sitter", "tree-sitter-javascript",
        },
        "aqc-package-json-engine": {
            "aqc-file-engine-core",
            "aqc-json-engine-core",
            "schemars",
            "serde",
        },
        "aqc-tsconfig-json-engine": {
            "aqc-file-engine-core",
            "aqc-json-engine-core",
            "schemars",
            "serde",
        },
    }
    for package, expected_dependencies in exact_dependencies.items():
        with (PACKAGES[package] / "Cargo.toml").open("rb") as handle:
            manifest = tomllib.load(handle)
        if manifest.get("package", {}).get("publish") is not True:
            errors.append(f"{package} must set package.publish = true")
        for table_name in ("dependencies", "dev-dependencies", "build-dependencies"):
            for dependency, declaration in manifest.get(table_name, {}).items():
                if isinstance(declaration, dict) and "path" in declaration:
                    errors.append(f"{package} {table_name}.{dependency} must not use path")
        observed_dependencies = set(manifest.get("dependencies", {}))
        if observed_dependencies != expected_dependencies:
            errors.append(
                f"{package} dependencies expected {sorted(expected_dependencies)}, found {sorted(observed_dependencies)}"
            )
    return errors


def inventory_errors() -> list[str]:
    retired = ["aqc-jsonc-engine-core", "aqc_jsonc_engine_core"]
    excluded_files = {
        ".plans/2026-07-14-174701-unify-json-engine-core.md",
        "specs/json-engine-core-consolidation.spec.json",
        "specs/json-engine-core-consolidation.spec.coverage.md",
        "specs/verifiers/verify-json-engine-core-consolidation.py",
    }
    errors = []
    for repository in (ROOT, SHACKLES_ROOT):
        result = subprocess.run(
            ["git", "ls-files", "--cached", "--others", "--exclude-standard"],
            cwd=repository,
            check=True,
            capture_output=True,
            text=True,
        )
        for relative in result.stdout.splitlines():
            if relative.startswith(".worklogs/") or relative in excluded_files:
                continue
            path = repository / relative
            if not path.is_file():
                continue
            try:
                text = path.read_text()
            except UnicodeDecodeError:
                continue
            for item in retired:
                if item in text:
                    errors.append(
                        f"{repository.name}/{relative} contains retired vocabulary {item}"
                    )
            if re.search(r"(?i)\bjsonc(?:object|parseoptions)\b", text):
                errors.append(
                    f"{repository.name}/{relative} contains retired JSONC API vocabulary"
                )
            if re.search(r"\brender_object\b", text):
                errors.append(
                    f"{repository.name}/{relative} contains retired API render_object"
                )
            if path.name == "Cargo.lock" and "[[patch.unused]]" in text:
                errors.append(f"{repository.name}/{relative} contains unused Cargo patches")
    return errors


def release_inventory_errors() -> list[str]:
    with (ROOT / "release-plz.toml").open("rb") as handle:
        release = tomllib.load(handle)
    names = [entry.get("name") for entry in release.get("package", [])]
    errors = []
    for name in (
        "aqc-json-engine-core",
        "aqc-package-json-engine",
        "aqc-tsconfig-json-engine",
    ):
        if names.count(name) != 1:
            errors.append(f"release-plz.toml must list {name} exactly once")
    if "aqc-jsonc-engine-core" in names:
        errors.append("release-plz.toml retains aqc-jsonc-engine-core")

    workflow = (ROOT / ".github/workflows/release.yml").read_text()
    expected_matrices = {
        "release-format-cores": {"packages/file-types/json/aqc-json-engine-core/Cargo.toml"},
        "release-file-engines": {
            "packages/file-types/json/aqc-package-json-engine/Cargo.toml",
            "packages/file-types/jsonc/aqc-tsconfig-json-engine/Cargo.toml",
        },
    }
    job_starts = list(re.finditer(r"(?m)^  ([A-Za-z0-9_-]+):\s*$", workflow))
    job_blocks = {}
    for index, match in enumerate(job_starts):
        end = job_starts[index + 1].start() if index + 1 < len(job_starts) else len(workflow)
        job_blocks[match.group(1)] = workflow[match.end():end]
    for job, expected in expected_matrices.items():
        block = job_blocks.get(job, "")
        manifest_match = re.search(
            r"(?ms)^\s{8}manifest:\s*$\n(?P<items>(?:\s{10}-[^\n]+\n)+)", block
        )
        observed = set()
        if manifest_match:
            observed = {
                line.strip().removeprefix("-").strip()
                for line in manifest_match.group("items").splitlines()
            }
        if not expected.issubset(observed):
            errors.append(f"release job {job} is missing {sorted(expected - observed)}")
    wait_block = job_blocks.get("release-file-engines", "")
    if "packages/file-types/json/aqc-json-engine-core/Cargo.toml" not in wait_block:
        errors.append("release-file-engines does not wait for aqc-json-engine-core")
    if "aqc-jsonc-engine-core" in workflow:
        errors.append("release workflow retains aqc-jsonc-engine-core")
    return errors


def downstream_errors() -> list[str]:
    manifest = SHACKLES_ROOT / "fixtures/downstream-adapter-consumer/Cargo.toml"
    required_msrv_manifests = [
        "apps/shakts/Cargo.toml",
        "packages/rs/adapters/shakts-pnpm-adapter/Cargo.toml",
        "packages/rs/adapters/shakts-tsc-adapter/Cargo.toml",
        "packages/rs/policies/shakts-pnpm-policy/Cargo.toml",
        "packages/rs/policies/shakts-tsc-policy/Cargo.toml",
        "fixtures/downstream-adapter-consumer/Cargo.toml",
    ]
    errors = []
    for relative in required_msrv_manifests:
        with (SHACKLES_ROOT / relative).open("rb") as handle:
            rust_version = tomllib.load(handle).get("package", {}).get("rust-version")
        if rust_version != "1.88":
            errors.append(f"{relative} must declare rust-version 1.88, found {rust_version!r}")
    with tempfile.TemporaryDirectory(prefix="json-core-consolidation-") as directory:
        config = Path(directory) / "config.toml"
        prepare = subprocess.run(
            [
                "python3",
                str(SHACKLES_ROOT / "scripts/local_cargo_source.py"),
                "--root",
                str(SHACKLES_ROOT),
                "--config",
                str(config),
                "--manifest",
                str(manifest),
            ],
            cwd=SHACKLES_ROOT,
            check=False,
            capture_output=True,
            text=True,
        )
        if prepare.returncode != 0:
            return [*errors, f"cannot prepare downstream dependency source: {prepare.stdout}{prepare.stderr}"]
        result = subprocess.run(
            [
                "cargo",
                "+1.88.0",
                "check",
                "--locked",
                "--manifest-path",
                str(manifest),
                "--config",
                str(config),
            ],
            cwd=SHACKLES_ROOT,
            check=False,
            capture_output=True,
            text=True,
        )
    if result.returncode != 0:
        errors.append(f"downstream public-surface compile failed: {result.stdout}{result.stderr}")
    return errors


def coverage_errors() -> list[str]:
    plan = ROOT / ".plans/2026-07-14-174701-unify-json-engine-core.md"
    coverage = ROOT / "specs/json-engine-core-consolidation.spec.coverage.md"
    plan_text = plan.read_text()
    headings = re.findall(r"^#{1,6}\s+(.+)$", plan_text, re.MULTILINE)
    expected_lines = [
        "- Goal: tree, dependencies, exports, and `active-inventory-clean`.",
        "- Approach: the four approach subsections below map each implementation area.",
        "- Unified JSON core: core content, dependencies, exports, `exact-unified-api`, and `runtime-contracts`.",
        "- Concrete engines: concrete content and dependency blocks, exports, and `runtime-contracts`.",
        "- Remove the duplicate core: forbidden tree and `active-inventory-clean`.",
        "- Verification: `runtime-contracts` runs the exact TypeScript 7.0.2 syntax probe and Rust contracts; split `required-gates-*` checks run format, Clippy, cargo-deny, package, and boundary gates; AQC and Shackles fixtures and migrated specs run as independent mechanical gates because their integration suites exceed Specular's verifier timeout.",
        "- Key Decisions: core API checks, parser dependency boundaries, syntax isolation tests, and malformed-parent contract tests.",
        "- Files To Modify: required and forbidden tree entries cover planned AQC artifacts; `active-inventory-clean` scans active tracked and untracked files in both repositories.",
        "- AQC: built-ins check manifests, dependencies, source content, docs, and trees; parsed manifests enforce publication and dependency sources; migrated specs and release checks enforce downstream inventory.",
        "- Shackles: migrated PNPM/TSC specs check manifests, locks, deny files, docs, fixtures, and downstream artifacts; boundary scripts enforce layering.",
        "- Required End State: every custom check in the consolidation spec jointly enforces the stated end state.",
    ]
    observed_lines = [line for line in coverage.read_text().splitlines() if line.startswith("- ")]
    entries = [line[2:].split(":", 1)[0] for line in observed_lines]
    expected_hash = hashlib.sha256(plan_text.encode()).hexdigest()
    hash_lines = [line for line in coverage.read_text().splitlines() if line.startswith("Plan SHA256: ")]
    if entries != headings or observed_lines != expected_lines or hash_lines != [f"Plan SHA256: {expected_hash}"]:
        return ["coverage map does not exactly match the reviewed plan-to-verifier mapping"]
    return []


def runtime_errors() -> list[str]:
    file_core = ROOT / "packages/aqc-file-engine-core"
    patch_args = [
        "--config", f"patch.crates-io.aqc-file-engine-core.path='{file_core}'",
        "--config", f"patch.crates-io.aqc-json-engine-core.path='{PACKAGES['aqc-json-engine-core']}'",
    ]
    commands = [
        ["cargo", "+1.88.0", "test", "--locked", *patch_args, "--manifest-path", str(PACKAGES["aqc-json-engine-core"] / "Cargo.toml")],
        ["cargo", "+1.88.0", "test", "--locked", *patch_args, "--manifest-path", str(PACKAGES["aqc-package-json-engine"] / "Cargo.toml")],
        ["cargo", "+1.88.0", "test", "--locked", *patch_args, "--manifest-path", str(PACKAGES["aqc-tsconfig-json-engine"] / "Cargo.toml")],
    ]
    errors = []
    required_tests = {
        PACKAGES["aqc-json-engine-core"] / "tests/core_contract.rs": [
            "strict_json_rejects_non_json_characters_and_preserves_parser_messages",
            "jsonc_rejects_unescaped_control_characters_in_strings",
            "extended_numbers_reject_invalid_numeric_separator_positions",
            "duplicate_diagnostics_count_cr_and_crlf_line_endings",
            "masked_strings_do_not_shift_parse_or_duplicate_diagnostics",
            "intermediate_parent_replacement_discards_parent_number_metadata",
            "extended_string_escapes_decode_and_preserve_exact_bytes",
            "extended_strings_do_not_collide_with_number_markers_or_comment_text",
            "typescript_javascript_escape_set_decodes_and_preserves_exact_bytes",
            "typescript_rejects_octal_and_decimal_digit_string_escapes",
            "equivalent_unicode_escape_keys_are_duplicates",
            "replacing_an_unrepresentable_string_discards_its_mask_metadata",
            "extended_string_normalization_errors_report_source_locations",
            "extended_whitespace_is_independent_and_preserved",
            "extended_whitespace_filter_respects_strings_and_comments",
            "unrelated_syntax_options_do_not_enable_invalid_whitespace",
            "extended_string_errors_are_classified_as_jsonc",
        ],
        PACKAGES["aqc-package-json-engine"] / "tests/contract.rs": [
            "strict_package_json_rejects_every_json_extension",
            "keyed_map_wrong_parent_shape_is_reported_without_replacement",
            "writable_nested_requirements_replace_a_non_object_dev_engines_parent",
        ],
        PACKAGES["aqc-tsconfig-json-engine"] / "tests/contract.rs": [
            "typescript_syntax_extensions_survive_reconciliation",
            "typescript_string_escapes_survive_reconciliation",
            "wrong_compiler_options_shape_is_reported_once_and_preserved",
        ],
    }
    for path, names in required_tests.items():
        test_source = path.read_text()
        errors.extend(
            f"{path.relative_to(ROOT)} is missing required test {name}"
            for name in names
            if not re.search(rf"#\[test\]\s*fn\s+{re.escape(name)}\s*\(", test_source)
        )
    for command in commands:
        result = subprocess.run(command, cwd=ROOT, check=False, capture_output=True, text=True)
        if result.returncode != 0:
            errors.append(f"{' '.join(command)} failed: {result.stdout}{result.stderr}")
    errors.extend(typescript_7_syntax_errors())
    return errors


def typescript_7_syntax_errors() -> list[str]:
    with tempfile.TemporaryDirectory(prefix="json-core-typescript-7-") as directory:
        root = Path(directory)
        (root / "index.ts").write_text("export {};\n")
        accepted = (
            "\ufeff{\n"
            "  // retained JSONC syntax\n"
            "  \"compilerOptions\": {\"strict\": true,},\n"
            "  \"numbers\": [0x10, 0b10, 0o10, .5, 1., 1_000, -0b11],\n"
            "  \"strings\": [\"\\x41\", \"\\v\", \"\\0\", "
            "\"\\q\", \"\\u{1f600}\", \"\\u{d800}\", \"\\uD800\"],\n"
            "  \"files\": [\"index.ts\"],\n"
            "}\n"
        )
        rejected = {
            "single-quotes": "{'compilerOptions': {'strict': true}, 'files': ['index.ts']}\n",
            "octal-escape": "{\"probe\":\"\\01\",\"files\":[\"index.ts\"]}\n",
            "decimal-digit-escape": "{\"probe\":\"\\8\",\"files\":[\"index.ts\"]}\n",
        }
        command = [
            "npm", "exec", "--yes", "--package=typescript@7.0.2", "--",
            "tsc", "--project", "tsconfig.json", "--pretty", "false", "--noEmit",
        ]
        errors = []
        cases = [("accepted", accepted, True)]
        cases.extend((name, text, False) for name, text in rejected.items())
        for name, text, expected_success in cases:
            (root / "tsconfig.json").write_text(text)
            result = subprocess.run(
                command, cwd=root, check=False, capture_output=True, text=True
            )
            if (result.returncode == 0) != expected_success:
                errors.append(
                    f"TypeScript 7.0.2 {name} syntax probe returned {result.returncode}: "
                    f"{result.stdout}{result.stderr}"
                )
        return errors


def required_gate_errors(check: str) -> list[str]:
    errors = []
    file_core = ROOT / "packages/aqc-file-engine-core"
    package_names = {
        "json-core": "aqc-json-engine-core",
        "package-json": "aqc-package-json-engine",
        "tsconfig": "aqc-tsconfig-json-engine",
    }
    if check == "required-gates-boundaries":
        for script in ("scripts/check-dependency-boundaries.py", "scripts/check-pure-layers.py"):
            result = subprocess.run(
                ["python3", script], cwd=SHACKLES_ROOT, check=False, capture_output=True, text=True
            )
            if result.returncode != 0:
                errors.append(f"{script} failed: {result.stdout}{result.stderr}")
        return errors

    suffix = check.removeprefix("required-gates-")
    package_key = next(
        (name for name in package_names if suffix.startswith(f"{name}-")), None
    )
    if package_key is None:
        return [f"unknown required gate {check}"]
    gate = suffix.removeprefix(f"{package_key}-")
    package = PACKAGES[package_names[package_key]]
    with tempfile.TemporaryDirectory(prefix="json-core-cargo-home-") as directory:
        cargo_home = Path(directory)
        source_home = Path.home() / ".cargo"
        for child in ("registry", "git", "advisory-dbs"):
            source = source_home / child
            if source.exists():
                os.symlink(source, cargo_home / child, target_is_directory=True)
        (cargo_home / "config.toml").write_text(
            "[patch.crates-io]\n"
            f"aqc-file-engine-core = {{ path = {str(file_core)!r} }}\n"
            f"aqc-json-engine-core = {{ path = {str(PACKAGES['aqc-json-engine-core'])!r} }}\n"
        )
        environment = {**os.environ, "CARGO_HOME": str(cargo_home)}
        manifest = package / "Cargo.toml"
        commands = {
            "format": ["cargo", "+1.88.0", "fmt", "--all", "--check", "--manifest-path", str(manifest)],
            "clippy": ["cargo", "+1.88.0", "clippy", "--locked", "--all-targets", "--all-features", "--manifest-path", str(manifest), "--", "-D", "warnings"],
            "package": ["cargo", "+1.88.0", "package", "--allow-dirty", "--manifest-path", str(manifest)],
            "deny": ["cargo", "deny", "--offline", "--manifest-path", str(manifest), "--locked", "--all-features", "check"],
        }
        command = commands.get(gate)
        if command is None:
            return [f"unknown required gate {check}"]
        result = subprocess.run(
            command,
            cwd=ROOT,
            env=environment,
            check=False,
            capture_output=True,
            text=True,
        )
        if result.returncode != 0:
            errors.append(f"{' '.join(command)} failed: {result.stdout}{result.stderr}")

    return errors


def fixture_errors(repository: Path, suite: str) -> list[str]:
    result = subprocess.run(
        ["fixture3", "check", "--suite", suite, "--json"],
        cwd=repository,
        check=False,
        capture_output=True,
        text=True,
    )
    try:
        report = json.loads(result.stdout)
        statuses = [entry.get("status") for entry in report.get("suites", [])]
    except json.JSONDecodeError:
        statuses = []
    if result.returncode != 0 or statuses != ["matched"]:
        return [f"fixture3 {suite} failed: {result.stdout}{result.stderr}"]
    return []


if CATEGORY == "exports":
    emit_exports()
else:
    check = ENTRY["check"]
    if check == "exact-unified-api":
        errors = exact_api_errors()
    elif check == "active-inventory-clean":
        errors = inventory_errors()
    elif check == "runtime-contracts":
        errors = runtime_errors()
    elif check == "downstream-contract":
        errors = downstream_errors()
    elif check == "plan-coverage":
        errors = coverage_errors()
    elif check == "aqc-pnpm-fixtures":
        errors = fixture_errors(ROOT, "shakts-pnpm-aqc")
    elif check == "aqc-tsc-fixtures":
        errors = fixture_errors(ROOT, "shakts-tsc-aqc")
    elif check == "release-inventory":
        errors = release_inventory_errors()
    elif check.startswith("required-gates-"):
        errors = required_gate_errors(check)
    else:
        errors = [f"unknown check {check}"]
    evidence = {"check": check, "status": "fail" if errors else "pass"}
    if errors:
        evidence["message"] = "; ".join(errors)
    print(json.dumps(evidence))

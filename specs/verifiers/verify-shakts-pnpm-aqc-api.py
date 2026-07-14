#!/usr/bin/env python3
import json
import re
import sys
from pathlib import Path


SPEC = Path(sys.argv[1]).resolve()
ENTRY = json.loads(SPEC.read_text())["requirements"]["custom"][int(sys.argv[3])]
ROOT = SPEC.parent.parent

PACKAGE_JSON = ROOT / "packages/file-types/json/aqc-package-json-engine"
PNPM_YAML = ROOT / "packages/file-types/yaml/aqc-pnpm-workspace-yaml-engine"


def emit(errors: list[str]) -> None:
    evidence = {"check": ENTRY["check"], "status": "fail" if errors else "pass"}
    if errors:
        evidence["message"] = "; ".join(errors)
    print(json.dumps(evidence))


def sources(package: Path) -> str:
    source_dir = package / "src"
    if not source_dir.is_dir():
        return ""
    return "\n".join(path.read_text() for path in sorted(source_dir.glob("**/*.rs")))


def balanced_body(source: str, declaration: str) -> str | None:
    match = re.search(declaration + r"[^\{]*\{", source, re.S)
    if not match:
        return None
    depth = 1
    for index in range(match.end(), len(source)):
        if source[index] == "{":
            depth += 1
        elif source[index] == "}":
            depth -= 1
            if depth == 0:
                return source[match.end():index]
    return None


def balanced_bodies(source: str, declaration: str) -> list[str]:
    result = []
    for match in re.finditer(declaration + r"[^\{]*\{", source, re.S):
        start = match.start()
        body = balanced_body(source[start:], declaration)
        if body is not None:
            result.append(body)
    return result


def fields(body: str) -> dict[str, tuple[str, str]]:
    body = re.sub(r"(?m)^\s*///.*$", "", body)
    result: dict[str, tuple[str, str]] = {}
    start = 0
    depths = {"<": 0, "(": 0, "[": 0, "{": 0}
    closing = {">": "<", ")": "(", "]": "[", "}": "{"}
    for index, char in enumerate(body + ","):
        if char in depths:
            depths[char] += 1
        elif char in closing:
            depths[closing[char]] -= 1
        elif char == "," and not any(depths.values()):
            part = body[start:index].strip()
            start = index + 1
            match = re.match(
                r"(pub(?:\(crate\))?)\s+([A-Za-z_][A-Za-z0-9_]*)\s*:\s*(.+)",
                part,
                re.S,
            )
            if match:
                result[match.group(2)] = (match.group(1), re.sub(r"\s+", "", match.group(3)))
    return result


def check_struct(
    errors: list[str], source: str, name: str, expected: dict[str, str], visibility: str
) -> None:
    body = balanced_body(source, rf"pub\s+struct\s+{re.escape(name)}\b")
    if body is None:
        errors.append(f"missing public struct {name}")
        return
    found = fields(body)
    if set(found) != set(expected):
        errors.append(f"{name} fields {sorted(found)} != {sorted(expected)}")
        return
    for field, expected_type in expected.items():
        found_visibility, found_type = found[field]
        if found_visibility != visibility:
            errors.append(f"{name}.{field} visibility is {found_visibility}, expected {visibility}")
        if found_type != re.sub(r"\s+", "", expected_type):
            errors.append(f"{name}.{field} type is {found_type}, expected {expected_type}")


def check_getters(errors: list[str], source: str, name: str, expected: dict[str, str]) -> None:
    implementations = balanced_bodies(source, rf"impl\s+{re.escape(name)}\b")
    if not implementations:
        errors.append(f"missing impl {name}")
        return
    impl = "\n".join(implementations)
    methods = set(re.findall(r"pub\s+(?:const\s+)?fn\s+([A-Za-z_][A-Za-z0-9_]*)", impl))
    missing = set(expected) - methods
    extra = methods - set(expected)
    if missing:
        errors.append(f"{name} missing immutable getters {sorted(missing)}")
    if extra:
        errors.append(f"{name} exposes methods outside its getter surface {sorted(extra)}")
    if re.search(r"pub\s+(?:const\s+)?fn\s+\w+\s*\([^)]*&mut\s+self|->\s*&mut", impl, re.S):
        errors.append(f"{name} exposes mutable resolved state")
    for getter, field_type in expected.items():
        body = balanced_body(impl, rf"pub\s+(?:const\s+)?fn\s+{re.escape(getter)}\b")
        if body is None:
            continue
        normalized_body = re.sub(r"\s+", "", body)
        expected_body = (
            f"self.{getter}.as_ref()"
            if field_type.startswith("Option<")
            else f"&self.{getter}"
        )
        if normalized_body != expected_body:
            errors.append(f"{name}.{getter} does not return its matching immutable field")


def reject_default(errors: list[str], source: str, name: str) -> None:
    declaration = re.search(
        rf"#\[derive\(([^]]+)\)\]\s*(?:#\[[^]]+\]\s*)*pub\s+struct\s+{re.escape(name)}\b",
        source,
        re.S,
    )
    if declaration:
        derived = declaration.group(1)
        if "Default" in derived:
            errors.append(f"{name} exposes merge-bypassing Default")
        if "Deserialize" in derived:
            errors.append(f"{name} exposes merge-bypassing Deserialize")
    forbidden_impls = (
        rf"impl\s+Default\s+for\s+{re.escape(name)}\b",
        rf"impl(?:\s*<[^>]+>)?\s+From\s*<[^>]+>\s+for\s+{re.escape(name)}\b",
        rf"impl(?:\s*<[^>]+>)?\s+TryFrom\s*<[^>]+>\s+for\s+{re.escape(name)}\b",
        rf"impl(?:\s*<[^>]+>)?\s+(?:serde::)?Deserialize(?:\s*<[^>]+>)?\s+for\s+{re.escape(name)}\b",
    )
    for expression in forbidden_impls:
        if re.search(expression, source, re.S):
            errors.append(f"{name} exposes a merge-bypassing trait implementation")
            break


def require_tokens(errors: list[str], label: str, source: str, tokens: list[str]) -> None:
    normalized = re.sub(r"\s+", "", source)
    for token in tokens:
        if re.sub(r"\s+", "", token) not in normalized:
            errors.append(f"{label} missing contract {token}")


errors: list[str] = []
package_source = sources(PACKAGE_JSON)
pnpm_source = sources(PNPM_YAML)
if not package_source:
    errors.append("aqc-package-json-engine source is missing")
if not pnpm_source:
    errors.append("aqc-pnpm-workspace-yaml-engine source is missing")

check_struct(
    errors,
    package_source,
    "PackageJsonRequirements",
    {
        "package_manager": "Option<ScalarAssertion<String>>",
        "dev_engines_package_manager": "DevEnginePackageManagerRequirements",
    },
    "pub",
)
check_struct(
    errors,
    package_source,
    "DevEnginePackageManagerRequirements",
    {
        "name": "Option<ScalarAssertion<String>>",
        "version": "Option<ScalarAssertion<String>>",
        "on_fail": "Option<ScalarAssertion<PackageManagerOnFail>>",
    },
    "pub",
)
check_struct(
    errors,
    package_source,
    "ResolvedPackageJsonRequirements",
    {
        "package_manager": "Option<ResolvedRequirement<ScalarAssertion<String>,ScalarAssertion<String>>>",
        "dev_engines_package_manager": "ResolvedDevEnginePackageManagerRequirements",
    },
    "pub(crate)",
)
check_struct(
    errors,
    package_source,
    "ResolvedDevEnginePackageManagerRequirements",
    {
        "name": "Option<ResolvedRequirement<ScalarAssertion<String>,ScalarAssertion<String>>>",
        "version": "Option<ResolvedRequirement<ScalarAssertion<String>,ScalarAssertion<String>>>",
        "on_fail": "Option<ResolvedRequirement<ScalarAssertion<PackageManagerOnFail>,ScalarAssertion<PackageManagerOnFail>>>",
    },
    "pub(crate)",
)
check_getters(
    errors,
    package_source,
    "ResolvedPackageJsonRequirements",
    {
        "package_manager": "Option<ResolvedRequirement<ScalarAssertion<String>,ScalarAssertion<String>>>",
        "dev_engines_package_manager": "ResolvedDevEnginePackageManagerRequirements",
    },
)
check_getters(
    errors,
    package_source,
    "ResolvedDevEnginePackageManagerRequirements",
    {
        "name": "Option<ResolvedRequirement<ScalarAssertion<String>,ScalarAssertion<String>>>",
        "version": "Option<ResolvedRequirement<ScalarAssertion<String>,ScalarAssertion<String>>>",
        "on_fail": "Option<ResolvedRequirement<ScalarAssertion<PackageManagerOnFail>,ScalarAssertion<PackageManagerOnFail>>>",
    },
)
reject_default(errors, package_source, "ResolvedPackageJsonRequirements")
reject_default(errors, package_source, "ResolvedDevEnginePackageManagerRequirements")

pnpm_fields = {
    "strict_peer_dependencies": "Option<ScalarAssertion<bool>>",
    "engine_strict": "Option<ScalarAssertion<bool>>",
    "minimum_release_age": "Option<ScalarAssertion<PnpmReleaseAgeMinutes>>",
    "minimum_release_age_strict": "Option<ScalarAssertion<bool>>",
    "minimum_release_age_ignore_missing_time": "Option<ScalarAssertion<bool>>",
    "minimum_release_age_exclude": "ListRequirements",
    "forbidden_minimum_release_age_exclude_globs": "ForbiddenGlobRequirements<PnpmPackageSelectorGlob>",
    "trust_policy": "Option<ScalarAssertion<PnpmTrustPolicy>>",
    "trust_lockfile": "Option<ScalarAssertion<bool>>",
    "trust_policy_ignore_after": "Option<ScalarAssertion<u64>>",
    "trust_policy_exclude": "ListRequirements",
    "forbidden_trust_policy_exclude_globs": "ForbiddenGlobRequirements<PnpmPackageSelectorGlob>",
    "block_exotic_subdeps": "Option<ScalarAssertion<bool>>",
    "pm_on_fail": "Option<ScalarAssertion<PnpmOnFail>>",
    "strict_dep_builds": "Option<ScalarAssertion<bool>>",
    "dangerously_allow_all_builds": "Option<ScalarAssertion<bool>>",
    "allow_builds": "ItemRequirements<KeyedItem<bool>>",
    "forbidden_allowed_build_package_globs": "ForbiddenGlobRequirements<PnpmPackageSelectorGlob>",
    "exact_settings": "Option<String>",
}
check_struct(errors, pnpm_source, "PnpmWorkspaceYamlRequirements", pnpm_fields, "pub")
resolved_pnpm_fields = {
    "strict_peer_dependencies": "Option<ResolvedRequirement<ScalarAssertion<bool>,ScalarAssertion<bool>>>",
    "engine_strict": "Option<ResolvedRequirement<ScalarAssertion<bool>,ScalarAssertion<bool>>>",
    "minimum_release_age": "Option<ResolvedRequirement<ScalarAssertion<PnpmReleaseAgeMinutes>,ScalarAssertion<PnpmReleaseAgeMinutes>>>",
    "minimum_release_age_strict": "Option<ResolvedRequirement<ScalarAssertion<bool>,ScalarAssertion<bool>>>",
    "minimum_release_age_ignore_missing_time": "Option<ResolvedRequirement<ScalarAssertion<bool>,ScalarAssertion<bool>>>",
    "minimum_release_age_exclude": "ResolvedListRequirements",
    "forbidden_minimum_release_age_exclude_globs": "ResolvedForbiddenGlobRequirements<PnpmPackageSelectorGlob>",
    "trust_policy": "Option<ResolvedRequirement<ScalarAssertion<PnpmTrustPolicy>,ScalarAssertion<PnpmTrustPolicy>>>",
    "trust_lockfile": "Option<ResolvedRequirement<ScalarAssertion<bool>,ScalarAssertion<bool>>>",
    "trust_policy_ignore_after": "Option<ResolvedRequirement<ScalarAssertion<u64>,ScalarAssertion<u64>>>",
    "trust_policy_exclude": "ResolvedListRequirements",
    "forbidden_trust_policy_exclude_globs": "ResolvedForbiddenGlobRequirements<PnpmPackageSelectorGlob>",
    "block_exotic_subdeps": "Option<ResolvedRequirement<ScalarAssertion<bool>,ScalarAssertion<bool>>>",
    "pm_on_fail": "Option<ResolvedRequirement<ScalarAssertion<PnpmOnFail>,ScalarAssertion<PnpmOnFail>>>",
    "strict_dep_builds": "Option<ResolvedRequirement<ScalarAssertion<bool>,ScalarAssertion<bool>>>",
    "dangerously_allow_all_builds": "Option<ResolvedRequirement<ScalarAssertion<bool>,ScalarAssertion<bool>>>",
    "allow_builds": "ResolvedItemRequirements<KeyedItem<bool>>",
    "forbidden_allowed_build_package_globs": "ResolvedForbiddenGlobRequirements<PnpmPackageSelectorGlob>",
    "exact_settings": "Vec<(Provenance,String)>",
}
check_struct(errors, pnpm_source, "ResolvedPnpmWorkspaceYamlRequirements", resolved_pnpm_fields, "pub(crate)")
check_getters(errors, pnpm_source, "ResolvedPnpmWorkspaceYamlRequirements", resolved_pnpm_fields)
reject_default(errors, pnpm_source, "ResolvedPnpmWorkspaceYamlRequirements")

require_tokens(
    errors,
    "Package JSON engine",
    package_source,
    [
        "impl EngineRequirement for PackageJsonRequirements",
        "impl FileEngine<ResolvedPackageJsonRequirements> for PackageJsonEngine",
        "impl Engine for PackageJsonEngine",
        'ENGINE_ID: &str = "aqc-package-json-engine"',
        "impl ScalarValue for PackageManagerOnFail",
        '"download"',
        '"error"',
        '"warn"',
        '"ignore"',
    ],
)
require_tokens(
    errors,
    "pnpm YAML engine",
    pnpm_source,
    [
        "impl EngineRequirement for PnpmWorkspaceYamlRequirements",
        "impl FileEngine<ResolvedPnpmWorkspaceYamlRequirements> for PnpmWorkspaceYamlEngine",
        "impl Engine for PnpmWorkspaceYamlEngine",
        'ENGINE_ID: &str = "aqc-pnpm-workspace-yaml-engine"',
        "impl ScalarValue for PnpmOnFail",
        "impl ScalarValue for PnpmTrustPolicy",
        "impl ScalarValue for PnpmReleaseAgeMinutes",
        "9_007_199_254_740_991",
        '"no-downgrade"',
        '"off"',
        "pub fn exact_settings(&self) -> &[(Provenance, String)]",
    ],
)

for source, type_name, traits in (
    (package_source, "PackageManagerOnFail", {"Clone", "Eq", "Ord"}),
    (pnpm_source, "PnpmOnFail", {"Clone", "Eq", "Ord"}),
    (pnpm_source, "PnpmTrustPolicy", {"Clone", "Eq", "Ord"}),
    (pnpm_source, "PnpmReleaseAgeMinutes", {"Clone", "Eq", "Ord", "Serialize", "JsonSchema"}),
):
    declaration = re.search(rf"#\[derive\(([^]]+)\)\]\s*pub\s+(?:enum|struct)\s+{type_name}\b", source, re.S)
    found_traits = set(re.findall(r"[A-Za-z_][A-Za-z0-9_]*", declaration.group(1))) if declaration else set()
    if not traits.issubset(found_traits):
        errors.append(f"{type_name} missing derives {sorted(traits - found_traits)}")

require_tokens(
    errors,
    "PnpmReleaseAgeMinutes",
    pnpm_source,
    ["Deserialize<'de> for PnpmReleaseAgeMinutes", "Self::new(value)"],
)

emit(errors)

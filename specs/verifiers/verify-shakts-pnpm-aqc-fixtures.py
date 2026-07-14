#!/usr/bin/env python3
from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path


SPEC = Path(sys.argv[1]).resolve()
ROOT = SPEC.parent.parent
ENTRY = json.loads(SPEC.read_text())["requirements"]["custom"][int(sys.argv[3])]
APPROVED = ROOT / "fixtures/approved/shakts-pnpm-aqc/approved.normalized.json"
REQUIRED_RESULTS = {
    "duplicateJson",
    "missingPackageJson",
    "mergedYaml",
    "quotedMergeKey",
    "forbiddenSelector",
    "exactOnlyIsolation",
}


def command_errors(argv: list[str]) -> list[str]:
    result = subprocess.run(argv, cwd=ROOT, capture_output=True, text=True, check=False)
    if result.returncode == 0:
        return []
    detail = result.stderr.strip() or result.stdout.strip() or f"exit {result.returncode}"
    return [f"{' '.join(argv)} failed: {detail}"]


errors: list[str] = []
errors.extend(command_errors(["fixture3", "doctor", "--manifest", "fixture3.yaml", "--json"]))
errors.extend(command_errors(["fixture3", "check", "--suite", "shakts-pnpm-aqc", "--json"]))
try:
    approved = json.loads(APPROVED.read_text())
except (OSError, json.JSONDecodeError) as error:
    errors.append(f"approved AQC fixture output is unavailable: {error}")
    approved = {}
cases = approved.get("cases", []) if isinstance(approved, dict) else []
if len(cases) != 1 or cases[0].get("fixture") != "contracts.json":
    errors.append("AQC fixture approval must contain exactly the contracts.json public API probe")
elif set(cases[0].get("result", {})) != REQUIRED_RESULTS:
    result_keys = set(cases[0].get("result", {}))
    errors.append(
        f"AQC fixture result surface differs: missing={sorted(REQUIRED_RESULTS - result_keys)}, "
        f"extra={sorted(result_keys - REQUIRED_RESULTS)}"
    )
else:
    result = cases[0]["result"]
    duplicate = result["duplicateJson"]
    if len(duplicate) != 1 or duplicate[0].get("kind") != "parse" or "duplicate object member" not in duplicate[0].get("message", ""):
        errors.append("duplicateJson does not prove one duplicate-member parse failure")
    missing = result["missingPackageJson"]
    if missing.get("expected") != '{\n  "devEngines": {\n    "packageManager": {\n      "name": "tool",\n      "onFail": "error",\n      "version": "2.3.4"\n    }\n  },\n  "packageManager": "tool@2.3.4"\n}\n':
        errors.append("missingPackageJson expected bytes differ from the canonical document")
    if [item.get("key") for item in missing.get("findings", [])] != [
        "packageManager", "devEngines.packageManager.name",
        "devEngines.packageManager.version", "devEngines.packageManager.onFail",
    ]:
        errors.append("missingPackageJson does not report every managed field in order")
    merged = result["mergedYaml"]
    if merged != {"direct": True, "inherited": True, "keys": ["defaults", "direct", "inherited"]}:
        errors.append("mergedYaml does not prove direct and inherited lookup")
    if result["quotedMergeKey"] != {"keys": ["<<"], "value": "ordinary"}:
        errors.append("quotedMergeKey does not remain ordinary data")
    forbidden = result["forbiddenSelector"]
    forbidden_findings = forbidden.get("findings", [])
    if len(forbidden_findings) != 1 or forbidden_findings[0].get("selector") != "@scope/unsafe" or forbidden_findings[0].get("key") != "trustPolicyExclude":
        errors.append("forbiddenSelector does not preserve the offending selector")
    if "safe" not in forbidden.get("expected", "") or "unsafe" in forbidden.get("expected", ""):
        errors.append("forbiddenSelector expected bytes do not retain only the safe selector")
    exact = result["exactOnlyIsolation"]
    if exact.get("findingCount") != 1 or [item.get("key") for item in exact.get("findings", [])] != ["allowBuilds"]:
        errors.append("exactOnlyIsolation validates fields not represented by its requirement")
evidence = {"check": ENTRY["check"], "status": "fail" if errors else "pass"}
if errors:
    evidence["message"] = "; ".join(errors)
print(json.dumps(evidence, sort_keys=True))

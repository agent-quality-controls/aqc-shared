#!/usr/bin/env bash
# Layer 4: dependency rules.
#
# Three kinds of check:
#   - allowed_deps: each crate's direct Cargo.toml deps must be a subset
#     of the declared allow-list. Extras fail; missing Cargo.toml fails.
#   - forbidden_dep: explicit (from, to) pairs that must not appear.
#   - forbidden_import: per-crate forbidden module imports
#     (e.g. std::fs, std::process) enforced by source grep. Missing
#     src/ fails (we can't verify the rule without code).

set -euo pipefail
cd "$(dirname "$0")/.."

python3 - <<'PY'
import re
import sys
sys.path.insert(0, "scripts")
from _verify_lib import (
    CheckResult, cargo_dependencies, crate_cargo_toml_exists,
    crate_src_dir_exists, load_manifest, read_all_src, report,
)

manifest = load_manifest()
crate_paths = {row["crate"]: row["manifest_path"]
               for row in manifest.get("public_api", [])}

results = []

# --- allowed_deps ---
for row in manifest.get("allowed_deps", []):
    crate = row["crate"]
    allowed = set(row.get("allowed", []))
    mpath = crate_paths.get(crate)
    name = f"allowed_deps[{crate}]"
    if mpath is None:
        results.append(CheckResult(name, False, f"crate {crate} not in [[public_api]]"))
        continue
    actual = cargo_dependencies(mpath)
    if actual is None:
        results.append(CheckResult(
            name, False,
            f"crate {crate}'s Cargo.toml is missing at {mpath}/Cargo.toml; cannot verify deps"
        ))
        continue
    actual_set = set(actual)
    extras = actual_set - allowed
    if extras:
        results.append(CheckResult(
            name, False,
            f"crate {crate} declares unauthorized dependencies: {sorted(extras)}\n"
            f"  allowed: {sorted(allowed)}\n"
            f"  actual:  {sorted(actual_set)}",
        ))
    else:
        results.append(CheckResult(name, True))

# --- forbidden_dep ---
for row in manifest.get("forbidden_dep", []):
    src_crate = row["from"]
    dst = row["to"]
    mpath = crate_paths.get(src_crate)
    name = f"forbidden_dep[{src_crate} -> {dst}]"
    if mpath is None:
        results.append(CheckResult(
            name, False, f"source crate {src_crate} not in [[public_api]]; cannot check"
        ))
        continue
    deps = cargo_dependencies(mpath)
    if deps is None:
        results.append(CheckResult(
            name, False,
            f"crate {src_crate}'s Cargo.toml is missing at {mpath}/Cargo.toml; cannot verify"
        ))
        continue
    if dst in set(deps):
        results.append(CheckResult(
            name, False, f"crate {src_crate} declares forbidden dep on {dst}"
        ))
    else:
        results.append(CheckResult(name, True))

# --- forbidden_import ---
for row in manifest.get("forbidden_import", []):
    crate = row["from_crate"]
    imp = row["import"]
    mpath = crate_paths.get(crate)
    name = f"forbidden_import[{crate}: {imp}]"
    if mpath is None:
        results.append(CheckResult(name, False, f"crate {crate} not in [[public_api]]"))
        continue
    if not crate_src_dir_exists(mpath):
        results.append(CheckResult(
            name, False,
            f"crate {crate}'s src/ directory is missing at {mpath}/src; cannot verify"
        ))
        continue
    src = read_all_src(mpath)
    pattern = r"\b" + re.escape(imp) + r"(::|$|\s|;)"
    hits = []
    for lineno, line in enumerate(src.splitlines(), 1):
        if re.search(pattern, line):
            stripped = line.lstrip()
            if stripped.startswith("//") or stripped.startswith("*"):
                continue
            hits.append(f"  line {lineno}: {line.rstrip()}")
    if hits:
        results.append(CheckResult(
            name, False,
            f"crate {crate} contains forbidden import `{imp}`:\n" + "\n".join(hits[:10]),
        ))
    else:
        results.append(CheckResult(name, True))

sys.exit(report("layer 4 (dependency rules)", results))
PY

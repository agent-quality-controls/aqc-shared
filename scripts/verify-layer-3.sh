#!/usr/bin/env bash
# Layer 3: closed sets.
#
# For every [[closed_set]] row with kind="enum", parse the enum's body
# from source and verify variants match exactly (no extras, no missing).

set -euo pipefail
cd "$(dirname "$0")/.."

python3 - <<'PY'
import sys
sys.path.insert(0, "scripts")
from _verify_lib import (
    CheckResult, find_enum_block, load_manifest, parse_enum_variants,
    read_all_src, report,
)

manifest = load_manifest()
# Map crate -> manifest_path via the public_api rows.
crate_paths = {row["crate"]: row["manifest_path"]
               for row in manifest.get("public_api", [])}

results = []
for row in manifest.get("closed_set", []):
    if row.get("kind") != "enum":
        continue
    crate = row["crate"]
    type_name = row["type"]
    want = set(row["variants"])
    name = f"closed_set[{crate}::{type_name}]"

    mpath = crate_paths.get(crate)
    if mpath is None:
        results.append(CheckResult(
            name, False, f"crate {crate} not declared in [[public_api]]; cannot locate src tree"
        ))
        continue
    src = read_all_src(mpath)
    body = find_enum_block(src, type_name)
    if body is None:
        results.append(CheckResult(
            name, False, f"could not find `pub enum {type_name}` in {mpath}/src/"
        ))
        continue
    got = parse_enum_variants(body)
    missing = want - got
    extra = got - want
    if missing or extra:
        detail = []
        if missing:
            detail.append(f"missing variants: {sorted(missing)}")
        if extra:
            detail.append(f"unexpected variants: {sorted(extra)}")
        results.append(CheckResult(name, False, "\n".join(detail)))
    else:
        results.append(CheckResult(name, True))

sys.exit(report("layer 3 (closed_sets)", results))
PY

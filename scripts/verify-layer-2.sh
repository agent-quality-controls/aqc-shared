#!/usr/bin/env bash
# Layer 2: public API.
#
# For every [[public_api]] row, verify that each named type/trait/function
# is declared `pub` somewhere in the crate's src/ tree. Extras allowed;
# missing items fail.

set -euo pipefail
cd "$(dirname "$0")/.."

python3 - <<'PY'
import sys
sys.path.insert(0, "scripts")
from _verify_lib import (
    CheckResult, load_manifest, public_item_present, read_all_src, report,
)

manifest = load_manifest()
results = []
for row in manifest.get("public_api", []):
    crate = row["crate"]
    mpath = row["manifest_path"]
    src = read_all_src(mpath)
    if not src:
        for name in row.get("types", []) + row.get("functions", []):
            results.append(CheckResult(
                f"public_api[{crate}::{name}]", False,
                f"crate has no src/ tree at {mpath}/src",
            ))
        continue
    for name in row.get("types", []):
        if public_item_present(src, name):
            results.append(CheckResult(f"public_api[{crate}::{name}]", True))
        else:
            results.append(CheckResult(
                f"public_api[{crate}::{name}]", False,
                f"no `pub (struct|enum|trait|fn|type|union) {name}` found under {mpath}/src/",
            ))
    for name in row.get("functions", []):
        if public_item_present(src, name):
            results.append(CheckResult(f"public_api[{crate}::fn {name}]", True))
        else:
            results.append(CheckResult(
                f"public_api[{crate}::fn {name}]", False,
                f"no `pub fn {name}` found under {mpath}/src/",
            ))

sys.exit(report("layer 2 (public_api)", results))
PY

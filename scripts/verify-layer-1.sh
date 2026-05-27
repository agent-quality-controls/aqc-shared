#!/usr/bin/env bash
# Layer 1: file tree.
#
# Verify that every [[tree]] path declared in the manifest exists under
# the target repo root. Missing paths fail.

set -euo pipefail
cd "$(dirname "$0")/.."

python3 - <<'PY'
import sys
sys.path.insert(0, "scripts")
from _verify_lib import REPO_ROOT, load_manifest, CheckResult, report

manifest = load_manifest()
results = []
for row in manifest.get("tree", []):
    path = row["path"]
    full = REPO_ROOT / path
    name = f"tree: {path}"
    if full.exists():
        results.append(CheckResult(name, True))
    else:
        results.append(CheckResult(name, False, f"expected file missing: {full}"))

sys.exit(report("layer 1 (tree)", results))
PY

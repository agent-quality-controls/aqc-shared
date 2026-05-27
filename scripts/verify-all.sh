#!/usr/bin/env bash
# Run all manifest verifier layers. Each layer is independent; we run all of
# them and aggregate the result so the user sees every failure, not just the
# first.

set -uo pipefail
cd "$(dirname "$0")/.."

scripts=(
    scripts/verify-layer-1.sh
    scripts/verify-layer-2.sh
    scripts/verify-layer-3.sh
    scripts/verify-layer-4.sh
    scripts/verify-layer-5.sh
)

overall=0
for s in "${scripts[@]}"; do
    echo "=== $s ==="
    if ! bash "$s"; then
        overall=1
    fi
    echo
done

if [[ $overall -eq 0 ]]; then
    echo "=== ALL LAYERS PASS ==="
else
    echo "=== ONE OR MORE LAYERS FAILED ==="
fi
exit $overall

#!/usr/bin/env bash
set -euo pipefail

# Run all verification layers from the facade workspace root.
cd "$(dirname "$0")/.."

./verify/verify-layer-1.sh
./verify/verify-layer-2.sh
./verify/verify-layer-3.sh
./verify/verify-layer-4.sh
./verify/verify-layer-5.sh

echo "=== ALL LAYERS PASS ==="

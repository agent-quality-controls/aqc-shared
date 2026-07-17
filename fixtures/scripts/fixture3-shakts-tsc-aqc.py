#!/usr/bin/env python3
from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path

sys.dont_write_bytecode = True

import json
ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))

from local_cargo_source import fixture_cargo_home


PROBE = ROOT / "fixtures/probes/shakts-tsc-aqc/Cargo.toml"
CACHE_ROOT = Path(os.environ.get("FIXTURE3_CARGO_CACHE_ROOT", ROOT / ".cargo-target/fixtures"))


def cargo_home() -> Path:
    return fixture_cargo_home(ROOT, PROBE, "shakts-tsc-aqc")


def main() -> int:
    fixtures = [Path(argument).resolve() for argument in sys.argv[1:]]
    if not fixtures:
        raise SystemExit("fixture paths are required")
    env = os.environ.copy()
    env["CARGO_HOME"] = str(cargo_home())
    env["CARGO_TARGET_DIR"] = str(CACHE_ROOT / "shakts-tsc-aqc/target")
    outputs = []
    for fixture in fixtures:
        result = subprocess.run(
            ["cargo", "run", "--quiet", "--locked", "--manifest-path", str(PROBE), "--", str(fixture)],
            cwd=ROOT,
            env=env,
            capture_output=True,
            text=True,
            check=False,
        )
        if result.returncode != 0:
            raise SystemExit(result.stdout + result.stderr)
        outputs.append({"fixture": fixture.name, "result": json.loads(result.stdout)})
    print(json.dumps({"cases": outputs}, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

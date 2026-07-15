#!/usr/bin/env python3
from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path

sys.dont_write_bytecode = True

from local_cargo_source import fixture_cargo_home


ROOT = Path(__file__).resolve().parents[2]
PROBE = ROOT / "fixtures/probes/generic-json-file-engine/Cargo.toml"


def main() -> int:
    fixtures = [Path(argument).resolve() for argument in sys.argv[1:]]
    if not fixtures:
        raise SystemExit("fixture paths are required")
    env = os.environ.copy()
    env["CARGO_HOME"] = str(fixture_cargo_home(ROOT, PROBE, "generic-json-file-engine"))
    env["CARGO_TARGET_DIR"] = str(Path(tempfile.gettempdir()) / "generic-json-file-engine-fixture-target")
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

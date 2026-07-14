#!/usr/bin/env python3
from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
import tomllib
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
PROBE = ROOT / "fixtures/probes/shakts-pnpm-aqc/Cargo.toml"


def local_packages() -> dict[str, Path]:
    packages: dict[str, Path] = {}
    for manifest in (ROOT / "packages").rglob("Cargo.toml"):
        if "target" in manifest.parts or "fixtures" in manifest.parts:
            continue
        document = tomllib.loads(manifest.read_text())
        name = document.get("package", {}).get("name")
        if isinstance(name, str):
            packages[name] = manifest.parent.resolve()
    return packages


def cargo_home() -> Path:
    home = Path(tempfile.gettempdir()) / "shakts-pnpm-aqc-fixture-cargo-home"
    home.mkdir(parents=True, exist_ok=True)
    lines = ["[patch.crates-io]"]
    for name, path in sorted(local_packages().items()):
        lines.append(f"{json.dumps(name)} = {{ path = {json.dumps(str(path))} }}")
    (home / "config.toml").write_text("\n".join(lines) + "\n")
    return home


def main() -> int:
    fixtures = [Path(argument).resolve() for argument in sys.argv[1:]]
    if not fixtures:
        raise SystemExit("fixture paths are required")
    env = os.environ.copy()
    env["CARGO_HOME"] = str(cargo_home())
    env["CARGO_TARGET_DIR"] = str(Path(tempfile.gettempdir()) / "shakts-pnpm-aqc-fixture-target")
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

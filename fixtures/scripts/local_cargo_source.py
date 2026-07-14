from __future__ import annotations

import json
import tempfile
import tomllib
from pathlib import Path


def _local_packages(root: Path) -> dict[str, Path]:
    packages: dict[str, Path] = {}
    for manifest in (root / "packages").rglob("Cargo.toml"):
        if "target" in manifest.parts or "fixtures" in manifest.parts:
            continue
        document = tomllib.loads(manifest.read_text())
        name = document.get("package", {}).get("name")
        if isinstance(name, str):
            packages[name] = manifest.parent.resolve()
    return packages


def _manifest_dependencies(manifest: Path) -> set[str]:
    document = tomllib.loads(manifest.read_text())
    tables = [document, *document.get("target", {}).values()]
    names: set[str] = set()
    for table in tables:
        for section in ("dependencies", "dev-dependencies", "build-dependencies"):
            for key, value in table.get(section, {}).items():
                names.add(value.get("package", key) if isinstance(value, dict) else key)
    return names


def fixture_cargo_home(root: Path, probe: Path, suite: str) -> Path:
    packages = _local_packages(root)
    selected: dict[str, Path] = {}
    pending = _manifest_dependencies(probe)
    while pending:
        name = pending.pop()
        package = packages.get(name)
        if package is None or name in selected:
            continue
        selected[name] = package
        pending.update(_manifest_dependencies(package / "Cargo.toml"))

    home = Path(tempfile.gettempdir()) / f"{suite}-fixture-cargo-home"
    home.mkdir(parents=True, exist_ok=True)
    lines = ["[patch.crates-io]"]
    for name, path in sorted(selected.items()):
        lines.append(f"{json.dumps(name)} = {{ path = {json.dumps(str(path))} }}")
    (home / "config.toml").write_text("\n".join(lines) + "\n")
    return home

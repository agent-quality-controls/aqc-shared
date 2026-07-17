from __future__ import annotations

import argparse
import fcntl
import json
import os
import shutil
import subprocess
import tempfile
import tomllib
from pathlib import Path


def materialize_git_tree(root: Path, tree: str, destination: Path) -> None:
    destination.parent.mkdir(parents=True, exist_ok=True)
    lock_path = destination.with_suffix(".lock")
    with lock_path.open("w") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        marker = destination / ".aqc-complete"
        if marker.is_file():
            return
        shutil.rmtree(destination, ignore_errors=True)
        temporary = Path(tempfile.mkdtemp(dir=destination.parent, prefix="source."))
        index = destination.parent / f"index.{os.getpid()}"
        try:
            environment = {**os.environ, "GIT_INDEX_FILE": str(index)}
            subprocess.run(
                ["git", "-C", str(root), "read-tree", tree],
                check=True,
                env=environment,
            )
            subprocess.run(
                ["git", "-C", str(root), "checkout-index", "--all", f"--prefix={temporary}/"],
                check=True,
                env=environment,
            )
            (temporary / ".aqc-complete").touch()
            os.replace(temporary, destination)
        finally:
            index.unlink(missing_ok=True)
            shutil.rmtree(temporary, ignore_errors=True)


def local_packages(root: Path) -> dict[str, Path]:
    packages: dict[str, Path] = {}
    for manifest in (root / "packages").rglob("Cargo.toml"):
        if "target" in manifest.parts or "fixtures" in manifest.parts:
            continue
        document = tomllib.loads(manifest.read_text())
        name = document.get("package", {}).get("name")
        if not isinstance(name, str):
            continue
        package_path = manifest.parent.resolve()
        previous = packages.setdefault(name, package_path)
        if previous != package_path:
            raise RuntimeError(f"duplicate local Cargo package {name}: {previous}, {package_path}")
    return packages


def manifest_registry_dependencies(manifest: Path) -> set[str]:
    document = tomllib.loads(manifest.read_text())
    tables = [document, *document.get("target", {}).values()]
    names: set[str] = set()
    for table in tables:
        for section in ("dependencies", "dev-dependencies", "build-dependencies"):
            for key, value in table.get(section, {}).items():
                if isinstance(value, dict) and "path" in value:
                    continue
                names.add(value.get("package", key) if isinstance(value, dict) else key)
    return names


def transitive_local_packages(
    packages: dict[str, Path],
    manifests: tuple[Path, ...],
) -> dict[str, Path]:
    selected: dict[str, Path] = {}
    pending = set().union(
        *(manifest_registry_dependencies(manifest) for manifest in manifests)
    )
    while pending:
        name = pending.pop()
        package = packages.get(name)
        if package is None or name in selected:
            continue
        selected[name] = package
        pending.update(manifest_registry_dependencies(package / "Cargo.toml"))
    return selected


def write_patch_config(
    root: Path,
    config: Path,
    manifests: tuple[Path, ...] = (),
) -> None:
    packages = local_packages(root)
    if manifests:
        packages = transitive_local_packages(packages, manifests)
    lines = ["[patch.crates-io]"]
    for name, path in sorted(packages.items()):
        lines.append(f"{json.dumps(name)} = {{ path = {json.dumps(str(path))} }}")
    config.parent.mkdir(parents=True, exist_ok=True)
    rendered = "\n".join(lines) + "\n"
    if config.is_file() and config.read_text() == rendered:
        return
    descriptor, temporary = tempfile.mkstemp(dir=config.parent, prefix="config.", suffix=".toml")
    try:
        with os.fdopen(descriptor, "w") as output:
            output.write(rendered)
        os.replace(temporary, config)
    except BaseException:
        Path(temporary).unlink(missing_ok=True)
        raise


def fixture_cargo_home(root: Path, probe: Path, suite: str) -> Path:
    cache_root = Path(
        os.environ.get("FIXTURE3_CARGO_CACHE_ROOT", root / ".cargo-target" / "fixtures")
    )
    scope = os.environ.get("FIXTURE3_CARGO_CONFIG_SCOPE", "working-tree")
    home = cache_root / "cargo-homes" / scope / suite
    write_patch_config(root, home / "config.toml", (probe.resolve(),))
    return home


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", required=True, type=Path)
    parser.add_argument("--config", required=True, type=Path)
    parser.add_argument("--manifest", action="append", default=[], type=Path)
    parser.add_argument("--staged-root", type=Path)
    parser.add_argument("--staged-tree")
    args = parser.parse_args()
    root = args.root.resolve()
    manifests = tuple(manifest.resolve() for manifest in args.manifest)
    if args.staged_root is not None:
        if args.staged_tree is None:
            parser.error("--staged-root requires --staged-tree")
        staged_root = args.staged_root.resolve()
        materialize_git_tree(root, args.staged_tree, staged_root)
        manifests = tuple(staged_root / manifest.relative_to(root) for manifest in manifests)
        root = staged_root
    write_patch_config(
        root,
        args.config.resolve(),
        manifests,
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

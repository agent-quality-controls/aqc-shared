#!/usr/bin/env python3
from __future__ import annotations

import importlib.util
import hashlib
import os
import shutil
import subprocess
import tempfile
import threading
import tomllib
import unittest
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path


SCRIPT = Path(__file__).with_name("local_cargo_source.py")
SPEC = importlib.util.spec_from_file_location("local_cargo_source", SCRIPT)
assert SPEC and SPEC.loader
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


def manifest(name: str, dependencies: str = "") -> str:
    return f'''[package]
name = "{name}"
version = "0.1.0"
edition = "2024"

[dependencies]
{dependencies}'''


class LocalCargoSourceTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary.name)
        self.core = self.root / "packages" / "core"
        self.core.mkdir(parents=True)
        (self.core / "Cargo.toml").write_text(manifest("local-core"))
        self.consumer = self.root / "consumer" / "Cargo.toml"
        self.consumer.parent.mkdir()

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def test_unchanged_configuration_keeps_the_same_file(self) -> None:
        self.consumer.write_text(manifest("consumer", 'local-core = "0.1"\n'))
        config = self.root / "cargo-home" / "config.toml"
        MODULE.write_patch_config(self.root, config, (self.consumer,))
        first = config.stat()
        MODULE.write_patch_config(self.root, config, (self.consumer,))
        second = config.stat()
        self.assertEqual((first.st_ino, first.st_mtime_ns), (second.st_ino, second.st_mtime_ns))

    def test_concurrent_generation_never_exposes_partial_toml(self) -> None:
        self.consumer.write_text(manifest("consumer", 'local-core = "0.1"\n'))
        config = self.root / "cargo-home" / "config.toml"
        stop = threading.Event()
        parse_errors: list[Exception] = []
        observed = 0
        observation_lock = threading.Lock()

        def read_while_writing() -> None:
            nonlocal observed
            while not stop.is_set():
                if config.is_file():
                    try:
                        tomllib.loads(config.read_text())
                        with observation_lock:
                            observed += 1
                    except (OSError, tomllib.TOMLDecodeError) as error:
                        parse_errors.append(error)

        alternate = self.root / "packages" / "alternate"
        alternate.mkdir()
        (alternate / "Cargo.toml").write_text(manifest("alternate-core"))
        first = self.root / "consumer-first" / "Cargo.toml"
        second = self.root / "consumer-second" / "Cargo.toml"
        first.parent.mkdir()
        second.parent.mkdir()
        first.write_text(manifest("consumer-first", 'local-core = "0.1"\n'))
        second.write_text(manifest("consumer-second", 'alternate-core = "0.1"\n'))

        reader = threading.Thread(target=read_while_writing)
        reader.start()
        try:
            with ThreadPoolExecutor(max_workers=8) as workers:
                list(
                    workers.map(
                        lambda index: MODULE.write_patch_config(
                            self.root,
                            config,
                            (first if index % 2 == 0 else second,),
                        ),
                        range(64),
                    )
                )
        finally:
            stop.set()
            reader.join(timeout=5)
        self.assertFalse(reader.is_alive())
        self.assertEqual(parse_errors, [])
        self.assertGreater(observed, 0)
        tomllib.loads(config.read_text())

    def test_explicit_path_dependency_is_not_patched(self) -> None:
        self.consumer.write_text(manifest("consumer", 'local-core = { path = "../packages/core" }\n'))
        config = self.root / "cargo-home" / "config.toml"
        MODULE.write_patch_config(self.root, config, (self.consumer,))
        self.assertEqual(config.read_text(), "[patch.crates-io]\n")

    def test_wrapper_honors_staged_after_path_and_uses_workspace_repository(self) -> None:
        repository = Path(__file__).resolve().parents[1]
        workspace = repository / ".cargo-target" / "local-cargo-test-workspace"
        shutil.rmtree(workspace, ignore_errors=True)
        (workspace / "src").mkdir(parents=True)
        (workspace / "Cargo.toml").write_text(manifest("wrapper-consumer"))
        (workspace / "src/lib.rs").write_text("pub fn consumer() {}\n")
        manifest_path = workspace / "Cargo.toml"
        relative_manifest = manifest_path.relative_to(repository)
        try:
          with tempfile.TemporaryDirectory() as temporary:
            temporary_path = Path(temporary)
            index = temporary_path / "index"
            subprocess.run(
                ["git", "-C", str(repository), "read-tree", "HEAD"],
                check=True,
                env={**os.environ, "GIT_INDEX_FILE": str(index)},
            )
            staged = manifest("wrapper-consumer", 'staged-only = "0.1"\n')
            blob = subprocess.run(
                ["git", "-C", str(repository), "hash-object", "-w", "--stdin"],
                input=staged,
                text=True,
                capture_output=True,
                check=True,
            ).stdout.strip()
            subprocess.run(
                ["git", "-C", str(repository), "update-index", "--add", "--cacheinfo", "100644", blob, str(relative_manifest)],
                check=True,
                env={**os.environ, "GIT_INDEX_FILE": str(index)},
            )
            staged_package = '''[package]
name = "staged-only"
version = "0.1.0"
edition = "2024"
'''
            for relative, source in {
                "packages/staged-only/Cargo.toml": staged_package,
                "packages/staged-only/src/lib.rs": "pub fn staged_only() {}\n",
                f"{relative_manifest.parent.as_posix()}/src/lib.rs": "pub fn consumer() {}\n",
            }.items():
                package_blob = subprocess.run(
                    ["git", "-C", str(repository), "hash-object", "-w", "--stdin"],
                    input=source,
                    text=True,
                    capture_output=True,
                    check=True,
                ).stdout.strip()
                subprocess.run(
                    ["git", "-C", str(repository), "update-index", "--add", "--cacheinfo", "100644", package_blob, relative],
                    check=True,
                    env={**os.environ, "GIT_INDEX_FILE": str(index)},
                )
            identity = hashlib.sha256(str(workspace).encode()).hexdigest()[:16]
            config = repository / ".cargo-target" / "pre-commit" / "cargo-homes" / identity / "config.toml"
            config.unlink(missing_ok=True)
            wrapper = repository / ".githooks" / "bin" / "g3rs"
            subprocess.run(
                [str(wrapper), "validate", "workspace", "--path", str(workspace), "--staged"],
                cwd=repository,
                check=True,
                env={
                    **os.environ,
                    "GIT_INDEX_FILE": str(index),
                    "AQC_G3RS_BINARY": "/usr/bin/true",
                    "AQC_SHAKRS_BINARY": "/usr/bin/true",
                },
            )
            document = tomllib.loads(config.read_text())
            package_path = Path(document["patch"]["crates-io"]["staged-only"]["path"])
            self.assertIn("staged-sources", package_path.parts)
            self.assertEqual(tomllib.loads((package_path / "Cargo.toml").read_text())["package"]["name"], "staged-only")
            snapshot_manifest = package_path.parents[1] / relative_manifest
            subprocess.run(
                ["cargo", "check", "--offline", "--manifest-path", str(snapshot_manifest)],
                check=True,
                capture_output=True,
                env={**os.environ, "CARGO_HOME": str(config.parent)},
            )
        finally:
            shutil.rmtree(workspace, ignore_errors=True)

    def test_materialization_reads_the_named_tree_not_the_mutable_index(self) -> None:
        tracked = self.root / "tracked.txt"
        tracked.write_text("tree-a\n")
        subprocess.run(["git", "init", "-q", str(self.root)], check=True)
        subprocess.run(["git", "-C", str(self.root), "add", "."], check=True)
        tree_a = subprocess.run(
            ["git", "-C", str(self.root), "write-tree"],
            text=True,
            capture_output=True,
            check=True,
        ).stdout.strip()
        tracked.write_text("tree-b\n")
        subprocess.run(["git", "-C", str(self.root), "add", "tracked.txt"], check=True)
        destination = self.root / "snapshot"
        MODULE.materialize_git_tree(self.root, tree_a, destination)
        self.assertEqual((destination / "tracked.txt").read_text(), "tree-a\n")


if __name__ == "__main__":
    unittest.main()

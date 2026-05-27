#!/usr/bin/env bash
# Layer 5: structural details.
#
#   - struct_shape: struct field names and types match exactly.
#   - impl_required: a `impl <Trait> for <Type>` line is present.
#   - trait_sig: each signature_must_contain substring is present
#     near the trait declaration.
#   - verification_command: shell command exits with the expected code.

set -euo pipefail
cd "$(dirname "$0")/.."

python3 - <<'PY'
import re
import sys
sys.path.insert(0, "scripts")
from _verify_lib import (
    CheckResult, REPO_ROOT, find_struct_block, load_manifest,
    parse_struct_fields, read_all_src, report, run_command, workspace_root_for,
)

manifest = load_manifest()
crate_paths = {row["crate"]: row["manifest_path"]
               for row in manifest.get("public_api", [])}

results = []

# --- struct_shape ---
for row in manifest.get("struct_shape", []):
    crate = row["crate"]
    type_name = row["type"]
    want_fields = [(f["name"], f["type"].replace(" ", "")) for f in row["fields"]]
    name = f"struct_shape[{crate}::{type_name}]"
    mpath = crate_paths.get(crate)
    if mpath is None:
        results.append(CheckResult(name, False, f"crate {crate} not in [[public_api]]"))
        continue
    src = read_all_src(mpath)
    body = find_struct_block(src, type_name)
    if body is None:
        results.append(CheckResult(
            name, False, f"could not find `pub struct {type_name}` in {mpath}/src/"
        ))
        continue
    if body == "":
        # Unit struct, no fields permitted.
        if want_fields:
            results.append(CheckResult(
                name, False, f"struct {type_name} is a unit struct but manifest expects fields: {want_fields}"
            ))
        else:
            results.append(CheckResult(name, True))
        continue
    got = parse_struct_fields(body)
    got_norm = [(n, t) for (n, t) in got]
    want_norm = list(want_fields)
    if got_norm == want_norm:
        results.append(CheckResult(name, True))
    else:
        detail = [f"expected: {want_norm}", f"got:      {got_norm}"]
        results.append(CheckResult(name, False, "\n".join(detail)))

# --- impl_required ---
for row in manifest.get("impl_required", []):
    crate = row["crate"]
    trait_impl = row["trait_impl"]
    type_name = row["type"]
    name = f"impl_required[{crate}: impl {trait_impl} for {type_name}]"
    mpath = crate_paths.get(crate)
    if mpath is None:
        results.append(CheckResult(name, False, f"crate {crate} not in [[public_api]]"))
        continue
    src = read_all_src(mpath)
    # Allow surrounding whitespace; tolerate generics
    needle_re = (
        r"impl(?:<[^>]*>)?\s+"
        + re.escape(trait_impl)
        + r"\s+for\s+"
        + re.escape(type_name)
        + r"\b"
    )
    if re.search(needle_re, src):
        results.append(CheckResult(name, True))
    else:
        results.append(CheckResult(
            name, False, f"no `impl {trait_impl} for {type_name}` found in {mpath}/src/"
        ))

# --- trait_sig ---
for row in manifest.get("trait_sig", []):
    crate = row["crate"]
    trait_name = row["trait_name"]
    method = row["method"]
    needles = row.get("signature_must_contain", [])
    name = f"trait_sig[{crate}::{trait_name}::{method}]"
    mpath = crate_paths.get(crate)
    if mpath is None:
        results.append(CheckResult(name, False, f"crate {crate} not in [[public_api]]"))
        continue
    src = read_all_src(mpath)
    # Find the trait block first.
    trait_re = re.search(
        rf"\bpub\s+trait\s+{re.escape(trait_name)}\b[^{{]*{{",
        src,
    )
    if not trait_re:
        results.append(CheckResult(
            name, False, f"no `pub trait {trait_name}` found in {mpath}/src/"
        ))
        continue
    # Take a generous chunk after the trait header — the method body is
    # checked by substring presence, so over-including is fine.
    chunk_start = trait_re.start()
    chunk = src[chunk_start : chunk_start + 4000]
    missing = [n for n in needles if n not in chunk]
    if missing:
        results.append(CheckResult(
            name, False,
            f"trait {trait_name} body missing substrings: {missing}",
        ))
    else:
        results.append(CheckResult(name, True))

# --- verification_command ---
for row in manifest.get("verification_command", []):
    name = f"verification_command[{row['name']}]"
    cmd = row["command"]
    expect = int(row.get("expect_exit", 0))
    # Run from any tracked crate's workspace root. If no crate exists on
    # disk yet, this layer can't run cargo at all — fail loudly.
    cwd = None
    for mpath in crate_paths.values():
        ws = workspace_root_for(mpath)
        if ws is not None:
            cwd = ws
            break
    if cwd is None:
        results.append(CheckResult(
            name, False,
            f"no workspace root found for any tracked crate; cannot run `{cmd}`"
        ))
        continue
    rc, out = run_command(cmd, cwd)
    if rc == expect:
        results.append(CheckResult(name, True))
    else:
        tail = "\n".join(out.splitlines()[-15:])
        results.append(CheckResult(
            name, False,
            f"command `{cmd}` exited {rc} (expected {expect})\n"
            f"  last lines of output:\n{tail}",
        ))

sys.exit(report("layer 5 (structural)", results))
PY

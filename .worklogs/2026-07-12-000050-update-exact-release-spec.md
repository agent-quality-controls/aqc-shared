# Summary

Updated the exact-item Specular contract to the published patch releases and made it verify both manifest minimums and resolved lockfile versions.

# Decisions made

- Require file-engine core 0.5.2 and TOML core 0.5.1.
- Require every dependent lockfile to resolve file-engine core 0.5.2.
- Keep built-in content and dependency checks; use the custom verifier only for lockfile and multi-workspace gates.

# Key files for context

- `specs/create-only-init-and-exact-items.spec.json`
- `specs/verifiers/verify_create_only_exact_aqc.py`

# Next steps

- Complete and commit repository adoption separately.
- Re-run Specular after adoption leaves no unrelated working-tree changes.

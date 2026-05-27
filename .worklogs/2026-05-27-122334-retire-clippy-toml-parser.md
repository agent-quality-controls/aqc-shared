# Retire `aqc-clippy-toml-parser`

## Summary

Deleted the `packages/file-types/toml/aqc-clippy-toml-parser/` package
(generator + generated types + runtime stub + manifest + verifier).
Built earlier this session as a typed-schema source for `clippy.toml`,
but after the design walk-through showed every responsibility of a
"domain parser layer" dissolves, the artifact has no consumer.

Recoverable from git history if a future first-party
`clippy-config-validate` linter ever wants typed schema input.

## Decisions made

- **The package is removed wholesale**, not stubbed or replaced. The
  vertical slice will build `aqc-clippy-toml-engine` directly on
  `toml_edit`, with no typed-schema dependency.
- **Empty `aqc-*-*-parser/` stub directories were left in place** for
  now. They predate this session and removing all of them is a wider
  cleanup that should be a separate explicit decision.

## Verification

- `find packages/file-types/toml/aqc-clippy-toml-parser/` returns
  nothing.
- `git status` shows 30 deletions under that path.

## Key files

- `packages/file-types/toml/aqc-clippy-toml-parser/` - deleted.

## Plan reference

Plan changes that justified this deletion landed in guardrail3 as
`2b1f5e646 docs(plans): collapse domain-parser layer; engines go
direct on grammar crates`. See:

- `2026-05-21-195830-repo-workspace-plugin-generation-model.md` -
  "Why no domain parser layer" subsection.
- `2026-05-26-193045-aqc-parser-migration.md` - file-engines plan,
  status table now marks the parser as retired.

## Next steps

- Build `aqc-cargo-toml-engine` and `aqc-clippy-toml-engine` for the
  vertical slice once the user confirms direction.
- Open question: should the empty `aqc-*-*-parser/` stub directories
  also be removed? 18 of them, all just `.gitkeep`. Match the
  collapsed-layer plan but the user only authorized the populated
  package's removal in the previous turn.

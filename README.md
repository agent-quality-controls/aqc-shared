# aqc-shared

> Building a crate here? Read [CONVENTIONS.md](CONVENTIONS.md) first --
> the commit gates enforce structural rules that exist nowhere else in prose.

Neutral `aqc-*` libraries for Agent Quality Controls (Spec3, Guardrail3, Fixture3, …).

## Principles

- **No shared finding / evidence crate.** Spec3 evidence, `G3CheckResult`, websmasher `AuditFinding`, and `low-expectations` suites stay in their products.
- Small crates with `plan.md` before implementation.

## Packages

- `packages/aqc-filetree/` — filesystem walk → `FileTree` ([plan](packages/aqc-filetree/plan.md))
- `packages/aqc-fs-utils/` — read text/bytes with fixed edge-case rules ([plan](packages/aqc-fs-utils/plan.md))
- `packages/aqc-git-helpers/` — porcelain worktree state for lock/verify ([plan](packages/aqc-git-helpers/plan.md))
- `packages/file-types/` — parser/engine crate stubs by syntax family (`toml/`, `jsonc/`)

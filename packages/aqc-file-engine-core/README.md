# aqc-file-engine-core

Shared file-engine contracts for Agent Quality Controls.

This crate provides:

- `FileEngine` and `EngineRequirement` contracts.
- structured findings and engine output types.
- shared merge helpers for item, list, scalar, and forbidden-glob requirements.

The crate performs no filesystem I/O. Concrete engines own file parsing,
validation, and reconciliation.

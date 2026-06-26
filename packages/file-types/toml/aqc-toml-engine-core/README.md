# aqc-toml-engine-core

Shared TOML IO and application helpers for AQC TOML file engines.

This crate contains format-level TOML mechanics that are common to concrete
file engines, including parsing, scalar rendering, list reconciliation, table
lookup, and mismatch reporting.

It intentionally does not define Cargo, Clippy, rustfmt, or policy semantics.
Concrete engines keep their domain-specific requirement types and use this
crate only after their own requirements have resolved into core AQC shapes.

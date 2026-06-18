# aqc-rust-syntax

`aqc-rust-syntax` extracts file-local Rust syntax facts from one source string.

The first fact surface is enum declarations: enum name, inline module path,
variant names, visibility, and declaration line. The crate does not read files,
walk crates, resolve modules, infer public API, or apply policy.

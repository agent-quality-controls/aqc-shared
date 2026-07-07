# Deny Production Shape Repair

## Goal

`aqc-deny-toml-engine` must write `deny.toml` values that `cargo deny` can parse.
Shackles deny policy must use ordered threshold semantics for `licenses.confidence-threshold`.

## Current Failure

- Generated `maximum-db-staleness = "90d"` is rejected by `cargo deny`.
- Generated `confidence-threshold = "0.8"` is rejected by `cargo deny`; it expects a float.
- `DenyConfidenceThreshold` implements `ScalarValue::compare_for_order` as `None`, so `ScalarAssertion::AtLeast` cannot work for the field.

## Approach

- In `aqc-deny-toml-engine`, change `DenyConfidenceThreshold` from plain text semantics to a typed ratio:
  - parse constructor input like `"0.8"`
  - store a canonical text representation and an integer ordering key
  - implement `ScalarValue::compare_for_order` using the ordering key
  - parse TOML floats and write TOML floats
- In `aqc-deny-toml-engine`, strengthen `DenyDuration` enough to reject the known wrong product default:
  - require constructor input to start with `P`
  - keep TOML representation as string
- Add engine tests:
  - `AtLeast(0.8)` accepts `confidence-threshold = 0.9`
  - `AtLeast(0.8)` repairs `confidence-threshold = 0.7`
  - expected output writes `confidence-threshold = 0.8`, not a string
  - `DenyDuration::new("90d")` fails and `DenyDuration::new("P90D")` passes
- Bump and publish `aqc-deny-toml-engine` to `0.1.1`.
- In Shackles, update `shakrs-deny-policy`:
  - use `ScalarAssertion::AtLeast(DenyConfidenceThreshold::new("0.8"), ...)`
  - use `DenyDuration::new("P90D")`
  - update fixtures and spec/verifier expectations
  - bump and publish `shakrs-deny-policy` to `0.1.1`
  - bump and publish `shakrs` to `0.1.4`
- Verify installed `shakrs` output with `cargo deny check`, not only `shakrs validate`.

## Files To Modify

- `packages/file-types/toml/aqc-deny-toml-engine/Cargo.toml`
- `packages/file-types/toml/aqc-deny-toml-engine/src/requirement/value.rs`
- `packages/file-types/toml/aqc-deny-toml-engine/src/requirement/value/value_impls/core.rs`
- `packages/file-types/toml/aqc-deny-toml-engine/src/reconcile/scalar_value.rs`
- `packages/file-types/toml/aqc-deny-toml-engine/tests/*`
- Shackles deny policy, specs, fixtures, app lockfile, and release worklog

## Key Decisions

- Core already has `ScalarAssertion::AtLeast`; do not add another assertion type.
- The field-specific fix belongs in the deny engine value type because only that type knows whether ordering is meaningful.
- Do not make `DenyDuration` a generic duration parser here; this repair only prevents the rejected `90d` form and uses the cargo-deny `P90D` form.

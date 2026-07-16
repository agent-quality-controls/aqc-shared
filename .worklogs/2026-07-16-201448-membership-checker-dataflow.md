# Membership Checker Data Flow

## Summary

Closed cross-crate helper bypasses in the permanent requirement-architecture checker while preserving valid adapter transfers through requirement accessors and tuple destructuring.

## Decisions Made

- Track adapter requirement values separately from membership values so calling requirement accessors is not mistaken for mutating membership.
- Accept membership returned by known local accessors and track membership-named tuple elements.
- Reject unknown helper calls used for adapter or engine membership fields, including wrapper-field access.
- Keep legacy G3 rule families disabled for this internal checker workspace; its permanent Specular, strict-Clippy, cargo-deny, and adversarial gates are the owning controls.

## Key Files For Context

- `tools/aqc-requirement-architecture/src/expression.rs`
- `tools/aqc-requirement-architecture/tests/fixtures/rejected/src/lib.rs`
- `tools/aqc-requirement-architecture/tests/architecture_tests.rs`

## Next Steps

- Pin this checker revision in Shackles CI.

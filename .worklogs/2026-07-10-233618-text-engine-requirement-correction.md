# Summary

Replaced hook-shaped text requirements with generic exact-content and contained-content requirements built from file-engine-core scalar and item primitives. Added merge, reconciliation, attribution, and unsupported-operation coverage and prepared `aqc-text-engine-core` 0.3.0.

# Decisions made

- Kept exact whole-file state as `ScalarAssertion<TextFileContents>`.
- Modeled required contained bytes as `ItemRequirements<TextFileContents>` without synthetic snippet identities.
- Rejected unsupported scalar and item operations after merge by inspecting collected assertions.
- Preserved each policy's diagnostic message when multiple policies require the same contents.
- Used and then retired the temporary Specular spec `c9bbc492...` with verifier `b4480af2...` after it passed and adversarial review converged.

# Key files for context

- `packages/file-types/text/aqc-text-engine-core/src/requirement/model.rs`
- `packages/file-types/text/aqc-text-engine-core/src/requirement/merge.rs`
- `packages/file-types/text/aqc-text-engine-core/src/reconcile.rs`
- `packages/file-types/text/aqc-text-engine-core/tests/reconcile_tests.rs`

# Next steps

- Publish `aqc-text-engine-core` 0.3.0 before releasing the dependent Shackles adapter, policy, and CLI.

# Allowed Item Membership

## Goal

- Represent closed collections that permit optional identities without requiring them.
- Keep `exact` as the complete set of identities that must be present.
- Apply one merge and reconciliation model to every file engine.

## Architecture

`ItemRequirements<Item>` gains an identity-only allowed collection:

```rust
pub struct ItemRequirements<Item> {
    pub required: Vec<ItemAssertion<Item>>,
    pub forbidden: Vec<ItemAssertion<Item>>,
    pub allowed: Option<AllowedItems<Item>>,
    pub exact: Option<ExactItems<Item>>,
}

pub type AllowedItems<Item> = (Vec<Item>, String);
```

- `required` means an item and its value must be present.
- `forbidden` means an identity must be absent.
- `allowed` means every present identity must belong to this set; listed identities remain optional.
- `exact` means the listed items and values must be present and every other identity must be absent.
- `allowed` mirrors `forbidden`: input items are mapped across adapter boundaries, and only their merge identities constrain presence after engine resolution.

## Merge Rules

- Multiple allowed sets intersect because every policy restriction must hold.
- A required identity outside any allowed set conflicts.
- An exact identity outside any allowed set conflicts.
- Each rejected constructive identity produces one conflict containing every excluding allowed contributor and every required or exact contributor.
- A forbidden identity inside an allowed set does not conflict because allowed does not require presence.
- Exact inputs continue to require identical identity sets.
- Exact and allowed may coexist when every exact identity is allowed.
- Resolved allowed state retains all source collections for attribution.

## Reconciliation

- Missing identities come from `required` and `exact`, never `allowed`.
- Unexpected identities are identities outside `exact` when exact exists; otherwise identities outside resolved `allowed`.
- Explicitly forbidden identities are reported only as forbidden, not again as unexpected.
- Initialization never writes an identity merely because it is allowed.
- `ItemRequirements::map` preserves required, forbidden, allowed identities, exact items, and messages.

## Consumers

- Every existing literal and adapter transfer is migrated to the new canonical shape with no alias or compatibility API.
- Deny table membership uses required for the baseline and allowed for the required baseline plus valid optional package-specific keys such as `bans.deny` and `bans.features`.
- No Deny-specific membership behavior is added to the Deny engine.

## Architecture Gate

- The permanent requirement checker treats `allowed` as canonical item-membership vocabulary.
- Adapter rules reject constructing, mutating, destructuring, replacing, or hiding policy-supplied allowed membership under the same rules as required, forbidden, and exact.
- Checker fixtures prove allowed transfer and map operations pass while adapter-authored allowed membership fails.

## Verification

- Core tests cover optional allowed items, intersection, required/allowed conflict, exact/allowed conflict, forbidden/allowed compatibility, attribution, initialization behavior, and mapping.
- Existing file-engine suites prove unchanged behavior when `allowed` is absent.
- Deny fixtures prove existing package-specific entries validate and a missing file initializes without those optional entries.
- Specular verifies the public shape, merge behavior, checker coverage, and downstream use.
- Full repository gates and adversarial review must pass before release.

## Files

- `packages/aqc-file-engine-core/src/merge/{model,item_model,items,forbidden_globs}.rs`, exports, and tests.
- All AQC `ItemRequirements` construction sites required by the canonical public shape.
- `tools/aqc-requirement-architecture` source and fixtures.
- `specs/explicit-setting-membership.spec.json` and its verifier.
- Downstream Shackles Deny policy, tests, fixtures, plan, and spec.

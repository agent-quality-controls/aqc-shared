//! Engine traits: the typed `FileEngine<ResolvedRequirements>` and the erased `Engine`
//! contract the runner dispatches over.

use crate::finding::Finding;
use crate::merge::ConflictEntry;
use crate::requirement::EngineRequirement;
use crate::types::{EngineOutput, Provenance};

/// A file engine: reconciles bytes-on-disk against typed declarative
/// requirements, returning both the bytes `init` would write and the
/// findings `validate` would report.
///
/// Engines are pure functions. They do not perform I/O. They never
/// return an error: catastrophic failures (parse failures, internal
/// invariant violations) surface as `Finding`s inside `EngineOutput`.
#[expect(
    clippy::module_name_repetitions,
    reason = "FileEngine is the canonical trait name; renaming it loses the connection to the file-engines abstraction in plans and call sites."
)]
pub trait FileEngine<ResolvedRequirements> {
    /// Apply `resolved_requirements` against `current_bytes`, returning what `init`
    /// would write and what `validate` would report.
    fn reconcile(
        current_bytes: Option<&[u8]>,
        resolved_requirements: &ResolvedRequirements,
    ) -> EngineOutput;
}

/// Erased engine contract the runner dispatches over.
///
/// The runner supplies the current bytes and the type-erased requirements for
/// one target. The engine only reconciles bytes; it does not know file paths.
pub trait Engine {
    /// Stable engine id (matches the crate's `ENGINE_ID`).
    fn id(&self) -> &'static str;
    /// Reconcile current file state against the requirements routed to this
    /// engine and target slot.
    #[allow(
        clippy::type_complexity,
        reason = "`&[(Provenance, Box<dyn EngineRequirement>)]` is the erased multi-requirement input the registry dispatches; a type alias would hide the contract."
    )]
    fn reconcile(
        &self,
        current_bytes: Option<&[u8]>,
        reqs: &[(Provenance, Box<dyn EngineRequirement>)],
    ) -> EngineOutput;
}

/// The shared erased-reconcile body every engine's `Engine::reconcile` is.
///
/// Downcast the routed requirements to `Requirements`; with none, echo the
/// current bytes back unchanged. Otherwise run the engine's merge phase, then
/// reconcile the merged desired-state against the supplied bytes.
#[allow(
    clippy::type_complexity,
    reason = "The erased multi-requirement input and merge/reconcile closures are the public dispatch shape."
)]
pub fn merged_reconcile<Requirements, ResolvedRequirements, Merge, Reconcile>(
    current_bytes: Option<&[u8]>,
    reqs: &[(Provenance, Box<dyn EngineRequirement>)],
    merge: Merge,
    reconcile_one: Reconcile,
) -> EngineOutput
where
    Requirements: EngineRequirement + Clone,
    Merge: Fn(Vec<(Provenance, Requirements)>) -> Result<ResolvedRequirements, Vec<ConflictEntry>>,
    Reconcile: Fn(Option<&[u8]>, &ResolvedRequirements) -> EngineOutput,
{
    let typed: Vec<(Provenance, Requirements)> = reqs
        .iter()
        .filter_map(|(prov, r)| {
            r.as_any()
                .downcast_ref::<Requirements>()
                .map(|req| (prov.clone(), req.clone()))
        })
        .collect();
    if typed.is_empty() {
        return EngineOutput {
            expected_bytes: current_bytes.unwrap_or_default().to_vec(),
            findings: Vec::new(),
        };
    }
    match merge(typed) {
        Ok(resolved) => reconcile_one(current_bytes, &resolved),
        Err(conflicts) => EngineOutput {
            expected_bytes: current_bytes.unwrap_or_default().to_vec(),
            findings: conflicts
                .into_iter()
                .map(|conflict| Finding::ConflictingRequirements {
                    key: conflict.key,
                    reason: conflict.reason,
                    contributors: conflict
                        .contributors
                        .into_iter()
                        .map(|(provenance, value)| (provenance.policy, value))
                        .collect(),
                })
                .collect(),
        },
    }
}

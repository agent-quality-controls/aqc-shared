//! Engine traits: the typed `FileEngine<Req>` and the erased `Engine`
//! contract the runner dispatches over.

use std::path::{Path, PathBuf};

use crate::finding::Finding;
use crate::merge::ConflictEntry;
use crate::requirement::EngineRequirement;
use crate::types::{EngineFileState, EngineOutput, Provenance};

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
pub trait FileEngine<Req> {
    /// Apply `requirement` against `current_bytes`, returning what `init`
    /// would write and what `validate` would report.
    fn reconcile(current_bytes: Option<&[u8]>, requirement: &Req) -> EngineOutput;
}

/// Erased engine contract the runner dispatches over.
///
/// Each engine knows the files it owns and reconciles the type-erased
/// requirements routed to it (all carrying this engine's id). The result is
/// an `EngineOutput` containing one `EngineFileOutput` per touched file.
/// Object-safe so the runner registry can hold `Box<dyn Engine>`; the typed
/// `FileEngine` is what each engine calls internally.
pub trait Engine {
    /// Stable engine id (matches the crate's `ENGINE_ID`).
    fn id(&self) -> &'static str;
    /// The files this engine owns for the routed requirements.
    fn target_paths(
        &self,
        workspace_root: &Path,
        reqs: &[(Provenance, Box<dyn EngineRequirement>)],
    ) -> Vec<PathBuf>;
    /// Reconcile current file state against the requirements routed to this
    /// engine. The runner groups requirements by engine id, so every element
    /// downcasts to this engine's requirement type.
    #[expect(
        clippy::type_complexity,
        reason = "`&[(Provenance, Box<dyn EngineRequirement>)]` is the erased multi-requirement input the registry dispatches; a type alias would hide the contract."
    )]
    fn reconcile(
        &self,
        target_root: &Path,
        current: &[EngineFileState],
        reqs: &[(Provenance, Box<dyn EngineRequirement>)],
    ) -> EngineOutput;
}

/// The shared erased-reconcile body every engine's `Engine::reconcile` is.
///
/// Downcast the routed requirements to `Req`; with none, echo `current` back
/// unchanged. Otherwise run the engine's `merge` phase (pure), reconcile the
/// merged desired-state against disk via `reconcile_one`, then map each merge
/// `ConflictEntry` to a `Finding::ConflictingRequirements` keyed by `subject` (the
/// engine owns the file name). Only the requirement type, `subject`, and the
/// two functions differ between engines; the dance is identical, so it lives
/// here once.
#[expect(
    clippy::type_complexity,
    reason = "`Fn(Vec<(Provenance, Req)>) -> (Resolved, Vec<ConflictEntry>)` is the merge phase's signature as data; aliasing it would hide the raw-to-resolved contract."
)]
pub fn merged_reconcile<Req, Resolved, M, F>(
    current: &[EngineFileState],
    target_path: PathBuf,
    reqs: &[(Provenance, Box<dyn EngineRequirement>)],
    subject: &str,
    merge: M,
    reconcile_one: F,
) -> EngineOutput
where
    Req: EngineRequirement + Clone,
    M: Fn(Vec<(Provenance, Req)>) -> (Resolved, Vec<ConflictEntry>),
    F: Fn(Option<&[u8]>, &Resolved) -> EngineOutput,
{
    let typed: Vec<(Provenance, Req)> = reqs
        .iter()
        .filter_map(|(prov, r)| {
            r.as_any()
                .downcast_ref::<Req>()
                .map(|req| (prov.clone(), req.clone()))
        })
        .collect();
    let current_bytes = current
        .iter()
        .find(|state| state.path == target_path)
        .and_then(|state| state.bytes.as_deref());
    if typed.is_empty() {
        return EngineOutput::single(
            current_bytes.map(<[u8]>::to_vec).unwrap_or_default(),
            Vec::new(),
        )
        .with_single_path(target_path);
    }
    let (merged, conflicts) = merge(typed);
    let mut out = reconcile_one(current_bytes, &merged).with_single_path(target_path);
    for entry in conflicts {
        let finding = Finding::ConflictingRequirements {
            subject: subject.to_owned(),
            key: entry.key,
            contributors: entry
                .contributors
                .into_iter()
                .map(|(prov, value)| (prov.policy, value))
                .collect(),
            reason: entry.reason,
        };
        out.findings.push(finding);
    }
    out
}

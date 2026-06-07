//! Engine traits: the typed `FileEngine<Req>` and the erased `Engine`
//! contract the runner dispatches over.

use std::path::{Path, PathBuf};

use crate::finding::Finding;
use crate::merge::ConflictEntry;
use crate::requirement::EngineRequirement;
use crate::types::EngineOutput;

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
/// Each engine knows the file it owns and reconciles the type-erased
/// requirements routed to it (all carrying this engine's id). Object-safe so
/// the runner registry can hold `Box<dyn Engine>`; the typed `FileEngine` is
/// what each engine calls internally.
pub trait Engine {
    /// Stable engine id (matches the crate's `ENGINE_ID`).
    fn id(&self) -> &'static str;
    /// The workspace-relative file this engine owns (e.g. `Cargo.toml`).
    fn target_path(&self, workspace_root: &Path) -> PathBuf;
    /// Reconcile `current` bytes against the requirements routed to this
    /// engine. The runner groups requirements by engine id, so every element
    /// downcasts to this engine's requirement type.
    #[expect(
        clippy::type_complexity,
        reason = "`&[Box<dyn EngineRequirement>]` is the erased multi-requirement input the registry dispatches; a type alias would hide the contract."
    )]
    fn reconcile(
        &self,
        current: Option<&[u8]>,
        reqs: &[Box<dyn EngineRequirement>],
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
    reason = "`Fn(&[&Req]) -> (Req, Vec<ConflictEntry>)` is the merge phase's signature as data; aliasing it would hide the (resolved, conflicts) contract."
)]
pub fn merged_reconcile<Req, M, F>(
    current: Option<&[u8]>,
    reqs: &[Box<dyn EngineRequirement>],
    subject: &str,
    merge: M,
    reconcile_one: F,
) -> EngineOutput
where
    Req: EngineRequirement,
    M: Fn(&[&Req]) -> (Req, Vec<ConflictEntry>),
    F: Fn(Option<&[u8]>, &Req) -> EngineOutput,
{
    let typed: Vec<&Req> = reqs
        .iter()
        .filter_map(|r| r.as_any().downcast_ref::<Req>())
        .collect();
    if typed.is_empty() {
        return EngineOutput {
            expected_bytes: current.map(<[u8]>::to_vec).unwrap_or_default(),
            findings: Vec::new(),
        };
    }
    let (merged, conflicts) = merge(&typed);
    let mut out = reconcile_one(current, &merged);
    for entry in conflicts {
        out.findings.push(Finding::ConflictingRequirements {
            subject: subject.to_owned(),
            key: entry.key,
            contributors: entry
                .contributors
                .into_iter()
                .map(|(prov, value)| (prov.policy, value))
                .collect(),
            reason: entry.reason,
        });
    }
    out
}

//! Object-safe trait every concrete engine-requirement type implements.
//!
//! Adapters return provenance-tagged boxed engine requirements. The broker
//! matches [`EngineRequirement::engine_id`] against a runtime registry to find
//! the right reconciler, then downcasts via [`EngineRequirement::as_any`] to
//! the concrete `Req` type before calling that engine's `reconcile`.
//!
//! See the slice plan at
//! `guardrail3/.plans/g3v2-architecture/2026-05-27-185042-clippy-policy-and-adapter-slice.md`.

#![allow(
    clippy::disallowed_types,
    reason = "`Any` is required here as the dynamic-dispatch escape hatch the broker uses to downcast `Box<dyn EngineRequirement>` to each engine's concrete `Req` type; a closed enum was rejected because it would force every adapter to transitively depend on every engine crate (~20 planned)."
)]

use core::any::Any;
use core::fmt::Debug;

/// Marker contract for engine-side `Req` types crossing the adapter -> broker
/// boundary.
///
/// Implementors must:
///
/// 1. Return a stable `engine_id` matching the producing engine crate's
///    `Cargo.toml` `[package].name`.
/// 2. Return `self` from `as_any` to enable broker-side downcast to the
///    concrete `Req` type.
#[expect(
    clippy::module_name_repetitions,
    reason = "`EngineRequirement` is the canonical trait name used by the architecture plan and across adapter + broker call sites; renaming it loses that anchor."
)]
pub trait EngineRequirement: Any + Debug + Send + Sync {
    /// Stable identifier matching the producing engine crate's name.
    fn engine_id(&self) -> &'static str;

    /// Downcast escape hatch. Concrete impls return `self`.
    fn as_any(&self) -> &dyn Any;
}

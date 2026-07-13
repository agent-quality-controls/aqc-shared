//! Integration tests for the `EngineRequirement` impl on `ClippyTomlRequirements`.

use aqc_clippy_toml_engine::{ClippyTomlRequirements, ResolvedClippyTomlRequirements};
use aqc_file_engine_core::EngineRequirement;
use aqc_toml_engine_core as _;
use globset as _;
use toml_edit as _;

#[test]
fn impl_engine_requirement_id_matches_crate_name() {
    let req = ClippyTomlRequirements::default();
    assert_eq!(req.engine_id(), "aqc-clippy-toml-engine");
}

#[test]
fn impl_engine_requirement_downcast_roundtrip() {
    let req: Box<dyn EngineRequirement> = Box::new(ClippyTomlRequirements::default());
    let downcast = req.as_any().downcast_ref::<ClippyTomlRequirements>();
    assert!(downcast.is_some());
}

#[test]
fn resolved_root_exposes_every_field_through_borrowed_getters() {
    let resolved = ResolvedClippyTomlRequirements::default();

    let _ = resolved.msrv();
    let _ = resolved.thresholds();
    let _ = resolved.disallowed_methods();
    let _ = resolved.forbidden_disallowed_method_path_globs();
    let _ = resolved.disallowed_types();
    let _ = resolved.forbidden_disallowed_type_path_globs();
    let _ = resolved.disallowed_macros();
    let _ = resolved.forbidden_disallowed_macro_path_globs();
    let _ = resolved.bools();
    let _ = resolved.enums();
}

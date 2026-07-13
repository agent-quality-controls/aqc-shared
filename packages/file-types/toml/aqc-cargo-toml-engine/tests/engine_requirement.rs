//! Integration tests for the `EngineRequirement` impl on `CargoTomlRequirements`.

use aqc_cargo_toml_engine::{CargoTomlRequirements, ResolvedCargoTomlRequirements};
use aqc_file_engine_core::EngineRequirement;
use aqc_toml_engine_core as _;
use globset as _;
use toml_edit as _;

#[test]
fn impl_engine_requirement_id_matches_crate_name() {
    let req = CargoTomlRequirements::default();
    assert_eq!(req.engine_id(), "aqc-cargo-toml-engine");
}

#[test]
fn impl_engine_requirement_downcast_roundtrip() {
    let req: Box<dyn EngineRequirement> = Box::new(CargoTomlRequirements::default());
    let downcast = req.as_any().downcast_ref::<CargoTomlRequirements>();
    assert!(downcast.is_some());
}

#[test]
fn resolved_root_exposes_every_field_through_borrowed_getters() {
    let resolved = ResolvedCargoTomlRequirements::default();

    let _ = resolved.package_lints();
    let _ = resolved.package_lint_tables();
    let _ = resolved.workspace_lints();
    let _ = resolved.package_fields();
    let _ = resolved.workspace_package_fields();
    let _ = resolved.workspace_fields();
    let _ = resolved.section_presence();
    let _ = resolved.dependencies();
    let _ = resolved.forbidden_dependency_package_globs();
    let _ = resolved.workspace_dependencies();
    let _ = resolved.forbidden_workspace_dependency_package_globs();
    let _ = resolved.features();
    let _ = resolved.profiles();
    let _ = resolved.targets();
    let _ = resolved.patch();
    let _ = resolved.forbidden_patch_dependency_package_globs();
}

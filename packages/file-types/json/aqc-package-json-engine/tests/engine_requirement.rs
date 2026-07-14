use aqc_file_engine_core::EngineRequirement;
use aqc_json_engine_core as _;
use aqc_package_json_engine::{ENGINE_ID, PackageJsonRequirements, PackageManagerOnFail};
use schemars as _;
use serde as _;

#[test]
fn package_json_requirement_reports_engine_id() {
    assert_eq!(
        PackageJsonRequirements::default().engine_id(),
        ENGINE_ID,
        "The requirement must route to the package JSON engine."
    );
}

#[test]
fn package_manager_on_fail_uses_closed_wire_values() {
    let cases = [
        (PackageManagerOnFail::Download, "download"),
        (PackageManagerOnFail::Error, "error"),
        (PackageManagerOnFail::Warn, "warn"),
        (PackageManagerOnFail::Ignore, "ignore"),
    ];
    for (value, expected) in cases {
        assert_eq!(
            value.as_str(),
            expected,
            "Each variant must have its package.json spelling."
        );
        assert_eq!(
            PackageManagerOnFail::parse(expected),
            Some(value),
            "Each wire value must parse."
        );
    }
    assert_eq!(
        PackageManagerOnFail::parse("fatal"),
        None,
        "Unknown values must stay closed."
    );
}

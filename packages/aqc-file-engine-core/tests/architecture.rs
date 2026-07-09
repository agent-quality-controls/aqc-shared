use aqc_file_engine_core as _;
use serde as _;

const ENGINE_ENTRY_POINTS: &[(&str, &str)] = &[
    (
        "engine entry point aqc-file-engine-core",
        include_str!("../src/engine.rs"),
    ),
    (
        "engine entry point aqc-text-engine-core",
        include_str!("../../file-types/text/aqc-text-engine-core/src/engine.rs"),
    ),
    (
        "engine entry point aqc-cargo-toml-engine",
        include_str!("../../file-types/toml/aqc-cargo-toml-engine/src/engine.rs"),
    ),
    (
        "engine entry point aqc-clippy-toml-engine",
        include_str!("../../file-types/toml/aqc-clippy-toml-engine/src/engine.rs"),
    ),
    (
        "engine entry point aqc-deny-toml-engine",
        include_str!("../../file-types/toml/aqc-deny-toml-engine/src/engine.rs"),
    ),
    (
        "engine entry point aqc-rust-toolchain-toml-engine",
        include_str!("../../file-types/toml/aqc-rust-toolchain-toml-engine/src/engine.rs"),
    ),
    (
        "engine entry point aqc-rustfmt-toml-engine",
        include_str!("../../file-types/toml/aqc-rustfmt-toml-engine/src/engine.rs"),
    ),
];

const FORBIDDEN_ROUTING_TOKENS: &[&str] = &[
    "target_path",
    "target_paths",
    "EngineFileState",
    "EngineFileOutput",
    "workspace_root",
    "target_root",
];

#[test]
fn engine_entry_points_do_not_own_file_routing() {
    for (label, source) in ENGINE_ENTRY_POINTS {
        for token in FORBIDDEN_ROUTING_TOKENS {
            assert!(
                !source.contains(token),
                "{label} must not contain routing token {token}"
            );
        }
    }
}

#[test]
fn conflicting_requirements_do_not_carry_report_subject() {
    let finding_source = include_str!("../src/finding.rs");
    let start = finding_source
        .find("ConflictingRequirements")
        .unwrap_or(finding_source.len());
    let block = finding_source.get(start..).unwrap_or_default();
    let end = block.find("InternalError").unwrap_or(block.len());
    let block = block.get(..end).unwrap_or_default();
    assert!(
        !block.contains("subject"),
        "ConflictingRequirements must not carry a report subject"
    );
}

#[test]
fn allows_path_shaped_requirement_models() {
    let model_source = include_str!(
        "../../file-types/toml/aqc-rust-toolchain-toml-engine/src/requirement/model.rs"
    );
    assert!(
        model_source.contains("ToolchainPath"),
        "allows path-shaped requirement models when the file format contains path values"
    );
}

use aqc_toml_engine_core as _;
use globset as _;
use toml_edit as _;

use aqc_clippy_toml_engine::{
    ClippyPathGlob, ClippyTomlEngine, ClippyTomlRequirements, DisallowedEntry,
};
use aqc_file_engine_core::{
    Engine, EngineOutput, EngineRequirement, Finding, ForbiddenGlobRequirements, ItemRequirements,
    Provenance,
};

type ClippyRequirementInput = (Provenance, ClippyTomlRequirements);
type PathGlobInput<'a> = (&'a str, &'a str);
type DisallowedRequirementInput = (DisallowedEntry, String);

fn prov(policy: &str) -> Provenance {
    Provenance {
        policy: policy.to_owned(),
    }
}

fn clippy_findings(reqs: Vec<ClippyRequirementInput>) -> Vec<Finding> {
    clippy_output(Some(b""), reqs).findings
}

fn clippy_output(bytes: Option<&[u8]>, reqs: Vec<ClippyRequirementInput>) -> EngineOutput {
    let reqs = reqs
        .into_iter()
        .map(|(p, r)| {
            let requirement: Box<dyn EngineRequirement> = Box::new(r);
            (p, requirement)
        })
        .collect::<Vec<_>>();
    ClippyTomlEngine.reconcile(bytes, &reqs)
}

fn forbid(path: &str) -> DisallowedEntry {
    DisallowedEntry {
        path: path.to_owned(),
        message: "forbid".to_owned(),
    }
}

fn path_glob(glob: &str) -> ClippyPathGlob {
    ClippyPathGlob {
        glob: glob.to_owned(),
    }
}

fn path_globs(globs: Vec<PathGlobInput<'_>>) -> ForbiddenGlobRequirements<ClippyPathGlob> {
    ForbiddenGlobRequirements {
        globs: globs
            .into_iter()
            .map(|(glob, msg)| (path_glob(glob), msg.to_owned()))
            .collect(),
    }
}

const fn disallowed_items(
    required: Vec<DisallowedRequirementInput>,
    forbidden: Vec<DisallowedRequirementInput>,
    closed: Option<String>,
) -> ItemRequirements<DisallowedEntry> {
    ItemRequirements {
        required,
        forbidden,
        closed,
    }
}

#[test]
fn forbidden_disallowed_path_globs_remove_matching_entries() {
    let req = ClippyTomlRequirements {
        forbidden_disallowed_method_path_globs: path_globs(vec![("std::env::*", "no env methods")]),
        forbidden_disallowed_type_path_globs: path_globs(vec![("std::cell::*", "no cell types")]),
        forbidden_disallowed_macro_path_globs: path_globs(vec![("std::*", "no std macros")]),
        ..ClippyTomlRequirements::default()
    };
    let output = clippy_output(
        Some(
            br#"disallowed-methods = ["std::env::set_var", "std::fs::read_to_string"]
disallowed-types = ["std::cell::RefCell", "std::sync::Arc"]
disallowed-macros = ["std::println", "tracing::info"]
"#,
        ),
        vec![(prov("p1"), req)],
    );
    let text = String::from_utf8(output.expected_bytes)
        .expect("engine output should remain valid UTF-8 TOML text");
    assert!(!text.contains("std::env::set_var"));
    assert!(text.contains("std::fs::read_to_string"));
    assert!(!text.contains("std::cell::RefCell"));
    assert!(text.contains("std::sync::Arc"));
    assert!(!text.contains("std::println"));
    assert!(text.contains("tracing::info"));
    assert_eq!(output.findings.len(), 3);
}

#[test]
fn forbidden_disallowed_path_glob_conflict_does_not_remove_required_entry() {
    let req = ClippyTomlRequirements {
        disallowed_methods: disallowed_items(
            vec![(forbid("std::env::set_var"), "keep env".to_owned())],
            Vec::new(),
            None,
        ),
        forbidden_disallowed_method_path_globs: path_globs(vec![("std::env::*", "no env methods")]),
        ..ClippyTomlRequirements::default()
    };
    let output = clippy_output(
        Some(br#"disallowed-methods = ["std::env::set_var"]"#),
        vec![(prov("p1"), req)],
    );
    let text = String::from_utf8(output.expected_bytes)
        .expect("engine output should remain valid UTF-8 TOML text");
    assert!(text.contains("std::env::set_var"));
    assert!(output.findings.iter().any(|finding| {
        matches!(
            finding,
            Finding::ConflictingRequirements { reason, .. }
                if reason == "disallowed-method-path-glob-forbids-required-path"
        )
    }));
}

#[test]
fn invalid_forbidden_disallowed_path_glob_reports_invalid_requirements() {
    let req = ClippyTomlRequirements {
        forbidden_disallowed_method_path_globs: path_globs(vec![("[", "bad glob")]),
        ..ClippyTomlRequirements::default()
    };
    let output = clippy_output(
        Some(br#"disallowed-methods = ["std::env::set_var"]"#),
        vec![(prov("p1"), req)],
    );
    assert!(output.findings.iter().any(|finding| {
        matches!(
            finding,
            Finding::InvalidRequirements { message, .. }
                if message.contains("invalid path glob")
        )
    }));
}

#[test]
fn clippy_disallowed_required_and_forbidden_different_keys_compose() {
    let table = disallowed_items(
        vec![(forbid("std::mem::forget"), "forbid".to_owned())],
        vec![(forbid("std::mem::transmute"), "no forbid".to_owned())],
        None,
    );
    let req = ClippyTomlRequirements {
        disallowed_methods: table,
        ..ClippyTomlRequirements::default()
    };
    let (_, findings) = ClippyTomlRequirements::merge(vec![(prov("p1"), req)]);
    assert!(findings.is_empty());
}

#[test]
fn clippy_disallowed_required_and_forbidden_same_key_conflict() {
    let table = disallowed_items(
        vec![(forbid("std::mem::forget"), "forbid".to_owned())],
        vec![(forbid("std::mem::forget"), "remove".to_owned())],
        None,
    );
    let req = ClippyTomlRequirements {
        disallowed_methods: table,
        ..ClippyTomlRequirements::default()
    };
    let findings = clippy_findings(vec![(prov("p1"), req)]);
    assert!(matches!(
        findings
            .iter()
            .find(|f| matches!(f, Finding::ConflictingRequirements { .. })),
        Some(Finding::ConflictingRequirements { .. })
    ));
}

#[test]
fn clippy_disallowed_path_conflict_uses_item_identity() {
    let req = ClippyTomlRequirements {
        disallowed_methods: disallowed_items(
            vec![(forbid("std::env::set_var"), "forbid".to_owned())],
            vec![(forbid("std::env::set_var"), "remove".to_owned())],
            None,
        ),
        ..ClippyTomlRequirements::default()
    };
    let findings = clippy_findings(vec![(prov("p1"), req)]);
    assert!(findings.iter().any(|finding| {
        matches!(
            finding,
            Finding::ConflictingRequirements { key, .. }
                if key.contains("std::env::set_var")
        )
    }));
}

#[test]
fn init_writes_clippy_disallowed_array_item() {
    let req = ClippyTomlRequirements {
        disallowed_methods: disallowed_items(
            vec![(forbid("std::env::set_var"), "forbid".to_owned())],
            Vec::new(),
            None,
        ),
        ..ClippyTomlRequirements::default()
    };
    let output = clippy_output(None, vec![(prov("p1"), req)]);
    let text = String::from_utf8(output.expected_bytes)
        .expect("engine output should remain valid UTF-8 TOML text");
    assert!(text.contains("disallowed-methods"));
    assert!(text.contains("path = \"std::env::set_var\""));
    assert!(text.contains("reason = \"forbid\""));
}

#[test]
fn required_clippy_disallowed_updates_missing_reason() {
    let req = ClippyTomlRequirements {
        disallowed_methods: disallowed_items(
            vec![(forbid("std::env::set_var"), "forbid".to_owned())],
            Vec::new(),
            None,
        ),
        ..ClippyTomlRequirements::default()
    };
    let output = clippy_output(
        Some(b"disallowed-methods = [\"std::env::set_var\"]\n"),
        vec![(prov("p1"), req)],
    );
    let text = String::from_utf8(output.expected_bytes)
        .expect("engine output should remain valid UTF-8 TOML text");
    assert!(text.contains("reason = \"forbid\""));
    assert_eq!(output.findings.len(), 1);
}

#[test]
fn required_clippy_disallowed_handles_non_array_without_panic() {
    let req = ClippyTomlRequirements {
        disallowed_methods: disallowed_items(
            vec![(forbid("std::env::set_var"), "forbid".to_owned())],
            Vec::new(),
            None,
        ),
        ..ClippyTomlRequirements::default()
    };
    let output = clippy_output(
        Some(b"disallowed-methods = \"bad\"\n"),
        vec![(prov("p1"), req)],
    );
    let text = String::from_utf8(output.expected_bytes)
        .expect("engine output should remain valid UTF-8 TOML text");
    assert!(text.contains("path = \"std::env::set_var\""));
    assert!(!output.findings.is_empty());
}

#[test]
fn forbidden_clippy_disallowed_removes_duplicate_entries() {
    let req = ClippyTomlRequirements {
        disallowed_methods: disallowed_items(
            Vec::new(),
            vec![(forbid("std::env::set_var"), "remove".to_owned())],
            None,
        ),
        ..ClippyTomlRequirements::default()
    };
    let output = clippy_output(
        Some(
            b"disallowed-methods = [\"std::env::set_var\", { path = \"std::env::set_var\", reason = \"x\" }]\n",
        ),
        vec![(prov("p1"), req)],
    );
    let text = String::from_utf8(output.expected_bytes)
        .expect("engine output should remain valid UTF-8 TOML text");
    assert!(!text.contains("std::env::set_var"));
    assert_eq!(output.findings.len(), 2);
}

#[test]
fn clippy_forbidden_absent_does_not_create_empty_array() {
    let req = ClippyTomlRequirements {
        disallowed_methods: disallowed_items(
            Vec::new(),
            vec![(forbid("std::mem::forget"), "remove".to_owned())],
            None,
        ),
        ..ClippyTomlRequirements::default()
    };
    let output = clippy_output(Some(b""), vec![(prov("p1"), req)]);
    assert!(output.findings.is_empty());
    assert_eq!(
        String::from_utf8(output.expected_bytes)
            .expect("engine output should remain valid UTF-8 TOML text"),
        ""
    );
}

#[test]
fn clippy_closed_absent_does_not_create_empty_array() {
    let req = ClippyTomlRequirements {
        disallowed_methods: disallowed_items(Vec::new(), Vec::new(), Some("closed".to_owned())),
        ..ClippyTomlRequirements::default()
    };
    let output = clippy_output(Some(b""), vec![(prov("p1"), req)]);
    assert!(output.findings.is_empty());
    assert_eq!(
        String::from_utf8(output.expected_bytes)
            .expect("engine output should remain valid UTF-8 TOML text"),
        ""
    );
}

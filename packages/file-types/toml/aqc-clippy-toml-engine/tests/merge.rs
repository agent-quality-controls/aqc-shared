use std::collections::{BTreeMap, BTreeSet};

use globset as _;
use toml_edit as _;

use aqc_clippy_toml_engine::{
    BanEntry, BoolAssertion, ClippyPathGlob, ClippyTomlEngine, ClippyTomlRequirements,
    MsrvAssertion, NumericAssertion, StringAssertion,
};
use aqc_file_engine_core::{
    Engine, EngineOutput, EngineRequirement, Finding, ForbiddenGlobRequirements, ItemRequirements,
    Provenance,
};

fn prov(policy: &str) -> Provenance {
    Provenance {
        policy: policy.to_owned(),
    }
}

fn clippy_findings(reqs: Vec<(Provenance, ClippyTomlRequirements)>) -> Vec<Finding> {
    clippy_output(Some(b""), reqs).findings
}

fn clippy_output(
    bytes: Option<&[u8]>,
    reqs: Vec<(Provenance, ClippyTomlRequirements)>,
) -> EngineOutput {
    let reqs = reqs
        .into_iter()
        .map(|(p, r)| (p, Box::new(r) as Box<dyn EngineRequirement>))
        .collect::<Vec<_>>();
    ClippyTomlEngine.reconcile(bytes, &reqs)
}

fn ban(path: &str) -> BanEntry {
    BanEntry {
        path: path.to_owned(),
        message: "ban".to_owned(),
    }
}

fn path_glob(glob: &str) -> ClippyPathGlob {
    ClippyPathGlob {
        glob: glob.to_owned(),
    }
}

fn path_globs(globs: Vec<(&str, &str)>) -> ForbiddenGlobRequirements<ClippyPathGlob> {
    ForbiddenGlobRequirements {
        globs: globs
            .into_iter()
            .map(|(glob, msg)| (path_glob(glob), msg.to_owned()))
            .collect(),
    }
}

fn ban_items(
    required: Vec<(BanEntry, String)>,
    banned: Vec<(BanEntry, String)>,
    closed: Option<String>,
) -> ItemRequirements<BanEntry> {
    ItemRequirements {
        required,
        banned,
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
        disallowed_methods: ban_items(
            vec![(ban("std::env::set_var"), "keep env".to_owned())],
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
fn clippy_bans_required_and_banned_different_keys_compose() {
    let table = ban_items(
        vec![(ban("std::mem::forget"), "ban".to_owned())],
        vec![(ban("std::mem::transmute"), "no ban".to_owned())],
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
fn clippy_bans_required_and_banned_same_key_conflict() {
    let table = ban_items(
        vec![(ban("std::mem::forget"), "ban".to_owned())],
        vec![(ban("std::mem::forget"), "remove".to_owned())],
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
fn clippy_ban_path_conflict_uses_item_identity() {
    let req = ClippyTomlRequirements {
        disallowed_methods: ban_items(
            vec![(ban("std::env::set_var"), "ban".to_owned())],
            vec![(ban("std::env::set_var"), "remove".to_owned())],
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
fn init_writes_clippy_ban_array_item() {
    let req = ClippyTomlRequirements {
        disallowed_methods: ban_items(
            vec![(ban("std::env::set_var"), "ban".to_owned())],
            Vec::new(),
            None,
        ),
        ..ClippyTomlRequirements::default()
    };
    let output = clippy_output(None, vec![(prov("p1"), req)]);
    let text = String::from_utf8(output.expected_bytes).expect("utf8");
    assert!(text.contains("disallowed-methods"));
    assert!(text.contains("path = \"std::env::set_var\""));
    assert!(text.contains("reason = \"ban\""));
}

#[test]
fn required_clippy_ban_updates_missing_reason() {
    let req = ClippyTomlRequirements {
        disallowed_methods: ban_items(
            vec![(ban("std::env::set_var"), "ban".to_owned())],
            Vec::new(),
            None,
        ),
        ..ClippyTomlRequirements::default()
    };
    let output = clippy_output(
        Some(b"disallowed-methods = [\"std::env::set_var\"]\n"),
        vec![(prov("p1"), req)],
    );
    let text = String::from_utf8(output.expected_bytes).expect("utf8");
    assert!(text.contains("reason = \"ban\""));
    assert_eq!(output.findings.len(), 1);
}

#[test]
fn required_clippy_ban_handles_non_array_without_panic() {
    let req = ClippyTomlRequirements {
        disallowed_methods: ban_items(
            vec![(ban("std::env::set_var"), "ban".to_owned())],
            Vec::new(),
            None,
        ),
        ..ClippyTomlRequirements::default()
    };
    let output = clippy_output(
        Some(b"disallowed-methods = \"bad\"\n"),
        vec![(prov("p1"), req)],
    );
    let text = String::from_utf8(output.expected_bytes).expect("utf8");
    assert!(text.contains("path = \"std::env::set_var\""));
    assert!(!output.findings.is_empty());
}

#[test]
fn banned_clippy_ban_removes_duplicate_entries() {
    let req = ClippyTomlRequirements {
        disallowed_methods: ban_items(
            Vec::new(),
            vec![(ban("std::env::set_var"), "remove".to_owned())],
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
    let text = String::from_utf8(output.expected_bytes).expect("utf8");
    assert!(!text.contains("std::env::set_var"));
    assert_eq!(output.findings.len(), 2);
}

#[test]
fn clippy_banned_absent_does_not_create_empty_array() {
    let req = ClippyTomlRequirements {
        disallowed_methods: ban_items(
            Vec::new(),
            vec![(ban("std::mem::forget"), "remove".to_owned())],
            None,
        ),
        ..ClippyTomlRequirements::default()
    };
    let output = clippy_output(Some(b""), vec![(prov("p1"), req)]);
    assert!(output.findings.is_empty());
    assert_eq!(String::from_utf8(output.expected_bytes).expect("utf8"), "");
}

#[test]
fn clippy_closed_absent_does_not_create_empty_array() {
    let req = ClippyTomlRequirements {
        disallowed_methods: ban_items(Vec::new(), Vec::new(), Some("closed".to_owned())),
        ..ClippyTomlRequirements::default()
    };
    let output = clippy_output(Some(b""), vec![(prov("p1"), req)]);
    assert!(output.findings.is_empty());
    assert_eq!(String::from_utf8(output.expected_bytes).expect("utf8"), "");
}

#[test]
fn clippy_thresholds_compose_per_key() {
    let left = ClippyTomlRequirements {
        thresholds: BTreeMap::from([(
            "too-many-lines-threshold".to_owned(),
            NumericAssertion::AtMost(100, "limit".to_owned()),
        )]),
        ..ClippyTomlRequirements::default()
    };
    let right = ClippyTomlRequirements {
        thresholds: BTreeMap::from([(
            "too-many-lines-threshold".to_owned(),
            NumericAssertion::AtMost(80, "stricter".to_owned()),
        )]),
        ..ClippyTomlRequirements::default()
    };
    let (merged, conflicts) =
        ClippyTomlRequirements::merge(vec![(prov("p1"), left), (prov("p2"), right)]);
    let NumericAssertion::AtMost(value, _) = merged.thresholds["too-many-lines-threshold"].merged
    else {
        panic!("expected NumericAssertion");
    };
    assert!(conflicts.is_empty());
    assert_eq!(value, 80);
}

#[test]
fn clippy_threshold_range_bounds_compose() {
    let left = ClippyTomlRequirements {
        thresholds: BTreeMap::from([(
            "too-many-lines-threshold".to_owned(),
            NumericAssertion::AtLeast(40, "floor".to_owned()),
        )]),
        ..ClippyTomlRequirements::default()
    };
    let right = ClippyTomlRequirements {
        thresholds: BTreeMap::from([(
            "too-many-lines-threshold".to_owned(),
            NumericAssertion::AtMost(80, "ceiling".to_owned()),
        )]),
        ..ClippyTomlRequirements::default()
    };
    let (merged, conflicts) =
        ClippyTomlRequirements::merge(vec![(prov("p1"), left), (prov("p2"), right)]);
    let NumericAssertion::Range(min, max, _) = merged.thresholds["too-many-lines-threshold"].merged
    else {
        panic!("expected NumericAssertion::Range");
    };
    assert!(conflicts.is_empty());
    assert_eq!((min, max), (40, 80));
}

#[test]
fn clippy_msrv_keeps_strongest_floor() {
    let left = ClippyTomlRequirements {
        msrv: Some(MsrvAssertion::AtLeast("1.80".to_owned(), "old".to_owned())),
        ..ClippyTomlRequirements::default()
    };
    let right = ClippyTomlRequirements {
        msrv: Some(MsrvAssertion::AtLeast("1.85".to_owned(), "new".to_owned())),
        ..ClippyTomlRequirements::default()
    };
    let (merged, conflicts) =
        ClippyTomlRequirements::merge(vec![(prov("p1"), left), (prov("p2"), right)]);
    let MsrvAssertion::AtLeast(version, _) = &merged.msrv.expect("msrv").merged else {
        panic!("expected AtLeast");
    };
    assert!(conflicts.is_empty());
    assert_eq!(version, "1.85");
}

#[test]
fn clippy_scalar_implication_cases_compose() {
    let mut allowed = BTreeSet::new();
    let _ = allowed.insert("warn".to_owned());
    let left = ClippyTomlRequirements {
        enums: BTreeMap::from([(
            "disallowed-names".to_owned(),
            StringAssertion::Equals("warn".to_owned(), "equals".to_owned()),
        )]),
        ..ClippyTomlRequirements::default()
    };
    let right = ClippyTomlRequirements {
        enums: BTreeMap::from([(
            "disallowed-names".to_owned(),
            StringAssertion::OneOf(allowed, "one".to_owned()),
        )]),
        ..ClippyTomlRequirements::default()
    };
    let mut third = ClippyTomlRequirements::default();
    let _ = third.enums.insert(
        "disallowed-names".to_owned(),
        StringAssertion::Present("present".to_owned()),
    );
    let (merged, conflicts) = ClippyTomlRequirements::merge(vec![
        (prov("p1"), left),
        (prov("p2"), right),
        (prov("p3"), third),
    ]);
    assert!(conflicts.is_empty());
    assert!(matches!(
        merged.enums["disallowed-names"].merged,
        StringAssertion::Equals(ref value, _) if value == "warn"
    ));
}

#[test]
fn clippy_scalar_implication_attributes_only_failed_assertions() {
    let mut allowed = BTreeSet::new();
    let _ = allowed.insert("warn".to_owned());
    let _ = allowed.insert("deny".to_owned());
    let equals_policy = ClippyTomlRequirements {
        enums: BTreeMap::from([(
            "mode".to_owned(),
            StringAssertion::Equals("deny".to_owned(), "equals".to_owned()),
        )]),
        ..ClippyTomlRequirements::default()
    };
    let oneof_policy = ClippyTomlRequirements {
        enums: BTreeMap::from([(
            "mode".to_owned(),
            StringAssertion::OneOf(allowed, "one".to_owned()),
        )]),
        ..ClippyTomlRequirements::default()
    };
    let present_policy = ClippyTomlRequirements {
        enums: BTreeMap::from([(
            "mode".to_owned(),
            StringAssertion::Present("present".to_owned()),
        )]),
        ..ClippyTomlRequirements::default()
    };
    let output = clippy_output(
        Some(
            br#"mode = "warn"
"#,
        ),
        vec![
            (prov("equals-policy"), equals_policy),
            (prov("oneof-policy"), oneof_policy),
            (prov("present-policy"), present_policy),
        ],
    );
    let mismatch = output
        .findings
        .iter()
        .find(|finding| matches!(finding, Finding::Mismatch { message, .. } if message == "equals"))
        .expect("equals mismatch");
    assert!(matches!(
        mismatch,
        Finding::Mismatch { attribution, .. }
            if attribution.iter().any(|p| p.policy == "equals-policy")
                && attribution.iter().all(|p| p.policy != "oneof-policy" && p.policy != "present-policy")
    ));
}

#[test]
fn clippy_scalar_incompatible_cases_conflict() {
    let left = ClippyTomlRequirements {
        bools: BTreeMap::from([(
            "allow-dbg-in-tests".to_owned(),
            BoolAssertion::Equals(true, "yes".to_owned()),
        )]),
        ..ClippyTomlRequirements::default()
    };
    let right = ClippyTomlRequirements {
        bools: BTreeMap::from([(
            "allow-dbg-in-tests".to_owned(),
            BoolAssertion::Equals(false, "no".to_owned()),
        )]),
        ..ClippyTomlRequirements::default()
    };
    let findings = clippy_findings(vec![(prov("p1"), left), (prov("p2"), right)]);
    assert!(
        findings
            .iter()
            .any(|f| matches!(f, Finding::ConflictingRequirements { .. }))
    );
}

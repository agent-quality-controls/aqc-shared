use std::collections::BTreeMap;
use std::path::Path;

use aqc_file_engine_core::{FileEngine as _, Finding, ListRequirements, Provenance};
use aqc_json_file_engine::{JsonFileEngine, JsonFileRequirements, JsonPath};
use aqc_pnpm_workspace_yaml_engine::{PnpmWorkspaceYamlEngine, PnpmWorkspaceYamlRequirements};
use aqc_rustfmt_toml_engine::{RustfmtListSetting, RustfmtTomlEngine, RustfmtTomlRequirements};
use serde_json::{Value, json};

fn provenance() -> Provenance {
    Provenance {
        policy: "fixture-policy".to_owned(),
    }
}

fn finding(value: &Finding) -> Value {
    match value {
        Finding::Mismatch {
            key,
            selector,
            current,
            expected,
            attribution,
            ..
        } => json!({
            "kind": "mismatch",
            "key": key,
            "selector": selector,
            "current": current,
            "expected": expected,
            "policies": attribution.iter().map(|item| item.policy.as_str()).collect::<Vec<_>>(),
        }),
        other => json!({"kind": format!("{other:?}")}),
    }
}

fn list(exact: &[&str]) -> ListRequirements {
    ListRequirements {
        exact: Some((
            exact.iter().map(|value| (*value).to_owned()).collect(),
            "exact list".to_owned(),
        )),
        ..ListRequirements::default()
    }
}

fn json_case(input: &[u8], exact: &[&str]) -> Value {
    let requirement = JsonFileRequirements {
        string_lists: BTreeMap::from([(JsonPath::new("items"), list(exact))]),
        ..JsonFileRequirements::default()
    };
    let resolved = JsonFileRequirements::merge(vec![(provenance(), requirement)])
        .expect("JSON requirement resolves");
    let output = JsonFileEngine::reconcile(Some(input), &resolved);
    json!({
        "expectedBytes": String::from_utf8(output.expected_bytes).expect("JSON UTF-8"),
        "findings": output.findings.iter().map(finding).collect::<Vec<_>>(),
    })
}

fn toml_case(input: &[u8], exact: &[&str]) -> Value {
    let requirement = RustfmtTomlRequirements {
        list_settings: BTreeMap::from([(RustfmtListSetting::Ignore, list(exact))]),
        ..RustfmtTomlRequirements::default()
    };
    let resolved = RustfmtTomlRequirements::merge(vec![(provenance(), requirement)])
        .expect("TOML requirement resolves");
    let output = RustfmtTomlEngine::reconcile(Some(input), &resolved);
    json!({
        "expectedBytes": String::from_utf8(output.expected_bytes).expect("TOML UTF-8"),
        "findings": output.findings.iter().map(finding).collect::<Vec<_>>(),
    })
}

fn yaml_case(input: &[u8], exact: &[&str]) -> Value {
    let requirement = PnpmWorkspaceYamlRequirements {
        trust_policy_exclude: list(exact),
        ..PnpmWorkspaceYamlRequirements::default()
    };
    let resolved = PnpmWorkspaceYamlRequirements::merge(vec![(provenance(), requirement)])
        .expect("YAML requirement resolves");
    let output = PnpmWorkspaceYamlEngine::reconcile(Some(input), &resolved);
    json!({
        "expectedBytes": String::from_utf8(output.expected_bytes).expect("YAML UTF-8"),
        "findings": output.findings.iter().map(finding).collect::<Vec<_>>(),
    })
}

fn format_case(json_input: &[u8], toml_input: &[u8], yaml_input: &[u8], exact: &[&str]) -> Value {
    json!({
        "json": json_case(json_input, exact),
        "toml": toml_case(toml_input, exact),
        "yaml": yaml_case(yaml_input, exact),
    })
}

fn main() {
    let fixture = std::env::args().nth(1).expect("fixture path is required");
    let contract: Value =
        serde_json::from_slice(&std::fs::read(Path::new(&fixture)).expect("read fixture"))
            .expect("parse fixture");
    assert_eq!(
        contract["cases"],
        json!(["membership", "order", "absent-exact-empty"])
    );

    println!(
        "{}",
        json!({
            "membership": format_case(
                br#"{"items":["extra"]}"#,
                b"ignore = [\"extra\"]\n",
                b"trustPolicyExclude: [extra]\n",
                &["required"],
            ),
            "order": format_case(
                br#"{"items":["b","a"]}"#,
                b"ignore = [\"b\", \"a\"]\n",
                b"trustPolicyExclude: [b, a]\n",
                &["a", "b"],
            ),
            "absentExactEmpty": format_case(b"{}", b"", b"existing: true\n", &[]),
        })
    );
}

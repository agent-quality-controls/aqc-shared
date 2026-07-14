use aqc_file_engine_core::{
    ConfigScalar, EngineOutput, Finding, Severity, resolved_map_attribution,
};
use aqc_jsonc_engine_core::{
    JsoncParseOptions, parse_object_or_report, reconcile_scalar_assertion,
};

use crate::ResolvedTsconfigJsonRequirements;

pub(super) fn reconcile_document(
    current_bytes: Option<&[u8]>,
    requirement: &ResolvedTsconfigJsonRequirements,
) -> EngineOutput {
    let (document, mut findings) =
        parse_object_or_report(current_bytes, "tsconfig.json", typescript_parse_options());
    let Some(mut document) = document else {
        return EngineOutput {
            expected_bytes: current_bytes.unwrap_or_default().to_vec(),
            findings,
        };
    };
    let options = requirement.boolean_compiler_options();
    if !options.is_empty()
        && document.value_exists(&["compilerOptions"])
        && !document.object_exists(&["compilerOptions"])
    {
        findings.push(Finding::Mismatch {
            key: "compilerOptions".to_owned(),
            selector: None,
            current: document.rendered_value(&["compilerOptions"]),
            expected: "object".to_owned(),
            message: "compilerOptions must be an object.".to_owned(),
            severity: Severity::Error,
            attribution: resolved_map_attribution(requirement.boolean_compiler_options()),
        });
        return EngineOutput {
            expected_bytes: current_bytes.unwrap_or_default().to_vec(),
            findings,
        };
    }
    for (option, resolved) in options {
        reconcile_scalar_assertion(
            &mut document,
            &["compilerOptions", option.file_key()],
            Some(option.file_key().to_owned()),
            resolved,
            |value| Some(ConfigScalar::Bool(*value)),
            |scalar| match scalar {
                ConfigScalar::Bool(value) => Some(value),
                ConfigScalar::Str(_) | ConfigScalar::Int(_) => None,
            },
            &mut findings,
        );
    }
    EngineOutput {
        expected_bytes: document.render(),
        findings,
    }
}

const fn typescript_parse_options() -> JsoncParseOptions {
    JsoncParseOptions {
        allow_comments: true,
        allow_loose_object_property_names: false,
        allow_trailing_commas: true,
        allow_missing_commas: false,
        allow_single_quoted_strings: false,
        allow_hexadecimal_numbers: true,
        allow_unary_plus_numbers: false,
        allow_extended_json_numbers: true,
        allow_utf8_bom: true,
    }
}

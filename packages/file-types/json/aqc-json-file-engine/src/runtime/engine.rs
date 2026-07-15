use aqc_file_engine_core::{
    Engine, EngineOutput, EngineRequirement, FileEngine, Provenance, merged_reconcile,
};
use aqc_json_engine_core::{JsonParseOptions, parse_object_or_report};

use crate::runtime::reconcile::reconcile_document;
use crate::types::{JsonFileRequirements, ResolvedJsonFileRequirements};

pub const ENGINE_ID: &str = "aqc-json-file-engine";

#[derive(Debug, Default)]
pub struct JsonFileEngine;

impl FileEngine<ResolvedJsonFileRequirements> for JsonFileEngine {
    fn reconcile(
        current_bytes: Option<&[u8]>,
        requirement: &ResolvedJsonFileRequirements,
    ) -> EngineOutput {
        let (document, mut findings) = parse_object_or_report(
            current_bytes,
            "JSON document",
            JsonParseOptions {
                allow_comments: false,
                allow_loose_object_property_names: false,
                allow_trailing_commas: false,
                allow_missing_commas: false,
                allow_single_quoted_strings: false,
                allow_hexadecimal_numbers: false,
                allow_unary_plus_numbers: false,
                allow_extended_json_numbers: false,
                allow_extended_string_escapes: false,
                allow_extended_whitespace: false,
                allow_utf8_bom: false,
            },
        );
        let Some(mut document) = document else {
            return EngineOutput {
                expected_bytes: current_bytes.unwrap_or_default().to_vec(),
                findings,
            };
        };
        reconcile_document(&mut document, requirement, &mut findings);
        EngineOutput {
            expected_bytes: document.render(),
            findings,
        }
    }
}

impl Engine for JsonFileEngine {
    fn id(&self) -> &'static str {
        ENGINE_ID
    }

    fn reconcile(
        &self,
        current_bytes: Option<&[u8]>,
        requirements: &[(Provenance, Box<dyn EngineRequirement>)],
    ) -> EngineOutput {
        merged_reconcile(
            current_bytes,
            requirements,
            JsonFileRequirements::merge,
            <Self as FileEngine<ResolvedJsonFileRequirements>>::reconcile,
        )
    }
}

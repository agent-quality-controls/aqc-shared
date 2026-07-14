use aqc_file_engine_core::{
    Engine, EngineOutput, EngineRequirement, FileEngine, Provenance, merged_reconcile,
};
use aqc_json_engine_core::parse_object_or_report;

use crate::runtime::reconcile::reconcile_document;
use crate::types::{PackageJsonRequirements, ResolvedPackageJsonRequirements};

pub const ENGINE_ID: &str = "aqc-package-json-engine";

#[derive(Debug, Default)]
pub struct PackageJsonEngine;

impl FileEngine<ResolvedPackageJsonRequirements> for PackageJsonEngine {
    fn reconcile(
        current_bytes: Option<&[u8]>,
        requirement: &ResolvedPackageJsonRequirements,
    ) -> EngineOutput {
        let (document, mut findings) = parse_object_or_report(current_bytes, "JSON document");
        let Some(mut document) = document else {
            return EngineOutput {
                expected_bytes: current_bytes.unwrap_or_default().to_vec(),
                findings,
            };
        };
        let original_document = document.clone();
        reconcile_document(&mut document, requirement, &mut findings);
        let expected_bytes = match current_bytes {
            Some(bytes) if document == original_document => bytes.to_vec(),
            Some(_) | None => aqc_json_engine_core::render_object(&document),
        };
        EngineOutput {
            expected_bytes,
            findings,
        }
    }
}

impl Engine for PackageJsonEngine {
    fn id(&self) -> &'static str {
        crate::ENGINE_ID
    }

    fn reconcile(
        &self,
        current_bytes: Option<&[u8]>,
        requirements: &[(Provenance, Box<dyn EngineRequirement>)],
    ) -> EngineOutput {
        merged_reconcile(
            current_bytes,
            requirements,
            PackageJsonRequirements::merge,
            <Self as FileEngine<ResolvedPackageJsonRequirements>>::reconcile,
        )
    }
}

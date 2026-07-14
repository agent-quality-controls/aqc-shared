#![allow(
    clippy::disallowed_types,
    reason = "Any is required by the engine requirement downcast contract."
)]

use core::any::Any;
use std::collections::BTreeMap;

use aqc_file_engine_core::merge::ResolvedMap;
use aqc_file_engine_core::{EngineRequirement, ScalarAssertion};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
pub enum TsconfigBooleanCompilerOption {
    Strict,
    AlwaysStrict,
    NoImplicitAny,
    NoImplicitThis,
    StrictBindCallApply,
    StrictFunctionTypes,
    StrictNullChecks,
    StrictPropertyInitialization,
    UseUnknownInCatchVariables,
    StrictBuiltinIteratorReturn,
    NoImplicitReturns,
    NoUnusedLocals,
    NoUnusedParameters,
    NoUncheckedIndexedAccess,
    ExactOptionalPropertyTypes,
    NoPropertyAccessFromIndexSignature,
    NoImplicitOverride,
    NoFallthroughCasesInSwitch,
    ForceConsistentCasingInFileNames,
    AllowUnreachableCode,
    AllowUnusedLabels,
    NoCheck,
}

impl TsconfigBooleanCompilerOption {
    #[must_use]
    pub const fn file_key(self) -> &'static str {
        match self {
            Self::Strict => "strict",
            Self::AlwaysStrict => "alwaysStrict",
            Self::NoImplicitAny => "noImplicitAny",
            Self::NoImplicitThis => "noImplicitThis",
            Self::StrictBindCallApply => "strictBindCallApply",
            Self::StrictFunctionTypes => "strictFunctionTypes",
            Self::StrictNullChecks => "strictNullChecks",
            Self::StrictPropertyInitialization => "strictPropertyInitialization",
            Self::UseUnknownInCatchVariables => "useUnknownInCatchVariables",
            Self::StrictBuiltinIteratorReturn => "strictBuiltinIteratorReturn",
            Self::NoImplicitReturns => "noImplicitReturns",
            Self::NoUnusedLocals => "noUnusedLocals",
            Self::NoUnusedParameters => "noUnusedParameters",
            Self::NoUncheckedIndexedAccess => "noUncheckedIndexedAccess",
            Self::ExactOptionalPropertyTypes => "exactOptionalPropertyTypes",
            Self::NoPropertyAccessFromIndexSignature => "noPropertyAccessFromIndexSignature",
            Self::NoImplicitOverride => "noImplicitOverride",
            Self::NoFallthroughCasesInSwitch => "noFallthroughCasesInSwitch",
            Self::ForceConsistentCasingInFileNames => "forceConsistentCasingInFileNames",
            Self::AllowUnreachableCode => "allowUnreachableCode",
            Self::AllowUnusedLabels => "allowUnusedLabels",
            Self::NoCheck => "noCheck",
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct TsconfigJsonRequirements {
    pub boolean_compiler_options: BTreeMap<TsconfigBooleanCompilerOption, ScalarAssertion<bool>>,
}

#[derive(Debug, Clone)]
pub struct ResolvedTsconfigJsonRequirements {
    pub(crate) boolean_compiler_options:
        ResolvedMap<TsconfigBooleanCompilerOption, ScalarAssertion<bool>>,
}

impl ResolvedTsconfigJsonRequirements {
    #[must_use]
    pub const fn boolean_compiler_options(
        &self,
    ) -> &ResolvedMap<TsconfigBooleanCompilerOption, ScalarAssertion<bool>> {
        &self.boolean_compiler_options
    }
}

impl EngineRequirement for TsconfigJsonRequirements {
    fn engine_id(&self) -> &'static str {
        crate::ENGINE_ID
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

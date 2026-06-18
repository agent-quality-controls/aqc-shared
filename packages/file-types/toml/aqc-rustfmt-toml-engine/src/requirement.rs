//! Declarative requirement and assertion types accepted by `RustfmtTomlEngine`.

#![expect(
    clippy::disallowed_types,
    reason = "`Any` is used only for EngineRequirement downcast dispatch."
)]

use core::any::Any;
use std::collections::{BTreeMap, BTreeSet};

use aqc_file_engine_core::{
    ConfigScalar, ConflictEntry, EngineRequirement, ForbiddenGlobRequirement,
    ForbiddenGlobRequirements, ListRequirements, OnEmpty, OnEmptyClass, Provenance, Resolve,
    ResolvedForbiddenGlobRequirements, ResolvedListRequirements, ResolvedRequirement,
    resolve_forbidden_globs, resolve_list, resolve_map,
};
use globset::GlobBuilder;

#[derive(Debug, Clone, Default)]
pub struct RustfmtTomlRequirements {
    pub scalar_settings: BTreeMap<RustfmtScalarSetting, RustfmtScalarAssertion>,
    pub list_settings: BTreeMap<RustfmtListSetting, ListRequirements>,
    pub forbidden_ignore_path_globs: ForbiddenGlobRequirements<RustfmtIgnorePathGlob>,
    pub closed_settings: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ResolvedRustfmtTomlRequirements {
    pub scalar_settings: BTreeMap<
        RustfmtScalarSetting,
        ResolvedRequirement<ResolvedRustfmtScalarAssertion, RustfmtScalarAssertion>,
    >,
    pub list_settings: BTreeMap<RustfmtListSetting, ResolvedListRequirements>,
    pub forbidden_ignore_path_globs: ResolvedForbiddenGlobRequirements<RustfmtIgnorePathGlob>,
    pub ignore_glob_conflicts: RustfmtForbiddenIgnoreGlobConflictBlocks,
    pub closed_settings: Vec<(Provenance, String)>,
}

impl RustfmtTomlRequirements {
    #[must_use]
    pub fn merge(
        reqs: Vec<(Provenance, RustfmtTomlRequirements)>,
    ) -> (ResolvedRustfmtTomlRequirements, Vec<ConflictEntry>) {
        let mut conflicts = Vec::new();
        let scalar_settings = resolve_map(
            reqs.iter()
                .map(|(prov, req)| (prov.clone(), req.scalar_settings.clone()))
                .collect(),
            |key| key.file_key().to_owned(),
            &mut conflicts,
        );
        let forbidden_ignore_path_globs = resolve_forbidden_globs(
            RustfmtListSetting::Ignore.file_key(),
            reqs.iter()
                .map(|(prov, req)| (prov.clone(), req.forbidden_ignore_path_globs.clone()))
                .collect(),
            &mut conflicts,
        );

        let mut lists_by_key: BTreeMap<RustfmtListSetting, Vec<(Provenance, ListRequirements)>> =
            BTreeMap::new();
        let mut closed_settings = Vec::new();
        for (prov, req) in reqs {
            for (key, list) in req.list_settings {
                lists_by_key
                    .entry(key)
                    .or_default()
                    .push((prov.clone(), list));
            }
            if let Some(message) = req.closed_settings {
                closed_settings.push((prov, message));
            }
        }

        let mut list_settings = BTreeMap::new();
        for (key, lists) in lists_by_key {
            let _ = list_settings.insert(key, resolve_list(key.file_key(), lists, &mut conflicts));
        }
        let ignore_glob_conflicts = list_settings.get(&RustfmtListSetting::Ignore).map_or_else(
            RustfmtForbiddenIgnoreGlobConflictBlocks::default,
            |ignore| {
                push_ignore_glob_conflicts(ignore, &forbidden_ignore_path_globs, &mut conflicts)
            },
        );

        (
            ResolvedRustfmtTomlRequirements {
                scalar_settings,
                list_settings,
                forbidden_ignore_path_globs,
                ignore_glob_conflicts,
                closed_settings,
            },
            conflicts,
        )
    }
}

impl EngineRequirement for RustfmtTomlRequirements {
    fn engine_id(&self) -> &'static str {
        crate::ENGINE_ID
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Debug, Clone)]
pub enum RustfmtScalarAssertion {
    Equals(ConfigScalar, String),
    OneOf(BTreeSet<String>, String),
    Present(String),
    Absent(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolvedRustfmtScalarAssertion {
    Equals(ConfigScalar, String),
    OneOf(BTreeSet<String>, String),
    Present(String),
    Absent(String),
}

impl PartialEq for RustfmtScalarAssertion {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Equals(left, _), Self::Equals(right, _)) => left == right,
            (Self::OneOf(left, _), Self::OneOf(right, _)) => left == right,
            (Self::Present(_), Self::Present(_)) | (Self::Absent(_), Self::Absent(_)) => true,
            _ => false,
        }
    }
}

impl Resolve for RustfmtScalarAssertion {
    type Merged = ResolvedRustfmtScalarAssertion;

    fn resolve(
        key: &str,
        items: Vec<(Provenance, Self)>,
        conflicts: &mut Vec<ConflictEntry>,
    ) -> Option<ResolvedRequirement<Self::Merged, Self>> {
        let mut iter = items.iter();
        let (_, first) = iter.next()?;
        if iter.any(|(_, item)| item != first) {
            conflicts.push(ConflictEntry {
                key: key.to_owned(),
                reason: "scalar-disagree".to_owned(),
                contributors: items
                    .iter()
                    .map(|(prov, value)| (prov.clone(), render_assertion(value)))
                    .collect(),
            });
            return None;
        }
        Some(ResolvedRequirement {
            merged: resolve_assertion(first),
            collected: items,
        })
    }
}

impl OnEmptyClass for RustfmtScalarAssertion {
    fn on_empty(&self) -> OnEmpty {
        match self {
            Self::Equals(..) | Self::Absent(..) => OnEmpty::Writes,
            Self::OneOf(..) | Self::Present(..) => OnEmpty::ChecksOnly,
        }
    }
}

impl OnEmptyClass for ResolvedRustfmtScalarAssertion {
    fn on_empty(&self) -> OnEmpty {
        match self {
            Self::Equals(..) | Self::Absent(..) => OnEmpty::Writes,
            Self::OneOf(..) | Self::Present(..) => OnEmpty::ChecksOnly,
        }
    }
}

fn resolve_assertion(assertion: &RustfmtScalarAssertion) -> ResolvedRustfmtScalarAssertion {
    match assertion {
        RustfmtScalarAssertion::Equals(value, message) => {
            ResolvedRustfmtScalarAssertion::Equals(value.clone(), message.clone())
        }
        RustfmtScalarAssertion::OneOf(values, message) => {
            ResolvedRustfmtScalarAssertion::OneOf(values.clone(), message.clone())
        }
        RustfmtScalarAssertion::Present(message) => {
            ResolvedRustfmtScalarAssertion::Present(message.clone())
        }
        RustfmtScalarAssertion::Absent(message) => {
            ResolvedRustfmtScalarAssertion::Absent(message.clone())
        }
    }
}

fn render_assertion(assertion: &RustfmtScalarAssertion) -> String {
    match assertion {
        RustfmtScalarAssertion::Equals(value, _) => format!("equals {value:?}"),
        RustfmtScalarAssertion::OneOf(values, _) => format!("one of {values:?}"),
        RustfmtScalarAssertion::Present(_) => "present".to_owned(),
        RustfmtScalarAssertion::Absent(_) => "absent".to_owned(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RustfmtScalarSetting {
    MaxWidth,
    HardTabs,
    TabSpaces,
    NewlineStyle,
    IndentStyle,
    UseSmallHeuristics,
    FnCallWidth,
    AttrFnLikeWidth,
    BlankLinesLowerBound,
    BlankLinesUpperBound,
    StructLitWidth,
    StructVariantWidth,
    ArrayWidth,
    ChainWidth,
    SingleLineIfElseMaxWidth,
    SingleLineLetElseMaxWidth,
    WrapComments,
    FormatCodeInDocComments,
    DocCommentCodeBlockWidth,
    CommentWidth,
    NormalizeComments,
    NormalizeDocAttributes,
    OverflowDelimitedExpr,
    FormatStrings,
    HexLiteralCase,
    FloatLiteralTrailingZero,
    FormatMacroBodies,
    FormatMacroMatchers,
    Color,
    ReorderImports,
    ReorderModules,
    GroupImports,
    ImportsGranularity,
    ImportsIndent,
    ImportsLayout,
    MergeImports,
    ReorderImplItems,
    EmptyItemSingleLine,
    StructLitSingleLine,
    FnSingleLine,
    WhereSingleLine,
    SpaceBeforeColon,
    SpaceAfterColon,
    SpacesAroundRanges,
    TypePunctuationDensity,
    BinopSeparator,
    BraceStyle,
    ControlBraceStyle,
    MatchArmBlocks,
    MatchArmLeadingPipes,
    MatchArmIndent,
    MatchBlockTrailingComma,
    ForceMultilineBlocks,
    FnArgsLayout,
    FnParamsLayout,
    MergeDerives,
    UseTryShorthand,
    UseFieldInitShorthand,
    RemoveNestedParens,
    CondenseWildcardSuffixes,
    ForceExplicitAbi,
    TrailingSemicolon,
    TrailingComma,
    CombineControlExpr,
    ShortArrayElementWidthThreshold,
    StructFieldAlignThreshold,
    EnumDiscrimAlignThreshold,
    InlineAttributeWidth,
    FormatGeneratedFiles,
    GeneratedMarkerLineSearchLimit,
    Edition,
    StyleEdition,
    Version,
    RequiredVersion,
    EmitMode,
    MakeBackup,
    PrintMisformattedFileNames,
    UnstableFeatures,
    DisableAllFormatting,
    SkipChildren,
    ErrorOnLineOverflow,
    ErrorOnUnformatted,
    HideParseErrors,
    ShowParseErrors,
}

impl RustfmtScalarSetting {
    #[must_use]
    pub const fn file_key(self) -> &'static str {
        match self {
            Self::MaxWidth => "max_width",
            Self::HardTabs => "hard_tabs",
            Self::TabSpaces => "tab_spaces",
            Self::NewlineStyle => "newline_style",
            Self::IndentStyle => "indent_style",
            Self::UseSmallHeuristics => "use_small_heuristics",
            Self::FnCallWidth => "fn_call_width",
            Self::AttrFnLikeWidth => "attr_fn_like_width",
            Self::BlankLinesLowerBound => "blank_lines_lower_bound",
            Self::BlankLinesUpperBound => "blank_lines_upper_bound",
            Self::StructLitWidth => "struct_lit_width",
            Self::StructVariantWidth => "struct_variant_width",
            Self::ArrayWidth => "array_width",
            Self::ChainWidth => "chain_width",
            Self::SingleLineIfElseMaxWidth => "single_line_if_else_max_width",
            Self::SingleLineLetElseMaxWidth => "single_line_let_else_max_width",
            Self::WrapComments => "wrap_comments",
            Self::FormatCodeInDocComments => "format_code_in_doc_comments",
            Self::DocCommentCodeBlockWidth => "doc_comment_code_block_width",
            Self::CommentWidth => "comment_width",
            Self::NormalizeComments => "normalize_comments",
            Self::NormalizeDocAttributes => "normalize_doc_attributes",
            Self::OverflowDelimitedExpr => "overflow_delimited_expr",
            Self::FormatStrings => "format_strings",
            Self::HexLiteralCase => "hex_literal_case",
            Self::FloatLiteralTrailingZero => "float_literal_trailing_zero",
            Self::FormatMacroBodies => "format_macro_bodies",
            Self::FormatMacroMatchers => "format_macro_matchers",
            Self::Color => "color",
            Self::ReorderImports => "reorder_imports",
            Self::ReorderModules => "reorder_modules",
            Self::GroupImports => "group_imports",
            Self::ImportsGranularity => "imports_granularity",
            Self::ImportsIndent => "imports_indent",
            Self::ImportsLayout => "imports_layout",
            Self::MergeImports => "merge_imports",
            Self::ReorderImplItems => "reorder_impl_items",
            Self::EmptyItemSingleLine => "empty_item_single_line",
            Self::StructLitSingleLine => "struct_lit_single_line",
            Self::FnSingleLine => "fn_single_line",
            Self::WhereSingleLine => "where_single_line",
            Self::SpaceBeforeColon => "space_before_colon",
            Self::SpaceAfterColon => "space_after_colon",
            Self::SpacesAroundRanges => "spaces_around_ranges",
            Self::TypePunctuationDensity => "type_punctuation_density",
            Self::BinopSeparator => "binop_separator",
            Self::BraceStyle => "brace_style",
            Self::ControlBraceStyle => "control_brace_style",
            Self::MatchArmBlocks => "match_arm_blocks",
            Self::MatchArmLeadingPipes => "match_arm_leading_pipes",
            Self::MatchArmIndent => "match_arm_indent",
            Self::MatchBlockTrailingComma => "match_block_trailing_comma",
            Self::ForceMultilineBlocks => "force_multiline_blocks",
            Self::FnArgsLayout => "fn_args_layout",
            Self::FnParamsLayout => "fn_params_layout",
            Self::MergeDerives => "merge_derives",
            Self::UseTryShorthand => "use_try_shorthand",
            Self::UseFieldInitShorthand => "use_field_init_shorthand",
            Self::RemoveNestedParens => "remove_nested_parens",
            Self::CondenseWildcardSuffixes => "condense_wildcard_suffixes",
            Self::ForceExplicitAbi => "force_explicit_abi",
            Self::TrailingSemicolon => "trailing_semicolon",
            Self::TrailingComma => "trailing_comma",
            Self::CombineControlExpr => "combine_control_expr",
            Self::ShortArrayElementWidthThreshold => "short_array_element_width_threshold",
            Self::StructFieldAlignThreshold => "struct_field_align_threshold",
            Self::EnumDiscrimAlignThreshold => "enum_discrim_align_threshold",
            Self::InlineAttributeWidth => "inline_attribute_width",
            Self::FormatGeneratedFiles => "format_generated_files",
            Self::GeneratedMarkerLineSearchLimit => "generated_marker_line_search_limit",
            Self::Edition => "edition",
            Self::StyleEdition => "style_edition",
            Self::Version => "version",
            Self::RequiredVersion => "required_version",
            Self::EmitMode => "emit_mode",
            Self::MakeBackup => "make_backup",
            Self::PrintMisformattedFileNames => "print_misformatted_file_names",
            Self::UnstableFeatures => "unstable_features",
            Self::DisableAllFormatting => "disable_all_formatting",
            Self::SkipChildren => "skip_children",
            Self::ErrorOnLineOverflow => "error_on_line_overflow",
            Self::ErrorOnUnformatted => "error_on_unformatted",
            Self::HideParseErrors => "hide_parse_errors",
            Self::ShowParseErrors => "show_parse_errors",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RustfmtListSetting {
    Ignore,
    SkipMacroInvocations,
}

impl RustfmtListSetting {
    #[must_use]
    pub const fn file_key(self) -> &'static str {
        match self {
            Self::Ignore => "ignore",
            Self::SkipMacroInvocations => "skip_macro_invocations",
        }
    }
}

/// Path glob used only for forbidden `ignore` entries.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RustfmtIgnorePathGlob {
    pub glob: String,
}

impl ForbiddenGlobRequirement for RustfmtIgnorePathGlob {
    type Identity = String;

    fn merge_identity(&self) -> Self::Identity {
        self.glob.clone()
    }

    fn render(&self) -> String {
        self.glob.clone()
    }
}

/// Required `ignore` values and forbidden `ignore` globs that conflict.
#[derive(Debug, Clone, Default)]
pub struct RustfmtForbiddenIgnoreGlobConflictBlocks {
    /// Required `ignore` values blocked during reconciliation.
    pub required: BTreeSet<String>,
    /// Forbidden `ignore` globs blocked during reconciliation.
    pub path_globs: BTreeSet<String>,
}

fn push_ignore_glob_conflicts(
    ignore: &ResolvedListRequirements,
    globs: &ResolvedForbiddenGlobRequirements<RustfmtIgnorePathGlob>,
    conflicts: &mut Vec<ConflictEntry>,
) -> RustfmtForbiddenIgnoreGlobConflictBlocks {
    let mut blocks = RustfmtForbiddenIgnoreGlobConflictBlocks::default();
    for (glob_identity, glob) in &globs.globs {
        let Ok(compiled) = GlobBuilder::new(&glob.merged.glob).build() else {
            continue;
        };
        let matcher = compiled.compile_matcher();
        let required = ignore
            .contains
            .keys()
            .chain(ignore.exact.iter().flat_map(|exact| exact.merged.iter()))
            .filter(|path| matcher.is_match(path.as_str()))
            .cloned()
            .collect::<BTreeSet<_>>();
        if required.is_empty() {
            continue;
        }
        for path in required {
            let mut contributors = ignore
                .contains
                .get(&path)
                .into_iter()
                .flat_map(|req| req.collected.iter())
                .map(|(prov, _)| (prov.clone(), "required".to_owned()))
                .collect::<Vec<_>>();
            contributors.extend(
                ignore
                    .exact
                    .iter()
                    .flat_map(|req| req.collected.iter())
                    .filter(|(_, (values, _))| values.iter().any(|value| value == &path))
                    .map(|(prov, _)| (prov.clone(), "required".to_owned())),
            );
            contributors.extend(
                glob.collected
                    .iter()
                    .map(|(prov, _)| (prov.clone(), "forbidden".to_owned())),
            );
            conflicts.push(ConflictEntry {
                key: format!("{}.{}", RustfmtListSetting::Ignore.file_key(), path),
                reason: "ignore-path-glob-forbids-required-path".to_owned(),
                contributors,
            });
            let _ = blocks.required.insert(path);
            let _ = blocks.path_globs.insert(glob_identity.clone());
        }
    }
    blocks
}

//! Rustfmt setting names and scalar-operation legality.

use aqc_file_engine_core::{ConfigScalar, ScalarAssertion};

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
    #[expect(
        clippy::too_many_lines,
        reason = "Exhaustive rustfmt setting map is intentionally one match table."
    )]
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

    pub(super) fn scalar_assertion_is_legal(
        self,
        assertion: &ScalarAssertion<ConfigScalar>,
    ) -> bool {
        if matches!(
            assertion,
            ScalarAssertion::AtLeast(..) | ScalarAssertion::AtMost(..) | ScalarAssertion::Range(..)
        ) {
            return false;
        }
        match self.scalar_kind() {
            RustfmtScalarKind::Bool => matches!(
                assertion,
                ScalarAssertion::Equals(ConfigScalar::Bool(_), _)
                    | ScalarAssertion::Present(_)
                    | ScalarAssertion::Absent(_)
            ),
            RustfmtScalarKind::Int => match assertion {
                ScalarAssertion::Equals(ConfigScalar::Int(_), _)
                | ScalarAssertion::Present(_)
                | ScalarAssertion::Absent(_) => true,
                ScalarAssertion::OneOf(values, _) => values
                    .iter()
                    .all(|value| matches!(value, ConfigScalar::Int(_))),
                _ => false,
            },
            RustfmtScalarKind::Text => match assertion {
                ScalarAssertion::Equals(ConfigScalar::Str(_), _)
                | ScalarAssertion::Present(_)
                | ScalarAssertion::Absent(_) => true,
                ScalarAssertion::OneOf(values, _) => values
                    .iter()
                    .all(|value| matches!(value, ConfigScalar::Str(_))),
                _ => false,
            },
        }
    }

    fn scalar_kind(self) -> RustfmtScalarKind {
        match self {
            Self::MaxWidth
            | Self::TabSpaces
            | Self::FnCallWidth
            | Self::AttrFnLikeWidth
            | Self::BlankLinesLowerBound
            | Self::BlankLinesUpperBound
            | Self::StructLitWidth
            | Self::StructVariantWidth
            | Self::ArrayWidth
            | Self::ChainWidth
            | Self::SingleLineIfElseMaxWidth
            | Self::SingleLineLetElseMaxWidth
            | Self::DocCommentCodeBlockWidth
            | Self::CommentWidth
            | Self::ShortArrayElementWidthThreshold
            | Self::StructFieldAlignThreshold
            | Self::EnumDiscrimAlignThreshold
            | Self::InlineAttributeWidth
            | Self::GeneratedMarkerLineSearchLimit => RustfmtScalarKind::Int,
            Self::HardTabs
            | Self::WrapComments
            | Self::FormatCodeInDocComments
            | Self::NormalizeComments
            | Self::NormalizeDocAttributes
            | Self::OverflowDelimitedExpr
            | Self::FormatStrings
            | Self::FormatMacroBodies
            | Self::FormatMacroMatchers
            | Self::ReorderImports
            | Self::ReorderModules
            | Self::MergeImports
            | Self::ReorderImplItems
            | Self::EmptyItemSingleLine
            | Self::StructLitSingleLine
            | Self::FnSingleLine
            | Self::WhereSingleLine
            | Self::SpaceBeforeColon
            | Self::SpaceAfterColon
            | Self::SpacesAroundRanges
            | Self::MatchBlockTrailingComma
            | Self::ForceMultilineBlocks
            | Self::MergeDerives
            | Self::UseTryShorthand
            | Self::UseFieldInitShorthand
            | Self::RemoveNestedParens
            | Self::CondenseWildcardSuffixes
            | Self::ForceExplicitAbi
            | Self::CombineControlExpr
            | Self::FormatGeneratedFiles
            | Self::MakeBackup
            | Self::PrintMisformattedFileNames
            | Self::UnstableFeatures
            | Self::DisableAllFormatting
            | Self::SkipChildren
            | Self::ErrorOnLineOverflow
            | Self::ErrorOnUnformatted
            | Self::HideParseErrors
            | Self::ShowParseErrors => RustfmtScalarKind::Bool,
            Self::NewlineStyle
            | Self::IndentStyle
            | Self::UseSmallHeuristics
            | Self::HexLiteralCase
            | Self::FloatLiteralTrailingZero
            | Self::Color
            | Self::GroupImports
            | Self::ImportsGranularity
            | Self::ImportsIndent
            | Self::ImportsLayout
            | Self::TypePunctuationDensity
            | Self::BinopSeparator
            | Self::BraceStyle
            | Self::ControlBraceStyle
            | Self::MatchArmBlocks
            | Self::MatchArmLeadingPipes
            | Self::MatchArmIndent
            | Self::TrailingSemicolon
            | Self::TrailingComma
            | Self::FnArgsLayout
            | Self::FnParamsLayout
            | Self::Edition
            | Self::StyleEdition
            | Self::Version
            | Self::RequiredVersion
            | Self::EmitMode => RustfmtScalarKind::Text,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RustfmtScalarKind {
    Bool,
    Int,
    Text,
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

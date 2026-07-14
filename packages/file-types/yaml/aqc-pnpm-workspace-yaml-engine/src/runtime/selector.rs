//! pnpm package-selector glob matching.

use globset::{GlobBuilder, GlobMatcher};

pub(crate) fn compile_selector_glob(glob: &str) -> Result<GlobMatcher, globset::Error> {
    GlobBuilder::new(glob)
        .literal_separator(false)
        .backslash_escape(true)
        .build()
        .map(|compiled| compiled.compile_matcher())
}

pub(crate) fn selector_matches(glob: &str, selector: &str) -> Result<bool, globset::Error> {
    compile_selector_glob(glob).map(|compiled| compiled.is_match(selector))
}

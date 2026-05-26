//! Code generator for aqc-clippy-toml-parser-types.
//!
//! Downloads the pinned upstream Clippy `conf.rs`, parses the `define_Conf!`
//! macro, and emits a typed `ClippyToml` struct with serde derives and a
//! `Default` impl into the sibling `types` crate.
//!
//! Re-run with: `cargo run -p aqc-clippy-toml-parser-generator --bin generate`
//! from the facade workspace root.

#![allow(
    clippy::missing_docs_in_private_items,
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::disallowed_macros,
    clippy::disallowed_methods,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::string_slice,
    clippy::str_to_string,
    clippy::format_push_string,
    clippy::uninlined_format_args,
    clippy::missing_const_for_fn,
    clippy::too_many_lines,
    clippy::excessive_nesting,
    clippy::needless_raw_string_hashes,
    reason = "internal one-shot generator binary; println/expect/std::fs are the right tools for a build-time CLI and the workspace's library-grade lints fight that shape"
)]

mod fs;
mod parse;
mod render;

use std::path::{Path, PathBuf};

const CLIPPY_TAG: &str = "rust-1.95.0";
const OUTPUT_RELATIVE: &str = "crates/types/src/clippy_toml.rs";
const CACHE_RELATIVE: &str = "crates/generator/.cache/conf.rs";

fn conf_rs_url() -> String {
    format!(
        "https://raw.githubusercontent.com/rust-lang/rust-clippy/{tag}/clippy_config/src/conf.rs",
        tag = CLIPPY_TAG
    )
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR points at crates/generator. Two levels up = facade root.
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .and_then(Path::parent)
        .expect("generator manifest is nested under <facade>/crates/generator")
        .to_path_buf()
}

fn main() {
    println!("aqc-clippy-toml-parser generator (clippy pin: {CLIPPY_TAG})");

    let root = workspace_root();
    let cache_path = root.join(CACHE_RELATIVE);
    let output_path = root.join(OUTPUT_RELATIVE);

    let conf_rs = fs::load_or_fetch(&cache_path, &conf_rs_url());
    println!("Loaded {} bytes of conf.rs", conf_rs.len());

    let fields = parse::define_conf(&conf_rs);
    println!("Extracted {} fields from define_Conf!", fields.len());

    let generated = render::types_file(&fields, CLIPPY_TAG);
    fs::write_file(&output_path, &generated);
    println!("Wrote {}", output_path.display());
}

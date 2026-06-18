#![expect(clippy::expect_used, reason = "tests need concise fixture assertions")]

use aqc_rust_syntax::{RustVisibility, parse_rust_syntax};
use proc_macro2 as _;
use syn as _;

fn parsed(source: &str) -> aqc_rust_syntax::RustFileSyntax {
    parse_rust_syntax(source).expect("Rust fixture must parse")
}

#[test]
fn captures_unit_tuple_and_struct_variants() {
    let syntax = parsed(
        r"
enum Shape {
    Unit,
    Tuple(u8),
    Struct { value: u8 },
}
",
    );
    let enum_decl = syntax.enums.first().expect("fixture must contain an enum");
    assert_eq!(enum_decl.name, "Shape");
    assert_eq!(enum_decl.variants, ["Unit", "Tuple", "Struct"]);
}

#[test]
fn captures_public_crate_restricted_and_private_visibilities() {
    let syntax = parsed(
        r"
enum PrivateEnum { A }
pub enum PublicEnum { A }
pub(crate) enum CrateEnum { A }
pub(super) enum RestrictedEnum { A }
",
    );
    let visibilities = syntax
        .enums
        .iter()
        .map(|enum_decl| enum_decl.visibility.clone())
        .collect::<Vec<_>>();
    assert_eq!(
        visibilities,
        [
            RustVisibility::Private,
            RustVisibility::Public,
            RustVisibility::Crate,
            RustVisibility::Restricted("super".to_owned())
        ]
    );
}

#[test]
fn captures_inline_nested_modules_and_ignores_external_modules() {
    let syntax = parsed(
        r"
mod external;
mod wire {
    pub enum Status { Ready, Done }
    mod nested {
        enum Mode { Fast }
    }
}
",
    );
    let names = syntax
        .enums
        .iter()
        .map(|enum_decl| {
            let path = enum_decl.module_path.join("::");
            format!("{path}::{}", enum_decl.name)
        })
        .collect::<Vec<_>>();
    assert_eq!(names, ["wire::Status", "wire::nested::Mode"]);
}

#[test]
fn preserves_duplicate_enum_names_as_separate_declarations() {
    let syntax = parsed(
        r"
mod a { enum Status { One } }
mod b { enum Status { Two } }
",
    );
    assert_eq!(syntax.enums.len(), 2);
    assert_eq!(
        syntax.enums.first().map(|decl| decl.name.as_str()),
        Some("Status")
    );
    assert_eq!(
        syntax.enums.get(1).map(|decl| decl.name.as_str()),
        Some("Status")
    );
}

#[test]
fn strips_bom_before_parsing() {
    let syntax = parsed("\u{feff}enum Bom { Works }\n");
    assert_eq!(
        syntax.enums.first().map(|decl| decl.name.as_str()),
        Some("Bom")
    );
}

#[test]
fn returns_parse_error_for_malformed_rust() {
    let result = parse_rust_syntax("enum Broken {");
    assert!(result.is_err(), "malformed Rust should fail");
}

#[test]
fn attributes_do_not_change_enum_facts() {
    let syntax = parsed(
        r#"
#[derive(Debug)]
#[cfg(feature = "serde")]
enum Category {
    #[serde(rename = "tree")]
    Tree,
}
"#,
    );
    let enum_decl = syntax.enums.first().expect("fixture must contain an enum");
    assert_eq!(enum_decl.name, "Category");
    assert_eq!(enum_decl.variants, ["Tree"]);
}

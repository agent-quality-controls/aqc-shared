//! Rust source parser implementation.

use syn::visit::Visit as _;

use crate::{RustEnumDecl, RustFileSyntax, RustSyntaxError, RustVisibility};

/// Parse one Rust source string into file-local syntax facts.
///
/// # Errors
///
/// Returns [`RustSyntaxError`] when `syn` cannot parse the source.
pub fn parse_rust_syntax(source: &str) -> Result<RustFileSyntax, RustSyntaxError> {
    let source = source.strip_prefix('\u{feff}').unwrap_or(source);
    let file = syn::parse_file(source).map_err(|error| RustSyntaxError {
        message: error.to_string(),
    })?;
    let mut visitor = EnumVisitor::default();
    for item in &file.items {
        visitor.visit_item(item);
    }
    Ok(RustFileSyntax {
        enums: visitor.enums,
    })
}

/// Visitor that only follows file items and inline module items.
#[derive(Debug, Default)]
struct EnumVisitor {
    /// Current inline module path.
    module_path: Vec<String>,
    /// Collected enum declarations.
    enums: Vec<RustEnumDecl>,
}

impl<'ast> syn::visit::Visit<'ast> for EnumVisitor {
    fn visit_item(&mut self, item: &'ast syn::Item) {
        match item {
            syn::Item::Enum(item_enum) => self.record_enum(item_enum),
            syn::Item::Mod(item_mod) => self.visit_inline_module(item_mod),
            syn::Item::Const(_)
            | syn::Item::ExternCrate(_)
            | syn::Item::Fn(_)
            | syn::Item::ForeignMod(_)
            | syn::Item::Impl(_)
            | syn::Item::Macro(_)
            | syn::Item::Static(_)
            | syn::Item::Struct(_)
            | syn::Item::Trait(_)
            | syn::Item::TraitAlias(_)
            | syn::Item::Type(_)
            | syn::Item::Union(_)
            | syn::Item::Use(_)
            | syn::Item::Verbatim(_) => {}
            _ => ignore_future_syn_item(item),
        }
    }
}

impl EnumVisitor {
    /// Record an enum at the current inline module path.
    fn record_enum(&mut self, item: &syn::ItemEnum) {
        self.enums.push(RustEnumDecl {
            name: item.ident.to_string(),
            module_path: self.module_path.clone(),
            variants: item
                .variants
                .iter()
                .map(|variant| variant.ident.to_string())
                .collect(),
            visibility: rust_visibility(&item.vis),
            line: line_of_ident(&item.ident),
        });
    }

    /// Visit an inline module and ignore external `mod name;` declarations.
    fn visit_inline_module(&mut self, item: &syn::ItemMod) {
        let Some((_, items)) = &item.content else {
            return;
        };
        self.module_path.push(item.ident.to_string());
        for child in items {
            self.visit_item(child);
        }
        let _ = self.module_path.pop();
    }
}

/// Convert `syn` visibility to the public fact enum.
fn rust_visibility(vis: &syn::Visibility) -> RustVisibility {
    match vis {
        syn::Visibility::Inherited => RustVisibility::Private,
        syn::Visibility::Public(_) => RustVisibility::Public,
        syn::Visibility::Restricted(restricted) => {
            let path = path_label(&restricted.path);
            if path == "crate" {
                RustVisibility::Crate
            } else {
                RustVisibility::Restricted(path)
            }
        }
    }
}

/// Format a Rust path without interpreting it.
fn path_label(path: &syn::Path) -> String {
    let parts = path
        .segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect::<Vec<_>>();
    let prefix = if path.leading_colon.is_some() {
        "::"
    } else {
        ""
    };
    format!("{prefix}{}", parts.join("::"))
}

/// Get the one-based source line for an identifier.
fn line_of_ident(ident: &syn::Ident) -> usize {
    let span: proc_macro2::Span = ident.span();
    span.start().line
}

/// Ignore future `syn::Item` variants without treating them as enum facts.
const fn ignore_future_syn_item(_item: &syn::Item) {}

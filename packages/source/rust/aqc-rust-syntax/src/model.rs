//! Public syntax fact types.

/// Syntax facts extracted from one Rust source file.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RustFileSyntax {
    /// Enum declarations found at file scope or inside inline modules.
    pub enums: Vec<RustEnumDecl>,
}

/// One Rust enum declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustEnumDecl {
    /// The enum identifier.
    pub name: String,
    /// Inline module path containing the enum.
    pub module_path: Vec<String>,
    /// Variant identifiers in declaration order.
    pub variants: Vec<String>,
    /// Rust visibility on the enum declaration.
    pub visibility: RustVisibility,
    /// One-based source line for the enum identifier.
    pub line: usize,
}

/// Rust visibility facts for an enum declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RustVisibility {
    /// No `pub` marker.
    Private,
    /// Plain `pub`.
    Public,
    /// `pub(crate)`.
    Crate,
    /// `pub(self)`, `pub(super)`, or `pub(in path)`.
    Restricted(String),
}

/// Rust parse failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustSyntaxError {
    /// Parser error message.
    pub message: String,
}

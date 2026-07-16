use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use quote::ToTokens;
use syn::visit::Visit;
use syn::{
    Fields, ImplItemFn, ItemFn, ItemImpl, ItemMacro, ItemStruct, ItemType, ItemUse, ReturnType,
    Type, UseTree, Visibility,
};

use crate::discover::{ParsedCrate, discover};
use crate::expression::adapter_violations;
use crate::model::{
    ArchitectureError, ArchitectureReport, ArchitectureViolation, MembershipField, RequirementKind,
    RequirementRoot, ViolationCode,
};

struct PublicStruct {
    fields: Vec<(String, Type)>,
    has_named_fields: bool,
    is_public: bool,
    source: PathBuf,
}

#[derive(Default)]
pub(crate) struct CrateIndex {
    pub aliases: BTreeMap<String, Type>,
    local_macros: BTreeMap<String, String>,
    trait_aliases: BTreeMap<String, String>,
    roots: Vec<(String, RequirementKind)>,
    structs: BTreeMap<String, Vec<PublicStruct>>,
    reimplemented_core_types: Vec<(String, PathBuf)>,
    uninspectable_macros: Vec<(String, PathBuf)>,
    membership_functions: BTreeSet<String>,
    membership_field_names: BTreeSet<String>,
    adapter_root_names: BTreeSet<String>,
}

struct IndexVisitor<'a> {
    index: &'a mut CrateIndex,
    source: &'a Path,
}

impl Visit<'_> for IndexVisitor<'_> {
    fn visit_item_impl(&mut self, item: &ItemImpl) {
        if let Some((_, trait_path, _)) = &item.trait_ {
            let kind = trait_path.segments.last().and_then(|segment| {
                let name =
                    resolve_trait_alias(&segment.ident.to_string(), &self.index.trait_aliases);
                match name.as_str() {
                    "EngineRequirement" => Some(RequirementKind::Engine),
                    "AdapterRequirement" => Some(RequirementKind::Adapter),
                    _ => None,
                }
            });
            if let (Some(kind), Some(name)) = (kind, type_name(item.self_ty.as_ref())) {
                self.index.roots.push((name, kind));
            }
        }
        syn::visit::visit_item_impl(self, item);
    }

    fn visit_item_macro(&mut self, item: &ItemMacro) {
        if let Some(identifier) = &item.ident {
            self.index.local_macros.insert(
                identifier.to_string(),
                item.mac.tokens.to_token_stream().to_string(),
            );
        } else {
            self.index.uninspectable_macros.push((
                item.mac.path.to_token_stream().to_string(),
                self.source.to_path_buf(),
            ));
        }
        syn::visit::visit_item_macro(self, item);
    }

    fn visit_item_use(&mut self, item: &ItemUse) {
        collect_trait_aliases(&item.tree, &mut self.index.trait_aliases);
        collect_type_import_aliases(&item.tree, &mut self.index.aliases);
        syn::visit::visit_item_use(self, item);
    }

    fn visit_item_struct(&mut self, item: &ItemStruct) {
        if matches!(
            item.ident.to_string().as_str(),
            "ItemRequirements" | "KeyedItem"
        ) {
            self.index
                .reimplemented_core_types
                .push((item.ident.to_string(), self.source.to_path_buf()));
        }
        let (fields, has_named_fields) = match &item.fields {
            Fields::Named(fields) => (
                fields
                    .named
                    .iter()
                    .filter_map(|field| {
                        field
                            .ident
                            .as_ref()
                            .map(|identifier| (identifier.to_string(), field.ty.clone()))
                    })
                    .collect(),
                true,
            ),
            Fields::Unnamed(_) | Fields::Unit => (Vec::new(), false),
        };
        self.index
            .structs
            .entry(item.ident.to_string())
            .or_default()
            .push(PublicStruct {
                fields,
                has_named_fields,
                is_public: matches!(item.vis, Visibility::Public(_)),
                source: self.source.to_path_buf(),
            });
        syn::visit::visit_item_struct(self, item);
    }

    fn visit_item_type(&mut self, item: &ItemType) {
        self.index
            .aliases
            .insert(item.ident.to_string(), item.ty.as_ref().clone());
        syn::visit::visit_item_type(self, item);
    }
}

struct MembershipFunctionVisitor<'a> {
    aliases: &'a BTreeMap<String, Type>,
    functions: &'a mut BTreeSet<String>,
}

impl Visit<'_> for MembershipFunctionVisitor<'_> {
    fn visit_item_fn(&mut self, item: &ItemFn) {
        if return_type_contains_membership(&item.sig.output, self.aliases) {
            let _ = self.functions.insert(item.sig.ident.to_string());
        }
        syn::visit::visit_item_fn(self, item);
    }

    fn visit_impl_item_fn(&mut self, item: &ImplItemFn) {
        if return_type_contains_membership(&item.sig.output, self.aliases) {
            let _ = self.functions.insert(item.sig.ident.to_string());
        }
        syn::visit::visit_impl_item_fn(self, item);
    }
}

pub fn check_repository_roots(roots: &[PathBuf]) -> Result<ArchitectureReport, ArchitectureError> {
    let mut report = ArchitectureReport::default();
    for parsed_crate in discover(roots)? {
        analyze_crate(&parsed_crate, &mut report);
    }
    report.roots.sort();
    report.roots.dedup();
    report.violations.sort();
    report.violations.dedup();
    Ok(report)
}

fn analyze_crate(parsed_crate: &ParsedCrate, report: &mut ArchitectureReport) {
    let index = build_index(parsed_crate);
    let is_requirement_crate = parsed_crate.crate_name.ends_with("-adapter")
        || parsed_crate.crate_name.ends_with("-engine")
        || !index.roots.is_empty();
    if is_requirement_crate {
        report_uninspectable_surface(parsed_crate, &index, report);
    }
    inspect_requirement_roots(parsed_crate, &index, report);
    if parsed_crate.crate_name.ends_with("-adapter")
        || index
            .roots
            .iter()
            .any(|(_, kind)| *kind == RequirementKind::Adapter)
    {
        report.violations.extend(adapter_violations(
            parsed_crate,
            &index.aliases,
            &index.local_macros,
            &index.membership_functions,
            &index.membership_field_names,
            &index.adapter_root_names,
        ));
    }
}

fn build_index(parsed_crate: &ParsedCrate) -> CrateIndex {
    let mut index = CrateIndex::default();
    for source in &parsed_crate.sources {
        IndexVisitor {
            index: &mut index,
            source: &source.path,
        }
        .visit_file(&source.syntax);
    }
    for source in &parsed_crate.sources {
        MembershipFunctionVisitor {
            aliases: &index.aliases,
            functions: &mut index.membership_functions,
        }
        .visit_file(&source.syntax);
    }
    index.membership_field_names = index
        .structs
        .values()
        .flatten()
        .flat_map(|item| &item.fields)
        .filter(|(_, rust_type)| {
            contains_item_requirements(rust_type, &index.aliases, &mut BTreeSet::new())
        })
        .map(|(name, _)| name.clone())
        .collect();
    index.roots.sort();
    index.roots.dedup();
    index.adapter_root_names = index
        .roots
        .iter()
        .filter(|(_, kind)| *kind == RequirementKind::Adapter)
        .map(|(name, _)| name.clone())
        .collect();
    index
}

fn inspect_requirement_roots(
    parsed_crate: &ParsedCrate,
    index: &CrateIndex,
    report: &mut ArchitectureReport,
) {
    for (name, kind) in &index.roots {
        let Some(structs) = index.structs.get(name) else {
            let detail = if index.aliases.contains_key(name) {
                "is a type alias; requirement roots must be public named-field structs"
            } else {
                "cannot be resolved to a public named-field struct"
            };
            report_noncanonical_root(
                parsed_crate,
                name,
                detail,
                parsed_crate.manifest.as_path(),
                report,
            );
            continue;
        };
        for public_struct in structs {
            if !public_struct.is_public {
                report_noncanonical_root(
                    parsed_crate,
                    name,
                    "is private; requirement roots must be public named-field structs",
                    &public_struct.source,
                    report,
                );
                continue;
            }
            if !public_struct.has_named_fields {
                report_noncanonical_root(
                    parsed_crate,
                    name,
                    "uses unnamed or unit fields; requirement roots must expose public named fields",
                    &public_struct.source,
                    report,
                );
                continue;
            }
            let membership_fields = inspect_fields(
                parsed_crate,
                index,
                name,
                public_struct,
                report,
                &mut BTreeSet::new(),
            );
            report.roots.push(RequirementRoot {
                crate_name: parsed_crate.crate_name.clone(),
                kind: *kind,
                manifest: display_path(&parsed_crate.repository_root, &parsed_crate.manifest),
                membership_fields,
                name: name.clone(),
                repository_root: parsed_crate.repository_root.display().to_string(),
                source: display_path(&parsed_crate.repository_root, &public_struct.source),
            });
        }
    }
}

fn return_type_contains_membership(output: &ReturnType, aliases: &BTreeMap<String, Type>) -> bool {
    let ReturnType::Type(_, rust_type) = output else {
        return false;
    };
    contains_explicit_membership(rust_type, aliases, &mut BTreeSet::new())
}

fn report_uninspectable_surface(
    parsed_crate: &ParsedCrate,
    index: &CrateIndex,
    report: &mut ArchitectureReport,
) {
    for (name, source) in &index.reimplemented_core_types {
        report.violations.push(ArchitectureViolation {
            code: ViolationCode::ReimplementedCoreVocabulary,
            crate_name: parsed_crate.crate_name.clone(),
            message: format!(
                "requirement crate reimplements core vocabulary type {name}; import the canonical type through the approved facade"
            ),
            source: display_path(&parsed_crate.repository_root, source),
        });
    }
    for (name, source) in &index.uninspectable_macros {
        let local_name = name.split_whitespace().last().unwrap_or(name);
        if index
            .local_macros
            .get(local_name)
            .is_some_and(|body| !macro_can_emit_requirement_root(body))
        {
            continue;
        }
        report.violations.push(ArchitectureViolation {
            code: ViolationCode::UninspectableRequirementMacro,
            crate_name: parsed_crate.crate_name.clone(),
            message: format!(
                "requirement crate invokes module-level macro {name}; requirement roots must remain visible to source inventory"
            ),
            source: display_path(&parsed_crate.repository_root, source),
        });
    }
}

fn macro_can_emit_requirement_root(body: &str) -> bool {
    body.contains("EngineRequirement") || body.contains("AdapterRequirement")
}

fn report_noncanonical_root(
    parsed_crate: &ParsedCrate,
    root_name: &str,
    detail: &str,
    source: &Path,
    report: &mut ArchitectureReport,
) {
    report.violations.push(ArchitectureViolation {
        code: ViolationCode::NonCanonicalRequirementRoot,
        crate_name: parsed_crate.crate_name.clone(),
        message: format!("public requirement root {root_name} {detail}"),
        source: display_path(&parsed_crate.repository_root, source),
    });
}

fn inspect_fields(
    parsed_crate: &ParsedCrate,
    index: &CrateIndex,
    root_name: &str,
    public_struct: &PublicStruct,
    report: &mut ArchitectureReport,
    visited: &mut BTreeSet<String>,
) -> Vec<MembershipField> {
    let visit_key = format!("{}::{root_name}", public_struct.source.display());
    if !visited.insert(visit_key) {
        return Vec::new();
    }
    let mut membership_fields = Vec::new();
    for (name, rust_type) in &public_struct.fields {
        let explicit =
            contains_explicit_membership(rust_type, &index.aliases, &mut BTreeSet::new());
        if explicit {
            membership_fields.push(MembershipField {
                name: name.clone(),
                rust_type: type_text(rust_type),
            });
        }
        if (has_closure_semantics(name, rust_type, &index.aliases)
            || names_key_membership(name)
            || contains_nested_membership(rust_type, &index.aliases, &mut BTreeSet::new()))
            && !explicit
        {
            report.violations.push(ArchitectureViolation {
                code: ViolationCode::SemanticClosureField,
                crate_name: parsed_crate.crate_name.clone(),
                message: format!(
                    "public requirement root {root_name} field {name} encodes closure without ItemRequirements<KeyedItem<()>>"
                ),
                source: display_path(&parsed_crate.repository_root, &public_struct.source),
            });
        }
        inspect_nested_fields(parsed_crate, index, rust_type, report, visited);
    }
    membership_fields.sort();
    membership_fields
}

fn inspect_nested_fields(
    parsed_crate: &ParsedCrate,
    index: &CrateIndex,
    rust_type: &Type,
    report: &mut ArchitectureReport,
    visited: &mut BTreeSet<String>,
) {
    for nested_name in type_names(rust_type, &index.aliases, &mut BTreeSet::new()) {
        let Some(nested_structs) = index.structs.get(&nested_name) else {
            continue;
        };
        if nested_structs.len() > 1 {
            report.violations.push(ArchitectureViolation {
                code: ViolationCode::NonCanonicalRequirementRoot,
                crate_name: parsed_crate.crate_name.clone(),
                message: format!(
                    "requirement surface references ambiguous local type {nested_name}; reachable local child type names must be unique"
                ),
                source: display_path(&parsed_crate.repository_root, &nested_structs[0].source),
            });
        }
        for nested_struct in nested_structs {
            let _ = inspect_fields(
                parsed_crate,
                index,
                &nested_name,
                nested_struct,
                report,
                visited,
            );
        }
    }
}

pub(crate) fn contains_item_requirements(
    rust_type: &Type,
    aliases: &BTreeMap<String, Type>,
    visited: &mut BTreeSet<String>,
) -> bool {
    match rust_type {
        Type::Group(group) => {
            return contains_item_requirements(&group.elem, aliases, visited);
        }
        Type::Paren(paren) => {
            return contains_item_requirements(&paren.elem, aliases, visited);
        }
        Type::Reference(reference) => {
            return contains_item_requirements(&reference.elem, aliases, visited);
        }
        _ => {}
    }
    let resolved = resolve_outer_alias(rust_type, aliases, visited);
    let Type::Path(type_path) = resolved else {
        return false;
    };
    type_path.path.segments.iter().any(|segment| {
        segment.ident == "ItemRequirements"
            || matches!(&segment.arguments, syn::PathArguments::AngleBracketed(arguments) if arguments.args.iter().any(|argument| matches!(argument, syn::GenericArgument::Type(nested) if contains_item_requirements(nested, aliases, visited))))
    })
}

fn names_key_membership(field_name: &str) -> bool {
    let normalized = field_name
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .flat_map(char::to_lowercase)
        .collect::<String>();
    normalized.ends_with("keys") || normalized.ends_with("membership")
}

fn contains_nested_membership(
    rust_type: &Type,
    aliases: &BTreeMap<String, Type>,
    visited: &mut BTreeSet<String>,
) -> bool {
    let resolved = resolve_outer_alias(rust_type, aliases, visited);
    if is_direct_membership(resolved, aliases) {
        return true;
    }
    let Type::Path(type_path) = resolved else {
        return false;
    };
    type_path.path.segments.iter().any(|segment| {
        let syn::PathArguments::AngleBracketed(arguments) = &segment.arguments else {
            return false;
        };
        arguments.args.iter().any(|argument| {
            matches!(argument, syn::GenericArgument::Type(nested) if contains_nested_membership(nested, aliases, visited))
        })
    })
}

fn has_closure_semantics(
    field_name: &str,
    rust_type: &Type,
    aliases: &BTreeMap<String, Type>,
) -> bool {
    semantic_closure_name(field_name)
        || type_names(rust_type, aliases, &mut BTreeSet::new())
            .iter()
            .any(|name| semantic_closure_name(name))
}

fn semantic_closure_name(name: &str) -> bool {
    let normalized = name
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .flat_map(char::to_lowercase)
        .collect::<String>();
    let closure = [
        "closed",
        "closure",
        "complete",
        "exact",
        "exclusive",
        "only",
    ]
    .iter()
    .any(|term| normalized.contains(term));
    let membership = [
        "collection",
        "field",
        "key",
        "member",
        "membership",
        "setting",
    ]
    .iter()
    .any(|term| normalized.contains(term));
    closure && membership
}

fn type_names(
    rust_type: &Type,
    aliases: &BTreeMap<String, Type>,
    visited: &mut BTreeSet<String>,
) -> Vec<String> {
    let mut names = Vec::new();
    let Type::Path(type_path) = rust_type else {
        return names;
    };
    for segment in &type_path.path.segments {
        let name = segment.ident.to_string();
        names.push(name.clone());
        names.extend(alias_type_names(&name, aliases, visited));
        names.extend(argument_type_names(&segment.arguments, aliases, visited));
    }
    names
}

fn alias_type_names(
    name: &str,
    aliases: &BTreeMap<String, Type>,
    visited: &mut BTreeSet<String>,
) -> Vec<String> {
    if !visited.insert(name.to_owned()) {
        return Vec::new();
    }
    aliases
        .get(name)
        .map_or_else(Vec::new, |alias| type_names(alias, aliases, visited))
}

fn argument_type_names(
    arguments: &syn::PathArguments,
    aliases: &BTreeMap<String, Type>,
    visited: &mut BTreeSet<String>,
) -> Vec<String> {
    let syn::PathArguments::AngleBracketed(arguments) = arguments else {
        return Vec::new();
    };
    arguments
        .args
        .iter()
        .filter_map(|argument| match argument {
            syn::GenericArgument::Type(nested) => Some(nested),
            _ => None,
        })
        .flat_map(|nested| type_names(nested, aliases, visited))
        .collect()
}

pub(crate) fn contains_explicit_membership(
    rust_type: &Type,
    aliases: &BTreeMap<String, Type>,
    visited: &mut BTreeSet<String>,
) -> bool {
    match rust_type {
        Type::Group(group) => {
            return contains_explicit_membership(&group.elem, aliases, visited);
        }
        Type::Paren(paren) => {
            return contains_explicit_membership(&paren.elem, aliases, visited);
        }
        Type::Reference(reference) => {
            return contains_explicit_membership(&reference.elem, aliases, visited);
        }
        _ => {}
    }
    let resolved = resolve_outer_alias(rust_type, aliases, visited);
    if is_direct_membership(resolved, aliases) {
        return true;
    }
    let Type::Path(type_path) = resolved else {
        return false;
    };
    let Some(segment) = type_path.path.segments.last() else {
        return false;
    };
    if segment.ident != "BTreeMap" {
        return false;
    }
    let syn::PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return false;
    };
    let types = arguments
        .args
        .iter()
        .filter_map(|argument| match argument {
            syn::GenericArgument::Type(nested) => Some(nested),
            _ => None,
        })
        .collect::<Vec<_>>();
    types.len() == 2 && is_direct_membership(types[1], aliases)
}

fn is_direct_membership(rust_type: &Type, aliases: &BTreeMap<String, Type>) -> bool {
    let resolved = resolve_outer_alias(rust_type, aliases, &mut BTreeSet::new());
    let Type::Path(type_path) = resolved else {
        return false;
    };
    let Some(segment) = type_path.path.segments.last() else {
        return false;
    };
    if resolve_alias_name(&segment.ident.to_string(), aliases) != "ItemRequirements" {
        return false;
    }
    let syn::PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return false;
    };
    let types = arguments
        .args
        .iter()
        .filter_map(|argument| match argument {
            syn::GenericArgument::Type(nested) => Some(nested),
            _ => None,
        })
        .collect::<Vec<_>>();
    types.len() == 1 && is_keyed_unit(types[0], aliases, &mut BTreeSet::new())
}

fn resolve_outer_alias<'a>(
    rust_type: &'a Type,
    aliases: &'a BTreeMap<String, Type>,
    visited: &mut BTreeSet<String>,
) -> &'a Type {
    let Some(name) = outer_type_name(rust_type) else {
        return rust_type;
    };
    if !visited.insert(name.clone()) {
        return rust_type;
    }
    aliases.get(&name).map_or(rust_type, |alias| {
        if type_has_arguments(rust_type) && !type_has_arguments(alias) {
            rust_type
        } else {
            resolve_outer_alias(alias, aliases, visited)
        }
    })
}

fn is_keyed_unit(
    rust_type: &Type,
    aliases: &BTreeMap<String, Type>,
    visited: &mut BTreeSet<String>,
) -> bool {
    let resolved = resolve_outer_alias(rust_type, aliases, visited);
    let Type::Path(type_path) = resolved else {
        return false;
    };
    let Some(segment) = type_path.path.segments.last() else {
        return false;
    };
    if resolve_alias_name(&segment.ident.to_string(), aliases) != "KeyedItem" {
        return false;
    }
    let syn::PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return false;
    };
    arguments.args.len() == 1
        && matches!(
            arguments.args.first(),
            Some(syn::GenericArgument::Type(Type::Tuple(tuple))) if tuple.elems.is_empty()
        )
}

fn collect_trait_aliases(tree: &UseTree, aliases: &mut BTreeMap<String, String>) {
    match tree {
        UseTree::Rename(rename) => {
            let source = rename.ident.to_string();
            aliases.insert(rename.rename.to_string(), source);
        }
        UseTree::Path(path) => collect_trait_aliases(&path.tree, aliases),
        UseTree::Group(group) => {
            for item in &group.items {
                collect_trait_aliases(item, aliases);
            }
        }
        UseTree::Name(_) | UseTree::Glob(_) => {}
    }
}

fn collect_type_import_aliases(tree: &UseTree, aliases: &mut BTreeMap<String, Type>) {
    match tree {
        UseTree::Rename(rename) => {
            if let Ok(source) = syn::parse_str::<Type>(&rename.ident.to_string()) {
                aliases.insert(rename.rename.to_string(), source);
            }
        }
        UseTree::Path(path) => collect_type_import_aliases(&path.tree, aliases),
        UseTree::Group(group) => {
            for item in &group.items {
                collect_type_import_aliases(item, aliases);
            }
        }
        UseTree::Name(_) | UseTree::Glob(_) => {}
    }
}

fn resolve_alias_name(name: &str, aliases: &BTreeMap<String, Type>) -> String {
    let mut current = name.to_owned();
    let mut visited = BTreeSet::new();
    while visited.insert(current.clone()) {
        let Some(next) = aliases.get(&current).and_then(outer_type_name) else {
            break;
        };
        current = next;
    }
    current
}

fn type_has_arguments(rust_type: &Type) -> bool {
    let Type::Path(type_path) = rust_type else {
        return false;
    };
    type_path
        .path
        .segments
        .last()
        .is_some_and(|segment| matches!(segment.arguments, syn::PathArguments::AngleBracketed(_)))
}

fn resolve_trait_alias(name: &str, aliases: &BTreeMap<String, String>) -> String {
    let mut current = name.to_owned();
    let mut visited = BTreeSet::new();
    while visited.insert(current.clone()) {
        let Some(next) = aliases.get(&current) else {
            break;
        };
        current.clone_from(next);
    }
    current
}

pub(crate) fn type_name(rust_type: &Type) -> Option<String> {
    let Type::Path(type_path) = rust_type else {
        return None;
    };
    type_path
        .path
        .segments
        .last()
        .map(|segment| segment.ident.to_string())
}

pub(crate) fn outer_type_name(rust_type: &Type) -> Option<String> {
    type_name(rust_type)
}

pub(crate) fn type_text(rust_type: &Type) -> String {
    rust_type.to_token_stream().to_string()
}

pub(crate) fn display_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}

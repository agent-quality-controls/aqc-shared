use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use quote::ToTokens;
use syn::visit::Visit;
use syn::{
    Fields, ImplItemFn, ItemFn, ItemMacro, ItemStruct, ItemTrait, ItemType, ItemUse, ReturnType,
    Type, UseTree, Visibility,
};

use crate::discover::{ParsedCrate, discover};
use crate::expression::{AdapterSurface, adapter_violations};
use crate::model::{
    ArchitectureError, ArchitectureReport, ArchitectureViolation, MembershipField, RequirementKind,
    RequirementRoot, ViolationCode,
};

struct PublicStruct {
    fields: Vec<(String, Type)>,
    has_named_fields: bool,
    is_public: bool,
    scope: DeclarationScope,
    source: PathBuf,
}

#[derive(Clone, Eq, Ord, PartialEq, PartialOrd)]
struct DeclarationScope {
    module_path: Vec<String>,
    source: PathBuf,
}

#[derive(Clone, Default)]
pub(crate) struct CanonicalVocabulary {
    item_names: BTreeSet<String>,
    item_providers: BTreeSet<String>,
    keyed_names: BTreeSet<String>,
    keyed_providers: BTreeSet<String>,
}

impl CanonicalVocabulary {
    pub(crate) fn is_item_path(&self, path: &syn::Path) -> bool {
        path_is_canonical(path, self, CanonicalType::ItemRequirements)
    }

    pub(crate) fn is_item_name(&self, name: &str) -> bool {
        self.item_names.contains(name)
    }

    pub(crate) fn is_item_constructor_path(&self, path: &syn::Path) -> bool {
        let mut item_path = path.clone();
        let _ = item_path.segments.pop();
        self.is_item_path(&item_path)
    }
}

struct TypeImport {
    local: String,
    module_path: Vec<String>,
    public: bool,
    source: Vec<String>,
}

pub(crate) type ScopedImports = BTreeMap<Vec<String>, BTreeMap<String, Vec<String>>>;

#[derive(Default)]
pub(crate) struct CrateIndex {
    aliases_by_module: BTreeMap<Vec<String>, BTreeMap<String, Type>>,
    type_aliases: BTreeMap<Vec<String>, Type>,
    local_macros: BTreeMap<String, String>,
    requirement_trait_visible: bool,
    requirement_trait_imported: bool,
    roots: Vec<(String, RequirementKind)>,
    root_scopes: BTreeMap<(String, RequirementKind), BTreeSet<DeclarationScope>>,
    structs: BTreeMap<String, Vec<PublicStruct>>,
    core_vocabulary_aliases: Vec<(String, String, PathBuf)>,
    reimplemented_core_types: Vec<(String, PathBuf)>,
    uninspectable_macros: Vec<(String, String, PathBuf)>,
    uninspectable_imports: Vec<(String, PathBuf)>,
    membership_field_names: BTreeSet<String>,
    membership_struct_paths: BTreeSet<Vec<String>>,
    membership_functions: BTreeSet<String>,
    adapter_root_paths: BTreeSet<Vec<String>>,
    requirement_trait_paths: BTreeMap<Vec<String>, RequirementKind>,
    scoped_imports: ScopedImports,
    imports: Vec<TypeImport>,
    vocabulary: CanonicalVocabulary,
}

struct IndexVisitor<'a> {
    block_depth: usize,
    dependencies: &'a BTreeSet<String>,
    index: &'a mut CrateIndex,
    module_path: Vec<String>,
    source: &'a Path,
}

struct MembershipFunctionVisitor<'a> {
    aliases_by_module: &'a BTreeMap<Vec<String>, BTreeMap<String, Type>>,
    empty_aliases: BTreeMap<String, Type>,
    functions: &'a mut BTreeSet<String>,
    module_path: Vec<String>,
    vocabulary: &'a CanonicalVocabulary,
}

impl Visit<'_> for MembershipFunctionVisitor<'_> {
    fn visit_item_mod(&mut self, item: &syn::ItemMod) {
        if item.content.is_some() {
            self.module_path.push(item.ident.to_string());
            syn::visit::visit_item_mod(self, item);
            let _ = self.module_path.pop();
        }
    }

    fn visit_item_fn(&mut self, item: &ItemFn) {
        if return_type_contains_membership(&item.sig.output, self.aliases(), self.vocabulary) {
            self.functions.insert(item.sig.ident.to_string());
        }
        syn::visit::visit_item_fn(self, item);
    }

    fn visit_impl_item_fn(&mut self, item: &ImplItemFn) {
        if return_type_contains_membership(&item.sig.output, self.aliases(), self.vocabulary) {
            self.functions.insert(item.sig.ident.to_string());
        }
        syn::visit::visit_impl_item_fn(self, item);
    }
}

impl MembershipFunctionVisitor<'_> {
    fn aliases(&self) -> &BTreeMap<String, Type> {
        self.aliases_by_module
            .get(&self.module_path)
            .unwrap_or(&self.empty_aliases)
    }
}

impl Visit<'_> for IndexVisitor<'_> {
    fn visit_block(&mut self, block: &syn::Block) {
        self.block_depth += 1;
        syn::visit::visit_block(self, block);
        self.block_depth -= 1;
    }

    fn visit_item_mod(&mut self, item: &syn::ItemMod) {
        if item.content.is_some() {
            self.module_path.push(item.ident.to_string());
            self.index
                .aliases_by_module
                .entry(self.module_path.clone())
                .or_default();
            syn::visit::visit_item_mod(self, item);
            let _ = self.module_path.pop();
        }
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
                item.mac.tokens.to_token_stream().to_string(),
                self.source.to_path_buf(),
            ));
        }
        syn::visit::visit_item_macro(self, item);
    }

    fn visit_item_use(&mut self, item: &ItemUse) {
        if self.block_depth > 0 || use_tree_has_glob(&item.tree) {
            self.index.uninspectable_imports.push((
                item.to_token_stream().to_string(),
                self.source.to_path_buf(),
            ));
            return;
        }
        if use_tree_imports_requirement_trait(&item.tree) {
            self.index.requirement_trait_imported = true;
        }
        collect_core_vocabulary_aliases(
            &item.tree,
            self.source,
            &mut self.index.core_vocabulary_aliases,
        );
        let mut imports = Vec::new();
        collect_type_imports(
            &item.tree,
            &mut Vec::new(),
            &self.module_path,
            matches!(item.vis, Visibility::Public(_)),
            &mut imports,
        );
        for import in imports {
            let mut scoped_source = import.source.clone();
            if item.leading_colon.is_some()
                || scoped_source
                    .first()
                    .is_some_and(|name| self.dependencies.contains(name))
            {
                scoped_source.insert(0, "::".to_owned());
            }
            self.index
                .scoped_imports
                .entry(self.module_path.clone())
                .or_default()
                .insert(import.local.clone(), scoped_source);
            self.index.imports.push(import);
        }
        syn::visit::visit_item_use(self, item);
    }

    fn visit_item_trait(&mut self, item: &ItemTrait) {
        if matches!(
            item.ident.to_string().as_str(),
            "EngineRequirement" | "AdapterRequirement"
        ) {
            self.index.requirement_trait_visible = true;
            let mut path = self.module_path.clone();
            path.push(item.ident.to_string());
            let kind = if item.ident == "EngineRequirement" {
                RequirementKind::Engine
            } else {
                RequirementKind::Adapter
            };
            self.index
                .requirement_trait_paths
                .insert(path.clone(), kind);
        }
        syn::visit::visit_item_trait(self, item);
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
                scope: DeclarationScope {
                    module_path: self.module_path.clone(),
                    source: self.source.to_path_buf(),
                },
                source: self.source.to_path_buf(),
            });
        syn::visit::visit_item_struct(self, item);
    }

    fn visit_item_type(&mut self, item: &ItemType) {
        let mut declaration_path = self.module_path.clone();
        declaration_path.push(item.ident.to_string());
        self.index
            .type_aliases
            .insert(declaration_path, item.ty.as_ref().clone());
        self.index
            .aliases_by_module
            .entry(self.module_path.clone())
            .or_default()
            .insert(item.ident.to_string(), item.ty.as_ref().clone());
        syn::visit::visit_item_type(self, item);
    }
}

pub fn check_repository_roots(
    core_manifest: &Path,
    roots: &[PathBuf],
) -> Result<ArchitectureReport, ArchitectureError> {
    let mut report = ArchitectureReport::default();
    let core_manifest = crate::fs::canonicalize(core_manifest)?;
    let parsed_crates = discover(roots)?;
    let mut indexes = parsed_crates.iter().map(build_index).collect::<Vec<_>>();
    assign_canonical_vocabularies(&parsed_crates, &mut indexes, &core_manifest);
    assign_dependency_requirement_traits(&parsed_crates, &mut indexes, &core_manifest);
    for (parsed_crate, index) in parsed_crates.iter().zip(&mut indexes) {
        finish_index(parsed_crate, index);
    }
    let engine_root_names = indexes
        .iter()
        .flat_map(|index| &index.roots)
        .filter(|(_, kind)| *kind == RequirementKind::Engine)
        .map(|(name, _)| name.clone())
        .collect::<BTreeSet<_>>();
    for (parsed_crate, index) in parsed_crates.iter().zip(&indexes) {
        analyze_crate(
            parsed_crate,
            index,
            &engine_root_names,
            &core_manifest,
            &mut report,
        );
    }
    report.roots.sort();
    report.roots.dedup();
    report.violations.sort();
    report.violations.dedup();
    Ok(report)
}

fn analyze_crate(
    parsed_crate: &ParsedCrate,
    index: &CrateIndex,
    engine_root_names: &BTreeSet<String>,
    core_manifest: &Path,
    report: &mut ArchitectureReport,
) {
    let is_requirement_crate = parsed_crate.crate_name.ends_with("-adapter")
        || parsed_crate.crate_name.ends_with("-engine")
        || !index.roots.is_empty()
        || index.requirement_trait_visible
        || index.requirement_trait_imported
        || index
            .local_macros
            .values()
            .any(|body| macro_can_emit_requirement_root(body));
    if is_requirement_crate {
        report_uninspectable_surface(parsed_crate, index, core_manifest, report);
    }
    inspect_requirement_roots(parsed_crate, index, report);
    if parsed_crate.crate_name.ends_with("-adapter")
        || index
            .roots
            .iter()
            .any(|(_, kind)| *kind == RequirementKind::Adapter)
    {
        report.violations.extend(adapter_violations(
            parsed_crate,
            &AdapterSurface {
                aliases_by_module: &index.aliases_by_module,
                vocabulary: &index.vocabulary,
                membership_functions: &index.membership_functions,
                local_macros: &index.local_macros,
                membership_field_names: &index.membership_field_names,
                membership_struct_paths: &index.membership_struct_paths,
                adapter_root_paths: &index.adapter_root_paths,
                scoped_imports: &index.scoped_imports,
                type_aliases: &index.type_aliases,
                engine_root_names,
            },
        ));
    }
}

fn build_index(parsed_crate: &ParsedCrate) -> CrateIndex {
    let mut index = CrateIndex::default();
    let dependencies = parsed_crate.dependencies.keys().cloned().collect();
    for source in &parsed_crate.sources {
        index
            .aliases_by_module
            .entry(source.module_path.clone())
            .or_default();
        IndexVisitor {
            block_depth: 0,
            dependencies: &dependencies,
            index: &mut index,
            module_path: source.module_path.clone(),
            source: &source.path,
        }
        .visit_file(&source.syntax);
    }
    index
}

fn add_imported_type_aliases(index: &mut CrateIndex) {
    for (module_path, imports) in &index.scoped_imports {
        for (local, source) in imports {
            let resolved = resolve_scoped_segments(
                source,
                module_path,
                &index.scoped_imports,
                &mut BTreeSet::new(),
            );
            let Some(alias) = index.type_aliases.get(&resolved) else {
                continue;
            };
            index
                .aliases_by_module
                .entry(module_path.clone())
                .or_default()
                .insert(local.clone(), alias.clone());
        }
    }
}

fn finish_index(parsed_crate: &ParsedCrate, index: &mut CrateIndex) {
    add_imported_type_aliases(index);
    for source in &parsed_crate.sources {
        collect_scoped_requirement_roots(
            &source.syntax.items,
            &source.path,
            &mut source.module_path.clone(),
            &index.scoped_imports,
            &index.requirement_trait_paths,
            &mut index.roots,
            &mut index.root_scopes,
        );
    }
    for source in &parsed_crate.sources {
        MembershipFunctionVisitor {
            aliases_by_module: &index.aliases_by_module,
            empty_aliases: BTreeMap::new(),
            functions: &mut index.membership_functions,
            module_path: source.module_path.clone(),
            vocabulary: &index.vocabulary,
        }
        .visit_file(&source.syntax);
    }
    let (membership_field_names, membership_struct_paths) = collect_membership_surface(index);
    index.membership_field_names = membership_field_names;
    index.membership_struct_paths = membership_struct_paths;
    index.roots.sort();
    index.roots.dedup();
    index.adapter_root_paths = index
        .root_scopes
        .iter()
        .filter(|((_, kind), _)| *kind == RequirementKind::Adapter)
        .flat_map(|((name, _), scopes)| {
            scopes.iter().map(|scope| {
                let mut path = scope.module_path.clone();
                path.push(name.clone());
                path
            })
        })
        .collect();
}

fn assign_dependency_requirement_traits(
    parsed_crates: &[ParsedCrate],
    indexes: &mut [CrateIndex],
    core_manifest: &Path,
) {
    let mut exports = parsed_crates
        .iter()
        .map(|parsed_crate| (parsed_crate.manifest.clone(), BTreeMap::new()))
        .collect::<BTreeMap<_, _>>();
    loop {
        let mut changed = false;
        for (parsed_crate, index) in parsed_crates.iter().zip(indexes.iter()) {
            changed |= add_public_requirement_trait_exports(parsed_crate, index, &mut exports);
        }
        for (parsed_crate, index) in parsed_crates.iter().zip(indexes.iter_mut()) {
            changed |= add_dependency_requirement_trait_paths(
                parsed_crate,
                index,
                parsed_crates,
                &exports,
                core_manifest,
            );
        }
        if !changed {
            break;
        }
    }
}

fn add_public_requirement_trait_exports(
    parsed_crate: &ParsedCrate,
    index: &CrateIndex,
    exports: &mut BTreeMap<PathBuf, BTreeMap<Vec<String>, RequirementKind>>,
) -> bool {
    let exported = exports.entry(parsed_crate.manifest.clone()).or_default();
    let mut changed = false;
    for import in index.imports.iter().filter(|import| import.public) {
        let resolved = resolve_scoped_segments(
            &import.source,
            &import.module_path,
            &index.scoped_imports,
            &mut BTreeSet::new(),
        );
        let Some(kind) = index.requirement_trait_paths.get(&resolved) else {
            continue;
        };
        let mut path = import.module_path.clone();
        path.push(import.local.clone());
        changed |= exported.insert(path, *kind) != Some(*kind);
    }
    changed
}

fn add_dependency_requirement_trait_paths(
    parsed_crate: &ParsedCrate,
    index: &mut CrateIndex,
    parsed_crates: &[ParsedCrate],
    exports: &BTreeMap<PathBuf, BTreeMap<Vec<String>, RequirementKind>>,
    core_manifest: &Path,
) -> bool {
    let mut changed = false;
    for (local, dependency) in &parsed_crate.dependencies {
        let Some(manifest) =
            requirement_dependency_manifest(dependency, parsed_crates, exports, core_manifest)
        else {
            continue;
        };
        let Some(exported) = exports.get(manifest) else {
            continue;
        };
        for (path, kind) in exported {
            let mut dependency_path = vec![local.clone()];
            dependency_path.extend(path.iter().cloned());
            changed |= index.requirement_trait_paths.insert(dependency_path, *kind) != Some(*kind);
        }
    }
    changed
}

fn requirement_dependency_manifest<'a>(
    dependency: &crate::discover::ParsedDependency,
    parsed_crates: &'a [ParsedCrate],
    exports: &'a BTreeMap<PathBuf, BTreeMap<Vec<String>, RequirementKind>>,
    core_manifest: &'a Path,
) -> Option<&'a PathBuf> {
    if let Some(manifest) = &dependency.manifest {
        return exports.get_key_value(manifest).map(|(path, _)| path);
    }
    if dependency.default_registry && dependency.package == "aqc-file-engine-core" {
        return exports.get_key_value(core_manifest).map(|(path, _)| path);
    }
    let mut manifests = parsed_crates
        .iter()
        .filter(|item| item.crate_name == dependency.package)
        .map(|item| &item.manifest);
    let manifest = manifests.next()?;
    if manifests.next().is_some() {
        return None;
    }
    exports.get_key_value(manifest).map(|(path, _)| path)
}

fn collect_membership_surface(index: &CrateIndex) -> (BTreeSet<String>, BTreeSet<Vec<String>>) {
    let mut field_names = BTreeSet::new();
    let mut struct_paths = BTreeSet::new();
    for (struct_name, declarations) in &index.structs {
        for item in declarations {
            let aliases = index
                .aliases_by_module
                .get(&item.scope.module_path)
                .expect("Every parsed module has an alias scope");
            let membership_fields = item.fields.iter().filter(|(_, rust_type)| {
                contains_item_requirements(
                    rust_type,
                    aliases,
                    &index.vocabulary,
                    &mut BTreeSet::new(),
                )
            });
            for (field_name, _) in membership_fields {
                field_names.insert(field_name.clone());
                let mut path = item.scope.module_path.clone();
                path.push(struct_name.clone());
                struct_paths.insert(path);
            }
        }
    }
    (field_names, struct_paths)
}

fn assign_canonical_vocabularies(
    parsed_crates: &[ParsedCrate],
    indexes: &mut [CrateIndex],
    core_manifest: &Path,
) {
    let mut exports = BTreeMap::<PathBuf, CanonicalVocabulary>::new();
    exports.insert(
        core_manifest.to_path_buf(),
        CanonicalVocabulary {
            item_names: BTreeSet::from(["ItemRequirements".to_owned()]),
            keyed_names: BTreeSet::from(["KeyedItem".to_owned()]),
            ..CanonicalVocabulary::default()
        },
    );
    loop {
        let mut changed = false;
        for (parsed_crate, index) in parsed_crates.iter().zip(indexes.iter()) {
            let vocabulary =
                vocabulary_for(parsed_crate, index, parsed_crates, &exports, core_manifest);
            changed |= add_public_exports(
                parsed_crate,
                index,
                parsed_crates,
                &vocabulary,
                &mut exports,
                core_manifest,
            );
        }
        if !changed {
            break;
        }
    }
    for (parsed_crate, index) in parsed_crates.iter().zip(indexes) {
        index.vocabulary =
            vocabulary_for(parsed_crate, index, parsed_crates, &exports, core_manifest);
    }
}

fn add_public_exports(
    parsed_crate: &ParsedCrate,
    index: &CrateIndex,
    parsed_crates: &[ParsedCrate],
    vocabulary: &CanonicalVocabulary,
    exports: &mut BTreeMap<PathBuf, CanonicalVocabulary>,
    core_manifest: &Path,
) -> bool {
    let additions = index
        .imports
        .iter()
        .filter(|import| import.public)
        .filter_map(|import| {
            let source_name = import.source.last()?;
            let kinds = canonical_import_source(
                parsed_crate,
                import,
                source_name,
                parsed_crates,
                vocabulary,
                exports,
                core_manifest,
            );
            Some((import.local.clone(), kinds))
        })
        .fold(
            CanonicalVocabulary::default(),
            |mut additions, (name, kinds)| {
                if kinds.0 {
                    additions.item_names.insert(name.clone());
                }
                if kinds.1 {
                    additions.keyed_names.insert(name);
                }
                additions
            },
        );
    let crate_exports = exports.entry(parsed_crate.manifest.clone()).or_default();
    let mut changed = false;
    for name in additions.item_names {
        changed |= crate_exports.item_names.insert(name);
    }
    for name in additions.keyed_names {
        changed |= crate_exports.keyed_names.insert(name);
    }
    changed
}

fn vocabulary_for(
    parsed_crate: &ParsedCrate,
    index: &CrateIndex,
    parsed_crates: &[ParsedCrate],
    exports: &BTreeMap<PathBuf, CanonicalVocabulary>,
    core_manifest: &Path,
) -> CanonicalVocabulary {
    let mut vocabulary = CanonicalVocabulary::default();
    if parsed_crate.manifest == core_manifest {
        vocabulary.item_names.insert("ItemRequirements".to_owned());
        vocabulary.keyed_names.insert("KeyedItem".to_owned());
    }
    for (local, dependency) in &parsed_crate.dependencies {
        let provided = dependency_export(dependency, parsed_crates, exports, core_manifest);
        if provided.is_some_and(|items| !items.item_names.is_empty()) {
            vocabulary.item_providers.insert(local.clone());
        }
        if provided.is_some_and(|items| !items.keyed_names.is_empty()) {
            vocabulary.keyed_providers.insert(local.clone());
        }
    }
    loop {
        let previous = vocabulary.clone();
        for import in &index.imports {
            apply_canonical_import(
                parsed_crate,
                import,
                parsed_crates,
                exports,
                core_manifest,
                &mut vocabulary,
            );
        }
        if vocabulary.item_names == previous.item_names
            && vocabulary.keyed_names == previous.keyed_names
            && vocabulary.item_providers == previous.item_providers
            && vocabulary.keyed_providers == previous.keyed_providers
        {
            break;
        }
    }
    vocabulary
}

fn apply_canonical_import(
    parsed_crate: &ParsedCrate,
    import: &TypeImport,
    parsed_crates: &[ParsedCrate],
    exports: &BTreeMap<PathBuf, CanonicalVocabulary>,
    core_manifest: &Path,
    vocabulary: &mut CanonicalVocabulary,
) {
    if let [source] = import.source.as_slice() {
        if vocabulary.item_providers.contains(source) {
            vocabulary.item_providers.insert(import.local.clone());
        }
        if vocabulary.keyed_providers.contains(source) {
            vocabulary.keyed_providers.insert(import.local.clone());
        }
    }
    let Some(source_name) = import.source.last() else {
        return;
    };
    let (is_item, is_keyed) = canonical_import_source(
        parsed_crate,
        import,
        source_name,
        parsed_crates,
        vocabulary,
        exports,
        core_manifest,
    );
    if is_item {
        vocabulary.item_names.insert(import.local.clone());
    }
    if is_keyed {
        vocabulary.keyed_names.insert(import.local.clone());
    }
}

fn canonical_import_source(
    parsed_crate: &ParsedCrate,
    import: &TypeImport,
    source_name: &str,
    parsed_crates: &[ParsedCrate],
    vocabulary: &CanonicalVocabulary,
    exports: &BTreeMap<PathBuf, CanonicalVocabulary>,
    core_manifest: &Path,
) -> (bool, bool) {
    let Some(first) = import.source.first() else {
        return (false, false);
    };
    let local = matches!(first.as_str(), "crate" | "self" | "super");
    if local {
        return (
            vocabulary.item_names.contains(source_name),
            vocabulary.keyed_names.contains(source_name),
        );
    }
    parsed_crate
        .dependencies
        .get(first)
        .and_then(|dependency| dependency_export(dependency, parsed_crates, exports, core_manifest))
        .map_or((false, false), |items| {
            let item =
                vocabulary.item_providers.contains(first) && items.item_names.contains(source_name);
            let keyed = vocabulary.keyed_providers.contains(first)
                && items.keyed_names.contains(source_name);
            (item, keyed)
        })
}

fn dependency_export<'a>(
    dependency: &crate::discover::ParsedDependency,
    parsed_crates: &[ParsedCrate],
    exports: &'a BTreeMap<PathBuf, CanonicalVocabulary>,
    core_manifest: &Path,
) -> Option<&'a CanonicalVocabulary> {
    if let Some(manifest) = &dependency.manifest {
        return exports.get(manifest);
    }
    if !dependency.default_registry {
        return None;
    }
    if dependency.package == "aqc-file-engine-core" && !core_manifest.as_os_str().is_empty() {
        return exports.get(core_manifest);
    }
    let mut manifests = parsed_crates
        .iter()
        .filter(|item| item.crate_name == dependency.package)
        .map(|item| &item.manifest);
    let manifest = manifests.next()?;
    if manifests.next().is_some() {
        return None;
    }
    exports.get(manifest)
}

fn inspect_requirement_roots(
    parsed_crate: &ParsedCrate,
    index: &CrateIndex,
    report: &mut ArchitectureReport,
) {
    for (name, kind) in &index.roots {
        let scopes = index.root_scopes.get(&(name.clone(), *kind));
        let Some(structs) = index.structs.get(name) else {
            let detail = if root_is_type_alias(name, scopes, index) {
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
        let matching_structs = structs
            .iter()
            .filter(|item| scopes.is_some_and(|root_scopes| root_scopes.contains(&item.scope)))
            .collect::<Vec<_>>();
        if matching_structs.is_empty() {
            report_noncanonical_root(
                parsed_crate,
                name,
                "cannot be resolved to a public named-field struct in its declaring module",
                parsed_crate.manifest.as_path(),
                report,
            );
            continue;
        }
        for public_struct in matching_structs {
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

fn root_is_type_alias(
    name: &str,
    scopes: Option<&BTreeSet<DeclarationScope>>,
    index: &CrateIndex,
) -> bool {
    scopes.is_some_and(|root_scopes| {
        root_scopes.iter().any(|scope| {
            index
                .aliases_by_module
                .get(&scope.module_path)
                .is_some_and(|aliases| aliases.contains_key(name))
        })
    })
}

fn return_type_contains_membership(
    output: &ReturnType,
    aliases: &BTreeMap<String, Type>,
    vocabulary: &CanonicalVocabulary,
) -> bool {
    let ReturnType::Type(_, rust_type) = output else {
        return false;
    };
    contains_explicit_membership(rust_type, aliases, vocabulary, &mut BTreeSet::new())
}

fn report_uninspectable_surface(
    parsed_crate: &ParsedCrate,
    index: &CrateIndex,
    core_manifest: &Path,
    report: &mut ArchitectureReport,
) {
    if parsed_crate.manifest != core_manifest {
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
    }
    for (source_name, local_name, source) in &index.core_vocabulary_aliases {
        report.violations.push(ArchitectureViolation {
            code: ViolationCode::ReimplementedCoreVocabulary,
            crate_name: parsed_crate.crate_name.clone(),
            message: format!(
                "requirement crate renames core vocabulary type {source_name} to {local_name}; use the established core name"
            ),
            source: display_path(&parsed_crate.repository_root, source),
        });
    }
    for (name, invocation, source) in &index.uninspectable_macros {
        let local_name = name.split_whitespace().last().unwrap_or(name);
        if index.local_macros.get(local_name).is_some_and(|body| {
            !macro_can_emit_requirement_root(body) && !macro_can_emit_requirement_root(invocation)
        }) {
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
    for (item, source) in &index.uninspectable_imports {
        report.violations.push(ArchitectureViolation {
            code: ViolationCode::UninspectableRequirementImport,
            crate_name: parsed_crate.crate_name.clone(),
            message: format!(
                "requirement crate uses opaque import {item}; imports must be explicit module-level bindings"
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
    let empty_aliases = BTreeMap::new();
    let aliases = index
        .aliases_by_module
        .get(&public_struct.scope.module_path)
        .unwrap_or(&empty_aliases);
    for (name, rust_type) in &public_struct.fields {
        let explicit = contains_explicit_membership(
            rust_type,
            aliases,
            &index.vocabulary,
            &mut BTreeSet::new(),
        );
        if explicit {
            membership_fields.push(MembershipField {
                name: name.clone(),
                rust_type: type_text(rust_type),
            });
        }
        if (has_closure_semantics(name, rust_type, aliases)
            || contains_nested_membership(
                rust_type,
                aliases,
                &index.vocabulary,
                &mut BTreeSet::new(),
            ))
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
        inspect_nested_fields(parsed_crate, index, rust_type, aliases, report, visited);
    }
    membership_fields.sort();
    membership_fields
}

fn inspect_nested_fields(
    parsed_crate: &ParsedCrate,
    index: &CrateIndex,
    rust_type: &Type,
    aliases: &BTreeMap<String, Type>,
    report: &mut ArchitectureReport,
    visited: &mut BTreeSet<String>,
) {
    for nested_name in type_names(rust_type, aliases, &mut BTreeSet::new()) {
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
    vocabulary: &CanonicalVocabulary,
    visited: &mut BTreeSet<String>,
) -> bool {
    match rust_type {
        Type::Group(group) => {
            return contains_item_requirements(&group.elem, aliases, vocabulary, visited);
        }
        Type::Paren(paren) => {
            return contains_item_requirements(&paren.elem, aliases, vocabulary, visited);
        }
        Type::Reference(reference) => {
            return contains_item_requirements(&reference.elem, aliases, vocabulary, visited);
        }
        _ => {}
    }
    let resolved = resolve_outer_alias(rust_type, aliases, visited);
    let Type::Path(type_path) = resolved else {
        return false;
    };
    path_is_canonical(&type_path.path, vocabulary, CanonicalType::ItemRequirements)
        || type_path.path.segments.iter().any(|segment| {
            matches!(&segment.arguments, syn::PathArguments::AngleBracketed(arguments) if arguments.args.iter().any(|argument| matches!(argument, syn::GenericArgument::Type(nested) if contains_item_requirements(nested, aliases, vocabulary, visited))))
        })
}

fn contains_nested_membership(
    rust_type: &Type,
    aliases: &BTreeMap<String, Type>,
    vocabulary: &CanonicalVocabulary,
    visited: &mut BTreeSet<String>,
) -> bool {
    let resolved = resolve_outer_alias(rust_type, aliases, visited);
    if is_direct_membership(resolved, aliases, vocabulary) {
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
            matches!(argument, syn::GenericArgument::Type(nested) if contains_nested_membership(nested, aliases, vocabulary, visited))
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
    vocabulary: &CanonicalVocabulary,
    visited: &mut BTreeSet<String>,
) -> bool {
    match rust_type {
        Type::Group(group) => {
            return contains_explicit_membership(&group.elem, aliases, vocabulary, visited);
        }
        Type::Paren(paren) => {
            return contains_explicit_membership(&paren.elem, aliases, vocabulary, visited);
        }
        Type::Reference(reference) => {
            return contains_explicit_membership(&reference.elem, aliases, vocabulary, visited);
        }
        _ => {}
    }
    let resolved = resolve_outer_alias(rust_type, aliases, visited);
    if is_direct_membership(resolved, aliases, vocabulary) {
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
    types.len() == 2 && is_direct_membership(types[1], aliases, vocabulary)
}

fn is_direct_membership(
    rust_type: &Type,
    aliases: &BTreeMap<String, Type>,
    vocabulary: &CanonicalVocabulary,
) -> bool {
    let resolved = resolve_outer_alias(rust_type, aliases, &mut BTreeSet::new());
    let Type::Path(type_path) = resolved else {
        return false;
    };
    let Some(segment) = type_path.path.segments.last() else {
        return false;
    };
    if !path_is_canonical(&type_path.path, vocabulary, CanonicalType::ItemRequirements) {
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
    types.len() == 1 && is_keyed_unit(types[0], aliases, vocabulary, &mut BTreeSet::new())
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
    vocabulary: &CanonicalVocabulary,
    visited: &mut BTreeSet<String>,
) -> bool {
    let resolved = resolve_outer_alias(rust_type, aliases, visited);
    let Type::Path(type_path) = resolved else {
        return false;
    };
    let Some(segment) = type_path.path.segments.last() else {
        return false;
    };
    if !path_is_canonical(&type_path.path, vocabulary, CanonicalType::KeyedItem) {
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

fn use_tree_imports_requirement_trait(tree: &UseTree) -> bool {
    match tree {
        UseTree::Rename(rename) => is_requirement_trait_name(&rename.ident.to_string()),
        UseTree::Path(path) => use_tree_imports_requirement_trait(&path.tree),
        UseTree::Group(group) => group.items.iter().any(use_tree_imports_requirement_trait),
        UseTree::Name(name) => is_requirement_trait_name(&name.ident.to_string()),
        UseTree::Glob(_) => false,
    }
}

fn use_tree_has_glob(tree: &UseTree) -> bool {
    match tree {
        UseTree::Glob(_) => true,
        UseTree::Path(path) => use_tree_has_glob(&path.tree),
        UseTree::Group(group) => group.items.iter().any(use_tree_has_glob),
        UseTree::Name(_) | UseTree::Rename(_) => false,
    }
}

fn collect_scoped_requirement_roots(
    items: &[syn::Item],
    source: &Path,
    module_path: &mut Vec<String>,
    scoped_imports: &ScopedImports,
    requirement_trait_paths: &BTreeMap<Vec<String>, RequirementKind>,
    roots: &mut Vec<(String, RequirementKind)>,
    root_scopes: &mut BTreeMap<(String, RequirementKind), BTreeSet<DeclarationScope>>,
) {
    for item in items {
        match item {
            syn::Item::Impl(item_impl) => {
                let Some((_, trait_path, _)) = &item_impl.trait_ else {
                    continue;
                };
                let resolved = resolve_scoped_path(trait_path, module_path, scoped_imports);
                let kind = requirement_trait_paths.get(&resolved).copied();
                if let (Some(kind), Some(name)) = (kind, type_name(item_impl.self_ty.as_ref())) {
                    roots.push((name.clone(), kind));
                    root_scopes
                        .entry((name, kind))
                        .or_default()
                        .insert(DeclarationScope {
                            module_path: module_path.clone(),
                            source: source.to_path_buf(),
                        });
                }
            }
            syn::Item::Mod(module) => {
                if let Some((_, nested)) = &module.content {
                    module_path.push(module.ident.to_string());
                    collect_scoped_requirement_roots(
                        nested,
                        source,
                        module_path,
                        scoped_imports,
                        requirement_trait_paths,
                        roots,
                        root_scopes,
                    );
                    let _ = module_path.pop();
                }
            }
            _ => {}
        }
    }
}

pub(crate) fn resolve_scoped_path(
    path: &syn::Path,
    current_module: &[String],
    imports: &ScopedImports,
) -> Vec<String> {
    let segments = path
        .segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect::<Vec<_>>();
    resolve_scoped_segments(&segments, current_module, imports, &mut BTreeSet::new())
}

fn resolve_scoped_segments(
    segments: &[String],
    current_module: &[String],
    imports: &ScopedImports,
    visited: &mut BTreeSet<(Vec<String>, String)>,
) -> Vec<String> {
    let mut module = current_module.to_vec();
    let mut offset = 0;
    while let Some(segment) = segments.get(offset) {
        match segment.as_str() {
            "::" | "crate" => module.clear(),
            "self" => {}
            "super" => {
                let _ = module.pop();
            }
            _ => break,
        }
        offset += 1;
    }
    let remaining = &segments[offset..];
    let Some(first) = remaining.first() else {
        return module;
    };
    let import_key = (module.clone(), first.clone());
    if visited.insert(import_key) {
        if let Some(source) = imports.get(&module).and_then(|scope| scope.get(first)) {
            let mut resolved = resolve_scoped_segments(source, &module, imports, visited);
            resolved.extend_from_slice(&remaining[1..]);
            return resolved;
        }
    }
    module.extend_from_slice(remaining);
    module
}

fn is_requirement_trait_name(name: &str) -> bool {
    matches!(name, "EngineRequirement" | "AdapterRequirement")
}

fn collect_core_vocabulary_aliases(
    tree: &UseTree,
    source: &Path,
    aliases: &mut Vec<(String, String, PathBuf)>,
) {
    match tree {
        UseTree::Rename(rename) => {
            let source_name = rename.ident.to_string();
            if matches!(source_name.as_str(), "ItemRequirements" | "KeyedItem") {
                aliases.push((source_name, rename.rename.to_string(), source.to_path_buf()));
            }
        }
        UseTree::Path(path) => collect_core_vocabulary_aliases(&path.tree, source, aliases),
        UseTree::Group(group) => {
            for item in &group.items {
                collect_core_vocabulary_aliases(item, source, aliases);
            }
        }
        UseTree::Name(_) | UseTree::Glob(_) => {}
    }
}

fn collect_type_imports(
    tree: &UseTree,
    prefix: &mut Vec<String>,
    module_path: &[String],
    public: bool,
    imports: &mut Vec<TypeImport>,
) {
    match tree {
        UseTree::Path(path) => {
            prefix.push(path.ident.to_string());
            collect_type_imports(&path.tree, prefix, module_path, public, imports);
            let _ = prefix.pop();
        }
        UseTree::Name(name) => {
            let mut source = prefix.clone();
            source.push(name.ident.to_string());
            imports.push(TypeImport {
                local: name.ident.to_string(),
                module_path: module_path.to_vec(),
                public,
                source,
            });
        }
        UseTree::Rename(rename) => {
            let mut source = prefix.clone();
            source.push(rename.ident.to_string());
            imports.push(TypeImport {
                local: rename.rename.to_string(),
                module_path: module_path.to_vec(),
                public,
                source,
            });
        }
        UseTree::Group(group) => {
            for item in &group.items {
                collect_type_imports(item, prefix, module_path, public, imports);
            }
        }
        UseTree::Glob(_) => {}
    }
}

#[derive(Clone, Copy)]
enum CanonicalType {
    ItemRequirements,
    KeyedItem,
}

fn path_is_canonical(
    path: &syn::Path,
    vocabulary: &CanonicalVocabulary,
    canonical_type: CanonicalType,
) -> bool {
    let Some(last) = path
        .segments
        .last()
        .map(|segment| segment.ident.to_string())
    else {
        return false;
    };
    let names = match canonical_type {
        CanonicalType::ItemRequirements => &vocabulary.item_names,
        CanonicalType::KeyedItem => &vocabulary.keyed_names,
    };
    if path.segments.len() == 1 {
        return names.contains(&last);
    }
    let Some(first) = path
        .segments
        .first()
        .map(|segment| segment.ident.to_string())
    else {
        return false;
    };
    let providers = match canonical_type {
        CanonicalType::ItemRequirements => &vocabulary.item_providers,
        CanonicalType::KeyedItem => &vocabulary.keyed_providers,
    };
    path.segments.len() == 2
        && providers.contains(&first)
        && match canonical_type {
            CanonicalType::ItemRequirements => last == "ItemRequirements",
            CanonicalType::KeyedItem => last == "KeyedItem",
        }
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

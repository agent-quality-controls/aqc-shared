use std::collections::{BTreeMap, BTreeSet};

use quote::ToTokens;
use syn::visit::Visit;
use syn::{
    Expr, ExprAssign, ExprCall, ExprMethodCall, ExprReference, ExprReturn, ExprStruct, FnArg,
    ImplItemFn, ItemFn, ItemImpl, Local, Macro, Member, Pat, PatStruct, Signature, Stmt, Type,
};

use crate::analyze::{
    CanonicalVocabulary, ScopedImports, contains_explicit_membership, contains_item_requirements,
    display_path, outer_type_name, resolve_scoped_path,
};
use crate::discover::ParsedCrate;
use crate::model::{ArchitectureViolation, ViolationCode};

pub(crate) struct AdapterSurface<'a> {
    pub aliases_by_module: &'a BTreeMap<Vec<String>, BTreeMap<String, Type>>,
    pub vocabulary: &'a CanonicalVocabulary,
    pub membership_functions: &'a BTreeSet<String>,
    pub local_macros: &'a BTreeMap<String, String>,
    pub membership_field_names: &'a BTreeSet<String>,
    pub membership_struct_paths: &'a BTreeSet<Vec<String>>,
    pub adapter_root_paths: &'a BTreeSet<Vec<String>>,
    pub scoped_imports: &'a ScopedImports,
    pub type_aliases: &'a BTreeMap<Vec<String>, Type>,
    pub engine_root_names: &'a BTreeSet<String>,
}

pub(crate) fn adapter_violations(
    parsed_crate: &ParsedCrate,
    surface: &AdapterSurface<'_>,
) -> Vec<ArchitectureViolation> {
    let mut violations = Vec::new();
    for source in &parsed_crate.sources {
        AdapterExpressionVisitor {
            aliases_by_module: surface.aliases_by_module,
            empty_aliases: BTreeMap::new(),
            vocabulary: surface.vocabulary,
            membership_functions: surface.membership_functions,
            local_macros: surface.local_macros,
            membership_field_names: surface.membership_field_names,
            membership_struct_paths: surface.membership_struct_paths,
            adapter_root_paths: surface.adapter_root_paths,
            scoped_imports: surface.scoped_imports,
            type_aliases: surface.type_aliases,
            engine_root_names: surface.engine_root_names,
            dependency_names: parsed_crate.dependencies.keys().cloned().collect(),
            membership_bindings: BTreeSet::new(),
            transferable_bindings: BTreeSet::new(),
            requirement_bindings: BTreeSet::new(),
            helper_engine_bindings: BTreeSet::new(),
            generic_type_names: BTreeSet::new(),
            closure_bindings: BTreeMap::new(),
            context: None,
            function: FunctionContext::default(),
            current_impl_adapter: false,
            crate_name: &parsed_crate.crate_name,
            repository_root: &parsed_crate.repository_root,
            source: &source.path,
            module_path: source.module_path.clone(),
            violations: &mut violations,
        }
        .visit_file(&source.syntax);
    }
    violations
}

#[derive(Clone, Copy, Default)]
struct FunctionContext {
    has_requirement_input: bool,
    returns_engine_requirement: bool,
    returns_membership: bool,
}

struct AdapterExpressionVisitor<'a> {
    aliases_by_module: &'a BTreeMap<Vec<String>, BTreeMap<String, Type>>,
    empty_aliases: BTreeMap<String, Type>,
    vocabulary: &'a CanonicalVocabulary,
    membership_functions: &'a BTreeSet<String>,
    local_macros: &'a BTreeMap<String, String>,
    membership_field_names: &'a BTreeSet<String>,
    membership_struct_paths: &'a BTreeSet<Vec<String>>,
    adapter_root_paths: &'a BTreeSet<Vec<String>>,
    scoped_imports: &'a ScopedImports,
    type_aliases: &'a BTreeMap<Vec<String>, Type>,
    engine_root_names: &'a BTreeSet<String>,
    dependency_names: BTreeSet<String>,
    membership_bindings: BTreeSet<String>,
    transferable_bindings: BTreeSet<String>,
    requirement_bindings: BTreeSet<String>,
    helper_engine_bindings: BTreeSet<String>,
    generic_type_names: BTreeSet<String>,
    closure_bindings: BTreeMap<String, usize>,
    context: Option<String>,
    function: FunctionContext,
    current_impl_adapter: bool,
    crate_name: &'a str,
    repository_root: &'a std::path::Path,
    source: &'a std::path::Path,
    module_path: Vec<String>,
    violations: &'a mut Vec<ArchitectureViolation>,
}

impl AdapterExpressionVisitor<'_> {
    fn aliases(&self) -> &BTreeMap<String, Type> {
        self.aliases_by_module
            .get(&self.module_path)
            .unwrap_or(&self.empty_aliases)
    }

    fn type_is_adapter_root(&self, rust_type: &Type) -> bool {
        AdapterTypeResolver {
            aliases: self.aliases(),
            adapter_root_paths: self.adapter_root_paths,
            scoped_imports: self.scoped_imports,
            type_aliases: self.type_aliases,
            generic_type_names: &self.generic_type_names,
        }
        .resolves(rust_type, &self.module_path, &mut BTreeSet::new())
    }

    fn report(&mut self, operation: &str) {
        let context = self
            .context
            .as_ref()
            .map_or_else(String::new, |name| format!(" function {name}"));
        self.violations.push(ArchitectureViolation {
            code: ViolationCode::AdapterMembershipConstruction,
            crate_name: self.crate_name.to_owned(),
            message: format!(
                "adapter{context} {operation}; policy must supply membership and adapter may only transfer or map it"
            ),
            source: display_path(self.repository_root, self.source),
        });
    }

    fn is_item_requirements(&self, expression: &ExprStruct) -> bool {
        if self.vocabulary.is_item_path(&expression.path) {
            return true;
        }
        let Some(name) = expression
            .path
            .segments
            .last()
            .map(|segment| segment.ident.to_string())
        else {
            return false;
        };
        resolves_to_item_requirements(&name, self.aliases(), self.vocabulary, &mut BTreeSet::new())
    }

    fn is_membership_value(&self, expression: &Expr) -> bool {
        match expression {
            Expr::Field(field) => {
                matches!(&field.member, Member::Named(identifier) if self.membership_field_names.contains(&identifier.to_string()))
                    && self.is_tracked_requirement_access(field.base.as_ref())
            }
            Expr::Group(group) => self.is_membership_value(group.expr.as_ref()),
            Expr::MethodCall(call) => {
                call.method == "map" && self.is_membership_value(call.receiver.as_ref())
            }
            Expr::Paren(paren) => self.is_membership_value(paren.expr.as_ref()),
            Expr::Path(path) => path.path.segments.last().is_some_and(|segment| {
                self.transferable_bindings
                    .contains(&segment.ident.to_string())
            }),
            Expr::Reference(reference) => self.is_membership_value(reference.expr.as_ref()),
            Expr::Struct(item) => self.is_item_requirements(item),
            _ => false,
        }
    }

    fn is_membership_collection_access(&self, expression: &Expr) -> bool {
        match expression {
            Expr::Field(field) => {
                let Member::Named(identifier) = &field.member else {
                    return false;
                };
                let name = identifier.to_string();
                self.membership_field_names.contains(&name)
                    && self.is_tracked_requirement_access(field.base.as_ref())
                    || matches!(name.as_str(), "required" | "forbidden" | "exact")
                        && self.is_membership_collection_access(field.base.as_ref())
            }
            Expr::Group(group) => self.is_membership_collection_access(group.expr.as_ref()),
            Expr::Index(index) => self.is_membership_collection_access(index.expr.as_ref()),
            Expr::Paren(paren) => self.is_membership_collection_access(paren.expr.as_ref()),
            Expr::Path(path) => path.path.segments.last().is_some_and(|segment| {
                self.membership_bindings
                    .contains(&segment.ident.to_string())
            }),
            Expr::Reference(reference) => {
                self.is_membership_collection_access(reference.expr.as_ref())
            }
            Expr::Unary(unary) => self.is_membership_collection_access(unary.expr.as_ref()),
            _ => false,
        }
    }

    fn is_tracked_requirement_access(&self, expression: &Expr) -> bool {
        match expression {
            Expr::Field(field) => self.is_tracked_requirement_access(field.base.as_ref()),
            Expr::Group(group) => self.is_tracked_requirement_access(group.expr.as_ref()),
            Expr::Paren(paren) => self.is_tracked_requirement_access(paren.expr.as_ref()),
            Expr::Path(path) => path.path.segments.last().is_some_and(|segment| {
                self.membership_bindings
                    .contains(&segment.ident.to_string())
                    || self
                        .requirement_bindings
                        .contains(&segment.ident.to_string())
            }),
            Expr::Reference(reference) => {
                self.is_tracked_requirement_access(reference.expr.as_ref())
            }
            _ => false,
        }
    }

    fn track_destructured_membership(&mut self, pattern: &PatStruct) {
        let is_adapter_requirement = path_resolves_to_adapter_root(
            &pattern.path,
            &self.module_path,
            self.adapter_root_paths,
            self.scoped_imports,
        );
        if !is_adapter_requirement {
            return;
        }
        for field in &pattern.fields {
            let Member::Named(identifier) = &field.member else {
                continue;
            };
            if !self
                .membership_field_names
                .contains(&identifier.to_string())
            {
                continue;
            }
            let Some(name) = local_binding_name(&field.pat) else {
                continue;
            };
            let _ = self.membership_bindings.insert(name.clone());
            let _ = self.transferable_bindings.insert(name);
        }
    }

    fn track_tuple_membership(&mut self, local: &Local) {
        let (Pat::Tuple(tuple), Some(init)) = (&local.pat, local.init.as_ref()) else {
            return;
        };
        if !self.is_membership_value(init.expr.as_ref()) {
            return;
        }
        let bindings = tuple
            .elems
            .iter()
            .filter_map(local_binding_name)
            .filter(|name| self.membership_field_names.contains(name))
            .collect::<Vec<_>>();
        self.membership_bindings.extend(bindings.iter().cloned());
        self.transferable_bindings.extend(bindings);
    }

    fn track_helper_pattern(&mut self, pattern: &Pat, expression: &Expr) {
        match (pattern, expression) {
            (Pat::Ident(binding), value) => {
                let name = binding.ident.to_string();
                self.helper_engine_bindings.remove(&name);
                if expression_returns_helper_result(value, &self.helper_engine_bindings) {
                    self.helper_engine_bindings.insert(name);
                }
            }
            (Pat::Tuple(pattern), Expr::Tuple(value)) => {
                for (part, expression) in pattern.elems.iter().zip(&value.elems) {
                    self.track_helper_pattern(part, expression);
                }
            }
            (Pat::TupleStruct(pattern), Expr::Call(value)) => {
                for (part, expression) in pattern.elems.iter().zip(&value.args) {
                    self.track_helper_pattern(part, expression);
                }
            }
            (Pat::Struct(pattern), Expr::Struct(value)) => {
                self.track_helper_struct_pattern(pattern, value);
            }
            (Pat::Paren(pattern), Expr::Paren(value)) => {
                self.track_helper_pattern(pattern.pat.as_ref(), value.expr.as_ref());
            }
            (Pat::Type(pattern), value) => {
                self.track_helper_pattern(pattern.pat.as_ref(), value);
            }
            (_, _) => {}
        }
    }

    fn track_helper_struct_pattern(&mut self, pattern: &PatStruct, value: &ExprStruct) {
        for field in &pattern.fields {
            if let Some(value) = value
                .fields
                .iter()
                .find(|candidate| candidate.member == field.member)
            {
                self.track_helper_pattern(&field.pat, &value.expr);
            }
        }
    }

    fn track_helper_assignment(&mut self, target: &Expr, value: &Expr) {
        match (target, value) {
            (Expr::Path(path), value) => {
                let Some(name) = path.path.get_ident().map(ToString::to_string) else {
                    return;
                };
                self.helper_engine_bindings.remove(&name);
                if expression_returns_helper_result(value, &self.helper_engine_bindings) {
                    self.helper_engine_bindings.insert(name);
                }
            }
            (Expr::Tuple(target), Expr::Tuple(value)) => {
                for (part, expression) in target.elems.iter().zip(&value.elems) {
                    self.track_helper_assignment(part, expression);
                }
            }
            (Expr::Paren(target), Expr::Paren(value)) => {
                self.track_helper_assignment(target.expr.as_ref(), value.expr.as_ref());
            }
            (_, _) => {}
        }
    }

    fn is_membership_argument(&self, expression: &Expr) -> bool {
        self.is_membership_value(expression)
            || matches!(expression, Expr::MethodCall(call) if call.method == "clone" && self.is_membership_value(call.receiver.as_ref()))
    }

    fn track_closure_binding(&mut self, name: &str, expression: &Expr) {
        self.closure_bindings.remove(name);
        let arity = closure_expression_arity(expression, &self.closure_bindings);
        if let Some(arity) = arity {
            self.closure_bindings.insert(name.to_owned(), arity);
        }
    }

    fn track_closure_pattern(&mut self, pattern: &Pat, expression: &Expr) {
        match (pattern, expression) {
            (Pat::Ident(binding), value) => {
                self.track_closure_binding(&binding.ident.to_string(), value);
            }
            (Pat::Tuple(pattern), Expr::Tuple(value)) => {
                for (part, value) in pattern.elems.iter().zip(&value.elems) {
                    self.track_closure_pattern(part, value);
                }
            }
            (Pat::TupleStruct(pattern), Expr::Call(value)) => {
                for (part, value) in pattern.elems.iter().zip(&value.args) {
                    self.track_closure_pattern(part, value);
                }
            }
            (Pat::Struct(pattern), Expr::Struct(value)) => {
                self.track_closure_struct_pattern(pattern, value);
            }
            (Pat::Paren(pattern), Expr::Paren(value)) => {
                self.track_closure_pattern(pattern.pat.as_ref(), value.expr.as_ref());
            }
            (Pat::Type(pattern), value) => self.track_closure_pattern(pattern.pat.as_ref(), value),
            _ => {}
        }
    }

    fn track_closure_assignment(&mut self, target: &Expr, value: &Expr) {
        match (target, value) {
            (Expr::Path(path), value) => {
                if let Some(name) = path.path.get_ident() {
                    self.track_closure_binding(&name.to_string(), value);
                }
            }
            (Expr::Tuple(target), Expr::Tuple(value)) => {
                for (part, value) in target.elems.iter().zip(&value.elems) {
                    self.track_closure_assignment(part, value);
                }
            }
            (Expr::Paren(target), Expr::Paren(value)) => {
                self.track_closure_assignment(target.expr.as_ref(), value.expr.as_ref());
            }
            _ => {}
        }
    }

    fn track_closure_struct_pattern(&mut self, pattern: &PatStruct, value: &ExprStruct) {
        for field in &pattern.fields {
            let Some(value) = value
                .fields
                .iter()
                .find(|candidate| candidate.member == field.member)
            else {
                continue;
            };
            self.track_closure_pattern(&field.pat, &value.expr);
        }
    }
}

impl Visit<'_> for AdapterExpressionVisitor<'_> {
    fn visit_item_mod(&mut self, item: &syn::ItemMod) {
        if item.content.is_some() {
            self.module_path.push(item.ident.to_string());
            syn::visit::visit_item_mod(self, item);
            let _ = self.module_path.pop();
        }
    }

    fn visit_item_impl(&mut self, item: &ItemImpl) {
        let previous = self.current_impl_adapter;
        let previous_generics = self.generic_type_names.clone();
        self.generic_type_names.extend(
            item.generics
                .type_params()
                .map(|item| item.ident.to_string()),
        );
        self.current_impl_adapter = self.type_is_adapter_root(item.self_ty.as_ref());
        syn::visit::visit_item_impl(self, item);
        self.current_impl_adapter = previous;
        self.generic_type_names = previous_generics;
    }

    fn visit_item_fn(&mut self, item: &ItemFn) {
        let previous = self.context.replace(item.sig.ident.to_string());
        let previous_function = self.function;
        let previous_generics = std::mem::take(&mut self.generic_type_names);
        self.generic_type_names = item
            .sig
            .generics
            .type_params()
            .map(|item| item.ident.to_string())
            .collect();
        let previous_bindings = std::mem::take(&mut self.membership_bindings);
        let previous_transferable = std::mem::take(&mut self.transferable_bindings);
        let previous_requirements = std::mem::take(&mut self.requirement_bindings);
        let previous_helpers = std::mem::take(&mut self.helper_engine_bindings);
        let previous_closures = std::mem::take(&mut self.closure_bindings);
        self.track_membership_parameters(&item.sig);
        self.function.returns_membership =
            return_type_is_membership(&item.sig.output, self.aliases(), self.vocabulary);
        self.function.returns_engine_requirement = return_type_is_engine_requirement(
            &item.sig.output,
            self.aliases(),
            self.membership_struct_paths,
            self.engine_root_names,
            &self.dependency_names,
            self.scoped_imports,
            &self.module_path,
        );
        syn::visit::visit_item_fn(self, item);
        if self.function.returns_engine_requirement
            && block_returns_helper_result(&item.block, &self.helper_engine_bindings)
        {
            self.report("obtains an engine requirement through a helper");
        }
        self.membership_bindings = previous_bindings;
        self.transferable_bindings = previous_transferable;
        self.requirement_bindings = previous_requirements;
        self.helper_engine_bindings = previous_helpers;
        self.closure_bindings = previous_closures;
        self.function = previous_function;
        self.generic_type_names = previous_generics;
        self.context = previous;
    }

    fn visit_impl_item_fn(&mut self, item: &ImplItemFn) {
        let previous = self.context.replace(item.sig.ident.to_string());
        let previous_function = self.function;
        let previous_generics = self.generic_type_names.clone();
        self.generic_type_names.extend(
            item.sig
                .generics
                .type_params()
                .map(|item| item.ident.to_string()),
        );
        let previous_bindings = std::mem::take(&mut self.membership_bindings);
        let previous_transferable = std::mem::take(&mut self.transferable_bindings);
        let previous_requirements = std::mem::take(&mut self.requirement_bindings);
        let previous_helpers = std::mem::take(&mut self.helper_engine_bindings);
        let previous_closures = std::mem::take(&mut self.closure_bindings);
        self.track_membership_parameters(&item.sig);
        self.function.returns_membership =
            return_type_is_membership(&item.sig.output, self.aliases(), self.vocabulary);
        self.function.returns_engine_requirement = return_type_is_engine_requirement(
            &item.sig.output,
            self.aliases(),
            self.membership_struct_paths,
            self.engine_root_names,
            &self.dependency_names,
            self.scoped_imports,
            &self.module_path,
        );
        syn::visit::visit_impl_item_fn(self, item);
        if self.function.returns_engine_requirement
            && block_returns_helper_result(&item.block, &self.helper_engine_bindings)
        {
            self.report("obtains an engine requirement through a helper");
        }
        self.membership_bindings = previous_bindings;
        self.transferable_bindings = previous_transferable;
        self.requirement_bindings = previous_requirements;
        self.helper_engine_bindings = previous_helpers;
        self.closure_bindings = previous_closures;
        self.function = previous_function;
        self.generic_type_names = previous_generics;
        self.context = previous;
    }

    fn visit_expr_assign(&mut self, expression: &ExprAssign) {
        if self.is_membership_collection_access(expression.left.as_ref()) {
            self.report("mutates a membership collection");
        }
        if self.function.returns_engine_requirement {
            self.track_helper_assignment(expression.left.as_ref(), expression.right.as_ref());
        }
        self.track_closure_assignment(expression.left.as_ref(), expression.right.as_ref());
        syn::visit::visit_expr_assign(self, expression);
    }

    fn visit_expr_closure(&mut self, expression: &syn::ExprClosure) {
        let previous_membership = self.membership_bindings.clone();
        let previous_transferable = self.transferable_bindings.clone();
        let previous_requirements = self.requirement_bindings.clone();
        let previous_requirement_input = self.function.has_requirement_input;
        for pattern in &expression.inputs {
            let Pat::Type(typed) = pattern else {
                continue;
            };
            let is_membership = contains_item_requirements(
                typed.ty.as_ref(),
                self.aliases(),
                self.vocabulary,
                &mut BTreeSet::new(),
            );
            let is_requirement = self.type_is_adapter_root(typed.ty.as_ref());
            let mut names = Vec::new();
            collect_binding_names(typed.pat.as_ref(), &mut names);
            if is_membership {
                self.membership_bindings.extend(names);
                self.report("accepts membership through a helper parameter");
            } else if is_requirement {
                self.requirement_bindings.extend(names);
                self.function.has_requirement_input = true;
            }
        }
        self.visit_expr(expression.body.as_ref());
        self.membership_bindings = previous_membership;
        self.transferable_bindings = previous_transferable;
        self.requirement_bindings = previous_requirements;
        self.function.has_requirement_input = previous_requirement_input;
    }

    fn visit_expr_call(&mut self, expression: &ExprCall) {
        if matches!(expression.func.as_ref(), Expr::Path(path) if path.path.get_ident().is_some_and(|name| self.closure_bindings.contains_key(&name.to_string())))
            && expression
                .args
                .iter()
                .any(|argument| self.is_membership_argument(argument))
        {
            self.report("passes membership through a local closure");
        }
        if self.function.returns_membership
            && (expression_constructs_membership(
                &Expr::Call(expression.clone()),
                self.aliases(),
                self.vocabulary,
            ) || is_default_call(expression))
        {
            self.report("constructs membership through a membership-returning helper");
        }
        syn::visit::visit_expr_call(self, expression);
    }

    fn visit_expr_method_call(&mut self, expression: &ExprMethodCall) {
        if self.is_membership_collection_access(expression.receiver.as_ref())
            && !is_read_only_membership_method(&expression.method.to_string())
        {
            self.report("calls a mutating or unrecognized method on a membership collection");
        }
        syn::visit::visit_expr_method_call(self, expression);
    }

    fn visit_expr_reference(&mut self, expression: &ExprReference) {
        if expression.mutability.is_some()
            && self.is_membership_collection_access(expression.expr.as_ref())
        {
            self.report("borrows a membership collection mutably");
        }
        syn::visit::visit_expr_reference(self, expression);
    }

    fn visit_expr_return(&mut self, expression: &ExprReturn) {
        if self.function.returns_engine_requirement
            && expression.expr.as_deref().is_some_and(|value| {
                expression_returns_helper_result(value, &self.helper_engine_bindings)
            })
        {
            self.report("obtains an engine requirement through a helper");
        }
        syn::visit::visit_expr_return(self, expression);
    }

    fn visit_expr_struct(&mut self, expression: &ExprStruct) {
        if self.is_item_requirements(expression) {
            self.report("constructs ItemRequirements");
        }
        let constructs_adapter_root = path_resolves_to_adapter_root(
            &expression.path,
            &self.module_path,
            self.adapter_root_paths,
            self.scoped_imports,
        );
        let constructs_membership_struct = self.membership_struct_paths.contains(
            &resolve_scoped_path(&expression.path, &self.module_path, self.scoped_imports),
        );
        if !constructs_adapter_root && !constructs_membership_struct {
            syn::visit::visit_expr_struct(self, expression);
            return;
        }
        for field in &expression.fields {
            let is_membership_field = matches!(&field.member, Member::Named(identifier) if self.membership_field_names.contains(&identifier.to_string()));
            if !is_membership_field {
                continue;
            }
            let constructs = expression_contains_membership_construction(
                &field.expr,
                self.aliases(),
                self.vocabulary,
            );
            let transfers = self.is_membership_value(&field.expr);
            let neutral =
                is_neutral_membership_default(&field.expr, self.aliases(), self.vocabulary);
            let neutral_without_policy_input = neutral && !self.function.has_requirement_input;
            let invalid = if constructs_adapter_root {
                constructs || !transfers
            } else {
                !neutral_without_policy_input && (constructs || !transfers)
            };
            if invalid {
                self.report("constructs or obtains membership instead of transferring policy membership for an adapter requirement field");
            }
        }
        syn::visit::visit_expr_struct(self, expression);
    }

    fn visit_macro(&mut self, item: &Macro) {
        let tokens = item.tokens.to_token_stream().to_string();
        let macro_name = item
            .path
            .segments
            .last()
            .map(|segment| segment.ident.to_string());
        let names_membership =
            macro_constructs_membership(&tokens, self.aliases(), self.vocabulary)
                || macro_name
                    .as_ref()
                    .and_then(|name| self.local_macros.get(name))
                    .is_some_and(|body| {
                        macro_constructs_membership(body, self.aliases(), self.vocabulary)
                    });
        if names_membership {
            self.report("uses a macro that can construct membership");
        }
        syn::visit::visit_macro(self, item);
    }

    fn visit_pat_struct(&mut self, pattern: &PatStruct) {
        self.track_destructured_membership(pattern);
        let is_membership = pattern
            .path
            .segments
            .last()
            .map(|segment| segment.ident.to_string())
            .is_some_and(|name| {
                resolves_to_item_requirements(
                    &name,
                    self.aliases(),
                    self.vocabulary,
                    &mut BTreeSet::new(),
                )
            });
        if is_membership && pattern.fields.iter().any(|field| {
            matches!(&field.member, Member::Named(identifier) if matches!(identifier.to_string().as_str(), "required" | "forbidden" | "exact"))
        }) {
            self.report("destructures membership internals");
        }
        syn::visit::visit_pat_struct(self, pattern);
    }

    fn visit_local(&mut self, local: &Local) {
        let mut shadowed = Vec::new();
        collect_binding_names(&local.pat, &mut shadowed);
        for name in shadowed {
            self.membership_bindings.remove(&name);
            self.transferable_bindings.remove(&name);
            self.requirement_bindings.remove(&name);
            self.helper_engine_bindings.remove(&name);
            self.closure_bindings.remove(&name);
        }
        if let (Some(name), Some(init)) = (local_binding_name(&local.pat), local.init.as_ref()) {
            let constructs = expression_constructs_membership(
                init.expr.as_ref(),
                self.aliases(),
                self.vocabulary,
            );
            if constructs {
                self.report("replaces policy membership with a locally constructed default");
            }
            let transfers = self.is_membership_value(init.expr.as_ref());
            let helper_produced = matches!(init.expr.as_ref(), Expr::Call(call) if matches!(call.func.as_ref(), Expr::Path(path) if path.path.segments.last().is_some_and(|segment| self.membership_functions.contains(&segment.ident.to_string()))));
            if constructs
                || transfers
                || helper_produced
                || local_type_is_membership(local, self.aliases(), self.vocabulary)
            {
                let _ = self.membership_bindings.insert(name.clone());
            }
            if transfers {
                let _ = self.transferable_bindings.insert(name.clone());
            }
            if self.function.returns_engine_requirement
                && expression_returns_helper_result(
                    init.expr.as_ref(),
                    &self.helper_engine_bindings,
                )
            {
                let _ = self.helper_engine_bindings.insert(name);
            }
        }
        if let Some(init) = &local.init {
            self.track_closure_pattern(&local.pat, init.expr.as_ref());
        }
        if self.function.returns_engine_requirement {
            if let Some(init) = &local.init {
                self.track_helper_pattern(&local.pat, init.expr.as_ref());
            }
        }
        self.track_tuple_membership(local);
        syn::visit::visit_local(self, local);
    }
}

fn closure_expression_arity(
    expression: &Expr,
    bindings: &BTreeMap<String, usize>,
) -> Option<usize> {
    match expression {
        Expr::Closure(closure) => Some(closure.inputs.len()),
        Expr::Group(group) => closure_expression_arity(group.expr.as_ref(), bindings),
        Expr::Paren(paren) => closure_expression_arity(paren.expr.as_ref(), bindings),
        Expr::Path(path) => path
            .path
            .get_ident()
            .and_then(|source| bindings.get(&source.to_string()))
            .copied(),
        Expr::Reference(reference) => closure_expression_arity(reference.expr.as_ref(), bindings),
        _ => None,
    }
}

impl AdapterExpressionVisitor<'_> {
    fn track_membership_parameters(&mut self, signature: &Signature) {
        for argument in &signature.inputs {
            if matches!(argument, FnArg::Receiver(_)) {
                self.track_receiver();
                continue;
            }
            let FnArg::Typed(argument) = argument else {
                continue;
            };
            let is_membership = contains_item_requirements(
                &argument.ty,
                self.aliases(),
                self.vocabulary,
                &mut BTreeSet::new(),
            );
            let is_requirement = self.type_is_adapter_root(&argument.ty);
            if !is_membership && !is_requirement {
                continue;
            }
            if is_requirement {
                self.function.has_requirement_input = true;
            }
            let Some(name) = local_binding_name(&argument.pat) else {
                continue;
            };
            if is_membership {
                self.membership_bindings.insert(name.clone());
                self.report("accepts membership through a helper parameter");
            } else {
                self.requirement_bindings.insert(name);
                self.function.has_requirement_input = true;
            }
        }
    }

    fn track_receiver(&mut self) {
        if self.current_impl_adapter {
            self.function.has_requirement_input = true;
            self.requirement_bindings.insert("self".to_owned());
        }
    }
}

fn return_type_is_engine_requirement(
    output: &syn::ReturnType,
    aliases: &BTreeMap<String, Type>,
    membership_struct_paths: &BTreeSet<Vec<String>>,
    engine_root_names: &BTreeSet<String>,
    dependency_names: &BTreeSet<String>,
    scoped_imports: &ScopedImports,
    current_module: &[String],
) -> bool {
    let syn::ReturnType::Type(_, rust_type) = output else {
        return false;
    };
    let resolved = resolve_type_alias(rust_type, aliases, &mut BTreeSet::new());
    let Type::Path(type_path) = resolved else {
        return false;
    };
    let path = resolve_scoped_path(&type_path.path, current_module, scoped_imports);
    membership_struct_paths.contains(&path)
        || path
            .first()
            .is_some_and(|name| dependency_names.contains(name))
            && path
                .last()
                .is_some_and(|name| engine_root_names.contains(name))
}

fn resolve_type_alias<'a>(
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
        resolve_type_alias(alias, aliases, visited)
    })
}

fn block_returns_helper_result(block: &syn::Block, bindings: &BTreeSet<String>) -> bool {
    matches!(block.stmts.last(), Some(Stmt::Expr(expression, None)) if expression_returns_helper_result(expression, bindings))
}

fn expression_returns_helper_result(expression: &Expr, bindings: &BTreeSet<String>) -> bool {
    match expression {
        Expr::Call(_) | Expr::MethodCall(_) => true,
        Expr::Await(awaited) => expression_returns_helper_result(awaited.base.as_ref(), bindings),
        Expr::Block(block) => block_returns_helper_result(&block.block, bindings),
        Expr::Group(group) => expression_returns_helper_result(group.expr.as_ref(), bindings),
        Expr::If(branch) => {
            block_returns_helper_result(&branch.then_branch, bindings)
                || branch
                    .else_branch
                    .as_ref()
                    .is_some_and(|(_, value)| expression_returns_helper_result(value, bindings))
        }
        Expr::Match(matched) => matched
            .arms
            .iter()
            .any(|arm| expression_returns_helper_result(arm.body.as_ref(), bindings)),
        Expr::Paren(paren) => expression_returns_helper_result(paren.expr.as_ref(), bindings),
        Expr::Path(path) => path
            .path
            .segments
            .last()
            .is_some_and(|segment| bindings.contains(&segment.ident.to_string())),
        Expr::Try(tried) => expression_returns_helper_result(tried.expr.as_ref(), bindings),
        _ => false,
    }
}

struct AdapterTypeResolver<'a> {
    aliases: &'a BTreeMap<String, Type>,
    adapter_root_paths: &'a BTreeSet<Vec<String>>,
    scoped_imports: &'a ScopedImports,
    type_aliases: &'a BTreeMap<Vec<String>, Type>,
    generic_type_names: &'a BTreeSet<String>,
}

impl AdapterTypeResolver<'_> {
    fn resolves(
        &self,
        rust_type: &Type,
        current_module: &[String],
        visited: &mut BTreeSet<Vec<String>>,
    ) -> bool {
        if let Type::Reference(reference) = rust_type {
            return self.resolves(&reference.elem, current_module, visited);
        }
        let Type::Path(type_path) = rust_type else {
            return false;
        };
        if type_path.path.segments.len() == 1
            && type_path
                .path
                .get_ident()
                .is_some_and(|name| self.generic_type_names.contains(&name.to_string()))
        {
            return false;
        }
        let resolved = resolve_scoped_path(&type_path.path, current_module, self.scoped_imports);
        if self.adapter_root_paths.contains(&resolved) {
            return true;
        }
        if !visited.insert(resolved.clone()) {
            return false;
        }
        if let Some(alias) = self.type_aliases.get(&resolved) {
            let alias_module = &resolved[..resolved.len().saturating_sub(1)];
            return self.resolves(alias, alias_module, visited);
        }
        let Some(name) = outer_type_name(rust_type) else {
            return false;
        };
        self.aliases
            .get(&name)
            .is_some_and(|alias| self.resolves(alias, current_module, visited))
    }
}

fn path_resolves_to_adapter_root(
    path: &syn::Path,
    current_module: &[String],
    adapter_root_paths: &BTreeSet<Vec<String>>,
    scoped_imports: &ScopedImports,
) -> bool {
    let resolved = resolve_scoped_path(path, current_module, scoped_imports);
    adapter_root_paths.contains(&resolved)
}

fn macro_constructs_membership(
    tokens: &str,
    aliases: &BTreeMap<String, Type>,
    vocabulary: &CanonicalVocabulary,
) -> bool {
    tokens.contains("ItemRequirements {")
        || aliases.iter().any(|(name, rust_type)| {
            contains_explicit_membership(rust_type, aliases, vocabulary, &mut BTreeSet::new())
                && tokens.contains(&format!("{name} {{"))
        })
        || tokens.contains("default")
            && tokens.split_whitespace().any(|token| {
                resolves_to_item_requirements(token, aliases, vocabulary, &mut BTreeSet::new())
            })
}

fn return_type_is_membership(
    output: &syn::ReturnType,
    aliases: &BTreeMap<String, Type>,
    vocabulary: &CanonicalVocabulary,
) -> bool {
    let syn::ReturnType::Type(_, rust_type) = output else {
        return false;
    };
    contains_item_requirements(rust_type, aliases, vocabulary, &mut BTreeSet::new())
}

fn expression_contains_membership_construction(
    expression: &Expr,
    aliases: &BTreeMap<String, Type>,
    vocabulary: &CanonicalVocabulary,
) -> bool {
    match expression {
        Expr::Struct(item) => item
            .path
            .segments
            .last()
            .map(|segment| segment.ident.to_string())
            .is_some_and(|name| {
                resolves_to_item_requirements(&name, aliases, vocabulary, &mut BTreeSet::new())
            }),
        Expr::Call(_) => expression_constructs_membership(expression, aliases, vocabulary),
        Expr::Group(group) => {
            expression_contains_membership_construction(&group.expr, aliases, vocabulary)
        }
        Expr::Paren(paren) => {
            expression_contains_membership_construction(&paren.expr, aliases, vocabulary)
        }
        _ => false,
    }
}

fn is_neutral_membership_default(
    expression: &Expr,
    aliases: &BTreeMap<String, Type>,
    vocabulary: &CanonicalVocabulary,
) -> bool {
    let Expr::Call(call) = expression else {
        return false;
    };
    let Expr::Path(path) = call.func.as_ref() else {
        return false;
    };
    let segments = path.path.segments.iter().collect::<Vec<_>>();
    segments.len() >= 2
        && segments[segments.len() - 1].ident == "default"
        && (vocabulary.is_item_constructor_path(&path.path)
            || resolves_to_item_requirements(
                &segments[segments.len() - 2].ident.to_string(),
                aliases,
                vocabulary,
                &mut BTreeSet::new(),
            ))
}

fn resolves_to_item_requirements(
    name: &str,
    aliases: &BTreeMap<String, Type>,
    vocabulary: &CanonicalVocabulary,
    visited: &mut BTreeSet<String>,
) -> bool {
    if vocabulary.is_item_name(name) {
        return true;
    }
    if !visited.insert(name.to_owned()) {
        return false;
    }
    aliases
        .get(name)
        .and_then(outer_type_name)
        .is_some_and(|outer| resolves_to_item_requirements(&outer, aliases, vocabulary, visited))
}

fn local_binding_name(pattern: &Pat) -> Option<String> {
    match pattern {
        Pat::Ident(identifier) => Some(identifier.ident.to_string()),
        Pat::Type(typed) => local_binding_name(&typed.pat),
        _ => None,
    }
}

fn collect_binding_names(pattern: &Pat, names: &mut Vec<String>) {
    match pattern {
        Pat::Ident(identifier) => names.push(identifier.ident.to_string()),
        Pat::Or(pattern) => {
            for item in &pattern.cases {
                collect_binding_names(item, names);
            }
        }
        Pat::Paren(pattern) => collect_binding_names(&pattern.pat, names),
        Pat::Reference(pattern) => collect_binding_names(&pattern.pat, names),
        Pat::Slice(pattern) => {
            for item in &pattern.elems {
                collect_binding_names(item, names);
            }
        }
        Pat::Struct(pattern) => {
            for field in &pattern.fields {
                collect_binding_names(&field.pat, names);
            }
        }
        Pat::Tuple(pattern) => {
            for item in &pattern.elems {
                collect_binding_names(item, names);
            }
        }
        Pat::TupleStruct(pattern) => {
            for item in &pattern.elems {
                collect_binding_names(item, names);
            }
        }
        Pat::Type(pattern) => collect_binding_names(&pattern.pat, names),
        _ => {}
    }
}

fn local_type_is_membership(
    local: &Local,
    aliases: &BTreeMap<String, Type>,
    vocabulary: &CanonicalVocabulary,
) -> bool {
    let Pat::Type(typed) = &local.pat else {
        return false;
    };
    contains_item_requirements(&typed.ty, aliases, vocabulary, &mut BTreeSet::new())
}

fn expression_constructs_membership(
    expression: &Expr,
    aliases: &BTreeMap<String, Type>,
    vocabulary: &CanonicalVocabulary,
) -> bool {
    let Expr::Call(call) = expression else {
        return false;
    };
    let Expr::Path(path) = call.func.as_ref() else {
        return false;
    };
    let segments = path.path.segments.iter().collect::<Vec<_>>();
    segments.len() >= 2
        && (vocabulary.is_item_constructor_path(&path.path)
            || resolves_to_item_requirements(
                &segments[segments.len() - 2].ident.to_string(),
                aliases,
                vocabulary,
                &mut BTreeSet::new(),
            ))
}

fn is_default_call(call: &ExprCall) -> bool {
    let Expr::Path(path) = call.func.as_ref() else {
        return false;
    };
    let segments = path.path.segments.iter().collect::<Vec<_>>();
    segments.len() >= 2
        && segments[segments.len() - 2].ident == "Default"
        && segments[segments.len() - 1].ident == "default"
}

fn is_read_only_membership_method(method: &str) -> bool {
    matches!(
        method,
        "as_ref"
            | "contains"
            | "first"
            | "get"
            | "is_empty"
            | "is_none"
            | "is_some"
            | "iter"
            | "last"
            | "len"
            | "map"
    )
}

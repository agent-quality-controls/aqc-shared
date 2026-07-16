use std::collections::{BTreeMap, BTreeSet};

use quote::ToTokens;
use syn::visit::Visit;
use syn::{
    Expr, ExprAssign, ExprCall, ExprMethodCall, ExprReference, ExprStruct, FnArg, ImplItemFn,
    ItemFn, Local, Macro, Member, Pat, PatStruct, Signature, Type,
};

use crate::analyze::{
    contains_explicit_membership, contains_item_requirements, display_path, outer_type_name,
};
use crate::discover::ParsedCrate;
use crate::model::{ArchitectureViolation, ViolationCode};

pub(crate) fn adapter_violations(
    parsed_crate: &ParsedCrate,
    aliases: &BTreeMap<String, Type>,
    local_macros: &BTreeMap<String, String>,
    membership_functions: &BTreeSet<String>,
    membership_field_names: &BTreeSet<String>,
    adapter_root_names: &BTreeSet<String>,
) -> Vec<ArchitectureViolation> {
    let mut violations = Vec::new();
    for source in &parsed_crate.sources {
        AdapterExpressionVisitor {
            aliases,
            local_macros,
            membership_functions,
            membership_field_names,
            adapter_root_names,
            membership_bindings: BTreeSet::new(),
            requirement_bindings: BTreeSet::new(),
            context: None,
            context_returns_membership: false,
            crate_name: &parsed_crate.crate_name,
            repository_root: &parsed_crate.repository_root,
            source: &source.path,
            violations: &mut violations,
        }
        .visit_file(&source.syntax);
    }
    violations
}

struct AdapterExpressionVisitor<'a> {
    aliases: &'a BTreeMap<String, Type>,
    local_macros: &'a BTreeMap<String, String>,
    membership_functions: &'a BTreeSet<String>,
    membership_field_names: &'a BTreeSet<String>,
    adapter_root_names: &'a BTreeSet<String>,
    membership_bindings: BTreeSet<String>,
    requirement_bindings: BTreeSet<String>,
    context: Option<String>,
    context_returns_membership: bool,
    crate_name: &'a str,
    repository_root: &'a std::path::Path,
    source: &'a std::path::Path,
    violations: &'a mut Vec<ArchitectureViolation>,
}

impl AdapterExpressionVisitor<'_> {
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
        let Some(name) = expression
            .path
            .segments
            .last()
            .map(|segment| segment.ident.to_string())
        else {
            return false;
        };
        resolves_to_item_requirements(&name, self.aliases, &mut BTreeSet::new())
    }

    fn is_membership_value(&self, expression: &Expr) -> bool {
        match expression {
            Expr::Call(call) => {
                expression_constructs_membership(expression, self.aliases)
                    || matches!(call.func.as_ref(), Expr::Path(path) if path.path.segments.last().is_some_and(|segment| self.membership_functions.contains(&segment.ident.to_string())))
            }
            Expr::Field(field) => {
                matches!(&field.member, Member::Named(identifier) if self.membership_field_names.contains(&identifier.to_string()))
                    && self.is_tracked_requirement_access(field.base.as_ref())
            }
            Expr::Group(group) => self.is_membership_value(group.expr.as_ref()),
            Expr::MethodCall(call) => {
                self.membership_functions.contains(&call.method.to_string())
                    || call.method == "map" && self.is_membership_value(call.receiver.as_ref())
            }
            Expr::Paren(paren) => self.is_membership_value(paren.expr.as_ref()),
            Expr::Path(path) => path.path.segments.last().is_some_and(|segment| {
                self.membership_bindings
                    .contains(&segment.ident.to_string())
            }),
            Expr::Reference(reference) => self.is_membership_value(reference.expr.as_ref()),
            Expr::Struct(item) => self.is_item_requirements(item),
            _ => false,
        }
    }

    fn is_membership_collection_access(&self, expression: &Expr) -> bool {
        let mut names = Vec::new();
        collect_access_names(expression, &mut names);
        let Some(last) = names.last() else {
            return false;
        };
        self.membership_field_names.contains(last)
            || self.membership_bindings.contains(last)
            || matches!(last.as_str(), "required" | "forbidden" | "exact")
                && names[..names.len().saturating_sub(1)].iter().any(|name| {
                    self.membership_field_names.contains(name)
                        || self.membership_bindings.contains(name)
                })
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
            let _ = self.membership_bindings.insert(name);
        }
    }

    fn track_tuple_membership(&mut self, local: &Local) {
        let (Pat::Tuple(tuple), Some(init)) = (&local.pat, local.init.as_ref()) else {
            return;
        };
        if !self.is_membership_value(init.expr.as_ref()) {
            return;
        }
        self.membership_bindings.extend(
            tuple
                .elems
                .iter()
                .filter_map(local_binding_name)
                .filter(|name| self.membership_field_names.contains(name)),
        );
    }
}

impl Visit<'_> for AdapterExpressionVisitor<'_> {
    fn visit_item_fn(&mut self, item: &ItemFn) {
        let previous = self.context.replace(item.sig.ident.to_string());
        let previous_membership = self.context_returns_membership;
        let previous_bindings = std::mem::take(&mut self.membership_bindings);
        let previous_requirements = std::mem::take(&mut self.requirement_bindings);
        self.track_membership_parameters(&item.sig);
        self.context_returns_membership = return_type_is_membership(&item.sig.output, self.aliases);
        syn::visit::visit_item_fn(self, item);
        self.membership_bindings = previous_bindings;
        self.requirement_bindings = previous_requirements;
        self.context_returns_membership = previous_membership;
        self.context = previous;
    }

    fn visit_impl_item_fn(&mut self, item: &ImplItemFn) {
        let previous = self.context.replace(item.sig.ident.to_string());
        let previous_membership = self.context_returns_membership;
        let previous_bindings = std::mem::take(&mut self.membership_bindings);
        let previous_requirements = std::mem::take(&mut self.requirement_bindings);
        self.track_membership_parameters(&item.sig);
        self.context_returns_membership = return_type_is_membership(&item.sig.output, self.aliases);
        syn::visit::visit_impl_item_fn(self, item);
        self.membership_bindings = previous_bindings;
        self.requirement_bindings = previous_requirements;
        self.context_returns_membership = previous_membership;
        self.context = previous;
    }

    fn visit_expr_assign(&mut self, expression: &ExprAssign) {
        if self.is_membership_collection_access(expression.left.as_ref()) {
            self.report("mutates a membership collection");
        }
        syn::visit::visit_expr_assign(self, expression);
    }

    fn visit_expr_call(&mut self, expression: &ExprCall) {
        if self.context_returns_membership
            && (expression_constructs_membership(&Expr::Call(expression.clone()), self.aliases)
                || is_default_call(expression))
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

    fn visit_expr_struct(&mut self, expression: &ExprStruct) {
        if self.is_item_requirements(expression) {
            self.report("constructs ItemRequirements");
        }
        let constructs_adapter_root =
            expression.path.segments.last().is_some_and(|segment| {
                self.adapter_root_names.contains(&segment.ident.to_string())
            });
        for field in &expression.fields {
            let is_membership_field = matches!(&field.member, Member::Named(identifier) if self.membership_field_names.contains(&identifier.to_string()));
            if !is_membership_field {
                continue;
            }
            let constructs = expression_contains_membership_construction(&field.expr, self.aliases);
            let transfers = self.is_membership_value(&field.expr);
            let neutral = is_neutral_membership_default(&field.expr, self.aliases);
            let invalid = if constructs_adapter_root {
                constructs || !transfers
            } else {
                !neutral && (constructs || !transfers)
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
        let names_membership = macro_constructs_membership(&tokens, self.aliases)
            || macro_name
                .as_ref()
                .and_then(|name| self.local_macros.get(name))
                .is_some_and(|body| macro_constructs_membership(body, self.aliases));
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
                resolves_to_item_requirements(&name, self.aliases, &mut BTreeSet::new())
            });
        if is_membership && pattern.fields.iter().any(|field| {
            matches!(&field.member, Member::Named(identifier) if matches!(identifier.to_string().as_str(), "required" | "forbidden" | "exact"))
        }) {
            self.report("destructures membership internals");
        }
        syn::visit::visit_pat_struct(self, pattern);
    }

    fn visit_local(&mut self, local: &Local) {
        if let (Some(name), Some(init)) = (local_binding_name(&local.pat), local.init.as_ref()) {
            let constructs = expression_constructs_membership(init.expr.as_ref(), self.aliases);
            if constructs {
                self.report("replaces policy membership with a locally constructed default");
            }
            if constructs || self.is_membership_value(init.expr.as_ref()) {
                let _ = self.membership_bindings.insert(name);
            }
        }
        self.track_tuple_membership(local);
        syn::visit::visit_local(self, local);
    }
}

impl AdapterExpressionVisitor<'_> {
    fn track_membership_parameters(&mut self, signature: &Signature) {
        for argument in &signature.inputs {
            let FnArg::Typed(argument) = argument else {
                continue;
            };
            let is_membership =
                contains_item_requirements(&argument.ty, self.aliases, &mut BTreeSet::new());
            let is_requirement = type_resolves_to_adapter_root(
                &argument.ty,
                self.aliases,
                self.adapter_root_names,
                &mut BTreeSet::new(),
            );
            if !is_membership && !is_requirement {
                continue;
            }
            let Some(name) = local_binding_name(&argument.pat) else {
                continue;
            };
            let bindings = if is_membership {
                &mut self.membership_bindings
            } else {
                &mut self.requirement_bindings
            };
            let _ = bindings.insert(name);
        }
    }
}

fn type_resolves_to_adapter_root(
    rust_type: &Type,
    aliases: &BTreeMap<String, Type>,
    adapter_root_names: &BTreeSet<String>,
    visited: &mut BTreeSet<String>,
) -> bool {
    if let Type::Reference(reference) = rust_type {
        return type_resolves_to_adapter_root(
            &reference.elem,
            aliases,
            adapter_root_names,
            visited,
        );
    }
    let Some(name) = outer_type_name(rust_type) else {
        return false;
    };
    if adapter_root_names.contains(&name) {
        return true;
    }
    if !visited.insert(name.clone()) {
        return false;
    }
    aliases.get(&name).is_some_and(|alias| {
        type_resolves_to_adapter_root(alias, aliases, adapter_root_names, visited)
    })
}

fn macro_constructs_membership(tokens: &str, aliases: &BTreeMap<String, Type>) -> bool {
    tokens.contains("ItemRequirements {")
        || aliases.iter().any(|(name, rust_type)| {
            contains_explicit_membership(rust_type, aliases, &mut BTreeSet::new())
                && tokens.contains(&format!("{name} {{"))
        })
        || tokens.contains("default")
            && tokens
                .split_whitespace()
                .any(|token| resolves_to_item_requirements(token, aliases, &mut BTreeSet::new()))
}

fn return_type_is_membership(output: &syn::ReturnType, aliases: &BTreeMap<String, Type>) -> bool {
    let syn::ReturnType::Type(_, rust_type) = output else {
        return false;
    };
    contains_item_requirements(rust_type, aliases, &mut BTreeSet::new())
}

fn expression_contains_membership_construction(
    expression: &Expr,
    aliases: &BTreeMap<String, Type>,
) -> bool {
    match expression {
        Expr::Struct(item) => item
            .path
            .segments
            .last()
            .map(|segment| segment.ident.to_string())
            .is_some_and(|name| {
                resolves_to_item_requirements(&name, aliases, &mut BTreeSet::new())
            }),
        Expr::Call(_) => expression_constructs_membership(expression, aliases),
        Expr::Group(group) => expression_contains_membership_construction(&group.expr, aliases),
        Expr::Paren(paren) => expression_contains_membership_construction(&paren.expr, aliases),
        _ => false,
    }
}

fn is_neutral_membership_default(expression: &Expr, aliases: &BTreeMap<String, Type>) -> bool {
    let Expr::Call(call) = expression else {
        return false;
    };
    let Expr::Path(path) = call.func.as_ref() else {
        return false;
    };
    let segments = path.path.segments.iter().collect::<Vec<_>>();
    segments.len() >= 2
        && segments[segments.len() - 1].ident == "default"
        && resolves_to_item_requirements(
            &segments[segments.len() - 2].ident.to_string(),
            aliases,
            &mut BTreeSet::new(),
        )
}

fn resolves_to_item_requirements(
    name: &str,
    aliases: &BTreeMap<String, Type>,
    visited: &mut BTreeSet<String>,
) -> bool {
    if name == "ItemRequirements" {
        return true;
    }
    if !visited.insert(name.to_owned()) {
        return false;
    }
    aliases
        .get(name)
        .and_then(outer_type_name)
        .is_some_and(|outer| resolves_to_item_requirements(&outer, aliases, visited))
}

fn collect_access_names(expression: &Expr, names: &mut Vec<String>) {
    match expression {
        Expr::Field(field) => {
            collect_access_names(field.base.as_ref(), names);
            if let Member::Named(identifier) = &field.member {
                names.push(identifier.to_string());
            }
        }
        Expr::Group(group) => collect_access_names(group.expr.as_ref(), names),
        Expr::Index(index) => collect_access_names(index.expr.as_ref(), names),
        Expr::Paren(paren) => collect_access_names(paren.expr.as_ref(), names),
        Expr::Path(path) => {
            if let Some(segment) = path.path.segments.last() {
                names.push(segment.ident.to_string());
            }
        }
        Expr::Reference(reference) => collect_access_names(reference.expr.as_ref(), names),
        Expr::Unary(unary) => collect_access_names(unary.expr.as_ref(), names),
        _ => {}
    }
}

fn local_binding_name(pattern: &Pat) -> Option<String> {
    let Pat::Ident(identifier) = pattern else {
        return None;
    };
    Some(identifier.ident.to_string())
}

fn expression_constructs_membership(expression: &Expr, aliases: &BTreeMap<String, Type>) -> bool {
    let Expr::Call(call) = expression else {
        return false;
    };
    let Expr::Path(path) = call.func.as_ref() else {
        return false;
    };
    let segments = path.path.segments.iter().collect::<Vec<_>>();
    segments.len() >= 2
        && resolves_to_item_requirements(
            &segments[segments.len() - 2].ident.to_string(),
            aliases,
            &mut BTreeSet::new(),
        )
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

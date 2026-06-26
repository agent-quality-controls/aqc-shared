//! `[lints.<tool>]` / `[workspace.lints.<tool>]` tables and `[lints]`.

#![expect(
    clippy::excessive_nesting,
    clippy::type_complexity,
    reason = "Lint requirement composition groups collected requirements by lint tool."
)]

use std::collections::BTreeMap;

use aqc_file_engine_core::{
    ConflictEntry, ItemRequirements, KeyedItem, Provenance, ResolvedItemRequirements,
    ResolvedRequirement, push_conflict, resolve_items, resolve_scalar,
};

/// Required lint level and optional priority for one lint key.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LintSetting {
    pub level: String,
    pub priority: Option<i64>,
}

/// `[lints]` is one either/or decision.
#[derive(Debug, Clone)]
pub enum PackageLintsAssertion {
    Inherit(bool, String),
    Inline(BTreeMap<String, ItemRequirements<KeyedItem<LintSetting>>>),
}

/// Resolved package-lints decision.
#[derive(Debug, Clone)]
pub enum ResolvedPackageLintsAssertion {
    Inherit(ResolvedRequirement<bool, (bool, String)>),
    Inline(BTreeMap<String, ResolvedItemRequirements<KeyedItem<LintSetting>>>),
}

impl PartialEq for PackageLintsAssertion {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Inherit(a, _), Self::Inherit(b, _)) => a == b,
            (Self::Inline(a), Self::Inline(b)) => a == b,
            _ => false,
        }
    }
}

impl PackageLintsAssertion {
    pub fn resolve(
        key: &str,
        items: Vec<(Provenance, Self)>,
        conflicts: &mut Vec<ConflictEntry>,
    ) -> Option<ResolvedPackageLintsAssertion> {
        let all_inherit = items.iter().all(|(_, a)| matches!(a, Self::Inherit(..)));
        let all_inline = items.iter().all(|(_, a)| matches!(a, Self::Inline(..)));

        if all_inherit {
            let inherited = items
                .into_iter()
                .filter_map(|(prov, assertion)| match assertion {
                    Self::Inherit(value, msg) => Some((prov, (value, msg))),
                    Self::Inline(_) => None,
                })
                .collect::<Vec<_>>();
            let resolved =
                resolve_scalar(key, inherited, |(value, _)| value.to_string(), conflicts)?;
            return Some(ResolvedPackageLintsAssertion::Inherit(
                ResolvedRequirement {
                    merged: resolved.merged.0,
                    collected: resolved.collected,
                },
            ));
        }

        if all_inline {
            let mut by_tool: BTreeMap<
                String,
                Vec<(Provenance, ItemRequirements<KeyedItem<LintSetting>>)>,
            > = BTreeMap::new();
            for (prov, assertion) in items {
                let Self::Inline(tables) = assertion else {
                    continue;
                };
                for (tool, table) in tables {
                    by_tool.entry(tool).or_default().push((prov.clone(), table));
                }
            }
            let mut resolved = BTreeMap::new();
            for (tool, tables) in by_tool {
                let _ = resolved.insert(
                    tool.clone(),
                    resolve_items(&format!("{key}.{tool}"), tables, conflicts),
                );
            }
            return Some(ResolvedPackageLintsAssertion::Inline(resolved));
        }

        push_conflict(
            key,
            "scalar-disagree",
            &items,
            |assertion| match assertion {
                Self::Inherit(value, _) => format!("inherit workspace={value}"),
                Self::Inline(tables) => {
                    let names = tables.keys().cloned().collect::<Vec<_>>().join(", ");
                    format!("inline tables {names}")
                }
            },
            conflicts,
        );
        None
    }
}

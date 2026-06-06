//! The shared assertion-impl macros used by the per-target modules.

/// Implement [`Resolve`](aqc_file_engine_core::Resolve) for a
/// `Contains(map) | Excludes(map) | IsExactly(map)` assertion enum.
///
/// All-`Contains` unions the entry maps via
/// [`merge_map_by`](aqc_file_engine_core::merge_map_by) comparing only the
/// projected semantic value (`$project` — the policy message never
/// participates); all-`Excludes` unions first-wins; anything else (mixed
/// kinds, or all-`IsExactly`) must agree identically via the scalar rule.
macro_rules! impl_set_resolve {
    ($t:ty, $entries:ty, $project:expr) => {
        impl aqc_file_engine_core::Resolve for $t {
            fn resolve(
                key: &str,
                contributions: aqc_file_engine_core::merge::Contributions<Self>,
                conflicts: &mut Vec<aqc_file_engine_core::ConflictEntry>,
            ) -> Option<Self> {
                if contributions
                    .iter()
                    .all(|(_, a)| matches!(a, Self::Contains(_)))
                {
                    let maps: aqc_file_engine_core::merge::Contributions<$entries> = contributions
                        .into_iter()
                        .filter_map(|(p, a)| match a {
                            Self::Contains(m) => Some((p, m)),
                            Self::Excludes(_) | Self::IsExactly(_) => None,
                        })
                        .collect();
                    let project = $project;
                    return Some(Self::Contains(aqc_file_engine_core::merge_map_by(
                        key,
                        maps,
                        &project,
                        |v| format!("{:?}", project(v)),
                        conflicts,
                    )));
                }
                if contributions
                    .iter()
                    .all(|(_, a)| matches!(a, Self::Excludes(_)))
                {
                    let maps: Vec<_> = contributions
                        .into_iter()
                        .filter_map(|(_, a)| match a {
                            Self::Excludes(m) => Some(m),
                            Self::Contains(_) | Self::IsExactly(_) => None,
                        })
                        .collect();
                    return Some(Self::Excludes(aqc_file_engine_core::union_first_wins(maps)));
                }
                aqc_file_engine_core::resolve_scalar(
                    key,
                    contributions,
                    |a| format!("{a:?}"),
                    conflicts,
                )
            }
        }
    };
}
pub(crate) use impl_set_resolve;

/// Implement semantic [`PartialEq`] for a keyed-entries assertion enum:
/// `Contains`/`IsExactly` compare names + values (messages excluded) via
/// [`keyed_entries_eq`](aqc_file_engine_core::keyed_entries_eq); `Excludes`
/// compares the banned names only.
macro_rules! impl_keyed_entries_eq {
    ($t:ty) => {
        impl PartialEq for $t {
            fn eq(&self, other: &Self) -> bool {
                match (self, other) {
                    (Self::Contains(a), Self::Contains(b))
                    | (Self::IsExactly(a), Self::IsExactly(b)) => {
                        aqc_file_engine_core::keyed_entries_eq(a, b)
                    }
                    (Self::Excludes(a), Self::Excludes(b)) => {
                        a.len() == b.len() && a.keys().eq(b.keys())
                    }
                    _ => false,
                }
            }
        }
    };
}
pub(crate) use impl_keyed_entries_eq;

//! Resolved item membership data model.

use std::collections::{BTreeMap, BTreeSet};

use super::{
    AllowedItemsInput, ConflictEntry, ExactItemsInput, ForbiddenItemMap, ForbiddenItemResolution,
    ItemAssertionInput, ItemRequirementMap, RequiredItemResolution,
};
use crate::types::Provenance;

/// A composed exact item collection with its complete attribution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedExactItems<Item>
where
    Item: FileItemRequirement,
{
    pub identities: BTreeSet<Item::Identity>,
    pub items: ItemRequirementMap<Item>,
    pub collected: Vec<ExactItemsInput<Item>>,
}

/// A composed optional-identity allowlist with complete attribution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedAllowedItems<Item>
where
    Item: FileItemRequirement,
{
    pub identities: BTreeSet<Item::Identity>,
    pub collected: Vec<AllowedItemsInput<Item>>,
}

/// Resolved item requirements with attribution on every member.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedItemRequirements<Item>
where
    Item: FileItemRequirement,
{
    pub required: ItemRequirementMap<Item>,
    pub forbidden: ForbiddenItemMap<Item>,
    pub allowed: Option<ResolvedAllowedItems<Item>>,
    pub exact: Option<ResolvedExactItems<Item>>,
}

/// The active closed-membership assertion for a resolved item collection.
#[derive(Debug, Clone, Copy)]
pub enum ResolvedItemMembership<'a, Item>
where
    Item: FileItemRequirement,
{
    Allowed(&'a ResolvedAllowedItems<Item>),
    Exact(&'a ResolvedExactItems<Item>),
}

impl<'a, Item> ResolvedItemMembership<'a, Item>
where
    Item: FileItemRequirement,
{
    #[must_use]
    pub const fn is_exact(&self) -> bool {
        matches!(self, Self::Exact(_))
    }

    #[must_use]
    pub const fn identities(&self) -> &'a BTreeSet<Item::Identity> {
        match self {
            Self::Allowed(allowed) => &allowed.identities,
            Self::Exact(exact) => &exact.identities,
        }
    }

    #[must_use]
    pub fn all_attribution(&self) -> Vec<Provenance> {
        match self {
            Self::Allowed(allowed) => allowed
                .collected
                .iter()
                .map(|(provenance, _)| provenance.clone())
                .collect(),
            Self::Exact(exact) => exact
                .collected
                .iter()
                .map(|(provenance, _)| provenance.clone())
                .collect(),
        }
    }

    #[must_use]
    pub fn message_for_rejected(&self, mut is_allowed: impl FnMut(&Item) -> bool) -> &'a str {
        match self {
            Self::Allowed(allowed) => allowed
                .collected
                .iter()
                .find(|(_, (items, _))| !items.iter().any(&mut is_allowed))
                .map_or("", |(_, (_, message))| message.as_str()),
            Self::Exact(exact) => exact
                .collected
                .iter()
                .find(|(_, (items, _))| !items.iter().any(&mut is_allowed))
                .map_or("", |(_, (_, message))| message.as_str()),
        }
    }

    #[must_use]
    pub fn attribution_for_rejected(
        &self,
        mut is_allowed: impl FnMut(&Item) -> bool,
    ) -> Vec<Provenance> {
        match self {
            Self::Allowed(allowed) => allowed
                .collected
                .iter()
                .filter(|(_, (items, _))| !items.iter().any(&mut is_allowed))
                .map(|(provenance, _)| provenance.clone())
                .collect(),
            Self::Exact(exact) => exact
                .collected
                .iter()
                .filter(|(_, (items, _))| !items.iter().any(&mut is_allowed))
                .map(|(provenance, _)| provenance.clone())
                .collect(),
        }
    }
}

impl<Item> ResolvedItemRequirements<Item>
where
    Item: FileItemRequirement,
{
    /// Return the strongest active closed-membership assertion.
    #[must_use]
    pub const fn membership(&self) -> Option<ResolvedItemMembership<'_, Item>> {
        if let Some(exact) = &self.exact {
            Some(ResolvedItemMembership::Exact(exact))
        } else if let Some(allowed) = &self.allowed {
            Some(ResolvedItemMembership::Allowed(allowed))
        } else {
            None
        }
    }
}

/// A file-item requirement that can identify and compose matching policy input.
pub trait FileItemRequirement: Sized + Clone {
    type Identity: Ord + Clone + std::fmt::Debug;

    fn merge_identity(&self) -> Self::Identity;

    fn compose_item(
        key: &str,
        items: Vec<ItemAssertionInput<Self>>,
        conflicts: &mut Vec<ConflictEntry>,
    ) -> Option<RequiredItemResolution<Self>>;
}

/// Differences between present file-item identities and resolved requirements.
#[derive(Debug)]
pub struct ItemPresenceDifference<'a, Item>
where
    Item: FileItemRequirement,
{
    pub missing: Vec<(&'a Item::Identity, &'a RequiredItemResolution<Item>)>,
    pub forbidden: Vec<(&'a Item::Identity, &'a ForbiddenItemResolution<Item>)>,
    pub unexpected: Vec<&'a Item::Identity>,
}

impl<Item> Default for ResolvedItemRequirements<Item>
where
    Item: FileItemRequirement,
{
    fn default() -> Self {
        Self {
            required: BTreeMap::new(),
            forbidden: BTreeMap::new(),
            allowed: None,
            exact: None,
        }
    }
}

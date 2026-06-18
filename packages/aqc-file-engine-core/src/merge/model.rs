//! Merge data model and public aliases.

use std::collections::{BTreeMap, BTreeSet};

use crate::types::Provenance;

pub type Provenanced<T> = (Provenance, T);
pub type Contributor = (Provenance, String);
pub type Contributors = Vec<Contributor>;
pub type Collected<A> = Vec<Provenanced<A>>;
pub type MessagePair<T> = (T, String);
pub type ItemAssertion<Item> = MessagePair<Item>;
pub type ItemAssertionInput<Item> = Provenanced<ItemAssertion<Item>>;
pub type ItemInput<Item> = Provenanced<ItemRequirements<Item>>;
pub type ItemAssertionGroups<Item> =
    BTreeMap<<Item as FileItemRequirement>::Identity, Vec<ItemAssertionInput<Item>>>;
pub type RequiredItemResolution<Item> = ResolvedRequirement<Item, ItemAssertion<Item>>;
pub type BannedItemResolution<Item> = ResolvedRequirement<Item, String>;
pub type ItemRequirementMap<Item> =
    BTreeMap<<Item as FileItemRequirement>::Identity, RequiredItemResolution<Item>>;
pub type BannedItemMap<Item> =
    BTreeMap<<Item as FileItemRequirement>::Identity, BannedItemResolution<Item>>;
pub type GlobAssertion<Glob> = MessagePair<Glob>;
pub type GlobAssertionInput<Glob> = Provenanced<GlobAssertion<Glob>>;
pub type GlobInput<Glob> = Provenanced<ForbiddenGlobRequirements<Glob>>;
pub type GlobAssertionGroups<Glob> =
    BTreeMap<<Glob as ForbiddenGlobRequirement>::Identity, Vec<GlobAssertionInput<Glob>>>;
pub type GlobResolutionMap<Glob> =
    BTreeMap<<Glob as ForbiddenGlobRequirement>::Identity, ResolvedRequirement<Glob, String>>;
pub type ListInput = Provenanced<ListRequirements>;
pub type ListExact = MessagePair<Vec<String>>;
pub type ExactInput = Provenanced<ListExact>;
pub type ResolvedExactList = ResolvedRequirement<Vec<String>, ListExact>;
pub type ResolvedStringMembers = BTreeMap<String, ResolvedRequirement<(), String>>;
pub type MemberInputs = BTreeMap<String, Contributors>;
pub type ClosedInput<Item> = (
    Provenance,
    String,
    BTreeSet<<Item as FileItemRequirement>::Identity>,
);
pub type MapInput<K, A> = Provenanced<BTreeMap<K, A>>;
pub type MapInputs<K, A> = Vec<MapInput<K, A>>;
pub type GroupedAssertions<K, A> = BTreeMap<K, Vec<Provenanced<A>>>;
pub type ResolvedMap<K, A> = BTreeMap<K, ResolvedRequirement<<A as Resolve>::Merged, A>>;
pub type ResolvedAssertion<A> = ResolvedRequirement<<A as Resolve>::Merged, A>;
pub type ResolvedAssertionOption<A> = Option<ResolvedAssertion<A>>;
pub type ResolvedSame<T> = ResolvedRequirement<T, T>;
pub type ResolvedSameOption<T> = Option<ResolvedSame<T>>;
pub type OptionalInput<T> = Provenanced<Option<T>>;
pub type VersionFloor = MessagePair<String>;
pub type KeyedValueMap<S, M> = BTreeMap<String, (S, M)>;

/// One key on which policies irreconcilably disagree, with each value.
#[derive(Debug, Clone)]
pub struct ConflictEntry {
    /// The disagreeing key.
    pub key: String,
    /// Which composition rule found the conflict.
    pub reason: String,
    /// Each provenance paired with its value, rendered for display.
    pub contributors: Contributors,
}

/// A composed requirement plus the policy assertions used to compose it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedRequirement<Merged, A> {
    pub merged: Merged,
    pub collected: Collected<A>,
}

/// Product requirement for collections of identifiable file items.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemRequirements<Item> {
    pub required: Vec<ItemAssertion<Item>>,
    pub banned: Vec<ItemAssertion<Item>>,
    pub closed: Option<String>,
}

impl<Item> Default for ItemRequirements<Item> {
    fn default() -> Self {
        Self {
            required: Vec::new(),
            banned: Vec::new(),
            closed: None,
        }
    }
}

/// Resolved item requirements with attribution on every member.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedItemRequirements<Item>
where
    Item: FileItemRequirement,
{
    pub required: ItemRequirementMap<Item>,
    pub banned: BannedItemMap<Item>,
    pub closed_by: Contributors,
}

impl<Item> Default for ResolvedItemRequirements<Item>
where
    Item: FileItemRequirement,
{
    fn default() -> Self {
        Self {
            required: BTreeMap::new(),
            banned: BTreeMap::new(),
            closed_by: Vec::new(),
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

/// Forbidden-glob requirement for collections where policy names a glob.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForbiddenGlobRequirements<Glob> {
    pub globs: Vec<GlobAssertion<Glob>>,
}

impl<Glob> Default for ForbiddenGlobRequirements<Glob> {
    fn default() -> Self {
        Self { globs: Vec::new() }
    }
}

/// Resolved glob forbids with attribution on each glob.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedForbiddenGlobRequirements<Glob>
where
    Glob: ForbiddenGlobRequirement,
{
    pub globs: GlobResolutionMap<Glob>,
}

impl<Glob> Default for ResolvedForbiddenGlobRequirements<Glob>
where
    Glob: ForbiddenGlobRequirement,
{
    fn default() -> Self {
        Self {
            globs: BTreeMap::new(),
        }
    }
}

/// A forbidden glob that can be deduped across policy input.
pub trait ForbiddenGlobRequirement: Sized + Clone {
    type Identity: Ord + Clone + std::fmt::Debug;

    fn merge_identity(&self) -> Self::Identity;

    fn render(&self) -> String;
}

/// Requirement for collections where the file key is the item identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyedItem<Value> {
    pub file_key: String,
    pub value: Value,
}

/// Product requirement for list-like TOML fields.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ListRequirements {
    pub contains: BTreeMap<String, String>,
    pub excludes: BTreeMap<String, String>,
    pub exact: Option<ListExact>,
}

/// Resolved list requirements with per-item and exact-list attribution.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ResolvedListRequirements {
    pub contains: ResolvedStringMembers,
    pub excludes: ResolvedStringMembers,
    pub exact: Option<ResolvedExactList>,
}

/// Assertion types that compose several policy assertions into one value.
pub trait Resolve: Sized + Clone {
    type Merged: Clone;

    fn resolve(
        key: &str,
        items: Vec<Provenanced<Self>>,
        conflicts: &mut Vec<ConflictEntry>,
    ) -> ResolvedAssertionOption<Self>;
}

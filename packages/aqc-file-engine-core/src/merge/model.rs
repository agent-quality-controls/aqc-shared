//! Merge data model and public aliases.

use std::cmp::Ordering;
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
pub type ForbiddenItemResolution<Item> = ResolvedRequirement<Item, String>;
pub type ItemRequirementMap<Item> =
    BTreeMap<<Item as FileItemRequirement>::Identity, RequiredItemResolution<Item>>;
pub type ForbiddenItemMap<Item> =
    BTreeMap<<Item as FileItemRequirement>::Identity, ForbiddenItemResolution<Item>>;
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
pub type ExactItems<Item> = MessagePair<Vec<Item>>;
pub type ExactItemsInput<Item> = Provenanced<ExactItems<Item>>;
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

pub(crate) fn sort_provenanced<T>(items: &mut [Provenanced<T>]) {
    items.sort_by(|left, right| left.0.cmp(&right.0));
}

/// Generic scalar assertion verbs shared by file engines.
#[derive(Debug, Clone)]
pub enum ScalarAssertion<T> {
    Equals(T, String),
    AtLeast(T, String),
    AtMost(T, String),
    Range(T, T, String),
    OneOf(BTreeSet<T>, String),
    Present(String),
    Absent(String),
}

/// Scalar assertion operation without its value payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ScalarOperation {
    Equals,
    AtLeast,
    AtMost,
    Range,
    OneOf,
    Present,
    Absent,
}

/// Value behavior needed by generic scalar assertion composition.
pub trait ScalarValue: Clone + Eq + Ord {
    fn render(&self) -> String;

    fn compare_for_order(&self, other: &Self) -> Option<Ordering>;
}

impl<T> ScalarAssertion<T> {
    #[must_use]
    pub const fn operation(&self) -> ScalarOperation {
        match self {
            Self::Equals(..) => ScalarOperation::Equals,
            Self::AtLeast(..) => ScalarOperation::AtLeast,
            Self::AtMost(..) => ScalarOperation::AtMost,
            Self::Range(..) => ScalarOperation::Range,
            Self::OneOf(..) => ScalarOperation::OneOf,
            Self::Present(_) => ScalarOperation::Present,
            Self::Absent(_) => ScalarOperation::Absent,
        }
    }

    #[must_use]
    pub fn message(&self) -> &str {
        match self {
            Self::Equals(_, msg)
            | Self::AtLeast(_, msg)
            | Self::AtMost(_, msg)
            | Self::Range(_, _, msg)
            | Self::OneOf(_, msg)
            | Self::Present(msg)
            | Self::Absent(msg) => msg,
        }
    }
}

impl<T: PartialEq> PartialEq for ScalarAssertion<T> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Equals(a, _), Self::Equals(b, _))
            | (Self::AtLeast(a, _), Self::AtLeast(b, _))
            | (Self::AtMost(a, _), Self::AtMost(b, _)) => a == b,
            (Self::Range(a_min, a_max, _), Self::Range(b_min, b_max, _)) => {
                a_min == b_min && a_max == b_max
            }
            (Self::OneOf(a, _), Self::OneOf(b, _)) => a == b,
            (Self::Present(_), Self::Present(_)) | (Self::Absent(_), Self::Absent(_)) => true,
            _ => false,
        }
    }
}

impl<T: Eq> Eq for ScalarAssertion<T> {}

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

impl<Merged, Assertion> ResolvedRequirement<Merged, Assertion> {
    /// Return every contributor in the established collection order.
    #[must_use]
    pub fn attribution(&self) -> Vec<Provenance> {
        self.collected
            .iter()
            .map(|(provenance, _)| provenance.clone())
            .collect()
    }
}

/// Return the distinct contributors to a resolved map in provenance order.
#[must_use]
pub fn resolved_map_attribution<K, A>(requirements: &ResolvedMap<K, A>) -> Vec<Provenance>
where
    K: Ord,
    A: Resolve,
{
    let mut attribution = requirements
        .values()
        .flat_map(ResolvedRequirement::attribution)
        .collect::<Vec<_>>();
    attribution.sort();
    attribution.dedup();
    attribution
}

/// Product requirement for collections of identifiable file items.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemRequirements<Item> {
    pub required: Vec<ItemAssertion<Item>>,
    pub forbidden: Vec<ItemAssertion<Item>>,
    pub exact: Option<ExactItems<Item>>,
}

/// A requirement whose value semantics constrain whether its containing file key exists.
pub trait FileKeyRequirement {
    fn constrain_file_key(&self, file_key: &str, membership: &mut ItemRequirements<KeyedItem<()>>);
}

impl<Requirement> FileKeyRequirement for Option<Requirement>
where
    Requirement: FileKeyRequirement,
{
    fn constrain_file_key(&self, file_key: &str, membership: &mut ItemRequirements<KeyedItem<()>>) {
        if let Some(requirement) = self {
            requirement.constrain_file_key(file_key, membership);
        }
    }
}

impl<T> FileKeyRequirement for ScalarAssertion<T> {
    fn constrain_file_key(&self, file_key: &str, membership: &mut ItemRequirements<KeyedItem<()>>) {
        let item = KeyedItem {
            file_key: file_key.to_owned(),
            value: (),
        };
        match self {
            Self::Absent(message) => membership.forbidden.push((item, message.clone())),
            Self::Equals(_, message)
            | Self::AtLeast(_, message)
            | Self::AtMost(_, message)
            | Self::Range(_, _, message)
            | Self::OneOf(_, message)
            | Self::Present(message) => membership.required.push((item, message.clone())),
        }
    }
}

impl FileKeyRequirement for ListRequirements {
    fn constrain_file_key(&self, file_key: &str, membership: &mut ItemRequirements<KeyedItem<()>>) {
        for message in self
            .contains
            .values()
            .chain(self.exact.iter().map(|(_, message)| message))
        {
            membership.required.push((
                KeyedItem {
                    file_key: file_key.to_owned(),
                    value: (),
                },
                message.clone(),
            ));
        }
    }
}

impl<Item> FileKeyRequirement for ItemRequirements<Item> {
    fn constrain_file_key(&self, file_key: &str, membership: &mut ItemRequirements<KeyedItem<()>>) {
        for message in self.required.iter().map(|(_, message)| message).chain(
            self.exact
                .iter()
                .filter(|(items, _)| !items.is_empty())
                .map(|(_, message)| message),
        ) {
            membership.required.push((
                KeyedItem {
                    file_key: file_key.to_owned(),
                    value: (),
                },
                message.clone(),
            ));
        }
    }
}

impl<Item> ItemRequirements<Item> {
    /// Transform every item while preserving messages and collection structure.
    pub fn map<Mapped>(self, mut map_item: impl FnMut(Item) -> Mapped) -> ItemRequirements<Mapped> {
        ItemRequirements {
            required: self
                .required
                .into_iter()
                .map(|(item, message)| (map_item(item), message))
                .collect(),
            forbidden: self
                .forbidden
                .into_iter()
                .map(|(item, message)| (map_item(item), message))
                .collect(),
            exact: self
                .exact
                .map(|(items, message)| (items.into_iter().map(&mut map_item).collect(), message)),
        }
    }
}

impl<Item> Default for ItemRequirements<Item> {
    fn default() -> Self {
        Self {
            required: Vec::new(),
            forbidden: Vec::new(),
            exact: None,
        }
    }
}

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

/// Resolved item requirements with attribution on every member.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedItemRequirements<Item>
where
    Item: FileItemRequirement,
{
    pub required: ItemRequirementMap<Item>,
    pub forbidden: ForbiddenItemMap<Item>,
    pub exact: Option<ResolvedExactItems<Item>>,
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
            exact: None,
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

/// Membership and order differences between two exact string lists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExactListDifference {
    current_counts: BTreeMap<String, usize>,
    expected_counts: BTreeMap<String, usize>,
    missing: BTreeMap<String, usize>,
    unexpected: BTreeMap<String, usize>,
    order_mismatch: bool,
}

impl ExactListDifference {
    pub(crate) const fn new(
        current_counts: BTreeMap<String, usize>,
        expected_counts: BTreeMap<String, usize>,
        missing: BTreeMap<String, usize>,
        unexpected: BTreeMap<String, usize>,
        order_mismatch: bool,
    ) -> Self {
        Self {
            current_counts,
            expected_counts,
            missing,
            unexpected,
            order_mismatch,
        }
    }

    /// Number of occurrences in the current list.
    #[must_use]
    pub fn current_count(&self, member: &str) -> usize {
        self.current_counts.get(member).copied().unwrap_or_default()
    }

    /// Number of occurrences in the expected list.
    #[must_use]
    pub fn expected_count(&self, member: &str) -> usize {
        self.expected_counts
            .get(member)
            .copied()
            .unwrap_or_default()
    }

    /// Expected member counts not present in the current list.
    #[must_use]
    pub const fn missing(&self) -> &BTreeMap<String, usize> {
        &self.missing
    }

    /// Current member counts not allowed by the expected list.
    #[must_use]
    pub const fn unexpected(&self) -> &BTreeMap<String, usize> {
        &self.unexpected
    }

    /// Whether only list order differs.
    #[must_use]
    pub const fn order_mismatch(&self) -> bool {
        self.order_mismatch
    }

    /// Whether the current and expected lists are equal.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.missing.is_empty() && self.unexpected.is_empty() && !self.order_mismatch
    }
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

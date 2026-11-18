//! Typed indices and the [`IndexMap`] that holds indexed metadata.
//!
//! Every `element[n]` in the metadata section has its own index type, so a
//! reference cannot point at the wrong kind of element:
//!
//! ```compile_fail
//! use mztab_m::{AssayId, SampleId};
//!
//! fn sample_of(_: SampleId) {}
//! sample_of(AssayId::new(1).unwrap()); // an assay is not a sample
//! ```
//!
//! ```
//! use mztab_m::{AssayId, IndexMap};
//!
//! let mut names = IndexMap::new();
//! let a2 = AssayId::new(2).ok_or("zero")?;
//! assert_eq!(names.insert(a2, "second"), None);
//! assert_eq!(names.insert(AssayId::new(1).ok_or("zero")?, "first"), None);
//! assert_eq!(names.get(a2), Some(&"second"));
//! assert_eq!(names.keys().map(AssayId::get).collect::<Vec<_>>(), [1, 2]);
//! assert_eq!(a2.to_string(), "assay[2]");
//! # Ok::<(), &str>(())
//! ```

use alloc::vec::Vec;
use core::fmt;
use core::num::NonZeroU32;

/// An index type for an `element[n]`, used by the reader to parse
/// references such as `assay[3]` generically.
pub(crate) trait ElementId: Copy {
    /// The element name.
    const ELEMENT: &'static str;

    /// The id with index `n`, or `None` for `0`.
    fn from_index(n: u32) -> Option<Self>;
}

macro_rules! ids {
    ($($(#[$doc:meta])* $name:ident => $element:literal;)+) => {$(
        $(#[$doc])*
        ///
        /// Wraps the 1-based index `n` of the element; `0` is unrepresentable.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(NonZeroU32);

        impl $name {
            /// The metadata element name, such as `assay` for [`AssayId`]. Empty
            /// for [`Ix`], which indexes repeated fields rather than elements.
            pub const ELEMENT: &'static str = $element;

            /// The id with index `n`, or `None` for `0`.
            #[must_use]
            pub const fn new(n: u32) -> Option<Self> {
                match NonZeroU32::new(n) {
                    Some(n) => Some(Self(n)),
                    None => None,
                }
            }

            /// The index `n`.
            #[must_use]
            pub const fn get(self) -> u32 {
                self.0.get()
            }
        }

        impl ElementId for $name {
            const ELEMENT: &'static str = $element;

            fn from_index(n: u32) -> Option<Self> {
                Self::new(n)
            }
        }

        impl From<NonZeroU32> for $name {
            fn from(n: NonZeroU32) -> Self {
                Self(n)
            }
        }

        #[doc = concat!("Formats as `", $element, "[n]`.")]
        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}[{}]", $element, self.0)
            }
        }
    )+};
}

ids! {
    /// The index of an `assay[n]`.
    AssayId => "assay";
    /// The index of a `contact[n]`.
    ContactId => "contact";
    /// The index of a `cv[n]`.
    CvId => "cv";
    /// The index of a `database[n]`.
    DatabaseId => "database";
    /// The index of an `id_confidence_measure[n]`.
    IdConfidenceMeasureId => "id_confidence_measure";
    /// The index of an `instrument[n]`.
    InstrumentId => "instrument";
    /// The index of an `ms_run[n]`.
    MsRunId => "ms_run";
    /// The index of a `protocol[n]`.
    ProtocolId => "protocol";
    /// The index of a `publication[n]`.
    PublicationId => "publication";
    /// The index of a `sample[n]`.
    SampleId => "sample";
    /// The index of a `software[n]`.
    SoftwareId => "software";
    /// The index of a `study_variable[n]`.
    StudyVariableId => "study_variable";
    /// The index of a `study_variable_group[n]`.
    StudyVariableGroupId => "study_variable_group";
    /// An index of a repeated field that is not itself a referable element,
    /// such as the `[m]` in `sample[n]-species[m]` or in `custom[m]`.
    Ix => "";
}

/// A map from an index type to values, kept sorted by index.
///
/// mzTab indices are small and dense, so a sorted `Vec` beats a tree or a
/// hash map on both speed and size, and keeps iteration deterministic with no
/// dependency. Indices need not be contiguous: a file with `assay[1]` and
/// `assay[3]` is representable, so the validator can report the gap.
///
/// # Examples
///
/// ```
/// use mztab_m::{IndexMap, MsRunId};
///
/// let run = |n| MsRunId::new(n).ok_or("zero");
/// let mut locations = IndexMap::new();
/// locations.insert(run(3)?, "file:///c.mzML");
/// locations.insert(run(1)?, "file:///a.mzML");
/// assert_eq!(locations.len(), 2);
/// assert_eq!(locations.insert(run(1)?, "file:///b.mzML"), Some("file:///a.mzML"));
/// assert_eq!(locations.get(run(2)?), None);
/// assert_eq!(locations.iter().map(|(k, _)| k.get()).collect::<Vec<_>>(), [1, 3]);
/// # Ok::<(), &str>(())
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct IndexMap<K, V> {
    entries: Vec<(K, V)>,
}

impl<K, V> Default for IndexMap<K, V> {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
        }
    }
}

impl<K: Ord + Copy, V> IndexMap<K, V> {
    /// An empty map.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Inserts `value` at `key`, returning the value it replaces.
    ///
    /// Appending in increasing key order, the common case when reading a
    /// file, is amortised O(1).
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        if self.entries.last().is_none_or(|(last, _)| *last < key) {
            self.entries.push((key, value));
            return None;
        }
        match self.entries.binary_search_by(|(k, _)| k.cmp(&key)) {
            Ok(at) => self
                .entries
                .get_mut(at)
                .map(|(_, slot)| core::mem::replace(slot, value)),
            Err(at) => {
                self.entries.insert(at, (key, value));
                None
            }
        }
    }

    /// The value at `key`.
    #[must_use]
    pub fn get(&self, key: K) -> Option<&V> {
        let at = self.entries.binary_search_by(|(k, _)| k.cmp(&key)).ok()?;
        self.entries.get(at).map(|(_, v)| v)
    }

    /// The value at `key`, mutably.
    ///
    /// # Examples
    ///
    /// ```
    /// use mztab_m::{IndexMap, Ix};
    ///
    /// let one = Ix::new(1).ok_or("zero")?;
    /// let mut map = IndexMap::new();
    /// map.insert(one, 1);
    /// if let Some(v) = map.get_mut(one) {
    ///     *v += 1;
    /// }
    /// assert_eq!(map.get(one), Some(&2));
    /// # Ok::<(), &str>(())
    /// ```
    #[must_use]
    pub fn get_mut(&mut self, key: K) -> Option<&mut V> {
        let at = self.entries.binary_search_by(|(k, _)| k.cmp(&key)).ok()?;
        self.entries.get_mut(at).map(|(_, v)| v)
    }

    /// Whether `key` has a value.
    #[must_use]
    pub fn contains_key(&self, key: K) -> bool {
        self.get(key).is_some()
    }

    /// The number of entries.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the map is empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The entries in increasing key order.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = (K, &V)> {
        self.entries.iter().map(|(k, v)| (*k, v))
    }

    /// The keys in increasing order.
    pub fn keys(&self) -> impl ExactSizeIterator<Item = K> {
        self.entries.iter().map(|(k, _)| *k)
    }

    /// The values in increasing key order.
    pub fn values(&self) -> impl ExactSizeIterator<Item = &V> {
        self.entries.iter().map(|(_, v)| v)
    }

    /// The value at `key`, inserting `V::default()` first if there is none.
    ///
    /// # Examples
    ///
    /// ```
    /// use mztab_m::{IndexMap, Ix};
    ///
    /// let one = Ix::new(1).ok_or("zero")?;
    /// let mut map: IndexMap<Ix, Vec<u8>> = IndexMap::new();
    /// map.get_or_insert_default(one).push(7);
    /// map.get_or_insert_default(one).push(8);
    /// assert_eq!(map.get(one), Some(&vec![7, 8]));
    /// # Ok::<(), &str>(())
    /// ```
    #[expect(
        clippy::indexing_slicing,
        reason = "`at` was just found or just inserted, so it is in bounds"
    )]
    pub fn get_or_insert_default(&mut self, key: K) -> &mut V
    where
        V: Default,
    {
        let at = match self.entries.binary_search_by(|(k, _)| k.cmp(&key)) {
            Ok(at) => at,
            Err(at) => {
                self.entries.insert(at, (key, V::default()));
                at
            }
        };
        let (_, value) = &mut self.entries[at];
        value
    }
}

impl<K, V> IntoIterator for IndexMap<K, V> {
    type Item = (K, V);
    type IntoIter = alloc::vec::IntoIter<(K, V)>;

    /// The entries in increasing key order.
    fn into_iter(self) -> Self::IntoIter {
        self.entries.into_iter()
    }
}

/// Collects entries in any order; a later entry replaces an earlier one with
/// the same key.
impl<K: Ord + Copy, V> FromIterator<(K, V)> for IndexMap<K, V> {
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        let mut map = Self::new();
        for (k, v) in iter {
            map.insert(k, v);
        }
        map
    }
}

#[cfg(test)]
mod tests {
    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    use super::{AssayId, IndexMap, Ix, StudyVariableGroupId};
    use alloc::string::ToString as _;
    use alloc::vec::Vec;
    use core::num::NonZeroU32;

    fn ix(n: u32) -> Ix {
        Ix::new(n).expect("non-zero index")
    }

    #[test]
    fn ids() {
        assert_eq!(AssayId::new(0), None, "zero");
        assert_eq!(AssayId::new(7).map(AssayId::get), Some(7), "get");
        assert_eq!(AssayId::from(NonZeroU32::MIN).get(), 1, "from");
        assert_eq!(
            StudyVariableGroupId::ELEMENT,
            "study_variable_group",
            "element"
        );
        assert_eq!(
            StudyVariableGroupId::new(4).map(|i| i.to_string()),
            Some("study_variable_group[4]".into()),
            "display"
        );
    }

    #[test]
    fn index_map_orders_and_replaces() {
        assert_eq!(
            IndexMap::<Ix, u8>::new(),
            IndexMap::default(),
            "new is default"
        );
        let mut map: IndexMap<Ix, u8> = IndexMap::default();
        assert!(map.is_empty(), "empty");
        for n in [5, 1, 3, 9, 2] {
            assert_eq!(
                map.insert(ix(n), u8::try_from(n).unwrap_or_default()),
                None,
                "insert {n}"
            );
        }
        assert_eq!(map.insert(ix(3), 30), Some(3), "replace middle");
        assert_eq!(map.insert(ix(9), 90), Some(9), "replace last");
        assert_eq!(map.insert(ix(1), 10), Some(1), "replace first");
        assert_eq!(
            map.keys().map(Ix::get).collect::<Vec<_>>(),
            [1, 2, 3, 5, 9],
            "sorted"
        );
        assert_eq!(
            map.values().copied().collect::<Vec<_>>(),
            [10, 2, 30, 5, 90],
            "values"
        );
        assert_eq!(map.len(), 5, "len");
        assert!(!map.is_empty(), "not empty");
        assert!(map.contains_key(ix(5)), "has 5");
        assert!(!map.contains_key(ix(4)), "no 4");
        assert_eq!(map.get_mut(ix(4)), None, "no 4 mut");
        map.get_mut(ix(2)).into_iter().for_each(|v| *v = 20);
        assert_eq!(map.get(ix(2)), Some(&20), "get_mut writes");
        assert_eq!(map.iter().next(), Some((ix(1), &10)), "iter");
    }
}

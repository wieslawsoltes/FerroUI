//! The candidate index of a collection of styles: for a style key, the entries of the collection
//! that can match an element of that key, in source order.
//!
//! Only a style whose selector was index-enabled (`Style::enable_selector_index`) while it was
//! added to the collection can be left out, and only when the type its selector requires rules
//! the key out. Every other entry stays a candidate and goes through the ordinary matcher. In
//! particular the type requirement of a parent says nothing about its children, which are
//! visited on their own: a style with children is always a candidate.
//!
//! A collection and the index-enabled styles in it share one [`IndexState`]. The collection
//! allocates it when the first such style is added; a collection that never held one has no
//! state and is traversed as it always was, after one check. The styles hold the state weakly
//! and report what the index read from them (the selector, the parent, whether there are
//! children), so a change invalidates the lists of the collection that holds the style and of
//! no other.

use super::IStyle;
use crate::TypeInfo;
use rustc_hash::{FxHashMap, FxHashSet};
use std::cell::{Cell, OnceCell, RefCell};
use std::rc::{Rc, Weak};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct TypeCandidate {
    pub type_: &'static TypeInfo,
    pub exact: bool,
}

impl TypeCandidate {
    fn matches(self, target: &'static TypeInfo) -> bool {
        if self.exact {
            self.type_ == target
        } else {
            self.type_.is_assignable_from(target)
        }
    }
}

/// A collection shorter than this is traversed whole: a list would save nothing.
const MIN_ENTRIES: usize = 8;

/// The style keys a collection remembers a list for. The keys are the types of the elements the
/// collection was applied to, so an application stays far below this; a key beyond it gets its
/// list computed at each use and nothing that is remembered is dropped for it.
pub(super) const MAX_KEYS: usize = 1024;

#[cfg(test)]
crate::ferro_thread_local! {
    static SCANS: Cell<usize> = const { Cell::new(0) };
}

/// The number of times a collection was scanned for the candidates of a key, on this thread.
#[cfg(test)]
pub(super) fn scans() -> usize {
    SCANS.with(Cell::get)
}

/// What a collection and the index-enabled styles in it share.
#[derive(Default)]
pub(super) struct IndexState {
    /// The entries linked to this state that have a type candidate. At zero the collection is
    /// traversed whole and nothing else is read.
    enabled: Cell<u32>,
    /// Advances when the collection changes or when a linked entry changes what the index read
    /// from it. The lists of an older generation are not used, and a traversal that began in an
    /// older generation goes on through every remaining entry.
    generation: Cell<u64>,
    cache: RefCell<Option<Box<Cache>>>,
}

/// The lists of one snapshot of the collection in one generation.
struct Cache {
    /// The snapshot, held weakly: a change of the collection makes a new allocation, and this
    /// one cannot be given out again while it is held here.
    source: Weak<Vec<Rc<dyn IStyle>>>,
    generation: u64,
    /// Style key to its list; `None` when every entry is a candidate.
    keys: FxHashMap<usize, Option<Rc<[u32]>>>,
    /// The distinct lists: keys that leave the same entries out share one.
    lists: FxHashSet<Rc<[u32]>>,
}

impl Cache {
    fn is_for(&self, source: &Rc<Vec<Rc<dyn IStyle>>>, generation: u64) -> bool {
        self.generation == generation && std::ptr::eq(self.source.as_ptr(), Rc::as_ptr(source))
    }
}

impl IndexState {
    fn changed(&self) {
        self.generation.set(self.generation.get().wrapping_add(1));
        // Nothing of the old generation is used again. A lookup never runs code of a style
        // while it holds the borrow, so this is free; if it were not, the generation alone
        // already rules the lists out.
        if let Ok(mut cache) = self.cache.try_borrow_mut() {
            *cache = None;
        }
    }

    fn cached(&self, source: &Rc<Vec<Rc<dyn IStyle>>>, generation: u64, key: usize) -> Option<Option<Rc<[u32]>>> {
        let cache = self.cache.borrow();
        cache.as_ref().filter(|cache| cache.is_for(source, generation))?.keys.get(&key).cloned()
    }

    /// The entries of `source` that can match `target`, in source order. Reads the styles
    /// linked to this state and nothing of any other entry: an unknown implementation of
    /// [`IStyle`] is not asked for its object or its children ahead of its turn.
    fn scan(self: &Rc<Self>, source: &[Rc<dyn IStyle>], target: &'static TypeInfo) -> Vec<u32> {
        #[cfg(test)]
        SCANS.with(|scans| scans.set(scans.get() + 1));
        source
            .iter()
            .enumerate()
            .filter_map(|(index, entry)| {
                let keep = match entry.indexed_style() {
                    Some(style) if style.index_link().is(self) => {
                        style.has_children() || style.type_candidate().is_none_or(|candidate| candidate.matches(target))
                    }
                    _ => true,
                };
                keep.then_some(index as u32)
            })
            .collect()
    }

    fn remember(
        &self,
        source: &Rc<Vec<Rc<dyn IStyle>>>,
        generation: u64,
        key: usize,
        indices: Vec<u32>,
    ) -> Option<Rc<[u32]>> {
        let mut cache = self.cache.borrow_mut();
        if !cache.as_ref().is_some_and(|cache| cache.is_for(source, generation)) {
            *cache = None;
        }
        let cache = cache.get_or_insert_with(|| {
            Box::new(Cache {
                source: Rc::downgrade(source),
                generation,
                keys: FxHashMap::default(),
                lists: FxHashSet::default(),
            })
        });
        let room = cache.keys.len() < MAX_KEYS;
        if indices.len() == source.len() {
            if room {
                cache.keys.insert(key, None);
            }
            return None;
        }
        let list = match cache.lists.get(indices.as_slice()) {
            Some(list) => list.clone(),
            None => {
                let list: Rc<[u32]> = indices.into();
                if room {
                    cache.lists.insert(list.clone());
                }
                list
            }
        };
        if room {
            cache.keys.insert(key, Some(list.clone()));
        }
        Some(list)
    }
}

/// The link of a style to the state of the collection that counts it as index-enabled: the
/// collection the style was added to while it had a type candidate. A style is linked to one
/// collection at a time; in any other collection it is a candidate for every key.
#[derive(Default)]
pub(super) struct IndexLink(Cell<Weak<IndexState>>);

impl IndexLink {
    fn state(&self) -> Option<Rc<IndexState>> {
        let link = self.0.take();
        let state = link.upgrade();
        self.0.set(link);
        state
    }

    fn is(&self, state: &Rc<IndexState>) -> bool {
        let link = self.0.take();
        let same = std::ptr::eq(link.as_ptr(), Rc::as_ptr(state));
        self.0.set(link);
        same
    }

    /// Something the index reads from the style, other than its type candidate, changed: its
    /// parent, or whether it has children.
    pub(super) fn changed(&self) {
        if let Some(state) = self.state() {
            state.changed();
        }
    }

    /// The type candidate of the style changed; `was` and `is` say whether it had and has one.
    pub(super) fn candidate_changed(&self, was: bool, is: bool) {
        if let Some(state) = self.state() {
            match (was, is) {
                (false, true) => state.enabled.set(state.enabled.get() + 1),
                (true, false) => state.enabled.set(state.enabled.get().saturating_sub(1)),
                _ => {}
            }
            state.changed();
        }
    }
}

/// The candidate index of one collection of styles.
#[derive(Default)]
pub(crate) struct CandidateCache(OnceCell<Rc<IndexState>>);

impl CandidateCache {
    /// `entry` was added to the collection; a child style after its parent was set.
    pub(crate) fn added(&self, entry: &Rc<dyn IStyle>) {
        if let Some(style) = entry.indexed_style() {
            // The candidate as the index reads it: none below a container query.
            if style.type_candidate().is_some() {
                let state = self.0.get_or_init(Default::default);
                // A style that another collection still counts stays with that one.
                if style.index_link().state().is_none() {
                    style.index_link().0.set(Rc::downgrade(state));
                    state.enabled.set(state.enabled.get() + 1);
                }
            }
        }
        self.changed();
    }

    /// `entry` was removed from the collection.
    pub(crate) fn removed(&self, entry: &Rc<dyn IStyle>) {
        let Some(state) = self.0.get() else { return };
        if let Some(style) = entry.indexed_style() {
            if style.index_link().is(state) {
                style.index_link().0.set(Weak::new());
                if style.has_type_candidate() {
                    state.enabled.set(state.enabled.get().saturating_sub(1));
                }
            }
        }
        state.changed();
    }

    /// The collection changed in a way that added and removed nothing (a move).
    pub(crate) fn changed(&self) {
        if let Some(state) = self.0.get() {
            state.changed();
        }
    }

    pub(crate) fn select<'a>(
        &self,
        source: Rc<Vec<Rc<dyn IStyle>>>,
        target: &'a crate::StyledElement,
    ) -> StyleCandidates<'a> {
        // A collection without an index-enabled entry costs this check.
        let Some(state) = self.0.get().filter(|state| state.enabled.get() != 0) else {
            return StyleCandidates::all(source);
        };
        if source.len() < MIN_ENTRIES || source.len() > u32::MAX as usize {
            return StyleCandidates::all(source);
        }
        let generation = state.generation.get();
        // The getter of the key is virtual and can run code that changes styles.
        let key = target.style_key();
        let address = key as *const TypeInfo as usize;
        let indices = match state.cached(&source, generation, address) {
            Some(indices) => indices,
            None => {
                let indices = state.scan(&source, key);
                if target.style_key() != key || generation != state.generation.get() {
                    return StyleCandidates::all(source);
                }
                state.remember(&source, generation, address, indices)
            }
        };
        match indices {
            Some(indices) => StyleCandidates {
                source,
                cursor: 0,
                narrowed: Some(Narrowed { indices, offset: 0, state: state.clone(), generation, target, key }),
            },
            None => StyleCandidates::all(source),
        }
    }

    /// The entries counted as index-enabled, the keys with a remembered list and the distinct
    /// lists; `None` when the collection has no state.
    #[cfg(test)]
    pub(crate) fn stats(&self) -> Option<(u32, usize, usize)> {
        let state = self.0.get()?;
        let cache = state.cache.borrow();
        let (keys, lists) = cache.as_ref().map_or((0, 0), |cache| (cache.keys.len(), cache.lists.len()));
        Some((state.enabled.get(), keys, lists))
    }
}

/// The candidate styles of a collection for an element, in source order. Styles that are not
/// index-enabled are all candidates and go through the ordinary matcher.
#[doc(hidden)]
pub struct StyleCandidates<'a> {
    source: Rc<Vec<Rc<dyn IStyle>>>,
    /// The position after the entry returned last.
    cursor: usize,
    narrowed: Option<Narrowed<'a>>,
}

struct Narrowed<'a> {
    indices: Rc<[u32]>,
    offset: usize,
    state: Rc<IndexState>,
    generation: u64,
    target: &'a crate::StyledElement,
    key: &'static TypeInfo,
}

impl StyleCandidates<'_> {
    pub(crate) fn all(source: Rc<Vec<Rc<dyn IStyle>>>) -> Self {
        Self { source, cursor: 0, narrowed: None }
    }

    fn next_index(&mut self) -> Option<usize> {
        // Attaching a style can run code that replaces a later selector, adds children to a
        // later style or changes the key of the element. The traversal then goes on from this
        // position through every entry of the snapshot. The key is read first: its getter can
        // run such code itself, and the generation is read after it.
        if let Some(narrowed) = &self.narrowed {
            if narrowed.target.style_key() != narrowed.key || narrowed.generation != narrowed.state.generation.get() {
                self.narrowed = None;
            }
        }
        let index = match &mut self.narrowed {
            Some(narrowed) => {
                let index = *narrowed.indices.get(narrowed.offset)? as usize;
                narrowed.offset += 1;
                index
            }
            None => self.cursor,
        };
        self.source.get(index)?;
        self.cursor = index + 1;
        Some(index)
    }

    pub(crate) fn unfiltered(&self) -> Option<&[Rc<dyn IStyle>]> {
        self.narrowed.is_none().then_some(self.source.as_slice())
    }

    /// Visits a candidate without cloning its style handle.
    pub fn next_style(&mut self) -> Option<&dyn IStyle> {
        let index = self.next_index()?;
        Some(&*self.source[index])
    }
}

impl Iterator for StyleCandidates<'_> {
    type Item = Rc<dyn IStyle>;

    fn next(&mut self) -> Option<Self::Item> {
        let index = self.next_index()?;
        Some(self.source[index].clone())
    }
}

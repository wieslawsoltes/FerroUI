use super::IStyle;
use crate::TypeInfo;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::{Rc, Weak};

crate::ferro_thread_local! {
    static EPOCH: Cell<u64> = const { Cell::new(0) };
}

pub(super) fn invalidate() {
    EPOCH.with(|epoch| {
        epoch.set(
            epoch
                .get()
                .checked_add(1)
                .expect("selector index epoch exhausted"),
        )
    });
}

fn epoch() -> u64 {
    EPOCH.with(Cell::get)
}

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

// Index only generated styles with a proven type requirement. Everything else
// stays in source order and goes through the ordinary matcher. In particular a
// parent's type requirement says nothing about its independently visited children.
#[derive(Default)]
pub(crate) struct CandidateCache(RefCell<Option<Box<Cache>>>);

struct Cache {
    source: Weak<Vec<Rc<dyn IStyle>>>,
    epoch: u64,
    types: HashMap<usize, Option<Rc<Vec<usize>>>>,
}

impl CandidateCache {
    pub(crate) fn select<'a>(
        &self,
        source: Rc<Vec<Rc<dyn IStyle>>>,
        target: &'a crate::StyledElement,
    ) -> StyleCandidates<'a> {
        // Tiny collections stay on the original snapshot traversal path.
        if source.len() < 8 {
            return StyleCandidates::all(source);
        }
        let epoch = epoch();
        let mut result = StyleCandidates {
            source,
            indices: None,
            epoch,
            cursor: 0,
            offset: 0,
            target: None,
        };
        let element = target;
        let target = target.style_key();
        let key = target as *const TypeInfo as usize;
        {
            let cache = self.0.borrow();
            if let Some(cache) = cache
                .as_ref()
                .filter(|c| c.epoch == epoch && c.source.as_ptr() == Rc::as_ptr(&result.source))
            {
                if let Some(indices) = cache.types.get(&key) {
                    result.indices = indices.clone();
                    result.target = result.indices.as_ref().map(|_| (element, target));
                    return result;
                }
            }
        }
        // Unknown adapters do not expose a concrete style. In particular, do
        // not call their as_object/children methods ahead of source-order work.
        let indices: Vec<_> = result
            .source
            .iter()
            .enumerate()
            .filter_map(|(i, entry)| {
                let keep = entry.indexed_style().is_none_or(|s| {
                    s.has_children() || s.type_candidate().is_none_or(|c| c.matches(target))
                });
                keep.then_some(i)
            })
            .collect();
        if element.style_key() != target || epoch != self::epoch() {
            return result;
        }
        result.indices = (indices.len() != result.source.len()).then(|| Rc::new(indices));
        let mut cache = self.0.borrow_mut();
        if cache
            .as_ref()
            .is_none_or(|c| c.epoch != epoch || c.source.as_ptr() != Rc::as_ptr(&result.source))
        {
            *cache = Some(Box::new(Cache {
                source: Rc::downgrade(&result.source),
                epoch,
                types: HashMap::new(),
            }));
        }
        let cache = cache.as_mut().unwrap();
        // Bound per-host storage even for applications with many style keys.
        if cache.types.len() >= 32 && !cache.types.contains_key(&key) {
            cache.types.clear();
        }
        cache.types.insert(key, result.indices.clone());
        result.target = result.indices.as_ref().map(|_| (element, target));
        result
    }
}

/// Source-ordered candidate styles. Unknown selectors use the full matcher.
#[doc(hidden)]
pub struct StyleCandidates<'a> {
    source: Rc<Vec<Rc<dyn IStyle>>>,
    indices: Option<Rc<Vec<usize>>>,
    epoch: u64,
    cursor: usize,
    offset: usize,
    target: Option<(&'a crate::StyledElement, &'static TypeInfo)>,
}

impl StyleCandidates<'_> {
    pub(crate) fn all(source: Rc<Vec<Rc<dyn IStyle>>>) -> Self {
        Self {
            source,
            indices: None,
            epoch: 0,
            cursor: 0,
            offset: 0,
            target: None,
        }
    }
}

impl StyleCandidates<'_> {
    fn next_index(&mut self) -> Option<usize> {
        // A setter/custom style can replace a later selector or add children
        // during attachment. Continue the original snapshot from this position
        // through the full matcher; never hold the cache borrow while attaching.
        if self.indices.is_some() {
            // A virtual key getter can itself reenter styling. Read the epoch
            // after that call, before consuming a narrowed source index.
            let key_changed = self
                .target
                .is_some_and(|(target, key)| target.style_key() != key);
            if key_changed || self.epoch != epoch() {
                self.indices = None;
            }
        }
        let index = match &self.indices {
            Some(indices) => {
                let index = *indices.get(self.offset)?;
                self.offset += 1;
                index
            }
            None => self.cursor,
        };
        self.source.get(index)?;
        self.cursor = index + 1;
        Some(index)
    }
}

impl StyleCandidates<'_> {
    pub(crate) fn unfiltered(&self) -> Option<&[Rc<dyn IStyle>]> {
        self.indices.is_none().then_some(self.source.as_slice())
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

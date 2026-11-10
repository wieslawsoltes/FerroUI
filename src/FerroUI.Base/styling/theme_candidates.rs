//! The nested styles of a control theme that can apply to an element of one style key, in the
//! order the styles are applied in.
//!
//! A control theme is applied to the control it themes and to every part of the template of
//! that control, and each application visits every nested style of the theme, at any depth: a
//! part that no style selects still pays for all of them. The plan of a theme for a style key
//! lists, in the order of that traversal, the nested styles whose selector does not rule the key
//! out, so an application visits those and no other.
//!
//! A style is left out only when the type its selector requires is known and rules the key out
//! ([`SelectorBound`]): the type of the first node of the target compound of the selector, or,
//! for a selector that begins with the nesting selector, the type the selector of the parent
//! requires, up to the target type of the theme. Either is what the matcher evaluates first,
//! and a mismatch ends the match there, before anything is subscribed to. Only a [`Style`]
//! whose selector was index-enabled (`Style::enable_selector_index`) has such a type. A style
//! that has none is always applied. A container query, an unknown implementation of [`IStyle`]
//! and a style whose parent is not the style that holds it are applied with everything below
//! them by the ordinary traversal, in their place.
//!
//! The plans of a theme are dropped when something they were made from changes: a collection
//! of nested styles, a selector, a parent, the target type of the theme. A change while a plan
//! is applied (attaching a style can run code) ends the use of the plan, and the traversal goes
//! on from that entry as the ordinary one does (`StyledElement::apply_control_theme_core`).

use super::selector_candidates::TypeCandidate;
use super::{ControlTheme, IStyle, StyleBase};
use crate::{StyledElement, TypeInfo};
use rustc_hash::FxHashMap;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The type an element must have for a selector to match it, as far as the selector says so
/// before it evaluates anything else.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum SelectorBound {
    /// The selector is not index-enabled, or it begins with a node that says nothing of a type.
    #[default]
    Unknown,
    /// The first node of the target compound requires the type.
    Type(TypeCandidate),
    /// The target compound begins with the nesting selector: the selector matches no element
    /// the selector of the parent style does not match.
    Parent,
}

impl SelectorBound {
    pub(super) fn type_candidate(self) -> Option<TypeCandidate> {
        match self {
            SelectorBound::Type(candidate) => Some(candidate),
            _ => None,
        }
    }
}

/// The style keys a theme remembers a plan for: the types of the control and of the parts of
/// its template. A key beyond this is applied by the ordinary traversal.
pub(super) const MAX_KEYS: usize = 64;

/// The depth of nested styles a plan follows. A theme nested deeper than this has no plans,
/// which it finds out once.
/// The walk that reports a change to the theme (`StyleBase::structure_changed`) goes further
/// up than this, so every style a plan holds reaches its theme.
pub(super) const MAX_DEPTH: usize = 16;

#[cfg(test)]
crate::ferro_thread_local! {
    static BUILDS: Cell<usize> = const { Cell::new(0) };
}

/// The number of plans built on this thread.
#[cfg(test)]
pub(super) fn builds() -> usize {
    BUILDS.with(Cell::get)
}

/// What the traversal does with an entry of a plan.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Step {
    /// The style is attached if it matches; what is below it follows in the plan.
    Attach,
    /// The style cannot match the key and is not attached; styles below it follow in the plan.
    Descend,
    /// The entry is applied by the ordinary traversal, with everything below it.
    Whole,
}

const NO_PARENT: u32 = u32::MAX;

#[derive(Clone, Copy, PartialEq, Eq)]
struct Node {
    /// The collection the style is in, an index into [`Plan::collections`].
    collection: u32,
    /// The position of the style in that collection.
    index: u32,
    /// The entry of the style that owns the collection, [`NO_PARENT`] for the theme.
    parent: u32,
    step: Step,
}

/// The entries of a theme for one style key, parents before their children, in source order.
struct Plan {
    /// The collections the entries are in, as they were when the plan was made.
    collections: Box<[Rc<Vec<Rc<dyn IStyle>>>]>,
    nodes: Box<[Node]>,
}

struct Builder {
    key: &'static TypeInfo,
    collections: Vec<Rc<Vec<Rc<dyn IStyle>>>>,
    nodes: Vec<Node>,
}

impl Builder {
    /// Adds the entries of `source`, the children of `owner`, that can apply to the key, and
    /// what is below them. `bound` is the type the selector of `owner` requires, when it is
    /// known. `None` when the styles are nested too deep for a plan.
    fn collection(
        &mut self,
        source: Rc<Vec<Rc<dyn IStyle>>>,
        owner: &StyleBase,
        bound: Option<TypeCandidate>,
        parent: u32,
        depth: usize,
    ) -> Option<()> {
        if depth > MAX_DEPTH || source.len() >= NO_PARENT as usize {
            return None;
        }
        let collection = self.collections.len() as u32;
        self.collections.push(source.clone());
        let first = self.nodes.len();
        for (index, entry) in source.iter().enumerate() {
            let index = index as u32;
            // The nesting selector of a style reads the parent of the style, which is the
            // owner of the collection unless the style was added to another one since.
            let Some(style) = entry.indexed_style().filter(|style| style.is_child_of(owner)) else {
                self.nodes.push(Node { collection, index, parent, step: Step::Whole });
                continue;
            };
            let own = match style.selector_bound() {
                SelectorBound::Type(candidate) => Some(candidate),
                SelectorBound::Parent => bound,
                SelectorBound::Unknown => None,
            };
            let applies = own.is_none_or(|bound| bound.matches(self.key));
            let position = self.nodes.len();
            if position >= NO_PARENT as usize {
                return None;
            }
            self.nodes.push(Node { collection, index, parent, step: if applies { Step::Attach } else { Step::Descend } });
            if style.has_children() {
                self.collection(style.children().snapshot(), style, own, position as u32, depth + 1)?;
            }
            // A style that cannot match is kept for the styles below it alone.
            if !applies && self.nodes.len() == position + 1 {
                self.nodes.pop();
            }
        }
        if self.nodes.len() == first {
            self.collections.pop();
        }
        Some(())
    }
}

fn build(theme: &ControlTheme, key: &'static TypeInfo) -> Option<Plan> {
    #[cfg(test)]
    BUILDS.with(|builds| builds.set(builds.get() + 1));
    let mut builder = Builder { key, collections: Vec::new(), nodes: Vec::new() };
    // What the nesting selector of a child of the theme evaluates.
    let bound = theme.target_type().map(|type_| TypeCandidate { type_, exact: false });
    builder.collection(theme.children().snapshot(), theme, bound, NO_PARENT, 0)?;
    Some(Plan { collections: builder.collections.into(), nodes: builder.nodes.into() })
}

/// The plans of one snapshot of the nested styles of a theme.
struct Cache {
    version: u64,
    /// Style key to its plan. Keys with the same entries share one plan.
    keys: FxHashMap<usize, Rc<Plan>>,
    /// The styles are nested too deep for a plan, whatever the key.
    too_deep: bool,
}

/// The plans of a control theme.
#[derive(Default)]
pub(crate) struct ThemePlans {
    /// Advances when something a plan is made from changes. The plans of an older version are
    /// not used, and an application that began in an older version goes on without its plan.
    version: Cell<u64>,
    cache: RefCell<Option<Box<Cache>>>,
}

impl ThemePlans {
    /// Something a plan is made from changed.
    pub(super) fn changed(&self) {
        self.version.set(self.version.get().wrapping_add(1));
        // Nothing of the old version is used again. A lookup never runs code of a style while
        // it holds the borrow, so this is free; if it were not, the version alone already
        // rules the plans out.
        if let Ok(mut cache) = self.cache.try_borrow_mut() {
            *cache = None;
        }
    }

    /// The nested styles of `theme` that can apply to `target`; `None` when the ordinary
    /// traversal applies them.
    pub(super) fn select<'a>(&'a self, theme: &ControlTheme, target: &'a StyledElement) -> Option<ThemeCandidates<'a>> {
        // The getter of the key is virtual and can run code that changes styles: the version is
        // read after it.
        let key = target.style_key();
        let version = self.version.get();
        let address = key as *const TypeInfo as usize;
        let cached = {
            let cache = self.cache.borrow();
            match cache.as_ref().filter(|cache| cache.version == version) {
                Some(cache) if cache.too_deep => return None,
                Some(cache) => match cache.keys.get(&address) {
                    Some(plan) => Some(plan.clone()),
                    None if cache.keys.len() >= MAX_KEYS => return None,
                    None => None,
                },
                None => None,
            }
        };
        let plan = match cached {
            Some(plan) => plan,
            None => {
                let plan = build(theme, key);
                if target.style_key() != key || self.version.get() != version {
                    return None;
                }
                self.remember(version, address, plan)?
            }
        };
        Some(ThemeCandidates { plan, plans: self, version, target, key })
    }

    fn remember(&self, version: u64, key: usize, plan: Option<Plan>) -> Option<Rc<Plan>> {
        let mut cache = self.cache.borrow_mut();
        if !cache.as_ref().is_some_and(|cache| cache.version == version) {
            *cache = None;
        }
        let cache =
            cache.get_or_insert_with(|| Box::new(Cache { version, keys: FxHashMap::default(), too_deep: false }));
        let Some(plan) = plan else {
            cache.too_deep = true;
            return None;
        };
        // Equal entries are in equal collections: a collection is numbered by the first entry
        // in it, and the plans of one version are made from the same styles.
        let plan = match cache.keys.values().find(|known| known.nodes == plan.nodes) {
            Some(known) => known.clone(),
            None => Rc::new(plan),
        };
        cache.keys.insert(key, plan.clone());
        Some(plan)
    }

    /// The keys with a plan, the distinct plans and the entries of all of them.
    #[cfg(test)]
    pub(super) fn stats(&self) -> (usize, usize, usize) {
        let cache = self.cache.borrow();
        let Some(cache) = cache.as_ref() else { return (0, 0, 0) };
        let mut distinct: Vec<&Rc<Plan>> = Vec::new();
        for plan in cache.keys.values() {
            if !distinct.iter().any(|known| Rc::ptr_eq(known, plan)) {
                distinct.push(plan);
            }
        }
        (cache.keys.len(), distinct.len(), distinct.iter().map(|plan| plan.nodes.len()).sum())
    }
}

/// The nested styles of a control theme that can apply to an element, in the order they are
/// applied in.
pub(crate) struct ThemeCandidates<'a> {
    plan: Rc<Plan>,
    plans: &'a ThemePlans,
    version: u64,
    target: &'a StyledElement,
    key: &'static TypeInfo,
}

impl ThemeCandidates<'_> {
    pub(crate) fn len(&self) -> usize {
        self.plan.nodes.len()
    }

    /// The entry at `position` and what to do with it.
    pub(crate) fn get(&self, position: usize) -> (&dyn IStyle, Step) {
        let node = &self.plan.nodes[position];
        (&*self.plan.collections[node.collection as usize][node.index as usize], node.step)
    }

    /// Whether the plan still says what the ordinary traversal would visit. Attaching a style
    /// can run code that replaces a selector, changes a collection of nested styles or changes
    /// the key of the element. The key is read first: its getter can run such code itself.
    pub(crate) fn is_current(&self) -> bool {
        self.target.style_key() == self.key && self.version == self.plans.version.get()
    }

    /// The styles that follow the entry at `position` in its collection, as the collection was
    /// when the plan was made: every one of them, whatever the plan says of it.
    pub(crate) fn after(&self, position: usize) -> &[Rc<dyn IStyle>] {
        let node = &self.plan.nodes[position];
        &self.plan.collections[node.collection as usize][node.index as usize + 1..]
    }

    /// The entry of the style that holds the entry at `position`; `None` for a child of the
    /// theme.
    pub(crate) fn parent(&self, position: usize) -> Option<usize> {
        let parent = self.plan.nodes[position].parent;
        (parent != NO_PARENT).then_some(parent as usize)
    }

    /// The steps of the plan, in order.
    #[cfg(test)]
    pub(super) fn steps(&self) -> Vec<(Rc<dyn IStyle>, Step)> {
        self.plan
            .nodes
            .iter()
            .map(|node| (self.plan.collections[node.collection as usize][node.index as usize].clone(), node.step))
            .collect()
    }
}

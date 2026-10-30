//! Shared visual highlighter layers with weak item and collection subscriptions.
//! Source: WinUI TextHighlighterCollection.cpp; see NOTICE.md.

use super::{TextHighlightMerge, TextHighlightRegion, TextHighlighter};
use ferroui_base::collections::FerroList;
use ferroui_base::media::IBrush;
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{IntoRef, Ref};
use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::fmt;
use std::ops::{Deref, Range};
use std::rc::{Rc, Weak};

#[derive(Clone)]
pub struct TextHighlighterCollection(Rc<HighlighterCollectionData>);

struct HighlighterCollectionData {
    list: FerroList<Ref<TextHighlighter>>,
    list_subscription: Cell<Option<u64>>,
    item_subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
    invalidated: Rc<HandlerList<dyn Fn()>>,
}

/// A weak handle suitable for callbacks retained by a highlighter or a view.
#[derive(Clone)]
pub struct WeakTextHighlighterCollection(Weak<HighlighterCollectionData>);
impl WeakTextHighlighterCollection {
    pub fn upgrade(&self) -> Option<TextHighlighterCollection> {
        self.0.upgrade().map(TextHighlighterCollection)
    }
}

impl PartialEq for TextHighlighterCollection {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
impl fmt::Debug for TextHighlighterCollection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("TextHighlighterCollection")
            .field(&self.count())
            .finish()
    }
}
impl Default for TextHighlighterCollection {
    fn default() -> Self {
        Self::new()
    }
}
impl Deref for TextHighlighterCollection {
    type Target = FerroList<Ref<TextHighlighter>>;
    fn deref(&self) -> &Self::Target {
        &self.0.list
    }
}

impl TextHighlighterCollection {
    pub fn new() -> Self {
        let collection = Self(Rc::new(HighlighterCollectionData {
            list: FerroList::new(),
            list_subscription: Cell::new(None),
            item_subscriptions: RefCell::new(Vec::new()),
            invalidated: Rc::new(HandlerList::new()),
        }));
        let weak = collection.downgrade();
        let token = collection.0.list.add_collection_changed(Rc::new(move |_| {
            if let Some(collection) = weak.upgrade() {
                collection.refresh_item_subscriptions();
                collection.invalidate();
            }
        }));
        collection.0.list_subscription.set(Some(token));
        collection
    }

    pub fn from_items(items: impl IntoIterator<Item = Ref<TextHighlighter>>) -> Self {
        let collection = Self::new();
        collection.0.list.add_range(items);
        collection
    }

    pub fn downgrade(&self) -> WeakTextHighlighterCollection {
        WeakTextHighlighterCollection(Rc::downgrade(&self.0))
    }
    pub fn add(&self, highlighter: impl IntoRef<TextHighlighter>) {
        self.0.list.add(highlighter.into_ref());
    }
    pub fn insert(&self, index: usize, highlighter: impl IntoRef<TextHighlighter>) {
        self.0
            .list
            .insert(index.min(self.count()), highlighter.into_ref());
    }
    pub fn set(&self, index: usize, highlighter: impl IntoRef<TextHighlighter>) -> bool {
        if index >= self.count() {
            return false;
        }
        self.0.list.set(index, highlighter.into_ref());
        true
    }
    pub fn remove(&self, highlighter: impl IntoRef<TextHighlighter>) -> bool {
        self.0.list.remove(&highlighter.into_ref())
    }
    pub fn remove_at(&self, index: usize) -> Option<Ref<TextHighlighter>> {
        let old = self.0.list.try_get(index)?;
        self.0.list.remove_at(index);
        Some(old)
    }
    pub fn snapshot(&self) -> Rc<Vec<Ref<TextHighlighter>>> {
        self.0.list.snapshot()
    }

    pub fn invalidated(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        let token = self.0.invalidated.add(Rc::new(handler));
        let weak = Rc::downgrade(&self.0.invalidated);
        Disposable::create(move || {
            if let Some(handlers) = weak.upgrade() {
                handlers.remove(token);
            }
        })
    }

    /// Produces ordered, disjoint regions, ignoring invalid/empty ranges and
    /// clipping ranges to the view's content. Null brushes use the supplied
    /// theme defaults rather than inheriting a lower layer's brush.
    pub fn merged_regions(
        &self,
        content_length: usize,
        default_foreground: Option<Rc<dyn IBrush>>,
        default_background: Option<Rc<dyn IBrush>>,
    ) -> Vec<TextHighlightRegion> {
        self.merged_regions_with(
            content_length,
            default_foreground,
            default_background,
            |range| range,
        )
    }

    /// Normalizes each source layer before merging. A document view can widen
    /// boundaries that fall inside a UTF-16 surrogate pair to scalar boundaries,
    /// retaining collection-order priority even when two half-pairs overlap.
    pub fn merged_regions_with(
        &self,
        content_length: usize,
        default_foreground: Option<Rc<dyn IBrush>>,
        default_background: Option<Rc<dyn IBrush>>,
        mut normalize: impl FnMut(Range<usize>) -> Range<usize>,
    ) -> Vec<TextHighlightRegion> {
        let mut merge = TextHighlightMerge::new();
        for highlighter in self.snapshot().iter() {
            let foreground = highlighter
                .foreground()
                .or_else(|| default_foreground.clone());
            let background = highlighter
                .background()
                .or_else(|| default_background.clone());
            for value in highlighter.ranges().snapshot().iter() {
                let Some(range) = value.range() else {
                    continue;
                };
                let range = range.start.min(content_length)..range.end.min(content_length);
                if range.is_empty() {
                    continue;
                }
                let range = normalize(range);
                let range = range.start.min(content_length)..range.end.min(content_length);
                merge.add_region(TextHighlightRegion {
                    range,
                    foreground: foreground.clone(),
                    background: background.clone(),
                });
            }
        }
        merge.into_regions()
    }

    fn invalidate(&self) {
        for (_, handler) in self.0.invalidated.snapshot().iter() {
            handler();
        }
    }

    fn refresh_item_subscriptions(&self) {
        let old = std::mem::take(&mut *self.0.item_subscriptions.borrow_mut());
        for subscription in old {
            subscription.dispose();
        }
        let mut seen = HashSet::new();
        let mut subscriptions = Vec::new();
        for highlighter in self.snapshot().iter() {
            if !seen.insert(highlighter.clone()) {
                continue;
            }
            let weak = self.downgrade();
            subscriptions.push(highlighter.invalidated(move || {
                if let Some(collection) = weak.upgrade() {
                    collection.invalidate();
                }
            }));
        }
        *self.0.item_subscriptions.borrow_mut() = subscriptions;
    }
}

impl Drop for HighlighterCollectionData {
    fn drop(&mut self) {
        if let Some(token) = self.list_subscription.take() {
            self.list.remove_collection_changed(token);
        }
        for subscription in self.item_subscriptions.get_mut().drain(..) {
            subscription.dispose();
        }
    }
}

ferroui_base::ferro_markup_type!(class TextHighlighterCollection {
    namespace: "FerroUI.Controls.Documents",
    handles: [TextHighlighterCollection, Option<TextHighlighterCollection>],
    base: FerroList<Ref<TextHighlighter>>,
    constructors: [() => TextHighlighterCollection::new],
    methods: [
        fn Add(Ref<TextHighlighter>) => |collection: &TextHighlighterCollection, item: Ref<TextHighlighter>| collection.add(item),
    ],
});

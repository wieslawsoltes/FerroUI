// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferrodock_model::collections::ListHandle;
use ferroui_base::collections::FerroList;
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::Ref;
use ferroui_controls::Control;
use std::rc::Rc;
/// Source IList<Control> boundary. Native, plain and complete custom source lists
/// retain their own storage, mutation and notification capabilities.
#[derive(Clone, PartialEq)]
pub enum OverlayControlList {
    /// Actual UI framework list (the OverlayHost constructor's source default).
    Native(FerroList<Option<Ref<Control>>>),
    /// General source IList, including nonnotifying and custom implementations.
    General(ListHandle<Option<Ref<Control>>>),
}
impl Default for OverlayControlList {
    fn default() -> Self {
        Self::new()
    }
}
impl OverlayControlList {
    /// Creates a native notifying list.
    pub fn new() -> Self {
        Self::Native(FerroList::new())
    }
    /// Creates an ordinary nonnotifying list.
    pub fn plain(items: Vec<Option<Ref<Control>>>) -> Self {
        Self::General(ListHandle::from_plain(items))
    }
    /// Gets Count.
    pub fn count(&self) -> usize {
        match self {
            Self::Native(l) => l.count(),
            Self::General(l) => l.count(),
        }
    }
    /// Gets whether this backend rejects mutation.
    pub fn is_read_only(&self) -> bool {
        match self {
            Self::Native(_) => false,
            Self::General(list) => list.is_read_only(),
        }
    }
    /// Finds the first matching element with this backend's comparer.
    pub fn index_of(&self, item: &Option<Ref<Control>>) -> Option<usize> {
        match self {
            Self::Native(list) => list.index_of(item),
            Self::General(list) => list.index_of(item),
        }
    }
    /// Tests membership using this backend's original implementation.
    pub fn contains(&self, item: &Option<Ref<Control>>) -> bool {
        match self {
            Self::Native(list) => list.contains(item),
            Self::General(list) => list.contains(item),
        }
    }
    /// Copies into an existing destination buffer.
    pub fn copy_to(&self, destination: &mut [Option<Ref<Control>>], index: usize) {
        match self {
            Self::Native(list) => list.copy_to(destination, index),
            Self::General(list) => list.copy_to(destination, index),
        }
    }
    /// Gets an item.
    pub fn get(&self, index: usize) -> Option<Ref<Control>> {
        match self {
            Self::Native(l) => l.get(index),
            Self::General(l) => l.get(index),
        }
    }
    /// Sets an item.
    pub fn set(&self, index: usize, item: Option<Ref<Control>>) {
        match self {
            Self::Native(l) => l.set(index, item),
            Self::General(l) => l.set(index, item),
        }
    }
    /// Adds an item.
    pub fn add(&self, item: Option<Ref<Control>>) {
        match self {
            Self::Native(l) => l.add(item),
            Self::General(l) => l.add(item),
        }
    }
    /// Inserts an item.
    pub fn insert(&self, index: usize, item: Option<Ref<Control>>) {
        match self {
            Self::Native(l) => l.insert(index, item),
            Self::General(l) => l.insert(index, item),
        }
    }
    /// Removes an item by index.
    pub fn remove_at(&self, index: usize) {
        match self {
            Self::Native(l) => l.remove_at(index),
            Self::General(l) => l.remove_at(index),
        }
    }
    /// Removes the first matching item.
    pub fn remove(&self, item: &Option<Ref<Control>>) -> bool {
        match self {
            Self::Native(l) => l.remove(item),
            Self::General(l) => l.remove(item),
        }
    }
    /// Clears this list.
    pub fn clear(&self) {
        match self {
            Self::Native(l) => l.clear(),
            Self::General(l) => l.clear(),
        }
    }
    /// Returns a fresh source enumerator.
    pub fn iter(&self) -> Box<dyn Iterator<Item = Option<Ref<Control>>>> {
        match self {
            Self::General(list) => list.iter(),
            Self::Native(list) => {
                let list = list.clone();
                let storage = list.snapshot();
                let mut index = 0;
                Box::new(std::iter::from_fn(move || {
                    assert!(
                        Rc::ptr_eq(&storage, &list.snapshot()),
                        "source list changed during enumeration"
                    );
                    if index == storage.len() {
                        None
                    } else {
                        let v = storage[index].clone();
                        index += 1;
                        Some(v)
                    }
                }))
            }
        }
    }
    pub(crate) fn subscribe(&self, handler: Rc<dyn Fn()>) -> Option<Rc<dyn IDisposable>> {
        match self {
            Self::Native(list) => {
                let token = list.add_collection_changed(Rc::new(move |_| handler()));
                let list = list.clone();
                Some(Disposable::create(move || {
                    list.remove_collection_changed(token);
                }))
            }
            Self::General(list) => {
                let notifications = list.as_notify_collection_changed()?;
                let token = notifications
                    .collection_changed()
                    .add(Rc::new(move |_| handler()));
                let list = list.clone();
                Some(Disposable::create(move || {
                    if let Some(n) = list.as_notify_collection_changed() {
                        n.collection_changed().remove(token);
                    }
                }))
            }
        }
    }
}

ferroui_base::ferro_markup_type!(class OverlayControlList {
    this:OverlayControlList,
    handles:[OverlayControlList,Option<OverlayControlList>],
    methods:[fn Add(Option<Ref<Control>>)=>|this:&OverlayControlList,value|this.add(value)]
});

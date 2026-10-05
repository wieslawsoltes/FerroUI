use crate::{DefinitionBase, Grid};
use ferroui_base::collections::{FerroList, NotifyCollectionChangedEventArgs, ResetBehavior};
use ferroui_base::{ObjectType, Ref, Upcast, WeakRef};
use std::cell::{Cell, RefCell};
use std::ops::Deref;
use std::rc::Rc;

/// The state shared between a [`DefinitionList`] and its collection changed
/// handler.
struct DefinitionListData<T: ObjectType + Upcast<DefinitionBase>> {
    list: FerroList<Ref<T>>,
    is_dirty: Cell<bool>,
    parent: RefCell<Option<WeakRef<Grid>>>,
}

/// Base class of the collections of column and row definitions.
///
/// When parsed from a string, the items are separated by "," or " ".
pub struct DefinitionList<T: ObjectType + Upcast<DefinitionBase>> {
    data: Rc<DefinitionListData<T>>,
}

impl<T: ObjectType + Upcast<DefinitionBase>> Clone for DefinitionList<T> {
    fn clone(&self) -> Self {
        Self { data: self.data.clone() }
    }
}

/// Handles compare by identity.
impl<T: ObjectType + Upcast<DefinitionBase>> PartialEq for DefinitionList<T> {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.data, &other.data)
    }
}

impl<T: ObjectType + Upcast<DefinitionBase>> Default for DefinitionList<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: ObjectType + Upcast<DefinitionBase>> DefinitionList<T> {
    /// Creates an empty collection.
    pub fn new() -> Self {
        let data = Rc::new(DefinitionListData {
            list: FerroList::new(),
            is_dirty: Cell::new(true),
            parent: RefCell::new(None),
        });
        data.list.set_reset_behavior(ResetBehavior::Remove);
        let weak = Rc::downgrade(&data);
        data.list
            .add_collection_changed(Rc::new(move |e: &NotifyCollectionChangedEventArgs<'_, Ref<T>>| {
                if let Some(data) = weak.upgrade() {
                    data.on_collection_changed(e);
                }
            }));
        Self { data }
    }

    /// Whether the collection changed since the grid last validated its
    /// structure.
    #[inline]
    pub(crate) fn is_dirty(&self) -> bool {
        self.data.is_dirty.get()
    }

    #[inline]
    pub(crate) fn set_is_dirty(&self, value: bool) {
        self.data.is_dirty.set(value)
    }

    /// The grid that owns the collection.
    #[allow(dead_code)]
    pub(crate) fn parent(&self) -> Option<Ref<Grid>> {
        self.data.parent()
    }

    pub(crate) fn set_parent(&self, value: Option<&Ref<Grid>>) {
        self.data.set_parent(value)
    }
}

impl<T: ObjectType + Upcast<DefinitionBase>> Deref for DefinitionList<T> {
    type Target = FerroList<Ref<T>>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.data.list
    }
}

impl<T: ObjectType + Upcast<DefinitionBase>> DefinitionListData<T> {
    fn parent(&self) -> Option<Ref<Grid>> {
        self.parent.borrow().as_ref().and_then(WeakRef::upgrade)
    }

    fn set_parent(&self, value: Option<&Ref<Grid>>) {
        if self.parent().as_ref() == value {
            return;
        }

        *self.parent.borrow_mut() = value.map(Ref::downgrade);

        // Definitions already present when the grid claims the collection
        // never pass through the collection changed handler, so they have to
        // change trees here.
        for (idx, definition) in self.list.snapshot().iter().enumerate() {
            let definition: &DefinitionBase = (**definition).upcast();
            Self::set_definition_parent(definition, value);
            definition.set_index(idx as i32);
        }
    }

    /// Moves a definition from its current parent tree to `parent`. Every
    /// route that changes a definition's owner goes through here.
    ///
    /// Ownership is more than the parent pointer: entering a tree also
    /// establishes the property inheritance link a definition needs to see
    /// its shared size scope, and leaving one releases that link and its
    /// shared size registration.
    fn set_definition_parent(definition: &DefinitionBase, parent: Option<&Ref<Grid>>) {
        let current = definition.parent();
        if current.as_ref() == parent {
            return;
        }

        if current.is_some() {
            definition.on_exit_parent_tree();
        }

        definition.set_parent(parent);

        if parent.is_some() {
            definition.on_enter_parent_tree();
        }
    }

    fn on_collection_changed(&self, e: &NotifyCollectionChangedEventArgs<'_, Ref<T>>) {
        for (idx, definition) in self.list.snapshot().iter().enumerate() {
            let definition: &DefinitionBase = (**definition).upcast();
            definition.set_index(idx as i32);
        }

        self.update_definition_parent(e.new_items, false);
        self.update_definition_parent(e.old_items, true);

        self.is_dirty.set(true);
    }

    fn update_definition_parent(&self, items: &[Ref<T>], was_removed: bool) {
        if items.is_empty() {
            return;
        }

        let parent = if was_removed { None } else { self.parent() };

        for item in items {
            Self::set_definition_parent((**item).upcast(), parent.as_ref());
        }
    }
}

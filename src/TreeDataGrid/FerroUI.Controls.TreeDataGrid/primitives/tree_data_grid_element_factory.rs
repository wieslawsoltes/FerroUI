//! `Primitives/TreeDataGridElementFactory.cs`: creates and recycles the rows, cells and column
//! headers of the presenters.
//!
//! The data an element is created for is the reference's `object`: a cell model ([`CellRef`]), a
//! view column ([`ColumnRef`]), a row of the core ([`RowRef`]) or, for a factory that overrides the
//! creation, any other value; it is passed as `&dyn Any` holding one of these handles. The
//! overridable members of the reference (`CreateElement`, `GetDataRecycleKey`,
//! `GetElementRecycleKey`) are the members of [`TreeDataGridElementFactoryImpl`], which a factory is
//! created with.

use super::{
    TreeDataGridCellsPresenter, TreeDataGridCheckBoxCell, TreeDataGridColumnHeader, TreeDataGridColumnHeadersPresenter,
    TreeDataGridExpanderCell, TreeDataGridRow, TreeDataGridRowsPresenter, TreeDataGridTemplateCell, TreeDataGridTextCell,
};
use crate::models::{CellRef, CheckBoxCell, ColumnRef, TemplateCell};
use ferroui_base::{Ref, StyledElement};
use ferroui_controls::{Control, Panel};
use ferroui_treedatagrid_core::models::RowRef;
use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// The overridable members of [`TreeDataGridElementFactory`] (the reference's protected virtual
/// members). Every member defaults to the factory's own behaviour.
pub trait TreeDataGridElementFactoryImpl {
    /// Creates a new element for `data` (`CreateElement`).
    fn create_element(&self, factory: &TreeDataGridElementFactory, data: &dyn Any) -> Ref<Control> {
        factory.default_create_element(data)
    }

    /// The recycle key of the element that shows `data` (`GetDataRecycleKey`).
    fn get_data_recycle_key(&self, factory: &TreeDataGridElementFactory, data: &dyn Any) -> String {
        factory.default_data_recycle_key(data)
    }

    /// The recycle key of `element` (`GetElementRecycleKey`).
    fn get_element_recycle_key(&self, factory: &TreeDataGridElementFactory, element: &Control) -> String {
        factory.default_element_recycle_key(element)
    }
}

impl<T: TreeDataGridElementFactoryImpl + ?Sized> TreeDataGridElementFactoryImpl for Rc<T> {
    fn create_element(&self, factory: &TreeDataGridElementFactory, data: &dyn Any) -> Ref<Control> {
        (**self).create_element(factory, data)
    }

    fn get_data_recycle_key(&self, factory: &TreeDataGridElementFactory, data: &dyn Any) -> String {
        (**self).get_data_recycle_key(factory, data)
    }

    fn get_element_recycle_key(&self, factory: &TreeDataGridElementFactory, element: &Control) -> String {
        (**self).get_element_recycle_key(factory, element)
    }
}

/// The behaviour of a factory that overrides nothing.
struct DefaultBehavior;

impl TreeDataGridElementFactoryImpl for DefaultBehavior {}

/// Creates the elements of the presenters and keeps the recycled ones, by recycle key and by the
/// parent they were recycled from.
pub struct TreeDataGridElementFactory {
    recycle_pools: RefCell<HashMap<String, RecyclePool>>,
    behavior: Box<dyn TreeDataGridElementFactoryImpl>,
}

impl PartialEq for TreeDataGridElementFactory {
    /// Factories compare by identity, so that they can be held in property values.
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl Default for TreeDataGridElementFactory {
    fn default() -> Self {
        Self::construct(Box::new(DefaultBehavior))
    }
}

impl TreeDataGridElementFactory {
    fn construct(behavior: Box<dyn TreeDataGridElementFactoryImpl>) -> Self {
        Self { recycle_pools: RefCell::new(HashMap::new()), behavior }
    }

    /// A factory with the default behaviour.
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    /// A factory whose overridable members are those of `behavior` (a subclass of the reference's
    /// factory).
    pub fn with_impl(behavior: impl TreeDataGridElementFactoryImpl + 'static) -> Rc<Self> {
        Rc::new(Self::construct(Box::new(behavior)))
    }

    /// Gets a recycled element for `data`, preferring one recycled from `parent`, or creates one.
    pub fn get_or_create_element(&self, data: &dyn Any, parent: &Control) -> Ref<Control> {
        let recycle_key = self.get_data_recycle_key(data);
        let parent_ref: Ref<StyledElement> = parent.to_ref().upcast();

        // Take the entry while the pools are borrowed; the reparenting below runs code outside the
        // factory.
        let taken = self.recycle_pools.borrow_mut().get_mut(&recycle_key).and_then(|pool| pool.try_take(&parent_ref));

        if let Some((element, old_panel)) = taken {
            if let Some(old_panel) = old_panel {
                old_panel.children().remove(&element);
            }

            debug_assert!(element.parent().is_none_or(|p| p.ptr_eq(&parent_ref)));

            if parent.is_measure_valid() && !Self::is_parent_layouting(parent) {
                parent.invalidate_measure();
            }
            return element;
        }

        // Otherwise create a new element.
        self.create_element(data)
    }

    /// Returns `element` to the pool of its recycle key.
    pub fn recycle_element(&self, element: &Ref<Control>) {
        let recycle_key = self.get_element_recycle_key(element);
        let parent = element.parent();
        let is_panel = parent.as_ref().is_some_and(|parent| parent.is::<Panel>());

        self.recycle_pools.borrow_mut().entry(recycle_key).or_default().add(element.clone(), parent, is_panel);
    }

    /// Determines whether an existing element can be reused for the supplied data without returning
    /// it to this factory's recycle pool.
    ///
    /// This is useful for controls which retain a realized child in the visual tree while changing
    /// its model, avoiding a detach and subsequent style reapplication.
    pub fn can_reuse_element(&self, element: &Control, data: &dyn Any) -> bool {
        self.get_element_recycle_key(element) == self.get_data_recycle_key(data)
    }

    /// Creates a new element for `data` through the factory's behaviour (`CreateElement`).
    pub fn create_element(&self, data: &dyn Any) -> Ref<Control> {
        self.behavior.create_element(self, data)
    }

    /// The recycle key of the element that shows `data` (`GetDataRecycleKey`).
    pub fn get_data_recycle_key(&self, data: &dyn Any) -> String {
        self.behavior.get_data_recycle_key(self, data)
    }

    /// The recycle key of `element` (`GetElementRecycleKey`).
    pub fn get_element_recycle_key(&self, element: &Control) -> String {
        self.behavior.get_element_recycle_key(self, element)
    }

    /// The factory's own `CreateElement`: the control for a check box cell, a template cell, an
    /// expander cell, any other cell, a column header or a row.
    pub fn default_create_element(&self, data: &dyn Any) -> Ref<Control> {
        if let Some(cell) = data.downcast_ref::<CellRef>() {
            if cell.as_any().is::<CheckBoxCell>() {
                return TreeDataGridCheckBoxCell::new().upcast();
            }
            if cell.as_any().is::<TemplateCell>() {
                return TreeDataGridTemplateCell::new().upcast();
            }
            if cell.as_expander_cell().is_some() {
                return TreeDataGridExpanderCell::new().upcast();
            }
            return TreeDataGridTextCell::new().upcast();
        }

        if data.is::<ColumnRef>() {
            return TreeDataGridColumnHeader::new().upcast();
        }

        if data.is::<RowRef>() {
            return TreeDataGridRow::new().upcast();
        }

        panic!("Specified method is not supported.");
    }

    /// The factory's own `GetDataRecycleKey`: the full name of the type of the element
    /// [`default_create_element`](Self::default_create_element) creates for `data`.
    pub fn default_data_recycle_key(&self, data: &dyn Any) -> String {
        if let Some(cell) = data.downcast_ref::<CellRef>() {
            if cell.as_any().is::<CheckBoxCell>() {
                return TreeDataGridCheckBoxCell::TYPE.full_name();
            }
            if cell.as_any().is::<TemplateCell>() {
                return TreeDataGridTemplateCell::TYPE.full_name();
            }
            if cell.as_expander_cell().is_some() {
                return TreeDataGridExpanderCell::TYPE.full_name();
            }
            return TreeDataGridTextCell::TYPE.full_name();
        }

        if data.is::<ColumnRef>() {
            return TreeDataGridColumnHeader::TYPE.full_name();
        }

        if data.is::<RowRef>() {
            return TreeDataGridRow::TYPE.full_name();
        }

        panic!("Specified method is not supported.");
    }

    /// The factory's own `GetElementRecycleKey`: the full name of the element's type.
    pub fn default_element_recycle_key(&self, element: &Control) -> String {
        element.get_type().full_name()
    }

    fn is_parent_layouting(parent: &Control) -> bool {
        if let Some(rows) = parent.downcast_ref::<TreeDataGridRowsPresenter>() {
            return rows.is_layout_in_progress();
        }
        if let Some(cells) = parent.downcast_ref::<TreeDataGridCellsPresenter>() {
            return cells.is_layout_in_progress();
        }
        if let Some(headers) = parent.downcast_ref::<TreeDataGridColumnHeadersPresenter>() {
            return headers.is_layout_in_progress();
        }
        false
    }
}

/// An element in a pool: listed in the bucket of its parent and, when it can be reparented, in the
/// fallback list. Both lists are intrusive, so that taking an element out of either is O(1).
struct Entry {
    element: Ref<Control>,
    parent: Option<Ref<StyledElement>>,
    parent_previous: Option<usize>,
    parent_next: Option<usize>,
    is_fallback: bool,
    fallback_previous: Option<usize>,
    fallback_next: Option<usize>,
}

/// The elements of a pool recycled from one parent, oldest first.
#[derive(Default)]
struct ParentBucket {
    first: Option<usize>,
    last: Option<usize>,
}

/// The elements of one recycle key.
///
/// Each pooled element is indexed both by its current parent and, when it can be reparented, by the
/// fallback list. An element leaves the pool entirely when it is taken: the pool does not retain
/// controls after they are checked out.
#[derive(Default)]
struct RecyclePool {
    entries: Vec<Option<Entry>>,
    free: Vec<usize>,
    by_parent: HashMap<Ref<StyledElement>, ParentBucket>,
    fallback_first: Option<usize>,
    fallback_last: Option<usize>,
}

impl RecyclePool {
    fn entry(&mut self, id: usize) -> &mut Entry {
        self.entries[id].as_mut().expect("a pooled entry")
    }

    fn add(&mut self, element: Ref<Control>, parent: Option<Ref<StyledElement>>, parent_is_panel: bool) {
        let entry = Entry {
            element,
            parent: parent.clone(),
            parent_previous: None,
            parent_next: None,
            is_fallback: false,
            fallback_previous: None,
            fallback_next: None,
        };

        let id = match self.free.pop() {
            Some(id) => {
                self.entries[id] = Some(entry);
                id
            }
            None => {
                self.entries.push(Some(entry));
                self.entries.len() - 1
            }
        };

        if let Some(parent) = &parent {
            let last = self.by_parent.entry(parent.clone()).or_default().last;
            self.entry(id).parent_previous = last;

            match last {
                Some(last) => self.entry(last).parent_next = Some(id),
                None => {
                    if let Some(bucket) = self.by_parent.get_mut(parent) {
                        bucket.first = Some(id);
                    }
                }
            }

            if let Some(bucket) = self.by_parent.get_mut(parent) {
                bucket.last = Some(id);
            }
        }

        if parent.is_none() || parent_is_panel {
            self.add_fallback(id);
        }
    }

    /// Takes an element, preferring one recycled from `parent`. Returns the element and, when it
    /// was recycled from another panel, that panel, which the caller removes it from.
    fn try_take(&mut self, parent: &Ref<StyledElement>) -> Option<(Ref<Control>, Option<Ref<Panel>>)> {
        let id = self.by_parent.get(parent).and_then(|bucket| bucket.first).or(self.fallback_first)?;

        self.remove(id);
        let entry = self.entries[id].take().expect("a pooled entry");
        self.free.push(id);

        let old_panel = entry
            .element
            .parent()
            .filter(|old_parent| !old_parent.ptr_eq(parent))
            .and_then(|old_parent| old_parent.cast::<Panel>());

        Some((entry.element, old_panel))
    }

    fn remove(&mut self, id: usize) {
        let (parent, previous, next) = {
            let entry = self.entry(id);
            (entry.parent.clone(), entry.parent_previous.take(), entry.parent_next.take())
        };

        if let Some(parent) = parent {
            match previous {
                Some(previous) => self.entry(previous).parent_next = next,
                None => {
                    if let Some(bucket) = self.by_parent.get_mut(&parent) {
                        bucket.first = next;
                    }
                }
            }

            match next {
                Some(next) => self.entry(next).parent_previous = previous,
                None => {
                    if let Some(bucket) = self.by_parent.get_mut(&parent) {
                        bucket.last = previous;
                    }
                }
            }

            if self.by_parent.get(&parent).is_some_and(|bucket| bucket.first.is_none()) {
                self.by_parent.remove(&parent);
            }
        }

        if self.entry(id).is_fallback {
            self.remove_fallback(id);
        }
    }

    fn add_fallback(&mut self, id: usize) {
        let last = self.fallback_last;
        {
            let entry = self.entry(id);
            entry.is_fallback = true;
            entry.fallback_previous = last;
        }

        match last {
            Some(last) => self.entry(last).fallback_next = Some(id),
            None => self.fallback_first = Some(id),
        }

        self.fallback_last = Some(id);
    }

    fn remove_fallback(&mut self, id: usize) {
        let (previous, next) = {
            let entry = self.entry(id);
            entry.is_fallback = false;
            (entry.fallback_previous.take(), entry.fallback_next.take())
        };

        match previous {
            Some(previous) => self.entry(previous).fallback_next = next,
            None => self.fallback_first = next,
        }

        match next {
            Some(next) => self.entry(next).fallback_previous = previous,
            None => self.fallback_last = previous,
        }
    }
}

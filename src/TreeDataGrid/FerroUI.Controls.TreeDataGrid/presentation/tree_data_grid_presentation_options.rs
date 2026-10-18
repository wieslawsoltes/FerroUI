//! `Presentation/TreeDataGridPresentationOptions.cs`: the factories of the view columns of the
//! neutral columns of a source, owned by the view. The core's columns contain only presentation
//! keys.

use super::{PresentationRef, TreeDataGridSourcePresentation};
use crate::models::ColumnRef;
use ferroui_treedatagrid_core::models::IColumn as CoreColumn;
use ferroui_treedatagrid_core::ITreeDataGridSource;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// What creates the presentation of a source (`ITreeDataGridPresentationOptions`).
pub trait ITreeDataGridPresentationOptions {
    /// The presentation of `model`.
    fn create(&self, model: &Rc<dyn ITreeDataGridSource>) -> PresentationRef;
}

impl PartialEq for dyn ITreeDataGridPresentationOptions {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}

/// The factory of the view column of a core column.
pub type ColumnPresentationFactory = Rc<dyn Fn(&Rc<dyn CoreColumn>) -> ColumnRef>;

/// The factories of the view columns of the presentation keys of a source's columns
/// (`TreeDataGridPresentationOptions<TModel>`); keys compare ordinally.
pub struct TreeDataGridPresentationOptions {
    weak_self: std::rc::Weak<Self>,
    columns: RefCell<HashMap<String, ColumnPresentationFactory>>,
}

impl TreeDataGridPresentationOptions {
    pub fn new() -> Rc<Self> {
        Rc::new_cyclic(|weak_self| Self { weak_self: weak_self.clone(), columns: RefCell::new(HashMap::new()) })
    }

    /// The shared options.
    pub fn this(&self) -> Rc<Self> {
        self.weak_self.upgrade().expect("presentation options that are alive")
    }

    /// Registers the factory of the columns whose presentation key is `key` (`Columns[key] = factory`).
    pub fn set_column(&self, key: impl Into<String>, factory: impl Fn(&Rc<dyn CoreColumn>) -> ColumnRef + 'static) {
        self.columns.borrow_mut().insert(key.into(), Rc::new(factory));
    }

    /// Adds a factory and returns the options (a collection initializer of the reference).
    pub fn with_column(self: Rc<Self>, key: impl Into<String>, factory: impl Fn(&Rc<dyn CoreColumn>) -> ColumnRef + 'static) -> Rc<Self> {
        self.set_column(key, factory);
        self
    }

    /// Removes the factory of `key`; false when there is none.
    pub fn remove_column(&self, key: &str) -> bool {
        self.columns.borrow_mut().remove(key).is_some()
    }

    /// The factory of `key` (`Columns.TryGetValue`).
    pub fn column(&self, key: &str) -> Option<ColumnPresentationFactory> {
        self.columns.borrow().get(key).cloned()
    }

    /// The keys that have a factory.
    pub fn keys(&self) -> Vec<String> {
        self.columns.borrow().keys().cloned().collect()
    }
}

impl PartialEq for TreeDataGridPresentationOptions {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl ITreeDataGridPresentationOptions for TreeDataGridPresentationOptions {
    fn create(&self, model: &Rc<dyn ITreeDataGridSource>) -> PresentationRef {
        TreeDataGridSourcePresentation::new(model.clone(), Some(self.this())) as PresentationRef
    }
}

//! Port of `ViewModels/DragNodeViewModel.cs`.

use super::items::{items_source, ItemList};
use super::ViewModelBase;
use ferroui_base::ferro_markup_type;
use ferroui_controls::ItemsSource;
use ferroui_reactiveui::{reactive_object, IReactiveObject};
use std::cell::RefCell;
use std::fmt;
use std::rc::{Rc, Weak};

/// A node of the drag and drop trees: a title, its parent and its children.
#[derive(Default)]
pub struct DragNodeViewModel {
    base: ViewModelBase,
    /// The parent, held weakly (the parent holds its children).
    parent: RefCell<Option<Weak<DragNodeViewModel>>>,
    title: RefCell<Option<String>>,
    nodes: RefCell<Option<ItemList>>,
}
reactive_object!(DragNodeViewModel, base);
super::reactive_accessors!(DragNodeViewModel {
    title, set_title, "Title": Option<String>;
    nodes, set_nodes, "Nodes": Option<ItemList>;
});

impl PartialEq for DragNodeViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl DragNodeViewModel {
    /// `new DragNodeViewModel { Title = title }`.
    pub fn new(title: &str) -> Rc<Self> {
        let node = Rc::new(Self::default());
        node.set_title(Some(title.to_string()));
        node
    }

    /// `Parent`.
    pub fn parent(&self) -> Option<Rc<DragNodeViewModel>> {
        self.parent.borrow().as_ref().and_then(Weak::upgrade)
    }

    /// Sets `Parent`.
    ///
    /// Deviation (DEVIATIONS.md, Behaviors gallery): the parent is held weakly, so that a node
    /// and its children do not keep each other alive.
    pub fn set_parent(&self, value: Option<&Rc<DragNodeViewModel>>) {
        let current = self.parent();
        if current.as_ref().map(Rc::as_ptr) == value.map(Rc::as_ptr) {
            return;
        }
        *self.parent.borrow_mut() = value.map(Rc::downgrade);
        self.raise_property_changed("Parent");
    }

    /// `IsDescendantOf(DragNodeViewModel)`.
    pub fn is_descendant_of(&self, possible_ancestor: &Rc<DragNodeViewModel>) -> bool {
        let mut current = self.parent();
        while let Some(node) = current {
            if Rc::ptr_eq(&node, possible_ancestor) {
                return true;
            }
            current = node.parent();
        }
        false
    }
}

impl fmt::Display for DragNodeViewModel {
    /// `ToString()`: the title.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.title.borrow().as_deref().unwrap_or_default())
    }
}

ferro_markup_type!(class DragNodeViewModel {
    this: Rc<DragNodeViewModel>,
    handles: [DragNodeViewModel, Rc<DragNodeViewModel>, Option<Rc<DragNodeViewModel>>],
    constructors: [() => || Rc::new(DragNodeViewModel::default())],
    properties: [
        Title: Option<String> {
            get: |this: &Rc<DragNodeViewModel>| this.title(),
            set: |this: &Rc<DragNodeViewModel>, value: Option<String>| this.set_title(value)
        },
        Parent: Option<Rc<DragNodeViewModel>> {
            get: |this: &Rc<DragNodeViewModel>| this.parent(),
            set: |this: &Rc<DragNodeViewModel>, value: Option<Rc<DragNodeViewModel>>| this.set_parent(value.as_ref())
        },
        Nodes: Option<ItemsSource> { get: |this: &Rc<DragNodeViewModel>| items_source(&this.nodes()) },
    ],
    notify_property_changed: DragNodeViewModel,
});

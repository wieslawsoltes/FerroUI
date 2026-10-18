//! The helpers of the ported control tests: the helpers the primitives tests share
//! (`TestWindow.cs`, `TestTemplates.cs`, `TestElementFactory.cs`, the layout test columns and
//! cells, re-exported from `crate::primitives::tests::support`), and `CollectionExtensions.cs`,
//! the debug list of `Collections/`, `Collections/ResettingCollection.cs` and `NotifyingBase.cs`
//! of the reference's tests.

#![allow(dead_code)]

pub use crate::primitives::tests::support::*;

use crate::presentation::TreeDataGridSourcePresentation;
use crate::{TreeDataGrid, TreeDataGridSourceRef};
use ferroui_base::collections::{FerroList, NotifyCollectionChangedAction, NotifyCollectionChangedEventArgs};
use ferroui_base::data::model::Event;
use ferroui_base::utilities::HandlerList;
use ferroui_treedatagrid_core::accessor::TreeModel;
use ferroui_treedatagrid_core::model_collection::{ModelCollection, ModelsChangedHandler, MutableModelCollection};
use ferroui_treedatagrid_core::{ITreeDataGridSourceOf, Models};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// `NotifyingBase` of the tests: the `PropertyChanged` of a test model, which counts its
/// subscribers (`PropertyChangedSubscriberCount`). A test model holds one and implements
/// `INotifyPropertyChanged` by returning [`TestNotifier::event`], and registers itself with
/// `ferro_model!(..., |b| b.notify_property_changed() ...)` so that the bindings and cells find it.
#[derive(Default)]
pub struct TestNotifier {
    event: Event<str>,
}

impl TestNotifier {
    pub fn new() -> Self {
        Self::default()
    }

    /// The event.
    pub fn event(&self) -> &Event<str> {
        &self.event
    }

    /// Raises the change of `property_name` (`RaisePropertyChanged`).
    pub fn raise(&self, property_name: &str) {
        self.event.raise(property_name);
    }

    /// Sets `field` and raises the change when the value changed (`RaiseAndSetIfChanged`).
    pub fn raise_and_set_if_changed<T: PartialEq>(&self, field: &RefCell<T>, value: T, property_name: &str) -> bool {
        if *field.borrow() != value {
            *field.borrow_mut() = value;
            self.raise(property_name);
            true
        } else {
            false
        }
    }

    /// [`raise_and_set_if_changed`](Self::raise_and_set_if_changed) for a `Cell`.
    pub fn raise_and_set_cell_if_changed<T: PartialEq + Copy>(&self, field: &Cell<T>, value: T, property_name: &str) -> bool {
        if field.get() != value {
            field.set(value);
            self.raise(property_name);
            true
        } else {
            false
        }
    }

    /// The number of subscribers (`PropertyChangedSubscriberCount`).
    pub fn property_changed_subscriber_count(&self) -> usize {
        self.event.handler_count()
    }
}

/// The debug list of the reference's tests: a notifying list of models that counts the subscribers to its changes
/// (`CollectionChangedSubscriberCount`).
pub struct ListDebug<M: TreeModel> {
    list: FerroList<Rc<M>>,
    subscribers: Cell<usize>,
}

impl<M: TreeModel> ListDebug<M> {
    pub fn new() -> Rc<Self> {
        Rc::new(Self { list: FerroList::new(), subscribers: Cell::new(0) })
    }

    pub fn from_items(items: impl IntoIterator<Item = Rc<M>>) -> Rc<Self> {
        let this = Self::new();
        for item in items {
            this.list.add(item);
        }
        this
    }

    /// The list.
    pub fn list(&self) -> &FerroList<Rc<M>> {
        &self.list
    }

    /// `CollectionChangedSubscriberCount`.
    pub fn collection_changed_subscriber_count(&self) -> usize {
        self.subscribers.get()
    }

    /// The list as the models of a source.
    pub fn models(self: &Rc<Self>) -> Models<M> {
        self.clone() as Models<M>
    }
}

impl<M: TreeModel> ModelCollection<M> for ListDebug<M> {
    fn count(&self) -> usize {
        self.list.count()
    }

    fn get(&self, index: usize) -> Rc<M> {
        self.list.get(index)
    }

    fn add_collection_changed(&self, handler: Rc<ModelsChangedHandler<M>>) -> Option<u64> {
        self.subscribers.set(self.subscribers.get() + 1);
        Some(self.list.add_collection_changed(handler))
    }

    fn remove_collection_changed(&self, token: u64) {
        if self.list.remove_collection_changed(token) {
            self.subscribers.set(self.subscribers.get().saturating_sub(1));
        }
    }

    fn is_notifying(&self) -> bool {
        true
    }

    fn as_mutable(&self) -> Option<&dyn MutableModelCollection<M>> {
        Some(self)
    }
}

impl<M: TreeModel> MutableModelCollection<M> for ListDebug<M> {
    fn insert(&self, index: usize, model: Rc<M>) {
        self.list.insert(index, model);
    }

    fn remove_at(&self, index: usize) {
        self.list.remove_at(index);
    }

    fn set(&self, index: usize, model: Rc<M>) {
        self.list.set(index, model);
    }
}

/// `ResettingCollection<T>`: a list that notifies removals one by one and replacements of all its
/// items as a reset.
pub struct ResettingCollection<M: TreeModel> {
    items: RefCell<Vec<Rc<M>>>,
    handlers: HandlerList<ModelsChangedHandler<M>>,
}

impl<M: TreeModel> ResettingCollection<M> {
    pub fn new(items: impl IntoIterator<Item = Rc<M>>) -> Rc<Self> {
        Rc::new(Self { items: RefCell::new(items.into_iter().collect()), handlers: HandlerList::new() })
    }

    fn raise(&self, args: &NotifyCollectionChangedEventArgs<'_, Rc<M>>) {
        for (_, handler) in self.handlers.snapshot().iter() {
            handler(args);
        }
    }

    /// Removes the item at `index` and notifies the removal (`RemoveAt`).
    pub fn remove_at(&self, index: usize) {
        let item = self.items.borrow_mut().remove(index);
        let removed = [item];
        let args = NotifyCollectionChangedEventArgs {
            action: NotifyCollectionChangedAction::Remove,
            new_items: &[],
            old_items: &removed,
            new_starting_index: -1,
            old_starting_index: index as i32,
        };
        self.raise(&args);
    }

    /// Replaces the items and notifies a reset (`Reset`).
    pub fn reset(&self, items: impl IntoIterator<Item = Rc<M>>) {
        *self.items.borrow_mut() = items.into_iter().collect();
        let args = NotifyCollectionChangedEventArgs {
            action: NotifyCollectionChangedAction::Reset,
            new_items: &[],
            old_items: &[],
            new_starting_index: -1,
            old_starting_index: -1,
        };
        self.raise(&args);
    }

    /// The collection as the models of a source.
    pub fn models(self: &Rc<Self>) -> Models<M> {
        self.clone() as Models<M>
    }
}

impl<M: TreeModel> ModelCollection<M> for ResettingCollection<M> {
    fn count(&self) -> usize {
        self.items.borrow().len()
    }

    fn get(&self, index: usize) -> Rc<M> {
        self.items.borrow()[index].clone()
    }

    fn add_collection_changed(&self, handler: Rc<ModelsChangedHandler<M>>) -> Option<u64> {
        Some(self.handlers.add(handler))
    }

    fn remove_collection_changed(&self, token: u64) {
        self.handlers.remove(token);
    }

    fn is_notifying(&self) -> bool {
        true
    }
}

/// A notifying list of `items` as the models of a source (`new ObservableCollection<T>(items)`).
pub fn observable<M: TreeModel>(items: impl IntoIterator<Item = Rc<M>>) -> Rc<FerroList<Rc<M>>> {
    let list = FerroList::new();
    for item in items {
        list.add(item);
    }
    Rc::new(list)
}

/// The handle of a typed source for the grid's `Source` or `Model` (`target.Source = source`).
pub fn source_ref<M: TreeModel, S: ITreeDataGridSourceOf<M> + 'static>(source: &Rc<S>) -> Option<TreeDataGridSourceRef> {
    Some(TreeDataGridSourceRef::of::<M, S>(source.clone()))
}

/// The presentation of a typed source (`new TreeDataGridPresentation<TModel>(source)`).
pub fn presentation_of<M: TreeModel, S: ITreeDataGridSourceOf<M> + 'static>(source: &Rc<S>) -> Rc<TreeDataGridSourcePresentation> {
    TreeDataGridSourcePresentation::of::<M, S>(source.clone(), None)
}

/// A grid with the test template (`new TreeDataGrid { Template = TestTemplates.TreeDataGridTemplate() }`).
pub fn tree_data_grid_with_template() -> ferroui_base::Ref<TreeDataGrid> {
    let target = TreeDataGrid::new();
    target.set_template(Some(tree_data_grid_template(0.0)));
    target
}

/// `TestTemplates.TreeDataGridRowStyle`: rows with the test template.
pub fn tree_data_grid_row_style() -> ferroui_base::Ref<ferroui_base::styling::Style> {
    use ferroui_base::styling::{Selectors, Setter, Style};
    Style::with_setters(
        Selectors::of_type::<crate::primitives::TreeDataGridRow>(),
        [Setter::new(ferroui_controls::primitives::TemplatedControl::template_property(), Some(tree_data_grid_row_template()))],
    )
}

/// `TestTemplates.TreeDataGridExpanderCellStyle`: expander cells with the test template.
pub fn tree_data_grid_expander_cell_style() -> ferroui_base::Ref<ferroui_base::styling::Style> {
    use ferroui_base::styling::{Selectors, Setter, Style};
    Style::with_setters(
        Selectors::of_type::<crate::primitives::TreeDataGridExpanderCell>(),
        [Setter::new(
            ferroui_controls::primitives::TemplatedControl::template_property(),
            Some(tree_data_grid_expander_cell_template()),
        )],
    )
}

/// `TestTemplates.TreeDataGridTemplateCellStyle`: template cells with the test template.
pub fn tree_data_grid_template_cell_style() -> ferroui_base::Ref<ferroui_base::styling::Style> {
    use ferroui_base::styling::{Selectors, Setter, Style};
    Style::with_setters(
        Selectors::of_type::<crate::primitives::TreeDataGridTemplateCell>(),
        [Setter::new(
            ferroui_controls::primitives::TemplatedControl::template_property(),
            Some(tree_data_grid_template_cell_template()),
        )],
    )
}

/// `TestTemplates.ScrollViewerStyle`: scroll viewers with the test template.
pub fn scroll_viewer_style() -> ferroui_base::Ref<ferroui_base::styling::Style> {
    use ferroui_base::styling::{Selectors, Setter, Style};
    Style::with_setters(
        Selectors::of_type::<ferroui_controls::ScrollViewer>(),
        [Setter::new(ferroui_controls::primitives::TemplatedControl::template_property(), Some(scroll_viewer_template()))],
    )
}

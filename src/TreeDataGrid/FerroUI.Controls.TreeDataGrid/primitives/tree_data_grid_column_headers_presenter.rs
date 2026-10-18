//! `Primitives/TreeDataGridColumnHeadersPresenter.cs`: the presenter of the column headers, a
//! horizontal virtualizing presenter of [`TreeDataGridColumnHeader`]s over the columns.

use super::{
    IFinalMeasureSelector, PresenterItemsChange, TreeDataGridColumnHeader, TreeDataGridColumnarPresenterBase,
    TreeDataGridElementFactory, TreeDataGridPresenterBase, TreeDataGridPresenterHooks,
};
use crate::models::{ColumnRef, ColumnsChangedArgs, ColumnsChangedHandler, IColumns};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{LayoutableImpl, Orientation};
use ferroui_base::logical_tree::{ChildIndexChangedEventArgs, IChildIndexProvider, LogicalTreeAttachmentEventArgs};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    ferro_class, ferro_property, instantiate, DirectProperty, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, Rect, Ref, Size, StyledElement, StyledElementImpl, StyledElementImplExt,
    VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs, WeakRef,
};
use ferroui_controls::automation::peers::AutomationPeer;
use ferroui_controls::{Border, Control, ControlImpl};
use std::cell::{Cell, OnceCell, RefCell};
use std::rc::Rc;

type ChildIndexChangedHandlers = HandlerList<dyn Fn(&ChildIndexChangedEventArgs)>;

/// Presents the column headers of a [`TreeDataGrid`](crate::TreeDataGrid), realizing only the
/// headers of the columns in the viewport.
#[repr(C)]
pub struct TreeDataGridColumnHeadersPresenter {
    base: Border,
    presenter: TreeDataGridPresenterBase,
    columnar: TreeDataGridColumnarPresenterBase,
    items: RefCell<Option<Rc<dyn IColumns>>>,
    is_attached_to_visual_tree: Cell<bool>,
    /// The columns whose `LayoutInvalidated` the presenter handles, and the token of the handler.
    layout_invalidated_columns: RefCell<Option<(Rc<dyn IColumns>, u64)>>,
    child_index_changed: Rc<ChildIndexChangedHandlers>,
    child_index_provider: OnceCell<Rc<ColumnHeadersPresenterChildIndexProvider>>,
}

ferro_class!(TreeDataGridColumnHeadersPresenter: Border);
ferroui_base::ferro_class_info!(TreeDataGridColumnHeadersPresenter { new: TreeDataGridColumnHeadersPresenter::new });
ferroui_base::ferro_impl_classes!(TreeDataGridColumnHeadersPresenter: InteractiveImpl, InputElementImpl);

ferroui_base::ferro_properties! { impl TreeDataGridColumnHeadersPresenter {
    ferro_property!(
        /// Defines the `ElementFactory` property.
        pub fn element_factory_property() -> DirectProperty<TreeDataGridColumnHeadersPresenter, Option<Rc<TreeDataGridElementFactory>>> {
            FerroProperty::register_direct::<TreeDataGridColumnHeadersPresenter, _>(
                "ElementFactory",
                |o| o.element_factory(),
                Some(|o, v| o.set_element_factory(v)),
                None,
            )
        }
    );

    ferro_property!(
        /// Defines the `Items` property: the columns.
        pub fn items_property() -> DirectProperty<TreeDataGridColumnHeadersPresenter, Option<Rc<dyn IColumns>>> {
            FerroProperty::register_direct::<TreeDataGridColumnHeadersPresenter, _>(
                "Items",
                |o| o.items(),
                Some(|o, v| o.set_items(v)),
                None,
            )
        }
    );
} }

impl FerroObjectImpl for TreeDataGridColumnHeadersPresenter {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        if change.property() == Self::items_property().as_property() {
            let old_value = change.get_old_value::<Option<Rc<dyn IColumns>>>().flatten();
            let new_value = change.get_new_value::<Option<Rc<dyn IColumns>>>();

            if let Some(old_columns) = &old_value {
                this.unsubscribe_from_column_layout_invalidated(old_columns);
            }
            if let Some(new_columns) = &new_value {
                this.subscribe_to_column_layout_invalidated(new_columns);
            }
        }

        Self::parent_on_property_changed(this, change);
    }
}

impl StyledElementImpl for TreeDataGridColumnHeadersPresenter {
    fn on_detached_from_logical_tree(this: &Self, e: &LogicalTreeAttachmentEventArgs) {
        Self::parent_on_detached_from_logical_tree(this, e);
        this.presenter.on_detached_from_logical_tree(this);
    }

    fn child_index_provider(this: &Self) -> Option<Rc<dyn IChildIndexProvider>> {
        let provider = this
            .child_index_provider
            .get_or_init(|| Rc::new(ColumnHeadersPresenterChildIndexProvider { owner: this.to_ref().downgrade() }))
            .clone();
        Some(provider)
    }
}

impl VisualImpl for TreeDataGridColumnHeadersPresenter {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);
        this.presenter.on_attached_to_visual_tree(this);
        this.is_attached_to_visual_tree.set(true);

        if let Some(columns) = this.items() {
            this.subscribe_to_column_layout_invalidated(&columns);
        }
    }

    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        let columns = this.layout_invalidated_columns.borrow().as_ref().map(|(columns, _)| columns.clone());
        if let Some(columns) = columns {
            this.unsubscribe_from_column_layout_invalidated(&columns);
        }

        this.is_attached_to_visual_tree.set(false);
        Self::parent_on_detached_from_visual_tree(this, e);
        this.presenter.on_detached_from_visual_tree(this);
    }
}

impl LayoutableImpl for TreeDataGridColumnHeadersPresenter {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        this.presenter.measure_override(this, available_size)
    }

    fn arrange_override(this: &Self, final_size: Size) -> Size {
        if let Some(columns) = this.items() {
            columns.commit_actual_widths();
        }
        this.presenter.arrange_override(this, final_size)
    }
}

impl ControlImpl for TreeDataGridColumnHeadersPresenter {
    fn on_create_automation_peer(this: &Self) -> Ref<AutomationPeer> {
        crate::automation::peers::TreeDataGridColumnHeadersPresenterAutomationPeer::new(this).upcast()
    }
}

impl IFinalMeasureSelector for TreeDataGridColumnHeadersPresenter {
    fn needs_final_measure(&self, element: &Ref<Control>, index: i32) -> bool {
        let columns = self.items().expect("the columns of the column headers presenter");
        TreeDataGridColumnarPresenterBase::needs_final_measure(&*columns, element, index)
    }
}

impl TreeDataGridPresenterHooks for TreeDataGridColumnHeadersPresenter {
    type Item = ColumnRef;

    fn presenter_base(&self) -> &TreeDataGridPresenterBase {
        &self.presenter
    }

    fn orientation(&self) -> Orientation {
        Orientation::Horizontal
    }

    fn item_count(&self) -> Option<usize> {
        self.items.borrow().as_ref().map(|items| items.count())
    }

    fn item_at(&self, index: usize) -> ColumnRef {
        self.items().expect("the columns of the column headers presenter").get(index)
    }

    fn subscribe_to_items(&self, handler: Rc<dyn Fn(&PresenterItemsChange)>) -> Option<u64> {
        let items = self.items()?;
        let handler: Rc<ColumnsChangedHandler> =
            Rc::new(move |e: &ColumnsChangedArgs<'_>| handler(&PresenterItemsChange::from_args(e)));
        Some(items.add_collection_changed(handler))
    }

    fn unsubscribe_from_items(&self, token: u64) {
        if let Some(items) = self.items() {
            items.remove_collection_changed(token);
        }
    }

    fn realize_element(&self, element: &Ref<Control>, _column: &ColumnRef, index: i32) {
        let header = element
            .cast::<TreeDataGridColumnHeader>()
            .expect("an element of the column headers presenter is a column header");
        let columns = self.items().expect("the columns of the column headers presenter");
        header.realize(&columns, index);
        self.raise_child_index_changed(element, index);
    }

    fn update_element_index(&self, element: &Ref<Control>, _old_index: i32, new_index: i32) {
        let header = element
            .cast::<TreeDataGridColumnHeader>()
            .expect("an element of the column headers presenter is a column header");
        header.update_column_index(new_index);
        self.raise_child_index_changed(element, new_index);
    }

    fn unrealize_element(&self, element: &Ref<Control>) {
        let header = element
            .cast::<TreeDataGridColumnHeader>()
            .expect("an element of the column headers presenter is a column header");
        header.unrealize();
        self.raise_child_index_changed(element, header.column_index());
    }

    fn measure_element(&self, index: i32, element: &Ref<Control>, available_size: Size) -> Size {
        let columns = self.items().expect("the columns of the column headers presenter");
        TreeDataGridColumnarPresenterBase::measure_column_element(&*columns, index, -1, element, available_size)
    }

    fn get_initial_constraint(&self, _element: &Ref<Control>, index: i32, available_size: Size) -> Size {
        let columns = self.items().expect("the columns of the column headers presenter");
        TreeDataGridColumnarPresenterBase::get_initial_constraint(&*columns, index, available_size)
    }

    fn needs_final_measure_pass(&self, first_index: i32, elements: &[Option<Ref<Control>>]) -> bool {
        let columns = self.items().expect("the columns of the column headers presenter");
        TreeDataGridColumnarPresenterBase::needs_final_measure_pass(&*columns, first_index, elements)
    }

    fn get_final_constraint(&self, _element: &Ref<Control>, index: i32, _available_size: Size) -> Size {
        let columns = self.items().expect("the columns of the column headers presenter");
        TreeDataGridColumnarPresenterBase::get_final_constraint(&*columns, index)
    }

    fn get_element_at(&self, position: f64) -> (i32, f64) {
        let columns = self.items().expect("the columns of the column headers presenter");
        TreeDataGridColumnarPresenterBase::get_element_at(&*columns, position)
    }

    fn calculate_size_u(&self, available_size: Size) -> f64 {
        TreeDataGridColumnarPresenterBase::calculate_size_u(self.items().as_deref(), available_size)
    }

    fn get_or_estimate_anchor_element_for_viewport(
        &self,
        viewport_start: f64,
        viewport_end: f64,
        item_count: i32,
    ) -> (i32, f64) {
        self.columnar.get_or_estimate_anchor_element_for_viewport(
            &self.presenter,
            self.items().as_deref(),
            viewport_start,
            viewport_end,
            item_count,
        )
    }

    fn estimate_element_size_u(&self) -> f64 {
        self.columnar.estimate_element_size_u(self.items().as_deref())
    }

    fn needs_measure_for_viewport_change(&self, measure_viewport: Rect, viewport: Rect) -> bool {
        TreeDataGridColumnarPresenterBase::needs_measure_for_viewport_change(
            &self.presenter,
            Orientation::Horizontal,
            measure_viewport,
            viewport,
        )
    }

    fn as_final_measure_selector(&self) -> Option<&dyn IFinalMeasureSelector> {
        Some(self)
    }
}

impl TreeDataGridColumnHeadersPresenter {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: Border::construct(),
            presenter: TreeDataGridPresenterBase::new(),
            columnar: TreeDataGridColumnarPresenterBase::new(),
            items: RefCell::new(None),
            is_attached_to_visual_tree: Cell::new(false),
            layout_invalidated_columns: RefCell::new(None),
            child_index_changed: Rc::new(HandlerList::new()),
            child_index_provider: OnceCell::new(),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The state of the presenter base.
    pub fn presenter_base(&self) -> &TreeDataGridPresenterBase {
        &self.presenter
    }

    /// The factory of the column headers.
    pub fn element_factory(&self) -> Option<Rc<TreeDataGridElementFactory>> {
        self.presenter.element_factory()
    }

    pub fn set_element_factory(&self, value: Option<Rc<TreeDataGridElementFactory>>) {
        self.set_and_raise(Self::element_factory_property(), self.presenter.element_factory_field(), value);
    }

    /// The columns whose headers the presenter shows.
    pub fn items(&self) -> Option<Rc<dyn IColumns>> {
        self.items.borrow().clone()
    }

    pub fn set_items(&self, value: Option<Rc<dyn IColumns>>) {
        let same = match (&*self.items.borrow(), &value) {
            (Some(a), Some(b)) => Rc::ptr_eq(a, b),
            (None, None) => true,
            _ => false,
        };

        if !same {
            self.presenter.unsubscribe_from_item_changes(self);

            let old_value = self.items.replace(value.clone());

            self.presenter.subscribe_to_item_changes(self);

            self.raise_direct_property_changed(Self::items_property(), &old_value, &value);
            self.presenter.on_items_collection_changed(self, &PresenterItemsChange::reset());
        }
    }

    /// The realized column headers, with an empty slot for each column of the realized range that
    /// has no header yet (`RealizedElements`).
    pub fn realized_elements(&self) -> Vec<Option<Ref<Control>>> {
        self.presenter.realized_elements()
    }

    /// Whether the presenter is in its measure or arrange pass.
    pub fn is_layout_in_progress(&self) -> bool {
        self.presenter.is_layout_in_progress()
    }

    /// Brings the header of the column at `index` into view (`BringIntoView`).
    pub fn bring_into_view(&self, index: i32, rect: Option<Rect>) -> Option<Ref<Control>> {
        self.presenter.bring_into_view(self, index, rect)
    }

    /// The realized column headers (`GetRealizedElements`).
    pub fn get_realized_elements(&self) -> Vec<Ref<Control>> {
        self.presenter.get_realized_elements()
    }

    /// The realized header of the column at `index`, if any (`TryGetElement`).
    pub fn try_get_element(&self, index: i32) -> Option<Ref<Control>> {
        self.presenter.try_get_element(index)
    }

    /// Occurs when the index of a realized header changes (`ChildIndexChanged`). Disposing the
    /// returned handle unsubscribes.
    pub fn child_index_changed(&self, handler: impl Fn(&ChildIndexChangedEventArgs) + 'static) -> Rc<dyn IDisposable> {
        let token = self.child_index_changed.add(Rc::new(handler));
        let handlers = Rc::downgrade(&self.child_index_changed);
        Disposable::create(move || {
            if let Some(handlers) = handlers.upgrade() {
                handlers.remove(token);
            }
        })
    }

    /// The index of a column header (`IChildIndexProvider.GetChildIndex`).
    pub fn get_child_index(&self, child: &StyledElement) -> i32 {
        match child.downcast_ref::<TreeDataGridColumnHeader>() {
            Some(header) => header.column_index(),
            None => -1,
        }
    }

    /// The number of columns (`IChildIndexProvider.TryGetTotalCount`).
    pub fn try_get_total_count(&self) -> Option<i32> {
        self.items().map(|items| items.count() as i32)
    }

    fn raise_child_index_changed(&self, element: &Ref<Control>, index: i32) {
        if !self.child_index_changed.is_empty() {
            let e = ChildIndexChangedEventArgs::new(element.clone().upcast(), index);
            for (_, handler) in self.child_index_changed.snapshot().iter() {
                handler(&e);
            }
        }
    }

    fn subscribe_to_column_layout_invalidated(&self, columns: &Rc<dyn IColumns>) {
        let current = self.layout_invalidated_columns.borrow().as_ref().map(|(columns, _)| columns.clone());

        if !self.is_attached_to_visual_tree.get() || current.as_ref().is_some_and(|current| Rc::ptr_eq(current, columns)) {
            return;
        }

        if let Some(old_columns) = current {
            self.unsubscribe_from_column_layout_invalidated(&old_columns);
        }

        let weak: WeakRef<TreeDataGridColumnHeadersPresenter> = self.to_ref().downgrade();
        let token = columns.add_layout_invalidated(Rc::new(move || {
            if let Some(this) = weak.upgrade() {
                this.invalidate_measure();
            }
        }));
        *self.layout_invalidated_columns.borrow_mut() = Some((columns.clone(), token));
    }

    fn unsubscribe_from_column_layout_invalidated(&self, columns: &Rc<dyn IColumns>) {
        let subscription = {
            let mut current = self.layout_invalidated_columns.borrow_mut();
            match &*current {
                Some((current_columns, _)) if Rc::ptr_eq(current_columns, columns) => current.take(),
                _ => None,
            }
        };

        if let Some((columns, token)) = subscription {
            columns.remove_layout_invalidated(token);
        }
    }
}

/// The child index provider of a column headers presenter: a header's index is its column index.
struct ColumnHeadersPresenterChildIndexProvider {
    owner: WeakRef<TreeDataGridColumnHeadersPresenter>,
}

impl IChildIndexProvider for ColumnHeadersPresenterChildIndexProvider {
    fn get_child_index(&self, child: &StyledElement) -> i32 {
        self.owner.upgrade().map_or(-1, |owner| owner.get_child_index(child))
    }

    fn try_get_total_count(&self) -> Option<i32> {
        self.owner.upgrade()?.try_get_total_count()
    }

    fn child_index_changed(&self, handler: Rc<dyn Fn(&ChildIndexChangedEventArgs)>) -> Rc<dyn IDisposable> {
        let Some(owner) = self.owner.upgrade() else { return Disposable::empty() };
        let token = owner.child_index_changed.add(handler);
        let handlers = Rc::downgrade(&owner.child_index_changed);
        Disposable::create(move || {
            if let Some(handlers) = handlers.upgrade() {
                handlers.remove(token);
            }
        })
    }
}

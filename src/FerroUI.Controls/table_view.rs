use crate::generators::RecycleKey;
use crate::presenters::table_view_layout_helper::TableViewLayoutHelper;
use crate::presenters::TableViewColumnHeadersPresenter;
use crate::primitives::{SelectingItemsControlImpl, TemplatedControlImpl};
use crate::table_view_sort::SortedItems;
use crate::templates::IDataTemplate;
use crate::{
    SortDirection,
    AssignedBinding, Control, ControlImpl, ItemsControl, ItemsControlImpl, ItemsControlImplExt, ListBox, TableViewColumn, TableViewRow,
};
use ferroui_base::collections::{FerroList, NotifyCollectionChangedAction, ResetBehavior};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::reactive::{Disposable, IDisposable, Subscription};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, BoxedValue, DirectProperty,
    FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref, StyledElement,
    StyledElementImpl, StyledElementImplExt, StyledProperty, VisualImpl, WeakRef,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// A read-only tabular control that presents items in configurable columns.
#[repr(C)]
pub struct TableView {
    base: ListBox,
    columns_subscription: RefCell<Option<Subscription>>,
    columns: RefCell<Option<FerroList<Ref<TableViewColumn>>>>,
    /// The presenter of the column headers of the template; it is part of
    /// the tree below the control and held weakly.
    headers_presenter: RefCell<Option<WeakRef<TableViewColumnHeadersPresenter>>>,
    /// The column the rows are sorted by, and its view of the items source.
    sort: RefCell<Option<(Ref<TableViewColumn>, SortDirection)>>,
    /// Set while the control replaces the source of its items for a sort.
    applying_sort: Cell<bool>,
    sorted_handlers: Rc<HandlerList<dyn Fn(&Ref<TableViewColumn>, SortDirection)>>,
}

ferro_class!(TableView: ListBox);
ferro_class_info!(TableView { new: TableView::new });
ferro_impl_classes!(
    TableView: VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    SelectingItemsControlImpl
);

impl FerroObjectImpl for TableView {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        if change.property() == Self::can_user_resize_columns_property().as_property() {
            let columns = this.columns.borrow().clone();
            if let Some(columns) = columns {
                for column in columns.snapshot().iter() {
                    column.update_can_user_effectively_resize();
                }
            }
        } else if change.property() == ItemsControl::items_source_property().as_property() {
            // A new source of the application, in the order of the column it is sorted by.
            this.apply_sort();
        }
    }
}

impl StyledElementImpl for TableView {
    fn on_initialized(this: &Self) {
        Self::parent_on_initialized(this);

        let columns = this.columns.borrow().clone();
        if let Some(columns) = columns {
            this.subscribe_to_columns(&columns);
        }
    }
}

impl ItemsControlImpl for TableView {
    fn create_container_for_item_override(
        _this: &Self,
        _item: &Option<BoxedValue>,
        _index: i32,
        _recycle_key: Option<RecycleKey>,
    ) -> Ref<Control> {
        TableViewRow::new().upcast()
    }

    fn prepare_container_for_item_override(
        this: &Self,
        container: &Ref<Control>,
        item: &Option<BoxedValue>,
        index: i32,
    ) {
        Self::parent_prepare_container_for_item_override(this, container, item, index);

        if let Some(row) = container.downcast_ref::<TableViewRow>() {
            row.set_columns(Some(this.columns()));
            row.rebuild_cells();
        }
    }

    fn needs_container_override(this: &Self, item: &Option<BoxedValue>, _index: i32) -> (bool, Option<RecycleKey>) {
        this.needs_container::<TableViewRow>(item)
    }

    fn clear_container_for_item_override(this: &Self, element: &Ref<Control>) {
        Self::parent_clear_container_for_item_override(this, element);

        if let Some(row) = element.downcast_ref::<TableViewRow>() {
            row.set_columns(None);
            row.clear_cells();
        }
    }
}

ferro_properties! {
    impl TableView {
        /// Defines the `Columns` property.
        pub fn columns_property() -> DirectProperty<TableView, Option<FerroList<Ref<TableViewColumn>>>> {
            FerroProperty::register_direct::<TableView, _>(
                "Columns",
                |o| Some(o.columns()),
                Some(|o, v| o.set_columns(v)),
                None,
            )
        }

        /// Defines the `CanUserResizeColumns` property.
        pub fn can_user_resize_columns_property() -> StyledProperty<bool> {
            FerroProperty::register::<TableView, _>("CanUserResizeColumns", true)
        }

        /// Defines the `CanUserSortColumns` property. Off by default, unlike WinUI: a table
        /// view of FerroUI does not sort unless it is asked to.
        pub fn can_user_sort_columns_property() -> StyledProperty<bool> {
            FerroProperty::register::<TableView, _>("CanUserSortColumns", false)
        }
    }
}

impl TableView {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: ListBox::construct(),
            columns_subscription: RefCell::new(None),
            columns: RefCell::new(None),
            headers_presenter: RefCell::new(None),
            sort: RefCell::new(None),
            applying_sort: Cell::new(false),
            sorted_handlers: Rc::new(HandlerList::new()),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The binding of the display member of the items control: it has no
    /// effect on a table view.
    #[deprecated(note = "DisplayMemberBinding has no effect on a TableView. Use TableViewColumn.Binding instead.")]
    pub fn display_member_binding(&self) -> Option<AssignedBinding> {
        ItemsControl::display_member_binding(self)
    }

    #[deprecated(note = "DisplayMemberBinding has no effect on a TableView. Use TableViewColumn.Binding instead.")]
    pub fn set_display_member_binding(&self, value: Option<AssignedBinding>) {
        ItemsControl::set_display_member_binding(self, value)
    }

    /// The item template of the items control: it has no effect on a table
    /// view.
    #[deprecated(note = "ItemTemplate has no effect on a TableView. Use TableViewColumn.CellTemplate instead.")]
    pub fn item_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        ItemsControl::item_template(self)
    }

    #[deprecated(note = "ItemTemplate has no effect on a TableView. Use TableViewColumn.CellTemplate instead.")]
    pub fn set_item_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        ItemsControl::set_item_template(self, value)
    }

    /// Gets or sets the collection of columns displayed by this
    /// [`TableView`]. The collection is created when it is first read.
    pub fn columns(&self) -> FerroList<Ref<TableViewColumn>> {
        if let Some(columns) = self.columns.borrow().clone() {
            return columns;
        }

        let columns = FerroList::new();
        columns.set_reset_behavior(ResetBehavior::Remove);
        *self.columns.borrow_mut() = Some(columns.clone());
        if self.is_initialized() {
            self.subscribe_to_columns(&columns);
        }

        columns
    }

    pub fn set_columns(&self, value: Option<FerroList<Ref<TableViewColumn>>>) {
        let old_value = self.columns.borrow().clone();
        if old_value == value {
            return;
        }

        self.unsubscribe_from_columns();
        if let Some(value) = &value {
            value.set_reset_behavior(ResetBehavior::Remove);
        }
        self.set_and_raise(Self::columns_property(), &self.columns, value.clone());

        if self.is_initialized() {
            if let Some(value) = &value {
                self.subscribe_to_columns(value);
            }

            self.rebuild_headers();
            self.rebuild_cells();
        }
    }

    /// Gets or sets a value indicating whether the user can resize columns
    /// by dragging the separator between column headers.
    pub fn can_user_resize_columns(&self) -> bool {
        self.get_value(Self::can_user_resize_columns_property())
    }

    pub fn set_can_user_resize_columns(&self, value: bool) {
        self.set_value(Self::can_user_resize_columns_property(), value)
    }

    /// Gets or sets whether a click on a column header sorts the rows. The default is false.
    pub fn can_user_sort_columns(&self) -> bool {
        self.get_value(Self::can_user_sort_columns_property())
    }

    pub fn set_can_user_sort_columns(&self, value: bool) {
        self.set_value(Self::can_user_sort_columns_property(), value)
    }

    /// Occurs after the rows were sorted, with the column and its direction
    /// ([`SortDirection::None`] when they are back in the order of the source).
    pub fn sorted(&self, handler: impl Fn(&Ref<TableViewColumn>, SortDirection) + 'static) -> Subscription {
        let handlers = self.sorted_handlers.clone();
        let token = handlers.add(Rc::new(handler));
        Subscription::new(Disposable::create(move || {
            handlers.remove(token);
        }))
    }

    /// Sorts the rows by `column`, which must be a column of this table with a comparer or a
    /// binding, or puts them back in the order of the source with [`SortDirection::None`]. The
    /// items source is not changed, and the selected items stay selected. Returns whether the
    /// sort was applied; it needs an `ItemsSource`.
    pub fn sort_by_column(&self, column: &Ref<TableViewColumn>, direction: SortDirection) -> bool {
        if self.columns().index_of(column).is_none() || self.items_source().is_none() {
            return false;
        }
        if direction != SortDirection::None && !column.is_sortable() {
            return false;
        }
        for other in self.columns().snapshot().iter() {
            let value = if other == column { direction } else { SortDirection::None };
            if other.sort_direction() != value {
                other.set_sort_direction(value);
                self.refresh_column_headers(other);
            }
        }
        *self.sort.borrow_mut() = (direction != SortDirection::None).then(|| (column.clone(), direction));
        self.apply_sort();
        for (_, handler) in self.sorted_handlers.snapshot().iter() {
            handler(column, direction);
        }
        true
    }

    /// A click on the header of `column`: the next direction of its cycle.
    pub(crate) fn toggle_sort_from_header(&self, column: &Ref<TableViewColumn>) -> bool {
        if !self.can_user_sort_columns() || !column.is_sortable() {
            return false;
        }
        self.sort_by_column(column, column.sort_cycle().next(column.sort_direction()))
    }

    /// Gives the items the source of the application, in the order of the current sort.
    fn apply_sort(&self) {
        if self.applying_sort.get() {
            return;
        }
        let source = self.items_source();
        let sort = self.sort.borrow().clone();
        let view = match (&source, sort) {
            (Some(source), Some((column, direction))) => {
                SortedItems::new(source.clone(), column, direction).map(|items| ItemsSource::new(items))
            }
            _ => None,
        };
        let Some(target) = view.or(source) else { return };

        let selected: Vec<Option<BoxedValue>> =
            self.selected_items().map(|items| items.to_vec()).unwrap_or_default();
        self.applying_sort.set(true);
        self.items().set_items_source(Some(target));
        self.applying_sort.set(false);
        if let Some(items) = self.selected_items() {
            for item in selected {
                if self.items_view().index_of(&item) >= 0 && !items.contains(&item) {
                    items.add(item);
                }
            }
        }
    }

    pub(crate) fn headers_presenter(&self) -> Option<Ref<TableViewColumnHeadersPresenter>> {
        self.headers_presenter.borrow().as_ref().and_then(WeakRef::upgrade)
    }

    pub(crate) fn set_headers_presenter(&self, value: Option<&Ref<TableViewColumnHeadersPresenter>>) {
        *self.headers_presenter.borrow_mut() = value.map(Ref::downgrade);
    }

    fn subscribe_to_columns(&self, columns: &FerroList<Ref<TableViewColumn>>) {
        debug_assert!(self.columns_subscription.borrow().is_none());

        let is_initial_iteration = Rc::new(Cell::new(true));

        let weak = self.to_ref().downgrade();
        let added = {
            let weak = weak.clone();
            let is_initial_iteration = is_initial_iteration.clone();
            move |column: &Ref<TableViewColumn>| {
                let Some(this) = weak.upgrade() else { return };
                this.attach_column(column);
                if !is_initial_iteration.get() {
                    this.on_columns_changed();
                }
            }
        };
        let removed = {
            let weak = weak.clone();
            move |column: &Ref<TableViewColumn>| {
                let Some(this) = weak.upgrade() else { return };
                this.detach_column(column);
                this.on_columns_changed();
            }
        };
        let items = move || {
            let this = weak.upgrade()?;
            let columns = this.columns.borrow().clone();
            columns.map(|columns| columns.snapshot())
        };

        let subscription = for_each_item(columns, items, added, removed, || {});
        *self.columns_subscription.borrow_mut() = Some(Subscription::new(subscription));

        is_initial_iteration.set(false);
    }

    fn attach_column(&self, column: &Ref<TableViewColumn>) {
        let this = self.to_ref();

        if column.table_view().is_some_and(|table_view| table_view != this) {
            panic!("The column {} is already attached to a TableView.", column.debug_display());
        }

        // Resolve styles and bindings before enabling refresh notifications.
        // The headers and cells are rebuilt after attachment to apply the
        // column's current values.
        column.set_parent(this.clone().upcast::<StyledElement>());
        column.set_table_view(Some(this));
    }

    fn detach_column(&self, column: &Ref<TableViewColumn>) {
        debug_assert!(column.table_view().is_none_or(|table_view| table_view == self.to_ref()));

        column.set_table_view(None);
        column.set_parent(None::<Ref<StyledElement>>);
        column.set_actual_width(f64::NAN);
    }

    fn unsubscribe_from_columns(&self) {
        let Some(subscription) = self.columns_subscription.take() else { return };

        subscription.dispose();

        for column in self.columns().snapshot().iter() {
            self.detach_column(column);
        }
    }

    fn on_columns_changed(&self) {
        TableViewLayoutHelper::reset_actual_widths(&self.columns());
        self.rebuild_headers();
        self.rebuild_cells();
    }

    pub(crate) fn on_columns_size_changed(&self) {
        TableViewLayoutHelper::reset_actual_widths(&self.columns());
        self.invalidate_headers_measure();
        self.invalidate_cells_measure();
    }

    fn rebuild_headers(&self) {
        if let Some(headers_presenter) = self.headers_presenter() {
            headers_presenter.rebuild_headers();
        }
    }

    fn rebuild_cells(&self) {
        let columns = self.columns();

        for row in self.get_realized_containers() {
            if let Some(table_view_row) = row.downcast_ref::<TableViewRow>() {
                table_view_row.set_columns(Some(columns.clone()));
                table_view_row.rebuild_cells();
            } else {
                row.invalidate_measure();
            }
        }
    }

    pub(crate) fn invalidate_headers_measure(&self) {
        if let Some(headers_presenter) = self.headers_presenter() {
            headers_presenter.invalidate_measure();
        }
    }

    pub(crate) fn invalidate_cells_measure(&self) {
        for row in self.get_realized_containers() {
            if let Some(table_view_row) = row.downcast_ref::<TableViewRow>() {
                table_view_row.invalidate_cells_measure();
            } else {
                row.invalidate_measure();
            }
        }
    }

    pub(crate) fn refresh_column_headers(&self, column: &Ref<TableViewColumn>) {
        let Some(column_index) = self.columns().index_of(column) else { return };

        if let Some(headers_presenter) = self.headers_presenter() {
            headers_presenter.refresh_header(column_index);
        }
    }

    pub(crate) fn refresh_column_cells(&self, column: &Ref<TableViewColumn>) {
        let Some(column_index) = self.columns().index_of(column) else { return };

        for row in self.get_realized_containers() {
            if let Some(table_view_row) = row.downcast_ref::<TableViewRow>() {
                table_view_row.refresh_cell(column_index);
            }
        }
    }
}

/// Invokes `added` for each item of the list and for each item added to it
/// later, `removed` for each item removed from it and `reset` when the list
/// is reset; the items of the list, read with `items`, are then added again.
/// The subscription lasts until the returned handle is disposed.
///
/// The handler belongs to the list, so it cannot hold the list itself:
/// `items` reads the list through its owner.
fn for_each_item<T: Clone + 'static>(
    collection: &FerroList<T>,
    items: impl Fn() -> Option<Rc<Vec<T>>> + 'static,
    added: impl Fn(&T) + 'static,
    removed: impl Fn(&T) + 'static,
    reset: impl Fn() + 'static,
) -> Rc<dyn IDisposable> {
    let add = move |items: &[T]| {
        for item in items {
            added(item);
        }
    };
    let remove = move |items: &[T]| {
        for item in items.iter().rev() {
            removed(item);
        }
    };

    add(&collection.snapshot());

    let token = collection.add_collection_changed(Rc::new(move |e| match e.action {
        NotifyCollectionChangedAction::Add => add(e.new_items),
        NotifyCollectionChangedAction::Move | NotifyCollectionChangedAction::Replace
            if e.old_starting_index >= 0 =>
        {
            remove(e.old_items);
            add(e.new_items);
        }
        NotifyCollectionChangedAction::Remove => remove(e.old_items),
        NotifyCollectionChangedAction::Move
        | NotifyCollectionChangedAction::Replace
        | NotifyCollectionChangedAction::Reset => {
            reset();
            if let Some(items) = items() {
                add(&items);
            }
        }
    }));

    let collection = collection.clone();
    Disposable::create(move || {
        collection.remove_collection_changed(token);
    })
}

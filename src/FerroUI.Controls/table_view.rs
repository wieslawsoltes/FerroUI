use crate::generators::RecycleKey;
use crate::presenters::table_view_layout_helper::TableViewLayoutHelper;
use crate::presenters::TableViewColumnHeadersPresenter;
use crate::primitives::{SelectingItemsControlImpl, TemplatedControlImpl};
use crate::templates::IDataTemplate;
use crate::table_view_source::TableViewSortDescription;
use crate::{
    AssignedBinding, Control, ControlImpl, ItemsControl, ItemsControlImpl, ItemsControlImplExt, ItemsSource, ListBox,
    TableViewColumn, TableViewRow, TableViewSortDirection, TableViewSortedEventArgs, TableViewSource,
};
use ferroui_base::interactivity::{Interactive, RoutedEvent, RoutedEventHandlerToken, RoutingStrategies};
use ferroui_base::collections::{FerroList, NotifyCollectionChangedAction, ResetBehavior};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::IBrush;
use ferroui_base::reactive::{Disposable, IDisposable, Subscription};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_markup_enum, ferro_properties, ferro_routed_event,
    instantiate, BoxedValue, DirectProperty,
    FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref, StyledElement,
    StyledElementImpl, StyledElementImplExt, StyledProperty, VisualImpl, WeakRef,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Which grid lines a [`TableView`] draws (WinUI's `TableViewGridLinesVisibility`).
///
/// The value removes lines the theme draws: the WinUI theme and its derived themes draw both,
/// the Fluent and Simple themes draw none (the upstream look), so there it changes nothing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum TableViewGridLinesVisibility {
    /// The horizontal lines between the rows and the vertical lines between the columns (the
    /// default).
    #[default]
    All = 0,
    /// Only the lines between the rows.
    Horizontal = 1,
    /// Only the lines between the columns.
    Vertical = 2,
    /// No grid lines.
    None = 3,
}

/// The sizes of the rows, the cells and the headers of a [`TableView`] (WinUI's
/// `TableViewDensity`, with the three sizes of the WinUI resources: `TableViewRowMinHeight`,
/// `TableViewRowMinHeightCompact` and `TableViewRowMinHeightComfortable`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum TableViewDensity {
    /// The sizes of the theme (the default).
    #[default]
    Standard = 0,
    /// Lower rows and less padding.
    Compact = 1,
    /// Higher rows and more padding.
    Comfortable = 2,
}

ferro_markup_enum!(TableViewGridLinesVisibility { All, Horizontal, Vertical, None }, { namespace: "FerroUI.Controls" });
ferro_markup_enum!(TableViewDensity { Standard, Compact, Comfortable }, { namespace: "FerroUI.Controls" });

/// A read-only tabular control that presents items in configurable columns.
///
/// The WinUI additions (`docs/porting/winui-controls.md`, section 10.11) are opt-in: with their
/// defaults the table behaves as upstream.
#[repr(C)]
pub struct TableView {
    base: ListBox,
    columns_subscription: RefCell<Option<Subscription>>,
    columns: RefCell<Option<FerroList<Ref<TableViewColumn>>>>,
    /// The presenter of the column headers of the template; it is part of
    /// the tree below the control and held weakly.
    headers_presenter: RefCell<Option<WeakRef<TableViewColumnHeadersPresenter>>>,
    /// The sorted view the table presents over an items source that is not a
    /// [`TableViewSource`] while it is sorted (WinUI's sorting).
    sort_view: RefCell<Option<Rc<TableViewSource>>>,
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

        let property = change.property();
        if property == Self::can_user_resize_columns_property().as_property() {
            let columns = this.columns.borrow().clone();
            if let Some(columns) = columns {
                for column in columns.snapshot().iter() {
                    column.update_can_user_effectively_resize();
                }
            }
        } else if property == Self::can_user_sort_columns_property().as_property() {
            let columns = this.columns.borrow().clone();
            if let Some(columns) = columns {
                for column in columns.snapshot().iter() {
                    column.update_can_user_effectively_sort();
                }
            }
        } else if property == ItemsControl::items_source_property().as_property() {
            // The items collection presents the new source; a sorted table sorts it again.
            *this.sort_view.borrow_mut() = None;
            if let Some(column) = this.sorted_column() {
                this.apply_sort(Some((&column, column.sort_direction())));
            }
        } else if property == Self::grid_lines_visibility_property().as_property() {
            this.update_grid_lines_pseudo_classes();
        } else if property == Self::density_property().as_property() {
            let density = this.density();
            this.pseudo_classes().set(":compact", density == TableViewDensity::Compact);
            this.pseudo_classes().set(":comfortable", density == TableViewDensity::Comfortable);
        } else if property == Self::row_background_property().as_property()
            || property == Self::alternating_row_background_property().as_property()
        {
            for row in this.get_realized_containers() {
                if let Some(table_view_row) = row.downcast_ref::<TableViewRow>() {
                    this.apply_row_background(table_view_row, this.index_from_container(&row));
                }
            }
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
            this.apply_row_background(row, index);
        }
    }

    fn container_index_changed_override(this: &Self, container: &Ref<Control>, old_index: i32, new_index: i32) {
        Self::parent_container_index_changed_override(this, container, old_index, new_index);

        if let Some(row) = container.downcast_ref::<TableViewRow>() {
            this.apply_row_background(row, new_index);
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

        /// Defines the `CanUserSortColumns` property (WinUI).
        pub fn can_user_sort_columns_property() -> StyledProperty<bool> {
            FerroProperty::register::<TableView, _>("CanUserSortColumns", false)
        }

        /// Defines the `GridLinesVisibility` property (WinUI).
        pub fn grid_lines_visibility_property() -> StyledProperty<TableViewGridLinesVisibility> {
            FerroProperty::register::<TableView, _>("GridLinesVisibility", TableViewGridLinesVisibility::All)
        }

        /// Defines the `Density` property (WinUI).
        pub fn density_property() -> StyledProperty<TableViewDensity> {
            FerroProperty::register::<TableView, _>("Density", TableViewDensity::Standard)
        }

        /// Defines the `RowBackground` property (WinUI).
        pub fn row_background_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
            FerroProperty::register::<TableView, _>("RowBackground", None)
        }

        /// Defines the `AlternatingRowBackground` property (WinUI).
        pub fn alternating_row_background_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
            FerroProperty::register::<TableView, _>("AlternatingRowBackground", None)
        }

        /// Defines the `EmptyTemplate` property (WinUI).
        pub fn empty_template_property() -> StyledProperty<Option<Rc<dyn IDataTemplate>>> {
            FerroProperty::register::<TableView, _>("EmptyTemplate", None)
        }
    }
}

ferro_properties! {
    impl TableView {
        ferro_routed_event!(
            /// Defines the `Sorted` event (WinUI): raised after the rows were sorted by a column,
            /// or put in the order of the source again.
            pub fn sorted_event() -> RoutedEvent<TableViewSortedEventArgs> {
                RoutedEvent::register::<TableView, _>("Sorted", RoutingStrategies::BUBBLE)
            }
        );
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
            sort_view: RefCell::new(None),
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

    /// Gets or sets whether a click on the header of a column sorts the rows by it (WinUI's
    /// `CanUserSortColumns`). The default is `false`, where WinUI's is `true`: an upstream table
    /// does not sort. A column decides for itself with [`TableViewColumn::can_sort`].
    pub fn can_user_sort_columns(&self) -> bool {
        self.get_value(Self::can_user_sort_columns_property())
    }

    pub fn set_can_user_sort_columns(&self, value: bool) {
        self.set_value(Self::can_user_sort_columns_property(), value)
    }

    /// Adds a handler of the `Sorted` event ([`sorted_event`](Self::sorted_event)).
    pub fn sorted(&self, handler: impl Fn(&Interactive, &TableViewSortedEventArgs) + 'static) -> RoutedEventHandlerToken {
        self.add_handler(Self::sorted_event(), handler)
    }

    /// Sorts the rows by `column` in `direction`, or puts them in the order of the source with
    /// [`None`](TableViewSortDirection::None), and raises `Sorted`. The other columns are no
    /// longer sorted. The items source is not changed: the table presents a sorted view of it,
    /// or sets the order of a [`TableViewSource`]. The selected items stay selected.
    ///
    /// The column must belong to the table.
    pub fn sort(&self, column: &Ref<TableViewColumn>, direction: TableViewSortDirection) {
        let columns = self.columns();
        assert!(columns.index_of(column).is_some(), "The column {} is not a column of this TableView.", column.debug_display());

        for other in columns.snapshot().iter() {
            if other != column && other.sort_direction() != TableViewSortDirection::None {
                other.set_current_value(TableViewColumn::sort_direction_property(), TableViewSortDirection::None);
            }
        }
        column.set_current_value(TableViewColumn::sort_direction_property(), direction);

        self.apply_sort(Some((column, direction)));

        let args = TableViewSortedEventArgs::new(Some(Self::sorted_event()), column.clone(), direction);
        self.raise_event(&args);
    }

    /// Puts the rows in the order of the source (no column is sorted); raises no event.
    pub fn clear_sort(&self) {
        for column in self.columns().snapshot().iter() {
            if column.sort_direction() != TableViewSortDirection::None {
                column.set_current_value(TableViewColumn::sort_direction_property(), TableViewSortDirection::None);
            }
        }
        self.apply_sort(None);
    }

    /// A click on the header of `column`: the next direction of its sort cycle, if the user can
    /// sort by it.
    pub(crate) fn on_column_header_clicked(&self, column: &Ref<TableViewColumn>) {
        if !column.can_user_effectively_sort() || self.columns().index_of(column).is_none() {
            return;
        }
        let direction = column.sort_cycle().next(column.sort_direction());
        self.sort(column, direction);
    }

    /// The column the rows are sorted by, if any.
    fn sorted_column(&self) -> Option<Ref<TableViewColumn>> {
        let columns = self.columns.borrow().clone()?;
        let column = columns.snapshot().iter().find(|column| column.sort_direction() != TableViewSortDirection::None).cloned();
        column
    }

    /// Presents the items in the order of `sort`, keeping the selected items selected.
    fn apply_sort(&self, sort: Option<(&Ref<TableViewColumn>, TableViewSortDirection)>) {
        let description = sort.and_then(|(column, direction)| match direction {
            TableViewSortDirection::None => None,
            direction => Some(TableViewSortDescription {
                key: column.sort_key(),
                descending: direction == TableViewSortDirection::Descending,
            }),
        });

        // A view of the table (or an unsorted table) has nothing to do without a sort.
        let own_source = self.items_source().and_then(|source| source.downcast_ref::<TableViewSource>().map(|_| source.clone()));
        if description.is_none() && own_source.is_none() && self.sort_view.borrow().is_none() {
            return;
        }

        let selected: Vec<Option<BoxedValue>> = self.selection().selected_items().iter().collect();

        if let Some(source) = own_source {
            let view = source.downcast_ref::<TableViewSource>().expect("a TableViewSource");
            view.set_sort(description);
        } else {
            match description {
                Some(description) => {
                    let existing = self.sort_view.borrow().clone();
                    match existing {
                        Some(view) => view.set_sort(Some(description)),
                        None => {
                            let view = TableViewSource::from(self.items().presented_source());
                            view.set_sort(Some(description));
                            *self.sort_view.borrow_mut() = Some(view.clone());
                            self.items().set_presented_source(ItemsSource::from(view));
                        }
                    }
                }
                None => {
                    let view = self.sort_view.borrow_mut().take();
                    if let Some(view) = view {
                        self.items().set_presented_source(view.source().clone());
                    }
                }
            }
        }

        self.restore_selection(&selected);
    }

    fn restore_selection(&self, selected: &[Option<BoxedValue>]) {
        if selected.is_empty() {
            return;
        }
        let selection = self.selection();
        let view = self.items_view().clone();
        selection.begin_batch_update();
        selection.clear();
        for item in selected {
            let index = view.index_of(item);
            if index >= 0 {
                selection.select(index);
            }
        }
        selection.end_batch_update();
    }

    /// Gets or sets which grid lines the table draws (WinUI's `GridLinesVisibility`). The default
    /// is [`All`](TableViewGridLinesVisibility::All): the lines of the theme. The other values
    /// set the pseudo-classes `:nohorizontalgridlines` and `:noverticalgridlines`, which the
    /// themes that draw grid lines honour.
    pub fn grid_lines_visibility(&self) -> TableViewGridLinesVisibility {
        self.get_value(Self::grid_lines_visibility_property())
    }

    pub fn set_grid_lines_visibility(&self, value: TableViewGridLinesVisibility) {
        self.set_value(Self::grid_lines_visibility_property(), value)
    }

    /// Gets or sets the sizes of the rows, the cells and the headers (WinUI's `Density`). The
    /// default is [`Standard`](TableViewDensity::Standard): the sizes of the theme. The other
    /// values set the pseudo-classes `:compact` and `:comfortable`.
    pub fn density(&self) -> TableViewDensity {
        self.get_value(Self::density_property())
    }

    pub fn set_density(&self, value: TableViewDensity) {
        self.set_value(Self::density_property(), value)
    }

    /// Gets or sets the background of the rows (WinUI's `RowBackground`). `None`, the default,
    /// leaves the background of the rows to their theme.
    pub fn row_background(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::row_background_property())
    }

    pub fn set_row_background(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::row_background_property(), value)
    }

    /// Gets or sets the background of every second row, the rows at odd indexes (WinUI's
    /// `AlternatingRowBackground`). `None`, the default, gives them the background of the
    /// others.
    pub fn alternating_row_background(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::alternating_row_background_property())
    }

    pub fn set_alternating_row_background(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::alternating_row_background_property(), value)
    }

    /// Gets or sets the template of what the table shows while it has no rows (WinUI's
    /// `EmptyTemplate`). The themes present it in `PART_EmptyStatePresenter` while the table is
    /// `:empty`; `None`, the default, shows nothing.
    pub fn empty_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.get_value(Self::empty_template_property())
    }

    pub fn set_empty_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        self.set_value(Self::empty_template_property(), value)
    }

    fn update_grid_lines_pseudo_classes(&self) {
        let visibility = self.grid_lines_visibility();
        let horizontal = matches!(visibility, TableViewGridLinesVisibility::All | TableViewGridLinesVisibility::Horizontal);
        let vertical = matches!(visibility, TableViewGridLinesVisibility::All | TableViewGridLinesVisibility::Vertical);
        self.pseudo_classes().set(":nohorizontalgridlines", !horizontal);
        self.pseudo_classes().set(":noverticalgridlines", !vertical);
    }

    /// Sets the background of a row from [`row_background`](Self::row_background) and
    /// [`alternating_row_background`](Self::alternating_row_background), or clears the one the
    /// table set when neither applies; a background the table did not set is left alone.
    fn apply_row_background(&self, row: &TableViewRow, index: i32) {
        let alternating = if index >= 0 && index % 2 == 1 { self.alternating_row_background() } else { None };
        row.set_table_background(alternating.or_else(|| self.row_background()));
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

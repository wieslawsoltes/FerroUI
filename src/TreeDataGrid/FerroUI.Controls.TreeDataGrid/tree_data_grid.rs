//! `TreeDataGrid.cs`: a control that shows hierarchical and tabular data together in one
//! virtualizing view.
//!
//! The parts of the reference's partial class are child modules: `TreeDataGrid.CoreModel.cs`
//! (`tree_data_grid_core_model.rs`) and `TreeDataGrid.V12.cs` (`tree_data_grid_v12.rs`).

#[path = "tree_data_grid_core_model.rs"]
mod core_model;
#[path = "tree_data_grid_v12.rs"]
mod v12;

pub use core_model::TreeDataGridSourceRef;
pub use v12::ItemsSourceModels;

use crate::automation::peers::TreeDataGridAutomationPeer;
use crate::models::{DragInfo, IColumns, ITreeDataGridRows};
use crate::presentation::{ITreeDataGridPresentationOptions, PresentationRef};
use crate::primitives::{
    TreeDataGridCell, TreeDataGridColumnHeader, TreeDataGridColumnHeadersPresenter, TreeDataGridElementFactory,
    TreeDataGridRow, TreeDataGridRowsPresenter,
};
use crate::selection::ITreeDataGridSelectionInteraction;
use crate::{
    TreeDataGridCellEventArgs, TreeDataGridColumns, TreeDataGridRowDragEventArgs, TreeDataGridRowDragStartedEventArgs,
    TreeDataGridRowDropPosition, TreeDataGridRowEventArgs, TreeDataGridSelectionChangedEventArgs, TreeDataGridSelectionMode,
};
use ferroui_base::input::{
    DataTransfer, DataTransferExtensions, DataTransferItem, DragDrop, DragDropEffects, DragEventArgs, IDataTransfer,
    InputElement, InputElementImpl, InputElementImplExt, KeyEventArgs, PointerEventArgs, PointerPressedEventArgs,
    PointerReleasedEventArgs, TextInputEventArgs,
};
use ferroui_base::interactivity::{InteractiveImpl, RoutedEvent, RoutedEventArgs, RoutedEventHandlerToken, RoutingStrategies};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::threading::{Dispatcher, DispatcherPriority, DispatcherTimer};
use ferroui_base::utilities::{CancelEventArgs, HandlerList};
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_properties, ferro_property, ferro_routed_event, instantiate,
    DirectProperty, FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Rect, Ref,
    StyledElement, StyledElementImpl, StyledProperty, Visual, VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs,
};
use ferroui_controls::automation::peers::AutomationPeer;
use ferroui_controls::documents::TextElement;
use ferroui_controls::metadata::TemplatePartAttribute;
use ferroui_controls::primitives::{
    AdornerLayer, TemplateAppliedEventArgs, TemplatedControl, TemplatedControlImpl, TemplatedControlImplExt,
};
use ferroui_controls::shapes::Rectangle;
use ferroui_controls::{Button, Canvas, Control, ControlImpl, ItemsSource, ScrollChangedEventArgs, ScrollViewer};
use ferroui_treedatagrid_core::models::ListSortDirection;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Duration;

const AUTO_SCROLL_MARGIN: f64 = 60.0;
const AUTO_SCROLL_SPEED: u64 = 50;

ferroui_base::ferro_thread_local! {
    // The drags of rows in progress, by the token their data transfer carries (`s_autoDragData`).
    static AUTO_DRAG_DATA: RefCell<HashMap<String, Rc<DragInfo>>> = RefCell::new(HashMap::new());
    static NEXT_DRAG_TOKEN: Cell<u64> = const { Cell::new(1) };
}

/// The subscriptions of the control to the `ScrollChanged` events of its scroll viewers.
struct ScrollSubscriptions {
    scroll: Ref<ScrollViewer>,
    scroll_token: RoutedEventHandlerToken,
    header_scroll: Ref<ScrollViewer>,
    header_scroll_token: RoutedEventHandlerToken,
}

/// Displays hierarchical and tabular data together in a single view (`TreeDataGrid`).
#[repr(C)]
pub struct TreeDataGrid {
    base: TemplatedControl,
    element_factory: RefCell<Option<Rc<TreeDataGridElementFactory>>>,
    source: RefCell<Option<TreeDataGridSourceRef>>,
    columns: RefCell<Option<Rc<dyn IColumns>>>,
    rows: RefCell<Option<Rc<dyn ITreeDataGridRows>>>,
    scroll: RefCell<Option<Ref<ScrollViewer>>>,
    header_scroll: RefCell<Option<Ref<ScrollViewer>>>,
    scroll_subscriptions: RefCell<Option<ScrollSubscriptions>>,
    selection: RefCell<Option<Rc<dyn ITreeDataGridSelectionInteraction>>>,
    selection_interaction_token: Cell<Option<u64>>,
    user_sort_column: RefCell<Option<Ref<Control>>>,
    user_sort_direction: Cell<ListSortDirection>,
    // A cell or row event in progress, to detect a nested one (`_cellArgs`, `_rowArgs`).
    raising_cell_event: Cell<bool>,
    raising_row_event: Cell<bool>,
    drag_adorner: RefCell<Option<Ref<Canvas>>>,
    hide_drag_adorner: Cell<bool>,
    auto_scroll_timer: RefCell<Option<(Rc<DispatcherTimer>, Rc<dyn IDisposable>)>>,
    auto_scroll_direction: Cell<bool>,
    is_attached_to_visual_tree: Cell<bool>,
    // The subscriptions to the presentation's `PropertyChanged` and `Sorted` (`_sourceEventsSubscribed`).
    source_events: Cell<Option<(u64, u64)>>,
    column_headers_presenter: RefCell<Option<Ref<TreeDataGridColumnHeadersPresenter>>>,
    rows_presenter: RefCell<Option<Ref<TreeDataGridRowsPresenter>>>,
    cell_clearing: HandlerList<dyn Fn(&TreeDataGridCellEventArgs)>,
    cell_prepared: HandlerList<dyn Fn(&TreeDataGridCellEventArgs)>,
    cell_value_changed: HandlerList<dyn Fn(&TreeDataGridCellEventArgs)>,
    row_clearing: HandlerList<dyn Fn(&TreeDataGridRowEventArgs)>,
    row_prepared: HandlerList<dyn Fn(&TreeDataGridRowEventArgs)>,
    selection_changing: HandlerList<dyn Fn(&CancelEventArgs)>,

    // `TreeDataGrid.CoreModel.cs`.
    presentation: RefCell<Option<PresentationRef>>,
    native_selection_token: Cell<Option<u64>>,

    // `TreeDataGrid.V12.cs`.
    explicit_source: RefCell<Option<TreeDataGridSourceRef>>,
    generated_source: RefCell<Option<TreeDataGridSourceRef>>,
    updating_selection_mode: Cell<bool>,
    column_definitions: TreeDataGridColumns,
    column_definitions_token: Cell<Option<u64>>,
    selection_changed: HandlerList<dyn Fn(&TreeDataGridSelectionChangedEventArgs)>,
}

ferro_class! {
    TreeDataGrid: TemplatedControl, virtuals TreeDataGridImpl: TemplatedControlImpl {
        /// Creates the element factory of the grid when none is set (`CreateDefaultElementFactory`).
        fn create_default_element_factory(this) -> Rc<TreeDataGridElementFactory>;
    }
}
ferroui_base::ferro_class_info!(TreeDataGrid {
    new: TreeDataGrid::new,
    markup: {
        namespace: "FerroUI.Controls",
        // The declarative columns are the content of the grid (`[Content] ColumnDefinitions`).
        content: ColumnDefinitions,
        properties: [
            ColumnDefinitions: crate::TreeDataGridColumns {
                get: |this: &Ref<TreeDataGrid>| this.column_definitions().clone()
            },
        ],
        fields: [
            RowDragStartedEvent: RoutedEvent<crate::TreeDataGridRowDragStartedEventArgs> => || *TreeDataGrid::row_drag_started_event(),
            RowDragOverEvent: RoutedEvent<crate::TreeDataGridRowDragEventArgs> => || *TreeDataGrid::row_drag_over_event(),
            RowDropEvent: RoutedEvent<crate::TreeDataGridRowDragEventArgs> => || *TreeDataGrid::row_drop_event(),
        ],
        events: [
            SelectionChanged(Option<ferroui_base::BoxedValue>, crate::TreeDataGridSelectionChangedEventArgs) =>
                |this: &Ref<TreeDataGrid>, handler: ferroui_base::metadata::MarkupDelegate| {
                    let sender = this.downgrade();
                    this.selection_changed(move |e: &crate::TreeDataGridSelectionChangedEventArgs| {
                        handler.invoke(&[
                            ferroui_base::metadata::into_markup_value(sender.upgrade()),
                            ferroui_base::metadata::into_markup_value(e.clone()),
                        ]);
                    })
                },
        ],
    },
});
ferro_impl_classes!(TreeDataGrid: StyledElementImpl, LayoutableImpl, InteractiveImpl);

ferro_properties! { impl TreeDataGrid {
    ferro_property!(
        /// Defines the `AutoDragDropRows` property.
        pub fn auto_drag_drop_rows_property() -> StyledProperty<bool> {
            FerroProperty::register::<TreeDataGrid, _>("AutoDragDropRows", false)
        }
    );

    ferro_property!(
        /// Defines the `CanUserResizeColumns` property.
        pub fn can_user_resize_columns_property() -> StyledProperty<bool> {
            FerroProperty::register::<TreeDataGrid, _>("CanUserResizeColumns", false)
        }
    );

    ferro_property!(
        /// Defines the `CanUserSortColumns` property.
        pub fn can_user_sort_columns_property() -> StyledProperty<bool> {
            FerroProperty::register::<TreeDataGrid, _>("CanUserSortColumns", true)
        }
    );

    ferro_property!(
        /// Defines the `Columns` property.
        pub fn columns_property() -> DirectProperty<TreeDataGrid, Option<Rc<dyn IColumns>>> {
            FerroProperty::register_direct::<TreeDataGrid, _>("Columns", |o| o.columns(), None, None)
        }
    );

    ferro_property!(
        /// Defines the `ElementFactory` property.
        pub fn element_factory_property() -> DirectProperty<TreeDataGrid, Option<Rc<TreeDataGridElementFactory>>> {
            FerroProperty::register_direct::<TreeDataGrid, _>(
                "ElementFactory",
                |o| Some(o.element_factory()),
                Some(|o, v| o.set_element_factory(v.expect("Value cannot be null. (Parameter 'value')"))),
                None,
            )
        }
    );

    ferro_property!(
        /// Defines the `Rows` property.
        pub fn rows_property() -> DirectProperty<TreeDataGrid, Option<Rc<dyn ITreeDataGridRows>>> {
            FerroProperty::register_direct::<TreeDataGrid, _>("Rows", |o| o.rows(), Some(|o, v| o.set_rows(v)), None)
        }
    );

    ferro_property!(
        /// Defines the `Scroll` property.
        pub fn scroll_property() -> DirectProperty<TreeDataGrid, Option<Ref<ScrollViewer>>> {
            FerroProperty::register_direct::<TreeDataGrid, _>("Scroll", |o| o.scroll(), None, None)
        }
    );

    ferro_property!(
        /// Defines the `ShowColumnHeaders` property.
        pub fn show_column_headers_property() -> StyledProperty<bool> {
            FerroProperty::register::<TreeDataGrid, _>("ShowColumnHeaders", true)
        }
    );

    ferro_property!(
        /// Defines the `Source` property.
        pub fn source_property() -> DirectProperty<TreeDataGrid, Option<TreeDataGridSourceRef>> {
            FerroProperty::register_direct::<TreeDataGrid, _>("Source", |o| o.source(), Some(|o, v| o.set_source(v)), None)
        }
    );

    ferro_property!(
        /// Defines the `Model` property: the primary, framework-neutral source. The control owns
        /// view state, never this model.
        pub fn model_property() -> StyledProperty<Option<TreeDataGridSourceRef>> {
            FerroProperty::register::<TreeDataGrid, _>("Model", None)
        }
    );

    ferro_property!(
        /// Defines the `PresentationOptions` property.
        pub fn presentation_options_property() -> StyledProperty<Option<Rc<dyn ITreeDataGridPresentationOptions>>> {
            FerroProperty::register::<TreeDataGrid, _>("PresentationOptions", None)
        }
    );

    ferro_property!(
        /// Defines the `Presentation` property.
        pub fn presentation_property() -> DirectProperty<TreeDataGrid, Option<PresentationRef>> {
            FerroProperty::register_direct::<TreeDataGrid, _>("Presentation", |o| o.presentation(), None, None)
        }
    );

    ferro_property!(
        /// Defines the `ItemsSource` property.
        pub fn items_source_property() -> StyledProperty<Option<ItemsSource>> {
            FerroProperty::register::<TreeDataGrid, _>("ItemsSource", None)
        }
    );

    ferro_property!(
        /// Defines the `SelectionMode` property.
        pub fn selection_mode_property() -> StyledProperty<TreeDataGridSelectionMode> {
            FerroProperty::register::<TreeDataGrid, _>("SelectionMode", TreeDataGridSelectionMode::ROW)
        }
    );
} }

impl TreeDataGrid {
    ferro_routed_event!(
        /// Defines the `RowDragStarted` event.
        pub fn row_drag_started_event() -> RoutedEvent<TreeDataGridRowDragStartedEventArgs> {
            RoutedEvent::register::<TreeDataGrid, _>("RowDragStarted", RoutingStrategies::BUBBLE)
        }
    );

    ferro_routed_event!(
        /// Defines the `RowDragOver` event.
        pub fn row_drag_over_event() -> RoutedEvent<TreeDataGridRowDragEventArgs> {
            RoutedEvent::register::<TreeDataGrid, _>("RowDragOver", RoutingStrategies::BUBBLE)
        }
    );

    ferro_routed_event!(
        /// Defines the `RowDrop` event.
        pub fn row_drop_event() -> RoutedEvent<TreeDataGridRowDragEventArgs> {
            RoutedEvent::register::<TreeDataGrid, _>("RowDrop", RoutingStrategies::BUBBLE)
        }
    );

    /// The named parts expected in the control template.
    pub const TEMPLATE_PARTS: &'static [TemplatePartAttribute] = &[
        TemplatePartAttribute::new("PART_ColumnHeadersPresenter", <TreeDataGridColumnHeadersPresenter as ferroui_base::StaticType>::TYPE),
        TemplatePartAttribute::new("PART_RowsPresenter", <TreeDataGridRowsPresenter as ferroui_base::StaticType>::TYPE),
        TemplatePartAttribute::new("PART_ScrollViewer", <ScrollViewer as ferroui_base::StaticType>::TYPE),
        TemplatePartAttribute::new("PART_HeaderScrollViewer", <ScrollViewer as ferroui_base::StaticType>::TYPE),
    ];

    fn static_constructor() {
        let _ = DragDrop::drag_over_event().add_class_handler::<TreeDataGrid>(|x, e| x.on_drag_over(e));
        let _ = DragDrop::drag_leave_event().add_class_handler::<TreeDataGrid>(|x, e| x.on_drag_leave(e));
        let _ = DragDrop::drop_event().add_class_handler::<TreeDataGrid>(|x, e| x.on_drop(e));
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: TemplatedControl::construct(),
            element_factory: RefCell::new(None),
            source: RefCell::new(None),
            columns: RefCell::new(None),
            rows: RefCell::new(None),
            scroll: RefCell::new(None),
            header_scroll: RefCell::new(None),
            scroll_subscriptions: RefCell::new(None),
            selection: RefCell::new(None),
            selection_interaction_token: Cell::new(None),
            user_sort_column: RefCell::new(None),
            user_sort_direction: Cell::new(ListSortDirection::Ascending),
            raising_cell_event: Cell::new(false),
            raising_row_event: Cell::new(false),
            drag_adorner: RefCell::new(None),
            hide_drag_adorner: Cell::new(false),
            auto_scroll_timer: RefCell::new(None),
            auto_scroll_direction: Cell::new(false),
            is_attached_to_visual_tree: Cell::new(false),
            source_events: Cell::new(None),
            column_headers_presenter: RefCell::new(None),
            rows_presenter: RefCell::new(None),
            cell_clearing: HandlerList::new(),
            cell_prepared: HandlerList::new(),
            cell_value_changed: HandlerList::new(),
            row_clearing: HandlerList::new(),
            row_prepared: HandlerList::new(),
            selection_changing: HandlerList::new(),
            presentation: RefCell::new(None),
            native_selection_token: Cell::new(None),
            explicit_source: RefCell::new(None),
            generated_source: RefCell::new(None),
            updating_selection_mode: Cell::new(false),
            column_definitions: TreeDataGridColumns::new(),
            column_definitions_token: Cell::new(None),
            selection_changed: HandlerList::new(),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Whether rows can be dragged and dropped to move them (`AutoDragDropRows`).
    pub fn auto_drag_drop_rows(&self) -> bool {
        self.get_value(Self::auto_drag_drop_rows_property())
    }

    /// Sets [`auto_drag_drop_rows`](Self::auto_drag_drop_rows).
    pub fn set_auto_drag_drop_rows(&self, value: bool) {
        self.set_value(Self::auto_drag_drop_rows_property(), value)
    }

    /// Whether the user can resize the columns (`CanUserResizeColumns`).
    pub fn can_user_resize_columns(&self) -> bool {
        self.get_value(Self::can_user_resize_columns_property())
    }

    /// Sets [`can_user_resize_columns`](Self::can_user_resize_columns).
    pub fn set_can_user_resize_columns(&self, value: bool) {
        self.set_value(Self::can_user_resize_columns_property(), value)
    }

    /// Whether the user can sort by clicking a column header (`CanUserSortColumns`).
    pub fn can_user_sort_columns(&self) -> bool {
        self.get_value(Self::can_user_sort_columns_property())
    }

    /// Sets [`can_user_sort_columns`](Self::can_user_sort_columns).
    pub fn set_can_user_sort_columns(&self, value: bool) {
        self.set_value(Self::can_user_sort_columns_property(), value)
    }

    /// The columns presented (`Columns`).
    pub fn columns(&self) -> Option<Rc<dyn IColumns>> {
        self.columns.borrow().clone()
    }

    pub(crate) fn set_columns(&self, value: Option<Rc<dyn IColumns>>) {
        self.set_and_raise(Self::columns_property(), &self.columns, value);
    }

    /// The factory of the rows, cells and column headers (`ElementFactory`); created by
    /// [`create_default_element_factory`](TreeDataGridImpl::create_default_element_factory) when
    /// none is set.
    pub fn element_factory(&self) -> Rc<TreeDataGridElementFactory> {
        if let Some(factory) = self.element_factory.borrow().clone() {
            return factory;
        }
        let factory = self.create_default_element_factory();
        *self.element_factory.borrow_mut() = Some(factory.clone());
        factory
    }

    /// Sets [`element_factory`](Self::element_factory).
    pub fn set_element_factory(&self, value: Rc<TreeDataGridElementFactory>) {
        self.set_and_raise(Self::element_factory_property(), &self.element_factory, Some(value));
    }

    /// The rows presented (`Rows`).
    pub fn rows(&self) -> Option<Rc<dyn ITreeDataGridRows>> {
        self.rows.borrow().clone()
    }

    pub(crate) fn set_rows(&self, value: Option<Rc<dyn ITreeDataGridRows>>) {
        self.set_and_raise(Self::rows_property(), &self.rows, value);
    }

    /// The presenter of the column headers of the template (`PART_ColumnHeadersPresenter`).
    pub fn column_headers_presenter(&self) -> Option<Ref<TreeDataGridColumnHeadersPresenter>> {
        self.column_headers_presenter.borrow().clone()
    }

    /// The presenter of the rows of the template (`PART_RowsPresenter`).
    pub fn rows_presenter(&self) -> Option<Ref<TreeDataGridRowsPresenter>> {
        self.rows_presenter.borrow().clone()
    }

    /// The scroll viewer of the rows (`Scroll`).
    pub fn scroll(&self) -> Option<Ref<ScrollViewer>> {
        self.scroll.borrow().clone()
    }

    fn set_scroll(&self, value: Option<Ref<ScrollViewer>>) {
        self.set_and_raise(Self::scroll_property(), &self.scroll, value);
    }

    /// Whether the column headers are shown (`ShowColumnHeaders`).
    pub fn show_column_headers(&self) -> bool {
        self.get_value(Self::show_column_headers_property())
    }

    /// Sets [`show_column_headers`](Self::show_column_headers).
    pub fn set_show_column_headers(&self, value: bool) {
        self.set_value(Self::show_column_headers_property(), value)
    }

    /// The cell selection of the source, if it is one (`ColumnSelection`).
    pub fn column_selection(&self) -> Option<Rc<dyn crate::selection::IUntypedCellSelection>> {
        let source = self.source()?;
        crate::selection::as_cell_selection(&source.selection()?)
    }

    /// The row selection of the source, if it is one (`RowSelection`).
    pub fn row_selection(&self) -> Option<Rc<dyn ferroui_treedatagrid_core::selection::ITreeSelectionModel>> {
        let source = self.source()?;
        let selection = source.selection()?;
        if crate::selection::as_cell_selection(&selection).is_some() {
            return None;
        }
        selection.row_selection_model()
    }

    /// The source shown (`Source`): the explicit source, or the source generated from the items
    /// and declarative columns; `None` while a model is shown.
    pub fn source(&self) -> Option<TreeDataGridSourceRef> {
        self.source.borrow().clone()
    }

    /// Sets the explicit source (`Source`): it replaces the model.
    pub fn set_source(&self, value: Option<TreeDataGridSourceRef>) {
        let same = *self.explicit_source.borrow() == value;
        if !same || self.model().is_some() {
            if self.model().is_some() {
                self.set_current_value(Self::model_property(), None);
            }
            *self.explicit_source.borrow_mut() = value;
            self.update_active_source();
        }
    }

    /// The interaction of the selection with the control (`SelectionInteraction`).
    pub fn selection_interaction(&self) -> Option<Rc<dyn ITreeDataGridSelectionInteraction>> {
        self.selection.borrow().clone()
    }

    pub(crate) fn set_selection_interaction(&self, value: Option<Rc<dyn ITreeDataGridSelectionInteraction>>) {
        let same = match (&*self.selection.borrow(), &value) {
            (Some(a), Some(b)) => std::ptr::addr_eq(Rc::as_ptr(a), Rc::as_ptr(b)),
            (None, None) => true,
            _ => false,
        };
        if !same {
            self.unsubscribe_selection_interaction();
            *self.selection.borrow_mut() = value;
            self.subscribe_selection_interaction();
        }
    }

    fn subscribe_list<A: ?Sized + 'static>(
        &self,
        select: fn(&TreeDataGrid) -> &HandlerList<dyn Fn(&A)>,
        handler: Rc<dyn Fn(&A)>,
    ) -> Rc<dyn IDisposable> {
        let token = select(self).add(handler);
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                select(&this).remove(token);
            }
        })
    }

    /// `CellClearing`: a cell is about to be cleared. Disposing the returned handle unsubscribes.
    pub fn cell_clearing(&self, handler: impl Fn(&TreeDataGridCellEventArgs) + 'static) -> Rc<dyn IDisposable> {
        self.subscribe_list(|o| &o.cell_clearing, Rc::new(handler))
    }

    /// `CellPrepared`: a cell was prepared for display.
    pub fn cell_prepared(&self, handler: impl Fn(&TreeDataGridCellEventArgs) + 'static) -> Rc<dyn IDisposable> {
        self.subscribe_list(|o| &o.cell_prepared, Rc::new(handler))
    }

    /// `CellValueChanged`: the value of a cell changed.
    pub fn cell_value_changed(&self, handler: impl Fn(&TreeDataGridCellEventArgs) + 'static) -> Rc<dyn IDisposable> {
        self.subscribe_list(|o| &o.cell_value_changed, Rc::new(handler))
    }

    /// `RowClearing`: a row is about to be cleared.
    pub fn row_clearing(&self, handler: impl Fn(&TreeDataGridRowEventArgs) + 'static) -> Rc<dyn IDisposable> {
        self.subscribe_list(|o| &o.row_clearing, Rc::new(handler))
    }

    /// `RowPrepared`: a row was prepared for display.
    pub fn row_prepared(&self, handler: impl Fn(&TreeDataGridRowEventArgs) + 'static) -> Rc<dyn IDisposable> {
        self.subscribe_list(|o| &o.row_prepared, Rc::new(handler))
    }

    /// `SelectionChanging`: the selection is about to change; a handler cancels it with
    /// [`CancelEventArgs::set_cancel`].
    pub fn selection_changing(&self, handler: impl Fn(&CancelEventArgs) + 'static) -> Rc<dyn IDisposable> {
        self.subscribe_list(|o| &o.selection_changing, Rc::new(handler))
    }

    /// `RowDragStarted`: a drag of rows started.
    pub fn row_drag_started(
        &self,
        handler: impl Fn(&ferroui_base::interactivity::Interactive, &TreeDataGridRowDragStartedEventArgs) + 'static,
    ) -> RoutedEventHandlerToken {
        self.add_handler(Self::row_drag_started_event(), handler)
    }

    /// `RowDragOver`: rows are dragged over a row.
    pub fn row_drag_over(
        &self,
        handler: impl Fn(&ferroui_base::interactivity::Interactive, &TreeDataGridRowDragEventArgs) + 'static,
    ) -> RoutedEventHandlerToken {
        self.add_handler(Self::row_drag_over_event(), handler)
    }

    /// `RowDrop`: rows are dropped on a row.
    pub fn row_drop(
        &self,
        handler: impl Fn(&ferroui_base::interactivity::Interactive, &TreeDataGridRowDragEventArgs) + 'static,
    ) -> RoutedEventHandlerToken {
        self.add_handler(Self::row_drop_event(), handler)
    }

    /// The cell at `column_index` of the row at `row_index`, if it is realized (`TryGetCell`).
    pub fn try_get_cell(&self, column_index: i32, row_index: i32) -> Option<Ref<Control>> {
        self.try_get_row(row_index)?.try_get_cell(column_index)
    }

    /// The row at `row_index`, if it is realized (`TryGetRow`).
    pub fn try_get_row(&self, row_index: i32) -> Option<Ref<TreeDataGridRow>> {
        self.rows_presenter()?.try_get_element(row_index)?.cast::<TreeDataGridRow>()
    }

    /// The cell that is `element` or contains it (`TryGetCell(Control, out TreeDataGridCell)`).
    pub fn try_get_cell_of(&self, element: Option<&Control>) -> Option<Ref<TreeDataGridCell>> {
        let cell = element?.find_ancestor_of_type::<TreeDataGridCell>(true)?;
        if cell.column_index() >= 0 && cell.row_index() >= 0 {
            Some(cell)
        } else {
            None
        }
    }

    /// The row that is `element` or contains it (`TryGetRow(Control, out TreeDataGridRow)`).
    pub fn try_get_row_of(&self, element: Option<&Control>) -> Option<Ref<TreeDataGridRow>> {
        let element = element?;
        if let Some(row) = element.to_ref().cast::<TreeDataGridRow>() {
            if row.row_index() >= 0 {
                return Some(row);
            }
        }

        let mut current: Ref<Visual> = element.to_ref().upcast::<Visual>();
        loop {
            let result = current.find_ancestor_of_type::<TreeDataGridRow>(false)?;
            if result.row_index() >= 0 {
                return Some(result);
            }
            current = result.upcast::<Visual>();
        }
    }

    /// The model of the row that is `element` or contains it, if it is a model of type `M`
    /// (`TryGetRowModel<TModel>`).
    pub fn try_get_row_model<M: 'static>(&self, element: &Control) -> Option<Rc<M>> {
        let presentation = self.presentation()?;
        let row = self.try_get_row_of(Some(element))?;
        let rows = presentation.rows();
        if row.row_index() < 0 || row.row_index() as usize >= rows.count() {
            return None;
        }
        let model = rows.get(row.row_index() as usize).model()?;
        let any: Rc<dyn std::any::Any> = model;
        any.downcast::<M>().ok()
    }

    /// Asks the `SelectionChanging` handlers whether the selection may change; true when one
    /// cancels it (`QueryCancelSelection`).
    pub fn query_cancel_selection(&self) -> bool {
        if self.selection_changing.is_empty() {
            return false;
        }
        let e = CancelEventArgs::new();
        for (_, handler) in self.selection_changing.snapshot().iter() {
            handler(&e);
        }
        e.cancel()
    }

    fn subscribe_source_events(&self) {
        if !self.is_attached_to_visual_tree.get() || self.source_events.get().is_some() {
            return;
        }
        let Some(presentation) = self.presentation() else {
            return;
        };
        let weak = self.to_ref().downgrade();
        let property_token = presentation.property_changed().add(Rc::new(move |name: &str| {
            if let Some(this) = weak.upgrade() {
                this.on_source_property_changed(name);
            }
        }));
        let weak = self.to_ref().downgrade();
        let sorted_token = presentation.add_sorted(Rc::new(move || {
            if let Some(this) = weak.upgrade() {
                this.on_source_sorted();
            }
        }));
        self.source_events.set(Some((property_token, sorted_token)));
    }

    fn unsubscribe_source_events(&self) {
        let Some(presentation) = self.presentation() else {
            return;
        };
        if let Some((property_token, sorted_token)) = self.source_events.take() {
            presentation.property_changed().remove(property_token);
            presentation.remove_sorted(sorted_token);
        }
    }

    fn subscribe_selection_interaction(&self) {
        if !self.is_attached_to_visual_tree.get() || self.selection_interaction_token.get().is_some() {
            return;
        }
        let Some(selection) = self.selection_interaction() else {
            return;
        };
        let weak = self.to_ref().downgrade();
        let token = selection.add_selection_changed(Rc::new(move || {
            if let Some(this) = weak.upgrade() {
                this.on_selection_interaction_changed();
            }
        }));
        self.selection_interaction_token.set(Some(token));
    }

    fn unsubscribe_selection_interaction(&self) {
        let Some(selection) = self.selection_interaction() else {
            return;
        };
        if let Some(token) = self.selection_interaction_token.take() {
            selection.remove_selection_changed(token);
        }
    }

    fn on_preview_key_down(&self, e: &KeyEventArgs) {
        if let Some(selection) = self.selection_interaction() {
            selection.on_preview_key_down(self, e);
        }
    }

    fn raise_cell_event(
        &self,
        list: &HandlerList<dyn Fn(&TreeDataGridCellEventArgs)>,
        cell: &TreeDataGridCell,
        column_index: i32,
        row_index: i32,
    ) {
        if list.is_empty() {
            return;
        }
        if self.raising_cell_event.replace(true) {
            panic!("Nested TreeDataGrid cell prepared/clearing detected.");
        }
        struct Reset<'a>(&'a Cell<bool>);
        impl Drop for Reset<'_> {
            fn drop(&mut self) {
                self.0.set(false);
            }
        }
        let _reset = Reset(&self.raising_cell_event);
        let e = TreeDataGridCellEventArgs::new(cell.to_ref().upcast::<Control>(), column_index, row_index);
        for (_, handler) in list.snapshot().iter() {
            handler(&e);
        }
    }

    fn raise_row_event(&self, list: &HandlerList<dyn Fn(&TreeDataGridRowEventArgs)>, row: &TreeDataGridRow, row_index: i32) {
        if list.is_empty() {
            return;
        }
        if self.raising_row_event.replace(true) {
            panic!("Nested TreeDataGrid row prepared/clearing detected.");
        }
        struct Reset<'a>(&'a Cell<bool>);
        impl Drop for Reset<'_> {
            fn drop(&mut self) {
                self.0.set(false);
            }
        }
        let _reset = Reset(&self.raising_row_event);
        let e = TreeDataGridRowEventArgs::new(row.to_ref(), row_index);
        for (_, handler) in list.snapshot().iter() {
            handler(&e);
        }
    }

    /// Raises `CellClearing` (`RaiseCellClearing`).
    pub fn raise_cell_clearing(&self, cell: &TreeDataGridCell, column_index: i32, row_index: i32) {
        self.raise_cell_event(&self.cell_clearing, cell, column_index, row_index);
    }

    /// Raises `CellPrepared` (`RaiseCellPrepared`).
    pub fn raise_cell_prepared(&self, cell: &TreeDataGridCell, column_index: i32, row_index: i32) {
        self.raise_cell_event(&self.cell_prepared, cell, column_index, row_index);
    }

    /// Raises `CellValueChanged` (`RaiseCellValueChanged`).
    pub fn raise_cell_value_changed(&self, cell: &TreeDataGridCell, column_index: i32, row_index: i32) {
        self.raise_cell_event(&self.cell_value_changed, cell, column_index, row_index);
    }

    /// Raises `RowClearing` (`RaiseRowClearing`).
    pub fn raise_row_clearing(&self, row: &TreeDataGridRow, row_index: i32) {
        self.raise_row_event(&self.row_clearing, row, row_index);
    }

    /// Raises `RowPrepared` (`RaiseRowPrepared`).
    pub fn raise_row_prepared(&self, row: &TreeDataGridRow, row_index: i32) {
        self.raise_row_event(&self.row_prepared, row, row_index);
    }

    /// Starts a drag of the selected rows after the press `trigger` (`RaiseRowDragStarted`):
    /// raises `RowDragStarted`, then starts the drag and drop operation with the effects allowed.
    pub fn raise_row_drag_started(&self, trigger: &PointerPressedEventArgs) {
        let Some(presentation) = self.presentation() else {
            return;
        };
        let Some(selected_indexes) = presentation.selected_indexes() else {
            return;
        };

        let mut allowed_effects = if self.auto_drag_drop_rows() && !presentation.is_sorted() {
            DragDropEffects::MOVE
        } else {
            DragDropEffects::NONE
        };
        let route = self.build_event_route(Self::row_drag_started_event());

        if route.has_handlers() {
            let e = TreeDataGridRowDragStartedEventArgs::new(presentation.selected_items().unwrap_or_default());
            e.set_allowed_effects(allowed_effects);
            self.raise_event(&e);
            allowed_effects = e.allowed_effects();
        }

        if allowed_effects != DragDropEffects::NONE {
            let source = self.model().or_else(|| self.source()).map(|source| source.source().clone());
            let info = Rc::new(DragInfo::from_presentation(&*presentation, source, selected_indexes));
            Self::do_drag_drop_async(trigger, info, allowed_effects);
        }
    }

    fn do_drag_drop_async(trigger: &PointerPressedEventArgs, info: Rc<DragInfo>, allowed_effects: DragDropEffects) {
        let token = NEXT_DRAG_TOKEN.with(|next| {
            let token = next.get();
            next.set(token + 1);
            format!("{token:032x}")
        });

        AUTO_DRAG_DATA.with(|data| {
            data.borrow_mut().insert(token.clone(), info);
        });

        let item = DataTransferItem::create(&DragInfo::data_format(), Some(token.clone()));
        let transfer = DataTransfer::new();
        transfer.add(item);
        let transfer: Rc<dyn IDataTransfer> = transfer;
        let operation = DragDrop::do_drag_drop_async(trigger, transfer, allowed_effects);
        drop(Dispatcher::ui_thread().invoke_async_task_local(move || async move {
            let _ = operation.await;
            AUTO_DRAG_DATA.with(|data| {
                data.borrow_mut().remove(&token);
            });
        }));
    }

    fn on_click(&self, e: &RoutedEventArgs) {
        let Some(presentation) = self.presentation() else {
            return;
        };
        let Some(column_header) = e.source().and_then(|source| source.cast::<TreeDataGridColumnHeader>()) else {
            return;
        };
        let columns = presentation.columns();
        if column_header.column_index() < 0 || column_header.column_index() as usize >= columns.count() || !self.can_user_sort_columns()
        {
            return;
        }

        let header: Ref<Control> = column_header.clone().upcast::<Control>();
        let same = self.user_sort_column.borrow().as_ref().is_some_and(|current| current.ptr_eq(&header));
        if !same {
            *self.user_sort_column.borrow_mut() = Some(header);
            self.user_sort_direction.set(ListSortDirection::Ascending);
        } else {
            let direction = if self.user_sort_direction.get() == ListSortDirection::Ascending {
                ListSortDirection::Descending
            } else {
                ListSortDirection::Ascending
            };
            self.user_sort_direction.set(direction);
        }

        let column = columns.get(column_header.column_index() as usize);
        presentation.sort_by(&column, self.user_sort_direction.get());
    }

    fn get_or_create_drag_adorner(&self) -> Option<Ref<Canvas>> {
        self.hide_drag_adorner.set(false);

        if let Some(adorner) = self.drag_adorner.borrow().clone() {
            return Some(adorner);
        }

        let adorner_layer = AdornerLayer::get_adorner_layer(self)?;

        let rectangle = Rectangle::new();
        rectangle.set_stroke(TextElement::get_foreground(self));
        rectangle.set_stroke_thickness(2.0);
        let adorner = Canvas::new();
        adorner.children().add(rectangle.upcast::<Control>());
        adorner.set_is_hit_test_visible(false);

        adorner_layer.children().add(adorner.clone().upcast::<Control>());
        AdornerLayer::set_adorned_element(&adorner, self.to_ref().upcast::<Visual>());
        *self.drag_adorner.borrow_mut() = Some(adorner.clone());
        Some(adorner)
    }

    fn show_drag_adorner(&self, row: &TreeDataGridRow, position: TreeDataGridRowDropPosition) {
        let transform = if position == TreeDataGridRowDropPosition::None { None } else { row.transform_to_visual(self) };
        let Some(transform) = transform else {
            self.hide_drag_adorner();
            return;
        };

        let Some(adorner) = self.get_or_create_drag_adorner() else {
            return;
        };

        let Some(rectangle) = adorner.children().get(0).cast::<Rectangle>() else {
            return;
        };
        let row_bounds = Rect::from_size(row.bounds().size()).transform_to_aabb(transform);

        Canvas::set_left(&rectangle, row_bounds.x);
        rectangle.set_width(row_bounds.width);

        match position {
            TreeDataGridRowDropPosition::Before => {
                Canvas::set_top(&rectangle, row_bounds.y);
                rectangle.set_height(0.0);
            }
            TreeDataGridRowDropPosition::After => {
                Canvas::set_top(&rectangle, row_bounds.bottom());
                rectangle.set_height(0.0);
            }
            TreeDataGridRowDropPosition::Inside => {
                Canvas::set_top(&rectangle, row_bounds.y);
                rectangle.set_height(row_bounds.height);
            }
            TreeDataGridRowDropPosition::None => {}
        }
    }

    fn hide_drag_adorner(&self) {
        self.hide_drag_adorner.set(true);

        let weak = self.to_ref().downgrade();
        let _ = DispatcherTimer::run_once(
            move || {
                let Some(this) = weak.upgrade() else {
                    return;
                };
                if !this.hide_drag_adorner.get() {
                    return;
                }
                let adorner = this.drag_adorner.borrow().clone();
                if let Some(adorner) = adorner {
                    if let Some(layer) = adorner.parent().and_then(|parent| parent.cast::<AdornerLayer>()) {
                        layer.children().remove(adorner.upcast::<Control>());
                        *this.drag_adorner.borrow_mut() = None;
                    }
                }
            },
            Duration::from_millis(50),
            DispatcherPriority::DEFAULT,
        );
    }

    fn stop_drag(&self) {
        self.hide_drag_adorner();
        let timer = self.auto_scroll_timer.borrow().as_ref().map(|(timer, _)| timer.clone());
        if let Some(timer) = timer {
            timer.stop();
        }
    }

    fn auto_scroll(&self, direction: bool) {
        if self.auto_scroll_timer.borrow().is_none() {
            let timer = DispatcherTimer::new();
            timer.set_interval(Duration::from_millis(AUTO_SCROLL_SPEED));
            let weak = self.to_ref().downgrade();
            let tick = timer.tick(move |_| {
                if let Some(this) = weak.upgrade() {
                    this.on_auto_scroll_tick();
                }
            });
            *self.auto_scroll_timer.borrow_mut() = Some((timer, tick));
        }

        self.auto_scroll_direction.set(direction);

        let timer = self.auto_scroll_timer.borrow().as_ref().map(|(timer, _)| timer.clone());
        if let Some(timer) = timer {
            if !timer.is_enabled() {
                self.on_auto_scroll_tick();
            }
            timer.start();
        }
    }

    // `CalculateAutoDragDrop`: the drag of rows of this grid that `e` carries, and where it drops
    // on `target_row`; `None` when the rows cannot be dropped there.
    fn calculate_auto_drag_drop(
        &self,
        target_row: Option<&Ref<TreeDataGridRow>>,
        e: &DragEventArgs,
    ) -> (Option<Rc<DragInfo>>, TreeDataGridRowDropPosition) {
        let none = (None, TreeDataGridRowDropPosition::None);
        if !self.auto_drag_drop_rows() {
            return none;
        }
        let Some(di) = Self::try_get_drag_info(e) else {
            return none;
        };
        let Some(presentation) = self.presentation() else {
            return none;
        };
        let Some(target_row) = target_row else {
            return none;
        };
        if presentation.is_sorted() || !std::ptr::addr_eq(Rc::as_ptr(di.source_identity()), Rc::as_ptr(&presentation.source_identity())) {
            return none;
        }

        let target_index = presentation.rows().row_index_to_model_index(target_row.row_index());
        let position = Self::get_drop_position(&*presentation, e, target_row);

        // We can't drop rows into themselves or their descendents.
        for source_index in di.indexes() {
            if source_index.is_ancestor_of(&target_index)
                || (*source_index == target_index && position == TreeDataGridRowDropPosition::Inside)
            {
                return none;
            }
        }

        (Some(di), position)
    }

    fn try_get_drag_info(e: &DragEventArgs) -> Option<Rc<DragInfo>> {
        let token = e.data_transfer().try_get_value(&DragInfo::data_format())?;
        AUTO_DRAG_DATA.with(|data| data.borrow().get(&token).cloned())
    }

    fn source_control(e: &RoutedEventArgs) -> Option<Ref<Control>> {
        e.source().and_then(|source| source.cast::<Control>())
    }

    fn on_drag_over(&self, e: &DragEventArgs) {
        let row = self.try_get_row_of(Self::source_control(e).as_deref());
        if row.is_none() {
            e.set_drag_effects(DragDropEffects::NONE);
        }

        let (data, mut adorner) = self.calculate_auto_drag_drop(row.as_ref(), e);
        if data.is_none() {
            e.set_drag_effects(DragDropEffects::NONE);
        }

        let route = self.build_event_route(Self::row_drag_over_event());

        if route.has_handlers() {
            let ev = TreeDataGridRowDragEventArgs::new(Self::row_drag_over_event(), row.clone(), e.clone());
            ev.set_position(adorner);
            self.raise_event(&ev);
            adorner = ev.position();
        }

        if let Some(row) = &row {
            self.show_drag_adorner(row, adorner);
        }

        if let Some(scroller) = self.scroll() {
            let rows_position = e.get_position(&scroller);

            if rows_position.y < AUTO_SCROLL_MARGIN {
                self.auto_scroll(false);
            } else if rows_position.y > self.bounds().height - AUTO_SCROLL_MARGIN {
                self.auto_scroll(true);
            } else {
                let timer = self.auto_scroll_timer.borrow().as_ref().map(|(timer, _)| timer.clone());
                if let Some(timer) = timer {
                    timer.stop();
                }
            }
        }
    }

    fn on_drag_leave(&self, _e: &DragEventArgs) {
        self.stop_drag();
    }

    fn on_drop(&self, e: &DragEventArgs) {
        self.stop_drag();

        let row = self.try_get_row_of(Self::source_control(e).as_deref());

        let (data, mut position) = self.calculate_auto_drag_drop(row.as_ref(), e);
        let route = self.build_event_route(Self::row_drop_event());

        if route.has_handlers() {
            let ev = TreeDataGridRowDragEventArgs::new(Self::row_drop_event(), row.clone(), e.clone());
            ev.set_position(position);
            self.raise_event(&ev);

            if ev.handled() || e.drag_effects() != DragDropEffects::MOVE {
                return;
            }

            position = ev.position();
        }

        if let (Some(data), Some(presentation), Some(row)) = (data, self.presentation(), row) {
            if position != TreeDataGridRowDropPosition::None {
                let target_index = presentation.rows().row_index_to_model_index(row.row_index());
                presentation.move_rows(data.indexes(), &target_index, position, e.drag_effects());
            }
        }
    }

    fn on_scroll_changed(&self, e: &ScrollChangedEventArgs) {
        if let (Some(scroll), Some(header_scroll)) = (self.scroll(), self.header_scroll.borrow().clone()) {
            if !crate::presentation::is_zero(e.offset_delta().x) {
                header_scroll.set_offset(header_scroll.offset().with_x(scroll.offset().x));
            }
        }
    }

    fn on_header_scroll_changed(&self, e: &ScrollChangedEventArgs) {
        if let (Some(scroll), Some(header_scroll)) = (self.scroll(), self.header_scroll.borrow().clone()) {
            if !crate::presentation::is_zero(e.offset_delta().x) {
                scroll.set_offset(scroll.offset().with_x(header_scroll.offset().x));
            }
        }
    }

    fn on_auto_scroll_tick(&self) {
        if let Some(scroll) = self.scroll() {
            if !self.auto_scroll_direction.get() {
                scroll.line_up();
            } else {
                scroll.line_down();
            }
        }
    }

    fn on_selection_interaction_changed(&self) {
        if let Some(rows_presenter) = self.rows_presenter() {
            let selection = self.selection_interaction();
            rows_presenter.update_selection(selection.as_deref());
        }
    }

    fn on_source_property_changed(&self, name: &str) {
        if name == "Selection" || name == "SelectionInteraction" {
            self.set_selection_interaction(self.presentation().and_then(|p| p.selection_interaction()));
            if let Some(rows_presenter) = self.rows_presenter() {
                let selection = self.selection_interaction();
                rows_presenter.update_selection(selection.as_deref());
            }
            self.apply_selection_mode();
        } else if name == "Rows" {
            self.set_rows(self.presentation().map(|p| p.rows()));
            self.recycle_rows();
        } else if name == "Columns" {
            self.set_columns(self.presentation().map(|p| p.columns()));
            self.recycle_rows();
        }
    }

    fn on_source_sorted(&self) {
        self.recycle_rows();
    }

    fn recycle_rows(&self) {
        if let Some(rows_presenter) = self.rows_presenter() {
            rows_presenter.recycle_all_elements();
            rows_presenter.invalidate_measure();
        }
    }

    fn get_drop_position(
        source: &dyn crate::presentation::TreeDataGridPresentation,
        e: &DragEventArgs,
        row: &TreeDataGridRow,
    ) -> TreeDataGridRowDropPosition {
        let row_y = e.get_position(row).y / row.bounds().height;

        if source.is_hierarchical() {
            if row_y < 0.33 {
                TreeDataGridRowDropPosition::Before
            } else if row_y > 0.66 {
                TreeDataGridRowDropPosition::After
            } else {
                TreeDataGridRowDropPosition::Inside
            }
        } else if row_y < 0.5 {
            TreeDataGridRowDropPosition::Before
        } else {
            TreeDataGridRowDropPosition::After
        }
    }

    fn detach_scroll_viewers(&self) {
        if let Some(subscriptions) = self.scroll_subscriptions.borrow_mut().take() {
            subscriptions.scroll.remove_handler(ScrollViewer::scroll_changed_event(), subscriptions.scroll_token);
            subscriptions
                .header_scroll
                .remove_handler(ScrollViewer::scroll_changed_event(), subscriptions.header_scroll_token);
        }
    }
}

impl TreeDataGridImpl for TreeDataGrid {
    fn create_default_element_factory(_this: &Self) -> Rc<TreeDataGridElementFactory> {
        TreeDataGridElementFactory::new()
    }
}

impl FerroObjectImpl for TreeDataGrid {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);

        let weak = this.to_ref().downgrade();
        let _ = this.add_handler(Button::click_event(), move |_, e: &RoutedEventArgs| {
            if let Some(this) = weak.upgrade() {
                this.on_click(e);
            }
        });
        let weak = this.to_ref().downgrade();
        let _ = this.add_handler_with(
            InputElement::key_down_event(),
            move |_, e: &KeyEventArgs| {
                if let Some(this) = weak.upgrade() {
                    this.on_preview_key_down(e);
                }
            },
            RoutingStrategies::TUNNEL,
            false,
        );
        this.initialize_v12_support();
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        let property = change.property();

        if property == Self::model_property().as_property() || property == Self::presentation_options_property().as_property() {
            this.update_core_presentation();
        }

        if property == Self::auto_drag_drop_rows_property().as_property() {
            DragDrop::set_allow_drop(this, change.get_new_value::<bool>());
        } else if property == Self::items_source_property().as_property() {
            this.rebuild_generated_source();
        } else if property == Self::selection_mode_property().as_property() {
            this.apply_selection_mode();
        }
    }
}

impl VisualImpl for TreeDataGrid {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);
        this.is_attached_to_visual_tree.set(true);
        if this.model().is_some() && this.presentation().is_none() {
            this.update_core_presentation();
        }
        if let Some(presentation) = this.presentation() {
            presentation.resume();
        }
        this.set_selection_interaction(this.presentation().and_then(|p| p.selection_interaction()));
        this.subscribe_source_events();
        this.subscribe_selection_interaction();
        if let Some(rows_presenter) = this.rows_presenter() {
            let selection = this.selection_interaction();
            rows_presenter.update_selection(selection.as_deref());
            rows_presenter.recycle_all_elements();
            rows_presenter.invalidate_measure();
        }
    }

    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        this.unsubscribe_source_events();
        this.unsubscribe_selection_interaction();
        this.is_attached_to_visual_tree.set(false);
        Self::parent_on_detached_from_visual_tree(this, e);
        this.stop_drag();
        this.release_core_presentation();
    }
}

impl TemplatedControlImpl for TreeDataGrid {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        let old_headers = this.column_headers_presenter.borrow_mut().take();
        if let Some(old_headers) = old_headers {
            this.logical_children().remove(&old_headers.upcast::<StyledElement>());
        }

        let old_rows = this.rows_presenter.borrow_mut().take();
        if let Some(old_rows) = old_rows {
            this.logical_children().remove(&old_rows.upcast::<StyledElement>());
        }

        this.detach_scroll_viewers();

        Self::parent_on_apply_template(this, e);

        let headers = e.name_scope().find_as::<TreeDataGridColumnHeadersPresenter>("PART_ColumnHeadersPresenter");
        let rows = e.name_scope().find_as::<TreeDataGridRowsPresenter>("PART_RowsPresenter");
        *this.column_headers_presenter.borrow_mut() = headers.clone();
        *this.rows_presenter.borrow_mut() = rows.clone();

        if let Some(headers) = headers {
            this.logical_children().add(headers.upcast::<StyledElement>());
        }

        if let Some(rows) = rows {
            this.logical_children().add(rows.upcast::<StyledElement>());
        }

        this.set_scroll(e.name_scope().find_as::<ScrollViewer>("PART_ScrollViewer"));
        *this.header_scroll.borrow_mut() = e.name_scope().find_as::<ScrollViewer>("PART_HeaderScrollViewer");

        if let (Some(scroll), Some(header_scroll)) = (this.scroll(), this.header_scroll.borrow().clone()) {
            let weak = this.to_ref().downgrade();
            let scroll_token = scroll.scroll_changed(move |_, e: &ScrollChangedEventArgs| {
                if let Some(this) = weak.upgrade() {
                    this.on_scroll_changed(e);
                }
            });
            let weak = this.to_ref().downgrade();
            let header_scroll_token = header_scroll.scroll_changed(move |_, e: &ScrollChangedEventArgs| {
                if let Some(this) = weak.upgrade() {
                    this.on_header_scroll_changed(e);
                }
            });
            *this.scroll_subscriptions.borrow_mut() =
                Some(ScrollSubscriptions { scroll, scroll_token, header_scroll, header_scroll_token });
        }
    }
}

impl InputElementImpl for TreeDataGrid {
    fn on_key_down(this: &Self, e: &KeyEventArgs) {
        Self::parent_on_key_down(this, e);
        if let Some(selection) = this.selection_interaction() {
            selection.on_key_down(this, e);
        }
    }

    fn on_key_up(this: &Self, e: &KeyEventArgs) {
        Self::parent_on_key_up(this, e);
        if let Some(selection) = this.selection_interaction() {
            selection.on_key_up(this, e);
        }
    }

    fn on_text_input(this: &Self, e: &TextInputEventArgs) {
        Self::parent_on_text_input(this, e);

        if e.text.as_deref().and_then(|text| text.chars().next()).is_some_and(char::is_control) {
            return;
        }

        if let Some(selection) = this.selection_interaction() {
            selection.on_text_input(this, e);
        }
    }

    fn on_pointer_pressed(this: &Self, e: &PointerPressedEventArgs) {
        Self::parent_on_pointer_pressed(this, e);
        if let Some(selection) = this.selection_interaction() {
            selection.on_pointer_pressed(this, e);
        }
    }

    fn on_pointer_moved(this: &Self, e: &PointerEventArgs) {
        Self::parent_on_pointer_moved(this, e);
        if let Some(selection) = this.selection_interaction() {
            selection.on_pointer_moved(this, e);
        }
    }

    fn on_pointer_released(this: &Self, e: &PointerReleasedEventArgs) {
        Self::parent_on_pointer_released(this, e);
        if let Some(selection) = this.selection_interaction() {
            selection.on_pointer_released(this, e);
        }
    }
}

impl ControlImpl for TreeDataGrid {
    fn on_create_automation_peer(this: &Self) -> Ref<AutomationPeer> {
        TreeDataGridAutomationPeer::new(this).upcast()
    }
}

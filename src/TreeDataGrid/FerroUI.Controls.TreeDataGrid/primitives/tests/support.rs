//! The helpers of the ported primitives tests: `TestWindow.cs`, `TestTemplates.cs`,
//! `TestElementFactory.cs`, `LayoutTestColumn.cs`, `LayoutTestCell.cs` and
//! `LayoutTestCellControl.cs` of the reference's tests, and the assertions the tests share.
//!
//! The reference's tests run on the headless platform with a window whose template is a content
//! presenter; here they run in the unit test application of the framework, with the services of a
//! styled window (whose test theme gives windows that template) and of keyboard focus.

#![allow(dead_code)]

use crate::models::{
    BeginEditGestures, CellRef, ColumnListBase, ColumnOptions, ColumnRef, IColumn, ICell, ICellColumn,
    IColumnOptions, IEditableObject, ITextCell, IUpdateColumnLayout, TextCell,
};
use crate::presentation::TreeDataGridRows;
use crate::primitives::{
    TreeDataGridCellImpl, TreeDataGridCellsPresenter, TreeDataGridColumnHeadersPresenter, TreeDataGridElementFactory,
    TreeDataGridElementFactoryImpl, TreeDataGridExpanderCell, TreeDataGridRow, TreeDataGridRowsPresenter,
    TreeDataGridTemplateCell, TreeDataGridTextCell,
};
use crate::TreeDataGrid;
use ferroui_base::collections::FerroList;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::data::model::{Event, INotifyPropertyChanged};
use ferroui_base::data::{BindingMode, TemplateBinding};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{LayoutableImpl, Orientation};
use ferroui_base::reactive::IDisposable;
use ferroui_base::threading::Dispatcher;
use ferroui_base::{
    ferro_class, instantiate, BoxedValue, FerroObjectImpl, FerroProperty, Ref, Size, StyledElementImpl, VisualImpl,
};
use ferroui_controls::presenters::{ContentPresenter, ScrollContentPresenter};
use ferroui_controls::primitives::{ScrollBar, TemplatedControlImpl, ToggleButton};
use ferroui_controls::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions, IControlTemplate};
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_controls::{
    ColumnDefinitions, Control, ControlImpl, Decorator, Dock, DockPanel, Grid, GridLength, RowDefinitions,
    ScrollViewer, Window,
};
use ferroui_treedatagrid_core::models::{AnonymousSortableRows, IRows, RowRef};
use ferroui_treedatagrid_core::{TreeDataGridItemsSourceView, TreeModel};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

// ---------------------------------------------------------------------------------------------
// Application, window and dispatcher
// ---------------------------------------------------------------------------------------------

/// Starts the unit test application of a test: the services of a styled window (windows, the test
/// theme, text) and of keyboard focus.
pub fn start_app() -> UnitTestApplicationScope {
    let mut services = TestServices::styled_window();
    let focus = TestServices::focusable_window();
    services.keyboard_device = focus.keyboard_device;
    services.keyboard_navigation = focus.keyboard_navigation;
    services.input_manager = focus.input_manager;
    UnitTestApplication::start(services)
}

/// `Dispatcher.UIThread.RunJobs()`.
pub fn run_jobs() {
    Dispatcher::ui_thread().run_jobs(None);
}

/// `new TestWindow(child, clientSize)`: a shown window of the given size (100 x 100 by default)
/// whose content is `child`.
pub fn test_window(child: Option<Ref<Control>>, client_size: Option<Size>) -> Ref<Window> {
    let window = Window::new();
    let size = client_size.unwrap_or(Size::new(100.0, 100.0));
    window.set_width(size.width);
    window.set_height(size.height);
    window.show();
    // Running the jobs applies the template and updates the client size.
    run_jobs();

    if let Some(child) = child {
        window.set_content(Some(Control::boxed(child)));
    }

    window
}

/// The control content of a window, if it has one.
pub fn window_content(window: &Window) -> Option<Ref<Control>> {
    window.content().as_ref().and_then(Control::from_boxed)
}

/// `Layout(target)`: a layout pass of the control's layout root.
pub fn layout(target: &Control) {
    target.update_layout();
}

// ---------------------------------------------------------------------------------------------
// Templates (`TestTemplates.cs`)
// ---------------------------------------------------------------------------------------------

fn template_bind(target: &Control, target_property: &'static FerroProperty, source_property: &'static FerroProperty) {
    target.bind_binding(target_property, &*TemplateBinding::new(source_property));
}

fn template_bind_two_way(
    target: &Control,
    target_property: &'static FerroProperty,
    source_property: &'static FerroProperty,
) {
    let binding = TemplateBinding::new(source_property);
    binding.set_mode(BindingMode::TwoWay);
    target.bind_binding(target_property, &*binding);
}

/// `TestTemplates.ScrollViewerTemplate()`: the scroll content presenter and two scroll bars in a
/// grid.
pub fn scroll_viewer_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::for_type::<ScrollViewer>(|_, scope| {
        let grid = Grid::new();
        grid.set_column_definitions(ColumnDefinitions::parse("*,Auto").expect("column definitions"));
        grid.set_row_definitions(RowDefinitions::parse("*,Auto").expect("row definitions"));

        let presenter = ScrollContentPresenter::new();
        presenter.set_name(Some("PART_ContentPresenter".to_string()));
        let presenter = presenter.register_in_name_scope(&**scope);

        let horizontal_scroll_bar = ScrollBar::new();
        horizontal_scroll_bar.set_name(Some("horizontalScrollBar".to_string()));
        horizontal_scroll_bar.set_orientation(Orientation::Horizontal);
        template_bind(
            &horizontal_scroll_bar,
            ScrollBar::visibility_property().as_property(),
            ScrollViewer::horizontal_scroll_bar_visibility_property().as_property(),
        );
        Grid::set_row(&horizontal_scroll_bar, 1);
        let horizontal_scroll_bar = horizontal_scroll_bar.register_in_name_scope(&**scope);

        let vertical_scroll_bar = ScrollBar::new();
        vertical_scroll_bar.set_name(Some("verticalScrollBar".to_string()));
        vertical_scroll_bar.set_orientation(Orientation::Vertical);
        template_bind(
            &vertical_scroll_bar,
            ScrollBar::visibility_property().as_property(),
            ScrollViewer::vertical_scroll_bar_visibility_property().as_property(),
        );
        Grid::set_column(&vertical_scroll_bar, 1);
        let vertical_scroll_bar = vertical_scroll_bar.register_in_name_scope(&**scope);

        grid.children().add(presenter);
        grid.children().add(horizontal_scroll_bar);
        grid.children().add(vertical_scroll_bar);
        grid.upcast()
    })
}

/// A scroll viewer with the test template, showing `content`.
pub fn scroll_viewer(content: Ref<Control>) -> Ref<ScrollViewer> {
    let scroll = ScrollViewer::new();
    scroll.set_template(Some(scroll_viewer_template()));
    scroll.set_content(Some(Control::boxed(content)));
    scroll
}

/// `TestTemplates.TreeDataGridTemplate(headerPresenterSize)`: the column headers presenter in a
/// scroll viewer docked at the top and the rows presenter in a scroll viewer.
pub fn tree_data_grid_template(header_presenter_size: f64) -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::for_type::<TreeDataGrid>(move |_, scope| {
        let headers_presenter = TreeDataGridColumnHeadersPresenter::new();
        headers_presenter.set_name(Some("PART_ColumnHeadersPresenter".to_string()));
        template_bind(
            &headers_presenter,
            TreeDataGridColumnHeadersPresenter::element_factory_property().as_property(),
            TreeDataGrid::element_factory_property().as_property(),
        );
        template_bind(
            &headers_presenter,
            TreeDataGridColumnHeadersPresenter::items_property().as_property(),
            TreeDataGrid::columns_property().as_property(),
        );
        let headers_presenter = headers_presenter.register_in_name_scope(&**scope);

        let header_scroll_viewer = ScrollViewer::new();
        header_scroll_viewer.set_name(Some("PART_HeaderScrollViewer".to_string()));
        header_scroll_viewer.set_template(Some(scroll_viewer_template()));
        header_scroll_viewer.set_height(header_presenter_size);
        header_scroll_viewer.set_horizontal_scroll_bar_visibility(ferroui_controls::primitives::ScrollBarVisibility::Hidden);
        header_scroll_viewer.set_vertical_scroll_bar_visibility(ferroui_controls::primitives::ScrollBarVisibility::Disabled);
        DockPanel::set_dock(&header_scroll_viewer, Dock::Top);
        header_scroll_viewer.set_content(Some(Control::boxed(headers_presenter)));
        let header_scroll_viewer = header_scroll_viewer.register_in_name_scope(&**scope);

        let rows_presenter = TreeDataGridRowsPresenter::new();
        rows_presenter.set_name(Some("PART_RowsPresenter".to_string()));
        template_bind(
            &rows_presenter,
            TreeDataGridRowsPresenter::columns_property().as_property(),
            TreeDataGrid::columns_property().as_property(),
        );
        template_bind(
            &rows_presenter,
            TreeDataGridRowsPresenter::element_factory_property().as_property(),
            TreeDataGrid::element_factory_property().as_property(),
        );
        template_bind(
            &rows_presenter,
            TreeDataGridRowsPresenter::items_property().as_property(),
            TreeDataGrid::rows_property().as_property(),
        );
        let rows_presenter = rows_presenter.register_in_name_scope(&**scope);

        let scroll_viewer = ScrollViewer::new();
        scroll_viewer.set_name(Some("PART_ScrollViewer".to_string()));
        scroll_viewer.set_template(Some(scroll_viewer_template()));
        scroll_viewer.set_horizontal_scroll_bar_visibility(ferroui_controls::primitives::ScrollBarVisibility::Auto);
        scroll_viewer.set_content(Some(Control::boxed(rows_presenter)));
        let scroll_viewer = scroll_viewer.register_in_name_scope(&**scope);

        let dock_panel = DockPanel::new();
        dock_panel.children().add(header_scroll_viewer);
        dock_panel.children().add(scroll_viewer);
        dock_panel.upcast()
    })
}

/// `TestTemplates.TreeDataGridRowTemplate()`: the cells presenter of the row in a dock panel.
pub fn tree_data_grid_row_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::for_type::<TreeDataGridRow>(|_, scope| {
        let cells_presenter = TreeDataGridCellsPresenter::new();
        cells_presenter.set_name(Some("PART_CellsPresenter".to_string()));
        bind_cells_presenter(&cells_presenter);
        let cells_presenter = cells_presenter.register_in_name_scope(&**scope);

        let dock_panel = DockPanel::new();
        dock_panel.children().add(cells_presenter);
        dock_panel.upcast()
    })
}

/// Binds the element factory, columns and rows of a cells presenter to those of its row.
pub fn bind_cells_presenter(cells_presenter: &TreeDataGridCellsPresenter) {
    template_bind(
        cells_presenter,
        TreeDataGridCellsPresenter::element_factory_property().as_property(),
        TreeDataGridRow::element_factory_property().as_property(),
    );
    template_bind(
        cells_presenter,
        TreeDataGridCellsPresenter::items_property().as_property(),
        TreeDataGridRow::columns_property().as_property(),
    );
    template_bind(
        cells_presenter,
        TreeDataGridCellsPresenter::rows_property().as_property(),
        TreeDataGridRow::rows_property().as_property(),
    );
}

/// `TestTemplates.TreeDataGridExpanderCellTemplate()`: a toggle button bound to the expanded
/// state and the content decorator.
pub fn tree_data_grid_expander_cell_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::for_type::<TreeDataGridExpanderCell>(|_, scope| {
        let toggle_button = ToggleButton::new();
        template_bind_two_way(
            &toggle_button,
            ToggleButton::is_checked_property().as_property(),
            TreeDataGridExpanderCell::is_expanded_property().as_property(),
        );

        let content = Decorator::new();
        content.set_name(Some("PART_Content".to_string()));
        let content = content.register_in_name_scope(&**scope);

        let inner = DockPanel::new();
        inner.children().add(toggle_button);
        inner.children().add(content);

        let outer = DockPanel::new();
        outer.children().add(inner);
        outer.upcast()
    })
}

/// `TestTemplates.TreeDataGridTemplateCellTemplate()`: a content presenter of the content of the
/// cell with its content template.
pub fn tree_data_grid_template_cell_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::for_type::<TreeDataGridTemplateCell>(|_, _| {
        let presenter = ContentPresenter::new();
        presenter.set_name(Some("PART_ContentPresenter".to_string()));
        template_bind(
            &presenter,
            ContentPresenter::content_property().as_property(),
            TreeDataGridTemplateCell::content_property().as_property(),
        );
        template_bind(
            &presenter,
            ContentPresenter::content_template_property().as_property(),
            TreeDataGridTemplateCell::content_template_property().as_property(),
        );
        presenter.upcast()
    })
}

// ---------------------------------------------------------------------------------------------
// Rows
// ---------------------------------------------------------------------------------------------

/// The rows of a presentation over the anonymous rows of `items` (the reference's tests give the
/// presenters an `AnonymousSortableRows<TModel>`, which realizes the cells of its rows).
pub fn rows_of<M: TreeModel>(items: Rc<FerroList<Rc<M>>>) -> Rc<TreeDataGridRows> {
    let view = TreeDataGridItemsSourceView::new(items);
    let rows: Rc<dyn IRows> = AnonymousSortableRows::new(view, None);
    TreeDataGridRows::new(rows)
}

/// The rows of a presentation over the anonymous rows of a fixed array of models.
pub fn rows_of_array<M: TreeModel>(items: Vec<Rc<M>>) -> Rc<TreeDataGridRows> {
    let view = TreeDataGridItemsSourceView::new(Rc::new(items));
    let rows: Rc<dyn IRows> = AnonymousSortableRows::new(view, None);
    TreeDataGridRows::new(rows)
}

/// The rows of a presentation over no models (`TreeDataGridItemsSourceView<TModel>.Empty`).
pub fn empty_rows<M: TreeModel>() -> Rc<TreeDataGridRows> {
    let rows: Rc<dyn IRows> = AnonymousSortableRows::<M>::new(TreeDataGridItemsSourceView::empty(), None);
    TreeDataGridRows::new(rows)
}

// ---------------------------------------------------------------------------------------------
// LayoutTestCell, LayoutTestCellControl, TestElementFactory
// ---------------------------------------------------------------------------------------------

/// `LayoutTestCell`: a read-only text cell of a string (`TextCell<string>`).
pub struct LayoutTestCell {
    inner: Rc<TextCell<String>>,
}

impl LayoutTestCell {
    pub fn new(value: Option<String>) -> Rc<Self> {
        Rc::new(Self { inner: TextCell::from_value(value) })
    }

    /// The text cell the test cell is.
    pub fn text_cell(&self) -> &Rc<TextCell<String>> {
        &self.inner
    }
}

impl ICell for LayoutTestCell {
    fn can_edit(&self) -> bool {
        ICell::can_edit(&*self.inner)
    }

    fn edit_gestures(&self) -> BeginEditGestures {
        ICell::edit_gestures(&*self.inner)
    }

    fn value(&self) -> Option<BoxedValue> {
        ICell::value(&*self.inner)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_text_cell(&self) -> Option<&dyn ITextCell> {
        ICell::as_text_cell(&*self.inner)
    }

    fn as_notify_property_changed(&self) -> Option<&dyn INotifyPropertyChanged> {
        ICell::as_notify_property_changed(&*self.inner)
    }

    fn as_editable(&self) -> Option<&dyn IEditableObject> {
        ICell::as_editable(&*self.inner)
    }

    fn dispose(&self) {
        ICell::dispose(&*self.inner);
    }
}

/// `LayoutTestCellControl`: a text cell control that records the constraints it is measured
/// with and desires 10 x 10.
#[repr(C)]
pub struct LayoutTestCellControl {
    base: TreeDataGridTextCell,
    measure_constraints: RefCell<Vec<Size>>,
}

ferro_class!(LayoutTestCellControl: TreeDataGridTextCell);
ferroui_base::ferro_class_info!(LayoutTestCellControl { new: LayoutTestCellControl::new });
ferroui_base::ferro_impl_classes!(
    LayoutTestCellControl: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    TreeDataGridCellImpl
);

impl LayoutableImpl for LayoutTestCellControl {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        this.measure_constraints.borrow_mut().push(available_size);
        Size::new(10.0, 10.0)
    }
}

impl LayoutTestCellControl {
    pub fn construct() -> Self {
        Self { base: TreeDataGridTextCell::construct(), measure_constraints: RefCell::new(Vec::new()) }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The constraints the control was measured with, in order.
    pub fn measure_constraints(&self) -> Vec<Size> {
        self.measure_constraints.borrow().clone()
    }
}

/// Whether an element factory data is a [`LayoutTestCell`].
pub fn is_layout_test_cell(data: &dyn Any) -> bool {
    data.downcast_ref::<CellRef>().is_some_and(|cell| cell.as_any().is::<LayoutTestCell>())
}

/// `TestElementFactory`: creates a [`LayoutTestCellControl`] for a [`LayoutTestCell`].
pub struct TestElementFactory;

impl TreeDataGridElementFactoryImpl for TestElementFactory {
    fn create_element(&self, factory: &TreeDataGridElementFactory, data: &dyn Any) -> Ref<Control> {
        if is_layout_test_cell(data) {
            LayoutTestCellControl::new().upcast()
        } else {
            factory.default_create_element(data)
        }
    }

    fn get_data_recycle_key(&self, factory: &TreeDataGridElementFactory, data: &dyn Any) -> String {
        if is_layout_test_cell(data) {
            LayoutTestCellControl::TYPE.full_name()
        } else {
            factory.default_data_recycle_key(data)
        }
    }
}

/// A new `TestElementFactory`.
pub fn test_element_factory() -> Rc<TreeDataGridElementFactory> {
    TreeDataGridElementFactory::with_impl(TestElementFactory)
}

// ---------------------------------------------------------------------------------------------
// LayoutTestColumn
// ---------------------------------------------------------------------------------------------

/// Creates the cell of a row for a [`LayoutTestColumn`] in place of the default.
pub type CreateCellOverride = Box<dyn Fn(&LayoutTestColumnState, &RowRef) -> CellRef>;

/// Retargets a cell for a [`LayoutTestColumn`] (`IReusableCellColumn<TModel>.TryReuseCell`).
pub type TryReuseCellOverride = Box<dyn Fn(&CellRef, &RowRef) -> bool>;

/// What a column override can read of its column.
pub struct LayoutTestColumnState {
    header: Option<BoxedValue>,
}

impl LayoutTestColumnState {
    /// The cell the column creates by default: a [`LayoutTestCell`] of `"{header} Row {index}"`.
    pub fn default_cell(&self, row: &RowRef) -> CellRef {
        let header = ValueTypes::to_display_string(self.header.as_ref());
        let model_index = row.as_model_indexable().map_or(-1, |row| row.model_index());
        LayoutTestCell::new(Some(format!("{header} Row {model_index}")))
    }
}

/// `LayoutTestColumn<TModel>`: a column of [`LayoutTestCell`]s whose default minimum width is 0.
pub struct LayoutTestColumn<M: TreeModel> {
    weak_self: Weak<Self>,
    base: crate::models::ColumnBase<M>,
    state: LayoutTestColumnState,
    create_cell: Option<CreateCellOverride>,
    try_reuse_cell: Option<TryReuseCellOverride>,
}

impl<M: TreeModel> LayoutTestColumn<M> {
    /// A column with `header`, `width` (`None`: `Auto`) and `options` (`None`: a minimum width of
    /// 0 pixels).
    pub fn new(header: &str, width: Option<GridLength>, options: Option<ColumnOptions<M>>) -> Rc<Self> {
        Self::with_overrides(header, width, options, None, None)
    }

    /// A column whose cell creation and reuse are those of a derived test column.
    pub fn with_overrides(
        header: &str,
        width: Option<GridLength>,
        options: Option<ColumnOptions<M>>,
        create_cell: Option<CreateCellOverride>,
        try_reuse_cell: Option<TryReuseCellOverride>,
    ) -> Rc<Self> {
        let options = options.unwrap_or_else(Self::default_options);
        let header: BoxedValue = Rc::new(header.to_string());
        let options: Rc<dyn IColumnOptions<M>> = Rc::new(options);
        Rc::new_cyclic(|weak_self| Self {
            weak_self: weak_self.clone(),
            base: crate::models::ColumnBase::new(Some(header.clone()), width, options),
            state: LayoutTestColumnState { header: Some(header) },
            create_cell,
            try_reuse_cell,
        })
    }

    fn default_options() -> ColumnOptions<M> {
        let options = ColumnOptions::new();
        options.set_min_width(GridLength::new(0.0, ferroui_controls::GridUnitType::Pixel));
        options
    }

    /// The column as a view column.
    pub fn as_column_ref(self: &Rc<Self>) -> ColumnRef {
        self.clone()
    }
}

impl<M: TreeModel> INotifyPropertyChanged for LayoutTestColumn<M> {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}

impl<M: TreeModel> IColumn for LayoutTestColumn<M> {
    fn actual_width(&self) -> f64 {
        self.base.actual_width()
    }

    fn can_user_resize(&self) -> Option<bool> {
        self.base.can_user_resize()
    }

    fn header(&self) -> Option<BoxedValue> {
        self.base.header()
    }

    fn width(&self) -> GridLength {
        self.base.width()
    }

    fn sort_direction(&self) -> Option<ferroui_treedatagrid_core::models::ListSortDirection> {
        self.base.sort_direction()
    }

    fn set_sort_direction(&self, value: Option<ferroui_treedatagrid_core::models::ListSortDirection>) {
        self.base.set_sort_direction(value)
    }

    fn tag(&self) -> Option<BoxedValue> {
        self.base.tag()
    }

    fn set_tag(&self, value: Option<BoxedValue>) {
        self.base.set_tag(value)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_update_column_layout(&self) -> Option<&dyn IUpdateColumnLayout> {
        Some(self)
    }

    fn as_cell_column(&self) -> Option<&dyn ICellColumn> {
        Some(self)
    }

    fn requires_unconstrained_width_measurement(&self) -> bool {
        self.base.requires_unconstrained_width_measurement()
    }
}

impl<M: TreeModel> IUpdateColumnLayout for LayoutTestColumn<M> {
    fn min_actual_width(&self) -> f64 {
        self.base.min_actual_width()
    }

    fn max_actual_width(&self) -> f64 {
        self.base.max_actual_width()
    }

    fn star_width_was_constrained(&self) -> bool {
        self.base.star_width_was_constrained()
    }

    fn cell_measured(&self, width: f64, row_index: i32) -> f64 {
        self.base.cell_measured(width, row_index)
    }

    fn calculate_star_width(&self, available_width: f64, total_stars: f64) {
        self.base.calculate_star_width(available_width, total_stars)
    }

    fn commit_actual_width(&self) -> bool {
        self.base.commit_actual_width()
    }

    fn set_width(&self, width: GridLength) {
        self.base.set_width(width)
    }
}

impl<M: TreeModel> ICellColumn for LayoutTestColumn<M> {
    fn create_cell(&self, row: &RowRef) -> CellRef {
        match &self.create_cell {
            Some(create_cell) => create_cell(&self.state, row),
            None => self.state.default_cell(row),
        }
    }

    fn try_reuse_cell(&self, cell: &CellRef, row: &RowRef) -> bool {
        match &self.try_reuse_cell {
            Some(try_reuse_cell) => try_reuse_cell(cell, row),
            None => false,
        }
    }
}

/// A column list (`ColumnList<TModel>`) of `columns`.
pub fn column_list(columns: impl IntoIterator<Item = ColumnRef>) -> Rc<ColumnListBase> {
    let list = ColumnListBase::new();
    for column in columns {
        list.add(column);
    }
    list
}

/// `count` layout test columns `"Column {i}"`, with the default width (`Auto`).
pub fn layout_test_columns<M: TreeModel>(count: usize) -> Rc<ColumnListBase> {
    column_list((0..count).map(|i| LayoutTestColumn::<M>::new(&format!("Column {i}"), None, None) as ColumnRef))
}

// ---------------------------------------------------------------------------------------------
// Tree attachment counters
// ---------------------------------------------------------------------------------------------

/// Counts the logical and visual tree attachments and detachments of controls
/// (`AttachedToLogicalTree += ...` and so on).
#[derive(Default)]
pub struct TreeAttachmentCounter {
    pub logical_attaches: Rc<Cell<i32>>,
    pub logical_detaches: Rc<Cell<i32>>,
    pub visual_attaches: Rc<Cell<i32>>,
    pub visual_detaches: Rc<Cell<i32>>,
    subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
}

impl TreeAttachmentCounter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Counts the attachments of `control`.
    pub fn watch(&self, control: &Control) {
        let mut subscriptions = self.subscriptions.borrow_mut();
        let counter = self.logical_attaches.clone();
        subscriptions.push(control.attached_to_logical_tree(move |_| counter.set(counter.get() + 1)));
        let counter = self.logical_detaches.clone();
        subscriptions.push(control.detached_from_logical_tree(move |_| counter.set(counter.get() + 1)));
        let counter = self.visual_attaches.clone();
        subscriptions.push(control.attached_to_visual_tree(move |_| counter.set(counter.get() + 1)));
        let counter = self.visual_detaches.clone();
        subscriptions.push(control.detached_from_visual_tree(move |_| counter.set(counter.get() + 1)));
    }

    /// Asserts that no control was attached or detached.
    pub fn assert_none(&self) {
        assert_eq!(0, self.logical_attaches.get(), "logical attaches");
        assert_eq!(0, self.logical_detaches.get(), "logical detaches");
        assert_eq!(0, self.visual_attaches.get(), "visual attaches");
        assert_eq!(0, self.visual_detaches.get(), "visual detaches");
    }
}

// ---------------------------------------------------------------------------------------------
// Assertions and helpers
// ---------------------------------------------------------------------------------------------

/// `Assert.Throws<T>(() => ...)`: the reference's exceptions are panics.
pub fn assert_panics(f: impl FnOnce()) {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
    assert!(result.is_err(), "expected the action to panic");
}

/// Whether a model and an untyped value are the same object (`Assert.Same(item, x.DataContext)`).
pub fn same_model<M: ?Sized>(model: &Rc<M>, value: Option<&BoxedValue>) -> bool {
    value.is_some_and(|value| std::ptr::addr_eq(Rc::as_ptr(model), Rc::as_ptr(value)))
}

/// Whether two optional shared values are the same object.
pub fn same_value(a: Option<&BoxedValue>, b: Option<&BoxedValue>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => std::ptr::addr_eq(Rc::as_ptr(a), Rc::as_ptr(b)),
        (None, None) => true,
        _ => false,
    }
}

/// The model of an untyped value (`(Model)x.DataContext!`).
pub fn model_of<M: 'static>(value: Option<BoxedValue>) -> Rc<M> {
    let value = value.expect("a model");
    let any: Rc<dyn Any> = value;
    any.downcast::<M>().unwrap_or_else(|_| panic!("a model of type {}", std::any::type_name::<M>()))
}

/// The visual children of a control that are of class `T`.
pub fn visual_children_of<T: ferroui_base::ObjectType>(control: &Control) -> Vec<Ref<T>> {
    control.get_visual_children().iter().filter_map(|child| child.cast::<T>()).collect()
}

/// The logical children of a control that are of class `T`.
pub fn logical_children_of<T: ferroui_base::ObjectType>(control: &Control) -> Vec<Ref<T>> {
    control.get_logical_children().iter().filter_map(|child| child.cast::<T>()).collect()
}

/// Whether the visual children of `control` contain `child`.
pub fn has_visual_child(control: &Control, child: &Control) -> bool {
    let child = child.to_ref();
    control.get_visual_children().iter().any(|visual| visual.ptr_eq(&child))
}

/// Whether the logical children of `control` contain `child`.
pub fn has_logical_child(control: &Control, child: &Control) -> bool {
    let child = child.to_ref();
    control.get_logical_children().iter().any(|element| element.ptr_eq(&child))
}

/// The realized elements of a presenter that are rows, in order, the empty slots left out.
pub fn realized_rows(elements: &[Option<Ref<Control>>]) -> Vec<Ref<TreeDataGridRow>> {
    elements.iter().flatten().map(|element| element.cast::<TreeDataGridRow>().expect("a row")).collect()
}

/// The sequence of the reference's `new Random(seed)`: the subtractive generator that a seeded
/// `System.Random` keeps for compatibility, so the tests see the reference's numbers.
pub struct TestRandom {
    seeds: [i32; 56],
    next: usize,
    next_p: usize,
}

impl TestRandom {
    const MBIG: i32 = i32::MAX;
    const MSEED: i32 = 161_803_398;

    pub fn new(seed: i32) -> Self {
        let mut seeds = [0i32; 56];
        let subtraction = if seed == i32::MIN { i32::MAX } else { seed.abs() };
        let mut mj = Self::MSEED - subtraction;
        seeds[55] = mj;
        let mut mk = 1i32;
        let mut ii = 0usize;
        for _ in 1..55 {
            ii += 21;
            if ii >= 55 {
                ii -= 55;
            }
            seeds[ii] = mk;
            mk = mj - mk;
            if mk < 0 {
                mk += Self::MBIG;
            }
            mj = seeds[ii];
        }
        for _ in 1..5 {
            for i in 1..56 {
                let mut n = i + 30;
                if n >= 55 {
                    n -= 55;
                }
                seeds[i] = seeds[i].wrapping_sub(seeds[1 + n]);
                if seeds[i] < 0 {
                    seeds[i] += Self::MBIG;
                }
            }
        }
        Self { seeds, next: 0, next_p: 21 }
    }

    fn internal_sample(&mut self) -> i32 {
        let mut next = self.next + 1;
        if next >= 56 {
            next = 1;
        }
        let mut next_p = self.next_p + 1;
        if next_p >= 56 {
            next_p = 1;
        }
        let mut value = self.seeds[next] - self.seeds[next_p];
        if value == Self::MBIG {
            value -= 1;
        }
        if value < 0 {
            value += Self::MBIG;
        }
        self.seeds[next] = value;
        self.next = next;
        self.next_p = next_p;
        value
    }

    /// A number in `0..max` (`Next(maxValue)`).
    pub fn next(&mut self, max: u32) -> u32 {
        let sample = f64::from(self.internal_sample()) * (1.0 / f64::from(Self::MBIG));
        (sample * f64::from(max)) as u32
    }
}

/// Whether a background brush is the solid red brush of the `NthChild` tests.
pub fn is_red(brush: Option<Rc<dyn ferroui_base::media::IBrush>>) -> bool {
    brush.is_some_and(|brush| {
        brush.as_solid_color_brush().is_some_and(|solid| solid.color() == ferroui_base::media::Colors::RED)
    })
}

/// A solid red brush.
pub fn red_brush() -> Option<Rc<dyn ferroui_base::media::IBrush>> {
    Some(ferroui_base::media::SolidColorBrush::with_color(ferroui_base::media::Colors::RED).into())
}

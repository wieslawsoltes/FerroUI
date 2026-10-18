//! `FirstLayoutBenchmarks.cs`: the first layout of a grid of a thousand rows and six text columns
//! (fixed, auto, star or mixed widths, with and without a cache of rows around the viewport),
//! with the columns' resolved widths as the constraint of the first measure of their cells,
//! against the natural width probe of columns that state no measurement options (the baseline).
//!
//! The cells and the column headers are synthetic: they desire four pixels a character plus
//! eight, 24 pixels high, so that the layout measured is the grid's and not the text's.
//!
//! The baseline's columns wrap the text columns in [`LegacyNaturalWidthColumn`], which forwards
//! the column and its layout but not the column's measurement options (the reference's
//! `IColumnMeasurementOptions`). The reference's presenters measure a column that has no such
//! options at its maximum width before its final measure; the port's columns state the options
//! with `IColumn::requires_unconstrained_width_measurement`, whose default is `false`, so the
//! wrapper answers `true`, which is what the presenters read for a column without the options.
//!
//! The reference's application is the headless platform without a theme; the port's is the unit
//! test application of the framework with the services of a styled window, whose test theme gives
//! windows their template. The reference runs thirty warm-up and thirty measured iterations; the
//! harness takes its counts from the environment.

use ferroui_base::data::model::{Event, INotifyPropertyChanged};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::data::TemplateBinding;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{Layoutable, LayoutableImpl};
use ferroui_base::styling::{Selectors, Setter, Style};
use ferroui_base::threading::Dispatcher;
use ferroui_base::{
    ferro_class, instantiate, BoxedValue, FerroObjectImpl, FerroProperty, Ref, Size, StyledElementImpl, VisualImpl,
};
use ferroui_benchmarks::harness::Registry;
use ferroui_controls::presenters::ScrollContentPresenter;
use ferroui_controls::primitives::{ScrollBarVisibility, TemplatedControl, TemplatedControlImpl};
use ferroui_controls::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions, IControlTemplate};
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_controls::{
    ButtonImpl, ContentControlImpl, Control, ControlImpl, Dock, DockPanel, GridLength, GridUnitType, ScrollViewer, Window,
};
use ferroui_controls_treedatagrid::models::{
    CellRef, ColumnRef, ICellColumn, IColumn, IUpdateColumnLayout, IViewColumn, TextColumn, ViewColumnHandle,
};
use ferroui_controls_treedatagrid::primitives::{
    TreeDataGridCellImpl, TreeDataGridCellsPresenter, TreeDataGridColumnHeader, TreeDataGridColumnHeadersPresenter,
    RecycleKey, TreeDataGridElementFactory, TreeDataGridElementFactoryImpl, TreeDataGridRow, TreeDataGridRowsPresenter,
    TreeDataGridTextCell,
};
use ferroui_controls_treedatagrid::{TreeDataGrid, TreeDataGridSourceRef};
use ferroui_treedatagrid_core::models::{
    self as core_models, ColumnKind, Comparison, IColumnOf, ListSortDirection, RowRef,
};
use ferroui_treedatagrid_core::{Accessor, FlatTreeDataGridSource, Models};
use ferroui_base::collections::FerroList;
use std::any::{Any, TypeId};
use std::rc::{Rc, Weak};

const FIRST_LAYOUT_OPERATIONS: u32 = 16;

/// `ColumnLayout`: the widths of the six columns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColumnLayout {
    Fixed,
    Auto,
    Star,
    Mixed,
}

/// `RowModel(int Id, string Title)`: a record, compared by value.
#[derive(PartialEq)]
struct RowModel {
    #[allow(dead_code)]
    id: i32,
    title: String,
}

/// A window of the benchmark and the grid it shows.
type GridWindow = (Ref<Window>, Ref<TreeDataGrid>);

pub struct FirstLayoutBenchmarks {
    layout: ColumnLayout,
    cache_length: f64,
    /// Set by the iteration setup of `resolved_width_constraint`.
    windows: Option<Vec<GridWindow>>,
    /// Set by the iteration setup of `legacy_natural_width_probe`.
    legacy_windows: Option<Vec<GridWindow>>,
    /// Disposed when the benchmark is dropped, after the windows (the fields drop in the order of
    /// their declaration).
    _app: UnitTestApplicationScope,
}

impl FirstLayoutBenchmarks {
    /// `[GlobalSetup] GlobalSetup`.
    pub fn new(layout: ColumnLayout, cache_length: f64) -> Self {
        let app = UnitTestApplication::start(TestServices::styled_window());
        Self { layout, cache_length, windows: None, legacy_windows: None, _app: app }
    }

    /// `[IterationSetup(Target = nameof(ResolvedWidthConstraint))] ResolvedIterationSetup`.
    pub fn resolved_iteration_setup(&mut self) {
        let windows = (0..FIRST_LAYOUT_OPERATIONS).map(|_| self.create_grid(false)).collect();
        self.windows = Some(windows);
    }

    /// `[IterationSetup(Target = nameof(LegacyNaturalWidthProbe))] LegacyIterationSetup`.
    pub fn legacy_iteration_setup(&mut self) {
        let windows = (0..FIRST_LAYOUT_OPERATIONS).map(|_| self.create_grid(true)).collect();
        self.legacy_windows = Some(windows);
    }

    /// `[IterationCleanup(Target = nameof(ResolvedWidthConstraint))] ResolvedIterationCleanup`.
    pub fn resolved_iteration_cleanup(&mut self) {
        if let Some(windows) = self.windows.take() {
            for (window, _) in &windows {
                window.close();
            }
        }

        Dispatcher::ui_thread().run_jobs(None);
    }

    /// `[IterationCleanup(Target = nameof(LegacyNaturalWidthProbe))] LegacyIterationCleanup`.
    pub fn legacy_iteration_cleanup(&mut self) {
        if let Some(windows) = self.legacy_windows.take() {
            for (window, _) in &windows {
                window.close();
            }
        }

        Dispatcher::ui_thread().run_jobs(None);
    }

    /// `[Benchmark(Baseline = true, OperationsPerInvoke = FirstLayoutOperations)] LegacyNaturalWidthProbe`.
    pub fn legacy_natural_width_probe(&self) -> i32 {
        Self::run_first_layout(self.legacy_windows.as_deref().expect("the iteration setup ran"))
    }

    /// `[Benchmark(OperationsPerInvoke = FirstLayoutOperations)] ResolvedWidthConstraint`.
    pub fn resolved_width_constraint(&self) -> i32 {
        Self::run_first_layout(self.windows.as_deref().expect("the iteration setup ran"))
    }

    fn run_first_layout(windows: &[GridWindow]) -> i32 {
        let mut checksum = 0;

        for (window, grid) in windows.iter().take(FIRST_LAYOUT_OPERATIONS as usize) {
            window.show();
            Dispatcher::ui_thread().run_jobs(None);
            window.update_layout();

            let columns = grid.columns().expect("the columns of the grid");
            checksum += grid.rows_presenter().expect("the rows presenter of the grid").get_realized_elements().len() as i32;

            for i in 0..columns.count() {
                checksum += columns.get(i).actual_width() as i32;
            }
        }

        checksum
    }

    fn create_grid(&self, use_legacy_natural_width_probe: bool) -> GridWindow {
        let items: Rc<FerroList<Rc<RowModel>>> = Rc::new(FerroList::from_items(
            (0..1_000).map(|x| Rc::new(RowModel { id: x, title: format!("Item {x}{}", "x".repeat((x % 20) as usize)) })),
        ));
        let source = FlatTreeDataGridSource::new(items.clone() as Models<RowModel>);

        for i in 0..6 {
            let column = i;
            let header: BoxedValue = Rc::new(format!("Column {column}"));
            let column_model = TextColumn::<RowModel, String>::new(
                Some(header),
                Accessor::new(move |x: &Rc<RowModel>| format!("{}-{column}", x.title)),
                Some(get_width(self.layout, column)),
                None,
            );
            if use_legacy_natural_width_probe {
                source.columns().add(LegacyNaturalWidthColumn::new(column_model) as Rc<dyn IColumnOf<RowModel>>);
            } else {
                source.columns().add(column_model as Rc<dyn IColumnOf<RowModel>>);
            }
        }

        let grid = TreeDataGrid::new();
        grid.set_element_factory(TreeDataGridElementFactory::with_impl(SyntheticElementFactory));
        grid.set_source(Some(TreeDataGridSourceRef::of::<RowModel, _>(source)));
        grid.set_template(Some(tree_data_grid_template(self.cache_length)));

        let window = Window::new();
        window.set_width(800.0);
        window.set_height(500.0);
        window.set_content(Some(Control::boxed(&grid)));
        let row_template: Option<Rc<dyn IControlTemplate>> = Some(row_template());
        window.styles().add(Style::with_setters(
            Selectors::of_type::<TreeDataGridRow>(),
            [
                Setter::new(TemplatedControl::template_property(), row_template),
                Setter::new(Layoutable::height_property(), 24.0),
            ],
        ));

        (window, grid)
    }
}

/// The windows of an iteration that did not end are closed with the benchmark.
impl Drop for FirstLayoutBenchmarks {
    fn drop(&mut self) {
        self.resolved_iteration_cleanup();
        self.legacy_iteration_cleanup();
    }
}

fn get_width(layout: ColumnLayout, index: usize) -> GridLength {
    match layout {
        ColumnLayout::Fixed => GridLength::new(124.0, GridUnitType::Pixel),
        ColumnLayout::Auto => GridLength::AUTO,
        ColumnLayout::Star => GridLength::new(1.0, GridUnitType::Star),
        ColumnLayout::Mixed => match index % 3 {
            0 => GridLength::AUTO,
            1 => GridLength::new(1.0, GridUnitType::Star),
            _ => GridLength::new(124.0, GridUnitType::Pixel),
        },
    }
}

// ---------------------------------------------------------------------------------------------
// Templates
// ---------------------------------------------------------------------------------------------

/// `[!target] = x[!source]`: a template binding of `target_property` to `source_property` of the
/// templated parent.
fn template_bind(target: &Control, target_property: &'static FerroProperty, source_property: &'static FerroProperty) {
    target.bind_binding(target_property, &*TemplateBinding::new(source_property));
}

/// `TreeDataGridTemplate(cacheLength)`: the column headers presenter in a scroll viewer 24
/// pixels high docked at the top, and the rows presenter in a scroll viewer.
fn tree_data_grid_template(cache_length: f64) -> Rc<dyn IControlTemplate> {
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
        header_scroll_viewer.set_height(24.0);
        header_scroll_viewer.set_horizontal_scroll_bar_visibility(ScrollBarVisibility::Hidden);
        header_scroll_viewer.set_vertical_scroll_bar_visibility(ScrollBarVisibility::Disabled);
        DockPanel::set_dock(&header_scroll_viewer, Dock::Top);
        header_scroll_viewer.set_content(Some(Control::boxed(headers_presenter)));
        let header_scroll_viewer = header_scroll_viewer.register_in_name_scope(&**scope);

        let rows_presenter = TreeDataGridRowsPresenter::new();
        rows_presenter.set_name(Some("PART_RowsPresenter".to_string()));
        rows_presenter.set_cache_length(cache_length);
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
        scroll_viewer.set_horizontal_scroll_bar_visibility(ScrollBarVisibility::Auto);
        scroll_viewer.set_vertical_scroll_bar_visibility(ScrollBarVisibility::Auto);
        scroll_viewer.set_content(Some(Control::boxed(rows_presenter)));
        let scroll_viewer = scroll_viewer.register_in_name_scope(&**scope);

        let dock_panel = DockPanel::new();
        dock_panel.children().add(header_scroll_viewer);
        dock_panel.children().add(scroll_viewer);
        dock_panel.upcast()
    })
}

/// `RowTemplate()`: the cells presenter of the row, bound to the row's element factory, columns
/// and rows.
fn row_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::for_type::<TreeDataGridRow>(|_, scope| {
        let cells_presenter = TreeDataGridCellsPresenter::new();
        cells_presenter.set_name(Some("PART_CellsPresenter".to_string()));
        template_bind(
            &cells_presenter,
            TreeDataGridCellsPresenter::element_factory_property().as_property(),
            TreeDataGridRow::element_factory_property().as_property(),
        );
        template_bind(
            &cells_presenter,
            TreeDataGridCellsPresenter::items_property().as_property(),
            TreeDataGridRow::columns_property().as_property(),
        );
        template_bind(
            &cells_presenter,
            TreeDataGridCellsPresenter::rows_property().as_property(),
            TreeDataGridRow::rows_property().as_property(),
        );
        cells_presenter.register_in_name_scope(&**scope).upcast()
    })
}

/// `ScrollViewerTemplate()`: a scroll content presenter alone. The reference binds the content
/// and the offset of the presenter to those of the scroll viewer in the template; the port's scroll
/// content presenter attaches itself to the scroll viewer it is in (its content and its offset),
/// so the template only names it.
fn scroll_viewer_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::for_type::<ScrollViewer>(|_, scope| {
        let presenter = ScrollContentPresenter::new();
        presenter.set_name(Some("PART_ContentPresenter".to_string()));
        presenter.register_in_name_scope(&**scope).upcast()
    })
}

// ---------------------------------------------------------------------------------------------
// The synthetic element factory, text cell and column header
// ---------------------------------------------------------------------------------------------

const SYNTHETIC_CELL: &str = "synthetic-cell";
const SYNTHETIC_HEADER: &str = "synthetic-header";

/// `SyntheticElementFactory`: synthetic text cells for the cells and synthetic headers for the
/// columns; the rows are the factory's own.
struct SyntheticElementFactory;

impl TreeDataGridElementFactoryImpl for SyntheticElementFactory {
    fn create_element(&self, factory: &TreeDataGridElementFactory, data: &dyn Any) -> Ref<Control> {
        if data.is::<CellRef>() {
            SyntheticTextCell::new().upcast()
        } else if data.is::<ColumnRef>() {
            SyntheticColumnHeader::new().upcast()
        } else {
            factory.default_create_element(data)
        }
    }

    fn get_data_recycle_key(&self, factory: &TreeDataGridElementFactory, data: &dyn Any) -> RecycleKey {
        if data.is::<CellRef>() {
            SYNTHETIC_CELL.into()
        } else if data.is::<ColumnRef>() {
            SYNTHETIC_HEADER.into()
        } else {
            factory.default_data_recycle_key(data)
        }
    }

    fn get_element_recycle_key(&self, factory: &TreeDataGridElementFactory, element: &Control) -> RecycleKey {
        if element.downcast_ref::<SyntheticTextCell>().is_some() {
            SYNTHETIC_CELL.into()
        } else if element.downcast_ref::<SyntheticColumnHeader>().is_some() {
            SYNTHETIC_HEADER.into()
        } else {
            factory.default_element_recycle_key(element)
        }
    }
}

/// `SyntheticTextCell`: a text cell four pixels a character plus eight wide (at most the width
/// it is given) and 24 pixels high.
#[repr(C)]
struct SyntheticTextCell {
    base: TreeDataGridTextCell,
}

ferro_class!(SyntheticTextCell: TreeDataGridTextCell);
ferroui_base::ferro_impl_classes!(
    SyntheticTextCell: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    TreeDataGridCellImpl
);

impl LayoutableImpl for SyntheticTextCell {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        let width = (this.value().map_or(0, |value| value.encode_utf16().count()) * 4 + 8) as f64;
        Size::new(width.min(available_size.width), 24.0)
    }
}

impl SyntheticTextCell {
    fn new() -> Ref<Self> {
        instantiate(Self { base: TreeDataGridTextCell::construct() })
    }
}

/// `SyntheticColumnHeader`: a column header four pixels a character of its header plus eight
/// wide (at most the width it is given) and 24 pixels high.
#[repr(C)]
struct SyntheticColumnHeader {
    base: TreeDataGridColumnHeader,
}

ferro_class!(SyntheticColumnHeader: TreeDataGridColumnHeader);
ferroui_base::ferro_impl_classes!(
    SyntheticColumnHeader: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl,
    ButtonImpl
);

impl LayoutableImpl for SyntheticColumnHeader {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        let header = this.header().map(|header| ValueTypes::to_display_string(Some(&header))).unwrap_or_default();
        let width = (header.encode_utf16().count() * 4 + 8) as f64;
        Size::new(width.min(available_size.width), 24.0)
    }
}

impl SyntheticColumnHeader {
    fn new() -> Ref<Self> {
        instantiate(Self { base: TreeDataGridColumnHeader::construct() })
    }
}

// ---------------------------------------------------------------------------------------------
// LegacyNaturalWidthColumn
// ---------------------------------------------------------------------------------------------

/// `LegacyNaturalWidthColumn`: a column that forwards a text column (`IColumn<RowModel>`) and its
/// layout (`IUpdateColumnLayout`), and nothing else of it: not its measurement options, so the
/// presenters probe its natural width with an unconstrained measure, and not the reuse of its
/// cells or its text search.
struct LegacyNaturalWidthColumn {
    weak_self: Weak<Self>,
    column: Rc<dyn IViewColumn<RowModel>>,
}

impl LegacyNaturalWidthColumn {
    fn new(column: Rc<dyn IViewColumn<RowModel>>) -> Rc<Self> {
        Rc::new_cyclic(|weak_self| Self { weak_self: weak_self.clone(), column })
    }
}

impl INotifyPropertyChanged for LegacyNaturalWidthColumn {
    fn property_changed(&self) -> &Event<str> {
        INotifyPropertyChanged::property_changed(&*self.column)
    }
}

impl IColumn for LegacyNaturalWidthColumn {
    fn actual_width(&self) -> f64 {
        IColumn::actual_width(&*self.column)
    }

    fn can_user_resize(&self) -> Option<bool> {
        IColumn::can_user_resize(&*self.column)
    }

    fn header(&self) -> Option<BoxedValue> {
        IColumn::header(&*self.column)
    }

    fn width(&self) -> GridLength {
        IColumn::width(&*self.column)
    }

    fn sort_direction(&self) -> Option<ListSortDirection> {
        IColumn::sort_direction(&*self.column)
    }

    fn set_sort_direction(&self, value: Option<ListSortDirection>) {
        IColumn::set_sort_direction(&*self.column, value)
    }

    fn tag(&self) -> Option<BoxedValue> {
        IColumn::tag(&*self.column)
    }

    fn set_tag(&self, value: Option<BoxedValue>) {
        IColumn::set_tag(&*self.column, value)
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

    /// The column states no measurement options (see the module documentation).
    fn requires_unconstrained_width_measurement(&self) -> bool {
        true
    }

    fn core_column(&self) -> Option<Rc<dyn core_models::IColumn>> {
        self.weak_self.upgrade().map(|column| column as Rc<dyn core_models::IColumn>)
    }
}

impl IUpdateColumnLayout for LegacyNaturalWidthColumn {
    fn min_actual_width(&self) -> f64 {
        IUpdateColumnLayout::min_actual_width(&*self.column)
    }

    fn max_actual_width(&self) -> f64 {
        IUpdateColumnLayout::max_actual_width(&*self.column)
    }

    fn star_width_was_constrained(&self) -> bool {
        IUpdateColumnLayout::star_width_was_constrained(&*self.column)
    }

    fn cell_measured(&self, width: f64, row_index: i32) -> f64 {
        IUpdateColumnLayout::cell_measured(&*self.column, width, row_index)
    }

    fn calculate_star_width(&self, available_width: f64, total_stars: f64) {
        IUpdateColumnLayout::calculate_star_width(&*self.column, available_width, total_stars)
    }

    fn commit_actual_width(&self) -> bool {
        IUpdateColumnLayout::commit_actual_width(&*self.column)
    }

    fn set_width(&self, width: GridLength) {
        IUpdateColumnLayout::set_width(&*self.column, width)
    }
}

impl ICellColumn for LegacyNaturalWidthColumn {
    fn create_cell(&self, row: &RowRef) -> CellRef {
        ICellColumn::create_cell(&*self.column, row)
    }
}

/// The wrapper is the column of the source too: the core column whose view is the wrapper.
impl core_models::IColumn for LegacyNaturalWidthColumn {
    fn id(&self) -> String {
        core_models::IColumn::id(&*self.column)
    }

    fn header(&self) -> Option<BoxedValue> {
        core_models::IColumn::header(&*self.column)
    }

    fn width(&self) -> ferroui_treedatagrid_core::GridLength {
        core_models::IColumn::width(&*self.column)
    }

    fn set_width(&self, value: ferroui_treedatagrid_core::GridLength) {
        core_models::IColumn::set_width(&*self.column, value)
    }

    fn is_visible(&self) -> bool {
        core_models::IColumn::is_visible(&*self.column)
    }

    fn set_is_visible(&self, value: bool) {
        core_models::IColumn::set_is_visible(&*self.column, value)
    }

    fn presentation_key(&self) -> Option<String> {
        core_models::IColumn::presentation_key(&*self.column)
    }

    fn set_presentation_key(&self, value: Option<String>) {
        core_models::IColumn::set_presentation_key(&*self.column, value)
    }

    fn sort_direction(&self) -> Option<ListSortDirection> {
        core_models::IColumn::sort_direction(&*self.column)
    }

    fn set_sort_direction(&self, value: Option<ListSortDirection>) {
        core_models::IColumn::set_sort_direction(&*self.column, value)
    }

    fn tag(&self) -> Option<BoxedValue> {
        core_models::IColumn::tag(&*self.column)
    }

    fn set_tag(&self, value: Option<BoxedValue>) {
        core_models::IColumn::set_tag(&*self.column, value)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn model_type(&self) -> TypeId {
        TypeId::of::<RowModel>()
    }

    fn view(&self) -> Option<Rc<dyn Any>> {
        let column = self.weak_self.upgrade()?;
        let view: ColumnRef = column;
        Some(Rc::new(ViewColumnHandle(view)) as Rc<dyn Any>)
    }
}

impl IColumnOf<RowModel> for LegacyNaturalWidthColumn {
    fn get_comparison(&self, direction: ListSortDirection) -> Option<Comparison<RowModel>> {
        IColumnOf::get_comparison(&*self.column, direction)
    }

    fn kind(&self) -> ColumnKind<'_, RowModel> {
        IColumnOf::kind(&*self.column)
    }
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("treedatagrid", "FirstLayoutBenchmarks");
    for layout in [ColumnLayout::Fixed, ColumnLayout::Auto, ColumnLayout::Star, ColumnLayout::Mixed] {
        for cache_length in [0.0, 0.5] {
            let parameters = format!("Layout={layout:?}, CacheLength={cache_length}");
            class
                .benchmark_with_iteration_setup(
                    "legacy_natural_width_probe",
                    parameters.clone(),
                    move || FirstLayoutBenchmarks::new(layout, cache_length),
                    // The cleanup of the previous iteration, then the setup of this one.
                    |b| {
                        b.legacy_iteration_cleanup();
                        b.legacy_iteration_setup();
                    },
                    |b| b.legacy_natural_width_probe(),
                )
                .baseline()
                .operations_per_invoke(FIRST_LAYOUT_OPERATIONS);
            class
                .benchmark_with_iteration_setup(
                    "resolved_width_constraint",
                    parameters,
                    move || FirstLayoutBenchmarks::new(layout, cache_length),
                    |b| {
                        b.resolved_iteration_cleanup();
                        b.resolved_iteration_setup();
                    },
                    |b| b.resolved_width_constraint(),
                )
                .operations_per_invoke(FIRST_LAYOUT_OPERATIONS);
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn first_layout_benchmarks() {
        ferroui_benchmarks::harness::smoke_class(super::register, "FirstLayoutBenchmarks");
    }
}

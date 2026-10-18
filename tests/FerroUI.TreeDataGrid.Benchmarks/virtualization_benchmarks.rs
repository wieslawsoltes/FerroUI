//! `VirtualizationBenchmarks.cs`: the layout of a virtualizing grid in a window of 800 x 500:
//! creating and laying it out, small, one-row, buffered and reversing vertical scrolls, forced
//! stationary layouts, small and column-sized horizontal scrolls over many columns (also near
//! column 900 of 1 000), insert/remove and move bursts inside the realized rows, and detaching
//! and reattaching the grid, each for the classic columns and for the framework-neutral columns
//! of the core (`NeutralSource`).
//!
//! The classic path is a core source holding the view columns of the control crate, shown
//! through the grid's `Source`; the neutral path is a core source holding the neutral columns of
//! the core, shown through the grid's `Model` (see `neutral_source_benchmarks.rs`).
//!
//! The unbatched scrolls use presenters that measure again for every change of their viewport.
//! The reference derives them from the presenters and overrides `NeedsMeasureForViewportChange`
//! to return `true`; the port's presenters decide that in their hooks
//! (`TreeDataGridPresenterHooks::needs_measure_for_viewport_change`), which a class derived from
//! a presenter cannot override (the presenters have no virtual for it). The derived presenters of
//! the port ([`AlwaysMeasureRowsPresenter`], [`AlwaysMeasureCellsPresenter`]) observe their
//! effective viewport themselves and invalidate their measure whenever it moves on their axis,
//! which is the measure the overridden decision asks for: the presenter's own handler invalidates
//! the measure in the same case when the decision is `true`.
//!
//! The reference's application is the headless platform without a theme; the port's is the unit
//! test application of the framework with the services of a styled window, whose test theme gives
//! windows their template. The reference's `[IterationCleanup]` of a benchmark runs here before
//! the next iteration setup and when the benchmark is dropped (the harness has no iteration
//! cleanup); it is outside the measurement either way.

use ferroui_base::collections::FerroList;
use ferroui_base::data::TemplateBinding;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{EffectiveViewportChangedEventArgs, Layoutable, LayoutableImpl};
use ferroui_base::reactive::IDisposable;
use ferroui_base::styling::{Selectors, Setter, Style};
use ferroui_base::threading::Dispatcher;
use ferroui_base::utilities::MathUtilities;
use ferroui_base::{
    ferro_class, instantiate, BoxedValue, FerroObjectImpl, FerroProperty, Rect, Ref, Size, StyledElementImpl, Vector,
    VisualImpl,
};
use ferroui_benchmarks::harness::Registry;
use ferroui_controls::presenters::ScrollContentPresenter;
use ferroui_controls::primitives::{ScrollBarVisibility, TemplatedControl};
use ferroui_controls::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions, IControlTemplate};
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_controls::{Control, ControlImpl, GridLength, GridUnitType, ScrollViewer, Window};
use ferroui_controls_treedatagrid::models::TextColumn;
use ferroui_controls_treedatagrid::primitives::{
    TreeDataGridCellsPresenter, TreeDataGridCellsPresenterImpl, TreeDataGridRow, TreeDataGridRowsPresenter,
};
use ferroui_controls_treedatagrid::{TreeDataGrid, TreeDataGridSourceRef};
use ferroui_treedatagrid_core::models::{IColumnOf, TextColumn as CoreTextColumn};
use ferroui_treedatagrid_core::{
    Accessor, FlatTreeDataGridSource, GridLength as CoreGridLength, GridUnitType as CoreGridUnitType, Models,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

pub const VERTICAL_SCROLL_OPERATIONS: u32 = 10_000;
pub const STATIONARY_LAYOUT_OPERATIONS: u32 = 25_000;
pub const HORIZONTAL_SCROLL_OPERATIONS: u32 = 6_000;
pub const HORIZONTAL_COLUMN_OPERATIONS: u32 = 1_000;
pub const COLLECTION_EDIT_OPERATIONS: u32 = 1_500;
pub const COLLECTION_MOVE_OPERATIONS: u32 = 6_000;
pub const DETACH_REATTACH_OPERATIONS: u32 = 200;

/// `RowModel(int Id, string Title)`: a record, compared by value.
#[derive(PartialEq)]
struct RowModel {
    id: i32,
    title: String,
}

impl RowModel {
    fn new(id: i32, title: String) -> Rc<Self> {
        Rc::new(Self { id, title })
    }
}

/// `[Params(false, true)] NeutralSource` and the state of the grid of an iteration.
pub struct VirtualizationBenchmarks {
    /// Whether the grid shows a source of neutral core columns through its `Model` (otherwise a
    /// source of the control's columns through its `Source`).
    pub neutral_source: bool,
    source_lifetime: Option<Rc<FlatTreeDataGridSource<RowModel>>>,
    window: Option<Ref<Window>>,
    grid: Option<Ref<TreeDataGrid>>,
    scroll: Option<Ref<ScrollViewer>>,
    items: Option<Rc<FerroList<Rc<RowModel>>>>,
    /// Disposed when the benchmark is dropped, after the grid (the fields drop in the order of
    /// their declaration).
    _app: UnitTestApplicationScope,
}

impl VirtualizationBenchmarks {
    /// `[GlobalSetup] GlobalSetup`: the application.
    pub fn new(neutral_source: bool) -> Self {
        let app = UnitTestApplication::start(TestServices::styled_window());
        Self { neutral_source, source_lifetime: None, window: None, grid: None, scroll: None, items: None, _app: app }
    }

    /// `[GlobalCleanup] GlobalCleanup`.
    pub fn global_cleanup(&mut self) {
        if let Some(window) = self.window.take() {
            window.close();
        }
        self.grid = None;
        self.scroll = None;
        self.items = None;
    }

    fn grid(&self) -> &Ref<TreeDataGrid> {
        self.grid.as_ref().expect("the iteration setup created the grid")
    }

    fn scroll(&self) -> &Ref<ScrollViewer> {
        self.scroll.as_ref().expect("the iteration setup created the grid")
    }

    fn items(&self) -> &Rc<FerroList<Rc<RowModel>>> {
        self.items.as_ref().expect("the iteration setup created the grid")
    }

    fn window(&self) -> &Ref<Window> {
        self.window.as_ref().expect("the iteration setup created the grid")
    }

    /// `_grid.RowsPresenter.GetRealizedElements().Count()`.
    fn realized_row_count(&self) -> usize {
        self.grid().rows_presenter().expect("the rows presenter of the grid").get_realized_elements().len()
    }

    /// `[Benchmark] CreateAndLayoutGrid`.
    pub fn create_and_layout_grid(&mut self) -> usize {
        self.create_grid(1_000, 12, 0.0, false, false);
        let count = self.realized_row_count();
        self.cleanup_grid();
        count
    }

    pub fn setup_vertical_small_scrolls(&mut self) {
        self.create_grid(10_000, 12, 0.0, false, false);
    }

    pub fn cleanup_vertical_small_scrolls(&mut self) {
        self.cleanup_grid();
    }

    /// `[Benchmark(OperationsPerInvoke = VerticalScrollOperations)] VerticalSmallScrolls`.
    pub fn vertical_small_scrolls(&self) -> usize {
        let scroll = self.scroll();

        for i in 1..=VERTICAL_SCROLL_OPERATIONS {
            scroll.set_offset(Vector::new(0.0, f64::from(i)));
            self.grid().update_layout();
        }

        self.realized_row_count()
    }

    pub fn setup_vertical_unbatched_small_scrolls(&mut self) {
        self.create_grid(10_000, 12, 0.0, true, false);
    }

    pub fn cleanup_vertical_unbatched_small_scrolls(&mut self) {
        self.cleanup_grid();
    }

    /// `[Benchmark(OperationsPerInvoke = VerticalScrollOperations)] VerticalUnbatchedSmallScrolls`.
    pub fn vertical_unbatched_small_scrolls(&self) -> usize {
        let scroll = self.scroll();

        for i in 1..=VERTICAL_SCROLL_OPERATIONS {
            scroll.set_offset(Vector::new(0.0, f64::from(i)));
            self.grid().update_layout();
        }

        self.realized_row_count()
    }

    pub fn setup_forced_stationary_vertical_layouts(&mut self) {
        self.create_grid(10_000, 12, 0.0, false, false);
    }

    pub fn cleanup_forced_stationary_vertical_layouts(&mut self) {
        self.cleanup_grid();
    }

    /// `[Benchmark(OperationsPerInvoke = StationaryLayoutOperations)] ForcedStationaryVerticalLayouts`.
    pub fn forced_stationary_vertical_layouts(&self) -> usize {
        let grid = self.grid();
        let presenter = grid.rows_presenter().expect("the rows presenter of the grid");

        for _ in 0..STATIONARY_LAYOUT_OPERATIONS {
            presenter.invalidate_measure();
            grid.update_layout();
        }

        presenter.get_realized_elements().len()
    }

    pub fn setup_vertical_row_scrolls(&mut self) {
        self.create_grid(10_000, 12, 0.0, false, false);
    }

    pub fn cleanup_vertical_row_scrolls(&mut self) {
        self.cleanup_grid();
    }

    /// `[Benchmark(OperationsPerInvoke = VerticalScrollOperations)] VerticalRowScrolls`.
    pub fn vertical_row_scrolls(&self) -> usize {
        let grid = self.grid();
        let presenter = grid.rows_presenter().expect("the rows presenter of the grid");
        let scroll = self.scroll();

        for i in 1..=VERTICAL_SCROLL_OPERATIONS {
            scroll.set_offset(Vector::new(0.0, f64::from(i * 24)));
            presenter.invalidate_measure();
            grid.update_layout();
        }

        presenter.get_realized_elements().len()
    }

    pub fn setup_vertical_buffered_small_scrolls(&mut self) {
        self.create_grid(10_000, 12, 0.1, false, false);
    }

    pub fn cleanup_vertical_buffered_small_scrolls(&mut self) {
        self.cleanup_grid();
    }

    /// `[Benchmark(OperationsPerInvoke = VerticalScrollOperations)] VerticalBufferedSmallScrolls`.
    pub fn vertical_buffered_small_scrolls(&self) -> usize {
        let scroll = self.scroll();

        for i in 1..=VERTICAL_SCROLL_OPERATIONS {
            scroll.set_offset(Vector::new(0.0, f64::from(i)));
            self.grid().update_layout();
        }

        self.realized_row_count()
    }

    pub fn setup_vertical_jitter_scrolls(&mut self) {
        self.create_grid(10_000, 12, 0.0, false, false);
    }

    pub fn cleanup_vertical_jitter_scrolls(&mut self) {
        self.cleanup_grid();
    }

    /// `[Benchmark(OperationsPerInvoke = VerticalScrollOperations)] VerticalJitterScrolls`.
    pub fn vertical_jitter_scrolls(&self) -> usize {
        let scroll = self.scroll();

        for i in 0..VERTICAL_SCROLL_OPERATIONS {
            scroll.set_offset(Vector::new(0.0, if (i & 1) == 0 { 100.0 } else { 0.0 }));
            self.grid().update_layout();
        }

        self.realized_row_count()
    }

    pub fn setup_vertical_buffered_jitter_scrolls(&mut self) {
        self.create_grid(10_000, 12, 0.1, false, false);
    }

    pub fn cleanup_vertical_buffered_jitter_scrolls(&mut self) {
        self.cleanup_grid();
    }

    /// `[Benchmark(OperationsPerInvoke = VerticalScrollOperations)] VerticalBufferedJitterScrolls`.
    pub fn vertical_buffered_jitter_scrolls(&self) -> usize {
        let scroll = self.scroll();

        for i in 0..VERTICAL_SCROLL_OPERATIONS {
            scroll.set_offset(Vector::new(0.0, if (i & 1) == 0 { 100.0 } else { 0.0 }));
            self.grid().update_layout();
        }

        self.realized_row_count()
    }

    pub fn setup_horizontal_small_scrolls(&mut self) {
        self.create_grid(1_000, 200, 0.0, false, false);
    }

    pub fn cleanup_horizontal_small_scrolls(&mut self) {
        self.cleanup_grid();
    }

    /// `[Benchmark(OperationsPerInvoke = HorizontalScrollOperations)] HorizontalSmallScrolls`.
    pub fn horizontal_small_scrolls(&self) -> usize {
        let scroll = self.scroll();

        for i in 1..=HORIZONTAL_SCROLL_OPERATIONS {
            scroll.set_offset(Vector::new(f64::from(i), 0.0));
            self.grid().update_layout();
        }

        self.realized_row_count()
    }

    pub fn setup_horizontal_unbatched_small_scrolls(&mut self) {
        self.create_grid(1_000, 200, 0.0, false, true);
    }

    pub fn cleanup_horizontal_unbatched_small_scrolls(&mut self) {
        self.cleanup_grid();
    }

    /// `[Benchmark(OperationsPerInvoke = HorizontalScrollOperations)] HorizontalUnbatchedSmallScrolls`.
    pub fn horizontal_unbatched_small_scrolls(&self) -> usize {
        let scroll = self.scroll();

        for i in 1..=HORIZONTAL_SCROLL_OPERATIONS {
            scroll.set_offset(Vector::new(f64::from(i), 0.0));
            self.grid().update_layout();
        }

        self.realized_row_count()
    }

    pub fn setup_horizontal_column_scrolls(&mut self) {
        self.create_grid(1_000, 200, 0.0, false, false);
    }

    pub fn cleanup_horizontal_column_scrolls(&mut self) {
        self.cleanup_grid();
    }

    /// `[Benchmark(OperationsPerInvoke = HorizontalColumnOperations)] HorizontalColumnScrolls`.
    pub fn horizontal_column_scrolls(&self) -> usize {
        let scroll = self.scroll();

        for i in 0..HORIZONTAL_COLUMN_OPERATIONS {
            scroll.set_offset(Vector::new(if (i & 1) == 0 { 100.0 } else { 200.0 }, 0.0));
            self.grid().update_layout();
        }

        self.realized_row_count()
    }

    pub fn setup_horizontal_far_column_scrolls(&mut self) {
        self.create_grid(1_000, 1_000, 0.0, false, false);
        self.scroll().set_offset(Vector::new(90_000.0, 0.0));
        self.grid().update_layout();
    }

    pub fn cleanup_horizontal_far_column_scrolls(&mut self) {
        self.cleanup_grid();
    }

    /// `[Benchmark(OperationsPerInvoke = HorizontalColumnOperations)] HorizontalFarColumnScrolls`.
    ///
    /// The initial jump is outside the timed region. Exercise the same recycling workload near
    /// column 900, where a scan from column zero is repeated by every realized row.
    pub fn horizontal_far_column_scrolls(&self) -> usize {
        let scroll = self.scroll();
        for i in 0..HORIZONTAL_COLUMN_OPERATIONS {
            scroll.set_offset(Vector::new(if (i & 1) == 0 { 90_100.0 } else { 90_000.0 }, 0.0));
            self.grid().update_layout();
        }
        self.realized_row_count()
    }

    pub fn setup_collection_insert_remove_burst(&mut self) {
        self.create_grid(10_000, 12, 0.0, false, false);
    }

    pub fn cleanup_collection_insert_remove_burst(&mut self) {
        self.cleanup_grid();
    }

    /// `[Benchmark(OperationsPerInvoke = CollectionEditOperations)] CollectionInsertRemoveBurst`.
    pub fn collection_insert_remove_burst(&self) -> usize {
        let items = self.items();

        for i in 0..COLLECTION_EDIT_OPERATIONS as i32 {
            items.insert(2, RowModel::new(-i, format!("Inserted {i}")));
            self.grid().update_layout();
            items.remove_at(2);
            self.grid().update_layout();
        }

        self.realized_row_count()
    }

    pub fn setup_collection_move_burst(&mut self) {
        self.create_grid(10_000, 12, 0.0, false, false);
    }

    pub fn cleanup_collection_move_burst(&mut self) {
        self.cleanup_grid();
    }

    /// `[Benchmark(OperationsPerInvoke = CollectionMoveOperations)] CollectionMoveBurst`.
    pub fn collection_move_burst(&self) -> usize {
        let items = self.items();

        for _ in 0..COLLECTION_MOVE_OPERATIONS {
            items.move_item(2, 12);
            self.grid().update_layout();
            items.move_item(12, 2);
            self.grid().update_layout();
        }

        self.realized_row_count()
    }

    pub fn setup_detach_reattach(&mut self) {
        self.create_grid(10_000, 50, 0.0, false, false);
    }

    pub fn cleanup_detach_reattach(&mut self) {
        self.cleanup_grid();
    }

    /// `[Benchmark(OperationsPerInvoke = DetachReattachOperations)] DetachReattach`.
    pub fn detach_reattach(&self) -> usize {
        let grid = self.grid();
        let window = self.window();

        for _ in 0..DETACH_REATTACH_OPERATIONS {
            window.set_content(None);
            window.update_layout();
            window.set_content(Some(Control::boxed(grid)));
            window.update_layout();
        }

        self.realized_row_count()
    }

    /// `CreateGrid`: a grid of `row_count` rows of an id column 80 pixels wide and
    /// `column_count - 1` text columns 100 pixels wide, in a shown window of 800 x 500 whose rows
    /// are 24 pixels high.
    fn create_grid(
        &mut self,
        row_count: i32,
        column_count: i32,
        cache_length: f64,
        always_measure_viewport_changes: bool,
        always_measure_column_viewport_changes: bool,
    ) {
        let items: Rc<FerroList<Rc<RowModel>>> =
            Rc::new(FerroList::from_items((0..row_count).map(|x| RowModel::new(x, format!("Item {x}")))));
        let source = FlatTreeDataGridSource::new(items.clone() as Models<RowModel>);

        if self.neutral_source {
            source.columns().add(CoreTextColumn::new(
                header("ID"),
                Accessor::member("Id", |x: &Rc<RowModel>| x.id),
                Some(CoreGridLength::new(80.0, CoreGridUnitType::Pixel)),
                None,
                None,
            ));
            for i in 1..column_count {
                let column = i;
                source.columns().add(CoreTextColumn::new(
                    header(&format!("Column {column}")),
                    Accessor::new(move |x: &Rc<RowModel>| format!("{}-{column}", x.title)),
                    Some(CoreGridLength::new(100.0, CoreGridUnitType::Pixel)),
                    None,
                    None,
                ));
            }
        } else {
            source.columns().add(TextColumn::<RowModel, i32>::new(
                header("ID"),
                Accessor::member("Id", |x: &Rc<RowModel>| x.id),
                Some(GridLength::new(80.0, GridUnitType::Pixel)),
                None,
            ) as Rc<dyn IColumnOf<RowModel>>);

            for i in 1..column_count {
                let column = i;
                source.columns().add(TextColumn::<RowModel, String>::new(
                    header(&format!("Column {column}")),
                    Accessor::new(move |x: &Rc<RowModel>| format!("{}-{column}", x.title)),
                    Some(GridLength::new(100.0, GridUnitType::Pixel)),
                    None,
                ) as Rc<dyn IColumnOf<RowModel>>);
            }
        }

        let grid = TreeDataGrid::new();
        let source_ref = Some(TreeDataGridSourceRef::of::<RowModel, _>(source.clone()));
        if self.neutral_source {
            grid.set_model(source_ref);
        } else {
            grid.set_source(source_ref);
        }
        grid.set_template(Some(tree_data_grid_template(cache_length, always_measure_viewport_changes)));

        let window = Window::new();
        window.set_width(800.0);
        window.set_height(500.0);
        window.set_content(Some(Control::boxed(&grid)));
        let row_template: Option<Rc<dyn IControlTemplate>> = Some(row_template(always_measure_column_viewport_changes));
        window.styles().add(Style::with_setters(
            Selectors::of_type::<TreeDataGridRow>(),
            [
                Setter::new(TemplatedControl::template_property(), row_template),
                Setter::new(Layoutable::height_property(), 24.0),
            ],
        ));

        window.show();
        Dispatcher::ui_thread().run_jobs(None);
        window.update_layout();
        // `PART_ScrollViewer` of the template, which the grid holds as its `Scroll`.
        let scroll = grid.scroll().expect("the template of the grid has PART_ScrollViewer");

        self.items = Some(items);
        self.source_lifetime = Some(source);
        self.grid = Some(grid);
        self.window = Some(window);
        self.scroll = Some(scroll);
    }

    /// `CleanupGrid`.
    fn cleanup_grid(&mut self) {
        if let Some(window) = self.window.take() {
            window.close();
        }
        Dispatcher::ui_thread().run_jobs(None);
        if let Some(source) = self.source_lifetime.take() {
            source.dispose();
        }
        self.grid = None;
        self.scroll = None;
        self.items = None;
    }
}

/// The grid of an iteration that did not end is cleaned up with the benchmark, and then the
/// global cleanup runs.
impl Drop for VirtualizationBenchmarks {
    fn drop(&mut self) {
        self.cleanup_grid();
        self.global_cleanup();
    }
}

fn header(text: &str) -> Option<BoxedValue> {
    Some(Rc::new(text.to_string()) as BoxedValue)
}

// ---------------------------------------------------------------------------------------------
// Templates
// ---------------------------------------------------------------------------------------------

/// `[!target] = x[!source]`: a template binding of `target_property` to `source_property` of the
/// templated parent.
fn template_bind(target: &Control, target_property: &'static FerroProperty, source_property: &'static FerroProperty) {
    target.bind_binding(target_property, &*TemplateBinding::new(source_property));
}

/// `TreeDataGridTemplate(cacheLength, alwaysMeasureViewportChanges)`: the rows presenter in a
/// scroll viewer, without column headers.
fn tree_data_grid_template(cache_length: f64, always_measure_viewport_changes: bool) -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::for_type::<TreeDataGrid>(move |_, scope| {
        let rows_presenter: Ref<TreeDataGridRowsPresenter> = if always_measure_viewport_changes {
            AlwaysMeasureRowsPresenter::new().upcast()
        } else {
            TreeDataGridRowsPresenter::new()
        };
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
        scroll_viewer.register_in_name_scope(&**scope).upcast()
    })
}

/// `RowTemplate(alwaysMeasureViewportChanges)`: the cells presenter of the row, bound to the
/// row's element factory, columns and rows.
fn row_template(always_measure_viewport_changes: bool) -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::for_type::<TreeDataGridRow>(move |_, scope| {
        let cells_presenter: Ref<TreeDataGridCellsPresenter> = if always_measure_viewport_changes {
            AlwaysMeasureCellsPresenter::new().upcast()
        } else {
            TreeDataGridCellsPresenter::new()
        };
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
// The presenters that measure for every change of their viewport
// ---------------------------------------------------------------------------------------------

/// What an always-measuring presenter remembers of its viewport: the start and the end of the
/// last effective viewport on its axis, clipped to its bounds as the presenters clip it.
#[derive(Default)]
struct ViewportTracker {
    last: Cell<Option<(f64, f64)>>,
    subscription: RefCell<Option<Rc<dyn IDisposable>>>,
}

impl ViewportTracker {
    /// Whether the viewport moved on the axis (`vertical` or horizontal) since the last change;
    /// an empty viewport (the presenter not yet arranged) is not a position.
    fn moved(&self, control: &Control, e: &EffectiveViewportChangedEventArgs, vertical: bool) -> bool {
        let effective_viewport = e.effective_viewport();
        if effective_viewport.size() == Size::default() {
            self.last.set(None);
            return false;
        }

        let viewport = effective_viewport.intersect(Rect::from_size(control.bounds().size()));
        let position = if vertical {
            (viewport.y, viewport.bottom())
        } else {
            (viewport.x, viewport.right())
        };
        let moved = self.last.get().is_some_and(|(start, end)| {
            !MathUtilities::are_close(start, position.0) || !MathUtilities::are_close(end, position.1)
        });
        self.last.set(Some(position));
        moved
    }
}

/// `AlwaysMeasureRowsPresenter`: a rows presenter that measures again whenever its viewport
/// moves (`NeedsMeasureForViewportChange` is `true`).
#[repr(C)]
pub struct AlwaysMeasureRowsPresenter {
    base: TreeDataGridRowsPresenter,
    viewport_tracker: ViewportTracker,
}

ferro_class!(AlwaysMeasureRowsPresenter: TreeDataGridRowsPresenter);
ferroui_base::ferro_impl_classes!(
    AlwaysMeasureRowsPresenter: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

impl AlwaysMeasureRowsPresenter {
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self { base: TreeDataGridRowsPresenter::construct(), viewport_tracker: ViewportTracker::default() });
        let weak = this.downgrade();
        let subscription = this.effective_viewport_changed(move |e| {
            if let Some(this) = weak.upgrade() {
                if this.viewport_tracker.moved(&this, e, true) {
                    this.invalidate_measure();
                }
            }
        });
        *this.viewport_tracker.subscription.borrow_mut() = Some(subscription);
        this
    }
}

/// `AlwaysMeasureCellsPresenter`: a cells presenter that measures again whenever its viewport
/// moves (`NeedsMeasureForViewportChange` is `true`).
#[repr(C)]
pub struct AlwaysMeasureCellsPresenter {
    base: TreeDataGridCellsPresenter,
    viewport_tracker: ViewportTracker,
}

ferro_class!(AlwaysMeasureCellsPresenter: TreeDataGridCellsPresenter);
ferroui_base::ferro_impl_classes!(
    AlwaysMeasureCellsPresenter: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TreeDataGridCellsPresenterImpl
);

impl AlwaysMeasureCellsPresenter {
    pub fn new() -> Ref<Self> {
        let this =
            instantiate(Self { base: TreeDataGridCellsPresenter::construct(), viewport_tracker: ViewportTracker::default() });
        let weak = this.downgrade();
        let subscription = this.effective_viewport_changed(move |e| {
            if let Some(this) = weak.upgrade() {
                if this.viewport_tracker.moved(&this, e, false) {
                    this.invalidate_measure();
                }
            }
        });
        *this.viewport_tracker.subscription.borrow_mut() = Some(subscription);
        this
    }
}

/// Registers the benchmarks of the class for one value of `NeutralSource`.
fn register_for(registry: &mut Registry, neutral_source: bool) {
    let parameters = format!("NeutralSource={neutral_source}");
    let setup = move || VirtualizationBenchmarks::new(neutral_source);
    let mut class = registry.class("treedatagrid", "VirtualizationBenchmarks");

    class.benchmark("create_and_layout_grid", parameters.clone(), setup, |b| b.create_and_layout_grid());

    // Each iteration setup runs the iteration cleanup of the previous iteration first.
    class
        .benchmark_with_iteration_setup(
            "vertical_small_scrolls",
            parameters.clone(),
            setup,
            |b| {
                b.cleanup_vertical_small_scrolls();
                b.setup_vertical_small_scrolls();
            },
            |b| b.vertical_small_scrolls(),
        )
        .operations_per_invoke(VERTICAL_SCROLL_OPERATIONS);
    class
        .benchmark_with_iteration_setup(
            "vertical_unbatched_small_scrolls",
            parameters.clone(),
            setup,
            |b| {
                b.cleanup_vertical_unbatched_small_scrolls();
                b.setup_vertical_unbatched_small_scrolls();
            },
            |b| b.vertical_unbatched_small_scrolls(),
        )
        .operations_per_invoke(VERTICAL_SCROLL_OPERATIONS);
    class
        .benchmark_with_iteration_setup(
            "forced_stationary_vertical_layouts",
            parameters.clone(),
            setup,
            |b| {
                b.cleanup_forced_stationary_vertical_layouts();
                b.setup_forced_stationary_vertical_layouts();
            },
            |b| b.forced_stationary_vertical_layouts(),
        )
        .operations_per_invoke(STATIONARY_LAYOUT_OPERATIONS);
    class
        .benchmark_with_iteration_setup(
            "vertical_row_scrolls",
            parameters.clone(),
            setup,
            |b| {
                b.cleanup_vertical_row_scrolls();
                b.setup_vertical_row_scrolls();
            },
            |b| b.vertical_row_scrolls(),
        )
        .operations_per_invoke(VERTICAL_SCROLL_OPERATIONS);
    class
        .benchmark_with_iteration_setup(
            "vertical_buffered_small_scrolls",
            parameters.clone(),
            setup,
            |b| {
                b.cleanup_vertical_buffered_small_scrolls();
                b.setup_vertical_buffered_small_scrolls();
            },
            |b| b.vertical_buffered_small_scrolls(),
        )
        .operations_per_invoke(VERTICAL_SCROLL_OPERATIONS);
    class
        .benchmark_with_iteration_setup(
            "vertical_jitter_scrolls",
            parameters.clone(),
            setup,
            |b| {
                b.cleanup_vertical_jitter_scrolls();
                b.setup_vertical_jitter_scrolls();
            },
            |b| b.vertical_jitter_scrolls(),
        )
        .operations_per_invoke(VERTICAL_SCROLL_OPERATIONS);
    class
        .benchmark_with_iteration_setup(
            "vertical_buffered_jitter_scrolls",
            parameters.clone(),
            setup,
            |b| {
                b.cleanup_vertical_buffered_jitter_scrolls();
                b.setup_vertical_buffered_jitter_scrolls();
            },
            |b| b.vertical_buffered_jitter_scrolls(),
        )
        .operations_per_invoke(VERTICAL_SCROLL_OPERATIONS);
    class
        .benchmark_with_iteration_setup(
            "horizontal_small_scrolls",
            parameters.clone(),
            setup,
            |b| {
                b.cleanup_horizontal_small_scrolls();
                b.setup_horizontal_small_scrolls();
            },
            |b| b.horizontal_small_scrolls(),
        )
        .operations_per_invoke(HORIZONTAL_SCROLL_OPERATIONS);
    class
        .benchmark_with_iteration_setup(
            "horizontal_unbatched_small_scrolls",
            parameters.clone(),
            setup,
            |b| {
                b.cleanup_horizontal_unbatched_small_scrolls();
                b.setup_horizontal_unbatched_small_scrolls();
            },
            |b| b.horizontal_unbatched_small_scrolls(),
        )
        .operations_per_invoke(HORIZONTAL_SCROLL_OPERATIONS);
    class
        .benchmark_with_iteration_setup(
            "horizontal_column_scrolls",
            parameters.clone(),
            setup,
            |b| {
                b.cleanup_horizontal_column_scrolls();
                b.setup_horizontal_column_scrolls();
            },
            |b| b.horizontal_column_scrolls(),
        )
        .operations_per_invoke(HORIZONTAL_COLUMN_OPERATIONS);
    class
        .benchmark_with_iteration_setup(
            "horizontal_far_column_scrolls",
            parameters.clone(),
            setup,
            |b| {
                b.cleanup_horizontal_far_column_scrolls();
                b.setup_horizontal_far_column_scrolls();
            },
            |b| b.horizontal_far_column_scrolls(),
        )
        .operations_per_invoke(HORIZONTAL_COLUMN_OPERATIONS);
    class
        .benchmark_with_iteration_setup(
            "collection_insert_remove_burst",
            parameters.clone(),
            setup,
            |b| {
                b.cleanup_collection_insert_remove_burst();
                b.setup_collection_insert_remove_burst();
            },
            |b| b.collection_insert_remove_burst(),
        )
        .operations_per_invoke(COLLECTION_EDIT_OPERATIONS);
    class
        .benchmark_with_iteration_setup(
            "collection_move_burst",
            parameters.clone(),
            setup,
            |b| {
                b.cleanup_collection_move_burst();
                b.setup_collection_move_burst();
            },
            |b| b.collection_move_burst(),
        )
        .operations_per_invoke(COLLECTION_MOVE_OPERATIONS);
    class
        .benchmark_with_iteration_setup(
            "detach_reattach",
            parameters,
            setup,
            |b| {
                b.cleanup_detach_reattach();
                b.setup_detach_reattach();
            },
            |b| b.detach_reattach(),
        )
        .operations_per_invoke(DETACH_REATTACH_OPERATIONS);
}

pub fn register(registry: &mut Registry) {
    for neutral_source in [false, true] {
        register_for(registry, neutral_source);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn virtualization_benchmarks() {
        ferroui_benchmarks::harness::smoke_class(super::register, "VirtualizationBenchmarks");
    }
}

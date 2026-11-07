//! The FerroUI side: the TreeDataGrid as `VirtualizationBenchmarks` creates it (the classic
//! columns of the control crate, shown through the grid's `Source`, which is
//! `NeutralSource = false`), in the unit test application with the services of a styled window.
//!
//! The grid, its rows and its scroll viewer have the templates of `virtualization_benchmarks.rs`
//! (no column headers, no scroll bars, no cache around the viewport), copied here so that the
//! row and column counts can vary; the setup is otherwise that of `CreateGrid` and `CleanupGrid`.
//!
//! [`FerroGrid`] has two modes. The layout mode is what the TreeDataGrid benchmarks measure: a
//! frame is a layout pass of the window. The render mode adds what the GPUI targets do in their
//! paint phase: the window's visual tree is drawn, every visual of it, into a recording drawing
//! context (`RenderDataDrawingContext` without a compositor, the recorder of the compositor's
//! render data), which records the drawing without drawing it. That is the drawing of a whole
//! frame, as an immediate-mode frame paints every element; the compositor of an application
//! records only the visuals that changed, so the render mode is an upper bound of what FerroUI
//! records a frame. The replay of the recording on a device is left out, as GPUI's present is.

use crate::model::{CellKind, ColumnSet, ColumnWidth, GridSpec, RowData, Text};
use crate::scenarios::{GridTarget, TreeTarget};
use ferroui_base::collections::FerroList;
use ferroui_base::data::TemplateBinding;
use ferroui_base::layout::Layoutable;
use ferroui_base::media::{DrawingContext, FontFamily};
use ferroui_base::rendering::composition::drawing::RenderDataDrawingContext;
use ferroui_base::rendering::ImmediateRenderer;
use ferroui_base::styling::{Selectors, Setter, Style};
use ferroui_base::threading::Dispatcher;
use ferroui_base::{BoxedValue, FerroProperty, Ref, Vector, Visual};
use ferroui_controls::presenters::ScrollContentPresenter;
use ferroui_controls::primitives::{ScrollBarVisibility, TemplatedControl};
use ferroui_controls::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions, IControlTemplate};
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_controls::{Control, GridLength, GridUnitType, ScrollViewer, Window};
use ferroui_controls_treedatagrid::models::{HierarchicalExpanderColumn, TextColumn};
use ferroui_controls_treedatagrid::primitives::{TreeDataGridCellsPresenter, TreeDataGridRow, TreeDataGridRowsPresenter};
use ferroui_controls_treedatagrid::{TreeDataGrid, TreeDataGridSourceRef};
use ferroui_treedatagrid_core::models::{IColumnOf, ListSortDirection};
use ferroui_treedatagrid_core::{Accessor, FlatTreeDataGridSource, HierarchicalTreeDataGridSource, IndexPath, Models};
use std::rc::Rc;

/// `RowModel(int Id, string Title)`: a record, compared by value.
#[derive(PartialEq)]
struct RowModel {
    id: i32,
    title: String,
}

impl RowModel {
    fn new(row: RowData) -> Rc<Self> {
        Rc::new(Self { id: row.id, title: row.title })
    }
}

/// The services of the application: those of the TreeDataGrid benchmarks, with the text of the
/// system for [`Text::Platform`].
fn services(text: Text) -> TestServices {
    let services = TestServices::styled_window();
    match text {
        Text::Synthetic => services,
        Text::Platform => services
            .with_font_manager_impl(Rc::new(ferroui_skia::FontManagerImpl::new()))
            .with_text_shaper_impl(Rc::new(ferroui_harfbuzz::HarfBuzzTextShaper::new())),
    }
}

fn header(text: &str) -> Option<BoxedValue> {
    Some(Rc::new(text.to_string()) as BoxedValue)
}

fn grid_length(width: ColumnWidth) -> GridLength {
    match width {
        ColumnWidth::Pixels(pixels) => GridLength::new(pixels, GridUnitType::Pixel),
        ColumnWidth::Auto => GridLength::AUTO,
        ColumnWidth::Star(stars) => GridLength::new(stars, GridUnitType::Star),
    }
}

/// The view columns of `columns` (`TextColumn<RowModel, int>` for the id, `TextColumn<RowModel,
/// string>` for the others).
fn add_columns(source: &FlatTreeDataGridSource<RowModel>, columns: ColumnSet) {
    for column in columns.columns() {
        let width = Some(grid_length(column.width));
        let model = match column.cell {
            CellKind::Id => TextColumn::<RowModel, i32>::new(
                header(&column.header),
                Accessor::member("Id", |x: &Rc<RowModel>| x.id),
                width,
                None,
            ) as Rc<dyn IColumnOf<RowModel>>,
            CellKind::Title => TextColumn::<RowModel, String>::new(
                header(&column.header),
                Accessor::member("Title", |x: &Rc<RowModel>| x.title.clone()),
                width,
                None,
            ) as Rc<dyn IColumnOf<RowModel>>,
            CellKind::Suffixed(index) => TextColumn::<RowModel, String>::new(
                header(&column.header),
                Accessor::new(move |x: &Rc<RowModel>| format!("{}-{index}", x.title)),
                width,
                None,
            ) as Rc<dyn IColumnOf<RowModel>>,
        };
        source.columns().add(model);
    }
}

/// Shows `grid` in a new window of the viewport, with the row style of the benchmarks, and lays
/// it out (`CreateGrid` after the grid is made).
fn show_in_window(grid: &Ref<TreeDataGrid>, text: Text) -> Ref<Window> {
    grid.set_template(Some(tree_data_grid_template()));
    if let Some(family) = text.font_family() {
        grid.set_font_family(FontFamily::new(family));
    }

    let window = Window::new();
    window.set_width(crate::model::VIEWPORT_WIDTH);
    window.set_height(crate::model::VIEWPORT_HEIGHT);
    window.set_content(Some(Control::boxed(grid)));
    let row_template: Option<Rc<dyn IControlTemplate>> = Some(row_template());
    window.styles().add(Style::with_setters(
        Selectors::of_type::<TreeDataGridRow>(),
        [
            Setter::new(TemplatedControl::template_property(), row_template),
            Setter::new(Layoutable::height_property(), crate::model::ROW_HEIGHT),
        ],
    ));

    window.show();
    Dispatcher::ui_thread().run_jobs(None);
    window.update_layout();
    window
}

/// The drawing of a frame: the whole visual tree of `window` recorded into `recorder`, which is
/// then cleared for the next frame.
fn record_frame(recorder: &mut RenderDataDrawingContext, window: &Ref<Window>) {
    {
        let mut context = DrawingContext::new(recorder);
        ImmediateRenderer::render(&mut context, window.upcast_ref::<Visual>());
    }
    recorder.reset();
}

/// The realized rows of `grid` (`_grid.RowsPresenter.GetRealizedElements().Count()`).
fn realized_row_count(grid: &Ref<TreeDataGrid>) -> usize {
    grid.rows_presenter().map_or(0, |presenter| presenter.get_realized_elements().len())
}

/// The TreeDataGrid of a scenario.
pub struct FerroGrid {
    text: Text,
    /// Whether a frame also records the drawing of the window.
    render: bool,
    recorder: RenderDataDrawingContext,
    descending: bool,
    source: Option<Rc<FlatTreeDataGridSource<RowModel>>>,
    window: Option<Ref<Window>>,
    grid: Option<Ref<TreeDataGrid>>,
    scroll: Option<Ref<ScrollViewer>>,
    items: Option<Rc<FerroList<Rc<RowModel>>>>,
    /// Disposed when the target is dropped, after the grid (the fields drop in the order of
    /// their declaration).
    _app: UnitTestApplicationScope,
}

impl FerroGrid {
    /// `[GlobalSetup] GlobalSetup`: the application.
    pub fn new(text: Text, render: bool) -> Self {
        let app = UnitTestApplication::start(services(text));
        Self {
            text,
            render,
            recorder: RenderDataDrawingContext::new(None),
            descending: false,
            source: None,
            window: None,
            grid: None,
            scroll: None,
            items: None,
            _app: app,
        }
    }

    fn grid(&self) -> &Ref<TreeDataGrid> {
        self.grid.as_ref().expect("the grid was created")
    }

    fn items(&self) -> &Rc<FerroList<Rc<RowModel>>> {
        self.items.as_ref().expect("the grid was created")
    }
}

impl GridTarget for FerroGrid {
    /// `CreateGrid`.
    fn create(&mut self, spec: &GridSpec) {
        let items: Rc<FerroList<Rc<RowModel>>> =
            Rc::new(FerroList::from_items(spec.rows.rows(spec.row_count).into_iter().map(RowModel::new)));
        let source = FlatTreeDataGridSource::new(items.clone() as Models<RowModel>);
        add_columns(&source, spec.columns);

        let grid = TreeDataGrid::new();
        grid.set_source(Some(TreeDataGridSourceRef::of::<RowModel, _>(source.clone())));
        let window = show_in_window(&grid, self.text);
        // `PART_ScrollViewer` of the template, which the grid holds as its `Scroll`.
        let scroll = grid.scroll().expect("the template of the grid has PART_ScrollViewer");
        if self.render {
            record_frame(&mut self.recorder, &window);
        }

        self.items = Some(items);
        self.source = Some(source);
        self.grid = Some(grid);
        self.window = Some(window);
        self.scroll = Some(scroll);
        self.descending = false;
    }

    /// `CleanupGrid`.
    fn destroy(&mut self) {
        if let Some(window) = self.window.take() {
            window.close();
        }
        Dispatcher::ui_thread().run_jobs(None);
        if let Some(source) = self.source.take() {
            source.dispose();
        }
        self.grid = None;
        self.scroll = None;
        self.items = None;
    }

    fn set_offset(&mut self, x: f64, y: f64) {
        self.scroll.as_ref().expect("the grid was created").set_offset(Vector::new(x, y));
    }

    fn invalidate_rows(&mut self) {
        self.grid().rows_presenter().expect("the rows presenter of the grid").invalidate_measure();
    }

    /// `_grid.UpdateLayout()` (the layout pass of the window the grid is in; the window's own
    /// when the grid is detached, as `DetachReattach` does), and the recording of the frame in
    /// the render mode.
    fn frame(&mut self) {
        let window = self.window.as_ref().expect("the grid was created");
        window.update_layout();
        if self.render {
            record_frame(&mut self.recorder, window);
        }
    }

    fn insert_row(&mut self, index: usize, row: RowData) {
        self.items().insert(index, RowModel::new(row));
    }

    fn remove_row(&mut self, index: usize) {
        self.items().remove_at(index);
    }

    fn move_row(&mut self, from: usize, to: usize) {
        self.items().move_item(from, to);
    }

    fn detach(&mut self) {
        self.window.as_ref().expect("the grid was created").set_content(None);
    }

    fn attach(&mut self) {
        let window = self.window.as_ref().expect("the grid was created");
        window.set_content(Some(Control::boxed(self.grid.as_ref().expect("the grid was created"))));
    }

    /// `SortThousandRows` on the source of the grid: the id column, in alternating directions.
    fn sort_next(&mut self) {
        self.descending = !self.descending;
        let direction = if self.descending { ListSortDirection::Descending } else { ListSortDirection::Ascending };
        let source = self.source.as_ref().expect("the grid was created");
        source.sort_by_column(&source.columns().get(0), direction);
    }

    fn checksum(&mut self) -> usize {
        realized_row_count(self.grid())
    }
}

/// The grid of an iteration that did not end is cleaned up with the target.
impl Drop for FerroGrid {
    fn drop(&mut self) {
        self.destroy();
    }
}

// ---------------------------------------------------------------------------------------------
// The tree
// ---------------------------------------------------------------------------------------------

/// `Row` of `NeutralSourceBenchmarks`: an id, a title and optional children, compared by
/// reference.
struct TreeRow {
    id: i32,
    title: String,
    children: Option<Models<TreeRow>>,
}

impl PartialEq for TreeRow {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl TreeRow {
    fn new(id: i32) -> Self {
        Self { id, title: format!("Row {id}"), children: None }
    }
}

/// The hierarchical source of `NeutralSourceBenchmarks` (a root with a thousand children, the
/// classic expander column over the id) with a title column, in a TreeDataGrid in a window of
/// the viewport. The rows are 24 pixels high; the columns are 80 and 100 pixels wide.
///
/// The expander cells have no template in the test application (its theme gives windows their
/// template and nothing else), so they lay out without presenting the id; the title cells are
/// text cells, which draw their text without a template.
pub struct FerroTree {
    render: bool,
    recorder: RenderDataDrawingContext,
    source: Rc<HierarchicalTreeDataGridSource<TreeRow>>,
    window: Ref<Window>,
    grid: Ref<TreeDataGrid>,
    _app: UnitTestApplicationScope,
}

impl FerroTree {
    pub fn new(text: Text, render: bool) -> Self {
        let app = UnitTestApplication::start(services(text));
        let items: Models<TreeRow> = Rc::new((0..1000).map(|x| Rc::new(TreeRow::new(1000 - x))).collect::<Vec<_>>());
        let root = Rc::new(TreeRow { children: Some(items), ..TreeRow::new(0) });
        let roots: Models<TreeRow> = Rc::new(vec![root]);

        let source = HierarchicalTreeDataGridSource::new(roots);
        let inner = TextColumn::<TreeRow, i32>::new(
            header("ID"),
            Accessor::member("Id", |x: &Rc<TreeRow>| x.id),
            Some(GridLength::new(80.0, GridUnitType::Pixel)),
            None,
        );
        let expander = HierarchicalExpanderColumn::new(inner, |x: &Rc<TreeRow>| x.children.clone(), None, None);
        source.columns().add(expander as Rc<dyn IColumnOf<TreeRow>>);
        source.columns().add(TextColumn::<TreeRow, String>::new(
            header("Title"),
            Accessor::member("Title", |x: &Rc<TreeRow>| x.title.clone()),
            Some(GridLength::new(100.0, GridUnitType::Pixel)),
            None,
        ) as Rc<dyn IColumnOf<TreeRow>>);

        let grid = TreeDataGrid::new();
        grid.set_source(Some(TreeDataGridSourceRef::of::<TreeRow, _>(source.clone())));
        let window = show_in_window(&grid, text);
        let mut recorder = RenderDataDrawingContext::new(None);
        if render {
            record_frame(&mut recorder, &window);
        }
        Self { render, recorder, source, window, grid, _app: app }
    }

    fn frame(&mut self) {
        self.window.update_layout();
        if self.render {
            record_frame(&mut self.recorder, &self.window);
        }
    }
}

impl TreeTarget for FerroTree {
    fn expand_collapse(&mut self, frames: bool) -> usize {
        self.source.expand(&IndexPath::new(0));
        if frames {
            self.frame();
        }
        let count = realized_row_count(&self.grid);
        debug_assert!(!frames || count > 1, "the expanded root realizes its children");
        self.source.collapse(&IndexPath::new(0));
        if frames {
            self.frame();
        }
        count
    }
}

impl Drop for FerroTree {
    fn drop(&mut self) {
        self.window.close();
        Dispatcher::ui_thread().run_jobs(None);
        self.source.dispose();
    }
}

// ---------------------------------------------------------------------------------------------
// Templates (those of `virtualization_benchmarks.rs` without the always-measuring presenters)
// ---------------------------------------------------------------------------------------------

/// `[!target] = x[!source]`: a template binding of `target_property` to `source_property` of the
/// templated parent.
fn template_bind(target: &Control, target_property: &'static FerroProperty, source_property: &'static FerroProperty) {
    target.bind_binding(target_property, &*TemplateBinding::new(source_property));
}

/// `TreeDataGridTemplate(cacheLength: 0)`: the rows presenter in a scroll viewer, without column
/// headers.
fn tree_data_grid_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::for_type::<TreeDataGrid>(move |_, scope| {
        let rows_presenter = TreeDataGridRowsPresenter::new();
        rows_presenter.set_name(Some("PART_RowsPresenter".to_string()));
        rows_presenter.set_cache_length(0.0);
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

/// `RowTemplate()`: the cells presenter of the row, bound to the row's element factory, columns
/// and rows.
fn row_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::for_type::<TreeDataGridRow>(move |_, scope| {
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

/// `ScrollViewerTemplate()`: a scroll content presenter alone, which attaches itself to the
/// scroll viewer it is in.
fn scroll_viewer_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::for_type::<ScrollViewer>(|_, scope| {
        let presenter = ScrollContentPresenter::new();
        presenter.set_name(Some("PART_ContentPresenter".to_string()));
        presenter.register_in_name_scope(&**scope).upcast()
    })
}

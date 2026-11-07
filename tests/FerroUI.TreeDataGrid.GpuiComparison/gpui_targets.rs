//! The GPUI side: the `DataTable` and the `Tree` of gpui-component, and a table and a tree of
//! one's own on GPUI's `uniform_list`, each the root view of a window of GPUI's test platform.
//!
//! GPUI draws in immediate mode: every frame renders the views again, lays the elements out,
//! prepaints them (where the virtualized lists render their visible items) and paints them into
//! the scene of the frame. A frame here is `Window::draw`, which does all of that; the scene is
//! not presented (the test platform has no GPU, and nothing is drawn on a device), as the
//! FerroUI render target records its drawing without replaying it.
//!
//! The windows are those of `TestAppContext` (GPUI's test platform, with the test dispatcher,
//! which runs every task on the thread of the benchmark), with accessibility turned off, as on
//! a desktop without an assistive technology connected. The text is the no-op text system of the
//! test platform ([`Text::Synthetic`]) or, on macOS, the CoreText text system of GPUI's macOS
//! platform ([`Text::Platform`]).

use crate::model::{
    sort_rows, ColumnDef, ColumnWidth, GridSpec, RowData, RowSet, Text, FONT_SIZE, ROW_HEIGHT, VIEWPORT_HEIGHT,
    VIEWPORT_WIDTH,
};
use crate::scenarios::{GridTarget, TreeTarget};
use gpui::prelude::*;
use gpui::{
    div, point, px, size, uniform_list, Action, AnyWindowHandle, App, Div, Entity, SharedString, TestAppContext,
    TestDispatcher, UniformListScrollHandle, Window, WindowHandle,
};
use gpui_base::actions::{SelectLeft, SelectRight};
use gpui_component::list::ListItem;
use gpui_component::table::{Column, ColumnSort, DataTable, TableDelegate, TableState};
use gpui_component::tree::{tree, TreeItem, TreeState};
use gpui_component::{h_flex, Sizable as _, Size as ComponentSize};
use std::ops::Range;
use std::sync::Arc;

fn pixels(value: f64) -> gpui::Pixels {
    px(value as f32)
}

/// The text system of the platform for [`Text::Platform`]: GPUI's macOS text system, which runs
/// on any thread (the macOS platform itself must be created on the main thread, and the harness
/// runs each benchmark on a thread of its own).
#[cfg(target_os = "macos")]
fn platform_text_system() -> Arc<dyn gpui::PlatformTextSystem> {
    Arc::new(gpui_apple::AppleTextSystem::new())
}

#[cfg(not(target_os = "macos"))]
fn platform_text_system() -> Arc<dyn gpui::PlatformTextSystem> {
    panic!("the GPUI targets have the text of the platform on macOS only (crate::model::GPUI_HAS_PLATFORM_TEXT)")
}

/// A GPUI application of the test platform, with the components initialised.
pub struct GpuiApp {
    pub cx: TestAppContext,
    dispatcher: TestDispatcher,
}

impl GpuiApp {
    pub fn new(text: Text) -> Self {
        let dispatcher = TestDispatcher::new(0);
        let mut cx = match text {
            Text::Synthetic => TestAppContext::build(dispatcher.clone(), None),
            Text::Platform => TestAppContext::build_with_text_system(dispatcher.clone(), None, platform_text_system()),
        };
        cx.disable_accessibility();
        cx.update(gpui_component::init);
        Self { cx, dispatcher }
    }

    /// Opens a window of the viewport with the root view `build` makes; GPUI draws its first
    /// frame before it returns.
    pub fn open<V: Render + 'static>(&mut self, build: impl FnOnce(&mut Window, &mut Context<V>) -> V) -> WindowHandle<V> {
        self.cx.open_window(size(pixels(VIEWPORT_WIDTH), pixels(VIEWPORT_HEIGHT)), build)
    }

    /// One frame of `window`: render, layout, prepaint and paint into the scene.
    pub fn draw(&mut self, window: AnyWindowHandle) {
        self.cx.update_window(window, |_, window, cx| window.draw(cx).clear(cx)).expect("the window is open");
    }

    pub fn close(&mut self, window: AnyWindowHandle) {
        let _ = self.cx.update_window(window, |_, window, _| window.remove_window());
        self.cx.run_until_parked();
    }

    pub fn dispatch(&mut self, window: AnyWindowHandle, action: Box<dyn Action>) {
        self.cx.update_window(window, |_, window, cx| window.dispatch_action(action, cx)).expect("the window is open");
    }
}

/// What the test macro of GPUI does at the end of a test.
impl Drop for GpuiApp {
    fn drop(&mut self) {
        self.cx.run_until_parked();
        self.cx.quit();
        self.dispatcher.run_until_parked();
    }
}

/// The root element of every window: the font size of FerroUI's text, and the family of the
/// platform variants.
fn root_div(font: Option<SharedString>) -> Div {
    div().size_full().text_size(pixels(FONT_SIZE)).when_some(font, |this, font| this.font_family(font))
}

fn font_of(text: Text) -> Option<SharedString> {
    text.font_family().map(SharedString::from)
}

fn fixed_width(column: &ColumnDef) -> f32 {
    match column.width {
        ColumnWidth::Pixels(width) => width as f32,
        ColumnWidth::Auto | ColumnWidth::Star(_) => unreachable!("the target does not support the width (supports)"),
    }
}

// ---------------------------------------------------------------------------------------------
// gpui-component: DataTable
// ---------------------------------------------------------------------------------------------

/// The delegate of the table: the rows and the columns of the scenario. The cells are the text
/// of the cell, as the text cells of the TreeDataGrid are.
pub struct BenchDelegate {
    rows: Vec<RowData>,
    columns: Vec<ColumnDef>,
    descending: bool,
}

impl TableDelegate for BenchDelegate {
    fn columns_count(&self, _: &App) -> usize {
        self.columns.len()
    }

    fn rows_count(&self, _: &App) -> usize {
        self.rows.len()
    }

    fn column(&self, col_ix: usize, _: &App) -> Column {
        let column = &self.columns[col_ix];
        Column::new(format!("column-{col_ix}"), column.header.clone()).width(px(fixed_width(column)))
    }

    fn render_td(
        &mut self,
        row_ix: usize,
        col_ix: usize,
        _: &mut Window,
        _: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        self.columns[col_ix].cell.text(&self.rows[row_ix])
    }

    /// The sort of a click on the header of the id column: the delegate's own code.
    fn perform_sort(&mut self, _: usize, sort: ColumnSort, _: &mut Window, _: &mut Context<TableState<Self>>) {
        sort_rows(&mut self.rows, sort == ColumnSort::Descending);
    }
}

/// The root view of the table's window: the `DataTable` with rows 24 pixels high (as the
/// TreeDataGrid's), its header row as high, and no borders, stripes, scroll bars or bounce (the
/// templates of the TreeDataGrid benchmarks have no scroll bars either). The table keeps its
/// default features: sortable, resizable and movable columns, row selection.
pub struct TableRoot {
    state: Entity<TableState<BenchDelegate>>,
    attached: bool,
    font: Option<SharedString>,
}

impl Render for TableRoot {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let root = root_div(self.font.clone());
        if !self.attached {
            return root;
        }
        root.child(
            DataTable::new(&self.state)
                .with_size(ComponentSize::Size(pixels(ROW_HEIGHT)))
                .header_row_height(pixels(ROW_HEIGHT))
                .bordered(false)
                .stripe(false)
                .scroll_bounce(false)
                .scrollbar_visible(false, false),
        )
    }
}

/// The `DataTable` of gpui-component. It virtualizes its rows (a `uniform_list`) and its columns
/// (a horizontal virtual list in every row and in the header, `table/state.rs`); its columns have
/// a width in pixels, without an automatic or a star width.
pub struct GpuiComponentGrid {
    text: Text,
    window: Option<WindowHandle<TableRoot>>,
    state: Option<Entity<TableState<BenchDelegate>>>,
    /// Dropped last: the application ends after the window is closed.
    app: GpuiApp,
}

impl GpuiComponentGrid {
    pub fn new(text: Text) -> Self {
        Self { text, window: None, state: None, app: GpuiApp::new(text) }
    }

    /// Whether the table can show the columns of `spec`: widths in pixels only.
    pub fn supports(spec: &GridSpec) -> bool {
        !spec.columns.has_auto() && !spec.columns.has_star()
    }

    fn window(&self) -> WindowHandle<TableRoot> {
        self.window.expect("the grid was created")
    }

    fn update_state<R>(
        &mut self,
        update: impl FnOnce(&mut TableState<BenchDelegate>, &mut Context<TableState<BenchDelegate>>) -> R,
    ) -> R {
        let state = self.state.as_ref().expect("the grid was created");
        state.update(&mut self.app.cx, update)
    }

    fn update_root<R>(&mut self, update: impl FnOnce(&mut TableRoot, &mut Window, &mut Context<TableRoot>) -> R) -> R {
        let window = self.window();
        window.update(&mut self.app.cx, update).expect("the window is open")
    }

    /// The rows and the columns the table rendered in its last frame.
    pub fn visible_range(&mut self) -> (Range<usize>, Range<usize>) {
        let state = self.state.as_ref().expect("the grid was created");
        self.app.cx.read(|cx| {
            let range = state.read(cx).visible_range();
            (range.rows().clone(), range.cols().clone())
        })
    }
}

impl GridTarget for GpuiComponentGrid {
    fn create(&mut self, spec: &GridSpec) {
        let delegate = BenchDelegate { rows: spec.rows.rows(spec.row_count), columns: spec.columns.columns(), descending: false };
        let font = font_of(self.text);
        let window = self.app.open(move |window, cx| {
            let state = cx.new(|cx| TableState::new(delegate, window, cx));
            TableRoot { state, attached: true, font }
        });
        let state = window.update(&mut self.app.cx, |root, _, _| root.state.clone()).expect("the window is open");
        self.window = Some(window);
        self.state = Some(state);
    }

    fn destroy(&mut self) {
        self.state = None;
        if let Some(window) = self.window.take() {
            self.app.close(window.into());
        }
    }

    /// The table's two scroll handles: the vertical one of its rows' `uniform_list`, the
    /// horizontal one its rows and its header share. GPUI's offsets are negative.
    fn set_offset(&mut self, x: f64, y: f64) {
        self.update_state(|state, cx| {
            state.horizontal_scroll_handle.set_offset(point(pixels(-x), px(0.)));
            state.vertical_scroll_handle.0.borrow().base_handle.set_offset(point(px(0.), pixels(-y)));
            cx.notify();
        });
    }

    fn frame(&mut self) {
        let window = self.window();
        self.app.draw(window.into());
    }

    fn insert_row(&mut self, index: usize, row: RowData) {
        self.update_state(|state, cx| {
            state.delegate_mut().rows.insert(index, row);
            cx.notify();
        });
    }

    fn remove_row(&mut self, index: usize) {
        self.update_state(|state, cx| {
            state.delegate_mut().rows.remove(index);
            cx.notify();
        });
    }

    fn move_row(&mut self, from: usize, to: usize) {
        self.update_state(|state, cx| {
            let rows = &mut state.delegate_mut().rows;
            let row = rows.remove(from);
            rows.insert(to, row);
            cx.notify();
        });
    }

    /// The table's state stays alive while its element is out of the window, as the
    /// TreeDataGrid does while it is out of its window.
    fn detach(&mut self) {
        self.update_root(|root, _, cx| {
            root.attached = false;
            cx.notify();
        });
    }

    fn attach(&mut self) {
        self.update_root(|root, _, cx| {
            root.attached = true;
            cx.notify();
        });
    }

    /// The delegate's `perform_sort`, as a click on the header calls it.
    fn sort_next(&mut self) {
        let state = self.state.clone().expect("the grid was created");
        self.update_root(move |_, window, cx| {
            state.update(cx, |state, cx| {
                let delegate = state.delegate_mut();
                delegate.descending = !delegate.descending;
                let sort = if delegate.descending { ColumnSort::Descending } else { ColumnSort::Ascending };
                delegate.perform_sort(0, sort, window, cx);
                cx.notify();
            });
        });
    }

    fn checksum(&mut self) -> usize {
        self.visible_range().0.end
    }
}

impl Drop for GpuiComponentGrid {
    fn drop(&mut self) {
        self.destroy();
    }
}

// ---------------------------------------------------------------------------------------------
// The table of one's own on uniform_list
// ---------------------------------------------------------------------------------------------

/// A table built directly on GPUI: a header row and a `uniform_list` of rows, each row a line of
/// cells of the width of their column with the text of the cell. The rows are virtualized by
/// the `uniform_list`; the columns by the table itself, which renders only the columns that
/// intersect the viewport (the widths are fixed, so the first and the last are a binary search),
/// placed at their offset. Star columns are flex items that share the width of the row; there
/// is no automatic width.
pub struct ListRoot {
    rows: Vec<RowData>,
    columns: Vec<ColumnDef>,
    /// The left edge of each column and the right edge of the last (fixed widths).
    edges: Vec<f32>,
    star: bool,
    scroll: UniformListScrollHandle,
    scroll_x: f32,
    attached: bool,
    descending: bool,
    font: Option<SharedString>,
    /// The first row of the last range the list rendered (its visible rows: the list measures
    /// with its first row before it renders them).
    first_rendered: usize,
}

impl ListRoot {
    fn new(spec: &GridSpec, font: Option<SharedString>) -> Self {
        let columns = spec.columns.columns();
        let star = columns.iter().any(|column| matches!(column.width, ColumnWidth::Star(_)));
        let mut edges = Vec::with_capacity(columns.len() + 1);
        let mut right = 0.0_f32;
        edges.push(right);
        for column in &columns {
            if let ColumnWidth::Pixels(width) = column.width {
                right += width as f32;
            }
            edges.push(right);
        }
        Self {
            rows: spec.rows.rows(spec.row_count),
            columns,
            edges,
            star,
            scroll: UniformListScrollHandle::new(),
            scroll_x: 0.0,
            attached: true,
            descending: false,
            font,
            first_rendered: 0,
        }
    }

    /// The columns that intersect the viewport at the horizontal offset.
    fn visible_columns(&self) -> Range<usize> {
        let count = self.columns.len();
        if self.star {
            return 0..count;
        }
        let left = self.scroll_x;
        let right = left + VIEWPORT_WIDTH as f32;
        let start = self.edges[1..].partition_point(|edge| *edge <= left).min(count);
        let end = self.edges[..count].partition_point(|edge| *edge < right).max(start);
        start..end
    }

    /// A header or a row: the cells of `columns` with the text `text` gives each.
    fn line(&self, columns: Range<usize>, text: impl Fn(&ColumnDef) -> String) -> Div {
        let line = div().h(pixels(ROW_HEIGHT)).w_full().flex_shrink_0();
        if self.star {
            return line.flex().flex_row().children(self.columns.iter().map(|column| cell(div().flex_1(), text(column))));
        }
        let offset = self.edges[columns.start] - self.scroll_x;
        line.relative().overflow_hidden().child(
            div().absolute().top_0().left(px(offset)).h_full().flex().flex_row().children(columns.map(|ix| {
                let width = self.edges[ix + 1] - self.edges[ix];
                cell(div().w(px(width)).flex_shrink_0(), text(&self.columns[ix]))
            })),
        )
    }
}

fn cell(cell: Div, text: String) -> Div {
    cell.h_full().overflow_hidden().whitespace_nowrap().child(text)
}

impl Render for ListRoot {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let root = root_div(self.font.clone()).flex().flex_col();
        if !self.attached {
            return root;
        }
        let columns = self.visible_columns();
        let header = self.line(columns.clone(), |column| column.header.clone());
        let rows = uniform_list(
            "rows",
            self.rows.len(),
            cx.processor(move |this: &mut Self, range: Range<usize>, _: &mut Window, _: &mut Context<Self>| {
                this.first_rendered = range.start;
                range
                    .map(|ix| {
                        let row = &this.rows[ix];
                        this.line(columns.clone(), |column| column.cell.text(row))
                    })
                    .collect::<Vec<_>>()
            }),
        )
        .track_scroll(&self.scroll)
        .w_full()
        .h(pixels(VIEWPORT_HEIGHT - ROW_HEIGHT));
        root.child(header).child(rows)
    }
}

/// The table of one's own in a window of the viewport.
pub struct GpuiListGrid {
    text: Text,
    window: Option<WindowHandle<ListRoot>>,
    app: GpuiApp,
}

impl GpuiListGrid {
    pub fn new(text: Text) -> Self {
        Self { text, window: None, app: GpuiApp::new(text) }
    }

    /// Whether the table can show the columns of `spec`: widths in pixels or in stars.
    pub fn supports(spec: &GridSpec) -> bool {
        !spec.columns.has_auto()
    }

    fn update_root<R>(&mut self, update: impl FnOnce(&mut ListRoot, &mut Window, &mut Context<ListRoot>) -> R) -> R {
        let window = self.window.expect("the grid was created");
        window.update(&mut self.app.cx, update).expect("the window is open")
    }

    /// The first row and the first column the table showed in its last frame.
    pub fn first_visible(&mut self) -> (usize, usize) {
        self.update_root(|root, _, _| (root.first_rendered, root.visible_columns().start))
    }
}

impl GridTarget for GpuiListGrid {
    fn create(&mut self, spec: &GridSpec) {
        let root = ListRoot::new(spec, font_of(self.text));
        self.window = Some(self.app.open(move |_, _| root));
    }

    fn destroy(&mut self) {
        if let Some(window) = self.window.take() {
            self.app.close(window.into());
        }
    }

    fn set_offset(&mut self, x: f64, y: f64) {
        self.update_root(|root, _, cx| {
            root.scroll_x = x as f32;
            root.scroll.0.borrow().base_handle.set_offset(point(px(0.), pixels(-y)));
            cx.notify();
        });
    }

    fn frame(&mut self) {
        let window = self.window.expect("the grid was created");
        self.app.draw(window.into());
    }

    fn insert_row(&mut self, index: usize, row: RowData) {
        self.update_root(|root, _, cx| {
            root.rows.insert(index, row);
            cx.notify();
        });
    }

    fn remove_row(&mut self, index: usize) {
        self.update_root(|root, _, cx| {
            root.rows.remove(index);
            cx.notify();
        });
    }

    fn move_row(&mut self, from: usize, to: usize) {
        self.update_root(|root, _, cx| {
            let row = root.rows.remove(from);
            root.rows.insert(to, row);
            cx.notify();
        });
    }

    fn detach(&mut self) {
        self.update_root(|root, _, cx| {
            root.attached = false;
            cx.notify();
        });
    }

    fn attach(&mut self) {
        self.update_root(|root, _, cx| {
            root.attached = true;
            cx.notify();
        });
    }

    fn sort_next(&mut self) {
        self.update_root(|root, _, cx| {
            root.descending = !root.descending;
            sort_rows(&mut root.rows, root.descending);
            cx.notify();
        });
    }

    fn checksum(&mut self) -> usize {
        self.first_visible().0
    }
}

impl Drop for GpuiListGrid {
    fn drop(&mut self) {
        self.destroy();
    }
}

// ---------------------------------------------------------------------------------------------
// The trees
// ---------------------------------------------------------------------------------------------

/// The children of the root of the tree of `NeutralSourceBenchmarks`: ids 1000 down to 1,
/// titled `Row {id}`.
fn tree_children() -> Vec<RowData> {
    RowSet::Descending.rows(1000)
}

/// An entry of a tree: the id and the title, indented by the depth, 24 pixels high.
fn tree_line(depth: usize, id: SharedString, title: SharedString) -> Div {
    h_flex().h(pixels(ROW_HEIGHT)).pl(px(16.0 * depth as f32)).child(id).child(title)
}

/// The root view of the window of the `Tree` of gpui-component.
pub struct TreeRoot {
    state: Entity<TreeState>,
    font: Option<SharedString>,
}

impl Render for TreeRoot {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        root_div(self.font.clone()).child(
            tree(&self.state, |ix, entry, _, _, _| {
                ListItem::new(ix).h(pixels(ROW_HEIGHT)).child(tree_line(
                    entry.depth(),
                    entry.item().id.clone(),
                    entry.item().label.clone(),
                ))
            })
            .scroll_bounce(false),
        )
    }
}

/// The `Tree` of gpui-component: a virtualized list of the visible entries of the tree (one
/// column, not a grid). Its expansion is private to its state; the public ways in are the
/// keyboard actions of a focused tree, so the root is selected and the tree focused, and
/// `SelectRight` expands the selected root and `SelectLeft` collapses it, as the arrow keys do.
pub struct GpuiComponentTree {
    window: WindowHandle<TreeRoot>,
    state: Entity<TreeState>,
    app: GpuiApp,
}

impl GpuiComponentTree {
    pub fn new(text: Text) -> Self {
        let mut app = GpuiApp::new(text);
        let children: Vec<TreeItem> =
            tree_children().into_iter().map(|row| TreeItem::new(row.id.to_string(), row.title)).collect();
        let root = TreeItem::new("0", "Row 0").children(children);
        let font = font_of(text);
        let window = app.open(move |_, cx| {
            let state = cx.new(|cx| TreeState::new(cx).items(vec![root]));
            TreeRoot { state, font }
        });
        let state = window
            .update(&mut app.cx, |root, window, cx| {
                root.state.update(cx, |state, cx| {
                    state.set_selected_index(Some(0), cx);
                    state.focus(window, cx);
                });
                root.state.clone()
            })
            .expect("the window is open");
        // The frame in which the tree has the focus, which the actions are dispatched to.
        app.draw(window.into());
        Self { window, state, app }
    }

    /// The entries the tree shows: the root alone, or the root and its thousand children.
    fn entry_count(&self) -> usize {
        let state = &self.state;
        self.app.cx.read(|cx| if state.read(cx).entry(1000).is_some() { 1001 } else { 1 })
    }
}

impl TreeTarget for GpuiComponentTree {
    fn expand_collapse(&mut self, frames: bool) -> usize {
        let window: AnyWindowHandle = self.window.into();
        self.app.dispatch(window, Box::new(SelectRight));
        if frames {
            self.app.draw(window);
        }
        let count = self.entry_count();
        debug_assert_eq!(count, 1001, "SelectRight expands the selected root");
        self.app.dispatch(window, Box::new(SelectLeft));
        if frames {
            self.app.draw(window);
        }
        count
    }
}

impl Drop for GpuiComponentTree {
    fn drop(&mut self) {
        self.app.close(self.window.into());
    }
}

/// The root view of the tree of one's own: a `uniform_list` of the root and, when it is
/// expanded, its children.
pub struct ListTreeRoot {
    children: Vec<RowData>,
    expanded: bool,
    scroll: UniformListScrollHandle,
    font: Option<SharedString>,
}

impl ListTreeRoot {
    fn entry(&self, ix: usize) -> Div {
        if ix == 0 {
            return tree_line(0, SharedString::from("0"), SharedString::from("Row 0"));
        }
        let row = &self.children[ix - 1];
        tree_line(1, SharedString::from(row.id.to_string()), SharedString::from(row.title.clone()))
    }
}

impl Render for ListTreeRoot {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let count = 1 + if self.expanded { self.children.len() } else { 0 };
        root_div(self.font.clone()).child(
            uniform_list(
                "entries",
                count,
                cx.processor(|this: &mut Self, range: Range<usize>, _: &mut Window, _: &mut Context<Self>| {
                    range.map(|ix| this.entry(ix)).collect::<Vec<_>>()
                }),
            )
            .track_scroll(&self.scroll)
            .size_full(),
        )
    }
}

/// The tree of one's own in a window of the viewport.
pub struct GpuiListTree {
    window: WindowHandle<ListTreeRoot>,
    app: GpuiApp,
}

impl GpuiListTree {
    pub fn new(text: Text) -> Self {
        let mut app = GpuiApp::new(text);
        let root = ListTreeRoot {
            children: tree_children(),
            expanded: false,
            scroll: UniformListScrollHandle::new(),
            font: font_of(text),
        };
        let window = app.open(move |_, _| root);
        Self { window, app }
    }

    fn set_expanded(&mut self, expanded: bool) {
        self.window
            .update(&mut self.app.cx, |root, _, cx| {
                root.expanded = expanded;
                cx.notify();
            })
            .expect("the window is open");
    }
}

impl TreeTarget for GpuiListTree {
    fn expand_collapse(&mut self, frames: bool) -> usize {
        let window: AnyWindowHandle = self.window.into();
        self.set_expanded(true);
        if frames {
            self.app.draw(window);
        }
        self.set_expanded(false);
        if frames {
            self.app.draw(window);
        }
        1001
    }
}

impl Drop for GpuiListTree {
    fn drop(&mut self) {
        self.app.close(self.window.into());
    }
}

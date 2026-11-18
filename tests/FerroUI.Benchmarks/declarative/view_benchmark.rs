//! Mounting and running large element trees with `ferroui-view`, against
//! controls made directly (the baseline), and the run-once model against
//! components that run again.

use crate::harness::Registry;
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::{Color, FontFamily, FontWeight, IBrush};
use ferroui_base::{CornerRadius, Ref, Thickness};
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_controls::{Border, ColumnDefinitions, Control, Grid, StackPanel, TextBlock, WrapPanel};
use ferroui_view::prelude::*;
use std::rc::Rc;

/// The rows of the list benchmarks.
const ROWS: u32 = 1000;
/// The tiles of the wall benchmarks: five elements each, a fill, a label over a dot and a value,
/// with the properties an application writes for them (a size, a corner radius, a fill given as
/// the text of a colour, a font family given by its name, a font size, a weight, alignments,
/// margins).
const TILES: u32 = 1000;
/// The fills of the wall.
const FILLS: [&str; 4] = ["#2E7D32", "#1565C0", "#6A1B9A", "#EF6C00"];
/// The family of the texts of the wall.
const FAMILY: &str = "Roboto";

/// The components of the component benchmarks (the cells of the 7GUIs
/// spreadsheet, 26 columns by 100 rows).
const COMPONENTS: u32 = 2600;

pub struct ViewBenchmark {
    owner: Owner,
    order: Signal<Vec<u32>>,
    version: Signal<u64>,
    changed: Signal<u32>,
    values: Vec<Signal<String>>,
    level: Option<Signal<f64>>,
    _root: Option<Ref<Control>>,
    _app: UnitTestApplicationScope,
}

impl ViewBenchmark {
    fn empty() -> Self {
        let app = UnitTestApplication::start(TestServices::mock_threading_interface());
        let owner = Owner::new_root();
        let (order, version, changed) = owner.with(|| (signal((0..ROWS).collect::<Vec<_>>()), signal(0_u64), signal(u32::MAX)));
        Self { owner, order, version, changed, values: Vec::new(), level: None, _root: None, _app: app }
    }

    /// A list of rows in a component that runs again, mounted.
    fn with_list() -> Self {
        let mut this = Self::empty();
        let (order, version, changed) = (this.order, this.version, this.changed);
        this._root = Some(this.owner.with(|| list_view(order, version, changed).mount()));
        this
    }

    /// A list of rows in the run-once model, each following a signal of its
    /// own, mounted.
    fn with_fine_grained_list() -> Self {
        let mut this = Self::empty();
        let values: Vec<Signal<String>> = this.owner.with(|| (0..ROWS).map(|i| signal(format!("row {i}"))).collect());
        let rows = values.clone();
        this._root = Some(this.owner.with(|| {
            stack_panel().children(rows.iter().map(|value| text_block().text(*value)).collect::<Vec<_>>()).into_element().mount()
        }));
        this.values = values;
        this
    }

    /// A thousand properties that follow one signal: every write of the signal runs the
    /// effect of each.
    fn with_shared_signal() -> Self {
        let mut this = Self::empty();
        let level = this.owner.with(|| signal(1.0_f64));
        this._root = Some(this.owner.with(|| {
            let rows = (0..ROWS).map(|_| text_block().opacity(level)).collect::<Vec<_>>();
            stack_panel().children(rows).into_element().mount()
        }));
        this.level = Some(level);
        this
    }

    /// A thousand grids of two star columns whose weights follow one signal, as a bar written as
    /// a grid does: each grid is given new column definitions by an effect of its own whenever
    /// the signal changes (what an application of the declarative layer writes for that with
    /// the control's setter).
    fn with_replaced_definitions() -> Self {
        let mut this = Self::empty();
        let version = this.version;
        this._root = Some(this.owner.with(|| {
            let rows = (0..ROWS)
                .map(|_| {
                    grid().with(move |grid| {
                        let grid = grid.clone();
                        effect(move || {
                            let text = bar_columns(version.get());
                            let columns = ColumnDefinitions::parse(&text).expect("column definitions");
                            untrack(|| grid.set_column_definitions(columns));
                        });
                    })
                })
                .collect::<Vec<_>>();
            stack_panel().children(rows).into_element().mount()
        }));
        this
    }

    /// The same bars with their columns given as a text that follows the signal
    /// (`GridDefinitionsText`): the definitions a grid has take the new weights.
    fn with_followed_definitions() -> Self {
        let mut this = Self::empty();
        let version = this.version;
        this._root = Some(this.owner.with(|| {
            let rows = (0..ROWS)
                .map(|_| grid().set(GridDefinitionsText::columns_property(), move || bar_columns(version.get())))
                .collect::<Vec<_>>();
            stack_panel().children(rows).into_element().mount()
        }));
        this
    }

    /// A parent that runs again over components, memoized or not.
    fn with_components(memoized: bool) -> Self {
        let mut this = Self::empty();
        let version = this.version;
        this._root = Some(this.owner.with(|| {
            rerender(move || {
                let _ = version.get();
                let cells: Vec<Element> = (0..COMPONENTS)
                    .map(|i| if memoized { memo_cell().index(i).into_element() } else { cell().index(i).into_element() })
                    .collect();
                stack_panel().children(cells)
            })
            .into_element()
            .mount()
        }));
        this
    }

    /// Controls made directly: the baseline of mounting.
    pub fn mount_list_controls(&mut self) -> Ref<StackPanel> {
        let panel = StackPanel::new();
        let children = panel.children();
        for i in 0..ROWS {
            let text = TextBlock::new();
            text.set_text(Some(format!("row {i}").as_str()));
            children.add(text);
        }
        panel
    }

    /// A wall of tiles made of controls directly and let go: the baseline of the wall of
    /// elements, which is mounted and taken out.
    pub fn mount_wall_controls(&mut self) {
        drop(self.wall_of_controls());
    }

    fn wall_of_controls(&self) -> Ref<WrapPanel> {
        let brush = |text: &str| -> Option<Rc<dyn IBrush>> {
            Some(Rc::new(ImmutableSolidColorBrush::new(Color::parse(text).expect("a colour"))))
        };
        let fills: Vec<_> = FILLS.iter().map(|fill| brush(fill)).collect();
        let (ink, dot_fill) = (brush("#FFFFFF"), brush("#FFC107"));
        let text = |content: String, size: f64, bold: bool, margin: Thickness| {
            let block = TextBlock::new();
            block.set_text(Some(content.as_str()));
            block.set_font_family(FontFamily::from(FAMILY));
            block.set_font_size(size);
            block.set_foreground(ink.clone());
            block.set_font_weight(if bold { FontWeight::Bold } else { FontWeight::Normal });
            block.set_horizontal_alignment(HorizontalAlignment::Left);
            block.set_vertical_alignment(VerticalAlignment::Top);
            block.set_margin(margin);
            block
        };
        let wall = WrapPanel::new();
        let tiles = wall.children();
        for i in 0..TILES {
            let dot = Border::new();
            dot.set_width(6.0);
            dot.set_height(6.0);
            dot.set_corner_radius(CornerRadius::uniform(3.0));
            dot.set_background(dot_fill.clone());
            dot.set_horizontal_alignment(HorizontalAlignment::Left);
            dot.set_vertical_alignment(VerticalAlignment::Top);
            dot.set_margin(Thickness::new(4.0, 22.0, 0.0, 0.0));
            let grid = Grid::new();
            let parts = grid.children();
            parts.add(text(format!("Tile {i}"), 11.0, true, Thickness::new(4.0, 4.0, 0.0, 0.0)));
            parts.add(dot);
            parts.add(text(format!("{}", i * 7), 10.0, false, Thickness::new(13.0, 19.0, 0.0, 0.0)));
            let tile = Border::new();
            tile.set_width(64.0);
            tile.set_height(40.0);
            tile.set_corner_radius(CornerRadius::uniform(4.0));
            tile.set_background(fills[i as usize % fills.len()].clone());
            tile.set_child(Some(grid.upcast()));
            tiles.add(tile);
        }
        wall
    }

    /// The same wall as elements in the run-once model, mounted and taken out.
    pub fn mount_wall_run_once(&mut self) {
        let text = |content: String, size: f64, bold: bool, margin: (f64, f64, f64, f64)| {
            text_block()
                .text(content)
                .font_family(FontFamily::from(FAMILY))
                .font_size(size)
                .foreground("#FFFFFF")
                .font_weight(if bold { FontWeight::Bold } else { FontWeight::Normal })
                .horizontal_alignment(HorizontalAlignment::Left)
                .vertical_alignment(VerticalAlignment::Top)
                .margin(margin)
        };
        let tile = |i: u32| {
            border().width(64.0).height(40.0).corner_radius(4.0).background(FILLS[i as usize % FILLS.len()]).child(
                grid()
                    .child(text(format!("Tile {i}"), 11.0, true, (4.0, 4.0, 0.0, 0.0)))
                    .child(
                        border()
                            .width(6.0)
                            .height(6.0)
                            .corner_radius(3.0)
                            .background("#FFC107")
                            .horizontal_alignment(HorizontalAlignment::Left)
                            .vertical_alignment(VerticalAlignment::Top)
                            .margin((4.0, 22.0, 0.0, 0.0)),
                    )
                    .child(text(format!("{}", i * 7), 10.0, false, (13.0, 19.0, 0.0, 0.0))),
            )
        };
        let owner = Owner::new_root();
        let _ = owner.with(|| wrap_panel().children((0..TILES).map(tile).collect::<Vec<_>>()).into_element().mount());
        owner.dispose();
    }

    /// The same list as elements in the run-once model, mounted and taken
    /// out.
    pub fn mount_list_run_once(&mut self) {
        let owner = Owner::new_root();
        let _ = owner.with(|| {
            stack_panel()
                .children((0..ROWS).map(|i| text_block().key(i).text(format!("row {i}"))).collect::<Vec<_>>())
                .into_element()
                .mount()
        });
        owner.dispose();
    }

    /// The same list in a component that runs again, mounted and taken out.
    pub fn mount_list_rerender(&mut self) {
        let owner = Owner::new_root();
        let (order, version, changed) = owner.with(|| (signal((0..ROWS).collect::<Vec<_>>()), signal(0_u64), signal(u32::MAX)));
        let _ = owner.with(|| list_view(order, version, changed).mount());
        owner.dispose();
    }

    /// A run of the list component in which nothing changed.
    pub fn run_unchanged(&mut self) {
        self.version.update(|v| *v += 1);
    }

    /// A run of the list component in which one row changed its text.
    pub fn run_one_row_changed(&mut self) {
        let next = (self.changed.get_untracked().wrapping_add(1)) % ROWS;
        self.changed.set(next);
    }

    /// One keyed row moved from the end to the front, and back.
    pub fn move_one_row(&mut self) {
        self.order.update(|order| {
            if order[0] == ROWS - 1 {
                order.remove(0);
                order.push(ROWS - 1);
            } else {
                let last = order.pop().unwrap();
                order.insert(0, last);
            }
        });
    }

    /// Every keyed row reversed.
    pub fn reverse_rows(&mut self) {
        self.order.update(|order| order.reverse());
    }

    /// One row of the run-once model changed through its signal: nothing
    /// runs but the property of that row.
    pub fn fine_grained_one_row_changed(&mut self) {
        let index = (self.version.get_untracked() % u64::from(ROWS)) as usize;
        self.version.update(|v| *v += 1);
        self.values[index].set(format!("changed {}", self.version.get_untracked()));
    }

    /// A parent that runs again over its components.
    /// Writes the signal a thousand properties follow.
    pub fn shared_signal_changed(&mut self) {
        let version = self.version.get_untracked() + 1;
        self.version.set(version);
        if let Some(level) = self.level {
            level.set(if version % 2 == 0 { 1.0 } else { 0.5 });
        }
    }

    /// Writes the signal the weights of a thousand bars follow.
    pub fn bar_weights_changed(&mut self) {
        self.version.update(|v| *v += 1);
    }

    pub fn run_parent_of_components(&mut self) {
        self.version.update(|v| *v += 1);
    }
}

impl Drop for ViewBenchmark {
    fn drop(&mut self) {
        self.owner.dispose();
    }
}

/// The columns of a bar at `version`: two star columns whose weights change.
fn bar_columns(version: u64) -> String {
    let filled = 1 + version % 9;
    format!("{filled}*,{}*", 10 - filled)
}

fn list_view(order: Signal<Vec<u32>>, version: Signal<u64>, changed: Signal<u32>) -> Element {
    rerender(move || {
        let _ = version.get();
        let changed = changed.get();
        stack_panel().children(
            order
                .get()
                .into_iter()
                .map(|i| {
                    let text = if i == changed { format!("row {i} (changed)") } else { format!("row {i}") };
                    text_block().key(i).text(text)
                })
                .collect::<Vec<_>>(),
        )
    })
}

#[component(rerender, always)]
fn Cell(index: u32) -> Element {
    rsx! { TextBlock { text: format!("cell {index}") } }
}

#[component(rerender)]
fn MemoCell(index: u32) -> Element {
    rsx! { TextBlock { text: format!("cell {index}") } }
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("declarative", "ViewBenchmark");
    class.benchmark("mount_list_controls", "", ViewBenchmark::empty, |b| b.mount_list_controls()).baseline();
    class.benchmark("mount_list_run_once", "", ViewBenchmark::empty, |b| b.mount_list_run_once());
    class.benchmark("mount_wall_controls", "", ViewBenchmark::empty, |b| b.mount_wall_controls());
    class.benchmark("mount_wall_run_once", "", ViewBenchmark::empty, |b| b.mount_wall_run_once());
    class.benchmark("mount_list_rerender", "", ViewBenchmark::empty, |b| b.mount_list_rerender());
    class.benchmark("run_unchanged", "", ViewBenchmark::with_list, |b| b.run_unchanged());
    class.benchmark("run_one_row_changed", "", ViewBenchmark::with_list, |b| b.run_one_row_changed());
    class.benchmark("move_one_row", "", ViewBenchmark::with_list, |b| b.move_one_row());
    class.benchmark("reverse_rows", "", ViewBenchmark::with_list, |b| b.reverse_rows());
    class.benchmark("fine_grained_one_row_changed", "", ViewBenchmark::with_fine_grained_list, |b| b.fine_grained_one_row_changed());
    class.benchmark("shared_signal_changed", "", ViewBenchmark::with_shared_signal, |b| b.shared_signal_changed());
    let replaced = ViewBenchmark::with_replaced_definitions;
    class.benchmark("grid_definitions_replaced", "", replaced, |b| b.bar_weights_changed());
    let followed = ViewBenchmark::with_followed_definitions;
    class.benchmark("grid_definitions_followed", "", followed, |b| b.bar_weights_changed());
    class.benchmark("run_parent_of_components", "always", || ViewBenchmark::with_components(false), |b| b.run_parent_of_components());
    class.benchmark("run_parent_of_components", "memoized by default", || ViewBenchmark::with_components(true), |b| b.run_parent_of_components());
}

#[cfg(test)]
mod tests {
    #[test]
    fn view_benchmark() {
        crate::harness::smoke_class(super::register, "ViewBenchmark");
    }
}

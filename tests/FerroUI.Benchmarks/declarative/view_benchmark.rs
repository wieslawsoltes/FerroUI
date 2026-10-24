//! Mounting and running large element trees with `ferroui-view`, against
//! controls made directly (the baseline), and the run-once model against
//! components that run again.

use crate::harness::Registry;
use ferroui_base::Ref;
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_controls::{Control, StackPanel, TextBlock};
use ferroui_view::prelude::*;

/// The rows of the list benchmarks.
const ROWS: u32 = 1000;
/// The components of the component benchmarks (the cells of the 7GUIs
/// spreadsheet, 26 columns by 100 rows).
const COMPONENTS: u32 = 2600;

pub struct ViewBenchmark {
    owner: Owner,
    order: Signal<Vec<u32>>,
    version: Signal<u64>,
    changed: Signal<u32>,
    values: Vec<Signal<String>>,
    _root: Option<Ref<Control>>,
    _app: UnitTestApplicationScope,
}

impl ViewBenchmark {
    fn empty() -> Self {
        let app = UnitTestApplication::start(TestServices::mock_threading_interface());
        let owner = Owner::new_root();
        let (order, version, changed) = owner.with(|| (signal((0..ROWS).collect::<Vec<_>>()), signal(0_u64), signal(u32::MAX)));
        Self { owner, order, version, changed, values: Vec::new(), _root: None, _app: app }
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
    pub fn run_parent_of_components(&mut self) {
        self.version.update(|v| *v += 1);
    }
}

impl Drop for ViewBenchmark {
    fn drop(&mut self) {
        self.owner.dispose();
    }
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
    class.benchmark("mount_list_rerender", "", ViewBenchmark::empty, |b| b.mount_list_rerender());
    class.benchmark("run_unchanged", "", ViewBenchmark::with_list, |b| b.run_unchanged());
    class.benchmark("run_one_row_changed", "", ViewBenchmark::with_list, |b| b.run_one_row_changed());
    class.benchmark("move_one_row", "", ViewBenchmark::with_list, |b| b.move_one_row());
    class.benchmark("reverse_rows", "", ViewBenchmark::with_list, |b| b.reverse_rows());
    class.benchmark("fine_grained_one_row_changed", "", ViewBenchmark::with_fine_grained_list, |b| b.fine_grained_one_row_changed());
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

//! `NeutralSourceBenchmarks.cs`: the source and presentation layer without a control: creating a
//! two-column flat source with its presentation, sorting its thousand rows, and expanding and
//! collapsing the thousand children of the root of a hierarchical source, for the classic columns
//! and for the framework-neutral columns of the core (`NeutralSource`).
//!
//! The reference compares its classic sources, presented through the compatibility presentation
//! (`LegacySourcePresentation`), with its core sources, presented by `TreeDataGridPresentation<T>`.
//! The port's sources are all the core's: the classic path is a core source holding the view
//! columns of the control crate (`ferroui_controls_treedatagrid::models`), the neutral path a core
//! source holding the neutral columns of the core (`ferroui_treedatagrid_core::models`), and both
//! are presented by the same presentation (`TreeDataGridSourcePresentation`), which presents a
//! view column itself and a neutral column through its presentation options.

use ferroui_base::BoxedValue;
use ferroui_benchmarks::harness::Registry;
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_controls_treedatagrid::models::{HierarchicalExpanderColumn, TextColumn};
use ferroui_controls_treedatagrid::presentation::{PresentationRef, TreeDataGridSourcePresentation};
use ferroui_treedatagrid_core::models::{
    ColumnRef as CoreColumnRef, HierarchicalExpanderColumn as CoreHierarchicalExpanderColumn, IColumnOf,
    ListSortDirection, RowsChangedArgs, RowsChangedHandler, TextColumn as CoreTextColumn,
};
use ferroui_treedatagrid_core::{Accessor, FlatTreeDataGridSource, HierarchicalTreeDataGridSource, IndexPath, Models};
use std::rc::Rc;

/// `Row`: a class with an id, a title and optional children, compared by reference.
struct Row {
    id: i32,
    title: String,
    children: Option<Models<Row>>,
}

impl PartialEq for Row {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl Row {
    fn new(id: i32) -> Self {
        Self { id, title: format!("Row {id}"), children: None }
    }

    /// `x => x.Id`.
    fn id_accessor() -> Accessor<Row, i32> {
        Accessor::member("Id", |x: &Rc<Row>| x.id)
    }

    /// `x => x.Title`.
    fn title_accessor() -> Accessor<Row, String> {
        Accessor::member("Title", |x: &Rc<Row>| x.title.clone())
    }

    /// `x => x.Children`.
    fn children(x: &Rc<Row>) -> Option<Models<Row>> {
        x.children.clone()
    }
}

fn header(text: &str) -> Option<BoxedValue> {
    Some(Rc::new(text.to_string()) as BoxedValue)
}

/// What disposes a source (the reference's `IDisposable` lifetime of a source).
type Lifetime = Box<dyn Fn()>;

pub struct NeutralSourceBenchmarks {
    neutral_source: bool,
    items: Models<Row>,
    flat: PresentationRef,
    flat_lifetime: Lifetime,
    tree: PresentationRef,
    tree_lifetime: Lifetime,
    expand: Box<dyn Fn()>,
    collapse: Box<dyn Fn()>,
    direction: ListSortDirection,
    /// Disposed when the benchmark is dropped, after the sources (the fields drop in the order
    /// of their declaration).
    _app: UnitTestApplicationScope,
}

impl NeutralSourceBenchmarks {
    /// `[GlobalSetup] Setup`.
    pub fn new(neutral_source: bool) -> Self {
        let app = UnitTestApplication::start(TestServices::styled_window());
        let items: Models<Row> = Rc::new((0..1000).map(|x| Rc::new(Row::new(1000 - x))).collect::<Vec<_>>());
        let (flat, flat_lifetime) = Self::create_flat_of(neutral_source, &items);
        let root = Rc::new(Row { children: Some(items.clone()), ..Row::new(0) });
        let roots: Models<Row> = Rc::new(vec![root]);

        let tree: PresentationRef;
        let tree_lifetime: Lifetime;
        let expand: Box<dyn Fn()>;
        let collapse: Box<dyn Fn()>;
        if neutral_source {
            let model = HierarchicalTreeDataGridSource::new(roots);
            let inner: CoreColumnRef<Row> = CoreTextColumn::new(header("ID"), Row::id_accessor(), None, None, None);
            let column: CoreColumnRef<Row> = CoreHierarchicalExpanderColumn::new(inner, Row::children, None, None, None);
            model.columns().add(column);
            tree = TreeDataGridSourcePresentation::of::<Row, _>(model.clone(), None) as PresentationRef;
            let lifetime_model = model.clone();
            tree_lifetime = Box::new(move || lifetime_model.dispose());
            let expand_model = model.clone();
            expand = Box::new(move || expand_model.expand(&IndexPath::new(0)));
            collapse = Box::new(move || model.collapse(&IndexPath::new(0)));
        } else {
            let model = HierarchicalTreeDataGridSource::new(roots);
            let inner = TextColumn::<Row, i32>::new(header("ID"), Row::id_accessor(), None, None);
            let column = HierarchicalExpanderColumn::new(inner, Row::children, None, None);
            model.columns().add(column as Rc<dyn IColumnOf<Row>>);
            tree = TreeDataGridSourcePresentation::of::<Row, _>(model.clone(), None) as PresentationRef;
            let lifetime_model = model.clone();
            tree_lifetime = Box::new(move || lifetime_model.dispose());
            let expand_model = model.clone();
            expand = Box::new(move || expand_model.expand(&IndexPath::new(0)));
            collapse = Box::new(move || model.collapse(&IndexPath::new(0)));
        }

        let _ = flat.rows().count();
        let _ = tree.rows().count();
        let flat_handler: Rc<RowsChangedHandler> = Rc::new(|_: &RowsChangedArgs<'_>| {});
        flat.rows().add_collection_changed(flat_handler);
        let tree_handler: Rc<RowsChangedHandler> = Rc::new(|_: &RowsChangedArgs<'_>| {});
        tree.rows().add_collection_changed(tree_handler);
        expand();
        collapse();

        Self {
            neutral_source,
            items,
            flat,
            flat_lifetime,
            tree,
            tree_lifetime,
            expand,
            collapse,
            direction: ListSortDirection::Ascending,
            _app: app,
        }
    }

    /// `[Benchmark] CreateSourceAndPresentation`.
    pub fn create_source_and_presentation(&self) -> usize {
        let (source, lifetime) = Self::create_flat_of(self.neutral_source, &self.items);
        let count = source.rows().count();
        source.dispose();
        lifetime();
        count
    }

    /// `[Benchmark] SortThousandRows`.
    pub fn sort_thousand_rows(&mut self) -> i32 {
        self.direction = if self.direction == ListSortDirection::Ascending {
            ListSortDirection::Descending
        } else {
            ListSortDirection::Ascending
        };
        let columns = self.flat.columns();
        self.flat.sort_by(&columns.get(0), self.direction);
        self.flat.rows().get(0).model().and_then(|model| model.downcast_ref::<Row>().map(|row| row.id)).unwrap_or(-1)
    }

    /// `[Benchmark] ExpandCollapseThousandRows`.
    pub fn expand_collapse_thousand_rows(&self) -> usize {
        (self.expand)();
        let count = self.tree.rows().count();
        (self.collapse)();
        count
    }

    /// `CreateFlat`: a flat source of the items with an id and a title column, and its
    /// presentation.
    fn create_flat_of(neutral_source: bool, items: &Models<Row>) -> (PresentationRef, Lifetime) {
        if neutral_source {
            let model = FlatTreeDataGridSource::new(items.clone());
            model.columns().add(CoreTextColumn::new(header("ID"), Row::id_accessor(), None, None, None));
            model.columns().add(CoreTextColumn::new(header("Title"), Row::title_accessor(), None, None, None));
            let presentation: PresentationRef = TreeDataGridSourcePresentation::of::<Row, _>(model.clone(), None);
            return (presentation, Box::new(move || model.dispose()));
        }

        let source = FlatTreeDataGridSource::new(items.clone());
        source.columns().add(TextColumn::<Row, i32>::new(header("ID"), Row::id_accessor(), None, None) as Rc<dyn IColumnOf<Row>>);
        source
            .columns()
            .add(TextColumn::<Row, String>::new(header("Title"), Row::title_accessor(), None, None) as Rc<dyn IColumnOf<Row>>);
        // Match native presentation's creation of selection and rows.
        let _ = source.row_selection();
        let presentation: PresentationRef = TreeDataGridSourcePresentation::of::<Row, _>(source.clone(), None);
        (presentation, Box::new(move || source.dispose()))
    }
}

/// `[GlobalCleanup] Cleanup`.
impl Drop for NeutralSourceBenchmarks {
    fn drop(&mut self) {
        self.flat.dispose();
        self.tree.dispose();
        (self.flat_lifetime)();
        (self.tree_lifetime)();
    }
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("treedatagrid", "NeutralSourceBenchmarks");
    for neutral_source in [false, true] {
        let parameters = format!("NeutralSource={neutral_source}");
        class.benchmark(
            "create_source_and_presentation",
            parameters.clone(),
            move || NeutralSourceBenchmarks::new(neutral_source),
            |b| b.create_source_and_presentation(),
        );
        class.benchmark(
            "sort_thousand_rows",
            parameters.clone(),
            move || NeutralSourceBenchmarks::new(neutral_source),
            |b| b.sort_thousand_rows(),
        );
        class.benchmark(
            "expand_collapse_thousand_rows",
            parameters,
            move || NeutralSourceBenchmarks::new(neutral_source),
            |b| b.expand_collapse_thousand_rows(),
        );
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn neutral_source_benchmarks() {
        ferroui_benchmarks::harness::smoke_class(super::register, "NeutralSourceBenchmarks");
    }
}

//! `TreeDataGridFirstLayoutTests.cs`: the measure counts of the first layout of a grid of 10 000
//! rows and six columns, and the final measure of constrained auto cells after recycling.
//!
//! The reference subscribes to `LayoutInvalidated` of the source's column list; the port's source
//! holds core columns, and the column list that lays them out is the grid's (`Columns`).

use super::support::{run_jobs, scroll_viewer_template, start_app, test_window};
use crate::models::{CellRef, ColumnRef, IColumns, TextColumn, TextColumnOptions};
use crate::primitives::{
    TreeDataGridCellImpl, TreeDataGridCellsPresenter, TreeDataGridCellsPresenterImpl,
    TreeDataGridCellsPresenterImplExt, TreeDataGridColumnHeader, TreeDataGridColumnHeadersPresenter,
    TreeDataGridElementFactory, TreeDataGridElementFactoryImpl, TreeDataGridRow, TreeDataGridRowImpl,
    TreeDataGridRowsPresenter, TreeDataGridTextCell,
};
use crate::{TreeDataGrid, TreeDataGridImpl, TreeDataGridSourceRef};
use ferroui_base::collections::FerroList;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::data::TemplateBinding;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{Layoutable, LayoutableImpl, LayoutableImplExt};
use ferroui_base::styling::{Selectors, Setter, Style};
use ferroui_base::{ferro_class, instantiate, BoxedValue, FerroObjectImpl, FerroProperty, Ref, Size, StyledElementImpl, Vector, VisualImpl};
use ferroui_controls::primitives::{ScrollBarVisibility, TemplatedControl, TemplatedControlImpl};
use ferroui_controls::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions, IControlTemplate};
use ferroui_controls::{
    ButtonImpl, ContentControlImpl, Control, ControlImpl, Dock, DockPanel, GridLength, GridUnitType, ScrollViewer,
};
use ferroui_treedatagrid_core::model_collection::Models;
use ferroui_treedatagrid_core::models::{IColumnOf, RowRef};
use ferroui_treedatagrid_core::{Accessor, FlatTreeDataGridSource};
use std::any::Any;
use std::cell::Cell;
use std::rc::Rc;

// ---------------------------------------------------------------------------------------------
// Model and counters
// ---------------------------------------------------------------------------------------------

struct RowModel {
    #[allow(dead_code)]
    id: i32,
    title: String,
}

impl PartialEq for RowModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ColumnLayout {
    Fixed,
    Auto,
    Star,
    Mixed,
}

#[derive(Default)]
struct LayoutCounters {
    grid_measures: Cell<i32>,
    grid_arranges: Cell<i32>,
    rows_presenter_measures: Cell<i32>,
    rows_presenter_arranges: Cell<i32>,
    headers_presenter_measures: Cell<i32>,
    headers_presenter_arranges: Cell<i32>,
    cells_presenter_measures: Cell<i32>,
    cells_presenter_arranges: Cell<i32>,
    cell_presenter_measures_with_same_constraint: Cell<i32>,
    cell_presenter_measures_from_finite_to_infinite_width: Cell<i32>,
    cell_presenter_measures_from_finite_to_wider_width: Cell<i32>,
    row_measures: Cell<i32>,
    row_arranges: Cell<i32>,
    cell_measures: Cell<i32>,
    cell_arranges: Cell<i32>,
    cell_measures_with_infinite_width: Cell<i32>,
    cell_measures_with_infinite_height: Cell<i32>,
    header_measures: Cell<i32>,
    header_arranges: Cell<i32>,
    column_layout_invalidations: Cell<i32>,
}

fn increment(counter: &Cell<i32>) {
    counter.set(counter.get() + 1);
}

impl LayoutCounters {
    fn reset(&self) {
        for counter in [
            &self.grid_measures,
            &self.grid_arranges,
            &self.rows_presenter_measures,
            &self.rows_presenter_arranges,
            &self.headers_presenter_measures,
            &self.headers_presenter_arranges,
            &self.cells_presenter_measures,
            &self.cells_presenter_arranges,
            &self.cell_presenter_measures_with_same_constraint,
            &self.cell_presenter_measures_from_finite_to_infinite_width,
            &self.cell_presenter_measures_from_finite_to_wider_width,
            &self.row_measures,
            &self.row_arranges,
            &self.cell_measures,
            &self.cell_arranges,
            &self.cell_measures_with_infinite_width,
            &self.cell_measures_with_infinite_height,
            &self.header_measures,
            &self.header_arranges,
            &self.column_layout_invalidations,
        ] {
            counter.set(0);
        }
    }
}

impl std::fmt::Display for LayoutCounters {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Grid={}/{}; RowsPresenter={}/{}; HeadersPresenter={}/{}; CellsPresenter={}/{}; \
             CellConstraints=Same:{},FiniteToInf:{},FiniteToWider:{}; Rows={}/{}; Cells={}/{} (InfW={},InfH={}); \
             Headers={}/{}; ColumnInvalidations={}",
            self.grid_measures.get(),
            self.grid_arranges.get(),
            self.rows_presenter_measures.get(),
            self.rows_presenter_arranges.get(),
            self.headers_presenter_measures.get(),
            self.headers_presenter_arranges.get(),
            self.cells_presenter_measures.get(),
            self.cells_presenter_arranges.get(),
            self.cell_presenter_measures_with_same_constraint.get(),
            self.cell_presenter_measures_from_finite_to_infinite_width.get(),
            self.cell_presenter_measures_from_finite_to_wider_width.get(),
            self.row_measures.get(),
            self.row_arranges.get(),
            self.cell_measures.get(),
            self.cell_arranges.get(),
            self.cell_measures_with_infinite_width.get(),
            self.cell_measures_with_infinite_height.get(),
            self.header_measures.get(),
            self.header_arranges.get(),
            self.column_layout_invalidations.get()
        )
    }
}

// ---------------------------------------------------------------------------------------------
// Counting controls
// ---------------------------------------------------------------------------------------------

/// `CountingTreeDataGrid`.
#[repr(C)]
struct CountingTreeDataGrid {
    base: TreeDataGrid,
    counters: Rc<LayoutCounters>,
}

ferro_class!(CountingTreeDataGrid: TreeDataGrid);
ferroui_base::ferro_impl_classes!(
    CountingTreeDataGrid: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    TreeDataGridImpl
);

impl LayoutableImpl for CountingTreeDataGrid {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        increment(&this.counters.grid_measures);
        Self::parent_measure_override(this, available_size)
    }

    fn arrange_override(this: &Self, final_size: Size) -> Size {
        increment(&this.counters.grid_arranges);
        Self::parent_arrange_override(this, final_size)
    }
}

impl CountingTreeDataGrid {
    fn new(counters: Rc<LayoutCounters>) -> Ref<Self> {
        instantiate(Self { base: TreeDataGrid::construct(), counters })
    }
}

/// `CountingRowsPresenter`.
#[repr(C)]
struct CountingRowsPresenter {
    base: TreeDataGridRowsPresenter,
    counters: Rc<LayoutCounters>,
}

ferro_class!(CountingRowsPresenter: TreeDataGridRowsPresenter);
ferroui_base::ferro_impl_classes!(
    CountingRowsPresenter: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

impl LayoutableImpl for CountingRowsPresenter {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        increment(&this.counters.rows_presenter_measures);
        Self::parent_measure_override(this, available_size)
    }

    fn arrange_override(this: &Self, final_size: Size) -> Size {
        increment(&this.counters.rows_presenter_arranges);
        Self::parent_arrange_override(this, final_size)
    }
}

impl CountingRowsPresenter {
    fn new(counters: Rc<LayoutCounters>) -> Ref<Self> {
        instantiate(Self { base: TreeDataGridRowsPresenter::construct(), counters })
    }
}

/// `CountingColumnHeadersPresenter`.
#[repr(C)]
struct CountingColumnHeadersPresenter {
    base: TreeDataGridColumnHeadersPresenter,
    counters: Rc<LayoutCounters>,
}

ferro_class!(CountingColumnHeadersPresenter: TreeDataGridColumnHeadersPresenter);
ferroui_base::ferro_impl_classes!(
    CountingColumnHeadersPresenter: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

impl LayoutableImpl for CountingColumnHeadersPresenter {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        increment(&this.counters.headers_presenter_measures);
        Self::parent_measure_override(this, available_size)
    }

    fn arrange_override(this: &Self, final_size: Size) -> Size {
        increment(&this.counters.headers_presenter_arranges);
        Self::parent_arrange_override(this, final_size)
    }
}

impl CountingColumnHeadersPresenter {
    fn new(counters: Rc<LayoutCounters>) -> Ref<Self> {
        instantiate(Self { base: TreeDataGridColumnHeadersPresenter::construct(), counters })
    }
}

/// `CountingCellsPresenter`.
#[repr(C)]
struct CountingCellsPresenter {
    base: TreeDataGridCellsPresenter,
    counters: Rc<LayoutCounters>,
}

ferro_class!(CountingCellsPresenter: TreeDataGridCellsPresenter);
ferroui_base::ferro_impl_classes!(
    CountingCellsPresenter: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

impl LayoutableImpl for CountingCellsPresenter {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        increment(&this.counters.cells_presenter_measures);
        Self::parent_measure_override(this, available_size)
    }

    fn arrange_override(this: &Self, final_size: Size) -> Size {
        increment(&this.counters.cells_presenter_arranges);
        Self::parent_arrange_override(this, final_size)
    }
}

impl TreeDataGridCellsPresenterImpl for CountingCellsPresenter {
    fn measure_element(this: &Self, index: i32, element: &Ref<Control>, available_size: Size) -> Size {
        if let Some(previous) = element.previous_measure() {
            if previous == available_size {
                increment(&this.counters.cell_presenter_measures_with_same_constraint);
            } else if previous.width.is_finite() && available_size.width == f64::INFINITY {
                increment(&this.counters.cell_presenter_measures_from_finite_to_infinite_width);
            } else if previous.width.is_finite() && available_size.width > previous.width {
                increment(&this.counters.cell_presenter_measures_from_finite_to_wider_width);
            }
        }

        Self::parent_measure_element(this, index, element, available_size)
    }
}

impl CountingCellsPresenter {
    fn new(counters: Rc<LayoutCounters>) -> Ref<Self> {
        instantiate(Self { base: TreeDataGridCellsPresenter::construct(), counters })
    }
}

/// `CountingRow`.
#[repr(C)]
struct CountingRow {
    base: TreeDataGridRow,
    counters: Rc<LayoutCounters>,
}

ferro_class!(CountingRow: TreeDataGridRow);
ferroui_base::ferro_impl_classes!(
    CountingRow: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    TreeDataGridRowImpl
);

impl LayoutableImpl for CountingRow {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        increment(&this.counters.row_measures);
        Self::parent_measure_override(this, available_size)
    }

    fn arrange_override(this: &Self, final_size: Size) -> Size {
        increment(&this.counters.row_arranges);
        Self::parent_arrange_override(this, final_size)
    }
}

impl CountingRow {
    fn new(counters: Rc<LayoutCounters>) -> Ref<Self> {
        instantiate(Self { base: TreeDataGridRow::construct(), counters })
    }
}

/// `CountingTextCell`: a text cell whose text wraps in lines of 24 pixels, at 4 pixels a
/// character plus 8.
#[repr(C)]
struct CountingTextCell {
    base: TreeDataGridTextCell,
    counters: Rc<LayoutCounters>,
}

ferro_class!(CountingTextCell: TreeDataGridTextCell);
ferroui_base::ferro_impl_classes!(
    CountingTextCell: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    TreeDataGridCellImpl
);

impl LayoutableImpl for CountingTextCell {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        increment(&this.counters.cell_measures);
        if available_size.width == f64::INFINITY {
            increment(&this.counters.cell_measures_with_infinite_width);
        }
        if available_size.height == f64::INFINITY {
            increment(&this.counters.cell_measures_with_infinite_height);
        }
        let width = (this.value().map_or(0, |value| value.chars().count()) * 4 + 8) as f64;
        let lines = if available_size.width.is_finite() && available_size.width > 0.0 {
            (width / available_size.width).ceil()
        } else {
            1.0
        };
        Size::new(width.min(available_size.width), lines * 24.0)
    }

    fn arrange_override(this: &Self, final_size: Size) -> Size {
        increment(&this.counters.cell_arranges);
        final_size
    }
}

impl CountingTextCell {
    fn new(counters: Rc<LayoutCounters>) -> Ref<Self> {
        instantiate(Self { base: TreeDataGridTextCell::construct(), counters })
    }
}

/// `CountingColumnHeader`: a header 24 pixels high, at 4 pixels a character plus 8 wide.
#[repr(C)]
struct CountingColumnHeader {
    base: TreeDataGridColumnHeader,
    counters: Rc<LayoutCounters>,
}

ferro_class!(CountingColumnHeader: TreeDataGridColumnHeader);
ferroui_base::ferro_impl_classes!(
    CountingColumnHeader: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl,
    ButtonImpl
);

impl LayoutableImpl for CountingColumnHeader {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        increment(&this.counters.header_measures);
        let header = this.header().map(|header| ValueTypes::to_display_string(Some(&header))).unwrap_or_default();
        let width = (header.chars().count() * 4 + 8) as f64;
        Size::new(width.min(available_size.width), 24.0)
    }

    fn arrange_override(this: &Self, final_size: Size) -> Size {
        increment(&this.counters.header_arranges);
        final_size
    }
}

impl CountingColumnHeader {
    fn new(counters: Rc<LayoutCounters>) -> Ref<Self> {
        instantiate(Self { base: TreeDataGridColumnHeader::construct(), counters })
    }
}

/// `CountingElementFactory`.
struct CountingElementFactory {
    counters: Rc<LayoutCounters>,
}

impl TreeDataGridElementFactoryImpl for CountingElementFactory {
    fn create_element(&self, factory: &TreeDataGridElementFactory, data: &dyn Any) -> Ref<Control> {
        if data.is::<CellRef>() {
            CountingTextCell::new(self.counters.clone()).upcast()
        } else if data.is::<ColumnRef>() {
            CountingColumnHeader::new(self.counters.clone()).upcast()
        } else if data.is::<RowRef>() {
            CountingRow::new(self.counters.clone()).upcast()
        } else {
            factory.default_create_element(data)
        }
    }

    fn get_data_recycle_key(&self, factory: &TreeDataGridElementFactory, data: &dyn Any) -> String {
        if data.is::<CellRef>() {
            "counting-cell".to_string()
        } else if data.is::<ColumnRef>() {
            "counting-header".to_string()
        } else if data.is::<RowRef>() {
            "counting-row".to_string()
        } else {
            factory.default_data_recycle_key(data)
        }
    }

    fn get_element_recycle_key(&self, factory: &TreeDataGridElementFactory, element: &Control) -> String {
        if element.downcast_ref::<CountingTextCell>().is_some() {
            "counting-cell".to_string()
        } else if element.downcast_ref::<CountingColumnHeader>().is_some() {
            "counting-header".to_string()
        } else if element.downcast_ref::<CountingRow>().is_some() {
            "counting-row".to_string()
        } else {
            factory.default_element_recycle_key(element)
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Templates
// ---------------------------------------------------------------------------------------------

fn template_bind(target: &Control, target_property: &'static FerroProperty, source_property: &'static FerroProperty) {
    target.bind_binding(target_property, &*TemplateBinding::new(source_property));
}

fn tree_data_grid_template(counters: Rc<LayoutCounters>, cache_length: f64) -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::for_type::<TreeDataGrid>(move |_, scope| {
        let headers_presenter = CountingColumnHeadersPresenter::new(counters.clone());
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

        let rows_presenter = CountingRowsPresenter::new(counters.clone());
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

fn row_template(counters: Rc<LayoutCounters>) -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::for_type::<TreeDataGridRow>(move |_, scope| {
        let cells_presenter = CountingCellsPresenter::new(counters.clone());
        cells_presenter.set_name(Some("PART_CellsPresenter".to_string()));
        super::support::bind_cells_presenter(&cells_presenter);
        cells_presenter.register_in_name_scope(&**scope).upcast()
    })
}

fn row_style(counters: &Rc<LayoutCounters>, height: Option<f64>) -> Ref<Style> {
    let template: Option<Rc<dyn IControlTemplate>> = Some(row_template(counters.clone()));
    let mut setters = vec![Setter::new(TemplatedControl::template_property(), template)];
    if let Some(height) = height {
        setters.push(Setter::new(Layoutable::height_property(), height));
    }
    Style::with_setters(Selectors::is::<TreeDataGridRow>(), setters)
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

fn header(text: &str) -> Option<BoxedValue> {
    Some(Rc::new(text.to_string()) as BoxedValue)
}

fn assert_close(expected: f64, actual: f64) {
    assert!((expected - actual).abs() < 5e-7, "expected {expected}, got {actual}");
}

fn presentation_columns(target: &TreeDataGrid) -> Rc<dyn IColumns> {
    target.columns().expect("the columns of the grid")
}

fn rows_presenter(target: &TreeDataGrid) -> Ref<TreeDataGridRowsPresenter> {
    target.rows_presenter().expect("the rows presenter of the grid")
}

// ---------------------------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------------------------

fn first_layout_pass_counts(layout: ColumnLayout, cache_length: f64) {
    let _app = start_app();
    let counters = Rc::new(LayoutCounters::default());
    let items: Rc<FerroList<Rc<RowModel>>> = Rc::new(FerroList::from_items((0..10_000).map(|x| {
        Rc::new(RowModel { id: x, title: format!("Item {x}{}", "x".repeat((x % 20) as usize)) })
    })));
    let source = FlatTreeDataGridSource::new(items.clone() as Models<RowModel>);

    for i in 0..6 {
        let column = TextColumn::<RowModel, String>::new(
            header(&format!("Column {i}")),
            Accessor::new(move |x: &Rc<RowModel>| format!("{}-{i}", x.title)),
            Some(get_width(layout, i)),
            None,
        );
        source.columns().add(column as Rc<dyn IColumnOf<RowModel>>);
    }

    let target = CountingTreeDataGrid::new(counters.clone());
    target.set_element_factory(TreeDataGridElementFactory::with_impl(CountingElementFactory {
        counters: counters.clone(),
    }));
    target.set_source(Some(TreeDataGridSourceRef::of::<RowModel, _>(source.clone())));
    target.set_template(Some(tree_data_grid_template(counters.clone(), cache_length)));
    let root = test_window(Some(target.clone().upcast()), Some(Size::new(800.0, 500.0)));
    root.styles().add(row_style(&counters, Some(24.0)));

    let columns = presentation_columns(&target);
    let invalidation_counters = counters.clone();
    let token = columns.add_layout_invalidated(Rc::new(move || increment(&invalidation_counters.column_layout_invalidations)));
    counters.reset();
    root.update_layout();
    let after_initial_update = counters.to_string();
    run_jobs();
    let after_dispatcher = counters.to_string();
    root.update_layout();

    println!("Initial={after_initial_update}");
    println!("Dispatcher={after_dispatcher}");
    println!("Final={counters}");
    println!("Rows={}", rows_presenter(&target).get_realized_elements().len());
    println!(
        "Widths={}",
        (0..columns.count()).map(|x| columns.get(x).actual_width().to_string()).collect::<Vec<_>>().join(",")
    );

    assert!(!rows_presenter(&target).get_realized_elements().is_empty());
    for i in 0..columns.count() {
        assert!(columns.get(i).actual_width().is_finite());
    }

    let has_cache = cache_length != 0.0;
    let expected_cell_measures = match (layout, has_cache) {
        (ColumnLayout::Fixed, false) => 126,
        (ColumnLayout::Fixed, true) => 252,
        (ColumnLayout::Auto, false) => 252,
        (ColumnLayout::Auto, true) => 504,
        (ColumnLayout::Star, false) => 246,
        (ColumnLayout::Star, true) => 492,
        (ColumnLayout::Mixed, false) => 208,
        (ColumnLayout::Mixed, true) => 416,
    };
    let expected_rows_presenter_measures = match layout {
        ColumnLayout::Fixed => 2,
        ColumnLayout::Auto => 3,
        ColumnLayout::Star => 3,
        ColumnLayout::Mixed => 4,
    };
    let expected_column_invalidations = match layout {
        ColumnLayout::Fixed => 0,
        ColumnLayout::Auto => 2,
        ColumnLayout::Star => 2,
        ColumnLayout::Mixed => 3,
    };

    assert_eq!(expected_cell_measures, counters.cell_measures.get());
    assert_eq!(expected_rows_presenter_measures, counters.rows_presenter_measures.get());
    assert_eq!(expected_column_invalidations, counters.column_layout_invalidations.get());

    for i in 0..columns.count() {
        let expected_width = match layout {
            ColumnLayout::Fixed => 124.0,
            ColumnLayout::Auto => 120.0,
            ColumnLayout::Star => 800.0 / 6.0,
            ColumnLayout::Mixed => match i % 3 {
                0 => 120.0,
                1 => 156.0,
                _ => 124.0,
            },
        };

        assert_close(expected_width, columns.get(i).actual_width());
    }

    columns.remove_layout_invalidated(token);
}

#[test]
fn first_layout_pass_counts_fixed() {
    first_layout_pass_counts(ColumnLayout::Fixed, 0.0);
}

#[test]
fn first_layout_pass_counts_fixed_with_cache() {
    first_layout_pass_counts(ColumnLayout::Fixed, 0.5);
}

#[test]
fn first_layout_pass_counts_auto() {
    first_layout_pass_counts(ColumnLayout::Auto, 0.0);
}

#[test]
fn first_layout_pass_counts_auto_with_cache() {
    first_layout_pass_counts(ColumnLayout::Auto, 0.5);
}

#[test]
fn first_layout_pass_counts_star() {
    first_layout_pass_counts(ColumnLayout::Star, 0.0);
}

#[test]
fn first_layout_pass_counts_star_with_cache() {
    first_layout_pass_counts(ColumnLayout::Star, 0.5);
}

#[test]
fn first_layout_pass_counts_mixed() {
    first_layout_pass_counts(ColumnLayout::Mixed, 0.0);
}

#[test]
fn first_layout_pass_counts_mixed_with_cache() {
    first_layout_pass_counts(ColumnLayout::Mixed, 0.5);
}

fn assert_constrained_rows(target: &TreeDataGrid, column: &ColumnRef) {
    assert_eq!(80.0, column.actual_width());

    let rows: Vec<Ref<TreeDataGridRow>> = rows_presenter(target)
        .get_realized_elements()
        .into_iter()
        .map(|element| element.cast::<TreeDataGridRow>().expect("a row"))
        .collect();
    assert!(!rows.is_empty());
    assert!(rows.iter().enumerate().all(|(i, row)| !rows[..i].contains(row)));

    for row in &rows {
        let cell = row.try_get_cell(0).and_then(|cell| cell.cast::<CountingTextCell>()).expect("a counting text cell");
        let constraint = cell.previous_measure().expect("the cell was measured");
        assert_eq!(80.0, constraint.width);
        assert!(cell.desired_size().height > 24.0);
        assert_eq!(cell.desired_size().height, row.desired_size().height);
    }
}

fn constrained_auto_cells_keep_final_measure_after_vertical_recycling(cache_length: f64) {
    let _app = start_app();
    let counters = Rc::new(LayoutCounters::default());
    let items: Rc<FerroList<Rc<RowModel>>> = Rc::new(FerroList::from_items(
        (0..1_000).map(|x| Rc::new(RowModel { id: x, title: format!("Item {x} {}", "x".repeat(60)) })),
    ));
    let source = FlatTreeDataGridSource::new(items.clone() as Models<RowModel>);
    let options = TextColumnOptions::new();
    options.set_min_width(GridLength::new(0.0, GridUnitType::Pixel));
    options.set_max_width(Some(GridLength::new(80.0, GridUnitType::Pixel)));
    let column = TextColumn::<RowModel, String>::new(
        header("Auto"),
        Accessor::new(|x: &Rc<RowModel>| x.title.clone()),
        Some(GridLength::AUTO),
        Some(Rc::new(options)),
    );
    source.columns().add(column as Rc<dyn IColumnOf<RowModel>>);

    let target = CountingTreeDataGrid::new(counters.clone());
    target.set_element_factory(TreeDataGridElementFactory::with_impl(CountingElementFactory {
        counters: counters.clone(),
    }));
    target.set_source(Some(TreeDataGridSourceRef::of::<RowModel, _>(source.clone())));
    target.set_template(Some(tree_data_grid_template(counters.clone(), cache_length)));
    let root = test_window(Some(target.clone().upcast()), Some(Size::new(240.0, 160.0)));
    root.styles().add(row_style(&counters, None));

    root.update_layout();
    run_jobs();
    root.update_layout();
    let column = presentation_columns(&target).get(0);
    assert_constrained_rows(&target, &column);

    let initial_rows: Vec<Ref<TreeDataGridRow>> = rows_presenter(&target)
        .get_realized_elements()
        .into_iter()
        .map(|element| element.cast::<TreeDataGridRow>().expect("a row"))
        .collect();
    let scroll = target.scroll().expect("the scroll viewer of the grid");
    scroll.set_offset(Vector::new(0.0, 1_000.0_f64.min(scroll.extent().height - scroll.viewport().height)));
    root.update_layout();
    run_jobs();
    root.update_layout();

    assert_constrained_rows(&target, &column);
    assert!(rows_presenter(&target)
        .get_realized_elements()
        .into_iter()
        .filter_map(|element| element.cast::<TreeDataGridRow>())
        .any(|row| initial_rows.contains(&row)));
}

#[test]
fn constrained_auto_cells_keep_final_measure_after_vertical_recycling_without_cache() {
    constrained_auto_cells_keep_final_measure_after_vertical_recycling(0.0);
}

#[test]
fn constrained_auto_cells_keep_final_measure_after_vertical_recycling_with_cache() {
    constrained_auto_cells_keep_final_measure_after_vertical_recycling(0.5);
}

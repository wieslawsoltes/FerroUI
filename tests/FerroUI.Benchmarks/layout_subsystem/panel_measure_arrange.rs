//! The measure and the arrange of one panel over many children: the panel and each of its
//! children are marked as not measured and not arranged, and the layout manager runs a pass. The
//! children are leaves of a fixed size, so the time is the panel's own: its walk of the children,
//! what it reads of each, and the measure and arrange of a layoutable (sections 9.9 and 9.28 of
//! the performance notes: the values read in place).
//!
//! - `stack_vertical`, `stack_horizontal`: a stack panel in each orientation;
//! - `grid`: a grid of 25 columns and 40 rows, the definitions in turn of a length in pixels, of
//!   the size of the content and of a share of the rest, one child in each cell;
//! - `grid_spans`: the same grid with a child in five over two columns and a child in seven over
//!   two rows, which takes the grid through the distribution of the spans;
//! - `dock`: a dock panel whose children are docked in turn to each side;
//! - `wrap`: a wrap panel of 16 lines;
//! - `canvas`: a canvas with a left and a top on each child;
//! - `uniform_grid`: a uniform grid of 40 columns.

use super::fixture::{lay_out_all, leaves, show};
use crate::harness::Registry;
use crate::trees::fixture::Fixture;
use ferroui_base::layout::Orientation;
use ferroui_base::Ref;
use ferroui_controls::primitives::UniformGrid;
use ferroui_controls::{
    Border, Canvas, ColumnDefinition, ColumnDefinitions, Control, Dock, DockPanel, Grid, GridLength, RowDefinition,
    RowDefinitions, StackPanel, WrapPanel,
};

/// The children of a panel.
const CHILDREN: usize = 1_000;

const GRID_COLUMNS: usize = 25;
const GRID_ROWS: usize = CHILDREN / GRID_COLUMNS;

/// The sides the children of the dock panel take in turn.
const SIDES: [Dock; 4] = [Dock::Left, Dock::Top, Dock::Right, Dock::Bottom];

pub struct PanelMeasureArrange {
    fixture: Fixture,
    /// The panel and its children.
    controls: Vec<Ref<Control>>,
}

/// The length of the definition at `index`: in turn `pixels`, the size of the content and a share
/// of the rest.
fn length(index: usize, pixels: f64) -> GridLength {
    match index % 3 {
        0 => GridLength::from_pixels(pixels),
        1 => GridLength::auto(),
        _ => GridLength::star(),
    }
}

impl PanelMeasureArrange {
    /// Shows `panel`, which holds `children`, and lays it out.
    fn shown(fixture: Fixture, panel: Ref<Control>, children: Vec<Ref<Border>>) -> Self {
        show(&fixture, &panel);
        let mut controls = Vec::with_capacity(children.len() + 1);
        controls.push(panel);
        controls.extend(children.into_iter().map(|child| child.upcast::<Control>()));
        Self { fixture, controls }
    }

    fn stack(orientation: Orientation) -> Self {
        let fixture = Fixture::new();
        let children = leaves(CHILDREN, 10.0, 4.0, &fixture.brushes);
        let panel = StackPanel::new();
        panel.set_orientation(orientation);
        for child in &children {
            panel.children().add(child);
        }
        Self::shown(fixture, panel.upcast(), children)
    }

    fn grid(spans: bool) -> Self {
        let fixture = Fixture::new();
        let children = leaves(CHILDREN, 10.0, 4.0, &fixture.brushes);
        let panel = Grid::new();
        panel.set_column_definitions(ColumnDefinitions::from_items(
            (0..GRID_COLUMNS).map(|index| ColumnDefinition::with_width(length(index, 12.0))),
        ));
        panel.set_row_definitions(RowDefinitions::from_items(
            (0..GRID_ROWS).map(|index| RowDefinition::with_height(length(index, 6.0))),
        ));
        for (index, child) in children.iter().enumerate() {
            let column = index % GRID_COLUMNS;
            let row = index / GRID_COLUMNS;
            Grid::set_column(child, column as i32);
            Grid::set_row(child, row as i32);
            if spans && index % 5 == 0 && column + 1 < GRID_COLUMNS {
                Grid::set_column_span(child, 2);
            }
            if spans && index % 7 == 0 && row + 1 < GRID_ROWS {
                Grid::set_row_span(child, 2);
            }
            panel.children().add(child);
        }
        Self::shown(fixture, panel.upcast(), children)
    }

    fn dock() -> Self {
        let fixture = Fixture::new();
        // Small enough for every child to get its side of the client area of the root.
        let children = leaves(CHILDREN, 2.0, 1.0, &fixture.brushes);
        let panel = DockPanel::new();
        for (index, child) in children.iter().enumerate() {
            DockPanel::set_dock(child, SIDES[index % SIDES.len()]);
            panel.children().add(child);
        }
        Self::shown(fixture, panel.upcast(), children)
    }

    fn wrap() -> Self {
        let fixture = Fixture::new();
        // 64 to a line of the 1280 of the root.
        let children = leaves(CHILDREN, 20.0, 10.0, &fixture.brushes);
        let panel = WrapPanel::new();
        panel.set_orientation(Orientation::Horizontal);
        for child in &children {
            panel.children().add(child);
        }
        Self::shown(fixture, panel.upcast(), children)
    }

    fn canvas() -> Self {
        let fixture = Fixture::new();
        let children = leaves(CHILDREN, 10.0, 4.0, &fixture.brushes);
        let panel = Canvas::new();
        for (index, child) in children.iter().enumerate() {
            Canvas::set_left(child, (index % 100) as f64 * 12.0);
            Canvas::set_top(child, (index / 100) as f64 * 6.0);
            panel.children().add(child);
        }
        Self::shown(fixture, panel.upcast(), children)
    }

    fn uniform_grid() -> Self {
        let fixture = Fixture::new();
        let children = leaves(CHILDREN, 10.0, 4.0, &fixture.brushes);
        let panel = UniformGrid::new();
        panel.set_columns(40);
        for child in &children {
            panel.children().add(child);
        }
        Self::shown(fixture, panel.upcast(), children)
    }

    fn lay_out(&mut self) {
        lay_out_all(&self.fixture, &self.controls);
    }
}

type Setup = fn() -> PanelMeasureArrange;

const PANELS: [(&str, Setup); 8] = [
    ("stack_vertical", || PanelMeasureArrange::stack(Orientation::Vertical)),
    ("stack_horizontal", || PanelMeasureArrange::stack(Orientation::Horizontal)),
    ("grid", || PanelMeasureArrange::grid(false)),
    ("grid_spans", || PanelMeasureArrange::grid(true)),
    ("dock", PanelMeasureArrange::dock),
    ("wrap", PanelMeasureArrange::wrap),
    ("canvas", PanelMeasureArrange::canvas),
    ("uniform_grid", PanelMeasureArrange::uniform_grid),
];

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("layout_subsystem", "PanelMeasureArrange").min_iteration_time(200);
    let parameters = format!("Children={CHILDREN}");
    for (name, setup) in PANELS {
        class.benchmark(name, parameters.clone(), setup, PanelMeasureArrange::lay_out);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn smoke() {
        crate::harness::smoke_class(super::register, "PanelMeasureArrange");
    }
}

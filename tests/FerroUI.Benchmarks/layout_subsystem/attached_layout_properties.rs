//! The attached properties that place a child in its panel: one child in ten of a panel of a
//! thousand gets a new place, and the layout manager runs a pass. The change of an attached
//! property on a child invalidates its panel: its measure for a cell of a grid and a side of a
//! dock panel, its arrange alone for a position on a canvas. The panel then visits every child,
//! and those whose place and constraint did not change are left alone (section 9.21 of the
//! performance notes; section 9.28: the attached values a panel reads of each child).
//!
//! - `grid_cell`: the column and the row of the child change, in a grid of 25 columns and 40 rows
//!   of a length in pixels;
//! - `dock`: the side the child is docked to changes;
//! - `canvas_position`: the left and the top of the child change.
//!
//! Each invocation moves the children or brings them back, in turn.

use super::fixture::{leaves, show};
use crate::harness::Registry;
use crate::trees::fixture::Fixture;
use ferroui_base::Ref;
use ferroui_controls::{
    Border, Canvas, ColumnDefinition, ColumnDefinitions, Control, Dock, DockPanel, Grid, GridLength, Panel,
    RowDefinition, RowDefinitions,
};
use std::cell::Cell;

const CHILDREN: usize = 1_000;

/// One child in this many gets a new place.
const CHANGED_STEP: usize = 10;

const GRID_COLUMNS: usize = 25;
const GRID_ROWS: usize = CHILDREN / GRID_COLUMNS;

const SIDES: [Dock; 4] = [Dock::Left, Dock::Top, Dock::Right, Dock::Bottom];

/// Gives the child at an index of its panel its place: the one it starts at, or the one it is
/// moved to.
type Place = fn(child: &Ref<Border>, index: usize, moved: bool);

pub struct AttachedLayoutProperties {
    fixture: Fixture,
    /// The children that get a new place, each with its index in the panel.
    changed: Vec<(usize, Ref<Border>)>,
    place: Place,
    moved: Cell<bool>,
}

fn grid() -> Ref<Panel> {
    let grid = Grid::new();
    grid.set_column_definitions(ColumnDefinitions::from_items(
        (0..GRID_COLUMNS).map(|_| ColumnDefinition::with_width(GridLength::from_pixels(12.0))),
    ));
    grid.set_row_definitions(RowDefinitions::from_items(
        (0..GRID_ROWS).map(|_| RowDefinition::with_height(GridLength::from_pixels(6.0))),
    ));
    grid.upcast()
}

fn dock_panel() -> Ref<Panel> {
    DockPanel::new().upcast()
}

fn canvas() -> Ref<Panel> {
    Canvas::new().upcast()
}

/// The next column and the next row, around the edges of the grid.
fn grid_cell(child: &Ref<Border>, index: usize, moved: bool) {
    let shift = usize::from(moved);
    Grid::set_column(child, ((index % GRID_COLUMNS + shift) % GRID_COLUMNS) as i32);
    Grid::set_row(child, ((index / GRID_COLUMNS + shift) % GRID_ROWS) as i32);
}

/// The next side.
fn dock_side(child: &Ref<Border>, index: usize, moved: bool) {
    DockPanel::set_dock(child, SIDES[(index + usize::from(moved)) % SIDES.len()]);
}

/// Three to the right and three down.
fn canvas_position(child: &Ref<Border>, index: usize, moved: bool) {
    let shift = if moved { 3.0 } else { 0.0 };
    Canvas::set_left(child, (index % 100) as f64 * 12.0 + shift);
    Canvas::set_top(child, (index / 100) as f64 * 6.0 + shift);
}

impl AttachedLayoutProperties {
    /// A panel of leaves of `width` x `height`, each at the place `place` starts it at.
    fn new(panel: fn() -> Ref<Panel>, width: f64, height: f64, place: Place) -> Self {
        let fixture = Fixture::new();
        let panel = panel();
        let children = leaves(CHILDREN, width, height, &fixture.brushes);
        for (index, child) in children.iter().enumerate() {
            place(child, index, false);
            panel.children().add(child);
        }
        show(&fixture, &panel.upcast::<Control>());
        let changed = children.into_iter().enumerate().step_by(CHANGED_STEP).collect();
        Self { fixture, changed, place, moved: Cell::new(false) }
    }

    fn change(&mut self) {
        let moved = !self.moved.get();
        self.moved.set(moved);
        for (index, child) in &self.changed {
            (self.place)(child, *index, moved);
        }
        self.fixture.lay_out();
    }
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("layout_subsystem", "AttachedLayoutProperties").min_iteration_time(200);
    let parameters = format!("Children={CHILDREN}");
    class.benchmark(
        "grid_cell",
        parameters.clone(),
        || AttachedLayoutProperties::new(grid, 10.0, 4.0, grid_cell),
        AttachedLayoutProperties::change,
    );
    // Small enough for every child to get its side of the client area of the root.
    class.benchmark(
        "dock",
        parameters.clone(),
        || AttachedLayoutProperties::new(dock_panel, 2.0, 1.0, dock_side),
        AttachedLayoutProperties::change,
    );
    class.benchmark(
        "canvas_position",
        parameters,
        || AttachedLayoutProperties::new(canvas, 10.0, 4.0, canvas_position),
        AttachedLayoutProperties::change,
    );
}

#[cfg(test)]
mod tests {
    #[test]
    fn smoke() {
        crate::harness::smoke_class(super::register, "AttachedLayoutProperties");
    }
}

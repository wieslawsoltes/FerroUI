//! A change of the size of the root, as when a window is resized: the width of the client area
//! goes from 1280 to 960 and back, the root is invalidated and the layout manager runs a pass.
//! How far the new size travels depends on the tree (sections 9.15 and 9.21 of the performance
//! notes: an element whose measure cannot depend on the constraint is not measured again, and a
//! child arranged where it was is not arranged again).
//!
//! - `wrap`: a wrap panel of a thousand leaves, whose lines break elsewhere: 64 leaves to a line,
//!   then 48. The measure of a leaf of a fixed size holds at the new constraint; each leaf gets a
//!   new place;
//! - `stretch_stack`: a stack of a thousand borders that stretch across it: the constraint of each
//!   changes, and each is arranged at the new width;
//! - `star_grid`: a grid of four columns that share the width and 250 rows as tall as their
//!   content, with a border that stretches in each cell: the grid resolves its columns again and
//!   every cell gets a new width;
//! - `canvas`: a canvas of a thousand leaves, which gives its children no constraint and no size:
//!   the pass that a tree independent of the size of the root costs.
//!
//! The root is arranged at the size its child desires, so the trees of `stretch_stack` and
//! `star_grid` begin with a line of 64 leaves that wraps: it desires the whole width, at 1280 and
//! at 960, and the stack below the root is arranged as wide as it was measured.

use super::fixture::{leaves, show};
use crate::harness::Registry;
use crate::trees::fixture::{Brushes, Fixture};
use ferroui_base::layout::Orientation;
use ferroui_base::{Ref, Size};
use ferroui_controls::{
    Border, Canvas, ColumnDefinition, ColumnDefinitions, Control, Grid, GridLength, GridUnitType, RowDefinition,
    RowDefinitions, StackPanel, WrapPanel,
};
use std::cell::Cell;

/// The elements a benchmark resizes.
const LEAVES: usize = 1_000;

const WIDE: f64 = 1280.0;
const NARROW: f64 = 960.0;
const HEIGHT: f64 = 820.0;

/// Both widths are a whole number of leaves of this width.
const LEAF_WIDTH: f64 = 20.0;

const GRID_COLUMNS: usize = 4;

pub struct SizeChange {
    fixture: Fixture,
    flip: Cell<bool>,
}

/// A wrap panel of `count` leaves in lines that fill the width of the root.
fn wrap_of(count: usize, brushes: &Brushes) -> Ref<WrapPanel> {
    let panel = WrapPanel::new();
    panel.set_orientation(Orientation::Horizontal);
    for leaf in leaves(count, LEAF_WIDTH, 10.0, brushes) {
        panel.children().add(leaf);
    }
    panel
}

/// A border as tall as `height` that takes the width it is given.
fn stretching(height: f64, brushes: &Brushes) -> Ref<Border> {
    let border = Border::new();
    border.set_height(height);
    border.set_background(Some(brushes.tile.clone()));
    border
}

impl SizeChange {
    fn shown(fixture: Fixture, child: Ref<Control>) -> Self {
        show(&fixture, &child);
        Self { fixture, flip: Cell::new(false) }
    }

    /// A stack whose first child desires the whole width of the root.
    fn stack_below_full_line(brushes: &Brushes) -> Ref<StackPanel> {
        let stack = StackPanel::new();
        stack.children().add(wrap_of((WIDE / LEAF_WIDTH) as usize, brushes));
        stack
    }

    fn wrap() -> Self {
        let fixture = Fixture::new();
        let panel = wrap_of(LEAVES, &fixture.brushes);
        Self::shown(fixture, panel.upcast())
    }

    fn stretch_stack() -> Self {
        let fixture = Fixture::new();
        let stack = Self::stack_below_full_line(&fixture.brushes);
        for _ in 0..LEAVES {
            stack.children().add(stretching(2.0, &fixture.brushes));
        }
        Self::shown(fixture, stack.upcast())
    }

    fn star_grid() -> Self {
        let fixture = Fixture::new();
        let grid = Grid::new();
        grid.set_column_definitions(ColumnDefinitions::from_items((0..GRID_COLUMNS).map(|index| {
            // The second column takes two shares.
            let shares = if index == 1 { 2.0 } else { 1.0 };
            ColumnDefinition::with_width(GridLength::new(shares, GridUnitType::Star))
        })));
        grid.set_row_definitions(RowDefinitions::from_items(
            (0..LEAVES / GRID_COLUMNS).map(|_| RowDefinition::with_height(GridLength::auto())),
        ));
        for index in 0..LEAVES {
            let cell = stretching(4.0, &fixture.brushes);
            Grid::set_column(&cell, (index % GRID_COLUMNS) as i32);
            Grid::set_row(&cell, (index / GRID_COLUMNS) as i32);
            grid.children().add(cell);
        }
        let stack = Self::stack_below_full_line(&fixture.brushes);
        stack.children().add(grid);
        Self::shown(fixture, stack.upcast())
    }

    fn canvas() -> Self {
        let fixture = Fixture::new();
        let panel = Canvas::new();
        for (index, leaf) in leaves(LEAVES, 10.0, 4.0, &fixture.brushes).into_iter().enumerate() {
            Canvas::set_left(&leaf, (index % 100) as f64 * 12.0);
            Canvas::set_top(&leaf, (index / 100) as f64 * 6.0);
            panel.children().add(leaf);
        }
        Self::shown(fixture, panel.upcast())
    }

    fn flip(&self) -> bool {
        let value = !self.flip.get();
        self.flip.set(value);
        value
    }

    fn resize(&mut self) {
        let width = if self.flip() { NARROW } else { WIDE };
        self.fixture.root.set_client_size(Size::new(width, HEIGHT));
        self.fixture.root.invalidate_measure();
        self.fixture.lay_out();
    }
}

type Setup = fn() -> SizeChange;

const TREES: [(&str, Setup); 4] = [
    ("wrap", SizeChange::wrap),
    ("stretch_stack", SizeChange::stretch_stack),
    ("star_grid", SizeChange::star_grid),
    ("canvas", SizeChange::canvas),
];

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("layout_subsystem", "SizeChange").min_iteration_time(200);
    let parameters = format!("Leaves={LEAVES}");
    for (name, setup) in TREES {
        class.benchmark(name, parameters.clone(), setup, SizeChange::resize);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn smoke() {
        crate::harness::smoke_class(super::register, "SizeChange");
    }
}

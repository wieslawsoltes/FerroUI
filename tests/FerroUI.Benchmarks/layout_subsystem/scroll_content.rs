//! The content of a scroll viewer: a stack of a thousand rows, twenty thousand high, in a viewer
//! with the template of the Fluent theme (its scroll bars hidden, as a list that scrolls by
//! touch or by wheel has them). The viewer shows about forty rows; nothing is virtualized, so
//! every row is a child the pass visits (section 9.21 of the performance notes: the rows that did
//! not change are left alone).
//!
//! - `offset`: the offset of the viewer changes and the layout manager runs a pass: the presenter
//!   of the content arranges the content at its new place, and no row moves inside it;
//! - `viewport`: the height of the root changes and the layout manager runs a pass: the viewer and
//!   the parts of its template are measured and arranged at the new size; the content, measured
//!   without a limit on its height, is not measured again;
//! - `content_resized`: the height of the last row changes and the layout manager runs a pass:
//!   the extent of the viewer changes, the size the viewer desires does not, so the change stops
//!   at the presenter of the content.

use super::fixture::{leaves, show};
use crate::harness::Registry;
use crate::trees::fixture::Fixture;
use ferroui_base::{Ref, Size, Vector};
use ferroui_controls::primitives::ScrollBarVisibility;
use ferroui_controls::{Border, Control, ScrollViewer, StackPanel};
use std::cell::Cell;

const ROWS: usize = 1_000;
const ROW_HEIGHT: f64 = 20.0;

/// Two offsets inside the range of the viewer, each a whole number of rows.
const NEAR: f64 = 2_000.0;
const FAR: f64 = 6_000.0;

pub struct ScrollContent {
    fixture: Fixture,
    viewer: Ref<ScrollViewer>,
    last_row: Ref<Border>,
    flip: Cell<bool>,
}

impl ScrollContent {
    fn new() -> Self {
        let fixture = Fixture::new();
        let rows = leaves(ROWS, 200.0, ROW_HEIGHT, &fixture.brushes);
        let content = StackPanel::new();
        for row in &rows {
            content.children().add(row);
        }
        let viewer = ScrollViewer::new();
        viewer.set_content(Some(Control::boxed(&content)));
        viewer.set_vertical_scroll_bar_visibility(ScrollBarVisibility::Hidden);
        viewer.set_horizontal_scroll_bar_visibility(ScrollBarVisibility::Disabled);
        show(&fixture, &viewer.clone().upcast::<Control>());
        // The viewer learns its extent and its viewport in the pass and may ask for another.
        fixture.lay_out();
        assert!(
            viewer.extent().height > viewer.viewport().height && viewer.viewport().height > 0.0,
            "the scroll viewer has no template or does not scroll: extent {:?}, viewport {:?}",
            viewer.extent(),
            viewer.viewport()
        );
        viewer.set_offset(Vector::new(0.0, FAR));
        fixture.lay_out();
        assert_eq!(viewer.offset().y, FAR, "the scroll viewer keeps the offset it is given");
        let last_row = rows.last().expect("a row").clone();
        Self { fixture, viewer, last_row, flip: Cell::new(false) }
    }

    fn flip(&self) -> bool {
        let value = !self.flip.get();
        self.flip.set(value);
        value
    }

    fn offset(&mut self) {
        let offset = if self.flip() { NEAR } else { FAR };
        self.viewer.set_offset(Vector::new(0.0, offset));
        self.fixture.lay_out();
    }

    fn viewport(&mut self) {
        let height = if self.flip() { 600.0 } else { 820.0 };
        self.fixture.root.set_client_size(Size::new(1280.0, height));
        self.fixture.root.invalidate_measure();
        self.fixture.lay_out();
    }

    fn content_resized(&mut self) {
        let height = if self.flip() { ROW_HEIGHT + 4.0 } else { ROW_HEIGHT };
        self.last_row.set_height(height);
        self.fixture.lay_out();
    }
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("layout_subsystem", "ScrollContent").min_iteration_time(200);
    let parameters = format!("Rows={ROWS}");
    class.benchmark("offset", parameters.clone(), ScrollContent::new, ScrollContent::offset);
    class.benchmark("viewport", parameters.clone(), ScrollContent::new, ScrollContent::viewport);
    class.benchmark("content_resized", parameters, ScrollContent::new, ScrollContent::content_resized);
}

#[cfg(test)]
mod tests {
    #[test]
    fn smoke() {
        crate::harness::smoke_class(super::register, "ScrollContent");
    }
}

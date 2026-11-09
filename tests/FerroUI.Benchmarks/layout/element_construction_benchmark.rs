//! What building many small elements costs, stage by stage: the tiles of the still wall
//! (`samples/GauntletBenchmark`, `wall.rs`: a bordered tile around a stack of a label and a line of
//! a dot and a value, seven elements a tile), placed on a canvas. An addition: the still wall's
//! first frame takes a few seconds at its highest tier (58,696 tiles), where it is spent creating,
//! attaching, styling and laying out elements; this measures those stages for every change to them
//! (docs/porting/desktop-performance.md).
//!
//! - `create`: the elements of the tiles, their properties set, not attached;
//! - `create_and_attach`: the same, the canvas then attached to a styled root;
//! - `create_attach_and_lay_out`: the same and a layout pass (measure and arrange).

use crate::harness::Registry;
use crate::test_root::TestRoot;
use ferroui_base::layout::{Orientation, VerticalAlignment};
use ferroui_base::media::Brushes;
use ferroui_base::{CornerRadius, Ref, Thickness};
use ferroui_controls::testing::{NullRenderer, TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_controls::{Border, Canvas, StackPanel, TextBlock};
use std::hint::black_box;

/// The tile of the wall at tier 16 (5 x 4).
const TILE: (f64, f64) = (5.0, 4.0);

pub struct ElementConstructionBenchmark {
    tiles: usize,
    root: Ref<TestRoot>,
    _app: UnitTestApplicationScope,
}

impl ElementConstructionBenchmark {
    pub fn new(tiles: usize) -> Self {
        let app = UnitTestApplication::start(TestServices::styled_window());
        let root = TestRoot::with_global_styles(true, None);
        root.set_renderer(NullRenderer::new());
        root.layout_manager().execute_initial_layout_pass();
        Self { tiles, root, _app: app }
    }

    fn text(size: f64, text: &str) -> Ref<TextBlock> {
        let block = TextBlock::new();
        block.set_font_size(size);
        block.set_foreground(Some(Brushes::white()));
        block.set_text(Some(text));
        block
    }

    /// The wall's tiles on a canvas, as `WallView::new` builds them.
    fn build(&self) -> Ref<Canvas> {
        let (width, height) = TILE;
        let dot = height * 0.22;
        let wall = Canvas::new();
        for index in 0..self.tiles {
            let label = Self::text(height * 0.30, &format!("{index:05}"));
            let value = Self::text(height * 0.26, &format!("{:03}", index % 1000));
            let spot = Border::new();
            spot.set_width(dot);
            spot.set_height(dot);
            spot.set_corner_radius(CornerRadius::uniform(dot / 2.0));
            spot.set_background(Some(Brushes::red()));
            spot.set_vertical_alignment(VerticalAlignment::Center);
            let line = StackPanel::new();
            line.set_orientation(Orientation::Horizontal);
            line.set_spacing(dot * 0.5);
            line.children().add(&spot);
            line.children().add(&value);
            let content = StackPanel::new();
            content.children().add(&label);
            content.children().add(&line);
            let tile = Border::new();
            tile.set_width(width);
            tile.set_height(height);
            tile.set_corner_radius(CornerRadius::uniform(2.0));
            tile.set_padding(Thickness::uniform(height * 0.06));
            tile.set_background(Some(Brushes::blue()));
            tile.set_clip_to_bounds(true);
            tile.set_child(&content);
            let column = (index % 200) as f64;
            let row = (index / 200) as f64;
            Canvas::set_left(&tile, column * (width + 1.0));
            Canvas::set_top(&tile, row * (height + 1.0));
            wall.children().add(&tile);
        }
        wall
    }

    pub fn create(&self) -> usize {
        let wall = self.build();
        black_box(wall.children().count())
    }

    pub fn create_and_attach(&self) -> usize {
        let wall = self.build();
        self.root.set_child(&wall);
        let count = wall.children().count();
        self.root.set_child(None::<Ref<ferroui_controls::Control>>);
        count
    }

    pub fn create_attach_and_lay_out(&self) -> usize {
        let wall = self.build();
        self.root.set_child(&wall);
        self.root.layout_manager().execute_layout_pass();
        let count = wall.children().count();
        self.root.set_child(None::<Ref<ferroui_controls::Control>>);
        count
    }
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("layout", "ElementConstructionBenchmark").min_iteration_time(300);
    for tiles in [1_000usize, 10_000] {
        let parameters = format!("Tiles={tiles}");
        class.benchmark("create", parameters.clone(), move || ElementConstructionBenchmark::new(tiles), |b| b.create()).baseline();
        class.benchmark(
            "create_and_attach",
            parameters.clone(),
            move || ElementConstructionBenchmark::new(tiles),
            |b| b.create_and_attach(),
        );
        class.benchmark(
            "create_attach_and_lay_out",
            parameters.clone(),
            move || ElementConstructionBenchmark::new(tiles),
            |b| b.create_attach_and_lay_out(),
        );
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn smoke() {
        crate::harness::smoke_class(super::register, "ElementConstructionBenchmark");
    }
}

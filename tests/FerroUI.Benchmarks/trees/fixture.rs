//! The application, the root and the tiles of the tree benchmarks.

use crate::test_root::TestRoot;
use ferroui_base::layout::{Orientation, VerticalAlignment};
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::{Color, IBrush};
use ferroui_base::styling::ThemeVariant;
use ferroui_base::{CornerRadius, Ref, Size, Thickness};
use ferroui_controls::testing::{NullRenderer, TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_controls::{Application, Border, Canvas, Control, StackPanel, TextBlock};
use ferroui_themes_fluent::FluentTheme;
use std::rc::Rc;

/// The tiles of a benchmark, by default.
pub const TILES: usize = 2_000;

/// The brushes of the tiles.
pub struct Brushes {
    pub label: Rc<dyn IBrush>,
    pub tile: Rc<dyn IBrush>,
    pub dot: Rc<dyn IBrush>,
    pub other: Rc<dyn IBrush>,
}

impl Brushes {
    pub fn new() -> Self {
        let brush = |argb: u32| -> Rc<dyn IBrush> { Rc::new(ImmutableSolidColorBrush::new(Color::from_uint32(argb))) };
        Self { label: brush(0xFFFFFFFF), tile: brush(0xFF1E40AF), dot: brush(0xFFDC2626), other: brush(0xFF0F766E) }
    }
}

impl Default for Brushes {
    fn default() -> Self {
        Self::new()
    }
}

/// An application with the services of a desktop and a root of 1280 x 820 that renders nothing.
pub struct Fixture {
    pub root: Ref<TestRoot>,
    pub brushes: Brushes,
    _app: UnitTestApplicationScope,
}

impl Fixture {
    pub fn new() -> Self {
        let app = UnitTestApplication::start(
            TestServices::styled_window()
                .with_render_interface(Rc::new(ferroui_skia::PlatformRenderInterface::new(None, None)))
                .with_font_manager_impl(Rc::new(ferroui_skia::FontManagerImpl::new()))
                .with_text_shaper_impl(Rc::new(ferroui_harfbuzz::HarfBuzzTextShaper::new()))
                .with_theme(|| FluentTheme::new().as_style()),
        );
        Application::current().expect("the application").set_requested_theme_variant(Some(ThemeVariant::light()));
        let root = TestRoot::with_global_styles(true, None);
        root.set_renderer(NullRenderer::new());
        root.set_client_size(Size::new(1280.0, 820.0));
        root.layout_manager().execute_initial_layout_pass();
        Self { root, brushes: Brushes::new(), _app: app }
    }

    /// Shows `child` in the root, in place of what it showed.
    pub fn show(&self, child: Option<&Ref<Control>>) {
        match child {
            Some(child) => self.root.set_child(child),
            None => self.root.set_child(None::<Ref<Control>>),
        }
    }

    pub fn lay_out(&self) {
        self.root.layout_manager().execute_layout_pass();
    }
}

impl Default for Fixture {
    fn default() -> Self {
        Self::new()
    }
}

/// The parts of a tile: a bordered box around a label and a line of a dot and a value, seven
/// elements, as the tiles of a still wall.
pub struct Tile {
    pub tile: Ref<Border>,
    pub content: Ref<StackPanel>,
    pub name: Ref<TextBlock>,
    pub line: Ref<StackPanel>,
    pub spot: Ref<Border>,
    pub value: Ref<TextBlock>,
}

/// The elements of a tile, not yet put together.
pub fn tile_parts(index: usize, brushes: &Brushes) -> Tile {
    let (width, height) = (5.0, 4.0);
    let dot = height * 0.22;
    let name = TextBlock::new();
    name.set_font_size(height * 0.30);
    name.set_foreground(Some(brushes.label.clone()));
    name.set_text(Some(&format!("{index:05}")));
    let value = TextBlock::new();
    value.set_font_size(height * 0.26);
    value.set_foreground(Some(brushes.label.clone()));
    value.set_text(Some(&format!("{:03}", index % 1000)));
    let spot = Border::new();
    spot.set_width(dot);
    spot.set_height(dot);
    spot.set_corner_radius(CornerRadius::uniform(dot / 2.0));
    spot.set_background(Some(brushes.dot.clone()));
    spot.set_vertical_alignment(VerticalAlignment::Center);
    let line = StackPanel::new();
    line.set_orientation(Orientation::Horizontal);
    line.set_spacing(dot * 0.5);
    let content = StackPanel::new();
    let tile = Border::new();
    tile.set_width(width);
    tile.set_height(height);
    tile.set_corner_radius(CornerRadius::uniform(1.0));
    tile.set_padding(Thickness::uniform(height * 0.06));
    tile.set_background(Some(brushes.tile.clone()));
    tile.set_clip_to_bounds(true);
    Canvas::set_left(&tile, (index % 200) as f64 * (width + 1.0));
    Canvas::set_top(&tile, (index / 200) as f64 * (height + 1.0));
    Tile { tile, content, name, line, spot, value }
}

/// A tile put together from its leaves up: the children are added before their parents are.
pub fn tile(index: usize, brushes: &Brushes) -> Ref<Border> {
    let parts = tile_parts(index, brushes);
    parts.line.children().add(&parts.spot);
    parts.line.children().add(&parts.value);
    parts.content.children().add(&parts.name);
    parts.content.children().add(&parts.line);
    parts.tile.set_child(&parts.content);
    parts.tile
}

/// A canvas of `count` tiles, built from the leaves up.
pub fn wall(count: usize, brushes: &Brushes) -> Ref<Canvas> {
    let canvas = Canvas::new();
    for index in 0..count {
        canvas.children().add(tile(index, brushes));
    }
    canvas
}

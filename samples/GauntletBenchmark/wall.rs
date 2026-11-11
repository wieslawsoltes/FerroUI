//! The still wall: a window of tiles that never change, under a clock line whose text changes
//! every frame, with a cursor outline that rests on another tile every frame. Its data follows the
//! wall data of the benchmark bit for bit.
//!
//! In FerroUI's idiom the tiles are controls made once on a canvas; the top level's animation
//! frame callback sets the clock's text and the cursor's translation, the two things a frame
//! changes.

use crate::app::{DeviceRoboto, Launch};
use crate::gauntlet::{brush, Clock, Look, Swatches};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{LayoutableImpl, Orientation, VerticalAlignment};
use ferroui_base::media::{FontFamily, TranslateTransform};
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_overrides, instantiate, CornerRadius, FerroObjectImpl, Point, Ref,
    StyledElementImpl, Thickness, Visual, VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs,
};
use ferroui_controls::documents::TextElement;
use ferroui_controls::{Border, Canvas, ControlImpl, Grid, PanelImpl, RowDefinitions, StackPanel, TextBlock, TopLevel};
use std::cell::Cell;

/// The wall's tiers and data.
pub struct WallData;

/// The tiles' width and height in points, tier 1 to 20.
const TIERS: [(f64, f64); 20] = [
    (64.0, 40.0), (54.0, 34.0), (46.0, 29.0), (40.0, 25.0), (34.0, 21.0), (30.0, 18.0), (26.0, 16.0), (22.0, 14.0),
    (19.0, 12.0), (16.0, 10.0), (14.0, 9.0), (12.0, 8.0), (10.0, 7.0), (8.0, 6.0), (7.0, 5.0), (6.0, 4.0),
    (5.0, 4.0), (5.0, 3.0), (4.0, 3.0), (3.0, 3.0),
];

impl WallData {
    pub const GAP: f64 = 1.0;
    pub const CLOCK_HEIGHT: f64 = 24.0;
    pub const CLOCK_TEXT: f64 = 14.0;
    pub const CORNER: f64 = 2.0;
    pub const CURSOR_WIDTH: f64 = 2.0;
    pub const BACKGROUND_ARGB: u32 = 0xFF0F1220;

    pub fn tier(tier: i32) -> (f64, f64) {
        TIERS[(tier.clamp(1, TIERS.len() as i32) - 1) as usize]
    }

    /// The tiles across and down that fill a wall `width` by `height` points.
    pub fn grid(tier: (f64, f64), width: f64, height: f64) -> (i32, i32) {
        fn fit(space: f64, tile: f64) -> i32 {
            1.max(((space - WallData::GAP) / (tile + WallData::GAP)).floor() as i32)
        }
        (fit(width, tier.0), fit(height, tier.1))
    }

    pub fn label(index: i32) -> String {
        format!("{index:05}")
    }

    pub fn value(index: i32) -> String {
        format!("{:03}", ((index as u32).wrapping_mul(2_654_435_761) >> 22) % 1000)
    }

    pub fn cursor(frame: i32, tiles: i32) -> i32 {
        (i64::from(frame) * 37 % i64::from(tiles.max(1))) as i32
    }

    pub fn clock(frame: i32) -> String {
        format!("frame {frame:06}")
    }

    /// Where tile `index` sits on a wall `columns` tiles across.
    pub fn origin(index: i32, columns: i32, tier: (f64, f64)) -> Point {
        Point::new(
            Self::GAP + f64::from(index % columns) * (tier.0 + Self::GAP),
            Self::GAP + f64::from(index / columns) * (tier.1 + Self::GAP),
        )
    }
}

/// The wall's area in the window every desktop app opens, under the top bar and the clock line.
const WALL_WIDTH: f64 = 1280.0;
const WALL_HEIGHT: f64 = 820.0 - 56.0 - WallData::CLOCK_HEIGHT;

#[repr(C)]
pub struct WallView {
    base: Grid,
    freeze: i32,
    tier: (f64, f64),
    columns: i32,
    tiles: i32,
    clock: Ref<TextBlock>,
    cursor_place: Ref<TranslateTransform>,
    first_frame_logged: Cell<bool>,
}

ferro_class!(WallView: Grid);
ferro_impl_classes!(
    WallView: FerroObjectImpl,
    StyledElementImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    PanelImpl
);

ferro_overrides! { impl VisualImpl for WallView {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);
        let visual: &Visual = this;
        if let Some(top_level) = TopLevel::get_top_level(Some(visual)) {
            let view = this.to_ref();
            top_level.request_animation_frame(move |_| view.on_frame());
        }
    }
} }

impl WallView {
    pub fn new(tier_index: i32, freeze: i32) -> Ref<Self> {
        let tier = WallData::tier(tier_index);
        let (columns, rows) = WallData::grid(tier, WALL_WIDTH, WALL_HEIGHT);
        let tiles = columns * rows;
        let swatches = Swatches::get();

        let look = Look::new(1.0);
        let clock = look.text(WallData::CLOCK_TEXT, &swatches.white, false);
        clock.set_margin(Thickness::symmetric(8.0, 0.0));
        clock.set_vertical_alignment(VerticalAlignment::Center);
        Grid::set_row(&clock, 1);

        let wall = Canvas::new();
        wall.set_clip_to_bounds(true);
        let height = tier.1;
        let dot = height * 0.22;
        for index in 0..tiles {
            let label = look.text(height * 0.30, &swatches.white, true);
            label.set_text(Some(&WallData::label(index)));
            let value = look.text(height * 0.26, &swatches.white, false);
            value.set_text(Some(&WallData::value(index)));
            let spot = Border::new();
            spot.set_width(dot);
            spot.set_height(dot);
            spot.set_corner_radius(CornerRadius::uniform(dot / 2.0));
            spot.set_background(Some(
                swatches.chip_background
                    [(index as usize / swatches.palette.len()) % swatches.chip_background.len()]
                .clone(),
            ));
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
            tile.set_width(tier.0);
            tile.set_height(height);
            tile.set_corner_radius(CornerRadius::uniform(WallData::CORNER));
            tile.set_padding(Thickness::uniform(height * 0.06));
            tile.set_background(Some(swatches.palette[index as usize % swatches.palette.len()].clone()));
            tile.set_clip_to_bounds(true);
            tile.set_child(&content);
            let at = WallData::origin(index, columns, tier);
            Canvas::set_left(&tile, at.x);
            Canvas::set_top(&tile, at.y);
            wall.children().add(&tile);
        }
        let cursor_place = TranslateTransform::new();
        let cursor = Border::new();
        cursor.set_width(tier.0);
        cursor.set_height(height);
        cursor.set_corner_radius(CornerRadius::uniform(WallData::CORNER));
        cursor.set_border_brush(Some(swatches.white.clone()));
        cursor.set_border_thickness(Thickness::uniform(WallData::CURSOR_WIDTH));
        cursor.set_render_transform(Some((&cursor_place).into()));
        wall.children().add(&cursor);
        Grid::set_row(&wall, 2);

        let this = instantiate(Self {
            base: Grid::construct(),
            freeze,
            tier,
            columns,
            tiles,
            clock: clock.clone(),
            cursor_place,
            first_frame_logged: Cell::new(false),
        });
        TextElement::set_font_family_on(&this, FontFamily::new(DeviceRoboto::FAMILY));
        this.set_background(Some(brush(WallData::BACKGROUND_ARGB)));
        this.set_row_definitions(
            RowDefinitions::parse(&format!("56,{},*", WallData::CLOCK_HEIGHT)).expect("row definitions"),
        );
        this.children().add(Look::top_bar("Still wall"));
        this.children().add(&clock);
        this.children().add(&wall);
        this.show(0);
        this
    }

    fn show(&self, frame: i32) {
        self.clock.set_text(Some(&WallData::clock(frame)));
        let at = WallData::origin(WallData::cursor(frame, self.tiles), self.columns, self.tier);
        self.cursor_place.set_x(at.x);
        self.cursor_place.set_y(at.y);
    }

    fn on_frame(&self) {
        #[cfg(windows)]
        if crate::measurement::on_frame() { return; }
        crate::gauntlet::report_frame_rate();
        if !self.first_frame_logged.get() {
            self.first_frame_logged.set(true);
            Launch::log("PERF first_frame");
        }
        let frame = Clock::advance();
        self.show(frame);
        if self.freeze > 0 && frame >= self.freeze {
            Launch::log(&format!("PERF frozen frame={frame}"));
            return;
        }
        let visual: &Visual = self;
        if let Some(top_level) = TopLevel::get_top_level(Some(visual)) {
            let view = self.to_ref();
            #[cfg(windows)]
            crate::measurement::request_next_scene(&top_level, move || view.on_frame());
            #[cfg(not(windows))]
            top_level.request_animation_frame(move |_| view.on_frame());
        }
    }
}

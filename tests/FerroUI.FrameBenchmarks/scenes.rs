//! The scenes. Each does one kind of per-frame work, so that a change of the cost of that work
//! shows in its own numbers. What each guards is listed in [`SCENES`] and in the performance
//! notes (`docs/porting/desktop-performance.md`, section 8.1).

use crate::clock::Clock;
use crate::fonts::Roboto;
use ferroui_base::layout::{HorizontalAlignment, Orientation, VerticalAlignment};
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::{
    BoxShadow, BoxShadows, Color, FontFamily, FontWeight, IBrush, RotateTransform, TextWrapping, Transform,
    TransformGroup, Transforms, TranslateTransform,
};
use ferroui_base::{CornerRadius, ObjectType, Ref, Thickness, Upcast, Vector};
use ferroui_controls::documents::TextElement;
use ferroui_controls::primitives::ScrollBarVisibility;
use ferroui_controls::templates::{FuncDataTemplate, FuncTemplate};
use ferroui_controls::{
    Border, Canvas, Control, Grid, ItemsControl, ItemsSource, Panel, ScrollViewer, StackPanel, TextBlock,
    VirtualizingStackPanel, WrapPanel,
};
use std::rc::Rc;

/// A scene of the frame benchmarks.
pub struct Scene {
    /// The name the gate reports it under (`frames/<name>`) and `FRAMES_SCENE` selects.
    pub name: &'static str,
    /// Builds the scene; its per-frame work follows the [`Clock`].
    pub build: fn() -> Ref<Control>,
    /// Whether the harness moves a pointer into the window after the warm-up.
    pub pointer: bool,
    /// What the scene guards.
    pub guards: &'static str,
}

/// Every scene, in the order the harness runs them.
pub const SCENES: &[Scene] = &[
    Scene {
        name: "text",
        build: text,
        pointer: false,
        guards: "text that changes every frame: text layout, shaping and line breaking, run \
                 properties, glyph runs and their drawing (sections 9.7, 9.8, 9.12, 9.16, 9.22)",
    },
    Scene {
        name: "list",
        build: list,
        pointer: false,
        guards: "a list of shadowed cards scrolling every frame: virtualization, recycling, creating \
                 and disposing content and composition visuals, box shadows (sections 9.1, 9.11)",
    },
    Scene {
        name: "transforms",
        build: transforms,
        pointer: false,
        guards: "panels that turn, move and fade by their render transform and opacity alone: no \
                 layout and no recording (sections 9.5, 9.13)",
    },
    Scene {
        name: "width",
        build: width,
        pointer: false,
        guards: "a layout whose width changes every frame: measure holds, text block layouts kept \
                 over their width hold, copies of sizes and margins (sections 9.7, 9.9, 9.12, 9.15)",
    },
    Scene {
        name: "wall",
        build: wall,
        pointer: false,
        guards: "a still wall of tiles under a clock and a moving cursor: dirty regions, unchanged \
                 drawings not recorded or sent again (sections 9.10, 9.13, 9.14)",
    },
    Scene {
        name: "wall+pointer",
        build: wall,
        pointer: true,
        guards: "the still wall with the pointer in the window: hit testing and pointer-over \
                 (sections 9.2, 9.6)",
    },
    Scene {
        name: "moves",
        build: moves,
        pointer: false,
        guards: "tiles of a canvas that move every frame: arrange only, moves without recording \
                 (section 9.13)",
    },
];

/// The scene named `name`.
pub fn scene(name: &str) -> Option<&'static Scene> {
    SCENES.iter().find(|scene| scene.name == name)
}

fn brush(argb: u32) -> Rc<dyn IBrush> {
    Rc::new(ImmutableSolidColorBrush::new(Color::from_uint32(argb)))
}

const PALETTE: [u32; 6] = [0xFF2563EB, 0xFF16A34A, 0xFFDC2626, 0xFF9333EA, 0xFFEA580C, 0xFF0891B2];

fn label(size: f64, color: &Rc<dyn IBrush>, bold: bool) -> Ref<TextBlock> {
    let text = TextBlock::new();
    text.set_font_size(size);
    text.set_foreground(Some(color.clone()));
    if bold {
        text.set_font_weight(FontWeight::Bold);
    }
    text
}

fn rounded(background: Rc<dyn IBrush>, radius: f64, padding: Thickness, child: &Ref<impl ObjectType + Upcast<Control>>) -> Ref<Border> {
    let border = Border::new();
    border.set_background(Some(background));
    border.set_corner_radius(CornerRadius::uniform(radius));
    border.set_padding(padding);
    border.set_child(child);
    border
}

/// The root of a scene: Roboto, a light background, the content.
fn root(content: &Ref<impl ObjectType + Upcast<Control>>) -> Ref<Control> {
    let grid = Grid::new();
    TextElement::set_font_family_on(&grid, FontFamily::new(Roboto::FAMILY));
    grid.set_background(Some(brush(0xFFEEF0F5)));
    grid.children().add(content);
    grid.upcast()
}

/// A triangle wave over `period` frames, from 0 to 1 and back.
fn wave(frame: i32, period: i32) -> f64 {
    let phase = frame.rem_euclid(period) as f64 / f64::from(period);
    1.0 - (2.0 * phase - 1.0).abs()
}

// --- text ----------------------------------------------------------------------------------------

const TEXT_TILES: usize = 480;

fn text() -> Ref<Control> {
    let ink = brush(0xFF111827);
    let body = brush(0xFF374151);
    let up = brush(0xFF16A34A);
    let down = brush(0xFFDC2626);
    let tiles = WrapPanel::new();
    tiles.set_item_spacing(4.0);
    tiles.set_line_spacing(4.0);
    for index in 0..TEXT_TILES {
        let symbol = label(10.0, &ink, true);
        symbol.set_text(Some(&format!("T{index:03}")));
        let price = label(10.0, &body, false);
        let change = label(10.0, &up, false);
        let bar = Border::new();
        bar.set_height(4.0);
        bar.set_background(Some(brush(PALETTE[index % PALETTE.len()])));
        bar.set_corner_radius(CornerRadius::uniform(2.0));
        let row = StackPanel::new();
        row.set_orientation(Orientation::Horizontal);
        row.set_spacing(4.0);
        for control in [symbol.upcast::<Control>(), price.clone().upcast(), change.clone().upcast(), bar.clone().upcast()] {
            control.set_vertical_alignment(VerticalAlignment::Center);
            row.children().add(control);
        }
        let tile = rounded(brush(0xFFFFFFFF), 6.0, Thickness::symmetric(6.0, 3.0), &row);
        tiles.children().add(&tile);
        let (up, down) = (up.clone(), down.clone());
        let base = 1_000 + (index as i32 * 37) % 9_000;
        let update = move |frame: i32| {
            let cents = base + ((frame + index as i32 * 7) % 200) - 100;
            price.set_text(Some(&format!("{}.{:02}", cents / 100, cents % 100)));
            let delta = cents - base;
            change.set_text(Some(&format!("{}{}.{:02}", if delta >= 0 { "+" } else { "-" }, delta.abs() / 100, delta.abs() % 100)));
            change.set_foreground(Some(if delta >= 0 { up.clone() } else { down.clone() }));
            bar.set_width(4.0 + f64::from((delta + 100) / 10));
        };
        update(0);
        Clock::follow(update);
    }
    let panel = Border::new();
    panel.set_padding(Thickness::uniform(6.0));
    panel.set_child(&tiles);
    root(&panel)
}

// --- list ----------------------------------------------------------------------------------------

const LIST_ITEMS: i32 = 20_000;
const SCROLL_PER_FRAME: f64 = 3.0;

fn card(index: i32) -> Ref<Control> {
    let ink = brush(0xFF111827);
    let body = brush(0xFF4B5563);
    let title = label(14.0, &ink, true);
    title.set_text_wrapping(TextWrapping::Wrap);
    title.set_max_lines(2);
    title.set_text(Some(&format!("Card {index}: a title long enough to wrap onto a second line of the card")));
    let subtitle = label(12.0, &body, false);
    subtitle.set_text(Some(&format!("Posted by author {} \u{2022} {} comments", index % 97, index % 41)));
    let fill = Border::new();
    fill.set_height(6.0);
    fill.set_width(f64::from(index % 100) * 2.0);
    fill.set_horizontal_alignment(HorizontalAlignment::Left);
    fill.set_background(Some(brush(PALETTE[index as usize % PALETTE.len()])));
    fill.set_corner_radius(CornerRadius::uniform(3.0));
    let track = rounded(brush(0xFFE5E7EB), 3.0, Thickness::uniform(0.0), &fill);
    let chips = WrapPanel::new();
    chips.set_item_spacing(4.0);
    chips.set_line_spacing(4.0);
    for chip in 0..4 {
        let text = label(10.0, &ink, false);
        text.set_text(Some(&format!("tag {}", (index + chip) % 23)));
        chips.children().add(rounded(brush(0xFFF3F4F6), 8.0, Thickness::symmetric(6.0, 2.0), &text));
    }
    let column = StackPanel::new();
    column.set_spacing(6.0);
    column.children().add(&title);
    column.children().add(&subtitle);
    column.children().add(&track);
    column.children().add(&chips);
    let frame = rounded(brush(0xFFFFFFFF), 12.0, Thickness::uniform(10.0), &column);
    frame.set_margin(Thickness::new(8.0, 4.0, 8.0, 4.0));
    frame.set_box_shadow(BoxShadows::new(BoxShadow {
        offset_y: 1.0,
        blur: 3.0,
        color: Color::from_argb(0x3D, 0, 0, 0),
        ..BoxShadow::default()
    }));
    frame.upcast()
}

fn list() -> Ref<Control> {
    let items = ItemsControl::new();
    items.set_items_source(Some(ItemsSource::from_values(0..LIST_ITEMS)));
    items.set_items_panel(FuncTemplate::new(|| Some(VirtualizingStackPanel::new().upcast::<Panel>())));
    items.set_item_template(Some(FuncDataTemplate::for_type::<i32>(|index, _| Some(card(*index)), false)));
    let scroller = ScrollViewer::new();
    scroller.set_content(Some(Control::boxed(&items)));
    scroller.set_vertical_scroll_bar_visibility(ScrollBarVisibility::Hidden);
    scroller.set_horizontal_scroll_bar_visibility(ScrollBarVisibility::Disabled);
    let follower = scroller.clone();
    Clock::follow(move |frame| follower.set_offset(Vector::new(0.0, f64::from(frame) * SCROLL_PER_FRAME)));
    root(&scroller)
}

// --- transforms ----------------------------------------------------------------------------------

const PANELS: i32 = 12;

fn transforms() -> Ref<Control> {
    let white = brush(0xFFFFFFFF);
    let area = Grid::new();
    area.set_clip_to_bounds(true);
    for panel in 0..PANELS {
        let cells = StackPanel::new();
        cells.set_spacing(3.0);
        for row in 0..6 {
            let line = StackPanel::new();
            line.set_orientation(Orientation::Horizontal);
            line.set_spacing(3.0);
            for column in 0..8 {
                let text = label(9.0, &white, false);
                text.set_text(Some(&format!("{}", row * 8 + column + 1)));
                text.set_horizontal_alignment(HorizontalAlignment::Center);
                text.set_vertical_alignment(VerticalAlignment::Center);
                let cell = rounded(brush(PALETTE[((panel + row + column) % 6) as usize]), 4.0, Thickness::uniform(0.0), &text);
                cell.set_width(22.0);
                cell.set_height(22.0);
                line.children().add(&cell);
            }
            cells.children().add(&line);
        }
        let title = label(13.0, &white, true);
        title.set_text(Some(&format!("Panel {}", panel + 1)));
        let column = StackPanel::new();
        column.set_spacing(8.0);
        column.children().add(&title);
        column.children().add(&cells);
        let card = rounded(brush(0xC01E293B), 12.0, Thickness::uniform(10.0), &column);
        card.set_horizontal_alignment(HorizontalAlignment::Left);
        card.set_vertical_alignment(VerticalAlignment::Top);
        card.set_margin(Thickness::new(f64::from(panel % 4) * 300.0 + 20.0, f64::from(panel / 4) * 250.0 + 20.0, 0.0, 0.0));
        let tilt = RotateTransform::new();
        let shift = TranslateTransform::new();
        let group = TransformGroup::new();
        group.set_children(Transforms::from_items([tilt.clone().upcast::<Transform>(), shift.clone().upcast::<Transform>()]));
        card.set_render_transform(Some((&group).into()));
        area.children().add(&card);
        let target = card.clone();
        Clock::follow(move |frame| {
            let w = wave(frame + panel * 13, 180);
            tilt.set_angle((w - 0.5) * 16.0);
            shift.set_x((w - 0.5) * 60.0);
            shift.set_y((wave(frame + panel * 29, 240) - 0.5) * 40.0);
            target.set_opacity(0.6 + 0.4 * w);
        });
    }
    root(&area)
}

// --- width ---------------------------------------------------------------------------------------

const WIDTH_CARDS: i32 = 300;
const CONTENT_WIDTH: f64 = 1240.0;

fn width() -> Ref<Control> {
    let ink = brush(0xFF111827);
    let body = brush(0xFF4B5563);
    let cards = WrapPanel::new();
    cards.set_item_spacing(6.0);
    cards.set_line_spacing(6.0);
    for index in 0..WIDTH_CARDS {
        let title = label(12.0, &ink, true);
        title.set_text(Some(&format!("Item {index}")));
        let text = label(11.0, &body, false);
        text.set_text_wrapping(TextWrapping::Wrap);
        text.set_max_lines(3);
        text.set_text(Some("A paragraph that wraps over a few lines and keeps its layout while its width holds."));
        let column = StackPanel::new();
        column.set_spacing(2.0);
        column.children().add(&title);
        column.children().add(&text);
        let card = rounded(brush(0xFFFFFFFF), 8.0, Thickness::uniform(6.0), &column);
        card.set_width(150.0 + f64::from(index % 5) * 10.0);
        cards.children().add(&card);
    }
    let content = Grid::new();
    content.set_horizontal_alignment(HorizontalAlignment::Left);
    content.children().add(&cards);
    let follower = content.clone();
    let resize = move |frame: i32| follower.set_width((CONTENT_WIDTH * (0.92 + 0.08 * wave(frame, 120))).round());
    resize(0);
    Clock::follow(resize);
    root(&content)
}

// --- wall ----------------------------------------------------------------------------------------

const WALL_TILES: usize = 20_000;
const WALL_COLUMNS: usize = 200;

/// A tile of a wall: a bordered box around a label and a line of a dot and a value, seven elements.
fn tile(index: usize, label_brush: &Rc<dyn IBrush>, tile_brush: &Rc<dyn IBrush>, dot_brush: &Rc<dyn IBrush>) -> Ref<Border> {
    let (width, height) = (5.0, 4.0);
    let dot = height * 0.22;
    let name = label(height * 0.30, label_brush, false);
    name.set_text(Some(&format!("{index:05}")));
    let value = label(height * 0.26, label_brush, false);
    value.set_text(Some(&format!("{:03}", index % 1000)));
    let spot = Border::new();
    spot.set_width(dot);
    spot.set_height(dot);
    spot.set_corner_radius(CornerRadius::uniform(dot / 2.0));
    spot.set_background(Some(dot_brush.clone()));
    spot.set_vertical_alignment(VerticalAlignment::Center);
    let line = StackPanel::new();
    line.set_orientation(Orientation::Horizontal);
    line.set_spacing(dot * 0.5);
    line.children().add(&spot);
    line.children().add(&value);
    let content = StackPanel::new();
    content.children().add(&name);
    content.children().add(&line);
    let tile = Border::new();
    tile.set_width(width);
    tile.set_height(height);
    tile.set_corner_radius(CornerRadius::uniform(1.0));
    tile.set_padding(Thickness::uniform(height * 0.06));
    tile.set_background(Some(tile_brush.clone()));
    tile.set_clip_to_bounds(true);
    tile.set_child(&content);
    Canvas::set_left(&tile, (index % WALL_COLUMNS) as f64 * (width + 1.0));
    Canvas::set_top(&tile, (index / WALL_COLUMNS) as f64 * (height + 1.0) + 24.0);
    tile
}

fn wall() -> Ref<Control> {
    let white = brush(0xFFFFFFFF);
    let blue = brush(0xFF1E40AF);
    let red = brush(0xFFDC2626);
    let canvas = Canvas::new();
    for index in 0..WALL_TILES {
        canvas.children().add(tile(index, &white, &blue, &red));
    }
    let clock = label(14.0, &brush(0xFF111827), true);
    let cursor = Border::new();
    cursor.set_width(24.0);
    cursor.set_height(24.0);
    cursor.set_border_brush(Some(red.clone()));
    cursor.set_border_thickness(Thickness::uniform(2.0));
    let shift = TranslateTransform::new();
    cursor.set_render_transform(Some((&shift).into()));
    canvas.children().add(&clock);
    canvas.children().add(&cursor);
    let update = move |frame: i32| {
        clock.set_text(Some(&format!("frame {frame:06}")));
        shift.set_x(200.0 + 400.0 * wave(frame, 300));
        shift.set_y(100.0 + 300.0 * wave(frame, 220));
    };
    update(0);
    Clock::follow(update);
    root(&canvas)
}

// --- moves ---------------------------------------------------------------------------------------

const MOVING_TILES: usize = 4_000;

fn moves() -> Ref<Control> {
    let white = brush(0xFFFFFFFF);
    let blue = brush(0xFF0F766E);
    let red = brush(0xFFEA580C);
    let canvas = Canvas::new();
    let tiles: Rc<Vec<Ref<Border>>> = Rc::new((0..MOVING_TILES).map(|index| tile(index, &white, &blue, &red)).collect());
    for tile in tiles.iter() {
        canvas.children().add(tile);
    }
    Clock::follow(move |frame| {
        // An eighth of the tiles moves each frame.
        let phase = frame.rem_euclid(8) as usize;
        for (index, tile) in tiles.iter().enumerate().skip(phase).step_by(8) {
            let offset = 3.0 * wave(frame + index as i32, 60);
            Canvas::set_left(tile, (index % WALL_COLUMNS) as f64 * 6.0 + offset);
        }
    });
    root(&canvas)
}

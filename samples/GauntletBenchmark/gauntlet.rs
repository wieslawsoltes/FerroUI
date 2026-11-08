//! The gauntlet: the screen every app of the comparison draws, element for element, in FerroUI's
//! own idiom. The benchmark's README describes it.
//!
//! Every frame advances a frame index and everything follows from it, never from wall time. The
//! top level's animation frame callback advances it once per frame; the attached controls read it
//! and set only what changed.

use crate::app::{DeviceRoboto, Launch};
use crate::perf_data::{
    avatar_rgba, badge_degrees, change_text, cents_text, gauntlet_tier, layer_cell_color, layer_degrees, layer_x,
    layer_y, posts, progress_permille, spark_value, ticker_cents, tickers, width_fraction, Post, Ticker, AVATAR_COUNT,
    AVATAR_SIZE, CARD_ROWS_PER_CLUSTER, CHIP_BACKGROUND_ARGB, GRADIENT_END_ARGB, LAYER_COLUMNS, PALETTE_ARGB,
    POST_COUNT, SPARK_POINTS,
};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{HorizontalAlignment, LayoutableImpl, Orientation, VerticalAlignment};
use ferroui_base::media::imaging::WriteableBitmap;
use ferroui_base::media::immutable::{
    ImmutableGradientStop, ImmutableLinearGradientBrush, ImmutablePen, ImmutableSolidColorBrush,
};
use ferroui_base::media::{
    BoxShadow, BoxShadows, Brushes, Color, DrawingContext, FontFamily, Geometry, FontWeight, GradientSpreadMethod, IBrush, IImage,
    IPen, RotateTransform, Stretch, StreamGeometry, TextTrimming, TextWrapping, Transform, TransformGroup, Transforms,
    TranslateTransform,
};
use ferroui_base::platform::{AlphaFormat, IGeometryContext, ILockedFramebuffer, PixelFormat};
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_overrides, instantiate, BoxedValue, CornerRadius, FerroObjectImpl,
    ObjectType, PixelSize, Point, Rect, Ref, RelativePoint, RelativeUnit, StyledElementImpl, Thickness, TypeInfo,
    Upcast, Vector, Visual, VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs,
};
use ferroui_controls::documents::{InlineCollection, Run, TextElement};
use ferroui_controls::generators::RecycleKey;
use ferroui_controls::platform::IInsetsManager;
use ferroui_controls::primitives::{ScrollBarVisibility, TemplatedControlImpl};
use ferroui_controls::templates::FuncTemplate;
use ferroui_controls::{
    Border, ColumnDefinition, ColumnDefinitions, Control, ControlImpl, Decorator, Grid, GridUnitType, Image, ItemsControl,
    ItemsControlImpl, ItemsControlImplExt, ItemsSource, Panel, PanelImpl, RowDefinitions, ScrollViewer, StackPanel,
    TextBlock, TopLevel, VirtualizingStackPanel, WrapPanel,
};
use std::any::TypeId;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

ferroui_base::ferro_thread_local! {
    static FRAME: Cell<i32> = const { Cell::new(0) };
    static ADVANCED: RefCell<Vec<(usize, Rc<dyn Fn(i32)>)>> = const { RefCell::new(Vec::new()) };
    static SWATCHES: Rc<Swatches> = Rc::new(Swatches::new());
}

/// The frame index every per-frame value follows.
pub struct Clock;

impl Clock {
    pub fn frame() -> i32 {
        FRAME.get()
    }

    /// Advances the index and calls every follower, in the order they started following.
    pub fn advance() -> i32 {
        let frame = FRAME.get() + 1;
        FRAME.set(frame);
        let mut index = 0;
        while let Some(on_frame) = ADVANCED.with_borrow(|advanced| advanced.get(index).map(|(_, f)| f.clone())) {
            on_frame(frame);
            index += 1;
        }
        frame
    }

    /// Calls `on_frame` every frame from now until [`Clock::unfollow`] with the same `control`:
    /// a control follows while it is on screen, from its attachment to its detachment.
    pub fn follow<T>(control: &T, on_frame: impl Fn(i32) + 'static) {
        let key = control as *const T as *const () as usize;
        ADVANCED.with_borrow_mut(|advanced| advanced.push((key, Rc::new(on_frame))));
    }

    pub fn unfollow<T>(control: &T) {
        let key = control as *const T as *const () as usize;
        ADVANCED.with_borrow_mut(|advanced| {
            if let Some(position) = advanced.iter().rposition(|(k, _)| *k == key) {
                advanced.remove(position);
            }
        });
    }
}

/// The brushes the gauntlet's controls share, made once.
pub struct Swatches {
    pub ink: Rc<dyn IBrush>,
    pub body: Rc<dyn IBrush>,
    pub muted: Rc<dyn IBrush>,
    pub hairline: Rc<dyn IBrush>,
    pub panel: Rc<dyn IBrush>,
    pub up: Rc<dyn IBrush>,
    pub down: Rc<dyn IBrush>,
    pub white: Rc<dyn IBrush>,
    pub black: Rc<dyn IBrush>,
    pub transparent: Rc<dyn IBrush>,
    pub level_background: [Rc<dyn IBrush>; 2],
    pub palette: Vec<Rc<dyn IBrush>>,
    pub chip_background: Vec<Rc<dyn IBrush>>,
    pub gradient_end: Vec<Rc<dyn IBrush>>,
}

pub fn brush(argb: u32) -> Rc<dyn IBrush> {
    Rc::new(ImmutableSolidColorBrush::from_uint32(argb))
}

impl Swatches {
    fn new() -> Self {
        Self {
            ink: brush(0xFF111827),
            body: brush(0xFF374151),
            muted: brush(0xFF6B7280),
            hairline: brush(0xFFE5E7EB),
            panel: brush(0xFFE2E8F0),
            up: brush(0xFF16A34A),
            down: brush(0xFFDC2626),
            white: Brushes::white(),
            black: Brushes::black(),
            transparent: Brushes::transparent(),
            level_background: [brush(0xFFF1F5F9), brush(0xFFCBD5E1)],
            palette: PALETTE_ARGB.iter().copied().map(brush).collect(),
            chip_background: CHIP_BACKGROUND_ARGB.iter().copied().map(brush).collect(),
            gradient_end: GRADIENT_END_ARGB.iter().copied().map(brush).collect(),
        }
    }

    pub fn get() -> Rc<Swatches> {
        SWATCHES.with(Rc::clone)
    }
}

/// Colors and text the gauntlet's controls share at one tier's scale.
#[derive(Clone, Copy)]
pub struct Look {
    pub s: f64,
}

impl Look {
    pub fn new(s: f64) -> Self {
        Self { s }
    }

    pub fn text(&self, size: f64, color: &Rc<dyn IBrush>, bold: bool) -> Ref<TextBlock> {
        let text = TextBlock::new();
        text.set_font_size(size * self.s);
        text.set_foreground(Some(color.clone()));
        text.set_font_weight(if bold { FontWeight::Bold } else { FontWeight::Normal });
        text
    }

    /// Lines 1.4 em apart, cut with an ellipsis after `max_lines`.
    pub fn paragraph(&self, size: f64, color: &Rc<dyn IBrush>, max_lines: i32, bold: bool) -> Ref<TextBlock> {
        let text = self.text(size, color, bold);
        text.set_line_height(size * self.s * 1.4);
        text.set_max_lines(max_lines);
        text.set_text_wrapping(TextWrapping::Wrap);
        text.set_text_trimming(<dyn TextTrimming>::character_ellipsis());
        text
    }

    /// The bar across the window's top, titled `title`.
    pub fn top_bar(title: &str) -> Ref<Border> {
        let text = Look::new(1.0).text(20.0, &Swatches::get().white, true);
        text.set_text(Some(title));
        text.set_vertical_alignment(VerticalAlignment::Center);
        let bar = Border::new();
        bar.set_background(Some(brush(0xFF1E2A4A)));
        bar.set_padding(Thickness::symmetric(16.0, 0.0));
        bar.set_child(&text);
        bar
    }

    pub fn rounded<T: ObjectType + Upcast<Control>>(
        background: Option<Rc<dyn IBrush>>,
        radius: f64,
        padding: Thickness,
        child: &Ref<T>,
    ) -> Ref<Border> {
        let border = Border::new();
        border.set_background(background);
        border.set_corner_radius(CornerRadius::uniform(radius));
        border.set_padding(padding);
        border.set_child(child);
        border
    }

    /// `count` equal columns `gap` apart: child `n` goes in column `2n`.
    pub fn columns(count: usize, gap: f64) -> Ref<Grid> {
        let grid = Grid::new();
        add_columns(&grid, count, gap);
        grid
    }

    pub fn place(grid: &Grid, child: &Control, column: i32) {
        Grid::set_column(child, column);
        grid.children().add(child.to_ref());
    }
}

/// Adds `count` star columns `gap` pixels apart to `grid`.
fn add_columns(grid: &Grid, count: usize, gap: f64) {
    let definitions = grid.column_definitions();
    for column in 0..count {
        if column > 0 {
            definitions.add(ColumnDefinition::with_value(gap, GridUnitType::Pixel));
        }
        definitions.add(ColumnDefinition::with_value(1.0, GridUnitType::Star));
    }
}

// --- the screen ----------------------------------------------------------------------------------

/// Blocks of five card rows and a cluster: no measurement window reaches the end.
const BLOCKS: i32 = 2000;

/// How far the list scrolls each frame, in dp.
const SCROLL_PER_FRAME: f64 = 3.0;

#[repr(C)]
pub struct GauntletView {
    base: Grid,
    freeze: i32,
    content: Ref<Grid>,
    scroller: Ref<ScrollViewer>,
    first_frame_logged: Cell<bool>,
}

ferro_class!(GauntletView: Grid);
ferro_impl_classes!(
    GauntletView: FerroObjectImpl,
    StyledElementImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    PanelImpl
);

ferro_overrides! { impl VisualImpl for GauntletView {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);
        let visual: &Visual = this;
        let top_level = TopLevel::get_top_level(Some(visual));
        // A platform with system bars draws behind them unless told otherwise: the window is
        // fullscreen and starts below the display cutout, as the other apps' windows do.
        if let Some(insets) = top_level.as_ref().and_then(|top_level| top_level.insets_manager()) {
            insets.set_is_system_bar_visible(Some(false));
            insets.set_display_edge_to_edge_preference(false);
        }
        if let Some(top_level) = top_level {
            let view = this.to_ref();
            top_level.request_animation_frame(move |_| view.on_frame());
        }
    }
} }

impl GauntletView {
    pub fn new(tier_index: i32, freeze: i32) -> Ref<Self> {
        let tier = gauntlet_tier(tier_index);
        let look = Look::new(f64::from(tier.scale));
        let s = look.s;
        let swatches = Swatches::get();

        let tiles = WrapPanel::new();
        tiles.set_item_spacing(4.0 * s);
        tiles.set_line_spacing(4.0 * s);
        for (index, quote) in tickers(tier.tickers).into_iter().enumerate() {
            tiles.children().add(TickerTile::new(look, quote, index));
        }
        let panel = Border::new();
        panel.set_background(Some(swatches.panel.clone()));
        panel.set_padding(Thickness::uniform(6.0 * s));
        panel.set_child(&tiles);

        let posts: Rc<[Post]> = posts().into();
        let avatars: Rc<[Rc<dyn IImage>]> = (0..AVATAR_COUNT).map(avatar).collect();
        let rows = Rows::new(look, tier.columns, tier.depth, posts, avatars);
        rows.set_items_source(Some(ItemsSource::from_values(0..BLOCKS * (CARD_ROWS_PER_CLUSTER + 1))));
        rows.set_items_panel(FuncTemplate::new(|| Some(VirtualizingStackPanel::new().upcast::<Panel>())));
        rows.set_padding(Thickness::new(0.0, 8.0 * s, 0.0, 0.0));
        let scroller = ScrollViewer::new();
        scroller.set_content(Some(Control::boxed(&rows)));
        scroller.set_vertical_scroll_bar_visibility(ScrollBarVisibility::Hidden);
        scroller.set_horizontal_scroll_bar_visibility(ScrollBarVisibility::Disabled);

        let content = Grid::new();
        content.set_horizontal_alignment(HorizontalAlignment::Left);
        content.set_row_definitions(RowDefinitions::parse("Auto,*").expect("row definitions"));
        content.children().add(&panel);
        // The panels stack over the list and show only over it, as every app clips them.
        let area = Grid::new();
        area.set_clip_to_bounds(true);
        area.children().add(&scroller);
        for layer in 0..tier.layers {
            area.children().add(StackedLayer::new(layer, tier.layer_rows));
        }
        Grid::set_row(&area, 1);
        content.children().add(&area);
        Grid::set_row(&content, 1);

        let this = instantiate(Self {
            base: Grid::construct(),
            freeze,
            content: content.clone(),
            scroller,
            first_frame_logged: Cell::new(false),
        });
        TextElement::set_font_family_on(&this, FontFamily::new(DeviceRoboto::FAMILY));
        this.set_background(Some(brush(0xFFEEF0F5)));
        this.set_row_definitions(RowDefinitions::parse("56,*").expect("row definitions"));
        this.children().add(Look::top_bar("Gauntlet"));
        this.children().add(&content);
        this
    }

    fn on_frame(&self) {
        if !self.first_frame_logged.get() {
            self.first_frame_logged.set(true);
            Launch::log("PERF first_frame");
        }
        let frame = Clock::advance();
        let visual: &Visual = self;
        let top_level = TopLevel::get_top_level(Some(visual));
        let scaling = top_level.as_ref().map_or(1.0, |top_level| top_level.render_scaling());
        // The width is set once per frame: everything below is measured again.
        self.content
            .set_width((self.bounds().width * scaling * f64::from(width_fraction(frame))).round_ties_even() / scaling);
        self.scroller.set_offset(Vector::new(0.0, f64::from(frame) * SCROLL_PER_FRAME));
        if self.freeze > 0 && frame >= self.freeze {
            Launch::log(&format!("PERF frozen frame={frame}"));
            return;
        }
        if let Some(top_level) = top_level {
            let view = self.to_ref();
            top_level.request_animation_frame(move |_| view.on_frame());
        }
    }
}

fn avatar(index: usize) -> Rc<dyn IImage> {
    let size = AVATAR_SIZE as i32;
    let bitmap = WriteableBitmap::new(
        PixelSize::new(size, size),
        Vector::new(96.0, 96.0),
        Some(PixelFormat::RGBA8888),
        Some(AlphaFormat::Opaque),
    );
    let buffer = bitmap.lock();
    let rgba = avatar_rgba(index);
    let row = AVATAR_SIZE * 4;
    let row_bytes = buffer.row_bytes() as usize;
    buffer.with_data(&mut |pixels| {
        for y in 0..AVATAR_SIZE {
            pixels[y * row_bytes..y * row_bytes + row].copy_from_slice(&rgba[y * row..(y + 1) * row]);
        }
    });
    buffer.dispose();
    Rc::new(bitmap)
}

// --- the list ------------------------------------------------------------------------------------

/// The list: a row of cards, or a cluster after every five. Each kind recycles only its own rows,
/// which rebind to the row they show.
#[repr(C)]
struct Rows {
    base: ItemsControl,
    look: Look,
    columns: usize,
    depth: i32,
    posts: Rc<[Post]>,
    avatars: Rc<[Rc<dyn IImage>]>,
}

ferro_class!(Rows: ItemsControl);
ferro_impl_classes!(
    Rows: FerroObjectImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl
);

ferro_overrides! { impl StyledElementImpl for Rows {
    fn style_key_override(_this: &Self) -> &'static TypeInfo {
        ItemsControl::TYPE
    }
} }

fn is_cluster(row: i32) -> bool {
    row % (CARD_ROWS_PER_CLUSTER + 1) == CARD_ROWS_PER_CLUSTER
}

fn card_key() -> RecycleKey {
    RecycleKey::Type(TypeId::of::<CardRow>())
}

fn cluster_key() -> RecycleKey {
    RecycleKey::Type(TypeId::of::<ClusterRow>())
}

ferro_overrides! { impl ItemsControlImpl for Rows {
    fn needs_container_override(_this: &Self, _item: &Option<BoxedValue>, index: i32) -> (bool, Option<RecycleKey>) {
        (true, Some(if is_cluster(index) { cluster_key() } else { card_key() }))
    }

    fn create_container_for_item_override(
        this: &Self,
        _item: &Option<BoxedValue>,
        _index: i32,
        recycle_key: Option<RecycleKey>
    ) -> Ref<Control> {
        if recycle_key == Some(cluster_key()) {
            ClusterRow::new(this.look, this.depth).upcast()
        } else {
            CardRow::new(this.look, this.columns, this.posts.clone(), this.avatars.clone()).upcast()
        }
    }

    fn prepare_container_for_item_override(this: &Self, container: &Ref<Control>, item: &Option<BoxedValue>, index: i32) {
        Self::parent_prepare_container_for_item_override(this, container, item, index);
        let block = index / (CARD_ROWS_PER_CLUSTER + 1);
        if let Some(cards) = container.downcast_ref::<CardRow>() {
            cards.bind((block * CARD_ROWS_PER_CLUSTER + index % (CARD_ROWS_PER_CLUSTER + 1)) as usize * this.columns);
        } else if let Some(cluster) = container.downcast_ref::<ClusterRow>() {
            cluster.bind(block);
        }
    }
} }

impl Rows {
    fn new(look: Look, columns: usize, depth: i32, posts: Rc<[Post]>, avatars: Rc<[Rc<dyn IImage>]>) -> Ref<Self> {
        instantiate(Self { base: ItemsControl::construct(), look, columns, depth, posts, avatars })
    }
}

// --- the stacked panels --------------------------------------------------------------------------

/// A translucent panel stacked over the list. Its controls never change: each frame sets only the
/// panel's translation and tilt and its nested card's tilt, which the compositor applies to their
/// retained drawing.
#[repr(C)]
struct StackedLayer {
    base: Border,
    layer: i32,
    tilt: Ref<RotateTransform>,
    shift: Ref<TranslateTransform>,
    nested_tilt: Ref<RotateTransform>,
}

ferro_class!(StackedLayer: Border);
ferro_impl_classes!(
    StackedLayer: FerroObjectImpl,
    StyledElementImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

ferro_overrides! { impl VisualImpl for StackedLayer {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);
        let layer = this.to_ref();
        Clock::follow(this, move |frame| layer.on_frame(frame));
    }

    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_detached_from_visual_tree(this, e);
        Clock::unfollow(this);
    }
} }

impl StackedLayer {
    fn new(layer: i32, rows: i32) -> Ref<Self> {
        let swatches = Swatches::get();
        let look = Look::new(1.0);
        let title = look.text(13.0, &swatches.white, true);
        title.set_text(Some(&format!("Layer {}", layer + 1)));
        let cells = StackPanel::new();
        cells.set_spacing(3.0);
        for row in 0..rows {
            let line = StackPanel::new();
            line.set_orientation(Orientation::Horizontal);
            line.set_spacing(3.0);
            for column in 0..LAYER_COLUMNS {
                let cell = row * LAYER_COLUMNS + column;
                let label = look.text(9.0, &swatches.white, false);
                label.set_text(Some(&format!("{}", cell + 1)));
                label.set_horizontal_alignment(HorizontalAlignment::Center);
                label.set_vertical_alignment(VerticalAlignment::Center);
                let cell_box = Look::rounded(
                    Some(swatches.palette[layer_cell_color(layer, cell)].clone()),
                    4.0,
                    Thickness::uniform(0.0),
                    &label,
                );
                cell_box.set_width(22.0);
                cell_box.set_height(22.0);
                line.children().add(&cell_box);
            }
            cells.children().add(&line);
        }
        let heading = look.text(11.0, &swatches.ink, true);
        heading.set_text(Some(&format!("Nested in layer {}", layer + 1)));
        let body = look.text(11.0, &swatches.body, false);
        body.set_text(Some("Tilts against its panel"));
        let nested_lines = StackPanel::new();
        nested_lines.set_spacing(2.0);
        nested_lines.children().add(&heading);
        nested_lines.children().add(&body);
        let nested = Look::rounded(Some(brush(0xE6FFFFFF)), 8.0, Thickness::uniform(8.0), &nested_lines);
        let nested_tilt = RotateTransform::new();
        nested.set_render_transform(Some((&nested_tilt).into()));
        let column = StackPanel::new();
        column.set_spacing(8.0);
        column.children().add(&title);
        column.children().add(&cells);
        column.children().add(&nested);

        let tilt = RotateTransform::new();
        let shift = TranslateTransform::new();
        let this = instantiate(Self {
            base: Border::construct(),
            layer,
            tilt: tilt.clone(),
            shift: shift.clone(),
            nested_tilt,
        });
        this.set_width(220.0);
        this.set_horizontal_alignment(HorizontalAlignment::Left);
        this.set_vertical_alignment(VerticalAlignment::Top);
        this.set_background(Some(brush(0xC01E293B)));
        this.set_corner_radius(CornerRadius::uniform(12.0));
        this.set_padding(Thickness::uniform(10.0));
        this.set_child(&column);
        // Tilted about its center, then moved.
        let group = TransformGroup::new();
        group.set_children(Transforms::from_items([tilt.upcast::<Transform>(), shift.upcast::<Transform>()]));
        this.set_render_transform(Some((&group).into()));
        this.on_frame(0);
        this
    }

    fn on_frame(&self, frame: i32) {
        let degrees = f64::from(layer_degrees(self.layer, frame));
        self.tilt.set_angle(degrees);
        self.shift.set_x(f64::from(layer_x(self.layer, frame)));
        self.shift.set_y(f64::from(layer_y(self.layer, frame)));
        self.nested_tilt.set_angle(-degrees);
    }
}

// --- bars and tickers ----------------------------------------------------------------------------

/// A rounded track filled to its share of the frame.
#[repr(C)]
struct Bar {
    base: Control,
    radius: f64,
    share: Box<dyn Fn(i32) -> f64>,
    fill: RefCell<Rc<dyn IBrush>>,
}

ferro_class!(Bar: Control);
ferro_impl_classes!(
    Bar: FerroObjectImpl,
    StyledElementImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

ferro_overrides! { impl VisualImpl for Bar {
    fn render(this: &Self, context: &mut DrawingContext) {
        let bounds = this.bounds();
        let track = Rect::new(0.0, 0.0, bounds.width, bounds.height);
        let radius = this.radius;
        let none = BoxShadows::default();
        context.draw_rectangle(Some(&Swatches::get().hairline), None, track, radius, radius, &none);
        let filled = Rect::new(0.0, 0.0, track.width * (this.share)(Clock::frame()), track.height);
        context.draw_rectangle(Some(&*this.fill.borrow()), None, filled, radius, radius, &none);
    }
} }

impl Bar {
    fn new(radius: f64, share: impl Fn(i32) -> f64 + 'static) -> Ref<Self> {
        instantiate(Self {
            base: Control::construct(),
            radius,
            share: Box::new(share),
            fill: RefCell::new(Swatches::get().black.clone()),
        })
    }

    fn set_fill(&self, fill: Rc<dyn IBrush>) {
        *self.fill.borrow_mut() = fill;
    }
}

#[repr(C)]
struct TickerTile {
    base: Border,
    quote: Ticker,
    price: Ref<TextBlock>,
    change: Ref<TextBlock>,
    bar: Ref<Bar>,
}

ferro_class!(TickerTile: Border);
ferro_impl_classes!(
    TickerTile: FerroObjectImpl,
    StyledElementImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

ferro_overrides! { impl VisualImpl for TickerTile {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);
        let tile = this.to_ref();
        Clock::follow(this, move |frame| tile.on_frame(frame));
    }

    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_detached_from_visual_tree(this, e);
        Clock::unfollow(this);
    }
} }

fn share(quote: &Ticker, frame: i32) -> f64 {
    (f64::from(ticker_cents(quote, frame) - quote.base_cents + quote.swing_cents) / (2.0 * f64::from(quote.swing_cents)))
        .clamp(0.0, 1.0)
}

impl TickerTile {
    fn new(look: Look, quote: Ticker, index: usize) -> Ref<Self> {
        let s = look.s;
        let swatches = Swatches::get();
        let symbol = look.text(10.0, &swatches.ink, true);
        symbol.set_text(Some(&quote.symbol));
        let price = look.text(10.0, &swatches.body, false);
        let change = look.text(10.0, &swatches.up, false);
        let bar_quote = quote.clone();
        let bar = Bar::new(2.0 * s, move |frame| share(&bar_quote, frame));
        bar.set_width(20.0 * s);
        bar.set_height(4.0 * s);
        bar.set_fill(swatches.palette[index % swatches.palette.len()].clone());
        let row = StackPanel::new();
        row.set_orientation(Orientation::Horizontal);
        row.set_spacing(4.0 * s);
        for control in [symbol.upcast::<Control>(), price.clone().upcast(), change.clone().upcast(), bar.clone().upcast()] {
            control.set_vertical_alignment(VerticalAlignment::Center);
            row.children().add(control);
        }
        let this = instantiate(Self { base: Border::construct(), quote, price, change, bar });
        this.set_background(Some(swatches.white.clone()));
        this.set_corner_radius(CornerRadius::uniform(6.0 * s));
        this.set_padding(Thickness::symmetric(6.0 * s, 3.0 * s));
        this.set_child(&row);
        this.on_frame(Clock::frame());
        this
    }

    fn on_frame(&self, frame: i32) {
        let swatches = Swatches::get();
        let cents = ticker_cents(&self.quote, frame);
        self.price.set_text(Some(&cents_text(cents)));
        self.change.set_text(Some(&change_text(&self.quote, cents)));
        self.change.set_foreground(Some(if cents >= self.quote.base_cents {
            swatches.up.clone()
        } else {
            swatches.down.clone()
        }));
        self.bar.invalidate_visual();
    }
}

// --- the cards -----------------------------------------------------------------------------------

/// A row of cards, rebound as the list reuses it.
#[repr(C)]
struct CardRow {
    base: Grid,
    cards: Vec<Ref<CardView>>,
    posts: Rc<[Post]>,
    avatars: Rc<[Rc<dyn IImage>]>,
}

ferro_class!(CardRow: Grid);
ferro_impl_classes!(
    CardRow: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    PanelImpl
);

impl CardRow {
    fn new(look: Look, columns: usize, posts: Rc<[Post]>, avatars: Rc<[Rc<dyn IImage>]>) -> Ref<Self> {
        let s = look.s;
        let cards: Vec<Ref<CardView>> = (0..columns)
            .map(|_| {
                let card = CardView::new(look);
                card.set_vertical_alignment(VerticalAlignment::Top);
                card
            })
            .collect();
        let this = instantiate(Self { base: Grid::construct(), cards, posts, avatars });
        this.set_margin(Thickness::new(8.0 * s, 0.0, 8.0 * s, 8.0 * s));
        add_columns(&this, columns, 8.0 * s);
        for (column, card) in this.cards.iter().enumerate() {
            Look::place(&this, card, column as i32 * 2);
        }
        this
    }

    fn bind(&self, first: usize) {
        for (column, view) in self.cards.iter().enumerate() {
            let card = first + column;
            view.bind(&self.posts[card % POST_COUNT], &self.avatars[card % AVATAR_COUNT], card as i32);
        }
    }
}

const FOOTER_LABELS: [&str; 3] = ["likes", "replies", "shares"];

#[repr(C)]
struct CardView {
    base: Panel,
    avatar: Ref<Image>,
    title: Ref<TextBlock>,
    author: Ref<Run>,
    tag: Ref<Run>,
    body: Ref<TextBlock>,
    progress: Ref<Bar>,
    percent: Ref<TextBlock>,
    sparkline: Ref<Sparkline>,
    chips: Vec<(Ref<Border>, Ref<TextBlock>)>,
    stats: Vec<Ref<TextBlock>>,
    badge: Ref<Border>,
    tilt: Ref<RotateTransform>,
    card: Rc<Cell<i32>>,
}

ferro_class!(CardView: Panel);
ferro_impl_classes!(
    CardView: FerroObjectImpl,
    StyledElementImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    PanelImpl
);

ferro_overrides! { impl VisualImpl for CardView {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);
        let view = this.to_ref();
        Clock::follow(this, move |frame| view.on_frame(frame));
    }

    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_detached_from_visual_tree(this, e);
        Clock::unfollow(this);
    }
} }

impl CardView {
    fn new(look: Look) -> Ref<Self> {
        let s = look.s;
        let swatches = Swatches::get();
        let card = Rc::new(Cell::new(0));

        let avatar = Image::new();
        avatar.set_width(32.0 * s);
        avatar.set_height(32.0 * s);
        avatar.set_stretch(Stretch::UniformToFill);
        let avatar_clip = Border::new();
        avatar_clip.set_corner_radius(CornerRadius::uniform(16.0 * s));
        avatar_clip.set_clip_to_bounds(true);
        avatar_clip.set_vertical_alignment(VerticalAlignment::Center);
        avatar_clip.set_margin(Thickness::new(0.0, 0.0, 8.0 * s, 0.0));
        avatar_clip.set_child(&avatar);
        let title = look.paragraph(13.0, &swatches.ink, 2, true);
        let author = Run::new();
        author.set_font_weight(FontWeight::Bold);
        let tag = Run::new();
        let subtitle = look.text(11.0, &swatches.muted, false);
        subtitle.set_max_lines(1);
        subtitle.set_text_trimming(<dyn TextTrimming>::character_ellipsis());
        // `by Ada Lovelace · #rust`: the author bold, the tag in its color.
        let inlines = InlineCollection::new();
        inlines.add(Run::with_text(Some("by ")));
        inlines.add(&author);
        inlines.add(Run::with_text(Some(" \u{b7} ")));
        inlines.add(&tag);
        subtitle.set_inlines(Some(inlines));
        let header_text = StackPanel::new();
        header_text.set_vertical_alignment(VerticalAlignment::Center);
        header_text.children().add(&title);
        header_text.children().add(&subtitle);
        let header = Grid::new();
        header.set_column_definitions(ColumnDefinitions::parse("Auto,*").expect("column definitions"));
        header.children().add(&avatar_clip);
        Look::place(&header, &header_text, 1);

        let body = look.paragraph(12.0, &swatches.body, 4, false);

        let progress_card = card.clone();
        let progress = Bar::new(3.0 * s, move |frame| f64::from(progress_permille(progress_card.get(), frame)) / 1000.0);
        progress.set_height(6.0 * s);
        progress.set_vertical_alignment(VerticalAlignment::Center);
        let percent = look.text(10.0, &swatches.muted, false);
        percent.set_vertical_alignment(VerticalAlignment::Center);
        percent.set_margin(Thickness::new(6.0 * s, 0.0, 0.0, 0.0));
        let progress_row = Grid::new();
        progress_row.set_column_definitions(ColumnDefinitions::parse("*,Auto").expect("column definitions"));
        progress_row.children().add(&progress);
        Look::place(&progress_row, &percent, 1);

        let sparkline = Sparkline::new(1.5 * s);
        sparkline.set_height(36.0 * s);

        let chip_flow = WrapPanel::new();
        chip_flow.set_item_spacing(4.0 * s);
        chip_flow.set_line_spacing(4.0 * s);
        let chips: Vec<(Ref<Border>, Ref<TextBlock>)> = (0..3)
            .map(|_| {
                let text = look.text(10.0, &swatches.ink, false);
                let chip = Look::rounded(None, 10.0 * s, Thickness::symmetric(8.0 * s, 3.0 * s), &text);
                chip_flow.children().add(&chip);
                (chip, text)
            })
            .collect();

        // Three counters split by dividers as tall as the tallest counter: the grid's row is as
        // tall as its tallest cell.
        let footer = Look::columns(FOOTER_LABELS.len(), 1.0);
        let mut stats = Vec::with_capacity(FOOTER_LABELS.len());
        for (index, footer_label) in FOOTER_LABELS.iter().enumerate() {
            let index = index as i32;
            if index > 0 {
                let divider = Border::new();
                divider.set_background(Some(swatches.hairline.clone()));
                Look::place(&footer, &divider, index * 2 - 1);
            }
            let stat = look.text(12.0, &swatches.ink, true);
            stat.set_horizontal_alignment(HorizontalAlignment::Center);
            let label = look.text(9.0, &swatches.muted, false);
            label.set_text(Some(footer_label));
            label.set_horizontal_alignment(HorizontalAlignment::Center);
            let counter = StackPanel::new();
            counter.children().add(&stat);
            counter.children().add(&label);
            Look::place(&footer, &counter, index * 2);
            stats.push(stat);
        }

        let column = StackPanel::new();
        column.set_spacing(6.0 * s);
        for child in [
            header.upcast::<Control>(),
            body.clone().upcast(),
            progress_row.upcast(),
            sparkline.clone().upcast(),
            chip_flow.upcast(),
            footer.upcast(),
        ] {
            column.children().add(child);
        }
        let frame_box = Border::new();
        frame_box.set_background(Some(swatches.white.clone()));
        frame_box.set_border_brush(Some(swatches.hairline.clone()));
        frame_box.set_border_thickness(Thickness::uniform(1.0));
        frame_box.set_corner_radius(CornerRadius::uniform(12.0 * s));
        // Compose draws its border over the padding; a FerroUI border insets the content by both,
        // so the padding gives the border's width back.
        frame_box.set_padding(Thickness::uniform(10.0 * s - 1.0));
        frame_box.set_box_shadow(BoxShadows::new(BoxShadow {
            offset_y: s,
            blur: 3.0 * s,
            color: Color::from_argb(0x3D, 0, 0, 0),
            ..BoxShadow::default()
        }));
        frame_box.set_child(&column);

        let hot = look.text(9.0, &swatches.white, true);
        hot.set_text(Some("HOT"));
        let badge = Look::rounded(Some(swatches.palette[0].clone()), 8.0 * s, Thickness::symmetric(6.0 * s, 2.0 * s), &hot);
        badge.set_horizontal_alignment(HorizontalAlignment::Right);
        badge.set_vertical_alignment(VerticalAlignment::Top);
        badge.set_margin(Thickness::uniform(6.0 * s));
        badge.set_opacity(0.9);
        // A translucent tag tilting with the frame: rotated, never measured again.
        let tilt = RotateTransform::new();
        badge.set_render_transform(Some((&tilt).into()));

        let this = instantiate(Self {
            base: Panel::construct(),
            avatar,
            title,
            author,
            tag,
            body,
            progress,
            percent,
            sparkline,
            chips,
            stats,
            badge: badge.clone(),
            tilt,
            card,
        });
        this.children().add(&frame_box);
        this.children().add(&badge);
        this
    }

    fn bind(&self, post: &Post, image: &Rc<dyn IImage>, card: i32) {
        let swatches = Swatches::get();
        self.card.set(card);
        self.avatar.set_source(Some(image.clone()));
        self.title.set_text(Some(&post.title));
        self.author.set_text(Some(&post.author));
        let (first_tag, tag_color) = post.tags[0];
        self.tag.set_text(Some(first_tag));
        self.tag.set_foreground(Some(swatches.gradient_end[tag_color].clone()));
        self.body.set_text(Some(&post.body));
        self.progress.set_fill(swatches.palette[card as usize % swatches.palette.len()].clone());
        self.sparkline.bind(card, PALETTE_ARGB[post.color]);
        for ((chip, text), (tag, color)) in self.chips.iter().zip(post.tags.iter()) {
            chip.set_background(Some(swatches.chip_background[*color].clone()));
            text.set_text(Some(tag));
            text.set_foreground(Some(swatches.gradient_end[*color].clone()));
        }
        for (stat, value) in self.stats.iter().zip(post.stats.iter()) {
            stat.set_text(Some(value));
        }
        self.badge.set_is_visible(card % 5 == 0);
        self.on_frame(Clock::frame());
    }

    fn on_frame(&self, frame: i32) {
        let card = self.card.get();
        self.percent.set_text(Some(&format!("{}%", progress_permille(card, frame) / 10)));
        self.progress.invalidate_visual();
        self.sparkline.invalidate_visual();
        if self.badge.is_visible() {
            self.tilt.set_angle(f64::from(badge_degrees(card, frame)));
        }
    }
}

/// A card's 48-point line over a fading fill, drawn from the frame.
#[repr(C)]
struct Sparkline {
    base: Control,
    stroke_width: f64,
    card: Cell<i32>,
    argb: Cell<u32>,
    line: RefCell<Rc<dyn IPen>>,
    fill: RefCell<Rc<dyn IBrush>>,
}

ferro_class!(Sparkline: Control);
ferro_impl_classes!(
    Sparkline: FerroObjectImpl,
    StyledElementImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

ferro_overrides! { impl VisualImpl for Sparkline {
    fn render(this: &Self, context: &mut DrawingContext) {
        let frame = Clock::frame();
        let card = this.card.get();
        let bounds = this.bounds();
        let (width, height) = (bounds.width, bounds.height);
        let step = width / (SPARK_POINTS - 1) as f64;
        let area = StreamGeometry::new();
        let stroke = StreamGeometry::new();
        {
            let mut area_path = area.open();
            let mut stroke_path = stroke.open();
            area_path.begin_figure(Point::new(0.0, height), true);
            for index in 0..SPARK_POINTS {
                let value = 1.0 - spark_value(card, index as i32, frame);
                let point = Point::new(index as f64 * step, height * f64::from(value));
                area_path.line_to(point, true);
                if index == 0 {
                    stroke_path.begin_figure(point, false);
                } else {
                    stroke_path.line_to(point, true);
                }
            }
            area_path.line_to(Point::new(width, height), true);
            area_path.end_figure(true);
            stroke_path.end_figure(false);
            area_path.dispose();
            stroke_path.dispose();
        }
        let fill = this.fill.borrow().clone();
        let line = this.line.borrow().clone();
        context.draw_geometry(Some(&fill), None, &area.upcast::<Geometry>());
        context.draw_geometry(None, Some(&line), &stroke.upcast::<Geometry>());
    }
} }

impl Sparkline {
    fn new(stroke_width: f64) -> Ref<Self> {
        let swatches = Swatches::get();
        let line: Rc<dyn IPen> = Rc::new(ImmutablePen::from_uint32(0xFF000000, 1.0));
        instantiate(Self {
            base: Control::construct(),
            stroke_width,
            card: Cell::new(0),
            argb: Cell::new(0),
            line: RefCell::new(line),
            fill: RefCell::new(swatches.transparent.clone()),
        })
    }

    fn bind(&self, card: i32, argb: u32) {
        self.card.set(card);
        if self.argb.get() == argb {
            return;
        }
        self.argb.set(argb);
        let color = Color::from_uint32(argb);
        *self.line.borrow_mut() = Rc::new(ImmutablePen::with_brush(
            Some(Rc::new(ImmutableSolidColorBrush::new(color))),
            self.stroke_width,
        ));
        *self.fill.borrow_mut() = Rc::new(ImmutableLinearGradientBrush::new(
            &[
                ImmutableGradientStop::new(0.0, Color::from_argb(0x40, color.r, color.g, color.b)),
                ImmutableGradientStop::new(1.0, Color::from_argb(0, color.r, color.g, color.b)),
            ],
            1.0,
            None,
            None,
            GradientSpreadMethod::Pad,
            Some(RelativePoint::new(0.0, 0.0, RelativeUnit::Relative)),
            Some(RelativePoint::new(0.0, 1.0, RelativeUnit::Relative)),
            None,
        ));
    }
}

// --- the clusters --------------------------------------------------------------------------------

/// `depth` levels nested inside one another, each a row of three labels above the next level.
#[repr(C)]
struct ClusterRow {
    base: Decorator,
    level: Level,
}

ferro_class!(ClusterRow: Decorator);
ferro_impl_classes!(
    ClusterRow: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

impl ClusterRow {
    fn new(look: Look, depth: i32) -> Ref<Self> {
        let level = Level::new(look, depth);
        let view = level.view.clone();
        let this = instantiate(Self { base: Decorator::construct(), level });
        this.set_margin(Thickness::new(8.0 * look.s, 0.0, 8.0 * look.s, 8.0 * look.s));
        this.set_child(&view);
        this
    }

    fn bind(&self, cluster: i32) {
        self.level.bind(cluster);
    }
}

struct Level {
    remaining: i32,
    chips: Vec<(Ref<Border>, Ref<TextBlock>)>,
    next: Option<Box<Level>>,
    view: Ref<Border>,
}

impl Level {
    fn new(look: Look, remaining: i32) -> Self {
        let s = look.s;
        let swatches = Swatches::get();
        let row = Look::columns(3, 2.0 * s);
        let chips: Vec<(Ref<Border>, Ref<TextBlock>)> = (0..3)
            .map(|chip| {
                let text = look.text(9.0, &swatches.white, false);
                text.set_text_wrapping(TextWrapping::Wrap);
                let border = Look::rounded(None, 2.0 * s, Thickness::uniform(0.0), &text);
                border.set_vertical_alignment(VerticalAlignment::Top);
                Look::place(&row, &border, chip * 2);
                (border, text)
            })
            .collect();
        let stack = StackPanel::new();
        stack.set_spacing(s);
        stack.children().add(&row);
        let next = if remaining > 0 {
            let next = Level::new(look, remaining - 1);
            stack.children().add(&next.view);
            Some(Box::new(next))
        } else {
            None
        };
        let view = Look::rounded(
            Some(swatches.level_background[(remaining % 2) as usize].clone()),
            4.0 * s,
            Thickness::new(3.0 * s, s, s, s),
            &stack,
        );
        Self { remaining, chips, next, view }
    }

    fn bind(&self, cluster: i32) {
        let swatches = Swatches::get();
        for (chip, (border, text)) in self.chips.iter().enumerate() {
            let chip = chip as i32;
            text.set_text(Some(&format!("C{cluster}.L{}.{chip}", self.remaining)));
            let color = ((self.remaining + chip + cluster) % swatches.palette.len() as i32) as usize;
            border.set_background(Some(swatches.palette[color].clone()));
        }
        if let Some(next) = &self.next {
            next.bind(cluster);
        }
    }
}

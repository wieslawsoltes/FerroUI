//! The patchwork: nine patches of the still wall's tiles beside a chat (`chat.rs`). Two patches
//! never change; in the others a frame changes a small part. Its data follows the patchwork data
//! of the benchmark bit for bit (`patchwork_data.rs`).
//!
//! In FerroUI's idiom every patch builds its controls once and a frame sets only the property that
//! changed: the brushes of the changed row of tiles, the text of the changed row of nests, the
//! translation of each card (a render transform, which lays out nothing), the order of two items
//! in a `WrapPanel`, one row of a virtualizing list, one word of a paragraph, the brushes of two tiles
//! of the pile. The charts are one control that builds its geometries once and draws them from the
//! retained render commands.

use crate::app::{DeviceRoboto, Launch};
use crate::chat::{ChatView, TextLeading};
use crate::chat_data::ChatData;
use crate::gauntlet::{brush, Clock, Look, Swatches};
use crate::patchwork_data::{
    card_tile, card_title, chart_color, chart_value, deep_color, deep_number, double, pairs, paragraph,
    paragraph_changed, paragraph_words, pile_grid, pile_opaque, recolor_color, reorder_count, reorder_swap,
    reorder_units, reorder_width, scroll_first, scroll_rows, tiles, Cards, InTurn, Patch, PatchTiles, CARD_FRAMES,
    CHART_POINTS, CHART_STROKE, DEPTH, GAP, PATCH_COLUMNS, PATCH_ROWS, PILE_ALPHA, PILE_OPAQUE_ARGB,
};
use crate::perf_data::PALETTE_ARGB;
use crate::wall::{TileView, WallData};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{LayoutableImpl, Orientation, VerticalAlignment};
use ferroui_base::media::immutable::ImmutablePen;
use ferroui_base::media::{
    DrawingContext, FontFamily, Geometry, IBrush, IPen, StreamGeometry, TextWrapping, TranslateTransform,
};
use ferroui_base::platform::IGeometryContext;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_overrides, instantiate, BoxedValue, CornerRadius, FerroObjectImpl, Point,
    Ref, StyledElementImpl, Thickness, TypeInfo, Vector, Visual, VisualImpl, VisualImplExt,
    VisualTreeAttachmentEventArgs,
};
use ferroui_controls::documents::TextElement;
use ferroui_controls::generators::RecycleKey;
use ferroui_controls::primitives::{ScrollBarVisibility, TemplatedControlImpl};
use ferroui_controls::templates::FuncTemplate;
use ferroui_controls::{
    Border, Canvas, CanvasImpl, ColumnDefinitions, Control, ControlImpl, Grid, ItemsControl, ItemsControlImpl,
    ItemsControlImplExt, ItemsSource, Panel, PanelImpl, RowDefinitions, ScrollViewer, StackPanel, StackPanelImpl,
    TextBlock, TopLevel, VirtualizingStackPanel, WrapPanel,
};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

/// One patch: its tiles and its size in points.
#[derive(Clone, Copy)]
struct PatchGrid {
    look: Look,
    tiles: PatchTiles,
    width: f64,
    height: f64,
}

impl PatchGrid {
    fn tile_width(&self) -> f64 {
        f64::from(self.tiles.width)
    }

    fn tile_height(&self) -> f64 {
        f64::from(self.tiles.height)
    }

    /// Where item `index` lies on a grid of `columns` items `width` by `height` points.
    fn cell(index: i32, columns: i32, width: f64, height: f64) -> Point {
        let gap = f64::from(GAP);
        Point::new(gap + f64::from(index % columns) * (width + gap), gap + f64::from(index / columns) * (height + gap))
    }

    /// A tile of the wall at the patch's tile size, its texts following that size.
    fn tile(&self, width: f64, height: f64, padding: f64) -> Ref<TileView> {
        TileView::new(self.look, width, height, self.tile_height(), padding)
    }
}

/// The window every desktop app opens, under the top bar and the clock line.
const AREA_WIDTH: f64 = 1280.0;
const AREA_HEIGHT: f64 = 820.0 - 56.0 - WallData::CLOCK_HEIGHT;

#[repr(C)]
pub struct PatchworkView {
    base: Grid,
    freeze: i32,
    clock: Ref<TextBlock>,
    first_frame_logged: Cell<bool>,
}

ferro_class!(PatchworkView: Grid);
ferro_impl_classes!(
    PatchworkView: FerroObjectImpl,
    StyledElementImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    PanelImpl
);

ferro_overrides! { impl VisualImpl for PatchworkView {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);
        let visual: &Visual = this;
        if let Some(top_level) = TopLevel::get_top_level(Some(visual)) {
            let view = this.to_ref();
            top_level.request_animation_frame(move |_| view.on_frame());
        }
    }
} }

impl PatchworkView {
    pub fn new(tier_index: i32, freeze: i32) -> Ref<Self> {
        let tier = WallData::tier(tier_index);
        let patch_width = ((AREA_WIDTH - f64::from(ChatData::WIDTH)) as f32 / PATCH_COLUMNS as f32).floor();
        let patch_height = (AREA_HEIGHT as f32 / PATCH_ROWS as f32).floor();
        let look = Look::new(1.0);
        let grid = PatchGrid {
            look,
            tiles: tiles(tier.0 as f32, tier.1 as f32, patch_width, patch_height),
            width: f64::from(patch_width),
            height: f64::from(patch_height),
        };
        let swatches = Swatches::get();

        let clock = look.text(WallData::CLOCK_TEXT, &swatches.white, false);
        clock.set_margin(Thickness::symmetric(8.0, 0.0));
        clock.set_vertical_alignment(VerticalAlignment::Center);
        Grid::set_row(&clock, 1);

        let patches = Canvas::new();
        for index in 0..PATCH_COLUMNS * PATCH_ROWS {
            let patch = build(Patch::ALL[index as usize], &grid);
            Canvas::set_left(&patch, f64::from((index % PATCH_COLUMNS) as f32 * patch_width));
            Canvas::set_top(&patch, f64::from((index / PATCH_COLUMNS) as f32 * patch_height));
            patches.children().add(&patch);
        }
        let body = Grid::new();
        body.set_column_definitions(
            ColumnDefinitions::parse(&format!("*,{}", ChatData::WIDTH)).expect("column definitions"),
        );
        body.children().add(&patches);
        let chat = ChatView::new();
        Grid::set_column(&chat, 1);
        body.children().add(&chat);
        Grid::set_row(&body, 2);

        let this = instantiate(Self {
            base: Grid::construct(),
            freeze,
            clock: clock.clone(),
            first_frame_logged: Cell::new(false),
        });
        TextElement::set_font_family_on(&this, FontFamily::new(DeviceRoboto::FAMILY));
        this.set_background(Some(brush(WallData::BACKGROUND_ARGB)));
        this.set_row_definitions(
            RowDefinitions::parse(&format!("56,{},*", WallData::CLOCK_HEIGHT)).expect("row definitions"),
        );
        this.children().add(Look::top_bar("Patchwork"));
        this.children().add(&clock);
        this.children().add(&body);
        this
    }

    fn on_frame(&self) {
        crate::gauntlet::report_frame_rate();
        if !self.first_frame_logged.get() {
            self.first_frame_logged.set(true);
            Launch::log("PERF first_frame");
        }
        let frame = Clock::advance();
        self.clock.set_text(Some(&WallData::clock(frame)));
        if self.freeze > 0 && frame >= self.freeze {
            Launch::log(&format!("PERF frozen frame={frame}"));
            return;
        }
        let visual: &Visual = self;
        if let Some(top_level) = TopLevel::get_top_level(Some(visual)) {
            let view = self.to_ref();
            top_level.request_animation_frame(move |_| view.on_frame());
        }
    }
}

fn build(patch: Patch, grid: &PatchGrid) -> Ref<Control> {
    match patch {
        Patch::Still => {
            let swatches = Swatches::get();
            tile_canvas(grid, |index| swatches.palette[index as usize % swatches.palette.len()].clone()).0.upcast()
        }
        Patch::Charts => charts_patch(grid),
        Patch::Recolor => RecolorPatch::build(grid),
        Patch::Cards => CardsPatch::build(grid),
        Patch::Reorder => ReorderPatch::build(grid),
        Patch::Deep => DeepPatch::build(grid),
        Patch::Scroll => ScrollPatch::build(grid),
        Patch::Paragraph => ParagraphPatch::build(grid),
        Patch::Pile => PilePatch::build(grid),
    }
}

// --- the patch's box -----------------------------------------------------------------------------

/// A patch's box, which cuts what lies past it. With [`PatchBox::follow`] it calls its patch every
/// frame while it is on screen.
#[repr(C)]
struct PatchBox {
    base: Canvas,
    on_frame: RefCell<Option<Rc<dyn Fn(i32)>>>,
}

ferro_class!(PatchBox: Canvas);
ferro_impl_classes!(
    PatchBox: FerroObjectImpl,
    StyledElementImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    PanelImpl,
    CanvasImpl
);

ferro_overrides! { impl VisualImpl for PatchBox {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);
        let on_frame = this.on_frame.borrow().clone();
        if let Some(on_frame) = on_frame {
            Clock::follow(this, move |frame| on_frame(frame));
        }
    }

    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_detached_from_visual_tree(this, e);
        if this.on_frame.borrow().is_some() {
            Clock::unfollow(this);
        }
    }
} }

impl PatchBox {
    fn new(grid: &PatchGrid) -> Ref<Self> {
        let this = instantiate(Self { base: Canvas::construct(), on_frame: RefCell::new(None) });
        this.set_width(grid.width);
        this.set_height(grid.height);
        this.set_clip_to_bounds(true);
        this
    }

    /// Calls `on_frame` every frame while the patch is on screen.
    fn follow(&self, on_frame: impl Fn(i32) + 'static) {
        *self.on_frame.borrow_mut() = Some(Rc::new(on_frame));
    }
}

/// Tiles on the grid of the still wall, each on the fill `fill` gives it.
fn tile_canvas(grid: &PatchGrid, fill: impl Fn(i32) -> Rc<dyn IBrush>) -> (Ref<PatchBox>, Vec<Ref<TileView>>) {
    let patch = PatchBox::new(grid);
    let tiles: Vec<Ref<TileView>> = (0..grid.tiles.count())
        .map(|index| {
            let tile = grid.tile(grid.tile_width(), grid.tile_height(), grid.tile_height() * 0.06);
            tile.bind(index);
            tile.set_background(Some(fill(index)));
            let at = PatchGrid::cell(index, grid.tiles.columns, grid.tile_width(), grid.tile_height());
            Canvas::set_left(&tile, at.x);
            Canvas::set_top(&tile, at.y);
            patch.children().add(&tile);
            tile
        })
        .collect();
    (patch, tiles)
}

// --- the charts ----------------------------------------------------------------------------------

/// Line charts that never change. One control builds every chart's geometry once; its render
/// commands are retained, so no frame draws them again.
fn charts_patch(grid: &PatchGrid) -> Ref<Control> {
    let patch = PatchBox::new(grid);
    let charts = Charts::new(grid);
    charts.set_width(grid.width);
    charts.set_height(grid.height);
    patch.children().add(&charts);
    patch.upcast()
}

#[repr(C)]
struct Charts {
    base: Control,
    lines: Vec<(Ref<Geometry>, Rc<dyn IPen>)>,
}

ferro_class!(Charts: Control);
ferro_impl_classes!(
    Charts: FerroObjectImpl,
    StyledElementImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

ferro_overrides! { impl VisualImpl for Charts {
    fn render(this: &Self, context: &mut DrawingContext) {
        for (line, pen) in &this.lines {
            context.draw_geometry(None, Some(pen), line);
        }
    }
} }

impl Charts {
    fn new(grid: &PatchGrid) -> Ref<Self> {
        let (columns, rows) = pairs(grid.tiles.columns, grid.tiles.rows);
        let width = double(grid.tiles.width);
        let height = double(grid.tiles.height);
        let lines = (0..columns * rows)
            .map(|index| {
                let origin = PatchGrid::cell(index, columns, f64::from(width), f64::from(height));
                let step = width / (CHART_POINTS - 1) as f32;
                let line = StreamGeometry::new();
                {
                    let mut path = line.open();
                    for point in 0..CHART_POINTS {
                        let at = Point::new(
                            origin.x + f64::from(point as f32 * step),
                            origin.y + f64::from(height * (1.0 - chart_value(index, point))),
                        );
                        if point == 0 {
                            path.begin_figure(at, false);
                        } else {
                            path.line_to(at, true);
                        }
                    }
                    path.end_figure(false);
                    path.dispose();
                }
                let pen: Rc<dyn IPen> =
                    Rc::new(ImmutablePen::from_uint32(PALETTE_ARGB[chart_color(index)], f64::from(CHART_STROKE)));
                (line.upcast::<Geometry>(), pen)
            })
            .collect();
        instantiate(Self { base: Control::construct(), lines })
    }
}

// --- recolor -------------------------------------------------------------------------------------

/// Tiles whose fill changes in turn: a frame sets the brushes of one row.
struct RecolorPatch {
    tiles: Vec<Ref<TileView>>,
    columns: i32,
    turn: InTurn,
}

impl RecolorPatch {
    fn build(grid: &PatchGrid) -> Ref<Control> {
        let swatches = Swatches::get();
        let turn = grid.tiles.in_turn();
        let (patch, tiles) = tile_canvas(grid, |index| swatches.palette[recolor_color(index, 0, turn)].clone());
        let recolor = RecolorPatch { tiles, columns: grid.tiles.columns, turn };
        patch.follow(move |frame| recolor.show(frame));
        patch.upcast()
    }

    fn show(&self, frame: i32) {
        let swatches = Swatches::get();
        let first = self.turn.changed_row(frame) * self.columns;
        for index in first..first + self.columns {
            self.tiles[index as usize]
                .set_background(Some(swatches.palette[recolor_color(index, frame, self.turn)].clone()));
        }
    }
}

// --- cards ---------------------------------------------------------------------------------------

/// A card: a header over rows of tiles. It moves by a render transform and rebinds to another
/// card's number without new controls.
#[repr(C)]
struct PatchCard {
    base: StackPanel,
    cards: Cards,
    title: Ref<TextBlock>,
    header: Ref<Border>,
    tiles: Vec<Ref<TileView>>,
    shift: Ref<TranslateTransform>,
}

ferro_class!(PatchCard: StackPanel);
ferro_impl_classes!(
    PatchCard: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    PanelImpl,
    StackPanelImpl
);

impl PatchCard {
    fn new(grid: &PatchGrid, cards: Cards) -> Ref<Self> {
        let swatches = Swatches::get();
        let title = grid.look.text(grid.tile_height() * 0.30, &swatches.white, true);
        let header = Border::new();
        header.set_width(f64::from(reorder_width(cards.columns, grid.tile_width() as f32)));
        header.set_height(grid.tile_height());
        header.set_corner_radius(CornerRadius::uniform(WallData::CORNER));
        header.set_padding(Thickness::uniform(grid.tile_height() * 0.06));
        header.set_clip_to_bounds(true);
        header.set_child(&title);
        let mut tiles = Vec::with_capacity(cards.tiles() as usize);
        let mut lines = Vec::with_capacity(cards.rows as usize);
        for _ in 0..cards.rows {
            let line = StackPanel::new();
            line.set_orientation(Orientation::Horizontal);
            line.set_spacing(f64::from(GAP));
            for _ in 0..cards.columns {
                let tile = grid.tile(grid.tile_width(), grid.tile_height(), grid.tile_height() * 0.06);
                line.children().add(&tile);
                tiles.push(tile);
            }
            lines.push(line);
        }
        let shift = TranslateTransform::new();
        let this = instantiate(Self {
            base: StackPanel::construct(),
            cards,
            title,
            header: header.clone(),
            tiles,
            shift: shift.clone(),
        });
        this.set_spacing(f64::from(GAP));
        this.set_margin(Thickness::symmetric(f64::from(GAP), 0.0));
        this.set_render_transform(Some((&shift).into()));
        this.children().add(&header);
        for line in &lines {
            this.children().add(line);
        }
        this
    }

    /// Shows card `id`.
    fn bind(&self, id: i32) {
        let swatches = Swatches::get();
        self.header.set_background(Some(swatches.gradient_end[id as usize % swatches.gradient_end.len()].clone()));
        self.title.set_text(Some(&card_title(id)));
        for (tile, view) in self.tiles.iter().enumerate() {
            let index = card_tile(id, tile as i32, self.cards.tiles());
            view.bind(index);
            view.set_background(Some(swatches.palette[index as usize % swatches.palette.len()].clone()));
        }
    }
}

/// Cards that move every frame. The set of cards changes when a move begins, one card in and one
/// out; within a move a frame sets each card's translation and nothing else.
struct CardsPatch {
    grid: PatchGrid,
    patch: Ref<PatchBox>,
    cards: Cards,
    placed: RefCell<Vec<Option<Ref<PatchCard>>>>,
    shown: RefCell<HashMap<i32, Ref<PatchCard>>>,
    spare: RefCell<Vec<Ref<PatchCard>>>,
    moves: Cell<i32>,
}

impl CardsPatch {
    fn build(grid: &PatchGrid) -> Ref<Control> {
        let patch = PatchBox::new(grid);
        let cards = Cards::of(grid.tiles.columns, grid.tiles.rows, grid.tile_height() as f32, grid.height as f32);
        let this = Rc::new(CardsPatch {
            grid: *grid,
            patch: patch.clone(),
            cards,
            placed: RefCell::new(vec![None; cards.places() as usize]),
            shown: RefCell::new(HashMap::new()),
            spare: RefCell::new(Vec::new()),
            moves: Cell::new(-1),
        });
        this.show(0);
        patch.follow(move |frame| this.show(frame));
        patch.upcast()
    }

    fn show(&self, frame: i32) {
        let moves = frame / CARD_FRAMES;
        if moves != self.moves.get() {
            self.moves.set(moves);
            self.place(moves * CARD_FRAMES);
        }
        for (place, card) in self.placed.borrow().iter().enumerate() {
            if let Some(card) = card {
                let top = self.cards.place(place as i32, frame).map_or(-self.cards.height, |at| at.top);
                card.shift.set_y(f64::from(top));
            }
        }
    }

    // The cards of a move: the ones of the move before keep their controls and a card that left
    // gives its controls to the one that came in.
    fn place(&self, begun: i32) {
        let ids: Vec<Option<i32>> =
            (0..self.cards.places()).map(|place| self.cards.place(place, begun).map(|at| at.id)).collect();
        let mut shown = self.shown.borrow_mut();
        let left: Vec<i32> = shown.keys().copied().filter(|id| !ids.contains(&Some(*id))).collect();
        for id in left {
            if let Some(card) = shown.remove(&id) {
                self.patch.children().remove(&card);
                self.spare.borrow_mut().push(card);
            }
        }
        let mut placed = self.placed.borrow_mut();
        for (place, id) in ids.iter().enumerate() {
            let Some(id) = *id else {
                placed[place] = None;
                continue;
            };
            let card = match shown.get(&id) {
                Some(card) => card.clone(),
                None => {
                    let card = self.spare.borrow_mut().pop().unwrap_or_else(|| PatchCard::new(&self.grid, self.cards));
                    card.bind(id);
                    shown.insert(id, card.clone());
                    self.patch.children().add(&card);
                    card
                }
            };
            // A later place lies over an earlier one.
            card.set_z_index(place as i32);
            placed[place] = Some(card);
        }
    }
}

// --- reorder -------------------------------------------------------------------------------------

/// A wrapping row of items, two of which change places every frame: a frame moves those two
/// children of the panel and creates none.
struct ReorderPatch {
    items: Ref<WrapPanel>,
    count: i32,
    swapped: Cell<i32>,
}

impl ReorderPatch {
    fn build(grid: &PatchGrid) -> Ref<Control> {
        let swatches = Swatches::get();
        let patch = PatchBox::new(grid);
        let count = reorder_count(grid.tiles.count());
        let items = WrapPanel::new();
        items.set_item_spacing(f64::from(GAP));
        items.set_line_spacing(f64::from(GAP));
        for item in 0..count {
            let tile = grid.tile(
                f64::from(reorder_width(reorder_units(item), grid.tile_width() as f32)),
                grid.tile_height(),
                grid.tile_height() * 0.06,
            );
            tile.bind(item);
            tile.set_background(Some(swatches.palette[item as usize % swatches.palette.len()].clone()));
            items.children().add(&tile);
        }
        let frame = Border::new();
        frame.set_width(grid.width);
        frame.set_height(grid.height);
        frame.set_padding(Thickness::uniform(f64::from(GAP)));
        frame.set_child(&items);
        patch.children().add(&frame);
        let reorder = ReorderPatch { items, count, swapped: Cell::new(-1) };
        reorder.show(0);
        patch.follow(move |frame| reorder.show(frame));
        patch.upcast()
    }

    fn show(&self, frame: i32) {
        if self.count < 2 {
            return;
        }
        let swap = reorder_swap(frame, self.count);
        let swapped = self.swapped.get();
        if swap == swapped {
            return;
        }
        let children = self.items.children();
        if swapped >= 0 {
            children.move_item(swapped as usize, swapped as usize + 1);
        }
        children.move_item(swap as usize, swap as usize + 1);
        self.swapped.set(swap);
    }
}

// --- deep ----------------------------------------------------------------------------------------

/// Nests of boxes around digits that change in turn: a frame sets the text of one row of nests.
struct DeepPatch {
    numbers: Vec<Ref<TextBlock>>,
    turn: InTurn,
}

impl DeepPatch {
    fn build(grid: &PatchGrid) -> Ref<Control> {
        let swatches = Swatches::get();
        let patch = PatchBox::new(grid);
        let (columns, rows) = pairs(grid.tiles.columns, grid.tiles.rows);
        let turn = InTurn::new(columns, rows);
        let width = f64::from(double(grid.tile_width() as f32));
        let height = f64::from(double(grid.tile_height() as f32));
        let numbers = (0..columns * rows)
            .map(|index| {
                let number = grid.look.text(grid.tile_height() * 0.30, &swatches.white, true);
                number.set_text(Some(&Self::digits(deep_number(index, 0, turn))));
                let mut inner: Ref<Control> = number.clone().upcast();
                for level in (0..DEPTH).rev() {
                    let nest = Border::new();
                    nest.set_background(Some(swatches.palette[deep_color(index, level)].clone()));
                    nest.set_padding(Thickness::uniform(grid.tile_height() * 0.06));
                    nest.set_child(&inner);
                    inner = nest.upcast();
                }
                inner.set_width(width);
                inner.set_height(height);
                let at = PatchGrid::cell(index, columns, width, height);
                Canvas::set_left(&inner, at.x);
                Canvas::set_top(&inner, at.y);
                patch.children().add(&inner);
                number
            })
            .collect();
        let deep = DeepPatch { numbers, turn };
        patch.follow(move |frame| deep.show(frame));
        patch.upcast()
    }

    fn digits(number: i32) -> String {
        format!("{number:03}")
    }

    fn show(&self, frame: i32) {
        let first = self.turn.changed_row(frame) * self.turn.columns;
        for index in first..first + self.turn.columns {
            self.numbers[index as usize].set_text(Some(&Self::digits(deep_number(index, frame, self.turn))));
        }
    }
}

// --- scroll --------------------------------------------------------------------------------------

/// A list of rows of tiles that scrolls by one row every frame. The list creates the rows on the
/// screen and binds each to its row of tiles. Its container override gives no recycle key, as the
/// .NET app's does, so a row that leaves is removed and the row that comes in is made anew.
struct ScrollPatch {
    scroller: Ref<ScrollViewer>,
    total: i32,
    visible: i32,
    pitch: f64,
}

impl ScrollPatch {
    fn build(grid: &PatchGrid) -> Ref<Control> {
        let patch = PatchBox::new(grid);
        let total = scroll_rows(grid.tiles.columns);
        let rows = TileRows::new(*grid);
        rows.set_items_source(Some(ItemsSource::from_values(0..total)));
        rows.set_items_panel(FuncTemplate::new(|| Some(VirtualizingStackPanel::new().upcast::<Panel>())));
        rows.set_padding(Thickness::new(0.0, f64::from(GAP), 0.0, 0.0));
        let scroller = ScrollViewer::new();
        scroller.set_width(grid.width);
        scroller.set_height(grid.height);
        scroller.set_content(Some(Control::boxed(&rows)));
        scroller.set_vertical_scroll_bar_visibility(ScrollBarVisibility::Hidden);
        scroller.set_horizontal_scroll_bar_visibility(ScrollBarVisibility::Disabled);
        patch.children().add(&scroller);
        let scroll = ScrollPatch {
            scroller,
            total,
            visible: grid.tiles.rows,
            pitch: grid.tile_height() + f64::from(GAP),
        };
        patch.follow(move |frame| scroll.show(frame));
        patch.upcast()
    }

    fn show(&self, frame: i32) {
        self.scroller
            .set_offset(Vector::new(0.0, f64::from(scroll_first(frame, self.total, self.visible)) * self.pitch));
    }
}

/// The list of the scroll patch: a row of tiles for every item.
#[repr(C)]
struct TileRows {
    base: ItemsControl,
    grid: PatchGrid,
}

ferro_class!(TileRows: ItemsControl);
ferro_impl_classes!(
    TileRows: FerroObjectImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl
);

ferro_overrides! { impl StyledElementImpl for TileRows {
    fn style_key_override(_this: &Self) -> &'static TypeInfo {
        ItemsControl::TYPE
    }
} }

ferro_overrides! { impl ItemsControlImpl for TileRows {
    fn needs_container_override(_this: &Self, _item: &Option<BoxedValue>, _index: i32) -> (bool, Option<RecycleKey>) {
        (true, None)
    }

    fn create_container_for_item_override(
        this: &Self,
        _item: &Option<BoxedValue>,
        _index: i32,
        _recycle_key: Option<RecycleKey>
    ) -> Ref<Control> {
        TileRow::new(&this.grid).upcast()
    }

    fn prepare_container_for_item_override(
        this: &Self,
        container: &Ref<Control>,
        item: &Option<BoxedValue>,
        index: i32
    ) {
        Self::parent_prepare_container_for_item_override(this, container, item, index);
        if let Some(row) = container.downcast_ref::<TileRow>() {
            row.bind(index);
        }
    }
} }

impl TileRows {
    fn new(grid: PatchGrid) -> Ref<Self> {
        instantiate(Self { base: ItemsControl::construct(), grid })
    }
}

/// One row of the scroll patch's list, rebound as the list reuses it.
#[repr(C)]
struct TileRow {
    base: StackPanel,
    tiles: Vec<Ref<TileView>>,
}

ferro_class!(TileRow: StackPanel);
ferro_impl_classes!(
    TileRow: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    PanelImpl,
    StackPanelImpl
);

impl TileRow {
    fn new(grid: &PatchGrid) -> Ref<Self> {
        let tiles: Vec<Ref<TileView>> = (0..grid.tiles.columns)
            .map(|_| grid.tile(grid.tile_width(), grid.tile_height(), grid.tile_height() * 0.06))
            .collect();
        let this = instantiate(Self { base: StackPanel::construct(), tiles });
        this.set_orientation(Orientation::Horizontal);
        this.set_spacing(f64::from(GAP));
        this.set_margin(Thickness::new(f64::from(GAP), 0.0, 0.0, f64::from(GAP)));
        for tile in &this.tiles {
            this.children().add(tile);
        }
        this
    }

    fn bind(&self, row: i32) {
        let swatches = Swatches::get();
        let columns = self.tiles.len() as i32;
        for (column, tile) in self.tiles.iter().enumerate() {
            let index = row * columns + column as i32;
            tile.bind(index);
            tile.set_background(Some(swatches.palette[index as usize % swatches.palette.len()].clone()));
        }
    }
}

// --- paragraph -----------------------------------------------------------------------------------

/// One paragraph, one word of which changes every frame. The paragraph's characters live in a
/// buffer that a frame patches at the word that changed and at the one that changed before.
struct ParagraphPatch {
    text: Ref<TextBlock>,
    characters: RefCell<Vec<u8>>,
    words: i32,
    changed: Cell<i32>,
}

impl ParagraphPatch {
    fn build(grid: &PatchGrid) -> Ref<Control> {
        let patch = PatchBox::new(grid);
        let words = paragraph_words(grid.tiles.count());
        let first = paragraph(0, words);
        let text = grid.look.text(grid.tile_height() * 0.30, &Swatches::get().white, false);
        text.set_line_height(text.font_size() * TextLeading::EM);
        text.set_text(Some(&first));
        text.set_width(grid.width - 2.0 * f64::from(GAP));
        text.set_text_wrapping(TextWrapping::Wrap);
        Canvas::set_left(&text, f64::from(GAP));
        Canvas::set_top(&text, f64::from(GAP));
        patch.children().add(&text);
        let paragraph = ParagraphPatch {
            text,
            characters: RefCell::new(first.into_bytes()),
            words,
            changed: Cell::new(paragraph_changed(0, words)),
        };
        patch.follow(move |frame| paragraph.show(frame));
        patch.upcast()
    }

    fn show(&self, frame: i32) {
        let mut characters = self.characters.borrow_mut();
        Self::write(&mut characters, self.changed.get(), self.changed.get());
        let changed = paragraph_changed(frame, self.words);
        self.changed.set(changed);
        Self::write(&mut characters, changed, frame % 100_000);
        // The buffer holds ASCII digits and spaces only.
        self.text.set_text(Some(std::str::from_utf8(&characters).unwrap_or_default()));
    }

    // The word at `word` shows `number` in five digits; a number of more digits leaves it as it
    // was, as the C# formatting into the word's five characters does.
    fn write(characters: &mut [u8], word: i32, number: i32) {
        if !(0..100_000).contains(&number) {
            return;
        }
        let start = word as usize * 6;
        let mut rest = number;
        for at in (start..start + 5).rev() {
            characters[at] = b'0' + (rest % 10) as u8;
            rest /= 10;
        }
    }
}

// --- pile ----------------------------------------------------------------------------------------

/// Translucent tiles lying over one another, one of which is opaque. A frame sets the fill of the
/// tile that was opaque and of the one that is.
struct PilePatch {
    tiles: Vec<Ref<TileView>>,
    translucent: Vec<Rc<dyn IBrush>>,
    opaque_fill: Rc<dyn IBrush>,
    opaque: Cell<i32>,
}

impl PilePatch {
    fn build(grid: &PatchGrid) -> Ref<Control> {
        let translucent: Vec<Rc<dyn IBrush>> =
            PALETTE_ARGB.iter().map(|argb| brush((argb & 0x00FF_FFFF) | (u32::from(PILE_ALPHA) << 24))).collect();
        let opaque_fill = brush(PILE_OPAQUE_ARGB);
        let patch = PatchBox::new(grid);
        let (columns, rows) = pile_grid(grid.tiles.columns, grid.tiles.rows);
        let tiles: Vec<Ref<TileView>> = (0..columns * rows)
            .map(|index| {
                let tile = grid.tile(
                    f64::from(double(grid.tile_width() as f32)),
                    f64::from(double(grid.tile_height() as f32)),
                    0.0,
                );
                tile.bind(index);
                tile.set_background(Some(translucent[index as usize % translucent.len()].clone()));
                let at = PatchGrid::cell(index, columns, grid.tile_width(), grid.tile_height());
                Canvas::set_left(&tile, at.x);
                Canvas::set_top(&tile, at.y);
                patch.children().add(&tile);
                tile
            })
            .collect();
        let opaque = pile_opaque(0, tiles.len() as i32);
        tiles[opaque as usize].set_background(Some(opaque_fill.clone()));
        let pile = PilePatch { tiles, translucent, opaque_fill, opaque: Cell::new(opaque) };
        patch.follow(move |frame| pile.show(frame));
        patch.upcast()
    }

    fn show(&self, frame: i32) {
        let was = self.opaque.get();
        self.tiles[was as usize].set_background(Some(self.translucent[was as usize % self.translucent.len()].clone()));
        let opaque = pile_opaque(frame, self.tiles.len() as i32);
        self.opaque.set(opaque);
        self.tiles[opaque as usize].set_background(Some(self.opaque_fill.clone()));
    }
}

//! The chat beside the patchwork: an agent's answers arrive from a data thread as tokens and the
//! list shows them as they come. Its data follows the chat data of the benchmark bit for bit
//! (`chat_data.rs`).
//!
//! In FerroUI's idiom the list is an `ItemsControl` over a `VirtualizingStackPanel` that holds only
//! the number of blocks. A block's control is made once per place in the turn and recycled for the
//! same place in the next turn. A token sets the text of the one block that grew: a code block
//! sets the length of its colored runs, a table adds a row it built before. The list follows its
//! end from the scroll viewer's own change event. The spinner turns by a render transform and the
//! status text moves its gradient brush.

use crate::app::Launch;
use crate::chat_data::{ChatBlock, ChatData, ChatToken, CodeKind};
use crate::chat_gif;
use crate::gauntlet::{brush, Clock, Look, Swatches};
use ferroui_base::collections::FerroList;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{HorizontalAlignment, LayoutableImpl, Orientation, VerticalAlignment};
use ferroui_base::media::fonts::{FontCollectionBase, FontCollectionBaseImpl};
use ferroui_base::media::imaging::WriteableBitmap;
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::{
    Color, FontFamily, FontManager, FontWeight, Geometry, GradientSpreadMethod, GradientStop, GradientStops, IBrush,
    IImage, LinearGradientBrush, PenLineCap, RotateTransform, TextWrapping,
};
use ferroui_base::platform::{AlphaFormat, ILockedFramebuffer, PixelFormat};
use ferroui_base::utilities::UriRef;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_overrides, instantiate, BoxedValue, CornerRadius, FerroObjectImpl,
    PixelSize, Ref, RelativePoint, RelativeUnit, StyledElementImpl, Thickness, TypeInfo, Vector, VisualImpl,
    VisualImplExt, VisualTreeAttachmentEventArgs,
};
use ferroui_controls::documents::{InlineCollection, Run};
use ferroui_controls::generators::RecycleKey;
use ferroui_controls::primitives::{ScrollBarVisibility, TemplatedControlImpl};
use ferroui_controls::shapes::Path;
use ferroui_controls::templates::FuncTemplate;
use ferroui_controls::{
    Border, Button, Canvas, ColumnDefinitions, Control, ControlImpl, Grid, Image, ItemsControl, ItemsControlImpl,
    ItemsControlImplExt, ItemsSource, Panel, PanelImpl, RowDefinitions, ScrollViewer, StackPanel, StackPanelImpl,
    TextBlock, TextBlockImpl, Viewbox, VirtualizingStackPanel,
};
use std::cell::{Cell, OnceCell};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::mpsc::{self, Receiver};
use std::thread;

/// The tokens of the chat as they leave the data layer: a thread of its own sends them through a
/// bounded queue and waits while the screen is behind.
struct ChatFeed {
    tokens: Receiver<ChatToken>,
}

impl ChatFeed {
    fn new() -> Self {
        let (sender, tokens) = mpsc::sync_channel(ChatData::TOKENS_PER_FRAME as usize * 4);
        thread::Builder::new()
            .name("chat-data".to_string())
            .spawn(move || {
                let mut count: i32 = 0;
                loop {
                    count = count.wrapping_add(1);
                    // The queue closes with the screen, and the thread ends with it.
                    if sender.send(ChatData::token_after(count)).is_err() {
                        return;
                    }
                }
            })
            .expect("the chat's data thread");
        Self { tokens }
    }

    /// The next token, when one has arrived. The screen never waits for one.
    fn try_take(&self) -> Option<ChatToken> {
        self.tokens.try_recv().ok()
    }
}

/// The chat's pictures and the monospaced face, read from the folders `PERF_CHAT` and `PERF_FONTS`
/// name.
struct ChatAssets;

impl ChatAssets {
    /// The path of `file` in the folder `PERF_CHAT` names (default `chat-assets`).
    fn path(file: &str) -> PathBuf {
        std::env::var_os("PERF_CHAT").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("chat-assets")).join(file)
    }

    fn read(file: &str) -> Vec<u8> {
        let path = Self::path(file);
        std::fs::read(&path).unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()))
    }

    /// The frames of the animated picture, decoded once for every block that shows it.
    fn gif() -> Rc<[Rc<dyn IImage>]> {
        ferroui_base::ferro_thread_local! {
            static GIF: OnceCell<Rc<[Rc<dyn IImage>]>> = const { OnceCell::new() };
        }
        GIF.with(|gif| gif.get_or_init(Self::decode_gif).clone())
    }

    fn decode_gif() -> Rc<[Rc<dyn IImage>]> {
        let picture = chat_gif::decode(&Self::read("spin.gif"))
            .unwrap_or_else(|error| panic!("cannot decode {}: {error}", Self::path("spin.gif").display()));
        let (width, height) = (picture.width, picture.height);
        picture
            .frames
            .iter()
            .map(|rgba| {
                let bitmap = WriteableBitmap::new(
                    PixelSize::new(width as i32, height as i32),
                    Vector::new(96.0, 96.0),
                    Some(PixelFormat::RGBA8888),
                    Some(AlphaFormat::Opaque),
                );
                let buffer = bitmap.lock();
                let row = width * 4;
                let row_bytes = buffer.row_bytes() as usize;
                buffer.with_data(&mut |pixels| {
                    for y in 0..height {
                        pixels[y * row_bytes..y * row_bytes + row].copy_from_slice(&rgba[y * row..(y + 1) * row]);
                    }
                });
                buffer.dispose();
                Rc::new(bitmap) as Rc<dyn IImage>
            })
            .collect()
    }

    /// The value of the attribute `name` in the text of the tag `tag`.
    fn attribute<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
        let pattern = format!("{name}=\"");
        let mut from = 0;
        while let Some(found) = tag[from..].find(&pattern) {
            let start = from + found;
            if tag[..start].ends_with(char::is_whitespace) {
                return tag[start + pattern.len()..].split('"').next();
            }
            from = start + pattern.len();
        }
        None
    }

    fn number(tag: &str, name: &str) -> f64 {
        Self::attribute(tag, name)
            .and_then(|value| value.trim().parse().ok())
            .unwrap_or_else(|| panic!("progress.svg: no number in the attribute {name}"))
    }

    /// The spinner of the status line: the paths of `progress.svg` on a canvas of its size.
    fn spinner() -> Ref<Canvas> {
        let bytes = Self::read("progress.svg");
        let svg = String::from_utf8_lossy(&bytes);
        // The text of each start tag, up to its `>`.
        let tags = |name: &str| -> Vec<String> {
            svg.split(&format!("<{name}"))
                .skip(1)
                .map(|tag| format!(" {}", tag.split('>').next().unwrap_or(tag)))
                .collect()
        };
        let root = tags("svg").into_iter().next().expect("progress.svg: no svg element");
        let canvas = Canvas::new();
        canvas.set_width(Self::number(&root, "width"));
        canvas.set_height(Self::number(&root, "height"));
        for tag in tags("path") {
            let data = Self::attribute(&tag, "d").expect("progress.svg: a path without data");
            let stroke = Self::attribute(&tag, "stroke").expect("progress.svg: a path without a stroke");
            let path = Path::new();
            path.set_data(Geometry::parse(data).expect("progress.svg: the data of a path"));
            path.set_stroke(Some(Rc::new(ImmutableSolidColorBrush::new(
                Color::parse(stroke).expect("progress.svg: the stroke of a path"),
            ))));
            path.set_stroke_thickness(Self::number(&tag, "stroke-width"));
            path.set_stroke_line_cap(if Self::attribute(&tag, "stroke-linecap") == Some("round") {
                PenLineCap::Round
            } else {
                PenLineCap::Flat
            });
            canvas.children().add(&path);
        }
        canvas
    }
}

/// Lines of the patchwork and the chat lie 1.4 em apart, as the gauntlet's paragraphs do.
pub struct TextLeading;

impl TextLeading {
    pub const EM: f64 = 1.4;
}

/// The monospaced face of the code, from the folder the other faces come from.
pub struct DeviceMono {
    base: FontCollectionBase,
    key: url::Url,
}

impl DeviceMono {
    pub const FAMILY: &'static str = "fonts:Mono#DejaVu Sans Mono";

    pub fn new() -> Self {
        let collection = Self {
            base: FontCollectionBase::new(),
            key: UriRef::parse_absolute("fonts:Mono").expect("the key of the collection"),
        };
        let mut stream = std::io::Cursor::new(Launch::read_font("DejaVuSansMono.ttf"));
        FontCollectionBase::try_add_glyph_typeface_from_stream(&collection, &mut stream);
        collection
    }
}

impl FontCollectionBaseImpl for DeviceMono {
    fn base(this: &Self) -> &FontCollectionBase {
        &this.base
    }

    fn key(this: &Self) -> url::Url {
        this.key.clone()
    }
}

/// What the chat shows: its last block and how much of it shows. Every block before it shows
/// whole.
struct ChatState {
    last_block: Cell<i32>,
    last_shown: Cell<i32>,
}

impl ChatState {
    fn new() -> Self {
        Self { last_block: Cell::new(-1), last_shown: Cell::new(0) }
    }

    /// The tokens of block `index` that show.
    fn shown(&self, index: i32) -> i32 {
        if index == self.last_block.get() {
            self.last_shown.get()
        } else {
            i32::MAX
        }
    }
}

/// A block of the chat that shows `shown` tokens of itself.
trait Block {
    fn bind(&self, index: i32, shown: i32);
}

/// The block a control of the chat's list is, if it is one.
fn as_block(control: &Control) -> Option<&dyn Block> {
    if let Some(block) = control.downcast_ref::<UserBlock>() {
        return Some(block);
    }
    if let Some(block) = control.downcast_ref::<TypedBlock>() {
        return Some(block);
    }
    if let Some(block) = control.downcast_ref::<CodeBlock>() {
        return Some(block);
    }
    if let Some(block) = control.downcast_ref::<TableBlock>() {
        return Some(block);
    }
    if let Some(block) = control.downcast_ref::<GifBlock>() {
        return Some(block);
    }
    if let Some(block) = control.downcast_ref::<ButtonsBlock>() {
        return Some(block);
    }
    None
}

#[repr(C)]
pub struct ChatView {
    base: Grid,
    feed: ChatFeed,
    state: Rc<ChatState>,
    blocks: FerroList<i32>,
    list: Ref<Blocks>,
    status: Ref<TextBlock>,
    shimmer: Ref<LinearGradientBrush>,
    turn: Ref<RotateTransform>,
}

ferro_class!(ChatView: Grid);
ferro_impl_classes!(
    ChatView: FerroObjectImpl,
    StyledElementImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    PanelImpl
);

ferro_overrides! { impl VisualImpl for ChatView {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);
        let view = this.to_ref();
        Clock::follow(this, move |frame| view.show(frame));
    }

    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_detached_from_visual_tree(this, e);
        Clock::unfollow(this);
    }
} }

impl ChatView {
    pub fn new() -> Ref<Self> {
        FontManager::current().add_font_collection(Rc::new(DeviceMono::new()));
        let state = Rc::new(ChatState::new());
        let blocks = FerroList::new();
        let list = Blocks::new(state.clone());
        list.set_items_source(Some(ItemsSource::new(Rc::new(blocks.clone()))));
        list.set_items_panel(FuncTemplate::new(|| Some(VirtualizingStackPanel::new().upcast::<Panel>())));
        list.set_padding(Thickness::new(8.0, 8.0, 8.0, 0.0));
        let scroller = ScrollViewer::new();
        scroller.set_content(Some(Control::boxed(&list)));
        scroller.set_vertical_scroll_bar_visibility(ScrollBarVisibility::Hidden);
        scroller.set_horizontal_scroll_bar_visibility(ScrollBarVisibility::Disabled);
        // A block that grew lengthens the list: the list follows its end.
        let follower = scroller.downgrade();
        let _ = scroller.scroll_changed(move |_, e| {
            if e.extent_delta().y != 0.0 {
                if let Some(scroller) = follower.upgrade() {
                    scroller.scroll_to_end();
                }
            }
        });

        let turn = RotateTransform::new();
        let spinner = Viewbox::new();
        spinner.set_width(f64::from(ChatData::SPINNER_SIZE));
        spinner.set_height(f64::from(ChatData::SPINNER_SIZE));
        spinner.set_child(&ChatAssets::spinner());
        spinner.set_render_transform(Some((&turn).into()));
        let shimmer = LinearGradientBrush::new();
        shimmer.set_spread_method(GradientSpreadMethod::Repeat);
        shimmer.set_gradient_stops(GradientStops::from_items([
            GradientStop::with_color_and_offset(Color::from_uint32(ChatData::SHIMMER_DIM_ARGB), 0.0),
            GradientStop::with_color_and_offset(Color::from_uint32(ChatData::SHIMMER_BRIGHT_ARGB), 0.5),
            GradientStop::with_color_and_offset(Color::from_uint32(ChatData::SHIMMER_DIM_ARGB), 1.0),
        ]));
        let shimmer_brush: Rc<dyn IBrush> = (&shimmer).into();
        let status = Look::new(1.0).text(f64::from(ChatData::TEXT), &shimmer_brush, false);
        status.set_line_height(f64::from(ChatData::TEXT) * TextLeading::EM);
        let line = StackPanel::new();
        line.set_orientation(Orientation::Horizontal);
        line.set_spacing(6.0);
        line.set_margin(Thickness::symmetric(8.0, 0.0));
        line.set_vertical_alignment(VerticalAlignment::Center);
        line.children().add(&spinner);
        line.children().add(&status);
        let bar = Border::new();
        bar.set_background(Some(brush(ChatData::PANEL_ARGB)));
        bar.set_child(&line);
        Grid::set_row(&bar, 1);

        let this = instantiate(Self {
            base: Grid::construct(),
            feed: ChatFeed::new(),
            state,
            blocks,
            list,
            status,
            shimmer,
            turn,
        });
        this.set_width(f64::from(ChatData::WIDTH));
        this.set_background(Some(brush(ChatData::BACKGROUND_ARGB)));
        this.set_row_definitions(
            RowDefinitions::parse(&format!("*,{}", ChatData::STATUS_HEIGHT)).expect("row definitions"),
        );
        this.children().add(&scroller);
        this.children().add(&bar);
        this
    }

    // Takes the frame's tokens, never waiting for one, and shows the last of them.
    fn show(&self, frame: i32) {
        self.turn.set_angle(f64::from(frame as f32 * ChatData::SPIN_DEGREES));
        let start = frame as f32 * ChatData::SHIMMER_STEP % ChatData::SHIMMER_SPAN;
        self.shimmer.set_start_point(RelativePoint::new(f64::from(start), 0.0, RelativeUnit::Absolute));
        self.shimmer.set_end_point(RelativePoint::new(
            f64::from(start + ChatData::SHIMMER_SPAN),
            0.0,
            RelativeUnit::Absolute,
        ));

        let mut last = None;
        for _ in 0..ChatData::TOKENS_PER_FRAME {
            match self.feed.try_take() {
                Some(token) => last = Some(token),
                None => break,
            }
        }
        let Some(arrived) = last else { return };
        let before = self.state.last_block.get();
        self.state.last_block.set(arrived.block);
        self.state.last_shown.set(arrived.shown);
        self.status.set_text(Some(&ChatData::status(arrived)));
        // The block that was last shows whole once another begins; a block of the list not on
        // screen binds when the list makes its control.
        if before >= 0 && before != arrived.block {
            if let Some(whole) = self.list.container_from_index(before) {
                if let Some(block) = as_block(&whole) {
                    block.bind(before, i32::MAX);
                }
            }
        }
        while self.blocks.count() <= arrived.block as usize {
            self.blocks.add(self.blocks.count() as i32);
        }
        if let Some(grown) = self.list.container_from_index(arrived.block) {
            if let Some(block) = as_block(&grown) {
                block.bind(arrived.block, arrived.shown);
            }
        }
    }
}

// --- the list ------------------------------------------------------------------------------------

/// The list: a control for each place of the turn, recycled for that place in the next turn.
#[repr(C)]
struct Blocks {
    base: ItemsControl,
    state: Rc<ChatState>,
}

ferro_class!(Blocks: ItemsControl);
ferro_impl_classes!(
    Blocks: FerroObjectImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl
);

ferro_overrides! { impl StyledElementImpl for Blocks {
    fn style_key_override(_this: &Self) -> &'static TypeInfo {
        ItemsControl::TYPE
    }
} }

/// The place of block `index` in its turn.
fn place(index: i32) -> usize {
    index as usize % ChatData::TURN.len()
}

ferro_overrides! { impl ItemsControlImpl for Blocks {
    fn needs_container_override(_this: &Self, _item: &Option<BoxedValue>, index: i32) -> (bool, Option<RecycleKey>) {
        (true, Some(RecycleKey::Id(place(index) as u64)))
    }

    fn create_container_for_item_override(
        _this: &Self,
        _item: &Option<BoxedValue>,
        index: i32,
        _recycle_key: Option<RecycleKey>
    ) -> Ref<Control> {
        Blocks::block(&ChatData::TURN[place(index)])
    }

    fn prepare_container_for_item_override(
        this: &Self,
        container: &Ref<Control>,
        item: &Option<BoxedValue>,
        index: i32
    ) {
        Self::parent_prepare_container_for_item_override(this, container, item, index);
        if let Some(block) = as_block(container) {
            block.bind(index, this.state.shown(index));
        }
    }
} }

impl Blocks {
    fn new(state: Rc<ChatState>) -> Ref<Self> {
        instantiate(Self { base: ItemsControl::construct(), state })
    }

    fn block(block: &ChatBlock) -> Ref<Control> {
        match *block {
            ChatBlock::User(text) => UserBlock::new(text).upcast(),
            ChatBlock::Heading(text) => TypedBlock::new(text, true).upcast(),
            ChatBlock::Paragraph(text) => TypedBlock::new(text, false).upcast(),
            ChatBlock::Code(code) => CodeBlock::new(code).upcast(),
            ChatBlock::Table(rows) => TableBlock::new(rows).upcast(),
            ChatBlock::Gif => GifBlock::new().upcast(),
            ChatBlock::Buttons(labels) => ButtonsBlock::new(labels).upcast(),
        }
    }
}

// --- the blocks ----------------------------------------------------------------------------------

/// The user's question, whole at once in a bubble at the right.
#[repr(C)]
struct UserBlock {
    base: Border,
}

ferro_class!(UserBlock: Border);
ferro_impl_classes!(
    UserBlock: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

impl UserBlock {
    fn new(text: &str) -> Ref<Self> {
        let line = Look::new(1.0).text(f64::from(ChatData::TEXT), &Swatches::get().white, false);
        line.set_text(Some(text));
        line.set_line_height(f64::from(ChatData::TEXT) * TextLeading::EM);
        line.set_text_wrapping(TextWrapping::Wrap);
        let this = instantiate(Self { base: Border::construct() });
        this.set_background(Some(brush(ChatData::USER_ARGB)));
        this.set_corner_radius(CornerRadius::uniform(10.0));
        this.set_padding(Thickness::symmetric(10.0, 6.0));
        this.set_horizontal_alignment(HorizontalAlignment::Right);
        this.set_margin(Thickness::new(0.0, 0.0, 0.0, 8.0));
        this.set_child(&line);
        this
    }
}

impl Block for UserBlock {
    fn bind(&self, _index: i32, _shown: i32) {}
}

/// A paragraph, or a heading in the bold face: the turn's number, then as much of the text as
/// arrived.
#[repr(C)]
struct TypedBlock {
    base: TextBlock,
    text: &'static str,
    turn: Cell<i32>,
    chars: Cell<usize>,
}

ferro_class!(TypedBlock: TextBlock);
ferro_impl_classes!(
    TypedBlock: FerroObjectImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TextBlockImpl
);

ferro_overrides! { impl StyledElementImpl for TypedBlock {
    fn style_key_override(_this: &Self) -> &'static TypeInfo {
        TextBlock::TYPE
    }
} }

impl TypedBlock {
    fn new(text: &'static str, heading: bool) -> Ref<Self> {
        let this =
            instantiate(Self { base: TextBlock::construct(), text, turn: Cell::new(-1), chars: Cell::new(usize::MAX) });
        this.set_font_size(f64::from(if heading { ChatData::HEADING_TEXT } else { ChatData::TEXT }));
        this.set_font_weight(if heading { FontWeight::Bold } else { FontWeight::Normal });
        this.set_line_height(this.font_size() * TextLeading::EM);
        this.set_foreground(Some(Swatches::get().white.clone()));
        this.set_text_wrapping(TextWrapping::Wrap);
        this.set_margin(Thickness::new(0.0, 0.0, 0.0, 8.0));
        this
    }
}

impl Block for TypedBlock {
    fn bind(&self, index: i32, shown: i32) {
        let now = ChatData::shown_chars(self.text, shown);
        let turn = ChatData::turn_of(index);
        if self.turn.get() == turn && self.chars.get() == now {
            return;
        }
        self.turn.set(turn);
        self.chars.set(now);
        self.set_text(Some(&format!("{} {}", ChatData::turn_label(turn), &self.text[..now])));
    }
}

/// Code in a monospaced face on a panel. Its runs are colored once; a token sets the length of the
/// runs it reaches.
#[repr(C)]
struct CodeBlock {
    base: Border,
    code: &'static str,
    head: Ref<Run>,
    runs: Vec<(Ref<Run>, &'static str, usize)>,
    turn: Cell<i32>,
    chars: Cell<usize>,
}

ferro_class!(CodeBlock: Border);
ferro_impl_classes!(
    CodeBlock: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

impl CodeBlock {
    fn new(code: &'static str) -> Ref<Self> {
        let white = Swatches::get().white.clone();
        let body = TextBlock::new();
        body.set_font_family(FontFamily::new(DeviceMono::FAMILY));
        body.set_font_size(f64::from(ChatData::CODE_TEXT));
        body.set_line_height(f64::from(ChatData::CODE_TEXT) * TextLeading::EM);
        body.set_foreground(Some(white.clone()));
        body.set_text_wrapping(TextWrapping::Wrap);
        let inlines = InlineCollection::new();
        let head = Run::new();
        head.set_foreground(Some(Self::tint(CodeKind::Comment)));
        inlines.add(&head);
        let mut runs = Vec::new();
        let mut add = |text: &'static str, color: Rc<dyn IBrush>, start: usize| {
            if text.is_empty() {
                return;
            }
            let run = Run::new();
            run.set_foreground(Some(color));
            inlines.add(&run);
            runs.push((run, text, start));
        };
        let mut at = 0;
        for span in ChatData::code_spans(code) {
            add(&code[at..span.start], white.clone(), at);
            add(&code[span.start..span.end], Self::tint(span.kind), span.start);
            at = span.end;
        }
        add(&code[at..], white.clone(), at);
        body.set_inlines(Some(inlines));
        let this = instantiate(Self {
            base: Border::construct(),
            code,
            head,
            runs,
            turn: Cell::new(-1),
            chars: Cell::new(0),
        });
        this.set_background(Some(brush(ChatData::PANEL_ARGB)));
        this.set_corner_radius(CornerRadius::uniform(6.0));
        this.set_padding(Thickness::uniform(8.0));
        this.set_margin(Thickness::new(0.0, 0.0, 0.0, 8.0));
        this.set_child(&body);
        this
    }

    fn tint(kind: CodeKind) -> Rc<dyn IBrush> {
        brush(ChatData::code_argb(kind))
    }
}

impl Block for CodeBlock {
    fn bind(&self, index: i32, shown: i32) {
        let turn = ChatData::turn_of(index);
        if self.turn.get() != turn {
            self.turn.set(turn);
            self.head.set_text(Some(&format!("// {}\n", ChatData::turn_label(turn))));
        }
        let now = ChatData::shown_chars(self.code, shown);
        let low = self.chars.get().min(now);
        let high = self.chars.get().max(now);
        for (run, text, start) in &self.runs {
            if start + text.len() <= low || *start >= high {
                continue;
            }
            let visible = now.saturating_sub(*start).min(text.len());
            run.set_text(Some(&text[..visible]));
        }
        self.chars.set(now);
    }
}

/// A table whose first row is its header. Its rows are built once; a token adds the next one.
#[repr(C)]
struct TableBlock {
    base: StackPanel,
    rows: Vec<Ref<Border>>,
}

ferro_class!(TableBlock: StackPanel);
ferro_impl_classes!(
    TableBlock: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    PanelImpl,
    StackPanelImpl
);

impl TableBlock {
    fn new(cells: &[&[&str]]) -> Ref<Self> {
        let look = Look::new(1.0);
        let white = Swatches::get().white.clone();
        let rows = cells
            .iter()
            .enumerate()
            .map(|(index, row)| {
                let line = Grid::new();
                line.set_column_definitions(ColumnDefinitions::parse("*,*,*,*").expect("column definitions"));
                for (column, text) in row.iter().enumerate() {
                    let cell = look.text(f64::from(ChatData::CODE_TEXT), &white, index == 0);
                    cell.set_text(Some(*text));
                    cell.set_line_height(f64::from(ChatData::CODE_TEXT) * TextLeading::EM);
                    Grid::set_column(&cell, column as i32);
                    line.children().add(&cell);
                }
                let border = Border::new();
                if index == 0 || index % 2 == 1 {
                    border.set_background(Some(brush(ChatData::PANEL_ARGB)));
                }
                border.set_padding(Thickness::symmetric(6.0, 3.0));
                border.set_child(&line);
                border
            })
            .collect();
        let this = instantiate(Self { base: StackPanel::construct(), rows });
        this.set_margin(Thickness::new(0.0, 0.0, 0.0, 8.0));
        this
    }
}

impl Block for TableBlock {
    fn bind(&self, _index: i32, shown: i32) {
        let count = (shown.max(0) as usize).min(self.rows.len());
        let children = self.children();
        while children.count() > count {
            children.remove_at(children.count() - 1);
        }
        while children.count() < count {
            children.add(&self.rows[children.count()]);
        }
    }
}

/// The animated picture: its frames were decoded once and a frame sets the picture it shows.
#[repr(C)]
struct GifBlock {
    base: Image,
    frames: Rc<[Rc<dyn IImage>]>,
    current: Cell<i32>,
}

ferro_class!(GifBlock: Image);
ferro_impl_classes!(
    GifBlock: FerroObjectImpl,
    StyledElementImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

ferro_overrides! { impl VisualImpl for GifBlock {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);
        let picture = this.to_ref();
        Clock::follow(this, move |frame| picture.show(frame));
    }

    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_detached_from_visual_tree(this, e);
        Clock::unfollow(this);
    }
} }

impl GifBlock {
    fn new() -> Ref<Self> {
        let this = instantiate(Self { base: Image::construct(), frames: ChatAssets::gif(), current: Cell::new(-1) });
        this.set_width(f64::from(ChatData::GIF_SIZE));
        this.set_height(f64::from(ChatData::GIF_SIZE));
        this.set_horizontal_alignment(HorizontalAlignment::Left);
        this.set_margin(Thickness::new(0.0, 0.0, 0.0, 8.0));
        this
    }

    fn show(&self, frame: i32) {
        let next = ChatData::gif_frame(frame, self.frames.len() as i32);
        if next == self.current.get() {
            return;
        }
        self.current.set(next);
        self.set_source(self.frames.get(next as usize).cloned());
    }
}

impl Block for GifBlock {
    fn bind(&self, _index: i32, _shown: i32) {
        self.show(Clock::frame());
    }
}

/// A row of buttons.
#[repr(C)]
struct ButtonsBlock {
    base: StackPanel,
}

ferro_class!(ButtonsBlock: StackPanel);
ferro_impl_classes!(
    ButtonsBlock: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    PanelImpl,
    StackPanelImpl
);

impl ButtonsBlock {
    fn new(labels: &[&str]) -> Ref<Self> {
        let this = instantiate(Self { base: StackPanel::construct() });
        this.set_orientation(Orientation::Horizontal);
        this.set_spacing(8.0);
        this.set_margin(Thickness::new(0.0, 0.0, 0.0, 8.0));
        for label in labels {
            let button = Button::new();
            button.set_content(Some(Rc::new(label.to_string())));
            button.set_background(Some(brush(ChatData::USER_ARGB)));
            button.set_foreground(Some(Swatches::get().white.clone()));
            button.set_font_size(f64::from(ChatData::TEXT));
            button.set_font_weight(FontWeight::Bold);
            button.set_corner_radius(CornerRadius::uniform(6.0));
            button.set_padding(Thickness::symmetric(10.0, 6.0));
            button.set_border_thickness(Thickness::uniform(0.0));
            this.children().add(&button);
        }
        this
    }
}

impl Block for ButtonsBlock {
    fn bind(&self, _index: i32, _shown: i32) {}
}

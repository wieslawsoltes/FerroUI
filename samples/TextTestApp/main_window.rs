//! Port of `MainWindow.xaml.cs`: the class of the document `MainWindow.xaml`. The window
//! lists the shaped buffers of the line the line control formats (one row per glyph) and its
//! character hits (one row per character), and shows the rows that are selected as rectangles
//! over the line.

use crate::grid_row::GridRow;
use crate::interactive_line_control::InteractiveLineControl;
use crate::markup::xaml_class;
use crate::selection_adorner::SelectionAdorner;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::input::{InputElementImpl, InputElementImplExt, Key, KeyEventArgs, PointerEventArgs};
use ferroui_base::interactivity::{IRoutedEventArgs, InteractiveImpl, RoutedEventArgs};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::text_formatting::{
    GlyphInfo, ShapedTextRun, TextCharacters, TextEndOfParagraph, TextLine, TextRun, UnshapedTextRun,
};
use ferroui_base::media::{
    Brushes, CharacterHit, Colors, DrawingImage, FontWeight, Geometry, GeometryDrawing, GlyphInfoList, GlyphRun,
    GlyphTypeface, IBrush, IImage, SolidColorBrush, TextWrapping,
};
use ferroui_base::utilities::ReadOnlyMemory;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, AnyValue, BoxedValue, FerroObjectImpl, Matrix, Point,
    Rect, Ref, StyledElementImpl, Thickness, Vector, VisualImpl,
};
use ferroui_controls::platform::PlatformManager;
use ferroui_controls::primitives::{AdornerLayer, TemplatedControlImpl};
use ferroui_controls::{
    Border, ContentControlImpl, Control, ControlImpl, Image, ListBox, SelectionChangedEventArgs, SelectionMode, TextBlock,
    ToggleSwitch, ToolTip, TopLevelImpl, Window, WindowBaseImpl, WindowImpl,
};
use std::cell::RefCell;
use std::rc::Rc;

#[repr(C)]
pub struct MainWindow {
    base: Window,
    selection_adorner: RefCell<Option<Ref<SelectionAdorner>>>,
}

ferro_class!(MainWindow: Window);
ferro_impl_classes!(
    MainWindow: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl,
    TopLevelImpl,
    WindowBaseImpl,
    WindowImpl
);
ferro_class_info!(MainWindow {
    new: MainWindow::new,
    markup: {
        methods: [
            fn OnNewWindowClick(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<MainWindow>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_new_window_click(&sender, e.as_routed_event_args())
                },
            fn OnPointerMoved(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<MainWindow>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_pointer_moved(&sender, pointer_args(&e))
                },
            fn OnHitTestMethodChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<MainWindow>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_hit_test_method_changed(&sender, e.as_routed_event_args())
                },
            fn OnHitsSelectionChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<MainWindow>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_hits_selection_changed(&sender, selection_changed_args(&e))
                },
            fn OnBufferSelectionChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<MainWindow>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_buffer_selection_changed(&sender, selection_changed_args(&e))
                },
        ],
    },
});
xaml_class!(MainWindow, "/MainWindow.xaml");

fn pointer_args(e: &Rc<dyn IRoutedEventArgs>) -> &PointerEventArgs {
    e.downcast_ref::<PointerEventArgs>().expect("the arguments of a pointer event")
}

fn selection_changed_args(e: &Rc<dyn IRoutedEventArgs>) -> &SelectionChangedEventArgs {
    e.downcast_ref::<SelectionChangedEventArgs>().expect("the arguments of a selection changed event")
}

thread_local! {
    static TRANSPARENT_ALICE_BLUE: Rc<dyn IBrush> = SolidColorBrush::from_uint32(0x0F0188FF).into();
    static TRANSPARENT_ANTIQUE_WHITE: Rc<dyn IBrush> = SolidColorBrush::from_uint32(0x28DF8000).into();
}

/// A text run as the tag of the row that names it: runs have no equality, and a tag is a
/// value that has one, so the tag compares the runs by identity.
#[derive(Clone)]
pub struct TextRunTag(pub Rc<dyn TextRun>);

impl PartialEq for TextRunTag {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

/// `run.GetType().Name`: the name of the class of a text run. A run does not state its class
/// (GAPS.md, T004): the runs of the framework are named, any other run by its base class.
fn type_name_of(run: &dyn TextRun) -> &'static str {
    if run.is::<ShapedTextRun>() {
        "ShapedTextRun"
    } else if run.is::<TextEndOfParagraph>() {
        "TextEndOfParagraph"
    } else if run.as_text_end_of_line().is_some() {
        "TextEndOfLine"
    } else if run.is::<TextCharacters>() {
        "TextCharacters"
    } else if run.is::<UnshapedTextRun>() {
        "UnshapedTextRun"
    } else if run.as_drawable().is_some() {
        "DrawableTextRun"
    } else {
        "TextRun"
    }
}

/// `text.Substring(start, length)` of a text in UTF-16 code units.
///
/// # Panics
/// Panics if the range is not inside the text (an argument out of range in the managed
/// original).
fn substring(text: &[u16], start: i32, length: i32) -> &[u16] {
    assert!(start >= 0 && length >= 0, "Index and length must refer to a location within the string.");
    &text[start as usize..start as usize + length as usize]
}

/// The tag of the control an item of a list box is, when the tag is a `T`
/// (`item is Control { Tag: T tag }`).
fn tag_of<T: Clone + 'static>(item: &Option<BoxedValue>) -> Option<T> {
    let control = Control::from_boxed(item.as_ref()?)?;
    let tag = control.tag()?;
    let tag: &dyn AnyValue = &*tag;
    tag.downcast_ref::<T>().cloned()
}

impl InputElementImpl for MainWindow {
    fn on_key_down(this: &Self, e: &KeyEventArgs) {
        if e.key == Key::F5 {
            this.rendering().invalidate_visual();
            this.on_shape_buffer_changed();
            e.set_handled(true);
        } else if e.key == Key::Escape {
            let hits = this.hits();
            let buffer = this.buffer();
            if hits.is_keyboard_focus_within() && hits.selected_index() != -1 {
                hits.set_selected_index(-1);
                e.set_handled(true);
            } else if buffer.is_keyboard_focus_within() && buffer.selected_index() != -1 {
                buffer.set_selected_index(-1);
                e.set_handled(true);
            }
        }

        Self::parent_on_key_down(this, e);
    }
}

impl MainWindow {
    pub fn construct() -> Self {
        Self { base: Window::construct(PlatformManager::create_window()), selection_adorner: RefCell::new(None) }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();

        let selection_adorner = SelectionAdorner::new();
        *this.selection_adorner.borrow_mut() = Some(selection_adorner.clone());
        selection_adorner.set_stroke(Some(Brushes::red() as Rc<dyn IBrush>));
        selection_adorner.set_fill(Some(SolidColorBrush::with_color_and_opacity(Colors::LIGHT_SKY_BLUE, 0.25).into()));
        selection_adorner.set_is_hit_test_visible(false);
        AdornerLayer::set_is_clip_enabled(&selection_adorner, false);
        AdornerLayer::set_adorner(&this.rendering(), selection_adorner);

        // The handler belongs to a child of the window: it holds the window weakly.
        let weak = this.downgrade();
        this.rendering().text_line_changed(move || {
            if let Some(this) = weak.upgrade() {
                this.on_shape_buffer_changed();
            }
        });
        this.on_shape_buffer_changed();

        this
    }

    // The named elements of the document (the fields the managed original generates).

    fn hit_range_toggle(&self) -> Ref<ToggleSwitch> {
        self.get_control::<ToggleSwitch>("_hitRangeToggle")
    }

    fn coordinates(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("_coordinates")
    }

    fn hit(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("_hit")
    }

    fn rendering(&self) -> Ref<InteractiveLineControl> {
        self.get_control::<InteractiveLineControl>("_rendering")
    }

    fn buffer(&self) -> Ref<ListBox> {
        self.get_control::<ListBox>("_buffer")
    }

    fn hits(&self) -> Ref<ListBox> {
        self.get_control::<ListBox>("_hits")
    }

    fn on_new_window_click(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let win = MainWindow::new();
        win.show();
    }

    fn on_shape_buffer_changed(&self) {
        let Some(selection_adorner) = self.selection_adorner.borrow().clone() else {
            return;
        };

        self.list_buffers();
        self.list_hits();

        let bounds = self.rendering().line_render_bounds();
        selection_adorner.set_transform(Matrix::create_translation(bounds.x, bounds.y));
    }

    fn list_buffers(&self) {
        let buffer = self.buffer();
        let mut i = buffer.item_count() - 1;
        while i >= 1 {
            buffer.items().remove_at(i as usize);
            i -= 1;
        }

        let rendering = self.rendering();
        let Some(text_line) = rendering.text_line() else {
            return;
        };

        let mut current_x = rendering.line_render_bounds().left();
        let text_runs: Vec<Rc<dyn TextRun>> = text_line.text_runs().to_vec();
        for run in text_runs {
            if let Some(shaped_run) = run.downcast_ref::<ShapedTextRun>() {
                let text_block = TextBlock::new();
                text_block.set_text(Some(&format!(
                    "{}: Bidi = {}, Font = {}",
                    type_name_of(&*run),
                    shaped_run.bidi_level(),
                    shaped_run.shaped_buffer().glyph_typeface().family_name()
                )));
                text_block.set_font_weight(FontWeight::Bold);
                text_block.set_padding(Thickness::symmetric(10.0, 0.0));
                text_block.set_tag(Some(Rc::new(TextRunTag(run.clone())) as BoxedValue));
                buffer.items().add(Some(Control::boxed(text_block)));

                self.list_buffer(&*text_line, shaped_run, &mut current_x);
            } else {
                let text_block = TextBlock::new();
                text_block.set_text(Some(type_name_of(&*run)));
                text_block.set_font_weight(FontWeight::Bold);
                text_block.set_padding(Thickness::symmetric(10.0, 0.0));
                text_block.set_tag(Some(Rc::new(TextRunTag(run.clone())) as BoxedValue));
                buffer.items().add(Some(Control::boxed(text_block)));
            }
        }
    }

    fn list_hits(&self) {
        let hits = self.hits();
        let mut i = hits.item_count() - 1;
        while i >= 1 {
            hits.items().remove_at(i as usize);
            i -= 1;
        }

        let rendering = self.rendering();
        let Some(text_line) = rendering.text_line() else {
            return;
        };

        for i in 0..text_line.length() {
            let text: Vec<u16> = rendering.text().expect("the text of a formatted line").encode_utf16().collect();
            let cluster = substring(&text, i, 1);
            let cluster_text = String::from_utf16_lossy(cluster);
            let cluster_hex = to_hex(cluster);

            let hit = CharacterHit::new(i);
            let prev_hit = text_line.get_previous_caret_character_hit(hit);
            let next_hit = text_line.get_next_caret_character_hit(hit);
            let bksp_hit = text_line.get_backspace_caret_character_hit(hit);

            let row = GridRow::new();
            row.set_column_spacing(10.0);
            row.children().add(Control::new());
            row.children()
                .add(text_block(Some(&format!("{}+{}", bksp_hit.first_character_index(), bksp_hit.trailing_length()))));
            row.children()
                .add(text_block(Some(&format!("{}+{}", prev_hit.first_character_index(), prev_hit.trailing_length()))));
            let index_block = text_block(Some(&i.to_string()));
            index_block.set_font_weight(FontWeight::Bold);
            row.children().add(index_block);
            row.children()
                .add(text_block(Some(&format!("{}+{}", next_hit.first_character_index(), next_hit.trailing_length()))));
            row.children().add(text_block(Some(&cluster_hex)));
            row.children().add(text_block(Some(&cluster_text)));
            row.children().add(text_block(Some(&text_line.get_distance_from_character_hit(hit).to_string())));
            row.set_tag(Some(Rc::new(i) as BoxedValue));

            hits.items().add(Some(Control::boxed(row)));
        }
    }

    fn list_buffer(&self, _text_line: &dyn TextLine, shaped_run: &ShapedTextRun, current_x: &mut f64) {
        let buffer = shaped_run.shaped_buffer();

        let mut last_cluster_start = -1;
        let mut odd_cluster = false;

        let glyph_infos: Vec<GlyphInfo> = buffer.glyph_infos().to_vec();

        let find_cluster_lenght_at = |index: usize| -> i32 {
            let cluster = glyph_infos[index].glyph_cluster;
            if shaped_run.bidi_level() % 2 == 0 {
                let mut index = index + 1;
                while index < glyph_infos.len() {
                    if glyph_infos[index].glyph_cluster != cluster {
                        return glyph_infos[index].glyph_cluster - cluster;
                    }
                    index += 1;
                }

                TextRun::length(shaped_run) + glyph_infos[0].glyph_cluster - cluster
            } else {
                let mut index = index;
                while index > 0 {
                    index -= 1;
                    if glyph_infos[index].glyph_cluster != cluster {
                        return glyph_infos[index].glyph_cluster - cluster;
                    }
                }

                TextRun::length(shaped_run) + glyph_infos[glyph_infos.len() - 1].glyph_cluster - cluster
            }
        };

        let rendering = self.rendering();
        let list = self.buffer();

        *current_x += shaped_run.glyph_run().baseline_origin().x;
        for i in 0..glyph_infos.len() {
            let info = glyph_infos[i];
            let cluster_start = info.glyph_cluster;
            let cluster_length = find_cluster_lenght_at(i);
            let text: Vec<u16> = rendering.text().expect("the text of a formatted line").encode_utf16().collect();
            let cluster = substring(&text, cluster_start, cluster_length);
            let mut cluster_text = Some(String::from_utf16_lossy(cluster));
            let mut cluster_hex = Some(to_hex(cluster));

            let border = Border::new();
            if cluster_start == last_cluster_start {
                cluster_text = None;
                cluster_hex = None;
            } else {
                odd_cluster = !odd_cluster;
                last_cluster_start = cluster_start;
            }
            border.set_background(Some(if odd_cluster {
                TRANSPARENT_ALICE_BLUE.with(Rc::clone)
            } else {
                TRANSPARENT_ANTIQUE_WHITE.with(Rc::clone)
            }));

            let row = GridRow::new();
            row.set_column_spacing(10.0);
            row.children().add(Control::new());
            row.children().add(text_block(Some(&cluster_start.to_string())));
            row.children().add(text_block(cluster_text.as_deref()));
            let hex_block = text_block(cluster_hex.as_deref());
            hex_block.set_text_wrapping(TextWrapping::Wrap);
            row.children().add(hex_block);
            let image = Image::new();
            image.set_source(Some(self.create_glyph_drawing(shaped_run.glyph_run().glyph_typeface(), self.font_size(), info)));
            image.set_margin(Thickness::uniform(2.0));
            row.children().add(image);
            row.children().add(text_block(Some(&info.glyph_index.to_string())));
            row.children().add(text_block(Some(&info.glyph_advance.to_string())));
            row.children().add(text_block(Some(&info.glyph_offset.to_string())));

            let glyph = self.get_glyph_outline(
                shaped_run.glyph_run().glyph_typeface(),
                shaped_run.glyph_run().font_rendering_em_size(),
                info,
            );
            let glyph_bounds = glyph.bounds();
            let offset_bounds =
                glyph_bounds.translate(Vector::new(*current_x + info.glyph_offset.x, info.glyph_offset.y));

            let bounds_block = text_block(Some(&offset_bounds.to_string()));
            ToolTip::set_tip(&bounds_block, Some(Rc::new(format!("Origin bounds: {glyph_bounds}")) as BoxedValue));
            row.children().add(bounds_block);

            border.set_child(row);
            border.set_tag(Some(Rc::new(offset_bounds) as BoxedValue));
            list.items().add(Some(Control::boxed(border)));

            *current_x += glyph_infos[i].glyph_advance;
        }
    }

    fn create_glyph_drawing(&self, glyph_typeface: &Rc<GlyphTypeface>, em_size: f64, info: GlyphInfo) -> Rc<dyn IImage> {
        let drawing = GeometryDrawing::new();
        drawing.set_brush(Some(Brushes::black() as Rc<dyn IBrush>));
        drawing.set_geometry(self.get_glyph_outline(glyph_typeface, em_size, info));
        let image = DrawingImage::new();
        image.set_drawing(drawing);
        image.into()
    }

    fn get_glyph_outline(&self, typeface: &Rc<GlyphTypeface>, em_size: f64, info: GlyphInfo) -> Ref<Geometry> {
        // substitute for GlyphTypeface.GetGlyphOutline
        GlyphRun::new(typeface.clone(), em_size, ReadOnlyMemory::from_slice(&[0u16]), GlyphInfoList::from(vec![info]), None, 0)
            .build_geometry()
    }

    fn on_pointer_moved(&self, sender: &Option<BoxedValue>, e: &PointerEventArgs) {
        let line_control = sender
            .as_ref()
            .and_then(|sender| ValueTypes::as_object(&**sender))
            .and_then(|sender| sender.cast::<InteractiveLineControl>())
            .expect("the sender of the pointer moved event handled by the window is the line control");
        let text_layout = line_control.text_layout();
        let line_bounds = line_control.line_render_bounds();

        let pointer_point = e.get_current_point(Some(&line_control));
        let point =
            Point::new(pointer_point.position.x - line_bounds.left(), pointer_point.position.y - line_bounds.top());
        self.coordinates().set_text(Some(&format!("{:.4}, {:.4}", pointer_point.position.x, pointer_point.position.y)));

        let text_hit = text_layout.hit_test_point(point);
        let hit = self.hit();
        hit.set_text(Some(&format!(
            "{} ({}+{})",
            text_hit.text_position(),
            text_hit.character_hit().first_character_index(),
            text_hit.character_hit().trailing_length()
        )));
        if text_hit.is_trailing() {
            hit.set_text(Some(&format!("{} T", hit.text().unwrap_or_default())));
        }

        if text_hit.is_inside() {
            self.hits().set_selected_index(text_hit.text_position() + 1); // header
        } else {
            self.hits().set_selected_index(-1);
        }
    }

    fn on_hit_test_method_changed(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.hits().set_selection_mode(if self.hit_range_toggle().is_checked() == Some(true) {
            SelectionMode::MULTIPLE
        } else {
            SelectionMode::SINGLE
        });
    }

    fn on_hits_selection_changed(&self, _sender: &Option<BoxedValue>, _e: &SelectionChangedEventArgs) {
        let Some(selection_adorner) = self.selection_adorner.borrow().clone() else {
            return;
        };

        let mut rectangles: Vec<Rect> = Vec::new();
        let text_layout = self.rendering().text_layout();
        let hits = self.hits();

        if self.hit_range_toggle().is_checked() == Some(true) {
            // collect continuous selected indices
            let mut selections: Vec<(i32, i32)> = Vec::with_capacity(1);

            let mut indices: Vec<i32> = hits.selection().selected_indexes().iter().collect();
            indices.sort();

            let mut current_index = -1;
            let mut current_length = 0;
            for i in 0..indices.len() {
                if let Some(index) = tag_of::<i32>(&hits.items().view().get_at(indices[i] as usize)) {
                    if index == current_index + current_length {
                        current_length += 1;
                    } else {
                        if current_length > 0 {
                            selections.push((current_index, current_length));
                        }

                        current_index = index;
                        current_length = 1;
                    }
                }
            }

            if current_length > 0 {
                selections.push((current_index, current_length));
            }

            for (start, length) in selections {
                let selection_rectangles = text_layout.hit_test_text_range(start, length);
                rectangles.extend(selection_rectangles);
            }
        } else if let Some(index) = tag_of::<i32>(&hits.selected_item()) {
            let rect = text_layout.hit_test_text_position(index);
            rectangles.push(rect);
        }

        selection_adorner.set_rectangles(Some(rectangles));
    }

    fn on_buffer_selection_changed(&self, _sender: &Option<BoxedValue>, _e: &SelectionChangedEventArgs) {
        let Some(selection_adorner) = self.selection_adorner.borrow().clone() else {
            return;
        };

        let buffer = self.buffer();
        let mut rectangles: Vec<Rect> = Vec::with_capacity(buffer.selection().count());

        if let Some(selected_items) = buffer.selected_items() {
            for row in selected_items.0.to_vec() {
                if let Some(rect) = tag_of::<Rect>(&row) {
                    rectangles.push(rect);
                }
            }
        }

        selection_adorner.set_rectangles(Some(rectangles));
    }
}

/// `new TextBlock { Text = text }`.
fn text_block(text: Option<&str>) -> Ref<TextBlock> {
    let text_block = TextBlock::new();
    text_block.set_text(text);
    text_block
}

/// The UTF-16 code units of a text as four hexadecimal digits each, separated by spaces.
fn to_hex(s: &[u16]) -> String {
    if s.is_empty() {
        return String::new();
    }

    s.iter().map(|c| format!("{c:04X}")).collect::<Vec<_>>().join(" ")
}

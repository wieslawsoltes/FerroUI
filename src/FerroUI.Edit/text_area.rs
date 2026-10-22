use crate::TextEditor;
use crate::rendering::{DisplayLine, HeightTree};
use crate::text_input_method_client::EditorTextInputMethodClient;
use ferroui_base::input::text_input::TextInputMethodClient;
use ferroui_base::input::{
    FocusChangedEventArgs, ICommand, IScrollable, InputElement, InputElementImpl,
    InputElementImplExt, Key, KeyEventArgs, KeyModifiers, NavigationDirection, PointerEventArgs,
    PointerPressedEventArgs, PointerReleasedEventArgs, TextInputEventArgs,
};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::text_formatting::{
    GenericTextRunProperties, TextLayout, TextLayoutOptions, TextRunProperties,
};
use ferroui_base::media::{
    BaselineAlignment, Color, DrawingContext, FontStretch, FontStyle, FontWeight, IBrush,
    TextDecorationCollection, TextWrapping, Typeface,
};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::threading::{Dispatcher, DispatcherPriority, DispatcherTimer};
use ferroui_base::utilities::{HandlerList, ReadOnlyMemory, ValueSpan};
use ferroui_base::{
    FerroObjectImpl, FerroObjectImplExt, Point, Rect, Ref, Size, StyledElementImpl, Vector, Visual,
    VisualImpl, WeakRef, ferro_class, ferro_impl_classes, instantiate,
};
use ferroui_controls::primitives::{ILogicalScrollable, register_logical_scrollable};
use ferroui_controls::{Application, Control, ControlImpl};
use ferroui_edit_core::editing::{CaretMovement, Selection, SelectionSegment};
use ferroui_edit_core::folding::FoldingSection;
use std::cell::{Cell, OnceCell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;
use std::time::Duration;

struct VisualLine {
    offset: usize,
    length: usize,
    display: DisplayLine,
    layout: TextLayout,
    preedit: Option<(usize, usize)>,
}

/// The editor's focus, input and rendering surface. A `ScrollViewer` discovers
/// its logical scrolling interface, so only visible logical lines are shaped.
#[repr(C)]
pub struct TextArea {
    base: Control,
    editor: RefCell<Option<WeakRef<TextEditor>>>,
    viewport: Cell<Size>,
    offset: Cell<Vector>,
    extent: Cell<Size>,
    horizontal: Cell<bool>,
    vertical: Cell<bool>,
    cache: RefCell<BTreeMap<usize, Rc<VisualLine>>>,
    heights: RefCell<HeightTree>,
    scroll_invalidated: Rc<HandlerList<dyn Fn()>>,
    rendering: Cell<bool>,
    scroll_notification_pending: Cell<bool>,
    dragging: Cell<bool>,
    drag_anchor: Cell<usize>,
    drag_anchor_virtual_space: Cell<usize>,
    rectangle_drag_anchor: Cell<Option<(usize, f64)>>,
    rectangle_keyboard_anchor: Cell<Option<(usize, f64)>>,
    space_advance: Cell<f64>,
    default_line_height: Cell<f64>,
    caret_visible: Cell<bool>,
    caret_at_end_of_line: Cell<bool>,
    blink_timer: OnceCell<Rc<DispatcherTimer>>,
    im_client: OnceCell<Rc<EditorTextInputMethodClient>>,
    preedit: RefCell<Option<(String, Option<i32>)>>,
    highlight_pending: Cell<bool>,
    folds: RefCell<Vec<FoldingSection>>,
    desired_x: Cell<Option<f64>>,
    moving_vertically: Cell<bool>,
}

ferro_class!(TextArea: Control);
ferroui_base::ferro_class_info!(TextArea { new: TextArea::new });
ferro_impl_classes!(TextArea: StyledElementImpl, InteractiveImpl, ControlImpl);

impl FerroObjectImpl for TextArea {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.im_client
            .set(EditorTextInputMethodClient::new(this.to_ref().downgrade()))
            .ok();
        let timer = DispatcherTimer::new();
        timer.set_interval(Duration::from_millis(530));
        let weak = this.to_ref().downgrade();
        timer.tick(move |_| {
            if let Some(area) = weak.upgrade() {
                area.caret_visible.set(!area.caret_visible.get());
                area.invalidate_visual();
            }
        });
        this.blink_timer.set(timer).ok();
    }
}

impl LayoutableImpl for TextArea {
    fn measure_override(_this: &Self, available: Size) -> Size {
        Size::new(available.width.min(640.0), available.height.min(400.0))
    }

    fn arrange_override(this: &Self, size: Size) -> Size {
        let old = this.viewport.replace(size);
        if old.width != size.width {
            this.reset_line_metrics();
        }
        this.update_extent();
        if old != size {
            this.raise_scroll_invalidated();
        }
        size
    }
}

impl VisualImpl for TextArea {
    fn render(this: &Self, context: &mut DrawingContext) {
        let Some(editor) = this.editor() else {
            return;
        };
        let size = this.viewport.get();
        if size.width <= 0.0 || size.height <= 0.0 {
            return;
        }
        struct RenderScope<'a>(&'a Cell<bool>, bool);
        impl Drop for RenderScope<'_> {
            fn drop(&mut self) {
                self.0.set(self.1);
            }
        }
        let _render_scope = RenderScope(&this.rendering, this.rendering.replace(true));
        let clip = context.push_clip(Rect::new(0.0, 0.0, size.width, size.height));
        if let Some(background) = editor.background() {
            context.fill_rectangle(
                &background,
                Rect::new(0.0, 0.0, size.width, size.height),
                0.0,
            );
        }
        let offset = this.offset.get();
        let first = this.heights.borrow().line_at(offset.y);
        let line_count = editor.document().line_count();
        let gutter = this.gutter_width();
        let selection = editor.selection();
        let options = editor.options();
        let marker_layout = |text: &str| {
            TextLayout::new(
                text,
                Typeface::new(editor.font_family()),
                TextLayoutOptions {
                    font_size: editor.font_size(),
                    foreground: editor.line_numbers_foreground(),
                    ..Default::default()
                },
            )
        };
        let space_marker = options.show_spaces.then(|| marker_layout("·"));
        let tab_marker = options.show_tabs.then(|| marker_layout("→"));
        let eol_marker = options.show_end_of_line.then(|| marker_layout("¶"));
        let caret = editor.caret_offset();
        let folding_sections = editor
            .folding_manager()
            .map(|m| m.all_foldings())
            .unwrap_or_default();
        let current_line = editor
            .document()
            .get_line_by_offset(caret)
            .map(|l| l.line_number() - 1)
            .unwrap_or(0);
        let mut last = first;
        for index in std::iter::successors(Some(first), |&index| {
            let next = this.heights.borrow().next_visible(index + 1);
            (next < line_count).then_some(next)
        }) {
            let top = this.heights.borrow().top(index) - offset.y;
            if top > size.height {
                break;
            }
            let Some(line) = this.line_layout(index) else {
                break;
            };
            this.heights
                .borrow_mut()
                .set_height(index, line.layout.height().max(this.line_height()));
            let origin = Point::new(gutter - offset.x, top);
            let text_clip = context.push_clip(Rect::new(
                gutter,
                0.0,
                (size.width - gutter).max(0.0),
                size.height,
            ));
            if index == current_line && editor.options().highlight_current_line {
                let brush =
                    Rc::new(ImmutableSolidColorBrush::from_uint32(0x14808080)) as Rc<dyn IBrush>;
                context.fill_rectangle(
                    &brush,
                    Rect::new(
                        gutter,
                        top,
                        (size.width - gutter).max(0.0),
                        line.layout.height(),
                    ),
                    0.0,
                );
            }
            if let Some(panel) = editor.installed_search_panel().filter(|p| p.is_open()) {
                let brush =
                    Rc::new(ImmutableSolidColorBrush::from_uint32(0x70e7b83a)) as Rc<dyn IBrush>;
                for matched in panel.results_in_range(line.offset, line.length) {
                    let start = matched.offset.max(line.offset);
                    let end = (matched.offset + matched.length).min(line.offset + line.length);
                    let a = line.display.visual_offset(start - line.offset);
                    let b = line.display.visual_offset(end - line.offset);
                    for rect in line
                        .layout
                        .hit_test_text_range(a as i32, (b - a).max(1) as i32)
                    {
                        context.fill_rectangle(
                            &brush,
                            Rect::new(
                                origin.x + rect.x,
                                origin.y + rect.y,
                                rect.width.max(2.0),
                                rect.height,
                            ),
                            0.0,
                        );
                    }
                }
            }
            if let Some(brush) = editor.selection_brush().filter(|_| line.preedit.is_none()) {
                for segment in selection.segments() {
                    let start = segment.start_offset().max(line.offset);
                    let end = segment.end_offset().min(line.offset + line.length);
                    if end > start {
                        let a = line.display.visual_offset(start - line.offset);
                        let b = line.display.visual_offset(end - line.offset);
                        for rect in line.layout.hit_test_text_range(a as i32, (b - a) as i32) {
                            context.fill_rectangle(
                                &brush,
                                Rect::new(
                                    rect.x + origin.x,
                                    rect.y + origin.y,
                                    rect.width,
                                    rect.height,
                                ),
                                0.0,
                            );
                        }
                    }
                    let line_end = line.offset + line.length;
                    if segment.end_offset() == line_end && segment.end_virtual_space() > 0 {
                        let first = if segment.start_offset() == line_end {
                            segment.start_virtual_space()
                        } else {
                            0
                        };
                        let last = segment.end_virtual_space();
                        if last > first {
                            let edge = Self::virtual_space_bounds(&line.layout);
                            let width = this.space_advance();
                            context.fill_rectangle(
                                &brush,
                                Rect::new(
                                    origin.x + edge.x + first as f64 * width,
                                    origin.y + edge.y,
                                    (last - first) as f64 * width,
                                    edge.height.max(this.line_height()),
                                ),
                                0.0,
                            );
                        }
                    }
                }
            }
            line.layout.draw(context, origin);
            if space_marker.is_some() || tab_marker.is_some() {
                for (position, tab) in line.display.whitespace_markers() {
                    if let Some(marker) = if tab {
                        tab_marker.as_ref()
                    } else {
                        space_marker.as_ref()
                    } {
                        let rect = line.layout.hit_test_text_position(position as i32);
                        marker.draw(context, Point::new(origin.x + rect.x, origin.y + rect.y));
                    }
                }
            }
            if let Some(marker) = &eol_marker {
                if line.offset + line.length < editor.document().text_length() {
                    let rect = line
                        .layout
                        .hit_test_text_position(line.display.text.len() as i32);
                    marker.draw(context, Point::new(origin.x + rect.x, origin.y + rect.y));
                }
            }
            if this.is_focused() && this.caret_visible.get() {
                for segment in selection.segments().iter().filter(|s| s.is_empty()) {
                    if segment.caret >= line.offset && segment.caret <= line.offset + line.length {
                        let position = if let Some((start, length)) = line.preedit {
                            start
                                + this
                                    .preedit
                                    .borrow()
                                    .as_ref()
                                    .and_then(|p| p.1)
                                    .map_or(length, |c| (c.max(0) as usize).min(length))
                        } else {
                            line.display.visual_offset(segment.caret - line.offset)
                        };
                        let r = if line.preedit.is_none() && segment.caret_virtual_space > 0 {
                            Self::virtual_space_bounds(&line.layout)
                        } else {
                            Self::caret_bounds(
                                &line.layout,
                                position,
                                line.preedit.is_none()
                                    && *segment == selection.primary()
                                    && this.caret_at_end_of_line.get(),
                            )
                        };
                        let brush = editor.foreground().unwrap_or_else(|| {
                            Rc::new(ImmutableSolidColorBrush::from_uint32(0xff202020))
                        });
                        context.fill_rectangle(
                            &brush,
                            Rect::new(
                                origin.x
                                    + r.x
                                    + if line.preedit.is_none() {
                                        segment.caret_virtual_space as f64 * this.space_advance()
                                    } else {
                                        0.0
                                    },
                                origin.y + r.y,
                                1.0,
                                r.height.max(this.line_height()),
                            ),
                            0.0,
                        );
                    }
                }
            }
            context.pop(text_clip);
            if editor.show_line_numbers() {
                let number = TextLayout::new(
                    &(index + 1).to_string(),
                    Typeface::new(editor.font_family()),
                    TextLayoutOptions {
                        font_size: editor.font_size(),
                        foreground: editor.line_numbers_foreground(),
                        ..Default::default()
                    },
                );
                number.draw(
                    context,
                    Point::new(
                        gutter - number.width() - this.folding_gutter_width() - 8.0,
                        top,
                    ),
                );
            }
            if !folding_sections.is_empty() {
                let first = folding_sections.partition_point(|s| s.start_offset() < line.offset);
                if let Some(section) = folding_sections.get(first).filter(|s| {
                    editor
                        .document()
                        .get_line_by_offset(s.start_offset())
                        .is_ok_and(|l| l.line_number() == index + 1)
                }) {
                    let marker = TextLayout::new(
                        if section.is_folded() { "+" } else { "−" },
                        Typeface::new(editor.font_family()),
                        TextLayoutOptions {
                            font_size: editor.font_size(),
                            foreground: editor.line_numbers_foreground(),
                            ..Default::default()
                        },
                    );
                    marker.draw(context, Point::new(gutter - 16.0, top));
                }
            }
            last = index;
        }
        this.cache
            .borrow_mut()
            .retain(|index, _| *index >= first.saturating_sub(8) && *index <= last + 8);
        this.update_extent();
        context.pop(clip);
    }
}

impl InputElementImpl for TextArea {
    fn as_scrollable(this: &Self) -> Option<&dyn IScrollable> {
        Some(this)
    }

    fn on_got_focus(this: &Self, e: &FocusChangedEventArgs) {
        Self::parent_on_got_focus(this, e);
        this.caret_visible.set(true);
        if let Some(timer) = this.blink_timer.get() {
            timer.start();
        }
        this.invalidate_visual();
    }
    fn on_lost_focus(this: &Self, e: &FocusChangedEventArgs) {
        Self::parent_on_lost_focus(this, e);
        if let Some(timer) = this.blink_timer.get() {
            timer.stop();
        }
        this.set_preedit(None, None);
        this.invalidate_visual();
    }
    fn on_text_input(this: &Self, e: &TextInputEventArgs) {
        if e.handled() {
            return;
        }
        if let (Some(editor), Some(text)) = (this.editor(), e.text.as_deref()) {
            this.set_preedit(None, None);
            if let Err(error) = editor.replace_selection_with_text(text) {
                editor.report_error(error.to_string());
            }
            e.set_handled(true);
        }
    }
    fn on_key_down(this: &Self, e: &KeyEventArgs) {
        let Some(editor) = this.editor() else {
            return;
        };
        if e.handled() {
            return;
        }
        if this.preedit.borrow().is_some() {
            return;
        }
        let keymap = Application::current()
            .and_then(|app| app.platform_settings())
            .map(|settings| settings.hotkey_configuration())
            .unwrap_or_default();
        let modifiers = e.key_modifiers;
        let command = modifiers.contains(keymap.command_modifiers);
        let word = modifiers.contains(keymap.whole_word_text_action_modifiers);
        let shift = modifiers.contains(keymap.selection_modifiers);
        let matches = |gestures: &[ferroui_base::input::KeyGesture]| {
            gestures.iter().any(|g| g.matches(Some(e)))
        };
        use crate::EditorCommandKind;
        for (gestures, kind) in [
            (&keymap.copy, EditorCommandKind::Copy),
            (&keymap.cut, EditorCommandKind::Cut),
            (&keymap.paste, EditorCommandKind::Paste),
            (&keymap.select_all, EditorCommandKind::SelectAll),
            (&keymap.undo, EditorCommandKind::Undo),
            (&keymap.redo, EditorCommandKind::Redo),
        ] {
            if matches(gestures) {
                editor.command(kind).execute(None);
                editor.selection_changed();
                e.set_handled(true);
                return;
            }
        }
        let session = editor.session();
        if modifiers.contains(KeyModifiers::ALT | KeyModifiers::SHIFT)
            && matches!(
                e.key,
                Key::Left | Key::Right | Key::Up | Key::Down | Key::Home | Key::End
            )
        {
            let selection = session.selection();
            let primary = selection.primary();
            if let Ok(logical) = editor.document().get_line_by_offset(primary.caret) {
                let index = this
                    .heights
                    .borrow()
                    .visible_owner(logical.line_number() - 1);
                let x = this.x_for_position(primary.caret, primary.caret_virtual_space);
                let anchor = this
                    .rectangle_keyboard_anchor
                    .get()
                    .filter(|_| selection.is_rectangular())
                    .unwrap_or_else(|| this.rectangle_anchor_for_selection(&selection));
                let mut target = (index, x);
                match e.key {
                    Key::Up => {
                        target.0 = this.heights.borrow().visible_owner(index.saturating_sub(1))
                    }
                    Key::Down => {
                        target.0 = this
                            .heights
                            .borrow()
                            .next_visible(index + 1)
                            .min(editor.document().line_count() - 1)
                    }
                    Key::Home => target.1 = 0.0,
                    Key::End => target.1 = this.x_for_position(logical.end_offset(), 0),
                    Key::Left | Key::Right => {
                        let right = e.key == Key::Right;
                        let (offset, extra) = if primary.caret_virtual_space > 0 {
                            (
                                primary.caret,
                                if right {
                                    primary.caret_virtual_space.saturating_add(1)
                                } else if word {
                                    0
                                } else {
                                    primary.caret_virtual_space - 1
                                },
                            )
                        } else if right && primary.caret == logical.end_offset() {
                            (primary.caret, 1)
                        } else {
                            let text = editor
                                .document()
                                .get_text(logical.offset(), logical.length())
                                .unwrap_or_default();
                            let movement = match (right, word) {
                                (false, false) => CaretMovement::CharacterLeft,
                                (true, false) => CaretMovement::CharacterRight,
                                (false, true) => CaretMovement::WordLeft,
                                (true, true) => CaretMovement::WordRight,
                            };
                            (
                                logical.offset()
                                    + ferroui_edit_core::editing::next_caret_position(
                                        &text,
                                        primary.caret - logical.offset(),
                                        movement,
                                    ),
                                0,
                            )
                        };
                        target.1 = this.x_for_position(offset, extra);
                    }
                    _ => {}
                }
                if let Err(error) = this.select_rectangle_to(anchor, target) {
                    editor.report_error(error.to_string());
                }
                this.rectangle_keyboard_anchor.set(Some(anchor));
                e.set_handled(true);
                return;
            }
        }
        this.rectangle_keyboard_anchor.set(None);
        if matches!(e.key, Key::Up | Key::Down | Key::PageUp | Key::PageDown)
            && !command
            && this.viewport.get().width > 0.0
        {
            let rect = this.cursor_rectangle();
            let scroll = this.offset.get();
            let x = this.desired_x.get().unwrap_or(rect.x + scroll.x);
            this.desired_x.set(Some(x));
            let current = session.selection().primary();
            let upward = matches!(e.key, Key::Up | Key::PageUp);
            let target_y = if matches!(e.key, Key::PageUp | Key::PageDown) {
                let distance = this.viewport.get().height.max(this.line_height());
                rect.y + rect.height * 0.5 + if upward { -distance } else { distance }
            } else {
                this.adjacent_row_y(upward)
                    .unwrap_or(rect.y + rect.height * 0.5)
            };
            let target_point = Point::new(x - scroll.x, target_y);
            let target_affinity = this
                .position_at_point(target_point, false)
                .is_some_and(|p| p.is_at_end_of_line);
            let (target, extra) = this.selection_position_at_point(
                target_point,
                editor.options().enable_virtual_space
                    || current.caret_virtual_space > 0
                    || session.selection().is_rectangular(),
            );
            let segment = if shift {
                SelectionSegment::with_virtual_space(
                    current.anchor,
                    current.anchor_virtual_space,
                    target,
                    extra,
                )
            } else {
                SelectionSegment::with_virtual_space(target, extra, target, extra)
            };
            if let Err(error) = session.set_selection(Selection::from_segments([segment], 0)) {
                editor.report_error(error.to_string());
            }
            this.moving_vertically.set(true);
            editor.selection_changed_with_affinity(target_affinity);
            this.moving_vertically.set(false);
            e.set_handled(true);
            return;
        }
        let edge = [
            (
                &keymap.move_cursor_to_the_start_of_line,
                CaretMovement::LineStart,
                false,
            ),
            (
                &keymap.move_cursor_to_the_end_of_line,
                CaretMovement::LineEnd,
                false,
            ),
            (
                &keymap.move_cursor_to_the_start_of_document,
                CaretMovement::DocumentStart,
                false,
            ),
            (
                &keymap.move_cursor_to_the_end_of_document,
                CaretMovement::DocumentEnd,
                false,
            ),
            (
                &keymap.move_cursor_to_the_start_of_line_with_selection,
                CaretMovement::LineStart,
                true,
            ),
            (
                &keymap.move_cursor_to_the_end_of_line_with_selection,
                CaretMovement::LineEnd,
                true,
            ),
            (
                &keymap.move_cursor_to_the_start_of_document_with_selection,
                CaretMovement::DocumentStart,
                true,
            ),
            (
                &keymap.move_cursor_to_the_end_of_document_with_selection,
                CaretMovement::DocumentEnd,
                true,
            ),
        ]
        .into_iter()
        .find(|(gestures, _, _)| matches(gestures));
        let extend = edge.as_ref().map_or(shift, |(_, _, extend)| *extend);
        let movement = edge
            .map(|(_, movement, _)| movement)
            .or_else(|| match e.key {
                Key::Left => Some(if word {
                    CaretMovement::WordLeft
                } else {
                    CaretMovement::CharacterLeft
                }),
                Key::Right => Some(if word {
                    CaretMovement::WordRight
                } else {
                    CaretMovement::CharacterRight
                }),
                Key::Up => Some(CaretMovement::LineUp),
                Key::Down => Some(CaretMovement::LineDown),
                _ => None,
            });
        let target_affinity = Cell::new(false);
        let result = if let Some(movement) = movement {
            if editor.word_wrap()
                && matches!(movement, CaretMovement::LineStart | CaretMovement::LineEnd)
                && this.viewport.get().width > 0.0
            {
                let rect = this.cursor_rectangle();
                let x = if movement == CaretMovement::LineStart {
                    this.gutter_width()
                } else {
                    f64::MAX
                };
                let point = Point::new(x, rect.y + rect.height / 2.0);
                let offset = this.offset_at_point(point);
                target_affinity.set(
                    this.position_at_point(point, false)
                        .is_some_and(|p| p.is_at_end_of_line),
                );
                let primary = session.selection().primary();
                session.set_selection(Selection::from_segments(
                    [SelectionSegment::with_virtual_space(
                        if extend { primary.anchor } else { offset },
                        if extend {
                            primary.anchor_virtual_space
                        } else {
                            0
                        },
                        offset,
                        0,
                    )],
                    0,
                ))
            } else {
                session.move_caret(movement, extend);
                Ok(())
            }
        } else {
            match e.key {
                Key::F if command => {
                    editor.search_panel().show(false);
                    Ok(())
                }
                Key::H if command => {
                    editor.search_panel().show(true);
                    Ok(())
                }
                Key::F3 => {
                    if shift {
                        editor.search_panel().find_previous();
                    } else {
                        editor.search_panel().find_next();
                    }
                    Ok(())
                }
                Key::Back => session.delete(true, word),
                Key::Delete => session.delete(false, word),
                Key::Return => session.insert_new_line(),
                Key::Tab if shift => session.indent_selection(true),
                Key::Tab => session.insert_tab(),
                Key::PageUp | Key::PageDown => {
                    let direction = if e.key == Key::PageUp { -1.0 } else { 1.0 };
                    let rect = this.cursor_rectangle();
                    let offset = this.offset_at_point(Point::new(
                        rect.x,
                        rect.y + direction * this.viewport.get().height,
                    ));
                    session.set_selection(if shift {
                        Selection::simple(session.selection().primary().anchor, offset)
                    } else {
                        Selection::empty(offset)
                    })
                }
                Key::Escape => session.set_caret_offset(session.caret_offset()),
                _ => {
                    Self::parent_on_key_down(this, e);
                    return;
                }
            }
        };
        if let Err(error) = result {
            editor.report_error(error.to_string());
        }
        editor.selection_changed_with_affinity(target_affinity.get());
        e.set_handled(true);
    }
    fn on_pointer_pressed(this: &Self, e: &PointerPressedEventArgs) {
        if e.handled() {
            return;
        }
        let point = e.get_current_point(Some(this));
        if !point.properties.is_left_button_pressed {
            return;
        }
        let Some(editor) = this.editor() else {
            return;
        };
        this.focus();
        this.rectangle_keyboard_anchor.set(None);
        let (offset, extra) = this.selection_position_at_point(
            point.position,
            editor.options().enable_virtual_space || e.key_modifiers().contains(KeyModifiers::ALT),
        );
        let point_affinity = this
            .position_at_point(point.position, false)
            .is_some_and(|p| p.is_at_end_of_line);
        if point.position.x >= this.gutter_width() - this.folding_gutter_width()
            && point.position.x < this.gutter_width()
        {
            if let Some(manager) = editor.folding_manager() {
                let line_number = editor
                    .document()
                    .get_line_by_offset(offset)
                    .map_or(1, |l| l.line_number());
                if let Some(section) = manager.all_foldings().into_iter().find(|s| {
                    editor
                        .document()
                        .get_line_by_offset(s.start_offset())
                        .is_ok_and(|l| l.line_number() == line_number)
                }) {
                    section.set_is_folded(!section.is_folded());
                    e.set_handled(true);
                    return;
                }
            }
        }
        let session = editor.session();
        this.rectangle_drag_anchor.set(
            e.key_modifiers()
                .contains(KeyModifiers::ALT)
                .then(|| this.rectangle_anchor_at_point(point.position)),
        );
        let selection = if e.key_modifiers().contains(KeyModifiers::SHIFT) {
            let current = session.selection().primary();
            Selection::from_segments(
                [SelectionSegment::with_virtual_space(
                    current.anchor,
                    current.anchor_virtual_space,
                    offset,
                    extra,
                )],
                0,
            )
        } else {
            Selection::from_segments(
                [SelectionSegment::with_virtual_space(
                    offset, extra, offset, extra,
                )],
                0,
            )
        };
        let _ = session.set_selection(selection);
        if e.click_count() >= 3 || point.position.x < this.gutter_width() {
            if let Ok(line) = editor.document().get_line_by_offset(offset) {
                let _ = session.select(line.offset(), line.total_length());
            }
        } else if e.click_count() == 2 {
            let _ = session.move_caret(CaretMovement::WordLeft, false);
            let _ = session.move_caret(CaretMovement::WordRight, true);
        }
        this.drag_anchor.set(session.selection().primary().anchor);
        this.drag_anchor_virtual_space
            .set(session.selection().primary().anchor_virtual_space);
        this.dragging.set(true);
        e.pointer().capture(Some(&this.to_ref().upcast()));
        editor.selection_changed_with_affinity(
            e.click_count() == 1 && point.position.x >= this.gutter_width() && point_affinity,
        );
        e.set_handled(true);
    }
    fn on_pointer_moved(this: &Self, e: &PointerEventArgs) {
        if !this.dragging.get() {
            return;
        }
        if let Some(editor) = this.editor() {
            let point = e.get_position(Some(this));
            if let Some(anchor) = this.rectangle_drag_anchor.get() {
                if let Err(error) =
                    this.select_rectangle_to(anchor, this.rectangle_anchor_at_point(point))
                {
                    editor.report_error(error.to_string());
                }
            } else {
                let affinity = this
                    .position_at_point(point, false)
                    .is_some_and(|p| p.is_at_end_of_line);
                let (offset, extra) = this.selection_position_at_point(
                    point,
                    editor.options().enable_virtual_space
                        || this.drag_anchor_virtual_space.get() > 0,
                );
                let result = editor.session().set_selection(Selection::from_segments(
                    [SelectionSegment::with_virtual_space(
                        this.drag_anchor.get(),
                        this.drag_anchor_virtual_space.get(),
                        offset,
                        extra,
                    )],
                    0,
                ));
                match result {
                    Ok(()) => editor.selection_changed_with_affinity(affinity),
                    Err(error) => editor.report_error(error.to_string()),
                }
            }
            e.set_handled(true);
        }
    }
    fn on_pointer_released(this: &Self, e: &PointerReleasedEventArgs) {
        if this.dragging.replace(false) {
            this.rectangle_drag_anchor.set(None);
            e.pointer().capture(None);
            e.set_handled(true);
        }
    }
}

impl TextArea {
    fn static_constructor() {
        InputElement::focusable_property().override_default_value::<TextArea>(true);
        Visual::clip_to_bounds_property().override_default_value::<TextArea>(true);
        register_logical_scrollable::<TextArea>();
        InputElement::text_input_method_client_requested_event().add_class_handler::<TextArea>(
            |area, e| {
                if area.editor().is_some_and(|editor| !editor.is_read_only()) {
                    e.set_client(
                        area.im_client
                            .get()
                            .map(|client| client.clone() as Rc<dyn TextInputMethodClient>),
                    );
                }
            },
        );
    }
    pub fn construct() -> Self {
        Self {
            base: Control::construct(),
            editor: RefCell::new(None),
            viewport: Cell::new(Size::default()),
            offset: Cell::new(Vector::default()),
            extent: Cell::new(Size::default()),
            horizontal: Cell::new(true),
            vertical: Cell::new(true),
            cache: RefCell::new(BTreeMap::new()),
            heights: RefCell::new(HeightTree::new(1, 20.0)),
            scroll_invalidated: Rc::new(HandlerList::new()),
            rendering: Cell::new(false),
            scroll_notification_pending: Cell::new(false),
            dragging: Cell::new(false),
            drag_anchor: Cell::new(0),
            drag_anchor_virtual_space: Cell::new(0),
            rectangle_drag_anchor: Cell::new(None),
            rectangle_keyboard_anchor: Cell::new(None),
            space_advance: Cell::new(0.0),
            default_line_height: Cell::new(0.0),
            caret_visible: Cell::new(true),
            caret_at_end_of_line: Cell::new(false),
            blink_timer: OnceCell::new(),
            im_client: OnceCell::new(),
            preedit: RefCell::new(None),
            highlight_pending: Cell::new(false),
            folds: RefCell::new(Vec::new()),
            desired_x: Cell::new(None),
            moving_vertically: Cell::new(false),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub(crate) fn set_editor(&self, editor: WeakRef<TextEditor>) {
        *self.editor.borrow_mut() = Some(editor);
    }
    pub fn editor(&self) -> Option<Ref<TextEditor>> {
        self.editor.borrow().as_ref().and_then(WeakRef::upgrade)
    }
    pub fn cached_visual_line_count(&self) -> usize {
        self.cache.borrow().len()
    }
    pub fn line_height(&self) -> f64 {
        if self.default_line_height.get() > 0.0 {
            return self.default_line_height.get();
        }
        let Some(editor) = self.editor() else {
            return 20.0;
        };
        let layout = TextLayout::new(
            "x",
            Typeface::new(editor.font_family()),
            TextLayoutOptions {
                font_size: editor.font_size(),
                ..Default::default()
            },
        );
        let height = layout.height().max(1.0);
        self.default_line_height.set(height);
        height
    }
    fn gutter_width(&self) -> f64 {
        self.editor().map_or(6.0, |editor| {
            (if editor.show_line_numbers() {
                editor.document().line_count().to_string().len() as f64 * editor.font_size() * 0.65
                    + 18.0
            } else {
                6.0
            }) + self.folding_gutter_width()
        })
    }
    fn folding_gutter_width(&self) -> f64 {
        if self.editor().is_some_and(|e| e.folding_manager().is_some()) {
            18.0
        } else {
            0.0
        }
    }
    pub(crate) fn invalidate_layout_cache(&self) {
        self.reset_line_metrics();
        self.extent.set(Size::default());
        self.invalidate_measure();
        self.invalidate_visual();
        self.update_extent();
    }
    fn reset_line_metrics(&self) {
        self.cache.borrow_mut().clear();
        self.space_advance.set(0.0);
        self.default_line_height.set(0.0);
        let mut active = Vec::<FoldingSection>::new();
        let mut collapsed = Vec::new();
        if let Some(editor) = self.editor() {
            let document = editor.document();
            let line_height = self.line_height();
            self.heights
                .borrow_mut()
                .reset(document.line_count(), line_height);
            if let Some(manager) = editor.folding_manager() {
                let mut sections = manager.all_foldings();
                sections.sort_by_key(|s| (s.start_offset(), std::cmp::Reverse(s.end_offset())));
                for section in sections.into_iter().filter(|s| s.is_folded()) {
                    if active
                        .last()
                        .is_some_and(|previous| section.start_offset() < previous.end_offset())
                    {
                        continue;
                    }
                    let start = document.get_line_by_offset(section.start_offset());
                    let end = document.get_line_by_offset(section.end_offset());
                    if let (Ok(start), Ok(end)) = (start, end) {
                        collapsed.push((start.line_number(), end.line_number()));
                    }
                    active.push(section);
                }
            }
        }
        self.heights.borrow_mut().set_collapsed(&collapsed);
        *self.folds.borrow_mut() = active;
    }
    pub(crate) fn document_changed(&self) {
        self.invalidate_layout_cache();
        if let Some(client) = self.im_client.get() {
            client.raise_surrounding_text_changed();
            client.raise_selection_changed();
        }
    }
    pub(crate) fn selection_changed_with_affinity(&self, at_end_of_line: bool) {
        self.caret_at_end_of_line.set(at_end_of_line);
        self.rectangle_keyboard_anchor.set(None);
        if !self.moving_vertically.get() {
            self.desired_x.set(None);
        }
        self.caret_visible.set(true);
        self.bring_caret_into_view();
        if let Some(client) = self.im_client.get() {
            client.raise_selection_changed();
            client.raise_cursor_rectangle_changed();
        }
        self.invalidate_visual();
    }
    fn update_extent(&self) {
        let viewport = self.viewport.get();
        let below = self
            .editor()
            .is_some_and(|e| e.options().allow_scroll_below_document);
        let height = self.heights.borrow().total()
            + if below {
                (viewport.height - self.line_height()).max(0.0)
            } else {
                0.0
            };
        let width = if self.editor().is_some_and(|e| e.word_wrap()) {
            viewport.width
        } else {
            self.cache
                .borrow()
                .values()
                .map(|l| l.layout.width_including_trailing_whitespace() + self.gutter_width() + 8.0)
                .fold(self.extent.get().width, f64::max)
                .max(viewport.width)
        };
        let virtual_width = self.editor().map_or(0.0, |editor| {
            let primary = editor.selection().primary();
            if primary.caret_virtual_space == 0 {
                return 0.0;
            }
            let Ok(document_line) = editor.document().get_line_by_offset(primary.caret) else {
                return 0.0;
            };
            let index = self
                .heights
                .borrow()
                .visible_owner(document_line.line_number() - 1);
            let line = self.cache.borrow().get(&index).cloned();
            line.map_or(0.0, |line| {
                let edge = Self::virtual_space_bounds(&line.layout);
                edge.x
                    + primary.caret_virtual_space as f64 * self.space_advance()
                    + self.gutter_width()
                    + 8.0
            })
        });
        let width = width.max(virtual_width);
        let extent = Size::new(width, height.max(viewport.height));
        if self.extent.replace(extent) != extent {
            self.raise_scroll_invalidated();
        }
    }
    fn line_layout(&self, index: usize) -> Option<Rc<VisualLine>> {
        if let Some(line) = self.cache.borrow().get(&index).cloned() {
            return Some(line);
        }
        let editor = self.editor()?;
        let document = editor.document();
        let line = document.get_line_by_number(index + 1).ok()?;
        let mut display = DisplayLine::empty();
        let mut cursor = line.offset();
        let mut end = line.end_offset();
        let mut source_lines = vec![line.line_number()];
        let folds = self.folds.borrow();
        let first = folds.partition_point(|s| s.start_offset() < cursor);
        for section in folds.iter().skip(first) {
            if section.start_offset() > end {
                break;
            }
            let text = document
                .get_text_utf16(cursor, section.start_offset() - cursor)
                .ok()?;
            display.push_text(
                &text,
                cursor - line.offset(),
                editor.options().indentation_size,
            );
            let title = section
                .title()
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| "...".into());
            display.push_fold(
                section.start_offset() - line.offset(),
                section.end_offset() - line.offset(),
                &title,
            );
            cursor = section.end_offset();
            let last_line = document.get_line_by_offset(cursor).ok()?;
            end = last_line.end_offset();
            if source_lines.last() != Some(&last_line.line_number()) {
                source_lines.push(last_line.line_number());
            }
        }
        drop(folds);
        display.push_text(
            &document.get_text_utf16(cursor, end - cursor).ok()?,
            cursor - line.offset(),
            editor.options().indentation_size,
        );
        let preedit = self.preedit.borrow().as_ref().and_then(|(text, _)| {
            let selected = editor.selection().primary();
            if selected.start_offset() >= line.offset() && selected.end_offset() <= end {
                match display.apply_preedit(
                    selected.start_offset() - line.offset(),
                    selected.end_offset() - line.offset(),
                    text,
                    selected.start_virtual_space(),
                ) {
                    Ok(range) => Some(range),
                    Err(error) => {
                        editor.report_error(error.to_string());
                        None
                    }
                }
            } else {
                None
            }
        });
        let mut styles = Vec::new();
        if let Some(installation) = editor.textmate_installation() {
            let target = *source_lines.last().unwrap_or(&(index + 1));
            if !installation.advance_to(&document, target, 32)
                && installation.last_error().is_none()
            {
                self.schedule_highlight(target);
            }
            for source_number in &source_lines {
                let source_offset =
                    document.get_line_by_number(*source_number).ok()?.offset() - line.offset();
                for (start, end, style) in installation.styled_tokens(*source_number) {
                    let foreground = style
                        .foreground
                        .map(|c| {
                            Rc::new(ImmutableSolidColorBrush::new(Color::from_argb(
                                c.a, c.r, c.g, c.b,
                            ))) as Rc<dyn IBrush>
                        })
                        .or_else(|| editor.foreground());
                    let background = style.background.map(|c| {
                        Rc::new(ImmutableSolidColorBrush::new(Color::from_argb(
                            c.a, c.r, c.g, c.b,
                        ))) as Rc<dyn IBrush>
                    });
                    let typeface = Typeface::with_style(
                        editor.font_family(),
                        if style
                            .font_style
                            .contains(ferroui_textmate::FontStyle::ITALIC)
                        {
                            FontStyle::Italic
                        } else {
                            FontStyle::Normal
                        },
                        if style.font_style.contains(ferroui_textmate::FontStyle::BOLD) {
                            FontWeight::Bold
                        } else {
                            FontWeight::Normal
                        },
                        FontStretch::Normal,
                    );
                    let decorations = match (
                        style
                            .font_style
                            .contains(ferroui_textmate::FontStyle::UNDERLINE),
                        style
                            .font_style
                            .contains(ferroui_textmate::FontStyle::STRIKETHROUGH),
                    ) {
                        (true, true) => {
                            TextDecorationCollection::parse("Underline,Strikethrough").ok()
                        }
                        (true, false) => TextDecorationCollection::parse("Underline").ok(),
                        (false, true) => TextDecorationCollection::parse("Strikethrough").ok(),
                        _ => None,
                    };
                    let properties = Rc::new(GenericTextRunProperties::with_all(
                        typeface,
                        editor.font_size(),
                        decorations,
                        foreground,
                        background,
                        BaselineAlignment::Baseline,
                        None,
                        None,
                    )) as Rc<dyn TextRunProperties>;
                    let start = display.visual_offset(source_offset + start);
                    let end = display.visual_offset(source_offset + end);
                    if end > start {
                        styles.push(ValueSpan::new(
                            start as i32,
                            (end - start) as i32,
                            properties,
                        ));
                    }
                }
            }
        } else if let Some(highlighter) = editor.document_highlighter() {
            for source_number in source_lines {
                match highlighter.highlight_line(source_number) {
                    Ok(highlighted) => {
                        for section in highlighted.sections {
                            let defaults = GenericTextRunProperties::with_all(
                                Typeface::new(editor.font_family()),
                                editor.font_size(),
                                None,
                                editor.foreground(),
                                None,
                                BaselineAlignment::Baseline,
                                None,
                                None,
                            );
                            let properties = match crate::NativeHighlightingStyle::resolve(
                                &section.color,
                                Some(&editor),
                            ) {
                                Ok(style) => style.text_run_properties(&defaults),
                                Err(error) => {
                                    editor.report_error(error.to_string());
                                    continue;
                                }
                            };
                            let start =
                                display.visual_offset(section.offset.saturating_sub(line.offset()));
                            let end = display.visual_offset(
                                (section.offset + section.length).saturating_sub(line.offset()),
                            );
                            if end > start {
                                styles.push(ValueSpan::new(
                                    start as i32,
                                    (end - start) as i32,
                                    properties,
                                ));
                            }
                        }
                    }
                    Err(error) => editor.report_error(error.to_string()),
                }
            }
        }
        if let Some((first, length)) = preedit {
            let start = first as i32;
            let end = (first + length) as i32;
            let mut split = Vec::new();
            for span in styles {
                let a = span.start();
                let b = a + span.length();
                if b <= start || a >= end {
                    split.push(span);
                } else {
                    if a < start {
                        split.push(ValueSpan::new(a, start - a, span.value().clone()));
                    }
                    if b > end {
                        split.push(ValueSpan::new(end, b - end, span.value().clone()));
                    }
                }
            }
            let properties = Rc::new(GenericTextRunProperties::with_all(
                Typeface::new(editor.font_family()),
                editor.font_size(),
                TextDecorationCollection::parse("Underline").ok(),
                editor.foreground(),
                None,
                BaselineAlignment::Baseline,
                None,
                None,
            )) as Rc<dyn TextRunProperties>;
            split.push(ValueSpan::new(start, length as i32, properties));
            split.sort_by_key(ValueSpan::start);
            styles = split;
        }
        let layout = TextLayout::from_utf16(
            ReadOnlyMemory::from_vec(display.text.clone()),
            Typeface::new(editor.font_family()),
            TextLayoutOptions {
                font_size: editor.font_size(),
                foreground: editor.foreground(),
                text_wrapping: if editor.word_wrap() {
                    TextWrapping::Wrap
                } else {
                    TextWrapping::NoWrap
                },
                max_width: if editor.word_wrap() {
                    (self.viewport.get().width - self.gutter_width() - 4.0).max(1.0)
                } else {
                    f64::INFINITY
                },
                line_height: f64::NAN,
                text_style_overrides: if styles.is_empty() {
                    None
                } else {
                    Some(Rc::from(styles))
                },
                ..Default::default()
            },
        );
        self.heights
            .borrow_mut()
            .set_height(index, layout.height().max(self.line_height()));
        let result = Rc::new(VisualLine {
            offset: line.offset(),
            length: end - line.offset(),
            display,
            layout,
            preedit,
        });
        self.cache.borrow_mut().insert(index, result.clone());
        Some(result)
    }
    fn schedule_highlight(&self, target: usize) {
        if self.highlight_pending.replace(true) {
            return;
        }
        let weak = self.to_ref().downgrade();
        Dispatcher::ui_thread().post_local(
            move || {
                if let Some(area) = weak.upgrade() {
                    area.highlight_pending.set(false);
                    if let Some(editor) = area.editor() {
                        if let Some(installation) = editor.textmate_installation() {
                            let done = installation.advance_to(&editor.document(), target, 256);
                            area.cache.borrow_mut().clear();
                            area.invalidate_visual();
                            if !done && installation.last_error().is_none() {
                                area.schedule_highlight(target);
                            }
                        }
                    }
                }
            },
            DispatcherPriority::BACKGROUND,
        );
    }
    pub fn offset_at_point(&self, point: Point) -> usize {
        self.selection_position_at_point(point, false).0
    }

    fn space_advance(&self) -> f64 {
        if self.space_advance.get() > 0.0 {
            return self.space_advance.get();
        }
        let Some(editor) = self.editor() else {
            return 7.0;
        };
        let layout = TextLayout::new(
            " ",
            Typeface::new(editor.font_family()),
            TextLayoutOptions {
                font_size: editor.font_size(),
                ..Default::default()
            },
        );
        let width = layout.width_including_trailing_whitespace().max(1.0);
        self.space_advance.set(width);
        width
    }

    fn selection_position_on_line(
        &self,
        line: &VisualLine,
        point: Point,
        allow_virtual_space: bool,
    ) -> (usize, usize) {
        if allow_virtual_space {
            let edge = Self::virtual_space_bounds(&line.layout);
            if point.x.is_finite() && point.x > edge.x && point.y >= edge.y {
                let extra = ((point.x - edge.x) / self.space_advance()).round().max(0.0) as usize;
                return (
                    line.offset + line.display.document_offset(line.display.text.len()),
                    extra,
                );
            }
        }
        let hit = line.layout.hit_test_point(point);
        let visual = (hit.text_position().max(0) as usize).min(line.display.text.len());
        let offset = line.offset + line.display.document_offset(visual);
        (offset, 0)
    }

    fn selection_position_at_point(
        &self,
        point: Point,
        allow_virtual_space: bool,
    ) -> (usize, usize) {
        let offset = self.offset.get();
        let y = (point.y + offset.y).max(0.0);
        let index = self.heights.borrow().line_at(y);
        let Some(line) = self.line_layout(index) else {
            return (0, 0);
        };
        let top = self.heights.borrow().top(index);
        self.selection_position_on_line(
            &line,
            Point::new(point.x + offset.x - self.gutter_width(), y - top),
            allow_virtual_space,
        )
    }

    /// Resolves a viewport point through the actual formatter, including tabs,
    /// folded text and optional cells beyond the final glyph of a logical line.
    pub fn position_at_point(
        &self,
        point: Point,
        allow_virtual_space: bool,
    ) -> Option<crate::TextViewPosition> {
        let editor = self.editor()?;
        let (offset, extra) = self.selection_position_at_point(point, allow_virtual_space);
        let location = editor.document().get_location(offset).ok()?;
        let index = self.heights.borrow().visible_owner(location.line - 1);
        let line = self.line_layout(index)?;
        let visual = line.display.visual_offset(offset - line.offset);
        let mut result =
            crate::TextViewPosition::with_visual_column(location, visual.saturating_add(extra));
        let normal = line.layout.hit_test_text_position(visual as i32);
        let local_y = point.y + self.offset.get().y - self.heights.borrow().top(index);
        result.is_at_end_of_line = extra == 0 && local_y < normal.y;
        Some(result)
    }

    pub(crate) fn caret_position(&self) -> Option<crate::TextViewPosition> {
        let editor = self.editor()?;
        let primary = editor.selection().primary();
        let location = editor.document().get_location(primary.caret).ok()?;
        let mut result = crate::TextViewPosition::with_visual_column(
            location,
            self.visual_column_for_offset(primary.caret)?
                .saturating_add(primary.caret_virtual_space),
        );
        result.is_at_end_of_line = self.caret_at_end_of_line.get();
        Some(result)
    }

    pub(crate) fn visual_column_for_offset(&self, offset: usize) -> Option<usize> {
        let editor = self.editor()?;
        let logical = editor.document().get_line_by_offset(offset).ok()?;
        if self.viewport.get().width > 0.0 {
            let index = self
                .heights
                .borrow()
                .visible_owner(logical.line_number() - 1);
            let line = self.line_layout(index)?;
            return Some(line.display.visual_offset(offset - line.offset));
        }
        let source = editor
            .document()
            .get_text_utf16(logical.offset(), logical.length())
            .ok()?;
        let mut display = DisplayLine::empty();
        display.push_text(&source, 0, editor.options().indentation_size);
        Some(display.visual_offset(offset - logical.offset()))
    }

    fn caret_bounds(layout: &TextLayout, visual: usize, at_end: bool) -> Rect {
        if !at_end {
            return layout.hit_test_text_position(visual as i32);
        }
        let index = layout
            .get_line_index_from_character_index(visual as i32, true)
            .max(0) as usize;
        let Some(row) = layout.text_lines().get(index) else {
            return Rect::default();
        };
        Rect::new(
            row.get_distance_from_character_hit(ferroui_base::media::CharacterHit::new(
                visual as i32,
            )),
            layout
                .text_lines()
                .iter()
                .take(index)
                .map(|line| line.height())
                .sum(),
            1.0,
            row.height(),
        )
    }

    /// Finds the center of the adjacent formatted row. Measuring that row
    /// before consulting its top keeps variable font sizes and wrapped lines
    /// from moving the caret back into the row it just left.
    fn adjacent_row_y(&self, upward: bool) -> Option<f64> {
        let editor = self.editor()?;
        let document = editor.document();
        let caret = editor.selection().primary();
        let logical = document.get_line_by_offset(caret.caret).ok()?;
        let index = self
            .heights
            .borrow()
            .visible_owner(logical.line_number() - 1);
        let line = self.line_layout(index)?;
        let visual = line.display.visual_offset(caret.caret - line.offset);
        let rows = line.layout.text_lines();
        let current = if caret.caret_virtual_space > 0 {
            rows.len().checked_sub(1)?
        } else {
            line.layout
                .get_line_index_from_character_index(visual as i32, self.caret_at_end_of_line.get())
                .max(0) as usize
        };
        let scroll_y = self.offset.get().y;
        let row_center = |layout: &TextLayout, row: usize| -> Option<f64> {
            Some(
                layout
                    .text_lines()
                    .iter()
                    .take(row)
                    .map(|row| row.height())
                    .sum::<f64>()
                    + layout.text_lines().get(row)?.height() * 0.5,
            )
        };
        if upward && current > 0 {
            return Some(
                self.heights.borrow().top(index) - scroll_y
                    + row_center(&line.layout, current - 1)?,
            );
        }
        if !upward && current + 1 < rows.len() {
            return Some(
                self.heights.borrow().top(index) - scroll_y
                    + row_center(&line.layout, current + 1)?,
            );
        }
        let next = if upward {
            self.heights.borrow().visible_owner(index.checked_sub(1)?)
        } else {
            let next = self.heights.borrow().next_visible(index + 1);
            if next >= document.line_count() {
                return None;
            }
            next
        };
        let target = self.line_layout(next)?;
        let row = if upward {
            target.layout.text_lines().len().checked_sub(1)?
        } else {
            0
        };
        Some(self.heights.borrow().top(next) - scroll_y + row_center(&target.layout, row)?)
    }

    /// Inserts rectangle clipboard rows at the same shaped horizontal
    /// position without temporarily changing the live selection. Folded rows
    /// are omitted. A view that has not been arranged uses core cell metrics.
    pub(crate) fn paste_rectangle(
        &self,
        text: &str,
        select_inserted_text: bool,
    ) -> Result<bool, ferroui_edit_core::editing::EditError> {
        use ferroui_edit_core::editing::EditError;
        let Some(editor) = self.editor() else {
            return Ok(false);
        };
        let session = editor.session();
        if self.viewport.get().width <= 0.0 {
            return session.paste_rectangle(text, select_inserted_text);
        }
        let selection = session.selection();
        if !selection.is_empty() || selection.segments().len() != 1 {
            return Ok(false);
        }
        let document = editor.document();
        let version = document.version();
        let caret = selection.primary();
        let first = document.get_line_by_offset(caret.caret)?;
        let mut row_count = 1_usize;
        let mut bytes = text.bytes().peekable();
        while let Some(byte) = bytes.next() {
            if byte == b'\r' || byte == b'\n' {
                if byte == b'\r' && bytes.peek() == Some(&b'\n') {
                    bytes.next();
                }
                row_count += 1;
                if row_count > document.line_count() {
                    return Ok(false);
                }
            }
        }
        let x = self.x_for_position(caret.caret, caret.caret_virtual_space);
        let mut positions = Vec::new();
        positions
            .try_reserve_exact(row_count)
            .map_err(|_| EditError::VirtualSpaceTooLarge)?;
        positions.push(caret);
        let mut index = self.heights.borrow().visible_owner(first.line_number() - 1);
        for _ in 1..row_count {
            index = self.heights.borrow().next_visible(index + 1);
            if index >= document.line_count() {
                return Ok(false);
            }
            let Some(line) = self.line_layout(index) else {
                return Ok(false);
            };
            let (offset, extra) = self.selection_position_on_line(&line, Point::new(x, 0.0), true);
            positions.push(SelectionSegment::with_virtual_space(
                offset, extra, offset, extra,
            ));
        }
        if !session.options().enable_virtual_space && !selection.is_rectangular() {
            let last = document.get_line_by_offset(positions.last().unwrap().caret)?;
            if last.length() < caret.caret - first.offset() {
                return Ok(false);
            }
        }
        if editor.document() != document
            || document.version() != version
            || editor.selection() != selection
        {
            return Err(EditError::StaleEditPlan);
        }
        session.paste_rectangle_at(
            &Selection::from_rectangle_segments(positions, row_count - 1),
            text,
            select_inserted_text,
        )?;
        Ok(true)
    }

    fn rectangle_anchor_at_point(&self, point: Point) -> (usize, f64) {
        let scroll = self.offset.get();
        let line = self.heights.borrow().line_at((point.y + scroll.y).max(0.0));
        (line, point.x + scroll.x - self.gutter_width())
    }

    /// Virtual cells follow the final formatted row's physical right edge.
    /// The logical EOL caret may be inside that row for a trailing RTL run.
    fn virtual_space_bounds(layout: &TextLayout) -> Rect {
        let rows = layout.text_lines();
        let Some(last) = rows.last() else {
            return layout.hit_test_text_position(0);
        };
        Rect::new(
            last.start() + last.width_including_trailing_whitespace(),
            rows.iter()
                .take(rows.len() - 1)
                .map(|row| row.height())
                .sum(),
            1.0,
            last.height(),
        )
    }

    fn rectangle_anchor_for_selection(&self, selection: &Selection) -> (usize, f64) {
        let primary = selection.primary();
        let segment = if selection.is_rectangular() {
            if selection.primary_index() == 0 {
                selection.segments().last().copied().unwrap_or(primary)
            } else {
                selection.segments().first().copied().unwrap_or(primary)
            }
        } else {
            primary
        };
        let index = self
            .editor()
            .and_then(|editor| editor.document().get_line_by_offset(segment.anchor).ok())
            .map_or(0, |line| {
                self.heights.borrow().visible_owner(line.line_number() - 1)
            });
        (
            index,
            self.x_for_position(segment.anchor, segment.anchor_virtual_space),
        )
    }

    fn x_for_position(&self, offset: usize, extra: usize) -> f64 {
        let Some(editor) = self.editor() else {
            return 0.0;
        };
        let Ok(document_line) = editor.document().get_line_by_offset(offset) else {
            return 0.0;
        };
        let index = self
            .heights
            .borrow()
            .visible_owner(document_line.line_number() - 1);
        let Some(line) = self.line_layout(index) else {
            return 0.0;
        };
        let visual = line.display.visual_offset(offset - line.offset);
        let edge = if extra > 0 {
            Self::virtual_space_bounds(&line.layout)
        } else {
            line.layout.hit_test_text_position(visual as i32)
        };
        edge.x + extra as f64 * self.space_advance()
    }

    /// Gets a visible insertion rectangle in this control's coordinates.
    /// Returns `None` for invalid offsets, hidden fold interiors, or positions
    /// outside the arranged viewport. Popup hosts can anchor to a tracked
    /// document offset without moving the editor's caret.
    pub fn rectangle_for_offset(&self, offset: usize) -> Option<Rect> {
        let viewport = self.viewport.get();
        if viewport.width <= 0.0 || viewport.height <= 0.0 {
            return None;
        }
        let editor = self.editor()?;
        let document_line = editor.document().get_line_by_offset(offset).ok()?;
        if self
            .folds
            .borrow()
            .iter()
            .any(|fold| fold.start_offset() < offset && offset < fold.end_offset())
        {
            return None;
        }
        let index = self
            .heights
            .borrow()
            .visible_owner(document_line.line_number() - 1);
        let scroll = self.offset.get();
        let top = self.heights.borrow().top(index) - scroll.y;
        if top >= viewport.height {
            return None;
        }
        let line = self.line_layout(index)?;
        let visual = line.display.visual_offset(offset - line.offset);
        let rect = line.layout.hit_test_text_position(visual as i32);
        let result = Rect::new(
            rect.x + self.gutter_width() - scroll.x,
            rect.y + top,
            1.0,
            rect.height.max(self.line_height()),
        );
        (result.y + result.height > 0.0
            && result.y < viewport.height
            && result.x + result.width > self.gutter_width()
            && result.x < viewport.width)
            .then_some(result)
    }

    fn select_rectangle_to(
        &self,
        anchor: (usize, f64),
        caret: (usize, f64),
    ) -> Result<(), ferroui_edit_core::editing::EditError> {
        let editor = self
            .editor()
            .ok_or(ferroui_edit_core::editing::EditError::InvalidSelection)?;
        let count = editor.document().line_count();
        let first = anchor.0.min(caret.0).min(count - 1);
        let last = anchor.0.max(caret.0).min(count - 1);
        let mut segments = Vec::new();
        let mut index = self.heights.borrow().visible_owner(first);
        while index <= last {
            let line = self
                .line_layout(index)
                .ok_or(ferroui_edit_core::editing::EditError::InvalidSelection)?;
            let (a, av) = self.selection_position_on_line(&line, Point::new(anchor.1, 0.0), true);
            let (c, cv) = self.selection_position_on_line(&line, Point::new(caret.1, 0.0), true);
            segments.push(SelectionSegment::with_virtual_space(a, av, c, cv));
            index = self.heights.borrow().next_visible(index + 1);
        }
        let primary = if caret.0 < anchor.0 {
            0
        } else {
            segments.len().saturating_sub(1)
        };
        editor.set_selection(Selection::from_rectangle_segments(segments, primary))
    }

    /// Selects a rectangle using viewport coordinates and the shaped glyph
    /// positions of each visible logical line. Collapsed lines are skipped.
    pub fn set_rectangular_selection(
        &self,
        anchor: Point,
        caret: Point,
    ) -> Result<(), ferroui_edit_core::editing::EditError> {
        self.select_rectangle_to(
            self.rectangle_anchor_at_point(anchor),
            self.rectangle_anchor_at_point(caret),
        )
    }
    pub fn cursor_rectangle(&self) -> Rect {
        if self.viewport.get().width <= 0.0 || self.viewport.get().height <= 0.0 {
            return Rect::default();
        }
        let Some(editor) = self.editor() else {
            return Rect::default();
        };
        let offset = editor.caret_offset();
        let Ok(document_line) = editor.document().get_line_by_offset(offset) else {
            return Rect::default();
        };
        let index = self
            .heights
            .borrow()
            .visible_owner(document_line.line_number() - 1);
        let Some(line) = self.line_layout(index) else {
            return Rect::default();
        };
        let position = if let Some((start, length)) = line.preedit {
            start
                + self
                    .preedit
                    .borrow()
                    .as_ref()
                    .and_then(|p| p.1)
                    .map_or(length, |c| (c.max(0) as usize).min(length))
        } else {
            line.display.visual_offset(offset - line.offset)
        };
        let r = if line.preedit.is_none() && editor.selection().primary().caret_virtual_space > 0 {
            Self::virtual_space_bounds(&line.layout)
        } else {
            Self::caret_bounds(
                &line.layout,
                position,
                line.preedit.is_none() && self.caret_at_end_of_line.get(),
            )
        };
        let scroll = self.offset.get();
        Rect::new(
            r.x + self.gutter_width() - scroll.x
                + if line.preedit.is_none() {
                    editor.selection().primary().caret_virtual_space as f64 * self.space_advance()
                } else {
                    0.0
                },
            r.y + self.heights.borrow().top(index) - scroll.y,
            1.0,
            r.height.max(self.line_height()),
        )
    }
    pub fn bring_caret_into_view(&self) {
        if let Some(editor) = self.editor() {
            if let Some(manager) = editor.folding_manager() {
                manager.unfold_at(editor.caret_offset());
            }
        }
        if self.viewport.get().width <= 0.0 || self.viewport.get().height <= 0.0 {
            return;
        }
        let r = self.cursor_rectangle();
        let size = self.viewport.get();
        let mut offset = self.offset.get();
        if r.y < 0.0 {
            offset.y += r.y;
        } else if r.y + r.height > size.height {
            offset.y += r.y + r.height - size.height;
        }
        if r.x < self.gutter_width() {
            offset.x += r.x - self.gutter_width();
        } else if r.x + 8.0 > size.width {
            offset.x += r.x + 8.0 - size.width;
        }
        self.update_extent();
        self.set_offset(offset);
    }
    pub(crate) fn set_preedit(&self, text: Option<&str>, cursor: Option<i32>) {
        *self.preedit.borrow_mut() = text
            .filter(|s| !s.is_empty())
            .map(|s| (s.to_owned(), cursor));
        self.cache.borrow_mut().clear();
        self.invalidate_visual();
        if let Some(client) = self.im_client.get() {
            client.raise_cursor_rectangle_changed();
        }
    }
}

impl IScrollable for TextArea {
    fn extent(&self) -> Size {
        self.extent.get()
    }
    fn viewport(&self) -> Size {
        self.viewport.get()
    }
    fn offset(&self) -> Vector {
        self.offset.get()
    }
    fn set_offset(&self, value: Vector) {
        let extent = self.extent.get();
        let viewport = self.viewport.get();
        let value = Vector::new(
            if self.horizontal.get() && value.x.is_finite() {
                value.x.clamp(0.0, (extent.width - viewport.width).max(0.0))
            } else {
                0.0
            },
            if self.vertical.get() && value.y.is_finite() {
                value
                    .y
                    .clamp(0.0, (extent.height - viewport.height).max(0.0))
            } else {
                0.0
            },
        );
        if self.offset.replace(value) != value {
            self.invalidate_visual();
            self.raise_scroll_invalidated();
            if let Some(client) = self.im_client.get() {
                client.raise_cursor_rectangle_changed();
            }
        }
    }
    fn can_horizontally_scroll(&self) -> bool {
        self.horizontal.get()
    }
    fn can_vertically_scroll(&self) -> bool {
        self.vertical.get()
    }
}
impl ILogicalScrollable for TextArea {
    fn set_can_horizontally_scroll(&self, value: bool) {
        self.horizontal.set(value);
        self.set_offset(self.offset.get());
    }
    fn set_can_vertically_scroll(&self, value: bool) {
        self.vertical.set(value);
        self.set_offset(self.offset.get());
    }
    fn is_logical_scroll_enabled(&self) -> bool {
        true
    }
    fn scroll_size(&self) -> Size {
        Size::new(self.line_height(), self.line_height() * 3.0)
    }
    fn page_scroll_size(&self) -> Size {
        self.viewport.get()
    }
    fn scroll_invalidated(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        let token = self.scroll_invalidated.add(handler);
        let handlers = self.scroll_invalidated.clone();
        Disposable::create(move || {
            handlers.remove(token);
        })
    }
    fn raise_scroll_invalidated(&self) {
        if self.rendering.get() {
            // Measured line heights/widths become available during drawing.
            // Scrollbar visibility can invalidate layout, which the compositor
            // forbids during the render pass. Publish once after it completes.
            if !self.scroll_notification_pending.replace(true) {
                let weak = self.to_ref().downgrade();
                Dispatcher::ui_thread().post_local(
                    move || {
                        if let Some(area) = weak.upgrade() {
                            area.scroll_notification_pending.set(false);
                            area.raise_scroll_invalidated();
                        }
                    },
                    DispatcherPriority::NORMAL,
                );
            }
            return;
        }
        for (_, handler) in self.scroll_invalidated.snapshot().iter() {
            handler();
        }
    }
    fn bring_into_view(&self, target: &Ref<Control>, rect: Rect) -> bool {
        if !target.ptr_eq(&self.to_ref()) {
            return false;
        }
        let old = self.offset.get();
        let mut next = old;
        let viewport = self.viewport.get();
        if rect.x < self.gutter_width() {
            next.x += rect.x - self.gutter_width();
        } else if rect.x + rect.width > viewport.width {
            next.x += rect.x + rect.width - viewport.width;
        }
        if rect.y < 0.0 {
            next.y += rect.y;
        } else if rect.y + rect.height > viewport.height {
            next.y += rect.y + rect.height - viewport.height;
        }
        self.set_offset(next);
        self.offset.get() != old
    }
    fn get_control_in_direction(
        &self,
        _direction: NavigationDirection,
        _from: Option<&Ref<Control>>,
    ) -> Option<Ref<Control>> {
        None
    }
}

impl Drop for TextArea {
    fn drop(&mut self) {
        if let Some(timer) = self.blink_timer.get() {
            timer.stop();
        }
    }
}

use super::*;
use crate::tests::{laid_out_editor, test_scope};
use crate::{
    BackgroundRendererRef, DottedLineMargin, EditorAdornmentError, IBackgroundRenderer, ITextViewConnect,
    ITextViewMargin, KnownLayer, LineNumberMargin, TextDocument, TextViewMargin, VisualLineError,
};
use ferroui_base::layout::ILayoutManager;
use ferroui_base::media::{DrawingContext, PlatformDrawingContext};
use ferroui_base::rendering::testing::{DrawingLog, MockDrawingContextImpl};
use ferroui_controls::{Border, Button, Panel, Window};

fn layout(area: &TextArea, width: f64, height: f64) {
    let size = Size::new(width, height);
    area.measure(size);
    area.arrange(Rect::from_size(size));
}

#[test]
fn line_numbers_host_a_native_separator_with_independent_spacing() {
    use ferroui_base::Thickness;

    let _scope = test_scope();
    let editor = laid_out_editor("one\ntwo");
    let area = editor.text_area();
    editor.set_line_numbers_margin(Thickness::new(8.0, 3.0, 4.0, 5.0));
    editor.set_line_numbers_foreground(Some(Rc::new(ImmutableSolidColorBrush::from_uint32(0xff2854ab))));
    let find_separator = || {
        area.visual_children().snapshot().iter().find_map(|child| child.cast::<DottedLineMargin>())
    };
    assert!(find_separator().is_none());
    editor.set_show_line_numbers(true);
    layout(&area, 220.0, 90.0);
    let separator = find_separator().unwrap();
    let numbers = area.line_number_margin().unwrap().clone();
    assert!(numbers.visual_parent().unwrap().ptr_eq(&area));
    assert!(numbers.text_view().unwrap().ptr_eq(&area));
    let content = area.content_viewport();
    let bounds = separator.bounds();
    assert!((content.x - bounds.right() - 4.0).abs() <= 0.5);
    assert_eq!((bounds.y, bounds.width, bounds.height), (3.0, 1.0, 82.0));
    assert!(!separator.is_hit_test_visible());
    assert!(area.line_number_separator_contains(bounds.center().x));

    let log = DrawingLog::new();
    let mut platform = MockDrawingContextImpl::new(log.clone());
    let mut core = PlatformDrawingContext::borrowed(&mut platform);
    let mut context = DrawingContext::new(&mut core);
    separator.render(&mut context);
    editor.set_line_numbers_foreground(None);
    separator.render(&mut context);
    context.dispose();
    let painted = log.entries();
    assert_eq!(painted.iter().filter(|call| call.starts_with("DrawLine")).count(), 1);
    assert!(painted.iter().any(|call| call.contains("#ff2854ab@1")));

    editor.set_show_line_numbers(false);
    layout(&area, 220.0, 90.0);
    assert!(find_separator().is_none());
    assert!(separator.parent().is_none() && separator.visual_parent().is_none());
    assert!(numbers.visual_parent().is_none() && numbers.text_view().is_none());
    assert_eq!(area.content_viewport().x, 6.0);
    let custom = TextViewMargin::new(DottedLineMargin::new().upcast());
    area.insert_left_margin(0, custom.clone()).unwrap();
    layout(&area, 220.0, 90.0);
    assert_eq!(custom.control().margin(), Thickness::symmetric(2.0, 0.0));
    assert_eq!(area.content_viewport().x, 11.0);
}

#[test]
fn gutters_stay_visible_when_a_selection_frame_queues_another_redraw() {
    let _scope = test_scope();
    let editor = laid_out_editor("{\nfirst\nsecond\n}");
    let area = editor.text_area();
    editor.set_show_line_numbers(true);
    editor.set_line_numbers_foreground(Some(Rc::new(ImmutableSolidColorBrush::from_uint32(0xff3157ac))));
    let manager = editor.install_folding();
    manager.create_folding(0, editor.document().text_length()).unwrap();
    layout(&area, 220.0, 90.0);

    let numbers = area.line_number_margin().unwrap().clone();
    let folding = area.folding_margin().unwrap().clone();
    editor.set_selection(Selection::simple(2, 10)).unwrap();
    crate::tests::render_editor(&editor);
    assert!(
        area.redraw_pending.get() == PendingRedraw::Requested,
        "a caret-line change queues a later frame"
    );

    let log = DrawingLog::new();
    let mut platform = MockDrawingContextImpl::new(log.clone());
    let mut core = PlatformDrawingContext::borrowed(&mut platform);
    let mut context = DrawingContext::new(&mut core);
    numbers.render(&mut context);
    folding.render(&mut context);
    context.dispose();
    let painted = log.entries();
    assert_eq!(painted.iter().filter(|entry| entry.starts_with("DrawGlyphRun #ff3157ac")).count(), 4);
    assert!(painted.iter().any(|entry| entry.starts_with("DrawLine")));
}

#[test]
fn native_line_number_margin_paints_and_selects_rows_from_a_sibling_panel() {
    use ferroui_base::input::{IPointer, MouseButton, Pointer, PointerPointProperties, PointerType, RawInputModifiers};
    use ferroui_controls::DockPanel;

    let _scope = test_scope();
    let editor = laid_out_editor("first\r\nsecond\r\nthird");
    let area = editor.text_area();
    let gutter = LineNumberMargin::new();
    let binding = gutter.as_margin();
    binding.set_text_view(Some(&area)).unwrap();
    assert!(gutter.as_margin().text_view().unwrap().ptr_eq(&area));
    gutter.set_foreground(Some(Rc::new(ImmutableSolidColorBrush::from_uint32(0xff3157ac))));
    gutter.set_min_width_in_digits(4);

    let panel = DockPanel::new();
    panel.children().add(gutter.clone());
    panel.children().add(editor.clone());
    let window = Window::new();
    window.set_width(300.0);
    window.set_height(140.0);
    window.set_content(Some(Control::boxed(panel)));
    window.show();
    window.layout_manager().execute_layout_pass();
    assert!(gutter.bounds().width > 0.0);
    assert!(!gutter.is_visual_ancestor_of(&area));
    assert!((gutter.bounds().width - gutter.preferred_width(3)).abs() <= 0.5);
    assert_eq!(area.gutter_width(), 6.0);

    let log = DrawingLog::new();
    let mut platform = MockDrawingContextImpl::new(log.clone());
    let mut core = PlatformDrawingContext::borrowed(&mut platform);
    let mut context = DrawingContext::new(&mut core);
    gutter.render(&mut context);
    context.dispose();
    assert_eq!(log.entries().iter().filter(|entry| entry.starts_with("DrawGlyphRun") && entry.contains("#ff3157ac")).count(), 3);

    let row = area.visual_line_bounds(2).unwrap();
    let center = area.translate_point(Point::new(0.0, row.center().y), &window).unwrap();
    let x = gutter.translate_point(Point::new(5.0, 0.0), &window).unwrap().x;
    let position = Point::new(x, center.y);
    let pointer = Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true);
    let press = PointerPressedEventArgs::new(
        gutter.clone(), pointer.clone(), &window, position, 1,
        PointerPointProperties::new(RawInputModifiers::LEFT_MOUSE_BUTTON, PointerUpdateKind::LeftButtonPressed),
        KeyModifiers::NONE, 1,
    );
    gutter.raise_event(&press);
    assert!(press.handled());
    assert!(pointer.captured().unwrap().ptr_eq(&area));
    assert_eq!(editor.selected_text().unwrap(), "second\r\n");
    let release = PointerReleasedEventArgs::new(
        area.clone(), pointer.clone(), &window, position, 2,
        PointerPointProperties::new(RawInputModifiers::NONE, PointerUpdateKind::LeftButtonReleased),
        KeyModifiers::NONE, MouseButton::Left,
    );
    area.raise_event(&release);
    assert!(release.handled() && pointer.captured().is_none());
    window.close();
}

#[derive(Default)]
struct MarginEvents {
    binding: RefCell<Vec<(Option<String>, Option<String>)>>,
    connection: RefCell<Vec<&'static str>>,
    first_lines: RefCell<Vec<usize>>,
}

impl ITextViewConnect for MarginEvents {
    fn add_to_text_view(&self, _: &TextArea) {
        self.connection.borrow_mut().push("add");
    }

    fn remove_from_text_view(&self, _: &TextArea) {
        self.connection.borrow_mut().push("remove");
    }
}

impl ITextViewMargin for MarginEvents {
    fn document_changed(&self, before: Option<&TextDocument>, after: Option<&TextDocument>) {
        self.binding.borrow_mut().push((
            before.map(TextDocument::text),
            after.map(TextDocument::text),
        ));
    }

    fn viewport_changed(&self, view: &TextArea) {
        if let Some(first) = view.visible_editor_lines().ok().and_then(|lines| lines.into_iter().next()) {
            self.first_lines
                .borrow_mut()
                .push(first.line.first_document_line().line_number());
        }
    }
}

#[test]
fn explicitly_bound_margin_tracks_document_and_viewport_outside_its_visual_host() {
    let _scope = test_scope();
    let first = laid_out_editor(&format!("first\n{}", "line\n".repeat(40)));
    let second = laid_out_editor(&format!("second\n{}", "other\n".repeat(40)));
    let first_area = first.text_area();
    let second_area = second.text_area();
    let panel = Panel::new();
    let control = Border::new();
    panel.children().add(control.clone());
    let events = Rc::new(MarginEvents::default());
    let margin = TextViewMargin::with_behavior(control.clone().upcast(), events.clone());
    margin.set_text_view(Some(&first_area)).unwrap();
    assert_eq!(margin.document(), Some(first.document()));
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(events.first_lines.borrow().last(), Some(&1));
    first_area.set_offset(Vector::new(0.0, 320.0));
    Dispatcher::ui_thread().run_jobs(None);
    assert!(events.first_lines.borrow().last().is_some_and(|&line| line > 1));

    margin.set_text_view(Some(&second_area)).unwrap();
    assert_eq!(margin.document(), Some(second.document()));
    assert_eq!(events.connection.borrow().as_slice(), ["add", "remove", "add"]);
    assert_eq!(events.binding.borrow().last(), Some(&(Some(first.text()), Some(second.text()))));
    Dispatcher::ui_thread().run_jobs(None);
    let notices = events.first_lines.borrow().len();
    first.set_document(TextDocument::from_text("unobserved"));
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(events.first_lines.borrow().len(), notices);

    let previous = second.text();
    second.set_document(TextDocument::from_text("replacement"));
    assert_eq!(events.binding.borrow().last(), Some(&(Some(previous), Some("replacement".into()))));
    drop(second);
    Dispatcher::ui_thread().run_jobs(None);
    assert!(margin.text_view().unwrap().ptr_eq(&second_area));
    assert_eq!(events.binding.borrow().last(), Some(&(Some("replacement".into()), None)));
    drop(second_area);
    assert!(margin.text_view().is_none());
    assert_eq!(events.connection.borrow().as_slice(), ["add", "remove", "add", "remove"]);
    assert!(control.parent().unwrap().ptr_eq(&panel));
}

#[test]
fn margin_mounting_preserves_an_explicit_assignment_and_rejects_a_different_view() {
    let _scope = test_scope();
    let first = laid_out_editor("first");
    let second = laid_out_editor("second");
    let first_area = first.text_area();
    let second_area = second.text_area();
    let events = Rc::new(MarginEvents::default());
    let margin = TextViewMargin::with_behavior(Border::new().upcast(), events.clone());
    margin.set_text_view(Some(&first_area)).unwrap();
    assert_eq!(second_area.insert_left_margin(0, margin.clone()), Err(EditorAdornmentError::DifferentTextView));
    assert!(second_area.left_margins().is_empty());
    assert!(margin.control().visual_parent().is_none());
    first_area.insert_left_margin(0, margin.clone()).unwrap();
    assert_eq!(margin.set_text_view(Some(&second_area)), Err(EditorAdornmentError::DifferentTextView));
    first_area.remove_left_margin(0).unwrap();
    assert!(margin.text_view().unwrap().ptr_eq(&first_area));
    assert_eq!(events.connection.borrow().as_slice(), ["add"]);
    first_area.insert_left_margin(0, margin.clone()).unwrap();
    margin.set_text_view(None).unwrap();
    assert!(margin.text_view().unwrap().ptr_eq(&first_area));
    first_area.remove_left_margin(0).unwrap();
    assert!(margin.text_view().is_none());
    assert_eq!(events.connection.borrow().as_slice(), ["add", "remove"]);
    assert_eq!(events.binding.borrow().as_slice(), [(None, Some("first".into())), (Some("first".into()), None)]);

    margin.set_text_view(Some(&second_area)).unwrap();
    drop(margin);
    assert_eq!(events.connection.borrow().as_slice(), ["add", "remove", "add", "remove"]);
}

#[test]
fn left_margins_resize_the_wrapped_view_and_follow_scroll_and_document_changes() {
    let _scope = test_scope();
    let text = format!("{}\n{}", "abcdefghij ".repeat(9), "line\n".repeat(25));
    let editor = laid_out_editor(&text);
    editor.set_word_wrap(true);
    let window = Window::new();
    window.set_width(168.0);
    window.set_height(100.0);
    window.set_content(Some(Control::boxed(editor.clone())));
    window.show();
    window.layout_manager().execute_layout_pass();
    let area = editor.text_area();
    layout(&area, 168.0, 68.0);
    let before = area.content_viewport();
    let initial_height = area.visual_line_bounds(1).unwrap().height;
    let events = Rc::new(MarginEvents::default());
    let probe = Border::new();
    probe.set_width(68.0);
    let plain = Border::new();
    plain.set_width(18.0);
    let margin = TextViewMargin::with_behavior(probe.clone().upcast(), events.clone());
    let changes = Rc::new(Cell::new(0));
    let calls = changes.clone();
    let _subscription = area.left_margins_changed(move || calls.set(calls.get() + 1));
    area.insert_left_margin(0, margin.clone()).unwrap();
    area.insert_left_margin(1, TextViewMargin::new(plain.clone().upcast()))
        .unwrap();
    layout(&area, 168.0, 68.0);
    assert_eq!(area.content_viewport().x, before.x + 86.0);
    assert!(area.visual_line_bounds(1).unwrap().height > initial_height);
    assert_eq!(probe.bounds().x, 0.0);
    assert_eq!(plain.bounds().x, 68.0);
    assert!(probe.parent().is_some());
    assert!(margin.text_view().unwrap().ptr_eq(&area));
    assert!(
        margin
            .document()
            .is_some_and(|doc| doc == editor.document())
    );
    assert_eq!(events.connection.borrow().as_slice(), ["add"]);

    area.move_left_margin(1, 0).unwrap();
    layout(&area, 168.0, 68.0);
    assert_eq!(plain.bounds().x, 0.0);
    assert_eq!(probe.bounds().x, 18.0);
    probe.set_width(32.0);
    window.layout_manager().execute_layout_pass();
    layout(&area, 168.0, 68.0);
    assert_eq!(area.content_viewport().x, before.x + 50.0);
    Dispatcher::ui_thread().run_jobs(None);
    area.set_offset(Vector::new(0.0, 400.0));
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(probe.bounds().y, 0.0);
    assert!(
        events
            .first_lines
            .borrow()
            .last()
            .is_some_and(|&line| line > 1)
    );

    editor.set_document(TextDocument::from_text("replacement"));
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(
        events.binding.borrow().last(),
        Some(&(Some(text), Some("replacement".into())))
    );
    area.remove_left_margin(1).unwrap();
    assert!(probe.parent().is_none() && probe.visual_parent().is_none());
    assert!(margin.text_view().is_none());
    assert_eq!(events.connection.borrow().as_slice(), ["add", "remove"]);
    assert_eq!(
        events.binding.borrow().last(),
        Some(&(Some("replacement".into()), None))
    );
    assert_eq!(changes.get(), 4);
    window.close();
}

#[test]
fn margin_control_input_and_attachment_are_owned_by_the_native_tree() {
    use ferroui_base::input::{IKeyboardDevice, KeyboardDevice};
    use ferroui_controls::testing::TestServices;
    let _scope = crate::tests::test_scope_with_services(
        TestServices::styled_window()
            .with_keyboard_device(|| Some(KeyboardDevice::new() as Rc<dyn IKeyboardDevice>)),
    );
    let editor = laid_out_editor("original");
    let window = Window::new();
    window.set_content(Some(Control::boxed(editor.clone())));
    window.show();
    let area = editor.text_area();
    let button = Button::new();
    button.set_width(48.0);
    let margin = TextViewMargin::new(button.clone().upcast());
    assert_eq!(
        area.insert_left_margin(1, margin.clone()),
        Err(EditorAdornmentError::InvalidIndex)
    );
    let owner = Panel::new();
    owner.children().add(button.clone());
    assert_eq!(
        area.insert_left_margin(0, margin.clone()),
        Err(EditorAdornmentError::AlreadyParented)
    );
    owner.children().remove(&button);
    area.insert_left_margin(0, margin).unwrap();
    layout(&area, 250.0, 100.0);
    assert!(button.focus());
    let mut input = TextInputEventArgs::new();
    input.text = Some("x".into());
    input.set_routed_event(Some(InputElement::text_input_event()));
    button.raise_event(&input);
    assert!(!input.handled());
    assert_eq!(editor.text(), "original");
    area.remove_left_margin(0).unwrap();
    assert!(area.is_focused());
    window.close();
}

struct PhaseBrush(KnownLayer, u32);

impl ITextViewConnect for PhaseBrush {}

impl IBackgroundRenderer for PhaseBrush {
    fn layer(&self) -> KnownLayer {
        self.0
    }
    fn draw(&self, area: &TextArea, ctx: &mut DrawingContext) -> Result<(), VisualLineError> {
        paint(ctx, self.1, area.content_viewport());
        Ok(())
    }
}

fn paint(ctx: &mut DrawingContext, color: u32, rect: Rect) {
    let brush = Rc::new(ImmutableSolidColorBrush::from_uint32(color)) as Rc<dyn IBrush>;
    ctx.fill_rectangle(&brush, rect, 0.0);
}

fn overlay(color: u32) -> Ref<Control> {
    let panel = Panel::new();
    panel.set_background(Some(Rc::new(ImmutableSolidColorBrush::from_uint32(color))));
    panel.upcast()
}

fn visual_paint_order(area: &TextArea, colors: &[u32]) -> Vec<usize> {
    fn render_tree(visual: &Visual, context: &mut DrawingContext) {
        if !visual.is_visible() {
            return;
        }
        visual.render(context);
        for child in visual.visual_children().snapshot().iter() {
            render_tree(child, context);
        }
    }

    let log = DrawingLog::new();
    let mut platform = MockDrawingContextImpl::new(log.clone());
    let mut core = PlatformDrawingContext::borrowed(&mut platform);
    let mut context = DrawingContext::new(&mut core);
    render_tree(area, &mut context);
    context.dispose();
    log.entries()
        .iter()
        .filter_map(|entry| {
            colors
                .iter()
                .position(|color| entry.contains(&format!("#{color:08x}")))
        })
        .collect()
}

#[test]
fn native_layers_interleave_with_editor_phases_and_restore_replaced_controls() {
    use LayerInsertionPosition::{Above, Below, Replace};
    let _scope = test_scope();
    let editor = laid_out_editor("first\nsecond");
    let area = editor.text_area();
    let colors = [
        0xffaa0101, 0xffaa0202, 0xffaa0303, 0xffaa0404, 0xffaa0505, 0xffaa0606, 0xffaa0707,
        0xffaa0808,
    ];
    for (index, phase) in [
        KnownLayer::Background,
        KnownLayer::Selection,
        KnownLayer::Text,
        KnownLayer::Caret,
    ]
    .into_iter()
    .enumerate()
    {
        let renderer: BackgroundRendererRef = Rc::new(PhaseBrush(phase, colors[index * 2])).into();
        area.background_renderers().add(renderer);
    }
    for (index, layer, position) in [
        (1, KnownLayer::Selection, Below),
        (3, KnownLayer::Text, Below),
        (5, KnownLayer::Text, Above),
        (7, KnownLayer::Caret, Above),
    ] {
        area.insert_layer(overlay(colors[index]), layer, position)
            .unwrap();
    }
    layout(&area, 360.0, 180.0);
    assert_eq!(visual_paint_order(&area, &colors), [0, 1, 2, 3, 4, 5, 6, 7]);
    let custom = area.layers()[0].control.clone();
    assert_eq!(
        custom.bounds(),
        Rect::from_size(area.content_viewport().size())
    );
    assert!(
        custom
            .visual_parent()
            .is_some_and(|parent| parent.clip_to_bounds())
    );
    assert_eq!(
        area.insert_layer(custom, KnownLayer::Caret, Above),
        Err(EditorAdornmentError::AlreadyParented)
    );
    assert_eq!(
        area.insert_layer(overlay(colors[4]), KnownLayer::Background, Replace),
        Err(EditorAdornmentError::InvalidLayerPosition)
    );
    let replacement = overlay(colors[4]);
    area.insert_layer(replacement.clone(), KnownLayer::Text, Replace)
        .unwrap();
    layout(&area, 360.0, 180.0);
    assert_eq!(visual_paint_order(&area, &colors), [0, 1, 2, 3, 4, 5, 6, 7]);
    let next = overlay(colors[5]);
    area.insert_layer(next.clone(), KnownLayer::Text, Replace)
        .unwrap();
    layout(&area, 360.0, 180.0);
    assert!(replacement.parent().is_none() && replacement.visual_parent().is_none());
    assert_eq!(visual_paint_order(&area, &colors), [0, 1, 2, 3, 5, 5, 6, 7]);
    assert!(area.remove_layer(&next).unwrap());
    assert!(!area.remove_layer(&next).unwrap());
    layout(&area, 360.0, 180.0);
    assert_eq!(visual_paint_order(&area, &colors), [0, 1, 2, 3, 4, 5, 6, 7]);
}

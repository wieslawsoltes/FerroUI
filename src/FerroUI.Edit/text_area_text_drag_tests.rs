use super::*;
use crate::tests::{test_scope_with_services, TestScope};
use ferroui_base::input::raw::{
    IDragDropDevice, IRawInputEventArgs, RawPointerEventArgs, RawPointerEventType,
};
use ferroui_base::input::{
    DragDropDevice, IInputManager, InputManager, MouseDevice, RawInputModifiers,
};
use ferroui_controls::platform::InProcessDragSource;
use ferroui_controls::testing::TestServices;
use ferroui_controls::Window;
use ferroui_edit_core::editing::TextSegmentReadOnlySectionProvider;

struct Scene {
    _scope: TestScope,
    window: Ref<Window>,
    editor: Ref<TextEditor>,
    input: Rc<dyn IInputManager>,
    mouse: Rc<MouseDevice>,
    timestamp: Cell<u64>,
}

impl Scene {
    fn new(text: &str) -> Self {
        let input: Rc<dyn IInputManager> = Rc::new(InputManager::new());
        let scope = test_scope_with_services(
            TestServices::styled_window().with_input_manager(input.clone()),
        );
        FerroLocator::current_mutable()
            .bind::<dyn IDragDropDevice>()
            .to_constant(DragDropDevice::instance() as Rc<dyn IDragDropDevice>)
            .bind::<dyn IPlatformDragSource>()
            .to_func(|| Some(InProcessDragSource::new()));
        let editor = TextEditor::new();
        editor.set_text(text).unwrap();
        let window = Window::new();
        window.set_width(320.0);
        window.set_height(100.0);
        window.set_content(Some(Control::boxed(editor.clone())));
        window.show();
        Dispatcher::ui_thread().run_jobs(None);
        editor.text_area().ensure_visual_lines().unwrap();
        Self {
            _scope: scope,
            editor,
            window,
            input,
            mouse: MouseDevice::new(),
            timestamp: Cell::new(1000),
        }
    }

    fn point(&self, offset: usize) -> Point {
        self.editor_point(&self.editor, offset)
    }

    fn editor_point(&self, editor: &TextEditor, offset: usize) -> Point {
        let area = editor.text_area();
        let rect = area.rectangle_for_offset(offset).unwrap();
        area.translate_point(
            Point::new(rect.x + 0.25, rect.y + rect.height / 2.0),
            &self.window,
        )
        .unwrap()
    }

    fn pointer(&self, event: RawPointerEventType, offset: usize, modifiers: RawInputModifiers) {
        self.pointer_at(event, self.point(offset), modifiers);
    }

    fn pointer_at(&self, event: RawPointerEventType, point: Point, modifiers: RawInputModifiers) {
        // Independent test clicks should not be interpreted as double clicks.
        let timestamp = self.timestamp.get();
        self.timestamp.set(timestamp + 1000);
        let args = Rc::new(RawPointerEventArgs::new(
            self.mouse.clone(),
            timestamp,
            self.window.input_root(),
            event,
            point,
            modifiers,
        ));
        self.input.process_input(args as Rc<dyn IRawInputEventArgs>);
        Dispatcher::ui_thread().run_jobs(None);
    }

    fn drag(&self, from: usize, to: usize, modifiers: RawInputModifiers) {
        let button = RawInputModifiers::LEFT_MOUSE_BUTTON;
        self.pointer(RawPointerEventType::LeftButtonDown, from, button);
        self.pointer(RawPointerEventType::Move, to, button | modifiers);
        self.pointer(RawPointerEventType::LeftButtonUp, to, modifiers);
    }

    fn right_click(&self, offset: usize) {
        self.pointer(
            RawPointerEventType::RightButtonDown,
            offset,
            RawInputModifiers::RIGHT_MOUSE_BUTTON,
        );
        self.pointer(
            RawPointerEventType::RightButtonUp,
            offset,
            RawInputModifiers::NONE,
        );
    }
}

impl Drop for Scene {
    fn drop(&mut self) {
        self.window.close();
    }
}

#[test]
fn pointer_drag_moves_as_one_undo_step_and_control_copies() {
    let scene = Scene::new("alpha beta gamma");
    let editor = &scene.editor;
    editor.select(6, 5).unwrap();
    let original = editor.selection();
    scene.drag(7, 0, RawInputModifiers::NONE);
    assert_eq!(editor.text(), "beta alpha gamma");
    assert_eq!(editor.selected_text().unwrap(), "beta ");
    assert!(editor.undo().unwrap());
    assert_eq!(editor.text(), "alpha beta gamma");
    assert_eq!(editor.selection(), original);
    assert!(editor.redo().unwrap());
    assert_eq!(editor.text(), "beta alpha gamma");
    editor.select(0, 5).unwrap();
    scene.drag(2, 16, RawInputModifiers::CONTROL);
    assert_eq!(editor.text(), "beta alpha gammabeta ");
    assert_eq!(editor.selected_text().unwrap(), "beta ");
}

#[test]
fn protected_drag_only_copies_and_drop_inside_the_source_is_cancelled() {
    let scene = Scene::new("alpha beta gamma");
    let editor = &scene.editor;
    let protected = Rc::new(TextSegmentReadOnlySectionProvider::new(editor.document()));
    protected.add(6, 4).unwrap();
    editor.session().set_read_only_section_provider(protected);
    editor.select(6, 4).unwrap();
    scene.drag(7, 9, RawInputModifiers::NONE);
    assert_eq!(editor.text(), "alpha beta gamma");
    assert_eq!(editor.selected_text().unwrap(), "beta");

    scene.drag(7, 0, RawInputModifiers::NONE);
    assert_eq!(editor.text(), "betaalpha beta gamma");
    assert_eq!(editor.selected_text().unwrap(), "beta");
}

#[test]
fn text_drag_preserves_rectangular_rows_in_a_single_transaction() {
    let scene = Scene::new("Afoo\nBbar\nC   \nD   ");
    let editor = &scene.editor;
    editor
        .set_selection(Selection::from_rectangle_segments(
            [SelectionSegment::new(1, 4), SelectionSegment::new(6, 9)],
            1,
        ))
        .unwrap();
    scene.drag(2, 11, RawInputModifiers::NONE);
    assert_eq!(editor.text(), "A\nB\nCfoo   \nDbar   ");
    assert!(editor.selection().is_rectangular());
    assert_eq!(editor.selected_text().unwrap(), "foo\nbar");
    assert!(editor.undo().unwrap());
    assert_eq!(editor.text(), "Afoo\nBbar\nC   \nD   ");
}

#[test]
fn control_word_gesture_and_release_option_keep_separate_selection_behavior() {
    let scene = Scene::new("alpha beta gamma");
    let editor = &scene.editor;
    let button = RawInputModifiers::LEFT_MOUSE_BUTTON;
    scene.pointer(
        RawPointerEventType::LeftButtonDown,
        7,
        button | RawInputModifiers::CONTROL,
    );
    assert_eq!(editor.selected_text().unwrap(), "beta");
    scene.pointer(
        RawPointerEventType::Move,
        13,
        button | RawInputModifiers::CONTROL,
    );
    assert_eq!(editor.selected_text().unwrap(), "beta gamma");
    scene.pointer(
        RawPointerEventType::LeftButtonUp,
        7,
        RawInputModifiers::CONTROL,
    );
    assert_eq!(editor.selected_text().unwrap(), "beta");

    editor.set_selection(Selection::empty(0)).unwrap();
    let mut options = editor.options();
    options.extend_selection_on_mouse_up = false;
    options.enable_text_drag_drop = false;
    editor.set_options(options);
    assert!(!DragDrop::get_allow_drop(&editor.text_area()));
    scene.pointer(RawPointerEventType::LeftButtonDown, 0, button);
    scene.pointer(RawPointerEventType::Move, 5, button);
    scene.pointer(
        RawPointerEventType::LeftButtonUp,
        10,
        RawInputModifiers::NONE,
    );
    assert_eq!(editor.selected_text().unwrap(), "alpha");
}

#[test]
fn apple_keymap_gives_box_word_and_copy_gestures_their_own_modifiers() {
    use ferroui_base::input::platform::PlatformHotkeyConfiguration;

    let scene = Scene::new("alpha beta\ngamma delta");
    FerroLocator::current_mutable().bind_to_self(Rc::new(PlatformHotkeyConfiguration::apple()));
    let editor = &scene.editor;
    let button = RawInputModifiers::LEFT_MOUSE_BUTTON;
    assert_eq!(crate::text_area::box_selection_modifier(), KeyModifiers::CONTROL);
    assert_eq!(crate::text_area::text_drag_copy_modifier(), KeyModifiers::ALT);

    scene.pointer(RawPointerEventType::LeftButtonDown, 7, button | RawInputModifiers::META);
    assert_eq!(editor.selected_text().unwrap(), "beta");
    scene.pointer(RawPointerEventType::LeftButtonUp, 7, RawInputModifiers::META);

    editor.set_selection(Selection::empty(0)).unwrap();
    // Control with a click is the secondary click on macOS: Option draws the rectangle.
    scene.pointer(RawPointerEventType::LeftButtonDown, 1, button | RawInputModifiers::ALT);
    scene.pointer(RawPointerEventType::Move, 14, button | RawInputModifiers::ALT);
    assert!(editor.selection().is_rectangular());
    scene.pointer(RawPointerEventType::LeftButtonUp, 14, RawInputModifiers::ALT);

    editor.set_selection(Selection::empty(0)).unwrap();
    scene.pointer(RawPointerEventType::LeftButtonDown, 1, button | RawInputModifiers::CONTROL);
    scene.pointer(RawPointerEventType::Move, 14, button | RawInputModifiers::CONTROL);
    assert!(!editor.selection().is_rectangular());
    scene.pointer(RawPointerEventType::LeftButtonUp, 14, RawInputModifiers::CONTROL);

    // Inside a selection Option is the copy of a text drag, not a new rectangle.
    editor.select(0, 5).unwrap();
    scene.pointer(RawPointerEventType::LeftButtonDown, 2, button | RawInputModifiers::ALT);
    assert_eq!(editor.selected_text().unwrap(), "alpha");
    assert!(!editor.selection().is_rectangular());
    scene.pointer(RawPointerEventType::LeftButtonUp, 2, RawInputModifiers::ALT);
}

#[test]
fn cross_document_move_completes_both_undo_groups_through_routed_input() {
    let scene = Scene::new("alpha beta");
    let target = TextEditor::new();
    target.set_text("destination").unwrap();
    scene.window.set_content(None);
    Dispatcher::ui_thread().run_jobs(None);
    let panel = ferroui_controls::StackPanel::new();
    for editor in [&scene.editor, &target] {
        editor.set_height(45.0);
        panel.children().add(editor.clone().upcast::<Control>());
    }
    scene.window.set_content(Some(Control::boxed(panel)));
    Dispatcher::ui_thread().run_jobs(None);

    scene.editor.select(0, 6).unwrap();
    let button = RawInputModifiers::LEFT_MOUSE_BUTTON;
    let destination = scene.editor_point(&target, 0);
    scene.pointer(RawPointerEventType::LeftButtonDown, 2, button);
    scene.pointer_at(RawPointerEventType::Move, destination, button);
    scene.pointer_at(RawPointerEventType::Move, destination, button);
    assert_eq!(
        target
            .text_area()
            .drop_caret
            .get()
            .map(|caret| caret.offset),
        Some(0)
    );
    scene.pointer_at(
        RawPointerEventType::LeftButtonUp,
        destination,
        RawInputModifiers::NONE,
    );
    assert!(target.text_area().drop_caret.get().is_none());
    assert_eq!(scene.editor.text(), "beta");
    assert_eq!(target.text(), "alpha destination");
    assert_eq!(target.selected_text().unwrap(), "alpha ");
    assert!(target.undo().unwrap());
    assert!(scene.editor.undo().unwrap());
    assert_eq!(scene.editor.selected_text().unwrap(), "alpha ");
    assert_eq!(target.text(), "destination");
}

#[test]
fn a_click_inside_selection_waits_for_release_and_drag_copy_can_be_cancelled() {
    let scene = Scene::new("alpha beta");
    let editor = &scene.editor;
    let button = RawInputModifiers::LEFT_MOUSE_BUTTON;
    editor.select(0, 5).unwrap();
    scene.pointer(RawPointerEventType::LeftButtonDown, 2, button);
    assert_eq!(editor.selected_text().unwrap(), "alpha");
    scene.pointer(
        RawPointerEventType::LeftButtonUp,
        2,
        RawInputModifiers::NONE,
    );
    assert_eq!(editor.selection(), Selection::empty(2));

    let cancellation = editor.add_handler(TextEditor::copying_to_clipboard_event(), |_, args| {
        assert!(args.is_drag_drop());
        assert!(!args.is_cut());
        args.set_cancel(true);
    });
    editor.select(0, 5).unwrap();
    scene.drag(2, 9, RawInputModifiers::NONE);
    assert_eq!(editor.text(), "alpha beta");
    assert_eq!(editor.selected_text().unwrap(), "alpha");
    drop(cancellation);
}

#[test]
fn external_text_drop_normalizes_input_and_obeys_destination_options() {
    let scene = Scene::new("alpha\r\nbeta");
    let editor = &scene.editor;
    let area = editor.text_area();
    let mut options = editor.options();
    options.convert_tabs_to_spaces = true;
    editor.set_options(options);
    editor.select(0, 5).unwrap();
    let position = scene.window.translate_point(scene.point(7), &area).unwrap();
    let data = DataTransfer::new();
    data.add(DataTransferItem::create_text(Some("x\n\ty")));
    let send_drop = |data: Rc<dyn IDataTransfer>| {
        let event = DragEventArgs::new(
            Some(DragDrop::drop_event()),
            data,
            area.clone().upcast(),
            position,
            KeyModifiers::NONE,
        );
        event.set_drag_effects(DragDropEffects::COPY);
        area.raise_event(&event);
        event.drag_effects()
    };

    assert_eq!(send_drop(DataTransfer::new()), DragDropEffects::NONE);
    editor.set_is_read_only(true);
    assert_eq!(send_drop(data.clone()), DragDropEffects::NONE);
    assert_eq!(editor.selected_text().unwrap(), "alpha");
    editor.set_is_read_only(false);
    assert_eq!(send_drop(data), DragDropEffects::COPY);
    assert_eq!(editor.text(), "alpha\r\nx\r\n    ybeta");
    assert_eq!(editor.selected_text().unwrap(), "x\r\n    y");
}

#[cfg(windows)]
#[test]
fn external_rectangular_drop_requires_a_valid_native_marker() {
    let name = "AvalonEditRectangularSelection";
    for (marker, expected) in [(&[1][..], "1X2\r\n3Y4"), (&[0][..], "1X\r\nY2\r\n34")] {
        let scene = Scene::new("12\r\n34");
        let area = scene.editor.text_area();
        let position = scene.window.translate_point(scene.point(1), &area).unwrap();
        let undo_count = scene.editor.document().undo_count();
        let item = DataTransferItem::create_text(Some("X\nY"));
        item.set(
            &DataFormat::create_bytes_platform_format(name),
            Some(Rc::from(marker)),
        );
        let data = DataTransfer::new();
        data.add(item);
        let event = DragEventArgs::new(
            Some(DragDrop::drop_event()),
            data,
            area.clone().upcast(),
            position,
            KeyModifiers::NONE,
        );
        event.set_drag_effects(DragDropEffects::COPY);
        area.raise_event(&event);
        assert_eq!(event.drag_effects(), DragDropEffects::COPY);
        assert_eq!(scene.editor.text(), expected);
        assert_eq!(scene.editor.document().undo_count(), undo_count + 1);
    }
}

#[test]
fn disabling_default_input_releases_a_pending_selection_gesture() {
    let scene = Scene::new("alpha beta");
    let area = scene.editor.text_area();
    let button = RawInputModifiers::LEFT_MOUSE_BUTTON;
    scene.editor.select(0, 5).unwrap();
    scene.pointer(RawPointerEventType::LeftButtonDown, 2, button);
    assert!(area.pointer_selection.borrow().is_some());
    area.set_active_input_mode(crate::EditorInputMode::Disabled).unwrap();
    assert!(area.pointer_selection.borrow().is_none());
    assert!(!DragDrop::get_allow_drop(&area));
    scene.pointer(RawPointerEventType::Move, 8, button);
    scene.pointer(RawPointerEventType::LeftButtonUp, 8, RawInputModifiers::NONE);
    assert_eq!(scene.editor.selected_text().unwrap(), "alpha");
}

#[test]
fn right_click_caret_is_opt_in_and_preserves_the_selection_for_context_commands() {
    let scene = Scene::new("alpha beta gamma");
    let editor = &scene.editor;
    let area = editor.text_area();
    editor.select(6, 4).unwrap();
    scene.right_click(7);
    assert_eq!(editor.caret_offset(), 10);
    assert_eq!(editor.selected_text().unwrap(), "beta");

    area.set_right_click_moves_caret(true);
    let delivered = Rc::new(Cell::new(false));
    let event_delivered = delivered.clone();
    let _listener = area.add_disposable_handler_with(
        InputElement::pointer_pressed_event(),
        move |_, event| {
            if event
                .get_current_point(None)
                .properties
                .is_right_button_pressed
            {
                event_delivered.set(!event.handled());
            }
        },
        ferroui_base::interactivity::RoutingStrategies::BUBBLE,
        false,
    );
    scene.right_click(7);
    assert!(delivered.get());
    assert!(area.pointer_selection.borrow().is_none());
    assert_eq!(editor.caret_offset(), 7);
    assert_eq!(editor.selected_text().unwrap(), "beta");
    assert_eq!(
        area.cursor_rectangle().x,
        area.rectangle_for_offset(7).unwrap().x
    );

    assert!(area.perform_text_input("x").unwrap());
    assert_eq!(editor.text(), "alpha x gamma");
    assert!(editor.undo().unwrap());
    assert_eq!(editor.caret_offset(), 7);
    assert_eq!(editor.selected_text().unwrap(), "beta");

    let mut key = KeyEventArgs::new();
    key.key = Key::Escape;
    key.set_routed_event(Some(InputElement::key_down_event()));
    area.raise_event(&key);
    assert!(key.handled());
    assert_eq!(editor.caret_offset(), 7);
    assert_eq!(editor.selected_text().unwrap(), "beta");

    editor.select(6, 4).unwrap();
    scene.right_click(13);
    assert_eq!(editor.caret_offset(), 13);
    assert!(editor.selection().is_empty());
}

#[test]
fn right_click_on_a_secondary_range_changes_the_active_caret_and_rebases_it() {
    let scene = Scene::new("alpha beta gamma");
    let editor = &scene.editor;
    let area = editor.text_area();
    area.set_right_click_moves_caret(true);
    let ranges = [SelectionSegment::new(1, 4), SelectionSegment::new(6, 10)];
    editor
        .set_selection(Selection::from_segments(ranges, 0))
        .unwrap();
    scene.right_click(8);
    assert_eq!(editor.selection().segments(), &ranges);
    assert_eq!(editor.selection().primary_index(), 1);
    assert_eq!(editor.caret_offset(), 8);

    editor.document().insert(0, "@").unwrap();
    assert_eq!(editor.caret_offset(), 9);
    assert_eq!(editor.selection().primary(), SelectionSegment::new(7, 11));
    let mut key = KeyEventArgs::new();
    key.key = Key::Right;
    key.set_routed_event(Some(InputElement::key_down_event()));
    area.raise_event(&key);
    assert!(key.handled());
    assert!(editor.selection().is_empty());
    assert_eq!(editor.caret_offset(), 10);
}

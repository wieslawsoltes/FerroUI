use super::*;
use ferroui_base::input::{
    Pointer, PointerPointProperties, PointerType, PointerUpdateKind, RawInputModifiers,
};
use ferroui_controls::Window;
use ferroui_controls::testing::{TestServices, UnitTestApplication};

fn key(control: &Control, key: Key, modifiers: KeyModifiers) -> KeyEventArgs {
    let mut event = KeyEventArgs::new();
    event.key = key;
    event.key_modifiers = modifiers;
    event.set_routed_event(Some(InputElement::key_down_event()));
    control.raise_event(&event);
    event
}

struct PointerDriver {
    window: Ref<Window>,
    control: Ref<PropertyNumberScrubber>,
    pointer: Rc<Pointer>,
}
impl PointerDriver {
    fn new(control: Ref<PropertyNumberScrubber>) -> Self {
        let window = Window::new();
        window.set_width(240.0);
        window.set_height(60.0);
        window.set_content(Some(Control::boxed(&control)));
        window.show();
        window.update_layout();
        Self {
            window,
            control,
            pointer: Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true),
        }
    }
    fn position(&self, x: f64) -> Point {
        self.control
            .translate_point(Point::new(x, 13.0), &self.window)
            .unwrap()
    }
    fn press(&self, x: f64) {
        self.control.raise_event(&PointerPressedEventArgs::new(
            self.control.to_ref(),
            self.pointer.clone(),
            &self.window,
            self.position(x),
            1,
            PointerPointProperties::new(
                RawInputModifiers::LEFT_MOUSE_BUTTON,
                PointerUpdateKind::LeftButtonPressed,
            ),
            KeyModifiers::NONE,
            1,
        ));
    }
    fn move_to(&self, x: f64, modifiers: KeyModifiers) {
        self.control.raise_event(&PointerEventArgs::new(
            Some(InputElement::pointer_moved_event()),
            self.control.to_ref(),
            self.pointer.clone(),
            Some(&self.window),
            self.position(x),
            2,
            PointerPointProperties::new(
                RawInputModifiers::LEFT_MOUSE_BUTTON,
                PointerUpdateKind::Other,
            ),
            modifiers,
        ));
    }
    fn release(&self, x: f64, modifiers: KeyModifiers) {
        self.control.raise_event(&PointerReleasedEventArgs::new(
            self.control.to_ref(),
            self.pointer.clone(),
            &self.window,
            self.position(x),
            3,
            PointerPointProperties::new(
                RawInputModifiers::NONE,
                PointerUpdateKind::LeftButtonReleased,
            ),
            modifiers,
            MouseButton::Left,
        ));
    }
}
impl Drop for PointerDriver {
    fn drop(&mut self) {
        self.pointer.capture(None);
        self.window.close();
    }
}

#[test]
fn numeric_scrubbing_keeps_drafts_local_and_commits_once_with_precision_and_bounds() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let control = PropertyNumberScrubber::new();
    control
        .configure(
            "Width",
            80.0,
            PropertyNumberOptions {
                minimum: 0.0,
                maximum: 100.0,
                ..Default::default()
            },
        )
        .unwrap();
    let values = Rc::new(RefCell::new(Vec::new()));
    let seen = values.clone();
    control.on_commit(move |value| seen.borrow_mut().push(value));
    let driver = PointerDriver::new(control);
    driver.press(20.0);
    driver.move_to(45.0, KeyModifiers::CONTROL);
    assert_eq!(driver.control.draft_value(), Some(82.5));
    assert!(values.borrow().is_empty());
    assert_eq!(
        driver.control.value(),
        80.0,
        "only the host can accept a value"
    );
    driver.release(45.0, KeyModifiers::CONTROL);
    assert_eq!(*values.borrow(), [82.5]);
    assert!(driver.pointer.captured().is_none());
    driver.press(20.0);
    driver.release(40.0, KeyModifiers::SHIFT);
    assert_eq!(*values.borrow(), [82.5, 100.0]);
    driver.press(20.0);
    driver.release(21.0, KeyModifiers::NONE);
    assert_eq!(
        values.borrow().len(),
        2,
        "a click does not author a property"
    );
}

#[test]
fn numeric_scrubbing_cancels_on_escape_capture_loss_or_reconfiguration() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let control = PropertyNumberScrubber::new();
    control.configure("X", 10.0, Default::default()).unwrap();
    let values = Rc::new(RefCell::new(Vec::new()));
    let seen = values.clone();
    control.on_commit(move |value| seen.borrow_mut().push(value));
    let driver = PointerDriver::new(control);
    driver.press(20.0);
    driver.move_to(50.0, KeyModifiers::NONE);
    assert!(key(&driver.control, Key::Escape, KeyModifiers::NONE).handled());
    assert!(driver.pointer.captured().is_none());
    driver.release(50.0, KeyModifiers::NONE);
    driver.press(20.0);
    driver.move_to(50.0, KeyModifiers::NONE);
    driver.pointer.platform_capture_lost();
    driver.release(50.0, KeyModifiers::NONE);
    driver.press(20.0);
    driver
        .control
        .configure("X", 11.0, Default::default())
        .unwrap();
    driver.release(50.0, KeyModifiers::NONE);
    assert!(values.borrow().is_empty());
    assert!(key(&driver.control, Key::Right, KeyModifiers::CONTROL).handled());
    assert_eq!(*values.borrow(), [11.1]);
    assert!(
        driver
            .control
            .configure("X", f64::NAN, Default::default())
            .is_err()
    );
    assert_eq!(driver.control.value(), 11.0);
}

#[test]
fn component_editor_preserves_xaml_order_and_commits_all_fields_atomically() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    assert_eq!(
        PropertyComponentsKind::Thickness.parse("2,5"),
        Some([2.0, 5.0, 2.0, 5.0])
    );
    assert_eq!(
        PropertyComponentsKind::CornerRadius.parse("2,5"),
        Some([2.0, 2.0, 5.0, 5.0])
    );
    assert!(PropertyComponentsKind::Thickness.parse("NaN").is_none());
    let editor = PropertyComponentsEditor::new(PropertyComponentsKind::Thickness, false);
    editor.configure(Some([1.0, 2.0, 3.0, 4.0])).unwrap();
    let values = Rc::new(RefCell::new(Vec::new()));
    let seen = values.clone();
    editor.on_commit(move |value| seen.borrow_mut().push(value));
    let left = editor.state.fields[0].upgrade().unwrap();
    let top = editor.state.fields[1].upgrade().unwrap();
    left.set_text(Some("12"));
    top.set_text(Some("24"));
    assert!(values.borrow().is_empty());
    assert!(key(&top, Key::Enter, KeyModifiers::NONE).handled());
    assert_eq!(*values.borrow(), [[12.0, 24.0, 3.0, 4.0]]);
    assert!(key(&left, Key::Escape, KeyModifiers::NONE).handled());
    assert_eq!(left.text().as_deref(), Some("1"));
    assert_eq!(top.text().as_deref(), Some("2"));
    key(&left, Key::Up, KeyModifiers::CONTROL);
    assert_eq!(left.text().as_deref(), Some("1.1"));
    assert_eq!(
        values.borrow().len(),
        1,
        "component arrow keys edit the draft until Enter"
    );
}

#[test]
fn component_editor_requires_explicit_values_for_mixed_selection_and_validates_before_commit() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let editor = PropertyComponentsEditor::new(PropertyComponentsKind::CornerRadius, true);
    editor.configure(None).unwrap();
    let values = Rc::new(RefCell::new(Vec::new()));
    let seen = values.clone();
    editor.on_commit(move |value| seen.borrow_mut().push(value));
    let first = editor.state.fields[0].upgrade().unwrap();
    first.set_text(Some("6"));
    key(&first, Key::Enter, KeyModifiers::NONE);
    assert!(
        values.borrow().is_empty(),
        "a partial mixed edit cannot silently zero other sides"
    );
    editor.set_linked(true);
    key(&first, Key::Enter, KeyModifiers::NONE);
    assert_eq!(*values.borrow(), [[6.0; 4]]);
    first.set_text(Some("-1"));
    key(&first, Key::Enter, KeyModifiers::NONE);
    assert!(DataValidationErrors::get_has_errors(&first));
    first.set_text(Some("inf"));
    key(&first, Key::Enter, KeyModifiers::NONE);
    assert!(DataValidationErrors::get_has_errors(&first));
    key(&first, Key::Escape, KeyModifiers::NONE);
    assert!(!DataValidationErrors::get_has_errors(&first));
    assert_eq!(values.borrow().len(), 1);
    assert!(editor.configure(Some([-1.0; 4])).is_err());
    assert!(editor.configure(Some([f64::NAN; 4])).is_err());
}

#[test]
fn linked_components_disclose_individual_sides_without_authoring() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let editor = PropertyComponentsEditor::new(PropertyComponentsKind::Thickness, false);
    editor.configure(Some([8.0; 4])).unwrap();
    let values = Rc::new(RefCell::new(Vec::new()));
    let seen = values.clone();
    editor.on_commit(move |value| seen.borrow_mut().push(value));
    editor.set_linked(true);
    assert!(values.borrow().is_empty());
    assert!(editor.state.fields[0].upgrade().unwrap().is_visible());
    assert!(!editor.state.fields[1].upgrade().unwrap().is_visible());
    let field = editor.state.fields[0].upgrade().unwrap();
    field.set_text(Some("12"));
    key(&field, Key::Enter, KeyModifiers::NONE);
    assert_eq!(*values.borrow(), [[12.0; 4]]);
    editor.configure(Some([12.0; 4])).unwrap();
    editor.set_linked(false);
    assert!(
        editor
            .state
            .fields
            .iter()
            .all(|field| field.upgrade().unwrap().is_visible())
    );
    assert_eq!(
        values.borrow().len(),
        1,
        "view expansion does not author a value"
    );
    let top = editor.state.fields[1].upgrade().unwrap();
    top.set_text(Some("24"));
    key(&top, Key::Enter, KeyModifiers::NONE);
    assert_eq!(*values.borrow(), [[12.0; 4], [12.0, 24.0, 12.0, 12.0]]);
}

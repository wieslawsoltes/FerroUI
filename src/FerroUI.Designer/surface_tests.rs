//! Pointer intents must stay bound to the document that armed or started them.
use super::*;
use ferroui_base::input::{
    IPointer, Pointer, PointerPointProperties, PointerType, PointerUpdateKind, RawInputModifiers,
};
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::{ContentControl, Grid, Window};

const SOURCE: &str = r#"<Canvas xmlns="https://github.com/ferroui" Width="400" Height="240" Background="White"><Button Name="Target" Canvas.Left="20" Canvas.Top="20" Width="80" Height="40" Content="Original" /></Canvas>"#;
const INSERT: &str = r#"<Button Name="Inserted" Content="Inserted" Width="60" Height="30" />"#;

struct SurfaceFixture {
    window: Ref<Window>,
    surface: Ref<DesignSurface>,
    session: Rc<RefCell<DesignerSession>>,
    pointer: Rc<Pointer>,
    commits: Rc<RefCell<Vec<DesignGesture>>>,
}
impl SurfaceFixture {
    fn new() -> Self {
        let preview = ContentControl::new();
        let session = Rc::new(RefCell::new(
            DesignerSession::with_uri(
                SOURCE,
                preview.clone(),
                "ferres://FerroUI.Designer/Surface.xaml".into(),
            )
            .unwrap(),
        ));
        let surface = DesignSurface::new();
        surface.attach(&session);
        surface.set_grid(0.0);
        let commits = Rc::new(RefCell::new(Vec::new()));
        let observed = commits.clone();
        let weak = surface.downgrade();
        surface.on_commit(move |gesture| {
            observed.borrow_mut().push(gesture.clone());
            let surface = weak.upgrade().unwrap();
            let session = surface.session().unwrap();
            crate::workspace_tools::commit(&mut session.borrow_mut(), gesture).unwrap();
        });
        let grid = Grid::new();
        grid.children().add(preview.upcast::<Control>());
        grid.children().add(surface.clone().upcast::<Control>());
        let window = Window::new();
        window.set_width(400.0);
        window.set_height(240.0);
        window.set_content(Some(Control::boxed(&grid)));
        window.show();
        window.update_layout();
        session.borrow_mut().refresh_runtime_mapping();
        assert!(session.borrow().can_manipulate());
        Self {
            window,
            surface,
            session,
            commits,
            pointer: Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true),
        }
    }
    fn position(&self, point: Point) -> Point {
        self.surface.translate_point(point, &self.window).unwrap()
    }
    fn press(&self, point: Point) {
        self.press_with(point, KeyModifiers::ALT);
    }
    fn press_with(&self, point: Point, modifiers: KeyModifiers) {
        self.surface.raise_event(&PointerPressedEventArgs::new(
            self.surface.to_ref(),
            self.pointer.clone(),
            &self.window,
            self.position(point),
            1,
            PointerPointProperties::new(
                RawInputModifiers::LEFT_MOUSE_BUTTON,
                PointerUpdateKind::LeftButtonPressed,
            ),
            modifiers,
            1,
        ));
    }
    fn move_to(&self, point: Point) {
        self.surface.raise_event(&PointerEventArgs::new(
            Some(InputElement::pointer_moved_event()),
            self.surface.to_ref(),
            self.pointer.clone(),
            Some(&self.window),
            self.position(point),
            2,
            PointerPointProperties::new(
                RawInputModifiers::LEFT_MOUSE_BUTTON,
                PointerUpdateKind::Other,
            ),
            KeyModifiers::ALT,
        ));
    }
    fn release(&self, point: Point) {
        self.release_with(point, KeyModifiers::ALT);
    }
    fn release_with(&self, point: Point, modifiers: KeyModifiers) {
        self.surface.raise_event(&PointerReleasedEventArgs::new(
            self.surface.to_ref(),
            self.pointer.clone(),
            &self.window,
            self.position(point),
            3,
            PointerPointProperties::new(
                RawInputModifiers::NONE,
                PointerUpdateKind::LeftButtonReleased,
            ),
            modifiers,
            MouseButton::Left,
        ));
    }
    fn change(&mut self, change: Change) {
        match change {
            Change::Source => {
                let target = self
                    .session
                    .borrow()
                    .document()
                    .ast()
                    .find_name("Target")
                    .unwrap();
                self.session
                    .borrow_mut()
                    .set_property(&[target], "Content", "Changed")
                    .unwrap();
            }
            Change::Uri => self
                .session
                .borrow_mut()
                .set_document_uri("ferres://FerroUI.Designer/AnotherSurface.xaml".into())
                .unwrap(),
            Change::SameSession | Change::OtherSession => {
                let session = self.session.borrow();
                let revision = session.document().revision();
                let uri = session.document_uri().to_owned();
                let syntax = session.document().ast_snapshot();
                let host = session.preview_host();
                drop(session);
                // Release the original URI's live hot-reload registration while
                // retaining the context that captured that URI and syntax.
                self.session
                    .borrow_mut()
                    .set_document_uri(format!("{uri}.retired"))
                    .unwrap();
                let replacement = DesignerSession::with_uri(
                    SOURCE.replace("Original", "Unrelated"),
                    host,
                    uri.clone(),
                )
                .unwrap();
                assert_eq!(replacement.document_uri(), uri);
                assert_eq!(replacement.document().revision(), revision);
                assert!(!std::ptr::eq(replacement.document().ast(), syntax.as_ref()));
                if matches!(change, Change::SameSession) {
                    *self.session.borrow_mut() = replacement;
                } else {
                    self.session = Rc::new(RefCell::new(replacement));
                    self.surface.attach(&self.session);
                }
            }
        }
        self.window.update_layout();
        self.session.borrow_mut().refresh_runtime_mapping();
        assert!(self.session.borrow().can_manipulate());
    }
    fn source(&self) -> String {
        self.session.borrow().document().source().to_owned()
    }
}
impl Drop for SurfaceFixture {
    fn drop(&mut self) {
        self.pointer.capture(None);
        self.window.close();
    }
}
#[derive(Clone, Copy, Debug)]
enum Change {
    Source,
    Uri,
    SameSession,
    OtherSession,
}

#[test]
fn armed_insert_survives_an_unchanged_source_refresh_and_commits_one_transaction() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let fixture = SurfaceFixture::new();
    fixture.surface.arm_insert(INSERT.into());
    assert!(fixture.session.borrow_mut().update_source(SOURCE).valid);
    assert_eq!(fixture.session.borrow().document().revision(), 0);
    fixture.press(Point::new(200.0, 120.0));
    fixture.release(Point::new(200.0, 120.0));
    assert_eq!(fixture.commits.borrow().len(), 1);
    assert!(matches!(
        fixture.commits.borrow()[0],
        DesignGesture::Insert { .. }
    ));
    assert!(
        fixture
            .session
            .borrow()
            .document()
            .ast()
            .find_name("Inserted")
            .is_some()
    );
    assert_eq!(fixture.session.borrow().document().revision(), 1);
    fixture.session.borrow_mut().undo().unwrap();
    assert_eq!(fixture.source(), SOURCE);
}

#[test]
fn armed_insert_cannot_be_rebound_by_pointer_press_after_document_or_session_changes() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    for change in [
        Change::Source,
        Change::Uri,
        Change::SameSession,
        Change::OtherSession,
    ] {
        let mut fixture = SurfaceFixture::new();
        fixture.surface.arm_insert(INSERT.into());
        fixture.change(change);
        let source = fixture.source();
        fixture.press(Point::new(200.0, 120.0));
        fixture.release(Point::new(200.0, 120.0));
        assert!(fixture.commits.borrow().is_empty(), "{change:?}");
        assert_eq!(fixture.source(), source, "{change:?}");
        assert!(fixture.surface.insert.borrow().is_none());
        assert!(fixture.surface.gesture_context.borrow().is_none());
        // An explicit new palette command is bound to the current document.
        fixture.surface.arm_insert(INSERT.into());
        fixture.press(Point::new(200.0, 120.0));
        fixture.release(Point::new(200.0, 120.0));
        assert_eq!(fixture.commits.borrow().len(), 1, "{change:?}");
    }
}

#[test]
fn insertion_release_rejects_source_or_same_revision_identity_changes_after_press() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    for change in [Change::Source, Change::SameSession, Change::OtherSession] {
        let mut fixture = SurfaceFixture::new();
        fixture.surface.arm_insert(INSERT.into());
        fixture.press(Point::new(200.0, 120.0));
        fixture.change(change);
        let source = fixture.source();
        fixture.release(Point::new(220.0, 130.0));
        assert!(fixture.commits.borrow().is_empty(), "{change:?}");
        assert_eq!(fixture.source(), source, "{change:?}");
        assert!(fixture.pointer.captured().is_none());
    }
}

#[test]
fn moving_a_selection_rejects_same_uri_same_revision_replacement_on_move_and_release() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let fixture = SurfaceFixture::new();
    fixture.press(Point::new(60.0, 40.0));
    assert!(matches!(
        fixture
            .surface
            .gesture
            .borrow()
            .as_ref()
            .map(|gesture| gesture.handle),
        Some(Handle::Move)
    ));
    fixture.move_to(Point::new(92.0, 64.0));
    fixture.release(Point::new(92.0, 64.0));
    assert_eq!(fixture.commits.borrow().len(), 1);
    assert!(matches!(
        fixture.commits.borrow()[0],
        DesignGesture::Move { .. }
    ));
    drop(fixture);
    for move_before_release in [false, true] {
        let mut fixture = SurfaceFixture::new();
        fixture.press(Point::new(60.0, 40.0));
        assert!(fixture.surface.gesture.borrow().is_some());
        fixture.change(Change::SameSession);
        let source = fixture.source();
        if move_before_release {
            fixture.move_to(Point::new(92.0, 64.0));
            assert!(fixture.surface.gesture.borrow().is_none());
            assert!(fixture.pointer.captured().is_none());
        }
        fixture.release(Point::new(92.0, 64.0));
        assert!(fixture.commits.borrow().is_empty());
        assert_eq!(fixture.source(), source);
    }
}

#[test]
fn surface_keyboard_nudges_use_native_layout_transactions_and_respect_precision_and_permissions() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let fixture = SurfaceFixture::new();
    let target = fixture
        .session
        .borrow()
        .document()
        .ast()
        .find_name("Target")
        .unwrap();
    fixture.session.borrow_mut().select([target]);
    for (key, modifiers, expected) in [
        (Key::Right, KeyModifiers::NONE, (21.0, 20.0)),
        (Key::Down, KeyModifiers::SHIFT, (21.0, 30.0)),
        (Key::Left, KeyModifiers::CONTROL, (20.9, 30.0)),
    ] {
        let mut event = KeyEventArgs::new();
        event.key = key;
        event.key_modifiers = modifiers;
        event.set_routed_event(Some(InputElement::key_down_event()));
        fixture.surface.raise_event(&event);
        assert!(event.handled());
        let session = fixture.session.borrow();
        let node = session.document().ast().node(target).unwrap();
        assert!(
            (node
                .attribute("Canvas.Left")
                .unwrap()
                .parse::<f64>()
                .unwrap()
                - expected.0)
                .abs()
                < 1e-9
        );
        assert!(
            (node
                .attribute("Canvas.Top")
                .unwrap()
                .parse::<f64>()
                .unwrap()
                - expected.1)
                .abs()
                < 1e-9
        );
    }
    assert_eq!(fixture.commits.borrow().len(), 3);
    fixture.session.borrow_mut().undo().unwrap();
    fixture.session.borrow_mut().undo().unwrap();
    fixture.session.borrow_mut().undo().unwrap();
    assert_eq!(fixture.source(), SOURCE);
    fixture.session.borrow_mut().set_read_only(true);
    let mut event = KeyEventArgs::new();
    event.key = Key::Right;
    event.set_routed_event(Some(InputElement::key_down_event()));
    fixture.surface.raise_event(&event);
    assert!(!event.handled());
    assert_eq!(fixture.commits.borrow().len(), 3);
    assert_eq!(fixture.source(), SOURCE);
}

#[test]
fn shift_move_locks_the_dominant_axis_even_with_snapping_or_alt_precision() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    for (end, expected, modifiers) in [
        (
            Point::new(92.0, 45.0),
            (52.0, 20.0),
            KeyModifiers::SHIFT | KeyModifiers::ALT,
        ),
        (
            Point::new(63.0, 72.0),
            (20.0, 52.0),
            KeyModifiers::SHIFT | KeyModifiers::ALT,
        ),
        (Point::new(92.0, 45.0), (52.0, 20.0), KeyModifiers::SHIFT),
    ] {
        let fixture = SurfaceFixture::new();
        fixture.press(Point::new(60.0, 40.0));
        fixture.release_with(end, modifiers);
        assert_eq!(fixture.commits.borrow().len(), 1);
        let session = fixture.session.borrow();
        let node = session
            .document()
            .ast()
            .node(session.selection()[0])
            .unwrap();
        assert_eq!(
            node.attribute("Canvas.Left")
                .unwrap()
                .parse::<f64>()
                .unwrap(),
            expected.0
        );
        assert_eq!(
            node.attribute("Canvas.Top")
                .unwrap()
                .parse::<f64>()
                .unwrap(),
            expected.1
        );
    }
}

#[test]
fn marquee_selects_by_direction_supports_set_operations_and_restores_selection_on_escape() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let fixture = SurfaceFixture::new();
    let target = fixture
        .session
        .borrow()
        .document()
        .ast()
        .find_name("Target")
        .unwrap();
    // A left-to-right rectangle must contain the whole control.
    fixture.press_with(Point::new(10.0, 10.0), KeyModifiers::NONE);
    fixture.release_with(Point::new(60.0, 80.0), KeyModifiers::NONE);
    assert!(fixture.session.borrow().selection().is_empty());
    // A right-to-left rectangle picks a partial intersection.
    fixture.press_with(Point::new(120.0, 10.0), KeyModifiers::NONE);
    fixture.release_with(Point::new(60.0, 80.0), KeyModifiers::NONE);
    assert_eq!(fixture.session.borrow().selection(), [target]);
    // Ctrl+Shift subtracts; Ctrl toggles; Shift extends the original selection.
    fixture.press_with(
        Point::new(120.0, 10.0),
        KeyModifiers::CONTROL | KeyModifiers::SHIFT,
    );
    fixture.release_with(Point::new(60.0, 80.0), KeyModifiers::NONE);
    assert!(fixture.session.borrow().selection().is_empty());
    fixture.press_with(Point::new(120.0, 10.0), KeyModifiers::SHIFT);
    fixture.release_with(Point::new(60.0, 80.0), KeyModifiers::NONE);
    assert_eq!(fixture.session.borrow().selection(), [target]);
    fixture.press_with(Point::new(120.0, 10.0), KeyModifiers::CONTROL);
    fixture.release_with(Point::new(60.0, 80.0), KeyModifiers::NONE);
    assert!(fixture.session.borrow().selection().is_empty());
    fixture.session.borrow_mut().select([target]);
    fixture.press_with(Point::new(200.0, 100.0), KeyModifiers::NONE);
    assert!(fixture.session.borrow().selection().is_empty());
    let mut cancel = KeyEventArgs::new();
    cancel.key = Key::Escape;
    cancel.set_routed_event(Some(InputElement::key_down_event()));
    fixture.surface.raise_event(&cancel);
    assert_eq!(fixture.session.borrow().selection(), [target]);
    fixture.release(Point::new(250.0, 140.0));
    assert_eq!(fixture.session.borrow().selection(), [target]);
    assert!(fixture.commits.borrow().is_empty());
    assert_eq!(fixture.source(), SOURCE);
}

#[test]
fn shift_deselect_is_not_accidentally_a_drag_of_the_remaining_selection() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let fixture = SurfaceFixture::new();
    let target = fixture
        .session
        .borrow()
        .document()
        .ast()
        .find_name("Target")
        .unwrap();
    fixture.session.borrow_mut().select([target]);
    fixture.press_with(Point::new(60.0, 40.0), KeyModifiers::SHIFT);
    fixture.release_with(Point::new(100.0, 80.0), KeyModifiers::SHIFT);
    assert!(fixture.session.borrow().selection().is_empty());
    assert!(fixture.commits.borrow().is_empty());
    assert_eq!(fixture.source(), SOURCE);
}

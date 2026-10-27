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
            KeyModifiers::ALT,
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
            KeyModifiers::ALT,
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

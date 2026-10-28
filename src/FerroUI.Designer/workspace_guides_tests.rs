use crate::{
    CanvasGuideAxis, CanvasGuideSettings, CanvasGuidesEditor, CanvasViewport, DesignerWorkspace,
};
use ferroui_base::input::{
    IPointer, InputElement, Key, KeyEventArgs, KeyModifiers, MouseButton, Pointer,
    PointerEventArgs, PointerPointProperties, PointerPressedEventArgs, PointerReleasedEventArgs,
    PointerType, PointerUpdateKind, RawInputModifiers,
};
use ferroui_base::{Point, Ref, Size, Vector};
use ferroui_controls::platform::ITopLevelImpl;
use ferroui_controls::testing::{
    MockWindowImpl, MockWindowingPlatform, TestServices, UnitTestApplication,
};
use ferroui_controls::{Control, Window};
use std::rc::Rc;

const SOURCE: &str = r#"<Canvas xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"><Canvas.Resources><ControlTemplate x:Key="Template" TargetType="Button"><Border><ContentPresenter Content="{TemplateBinding Content}"/></Border></ControlTemplate></Canvas.Resources><Button Name="Target" Canvas.Left="120" Canvas.Top="80" Width="100" Height="40" Content="Target"/></Canvas>"#;

fn services() -> TestServices {
    let mut services = TestServices::styled_window();
    let focus = TestServices::real_focus();
    services.keyboard_device = focus.keyboard_device;
    services.keyboard_navigation = focus.keyboard_navigation;
    services.input_manager = focus.input_manager;
    services.windowing_platform = Some(MockWindowingPlatform::with_window_impl(|| {
        let native = MockWindowImpl::window(1400.0, 950.0);
        native.max_auto_size_hint.set(Size::new(2048.0, 2048.0));
        native.setup_resize(|window, size, reason| {
            if window.client_size.get() != size {
                window.client_size.set(size);
                if let Some(resized) = ITopLevelImpl::resized(window) {
                    resized(size, reason);
                }
            }
        });
        native
    }));
    services
}
struct Fixture {
    workspace: Rc<DesignerWorkspace>,
    window: Ref<Window>,
    pointer: Rc<Pointer>,
}
impl Fixture {
    fn new() -> Self {
        let workspace =
            DesignerWorkspace::with_uri(SOURCE, "memory://guides-one.xaml".into()).unwrap();
        workspace.set_timeline_visible(false);
        workspace.set_layout_helpers_visible(false);
        let window = Window::new();
        window.set_width(1400.0);
        window.set_height(950.0);
        window.set_content(Some(Control::boxed(workspace.control())));
        window.show();
        window.update_layout();
        workspace.session().borrow_mut().refresh_runtime_mapping();
        Self {
            workspace,
            window,
            pointer: Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true),
        }
    }
    fn canvas(&self) -> Ref<Control> {
        self.workspace.control().get_control("ArtboardScroll")
    }
    fn position(&self, point: Point) -> Point {
        self.canvas().translate_point(point, &self.window).unwrap()
    }
    fn press(&self, point: Point) {
        self.press_on(&self.canvas(), point);
    }
    fn press_on(&self, source: &Control, point: Point) {
        let event = PointerPressedEventArgs::new(
            source.to_ref(),
            self.pointer.clone(),
            &self.window,
            self.position(point),
            1,
            PointerPointProperties::new(
                RawInputModifiers::LEFT_MOUSE_BUTTON,
                PointerUpdateKind::LeftButtonPressed,
            ),
            KeyModifiers::NONE,
            1,
        );
        source.raise_event(&event);
    }
    fn motion(&self, point: Point) {
        let source = self
            .pointer
            .captured()
            .and_then(|input| input.cast::<Control>())
            .unwrap_or_else(|| self.canvas());
        source.raise_event(&PointerEventArgs::new(
            Some(InputElement::pointer_moved_event()),
            source.to_ref(),
            self.pointer.clone(),
            Some(&self.window),
            self.position(point),
            2,
            PointerPointProperties::new(
                RawInputModifiers::LEFT_MOUSE_BUTTON,
                PointerUpdateKind::Other,
            ),
            KeyModifiers::NONE,
        ));
    }
    fn release(&self, point: Point) {
        let source = self
            .pointer
            .captured()
            .and_then(|input| input.cast::<Control>())
            .unwrap_or_else(|| self.canvas());
        source.raise_event(&PointerReleasedEventArgs::new(
            source.to_ref(),
            self.pointer.clone(),
            &self.window,
            self.position(point),
            3,
            PointerPointProperties::new(
                RawInputModifiers::NONE,
                PointerUpdateKind::LeftButtonReleased,
            ),
            KeyModifiers::NONE,
            MouseButton::Left,
        ));
    }
    fn key(&self, key: Key, modifiers: KeyModifiers) -> bool {
        let mut event = KeyEventArgs::new();
        event.key = key;
        event.key_modifiers = modifiers;
        event.set_routed_event(Some(InputElement::key_down_event()));
        event.set_source(self.canvas().to_ref());
        self.canvas().raise_event(&event);
        event.handled()
    }
    fn rulers(&self) {
        let mut settings = self.workspace.canvas_guides();
        settings.rulers_visible = true;
        settings.snap_to_grid = false;
        self.workspace.set_canvas_guides(settings).unwrap();
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.window.close();
    }
}

#[test]
fn native_guide_drag_uses_camera_and_commits_final_release_once_without_source_edits() {
    let _app = UnitTestApplication::start(services());
    let f = Fixture::new();
    f.rulers();
    let w = &f.workspace;
    w.set_canvas_viewport(CanvasViewport::new(2.0, Vector::new(80.0, 55.0)).unwrap());
    let session = w.session();
    let source = session.borrow().document().source().to_owned();
    let revision = session.borrow().document().revision();
    f.press(Point::new(180.0, 10.0));
    assert!(f.pointer.captured().is_some());
    f.motion(Point::new(180.0, 240.0));
    assert!(
        w.canvas_guides().guides.is_empty(),
        "Movement is a local ghost until release"
    );
    f.release(Point::new(180.0, 255.0));
    let guide = w.canvas_guides().guides[0];
    assert_eq!(guide.axis, CanvasGuideAxis::Horizontal);
    assert_eq!(guide.position, 100.0);
    assert!(f.pointer.captured().is_none());
    f.press(Point::new(260.0, 255.0));
    f.motion(Point::new(260.0, 295.0));
    f.release(Point::new(260.0, 295.0));
    assert_eq!(w.canvas_guides().guides[0].position, 120.0);
    assert_eq!(session.borrow().document().source(), source);
    assert_eq!(session.borrow().document().revision(), revision);
    assert!(!session.borrow().can_undo());
    assert!(f.key(Key::Down, KeyModifiers::SHIFT));
    assert_eq!(w.canvas_guides().guides[0].position, 130.0);
    f.press(Point::new(260.0, 315.0));
    f.release(Point::new(260.0, 10.0));
    assert!(w.canvas_guides().guides.is_empty());
}

#[test]
fn native_guide_capture_retires_on_escape_camera_resize_and_source_change() {
    let _app = UnitTestApplication::start(services());
    let f = Fixture::new();
    f.rulers();
    let w = &f.workspace;
    let original = w
        .add_canvas_guide(CanvasGuideAxis::Vertical, 200.0)
        .unwrap();
    let point = w.artboard_to_canvas(Point::new(200.0, 120.0));
    f.press(point);
    f.motion(point + Vector::new(30.0, 0.0));
    assert!(f.key(Key::Escape, KeyModifiers::NONE));
    assert_eq!(w.canvas_guides().guides[0].position, 200.0);
    f.press(point);
    f.motion(point + Vector::new(30.0, 0.0));
    w.pan_canvas(Vector::new(40.0, 20.0)).unwrap();
    assert!(f.pointer.captured().is_none());
    f.release(point);
    assert_eq!(w.canvas_guides().guides[0].position, 200.0);
    let point = w.artboard_to_canvas(Point::new(200.0, 120.0));
    f.press(point);
    f.motion(point + Vector::new(30.0, 0.0));
    f.window.set_width(1500.0);
    f.window.update_layout();
    assert!(f.pointer.captured().is_none());
    assert_eq!(w.canvas_guides().guides[0].position, 200.0);
    let point = w.artboard_to_canvas(Point::new(200.0, 120.0));
    f.press(point);
    f.motion(point + Vector::new(30.0, 0.0));
    let target = w
        .session()
        .borrow()
        .document()
        .ast()
        .find_name("Target")
        .unwrap();
    w.session()
        .borrow_mut()
        .set_property(&[target], "Content", "Changed")
        .unwrap();
    f.release(point + Vector::new(30.0, 0.0));
    assert_eq!(w.canvas_guides().guides[0].position, 200.0);
    assert_eq!(w.canvas_guides().guides[0].id, original);
}

#[test]
fn guide_settings_and_exact_editor_are_atomic_for_viewers_and_pending_source() {
    let _app = UnitTestApplication::start(services());
    let f = Fixture::new();
    let w = &f.workspace;
    let id = w
        .add_canvas_guide(CanvasGuideAxis::Vertical, 135.5)
        .unwrap();
    let before = w.canvas_guides();
    let editor = CanvasGuidesEditor::new(before.clone());
    let accepted = w.canvas_guide_snapshot();
    let weak = Rc::downgrade(w);
    editor.on_apply(move |value| {
        weak.upgrade()
            .unwrap()
            .apply_canvas_guides(&accepted, value)
    });
    editor.set_guide_position(id, "NaN").unwrap();
    assert!(editor.apply().is_err());
    assert_eq!(w.canvas_guides(), before);
    editor.set_guide_position(id, "151.75").unwrap();
    editor.apply().unwrap();
    assert_eq!(w.canvas_guides().guides[0].position, 151.75);
    editor.set_guide_position(id, "200").unwrap();
    assert!(
        editor.apply().is_err(),
        "A stale editor cannot overwrite a newer guide model"
    );
    editor.retire();
    assert!(editor.add_guide(CanvasGuideAxis::Horizontal, 0.0).is_err());
    w.source_editor().set_text(Some("<Canvas>"));
    w.session().borrow_mut().set_read_only(true);
    let session = w.session();
    let source = session.borrow().document().source().to_owned();
    let revision = session.borrow().document().revision();
    let generation = session.borrow().generation();
    w.move_canvas_guide(id, -20.0).unwrap();
    let valid = w.canvas_guides();
    for invalid in [f64::NAN, f64::INFINITY, 1_000_001.0] {
        assert!(w.move_canvas_guide(id, invalid).is_err());
        assert_eq!(w.canvas_guides(), valid);
    }
    assert_eq!(session.borrow().document().source(), source);
    assert_eq!(session.borrow().document().revision(), revision);
    assert_eq!(session.borrow().generation(), generation);
    assert_eq!(w.source_editor().text().as_deref(), Some("<Canvas>"));
}

#[test]
fn guides_survive_tabs_and_definition_roundtrips_but_not_new_documents_with_same_uri() {
    let _app = UnitTestApplication::start(services());
    let f = Fixture::new();
    let w = &f.workspace;
    w.add_canvas_guide(CanvasGuideAxis::Vertical, 90.0).unwrap();
    let document = w.canvas_guides();
    let template = w
        .session()
        .borrow()
        .component_catalog()
        .unwrap()
        .entries
        .iter()
        .find(|entry| entry.key.as_deref() == Some("Template"))
        .unwrap()
        .node;
    w.edit_component_on_canvas(template).unwrap();
    assert!(w.canvas_guides().guides.is_empty());
    w.add_canvas_guide(CanvasGuideAxis::Horizontal, 45.0)
        .unwrap();
    let definition = w.canvas_guides();
    w.return_to_document().unwrap();
    assert_eq!(w.canvas_guides(), document);
    w.edit_component_on_canvas(template).unwrap();
    assert_eq!(w.canvas_guides(), definition);
    w.open_document(SOURCE.into(), "memory://guides-two.xaml".into())
        .unwrap();
    assert!(w.canvas_guides().guides.is_empty());
    w.add_canvas_guide(CanvasGuideAxis::Vertical, 300.0)
        .unwrap();
    let second = w.canvas_guides();
    w.activate_document("memory://guides-one.xaml").unwrap();
    assert_eq!(w.canvas_guides(), document);
    w.edit_component_on_canvas(template).unwrap();
    assert_eq!(w.canvas_guides(), definition);
    w.return_to_document().unwrap();
    w.activate_document("memory://guides-two.xaml").unwrap();
    assert_eq!(w.canvas_guides(), second);
    let stale = w.canvas_guide_view_snapshot();
    w.open_document(SOURCE.into(), "memory://guides-two.xaml".into())
        .unwrap();
    assert!(w.canvas_guides().guides.is_empty());
    assert!(!w.restore_canvas_guide_view(&stale));
}

#[test]
fn authored_control_move_snaps_to_hidden_manual_guide_in_screen_tolerance() {
    let _app = UnitTestApplication::start(services());
    let f = Fixture::new();
    let w = &f.workspace;
    w.set_canvas_viewport(CanvasViewport::new(2.0, Vector::new(80.0, 55.0)).unwrap());
    let mut settings = CanvasGuideSettings {
        snap_to_grid: false,
        snap_to_objects: false,
        guides_visible: false,
        ..Default::default()
    };
    settings.add(CanvasGuideAxis::Vertical, 144.0).unwrap();
    w.set_canvas_guides(settings).unwrap();
    let start = w.artboard_to_canvas(Point::new(170.0, 100.0));
    f.press_on(&w.design_surface(), start);
    f.motion(start + Vector::new(44.0, 0.0));
    f.release(start + Vector::new(44.0, 0.0));
    let session = w.session();
    let session = session.borrow();
    let target = session.document().ast().find_name("Target").unwrap();
    assert_eq!(
        session
            .document()
            .ast()
            .node(target)
            .unwrap()
            .attribute("Canvas.Left"),
        Some("144")
    );
}

#[test]
fn guide_shortcuts_delete_only_selected_guides_and_do_not_touch_controls() {
    let _app = UnitTestApplication::start(services());
    let f = Fixture::new();
    let w = &f.workspace;
    assert!(f.key(Key::R, KeyModifiers::SHIFT));
    assert!(w.canvas_guides().rulers_visible);
    let target = w
        .session()
        .borrow()
        .document()
        .ast()
        .find_name("Target")
        .unwrap();
    w.session().borrow_mut().select([target]);
    w.add_canvas_guide(CanvasGuideAxis::Vertical, 200.0)
        .unwrap();
    assert!(f.key(Key::Delete, KeyModifiers::NONE));
    assert!(w.canvas_guides().guides.is_empty());
    assert!(
        w.session()
            .borrow()
            .document()
            .ast()
            .find_name("Target")
            .is_some()
    );
    assert!(!w.session().borrow().can_undo());
    let guide = w
        .add_canvas_guide(CanvasGuideAxis::Horizontal, 150.0)
        .unwrap();
    assert!(f.key(Key::G, KeyModifiers::SHIFT));
    assert!(!w.canvas_guides().guides_visible);
    assert_eq!(w.selected_canvas_guide(), None);
    assert_eq!(w.canvas_guides().guides[0].id, guide);
}

#[test]
fn guide_capture_and_authored_layout_drafts_cancel_on_focus_or_snap_changes() {
    let _app = UnitTestApplication::start(services());
    let f = Fixture::new();
    let w = &f.workspace;
    f.rulers();
    let id = w
        .add_canvas_guide(CanvasGuideAxis::Vertical, 200.0)
        .unwrap();
    let point = w.artboard_to_canvas(Point::new(200.0, 120.0));
    f.press(point);
    f.motion(point + Vector::new(25.0, 0.0));
    let search = w
        .control()
        .get_control::<ferroui_controls::TextBox>("PropertySearch");
    assert!(search.focus());
    assert!(f.pointer.captured().is_none());
    assert_eq!(w.selected_canvas_guide(), None);
    assert_eq!(w.canvas_guides().guides[0].position, 200.0);
    let mut event = KeyEventArgs::new();
    event.key = Key::Delete;
    event.set_routed_event(Some(InputElement::key_down_event()));
    event.set_source(search.to_ref());
    search.raise_event(&event);
    assert_eq!(
        w.canvas_guides().guides[0].id,
        id,
        "A property text input keeps its Delete key"
    );
    f.canvas().focus();
    let before = w.session().borrow().document().source().to_owned();
    let start = w.artboard_to_canvas(Point::new(170.0, 100.0));
    f.press_on(&w.design_surface(), start);
    f.motion(start + Vector::new(30.0, 0.0));
    let initial = w.canvas_guides();
    let mut changed = initial.clone();
    changed.snap_to_grid = !changed.snap_to_grid;
    w.set_canvas_guides(changed).unwrap();
    w.set_canvas_guides(initial).unwrap();
    f.release(start + Vector::new(30.0, 0.0));
    assert_eq!(w.session().borrow().document().source(), before);
    assert!(!w.session().borrow().can_undo());
}

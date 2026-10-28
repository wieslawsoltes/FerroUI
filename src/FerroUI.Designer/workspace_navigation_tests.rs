//! Native routing must preserve authored coordinates and keep view changes out of history.
use crate::{CanvasViewport, CanvasWheelMode, DesignerWorkspace};
use ferroui_base::input::{
    IPointer, InputElement, Key, KeyEventArgs, KeyModifiers, MouseButton, Pointer,
    PointerEventArgs, PointerPointProperties, PointerPressedEventArgs, PointerReleasedEventArgs,
    PointerType, PointerUpdateKind, PointerWheelEventArgs, RawInputModifiers,
};
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::{Point, Rect, Ref, Size, Thickness, Vector};
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::{Button, Control, TextBox, Window};
use std::rc::Rc;

const SOURCE: &str = r#"<Canvas xmlns="https://github.com/ferroui"><Button Name="Target" Canvas.Left="120" Canvas.Top="80" Width="100" Height="40" Content="Target"/><TextBox Name="Input" Canvas.Left="250" Canvas.Top="160" Width="120" Height="40"/></Canvas>"#;

struct Fixture {
    workspace: Rc<DesignerWorkspace>,
    window: Ref<Window>,
    pointer: Rc<Pointer>,
}
impl Fixture {
    fn new() -> Self {
        let workspace = DesignerWorkspace::with_uri(SOURCE, "memory://one.xaml".into()).unwrap();
        workspace.set_timeline_visible(false);
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
    fn wheel(
        &self,
        source: &Control,
        point: Point,
        delta: Vector,
        modifiers: KeyModifiers,
    ) -> bool {
        let event = PointerWheelEventArgs::new(
            source.to_ref(),
            self.pointer.clone(),
            &self.window,
            self.position(point),
            1,
            PointerPointProperties::new(RawInputModifiers::NONE, PointerUpdateKind::Other),
            modifiers,
            delta,
        );
        source.raise_event(&event);
        event.handled()
    }
    fn press(&self, point: Point, middle: bool) {
        let source = self.workspace.design_surface();
        let event = PointerPressedEventArgs::new(
            source.to_ref(),
            self.pointer.clone(),
            &self.window,
            self.position(point),
            2,
            PointerPointProperties::new(
                if middle {
                    RawInputModifiers::MIDDLE_MOUSE_BUTTON
                } else {
                    RawInputModifiers::LEFT_MOUSE_BUTTON
                },
                if middle {
                    PointerUpdateKind::MiddleButtonPressed
                } else {
                    PointerUpdateKind::LeftButtonPressed
                },
            ),
            KeyModifiers::NONE,
            1,
        );
        source.raise_event(&event);
        assert!(event.handled());
    }
    fn move_to(&self, point: Point, middle: bool) {
        let canvas = self
            .pointer
            .captured()
            .and_then(|input| input.cast::<Control>())
            .unwrap_or_else(|| self.canvas());
        canvas.raise_event(&PointerEventArgs::new(
            Some(InputElement::pointer_moved_event()),
            canvas.to_ref(),
            self.pointer.clone(),
            Some(&self.window),
            self.position(point),
            3,
            PointerPointProperties::new(
                if middle {
                    RawInputModifiers::MIDDLE_MOUSE_BUTTON
                } else {
                    RawInputModifiers::LEFT_MOUSE_BUTTON
                },
                PointerUpdateKind::Other,
            ),
            KeyModifiers::NONE,
        ));
    }
    fn release(&self, point: Point, middle: bool) {
        let canvas = self
            .pointer
            .captured()
            .and_then(|input| input.cast::<Control>())
            .unwrap_or_else(|| self.canvas());
        canvas.raise_event(&PointerReleasedEventArgs::new(
            canvas.to_ref(),
            self.pointer.clone(),
            &self.window,
            self.position(point),
            4,
            PointerPointProperties::new(
                RawInputModifiers::NONE,
                if middle {
                    PointerUpdateKind::MiddleButtonReleased
                } else {
                    PointerUpdateKind::LeftButtonReleased
                },
            ),
            KeyModifiers::NONE,
            if middle {
                MouseButton::Middle
            } else {
                MouseButton::Left
            },
        ));
    }
    fn key(&self, key: Key, modifiers: KeyModifiers, down: bool) -> bool {
        let mut event = KeyEventArgs::new();
        event.key = key;
        event.key_modifiers = modifiers;
        event.set_routed_event(Some(if down {
            InputElement::key_down_event()
        } else {
            InputElement::key_up_event()
        }));
        event.set_source(self.canvas().to_ref());
        self.canvas().raise_event(&event);
        event.handled()
    }
    fn click(&self, name: &str) {
        self.workspace
            .control()
            .get_control::<Button>(name)
            .raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.window.close();
    }
}

fn near(a: Point, b: Point) {
    assert!(
        (a.x - b.x).abs() < 0.000_001 && (a.y - b.y).abs() < 0.000_001,
        "{a:?} != {b:?}"
    );
}

#[test]
fn canvas_viewport_round_trips_and_preserves_the_pointer_at_zoom_limits() {
    let mut viewport = CanvasViewport::new(0.75, Vector::new(-371.25, 128.5)).unwrap();
    let anchor = Point::new(283.125, 417.875);
    let authored = viewport.viewport_to_artboard(anchor);
    for zoom in [0.8, 0.25, 0.00001, 0.1, 0.75, 20.0, 1.0] {
        viewport = viewport.zoom_at(zoom, anchor).unwrap();
        near(viewport.artboard_to_viewport(authored), anchor);
        near(
            viewport.viewport_to_artboard(viewport.artboard_to_viewport(Point::new(123.75, -48.5))),
            Point::new(123.75, -48.5),
        );
    }
    for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 0.0, -1.0] {
        assert!(viewport.zoom_at(invalid, anchor).is_err());
    }
    assert!(viewport.pan(Vector::new(f64::INFINITY, 0.0)).is_err());
    assert!(viewport.zoom_at(2.0, Point::new(f64::NAN, 0.0)).is_err());
    let fitted = CanvasViewport::fit(
        Rect::new(0.0, 0.0, 16384.0, 16384.0),
        Size::new(1000.0, 800.0),
        Thickness::uniform(40.0),
    )
    .unwrap();
    assert!(fitted.zoom() < 0.1);
    let bounds = Rect::new(0.0, 0.0, 1000.0, 800.0);
    assert!(bounds.contains(fitted.artboard_to_viewport(Point::new(0.0, 0.0))));
    assert!(bounds.contains(fitted.artboard_to_viewport(Point::new(16384.0, 16384.0))));
}

#[test]
fn canvas_wheel_preserves_fractional_deltas_and_uses_the_selected_mode() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let fixture = Fixture::new();
    let workspace = &fixture.workspace;
    let surface = workspace.design_surface().upcast::<Control>();
    let anchor = Point::new(240.5, 180.25);
    let before = workspace.canvas_to_artboard(anchor);
    let original = workspace.canvas_viewport();
    assert!(fixture.wheel(
        &surface,
        anchor,
        Vector::new(0.0, 0.25),
        KeyModifiers::CONTROL
    ));
    assert!((workspace.zoom() - original.zoom() * 1.2_f64.powf(0.25)).abs() < 1.0e-10);
    near(workspace.artboard_to_canvas(before), anchor);
    let view = workspace.canvas_viewport();
    assert!(fixture.wheel(
        &surface,
        anchor,
        Vector::new(0.5, -0.25),
        KeyModifiers::NONE
    ));
    assert_eq!(
        workspace.canvas_viewport().offset(),
        view.offset() + Vector::new(24.0, -12.0)
    );
    let view = workspace.canvas_viewport();
    assert!(fixture.wheel(
        &surface,
        anchor,
        Vector::new(0.0, -0.5),
        KeyModifiers::SHIFT
    ));
    assert_eq!(
        workspace.canvas_viewport().offset(),
        view.offset() + Vector::new(-24.0, 0.0)
    );
    workspace.set_canvas_wheel_mode(CanvasWheelMode::Zoom);
    let view = workspace.canvas_viewport();
    assert!(fixture.wheel(&surface, anchor, Vector::new(0.0, 0.5), KeyModifiers::NONE));
    assert!((workspace.zoom() - view.zoom() * 1.2_f64.powf(0.5)).abs() < 1.0e-10);
    let view = workspace.canvas_viewport();
    assert!(fixture.wheel(&surface, anchor, Vector::new(0.0, 0.5), KeyModifiers::META));
    assert_eq!(
        workspace.canvas_viewport().offset(),
        view.offset() + Vector::new(0.0, 24.0)
    );
    let view = workspace.canvas_viewport();
    assert!(!fixture.wheel(
        &surface,
        anchor,
        Vector::new(f64::NAN, 1.0),
        KeyModifiers::NONE
    ));
    assert!(!fixture.wheel(&surface, anchor, Vector::ZERO, KeyModifiers::NONE));
    assert_eq!(workspace.canvas_viewport(), view);
}

#[test]
fn canvas_pan_capture_escape_and_source_changes_never_author_geometry() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let fixture = Fixture::new();
    let workspace = &fixture.workspace;
    workspace.set_zoom(2.0);
    let view = workspace.canvas_viewport();
    let revision = workspace.session().borrow().document().revision();
    let point = Point::new(200.0, 160.0);
    fixture.press(point, true);
    fixture.move_to(point + Vector::new(35.0, -27.0), true);
    assert_eq!(
        workspace.canvas_viewport().offset(),
        view.offset() + Vector::new(35.0, -27.0)
    );
    assert!(fixture.key(Key::Escape, KeyModifiers::NONE, true));
    assert_eq!(workspace.canvas_viewport(), view);
    assert!(fixture.pointer.captured().is_none());
    assert!(fixture.key(Key::Space, KeyModifiers::NONE, true));
    fixture.press(point, false);
    fixture.move_to(point + Vector::new(-12.5, 8.5), false);
    fixture.release(point, false);
    assert_eq!(
        workspace.canvas_viewport().offset(),
        view.offset() + Vector::new(-12.5, 8.5)
    );
    // A release ends this drag, not the still-held temporary hand tool.
    fixture.press(point, false);
    fixture.move_to(point + Vector::new(8.0, 12.0), false);
    fixture.release(point, false);
    assert_eq!(
        workspace.canvas_viewport().offset(),
        view.offset() + Vector::new(-4.5, 20.5)
    );
    fixture.key(Key::Space, KeyModifiers::NONE, false);
    assert_eq!(workspace.session().borrow().document().revision(), revision);
    assert!(!workspace.session().borrow().can_undo());
    let view = workspace.canvas_viewport();
    fixture.press(point, true);
    workspace
        .session()
        .borrow_mut()
        .set_design_time_enabled(false)
        .unwrap();
    fixture.move_to(point + Vector::new(200.0, 200.0), true);
    assert_eq!(
        workspace.canvas_viewport(),
        view,
        "A changed preview scope retires captured pan input"
    );
    assert!(fixture.pointer.captured().is_none());
    fixture.click("PreviewTool");
    let view = workspace.canvas_viewport();
    fixture.press(point, true);
    fixture.move_to(point + Vector::new(20.0, 50.0), true);
    assert!(fixture.key(Key::Escape, KeyModifiers::NONE, true));
    assert_eq!(
        workspace.canvas_viewport(),
        view,
        "Escape also restores a middle-drag while interacting with application controls"
    );
}

#[test]
fn canvas_preview_routing_and_zoom_fields_preserve_pending_source() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let fixture = Fixture::new();
    let workspace = &fixture.workspace;
    fixture.click("PreviewTool");
    let session = workspace.session();
    let input = session
        .borrow()
        .document()
        .ast()
        .find_name("Input")
        .unwrap();
    let input = session
        .borrow()
        .preview
        .visual(input)
        .unwrap()
        .cast::<TextBox>()
        .unwrap();
    let view = workspace.canvas_viewport();
    fixture.wheel(
        &input,
        Point::new(200.0, 200.0),
        Vector::new(0.0, 1.0),
        KeyModifiers::NONE,
    );
    assert_eq!(workspace.canvas_viewport(), view);
    assert!(fixture.wheel(
        &input,
        Point::new(200.0, 200.0),
        Vector::new(0.0, 1.0),
        KeyModifiers::CONTROL
    ));
    fixture.click("SelectTool");
    workspace.source_editor().set_text(Some("<Canvas>"));
    let revision = session.borrow().document().revision();
    let generation = session.borrow().generation();
    let before = workspace.canvas_viewport();
    for invalid in ["0", "1601", "NaN", "inf", "100%%", "", "-3"] {
        assert!(workspace.set_zoom_percent(invalid).is_err());
        assert_eq!(workspace.canvas_viewport(), before);
    }
    workspace.set_zoom_percent(" 137.5 % ").unwrap();
    assert_eq!(workspace.zoom(), 1.375);
    for name in [
        "ZoomIn",
        "ZoomOut",
        "ZoomReset",
        "ZoomFit",
        "PanTool",
        "SelectTool",
        "PreviewTool",
    ] {
        fixture.click(name);
    }
    assert_eq!(
        workspace.source_editor().text().as_deref(),
        Some("<Canvas>")
    );
    assert_eq!(session.borrow().document().revision(), revision);
    assert_eq!(session.borrow().generation(), generation);
    assert!(!session.borrow().can_undo());
}

#[test]
fn canvas_resize_and_retained_tabs_keep_their_own_view() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let fixture = Fixture::new();
    let workspace = &fixture.workspace;
    workspace.set_canvas_viewport(CanvasViewport::new(2.0, Vector::new(-150.0, 35.0)).unwrap());
    let old_size = fixture.canvas().bounds().size();
    let center =
        workspace.canvas_to_artboard(Point::new(old_size.width / 2.0, old_size.height / 2.0));
    fixture.window.set_width(1540.0);
    fixture.window.set_height(1020.0);
    fixture.window.update_layout();
    let size = fixture.canvas().bounds().size();
    near(
        workspace.canvas_to_artboard(Point::new(size.width / 2.0, size.height / 2.0)),
        center,
    );
    let first = workspace.canvas_viewport();
    workspace
        .open_document(SOURCE.into(), "memory://two.xaml".into())
        .unwrap();
    fixture.window.update_layout();
    workspace.set_canvas_viewport(CanvasViewport::new(0.25, Vector::new(33.0, 77.0)).unwrap());
    let second = workspace.canvas_viewport();
    workspace.activate_document("memory://one.xaml").unwrap();
    assert_eq!(workspace.canvas_viewport(), first);
    workspace.activate_document("memory://two.xaml").unwrap();
    assert_eq!(workspace.canvas_viewport(), second);
}

#[test]
fn resizing_the_canvas_retires_an_authored_drag_before_its_inverse_coordinates_change() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let fixture = Fixture::new();
    let workspace = &fixture.workspace;
    let session = workspace.session();
    let source = session.borrow().document().source().to_owned();
    let point = workspace.artboard_to_canvas(Point::new(170.0, 100.0));
    fixture.press(point, false);
    assert_eq!(
        fixture.pointer.captured().as_ref(),
        Some(&workspace.design_surface().upcast::<InputElement>())
    );
    fixture.move_to(point + Vector::new(20.0, 10.0), false);
    fixture.window.set_width(1560.0);
    fixture.window.update_layout();
    fixture.release(point + Vector::new(20.0, 10.0), false);
    assert_eq!(session.borrow().document().source(), source);
    assert!(
        !session.borrow().can_undo(),
        "Resize must retire authored coordinates rather than commit a shifted release point"
    );
}

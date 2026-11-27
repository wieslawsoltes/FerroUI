//! Tests of the interactive 3D elements: `UIElement3D`, `ModelUIElement3D` and
//! `ContainerUIElement3D` in a `Viewport3D` shown by a window of the unit test application, with
//! the pointer events of the input system raised on the viewport as the input system raises
//! them (enter, moves, a press with the implicit capture of the pointer, a release that ends it,
//! exit).

use crate::*;
use ferroui_base::input::{
    CaptureSource, IPointer, InputElement, KeyModifiers, MouseButton, Pointer, PointerEventArgs,
    PointerPointProperties, PointerPressedEventArgs, PointerReleasedEventArgs, PointerType, PointerUpdateKind,
    RawInputModifiers,
};
use ferroui_base::interactivity::RoutedEvent;
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::media::{Brushes, IBrush};
use ferroui_base::{Point, Ref, Visual};
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_controls::{Control, Window};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

fn start() -> UnitTestApplicationScope {
    Control::reset_loaded_queue_for_unit_tests();
    register_types();
    UnitTestApplication::start(TestServices::styled_window())
}

fn material(brush: Rc<dyn IBrush>) -> Ref<Material> {
    DiffuseMaterial::with_brush(Some(brush)).upcast()
}

/// A square from (x0, y0) to (x1, y1) in the plane `z`, facing +z.
fn square(x0: f64, y0: f64, x1: f64, y1: f64, z: f64) -> Ref<GeometryModel3D> {
    let mesh = MeshGeometry3D::new();
    mesh.set_positions(
        Point3DCollection::parse(&format!("{x0},{y0},{z} {x1},{y0},{z} {x1},{y1},{z} {x0},{y1},{z}")).unwrap(),
    );
    mesh.set_triangle_indices(Int32Collection::parse("0 1 2 0 2 3").unwrap());
    GeometryModel3D::with_geometry_and_material(mesh.upcast(), material(Brushes::red()))
}

/// The scene of the tests, in a window: an orthographic camera of width 4 over a viewport of 100
/// by 100 pixels (x and y from -2 to 2, 25 pixels a unit). A container holds a left square
/// (pixels 5 to 45 across) and a right one (55 to 95), each a `ModelUIElement3D`; the gap
/// between them (pixel 50) is empty.
struct Scene {
    window: Ref<Window>,
    viewport: Ref<Viewport3D>,
    container: Ref<ContainerUIElement3D>,
    left: Ref<ModelUIElement3D>,
    right: Ref<ModelUIElement3D>,
}

fn scene() -> Scene {
    let viewport = Viewport3D::new();
    viewport.set_width(100.0);
    viewport.set_height(100.0);
    viewport.set_horizontal_alignment(HorizontalAlignment::Left);
    viewport.set_vertical_alignment(VerticalAlignment::Top);
    viewport.set_camera(Some(
        OrthographicCamera::with_values(
            Point3D::new(0.0, 0.0, 5.0),
            Vector3D::new(0.0, 0.0, -1.0),
            Vector3D::new(0.0, 1.0, 0.0),
            4.0,
        )
        .upcast(),
    ));
    let left = ModelUIElement3D::with_model(square(-1.8, -1.0, -0.2, 1.0, 0.0).upcast());
    let right = ModelUIElement3D::with_model(square(0.2, -1.0, 1.8, 1.0, 0.0).upcast());
    let container = ContainerUIElement3D::new();
    container.children().add(left.clone().upcast());
    container.children().add(right.clone().upcast());
    viewport.children().add(container.clone().upcast());
    let window = Window::new();
    window.set_content(Some(Control::boxed(viewport.clone())));
    window.show();
    window.update_layout();
    assert_eq!(viewport.bounds().width, 100.0, "the viewport is arranged");
    Scene { window, viewport, container, left, right }
}

const LEFT: Point = Point::new(25.0, 50.0);
const RIGHT: Point = Point::new(75.0, 50.0);
const GAP: Point = Point::new(50.0, 50.0);

/// The pointer of the tests: raises the 2D pointer events on the viewport at points of the
/// viewport, as the input system raises them.
struct Mouse {
    pointer: Rc<Pointer>,
    stamp: Cell<u64>,
}

impl Mouse {
    fn new() -> Self {
        Self { pointer: Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true), stamp: Cell::new(1) }
    }

    fn pointer(&self) -> Rc<dyn IPointer> {
        self.pointer.clone()
    }

    fn timestamp(&self) -> u64 {
        let stamp = self.stamp.get();
        self.stamp.set(stamp + 1);
        stamp
    }

    fn in_root(scene: &Scene, point: Point) -> Point {
        let root: &Visual = &scene.window;
        scene.viewport.translate_point(point, root).expect("the viewport is in the window")
    }

    fn event(
        &self,
        scene: &Scene,
        event: &RoutedEvent<PointerEventArgs>,
        point: Point,
        buttons: RawInputModifiers,
    ) -> PointerEventArgs {
        let root: &Visual = &scene.window;
        PointerEventArgs::new(
            Some(event),
            scene.viewport.clone(),
            self.pointer(),
            Some(root),
            Self::in_root(scene, point),
            self.timestamp(),
            PointerPointProperties::new(buttons, PointerUpdateKind::Other),
            KeyModifiers::NONE,
        )
    }

    fn enter(&self, scene: &Scene, point: Point) {
        scene.viewport.raise_event(&self.event(
            scene,
            InputElement::pointer_entered_event(),
            point,
            RawInputModifiers::NONE,
        ));
    }

    fn exit(&self, scene: &Scene, point: Point) {
        scene.viewport.raise_event(&self.event(
            scene,
            InputElement::pointer_exited_event(),
            point,
            RawInputModifiers::NONE,
        ));
    }

    /// A move; whether the 2D event was handled.
    fn move_to(&self, scene: &Scene, point: Point) -> bool {
        let e = self.event(scene, InputElement::pointer_moved_event(), point, RawInputModifiers::NONE);
        scene.viewport.raise_event(&e);
        e.handled()
    }

    /// A press of the left button, with the implicit capture of the pointer by the viewport;
    /// whether the 2D event was handled.
    fn press(&self, scene: &Scene, point: Point) -> bool {
        let input: Ref<InputElement> = scene.viewport.clone().upcast();
        self.pointer.capture_with_source(Some(&input), CaptureSource::Implicit);
        let root: &Visual = &scene.window;
        let e = PointerPressedEventArgs::new(
            scene.viewport.clone(),
            self.pointer(),
            root,
            Self::in_root(scene, point),
            self.timestamp(),
            PointerPointProperties::new(RawInputModifiers::LEFT_MOUSE_BUTTON, PointerUpdateKind::LeftButtonPressed),
            KeyModifiers::NONE,
            1,
        );
        scene.viewport.raise_event(&e);
        e.handled()
    }

    /// A release of the left button; the capture of the pointer ends after it, as the input
    /// system ends it.
    fn release(&self, scene: &Scene, point: Point) -> bool {
        let root: &Visual = &scene.window;
        let e = PointerReleasedEventArgs::new(
            scene.viewport.clone(),
            self.pointer(),
            root,
            Self::in_root(scene, point),
            self.timestamp(),
            PointerPointProperties::new(RawInputModifiers::NONE, PointerUpdateKind::LeftButtonReleased),
            KeyModifiers::NONE,
            MouseButton::Left,
        );
        scene.viewport.raise_event(&e);
        self.pointer.capture_with_source(None, CaptureSource::Implicit);
        e.handled()
    }
}

/// What the handlers of the tests saw: an event name, the element whose handler ran and the
/// source of the event.
type Log = Rc<RefCell<Vec<String>>>;

fn name_of(element: &Ref<UIElement3D>, scene: &Scene) -> &'static str {
    if element.ptr_eq(&scene.left) {
        "left"
    } else if element.ptr_eq(&scene.right) {
        "right"
    } else if element.ptr_eq(&scene.container) {
        "container"
    } else {
        "other"
    }
}

/// Logs every pointer event of `element` as `event element<-source`.
fn log_events(scene: &Rc<Scene>, element: &UIElement3D, log: &Log) -> Vec<ferroui_base::reactive::Subscription> {
    let mut subscriptions = Vec::new();
    type Add = fn(
        &UIElement3D,
        Box<dyn Fn(&Ref<UIElement3D>, &Pointer3DEventArgs<'_>)>,
    ) -> ferroui_base::reactive::Subscription;
    let events: [(&str, Add); 6] = [
        ("entered", |e, h| e.pointer_entered(h)),
        ("exited", |e, h| e.pointer_exited(h)),
        ("moved", |e, h| e.pointer_moved(h)),
        ("pressed", |e, h| e.pointer_pressed(h)),
        ("released", |e, h| e.pointer_released(h)),
        ("lost", |e, h| e.pointer_capture_lost(h)),
    ];
    for (name, add) in events {
        let (scene, log) = (Rc::downgrade(scene), log.clone());
        subscriptions.push(add(
            element,
            Box::new(move |this, args| {
                let scene = scene.upgrade().unwrap();
                log.borrow_mut().push(format!("{name} {}<-{}", name_of(this, &scene), name_of(args.source(), &scene)));
            }),
        ));
    }
    subscriptions
}

fn take(log: &Log) -> Vec<String> {
    std::mem::take(&mut *log.borrow_mut())
}

#[test]
fn the_viewport_is_hit_in_2d_only_over_its_models() {
    let _app = start();
    let scene = scene();
    let visual: &Visual = &scene.viewport;
    assert_eq!(visual.custom_hit_test(LEFT), Some(true));
    assert_eq!(visual.custom_hit_test(RIGHT), Some(true));
    assert_eq!(visual.custom_hit_test(GAP), Some(false));
    assert_eq!(visual.custom_hit_test(Point::new(150.0, 50.0)), Some(false));
}

#[test]
fn a_press_reaches_the_element_under_the_pointer_and_bubbles_to_its_container() {
    let _app = start();
    let scene = Rc::new(scene());
    let log: Log = Rc::default();
    let _left = log_events(&scene, &scene.left, &log);
    let _right = log_events(&scene, &scene.right, &log);
    let _container = log_events(&scene, &scene.container, &log);
    let mouse = Mouse::new();

    // The events carry the hit under the pointer.
    let hit_model = Rc::new(RefCell::new(None));
    let seen = hit_model.clone();
    let _hit = scene.left.pointer_pressed(move |_, args| {
        *seen.borrow_mut() = args.hit().map(|hit| hit.model_hit().clone());
        let position = args.position();
        assert!((position.x - 25.0).abs() < 1e-9 && (position.y - 50.0).abs() < 1e-9, "{position:?}");
    });

    mouse.enter(&scene, LEFT);
    assert_eq!(take(&log), ["entered container<-container", "entered left<-left"]);
    assert!(scene.left.is_pointer_over() && scene.container.is_pointer_over() && !scene.right.is_pointer_over());
    assert!(!mouse.press(&scene, LEFT));
    assert_eq!(take(&log), ["pressed left<-left", "pressed container<-left"]);
    let model = hit_model.borrow().clone().expect("the press carries the hit");
    assert!(scene.left.model().unwrap().ptr_eq(&model));
    assert!(!mouse.release(&scene, LEFT));
    assert_eq!(take(&log), ["released left<-left", "released container<-left"]);

    // A press on the other square goes to it.
    mouse.move_to(&scene, RIGHT);
    assert_eq!(
        take(&log),
        ["exited left<-left", "entered right<-right", "moved right<-right", "moved container<-right"]
    );
    mouse.press(&scene, RIGHT);
    mouse.release(&scene, RIGHT);
    assert_eq!(
        take(&log),
        ["pressed right<-right", "pressed container<-right", "released right<-right", "released container<-right"]
    );

    // A handler that handles the press stops it there, and the 2D event is handled.
    let _handled = scene.right.pointer_pressed(|_, args| args.set_handled(true));
    assert!(mouse.press(&scene, RIGHT));
    assert_eq!(take(&log), ["pressed right<-right"]);
    mouse.release(&scene, RIGHT);
    take(&log);

    // A press in the gap reaches no element.
    mouse.move_to(&scene, GAP);
    assert_eq!(take(&log), ["exited right<-right", "exited container<-container"]);
    assert!(!mouse.press(&scene, GAP));
    mouse.release(&scene, GAP);
    assert!(take(&log).is_empty());
}

#[test]
fn entered_and_exited_follow_the_element_under_the_pointer() {
    let _app = start();
    let scene = Rc::new(scene());
    let log: Log = Rc::default();
    let _left = log_events(&scene, &scene.left, &log);
    let _right = log_events(&scene, &scene.right, &log);
    let _container = log_events(&scene, &scene.container, &log);
    let mouse = Mouse::new();

    mouse.enter(&scene, GAP);
    assert!(take(&log).is_empty());
    mouse.move_to(&scene, LEFT);
    assert_eq!(
        take(&log),
        ["entered container<-container", "entered left<-left", "moved left<-left", "moved container<-left"]
    );
    mouse.move_to(&scene, Point::new(30.0, 40.0));
    assert_eq!(take(&log), ["moved left<-left", "moved container<-left"]);
    mouse.move_to(&scene, RIGHT);
    assert_eq!(
        take(&log),
        ["exited left<-left", "entered right<-right", "moved right<-right", "moved container<-right"]
    );
    assert!(scene.right.is_pointer_over() && !scene.left.is_pointer_over() && scene.container.is_pointer_over());
    // The pointer leaves the viewport: it is over nothing.
    mouse.exit(&scene, Point::new(120.0, 50.0));
    assert_eq!(take(&log), ["exited right<-right", "exited container<-container"]);
    assert!(!scene.right.is_pointer_over() && !scene.container.is_pointer_over());
}

#[test]
fn a_captured_pointer_goes_to_the_capturing_element_until_the_capture_ends() {
    let _app = start();
    let scene = Rc::new(scene());
    let log: Log = Rc::default();
    let _left = log_events(&scene, &scene.left, &log);
    let _right = log_events(&scene, &scene.right, &log);
    let mouse = Mouse::new();
    // Not routed to yet: the element has no viewport to capture through.
    assert!(!scene.left.capture_pointer(&mouse.pointer()));

    let captured = Rc::new(Cell::new(false));
    let result = captured.clone();
    let _capture = scene.left.pointer_pressed(move |this, args| result.set(this.capture_pointer(args.pointer())));
    mouse.move_to(&scene, LEFT);
    mouse.press(&scene, LEFT);
    assert!(captured.get());
    assert!(scene.left.is_pointer_captured() && !scene.right.is_pointer_captured());
    assert!(scene.viewport.viewport_3d_visual().snapshot().models.len() == 2);
    take(&log);

    // Over the right square, the moves go to the left one, which the pointer is no longer over;
    // the right one is not entered while the left one captures.
    let hit_of_move = Rc::new(RefCell::new(None));
    let seen = hit_of_move.clone();
    let _moves =
        scene.left.pointer_moved(move |_, args| *seen.borrow_mut() = args.hit().map(|hit| hit.model_hit().clone()));
    mouse.move_to(&scene, RIGHT);
    assert_eq!(take(&log), ["exited left<-left", "moved left<-left"]);
    assert!(!scene.left.is_pointer_over() && !scene.right.is_pointer_over());
    let model = hit_of_move.borrow().clone().expect("the move carries the hit under the pointer");
    assert!(scene.right.model().unwrap().ptr_eq(&model));
    mouse.move_to(&scene, LEFT);
    assert_eq!(take(&log), ["entered left<-left", "moved left<-left"]);

    // The release goes to the left square; the end of the capture of the pointer ends that of
    // the element.
    mouse.move_to(&scene, RIGHT);
    take(&log);
    mouse.release(&scene, RIGHT);
    assert_eq!(take(&log), ["released left<-left", "lost left<-left"]);
    assert!(!scene.left.is_pointer_captured());
    // Uncaptured, the next move enters the right square.
    mouse.move_to(&scene, RIGHT);
    assert_eq!(take(&log), ["entered right<-right", "moved right<-right"]);

    // An explicit release: the capture lost event, and the viewport releases the pointer.
    mouse.press(&scene, LEFT);
    assert!(scene.left.is_pointer_captured());
    take(&log);
    scene.left.release_pointer_capture(&mouse.pointer());
    assert_eq!(take(&log), ["lost left<-left"]);
    assert!(!scene.left.is_pointer_captured());
    assert!(mouse.pointer.captured().is_none());
    // Capturing another element takes the capture from the first.
    assert!(scene.left.capture_pointer(&mouse.pointer()));
    assert!(scene.right.capture_pointer(&mouse.pointer()));
    assert_eq!(take(&log), ["lost left<-left"]);
    assert!(scene.right.is_pointer_captured() && !scene.left.is_pointer_captured());
    // The release at the right square: the pointer is over the capturing element again.
    mouse.release(&scene, RIGHT);
    assert_eq!(take(&log), ["exited left<-left", "entered right<-right", "released right<-right", "lost right<-right"]);
}

#[test]
fn a_plain_visual_in_front_takes_no_input_and_hides_what_is_behind_unless_it_is_not_hit_test_visible() {
    let _app = start();
    let scene = Rc::new(scene());
    let log: Log = Rc::default();
    let _left = log_events(&scene, &scene.left, &log);
    let mouse = Mouse::new();

    // A plain visual in front of the left square: the nearest hit is in no element.
    let front = ModelVisual3D::with_content(square(-1.5, -0.5, -0.5, 0.5, 1.0).upcast());
    scene.viewport.children().add(front.clone().upcast());
    mouse.move_to(&scene, LEFT);
    mouse.press(&scene, LEFT);
    mouse.release(&scene, LEFT);
    assert!(take(&log).is_empty());
    // Beside it, the left square is reached.
    mouse.move_to(&scene, Point::new(10.0, 50.0));
    assert_eq!(take(&log), ["entered left<-left", "moved left<-left"]);

    // An element in front that is not visible to hit testing lets the pointer through.
    scene.viewport.children().remove(&front.clone().upcast());
    let glass = ModelUIElement3D::with_model(square(-1.5, -0.5, -0.5, 0.5, 1.0).upcast());
    glass.set_is_hit_test_visible(false);
    scene.viewport.children().add(glass.clone().upcast());
    let _glass = log_events(&scene, &glass, &log);
    mouse.move_to(&scene, LEFT);
    assert_eq!(take(&log), ["moved left<-left"]);
    let hit = VisualTreeHelper::hit_test(&*scene.viewport, LEFT).unwrap();
    assert!(hit.model_hit().ptr_eq(&scene.left.model().unwrap()));
    // Visible to hit testing, it takes the pointer.
    glass.set_is_hit_test_visible(true);
    mouse.move_to(&scene, LEFT);
    assert_eq!(take(&log), ["exited left<-left", "entered other<-other", "moved other<-other"]);
}

#[test]
fn model_and_container_elements_draw_their_models() {
    let _app = start();
    let scene = scene();
    let visual = scene.viewport.viewport_3d_visual();
    let snapshot = visual.snapshot();
    assert_eq!(snapshot.models.len(), 2);
    let (source_visual, _) = visual.model_source(0).unwrap();
    assert!(source_visual.ptr_eq(&scene.left));
    // A new model of an element, and a new child of a container, are changes of the structure.
    let structure = visual.structure_generation();
    scene.left.set_model(Some(square(-1.0, -1.0, 0.0, 0.0, 0.0).upcast()));
    assert_eq!(visual.structure_generation(), structure + 1);
    scene.container.children().add(ModelUIElement3D::with_model(square(0.0, 1.2, 1.0, 1.8, 0.0).upcast()).upcast());
    assert_eq!(visual.structure_generation(), structure + 2);
    assert_eq!(visual.snapshot().models.len(), 3);
    // The bounds of the scene hold the elements' models.
    let bounds = visual.descendant_bounds();
    assert!((bounds.x() - -1.0).abs() < 1e-9 && (bounds.x() + bounds.size_x() - 1.8).abs() < 1e-9);
    // A change of the transform of an element is a change of a record.
    let record = visual.record_generation();
    scene.right.set_transform(Some(TranslateTransform3D::with_offset_xyz(0.0, 0.1, 0.0).upcast()));
    assert_eq!(visual.record_generation(), record + 1);
}

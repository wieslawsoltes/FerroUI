//! Port of `Pages/PointerCanvas.cs`.

use ferroui_base::data::BindingMode;
use ferroui_base::input::{
    InputElement, InputElementImpl, InputElementImplExt, PointerCaptureLostEventArgs, PointerEventArgs,
    PointerPointProperties, PointerPressedEventArgs, PointerReleasedEventArgs, PointerType, PointerUpdateKind,
};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::{Brushes, DrawingContext, IBrush, IPen, Pen, PenLineCap, PenLineJoin};
use ferroui_base::reactive::IDisposable;
use ferroui_base::threading::{Dispatcher, DispatcherPriority, DispatcherTimer};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, DirectProperty,
    DirectPropertyMetadata, FerroObjectImpl, FerroProperty, Point, Ref, StyledElementImpl, Visual, VisualImpl,
    VisualImplExt, VisualTreeAttachmentEventArgs,
};
use ferroui_controls::{Control, ControlImpl};
use std::cell::{Cell, RefCell};
use std::fmt::Display;
use std::rc::Rc;
use std::time::Duration;

#[derive(Clone, Default)]
struct CanvasPoint {
    brush: Option<Rc<dyn IBrush>>,
    point: Point,
    radius: f64,
    pressure: Option<f64>,
}

/// `PointerCanvas.PointerPoints`: the last thousand points of a pointer.
struct PointerPoints {
    points: Vec<CanvasPoint>,
    index: usize,
}

impl PointerPoints {
    fn new() -> Self {
        Self { points: vec![CanvasPoint::default(); 1000], index: 0 }
    }

    fn render(&self, context: &mut DrawingContext, draw_points: bool) {
        let mut prev: Option<&CanvasPoint> = None;
        for c in 0..self.points.len() {
            let i = (c + self.index) % self.points.len();
            let pt = &self.points[i];
            let pressure = pt.pressure.or(prev.and_then(|prev| prev.pressure)).unwrap_or(0.5);
            let thickness = pressure * 10.0;
            let radius = pressure * pt.radius;

            if draw_points {
                if let Some(brush) = &pt.brush {
                    context.draw_ellipse_at(Some(brush), None, pt.point, radius, radius);
                }
            } else if let Some(prev) = prev {
                if prev.brush.is_some() && pt.brush.is_some() && prev.pressure.is_some() && pt.pressure.is_some() {
                    let black: Rc<dyn IBrush> = Brushes::black();
                    let line_pen: Rc<dyn IPen> =
                        Pen::with_all(Some(black), thickness, None, PenLineCap::Round, PenLineJoin::Round, 10.0).into();
                    context.draw_line(&line_pen, prev.point, pt.point);
                }
            }
            prev = Some(pt);
        }
    }

    fn add_point(&mut self, pt: Point, brush: Rc<dyn IBrush>, radius: f64, pressure: Option<f32>) {
        self.points[self.index] =
            CanvasPoint { point: pt, brush: Some(brush), radius, pressure: pressure.map(f64::from) };
        self.index = (self.index + 1) % self.points.len();
    }

    fn handle_event(&mut self, e: &PointerEventArgs, v: &Visual) {
        e.set_handled(true);
        e.prevent_gesture_recognition();
        let current_point = e.get_current_point(Some(v));
        if e.routed_event() == Some(InputElement::pointer_pressed_event().as_routed_event()) {
            self.add_point(current_point.position, Brushes::green(), 10.0, None);
        } else if e.routed_event() == Some(InputElement::pointer_released_event().as_routed_event()) {
            self.add_point(current_point.position, Brushes::red(), 10.0, None);
        } else {
            let pts = e.get_intermediate_points(Some(v));
            for (c, pt) in pts.iter().enumerate() {
                let last = c == pts.len() - 1;
                let brush: Rc<dyn IBrush> = if last { Brushes::blue() } else { Brushes::black() };
                self.add_point(pt.position, brush, if last { 5.0 } else { 2.0 }, Some(pt.properties.pressure));
            }
        }
    }
}

#[repr(C)]
pub struct PointerCanvas {
    base: Control,
    /// The time the stopwatch of the control was (re)started at, in the
    /// milliseconds of the clock of the dispatcher.
    stopwatch: Cell<i64>,
    events: Cell<i32>,
    status_updated: RefCell<Option<Rc<dyn IDisposable>>>,
    /// The points of each pointer by its identifier, in the order the
    /// pointers were first seen.
    pointers: RefCell<Vec<(i32, PointerPoints)>>,
    last_properties: Cell<Option<PointerPointProperties>>,
    last_non_other_update_kind: Cell<Option<PointerUpdateKind>>,
    thread_sleep: Cell<i32>,
    draw_only_points: Cell<bool>,
    status: RefCell<Option<String>>,
}

ferro_class!(PointerCanvas: Control);
ferro_impl_classes!(PointerCanvas: FerroObjectImpl, StyledElementImpl, LayoutableImpl, InteractiveImpl, ControlImpl);
ferro_class_info!(PointerCanvas { new: PointerCanvas::new });

ferro_properties! {
    impl PointerCanvas {
        pub fn thread_sleep_property() -> DirectProperty<PointerCanvas, i32> {
            FerroProperty::register_direct::<PointerCanvas, _>(
                "ThreadSleep",
                |c| c.thread_sleep(),
                Some(|c: &PointerCanvas, v| c.set_thread_sleep(v)),
                0,
            )
        }

        pub fn draw_only_points_property() -> DirectProperty<PointerCanvas, bool> {
            FerroProperty::register_direct::<PointerCanvas, _>(
                "DrawOnlyPoints",
                |c| c.draw_only_points(),
                Some(|c: &PointerCanvas, v| c.set_draw_only_points(v)),
                false,
            )
        }

        pub fn status_property() -> DirectProperty<PointerCanvas, Option<String>> {
            FerroProperty::register_direct_with::<PointerCanvas, _>(
                "Status",
                |c| c.status(),
                Some(|c: &PointerCanvas, v| c.set_status(v)),
                DirectPropertyMetadata::new(Some(None)).with_default_binding_mode(BindingMode::TwoWay),
            )
        }
    }
}

/// The text of a value that may be absent: empty when it is.
fn text<T: Display>(value: Option<T>) -> String {
    value.map(|value| value.to_string()).unwrap_or_default()
}

/// The text of a flag as the managed original writes it.
fn flag(value: bool) -> &'static str {
    if value {
        "True"
    } else {
        "False"
    }
}

impl VisualImpl for PointerCanvas {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);

        let weak = this.to_ref().downgrade();
        let status_updated = DispatcherTimer::run(
            move || {
                let Some(this) = weak.upgrade() else { return true };
                let elapsed_milliseconds = (Dispatcher::ui_thread().now() - this.stopwatch.get()) as f64;
                if elapsed_milliseconds > 250.0 {
                    let last_properties = this.last_properties.get();
                    let events_per_second = f64::from(this.events.get()) / (elapsed_milliseconds / 1000.0);
                    this.set_status(Some(format!(
                        "Events per second: {}\n\
                         PointerUpdateKind: {}\n\
                         Last PointerUpdateKind != Other: {}\n\
                         IsLeftButtonPressed: {}\n\
                         IsRightButtonPressed: {}\n\
                         IsMiddleButtonPressed: {}\n\
                         IsXButton1Pressed: {}\n\
                         IsXButton2Pressed: {}\n\
                         IsBarrelButtonPressed: {}\n\
                         IsEraser: {}\n\
                         IsInverted: {}\n\
                         Pressure: {}\n\
                         XTilt: {}\n\
                         YTilt: {}\n\
                         Twist: {}",
                        events_per_second,
                        text(last_properties.map(|p| format!("{:?}", p.pointer_update_kind))),
                        text(this.last_non_other_update_kind.get().map(|kind| format!("{kind:?}"))),
                        text(last_properties.map(|p| flag(p.is_left_button_pressed))),
                        text(last_properties.map(|p| flag(p.is_right_button_pressed))),
                        text(last_properties.map(|p| flag(p.is_middle_button_pressed))),
                        text(last_properties.map(|p| flag(p.is_x_button_1_pressed))),
                        text(last_properties.map(|p| flag(p.is_x_button_2_pressed))),
                        text(last_properties.map(|p| flag(p.is_barrel_button_pressed))),
                        text(last_properties.map(|p| flag(p.is_eraser))),
                        text(last_properties.map(|p| flag(p.is_inverted))),
                        text(last_properties.map(|p| p.pressure)),
                        text(last_properties.map(|p| p.x_tilt)),
                        text(last_properties.map(|p| p.y_tilt)),
                        text(last_properties.map(|p| p.twist)),
                    )));
                    this.stopwatch.set(Dispatcher::ui_thread().now());
                    this.events.set(0);
                }

                true
            },
            Duration::from_millis(10),
            DispatcherPriority::DEFAULT,
        );
        *this.status_updated.borrow_mut() = Some(status_updated);
    }

    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_detached_from_visual_tree(this, e);

        let status_updated = this.status_updated.borrow().clone();
        if let Some(status_updated) = status_updated {
            status_updated.dispose();
        }
    }

    fn render(this: &Self, context: &mut DrawingContext) {
        let white: Rc<dyn IBrush> = Brushes::white();
        context.fill_rectangle(&white, this.bounds(), 0.0);
        let draw_only_points = this.draw_only_points.get();
        for (_, pt) in this.pointers.borrow().iter() {
            pt.render(context, draw_only_points);
        }
        Self::parent_render(this, context);
    }
}

impl InputElementImpl for PointerCanvas {
    fn on_pointer_pressed(this: &Self, e: &PointerPressedEventArgs) {
        if e.click_count() == 2 {
            this.pointers.borrow_mut().clear();
            this.invalidate_visual();
            return;
        }

        this.handle_event(e);
        Self::parent_on_pointer_pressed(this, e);
    }

    fn on_pointer_moved(this: &Self, e: &PointerEventArgs) {
        this.handle_event(e);
        Self::parent_on_pointer_moved(this, e);
    }

    fn on_pointer_released(this: &Self, e: &PointerReleasedEventArgs) {
        this.handle_event(e);
        Self::parent_on_pointer_released(this, e);
    }

    fn on_pointer_capture_lost(this: &Self, e: &PointerCaptureLostEventArgs) {
        this.last_properties.set(None);
        Self::parent_on_pointer_capture_lost(this, e);
    }
}

impl PointerCanvas {
    pub fn construct() -> Self {
        Self {
            base: Control::construct(),
            stopwatch: Cell::new(Dispatcher::ui_thread().now()),
            events: Cell::new(0),
            status_updated: RefCell::new(None),
            pointers: RefCell::new(Vec::new()),
            last_properties: Cell::new(None),
            last_non_other_update_kind: Cell::new(None),
            thread_sleep: Cell::new(0),
            draw_only_points: Cell::new(false),
            status: RefCell::new(None),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    pub fn thread_sleep(&self) -> i32 {
        self.thread_sleep.get()
    }

    pub fn set_thread_sleep(&self, value: i32) {
        self.set_and_raise_cell(Self::thread_sleep_property(), &self.thread_sleep, value);
    }

    pub fn draw_only_points(&self) -> bool {
        self.draw_only_points.get()
    }

    pub fn set_draw_only_points(&self, value: bool) {
        self.set_and_raise_cell(Self::draw_only_points_property(), &self.draw_only_points, value);
    }

    pub fn status(&self) -> Option<String> {
        self.status.borrow().clone()
    }

    pub fn set_status(&self, value: Option<String>) {
        self.set_and_raise(Self::status_property(), &self.status, value);
    }

    /// # Panics
    /// Panics if `ThreadSleep` is negative.
    fn handle_event(&self, e: &PointerEventArgs) {
        self.events.set(self.events.get() + 1);

        if self.thread_sleep.get() != 0 {
            let milliseconds = u64::try_from(self.thread_sleep.get()).expect("the time to sleep is not negative");
            std::thread::sleep(Duration::from_millis(milliseconds));
        }
        self.invalidate_visual();

        let last_pointer = e.get_current_point(Some(self));
        self.last_properties.set(Some(last_pointer.properties));

        if last_pointer.properties.pointer_update_kind != PointerUpdateKind::Other {
            self.last_non_other_update_kind.set(Some(last_pointer.properties.pointer_update_kind));
        }

        let id = e.pointer().id();
        if e.routed_event() == Some(InputElement::pointer_released_event().as_routed_event())
            && e.pointer().type_() == PointerType::Touch
        {
            self.pointers.borrow_mut().retain(|(pointer, _)| *pointer != id);
            return;
        }

        if e.pointer().type_() != PointerType::Pen || last_pointer.properties.pressure > 0.0 {
            let mut pointers = self.pointers.borrow_mut();
            let index = match pointers.iter().position(|(pointer, _)| *pointer == id) {
                Some(index) => index,
                None => {
                    pointers.push((id, PointerPoints::new()));
                    pointers.len() - 1
                }
            };
            pointers[index].1.handle_event(e, self);
        }
    }
}

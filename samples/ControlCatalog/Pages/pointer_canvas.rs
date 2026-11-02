//! Port of `Pages/PointerCanvas.cs`.

use ferroui_base::input::{
    InputElementImpl, InputElementImplExt, PointerCaptureLostEventArgs, PointerEventArgs, PointerPointProperties,
    PointerPressedEventArgs, PointerReleasedEventArgs, PointerUpdateKind,
};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::immutable::{ImmutablePen, ImmutableSolidColorBrush};
use ferroui_base::media::{Brushes, Colors, DrawingContext, IBrush, ImmediateDrawingContext};
use ferroui_base::reactive::Subscription;
use ferroui_base::rendering::composition::{
    CompositionCustomVisual, CompositionCustomVisualHandler, ElementComposition, ICompositionCustomVisualHandler,
};
use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, DirectProperty, FerroObjectImpl,
    FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Matrix, Point, Rect, Ref, StyledElementImpl,
    StyledProperty, Vector, Visual, VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs,
};
use ferroui_controls::{Control, ControlImpl, TopLevel};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::rc::Rc;
use std::sync::{Arc, LazyLock, Mutex, PoisonError};
use std::time::Duration;

const MAX_POINTS: usize = 20000;
const CONTACT_RADIUS: f64 = 12.0;
const LOAD_PERIOD: Duration = Duration::from_millis(100);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum PointKind {
    #[default]
    Pressed,
    Intermediate,
    Primary,
    Released,
}

#[derive(Clone, Copy, Debug, Default)]
struct CanvasPoint {
    kind: PointKind,
    position: Point,
    in_contact: bool,
}

// Messages to the render thread handler.

/// The points of one input event (upstream's `CanvasPoint[]` message).
struct PointsMessage(Vec<CanvasPoint>);

/// The position of a pointer in contact, or `None` once the contact ends.
struct ContactMessage {
    pointer_id: i32,
    position: Option<Point>,
}

/// The clear message: the identity of the object is the message, as upstream's
/// `static readonly object` instance is.
struct ClearMessage;

type HandlerMessage = Arc<dyn Any + Send + Sync>;

static CLEAR_MESSAGE: LazyLock<HandlerMessage> = LazyLock::new(|| Arc::new(ClearMessage));

/// Upstream's `Stopwatch`, as far as the canvas uses it: `std::time::Instant`, which
/// the browser does not have; there the clock of the dispatcher, in whole milliseconds.
#[derive(Clone, Copy)]
struct Stopwatch {
    #[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
    started: std::time::Instant,
    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    started: i64,
}

#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
impl Stopwatch {
    fn start_new() -> Self {
        Self { started: std::time::Instant::now() }
    }

    fn elapsed_milliseconds(&self) -> f64 {
        self.started.elapsed().as_secs_f64() * 1000.0
    }
}

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
impl Stopwatch {
    fn start_new() -> Self {
        Self { started: ferroui_base::threading::Dispatcher::ui_thread().now() }
    }

    fn elapsed_milliseconds(&self) -> f64 {
        (ferroui_base::threading::Dispatcher::ui_thread().now() - self.started) as f64
    }
}

/// Frame statistics written by the render thread and collected by the UI thread.
#[derive(Default)]
struct FrameStats {
    frames: i32,
    events: i32,
    max_events_per_frame: i32,
}

/// Locks the frame statistics; a panic of the other thread while it held them leaves
/// them usable.
fn lock_frame_stats(frame_stats: &Mutex<FrameStats>) -> std::sync::MutexGuard<'_, FrameStats> {
    frame_stats.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Draws every pointer point it receives, including hover and intermediate points,
/// with input rate, batching and delay statistics.
#[repr(C)]
pub struct PointerCanvas {
    base: Control,
    contacts: RefCell<HashSet<i32>>,
    points_visual: RefCell<Option<CompositionCustomVisual>>,
    frame_stats: Arc<Mutex<FrameStats>>,
    top_level: RefCell<Option<Ref<TopLevel>>>,

    stopwatch: Cell<Stopwatch>,
    clock: Stopwatch,
    events: Cell<i32>,
    points: Cell<i32>,
    max_batch: Cell<i32>,
    delay_sum: Cell<f64>,
    max_delay: Cell<f64>,
    min_clock_offset: Cell<Option<f64>>,
    last_properties: Cell<Option<PointerPointProperties>>,
    last_non_other_update_kind: Cell<Option<PointerUpdateKind>>,
    status_timer: RefCell<Option<Subscription>>,
    load_timer: RefCell<Option<Rc<DispatcherTimer>>>,

    status: RefCell<Option<String>>,
    status_details: RefCell<Option<String>>,
}

ferro_class!(PointerCanvas: Control);
ferro_impl_classes!(PointerCanvas: StyledElementImpl, LayoutableImpl, InteractiveImpl, ControlImpl);
ferro_class_info!(PointerCanvas { new: PointerCanvas::new });

ferro_properties! {
    impl PointerCanvas {
        pub fn thread_sleep_property() -> StyledProperty<i32> {
            FerroProperty::register::<PointerCanvas, _>("ThreadSleep", 0)
        }

        pub fn load_work_property() -> StyledProperty<i32> {
            FerroProperty::register::<PointerCanvas, _>("LoadWork", 0)
        }

        /// One-line summary: input and frame rates, average delay.
        pub fn status_property() -> DirectProperty<PointerCanvas, Option<String>> {
            FerroProperty::register_direct::<PointerCanvas, _>("Status", PointerCanvas::status, None, None)
        }

        /// Batching and frame details plus the last pointer's properties, always the same
        /// number of lines.
        pub fn status_details_property() -> DirectProperty<PointerCanvas, Option<String>> {
            FerroProperty::register_direct::<PointerCanvas, _>(
                "StatusDetails",
                PointerCanvas::status_details,
                None,
                None,
            )
        }
    }
}

impl FerroObjectImpl for PointerCanvas {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if change.property() == Self::load_work_property().as_property() {
            this.update_load_timer();
        } else if change.property() == Visual::bounds_property().as_property() {
            this.update_points_visual_size();
        }
    }
}

impl VisualImpl for PointerCanvas {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);
        *this.top_level.borrow_mut() = TopLevel::get_top_level(Some(this));

        if let Some(compositor) = ElementComposition::get_element_visual(this).map(|visual| visual.compositor().clone())
        {
            let frame_stats = this.frame_stats.clone();
            let points_visual = compositor.create_custom_visual(move || {
                Rc::new(PointsHandler::new(frame_stats)) as Rc<dyn ICompositionCustomVisualHandler>
            });
            *this.points_visual.borrow_mut() = Some(points_visual.clone());
            this.update_points_visual_size();
            ElementComposition::set_element_child_visual(this, Some((*points_visual).clone()));
        }

        this.stopwatch.set(Stopwatch::start_new());
        let weak = this.to_ref().downgrade();
        let status_timer = DispatcherTimer::run(
            move || {
                if let Some(this) = weak.upgrade() {
                    this.update_status();
                }
                true
            },
            Duration::from_millis(250),
            DispatcherPriority::DEFAULT,
        );
        *this.status_timer.borrow_mut() = Some(status_timer);
        this.update_status();
        this.update_load_timer();
    }

    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_detached_from_visual_tree(this, e);
        ElementComposition::set_element_child_visual(this, None);
        *this.points_visual.borrow_mut() = None;
        *this.top_level.borrow_mut() = None;
        this.contacts.borrow_mut().clear();

        let status_timer = this.status_timer.borrow_mut().take();
        if let Some(status_timer) = status_timer {
            status_timer.dispose();
        }
        this.update_load_timer();
    }

    fn render(this: &Self, context: &mut DrawingContext) {
        let white: Rc<dyn IBrush> = Brushes::white();
        context.fill_rectangle(&white, Rect::from_size(this.bounds().size()), 0.0);
    }
}

impl InputElementImpl for PointerCanvas {
    fn on_pointer_pressed(this: &Self, e: &PointerPressedEventArgs) {
        Self::parent_on_pointer_pressed(this, e);
        if e.click_count() == 2 {
            this.clear();
            return;
        }

        this.handle_event(e, PointKind::Pressed);
    }

    fn on_pointer_moved(this: &Self, e: &PointerEventArgs) {
        Self::parent_on_pointer_moved(this, e);
        this.handle_event(e, PointKind::Primary);
    }

    fn on_pointer_released(this: &Self, e: &PointerReleasedEventArgs) {
        Self::parent_on_pointer_released(this, e);
        this.handle_event(e, PointKind::Released);
    }

    fn on_pointer_capture_lost(this: &Self, e: &PointerCaptureLostEventArgs) {
        Self::parent_on_pointer_capture_lost(this, e);
        this.remove_contact(e.pointer().id());
    }
}

impl PointerCanvas {
    pub fn construct() -> Self {
        Self {
            base: Control::construct(),
            contacts: RefCell::new(HashSet::new()),
            points_visual: RefCell::new(None),
            frame_stats: Arc::new(Mutex::new(FrameStats::default())),
            top_level: RefCell::new(None),
            stopwatch: Cell::new(Stopwatch::start_new()),
            clock: Stopwatch::start_new(),
            events: Cell::new(0),
            points: Cell::new(0),
            max_batch: Cell::new(0),
            delay_sum: Cell::new(0.0),
            max_delay: Cell::new(0.0),
            min_clock_offset: Cell::new(None),
            last_properties: Cell::new(None),
            last_non_other_update_kind: Cell::new(None),
            status_timer: RefCell::new(None),
            load_timer: RefCell::new(None),
            status: RefCell::new(None),
            status_details: RefCell::new(None),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    pub fn thread_sleep(&self) -> i32 {
        self.get_value(Self::thread_sleep_property())
    }

    pub fn set_thread_sleep(&self, value: i32) {
        self.set_value(Self::thread_sleep_property(), value)
    }

    pub fn load_work(&self) -> i32 {
        self.get_value(Self::load_work_property())
    }

    pub fn set_load_work(&self, value: i32) {
        self.set_value(Self::load_work_property(), value)
    }

    pub fn status(&self) -> Option<String> {
        self.status.borrow().clone()
    }

    fn set_status(&self, value: Option<String>) {
        self.set_and_raise(Self::status_property(), &self.status, value);
    }

    pub fn status_details(&self) -> Option<String> {
        self.status_details.borrow().clone()
    }

    fn set_status_details(&self, value: Option<String>) {
        self.set_and_raise(Self::status_details_property(), &self.status_details, value);
    }

    /// The size of the points visual follows the bounds of the control.
    fn update_points_visual_size(&self) {
        let points_visual = self.points_visual.borrow().clone();
        if let Some(points_visual) = points_visual {
            let bounds = self.bounds();
            points_visual.visual_props().set_size(&**points_visual, Vector::new(bounds.width, bounds.height));
        }
    }

    fn send_handler_message(&self, message: HandlerMessage) {
        let points_visual = self.points_visual.borrow().clone();
        if let Some(points_visual) = points_visual {
            points_visual.send_handler_message(message);
        }
    }

    pub fn clear(&self) {
        self.contacts.borrow_mut().clear();
        self.send_handler_message((*CLEAR_MESSAGE).clone());
        self.min_clock_offset.set(None);
    }

    fn handle_event(&self, e: &PointerEventArgs, kind: PointKind) {
        e.set_handled(true);
        e.prevent_gesture_recognition();

        self.events.set(self.events.get() + 1);
        self.track_delay(e.timestamp());

        let thread_sleep = self.thread_sleep();
        if thread_sleep > 0 {
            std::thread::sleep(Duration::from_millis(u64::from(thread_sleep.unsigned_abs())));
        }

        // Positions come in top-level coordinates and are transformed once per event.
        let to_canvas = self
            .top_level
            .borrow()
            .as_ref()
            .and_then(|top_level| top_level.transform_to_visual(self))
            .unwrap_or(Matrix::IDENTITY);
        let current = e.get_current_point(None);
        let position = current.position.transform(to_canvas);
        self.last_properties.set(Some(current.properties));
        if current.properties.pointer_update_kind != PointerUpdateKind::Other {
            self.last_non_other_update_kind.set(Some(current.properties.pointer_update_kind));
        }

        let id = e.pointer().id();
        let points: Vec<CanvasPoint> = if kind == PointKind::Primary {
            let in_contact = Self::is_in_contact(&current.properties);
            let raw_points = e.get_intermediate_points(None);
            let count = raw_points.len();
            let points: Vec<CanvasPoint> = raw_points
                .iter()
                .enumerate()
                .map(|(i, raw_point)| CanvasPoint {
                    kind: if i == count - 1 { PointKind::Primary } else { PointKind::Intermediate },
                    position: raw_point.position.transform(to_canvas),
                    in_contact,
                })
                .collect();

            if self.contacts.borrow().contains(&id) {
                self.send_handler_message(Arc::new(ContactMessage { pointer_id: id, position: Some(position) }));
            }
            points
        } else {
            // The release point still ends the stroke, even though no button is pressed anymore.
            let points = vec![CanvasPoint { kind, position, in_contact: true }];

            if kind == PointKind::Pressed {
                self.contacts.borrow_mut().insert(id);
                self.send_handler_message(Arc::new(ContactMessage { pointer_id: id, position: Some(position) }));
            } else {
                self.remove_contact(id);
            }
            points
        };

        let count = i32::try_from(points.len()).unwrap_or(i32::MAX);
        self.send_handler_message(Arc::new(PointsMessage(points)));
        self.points.set(self.points.get() + count);
        self.max_batch.set(self.max_batch.get().max(count));
    }

    fn remove_contact(&self, pointer_id: i32) {
        let removed = self.contacts.borrow_mut().remove(&pointer_id);
        if removed {
            self.send_handler_message(Arc::new(ContactMessage { pointer_id, position: None }));
        }
    }

    fn is_in_contact(properties: &PointerPointProperties) -> bool {
        properties.is_left_button_pressed || properties.is_right_button_pressed || properties.is_middle_button_pressed
    }

    /// Event timestamps come from a platform clock that can't be compared with ours directly,
    /// so the delay is measured relative to the fastest delivered event seen so far.
    fn track_delay(&self, timestamp: u64) {
        let offset = self.clock.elapsed_milliseconds() - timestamp as f64;
        let min = match self.min_clock_offset.get() {
            Some(min) if offset >= min => min,
            _ => {
                self.min_clock_offset.set(Some(offset));
                offset
            }
        };

        let delay = offset - min;
        self.delay_sum.set(self.delay_sum.get() + delay);
        self.max_delay.set(self.max_delay.get().max(delay));
    }

    fn update_load_timer(&self) {
        let load_timer = self.load_timer.borrow_mut().take();
        if let Some(load_timer) = load_timer {
            load_timer.stop();
        }
        let load_work = self.load_work();
        if load_work <= 0 || self.status_timer.borrow().is_none() {
            return;
        }

        // Leave a gap in every period, so the load can't lock the UI completely.
        let work_milliseconds = f64::from(load_work).min(LOAD_PERIOD.as_secs_f64() * 1000.0 - 10.0);
        let load_timer = DispatcherTimer::with_callback(LOAD_PERIOD, DispatcherPriority::DEFAULT, move |_| {
            // Busy-wait instead of sleeping: the thread can't sleep on the single-threaded browser.
            let sw = Stopwatch::start_new();
            while sw.elapsed_milliseconds() < work_milliseconds {
                for _ in 0..100 {
                    std::hint::spin_loop();
                }
            }
        });
        load_timer.start();
        *self.load_timer.borrow_mut() = Some(load_timer);
    }

    fn update_status(&self) {
        let seconds = (self.stopwatch.get().elapsed_milliseconds() / 1000.0).max(0.001);
        let (frames, frame_events, max_events_per_frame) = {
            let mut frame_stats = lock_frame_stats(&self.frame_stats);
            let stats = (frame_stats.frames, frame_stats.events, frame_stats.max_events_per_frame);
            *frame_stats = FrameStats::default();
            stats
        };

        let events = self.events.get();
        let points = self.points.get();
        let avg_delay = if events == 0 { 0.0 } else { self.delay_sum.get() / f64::from(events) };
        self.set_status(Some(format!(
            "{:.0} events/s · {:.0} points/s · {:.0} frames/s · delay {:.1} ms",
            f64::from(events) / seconds,
            f64::from(points) / seconds,
            f64::from(frames) / seconds,
            avg_delay
        )));

        let mut details = format!(
            "Points per event: avg {:.1}, max {}\nEvents per frame: avg {:.1}, max {} · delay max {:.1} ms",
            if events == 0 { 0.0 } else { f64::from(points) / f64::from(events) },
            self.max_batch.get(),
            if frames == 0 { 0.0 } else { f64::from(frame_events) / f64::from(frames) },
            max_events_per_frame,
            self.max_delay.get()
        );
        if let Some(p) = self.last_properties.get() {
            details += &format!(
                "\nUpdate: {:?} (last non-Other: {}) · buttons: {} · pen: {}\nPressure {:.3} · tilt {:.0}/{:.0} · twist {:.0}",
                p.pointer_update_kind,
                self.last_non_other_update_kind.get().map(|kind| format!("{kind:?}")).unwrap_or_default(),
                format_flags(&[
                    (p.is_left_button_pressed, "Left"),
                    (p.is_right_button_pressed, "Right"),
                    (p.is_middle_button_pressed, "Middle"),
                    (p.is_x_button_1_pressed, "X1"),
                    (p.is_x_button_2_pressed, "X2"),
                    (p.is_barrel_button_pressed, "Barrel"),
                ]),
                format_flags(&[(p.is_eraser, "Eraser"), (p.is_inverted, "Inverted")]),
                p.pressure,
                p.x_tilt,
                p.y_tilt,
                p.twist
            );
        } else {
            details += "\nUpdate: – · buttons: – · pen: –\nPressure – · tilt – · twist –";
        }
        self.set_status_details(Some(details));

        self.stopwatch.set(Stopwatch::start_new());
        self.events.set(0);
        self.points.set(0);
        self.max_batch.set(0);
        self.delay_sum.set(0.0);
        self.max_delay.set(0.0);
    }
}

/// The names of the flags that are set, separated by commas, or "none".
fn format_flags(flags: &[(bool, &str)]) -> String {
    let names: Vec<&str> = flags.iter().filter(|(is_set, _)| *is_set).map(|(_, name)| *name).collect();
    if names.is_empty() {
        "none".to_owned()
    } else {
        names.join(", ")
    }
}

/// Runs on the render thread: keeps the point history and redraws only invalidated areas.
struct PointsHandler {
    base: CompositionCustomVisualHandler,
    frame_stats: Arc<Mutex<FrameStats>>,
    hover_brush: ImmutableSolidColorBrush,
    pressed_brush: ImmutableSolidColorBrush,
    released_brush: ImmutableSolidColorBrush,
    primary_brush: ImmutableSolidColorBrush,
    intermediate_brush: ImmutableSolidColorBrush,
    contact_pens: Vec<ImmutablePen>,

    // All pointers share one buffer, the oldest points are overwritten first.
    buffer: RefCell<Vec<CanvasPoint>>,
    next: Cell<usize>,
    count: Cell<usize>,
    /// The contacts in the order they started.
    contacts: RefCell<Vec<(i32, Point)>>,
    events_this_frame: Cell<i32>,
}

impl PointsHandler {
    fn new(frame_stats: Arc<Mutex<FrameStats>>) -> Self {
        let contact_pens = [
            Colors::DODGER_BLUE,
            Colors::CRIMSON,
            Colors::FOREST_GREEN,
            Colors::ORANGE,
            Colors::MEDIUM_PURPLE,
            Colors::TEAL,
            Colors::GOLDENROD,
        ]
        .iter()
        .map(|color| ImmutablePen::from_uint32(color.to_uint32(), 2.0))
        .collect();
        Self {
            base: CompositionCustomVisualHandler::new(),
            frame_stats,
            hover_brush: ImmutableSolidColorBrush::new(Colors::LIGHT_GRAY),
            pressed_brush: ImmutableSolidColorBrush::new(Colors::GREEN),
            released_brush: ImmutableSolidColorBrush::new(Colors::RED),
            primary_brush: ImmutableSolidColorBrush::new(Colors::BLUE),
            intermediate_brush: ImmutableSolidColorBrush::new(Colors::BLACK),
            contact_pens,
            buffer: RefCell::new(vec![CanvasPoint::default(); MAX_POINTS]),
            next: Cell::new(0),
            count: Cell::new(0),
            contacts: RefCell::new(Vec::new()),
            events_this_frame: Cell::new(0),
        }
    }

    fn add_points(&self, points: &[CanvasPoint]) {
        {
            let mut buffer = self.buffer.borrow_mut();
            for point in points {
                buffer[self.next.get()] = *point;
                self.next.set((self.next.get() + 1) % MAX_POINTS);
                self.count.set((self.count.get() + 1).min(MAX_POINTS));
            }
        }

        self.base.invalidate();
    }

    // Hover points are faint; in contact: green press, red release, blue primary, black intermediate.
    fn get_style(&self, point: &CanvasPoint) -> (&ImmutableSolidColorBrush, f64) {
        if !point.in_contact {
            return (&self.hover_brush, 1.5);
        }
        match point.kind {
            PointKind::Pressed => (&self.pressed_brush, 5.0),
            PointKind::Released => (&self.released_brush, 5.0),
            PointKind::Primary => (&self.primary_brush, 3.0),
            PointKind::Intermediate => (&self.intermediate_brush, 1.5),
        }
    }
}

impl ICompositionCustomVisualHandler for PointsHandler {
    fn handler_base(&self) -> &CompositionCustomVisualHandler {
        &self.base
    }

    fn on_message(&self, message: HandlerMessage) {
        if let Some(PointsMessage(points)) = message.downcast_ref::<PointsMessage>() {
            // One points message per input event; count the frames that had input.
            let events_this_frame = self.events_this_frame.get();
            self.events_this_frame.set(events_this_frame + 1);
            if events_this_frame == 0 {
                self.base.register_for_next_animation_frame_update();
            }
            self.add_points(points);
        } else if let Some(contact) = message.downcast_ref::<ContactMessage>() {
            let mut contacts = self.contacts.borrow_mut();
            let index = contacts.iter().position(|(id, _)| *id == contact.pointer_id);
            if let Some(position) = contact.position {
                match index {
                    Some(index) => contacts[index].1 = position,
                    None => contacts.push((contact.pointer_id, position)),
                }
                drop(contacts);
                self.base.invalidate();
            } else if let Some(index) = index {
                contacts.remove(index);
                drop(contacts);
                self.base.invalidate();
            }
        } else if Arc::ptr_eq(&message, &*CLEAR_MESSAGE) {
            self.next.set(0);
            self.count.set(0);
            self.contacts.borrow_mut().clear();
            self.base.invalidate();
        }
    }

    fn on_animation_frame_update(&self) {
        {
            let mut frame_stats = lock_frame_stats(&self.frame_stats);
            let events_this_frame = self.events_this_frame.get();
            frame_stats.frames += 1;
            frame_stats.events += events_this_frame;
            frame_stats.max_events_per_frame = frame_stats.max_events_per_frame.max(events_this_frame);
        }
        self.events_this_frame.set(0);
    }

    fn on_render(&self, context: &mut ImmediateDrawingContext<'_>) {
        let count = self.count.get();
        let start = (self.next.get() + MAX_POINTS - count) % MAX_POINTS;
        let buffer = self.buffer.borrow();
        for i in 0..count {
            let point = &buffer[(start + i) % MAX_POINTS];
            let (brush, radius) = self.get_style(point);
            context.draw_ellipse(Some(brush), None, point.position, radius, radius);
        }

        for (id, position) in self.contacts.borrow().iter() {
            let pen = &self.contact_pens[(id & i32::MAX) as usize % self.contact_pens.len()];
            context.draw_ellipse(None, Some(pen), *position, CONTACT_RADIUS, CONTACT_RADIUS);
        }
    }
}

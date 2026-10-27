//! Native timeline track and ruler. Both share one time axis, so labels, keyframes
//! and scrubbing remain aligned while zooming. A drag edits a ghost marker and
//! commits once on release; capture loss or Escape leaves authored keys intact.
use ferroui_base::input::{
    IPointer, InputElement, InputElementImpl, InputElementImplExt, Key, KeyEventArgs, KeyModifiers,
    MouseButton, PointerCaptureLostEventArgs, PointerEventArgs, PointerPressedEventArgs,
    PointerReleasedEventArgs, PointerWheelEventArgs,
};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::{
    BoxShadows, DrawingContext, FlowDirection, FormattedText, IBrush, IPen, Pen, SolidColorBrush,
    Typeface,
};
use ferroui_base::utilities::CultureInfo;
use ferroui_base::{
    FerroObjectImpl, Matrix, Point, Rect, Ref, StyledElementImpl, VisualImpl, ferro_class,
    ferro_class_info, ferro_impl_classes, instantiate,
};
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::{Control, ControlImpl, ToolTip};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const INSET: f64 = 12.0;
const MIN_DURATION: f64 = 0.0000001;
type TimeAction = Rc<dyn Fn(f64)>;
type MoveAction = Rc<dyn Fn(f64, f64)>;
type SelectionAction = Rc<dyn Fn(Vec<f64>, Option<f64>, bool)>;
type BatchMoveAction = Rc<dyn Fn(Vec<f64>, f64)>;
type BatchAction = Rc<dyn Fn(Vec<f64>)>;

#[derive(Clone)]
enum Drag {
    Key {
        from: f64,
        to: f64,
        start_x: f64,
        keys: Vec<f64>,
        previous: Vec<f64>,
        previous_primary: Option<f64>,
        additive: bool,
    },
    Marquee {
        from: f64,
        to: f64,
        start_x: f64,
        base: Vec<f64>,
        previous: Vec<f64>,
        previous_primary: Option<f64>,
        additive: bool,
    },
    Scrub,
}
#[repr(C)]
pub struct TimelineLane {
    base: Control,
    keys: RefCell<Vec<f64>>,
    duration: Cell<f64>,
    view_start: Cell<f64>,
    view_end: Cell<f64>,
    playhead: Cell<f64>,
    ruler: Cell<bool>,
    highlighted: Cell<bool>,
    drag: RefCell<Option<Drag>>,
    pointer: RefCell<Option<Rc<dyn IPointer>>>,
    selected: Cell<Option<f64>>,
    selected_keys: RefCell<Vec<f64>>,
    selection_anchor: Cell<Option<f64>>,
    frame_rate: Cell<Option<u32>>,
    selection_changed: RefCell<Option<SelectionAction>>,
    move_selection: RefCell<Option<BatchMoveAction>>,
    remove_selection: RefCell<Option<BatchAction>>,
    duplicate_selection: RefCell<Option<BatchMoveAction>>,
    hover: Cell<Option<f64>>,
    activate: RefCell<Option<TimeAction>>,
    move_key: RefCell<Option<MoveAction>>,
    remove_key: RefCell<Option<TimeAction>>,
    insert_key: RefCell<Option<TimeAction>>,
    zoom: RefCell<Option<MoveAction>>,
    pan: RefCell<Option<TimeAction>>,
}
ferro_class!(TimelineLane:Control);
ferro_class_info!(TimelineLane {
    new: TimelineLane::new
});
ferro_impl_classes!(TimelineLane:FerroObjectImpl,StyledElementImpl,LayoutableImpl,InteractiveImpl,ControlImpl);
impl TimelineLane {
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self {
            base: Control::construct(),
            keys: RefCell::new(vec![]),
            duration: Cell::new(1.0),
            view_start: Cell::new(0.0),
            view_end: Cell::new(1.0),
            playhead: Cell::new(0.0),
            ruler: Cell::new(false),
            highlighted: Cell::new(false),
            drag: RefCell::new(None),
            pointer: RefCell::new(None),
            selected: Cell::new(None),
            selected_keys: RefCell::new(Vec::new()),
            selection_anchor: Cell::new(None),
            frame_rate: Cell::new(None),
            selection_changed: RefCell::new(None),
            move_selection: RefCell::new(None),
            remove_selection: RefCell::new(None),
            duplicate_selection: RefCell::new(None),
            hover: Cell::new(None),
            activate: RefCell::new(None),
            move_key: RefCell::new(None),
            remove_key: RefCell::new(None),
            insert_key: RefCell::new(None),
            zoom: RefCell::new(None),
            pan: RefCell::new(None),
        });
        this.set_height(28.0);
        this.set_min_width(120.0);
        this.set_focusable(true);
        AutomationProperties::set_name(&this, Some("Animation keyframe track"));
        ToolTip::set_tip(&this, Some(Rc::new("Drag selected keys to retime together. Ctrl+click toggles; Shift+click selects a range. Drag empty space to box-select. Double-click adds a key. Arrows nudge; Delete removes; Ctrl+D duplicates; Ctrl+A selects all. Ctrl+wheel zooms; Shift+wheel pans.".to_owned())));
        this
    }
    pub fn ruler() -> Ref<Self> {
        let this = Self::new();
        this.ruler.set(true);
        this.set_height(34.0);
        AutomationProperties::set_name(&this, Some("Animation timeline ruler"));
        ToolTip::set_tip(&this, Some(Rc::new("Drag to scrub. Ctrl+wheel zooms around the pointer; Shift+wheel pans. Home and End seek to timeline boundaries.".to_owned())));
        this
    }
    pub fn is_ruler(&self) -> bool {
        self.ruler.get()
    }
    /// The visible time interval, shared by the ruler and every track in a host.
    pub fn view_range(&self) -> (f64, f64) {
        (self.view_start.get(), self.view_end.get())
    }
    pub fn configure(&self, duration: f64, mut keys: Vec<f64>) {
        self.cancel_drag();
        let duration = if duration.is_finite() {
            duration.max(MIN_DURATION)
        } else {
            1.0
        };
        keys.retain(|time| time.is_finite() && (0.0..=duration).contains(time));
        keys.sort_by(f64::total_cmp);
        keys.dedup();
        self.duration.set(duration);
        self.view_start.set(0.0);
        self.view_end.set(duration);
        self.selected
            .set(self.selected.get().filter(|time| keys.contains(time)));
        self.selected_keys
            .borrow_mut()
            .retain(|time| keys.binary_search_by(|key| key.total_cmp(time)).is_ok());
        self.selection_anchor.set(
            self.selection_anchor
                .get()
                .filter(|time| keys.contains(time)),
        );
        *self.keys.borrow_mut() = keys;
        self.set_playhead(self.playhead.get());
        self.invalidate_visual();
    }
    pub fn set_view(&self, start: f64, end: f64) {
        self.cancel_drag();
        let duration = self.duration.get();
        if start.is_finite() && end.is_finite() && start >= 0.0 && end > start && end <= duration {
            self.view_start.set(start);
            self.view_end.set(end);
        } else {
            self.view_start.set(0.0);
            self.view_end.set(duration);
        }
        self.invalidate_visual();
    }
    pub fn set_playhead(&self, time: f64) {
        let time = if time.is_finite() {
            time.clamp(0.0, self.duration.get())
        } else {
            0.0
        };
        if self.playhead.replace(time) != time {
            self.invalidate_visual();
        }
    }
    pub fn set_selected(&self, time: Option<f64>) {
        self.set_selected_keys(time.into_iter().collect(), time);
    }
    /// Accepts a host selection. Times not present in this lane are discarded.
    /// The primary key is used for keyboard navigation and the contextual editor.
    pub fn set_selected_keys(&self, mut times: Vec<f64>, primary: Option<f64>) {
        self.cancel_drag();
        let keys = self.keys.borrow();
        times.retain(|time| keys.binary_search_by(|key| key.total_cmp(time)).is_ok());
        times.sort_by(f64::total_cmp);
        times.dedup();
        self.selected.set(
            primary
                .filter(|time| times.contains(time))
                .or_else(|| times.last().copied()),
        );
        self.selection_anchor.set(self.selected.get());
        *self.selected_keys.borrow_mut() = times;
        self.invalidate_visual();
    }
    pub fn selected_keys(&self) -> Vec<f64> {
        self.selected_keys.borrow().clone()
    }
    /// Sets frame snapping and frame labels, or restores adaptive seconds snapping.
    /// Supported rates are 1–1000 fps; invalid values restore seconds mode.
    pub fn set_frame_rate(&self, frames_per_second: Option<u32>) {
        self.frame_rate
            .set(frames_per_second.filter(|rate| (1..=1000).contains(rate)));
        self.invalidate_visual();
    }
    pub fn frame_rate(&self) -> Option<u32> {
        self.frame_rate.get()
    }
    /// Reports a completed local selection. `additive` preserves selections in other lanes.
    /// No callback is emitted during a pointer gesture, so hosts may rebuild on completion.
    pub fn on_selection_changed(&self, callback: impl Fn(Vec<f64>, Option<f64>, bool) + 'static) {
        *self.selection_changed.borrow_mut() = Some(Rc::new(callback));
    }
    /// Reports one delta for all selected keys. The host commits it atomically.
    pub fn on_move_selection(&self, callback: impl Fn(Vec<f64>, f64) + 'static) {
        *self.move_selection.borrow_mut() = Some(Rc::new(callback));
    }
    pub fn on_remove_selection(&self, callback: impl Fn(Vec<f64>) + 'static) {
        *self.remove_selection.borrow_mut() = Some(Rc::new(callback));
    }
    pub fn on_duplicate_selection(&self, callback: impl Fn(Vec<f64>, f64) + 'static) {
        *self.duplicate_selection.borrow_mut() = Some(Rc::new(callback));
    }
    pub fn set_highlighted(&self, highlighted: bool) {
        self.highlighted.set(highlighted);
        self.invalidate_visual();
    }
    pub fn on_activate(&self, callback: impl Fn(f64) + 'static) {
        *self.activate.borrow_mut() = Some(Rc::new(callback));
    }
    pub fn on_move(&self, callback: impl Fn(f64, f64) + 'static) {
        *self.move_key.borrow_mut() = Some(Rc::new(callback));
    }
    pub fn on_remove(&self, callback: impl Fn(f64) + 'static) {
        *self.remove_key.borrow_mut() = Some(Rc::new(callback));
    }
    pub fn on_insert(&self, callback: impl Fn(f64) + 'static) {
        *self.insert_key.borrow_mut() = Some(Rc::new(callback));
    }
    pub fn on_zoom(&self, callback: impl Fn(f64, f64) + 'static) {
        *self.zoom.borrow_mut() = Some(Rc::new(callback));
    }
    /// Requests a horizontal viewport shift, measured in timeline seconds.
    pub fn on_pan(&self, callback: impl Fn(f64) + 'static) {
        *self.pan.borrow_mut() = Some(Rc::new(callback));
    }
    fn plot_width(&self) -> f64 {
        (self.bounds().width - INSET * 2.0).max(1.0)
    }
    fn span(&self) -> f64 {
        self.view_end.get() - self.view_start.get()
    }
    fn x(&self, time: f64) -> f64 {
        INSET + (time - self.view_start.get()) / self.span() * self.plot_width()
    }
    fn time(&self, x: f64, modifiers: KeyModifiers) -> f64 {
        let value = (self.view_start.get() + (x - INSET) / self.plot_width() * self.span())
            .clamp(0.0, self.duration.get());
        let quantum = if let Some(rate) = self.frame_rate.get() {
            1.0 / f64::from(rate)
        } else if modifiers.contains(KeyModifiers::CONTROL) {
            0.01f64.min(self.span() / 100.0)
        } else {
            0.1f64.min(self.span() / 10.0)
        }
        .max(f64::MIN_POSITIVE);
        if modifiers.contains(KeyModifiers::ALT) {
            value
        } else {
            let steps = value / quantum;
            if steps.is_finite() {
                steps.round() * quantum
            } else {
                value
            }
        }
        .clamp(0.0, self.duration.get())
    }
    fn keyboard_step(&self, modifiers: KeyModifiers) -> f64 {
        if let Some(rate) = self.frame_rate.get() {
            (if modifiers.contains(KeyModifiers::SHIFT) {
                10.0
            } else {
                1.0
            }) / f64::from(rate)
        } else if modifiers.contains(KeyModifiers::SHIFT) {
            1.0
        } else if modifiers.contains(KeyModifiers::CONTROL) {
            0.01
        } else {
            0.1
        }
    }
    fn select_key(&self, time: f64, modifiers: KeyModifiers) {
        let mut selected = self.selected_keys.borrow_mut();
        if modifiers.contains(KeyModifiers::SHIFT) {
            let anchor = self.selection_anchor.get().unwrap_or(time);
            let (start, end) = (anchor.min(time), anchor.max(time));
            if !modifiers.contains(KeyModifiers::CONTROL) {
                selected.clear();
            }
            selected.extend(
                self.keys
                    .borrow()
                    .iter()
                    .copied()
                    .filter(|key| *key >= start && *key <= end),
            );
            selected.sort_by(f64::total_cmp);
            selected.dedup();
        } else if modifiers.contains(KeyModifiers::CONTROL) {
            if let Some(index) = selected.iter().position(|key| *key == time) {
                selected.remove(index);
            } else {
                selected.push(time);
                selected.sort_by(f64::total_cmp);
            }
            self.selection_anchor.set(Some(time));
        } else {
            if !selected.contains(&time) {
                *selected = vec![time];
            }
            self.selection_anchor.set(Some(time));
        }
        self.selected.set(
            selected
                .contains(&time)
                .then_some(time)
                .or_else(|| selected.last().copied()),
        );
    }
    fn publish_selection(&self, additive: bool) {
        let callback = self.selection_changed.borrow().clone();
        if let Some(callback) = callback {
            callback(self.selected_keys(), self.selected.get(), additive);
        } else if let Some(time) = self.selected.get() {
            self.activate_time(time);
        }
    }
    fn move_selected(&self, keys: Vec<f64>, delta: f64) {
        if keys.is_empty() || !delta.is_finite() {
            return;
        }
        let minimum = keys[0];
        let maximum = keys[keys.len() - 1];
        let delta = delta.clamp(-minimum, self.duration.get() - maximum);
        if delta.abs() <= MIN_DURATION {
            return;
        }
        let batch = self.move_selection.borrow().clone();
        if let Some(callback) = batch {
            callback(keys, delta);
        } else if keys.len() == 1 {
            let callback = self.move_key.borrow().clone();
            if let Some(callback) = callback {
                callback(keys[0], keys[0] + delta);
            }
        }
    }
    fn remove_selected(&self) {
        let keys = self.selected_keys();
        let batch = self.remove_selection.borrow().clone();
        if let Some(callback) = batch {
            if !keys.is_empty() {
                callback(keys);
            }
        } else if keys.len() == 1 {
            let callback = self.remove_key.borrow().clone();
            if let Some(callback) = callback {
                callback(keys[0]);
            }
        }
    }
    fn key_at(&self, x: f64) -> Option<f64> {
        let time = self.time(x, KeyModifiers::ALT);
        let keys = self.keys.borrow();
        let index = keys.partition_point(|key| *key < time);
        [index.checked_sub(1), (index < keys.len()).then_some(index)]
            .into_iter()
            .flatten()
            .map(|index| keys[index])
            .filter(|key| (self.view_start.get()..=self.view_end.get()).contains(key))
            .filter(|key| (self.x(*key) - x).abs() <= 8.0)
            .min_by(|a, b| (self.x(*a) - x).abs().total_cmp(&(self.x(*b) - x).abs()))
    }
    fn activate_time(&self, time: f64) {
        let callback = self.activate.borrow().clone();
        if let Some(callback) = callback {
            callback(time);
        }
    }
    fn zoom_at(&self, factor: f64, pivot: f64) {
        let callback = self.zoom.borrow().clone();
        if let Some(callback) = callback {
            callback(factor, pivot);
        }
    }
    fn pan_by(&self, seconds: f64) {
        let callback = self.pan.borrow().clone();
        if let Some(callback) = callback {
            callback(seconds);
        }
    }
    fn capture_drag(&self, drag: Drag, pointer: &Rc<dyn IPointer>) {
        self.cancel_drag();
        *self.drag.borrow_mut() = Some(drag);
        *self.pointer.borrow_mut() = Some(pointer.clone());
        pointer.capture(Some(&self.to_ref().upcast::<InputElement>()));
    }
    fn cancel_drag(&self) {
        if let Some(
            Drag::Marquee {
                previous,
                previous_primary,
                ..
            }
            | Drag::Key {
                previous,
                previous_primary,
                ..
            },
        ) = self.drag.borrow_mut().take()
        {
            *self.selected_keys.borrow_mut() = previous;
            self.selected.set(previous_primary);
        }
        let pointer = self.pointer.borrow_mut().take();
        if let Some(pointer) = pointer {
            if pointer.captured().as_ref() == Some(&self.to_ref().upcast::<InputElement>()) {
                pointer.capture(None);
            }
        }
    }
    fn owns_pointer(&self, pointer: &Rc<dyn IPointer>) -> bool {
        self.pointer
            .borrow()
            .as_ref()
            .is_some_and(|active| active.id() == pointer.id())
    }
}
impl InputElementImpl for TimelineLane {
    fn on_pointer_pressed(this: &Self, event: &PointerPressedEventArgs) {
        if !event.properties().is_left_button_pressed {
            return;
        }
        this.focus();
        let x = event.get_position(Some(this)).x;
        let time = this.time(x, event.key_modifiers());
        event.set_handled(true);
        if this.ruler.get() {
            this.capture_drag(Drag::Scrub, event.pointer());
            this.activate_time(time);
        } else if let Some(from) = this.key_at(x) {
            let previous = this.selected_keys();
            let previous_primary = this.selected.get();
            this.select_key(from, event.key_modifiers());
            let keys = this.selected_keys();
            let keys = if keys.contains(&from) {
                keys
            } else {
                Vec::new()
            };
            this.capture_drag(
                Drag::Key {
                    from,
                    to: from,
                    start_x: x,
                    keys,
                    previous,
                    previous_primary,
                    additive: event
                        .key_modifiers()
                        .intersects(KeyModifiers::CONTROL | KeyModifiers::SHIFT),
                },
                event.pointer(),
            );
        } else if event.click_count() >= 2 {
            let callback = this.insert_key.borrow().clone();
            if let Some(callback) = callback {
                callback(time);
            }
        } else {
            let additive = event
                .key_modifiers()
                .intersects(KeyModifiers::CONTROL | KeyModifiers::SHIFT);
            let base = if additive {
                this.selected_keys()
            } else {
                Vec::new()
            };
            this.capture_drag(
                Drag::Marquee {
                    from: time,
                    to: time,
                    start_x: x,
                    base,
                    previous: this.selected_keys(),
                    previous_primary: this.selected.get(),
                    additive,
                },
                event.pointer(),
            );
        }
        this.invalidate_visual();
        Self::parent_on_pointer_pressed(this, event);
    }
    fn on_pointer_moved(this: &Self, event: &PointerEventArgs) {
        let x = event.get_position(Some(this)).x;
        let time = this.time(x, event.key_modifiers());
        this.hover.set(Some(this.time(x, KeyModifiers::ALT)));
        let scrub = {
            let mut drag = this.drag.borrow_mut();
            match drag.as_mut().filter(|_| this.owns_pointer(event.pointer())) {
                Some(Drag::Key { to, .. }) => {
                    *to = time;
                    false
                }
                Some(Drag::Marquee { from, to, base, .. }) => {
                    *to = time;
                    let (start, end) = (from.min(time), from.max(time));
                    let mut keys = base.clone();
                    keys.extend(
                        this.keys
                            .borrow()
                            .iter()
                            .copied()
                            .filter(|key| *key >= start && *key <= end),
                    );
                    keys.sort_by(f64::total_cmp);
                    keys.dedup();
                    this.selected.set(keys.last().copied());
                    *this.selected_keys.borrow_mut() = keys;
                    false
                }
                Some(Drag::Scrub) => true,
                None => false,
            }
        };
        if scrub {
            this.activate_time(time);
        }
        this.invalidate_visual();
        Self::parent_on_pointer_moved(this, event);
    }
    fn on_pointer_released(this: &Self, event: &PointerReleasedEventArgs) {
        if event.initial_press_mouse_button() != MouseButton::Left
            || !this.owns_pointer(event.pointer())
        {
            Self::parent_on_pointer_released(this, event);
            return;
        }
        let drag = this.drag.borrow_mut().take();
        this.cancel_drag();
        event.set_handled(true);
        match drag {
            Some(Drag::Key {
                from,
                start_x,
                keys,
                additive,
                ..
            }) => {
                let x = event.get_position(Some(this)).x;
                let to = this.time(x, event.key_modifiers());
                if (x - start_x).abs() > 2.0 && (to - from).abs() > MIN_DURATION && !keys.is_empty()
                {
                    this.move_selected(keys, to - from);
                } else {
                    this.publish_selection(additive);
                }
            }
            Some(Drag::Marquee {
                from,
                start_x,
                additive,
                ..
            }) => {
                if (event.get_position(Some(this)).x - start_x).abs() > 2.0 {
                    this.publish_selection(additive);
                } else {
                    this.activate_time(from);
                }
            }
            Some(Drag::Scrub) => this
                .activate_time(this.time(event.get_position(Some(this)).x, event.key_modifiers())),
            None => {}
        }
        this.invalidate_visual();
        Self::parent_on_pointer_released(this, event);
    }
    fn on_pointer_capture_lost(this: &Self, event: &PointerCaptureLostEventArgs) {
        if this.owns_pointer(event.pointer()) {
            this.pointer.borrow_mut().take();
            this.cancel_drag();
        }
        this.invalidate_visual();
        Self::parent_on_pointer_capture_lost(this, event);
    }
    fn on_pointer_exited(this: &Self, event: &PointerEventArgs) {
        this.hover.set(None);
        this.invalidate_visual();
        Self::parent_on_pointer_exited(this, event);
    }
    fn on_pointer_wheel_changed(this: &Self, event: &PointerWheelEventArgs) {
        if event.key_modifiers().contains(KeyModifiers::CONTROL) && event.delta().y != 0.0 {
            let factor = if event.delta().y > 0.0 { 1.25 } else { 0.8 };
            event.set_handled(true);
            this.zoom_at(
                factor,
                this.time(event.get_position(Some(this)).x, KeyModifiers::ALT),
            );
        } else if event.key_modifiers().contains(KeyModifiers::SHIFT) || event.delta().x != 0.0 {
            let delta = if event.delta().x != 0.0 {
                event.delta().x
            } else {
                event.delta().y
            };
            event.set_handled(true);
            this.pan_by(-delta * this.span() * 0.1);
        }
        Self::parent_on_pointer_wheel_changed(this, event);
    }
    fn on_key_down(this: &Self, event: &KeyEventArgs) {
        match event.key {
            Key::Escape => {
                this.cancel_drag();
                event.set_handled(true);
            }
            Key::A if event.key_modifiers.contains(KeyModifiers::CONTROL) && !this.ruler.get() => {
                event.set_handled(true);
                this.set_selected_keys(this.keys.borrow().clone(), this.selected.get());
                this.publish_selection(false);
            }
            Key::D if event.key_modifiers.contains(KeyModifiers::CONTROL) && !this.ruler.get() => {
                event.set_handled(true);
                let callback = this.duplicate_selection.borrow().clone();
                let keys = this.selected_keys();
                if let Some(callback) = callback {
                    if let (Some(first), Some(last)) = (keys.first(), keys.last()) {
                        let delta = last - first + this.keyboard_step(KeyModifiers::NONE);
                        callback(keys, delta);
                    }
                }
            }
            Key::Delete | Key::Back => {
                event.set_handled(true);
                this.remove_selected();
            }
            Key::Left | Key::Right => {
                event.set_handled(true);
                let backwards = event.key == Key::Left;
                if event.key_modifiers.contains(KeyModifiers::ALT) {
                    let time = this.playhead.get();
                    let keys = this.keys.borrow();
                    let next = if backwards {
                        keys.iter()
                            .rev()
                            .copied()
                            .find(|key| *key < time - MIN_DURATION)
                    } else {
                        keys.iter().copied().find(|key| *key > time + MIN_DURATION)
                    };
                    if let Some(time) = next {
                        drop(keys);
                        this.activate_time(time);
                    }
                } else {
                    let delta = this.keyboard_step(event.key_modifiers)
                        * if backwards { -1.0 } else { 1.0 };
                    let keys = this.selected_keys();
                    if keys.is_empty() {
                        this.activate_time(
                            (this.playhead.get() + delta).clamp(0.0, this.duration.get()),
                        );
                    } else {
                        this.move_selected(keys, delta);
                    }
                }
            }
            Key::Home => {
                event.set_handled(true);
                this.activate_time(0.0);
            }
            Key::End => {
                event.set_handled(true);
                this.activate_time(this.duration.get());
            }
            Key::OemPlus | Key::Add => {
                event.set_handled(true);
                this.zoom_at(1.25, this.playhead.get());
            }
            Key::OemMinus | Key::Subtract => {
                event.set_handled(true);
                this.zoom_at(0.8, this.playhead.get());
            }
            Key::PageUp | Key::PageDown => {
                event.set_handled(true);
                this.pan_by(this.span() * if event.key == Key::PageUp { -0.8 } else { 0.8 });
            }
            _ => {}
        }
        this.invalidate_visual();
        Self::parent_on_key_down(this, event);
    }
}
impl VisualImpl for TimelineLane {
    fn render(this: &Self, context: &mut DrawingContext) {
        let bounds = Rect::from_size(this.bounds().size());
        let background = brush(if this.ruler.get() {
            0xff292929
        } else if this.highlighted.get() {
            0xff263943
        } else {
            0xff242424
        });
        context.draw_rectangle(
            Some(&background),
            None,
            bounds,
            3.0,
            3.0,
            &BoxShadows::default(),
        );
        let clip = context.push_clip(bounds);
        let interval = tick_interval(this.span(), this.plot_width());
        let major = this.frame_rate.get().map_or(interval, |rate| {
            (interval * f64::from(rate)).ceil().max(1.0) / f64::from(rate)
        });
        let minor = this.frame_rate.get().map_or(major / 5.0, |rate| {
            (major * f64::from(rate) / 5.0).ceil().max(1.0) / f64::from(rate)
        });
        let first = (this.view_start.get() / minor).ceil() * minor;
        let count = ((this.view_end.get() - first) / minor)
            .ceil()
            .clamp(0.0, 4096.0) as usize;
        let major_pen: Rc<dyn IPen> = Pen::from_uint32(0xff454545, 1.0).into();
        let minor_pen: Rc<dyn IPen> = Pen::from_uint32(0xff303030, 1.0).into();
        for index in 0..=count {
            let time = first + index as f64 * minor;
            if time > this.view_end.get() + minor * 0.01 {
                break;
            }
            let x = this.x(time);
            let is_major = (time / major - (time / major).round()).abs() < 0.00001;
            let y = if this.ruler.get() {
                if is_major { 21.0 } else { 27.0 }
            } else {
                0.0
            };
            context.draw_line(
                if is_major { &major_pen } else { &minor_pen },
                Point::new(x, y),
                Point::new(x, bounds.height),
            );
            if this.ruler.get() && is_major {
                let caption = this.frame_rate.get().map_or_else(
                    || format_time(time, major),
                    |rate| format!("{}f", (time * f64::from(rate)).round() as u64),
                );
                let text = text(&caption, 10.0, 0xffb8b8b8);
                let origin = (x - text.width() * 0.5)
                    .clamp(2.0, (bounds.width - text.width() - 2.0).max(2.0));
                context.draw_text(&text, Point::new(origin, 2.0));
            }
        }
        if !this.ruler.get() {
            let center = bounds.height * 0.5;
            let keys = this.keys.borrow();
            let start = keys.partition_point(|key| *key < this.view_start.get());
            let end = keys.partition_point(|key| *key <= this.view_end.get());
            let drag = this.drag.borrow();
            let selected_keys = this.selected_keys.borrow();
            if let Some(Drag::Marquee { from, to, .. }) = drag.as_ref() {
                let fill = brush(0x334fc3ff);
                let outline: Rc<dyn IPen> = Pen::from_uint32(0xff60cdff, 1.0).into();
                let left = this.x(from.min(*to));
                let right = this.x(from.max(*to));
                context.draw_rectangle(
                    Some(&fill),
                    Some(&outline),
                    Rect::new(left, 1.0, right - left, (bounds.height - 2.0).max(0.0)),
                    1.0,
                    1.0,
                    &BoxShadows::default(),
                );
            }
            for time in &keys[start..end] {
                let selected = selected_keys
                    .binary_search_by(|key| key.total_cmp(time))
                    .is_ok();
                let color = if matches!(drag.as_ref(), Some(Drag::Key { keys, .. }) if keys.binary_search_by(|key| key.total_cmp(time)).is_ok())
                {
                    0xff50616a
                } else if selected
                    || this
                        .hover
                        .get()
                        .is_some_and(|hover| (this.x(hover) - this.x(*time)).abs() <= 8.0)
                {
                    0xffffffff
                } else {
                    0xff7ccfff
                };
                diamond(
                    context,
                    this.x(*time),
                    center,
                    if selected { 5.5 } else { 4.5 },
                    color,
                );
            }
            if let Some(Drag::Key { from, to, keys, .. }) = drag.as_ref() {
                let minimum = keys.first().copied().unwrap_or(0.0);
                let maximum = keys.last().copied().unwrap_or(0.0);
                let delta = if keys.is_empty() {
                    0.0
                } else {
                    (to - from).clamp(-minimum, this.duration.get() - maximum)
                };
                let start = keys.partition_point(|key| *key + delta < this.view_start.get());
                let end = keys.partition_point(|key| *key + delta <= this.view_end.get());
                for key in &keys[start..end] {
                    diamond(context, this.x(*key + delta), center, 6.0, 0xff60cdff);
                }
                let to = from + delta;
                let text = text(&format!("{to:.3}s"), 10.0, 0xffffffff);
                context.draw_text(
                    &text,
                    Point::new(
                        (this.x(to) + 10.0).min((bounds.width - text.width()).max(0.0)),
                        1.0,
                    ),
                );
            }
        }
        let playhead = this.playhead.get();
        if (this.view_start.get()..=this.view_end.get()).contains(&playhead) {
            let x = this.x(playhead);
            let pen: Rc<dyn IPen> = Pen::from_uint32(0xff60cdff, 1.5).into();
            context.draw_line(&pen, Point::new(x, 0.0), Point::new(x, bounds.height));
            if this.ruler.get() {
                let accent = brush(0xff60cdff);
                context.draw_rectangle(
                    Some(&accent),
                    None,
                    Rect::new(x - 4.0, 22.0, 8.0, 11.0),
                    2.0,
                    2.0,
                    &BoxShadows::default(),
                );
            }
        }
        context.pop(clip);
    }
}
fn brush(color: u32) -> Rc<dyn IBrush> {
    SolidColorBrush::from_uint32(color).into()
}
fn text(value: &str, size: f64, color: u32) -> FormattedText {
    FormattedText::new(
        value,
        CultureInfo::current_culture(),
        FlowDirection::LeftToRight,
        Typeface::default(),
        size,
        Some(brush(color)),
    )
}
fn diamond(context: &mut DrawingContext, x: f64, y: f64, radius: f64, color: u32) {
    let state = context.push_transform(Matrix::create_rotation_at(
        std::f64::consts::FRAC_PI_4,
        Point::new(x, y),
    ));
    let side = radius * std::f64::consts::SQRT_2;
    let fill = brush(color);
    context.draw_rectangle(
        Some(&fill),
        None,
        Rect::new(x - side * 0.5, y - side * 0.5, side, side),
        0.5,
        0.5,
        &BoxShadows::default(),
    );
    context.pop(state);
}
fn tick_interval(span: f64, width: f64) -> f64 {
    let raw = (span / (width / 72.0).max(1.0)).max(f64::MIN_POSITIVE);
    let magnitude = 10.0f64.powf(raw.log10().floor());
    let scaled = raw / magnitude;
    let interval = magnitude
        * if scaled <= 1.0 {
            1.0
        } else if scaled <= 2.0 {
            2.0
        } else if scaled <= 5.0 {
            5.0
        } else {
            10.0
        };
    if interval.is_finite() {
        interval
    } else {
        magnitude
    }
}

fn format_time(time: f64, interval: f64) -> String {
    let precision = (-interval.log10().floor()).clamp(0.0, 9.0) as usize;
    if time >= 60.0 {
        let minutes = (time / 60.0).floor() as u64;
        let seconds = time % 60.0;
        let width = 2 + if precision > 0 { precision + 1 } else { 0 };
        format!("{minutes}:{seconds:0width$.precision$}")
    } else {
        format!("{time:.precision$}s")
    }
}

#[cfg(test)]
#[path = "timeline_lane_tests.rs"]
mod tests;

mod plot;
#[cfg(test)]
mod tests;
use ferroui_base::input::{
    FocusChangedEventArgs, IPointer, InputElement, InputElementImpl, InputElementImplExt, Key,
    KeyEventArgs, KeyModifiers, PointerCaptureLostEventArgs, PointerEventArgs,
    PointerPressedEventArgs, PointerReleasedEventArgs,
};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Point, Rect,
    Ref, StyledElementImpl,
};
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::{Control, ControlImpl, ToolTip};
use ferroui_designer_animation::Easing;
use plot::Plot;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

type Edit = Rc<dyn Fn(Easing)>;
const HELP: &str = "Drag a Bézier handle or nudge it with the arrow keys. Space switches handles. Shift makes larger steps and Ctrl or Command makes finer steps.";

/// A native graph of a keyframe's incoming easing. The sampled curve uses the
/// animation engine's interpolation, including values outside zero to one.
#[repr(C)]
pub struct EasingPreview {
    base: Control,
    easing: RefCell<Option<Easing>>,
    samples: RefCell<Vec<f64>>,
    change: RefCell<Option<Edit>>,
    handle: Cell<usize>,
    drag: RefCell<Option<HandleDrag>>,
}

struct HandleDrag {
    pointer: Rc<dyn IPointer>,
    plot: Plot,
    initial: Easing,
    start: Point,
    offset: Point,
}

ferro_class!(EasingPreview:Control);
ferro_class_info!(EasingPreview {
    new: EasingPreview::new
});
ferro_impl_classes!(EasingPreview:FerroObjectImpl,StyledElementImpl,LayoutableImpl,InteractiveImpl,ControlImpl);

impl EasingPreview {
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self {
            base: Control::construct(),
            easing: RefCell::new(None),
            samples: RefCell::new(Vec::new()),
            change: RefCell::new(None),
            handle: Cell::new(0),
            drag: RefCell::new(None),
        });
        this.set_width(48.0);
        this.set_height(26.0);
        this.set_easing(Some(Easing::Linear));
        this
    }

    /// An invalid draft clears the curve.
    pub fn set_easing(&self, easing: Option<Easing>) {
        let samples = easing
            .as_ref()
            .and_then(|curve| curve.preview_samples().ok());
        *self.easing.borrow_mut() = easing.filter(|_| samples.is_some());
        *self.samples.borrow_mut() = samples.unwrap_or_default();
        self.update_description();
        self.invalidate_visual();
    }

    pub fn easing(&self) -> Option<Easing> {
        self.easing.borrow().clone()
    }

    pub(crate) fn edit_handles(&self, callback: impl Fn(Easing) + 'static) {
        *self.change.borrow_mut() = Some(Rc::new(callback));
        self.set_focusable(true);
        self.set_width(156.0);
        self.set_height(78.0);
        AutomationProperties::set_help_text(self, Some(HELP));
        self.update_description();
        self.invalidate_visual();
    }

    fn handles(&self) -> Option<[Point; 2]> {
        if self.change.borrow().is_none() {
            return None;
        }
        control_points(self.easing.borrow().as_ref()?)
    }

    fn update_description(&self) {
        let description = if let Some(handles) = self.handles() {
            let active = self.handle.get();
            let point = handles[active];
            format!(
                "Incoming easing. Bézier handle {}. X {:.3}, Y {:.3}",
                active + 1,
                point.x,
                point.y
            )
        } else {
            self.easing.borrow().as_ref().map_or_else(
                || "Invalid easing curve".to_owned(),
                |curve| format!("Incoming easing: {}", curve.to_xaml()),
            )
        };
        AutomationProperties::set_name(self, Some(&description));
        ToolTip::set_tip(
            self,
            Some(Rc::new(if self.handles().is_some() {
                format!("{description}. {HELP}")
            } else {
                description
            })),
        );
    }

    fn emit_draft(&self, easing: Easing) {
        if self.easing.borrow().as_ref() == Some(&easing) || easing.validate().is_err() {
            return;
        }
        self.set_easing(Some(easing.clone()));
        let change = self.change.borrow().clone();
        if let Some(change) = change {
            change(easing);
        }
    }

    fn edit_point(&self, position: Point) {
        let Some(mut points) = self.handles() else {
            return;
        };
        if !position.x.is_finite() || !position.y.is_finite() {
            return;
        }
        let round = |number: f64| {
            if number.abs() < 1e9 {
                (number * 1e6).round() / 1e6
            } else {
                number
            }
        };
        let point = Point::new(round(position.x.clamp(0.0, 1.0)), round(position.y));
        if points[self.handle.get()] == point {
            return;
        }
        points[self.handle.get()] = point;
        let [start, end] = points;
        self.emit_draft(Easing::CubicBezier {
            x1: start.x,
            y1: start.y,
            x2: end.x,
            y2: end.y,
        });
    }

    fn plot(&self) -> Plot {
        if let Some(drag) = self.drag.borrow().as_ref() {
            return drag.plot;
        }
        Plot::new(
            Rect::from_size(self.bounds().size()),
            &self.samples.borrow(),
            self.handles(),
        )
    }

    fn move_pointer(&self, pointer: &Rc<dyn IPointer>, position: Point) -> bool {
        let Some((plot, start, offset, initial)) = self
            .drag
            .borrow()
            .as_ref()
            .filter(|drag| drag.pointer.id() == pointer.id())
            .map(|drag| (drag.plot, drag.start, drag.offset, drag.initial.clone()))
        else {
            return false;
        };
        if Point::distance(start, position) < 0.5 {
            self.emit_draft(initial);
        } else {
            self.edit_point(plot.value(position + offset));
        }
        true
    }

    fn finish_drag(&self, cancel: bool) {
        let drag = self.drag.borrow_mut().take();
        if let Some(drag) = drag {
            if cancel {
                self.emit_draft(drag.initial);
            }
            if drag.pointer.captured().as_ref() == Some(&self.to_ref().upcast::<InputElement>()) {
                drag.pointer.capture(None);
            }
            self.invalidate_visual();
        }
    }
}

impl InputElementImpl for EasingPreview {
    fn on_pointer_pressed(this: &Self, event: &PointerPressedEventArgs) {
        if !event.properties().is_left_button_pressed || this.drag.borrow().is_some() {
            return Self::parent_on_pointer_pressed(this, event);
        }
        if let Some((points, initial)) = this.handles().zip(this.easing()) {
            let plot = this.plot();
            let position = event.get_position(Some(this));
            let hits = points.map(|point| Point::distance(plot.point(point), position));
            let handle = usize::from(hits[1] < hits[0]);
            if hits[handle] <= 9.0 && plot.can_edit() {
                this.handle.set(handle);
                *this.drag.borrow_mut() = Some(HandleDrag {
                    pointer: event.pointer().clone(),
                    plot,
                    initial,
                    start: position,
                    offset: plot.point(points[handle]) - position,
                });
                this.focus();
                event
                    .pointer()
                    .capture(Some(&this.to_ref().upcast::<InputElement>()));
                event.set_handled(true);
                this.update_description();
                this.invalidate_visual();
            }
        }
        Self::parent_on_pointer_pressed(this, event);
    }

    fn on_pointer_moved(this: &Self, event: &PointerEventArgs) {
        if this.move_pointer(event.pointer(), event.get_position(Some(this))) {
            event.set_handled(true);
        }
        Self::parent_on_pointer_moved(this, event);
    }

    fn on_pointer_released(this: &Self, event: &PointerReleasedEventArgs) {
        if this.move_pointer(event.pointer(), event.get_position(Some(this))) {
            this.finish_drag(false);
            event.set_handled(true);
        }
        Self::parent_on_pointer_released(this, event);
    }

    fn on_pointer_capture_lost(this: &Self, event: &PointerCaptureLostEventArgs) {
        let owns = this
            .drag
            .borrow()
            .as_ref()
            .is_some_and(|drag| drag.pointer.id() == event.pointer().id());
        if owns {
            this.finish_drag(true);
        }
        Self::parent_on_pointer_capture_lost(this, event);
    }

    fn on_got_focus(this: &Self, event: &FocusChangedEventArgs) {
        this.invalidate_visual();
        Self::parent_on_got_focus(this, event);
    }

    fn on_lost_focus(this: &Self, event: &FocusChangedEventArgs) {
        this.invalidate_visual();
        Self::parent_on_lost_focus(this, event);
    }

    fn on_key_down(this: &Self, event: &KeyEventArgs) {
        if event.key_modifiers.contains(KeyModifiers::ALT) {
            return Self::parent_on_key_down(this, event);
        }
        let Some(points) = this.handles() else {
            return Self::parent_on_key_down(this, event);
        };
        if event.key == Key::Escape && this.drag.borrow().is_some() {
            this.finish_drag(true);
            event.set_handled(true);
            return;
        }
        if this.drag.borrow().is_none() {
            if event.key == Key::Space {
                this.handle.set(1 - this.handle.get());
                this.update_description();
                this.invalidate_visual();
                event.set_handled(true);
                return;
            }
            let step = if event
                .key_modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::META)
            {
                0.001
            } else if event.key_modifiers.contains(KeyModifiers::SHIFT) {
                0.1
            } else {
                0.01
            };
            let current = points[this.handle.get()];
            let next = match event.key {
                Key::Left => Point::new(current.x - step, current.y),
                Key::Right => Point::new(current.x + step, current.y),
                Key::Up => Point::new(current.x, current.y + step),
                Key::Down => Point::new(current.x, current.y - step),
                _ => return Self::parent_on_key_down(this, event),
            };
            this.edit_point(next);
            event.set_handled(true);
        }
    }
}

fn control_points(easing: &Easing) -> Option<[Point; 2]> {
    match *easing {
        Easing::Linear => Some([Point::new(0.0, 0.0), Point::new(1.0, 1.0)]),
        Easing::CubicBezier { x1, y1, x2, y2 } => Some([Point::new(x1, y1), Point::new(x2, y2)]),
        _ => None,
    }
}

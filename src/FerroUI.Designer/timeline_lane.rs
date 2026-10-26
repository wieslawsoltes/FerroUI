//! Native proportional keyframe lane. Dragging edits a ghost marker; pointer release commits
//! one timestamp transaction. Capture loss and Escape leave the authored track unchanged.
use ferroui_base::input::{
    InputElement, InputElementImpl, InputElementImplExt, Key, KeyEventArgs, KeyModifiers,
    PointerCaptureLostEventArgs, PointerEventArgs, PointerPressedEventArgs,
    PointerReleasedEventArgs,
};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::{
    BoxShadows, Brushes, DrawingContext, IBrush, IPen, Pen, SolidColorBrush,
};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Point, Rect,
    Ref, StyledElementImpl, VisualImpl,
};
use ferroui_controls::{Control, ControlImpl};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[derive(Clone, Copy)]
struct Drag {
    from: f64,
    to: f64,
    start_x: f64,
}
#[repr(C)]
pub struct TimelineLane {
    base: Control,
    keys: RefCell<Vec<f64>>,
    duration: Cell<f64>,
    playhead: Cell<f64>,
    drag: RefCell<Option<Drag>>,
    selected: Cell<Option<f64>>,
    activate: RefCell<Option<Rc<dyn Fn(f64)>>>,
    move_key: RefCell<Option<Rc<dyn Fn(f64, f64)>>>,
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
            playhead: Cell::new(0.0),
            drag: RefCell::new(None),
            selected: Cell::new(None),
            activate: RefCell::new(None),
            move_key: RefCell::new(None),
        });
        this.set_height(32.0);
        this.set_min_width(120.0);
        this.set_focusable(true);
        this
    }
    pub fn configure(&self, duration: f64, keys: Vec<f64>) {
        self.duration.set(duration.max(0.0000001));
        *self.keys.borrow_mut() = keys;
        self.invalidate_visual();
    }
    pub fn set_playhead(&self, time: f64) {
        self.playhead.set(time);
        self.invalidate_visual();
    }
    pub fn on_activate(&self, callback: impl Fn(f64) + 'static) {
        *self.activate.borrow_mut() = Some(Rc::new(callback));
    }
    pub fn on_move(&self, callback: impl Fn(f64, f64) + 'static) {
        *self.move_key.borrow_mut() = Some(Rc::new(callback));
    }
    fn x(&self, time: f64) -> f64 {
        8.0 + time / self.duration.get() * (self.bounds().width - 16.0).max(1.0)
    }
    fn time(&self, x: f64, modifiers: KeyModifiers) -> f64 {
        let value = ((x - 8.0) / (self.bounds().width - 16.0).max(1.0) * self.duration.get())
            .clamp(0.0, self.duration.get());
        if modifiers.contains(KeyModifiers::ALT) {
            value
        } else {
            (value * 10.0).round() / 10.0
        }
        .clamp(0.0, self.duration.get())
    }
}
impl InputElementImpl for TimelineLane {
    fn on_pointer_pressed(this: &Self, event: &PointerPressedEventArgs) {
        if !event.properties().is_left_button_pressed {
            return;
        }
        let x = event.get_position(Some(this)).x;
        let key = this
            .keys
            .borrow()
            .iter()
            .copied()
            .filter(|time| (this.x(*time) - x).abs() <= 8.0)
            .min_by(|a, b| (this.x(*a) - x).abs().total_cmp(&(this.x(*b) - x).abs()));
        if let Some(from) = key {
            this.selected.set(Some(from));
            *this.drag.borrow_mut() = Some(Drag {
                from,
                to: from,
                start_x: x,
            });
            event
                .pointer()
                .capture(Some(&this.to_ref().upcast::<InputElement>()));
        } else {
            let callback = this.activate.borrow().clone();
            if let Some(callback) = callback {
                callback(this.time(x, event.key_modifiers()));
            }
        }
        event.set_handled(true);
        this.invalidate_visual();
        Self::parent_on_pointer_pressed(this, event);
    }
    fn on_pointer_moved(this: &Self, event: &PointerEventArgs) {
        if let Some(drag) = this.drag.borrow_mut().as_mut() {
            drag.to = this.time(event.get_position(Some(this)).x, event.key_modifiers());
            this.invalidate_visual();
        }
        Self::parent_on_pointer_moved(this, event);
    }
    fn on_pointer_released(this: &Self, event: &PointerReleasedEventArgs) {
        let drag = this.drag.borrow_mut().take();
        event.pointer().capture(None);
        if let Some(mut drag) = drag {
            let x = event.get_position(Some(this)).x;
            drag.to = this.time(x, event.key_modifiers());
            if (x - drag.start_x).abs() > 2.0 && (drag.to - drag.from).abs() > 0.0000001 {
                let callback = this.move_key.borrow().clone();
                if let Some(callback) = callback {
                    callback(drag.from, drag.to);
                }
            } else {
                let callback = this.activate.borrow().clone();
                if let Some(callback) = callback {
                    callback(drag.from);
                }
            }
        }
        event.set_handled(true);
        this.invalidate_visual();
        Self::parent_on_pointer_released(this, event);
    }
    fn on_pointer_capture_lost(this: &Self, event: &PointerCaptureLostEventArgs) {
        this.drag.borrow_mut().take();
        this.invalidate_visual();
        Self::parent_on_pointer_capture_lost(this, event);
    }
    fn on_key_down(this: &Self, event: &KeyEventArgs) {
        if event.key == Key::Escape {
            this.drag.borrow_mut().take();
            this.invalidate_visual();
            event.set_handled(true);
        }
        Self::parent_on_key_down(this, event);
    }
}
impl VisualImpl for TimelineLane {
    fn render(this: &Self, context: &mut DrawingContext) {
        let background: Rc<dyn IBrush> = SolidColorBrush::from_uint32(0xff1b1f2b).into();
        context.draw_rectangle(
            Some(&background),
            None,
            Rect::from_size(this.bounds().size()),
            4.0,
            4.0,
            &BoxShadows::default(),
        );
        let tick: Rc<dyn IPen> = Pen::from_uint32(0xff394154, 1.0).into();
        let center = this.bounds().height * 0.5;
        for index in 0..=10 {
            let x = this.x(this.duration.get() * index as f64 / 10.0);
            context.draw_line(
                &tick,
                Point::new(x, 4.0),
                Point::new(x, this.bounds().height - 4.0),
            );
        }
        let key_pen: Rc<dyn IPen> = Pen::from_uint32(0xff9bb5ff, 2.0).into();
        let selected_pen: Rc<dyn IPen> = Pen::from_uint32(0xffffd78a, 2.0).into();
        let drag = *this.drag.borrow();
        for time in this.keys.borrow().iter() {
            let x = this.x(*time);
            diamond(
                context,
                if this.selected.get() == Some(*time) {
                    &selected_pen
                } else {
                    &key_pen
                },
                x,
                center,
                5.0,
            );
        }
        if let Some(drag) = drag {
            diamond(context, &selected_pen, this.x(drag.to), center, 7.0);
        }
        let playhead: Rc<dyn IPen> = Pen::from_uint32(0xfff6a07b, 1.5).into();
        let x = this.x(this.playhead.get().clamp(0.0, this.duration.get()));
        context.draw_line(
            &playhead,
            Point::new(x, 0.0),
            Point::new(x, this.bounds().height),
        );
        let fill: Rc<dyn IBrush> = Brushes::white();
        context.draw_ellipse_at(Some(&fill), None, Point::new(x, 3.0), 2.5, 2.5);
    }
}
fn diamond(context: &mut DrawingContext, pen: &Rc<dyn IPen>, x: f64, y: f64, size: f64) {
    let p = [
        Point::new(x, y - size),
        Point::new(x + size, y),
        Point::new(x, y + size),
        Point::new(x - size, y),
    ];
    for i in 0..4 {
        context.draw_line(pen, p[i], p[(i + 1) % 4]);
    }
}

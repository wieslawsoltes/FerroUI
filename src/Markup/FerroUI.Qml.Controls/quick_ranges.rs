//! Keep native limits monotonic while exposing both ascending and descending QML ranges.
use super::*;
use ferroui_base::{layout::Orientation, reactive::Disposable, utilities::Decimal};
use ferroui_controls::{
    primitives::{RangeBase, TemplatedControl},
    NumericUpDown, ProgressBar, Slider, TextBox,
};

enum Kind {
    Slider(Ref<Slider>),
    Progress(Ref<ProgressBar>),
    Spin(Ref<NumericUpDown>),
}
pub(super) struct RangePeer {
    kind: Kind,
    from: Cell<f64>,
    to: Cell<f64>,
    value: Cell<f64>,
    step: Cell<f64>,
    mirrored: Cell<bool>,
    applying: Rc<Cell<bool>>,
    editable: Rc<Cell<bool>>,
    editor: Rc<RefCell<Option<Ref<TextBox>>>>,
}
impl RangePeer {
    pub fn slider() -> Self {
        Self::new(Kind::Slider(Slider::new()), 1., 0.)
    }
    pub fn progress() -> Self {
        Self::new(Kind::Progress(ProgressBar::new()), 1., 0.)
    }
    pub fn spin() -> Self {
        let spin = NumericUpDown::new();
        spin.set_clip_value_to_min_max(true);
        Self::new(Kind::Spin(spin), 99., 1.)
    }
    fn new(kind: Kind, to: f64, step: f64) -> Self {
        Self {
            kind,
            from: Cell::new(0.),
            to: Cell::new(to),
            value: Cell::new(0.),
            step: Cell::new(step),
            mirrored: Default::default(),
            applying: Default::default(),
            editable: Default::default(),
            editor: Default::default(),
        }
    }
    pub fn control(&self) -> Ref<Control> {
        match &self.kind {
            Kind::Slider(x) => x.clone().upcast(),
            Kind::Progress(x) => x.clone().upcast(),
            Kind::Spin(x) => x.clone().upcast(),
        }
    }
    pub fn apply(&self, name: &str, value: &Value) {
        let old = self.applying.replace(true);
        match (name, value) {
            ("from", Value::Number(x)) => self.from.set(*x),
            ("to", Value::Number(x)) => self.to.set(*x),
            ("value", Value::Number(x)) => self.value.set(*x),
            ("stepSize", Value::Number(x)) => self.step.set(*x),
            ("LayoutMirroring.enabled", Value::Bool(x)) => self.mirrored.set(*x),
            ("orientation", Value::Number(x)) => {
                if let Kind::Slider(slider) = &self.kind {
                    slider.set_orientation(if *x == 2. {
                        Orientation::Vertical
                    } else {
                        Orientation::Horizontal
                    })
                }
            }
            ("indeterminate", Value::Bool(x)) => {
                if let Kind::Progress(bar) = &self.kind {
                    bar.set_is_indeterminate(*x)
                }
            }
            ("editable", Value::Bool(x)) => {
                self.editable.set(*x);
                if let Some(editor) = self.editor.borrow().as_ref() {
                    editor.set_is_read_only(!x);
                }
            }
            ("wrap", Value::Bool(x)) => {
                if let Kind::Spin(spin) = &self.kind {
                    spin.set_is_wrap_enabled(*x)
                }
            }
            _ => {
                self.applying.set(old);
                return;
            }
        }
        let a = self.from.get();
        let b = self.to.get();
        let min = a.min(b);
        let max = a.max(b);
        let v = self.value.get().clamp(min, max);
        match &self.kind {
            Kind::Spin(spin) => {
                spin.set_minimum(Decimal::from(min as i32));
                spin.set_maximum(Decimal::from(max as i32));
                let step = self.step.get() as i32;
                spin.set_increment(Decimal::from(if a > b {
                    -(step as i64)
                } else {
                    step as i64
                }));
                spin.set_numeric_value(Some(Decimal::from(v as i32)));
            }
            Kind::Slider(slider) => {
                slider.set_minimum(min);
                slider.set_maximum(max);
                slider.set_is_direction_reversed(
                    (a > b)
                        ^ (self.mirrored.get() && slider.orientation() == Orientation::Horizontal),
                );
                slider.set_small_change(if self.step.get() == 0. {
                    0.1
                } else {
                    self.step.get().abs()
                });
                slider.set_range_value(v);
            }
            Kind::Progress(bar) => {
                bar.set_minimum(min);
                bar.set_maximum(max);
                bar.set_range_value(v);
                bar.set_flow_direction(if (a > b) ^ self.mirrored.get() {
                    ferroui_base::media::FlowDirection::RightToLeft
                } else {
                    ferroui_base::media::FlowDirection::LeftToRight
                });
            }
        }
        self.applying.set(old);
    }
    pub fn connect(&self, endpoint: NativeEndpoint) -> Vec<Rc<dyn IDisposable>> {
        let guard = self.applying.clone();
        match &self.kind {
            Kind::Slider(c) => vec![c.property_changed(move |e| {
                if !guard.get()
                    && e.is_effective_value_change()
                    && e.property() == RangeBase::value_property().as_property()
                {
                    update_from_native(&endpoint, e.get_new_value::<f64>());
                }
            })],
            Kind::Spin(c) => {
                let editable = self.editable.clone();
                let part = self.editor.clone();
                // NumericUpDown's read-only state also disables its spin buttons.
                // Qt SpinBox.editable controls only the inner text editor.
                let token = c.template_applied(move |_, args| {
                    let editor = args.name_scope().find_as::<TextBox>("PART_TextBox");
                    if let Some(e) = editor.as_ref() {
                        e.set_is_read_only(!editable.get());
                    }
                    part.replace(editor);
                });
                let weak = c.downgrade();
                let template = Disposable::create(move || {
                    if let Some(c) = weak.upgrade() {
                        c.remove_handler(TemplatedControl::template_applied_event(), token);
                    }
                });
                vec![
                    template,
                    c.property_changed(move |e| {
                        if !guard.get()
                            && e.is_effective_value_change()
                            && e.property() == NumericUpDown::value_property().as_property()
                        {
                            if let Some(n) = e.get_new_value::<Option<Decimal>>() {
                                update_from_native(&endpoint, n.to_f64().round());
                            }
                        }
                    }),
                ]
            }
            Kind::Progress(_) => vec![],
        }
    }
}

fn update_from_native(endpoint: &NativeEndpoint, value: f64) {
    endpoint.dispatch(|view| {
        let previous = view.get(endpoint.object, "value")?;
        let result = view
            .call(
                endpoint.object,
                "$qml:rangeInput",
                vec![Value::Number(value)],
            )
            .map(|_| ());
        if result.is_err() {
            if let Some(peer) = view.peer(endpoint.object) {
                peer.apply("value", &previous)?;
            }
        }
        result
    });
}

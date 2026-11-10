//! Setters reaching the values of controls: a styling pass of attached controls with one style of
//! the root that matches them by type and sets 1, 8 or 32 of their properties. The controls are
//! of a type made for the benchmark, with 32 number properties nothing reads: the pass costs the
//! style instance, its setters and the values of the control, and nothing after them.
//!
//! - `plain`: setters with plain values; a style without an activator and with plain values is
//!   one instance shared by every control, so the cost of a control is its frame and the
//!   evaluation of its values, which grows with the setters (section 9.25);
//! - `binding`: setters whose value is a binding (an observable with one value): every control
//!   gets an instance of its own and a subscription for each setter;
//! - `template`: setters whose value is a template, built for every control;
//! - `under_local_value`: the plain setters on controls that set the same properties locally: the
//!   style is applied and every value stays the local one.
//!
//! A setter whose value is a dynamic resource is not measured: the markup extension that makes
//! one is in the markup crate, which the benchmarks do not depend on. A control that follows a
//! resource through a binding of its own is `ResourceLookup.bind`.

use super::wall::CONTROLS;
use crate::harness::Registry;
use crate::trees::fixture::Fixture;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::reactive::Observable;
use ferroui_base::styling::{ITemplate, Selectors, Setter, Style};
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_properties, instantiate, BoxedValue, FerroObjectImpl, FerroProperty, Ref,
    StyledElementImpl, StyledProperty, VisualImpl,
};
use ferroui_controls::{Canvas, Control, ControlImpl};
use std::rc::Rc;

/// The value the setters give a property; its default is 0.
const STYLED: f64 = 1.0;
/// The value a control gives a property locally.
const LOCAL: f64 = 2.0;

/// What the setters of the style hold.
#[derive(Clone, Copy)]
enum Value {
    Plain,
    Binding,
    Template,
    /// Plain values, on controls that set the same properties locally.
    UnderLocalValue,
}

/// The template of a setter: it builds the value of a number property.
struct ValueTemplate;

impl ITemplate for ValueTemplate {
    fn build(&self) -> BoxedValue {
        Rc::new(STYLED)
    }
}

pub struct SetterApply {
    fixture: Fixture,
    targets: Vec<Ref<SetterTarget>>,
}

impl SetterApply {
    fn new(setters: usize, value: Value) -> Self {
        SetterTarget::TYPE.ensure_class_init();
        let fixture = Fixture::new();
        let all = SetterTarget::properties();
        let properties = &all[..setters];
        let style = Style::with_setters(
            Selectors::of_type::<SetterTarget>(),
            properties.iter().map(|&property| match value {
                Value::Plain | Value::UnderLocalValue => Setter::new(property, STYLED),
                Value::Binding => {
                    let value: BoxedValue = Rc::new(STYLED);
                    Setter::new_binding(property, Observable::single_value(value))
                }
                Value::Template => Setter::new_template(property, Rc::new(ValueTemplate)),
            }),
        );
        fixture.root.styles().add(&style);
        let local = matches!(value, Value::UnderLocalValue);
        let canvas = Canvas::new();
        let mut targets = Vec::with_capacity(CONTROLS);
        for _ in 0..CONTROLS {
            let target = SetterTarget::new();
            if local {
                for &property in properties {
                    target.set_value(property, LOCAL);
                }
            }
            canvas.children().add(&target);
            targets.push(target);
        }
        fixture.show(Some(&canvas.upcast::<Control>()));
        fixture.lay_out();

        let this = Self { fixture, targets };
        let (styled, unstyled) = if local { (LOCAL, LOCAL) } else { (STYLED, 0.0) };
        this.expect(properties, styled, "after the attach");
        this.prepare();
        this.expect(properties, unstyled, "without the style");
        this.style();
        this.expect(properties, styled, "after the pass");
        this
    }

    fn expect(&self, properties: &[&'static StyledProperty<f64>], value: f64, when: &str) {
        for target in &self.targets {
            for &property in properties {
                let found = target.get_value(property);
                if found != value {
                    panic!("Invalid benchmark state: a property is {found} {when}, {value} is expected");
                }
            }
        }
    }

    /// Before a measured pass: what the last one queued is laid out, then the style is taken off
    /// the controls.
    fn prepare(&self) {
        self.fixture.lay_out();
        for target in &self.targets {
            target.invalidate_styles(false);
        }
    }

    /// A styling pass of every control.
    fn style(&self) {
        for target in &self.targets {
            target.apply_styling();
        }
    }
}

/// The control of the benchmark: 32 number properties that nothing reads.
#[repr(C)]
struct SetterTarget {
    base: Control,
}

ferro_class!(SetterTarget: Control);
ferro_impl_classes!(
    SetterTarget: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

ferro_properties! {
    impl SetterTarget {
        pub fn p00_property() -> StyledProperty<f64> {
            FerroProperty::register::<SetterTarget, _>("P00", 0.0)
        }

        pub fn p01_property() -> StyledProperty<f64> {
            FerroProperty::register::<SetterTarget, _>("P01", 0.0)
        }

        pub fn p02_property() -> StyledProperty<f64> {
            FerroProperty::register::<SetterTarget, _>("P02", 0.0)
        }

        pub fn p03_property() -> StyledProperty<f64> {
            FerroProperty::register::<SetterTarget, _>("P03", 0.0)
        }

        pub fn p04_property() -> StyledProperty<f64> {
            FerroProperty::register::<SetterTarget, _>("P04", 0.0)
        }

        pub fn p05_property() -> StyledProperty<f64> {
            FerroProperty::register::<SetterTarget, _>("P05", 0.0)
        }

        pub fn p06_property() -> StyledProperty<f64> {
            FerroProperty::register::<SetterTarget, _>("P06", 0.0)
        }

        pub fn p07_property() -> StyledProperty<f64> {
            FerroProperty::register::<SetterTarget, _>("P07", 0.0)
        }

        pub fn p08_property() -> StyledProperty<f64> {
            FerroProperty::register::<SetterTarget, _>("P08", 0.0)
        }

        pub fn p09_property() -> StyledProperty<f64> {
            FerroProperty::register::<SetterTarget, _>("P09", 0.0)
        }

        pub fn p10_property() -> StyledProperty<f64> {
            FerroProperty::register::<SetterTarget, _>("P10", 0.0)
        }

        pub fn p11_property() -> StyledProperty<f64> {
            FerroProperty::register::<SetterTarget, _>("P11", 0.0)
        }

        pub fn p12_property() -> StyledProperty<f64> {
            FerroProperty::register::<SetterTarget, _>("P12", 0.0)
        }

        pub fn p13_property() -> StyledProperty<f64> {
            FerroProperty::register::<SetterTarget, _>("P13", 0.0)
        }

        pub fn p14_property() -> StyledProperty<f64> {
            FerroProperty::register::<SetterTarget, _>("P14", 0.0)
        }

        pub fn p15_property() -> StyledProperty<f64> {
            FerroProperty::register::<SetterTarget, _>("P15", 0.0)
        }

        pub fn p16_property() -> StyledProperty<f64> {
            FerroProperty::register::<SetterTarget, _>("P16", 0.0)
        }

        pub fn p17_property() -> StyledProperty<f64> {
            FerroProperty::register::<SetterTarget, _>("P17", 0.0)
        }

        pub fn p18_property() -> StyledProperty<f64> {
            FerroProperty::register::<SetterTarget, _>("P18", 0.0)
        }

        pub fn p19_property() -> StyledProperty<f64> {
            FerroProperty::register::<SetterTarget, _>("P19", 0.0)
        }

        pub fn p20_property() -> StyledProperty<f64> {
            FerroProperty::register::<SetterTarget, _>("P20", 0.0)
        }

        pub fn p21_property() -> StyledProperty<f64> {
            FerroProperty::register::<SetterTarget, _>("P21", 0.0)
        }

        pub fn p22_property() -> StyledProperty<f64> {
            FerroProperty::register::<SetterTarget, _>("P22", 0.0)
        }

        pub fn p23_property() -> StyledProperty<f64> {
            FerroProperty::register::<SetterTarget, _>("P23", 0.0)
        }

        pub fn p24_property() -> StyledProperty<f64> {
            FerroProperty::register::<SetterTarget, _>("P24", 0.0)
        }

        pub fn p25_property() -> StyledProperty<f64> {
            FerroProperty::register::<SetterTarget, _>("P25", 0.0)
        }

        pub fn p26_property() -> StyledProperty<f64> {
            FerroProperty::register::<SetterTarget, _>("P26", 0.0)
        }

        pub fn p27_property() -> StyledProperty<f64> {
            FerroProperty::register::<SetterTarget, _>("P27", 0.0)
        }

        pub fn p28_property() -> StyledProperty<f64> {
            FerroProperty::register::<SetterTarget, _>("P28", 0.0)
        }

        pub fn p29_property() -> StyledProperty<f64> {
            FerroProperty::register::<SetterTarget, _>("P29", 0.0)
        }

        pub fn p30_property() -> StyledProperty<f64> {
            FerroProperty::register::<SetterTarget, _>("P30", 0.0)
        }

        pub fn p31_property() -> StyledProperty<f64> {
            FerroProperty::register::<SetterTarget, _>("P31", 0.0)
        }
    }
}

impl SetterTarget {
    fn construct() -> Self {
        Self { base: Control::construct() }
    }

    fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The properties, in the order the styles set them.
    fn properties() -> [&'static StyledProperty<f64>; 32] {
        [
            Self::p00_property(), Self::p01_property(), Self::p02_property(), Self::p03_property(),
            Self::p04_property(), Self::p05_property(), Self::p06_property(), Self::p07_property(),
            Self::p08_property(), Self::p09_property(), Self::p10_property(), Self::p11_property(),
            Self::p12_property(), Self::p13_property(), Self::p14_property(), Self::p15_property(),
            Self::p16_property(), Self::p17_property(), Self::p18_property(), Self::p19_property(),
            Self::p20_property(), Self::p21_property(), Self::p22_property(), Self::p23_property(),
            Self::p24_property(), Self::p25_property(), Self::p26_property(), Self::p27_property(),
            Self::p28_property(), Self::p29_property(), Self::p30_property(), Self::p31_property(),
        ]
    }
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("styling_subsystem", "SetterApply").min_iteration_time(200);
    let cases: [(&'static str, Value, &[usize]); 4] = [
        ("plain", Value::Plain, &[1, 8, 32]),
        ("binding", Value::Binding, &[8]),
        ("template", Value::Template, &[8]),
        ("under_local_value", Value::UnderLocalValue, &[8]),
    ];
    for (name, value, sizes) in cases {
        for &setters in sizes {
            let descriptor = class.benchmark_with_iteration_setup(
                name,
                format!("Controls={CONTROLS}, Setters={setters}"),
                move || SetterApply::new(setters, value),
                |b| b.prepare(),
                |b| b.style(),
            );
            if name == "plain" {
                descriptor.baseline();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn smoke() {
        crate::harness::smoke_class(super::register, "SetterApply");
    }
}

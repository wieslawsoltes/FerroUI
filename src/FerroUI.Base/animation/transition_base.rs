use crate::animation::easings::Easing;
use crate::animation::{Animatable, IClock, TimeSpan};
use crate::reactive::IDisposable;
use crate::{
    ferro_class, ferro_property, BoxedValue, DirectProperty, FerroObject, FerroObjectImpl, FerroObjectImplExt,
    FerroProperty, FerroPropertyChangedEventArgs,
};
use std::any::TypeId;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Defines how a property should be animated using a transition.
#[repr(C)]
pub struct TransitionBase {
    base: FerroObject,
    duration: Cell<TimeSpan>,
    delay: Cell<TimeSpan>,
    easing: RefCell<Easing>,
    prop: Cell<Option<&'static FerroProperty>>,
}

ferro_class! {
    TransitionBase: FerroObject, virtuals TransitionBaseImpl: FerroObjectImpl {
        /// Applies the transition to `control`: animates the transition's
        /// property from `old_value` to `new_value`.
        fn apply(this, control: &Animatable, clock: Rc<dyn IClock>, old_value: &BoxedValue, new_value: &BoxedValue) -> Rc<dyn IDisposable>;
        /// The type of the values the transition animates, which the value
        /// type of its property has to match.
        fn value_type(this) -> (TypeId, &'static str);
    }
}
crate::ferro_class_info!(TransitionBase { interfaces: [std::rc::Rc<dyn crate::animation::ITransition>] });

impl FerroObjectImpl for TransitionBase {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        if change.property() == Self::property_property().as_property() {
            if let Some(new_value) = change.get_new_value::<Option<&'static FerroProperty>>() {
                let (value_type, value_type_name) = this.value_type();
                if new_value.property_type() != value_type {
                    // The value is rejected after it was stored, as a
                    // failing change handler leaves it.
                    panic!(
                        "Invalid property type \"{}\" for this transition: {}.",
                        value_type_name,
                        this.get_type().name()
                    );
                }
            }
        }
    }
}

impl TransitionBaseImpl for TransitionBase {
    fn apply(
        _this: &Self,
        _control: &Animatable,
        _clock: Rc<dyn IClock>,
        _old_value: &BoxedValue,
        _new_value: &BoxedValue,
    ) -> Rc<dyn IDisposable> {
        panic!("TransitionBase is abstract: 'apply' must be implemented by the deriving class")
    }

    fn value_type(_this: &Self) -> (TypeId, &'static str) {
        panic!("TransitionBase is abstract: 'value_type' must be implemented by the deriving class")
    }
}

crate::ferro_properties! { impl TransitionBase {
    ferro_property!(
        /// Defines the `Duration` property.
        pub fn duration_property() -> DirectProperty<TransitionBase, TimeSpan> {
            FerroProperty::register_direct::<TransitionBase, _>(
                "Duration",
                |o| o.duration(),
                Some(|o, v| o.set_duration(v)),
                TimeSpan::ZERO,
            )
        }
    );

    ferro_property!(
        /// Defines the `Delay` property.
        pub fn delay_property() -> DirectProperty<TransitionBase, TimeSpan> {
            FerroProperty::register_direct::<TransitionBase, _>(
                "Delay",
                |o| o.delay(),
                Some(|o, v| o.set_delay(v)),
                TimeSpan::ZERO,
            )
        }
    );

    ferro_property!(
        /// Defines the `Easing` property.
        pub fn easing_property() -> DirectProperty<TransitionBase, Easing> {
            FerroProperty::register_direct::<TransitionBase, _>(
                "Easing",
                |o| o.easing(),
                Some(|o, v| o.set_easing(v)),
                Easing::default(),
            )
        }
    );

    ferro_property!(
        /// Defines the `Property` property.
        pub fn property_property() -> DirectProperty<TransitionBase, Option<&'static FerroProperty>> {
            FerroProperty::register_direct::<TransitionBase, _>(
                "Property",
                |o| o.property(),
                Some(|o, v| o.set_property(v)),
                None,
            )
        }
    );
} }

impl TransitionBase {
    /// Creates the class data; see [`FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: FerroObject::construct(),
            duration: Cell::new(TimeSpan::ZERO),
            delay: Cell::new(TimeSpan::ZERO),
            easing: RefCell::new(Easing::default()),
            prop: Cell::new(None),
        }
    }

    /// The duration of the transition.
    pub fn duration(&self) -> TimeSpan {
        self.duration.get()
    }

    pub fn set_duration(&self, value: TimeSpan) {
        self.set_and_raise_cell(Self::duration_property(), &self.duration, value);
    }

    /// The delay before the transition starts.
    pub fn delay(&self) -> TimeSpan {
        self.delay.get()
    }

    pub fn set_delay(&self, value: TimeSpan) {
        self.set_and_raise_cell(Self::delay_property(), &self.delay, value);
    }

    /// The easing class to be used.
    pub fn easing(&self) -> Easing {
        self.easing.borrow().clone()
    }

    pub fn set_easing(&self, value: impl Into<Easing>) {
        self.set_and_raise(Self::easing_property(), &self.easing, value.into());
    }

    /// The property to be animated.
    pub fn property(&self) -> Option<&'static FerroProperty> {
        self.prop.get()
    }

    pub fn set_property(&self, value: Option<&'static FerroProperty>) {
        self.set_and_raise_cell(Self::property_property(), &self.prop, value);
    }

    /// A description of the transition for diagnostics.
    pub fn debug_display(&self) -> String {
        let mut result = self.get_type().name().to_string();
        if let Some(property) = self.prop.get() {
            result.push_str(&format!(" Property = {property},"));
        }
        result.push_str(&format!(" Duration = {}", self.duration.get()));
        result
    }
}

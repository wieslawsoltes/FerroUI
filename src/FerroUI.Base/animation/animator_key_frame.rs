use crate::animation::{Animatable, Cue, IAnimationSetter, IAnimator, KeySpline};
use crate::reactive::{Disposable, IDisposable, IObservable, ObservableExt};
use crate::styling::SetterValue;
use crate::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, AnyValue, BoxedValue, DirectProperty, FerroObject,
    FerroObjectImpl, FerroProperty, PropertyValue, Ref,
};
use std::any::TypeId;
use std::cell::{Cell, RefCell};
use std::fmt;
use std::rc::Rc;

/// Creates the animator a key frame belongs to.
pub type AnimatorFactory = Rc<dyn Fn() -> Rc<dyn IAnimator>>;

/// Defines a key frame that is used for an animator.
#[repr(C)]
pub struct AnimatorKeyFrame {
    base: FerroObject,
    animator_type: Option<TypeId>,
    animator_factory: Option<AnimatorFactory>,
    cue: Cue,
    key_spline: Option<Ref<KeySpline>>,
    fill_before: Cell<bool>,
    fill_after: Cell<bool>,
    property: Cell<Option<&'static FerroProperty>>,
    value: RefCell<Option<BoxedValue>>,
}

ferro_class!(AnimatorKeyFrame: FerroObject);
ferro_impl_classes!(AnimatorKeyFrame: FerroObjectImpl);

crate::ferro_properties! { impl AnimatorKeyFrame {
    ferro_property!(
        /// Defines the `Value` property.
        pub fn value_property() -> DirectProperty<AnimatorKeyFrame, Option<BoxedValue>> {
            FerroProperty::register_direct::<AnimatorKeyFrame, _>(
                "Value",
                |k| k.value(),
                Some(|k, v| k.set_value(v)),
                None,
            )
        }
    );
} }

impl AnimatorKeyFrame {
    /// Creates a key frame for the animator type `animator_type`, which
    /// `animator_factory` creates.
    pub fn new(
        animator_type: Option<TypeId>,
        animator_factory: Option<AnimatorFactory>,
        cue: Cue,
        key_spline: Option<Ref<KeySpline>>,
    ) -> Ref<Self> {
        instantiate(Self {
            base: FerroObject::construct(),
            animator_type,
            animator_factory,
            cue,
            key_spline,
            fill_before: Cell::new(false),
            fill_after: Cell::new(false),
            property: Cell::new(None),
            value: RefCell::new(None),
        })
    }

    /// The type of the animator the key frame belongs to.
    pub fn animator_type(&self) -> Option<TypeId> {
        self.animator_type
    }

    /// The factory of the animator the key frame belongs to.
    pub fn animator_factory(&self) -> Option<AnimatorFactory> {
        self.animator_factory.clone()
    }

    /// The position of the key frame within an iteration.
    #[inline]
    pub fn cue(&self) -> Cue {
        self.cue
    }

    /// The spline applied to the progress towards this key frame.
    #[inline]
    pub fn key_spline(&self) -> Option<&Ref<KeySpline>> {
        self.key_spline.as_ref()
    }

    /// Whether the value of this key frame applies before its cue.
    #[inline]
    pub fn fill_before(&self) -> bool {
        self.fill_before.get()
    }

    pub fn set_fill_before(&self, value: bool) {
        self.fill_before.set(value)
    }

    /// Whether the value of this key frame applies after its cue.
    #[inline]
    pub fn fill_after(&self) -> bool {
        self.fill_after.get()
    }

    pub fn set_fill_after(&self, value: bool) {
        self.fill_after.set(value)
    }

    /// The property the key frame animates.
    pub fn property(&self) -> Option<&'static FerroProperty> {
        self.property.get()
    }

    /// The value of the key frame: a value of exactly the animated
    /// property's value type, or `None`.
    pub fn value(&self) -> Option<BoxedValue> {
        self.value.borrow().clone()
    }

    pub fn set_value(&self, value: Option<BoxedValue>) {
        self.set_and_raise(Self::value_property(), &self.value, value);
    }

    /// Calls `f` with the value of the key frame without cloning it.
    #[inline]
    pub(crate) fn with_value<R>(&self, f: impl FnOnce(Option<&dyn AnyValue>) -> R) -> R {
        let value = self.value.borrow();
        f(value.as_deref())
    }

    /// Takes the property and the value from `setter`. When the setter's
    /// value is a source of values, the key frame's value follows it for as
    /// long as the returned handle is not disposed.
    pub fn bind_setter(&self, setter: &dyn IAnimationSetter, target_control: &Animatable) -> Rc<dyn IDisposable> {
        self.property.set(setter.property());

        match setter.value() {
            Some(SetterValue::Binding(source)) => {
                let source: Rc<dyn IObservable<Option<BoxedValue>>> = source.select(Some);
                self.bind_direct(Self::value_property(), source)
            }
            Some(SetterValue::BindingBase(binding)) => {
                let anchor: Ref<FerroObject> = target_control.to_ref().upcast();
                let expression =
                    self.bind_binding_with_anchor(Self::value_property().as_property(), &*binding, Some(&anchor));
                Disposable::create(move || expression.dispose())
            }
            Some(SetterValue::Value(value)) => {
                self.set_value(Some(value));
                Disposable::empty()
            }
            Some(SetterValue::Template(_)) | None => {
                self.set_value(None);
                Disposable::empty()
            }
        }
    }

    /// The value of the key frame as a `T`. Panics when the key frame has no
    /// value or the value is of another type.
    pub fn get_typed_value<T: PropertyValue>(&self) -> T {
        self.with_value(|value| match value {
            None => panic!("KeyFrame value can't be null."),
            Some(value) => match value.downcast_ref::<T>() {
                Some(value) => value.clone(),
                None => panic!("KeyFrame value doesnt match property type."),
            },
        })
    }
}

impl fmt::Display for AnimatorKeyFrame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AnimatorKeyFrame")?;
        if let Some(property) = self.property.get() {
            write!(f, " Property = {property},")?;
        }
        write!(f, " Cue = {}", self.cue.cue_value())
    }
}

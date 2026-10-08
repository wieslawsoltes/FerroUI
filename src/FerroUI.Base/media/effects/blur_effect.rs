use crate::media::effects::{Effect, IImmutableEffect, ImmutableBlurEffect};
use crate::{
    ferro_class, ferro_property, instantiate, FerroObjectImpl, FerroProperty, Ref, StyledProperty,
};

/// An effect that blurs its content.
#[repr(C)]
pub struct BlurEffect {
    base: Effect,
}

ferro_class!(BlurEffect: Effect);
crate::ferro_class_info!(BlurEffect { new: BlurEffect::new });

impl FerroObjectImpl for BlurEffect {}

crate::ferro_properties! { impl BlurEffect {
    ferro_property!(pub fn radius_property() -> StyledProperty<f64> {
        FerroProperty::register::<BlurEffect, _>("Radius", 5.0)
    });
} }

impl BlurEffect {
    fn static_constructor() {
        Effect::affects_render::<BlurEffect>(&[Self::radius_property().as_property()]);
    }

    /// Creates the class data.
    pub fn construct() -> Self {
        Self { base: Effect::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The blur radius.
    pub fn radius(&self) -> f64 {
        self.get_value(Self::radius_property())
    }

    pub fn set_radius(&self, value: f64) {
        self.set_value(Self::radius_property(), value)
    }

    /// Creates an immutable clone of the effect.
    pub fn to_immutable(&self) -> std::sync::Arc<dyn IImmutableEffect> {
        std::sync::Arc::new(ImmutableBlurEffect::new(self.radius()))
    }
}

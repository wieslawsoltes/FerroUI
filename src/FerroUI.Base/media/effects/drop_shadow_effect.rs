use crate::media::effects::{
    Effect, IImmutableEffect, ImmutableDropShadowDirectionEffect, ImmutableDropShadowEffect,
};
use crate::media::{Color, Colors};
use crate::{
    ferro_class, ferro_property, instantiate, FerroObjectImpl, FerroProperty, Ref, StyledProperty,
};
use std::f64::consts::PI;

/// The properties shared by the drop shadow effects.
#[repr(C)]
pub struct DropShadowEffectBase {
    base: Effect,
}

ferro_class!(DropShadowEffectBase: Effect);

impl FerroObjectImpl for DropShadowEffectBase {}

crate::ferro_properties! { impl DropShadowEffectBase {
    ferro_property!(pub fn blur_radius_property() -> StyledProperty<f64> {
        FerroProperty::register::<DropShadowEffectBase, _>("BlurRadius", 5.0)
    });

    ferro_property!(pub fn color_property() -> StyledProperty<Color> {
        FerroProperty::register::<DropShadowEffectBase, _>("Color", Colors::BLACK)
    });

    ferro_property!(pub fn opacity_property() -> StyledProperty<f64> {
        FerroProperty::register::<DropShadowEffectBase, _>("Opacity", 1.0)
    });
} }

impl DropShadowEffectBase {
    fn static_constructor() {
        Effect::affects_render::<DropShadowEffectBase>(&[
            Self::blur_radius_property().as_property(),
            Self::color_property().as_property(),
            Self::opacity_property().as_property(),
        ]);
    }

    /// Creates the class data.
    pub fn construct() -> Self {
        Self { base: Effect::construct() }
    }

    /// The blur radius of the shadow.
    pub fn blur_radius(&self) -> f64 {
        self.get_value(Self::blur_radius_property())
    }

    pub fn set_blur_radius(&self, value: f64) {
        self.set_value(Self::blur_radius_property(), value)
    }

    /// The color of the shadow.
    pub fn color(&self) -> Color {
        self.get_value(Self::color_property())
    }

    pub fn set_color(&self, value: Color) {
        self.set_value(Self::color_property(), value)
    }

    /// The opacity of the shadow.
    pub fn opacity(&self) -> f64 {
        self.get_value(Self::opacity_property())
    }

    pub fn set_opacity(&self, value: f64) {
        self.set_value(Self::opacity_property(), value)
    }
}

/// An effect that draws a shadow behind its content, positioned by an
/// offset.
#[repr(C)]
pub struct DropShadowEffect {
    base: DropShadowEffectBase,
}

ferro_class!(DropShadowEffect: DropShadowEffectBase);
crate::ferro_class_info!(DropShadowEffect { new: DropShadowEffect::new });

impl FerroObjectImpl for DropShadowEffect {}

crate::ferro_properties! { impl DropShadowEffect {
    ferro_property!(pub fn offset_x_property() -> StyledProperty<f64> {
        FerroProperty::register::<DropShadowEffect, _>("OffsetX", 3.5355)
    });

    ferro_property!(pub fn offset_y_property() -> StyledProperty<f64> {
        FerroProperty::register::<DropShadowEffect, _>("OffsetY", 3.5355)
    });
} }

impl DropShadowEffect {
    fn static_constructor() {
        Effect::affects_render::<DropShadowEffect>(&[
            Self::offset_x_property().as_property(),
            Self::offset_y_property().as_property(),
        ]);
    }

    /// Creates the class data.
    pub fn construct() -> Self {
        Self { base: DropShadowEffectBase::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The horizontal offset of the shadow.
    pub fn offset_x(&self) -> f64 {
        self.get_value(Self::offset_x_property())
    }

    pub fn set_offset_x(&self, value: f64) {
        self.set_value(Self::offset_x_property(), value)
    }

    /// The vertical offset of the shadow.
    pub fn offset_y(&self) -> f64 {
        self.get_value(Self::offset_y_property())
    }

    pub fn set_offset_y(&self, value: f64) {
        self.set_value(Self::offset_y_property(), value)
    }

    /// Creates an immutable clone of the effect.
    pub fn to_immutable(&self) -> std::sync::Arc<dyn IImmutableEffect> {
        std::sync::Arc::new(ImmutableDropShadowEffect::new(
            self.offset_x(),
            self.offset_y(),
            self.blur_radius(),
            self.color(),
            self.opacity(),
        ))
    }
}

/// An effect that draws a shadow behind its content, positioned by a
/// direction and a depth.
#[repr(C)]
pub struct DropShadowDirectionEffect {
    base: DropShadowEffectBase,
}

ferro_class!(DropShadowDirectionEffect: DropShadowEffectBase);
crate::ferro_class_info!(DropShadowDirectionEffect { new: DropShadowDirectionEffect::new });

impl FerroObjectImpl for DropShadowDirectionEffect {}

crate::ferro_properties! { impl DropShadowDirectionEffect {
    ferro_property!(pub fn shadow_depth_property() -> StyledProperty<f64> {
        FerroProperty::register::<DropShadowDirectionEffect, _>("ShadowDepth", 5.0)
    });

    ferro_property!(pub fn direction_property() -> StyledProperty<f64> {
        FerroProperty::register::<DropShadowDirectionEffect, _>("Direction", 315.0)
    });
} }

impl DropShadowDirectionEffect {
    fn static_constructor() {
        Effect::affects_render::<DropShadowDirectionEffect>(&[
            Self::shadow_depth_property().as_property(),
            Self::direction_property().as_property(),
        ]);
    }

    /// Creates the class data.
    pub fn construct() -> Self {
        Self { base: DropShadowEffectBase::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The distance of the shadow from the content.
    pub fn shadow_depth(&self) -> f64 {
        self.get_value(Self::shadow_depth_property())
    }

    pub fn set_shadow_depth(&self, value: f64) {
        self.set_value(Self::shadow_depth_property(), value)
    }

    /// The direction of the shadow, in degrees.
    pub fn direction(&self) -> f64 {
        self.get_value(Self::direction_property())
    }

    pub fn set_direction(&self, value: f64) {
        self.set_value(Self::direction_property(), value)
    }

    /// The horizontal offset of the shadow.
    pub fn offset_x(&self) -> f64 {
        (self.direction() * PI / 180.0).cos() * self.shadow_depth()
    }

    /// The vertical offset of the shadow.
    pub fn offset_y(&self) -> f64 {
        (self.direction() * PI / 180.0).sin() * self.shadow_depth()
    }

    /// Creates an immutable clone of the effect.
    ///
    /// As in the reference implementation, the computed offsets are handed
    /// to the immutable effect as its direction and depth.
    pub fn to_immutable(&self) -> std::sync::Arc<dyn IImmutableEffect> {
        std::sync::Arc::new(ImmutableDropShadowDirectionEffect::new(
            self.offset_x(),
            self.offset_y(),
            self.blur_radius(),
            self.color(),
            self.opacity(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;
    // Not from upstream: there are no upstream unit tests for these classes.
    use super::*;
    use std::cell::Cell;
    use crate::media::effects::{BlurEffect, EffectExtensions, IEffect};

    #[test]
    fn changing_a_property_raises_invalidated() {
        let count = Rc::new(Cell::new(0));

        let blur = BlurEffect::new();
        let c = count.clone();
        let subscription = blur.invalidated(move || c.set(c.get() + 1));
        blur.set_radius(10.0);
        assert_eq!(1, count.get());
        subscription.dispose();
        blur.set_radius(11.0);
        assert_eq!(1, count.get());

        let shadow = DropShadowEffect::new();
        let c = count.clone();
        shadow.invalidated(move || c.set(c.get() + 1));
        shadow.set_offset_x(1.0);
        shadow.set_blur_radius(2.0);
        shadow.set_color(Colors::RED);
        assert_eq!(4, count.get());

        let direction = DropShadowDirectionEffect::new();
        let c = count.clone();
        direction.invalidated(move || c.set(c.get() + 1));
        direction.set_direction(0.0);
        direction.set_opacity(0.5);
        assert_eq!(6, count.get());
    }

    #[test]
    fn defaults_interfaces_and_immutable_copies() {
        let shadow = DropShadowEffect::new();
        assert_eq!(3.5355, shadow.offset_x());
        assert_eq!(5.0, shadow.blur_radius());
        assert_eq!(Colors::BLACK, shadow.color());

        let effect: Rc<dyn IEffect> = shadow.clone().into();
        assert!(effect.as_blur_effect().is_none());
        assert!(effect.as_direction_drop_shadow_effect().is_none());
        assert_eq!(3.5355, effect.as_drop_shadow_effect().unwrap().offset_y());
        assert!(effect.as_object().unwrap().is::<DropShadowEffect>());

        let immutable = EffectExtensions::to_immutable(&effect);
        assert!(immutable.equals(Some(&*effect)));
        shadow.set_opacity(0.5);
        assert!(!immutable.equals(Some(&*effect)));
        assert!(!immutable.equals(None));
        // An immutable effect converts to an equal one (a value in a handle
        // of its own, where the original returns the same object).
        let as_effect: Rc<dyn IEffect> = Rc::new(*immutable.as_any().downcast_ref::<ImmutableDropShadowEffect>().unwrap());
        let same = EffectExtensions::to_immutable(&as_effect);
        assert!(same.equals(Some(&*as_effect)));

        let direction = DropShadowDirectionEffect::new();
        direction.set_direction(0.0);
        direction.set_shadow_depth(4.0);
        assert_eq!(4.0, direction.offset_x());
        assert_eq!(0.0, direction.offset_y());
        let effect: Rc<dyn IEffect> = direction.into();
        assert_eq!(0.0, effect.as_direction_drop_shadow_effect().unwrap().direction());

        let blur: Rc<dyn IEffect> = BlurEffect::new().into();
        assert_eq!(5.0, blur.as_blur_effect().unwrap().radius());
        let other: Rc<dyn IEffect> = BlurEffect::new().into();
        assert!(*blur != *other);
        assert!(*blur == *blur.clone());
    }
}

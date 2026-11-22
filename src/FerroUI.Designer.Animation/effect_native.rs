use super::{AnimationEffect, AnimationError, AnimationValue, Result, ValueKind};
use ferroui_base::media::{Effect, IEffect};
use ferroui_base::{BoxedValue, FerroProperty};
use std::{any::TypeId, rc::Rc};

impl AnimationValue {
    pub fn parse_native_literal(source: &str, kind: ValueKind) -> Result<Self> {
        if kind == ValueKind::Effect {
            return AnimationEffect::parse_native(source).map(Self::Effect);
        }
        Self::parse(source, kind)
    }
}

impl AnimationEffect {
    pub fn is_effect_property(property: &FerroProperty) -> bool {
        property.property_type() == TypeId::of::<Option<Rc<dyn IEffect>>>()
    }

    pub fn from_boxed(value: &BoxedValue) -> Option<Self> {
        Self::from_native(
            value
                .downcast_ref::<Option<Rc<dyn IEffect>>>()?
                .as_deref()?,
        )
    }

    /// Use the runtime converter for the literal, including named and functional shadow colors.
    pub fn parse_native(source: &str) -> Result<Self> {
        let native = Effect::parse(source).map_err(|error| AnimationError(error.to_string()))?;
        let value = Self::from_native(&*native)
            .ok_or_else(|| AnimationError("Choose a literal blur or offset drop shadow".into()))?;
        value.validate()?;
        Ok(value)
    }

    pub fn from_native(effect: &dyn IEffect) -> Option<Self> {
        if let Some(blur) = effect.as_blur_effect() {
            return Some(Self::Blur {
                radius: blur.radius(),
            });
        }
        let shadow = effect.as_drop_shadow_effect().filter(|shadow| {
            shadow.opacity() == 1.0 && shadow.as_direction_drop_shadow_effect().is_none()
        })?;
        Some(Self::DropShadow {
            offset_x: shadow.offset_x(),
            offset_y: shadow.offset_y(),
            blur_radius: shadow.blur_radius(),
            argb: shadow.color().to_uint32().to_be_bytes(),
        })
    }
}

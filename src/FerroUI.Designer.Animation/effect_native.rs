use super::{AnimationEffect, AnimationError, AnimationValue, EffectForm, Result, ValueKind};
use ferroui_base::media::{
    Color, Effect, IEffect, ImmutableBlurEffect, ImmutableDropShadowDirectionEffect,
    ImmutableDropShadowEffect,
};
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

    /// Use the runtime color and effect converters for authored values.
    pub fn parse_native(source: &str) -> Result<Self> {
        if source.trim_start().starts_with('<') {
            return Self::parse_object(source, Some(Self::resolve_color));
        }
        let native = Effect::parse(source).map_err(|error| AnimationError(error.to_string()))?;
        let value = Self::from_native(&*native)
            .ok_or_else(|| AnimationError("Choose a literal blur or offset drop shadow".into()))?;
        value.validate()?;
        Ok(value)
    }

    pub fn resolve_color(source: &str) -> Option<[u8; 4]> {
        Color::try_parse(source).map(|color| color.to_uint32().to_be_bytes())
    }

    pub fn to_native(self) -> Rc<dyn IEffect> {
        match self {
            Self::Blur { radius, .. } => Rc::new(ImmutableBlurEffect::new(radius)),
            Self::DropShadow {
                offset_x,
                offset_y,
                blur_radius,
                argb,
                opacity,
                ..
            } => Rc::new(ImmutableDropShadowEffect::new(
                offset_x,
                offset_y,
                blur_radius,
                Color::from_uint32(u32::from_be_bytes(argb)),
                opacity,
            )),
            Self::DirectionalDropShadow {
                direction,
                shadow_depth,
                blur_radius,
                argb,
                opacity,
                ..
            } => Rc::new(ImmutableDropShadowDirectionEffect::new(
                direction,
                shadow_depth,
                blur_radius,
                Color::from_uint32(u32::from_be_bytes(argb)),
                opacity,
            )),
        }
    }

    pub fn from_native(effect: &dyn IEffect) -> Option<Self> {
        if let Some(blur) = effect.as_blur_effect() {
            return Some(Self::Blur {
                radius: blur.radius(),
                form: EffectForm::Literal,
            });
        }
        let shadow = effect.as_drop_shadow_effect()?;
        let argb = shadow.color().to_uint32().to_be_bytes();
        let blur_radius = shadow.blur_radius();
        let opacity = shadow.opacity();
        if let Some(directional) = shadow.as_direction_drop_shadow_effect() {
            return Some(Self::DirectionalDropShadow {
                direction: directional.direction(),
                shadow_depth: directional.shadow_depth(),
                blur_radius,
                argb,
                opacity,
                form: EffectForm::Object,
            });
        }
        let value = Self::DropShadow {
            offset_x: shadow.offset_x(),
            offset_y: shadow.offset_y(),
            blur_radius,
            argb,
            opacity,
            form: EffectForm::Object,
        };
        Some(value.with_form(EffectForm::Literal))
    }
}

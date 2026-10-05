use crate::media::effects::{
    DropShadowDirectionEffect, DropShadowEffect, DropShadowEffectBase, Effect, IEffect, IImmutableEffect,
};
use crate::media::ref_adapter::RefAdapter;
use crate::media::Color;
use crate::{ObjectType, Upcast};
use std::any::Any;
use std::f64::consts::PI;
use std::rc::Rc;

/// An effect that draws a shadow behind its content.
pub trait IDropShadowEffect: IEffect {
    /// The horizontal offset of the shadow.
    fn offset_x(&self) -> f64;

    /// The vertical offset of the shadow.
    fn offset_y(&self) -> f64;

    /// The blur radius of the shadow.
    fn blur_radius(&self) -> f64;

    /// The color of the shadow.
    fn color(&self) -> Color;

    /// The opacity of the shadow.
    fn opacity(&self) -> f64;
}

/// A drop shadow described by a direction and a depth.
pub trait IDirectionDropShadowEffect: IDropShadowEffect {
    /// The direction of the shadow, in degrees.
    fn direction(&self) -> f64;

    /// The distance of the shadow from the content.
    fn shadow_depth(&self) -> f64;
}

fn drop_shadow_equals(this: &dyn IDropShadowEffect, other: Option<&dyn IEffect>) -> bool {
    other.and_then(|other| other.as_drop_shadow_effect()).is_some_and(|d| {
        d.offset_x() == this.offset_x()
            && d.offset_y() == this.offset_y()
            && d.blur_radius() == this.blur_radius()
            && d.color() == this.color()
            && d.opacity() == this.opacity()
    })
}

/// An immutable drop shadow effect.
#[derive(Clone, Copy, Debug)]
pub struct ImmutableDropShadowEffect {
    offset_x: f64,
    offset_y: f64,
    blur_radius: f64,
    color: Color,
    opacity: f64,
}

impl ImmutableDropShadowEffect {
    pub fn new(offset_x: f64, offset_y: f64, blur_radius: f64, color: Color, opacity: f64) -> Self {
        Self { offset_x, offset_y, blur_radius, color, opacity }
    }

    /// The horizontal offset of the shadow.
    #[inline]
    pub fn offset_x(&self) -> f64 {
        self.offset_x
    }

    /// The vertical offset of the shadow.
    #[inline]
    pub fn offset_y(&self) -> f64 {
        self.offset_y
    }

    /// The blur radius of the shadow.
    #[inline]
    pub fn blur_radius(&self) -> f64 {
        self.blur_radius
    }

    /// The color of the shadow.
    #[inline]
    pub fn color(&self) -> Color {
        self.color
    }

    /// The opacity of the shadow.
    #[inline]
    pub fn opacity(&self) -> f64 {
        self.opacity
    }
}

impl IEffect for ImmutableDropShadowEffect {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_drop_shadow_effect(&self) -> Option<&dyn IDropShadowEffect> {
        Some(self)
    }

    fn as_immutable_effect(&self) -> Option<&dyn IImmutableEffect> {
        Some(self)
    }

    fn into_immutable_effect(self: Rc<Self>) -> Option<Rc<dyn IImmutableEffect>> {
        Some(self)
    }
}

impl IDropShadowEffect for ImmutableDropShadowEffect {
    #[inline]
    fn offset_x(&self) -> f64 {
        self.offset_x
    }

    #[inline]
    fn offset_y(&self) -> f64 {
        self.offset_y
    }

    #[inline]
    fn blur_radius(&self) -> f64 {
        self.blur_radius
    }

    #[inline]
    fn color(&self) -> Color {
        self.color
    }

    #[inline]
    fn opacity(&self) -> f64 {
        self.opacity
    }
}

impl IImmutableEffect for ImmutableDropShadowEffect {
    fn equals(&self, other: Option<&dyn IEffect>) -> bool {
        drop_shadow_equals(self, other)
    }
}

/// An immutable drop shadow effect described by a direction and a depth.
#[derive(Clone, Copy, Debug)]
pub struct ImmutableDropShadowDirectionEffect {
    direction: f64,
    shadow_depth: f64,
    blur_radius: f64,
    color: Color,
    opacity: f64,
}

impl ImmutableDropShadowDirectionEffect {
    pub fn new(direction: f64, shadow_depth: f64, blur_radius: f64, color: Color, opacity: f64) -> Self {
        Self { direction, shadow_depth, blur_radius, color, opacity }
    }

    /// The horizontal offset of the shadow.
    pub fn offset_x(&self) -> f64 {
        (self.direction * PI / 180.0).cos() * self.shadow_depth
    }

    /// The vertical offset of the shadow.
    pub fn offset_y(&self) -> f64 {
        (self.direction * PI / 180.0).sin() * self.shadow_depth
    }

    /// The direction of the shadow, in degrees.
    #[inline]
    pub fn direction(&self) -> f64 {
        self.direction
    }

    /// The distance of the shadow from the content.
    #[inline]
    pub fn shadow_depth(&self) -> f64 {
        self.shadow_depth
    }

    /// The blur radius of the shadow.
    #[inline]
    pub fn blur_radius(&self) -> f64 {
        self.blur_radius
    }

    /// The color of the shadow.
    #[inline]
    pub fn color(&self) -> Color {
        self.color
    }

    /// The opacity of the shadow.
    #[inline]
    pub fn opacity(&self) -> f64 {
        self.opacity
    }
}

impl IEffect for ImmutableDropShadowDirectionEffect {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_drop_shadow_effect(&self) -> Option<&dyn IDropShadowEffect> {
        Some(self)
    }

    fn as_direction_drop_shadow_effect(&self) -> Option<&dyn IDirectionDropShadowEffect> {
        Some(self)
    }

    fn as_immutable_effect(&self) -> Option<&dyn IImmutableEffect> {
        Some(self)
    }

    fn into_immutable_effect(self: Rc<Self>) -> Option<Rc<dyn IImmutableEffect>> {
        Some(self)
    }
}

impl IDropShadowEffect for ImmutableDropShadowDirectionEffect {
    fn offset_x(&self) -> f64 {
        ImmutableDropShadowDirectionEffect::offset_x(self)
    }

    fn offset_y(&self) -> f64 {
        ImmutableDropShadowDirectionEffect::offset_y(self)
    }

    #[inline]
    fn blur_radius(&self) -> f64 {
        self.blur_radius
    }

    #[inline]
    fn color(&self) -> Color {
        self.color
    }

    #[inline]
    fn opacity(&self) -> f64 {
        self.opacity
    }
}

impl IDirectionDropShadowEffect for ImmutableDropShadowDirectionEffect {
    #[inline]
    fn direction(&self) -> f64 {
        self.direction
    }

    #[inline]
    fn shadow_depth(&self) -> f64 {
        self.shadow_depth
    }
}

impl IImmutableEffect for ImmutableDropShadowDirectionEffect {
    fn equals(&self, other: Option<&dyn IEffect>) -> bool {
        drop_shadow_equals(self, other)
    }
}

impl<T: ObjectType + Upcast<Effect>> IDropShadowEffect for RefAdapter<T> {
    fn offset_x(&self) -> f64 {
        match self.object().downcast_ref::<DropShadowEffect>() {
            Some(shadow) => shadow.offset_x(),
            None => self.class::<DropShadowDirectionEffect>().offset_x(),
        }
    }

    fn offset_y(&self) -> f64 {
        match self.object().downcast_ref::<DropShadowEffect>() {
            Some(shadow) => shadow.offset_y(),
            None => self.class::<DropShadowDirectionEffect>().offset_y(),
        }
    }

    #[inline]
    fn blur_radius(&self) -> f64 {
        self.class::<DropShadowEffectBase>().blur_radius()
    }

    #[inline]
    fn color(&self) -> Color {
        self.class::<DropShadowEffectBase>().color()
    }

    #[inline]
    fn opacity(&self) -> f64 {
        self.class::<DropShadowEffectBase>().opacity()
    }
}

impl<T: ObjectType + Upcast<Effect>> IDirectionDropShadowEffect for RefAdapter<T> {
    #[inline]
    fn direction(&self) -> f64 {
        self.class::<DropShadowDirectionEffect>().direction()
    }

    #[inline]
    fn shadow_depth(&self) -> f64 {
        self.class::<DropShadowDirectionEffect>().shadow_depth()
    }
}

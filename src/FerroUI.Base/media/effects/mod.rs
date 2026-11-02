//! Bitmap effects applied to visuals and drawings: the built-in blur and
//! drop shadow, and custom effects (#455, docs/porting/custom-effects.md).

mod blur_effect;
mod custom_effect;
pub(crate) mod custom_effect_graph;
mod custom_effect_registry;
#[cfg(test)]
mod custom_effect_tests;
mod drop_shadow_effect;
mod effect;
mod effect_extensions;
mod i_blur_effect;
mod i_drop_shadow_effect;
mod i_effect;
mod immutable_luminance_to_alpha_effect;
pub mod library;

pub use blur_effect::BlurEffect;
pub use custom_effect::{
    CustomEffect, CustomEffectClass, EffectParameterValue, ImmutableCustomEffect, CUSTOM_EFFECT_CONTENT,
};
pub use custom_effect_registry::{CustomEffectRegistrationError, CustomEffectRegistry};
pub use drop_shadow_effect::{DropShadowDirectionEffect, DropShadowEffect, DropShadowEffectBase};
pub use effect::Effect;
pub use effect_extensions::EffectExtensions;
pub use i_blur_effect::{IBlurEffect, ImmutableBlurEffect};
pub use i_drop_shadow_effect::{
    IDirectionDropShadowEffect, IDropShadowEffect, ImmutableDropShadowDirectionEffect, ImmutableDropShadowEffect,
};
pub use i_effect::{IEffect, IImmutableEffect, IMutableEffect};
pub use immutable_luminance_to_alpha_effect::ImmutableLuminanceToAlphaEffect;

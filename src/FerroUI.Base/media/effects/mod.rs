//! Bitmap effects applied to visuals and drawings: the built-in blur and
//! drop shadow, and custom effects (#455, docs/porting/custom-effects.md):
//! effect graphs and runtime shaders, and the images shaders sample.

mod blur_effect;
mod custom_effect;
pub(crate) mod custom_effect_graph;
mod custom_effect_registry;
mod custom_effect_shader;
#[cfg(test)]
mod custom_effect_shader_tests;
#[cfg(test)]
mod custom_effect_tests;
mod drop_shadow_effect;
mod effect;
mod effect_extensions;
mod effect_image;
#[cfg(test)]
mod effect_image_tests;
mod i_blur_effect;
mod i_drop_shadow_effect;
mod i_effect;
mod immutable_luminance_to_alpha_effect;
pub mod library;
#[cfg(test)]
mod library_tests;

pub use blur_effect::BlurEffect;
pub use custom_effect::{
    CustomEffect, CustomEffectClass, EffectParameterValue, ImmutableCustomEffect, CUSTOM_EFFECT_CONTENT,
};
pub use custom_effect_registry::{CustomEffectRegistrationError, CustomEffectRegistry};
pub use custom_effect_shader::{
    shader_uniform_name, shader_uniform_name_matches, ShaderEffectDescription, ShaderEffectMode, ShaderEffectPadding,
    ShaderUniformType, ShaderUniformValue, MAX_SHADER_CHILDREN, SHADER_LAYER_SCALE, SHADER_LAYER_SIZE,
    SHADER_WGSL_CHILDREN_BINDING, SHADER_WGSL_CONTENT_BINDING, SHADER_WGSL_FRAGMENT_ENTRY, SHADER_WGSL_PASS_BINDING,
    SHADER_WGSL_UNIFORMS_BINDING, SHADER_WGSL_VERTEX_ENTRY,
};
pub use drop_shadow_effect::{DropShadowDirectionEffect, DropShadowEffect, DropShadowEffectBase};
pub use effect::Effect;
pub use effect_extensions::EffectExtensions;
pub use effect_image::{EffectImage, EffectImagePixels, EffectImageWeak};
pub(crate) use effect_image::sample_image;
pub use i_blur_effect::{IBlurEffect, ImmutableBlurEffect};
pub use i_drop_shadow_effect::{
    IDirectionDropShadowEffect, IDropShadowEffect, ImmutableDropShadowDirectionEffect, ImmutableDropShadowEffect,
};
pub use i_effect::{IEffect, IImmutableEffect, IMutableEffect};
pub use immutable_luminance_to_alpha_effect::ImmutableLuminanceToAlphaEffect;

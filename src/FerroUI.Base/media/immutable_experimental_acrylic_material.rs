use crate::media::{AcrylicBackgroundSource, Color, IExperimentalAcrylicMaterial};
use std::any::Any;
use std::hash::{Hash, Hasher};

/// An immutable acrylic-like material.
#[derive(Clone, Copy, Debug, Default)]
pub struct ImmutableExperimentalAcrylicMaterial {
    background_source: AcrylicBackgroundSource,
    tint_color: Color,
    material_color: Color,
    tint_opacity: f64,
    fallback_color: Color,
}

impl ImmutableExperimentalAcrylicMaterial {
    /// Creates an immutable copy of `brush`.
    pub fn new(brush: &dyn IExperimentalAcrylicMaterial) -> Self {
        Self {
            background_source: brush.background_source(),
            tint_color: brush.tint_color(),
            tint_opacity: brush.tint_opacity(),
            fallback_color: brush.fallback_color(),
            material_color: brush.material_color(),
        }
    }

    /// The background source mode.
    #[inline]
    pub fn background_source(&self) -> AcrylicBackgroundSource {
        self.background_source
    }

    /// The tint color of the material.
    #[inline]
    pub fn tint_color(&self) -> Color {
        self.tint_color
    }

    /// The effective material color.
    #[inline]
    pub fn material_color(&self) -> Color {
        self.material_color
    }

    /// The tint opacity of the material.
    #[inline]
    pub fn tint_opacity(&self) -> f64 {
        self.tint_opacity
    }

    /// The fallback color if acrylic is unavailable.
    #[inline]
    pub fn fallback_color(&self) -> Color {
        self.fallback_color
    }

    /// Whether the material has the same parameters as `other`.
    pub fn equals(&self, other: &ImmutableExperimentalAcrylicMaterial) -> bool {
        self.tint_color == other.tint_color
            && self.tint_opacity == other.tint_opacity
            && self.background_source == other.background_source
            && self.fallback_color == other.fallback_color
            && self.material_color == other.material_color
    }

    /// The tint color the material is drawn with.
    pub fn get_effective_tint_color(&self) -> Color {
        self.tint_color
    }
}

impl PartialEq for ImmutableExperimentalAcrylicMaterial {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.equals(other)
    }
}

impl Hash for ImmutableExperimentalAcrylicMaterial {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.tint_color.hash(state);
        self.tint_opacity.to_bits().hash(state);
        self.background_source.hash(state);
        self.fallback_color.hash(state);
        self.material_color.hash(state);
    }
}

impl IExperimentalAcrylicMaterial for ImmutableExperimentalAcrylicMaterial {
    #[inline]
    fn background_source(&self) -> AcrylicBackgroundSource {
        self.background_source
    }

    #[inline]
    fn tint_color(&self) -> Color {
        self.tint_color
    }

    #[inline]
    fn tint_opacity(&self) -> f64 {
        self.tint_opacity
    }

    #[inline]
    fn material_color(&self) -> Color {
        self.material_color
    }

    #[inline]
    fn fallback_color(&self) -> Color {
        self.fallback_color
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

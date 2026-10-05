/// The opacity compensation a platform needs for each transparency level in
/// order for acrylic materials to look the way they are defined.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AcrylicPlatformCompensationLevels {
    transparent_level: f64,
    blur_level: f64,
    acrylic_blur_level: f64,
}

impl AcrylicPlatformCompensationLevels {
    /// Creates the compensation levels.
    pub const fn new(transparent: f64, blurred: f64, acrylic: f64) -> Self {
        Self { transparent_level: transparent, blur_level: blurred, acrylic_blur_level: acrylic }
    }

    /// The compensation for the transparent level.
    pub const fn transparent_level(&self) -> f64 {
        self.transparent_level
    }

    /// The compensation for the blur level.
    pub const fn blur_level(&self) -> f64 {
        self.blur_level
    }

    /// The compensation for the acrylic blur level.
    pub const fn acrylic_blur_level(&self) -> f64 {
        self.acrylic_blur_level
    }
}

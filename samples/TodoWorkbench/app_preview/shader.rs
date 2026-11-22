use ferroui_base::media::effects::{
    library::SpotlightShaderEffect, CustomEffect, CustomEffectRegistry, EffectParameterValue,
    ShaderEffectDescription,
};

/// A project effect with a slower start than the library spotlight.
#[derive(Clone, Debug, Default, PartialEq)]
struct ReviewSpotlight(SpotlightShaderEffect);

impl CustomEffect for ReviewSpotlight {
    const NAME: &'static str = "review-spotlight";

    fn for_each_parameter(&self, visitor: &mut dyn FnMut(&'static str, EffectParameterValue)) {
        self.0.for_each_parameter(visitor);
    }

    fn set_parameter(&mut self, name: &str, value: EffectParameterValue) -> bool {
        self.0.set_parameter(name, value)
    }

    fn interpolate(&self, to: &Self, progress: f64) -> Self {
        Self(self.0.interpolate(&to.0, progress * progress))
    }

    fn neutral() -> Option<Self> {
        SpotlightShaderEffect::neutral().map(Self)
    }

    fn shader() -> Option<ShaderEffectDescription> {
        SpotlightShaderEffect::shader()
    }

    fn cpu_reference(&self, pixels: &mut [u8], width: usize, height: usize, scale: f64) -> bool {
        self.0.cpu_reference(pixels, width, height, scale)
    }
}

pub(super) fn register() {
    CustomEffectRegistry::register::<ReviewSpotlight>().expect("unique Todo Workbench shader name");
}

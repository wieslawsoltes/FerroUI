use crate::media::ref_adapter::RefAdapter;
use crate::media::{AcrylicBackgroundSource, Color, IExperimentalAcrylicMaterial, ImmutableExperimentalAcrylicMaterial};
use crate::reactive::{Disposable, IDisposable};
use crate::utilities::HandlerList;
use crate::{
    ferro_class, ferro_property, instantiate, FerroObject, FerroObjectImpl, FerroProperty,
    ObjectType, Ref, StyledProperty, Upcast,
};
use std::cell::Cell;
use std::rc::Rc;

/// An experimental acrylic-like material.
#[repr(C)]
pub struct ExperimentalAcrylicMaterial {
    base: FerroObject,
    effective_tint_color: Cell<Color>,
    effective_luminosity_color: Cell<Color>,
    invalidated: HandlerList<dyn Fn()>,
}

ferro_class!(ExperimentalAcrylicMaterial: FerroObject);
crate::ferro_class_info!(ExperimentalAcrylicMaterial { new: ExperimentalAcrylicMaterial::new, interfaces: [std::rc::Rc<dyn crate::media::IExperimentalAcrylicMaterial>] });

crate::ferro_impl_classes!(ExperimentalAcrylicMaterial: FerroObjectImpl);

crate::ferro_properties! { impl ExperimentalAcrylicMaterial {
    ferro_property!(
        /// Defines the `TintColor` property.
        pub fn tint_color_property() -> StyledProperty<Color> {
            FerroProperty::register::<ExperimentalAcrylicMaterial, _>("TintColor", Color::default())
        }
    );

    ferro_property!(
        /// Defines the `BackgroundSource` property.
        pub fn background_source_property() -> StyledProperty<AcrylicBackgroundSource> {
            FerroProperty::register::<ExperimentalAcrylicMaterial, _>("BackgroundSource", AcrylicBackgroundSource::None)
        }
    );

    ferro_property!(
        /// Defines the `TintOpacity` property.
        pub fn tint_opacity_property() -> StyledProperty<f64> {
            FerroProperty::register::<ExperimentalAcrylicMaterial, _>("TintOpacity", 0.8)
        }
    );

    ferro_property!(
        /// Defines the `MaterialOpacity` property.
        pub fn material_opacity_property() -> StyledProperty<f64> {
            FerroProperty::register::<ExperimentalAcrylicMaterial, _>("MaterialOpacity", 0.5)
        }
    );

    ferro_property!(
        /// Defines the `PlatformTransparencyCompensationLevel` property.
        pub fn platform_transparency_compensation_level_property() -> StyledProperty<f64> {
            FerroProperty::register::<ExperimentalAcrylicMaterial, _>("PlatformTransparencyCompensationLevel", 0.0)
        }
    );

    ferro_property!(
        /// Defines the `FallbackColor` property.
        pub fn fallback_color_property() -> StyledProperty<Color> {
            FerroProperty::register::<ExperimentalAcrylicMaterial, _>("FallbackColor", Color::default())
        }
    );
} }

impl ExperimentalAcrylicMaterial {
    fn static_constructor() {
        Self::affects_render::<ExperimentalAcrylicMaterial>(&[
            Self::tint_color_property().as_property(),
            Self::background_source_property().as_property(),
            Self::tint_opacity_property().as_property(),
            Self::material_opacity_property().as_property(),
            Self::platform_transparency_compensation_level_property().as_property(),
        ]);

        for property in [
            Self::tint_color_property().as_property(),
            Self::tint_opacity_property().as_property(),
            Self::material_opacity_property().as_property(),
            Self::platform_transparency_compensation_level_property().as_property(),
        ] {
            property.changed().add_class_handler::<ExperimentalAcrylicMaterial>(|b, _| {
                b.effective_tint_color.set(Self::get_effective_tint_color(b.tint_color(), b.tint_opacity()));
                b.effective_luminosity_color.set(b.get_effective_luminosity_color());
            });
        }
    }

    /// Creates the class data.
    pub fn construct() -> Self {
        Self {
            base: FerroObject::construct(),
            effective_tint_color: Cell::new(Color::default()),
            effective_luminosity_color: Cell::new(Color::default()),
            invalidated: HandlerList::new(),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Raised when the material changes.
    pub fn invalidated(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        let token = self.invalidated.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.invalidated.remove(token);
            }
        })
    }

    /// The background source mode.
    pub fn background_source(&self) -> AcrylicBackgroundSource {
        self.get_value(Self::background_source_property())
    }

    pub fn set_background_source(&self, value: AcrylicBackgroundSource) {
        self.set_value(Self::background_source_property(), value)
    }

    /// The tint color of the material.
    pub fn tint_color(&self) -> Color {
        self.get_value(Self::tint_color_property())
    }

    pub fn set_tint_color(&self, value: Color) {
        self.set_value(Self::tint_color_property(), value)
    }

    /// The tint opacity of the material.
    pub fn tint_opacity(&self) -> f64 {
        self.get_value(Self::tint_opacity_property())
    }

    pub fn set_tint_opacity(&self, value: f64) {
        self.set_value(Self::tint_opacity_property(), value)
    }

    /// The fallback color if acrylic is unavailable.
    pub fn fallback_color(&self) -> Color {
        self.get_value(Self::fallback_color_property())
    }

    pub fn set_fallback_color(&self, value: Color) {
        self.set_value(Self::fallback_color_property(), value)
    }

    /// The opacity of the material layer.
    pub fn material_opacity(&self) -> f64 {
        self.get_value(Self::material_opacity_property())
    }

    pub fn set_material_opacity(&self, value: f64) {
        self.set_value(Self::material_opacity_property(), value)
    }

    /// How much of the platform's own transparency is compensated for.
    pub fn platform_transparency_compensation_level(&self) -> f64 {
        self.get_value(Self::platform_transparency_compensation_level_property())
    }

    pub fn set_platform_transparency_compensation_level(&self, value: f64) {
        self.set_value(Self::platform_transparency_compensation_level_property(), value)
    }

    /// The material color reported through [`IExperimentalAcrylicMaterial`].
    pub(crate) fn effective_luminosity_color(&self) -> Color {
        self.effective_luminosity_color.get()
    }

    /// The tint color reported through [`IExperimentalAcrylicMaterial`].
    pub(crate) fn effective_tint_color(&self) -> Color {
        self.effective_tint_color.get()
    }

    fn get_effective_tint_color(tint_color: Color, tint_opacity: f64) -> Color {
        // Update the tint color's alpha with the combined opacity value.
        let tint_opacity_modifier = Self::get_tint_opacity_modifier(tint_color);

        Color::new(
            (255.0 * ((255.0 / tint_color.a as f64) * tint_opacity) * tint_opacity_modifier) as u8,
            tint_color.r,
            tint_color.g,
            tint_color.b,
        )
    }

    fn get_tint_opacity_modifier(tint_color: Color) -> f64 {
        // This method suppresses the maximum allowable tint opacity depending
        // on the luminosity and saturation of a color by compressing the range
        // of allowable values - for example, a user-defined value of 100% will
        // be mapped to 45% for pure white (100% luminosity), 85% for pure
        // black (0% luminosity), and 90% for pure gray (50% luminosity). The
        // intensity of the effect increases linearly as luminosity deviates
        // from 50%. After this effect is calculated, we cancel it out linearly
        // as saturation increases from zero.

        // Mid point of the HsvV range that these calculations are based on.
        // This is here for easy tuning.
        const MID_POINT: f64 = 0.5;

        const WHITE_MAX_OPACITY: f64 = 0.2; // 100% luminosity
        const MID_POINT_MAX_OPACITY: f64 = 0.45; // 50% luminosity
        const BLACK_MAX_OPACITY: f64 = 0.45; // 0% luminosity

        let hsv = tint_color.to_hsv();

        let mut opacity_modifier = MID_POINT_MAX_OPACITY;

        if hsv.v != MID_POINT {
            // Determine maximum suppression amount
            let mut lowest_max_opacity = MID_POINT_MAX_OPACITY;
            let mut max_deviation = MID_POINT;

            if hsv.v > MID_POINT {
                lowest_max_opacity = WHITE_MAX_OPACITY; // At white (100% hsvV)
                max_deviation = 1.0 - max_deviation;
            } else if hsv.v < MID_POINT {
                lowest_max_opacity = BLACK_MAX_OPACITY; // At black (0% hsvV)
            }

            let mut max_opacity_suppression = MID_POINT_MAX_OPACITY - lowest_max_opacity;

            // Determine normalized deviation from the midpoint
            let deviation = (hsv.v - MID_POINT).abs();
            let normalized_deviation = deviation / max_deviation;

            // If we have saturation, reduce opacity suppression to allow that
            // color to come through more
            if hsv.s > 0.0 {
                // Dampen opacity suppression based on how much saturation
                // there is
                max_opacity_suppression *= f64::max(1.0 - (hsv.s * 2.0), 0.0);
            }

            let opacity_suppression = max_opacity_suppression * normalized_deviation;

            opacity_modifier = MID_POINT_MAX_OPACITY - opacity_suppression;
        }

        opacity_modifier
    }

    fn get_effective_luminosity_color(&self) -> Color {
        let luminosity_opacity = Some(self.material_opacity());

        self.get_luminosity_color(luminosity_opacity)
    }

    fn trim(value: f64) -> u8 {
        let value = f64::min((value * 256.0).floor(), 255.0);

        if value < 0.0 {
            return 0;
        } else if value > 255.0 {
            return 255;
        }

        value as u8
    }

    fn rgb_max(color: Color) -> f32 {
        if color.r > color.g {
            if color.r > color.b {
                color.r as f32
            } else {
                color.b as f32
            }
        } else if color.g > color.b {
            color.g as f32
        } else {
            color.b as f32
        }
    }

    fn rgb_min(color: Color) -> f32 {
        if color.r < color.g {
            if color.r < color.b {
                color.r as f32
            } else {
                color.b as f32
            }
        } else if color.g < color.b {
            color.g as f32
        } else {
            color.b as f32
        }
    }

    // The tint color used here should be the original, unmodified color
    // created using user values for TintColor + TintOpacity.
    fn get_luminosity_color(&self, luminosity_opacity: Option<f64>) -> Color {
        let tint_color = self.tint_color();

        // Calculate the HSL lightness value of the color.
        let max = Self::rgb_max(tint_color) / 255.0f32;
        let min = Self::rgb_min(tint_color) / 255.0f32;

        let mut lightness = ((max + min) as f64) / 2.0;

        lightness = 1.0 - ((1.0 - lightness) * luminosity_opacity.unwrap_or(1.0));

        lightness = 0.13 + (lightness * 0.74);

        let luminosity_color = Color::new(255, Self::trim(lightness), Self::trim(lightness), Self::trim(lightness));

        let compensation_level = self.platform_transparency_compensation_level();
        let compensation_multiplier = 1.0 - compensation_level;
        Color::new(
            (255.0
                * f64::max(
                    f64::min(compensation_level + (luminosity_opacity.unwrap_or(1.0) * compensation_multiplier), 1.0),
                    0.0,
                )) as u8,
            luminosity_color.r,
            luminosity_color.g,
            luminosity_color.b,
        )
    }

    /// Marks a property on a class deriving from this one as affecting the
    /// material's visual appearance: a change raises
    /// [`invalidated`](Self::invalidated).
    pub fn affects_render<T: ObjectType + Upcast<ExperimentalAcrylicMaterial>>(properties: &[&'static FerroProperty]) {
        for property in properties {
            property.changed().subscribe(|e| {
                if let Some(sender) = e.sender().downcast_ref::<T>() {
                    let material: &ExperimentalAcrylicMaterial = sender.upcast();
                    material.raise_invalidated();
                }
            });
        }
    }

    /// Raises the invalidated notification.
    pub fn raise_invalidated(&self) {
        if self.invalidated.is_empty() {
            return;
        }
        for (_, handler) in self.invalidated.snapshot().iter() {
            handler();
        }
    }

    /// Creates an immutable clone of the material.
    pub fn to_immutable(&self) -> Rc<dyn IExperimentalAcrylicMaterial> {
        Rc::new(ImmutableExperimentalAcrylicMaterial::new(&RefAdapter(self.to_ref())))
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream: there are no upstream unit tests for this class.
    use super::*;
    use crate::media::{Colors, MaterialExtensions};

    #[test]
    fn changing_properties_raises_invalidated_and_updates_effective_colors() {
        let target = ExperimentalAcrylicMaterial::new();
        let count = Rc::new(Cell::new(0));
        let c = count.clone();
        target.invalidated(move || c.set(c.get() + 1));

        target.set_tint_color(Colors::WHITE);
        assert_eq!(1, count.get());
        target.set_tint_opacity(1.0);
        target.set_material_opacity(0.65);
        target.set_background_source(AcrylicBackgroundSource::Digger);
        assert_eq!(4, count.get());
        // The fallback color does not affect rendering.
        target.set_fallback_color(Colors::RED);
        assert_eq!(4, count.get());

        let material: Rc<dyn IExperimentalAcrylicMaterial> = target.clone().into();
        // White: the tint opacity is compressed to 20%.
        assert_eq!(Color::new(51, 255, 255, 255), material.tint_color());
        // Lightness 1 -> 0.13 + 0.74 = 0.87 -> floor(0.87 * 256) = 222;
        // alpha = 255 * 0.65 = 165.
        assert_eq!(Color::new(165, 222, 222, 222), material.material_color());
        assert_eq!(1.0, material.tint_opacity());
        assert_eq!(Colors::RED, material.fallback_color());
        assert_eq!(AcrylicBackgroundSource::Digger, material.background_source());

        let immutable = MaterialExtensions::to_immutable(&material);
        assert!(immutable.as_mutable_material().is_none());
        assert_eq!(Color::new(51, 255, 255, 255), immutable.tint_color());
        let again = MaterialExtensions::to_immutable(&immutable);
        assert!(*again == *immutable);

        let a = immutable.as_any().downcast_ref::<ImmutableExperimentalAcrylicMaterial>().unwrap();
        target.set_platform_transparency_compensation_level(1.0);
        let b = target.to_immutable();
        let b = b.as_any().downcast_ref::<ImmutableExperimentalAcrylicMaterial>().unwrap();
        assert_eq!(255, b.material_color().a);
        assert!(a != b);
        assert!(a == &a.clone());
    }
}

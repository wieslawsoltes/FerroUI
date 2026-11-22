use crate::{AnimationError, Result, require};
use ferroui_base::animation::Animatable;
use ferroui_base::animation::animators::BoxShadowsAnimator;
use ferroui_base::media::{BoxShadow, BoxShadows, Color};
use ferroui_base::{FerroProperty, FerroPropertyRegistry};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct AnimationBoxShadow {
    pub offset_x: f64,
    pub offset_y: f64,
    pub blur: f64,
    pub spread: f64,
    pub argb: [u8; 4],
    pub inset: bool,
}

impl From<BoxShadow> for AnimationBoxShadow {
    fn from(shadow: BoxShadow) -> Self {
        Self {
            offset_x: shadow.offset_x,
            offset_y: shadow.offset_y,
            blur: shadow.blur,
            spread: shadow.spread,
            argb: shadow.color.to_uint32().to_be_bytes(),
            inset: shadow.is_inset,
        }
    }
}

impl From<AnimationBoxShadow> for BoxShadow {
    fn from(shadow: AnimationBoxShadow) -> Self {
        Self {
            offset_x: shadow.offset_x,
            offset_y: shadow.offset_y,
            blur: shadow.blur,
            spread: shadow.spread,
            color: Color::from_uint32(u32::from_be_bytes(shadow.argb)),
            is_inset: shadow.inset,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AnimationBoxShadows {
    layers: Vec<AnimationBoxShadow>,
    // Keep unchanged XAML literals, including color functions and named colors.
    #[serde(skip)]
    spelling: Option<String>,
}

impl PartialEq for AnimationBoxShadows {
    fn eq(&self, other: &Self) -> bool {
        self.layers == other.layers
    }
}

impl AnimationBoxShadows {
    pub fn is_property(property: &FerroProperty) -> bool {
        property.property_type() == std::any::TypeId::of::<BoxShadows>()
    }

    pub fn base_value(target: &Animatable, property: &'static FerroProperty) -> Option<Self> {
        if !FerroPropertyRegistry::instance().is_registered_for(target, property) {
            return None;
        }
        Some(Self::from_native(&BoxShadowsAnimator::base_value(
            target,
            property.as_styled::<BoxShadows>()?,
        )))
    }

    pub fn parse(source: &str) -> Result<Self> {
        let mut value = Self::from_native(
            &BoxShadows::parse(source).map_err(|error| AnimationError(error.to_string()))?,
        );
        value.validate()?;
        if !source.trim().is_empty() {
            value.spelling = Some(source.to_owned());
        }
        Ok(value)
    }

    pub fn from_native(shadows: &BoxShadows) -> Self {
        Self {
            layers: shadows.iter().map(AnimationBoxShadow::from).collect(),
            spelling: None,
        }
    }

    pub fn layers(&self) -> &[AnimationBoxShadow] {
        &self.layers
    }

    pub fn to_native(&self) -> BoxShadows {
        let Some((first, rest)) = self.layers.split_first() else {
            return BoxShadows::default();
        };
        let rest = rest
            .iter()
            .copied()
            .map(BoxShadow::from)
            .collect::<Vec<_>>();
        BoxShadows::with_rest((*first).into(), &rest)
    }

    pub(crate) fn validate(&self) -> Result<()> {
        require(
            self.layers.iter().all(|shadow| {
                [shadow.offset_x, shadow.offset_y, shadow.blur, shadow.spread]
                    .iter()
                    .all(|value| value.is_finite())
            }),
            "Box shadow offsets, blur, and spread must be finite",
        )?;
        require(
            !matches!(self.layers.as_slice(), [layer] if BoxShadow::from(*layer) == BoxShadow::default()),
            "Use none for a single empty box shadow",
        )
    }

    pub(crate) fn interpolate(&self, to: &Self, progress: f64) -> Self {
        Self::from_native(&BoxShadowsAnimator::interpolate_core(
            progress,
            &self.to_native(),
            &to.to_native(),
        ))
    }

    pub fn to_xaml(&self) -> String {
        self.spelling
            .clone()
            .unwrap_or_else(|| self.to_native().to_string())
    }
}

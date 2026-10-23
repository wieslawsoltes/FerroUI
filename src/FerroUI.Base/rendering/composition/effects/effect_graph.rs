//! The loaded form of an effect description: the graph an effect factory
//! checked, with the animatable properties resolved, which its brushes
//! share with their server side.

use super::{
    BlendEffectMode, CanvasComposite, CanvasEdgeBehavior, CompositionEffectFactoryError,
    CompositionEffectFactoryLoadStatus, EffectBorderMode, GraphicsEffect, GraphicsEffectSource,
};
use crate::media::imaging::BitmapBlendingMode;
use crate::media::{Color, ColorMatrix, ColorMatrixAlphaMode};

/// The most effects (and source parameters) a graph may have; a larger
/// graph is `EffectTooComplex`.
pub(crate) const MAX_EFFECT_NODES: usize = 256;

/// The deepest nesting of effects a graph may have; a deeper graph is
/// `EffectTooComplex`.
pub(crate) const MAX_EFFECT_DEPTH: usize = 32;

/// Converts the `BlurAmount` of a Win2D blur (the standard deviation of the
/// Gaussian, in device independent pixels) to the radius of the blur effect
/// of the drawing contexts, which both backends turn into a standard
/// deviation as `0.288675 * radius + 0.5`. An amount of 0.5 or less is no
/// blur.
pub fn blur_amount_to_radius(blur_amount: f64) -> f64 {
    if blur_amount.is_nan() || blur_amount <= 0.5 {
        return 0.0;
    }
    (blur_amount - 0.5) / 0.288675
}

/// A scalar member of an effect: its value in the description, and the key
/// of its entry in the `Properties` of the brush when it is animatable.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct EffectScalar {
    pub(crate) key: Option<String>,
    pub(crate) value: f32,
}

impl EffectScalar {
    fn new(value: f32) -> Self {
        Self { key: None, value }
    }
}

/// A colour member of an effect; see [`EffectScalar`].
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct EffectColor {
    pub(crate) key: Option<String>,
    pub(crate) value: Color,
}

impl EffectColor {
    fn new(value: Color) -> Self {
        Self { key: None, value }
    }
}

/// The matrix of a colour matrix node: a fixed matrix, or one made from an
/// animatable member.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum EffectMatrix {
    Fixed(ColorMatrix),
    /// `ColorMatrix::saturation` of the member.
    Saturation(EffectScalar),
    /// `ColorMatrix::tint` of the member.
    Tint(EffectColor),
}

/// What a node of a loaded graph does.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum EffectNodeKind {
    /// A source parameter: the brush bound to the name, or the composed
    /// layer of a layer visual.
    Parameter(String),
    GaussianBlur {
        amount: EffectScalar,
        /// The `Hard` border mode: the blur reads the source beyond the area
        /// of the brush, so its edges do not fade.
        hard: bool,
        source: Box<EffectNode>,
    },
    Opacity { opacity: EffectScalar, source: Box<EffectNode> },
    CrossFade { weight: EffectScalar, source1: Box<EffectNode>, source2: Box<EffectNode> },
    /// The sources composed onto each other with a mode, the first at the
    /// bottom.
    Composite { mode: BitmapBlendingMode, sources: Vec<EffectNode> },
    ColorSource(EffectColor),
    /// The foreground mixed into the background.
    Blend { mode: BitmapBlendingMode, background: Box<EffectNode>, foreground: Box<EffectNode> },
    /// The source through a colour matrix.
    ColorMatrix { matrix: EffectMatrix, alpha_mode: ColorMatrixAlphaMode, source: Box<EffectNode> },
    /// The source multiplied by the alpha of the mask.
    AlphaMask { source: Box<EffectNode>, mask: Box<EffectNode> },
    /// The source, tiled when it is a surface brush and `tile` is set.
    Border { tile: bool, source: Box<EffectNode> },
    /// `multiply * s1 * s2 + source1 * s1 + source2 * s2 + offset` where
    /// one of the sources is a colour source (`color`): a colour matrix of
    /// the other (`image`).
    ArithmeticWithColor {
        multiply: EffectScalar,
        source1: EffectScalar,
        source2: EffectScalar,
        offset: EffectScalar,
        image_is_source1: bool,
        image: Box<EffectNode>,
        color: Box<EffectNode>,
    },
    /// The lighting of the lights that reach the painted visual.
    SceneLighting { ambient: EffectScalar, diffuse: EffectScalar, specular: EffectScalar, shine: EffectScalar },
}

/// A node of a loaded graph: an effect with its name, or a source
/// parameter (whose name is empty; the parameter's own name is in the
/// kind).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct EffectNode {
    pub(crate) name: String,
    pub(crate) kind: EffectNodeKind,
}

impl EffectNode {
    /// The inputs of the node, in order.
    pub(crate) fn sources(&self) -> Vec<&EffectNode> {
        match &self.kind {
            EffectNodeKind::Parameter(_) | EffectNodeKind::ColorSource(_) | EffectNodeKind::SceneLighting { .. } => {
                Vec::new()
            }
            EffectNodeKind::GaussianBlur { source, .. }
            | EffectNodeKind::Opacity { source, .. }
            | EffectNodeKind::ColorMatrix { source, .. }
            | EffectNodeKind::Border { source, .. } => vec![&**source],
            EffectNodeKind::CrossFade { source1, source2, .. } => vec![&**source1, &**source2],
            EffectNodeKind::Composite { sources, .. } => sources.iter().collect(),
            EffectNodeKind::Blend { background, foreground, .. } => vec![&**background, &**foreground],
            EffectNodeKind::AlphaMask { source, mask } => vec![&**source, &**mask],
            EffectNodeKind::ArithmeticWithColor { image, color, image_is_source1, .. } => {
                if *image_is_source1 {
                    vec![&**image, &**color]
                } else {
                    vec![&**color, &**image]
                }
            }
        }
    }

    fn sources_mut(&mut self) -> Vec<&mut EffectNode> {
        match &mut self.kind {
            EffectNodeKind::Parameter(_) | EffectNodeKind::ColorSource(_) | EffectNodeKind::SceneLighting { .. } => {
                Vec::new()
            }
            EffectNodeKind::GaussianBlur { source, .. }
            | EffectNodeKind::Opacity { source, .. }
            | EffectNodeKind::ColorMatrix { source, .. }
            | EffectNodeKind::Border { source, .. } => vec![&mut **source],
            EffectNodeKind::CrossFade { source1, source2, .. } => vec![&mut **source1, &mut **source2],
            EffectNodeKind::Composite { sources, .. } => sources.iter_mut().collect(),
            EffectNodeKind::Blend { background, foreground, .. } => vec![&mut **background, &mut **foreground],
            EffectNodeKind::AlphaMask { source, mask } => vec![&mut **source, &mut **mask],
            EffectNodeKind::ArithmeticWithColor { image, color, .. } => vec![&mut **image, &mut **color],
        }
    }

    /// Binds the property `property` of the effects named `effect` to the
    /// entry `key`; returns whether such a property was found.
    fn bind_animatable(&mut self, effect: &str, property: &str, key: &str) -> Result<bool, ()> {
        let mut found = false;
        if self.name == effect && !self.name.is_empty() {
            let is = |name: &str| property.eq_ignore_ascii_case(name);
            let key = Some(key.to_owned());
            match &mut self.kind {
                EffectNodeKind::GaussianBlur { amount, .. } if is("BlurAmount") => amount.key = key,
                EffectNodeKind::Opacity { opacity, .. } if is("Opacity") => opacity.key = key,
                EffectNodeKind::CrossFade { weight, .. } if is("Weight") || is("CrossFade") => weight.key = key,
                EffectNodeKind::ColorSource(color) if is("Color") => color.key = key,
                EffectNodeKind::ColorMatrix { matrix: EffectMatrix::Saturation(saturation), .. }
                    if is("Saturation") =>
                {
                    saturation.key = key
                }
                EffectNodeKind::ColorMatrix { matrix: EffectMatrix::Tint(color), .. } if is("Color") => {
                    color.key = key
                }
                EffectNodeKind::ArithmeticWithColor { multiply, .. } if is("MultiplyAmount") => multiply.key = key,
                EffectNodeKind::ArithmeticWithColor { source1, .. } if is("Source1Amount") => source1.key = key,
                EffectNodeKind::ArithmeticWithColor { source2, .. } if is("Source2Amount") => source2.key = key,
                EffectNodeKind::ArithmeticWithColor { offset, .. } if is("Offset") => offset.key = key,
                EffectNodeKind::SceneLighting { ambient, .. } if is("AmbientAmount") => ambient.key = key,
                EffectNodeKind::SceneLighting { diffuse, .. } if is("DiffuseAmount") => diffuse.key = key,
                EffectNodeKind::SceneLighting { specular, .. } if is("SpecularAmount") => specular.key = key,
                EffectNodeKind::SceneLighting { shine, .. } if is("SpecularShine") => shine.key = key,
                // A member that exists but is not animatable, or no member of
                // that name.
                _ => return Err(()),
            }
            found = true;
        }
        for source in self.sources_mut() {
            found |= source.bind_animatable(effect, property, key)?;
        }
        Ok(found)
    }

    /// The initial value of the animatable property bound to `key`.
    fn animatable_value(&self, key: &str) -> Option<EffectPropertyValue> {
        let scalar = |value: &EffectScalar| {
            (value.key.as_deref() == Some(key)).then_some(EffectPropertyValue::Scalar(value.value))
        };
        let color =
            |value: &EffectColor| (value.key.as_deref() == Some(key)).then_some(EffectPropertyValue::Color(value.value));
        let own = match &self.kind {
            EffectNodeKind::GaussianBlur { amount: value, .. }
            | EffectNodeKind::Opacity { opacity: value, .. }
            | EffectNodeKind::CrossFade { weight: value, .. } => scalar(value),
            EffectNodeKind::ColorSource(value) => color(value),
            EffectNodeKind::ColorMatrix { matrix: EffectMatrix::Saturation(value), .. } => scalar(value),
            EffectNodeKind::ColorMatrix { matrix: EffectMatrix::Tint(value), .. } => color(value),
            EffectNodeKind::ArithmeticWithColor { multiply, source1, source2, offset, .. } => {
                scalar(multiply).or_else(|| scalar(source1)).or_else(|| scalar(source2)).or_else(|| scalar(offset))
            }
            EffectNodeKind::SceneLighting { ambient, diffuse, specular, shine } => {
                scalar(ambient).or_else(|| scalar(diffuse)).or_else(|| scalar(specular)).or_else(|| scalar(shine))
            }
            EffectNodeKind::Parameter(_)
            | EffectNodeKind::Composite { .. }
            | EffectNodeKind::Blend { .. }
            | EffectNodeKind::ColorMatrix { matrix: EffectMatrix::Fixed(_), .. }
            | EffectNodeKind::AlphaMask { .. }
            | EffectNodeKind::Border { .. } => None,
        };
        own.or_else(|| self.sources().into_iter().find_map(|source| source.animatable_value(key)))
    }

    fn collect_parameters(&self, parameters: &mut Vec<String>) {
        if let EffectNodeKind::Parameter(name) = &self.kind {
            if !parameters.iter().any(|existing| existing == name) {
                parameters.push(name.clone());
            }
        }
        for source in self.sources() {
            source.collect_parameters(parameters);
        }
    }

    /// The path (the indices of the inputs) from this node to its first
    /// source parameter in graph order.
    fn first_parameter_path(&self, path: &mut Vec<usize>) -> bool {
        if matches!(self.kind, EffectNodeKind::Parameter(_)) {
            return true;
        }
        for (index, source) in self.sources().into_iter().enumerate() {
            path.push(index);
            if source.first_parameter_path(path) {
                return true;
            }
            path.pop();
        }
        false
    }
}

/// The initial value of an animatable property, which the brush inserts
/// into its `Properties`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum EffectPropertyValue {
    Scalar(f32),
    Color(Color),
}

/// An effect graph as a factory loaded it, shared (behind an `Arc`) by the
/// brushes of the factory and their server side.
#[derive(Debug, Default)]
pub(crate) struct EffectGraph {
    /// The root effect; `None` when the factory failed to load the effect,
    /// whose brushes then draw nothing.
    pub(crate) root: Option<EffectNode>,
    /// The names of the source parameters, in graph order, once each.
    pub(crate) parameters: Vec<String>,
    /// The animatable properties, by the key given to the factory
    /// (`Blur.BlurAmount`), with their initial values.
    pub(crate) animatable: Vec<(String, EffectPropertyValue)>,
    /// The path to the first source parameter in graph order: where the
    /// composed layer goes when the brush is the effect of a layer visual.
    pub(crate) layer_path: Option<Vec<usize>>,
}

impl EffectGraph {
    /// The graph of a factory that failed to load its effect.
    pub(crate) fn failed() -> EffectGraph {
        EffectGraph::default()
    }

    /// Loads `effect` with the animatable properties `animatable` (each
    /// `EffectName.PropertyName`): the load status and error of the factory
    /// when the effect cannot be drawn.
    pub(crate) fn load(
        effect: &GraphicsEffect,
        animatable: &[&str],
    ) -> Result<EffectGraph, (CompositionEffectFactoryLoadStatus, CompositionEffectFactoryError)> {
        let mut count = 0;
        let mut root = build_node(effect, 1, &mut count)?;
        let mut properties: Vec<(String, EffectPropertyValue)> = Vec::with_capacity(animatable.len());
        for &key in animatable {
            let invalid = || {
                (CompositionEffectFactoryLoadStatus::Other, CompositionEffectFactoryError::InvalidAnimatableProperty(key.to_owned()))
            };
            let (effect_name, property) = key.split_once('.').ok_or_else(invalid)?;
            if !root.bind_animatable(effect_name, property, key).map_err(|_| invalid())? {
                return Err(invalid());
            }
            if !properties.iter().any(|(existing, _)| existing == key) {
                let value = root.animatable_value(key).ok_or_else(invalid)?;
                properties.push((key.to_owned(), value));
            }
        }
        let mut parameters = Vec::new();
        root.collect_parameters(&mut parameters);
        let mut path = Vec::new();
        let layer_path = root.first_parameter_path(&mut path).then_some(path);
        Ok(EffectGraph { root: Some(root), parameters, animatable: properties, layer_path })
    }

    /// Whether the graph was loaded.
    pub(crate) fn is_loaded(&self) -> bool {
        self.root.is_some()
    }

    /// Whether `key` is one of the animatable properties of the graph.
    pub(crate) fn is_animatable(&self, key: &str) -> bool {
        self.animatable.iter().any(|(existing, _)| existing == key)
    }

    /// Whether the graph has a source parameter named `name`.
    pub(crate) fn has_parameter(&self, name: &str) -> bool {
        self.parameters.iter().any(|existing| existing == name)
    }
}

type LoadError = (CompositionEffectFactoryLoadStatus, CompositionEffectFactoryError);

fn too_complex() -> LoadError {
    (CompositionEffectFactoryLoadStatus::EffectTooComplex, CompositionEffectFactoryError::EffectTooComplex)
}

fn not_supported(effect: &GraphicsEffect) -> LoadError {
    (CompositionEffectFactoryLoadStatus::Other, CompositionEffectFactoryError::EffectTypeNotSupported(effect.type_name()))
}

fn build_source(source: &Option<GraphicsEffectSource>, owner: &GraphicsEffect, depth: usize, count: &mut usize) -> Result<EffectNode, LoadError> {
    match source {
        None => Err((
            CompositionEffectFactoryLoadStatus::Other,
            CompositionEffectFactoryError::MissingSource { effect: owner.type_name(), name: owner.name().to_owned() },
        )),
        Some(GraphicsEffectSource::Parameter(parameter)) => {
            *count += 1;
            if *count > MAX_EFFECT_NODES {
                return Err(too_complex());
            }
            Ok(EffectNode { name: String::new(), kind: EffectNodeKind::Parameter(parameter.name().to_owned()) })
        }
        Some(GraphicsEffectSource::Effect(effect)) => build_node(effect, depth + 1, count),
    }
}

/// The blend mode of the drawing contexts for a composite mode; `None` for
/// `MaskInvert`, which has none.
pub(crate) fn composite_blending_mode(mode: CanvasComposite) -> Option<BitmapBlendingMode> {
    Some(match mode {
        CanvasComposite::SourceOver => BitmapBlendingMode::SourceOver,
        CanvasComposite::DestinationOver => BitmapBlendingMode::DestinationOver,
        CanvasComposite::SourceIn => BitmapBlendingMode::SourceIn,
        CanvasComposite::DestinationIn => BitmapBlendingMode::DestinationIn,
        CanvasComposite::SourceOut => BitmapBlendingMode::SourceOut,
        CanvasComposite::DestinationOut => BitmapBlendingMode::DestinationOut,
        CanvasComposite::SourceAtop => BitmapBlendingMode::SourceAtop,
        CanvasComposite::DestinationAtop => BitmapBlendingMode::DestinationAtop,
        CanvasComposite::Xor => BitmapBlendingMode::Xor,
        CanvasComposite::Add => BitmapBlendingMode::Plus,
        CanvasComposite::Copy | CanvasComposite::BoundedCopy => BitmapBlendingMode::Source,
        CanvasComposite::MaskInvert => return None,
    })
}

/// The blend mode of the drawing contexts for a blend effect mode; `None`
/// for the modes they do not have.
pub(crate) fn blend_blending_mode(mode: BlendEffectMode) -> Option<BitmapBlendingMode> {
    Some(match mode {
        BlendEffectMode::Multiply => BitmapBlendingMode::Multiply,
        BlendEffectMode::Screen => BitmapBlendingMode::Screen,
        BlendEffectMode::Darken => BitmapBlendingMode::Darken,
        BlendEffectMode::Lighten => BitmapBlendingMode::Lighten,
        BlendEffectMode::ColorBurn => BitmapBlendingMode::ColorBurn,
        BlendEffectMode::ColorDodge => BitmapBlendingMode::ColorDodge,
        BlendEffectMode::LinearDodge => BitmapBlendingMode::Plus,
        BlendEffectMode::Overlay => BitmapBlendingMode::Overlay,
        BlendEffectMode::SoftLight => BitmapBlendingMode::SoftLight,
        BlendEffectMode::HardLight => BitmapBlendingMode::HardLight,
        BlendEffectMode::Difference => BitmapBlendingMode::Difference,
        BlendEffectMode::Exclusion => BitmapBlendingMode::Exclusion,
        BlendEffectMode::Hue => BitmapBlendingMode::Hue,
        BlendEffectMode::Saturation => BitmapBlendingMode::Saturation,
        BlendEffectMode::Color => BitmapBlendingMode::Color,
        BlendEffectMode::Luminosity => BitmapBlendingMode::Luminosity,
        BlendEffectMode::Dissolve
        | BlendEffectMode::LinearBurn
        | BlendEffectMode::DarkerColor
        | BlendEffectMode::LighterColor
        | BlendEffectMode::VividLight
        | BlendEffectMode::LinearLight
        | BlendEffectMode::PinLight
        | BlendEffectMode::HardMix
        | BlendEffectMode::Subtract
        | BlendEffectMode::Division => return None,
    })
}

/// The colour source an input is, if it is one: its loaded node.
fn color_source_of(source: &Option<GraphicsEffectSource>) -> Option<&GraphicsEffect> {
    match source {
        Some(GraphicsEffectSource::Effect(effect)) if matches!(**effect, GraphicsEffect::ColorSource(_)) => Some(effect),
        _ => None,
    }
}

fn build_node(effect: &GraphicsEffect, depth: usize, count: &mut usize) -> Result<EffectNode, LoadError> {
    *count += 1;
    if depth > MAX_EFFECT_DEPTH || *count > MAX_EFFECT_NODES {
        return Err(too_complex());
    }
    let boxed = |source: &Option<GraphicsEffectSource>, count: &mut usize| -> Result<Box<EffectNode>, LoadError> {
        Ok(Box::new(build_source(source, effect, depth, count)?))
    };
    let matrix_of = |matrix: EffectMatrix, alpha_mode, source: &Option<GraphicsEffectSource>, count: &mut usize| {
        Ok::<_, LoadError>(EffectNodeKind::ColorMatrix { matrix, alpha_mode, source: boxed(source, count)? })
    };
    let kind = match effect {
        GraphicsEffect::GaussianBlur(blur) => EffectNodeKind::GaussianBlur {
            amount: EffectScalar::new(blur.blur_amount),
            hard: blur.border_mode == EffectBorderMode::Hard,
            source: boxed(&blur.source, count)?,
        },
        GraphicsEffect::Opacity(opacity) => {
            EffectNodeKind::Opacity { opacity: EffectScalar::new(opacity.opacity), source: boxed(&opacity.source, count)? }
        }
        GraphicsEffect::CrossFade(cross_fade) => EffectNodeKind::CrossFade {
            weight: EffectScalar::new(cross_fade.weight),
            source1: boxed(&cross_fade.source1, count)?,
            source2: boxed(&cross_fade.source2, count)?,
        },
        GraphicsEffect::Composite(composite) => {
            let Some(mode) = composite_blending_mode(composite.mode) else {
                return Err((
                    CompositionEffectFactoryLoadStatus::Other,
                    CompositionEffectFactoryError::CompositeModeNotSupported(composite.mode),
                ));
            };
            let mut sources = Vec::with_capacity(composite.sources.len());
            for source in &composite.sources {
                sources.push(build_source(&Some(source.clone()), effect, depth, count)?);
            }
            EffectNodeKind::Composite { mode, sources }
        }
        GraphicsEffect::ColorSource(color) => EffectNodeKind::ColorSource(EffectColor::new(color.color)),
        GraphicsEffect::Saturation(saturation) => matrix_of(
            EffectMatrix::Saturation(EffectScalar::new(saturation.saturation)),
            ColorMatrixAlphaMode::Straight,
            &saturation.source,
            count,
        )?,
        GraphicsEffect::Blend(blend) => {
            let Some(mode) = blend_blending_mode(blend.mode) else {
                return Err((
                    CompositionEffectFactoryLoadStatus::Other,
                    CompositionEffectFactoryError::BlendModeNotSupported(blend.mode),
                ));
            };
            EffectNodeKind::Blend {
                mode,
                background: boxed(&blend.background, count)?,
                foreground: boxed(&blend.foreground, count)?,
            }
        }
        GraphicsEffect::ColorMatrix(matrix) => {
            matrix_of(EffectMatrix::Fixed(matrix.color_matrix), matrix.alpha_mode, &matrix.source, count)?
        }
        GraphicsEffect::Tint(tint) => matrix_of(
            EffectMatrix::Tint(EffectColor::new(tint.color)),
            ColorMatrixAlphaMode::Straight,
            &tint.source,
            count,
        )?,
        GraphicsEffect::Invert(invert) => {
            matrix_of(EffectMatrix::Fixed(ColorMatrix::invert()), ColorMatrixAlphaMode::Straight, &invert.source, count)?
        }
        GraphicsEffect::Grayscale(grayscale) => matrix_of(
            EffectMatrix::Fixed(ColorMatrix::grayscale()),
            ColorMatrixAlphaMode::Straight,
            &grayscale.source,
            count,
        )?,
        GraphicsEffect::LuminanceToAlpha(luminance) => matrix_of(
            EffectMatrix::Fixed(ColorMatrix::luminance_to_alpha()),
            ColorMatrixAlphaMode::Straight,
            &luminance.source,
            count,
        )?,
        GraphicsEffect::ArithmeticComposite(arithmetic) => {
            // Drawn as a colour matrix of the image source when the other
            // source is a colour.
            let (image_is_source1, image, color) = match (color_source_of(&arithmetic.source1), color_source_of(&arithmetic.source2)) {
                (_, Some(_)) => (true, &arithmetic.source1, &arithmetic.source2),
                (Some(_), None) => (false, &arithmetic.source2, &arithmetic.source1),
                (None, None) => return Err(not_supported(effect)),
            };
            EffectNodeKind::ArithmeticWithColor {
                multiply: EffectScalar::new(arithmetic.multiply_amount),
                source1: EffectScalar::new(arithmetic.source1_amount),
                source2: EffectScalar::new(arithmetic.source2_amount),
                offset: EffectScalar::new(arithmetic.offset),
                image_is_source1,
                image: boxed(image, count)?,
                color: boxed(color, count)?,
            }
        }
        GraphicsEffect::AlphaMask(mask) => {
            EffectNodeKind::AlphaMask { source: boxed(&mask.source, count)?, mask: boxed(&mask.alpha_mask, count)? }
        }
        GraphicsEffect::Border(border) => EffectNodeKind::Border {
            tile: border.extend_x != CanvasEdgeBehavior::Clamp || border.extend_y != CanvasEdgeBehavior::Clamp,
            source: boxed(&border.source, count)?,
        },
        GraphicsEffect::SceneLighting(lighting) => EffectNodeKind::SceneLighting {
            ambient: EffectScalar::new(lighting.ambient_amount),
            diffuse: EffectScalar::new(lighting.diffuse_amount),
            specular: EffectScalar::new(lighting.specular_amount),
            shine: EffectScalar::new(lighting.specular_shine),
        },
    };
    Ok(EffectNode { name: effect.name().to_owned(), kind })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::Colors;
    use crate::rendering::composition::effects::{
        ArithmeticCompositeEffect, BlendEffect, ColorSourceEffect, CompositionEffectSourceParameter, SaturationEffect,
        SceneLightingEffect,
    };

    fn parameter(name: &str) -> Option<GraphicsEffectSource> {
        Some(CompositionEffectSourceParameter::new(name).into())
    }

    #[test]
    fn saturation_loads_as_an_animatable_colour_matrix() {
        let effect: GraphicsEffect =
            SaturationEffect { name: "Saturation".into(), saturation: 0.25, source: parameter("source") }.into();
        let graph = EffectGraph::load(&effect, &["Saturation.Saturation"]).expect("loads");
        assert_eq!(graph.animatable, vec![("Saturation.Saturation".to_owned(), EffectPropertyValue::Scalar(0.25))]);
        assert!(matches!(graph.root.unwrap().kind, EffectNodeKind::ColorMatrix { matrix: EffectMatrix::Saturation(_), .. }));
    }

    #[test]
    fn blend_modes_without_a_counterpart_do_not_load() {
        let effect: GraphicsEffect = BlendEffect {
            mode: BlendEffectMode::Dissolve,
            background: parameter("a"),
            foreground: parameter("b"),
            ..Default::default()
        }
        .into();
        let error = EffectGraph::load(&effect, &[]).unwrap_err();
        assert_eq!(error.1, CompositionEffectFactoryError::BlendModeNotSupported(BlendEffectMode::Dissolve));
        let effect: GraphicsEffect = BlendEffect {
            mode: BlendEffectMode::Luminosity,
            background: parameter("a"),
            foreground: parameter("b"),
            ..Default::default()
        }
        .into();
        let graph = EffectGraph::load(&effect, &[]).expect("loads");
        assert!(matches!(
            graph.root.unwrap().kind,
            EffectNodeKind::Blend { mode: BitmapBlendingMode::Luminosity, .. }
        ));
    }

    #[test]
    fn arithmetic_composite_needs_a_colour_on_one_side() {
        let both_images: GraphicsEffect =
            ArithmeticCompositeEffect { source1: parameter("a"), source2: parameter("b"), ..Default::default() }.into();
        assert!(EffectGraph::load(&both_images, &[]).is_err());

        let with_color: GraphicsEffect = ArithmeticCompositeEffect {
            name: "Arithmetic".into(),
            source1: parameter("a"),
            source2: Some(ColorSourceEffect { name: "Base".into(), color: Colors::RED }.into()),
            ..Default::default()
        }
        .into();
        let graph = EffectGraph::load(&with_color, &["Base.Color", "Arithmetic.Offset"]).expect("loads");
        assert_eq!(graph.parameters, vec!["a".to_owned()]);
        assert_eq!(graph.animatable.len(), 2);
    }

    #[test]
    fn scene_lighting_amounts_are_animatable() {
        let effect: GraphicsEffect = SceneLightingEffect { name: "Light".into(), ..Default::default() }.into();
        let graph = EffectGraph::load(&effect, &["Light.DiffuseAmount"]).expect("loads");
        assert_eq!(graph.animatable, vec![("Light.DiffuseAmount".to_owned(), EffectPropertyValue::Scalar(1.0))]);
        assert!(graph.layer_path.is_none());
    }
}

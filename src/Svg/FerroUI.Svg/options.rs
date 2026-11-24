use crate::SvgError;
use ferroui_base::media::Color;
use ferroui_base::{Rect, Vector};
use std::collections::BTreeSet;

/// What to do when an operation cannot be represented by Skia's SVG writer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SvgFidelity {
    /// Keep supported layers as vectors; replay each affected, isolated layer to PNG.
    #[default]
    Automatic,
    /// Fail instead of substituting a raster layer. Intentional bitmap drawings are allowed.
    VectorOnly,
    /// Render every recorded layer to PNG. Useful for a reference or a compatibility export.
    Raster,
}

/// Text portability versus editability.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SvgTextMode {
    /// Preserve the actual shaped glyph outlines, independent of installed fonts.
    #[default]
    Outlines,
    /// Ask Skia to reconstruct text from glyph IDs. This is lossy for some scripts and fonts;
    /// an export diagnostic is always emitted. Fonts are referenced, not embedded.
    Editable,
}

/// Physical dimensions. Geometry and viewBox always remain in 96-DPI logical pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SvgUnits {
    #[default]
    Pixels,
    Millimeters,
    Centimeters,
    Inches,
    Points,
}

impl SvgUnits {
    pub(crate) fn length(self, pixels: f64) -> String {
        let (factor, suffix) = match self {
            Self::Pixels => (1.0, ""),
            Self::Millimeters => (25.4 / 96.0, "mm"),
            Self::Centimeters => (2.54 / 96.0, "cm"),
            Self::Inches => (1.0 / 96.0, "in"),
            Self::Points => (72.0 / 96.0, "pt"),
        };
        format!("{}{suffix}", pixels * factor)
    }
}

/// Match named semantic layers. Within each set, matching is OR; the ID and tag sets are
/// combined with AND when both are nonempty. Empty sets select all layers.
#[derive(Clone, Debug, Default)]
pub struct SvgLayerFilter {
    pub ids: BTreeSet<String>,
    pub tags: BTreeSet<String>,
}

impl SvgLayerFilter {
    pub fn all() -> Self {
        Self::default()
    }
    pub fn id(id: impl Into<String>) -> Self {
        Self {
            ids: [id.into()].into(),
            tags: BTreeSet::new(),
        }
    }
    pub fn tag(tag: impl Into<String>) -> Self {
        Self {
            tags: [tag.into()].into(),
            ids: BTreeSet::new(),
        }
    }
    pub(crate) fn matches(&self, id: &str, tags: &[String]) -> bool {
        (self.ids.is_empty() || self.ids.contains(id))
            && (self.tags.is_empty() || tags.iter().any(|tag| self.tags.contains(tag)))
    }
}

/// Inspection paint for vector shapes. Raster islands retain their pixels and are reported.
#[derive(Clone, Debug, Default)]
pub enum SvgInspection {
    #[default]
    Normal,
    XRay {
        fill_opacity: f64,
        stroke: Color,
        stroke_width: f64,
    },
    Wireframe {
        stroke: Color,
        stroke_width: f64,
    },
}

impl SvgInspection {
    pub fn x_ray() -> Self {
        Self::XRay {
            fill_opacity: 0.12,
            stroke: Color::from_argb(255, 40, 170, 220),
            stroke_width: 0.8,
        }
    }
    pub fn wireframe() -> Self {
        Self::Wireframe {
            stroke: Color::from_argb(255, 40, 170, 220),
            stroke_width: 0.8,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SvgCutawayMode {
    /// Remove this rectangular window from the selected layers, exposing layers below them.
    #[default]
    RemoveInside,
    /// Retain only the part of selected layers inside the window.
    KeepInside,
}

/// A 2D section window in document coordinates, applied after layer/explosion transforms.
/// It cuts selected semantic layers; it does not intersect arbitrary 3D solid meshes.
#[derive(Clone, Debug)]
pub struct SvgCutaway {
    pub rect: Rect,
    pub layers: SvgLayerFilter,
    pub mode: SvgCutawayMode,
    pub show_boundary: bool,
}

/// A reusable separation of layers by their declared depth.
#[derive(Clone, Debug)]
pub struct SvgExplodedView {
    pub step: Vector,
    pub show_connectors: bool,
    pub show_labels: bool,
}

impl Default for SvgExplodedView {
    fn default() -> Self {
        Self {
            step: Vector::new(32.0, -24.0),
            show_connectors: true,
            show_labels: true,
        }
    }
}

/// View transformations do not mutate the recorded document.
#[derive(Clone, Debug, Default)]
pub struct SvgView {
    pub inspection: SvgInspection,
    /// Apply inspection paint only to matching layers; annotation layers can retain normal paint.
    pub inspection_layers: SvgLayerFilter,
    pub cutaway: Option<SvgCutaway>,
    pub exploded: Option<SvgExplodedView>,
    pub include: SvgLayerFilter,
    pub exclude: Option<SvgLayerFilter>,
    pub include_hidden: bool,
    pub show_bounds: bool,
}

/// Export policy. Defaults favor portable design-tool import and preserve numeric precision.
#[derive(Clone, Debug)]
pub struct SvgExportOptions {
    pub fidelity: SvgFidelity,
    pub text: SvgTextMode,
    /// None retains Skia's numbers. A decimal precision is explicitly lossy (0..=12).
    pub precision: Option<u8>,
    pub pretty: bool,
    pub deduplicate: bool,
    pub relative_paths: bool,
    /// Keep the most recent native SVG per layer for repeated/inspection exports.
    pub cache_native_svg: bool,
    /// Keep the most recent encoded fallback PNG per layer. Budgets still apply on reuse.
    pub cache_raster_images: bool,
    pub responsive: bool,
    pub units: SvgUnits,
    /// Overrides the output viewport, including a nonzero origin for cropping/bleed.
    pub view_box: Option<Rect>,
    pub background: Option<Color>,
    /// Pixels per recording coordinate for raster fallback. Named layers normally use
    /// 96-DPI logical coordinates; compositor frames use device-pixel coordinates.
    /// The layer transform maps the image into the final logical document.
    pub raster_scale: f64,
    pub max_raster_pixels: u64,
    pub max_raster_dimension: u32,
    pub title: Option<String>,
    pub description: Option<String>,
    pub designer_layers: bool,
    pub view: SvgView,
}

impl Default for SvgExportOptions {
    fn default() -> Self {
        Self {
            fidelity: SvgFidelity::Automatic,
            text: SvgTextMode::Outlines,
            precision: None,
            pretty: true,
            deduplicate: true,
            relative_paths: false,
            cache_native_svg: true,
            cache_raster_images: true,
            responsive: false,
            units: SvgUnits::Pixels,
            view_box: None,
            background: None,
            raster_scale: 2.0,
            max_raster_pixels: 64_000_000,
            max_raster_dimension: 32_768,
            title: None,
            description: None,
            designer_layers: true,
            view: SvgView::default(),
        }
    }
}

impl SvgExportOptions {
    /// Minified output with unchanged geometry precision and portable outlined text.
    /// Relative paths remain opt-in: their serialized numbers can be longer for some scenes.
    pub fn compact() -> Self {
        Self {
            pretty: false,
            ..Self::default()
        }
    }

    pub(crate) fn validate(&self) -> Result<(), SvgError> {
        let invalid = |message: &str| SvgError::InvalidOptions(message.into());
        if self.precision.is_some_and(|precision| precision > 12) {
            return Err(invalid("precision must be in 0..=12"));
        }
        if !self.raster_scale.is_finite() || self.raster_scale <= 0.0 {
            return Err(invalid("raster_scale must be finite and positive"));
        }
        if self.max_raster_pixels == 0 || self.max_raster_dimension == 0 {
            return Err(invalid("raster limits must be positive"));
        }
        if let Some(rect) = self.view_box {
            validate_rect(rect)
                .map_err(|_| invalid("viewBox must be finite and have positive dimensions"))?;
        }
        if let Some(cutaway) = &self.view.cutaway {
            validate_rect(cutaway.rect).map_err(|_| {
                invalid("section window must be finite and have positive dimensions")
            })?;
        }
        if let Some(exploded) = &self.view.exploded {
            if !exploded.step.x.is_finite() || !exploded.step.y.is_finite() {
                return Err(invalid("explosion step must be finite"));
            }
        }
        match &self.view.inspection {
            SvgInspection::Normal => {}
            SvgInspection::XRay {
                fill_opacity,
                stroke_width,
                ..
            } => {
                if !fill_opacity.is_finite() || !(0.0..=1.0).contains(fill_opacity) {
                    return Err(invalid("x-ray fill opacity must be in 0..=1"));
                }
                if !stroke_width.is_finite() || *stroke_width <= 0.0 {
                    return Err(invalid(
                        "inspection stroke width must be finite and positive",
                    ));
                }
            }
            SvgInspection::Wireframe { stroke_width, .. } => {
                if !stroke_width.is_finite() || *stroke_width <= 0.0 {
                    return Err(invalid(
                        "inspection stroke width must be finite and positive",
                    ));
                }
            }
        }
        Ok(())
    }
}

pub(crate) fn validate_rect(rect: Rect) -> Result<(), ()> {
    if [
        rect.x,
        rect.y,
        rect.width,
        rect.height,
        rect.right(),
        rect.bottom(),
    ]
    .iter()
    .all(|n| n.is_finite())
        && rect.width > 0.0
        && rect.height > 0.0
    {
        Ok(())
    } else {
        Err(())
    }
}

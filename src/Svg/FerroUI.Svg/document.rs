use crate::xml::InspectionStyle;
use crate::{RecordingAudit, SvgDrawingContext, SvgError, SvgExportOptions, SvgOutput};
use ferroui_base::media::{DrawingContext, PlatformDrawingContext};
use ferroui_base::platform::IDrawingContextImpl;
use ferroui_base::rendering::ImmediateRenderer;
use ferroui_base::{Matrix, Rect, Ref, Size, Vector, Visual};
use ferroui_skia::PictureRenderTarget;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

/// A stable, caller-chosen layer identifier (without the SVG `ferro-` prefix).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SvgLayerId(String);

impl SvgLayerId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SvgLayerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Semantic layer properties. A layer is an isolated compositing island: operations inside
/// it can see their own backdrop, not earlier document layers. Record interdependent content
/// together in one layer. The layer's finished result is composited source-over.
#[derive(Clone, Debug)]
pub struct SvgLayerOptions {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub metadata: BTreeMap<String, String>,
    pub transform: Matrix,
    pub opacity: f64,
    pub visible: bool,
    /// Used by exploded views; it has no effect on normal painter order.
    pub depth: f64,
    /// Local recording bounds; None uses the document bounds. Bounds clip this layer.
    pub bounds: Option<Rect>,
}

impl SvgLayerOptions {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: None,
            tags: Vec::new(),
            metadata: BTreeMap::new(),
            transform: Matrix::IDENTITY,
            opacity: 1.0,
            visible: true,
            depth: 0.0,
            bounds: None,
        }
    }

    pub fn with_depth(mut self, depth: f64) -> Self {
        self.depth = depth;
        self
    }

    pub fn with_tags<I, S>(mut self, tags: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.tags = tags.into_iter().map(Into::into).collect();
        self
    }

    pub fn with_bounds(mut self, bounds: Rect) -> Self {
        self.bounds = Some(bounds);
        self
    }
    pub fn with_transform(mut self, transform: Matrix) -> Self {
        self.transform = transform;
        self
    }
    pub fn with_opacity(mut self, opacity: f64) -> Self {
        self.opacity = opacity;
        self
    }

    fn validate(&self) -> Result<(), SvgError> {
        // Stable XML names, not lossy slugification: an invalid ID must not alias another layer.
        if self.id.is_empty()
            || !self
                .id
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"_-".contains(&c))
        {
            return Err(SvgError::InvalidLayer(
                "IDs must contain ASCII letters, digits, '_' or '-'".into(),
            ));
        }
        if self.id.starts_with("internal-") {
            return Err(SvgError::InvalidLayer(
                "the 'internal-' ID prefix is reserved".into(),
            ));
        }
        if !self.opacity.is_finite() || !(0.0..=1.0).contains(&self.opacity) {
            return Err(SvgError::InvalidLayer("opacity must be in 0..=1".into()));
        }
        if !self.depth.is_finite() {
            return Err(SvgError::InvalidLayer("depth must be finite".into()));
        }
        let m = self.transform;
        if ![
            m.m11, m.m12, m.m13, m.m21, m.m22, m.m23, m.m31, m.m32, m.m33,
        ]
        .iter()
        .all(|n| n.is_finite())
            || m.m13 != 0.0
            || m.m23 != 0.0
            || m.m33 != 1.0
        {
            return Err(SvgError::InvalidLayer("layer transform must be a finite affine matrix; perspective belongs inside its drawing context".into()));
        }
        if let Some(bounds) = self.bounds {
            crate::options::validate_rect(bounds)
                .map_err(|_| SvgError::InvalidLayer("bounds must be finite and positive".into()))?;
        }
        for key in self.metadata.keys() {
            if key.is_empty()
                || !key
                    .bytes()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
            {
                return Err(SvgError::InvalidLayer(
                    "metadata keys must contain lowercase ASCII letters, digits or '-'".into(),
                ));
            }
        }
        Ok(())
    }
}

type RasterCacheEntry = (i32, i32, Rc<[u8]>);

/// Replacing the native representation also invalidates its prepared fragment.
/// A layer retains one native variant and at most one prepared view of it.
pub(crate) struct VectorCacheEntry {
    pub outlined: bool,
    pub relative_paths: bool,
    pub data: Rc<str>,
    pub prepared: Option<PreparedVectorCacheEntry>,
}

pub(crate) struct PreparedVectorCacheEntry {
    pub prefix: String,
    pub inspection: InspectionStyle,
    pub data: Rc<str>,
}

impl PreparedVectorCacheEntry {
    pub fn matches(&self, prefix: &str, inspection: &InspectionStyle) -> bool {
        if self.prefix != prefix {
            return false;
        }
        // Ordinary f64 equality would alias -0.0 and 0.0 even though the
        // fragment serializer preserves their distinct lexical values.
        match (&self.inspection, inspection) {
            (InspectionStyle::Normal, InspectionStyle::Normal) => true,
            (
                InspectionStyle::XRay {
                    fill_opacity: cached_fill,
                    stroke: cached_stroke,
                    stroke_width: cached_width,
                    stroke_opacity: cached_alpha,
                },
                InspectionStyle::XRay {
                    fill_opacity,
                    stroke,
                    stroke_width,
                    stroke_opacity,
                },
            ) => {
                cached_fill.to_bits() == fill_opacity.to_bits()
                    && cached_stroke == stroke
                    && cached_width.to_bits() == stroke_width.to_bits()
                    && cached_alpha.to_bits() == stroke_opacity.to_bits()
            }
            (
                InspectionStyle::Wireframe {
                    stroke: cached_stroke,
                    stroke_width: cached_width,
                    stroke_opacity: cached_alpha,
                },
                InspectionStyle::Wireframe {
                    stroke,
                    stroke_width,
                    stroke_opacity,
                },
            ) => {
                cached_stroke == stroke
                    && cached_width.to_bits() == stroke_width.to_bits()
                    && cached_alpha.to_bits() == stroke_opacity.to_bits()
            }
            _ => false,
        }
    }
}

/// A retained Skia recording with its semantic identity and capability audit.
pub struct SvgLayer {
    pub(crate) options: SvgLayerOptions,
    pub(crate) picture: skia_safe::Picture,
    pub(crate) audit: RecordingAudit,
    pub(crate) bounds: Rect,
    pub(crate) vector_cache: RefCell<Option<Box<VectorCacheEntry>>>,
    pub(crate) raster_cache: RefCell<Option<RasterCacheEntry>>,
}

impl SvgLayer {
    pub fn options(&self) -> &SvgLayerOptions {
        &self.options
    }
    pub fn audit(&self) -> &RecordingAudit {
        &self.audit
    }
    pub fn bounds(&self) -> Rect {
        self.bounds
    }
    pub fn approximate_recording_bytes(&self) -> usize {
        self.picture.approximate_bytes_used()
    }
}

/// A retained export document. Record once; generate several views without rendering the UI
/// again. Like FerroUI drawing contexts, a document stays on its owning render/UI thread.
pub struct SvgDocument {
    pub(crate) size: Size,
    pub(crate) layers: Vec<SvgLayer>,
}

impl SvgDocument {
    pub fn new(size: Size) -> Result<Self, SvgError> {
        crate::options::validate_rect(Rect::from_size(size)).map_err(|_| {
            SvgError::InvalidOptions("document dimensions must be finite and positive".into())
        })?;
        if size.width > f32::MAX as f64 || size.height > f32::MAX as f64 {
            return Err(SvgError::InvalidOptions(
                "document dimensions exceed Skia's coordinate range".into(),
            ));
        }
        Ok(Self {
            size,
            layers: Vec::new(),
        })
    }

    pub fn size(&self) -> Size {
        self.size
    }
    pub fn layers(&self) -> &[SvgLayer] {
        &self.layers
    }

    /// Record arbitrary FerroUI drawing commands into an isolated semantic layer. The callback
    /// runs exactly once. The context is finalized before this method returns.
    pub fn record_layer(
        &mut self,
        options: SvgLayerOptions,
        draw: impl FnOnce(&mut SvgDrawingContext),
    ) -> Result<SvgLayerId, SvgError> {
        self.try_record_layer(options, |context| {
            draw(context);
            Ok(())
        })
    }

    /// Fallible recording. A failed callback adds no layer; its context is still finalized.
    pub fn try_record_layer(
        &mut self,
        options: SvgLayerOptions,
        draw: impl FnOnce(&mut SvgDrawingContext) -> Result<(), SvgError>,
    ) -> Result<SvgLayerId, SvgError> {
        options.validate()?;
        if self
            .layers
            .iter()
            .any(|layer| layer.options.id == options.id)
        {
            return Err(SvgError::InvalidLayer(format!(
                "duplicate ID '{}'",
                options.id
            )));
        }
        let bounds = options.bounds.unwrap_or_else(|| Rect::from_size(self.size));
        if [
            bounds.x,
            bounds.y,
            bounds.width,
            bounds.height,
            bounds.right(),
            bounds.bottom(),
        ]
        .iter()
        .any(|n| n.abs() > f32::MAX as f64)
        {
            return Err(SvgError::InvalidLayer(
                "recording bounds exceed Skia's coordinate range".into(),
            ));
        }
        let target = PictureRenderTarget::new(None, None, Vector::new(96.0, 96.0));
        let mut inner = target.create_export_drawing_context_with_bounds(bounds, false);
        // The picture's cull rect is only an optimization hint. Keep this permanent clip
        // outside the caller's audited stack so it cannot be popped by application code.
        inner.push_clip(bounds);
        let mut context = SvgDrawingContext::new(Box::new(inner)).with_recording_bounds(bounds);
        let result = draw(&mut context);
        context.dispose();
        result?;
        let audit = context.audit();
        if !audit.errors.is_empty() {
            return Err(SvgError::Recording {
                layer: options.name,
                reasons: audit.errors,
            });
        }
        let picture = target.get_picture();
        let id = SvgLayerId(options.id.clone());
        self.layers.push(SvgLayer {
            options,
            picture,
            audit,
            bounds,
            vector_cache: RefCell::new(None),
            raster_cache: RefCell::new(None),
        });
        Ok(id)
    }

    /// Record a laid-out visual using FerroUI's immediate renderer. Call on its UI thread.
    /// As with RenderTargetBitmap, composition-only visuals and native child windows are
    /// outside the immediate renderer; composition brushes use their fallback colors.
    pub fn record_visual(
        &mut self,
        options: SvgLayerOptions,
        visual: &Ref<Visual>,
    ) -> Result<SvgLayerId, SvgError> {
        self.record_layer(options, |implementation| {
            let mut core = PlatformDrawingContext::borrowed(implementation);
            let mut context = DrawingContext::new(&mut core);
            ImmediateRenderer::render(&mut context, visual);
            context.dispose();
        })
    }

    /// Remove a semantic layer without changing the relative painter order of the others.
    pub fn remove_layer(&mut self, id: &SvgLayerId) -> bool {
        let count = self.layers.len();
        self.layers.retain(|layer| layer.options.id != id.0);
        count != self.layers.len()
    }

    /// Release native SVG, prepared fragments and PNG caches while retaining the pictures
    /// and semantic layer data. Each layer retains at most one representation of each type.
    pub fn clear_export_cache(&self) {
        for layer in &self.layers {
            layer.vector_cache.borrow_mut().take();
            layer.raster_cache.borrow_mut().take();
        }
    }

    /// Export a self-contained SVG. Validation/fallback decisions complete before output is
    /// returned; no external fonts, images or stylesheet requests are introduced by default.
    pub fn export(&self, options: &SvgExportOptions) -> Result<SvgOutput, SvgError> {
        crate::render::export(self, options)
    }
}

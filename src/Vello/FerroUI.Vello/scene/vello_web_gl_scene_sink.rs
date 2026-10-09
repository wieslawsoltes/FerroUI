use crate::scene::vello_cpu_scene_sink::{prepare, ALIASING_THRESHOLD, CURVE_TOLERANCE};
use crate::scene::{IVelloSceneSink, VelloSceneBrush, VelloSceneCapabilities, VelloSceneGlyphRun, VelloScenePaint};
use crate::vello_options::VelloRenderingMode;
use crate::web_gl::{log_render_failure, VelloWebGlGpu};
use glifo::{FontEmbolden, Glyph};
use kurbo::{Affine, BezPath, Diagonal2, Rect, Shape, Stroke, StrokeOpts};
use peniko::{BlendMode, Fill, ImageBrush, ImageData, ImageSampler};
use std::collections::HashMap;
use std::rc::Rc;
use vello_gpu::{ImageSource, PaintType, RectU16, Scene, TextureId};

/// The scene of the hybrid mode in a web page: the scene of `vello_gpu`,
/// drawn by its WebGL2 renderer into the drawing buffer of a canvas.
///
/// The scene is the one [`VelloHybridSceneSink`](crate::scene) records for
/// the renderer over `wgpu`, and it is recorded the same way (see there for
/// the three things the renderer does not do itself: destructive
/// compositions of a shape, images as pixels, mask layers). What differs is
/// where it ends: this sink belongs to the canvas of one WebGL2 context
/// ([`VelloWebGlGpu`]) and is rendered there and nowhere else
/// ([`render_to_canvas`](IVelloSceneSink::render_to_canvas)). A scene that
/// ends in memory is drawn by the CPU mode.
pub struct VelloWebGlSceneSink {
    gpu: Rc<VelloWebGlGpu>,
    scene: Scene,
    /// The images of the scene by the identity of their pixels.
    images: HashMap<u64, ImageData>,
}

impl VelloWebGlSceneSink {
    /// Creates the scene of a frame of the canvas of `gpu`, of the size the
    /// drawing buffer of the canvas has for that frame.
    pub fn new(gpu: Rc<VelloWebGlGpu>, width: u16, height: u16) -> Self {
        Self { gpu, scene: Scene::new(width, height), images: HashMap::new() }
    }

    fn set_anti_alias(&mut self, anti_alias: bool) {
        self.scene.set_aliasing_threshold(if anti_alias { None } else { Some(ALIASING_THRESHOLD) });
    }

    /// The paint in the form of the renderer, and its transform from the
    /// space of the paint to the pixels of the target.
    fn paint(&mut self, paint: &VelloScenePaint, transform: Affine) -> (PaintType, Affine) {
        let paint_type = match &paint.brush {
            VelloSceneBrush::Solid(color) => PaintType::Solid(*color),
            VelloSceneBrush::Gradient(gradient) => PaintType::Gradient(gradient.clone()),
            VelloSceneBrush::Image(image) => {
                let id = image.image.data.id();
                self.images.entry(id).or_insert_with(|| image.image.clone());

                PaintType::Image(ImageBrush {
                    image: ImageSource::ExternalTexture {
                        id: TextureId(id),
                        source_region: RectU16 {
                            x0: 0,
                            y0: 0,
                            x1: image.image.width.min(u32::from(u16::MAX)) as u16,
                            y1: image.image.height.min(u32::from(u16::MAX)) as u16,
                        },
                        may_have_transparency: true,
                    },
                    sampler: ImageSampler {
                        x_extend: image.x_extend,
                        y_extend: image.y_extend,
                        quality: image.quality,
                        alpha: image.alpha,
                    },
                })
            }
        };

        (paint_type, transform * paint.transform)
    }

    /// The rectangle of the whole target.
    fn target_path(&self) -> BezPath {
        Rect::new(0.0, 0.0, f64::from(self.scene.width()), f64::from(self.scene.height())).to_path(0.1)
    }
}

/// Whether a font has glyphs that are pictures of fixed sizes.
fn has_bitmap_strikes(font: &peniko::FontData) -> bool {
    use skrifa::MetadataProvider;

    skrifa::FontRef::from_index(font.data.as_ref(), font.index).is_ok_and(|font| !font.bitmap_strikes().is_empty())
}

impl IVelloSceneSink for VelloWebGlSceneSink {
    fn rendering_mode(&self) -> VelloRenderingMode {
        VelloRenderingMode::Hybrid
    }

    fn capabilities(&self) -> VelloSceneCapabilities {
        // The drawing buffer of a canvas is not read back: what ends in
        // memory is drawn by the CPU mode.
        VelloSceneCapabilities { blend_layers: true, aliased_edges: true, image_paints: true, read_back: false }
    }

    fn width(&self) -> u16 {
        self.scene.width()
    }

    fn height(&self) -> u16 {
        self.scene.height()
    }

    fn reset(&mut self) {
        self.scene.reset();
        self.images.clear();
    }

    fn fill(
        &mut self,
        path: &BezPath,
        fill_rule: Fill,
        transform: Affine,
        paint: &VelloScenePaint,
        blend_mode: BlendMode,
        anti_alias: bool,
    ) {
        let (path, path_transform) = prepare(path, transform);
        let (paint_type, paint_transform) = self.paint(paint, transform);

        self.set_anti_alias(anti_alias);
        self.scene.set_fill_rule(fill_rule);
        self.scene.set_blend_mode(BlendMode::default());

        if blend_mode.is_destructive() {
            // The scene refuses a destructive composition of a shape that
            // is not isolated. A layer that is clipped to the shape and
            // holds the paint everywhere composes the same pixels: inside
            // the shape the paint by the mode, outside it nothing, and on
            // the edge each by the coverage of the edge.
            self.scene.set_transform(path_transform);
            self.scene.push_layer(Some(&path), Some(blend_mode), None, None, None);

            let target = self.target_path();
            self.scene.set_transform(Affine::IDENTITY);
            self.scene.set_fill_rule(Fill::NonZero);
            self.scene.set_paint(paint_type);
            self.scene.set_paint_transform(paint_transform);
            self.scene.fill_path(&target);
            self.scene.pop_layer();
            return;
        }

        // The scene places a paint by the transform of the path times the
        // transform of the paint.
        let relative_paint_transform = match path_transform == transform {
            true => paint.transform,
            false => paint_transform,
        };

        self.scene.set_transform(path_transform);
        self.scene.set_paint(paint_type);
        self.scene.set_paint_transform(relative_paint_transform);
        self.scene.set_blend_mode(blend_mode);
        self.scene.fill_path(&path);
        self.scene.set_blend_mode(BlendMode::default());
    }

    fn stroke(&mut self, path: &BezPath, stroke: &Stroke, transform: Affine, paint: &VelloScenePaint, anti_alias: bool) {
        // As in the CPU mode: the outline of the stroke, made here to the
        // tolerance of the sinks at the scale of the transform.
        let scale = transform.determinant().abs().sqrt();
        if !(scale.is_finite() && scale > 0.0) {
            return;
        }
        let outline = kurbo::stroke(path.iter(), stroke, &StrokeOpts::default(), CURVE_TOLERANCE / scale);

        self.fill(&outline, Fill::NonZero, transform, paint, BlendMode::default(), anti_alias);
    }

    fn draw_glyph_run(
        &mut self,
        glyph_run: &VelloSceneGlyphRun<'_>,
        transform: Affine,
        paint: &VelloScenePaint,
        anti_alias: bool,
    ) {
        if glyph_run.glyphs.is_empty() || !(glyph_run.font_size.is_finite() && glyph_run.font_size > 0.0) {
            return;
        }

        // As the CPU mode draws a run, with the glyph renderer both scenes
        // share (`glifo`); see there for the bold simulation and hinting.
        let (paint_type, _) = self.paint(paint, transform);
        self.set_anti_alias(anti_alias);
        self.scene.set_blend_mode(BlendMode::default());
        self.scene.set_transform(transform);
        self.scene.set_fill_rule(Fill::NonZero);
        self.scene.set_paint(paint_type);
        self.scene.set_paint_transform(paint.transform);

        let emboldened = glyph_run.embolden > 0.0;
        let scene = &mut self.scene;

        // The glyphs are prepared with what the renderer of the canvas
        // keeps between frames. A canvas whose renderer was released draws
        // nothing any more, glyphs included.
        self.gpu.with_resources(|resources| {
            let mut builder = scene
                .glyph_run(resources, glyph_run.font)
                .font_size(glyph_run.font_size)
                .normalized_coords(glyph_run.normalized_coords)
                .hint(glyph_run.hint && !emboldened);

            if emboldened {
                let amount = glyph_run.embolden * glyph_run.units_per_em.max(1) as f64 / glyph_run.font_size as f64;
                builder = builder.font_embolden(FontEmbolden::new(Diagonal2::new(amount, amount)));
            }

            if glyph_run.skew != 0.0 {
                builder = builder.glyph_transform(Affine::skew(glyph_run.skew, 0.0));
            }

            // The glyphs of a font with bitmap strikes are images, which
            // this renderer takes from an atlas of the context only: the
            // glyph atlas of the renderer is on for such a font, and for no
            // other (the hybrid sink over `wgpu` says why).
            if has_bitmap_strikes(glyph_run.font) {
                builder = builder.atlas_cache(true);
            }

            // A glyph the renderer has no representation of is left out by
            // it and reported: the rest of the run is drawn.
            let _ = builder
                .fill_glyphs(glyph_run.glyphs.iter().map(|glyph| Glyph { id: glyph.id, x: glyph.x, y: glyph.y }));
        });
    }

    fn push_clip(&mut self, path: &BezPath, fill_rule: Fill, transform: Affine, anti_alias: bool) {
        let (path, path_transform) = prepare(path, transform);

        self.set_anti_alias(anti_alias);
        self.scene.set_transform(path_transform);
        self.scene.set_fill_rule(fill_rule);
        self.scene.push_clip_path(&path);
    }

    fn pop_clip(&mut self) {
        self.scene.pop_clip();
    }

    fn push_layer(&mut self, blend_mode: BlendMode, opacity: f32) {
        self.scene.push_layer(None, Some(blend_mode), Some(opacity), None, None);
    }

    fn pop_layer(&mut self) {
        self.scene.pop_layer();
    }

    fn render_to_pixels(&mut self, _pixels: &mut [u8]) {
        // Nothing asks for it: a target in memory gets a scene of the CPU
        // mode (`scene::create_scene_sink`), and this sink is made by the
        // render target of a canvas alone. The pixels stay as they were.
        log_render_failure(
            "The scene of a canvas is rendered into the drawing buffer of the canvas, not into memory (stage 9 of \
             docs/porting/vello-backend.md)",
        );
    }

    fn render_to_canvas(&mut self) -> Result<(), String> {
        self.gpu.render(&self.scene, &self.images)
    }
}

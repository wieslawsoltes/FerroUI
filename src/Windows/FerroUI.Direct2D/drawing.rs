use crate::{
    bitmap::Bitmap,
    ffi,
    fonts::{NativeFont, Typeface},
    geometry::{self, Geometry},
};
use ferroui_base::{
    media::{imaging::BitmapBlendingMode, *},
    platform::*,
    Matrix, PixelSize, Point, Rect, RoundedRect, Vector,
};
use kurbo::{BezPath, Shape};
use std::{
    any::{Any, TypeId},
    cell::RefCell,
    hash::{DefaultHasher, Hash, Hasher},
    rc::Rc,
    sync::{Arc, OnceLock},
};
#[cfg(feature = "gpu")]
#[path = "effects.rs"]
mod effects;
#[cfg(feature = "gpu")]
#[path = "tiles.rs"]
mod tiles;

pub(crate) fn unsupported(feature: &'static str) {
    static WARNED: std::sync::Mutex<Vec<&str>> = std::sync::Mutex::new(Vec::new());
    let mut warned = WARNED.lock().unwrap();
    if !warned.contains(&feature) {
        eprintln!("Direct2D prototype: unsupported {feature}");
        warned.push(feature)
    }
}
pub(crate) struct PositionedRun(ffi::Owned);
// SAFETY: native glyph data is immutable. Its bounded ink-bounds cache and all
// native access/release are serialized by the Windows bridge.
unsafe impl Sync for PositionedRun {}
pub(crate) struct NativePath(ffi::Owned);
// A finished Direct2D path is immutable. Bridge access and release are serialized.
unsafe impl Sync for NativePath {}
pub(crate) enum DrawResource {
    Glyphs(Arc<PositionedRun>),
    Path(Arc<NativePath>),
}
#[derive(Default)]
pub(crate) struct DrawBatch {
    pub commands: Vec<ffi::DrawCommand>,
    pub glyphs: Vec<Arc<PositionedRun>>,
    pub paths: Vec<Arc<NativePath>>,
}
// SAFETY: each queued pointer refers to an immutable resource retained until
// the batch is flushed. Bitmap access and queuing are serialized by its mutexes.
unsafe impl Send for DrawBatch {}
pub struct GlyphRun {
    pub font: Arc<NativeFont>,
    simulations: FontSimulations,
    pub size: f64,
    pub origin: Point,
    pub ids: Vec<u16>,
    pub positions: Vec<f64>,
    advances: Vec<f64>,
    outlines: OnceLock<BezPath>,
    positioned: OnceLock<Arc<PositionedRun>>,
    pub bounds: Rect,
}
impl GlyphRun {
    pub fn new(
        face: &GlyphTypeface,
        size: f64,
        glyphs: &[text_formatting::GlyphInfo],
        origin: Point,
    ) -> Self {
        let simulations = face.platform_typeface().font_simulations();
        let font = face
            .platform_typeface()
            .as_any()
            .downcast_ref::<Typeface>()
            .expect("Direct2D typeface")
            .font
            .clone();
        let mut ids = Vec::with_capacity(glyphs.len());
        let mut positions = Vec::with_capacity(glyphs.len().saturating_mul(2));
        let mut advances = Vec::with_capacity(glyphs.len());
        let mut x = origin.x;
        for g in glyphs {
            ids.push(g.glyph_index);
            advances.push(g.glyph_advance);
            positions.extend([x + g.glyph_offset.x, origin.y + g.glyph_offset.y]);
            x += g.glyph_advance;
        }
        let mut b = [0.; 4];
        if simulations.is_empty() {
            unsafe {
                ffi::d2_glyph_run_bounds(
                    font.raw(),
                    size,
                    ids.as_ptr(),
                    positions.as_ptr(),
                    ids.len(),
                    b.as_mut_ptr(),
                );
            }
        }
        let bounds = Rect::new(b[0], b[1], b[2], b[3]);
        let mut result = Self {
            font,
            simulations,
            size,
            origin,
            ids,
            positions,
            advances,
            outlines: OnceLock::new(),
            positioned: OnceLock::new(),
            bounds,
        };
        if !simulations.is_empty() {
            // Synthetic bold/oblique change ink bounds and use outline drawing.
            let b = result.outlines().bounding_box();
            result.bounds = Rect::new(b.x0, b.y0, b.width(), b.height());
        }
        result
    }
    pub(crate) fn outlines(&self) -> &BezPath {
        self.outlines.get_or_init(|| {
            let mut outlines = BezPath::new();
            for (&id, p) in self.ids.iter().zip(self.positions.chunks_exact(2)) {
                let raw = unsafe { ffi::d2_glyph_path(self.font.raw(), id, self.size) };
                if raw.is_null() {
                    continue;
                }
                let path = unsafe { ffi::Owned::new(raw) };
                let mut outline = geometry::from_native(path.0);
                if self.simulations.contains(FontSimulations::Oblique) {
                    outline = kurbo::Affine::new([1., 0., -0.2, 1., 0., 0.]) * outline;
                }
                if self.simulations.contains(FontSimulations::Bold) {
                    let raw = geometry::native(&outline);
                    unsafe {
                        let stroke = ffi::Owned::new(ffi::d2_stroke(
                            raw.0,
                            self.size / 24.,
                            1,
                            1,
                            10.,
                            std::ptr::null(),
                            0,
                            0.,
                        ));
                        let union = ffi::d2_combine(raw.0, stroke.0, 0, 0, 0);
                        if !union.is_null() {
                            outline = geometry::from_native(ffi::Owned::new(union).0);
                        }
                    }
                }
                outlines.extend(
                    (kurbo::Affine::translate((p[0], p[1])) * outline)
                        .elements()
                        .iter()
                        .copied(),
                );
            }
            outlines
        })
    }
    fn positioned(&self) -> Arc<PositionedRun> {
        self.positioned
            .get_or_init(|| unsafe {
                Arc::new(PositionedRun(ffi::Owned::new(
                    ffi::d2_positioned_glyph_run(
                        self.font.raw(),
                        self.size,
                        self.ids.as_ptr(),
                        self.positions.as_ptr(),
                        self.ids.len(),
                        self.advances.as_ptr(),
                    ),
                )))
            })
            .clone()
    }
}
impl IGlyphRunImpl for GlyphRun {
    fn font_rendering_em_size(&self) -> f64 {
        self.size
    }
    fn baseline_origin(&self) -> Point {
        self.origin
    }
    fn bounds(&self) -> Rect {
        self.bounds
    }
    fn get_intersections(&self, lower: f32, upper: f32) -> Vec<f32> {
        if !lower.is_finite() || !upper.is_finite() || upper <= lower {
            return vec![];
        }
        let outlines = self.outlines();
        if outlines.is_empty() {
            return vec![];
        }
        let path = geometry::native(
            &(kurbo::Affine::translate((-self.origin.x, -self.origin.y)) * outlines.clone()),
        );
        let bytes = unsafe {
            ffi::Owned::new(ffi::d2_path_intercepts(path.0, lower as f64, upper as f64)).bytes()
        };
        let mut intervals: Vec<_> = bytes
            .chunks_exact(8)
            .map(|p| {
                (
                    f32::from_ne_bytes(p[..4].try_into().unwrap()),
                    f32::from_ne_bytes(p[4..].try_into().unwrap()),
                )
            })
            .collect();
        intervals.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut merged: Vec<(f32, f32)> = vec![];
        for (a, b) in intervals {
            if let Some(last) = merged.last_mut() {
                if a <= last.1 {
                    last.1 = last.1.max(b);
                    continue;
                }
            }
            merged.push((a, b));
        }
        merged.into_iter().flat_map(|(a, b)| [a, b]).collect()
    }
    fn dispose(&self) {}
    fn as_any(&self) -> &dyn Any {
        self
    }
}
fn color(c: Color, opacity: f64) -> [f64; 4] {
    [
        c.r as f64 / 255.,
        c.g as f64 / 255.,
        c.b as f64 / 255.,
        c.a as f64 / 255. * opacity,
    ]
}
fn rounded(r: RoundedRect) -> BezPath {
    let b = r.rect;
    let mut p = BezPath::new();
    let [tl, tr, br, bl] = [
        r.radii_top_left,
        r.radii_top_right,
        r.radii_bottom_right,
        r.radii_bottom_left,
    ]
    .map(|v| {
        Vector::new(
            v.x.max(0.).min(b.width / 2.),
            v.y.max(0.).min(b.height / 2.),
        )
    });
    let k = 0.5522847498307936;
    p.move_to((b.x + tl.x, b.y));
    p.line_to((b.right() - tr.x, b.y));
    p.curve_to(
        (b.right() - tr.x + k * tr.x, b.y),
        (b.right(), b.y + tr.y - k * tr.y),
        (b.right(), b.y + tr.y),
    );
    p.line_to((b.right(), b.bottom() - br.y));
    p.curve_to(
        (b.right(), b.bottom() - br.y + k * br.y),
        (b.right() - br.x + k * br.x, b.bottom()),
        (b.right() - br.x, b.bottom()),
    );
    p.line_to((b.x + bl.x, b.bottom()));
    p.curve_to(
        (b.x + bl.x - k * bl.x, b.bottom()),
        (b.x, b.bottom() - bl.y + k * bl.y),
        (b.x, b.bottom() - bl.y),
    );
    p.line_to((b.x, b.y + tl.y));
    p.curve_to(
        (b.x, b.y + tl.y - k * tl.y),
        (b.x + tl.x - k * tl.x, b.y),
        (b.x + tl.x, b.y),
    );
    p.close_path();
    p
}

#[cfg(feature = "gpu")]
struct CachedRoundedPath {
    key: [u64; 12],
    path: Arc<NativePath>,
}

#[cfg(feature = "gpu")]
fn rounded_native(r: RoundedRect) -> Arc<NativePath> {
    ferroui_base::ferro_thread_local! {
        static PATHS: RefCell<Box<[Option<CachedRoundedPath>]>> =
            RefCell::new(std::iter::repeat_with(|| None).take(2048).collect());
    }
    let key = [
        r.rect.x, r.rect.y, r.rect.width, r.rect.height,
        r.radii_top_left.x, r.radii_top_left.y,
        r.radii_top_right.x, r.radii_top_right.y,
        r.radii_bottom_right.x, r.radii_bottom_right.y,
        r.radii_bottom_left.x, r.radii_bottom_left.y,
    ].map(f64::to_bits);
    let mut hash = DefaultHasher::new();
    key.hash(&mut hash);
    PATHS.with_borrow_mut(|paths| {
        let index = hash.finish() as usize % paths.len();
        let slot = &mut paths[index];
        if let Some(cached) = slot.as_ref().filter(|cached| cached.key == key) {
            return cached.path.clone();
        }
        let path = Arc::new(NativePath(geometry::native(&rounded(r))));
        *slot = Some(CachedRoundedPath { key, path: path.clone() });
        path
    })
}

fn solid_color(brush: &dyn IBrush) -> Option<Color> {
    // Immediate drawing can hand the backend a XAML brush without its
    // composition graph. Use the brush's fallback color in that case.
    brush.as_solid_color_brush()
        .map(|solid| solid.color())
        .or_else(|| XamlCompositionBrushBase::of(brush).map(|xaml| xaml.fallback_color()))
}
fn blend(b: BitmapBlendingMode) -> Option<i32> {
    Some(match b {
        BitmapBlendingMode::Unspecified | BitmapBlendingMode::SourceOver => 0,
        BitmapBlendingMode::Multiply => 1,
        BitmapBlendingMode::Screen => 2,
        BitmapBlendingMode::Overlay => 3,
        BitmapBlendingMode::Darken => 4,
        BitmapBlendingMode::Lighten => 5,
        BitmapBlendingMode::ColorDodge => 6,
        BitmapBlendingMode::ColorBurn => 7,
        BitmapBlendingMode::SoftLight => 8,
        BitmapBlendingMode::HardLight => 9,
        BitmapBlendingMode::Difference => 10,
        BitmapBlendingMode::Exclusion => 11,
        BitmapBlendingMode::Hue => 12,
        BitmapBlendingMode::Saturation => 13,
        BitmapBlendingMode::Color => 14,
        BitmapBlendingMode::Luminosity => 15,
        BitmapBlendingMode::Source => 17,
        BitmapBlendingMode::SourceIn => 18,
        BitmapBlendingMode::SourceOut => 19,
        BitmapBlendingMode::SourceAtop => 20,
        BitmapBlendingMode::DestinationOver => 21,
        BitmapBlendingMode::DestinationIn => 22,
        BitmapBlendingMode::DestinationOut => 23,
        BitmapBlendingMode::DestinationAtop => 24,
        BitmapBlendingMode::Xor => 25,
        BitmapBlendingMode::Plus => 27,
        BitmapBlendingMode::Destination => return None,
    })
}
pub struct DrawingContext {
    bitmap: Bitmap,
    transform: Matrix,
    scale: Vector,
    finished: Option<Box<dyn FnOnce()>>,
    disposed: bool,
    masks: Vec<(Option<SharedBrush>, Rect, Matrix)>,
    options: Vec<RenderOptions>,
    text_options: Vec<TextOptions>,
    #[cfg(feature = "gpu")]
    effects: Vec<effects::State>,
    #[cfg(feature = "gpu")]
    pixels: Vec<effects::PixelState>,
}
impl DrawingContext {
    pub fn new(bitmap: Bitmap, scaled: bool, finished: Option<Box<dyn FnOnce()>>) -> Self {
        let dpi = IBitmapImpl::dpi(&bitmap);
        bitmap.context(|c| unsafe { ffi::d2_save(c) });
        Self {
            bitmap,
            transform: Matrix::IDENTITY,
            scale: if scaled {
                Vector::new(dpi.x / 96., dpi.y / 96.)
            } else {
                Vector::new(1., 1.)
            },
            finished,
            disposed: false,
            masks: Vec::new(),
            options: Vec::new(),
            text_options: Vec::new(),
            #[cfg(feature = "gpu")]
            effects: Vec::new(),
            #[cfg(feature = "gpu")]
            pixels: Vec::new(),
        }
    }
    fn prepare(&self) -> bool {
        let m = self.transform;
        // Never install a singular CTM: resetting it with its inverse would
        // poison subsequent draws in the same native context.
        if (m.m11 * m.m22 - m.m12 * m.m21).abs() < 1e-15 {
            return false;
        }
        self.bitmap.context(|c| unsafe {
            ffi::d2_set_transform(
                c,
                m.m11,
                m.m12,
                m.m21,
                m.m22,
                m.m31,
                m.m32,
                self.scale.x,
                self.scale.y,
            );
            #[cfg(feature = "gpu")]
            {
                let options = self.options.last().copied().unwrap_or_default();
                ffi::d2_gpu_render_options(
                    c,
                    blend(options.bitmap_blending_mode).unwrap_or(28),
                    (options.edge_mode == EdgeMode::Aliased) as i32,
                    options.bitmap_interpolation_mode as i32,
                );
            }
        });
        true
    }
    fn command(&self, kind: i32) -> Option<ffi::DrawCommand> {
        let m = self.transform;
        if (m.m11 * m.m22 - m.m12 * m.m21).abs() < 1e-15 {
            return None;
        }
        let options = self.options.last().copied().unwrap_or_default();
        Some(ffi::DrawCommand {
            kind,
            matrix: [
                m.m11 * self.scale.x,
                m.m12 * self.scale.y,
                m.m21 * self.scale.x,
                m.m22 * self.scale.y,
                m.m31 * self.scale.x,
                m.m32 * self.scale.y,
            ],
            blend: blend(options.bitmap_blending_mode).unwrap_or(28),
            aliased: (options.edge_mode == EdgeMode::Aliased) as i32,
            interpolation: options.bitmap_interpolation_mode as i32,
            ..Default::default()
        })
    }
    #[cfg(feature = "gpu")]
    fn queue_solid(
        &self,
        geometry: impl FnOnce() -> Arc<NativePath>,
        paint: Color,
        opacity: f64,
        rule: FillRule,
    ) {
        if let Some(mut command) = self.command(3) {
            let geometry = geometry();
            command.resource = geometry.0 .0;
            command.params = color(paint, opacity);
            command.fill_even_odd = i32::from(rule == FillRule::EvenOdd);
            self.bitmap.queue(command, Some(DrawResource::Path(geometry)));
        }
    }
    fn fill(&mut self, path: &BezPath, rule: FillRule, brush: &dyn IBrush, bounds: impl FnOnce() -> Rect) {
        #[cfg(feature = "gpu")]
        if let Some(paint) = solid_color(brush) {
            self.queue_solid(
                || Arc::new(NativePath(geometry::native(path))),
                paint,
                brush.opacity(),
                rule,
            );
            return;
        }
        if !self.prepare() {
            return;
        }
        let native = geometry::native(path);
        let opacity = brush.opacity();
        #[cfg(not(feature = "gpu"))]
        if let Some(solid_color) = solid_color(brush) {
            let [r, g, b, a] = color(solid_color, opacity);
            self.bitmap.context(|c| unsafe {
                ffi::d2_fill(
                    c,
                    native.0,
                    i32::from(rule == FillRule::EvenOdd),
                    r,
                    g,
                    b,
                    a,
                )
            });
            return;
        }
        let bounds = bounds();
        self.bitmap.context(|c| unsafe {
            ffi::d2_save(c);
            ffi::d2_clip(c, native.0, i32::from(rule == FillRule::EvenOdd))
        });
        if let Some(gradient) = brush.as_gradient_brush() {
            // The geometry clip stays in the visual's coordinates. Brush transforms
            // affect only paint coordinates, including WinUI's reflected underline.
            let mut paint = Matrix::IDENTITY;
            if let Some(relative) = brush
                .relative_transform()
                .filter(|_| bounds.width > 0. && bounds.height > 0.)
            {
                paint = Matrix::create_translation(-bounds.x, -bounds.y)
                    * Matrix::create_scale(1. / bounds.width, 1. / bounds.height)
                    * relative.value()
                    * Matrix::create_scale(bounds.width, bounds.height)
                    * Matrix::create_translation(bounds.x, bounds.y);
            }
            if let Some(absolute) = brush.transform() {
                let origin = brush.transform_origin().to_pixels_rect(bounds);
                paint *= Matrix::create_translation(-origin.x, -origin.y)
                    * absolute.value()
                    * Matrix::create_translation(origin.x, origin.y);
            }
            let paint = paint * self.transform;
            if paint.try_invert().is_none() {
                self.bitmap.context(|c| unsafe { ffi::d2_restore(c) });
                return;
            }
            self.bitmap.context(|c| unsafe {
                ffi::d2_set_transform(
                    c,
                    paint.m11,
                    paint.m12,
                    paint.m21,
                    paint.m22,
                    paint.m31,
                    paint.m32,
                    self.scale.x,
                    self.scale.y,
                )
            });
            #[cfg(feature = "gpu")]
            self.bitmap.context(|c| unsafe {
                ffi::d2_gpu_gradient_options(c, gradient.spread_method() as i32, 1., 0.)
            });
            let stops = gradient.gradient_stops();
            let mut entries: Vec<_> = stops
                .iter()
                .map(|s| (s.offset().clamp(0., 1.), color(s.color(), opacity)))
                .collect();
            entries.sort_by(|a, b| a.0.total_cmp(&b.0));
            if !entries.is_empty() {
                let locations: Vec<_> = entries.iter().map(|e| e.0).collect();
                let colors: Vec<_> = entries.iter().flat_map(|e| e.1).collect();
                if let Some(g) = brush.as_linear_gradient_brush() {
                    let a = g.start_point().to_pixels_rect(bounds);
                    let b = g.end_point().to_pixels_rect(bounds);
                    self.bitmap.context(|c| unsafe {
                        ffi::d2_gradient(
                            c,
                            colors.as_ptr(),
                            locations.as_ptr(),
                            locations.len(),
                            a.x,
                            a.y,
                            b.x,
                            b.y,
                            0.,
                            0,
                        )
                    });
                } else if let Some(g) = brush.as_radial_gradient_brush() {
                    let a = g.gradient_origin().to_pixels_rect(bounds);
                    let b = g.center().to_pixels_rect(bounds);
                    let radius = g.radius_x().to_value(bounds.width).abs();
                    #[cfg(feature = "gpu")]
                    self.bitmap.context(|c| unsafe {
                        ffi::d2_gpu_gradient_options(
                            c,
                            gradient.spread_method() as i32,
                            radius / g.radius_y().to_value(bounds.height).abs().max(1e-12),
                            0.,
                        )
                    });
                    self.bitmap.context(|c| unsafe {
                        ffi::d2_gradient(
                            c,
                            colors.as_ptr(),
                            locations.as_ptr(),
                            locations.len(),
                            a.x,
                            a.y,
                            b.x,
                            b.y,
                            radius,
                            1,
                        )
                    });
                } else if let Some(g) = brush.as_conic_gradient_brush() {
                    #[cfg(feature = "gpu")]
                    {
                        let center = g.center().to_pixels_rect(bounds);
                        self.bitmap.context(|c| unsafe {
                            ffi::d2_gpu_gradient_options(
                                c,
                                gradient.spread_method() as i32,
                                1.,
                                g.angle(),
                            );
                            ffi::d2_gradient(
                                c,
                                colors.as_ptr(),
                                locations.as_ptr(),
                                locations.len(),
                                center.x,
                                center.y,
                                center.x,
                                center.y,
                                0.,
                                2,
                            );
                        });
                    }
                    #[cfg(not(feature = "gpu"))]
                    {
                        let _ = g;
                        unsupported("conic gradient");
                    }
                }
            }
        } else if let Some(image) = brush.as_image_brush() {
            #[cfg(feature = "gpu")]
            if let Some(bitmap) = image.source().and_then(|s| s.get_bitmap()) {
                self.tile_brush(image, &*bitmap, bounds);
            }
            #[cfg(not(feature = "gpu"))]
            {
                if let Some(bitmap) = image.source().and_then(|s| s.get_bitmap()) {
                    let source_size = bitmap.pixel_size().to_size_with_dpi(bitmap.dpi().x);
                    let calc = ferroui_base::rendering::utilities::TileBrushCalculator::from_brush(
                        image,
                        source_size,
                        bounds.size(),
                    );
                    let d = calc.destination_rect();
                    let src = calc.source_rect();
                    let dpi = bitmap.dpi();
                    self.draw_bitmap(
                        &*bitmap,
                        opacity,
                        Rect::new(
                            src.x * dpi.x / 96.,
                            src.y * dpi.y / 96.,
                            src.width * dpi.x / 96.,
                            src.height * dpi.y / 96.,
                        ),
                        Rect::new(bounds.x + d.x, bounds.y + d.y, d.width, d.height),
                    );
                }
            }
        } else if let Some(scene) = brush.as_scene_brush() {
            #[cfg(feature = "gpu")]
            self.scene_brush(scene, bounds);
            #[cfg(not(feature = "gpu"))]
            if let Some(content) = scene.create_content() {
                content.render(self, None);
                content.dispose();
            }
        } else {
            unsupported(
                brush
                    .as_object()
                    .map_or("immutable brush", |object| object.get_type().name()),
            )
        }
        self.bitmap.context(|c| unsafe { ffi::d2_restore(c) });
    }
    fn shape(
        &mut self,
        brush: Option<&dyn IBrush>,
        pen: Option<&dyn IPen>,
        stroke: &BezPath,
        fill: Option<&BezPath>,
        rule: FillRule,
    ) {
        let bounds = || {
            let b = stroke.bounding_box();
            Rect::new(b.x0, b.y0, b.width(), b.height())
        };
        if let (Some(brush), Some(path)) = (brush, fill) {
            self.fill(path, rule, brush, bounds)
        }
        if let Some(pen) = pen {
            if let Some(brush) = pen.brush() {
                #[cfg(feature = "gpu")]
                if let Some(paint) = solid_color(&*brush) {
                    self.queue_solid(
                        || Arc::new(NativePath(Geometry::widened(stroke, pen))),
                        paint,
                        brush.opacity(),
                        FillRule::NonZero,
                    );
                    return;
                }
                self.fill(
                    &Geometry::stroked(stroke, pen),
                    FillRule::NonZero,
                    &*brush,
                    bounds,
                )
            }
        }
    }
}
impl IDrawingContextImpl for DrawingContext {
    fn transform(&self) -> Matrix {
        self.transform
    }
    fn set_transform(&mut self, m: Matrix) {
        self.transform = m
    }
    fn clear(&mut self, c: Color) {
        let [r, g, b, a] = color(c, 1.);
        let size = self.bitmap.pixel_size();
        self.bitmap.context(|ctx| unsafe {
            ffi::d2_clear(ctx, size.width as f64, size.height as f64, r, g, b, a)
        })
    }
    fn draw_bitmap(&mut self, source: &dyn IBitmapImpl, opacity: f64, src: Rect, dst: Rect) {
        let converted = source
            .as_any()
            .downcast_ref::<crate::external_objects::SharedFrameBitmap>()
            .and_then(|f| f.bitmap());
        if source
            .as_any()
            .is::<crate::external_objects::SharedFrameBitmap>()
            && converted.is_none()
        {
            return;
        }
        let source: &dyn IBitmapImpl = converted.as_ref().map_or(source, |b| b as &dyn IBitmapImpl);
        let Some(bitmap) = Bitmap::from_impl(source) else {
            unsupported("foreign bitmap");
            return;
        };
        if !self.prepare() {
            return;
        }
        bitmap.draw_to(&self.bitmap, src, dst, opacity)
    }
    fn draw_bitmap_with_mask(
        &mut self,
        source: &dyn IBitmapImpl,
        mask: &dyn IBrush,
        bounds: Rect,
        dst: Rect,
    ) {
        self.push_opacity_mask(mask, bounds);
        let size = source.pixel_size();
        self.draw_bitmap(
            source,
            1.,
            Rect::new(0., 0., size.width as f64, size.height as f64),
            dst,
        );
        self.pop_opacity_mask()
    }
    fn draw_line(&mut self, pen: Option<&dyn IPen>, a: Point, b: Point) {
        let mut path = BezPath::new();
        path.move_to((a.x, a.y));
        path.line_to((b.x, b.y));
        self.shape(None, pen, &path, None, FillRule::NonZero)
    }
    fn draw_geometry(
        &mut self,
        brush: Option<&dyn IBrush>,
        pen: Option<&dyn IPen>,
        g: &dyn IGeometryImpl,
    ) {
        if let Some(g) = g.as_any().downcast_ref::<Geometry>() {
            self.shape(brush, pen, &g.stroke, g.fill.as_ref(), g.rule)
        }
    }
    fn draw_rectangle(
        &mut self,
        brush: Option<&dyn IBrush>,
        pen: Option<&dyn IPen>,
        r: RoundedRect,
        shadows: &BoxShadows,
    ) {
        #[cfg(feature = "gpu")]
        {
            for shadow in shadows.iter().filter(|s| !s.is_inset) {
                self.box_shadow_gpu(r, &shadow);
            }
            let mut path = None;
            if let Some(brush) = brush {
                if let Some(paint) = solid_color(brush) {
                    self.queue_solid(|| rounded_native(r), paint, brush.opacity(), FillRule::NonZero);
                } else {
                    let path = path.insert(rounded(r));
                    self.shape(Some(brush), None, path, Some(path), FillRule::NonZero);
                }
            }
            for shadow in shadows.iter().filter(|s| s.is_inset) {
                self.box_shadow_gpu(r, &shadow);
            }
            if let Some(pen) = pen {
                let path = path.get_or_insert_with(|| rounded(r));
                self.shape(None, Some(pen), path, None, FillRule::NonZero);
            }
            return;
        }
        #[cfg(not(feature = "gpu"))]
        {
            let path = rounded(r);
            for shadow in shadows.iter() {
                if shadow.is_inset {
                    unsupported("inset shadow");
                    continue;
                }
                if !self.prepare() {
                    return;
                }
                let [red, g, b, a] = color(shadow.color, 1.);
                self.bitmap.context(|c| unsafe {
                    ffi::d2_save(c);
                    ffi::d2_shadow(
                        c,
                        shadow.offset_x,
                        shadow.offset_y,
                        shadow.blur,
                        red,
                        g,
                        b,
                        a,
                    )
                });
                let shadow_brush = immutable::ImmutableSolidColorBrush::new(shadow.color);
                self.fill(&path, FillRule::NonZero, &shadow_brush, || r.rect);
                self.bitmap.context(|c| unsafe { ffi::d2_restore(c) });
            }
            self.shape(brush, pen, &path, Some(&path), FillRule::NonZero)
        }
    }
    fn draw_region(
        &mut self,
        brush: Option<&dyn IBrush>,
        pen: Option<&dyn IPen>,
        region: &dyn IPlatformRenderInterfaceRegion,
    ) {
        for r in region.rects() {
            self.draw_rectangle(
                brush,
                pen,
                RoundedRect::from_rect(Rect::new(
                    r.left as f64,
                    r.top as f64,
                    (r.right - r.left) as f64,
                    (r.bottom - r.top) as f64,
                )),
                &BoxShadows::default(),
            )
        }
    }
    fn draw_ellipse(&mut self, brush: Option<&dyn IBrush>, pen: Option<&dyn IPen>, r: Rect) {
        let g = Geometry::ellipse(r);
        self.draw_geometry(brush, pen, &*g)
    }
    fn draw_glyph_run(&mut self, brush: Option<&dyn IBrush>, run: &dyn IGlyphRunImpl) {
        let (Some(brush), Some(run)) = (brush, run.as_any().downcast_ref::<GlyphRun>()) else {
            return;
        };
        if let Some(solid) = brush
            .as_solid_color_brush()
            .filter(|_| run.simulations.is_empty())
        {
            let [r, g, b, a] = color(solid.color(), brush.opacity());
            let Some(mut command) = self.command(2) else {
                return;
            };
            #[cfg(feature = "gpu")]
            let (shift, aliased) = {
                let text = self.text_options.last().copied().unwrap_or_default();
                let options = self.options.last().copied().unwrap_or_default();
                let mode = if text.text_rendering_mode == TextRenderingMode::Unspecified {
                    options.text_rendering_mode
                } else {
                    text.text_rendering_mode
                };
                let shift = if text.baseline_pixel_alignment != BaselinePixelAlignment::Unaligned
                    && self.transform.m12 == 0.
                    && self.transform.m21 == 0.
                {
                    let baseline =
                        (run.origin.y * self.transform.m22 + self.transform.m32) * self.scale.y;
                    (baseline.round() - baseline) / (self.transform.m22 * self.scale.y)
                } else {
                    0.
                };
                (shift, (mode == TextRenderingMode::Alias) as i32)
            };
            #[cfg(not(feature = "gpu"))]
            let (shift, aliased) = (0., 0);
            let positioned = run.positioned();
            command.resource = positioned.0 .0;
            command.params = [r, g, b, a];
            command.shift = shift;
            command.text_aliased = aliased;
            self.bitmap.queue(command, Some(DrawResource::Glyphs(positioned)));
        } else {
            self.fill(run.outlines(), FillRule::NonZero, brush, || run.bounds)
        }
    }
    fn create_layer(&mut self, size: PixelSize) -> Rc<dyn IDrawingContextLayerImpl> {
        Rc::new(Bitmap::new(size, Vector::new(96., 96.)))
    }
    fn push_clip(&mut self, r: Rect) {
        let Some(mut command) = self.command(0) else {
            self.push_geometry_clip(&*Geometry::new(BezPath::new(), FillRule::NonZero));
            return;
        };
        command.params = [r.x, r.y, r.width, r.height];
        self.bitmap.queue(command, None);
    }
    fn push_clip_rounded(&mut self, r: RoundedRect) {
        if !r.is_rounded() {
            self.push_clip(r.rect);
            return;
        }
        #[cfg(feature = "gpu")]
        if let Some(mut command) = self.command(4) {
            let geometry = rounded_native(r);
            command.resource = geometry.0 .0;
            self.bitmap.queue(command, Some(DrawResource::Path(geometry)));
            return;
        }
        self.push_geometry_clip(&*Geometry::new(rounded(r), FillRule::NonZero))
    }
    fn push_clip_region(&mut self, region: &dyn IPlatformRenderInterfaceRegion) {
        let mut path = BezPath::new();
        for r in region.rects() {
            path.extend(
                Geometry::rectangle(Rect::new(
                    r.left as f64,
                    r.top as f64,
                    (r.right - r.left) as f64,
                    (r.bottom - r.top) as f64,
                ))
                .stroke
                .elements()
                .iter()
                .copied(),
            )
        }
        self.push_geometry_clip(&*Geometry::new(path, FillRule::NonZero))
    }
    fn pop_clip(&mut self) {
        self.bitmap.queue(
            ffi::DrawCommand {
                kind: 1,
                ..Default::default()
            },
            None,
        );
    }
    fn push_layer(&mut self, _: Rect) {
        self.bitmap
            .context(|c| unsafe { ffi::d2_begin_layer(c, 1., 0) })
    }
    fn pop_layer(&mut self) {
        self.bitmap.context(|c| unsafe { ffi::d2_end_layer(c) })
    }
    fn try_push_compositing_layer(&mut self, b: BitmapBlendingMode, opacity: f64, _: Rect) -> bool {
        if let Some(mode) = blend(b) {
            self.bitmap
                .context(|c| unsafe { ffi::d2_begin_layer(c, opacity, mode) });
            true
        } else {
            false
        }
    }
    fn pop_compositing_layer(&mut self) {
        self.pop_layer()
    }
    fn push_opacity(&mut self, opacity: f64, _: Option<Rect>) {
        self.bitmap
            .context(|c| unsafe { ffi::d2_begin_layer(c, opacity, 0) })
    }
    fn pop_opacity(&mut self) {
        self.pop_layer()
    }
    fn push_opacity_mask(&mut self, mask: &dyn IBrush, bounds: Rect) {
        self.push_layer(bounds);
        let mask = SharedBrush::from_brush(mask).or_else(|| {
            XamlCompositionBrushBase::of(mask).and_then(|xaml| {
                SharedBrush::from_brush(&immutable::ImmutableSolidColorBrush::with_opacity(
                    xaml.fallback_color(),
                    mask.opacity(),
                ))
            })
        });
        self.masks.push((mask, bounds, self.transform));
    }
    fn pop_opacity_mask(&mut self) {
        if let Some((brush, bounds, m)) = self.masks.pop() {
            let old = self.transform;
            self.transform = m;
            self.bitmap
                .context(|c| unsafe { ffi::d2_begin_layer(c, 1., 22) });
            if let Some(brush) = brush {
                self.fill(
                    &Geometry::rectangle(bounds).stroke,
                    FillRule::NonZero,
                    &brush,
                    || bounds,
                )
            } else {
                unsupported("opacity mask brush")
            }
            self.pop_layer();
            self.pop_layer();
            self.transform = old;
        }
    }
    fn push_geometry_clip(&mut self, g: &dyn IGeometryImpl) {
        if !self.prepare() {
            let empty = geometry::native(&BezPath::new());
            self.bitmap.context(|c| unsafe {
                ffi::d2_save(c);
                ffi::d2_clip(c, empty.0, 0);
            });
            return;
        }
        self.bitmap.context(|c| unsafe { ffi::d2_save(c) });
        if let Some(g) = g.as_any().downcast_ref::<Geometry>() {
            let path = geometry::native(g.fill.as_ref().unwrap_or(&g.stroke));
            self.bitmap.context(|c| unsafe {
                ffi::d2_clip(c, path.0, i32::from(g.rule == FillRule::EvenOdd))
            })
        }
    }
    fn pop_geometry_clip(&mut self) {
        self.pop_clip()
    }
    fn push_render_options(&mut self, o: RenderOptions) {
        self.options
            .push(o.merge_with(self.options.last().copied().unwrap_or_default()))
    }
    fn pop_render_options(&mut self) {
        self.options.pop();
    }
    fn push_text_options(&mut self, o: TextOptions) {
        self.text_options
            .push(o.merge_with(self.text_options.last().copied().unwrap_or_default()))
    }
    fn pop_text_options(&mut self) {
        self.text_options.pop();
    }
    #[cfg(feature = "gpu")]
    fn as_drawing_context_impl_with_effects(
        &mut self,
    ) -> Option<&mut dyn IDrawingContextImplWithEffects> {
        Some(self)
    }
    #[cfg(feature = "gpu")]
    fn as_drawing_context_impl_with_layer_effects(
        &mut self,
    ) -> Option<&mut dyn IDrawingContextImplWithLayerEffects> {
        Some(self)
    }
    #[cfg(feature = "gpu")]
    fn as_drawing_context_with_acrylic_like_support(
        &mut self,
    ) -> Option<&mut dyn IDrawingContextWithAcrylicLikeSupport> {
        Some(self)
    }
    fn get_feature(&mut self, _: TypeId) -> Option<Rc<dyn Any>> {
        None
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
    fn dispose(&mut self) {
        if !self.disposed {
            self.disposed = true;
            self.bitmap.context(|c| unsafe { ffi::d2_restore(c) });
            self.bitmap.finished();
            if let Some(f) = self.finished.take() {
                f()
            }
        }
    }
}
impl Drop for DrawingContext {
    fn drop(&mut self) {
        self.dispose()
    }
}

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
    rc::Rc,
    sync::Arc,
};
#[cfg(feature = "metal")]
#[path = "effects.rs"]
mod effects;
#[cfg(feature = "metal")]
#[path = "tiles.rs"]
mod tiles;

pub(crate) fn unsupported(feature: &'static str) {
    static WARNED: std::sync::Mutex<Vec<&str>> = std::sync::Mutex::new(Vec::new());
    let mut warned = WARNED.lock().unwrap();
    if !warned.contains(&feature) {
        eprintln!("Quartz prototype: unsupported {feature}");
        warned.push(feature)
    }
}
pub struct GlyphRun {
    pub font: Arc<NativeFont>,
    simulated: bool,
    pub size: f64,
    pub origin: Point,
    pub ids: Vec<u16>,
    pub positions: Vec<f64>,
    pub outlines: BezPath,
    #[cfg(feature = "metal")]
    color_indices: Vec<usize>,
    pub bounds: Rect,
}
impl GlyphRun {
    pub fn new(
        face: &GlyphTypeface,
        size: f64,
        glyphs: &[text_formatting::GlyphInfo],
        origin: Point,
    ) -> Self {
        let simulations = face.platform_typeface().font_simulations() | face.font_simulations();
        let font = face
            .platform_typeface()
            .as_any()
            .downcast_ref::<Typeface>()
            .expect("Quartz typeface")
            .font
            .clone();
        let mut ids = Vec::new();
        let mut positions = Vec::new();
        let mut outlines = BezPath::new();
        #[cfg(feature = "metal")]
        let mut color_indices = Vec::new();
        let mut x = origin.x;
        let mut ink: Option<kurbo::Rect> = None;
        for g in glyphs {
            ids.push(g.glyph_index);
            let p = Point::new(x + g.glyph_offset.x, origin.y + g.glyph_offset.y);
            positions.extend([p.x, p.y]);
            unsafe {
                // Bitmap/color glyphs can have ink without any outline path.
                let mut b = [0.0; 4];
                ffi::q_glyph_bounds(font.raw(), g.glyph_index, size, b.as_mut_ptr());
                if b.iter().all(|x| x.is_finite()) && b[2] > 0.0 && b[3] > 0.0 {
                    let r = kurbo::Rect::new(
                        p.x + b[0],
                        p.y + b[1],
                        p.x + b[0] + b[2],
                        p.y + b[1] + b[3],
                    );
                    ink = Some(ink.map_or(r, |old| old.union(r)));
                }
                let raw = ffi::q_glyph_path(font.raw(), g.glyph_index, size);
                if !raw.is_null() {
                    let path = ffi::Owned::new(raw);
                    let mut outline = geometry::from_native(path.0);
                    if simulations.contains(FontSimulations::Oblique) {
                        outline = kurbo::Affine::new([1., 0., -0.2, 1., 0., 0.]) * outline;
                    }
                    if simulations.contains(FontSimulations::Bold) {
                        let raw = geometry::native(&outline);
                        let stroke = ffi::Owned::new(ffi::q_stroke(
                            raw.0,
                            size / 24.,
                            1,
                            1,
                            10.,
                            std::ptr::null(),
                            0,
                            0.,
                        ));
                        let union = ffi::q_combine(raw.0, stroke.0, 0, 0, 0);
                        if !union.is_null() {
                            outline = geometry::from_native(ffi::Owned::new(union).0);
                        }
                    }
                    outlines.extend(
                        (kurbo::Affine::translate((p.x, p.y)) * outline)
                            .elements()
                            .iter()
                            .copied(),
                    );
                } else if b.iter().all(|v| v.is_finite()) && b[2] > 0. && b[3] > 0. {
                    #[cfg(feature = "metal")]
                    color_indices.push(ids.len() - 1);
                }
            }
            x += g.glyph_advance;
        }
        let b = ink.map_or_else(
            || outlines.bounding_box(),
            |ink| {
                if outlines.is_empty() {
                    ink
                } else {
                    ink.union(outlines.bounding_box())
                }
            },
        );
        let bounds = Rect::new(b.x0, b.y0, b.width(), b.height());
        Self {
            font,
            simulated: !simulations.is_empty(),
            size,
            origin,
            ids,
            positions,
            outlines,
            #[cfg(feature = "metal")]
            color_indices,
            bounds,
        }
    }
}
#[cfg(feature = "metal")]
impl DrawingContext {
    fn draw_color_glyphs(&mut self, brush: &dyn IBrush, run: &GlyphRun, shift: f64) {
        if run.color_indices.is_empty() || !self.prepare() {
            return;
        }
        let ids: Vec<_> = run.color_indices.iter().map(|i| run.ids[*i]).collect();
        let positions: Vec<_> = run
            .color_indices
            .iter()
            .flat_map(|i| [run.positions[*i * 2], run.positions[*i * 2 + 1] + shift])
            .collect();
        let bounds = run.bounds.translate(Vector::new(0., shift));
        self.push_layer(bounds);
        self.bitmap.context(|c| unsafe {
            ffi::q_save(c);
            #[cfg(feature = "metal")]
            ffi::qm_text_options(
                c,
                self.text_options
                    .last()
                    .copied()
                    .unwrap_or_default()
                    .text_hinting_mode as i32,
            );
            ffi::q_glyphs(
                c,
                run.font.raw(),
                run.size,
                ids.as_ptr(),
                positions.as_ptr(),
                ids.len(),
                1.,
                1.,
                1.,
                1.,
            );
            ffi::q_restore(c);
        });
        // A color font keeps its intrinsic RGB. The brush supplies only alpha,
        // including gradient stops, tiling and opacity. Pad the mask geometry
        // for raster coverage while keeping paint coordinates at the run bounds.
        self.bitmap
            .context(|c| unsafe { ffi::q_begin_layer(c, 1., 22) });
        self.fill(
            &Geometry::rectangle(bounds.inflate(2.)).stroke,
            FillRule::NonZero,
            brush,
            bounds,
        );
        self.pop_layer();
        self.pop_layer();
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
        if !lower.is_finite() || !upper.is_finite() || upper <= lower || self.outlines.is_empty() {
            return vec![];
        }
        let path = geometry::native(
            &(kurbo::Affine::translate((-self.origin.x, -self.origin.y)) * self.outlines.clone()),
        );
        let bytes = unsafe {
            ffi::Owned::new(ffi::q_path_intercepts(path.0, lower as f64, upper as f64)).bytes()
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
    if b.width <= 0. || b.height <= 0. {
        return p;
    }
    let mut radii = [
        r.radii_top_left,
        r.radii_top_right,
        r.radii_bottom_right,
        r.radii_bottom_left,
    ]
    .map(|v| {
        if !v.x.is_finite() || !v.y.is_finite() || v.x <= 0. || v.y <= 0. {
            Vector::ZERO
        } else {
            v
        }
    });
    // Neighboring radii share the available side. Scale all corners together
    // only when their sums overlap, preserving asymmetric elliptical corners.
    let [tl, tr, br, bl] = radii;
    let mut scale = 1.0_f64;
    for (length, sum) in [
        (b.width, tl.x + tr.x),
        (b.width, bl.x + br.x),
        (b.height, tl.y + bl.y),
        (b.height, tr.y + br.y),
    ] {
        if sum > length {
            scale = scale.min(length / sum);
        }
    }
    for radius in &mut radii {
        *radius *= scale;
    }
    let [tl, tr, br, bl] = radii;
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
    #[cfg(feature = "metal")]
    effects: Vec<effects::State>,
    #[cfg(feature = "metal")]
    pixels: Vec<effects::PixelState>,
}
impl DrawingContext {
    pub fn new(bitmap: Bitmap, scaled: bool, finished: Option<Box<dyn FnOnce()>>) -> Self {
        let dpi = IBitmapImpl::dpi(&bitmap);
        bitmap.context(|c| unsafe { ffi::q_save(c) });
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
            #[cfg(feature = "metal")]
            effects: Vec::new(),
            #[cfg(feature = "metal")]
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
        #[cfg(not(feature = "metal"))]
        self.bitmap.context(|c| unsafe {
            ffi::q_set_transform(
                c,
                m.m11,
                m.m12,
                m.m21,
                m.m22,
                m.m31,
                m.m32,
                self.scale.x,
                self.scale.y,
            )
        });
        #[cfg(feature = "metal")]
        {
            let options = self.options.last().copied().unwrap_or_default();
            let matrix = [
                m.m11,
                m.m12,
                m.m21,
                m.m22,
                m.m31,
                m.m32,
                self.scale.x,
                self.scale.y,
            ];
            self.bitmap.context(|c| unsafe {
                ffi::qm_prepare(
                    c,
                    matrix.as_ptr(),
                    blend(options.bitmap_blending_mode).unwrap_or(28),
                    (options.edge_mode == EdgeMode::Aliased) as i32,
                    options.bitmap_interpolation_mode as i32,
                )
            });
        }
        true
    }
    #[cfg(feature = "metal")]
    fn begin_layer(&self, opacity: f64, mode: i32, bounds: Option<Rect>) {
        if let Some(bounds) = bounds.filter(|r| r.width > 0. && r.height > 0.) {
            self.prepare();
            let rect = [bounds.x, bounds.y, bounds.width, bounds.height];
            self.bitmap.context(|c| unsafe {
                ffi::qm_begin_layer_bounds(c, opacity, mode, rect.as_ptr())
            });
        } else {
            self.bitmap
                .context(|c| unsafe { ffi::q_begin_layer(c, opacity, mode) });
        }
    }
    fn fill(&mut self, path: &BezPath, rule: FillRule, brush: &dyn IBrush, bounds: Rect) {
        if !self.prepare() {
            return;
        }
        let native = geometry::native(path);
        let opacity = brush.opacity();
        if let Some(solid) = brush.as_solid_color_brush() {
            let [r, g, b, a] = color(solid.color(), opacity);
            self.bitmap.context(|c| unsafe {
                ffi::q_fill(
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
        self.bitmap.context(|c| unsafe {
            ffi::q_save(c);
            ffi::q_clip(c, native.0, i32::from(rule == FillRule::EvenOdd))
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
                self.bitmap.context(|c| unsafe { ffi::q_restore(c) });
                return;
            }
            self.bitmap.context(|c| unsafe {
                ffi::q_set_transform(
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
            #[cfg(feature = "metal")]
            self.bitmap.context(|c| unsafe {
                ffi::qm_gradient_options(c, gradient.spread_method() as i32, 1., 0.)
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
                        ffi::q_gradient(
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
                    #[cfg(feature = "metal")]
                    self.bitmap.context(|c| unsafe {
                        ffi::qm_gradient_options(
                            c,
                            gradient.spread_method() as i32,
                            radius / g.radius_y().to_value(bounds.height).abs().max(1e-12),
                            0.,
                        )
                    });
                    self.bitmap.context(|c| unsafe {
                        ffi::q_gradient(
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
                    #[cfg(feature = "metal")]
                    {
                        let center = g.center().to_pixels_rect(bounds);
                        self.bitmap.context(|c| unsafe {
                            ffi::qm_gradient_options(
                                c,
                                gradient.spread_method() as i32,
                                1.,
                                g.angle(),
                            );
                            ffi::q_gradient(
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
                    #[cfg(not(feature = "metal"))]
                    {
                        let _ = g;
                        unsupported("conic gradient");
                    }
                }
            }
        } else if let Some(image) = brush.as_image_brush() {
            #[cfg(feature = "metal")]
            if let Some(bitmap) = image.source().and_then(|s| s.get_bitmap()) {
                self.tile_brush(image, &*bitmap, bounds);
            }
            #[cfg(not(feature = "metal"))]
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
            #[cfg(feature = "metal")]
            self.scene_brush(scene, bounds);
            #[cfg(not(feature = "metal"))]
            if let Some(content) = scene.create_content() {
                content.render(self, None);
                content.dispose();
            }
        } else {
            unsupported("brush")
        }
        self.bitmap.context(|c| unsafe { ffi::q_restore(c) });
    }
    fn shape(
        &mut self,
        brush: Option<&dyn IBrush>,
        pen: Option<&dyn IPen>,
        stroke: &BezPath,
        fill: Option<&BezPath>,
        rule: FillRule,
    ) {
        if brush.is_none() && pen.is_none() {
            return;
        }
        if pen.is_none() && brush.is_some_and(|b| b.as_solid_color_brush().is_some()) {
            if let (Some(brush), Some(path)) = (brush, fill) {
                self.fill(path, rule, brush, Rect::default());
            }
            return;
        }
        let b = match fill.filter(|p| !p.is_empty()) {
            Some(fill) if stroke.is_empty() => fill.bounding_box(),
            Some(fill) => stroke.bounding_box().union(fill.bounding_box()),
            None => stroke.bounding_box(),
        };
        let bounds = Rect::new(b.x0, b.y0, b.width(), b.height());
        if let (Some(brush), Some(path)) = (brush, fill) {
            self.fill(path, rule, brush, bounds)
        }
        if let Some(pen) = pen {
            if let Some(brush) = pen.brush() {
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
            ffi::q_clear(ctx, size.width as f64, size.height as f64, r, g, b, a)
        })
    }
    fn draw_bitmap(&mut self, source: &dyn IBitmapImpl, opacity: f64, src: Rect, dst: Rect) {
        #[cfg(feature = "metal")]
        let converted = source
            .as_any()
            .downcast_ref::<crate::external_objects::SharedFrameBitmap>()
            .and_then(|f| f.bitmap());
        #[cfg(feature = "metal")]
        if source
            .as_any()
            .is::<crate::external_objects::SharedFrameBitmap>()
            && converted.is_none()
        {
            return; // A retired producer frame must not be rendered.
        }
        #[cfg(feature = "metal")]
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
        let g = g
            .as_any()
            .downcast_ref::<Geometry>()
            .expect("Quartz drawing requires Quartz geometry");
        #[cfg(feature = "metal")]
        if pen.is_none() {
            if let Some(solid) = brush.and_then(|b| b.as_solid_color_brush()) {
                if let Some(path) = g.native_fill() {
                    if self.prepare() {
                        let [r, green, b, a] = color(solid.color(), brush.unwrap().opacity());
                        self.bitmap.context(|c| unsafe {
                            ffi::q_fill(
                                c,
                                path,
                                i32::from(g.rule == FillRule::EvenOdd),
                                r,
                                green,
                                b,
                                a,
                            )
                        });
                    }
                }
                return;
            }
        }
        self.shape(brush, pen, &g.stroke, g.fill.as_ref(), g.rule)
    }
    fn draw_rectangle(
        &mut self,
        brush: Option<&dyn IBrush>,
        pen: Option<&dyn IPen>,
        r: RoundedRect,
        shadows: &BoxShadows,
    ) {
        #[cfg(feature = "metal")]
        if pen.is_none() && r.is_uniform() {
            if let Some((brush, solid)) =
                brush.and_then(|b| b.as_solid_color_brush().map(|s| (b, s)))
            {
                for shadow in shadows.iter().filter(|s| !s.is_inset) {
                    self.box_shadow_gpu(r, &shadow);
                }
                if self.prepare() {
                    let rect = [
                        r.rect.x,
                        r.rect.y,
                        r.rect.width,
                        r.rect.height,
                        r.radii_top_left.x,
                        r.radii_top_left.y,
                    ];
                    let [red, green, blue, alpha] = color(solid.color(), brush.opacity());
                    self.bitmap.context(|c| unsafe {
                        ffi::qm_rectangle(c, rect.as_ptr(), red, green, blue, alpha)
                    });
                }
                for shadow in shadows.iter().filter(|s| s.is_inset) {
                    self.box_shadow_gpu(r, &shadow);
                }
                return;
            }
        }
        let path = rounded(r);
        #[cfg(feature = "metal")]
        {
            for shadow in shadows.iter().filter(|s| !s.is_inset) {
                self.box_shadow_gpu(r, &shadow);
            }
            self.shape(brush, None, &path, Some(&path), FillRule::NonZero);
            for shadow in shadows.iter().filter(|s| s.is_inset) {
                self.box_shadow_gpu(r, &shadow);
            }
            self.shape(None, pen, &path, Some(&path), FillRule::NonZero);
            return;
        }
        #[cfg(not(feature = "metal"))]
        {
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
                    ffi::q_save(c);
                    ffi::q_shadow(
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
                self.fill(&path, FillRule::NonZero, &shadow_brush, r.rect);
                self.bitmap.context(|c| unsafe { ffi::q_restore(c) });
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
        if let Some(solid) = brush.as_solid_color_brush().filter(|_| !run.simulated) {
            let [r, g, b, a] = color(solid.color(), brush.opacity());
            if !self.prepare() {
                return;
            }
            #[cfg(feature = "metal")]
            let text = self.text_options.last().copied().unwrap_or_default();
            #[cfg(feature = "metal")]
            let mut snapped = std::borrow::Cow::Borrowed(&run.positions[..]);
            #[cfg(feature = "metal")]
            if text.baseline_pixel_alignment != BaselinePixelAlignment::Unaligned
                && self.transform.m12 == 0.
                && self.transform.m21 == 0.
            {
                let baseline =
                    (run.origin.y * self.transform.m22 + self.transform.m32) * self.scale.y;
                let shift = (baseline.round() - baseline) / (self.transform.m22 * self.scale.y);
                if shift != 0. {
                    for position in snapped.to_mut().chunks_exact_mut(2) {
                        position[1] += shift;
                    }
                }
            }
            self.bitmap.context(|c| unsafe {
                #[cfg(feature = "metal")]
                {
                    let options = self.options.last().copied().unwrap_or_default();
                    let mode = if text.text_rendering_mode == TextRenderingMode::Unspecified {
                        options.text_rendering_mode
                    } else {
                        text.text_rendering_mode
                    };
                    ffi::qm_glyphs_options(
                        c,
                        run.font.raw(),
                        run.size,
                        run.ids.as_ptr(),
                        snapped.as_ptr(),
                        run.ids.len(),
                        r,
                        g,
                        b,
                        a,
                        text.text_hinting_mode as i32,
                        (mode == TextRenderingMode::Alias || options.edge_mode == EdgeMode::Aliased)
                            as i32,
                        options.bitmap_interpolation_mode as i32,
                    );
                }
                #[cfg(not(feature = "metal"))]
                ffi::q_glyphs(
                    c,
                    run.font.raw(),
                    run.size,
                    run.ids.as_ptr(),
                    run.positions.as_ptr(),
                    run.ids.len(),
                    r,
                    g,
                    b,
                    a,
                );
            })
        } else {
            #[cfg(feature = "metal")]
            {
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
                    && self.transform.m22 != 0.
                {
                    let baseline =
                        (run.origin.y * self.transform.m22 + self.transform.m32) * self.scale.y;
                    (baseline.round() - baseline) / (self.transform.m22 * self.scale.y)
                } else {
                    0.
                };
                self.options.push(RenderOptions {
                    edge_mode: if mode == TextRenderingMode::Alias
                        || options.edge_mode == EdgeMode::Aliased
                    {
                        EdgeMode::Aliased
                    } else {
                        EdgeMode::Antialias
                    },
                    ..options
                });
                let path = kurbo::Affine::translate((0., shift)) * run.outlines.clone();
                self.fill(
                    &path,
                    FillRule::NonZero,
                    brush,
                    run.bounds.translate(Vector::new(0., shift)),
                );
                self.draw_color_glyphs(brush, run, shift);
                self.options.pop();
            }
            #[cfg(not(feature = "metal"))]
            self.fill(&run.outlines, FillRule::NonZero, brush, run.bounds);
        }
    }
    fn create_layer(&mut self, size: PixelSize) -> Rc<dyn IDrawingContextLayerImpl> {
        Rc::new(Bitmap::new(size, Vector::new(96., 96.)))
    }
    fn push_clip(&mut self, r: Rect) {
        self.push_geometry_clip(&*Geometry::rectangle(r))
    }
    fn push_clip_rounded(&mut self, r: RoundedRect) {
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
        self.bitmap.context(|c| unsafe { ffi::q_restore(c) })
    }
    fn push_layer(&mut self, bounds: Rect) {
        #[cfg(feature = "metal")]
        self.begin_layer(1., 0, Some(bounds));
        #[cfg(not(feature = "metal"))]
        self.bitmap
            .context(|c| unsafe { ffi::q_begin_layer(c, 1., 0) });
    }
    fn pop_layer(&mut self) {
        self.bitmap.context(|c| unsafe { ffi::q_end_layer(c) })
    }
    fn try_push_compositing_layer(
        &mut self,
        b: BitmapBlendingMode,
        opacity: f64,
        bounds: Rect,
    ) -> bool {
        if let Some(mode) = blend(b) {
            #[cfg(feature = "metal")]
            self.begin_layer(opacity, mode, Some(bounds));
            #[cfg(not(feature = "metal"))]
            self.bitmap
                .context(|c| unsafe { ffi::q_begin_layer(c, opacity, mode) });
            true
        } else {
            false
        }
    }
    fn pop_compositing_layer(&mut self) {
        self.pop_layer()
    }
    fn push_opacity(&mut self, opacity: f64, bounds: Option<Rect>) {
        #[cfg(feature = "metal")]
        self.begin_layer(opacity, 0, bounds);
        #[cfg(not(feature = "metal"))]
        self.bitmap
            .context(|c| unsafe { ffi::q_begin_layer(c, opacity, 0) });
    }
    fn pop_opacity(&mut self) {
        self.pop_layer()
    }
    fn push_opacity_mask(&mut self, mask: &dyn IBrush, bounds: Rect) {
        self.push_layer(bounds);
        self.masks
            .push((SharedBrush::from_brush(mask), bounds, self.transform));
    }
    fn pop_opacity_mask(&mut self) {
        if let Some((brush, bounds, m)) = self.masks.pop() {
            let old = self.transform;
            self.transform = m;
            self.bitmap
                .context(|c| unsafe { ffi::q_begin_layer(c, 1., 22) });
            if let Some(brush) = brush {
                self.fill(
                    &Geometry::rectangle(bounds).stroke,
                    FillRule::NonZero,
                    &brush,
                    bounds,
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
        let g = g
            .as_any()
            .downcast_ref::<Geometry>()
            .expect("Quartz geometry clip requires Quartz geometry");
        let Some(fill) = &g.fill else {
            // No fill means no additional clip, regardless of the transform.
            self.bitmap.context(|c| unsafe { ffi::q_save(c) });
            return;
        };
        if !self.prepare() {
            let empty = geometry::native(&BezPath::new());
            self.bitmap.context(|c| unsafe {
                ffi::q_save(c);
                ffi::q_clip(c, empty.0, 0);
            });
            return;
        }
        self.bitmap.context(|c| unsafe { ffi::q_save(c) });
        let path = geometry::native(fill);
        self.bitmap
            .context(|c| unsafe { ffi::q_clip(c, path.0, i32::from(g.rule == FillRule::EvenOdd)) })
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
    #[cfg(feature = "metal")]
    fn as_drawing_context_impl_with_effects(
        &mut self,
    ) -> Option<&mut dyn IDrawingContextImplWithEffects> {
        Some(self)
    }
    #[cfg(feature = "metal")]
    fn as_drawing_context_impl_with_layer_effects(
        &mut self,
    ) -> Option<&mut dyn IDrawingContextImplWithLayerEffects> {
        Some(self)
    }
    #[cfg(feature = "metal")]
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
            self.bitmap.context(|c| unsafe { ffi::q_restore(c) });
            #[cfg(feature = "metal")]
            self.bitmap.context(|c| unsafe { ffi::qm_end_context(c) });
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

//! GPU filter layers. No effect reads canvas pixels back to the CPU.
use super::*;
use ferroui_base::media::{effects::IEffect, ColorMatrix, ColorMatrixAlphaMode, ConvolveMatrix};
use ferroui_base::rendering::composition::server::{LightSource, LIGHT_MAP_MAX_SCALE};

pub(super) enum Filter {
    None,
    #[cfg(feature = "runtime-shaders")]
    Custom {
        effect: ferroui_base::media::effects::ImmutableCustomEffect,
        area: Rect,
        scale: f32,
    },
    Blur {
        sigma: f64,
        axes: [f64; 4],
    },
    Shadow {
        sigma: f64,
        axes: [f64; 4],
        offset: [f32; 2],
        color: [f32; 4],
    },
    Kernel {
        kind: i32,
        data: Vec<f32>,
        secondary: Option<Bitmap>,
    },
}
pub(super) struct State {
    filter: Filter,
    input_clip: bool,
    output_clip: bool,
}
impl DrawingContext {
    fn effect_axes(&self) -> [f64; 4] {
        let m = self.transform;
        [
            m.m11 * self.scale.x,
            m.m12 * self.scale.y,
            m.m21 * self.scale.x,
            m.m22 * self.scale.y,
        ]
    }
    fn effect_matrix(&self) -> Matrix {
        self.transform * Matrix::create_scale(self.scale.x, self.scale.y)
    }
    fn begin_filter(&mut self, bounds: Option<Rect>, filter: Filter, clip_output: bool) {
        if clip_output {
            if let Some(bounds) = bounds {
                self.push_clip(bounds);
            }
        }
        self.push_layer(bounds.unwrap_or_default());
        if !clip_output {
            if let Some(bounds) = bounds {
                self.push_clip(bounds);
            }
        }
        self.effects.push(State {
            filter,
            input_clip: bounds.is_some() && !clip_output,
            output_clip: bounds.is_some() && clip_output,
        });
    }
    fn finish_filter(&mut self) {
        let state = self
            .effects
            .pop()
            .expect("Unbalanced Direct2D effect layer");
        if state.input_clip {
            self.pop_clip();
        }
        self.bitmap.context(|c| unsafe {
            match state.filter {
                Filter::None => {}
                #[cfg(feature = "runtime-shaders")]
                Filter::Custom {
                    effect,
                    area,
                    scale,
                } => crate::custom_shaders::apply(c, &effect, area, scale),
                Filter::Blur { sigma, axes } => {
                    ffi::d2_gpu_blur(c, sigma, axes[0], axes[1], axes[2], axes[3])
                }
                Filter::Shadow {
                    sigma,
                    axes,
                    offset,
                    color,
                } => {
                    let original = ffi::Owned::new(ffi::d2_gpu_layer_snapshot(c));
                    ffi::d2_gpu_blur(c, sigma, axes[0], axes[1], axes[2], axes[3]);
                    let data = [
                        offset[0], offset[1], 0., 0., color[0], color[1], color[2], color[3],
                    ];
                    ffi::d2_gpu_filter(c, 2, data.as_ptr(), data.len(), original.0);
                }
                Filter::Kernel {
                    kind,
                    data,
                    secondary,
                } => {
                    if (kind == 0 || kind == 1) && secondary.is_none() {
                        ffi::d2_gpu_defer_layer_filter(c, kind, data.as_ptr(), data.len());
                        return;
                    }
                    let secondary = secondary.map(|b| b.retained_surface());
                    ffi::d2_gpu_filter(
                        c,
                        kind,
                        data.as_ptr(),
                        data.len(),
                        secondary.as_ref().map_or(std::ptr::null_mut(), |s| s.0),
                    );
                }
            }
        });
        self.pop_layer();
        if state.output_clip {
            self.pop_clip();
        }
    }
    fn filter_coordinates(&self, bounds: Rect, scale: f32) -> Vec<f32> {
        let inv = self
            .effect_matrix()
            .try_invert()
            .unwrap_or(Matrix::IDENTITY);
        vec![
            inv.m11 as f32,
            inv.m12 as f32,
            inv.m21 as f32,
            inv.m22 as f32,
            inv.m31 as f32,
            inv.m32 as f32,
            scale,
            0.,
            bounds.x as f32,
            bounds.y as f32,
            bounds.width as f32,
            bounds.height as f32,
        ]
    }
}
fn sigma(radius: f64) -> f64 {
    if radius > 0. {
        0.288675 * radius + 0.5
    } else {
        0.
    }
}
impl IDrawingContextImplWithEffects for DrawingContext {
    fn push_effect(&mut self, bounds: Option<Rect>, effect: &dyn IEffect) {
        let filter = if let Some(blur) = effect.as_blur_effect() {
            Filter::Blur {
                sigma: sigma(blur.radius()),
                axes: self.effect_axes(),
            }
        } else if let Some(shadow) = effect.as_drop_shadow_effect() {
            let axes = self.effect_axes();
            let [r, g, b, a] = color(shadow.color(), shadow.opacity());
            Filter::Shadow {
                sigma: sigma(shadow.blur_radius()),
                axes,
                offset: [
                    (axes[0] * shadow.offset_x() + axes[2] * shadow.offset_y()) as f32,
                    (axes[1] * shadow.offset_x() + axes[3] * shadow.offset_y()) as f32,
                ],
                color: [r as f32, g as f32, b as f32, a as f32],
            }
        } else if effect.as_any().is::<ImmutableLuminanceToAlphaEffect>() {
            Filter::Kernel {
                kind: 1,
                data: vec![],
                secondary: None,
            }
        } else {
            if let Some(custom) = effect.as_custom_effect() {
                #[cfg(feature = "runtime-shaders")]
                if custom.has_shader() {
                    let area = bounds
                        .unwrap_or(Rect::new(
                            0.,
                            0.,
                            self.bitmap.pixel_size().width as f64 / self.scale.x,
                            self.bitmap.pixel_size().height as f64 / self.scale.y,
                        ))
                        .transform_to_aabb(self.effect_matrix());
                    self.begin_filter(
                        bounds,
                        Filter::Custom {
                            effect: custom.clone(),
                            area,
                            scale: self.effect_matrix().m11.hypot(self.effect_matrix().m12) as f32,
                        },
                        false,
                    );
                    return;
                }
                custom.report_unhandled("Direct2D");
            }
            Filter::None
        };
        self.begin_filter(bounds, filter, false);
    }
    fn pop_effect(&mut self) {
        self.finish_filter();
    }
}
impl IDrawingContextImplWithLayerEffects for DrawingContext {
    fn push_blend_layer(&mut self, _: Rect, mode: BitmapBlendingMode) {
        let mode = blend(mode).unwrap_or(28); // Destination: ignore the group's pixels.
        self.bitmap
            .context(|c| unsafe { ffi::d2_begin_layer(c, 1., mode) });
    }
    fn pop_blend_layer(&mut self) {
        self.pop_layer();
    }
    fn push_color_matrix_layer(
        &mut self,
        bounds: Rect,
        matrix: &ColorMatrix,
        alpha: ColorMatrixAlphaMode,
    ) {
        let mut data: Vec<f32> = matrix.rows.iter().flatten().copied().collect();
        data.extend([
            if alpha == ColorMatrixAlphaMode::Premultiplied {
                1.
            } else {
                0.
            },
            0.,
            0.,
            0.,
        ]);
        self.begin_filter(
            Some(bounds),
            Filter::Kernel {
                kind: 0,
                data,
                secondary: None,
            },
            true,
        );
    }
    fn pop_color_matrix_layer(&mut self) {
        self.finish_filter();
    }
    fn push_multiply_layer(&mut self, bounds: Rect, factors: &MultiplyLayerFactors) {
        let size = factors.size;
        let bitmap = Bitmap::from_pixels(
            PixelFormat::RGBA8888,
            AlphaFormat::Opaque,
            &factors.rgba,
            size,
            Vector::new(96., 96.),
            size.width * 4,
        );
        self.begin_filter(
            Some(bounds),
            Filter::Kernel {
                kind: 3,
                data: self.filter_coordinates(bounds, factors.scale),
                secondary: Some(bitmap),
            },
            true,
        );
    }
    fn pop_multiply_layer(&mut self) {
        self.finish_filter();
    }

    fn draw_lighting(&mut self, bounds: Rect, lighting: &LightingParameters) -> bool {
        if !lighting.fits_a_drawing_context() {
            return false;
        }
        let Some(region) =
            EffectRasterRegion::new(bounds, self.effect_matrix(), self.bitmap.pixel_size())
        else {
            return true;
        };
        let (x, y) = (region.origin.x as f64, region.origin.y as f64);
        let (width, height) = (region.size.width as f64, region.size.height as f64);
        let data = lighting_uniforms(lighting, region.to_local, false);
        self.push_clip(bounds);
        self.bitmap.context(|c| unsafe {
            ffi::d2_gpu_draw_lighting(c, width as u32, height as u32, x, y, data.as_ptr(), data.len());
        });
        self.pop_clip();
        true
    }
    fn push_lighting_multiply_layer(
        &mut self,
        bounds: Rect,
        lighting: &LightingParameters,
    ) -> bool {
        if !lighting.fits_a_drawing_context() {
            return false;
        }
        let Some(inv) = self.effect_matrix().try_invert() else {
            return false;
        };
        self.begin_filter(
            Some(bounds),
            Filter::Kernel {
                kind: 6,
                data: lighting_uniforms(lighting, inv, true),
                secondary: None,
            },
            true,
        );
        true
    }
    fn push_pixel_operation(&mut self, bounds: Rect, operation: &PixelOperation) -> bool {
        let Some(inv) = self.effect_matrix().try_invert() else {
            return false;
        };
        let mut data = vec![0.; 24];
        data[16..19].copy_from_slice(&[inv.m11 as f32, inv.m21 as f32, inv.m31 as f32]);
        data[20..23].copy_from_slice(&[inv.m12 as f32, inv.m22 as f32, inv.m32 as f32]);
        match *operation {
            PixelOperation::Blend {
                mode,
                noise_grid,
                noise_size,
            } => {
                if noise_grid.width <= 0.
                    || noise_grid.height <= 0.
                    || noise_size.width <= 0
                    || noise_size.height <= 0
                {
                    return false;
                }
                data[1] = mode as u32 as f32;
                data[8..12].copy_from_slice(&[
                    noise_grid.x as f32,
                    noise_grid.y as f32,
                    noise_grid.width as f32,
                    noise_grid.height as f32,
                ]);
                data[12..14].copy_from_slice(&[noise_size.width as f32, noise_size.height as f32]);
            }
            PixelOperation::Arithmetic { k } => {
                data[0] = 1.;
                data[4..8].copy_from_slice(&k);
            }
            PixelOperation::MaskInvert => data[0] = 2.,
            _ => return false,
        }
        self.begin_filter(Some(bounds), Filter::None, true);
        self.pixels.push(PixelState {
            data,
            accumulated: None,
        });
        true
    }
    fn next_pixel_operation_source(&mut self) {
        let state = self.pixels.last_mut().expect("Unbalanced pixel operation");
        self.bitmap.context(|c| unsafe {
            if let Some(previous) = state.accumulated.take() {
                ffi::d2_gpu_filter(c, 7, state.data.as_ptr(), state.data.len(), previous.0);
            }
            state.accumulated = Some(ffi::Owned::new(ffi::d2_gpu_take_layer(c)));
        });
    }
    fn pop_pixel_operation(&mut self) {
        let state = self.pixels.pop().expect("Unbalanced pixel operation");
        if let Some(previous) = state.accumulated {
            self.bitmap.context(|c| unsafe {
                ffi::d2_gpu_filter(c, 7, state.data.as_ptr(), state.data.len(), previous.0)
            });
        }
        self.finish_filter();
    }
    fn push_convolve_matrix_layer(&mut self, bounds: Rect, kernel: &ConvolveMatrix) {
        let matrix = self.effect_matrix();
        let corners = [
            Point::new(bounds.x, bounds.y),
            Point::new(bounds.right(), bounds.y),
            Point::new(bounds.right(), bounds.bottom()),
            Point::new(bounds.x, bounds.bottom()),
        ]
        .map(|p| matrix.transform(p));
        let x = corners
            .iter()
            .map(|p| p.x)
            .fold(f64::INFINITY, f64::min)
            .floor();
        let y = corners
            .iter()
            .map(|p| p.y)
            .fold(f64::INFINITY, f64::min)
            .floor();
        let right = corners
            .iter()
            .map(|p| p.x)
            .fold(f64::NEG_INFINITY, f64::max)
            .ceil();
        let bottom = corners
            .iter()
            .map(|p| p.y)
            .fold(f64::NEG_INFINITY, f64::max)
            .ceil();
        let mut data = vec![
            kernel.kernel_width as f32,
            kernel.kernel_height as f32,
            kernel.gain(),
            kernel.bias,
            if kernel.clamp_edges { 1. } else { 0. },
            if kernel.preserve_alpha { 1. } else { 0. },
            0.,
            0.,
            x as f32,
            y as f32,
            right as f32,
            bottom as f32,
        ];
        data.extend(kernel.correlation_kernel());
        self.begin_filter(
            Some(bounds),
            Filter::Kernel {
                kind: 4,
                data,
                secondary: None,
            },
            true,
        );
    }
    fn pop_convolve_matrix_layer(&mut self) {
        self.finish_filter();
    }
}

impl DrawingContext {
    pub(super) fn box_shadow_gpu(&mut self, rect: RoundedRect, shadow: &BoxShadow) {
        if shadow.color.a == 0 || !self.prepare() {
            return;
        }
        let spread = if shadow.is_inset {
            -shadow.spread
        } else {
            shadow.spread
        };
        let caster = rect.inflate(spread, spread);
        let matrix = self.effect_matrix();
        let axes = self.effect_axes();
        let offset = Point::new(
            axes[0] * shadow.offset_x + axes[2] * shadow.offset_y,
            axes[1] * shadow.offset_x + axes[3] * shadow.offset_y,
        );
        let mapped = |r: Rect| {
            let points = [
                Point::new(r.x, r.y),
                Point::new(r.right(), r.y),
                Point::new(r.right(), r.bottom()),
                Point::new(r.x, r.bottom()),
            ]
            .map(|p| matrix.transform(p));
            let x = points.iter().map(|p| p.x).fold(f64::INFINITY, f64::min);
            let y = points.iter().map(|p| p.y).fold(f64::INFINITY, f64::min);
            let right = points.iter().map(|p| p.x).fold(f64::NEG_INFINITY, f64::max);
            let bottom = points.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max);
            Rect::new(x, y, right - x, bottom - y)
        };
        let a = mapped(rect.rect);
        let b = mapped(caster.rect);
        let sigma = sigma(shadow.blur);
        let padding = 4. * sigma * axes[0].hypot(axes[1]).max(axes[2].hypot(axes[3])) + 2.;
        let x = (a.x.min(b.x + offset.x) - padding).floor().max(0.);
        let y = (a.y.min(b.y + offset.y) - padding).floor().max(0.);
        let size = self.bitmap.pixel_size();
        let right = (a.right().max(b.right() + offset.x) + padding)
            .ceil()
            .min(size.width as f64);
        let bottom = (a.bottom().max(b.bottom() + offset.y) + padding)
            .ceil()
            .min(size.height as f64);
        if right <= x || bottom <= y {
            return;
        }
        let size = PixelSize::new((right - x) as i32, (bottom - y) as i32);
        let transform = matrix * Matrix::create_translation(-x, -y);
        // Reuse the bitmap across integer translations only. The local
        // raster transform includes the fractional pixel phase and any crop
        // at the target edges. Direct2D takes the transform as floats.
        let r = rect.rect;
        let tl = rect.radii_top_left;
        let tr = rect.radii_top_right;
        let br = rect.radii_bottom_right;
        let bl = rect.radii_bottom_left;
        let c = shadow.color;
        let key = [
            r.x.to_bits(),
            r.y.to_bits(),
            r.width.to_bits(),
            r.height.to_bits(),
            tl.x.to_bits(),
            tl.y.to_bits(),
            tr.x.to_bits(),
            tr.y.to_bits(),
            br.x.to_bits(),
            br.y.to_bits(),
            bl.x.to_bits(),
            bl.y.to_bits(),
            u64::from((transform.m11 as f32).to_bits()),
            u64::from((transform.m12 as f32).to_bits()),
            u64::from((transform.m21 as f32).to_bits()),
            u64::from((transform.m22 as f32).to_bits()),
            u64::from((transform.m31 as f32).to_bits()),
            u64::from((transform.m32 as f32).to_bits()),
            spread.to_bits(),
            sigma.to_bits(),
            u64::from((offset.x as f32).to_bits()),
            u64::from((offset.y as f32).to_bits()),
            u64::from(u32::from_le_bytes([c.r, c.g, c.b, c.a])) | (u64::from(shadow.is_inset) << 32),
            u64::from(size.width as u32) | (u64::from(size.height as u32) << 32),
        ];
        let (x, y) = (x as f32, y as f32);
        if self
            .bitmap
            .context(|c| unsafe { ffi::d2_gpu_try_draw_box_shadow(c, &key, x, y) })
            != 0
        {
            return;
        }
        let mask = Bitmap::new(size, Vector::new(96., 96.));
        let cast = Bitmap::new(size, Vector::new(96., 96.));
        for (target, shape) in [(&mask, rect), (&cast, caster)] {
            if shape.rect.width <= 0. || shape.rect.height <= 0. {
                continue;
            }
            let mut draw = target.drawing(false, None);
            draw.set_transform(transform);
            draw.draw_rectangle(
                Some(&immutable::ImmutableSolidColorBrush::new(Colors::WHITE)),
                None,
                shape,
                &Default::default(),
            );
            draw.dispose();
        }
        let mask = mask.retained_surface();
        let cast = cast.retained_surface();
        let [r, g, b, a] = color(shadow.color, 1.);
        let data = [
            offset.x as f32,
            offset.y as f32,
            if shadow.is_inset { 1. } else { 0. },
            0.,
            r as f32,
            g as f32,
            b as f32,
            a as f32,
        ];
        self.bitmap.context(|c| unsafe {
            ffi::d2_gpu_draw_box_shadow(c, &key, x, y, cast.0, mask.0, sigma, &axes, &data);
        });
    }
}

pub(super) struct PixelState {
    data: Vec<f32>,
    accumulated: Option<ffi::Owned>,
}
// Packing follows the engine-independent lighting contracts and the existing Vello pass.
fn lighting_uniforms(lighting: &LightingParameters, to_local: Matrix, multiply: bool) -> Vec<f32> {
    const HEAD_FLOATS: usize = 24;
    const LIGHT_FLOATS: usize = 40;
    const MAX_LIGHTS: usize = LightingParameters::MAX_LIGHTS;
    let mut floats = vec![0f32; HEAD_FLOATS + LIGHT_FLOATS * MAX_LIGHTS];
    let constant = lighting.constant;
    let material = lighting.material;
    let (mode, scale) = (if multiply { 1. } else { 0. }, LIGHT_MAP_MAX_SCALE as f32);
    let [a, b, c, d, e, f] = [
        to_local.m11,
        to_local.m12,
        to_local.m21,
        to_local.m22,
        to_local.m31,
        to_local.m32,
    ];
    floats[0..4].copy_from_slice(&[constant.r as f32, constant.g as f32, constant.b as f32, 0.0]);
    floats[4..8].copy_from_slice(&[
        material.ambient as f32,
        material.diffuse as f32,
        material.specular as f32,
        material.shine as f32,
    ]);
    floats[8..12].copy_from_slice(&[
        mode,
        scale,
        lighting.lights.len().min(MAX_LIGHTS) as f32,
        0.0,
    ]);
    floats[12..16].copy_from_slice(&[0.0, 0.0, 0.0, 0.0]);
    // x' = a x + c y + e, y' = b x + d y + f
    floats[16..20].copy_from_slice(&[a as f32, c as f32, e as f32, 0.0]);
    floats[20..24].copy_from_slice(&[b as f32, d as f32, f as f32, 0.0]);
    let max_cutoff = |value: f64| -> [f32; 2] {
        if value.is_nan() || value >= f64::from(f32::MAX) {
            [0.0, 0.0]
        } else if value <= -f64::from(f32::MAX) {
            [-f32::MAX, 1.0]
        } else {
            [value as f32, 1.0]
        }
    };
    for (index, light) in lighting.lights.iter().take(MAX_LIGHTS).enumerate() {
        let at = HEAD_FLOATS + index * LIGHT_FLOATS;
        let slot = |field: usize| at + field * 4;
        let m = light.to_light_space;
        floats[slot(7)..slot(7) + 3].copy_from_slice(&[m.m11 as f32, m.m21 as f32, m.m31 as f32]);
        floats[slot(8)..slot(8) + 3].copy_from_slice(&[m.m12 as f32, m.m22 as f32, m.m32 as f32]);
        floats[slot(9)..slot(9) + 3].copy_from_slice(&[m.m13 as f32, m.m23 as f32, m.m33 as f32]);
        let (attenuation, position) = match light.source {
            LightSource::Point {
                color,
                position,
                attenuation,
            } => {
                floats[slot(0)] = 1.0;
                floats[slot(1)..slot(1) + 3].copy_from_slice(&[
                    color.r as f32,
                    color.g as f32,
                    color.b as f32,
                ]);
                (attenuation, position)
            }
            LightSource::Spot {
                inner_color,
                outer_color,
                position,
                direction,
                inner_cone_angle,
                outer_cone_angle,
                attenuation,
            } => {
                let outer = outer_cone_angle.clamp(0.0, std::f64::consts::PI);
                let inner = inner_cone_angle.clamp(0.0, outer);
                floats[slot(0)..slot(0) + 3].copy_from_slice(&[
                    2.0,
                    inner.cos() as f32,
                    outer.cos() as f32,
                ]);
                floats[slot(1)..slot(1) + 3].copy_from_slice(&[
                    inner_color.r as f32,
                    inner_color.g as f32,
                    inner_color.b as f32,
                ]);
                floats[slot(2)..slot(2) + 3].copy_from_slice(&[
                    outer_color.r as f32,
                    outer_color.g as f32,
                    outer_color.b as f32,
                ]);
                floats[slot(4)..slot(4) + 3].copy_from_slice(&[
                    direction.x as f32,
                    direction.y as f32,
                    direction.z as f32,
                ]);
                (attenuation, position)
            }
            // The parameters list only point and spot lights.
            LightSource::Ambient { .. } | LightSource::Distant { .. } => continue,
        };
        floats[slot(3)..slot(3) + 3].copy_from_slice(&[
            position.x as f32,
            position.y as f32,
            position.z as f32,
        ]);
        let min_cutoff = attenuation
            .min_cutoff
            .clamp(-f64::from(f32::MAX), f64::from(f32::MAX));
        floats[slot(5)..slot(5) + 4].copy_from_slice(&[
            attenuation.constant as f32,
            attenuation.linear as f32,
            attenuation.quadratic as f32,
            min_cutoff as f32,
        ]);
        floats[slot(6)..slot(6) + 2].copy_from_slice(&max_cutoff(attenuation.max_cutoff));
    }
    floats
}

impl IDrawingContextWithAcrylicLikeSupport for DrawingContext {
    fn draw_rectangle_with_material(
        &mut self,
        material: &dyn IExperimentalAcrylicMaterial,
        rect: RoundedRect,
    ) {
        if rect.rect.width <= 0. || rect.rect.height <= 0. || !self.prepare() {
            return;
        }
        static NOISE: std::sync::OnceLock<Bitmap> = std::sync::OnceLock::new();
        // Shared theme asset, not a dependency on the Skia renderer. WIC decodes it once.
        let noise = NOISE.get_or_init(|| {
            Bitmap::decode(&mut std::io::Cursor::new(include_bytes!(
                "../../Skia/FerroUI.Skia/assets/noise_asset_256x256_png.png"
            )))
            .expect("Built-in acrylic noise PNG")
        });
        self.push_clip_rounded(rect);
        let base = color(material.material_color(), 1.);
        let tint = color(material.tint_color(), 1.);
        let data: Vec<f64> = base.into_iter().chain(tint).collect();
        let source = noise.retained_surface();
        self.bitmap.context(|c| unsafe {
            ffi::d2_gpu_acrylic(
                c,
                data.as_ptr(),
                source.0,
                (material.background_source() == AcrylicBackgroundSource::Digger) as i32,
            )
        });
        self.pop_clip();
    }
}

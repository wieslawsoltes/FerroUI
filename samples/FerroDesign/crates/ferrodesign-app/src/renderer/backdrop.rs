//! A bounded, cached background filter built only from public FerroUI media
//! operations. It samples the already painted prefix of the current isolation
//! group. Its output replaces that prefix; it is never painted SourceOver on
//! top of a second copy of the same translucent pixels.

use super::*;
use ferroui_base::media::imaging::RenderTargetBitmap;
use ferroui_base::media::PlatformDrawingContext;
use ferroui_base::{PixelSize, Vector};

const MAX_DIMENSION: usize = 2048;
const MAX_PIXELS: usize = 4 * 1024 * 1024;
const CACHE_BYTES: usize = 32 * 1024 * 1024;
const WORKING_BYTES: usize = 128 * 1024 * 1024;
const MAX_BACKDROP_DEPTH: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct Key {
    index: usize,
    limit: usize,
    mask: bool,
    scope: u64,
    scale: u64,
}

#[derive(Clone, Copy)]
struct Raster {
    sample: Rect,
    output: Rect,
    scale: f64,
    size: PixelSize,
    bytes: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Failure {
    Unsupported,
    InvalidTransform,
    TooLarge,
}

struct Entry {
    image: Rc<RenderTargetBitmap>,
    raster: Raster,
    coverage: Option<MaskCoverage>,
    children: RenderStats,
    used: u64,
}

/// Keep the mask surface already produced by rendering until the first pick.
/// Only then read alpha once and release its full-color backing allocation.
/// Both representations participate in the same bounded cache as the output.
struct MaskCoverage {
    image: Option<Rc<RenderTargetBitmap>>,
    alpha: Vec<u8>,
}

impl Entry {
    fn bytes(&self) -> usize {
        self.raster.bytes
            + self.coverage.as_ref().map_or(0, |coverage| {
                if coverage.image.is_some() {
                    self.raster.bytes
                } else {
                    coverage.alpha.len()
                }
            })
    }
}

impl MaskCoverage {
    fn sample(&mut self, raster: Raster, point: Point) -> u8 {
        if !point.x.is_finite() || !point.y.is_finite() || !raster.output.contains(point) {
            return 0;
        }
        let x = ((point.x - raster.sample.x) * raster.scale).floor();
        let y = ((point.y - raster.sample.y) * raster.scale).floor();
        if x < 0.0 || y < 0.0 || x >= raster.size.width as f64 || y >= raster.size.height as f64 {
            return 0;
        }
        if let Some(image) = self.image.take() {
            use ferroui_base::platform::PixelFormat;
            if !matches!(
                image.format(),
                Some(PixelFormat::RGBA8888 | PixelFormat::BGRA8888)
            ) {
                return 0;
            }
            let mut rgba = vec![0; raster.bytes];
            image.copy_pixels(
                ferroui_base::PixelRect::new(0, 0, raster.size.width, raster.size.height),
                &mut rgba,
                raster.size.width * 4,
            );
            self.alpha = rgba.chunks_exact(4).map(|pixel| pixel[3]).collect();
        }
        self.alpha
            .get(y as usize * raster.size.width as usize + x as usize)
            .copied()
            .unwrap_or(0)
    }
}

pub(super) struct BackdropCache {
    entries: HashMap<Key, Entry>,
    retained: usize,
    working: usize,
    depth: usize,
    clock: u64,
    scale: f64,
    enabled: bool,
    hits: usize,
    rasterizations: usize,
    active_masks: Vec<usize>,
}

impl Default for BackdropCache {
    fn default() -> Self {
        Self {
            entries: HashMap::new(),
            retained: 0,
            working: 0,
            depth: 0,
            clock: 0,
            scale: 1.0,
            enabled: false,
            hits: 0,
            rasterizations: 0,
            active_masks: Vec::new(),
        }
    }
}

impl BackdropCache {
    pub(super) fn clear(&mut self) {
        self.entries.clear();
        self.retained = 0;
        self.enabled = false;
        self.active_masks.clear();
    }

    pub(super) fn set_raster_scale(&mut self, scale: f64) {
        if scale.is_finite() && scale > 0.0 {
            self.scale = scale;
        }
    }

    pub(super) fn begin_frame(&mut self) {
        self.hits = 0;
        self.rasterizations = 0;
    }

    pub(super) fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub(super) fn hits(&self) -> usize {
        self.hits
    }
    pub(super) fn rasterizations(&self) -> usize {
        self.rasterizations
    }

    pub(super) fn mask_is_open(&self, index: usize) -> bool {
        self.active_masks.contains(&index)
    }
    pub(super) fn push_mask_scope(&mut self, index: usize) {
        self.active_masks.push(index);
    }
    pub(super) fn pop_mask_scope(&mut self, index: usize) {
        let popped = self.active_masks.pop();
        debug_assert_eq!(popped, Some(index));
    }

    /// Called in node-local coordinates before the node's own opacity/blend
    /// scope. Success consumes the complete node, including its descendants.
    pub(super) fn try_paint(
        &mut self,
        context: &mut DrawingContext<'_>,
        scene: &[PreparedNode],
        roots: &[usize],
        index: usize,
        scene_reused: bool,
        stats: &mut RenderStats,
    ) -> bool {
        self.try_paint_operation(
            context,
            scene,
            roots,
            index,
            scene[index].end,
            false,
            scene_reused,
            stats,
        )
    }

    pub(super) fn try_paint_mask(
        &mut self,
        context: &mut DrawingContext<'_>,
        scene: &[PreparedNode],
        roots: &[usize],
        index: usize,
        limit: usize,
        scene_reused: bool,
        stats: &mut RenderStats,
    ) -> bool {
        self.try_paint_operation(
            context,
            scene,
            roots,
            index,
            limit,
            true,
            scene_reused,
            stats,
        )
    }

    fn try_paint_operation(
        &mut self,
        context: &mut DrawingContext<'_>,
        scene: &[PreparedNode],
        roots: &[usize],
        index: usize,
        limit: usize,
        mask: bool,
        scene_reused: bool,
        stats: &mut RenderStats,
    ) -> bool {
        let node = &scene[index];
        let count = node.visual.background_blurs.len().max(1);
        if !self.enabled {
            record_failure(stats, Failure::Unsupported, count);
            return false;
        }
        let (key, cached) =
            match self.ensure_operation(scene, roots, index, limit, mask, scene_reused) {
                Ok(result) => result,
                Err(error) => {
                    record_failure(stats, error, count);
                    return false;
                }
            };
        let entry = self.entries.get_mut(&key).unwrap();
        let clip = context.push_clip(ui_rect(entry.raster.output));
        replace_surface(context, entry.image.as_ref(), entry.raster.sample);
        context.pop(clip);
        if cached {
            self.hits += 1;
        }
        merge_children(stats, entry.children, cached);
        true
    }

    /// Picking uses the same authored mask source and resolution as rendering.
    /// A miss can prepare the bounded operation before the first repaint; later
    /// pointer events sample retained alpha without drawing or GPU readback.
    pub(super) fn mask_alpha(
        &mut self,
        scene: &[PreparedNode],
        roots: &[usize],
        index: usize,
        limit: usize,
        local: Point,
    ) -> u8 {
        let enabled = self.enabled;
        self.enabled = ferroui_base::platform::render_interface().supports_compositing_layers();
        let result = if self.enabled {
            self.ensure_operation(scene, roots, index, limit, true, true)
        } else {
            Err(Failure::Unsupported)
        };
        self.enabled = enabled;
        let Ok((key, _)) = result else { return 0 };
        let entry = self.entries.get_mut(&key).unwrap();
        let before = entry.bytes();
        let alpha = entry
            .coverage
            .as_mut()
            .map_or(0, |coverage| coverage.sample(entry.raster, local));
        self.retained = self.retained.saturating_sub(before - entry.bytes());
        alpha
    }

    fn ensure_operation(
        &mut self,
        scene: &[PreparedNode],
        roots: &[usize],
        index: usize,
        limit: usize,
        mask: bool,
        scene_reused: bool,
    ) -> Result<(Key, bool), Failure> {
        let node = &scene[index];
        self.clock = self.clock.wrapping_add(1);
        let mut scope = DefaultHasher::new();
        self.active_masks.hash(&mut scope);
        let key = Key {
            index,
            limit,
            mask,
            scope: scope.finish(),
            scale: self.scale.to_bits(),
        };
        let cached = self.entries.contains_key(&key);
        if !cached {
            let raster = raster_for(node, self.scale)?;
            // Account conservatively for the prefix, source, blur, replacement,
            // weighted output and backend filter intermediates. Nested filters
            // share this budget instead of multiplying the maximum allocation.
            let working = raster.bytes.saturating_mul(8);
            if self.depth >= MAX_BACKDROP_DEPTH
                || self.working.saturating_add(working) > WORKING_BYTES
            {
                return Err(Failure::TooLarge);
            }
            self.working += working;
            self.depth += 1;
            let result = if mask {
                self.build_mask(scene, roots, index, limit, raster, scene_reused)
            } else {
                self.build(scene, roots, index, raster, scene_reused)
            };
            self.depth -= 1;
            self.working -= working;
            let mut entry = result?;
            self.rasterizations += 1;
            entry.used = self.clock;
            while self.retained.saturating_add(entry.bytes()) > CACHE_BYTES
                && !self.entries.is_empty()
            {
                let oldest = self
                    .entries
                    .iter()
                    .min_by_key(|(_, value)| value.used)
                    .map(|(key, _)| *key)
                    .unwrap();
                if let Some(removed) = self.entries.remove(&oldest) {
                    self.retained -= removed.bytes();
                }
            }
            self.retained += entry.bytes();
            self.entries.insert(key, entry);
        }
        let entry = self.entries.get_mut(&key).unwrap();
        entry.used = self.clock;
        Ok((key, cached))
    }

    fn build(
        &mut self,
        scene: &[PreparedNode],
        roots: &[usize],
        index: usize,
        raster: Raster,
        scene_reused: bool,
    ) -> Result<Entry, Failure> {
        let node = &scene[index];
        let inverse = node.world.inverse().ok_or(Failure::InvalidTransform)?;
        let viewport = node.world.transform_rect(raster.sample);
        let prefix = surface(raster, |context| {
            let world_to_local = context.push_transform(ui_matrix(inverse));
            self.paint_prefix(context, scene, roots, index, viewport, scene_reused);
            context.pop(world_to_local);
            Ok(())
        })?;

        // Source C has neither the node's opacity nor its blend. Its own layer
        // blurs are applied here. Active ancestor effects remain outside the
        // replacement, where the live drawing context applies them once.
        let mut children = RenderStats::default();
        let source = if node.blend == BlendMode::PassThrough {
            None
        } else {
            Some(surface(raster, |context| {
                paint_blurs(
                    context,
                    scene,
                    index,
                    viewport,
                    scene_reused,
                    &mut children,
                    0,
                    self,
                    roots,
                );
                Ok(())
            })?)
        };

        let mut blurred = prefix.clone();
        for &radius in &node.visual.background_blurs {
            let input = blurred;
            blurred = surface(raster, |context| {
                let effect: Rc<dyn IEffect> = Rc::new(ImmutableBlurEffect::new(radius));
                let blur = context.push_effect(&effect, ui_rect(raster.sample));
                draw_surface(context, input.as_ref(), raster.sample);
                context.pop(blur);
                Ok(())
            })?;
        }

        let replacement = surface(raster, |context| {
            // Retain B outside the exact blur geometry, including rounded
            // corners and transparent holes. Only the filtered region changes.
            draw_surface(context, prefix.as_ref(), raster.sample);
            let clip = context.push_geometry_clip(&node.visual.geometry);
            replace_surface(context, blurred.as_ref(), raster.sample);
            context.pop(clip);
            if let Some(source) = &source {
                composite_image(
                    context,
                    source.as_ref(),
                    raster.sample,
                    bitmap_blend(node.blend),
                    1.0,
                )
            } else {
                // Pass-through children blend into the captured backdrop.
                // The completed result is interpolated once below; per-child
                // opacity would darken overlaps and change blend interactions.
                paint_blurs(
                    context,
                    scene,
                    index,
                    viewport,
                    scene_reused,
                    &mut children,
                    0,
                    self,
                    roots,
                );
                Ok(())
            }
        })?;
        let image = if node.opacity >= 1.0 {
            replacement
        } else {
            surface(raster, |context| {
                // Premultiplied interpolation F=(1-a)B+aR. SourceOver would
                // increase alpha when B/R contain translucent pixels.
                composite_image(
                    context,
                    prefix.as_ref(),
                    raster.sample,
                    BitmapBlendingMode::SourceOver,
                    1.0 - node.opacity,
                )?;
                composite_image(
                    context,
                    replacement.as_ref(),
                    raster.sample,
                    BitmapBlendingMode::Plus,
                    node.opacity,
                )
            })?
        };
        Ok(Entry {
            image,
            raster,
            coverage: None,
            children,
            used: 0,
        })
    }

    fn paint_prefix(
        &mut self,
        context: &mut DrawingContext<'_>,
        scene: &[PreparedNode],
        roots: &[usize],
        stop: usize,
        viewport: Rect,
        scene_reused: bool,
    ) {
        let mut ignored = RenderStats::default();
        let mut isolation = scene[stop].parent;
        while let Some(parent) = isolation {
            if scene[parent].isolates() {
                break;
            }
            isolation = scene[parent].parent;
        }
        if let Some(parent) = isolation {
            // Replay from the nearest actual isolation. Pass-through ancestors
            // remain open so child blend modes can see the external backdrop.
            let ancestor = &scene[parent];
            let transform = context.push_transform(ui_matrix(ancestor.world));
            paint_shadows(context, &ancestor.visual, false);
            paint_body(context, &ancestor.visual);
            let clip = push_content_clip(context, ancestor);
            masks::paint_siblings(
                context,
                scene,
                parent + 1,
                stop,
                viewport,
                scene_reused,
                &mut ignored,
                self,
                roots,
            );
            context.pop(clip);
            context.pop(transform);
        } else {
            masks::paint_siblings(
                context,
                scene,
                0,
                stop,
                viewport,
                scene_reused,
                &mut ignored,
                self,
                roots,
            );
        }
    }

    fn build_mask(
        &mut self,
        scene: &[PreparedNode],
        roots: &[usize],
        index: usize,
        limit: usize,
        raster: Raster,
        scene_reused: bool,
    ) -> Result<Entry, Failure> {
        let node = &scene[index];
        let inverse = node.world.inverse().ok_or(Failure::InvalidTransform)?;
        let parent = node.local.inverse().ok_or(Failure::InvalidTransform)?;
        let viewport = node.world.transform_rect(raster.sample);
        let prefix = surface(raster, |context| {
            let transform = context.push_transform(ui_matrix(inverse));
            self.paint_prefix(context, scene, roots, index, viewport, scene_reused);
            context.pop(transform);
            Ok(())
        })?;
        let mut blurred = prefix.clone();
        for &radius in &node.visual.background_blurs {
            let input = blurred;
            blurred = surface(raster, |context| {
                let effect: Rc<dyn IEffect> = Rc::new(ImmutableBlurEffect::new(radius));
                let blur = context.push_effect(&effect, ui_rect(raster.sample));
                draw_surface(context, input.as_ref(), raster.sample);
                context.pop(blur);
                Ok(())
            })?;
        }
        let mut children = RenderStats::default();
        let mask = surface(raster, |context| {
            let effect: Rc<dyn IEffect> =
                Rc::new(ferroui_base::media::ImmutableLuminanceToAlphaEffect);
            let luma = if node.mask == Some(MaskType::Luminance) {
                context.push_effect(&effect, ui_rect(raster.sample))
            } else {
                PushedState::NONE
            };
            let opacity = context
                .try_push_compositing_layer(
                    BitmapBlendingMode::SourceOver,
                    node.opacity,
                    ui_rect(raster.sample),
                )
                .ok_or(Failure::Unsupported)?;
            if !node.visual.background_blurs.is_empty() {
                let clip = context.push_geometry_clip(&node.visual.geometry);
                draw_surface(context, blurred.as_ref(), raster.sample);
                context.pop(clip);
            }
            paint_blurs(
                context,
                scene,
                index,
                viewport,
                scene_reused,
                &mut children,
                0,
                self,
                roots,
            );
            context.pop(opacity);
            context.pop(luma);
            Ok(())
        })?;
        self.push_mask_scope(index);
        let replacement = surface(raster, |context| {
            draw_surface(context, prefix.as_ref(), raster.sample);
            let transform = context.push_transform(ui_matrix(parent));
            masks::paint_siblings(
                context,
                scene,
                node.end,
                limit,
                viewport,
                scene_reused,
                &mut children,
                self,
                roots,
            );
            context.pop(transform);
            Ok(())
        });
        self.pop_mask_scope(index);
        let replacement = replacement?;
        let source = Rc::new(Bitmap::from_ref(
            ferroui_base::media::imaging::IBitmap::platform_impl(mask.as_ref()),
        ));
        let brush: Rc<dyn IBrush> = Rc::new(ImmutableImageBrush::new(
            Some(source),
            AlignmentX::Left,
            AlignmentY::Top,
            Some(ferroui_base::RelativeRect::from_rect(
                ui_rect(raster.sample),
                RelativeUnit::Absolute,
            )),
            1.0,
            None,
            RelativePoint::default(),
            None,
            Stretch::Fill,
            TileMode::None,
            None,
        ));
        let image = surface(raster, |context| {
            // F=(1-M)B+MR in premultiplied space. This retains translucent
            // backdrop pixels exactly once and lets masked child blends see B.
            draw_surface(context, prefix.as_ref(), raster.sample);
            composite_image(
                context,
                mask.as_ref(),
                raster.sample,
                BitmapBlendingMode::DestinationOut,
                1.0,
            )?;
            let plus = context
                .try_push_compositing_layer(BitmapBlendingMode::Plus, 1.0, ui_rect(raster.sample))
                .ok_or(Failure::Unsupported)?;
            let mask_scope = context.push_opacity_mask(&brush, ui_rect(raster.sample));
            draw_surface(context, replacement.as_ref(), raster.sample);
            context.pop(mask_scope);
            context.pop(plus);
            Ok(())
        })?;
        Ok(Entry {
            image,
            raster,
            coverage: Some(MaskCoverage {
                image: Some(mask),
                alpha: Vec::new(),
            }),
            children,
            used: 0,
        })
    }
}

fn record_failure(stats: &mut RenderStats, failure: Failure, effects: usize) {
    stats.unsupported_effects += effects;
    stats.backdrop_skipped += 1;
    stats.backdrop_too_large += usize::from(failure == Failure::TooLarge);
}

fn merge_children(stats: &mut RenderStats, children: RenderStats, cached: bool) {
    stats.visible_nodes += children.visible_nodes;
    stats.culled_nodes += children.culled_nodes;
    stats.cached_nodes += if cached {
        children.visible_nodes
    } else {
        children.cached_nodes
    };
    stats.missing_images += children.missing_images;
    stats.unsupported_effects += children.unsupported_effects;
    stats.invalid_nodes += children.invalid_nodes;
    stats.backdrop_skipped += children.backdrop_skipped;
    stats.backdrop_too_large += children.backdrop_too_large;
}

fn raster_for(node: &PreparedNode, caller_scale: f64) -> Result<Raster, Failure> {
    let scale = caller_scale * maximum_scale(node.world).ok_or(Failure::InvalidTransform)?;
    if !scale.is_finite()
        || scale <= 0.0
        || !(scale * 96.0).is_finite()
        || node.world.inverse().is_none()
    {
        return Err(Failure::InvalidTransform);
    }
    let output = node.local_subtree_bounds;
    if !output.is_finite() || output.width <= 0.0 || output.height <= 0.0 {
        return Err(Failure::InvalidTransform);
    }
    let padding = node.visual.background_blurs.iter().sum::<f64>() * 3.0 + 2.0 / scale;
    let sample = output.inflate(padding);
    let left = (sample.x * scale).floor();
    let top = (sample.y * scale).floor();
    let right = ((sample.x + sample.width) * scale).ceil();
    let bottom = ((sample.y + sample.height) * scale).ceil();
    let width = right - left;
    let height = bottom - top;
    if ![left, top, width, height]
        .iter()
        .all(|value| value.is_finite())
        || width <= 0.0
        || height <= 0.0
    {
        return Err(Failure::InvalidTransform);
    }
    if width > MAX_DIMENSION as f64
        || height > MAX_DIMENSION as f64
        || width * height > MAX_PIXELS as f64
    {
        return Err(Failure::TooLarge);
    }
    let size = PixelSize::new(width as i32, height as i32);
    Ok(Raster {
        sample: Rect::new(left / scale, top / scale, width / scale, height / scale),
        output,
        scale,
        size,
        bytes: width as usize * height as usize * 4,
    })
}

/// Largest singular value of the world transform's linear part; normalize
/// first so extreme but finite document transforms cannot overflow a square.
fn maximum_scale(transform: Transform) -> Option<f64> {
    let values = [transform.a, transform.b, transform.c, transform.d];
    if !values.iter().all(|value| value.is_finite()) {
        return None;
    }
    let maximum = values.iter().fold(0.0_f64, |a, b| a.max(b.abs()));
    if maximum <= 0.0 {
        return None;
    }
    let [a, b, c, d] = values.map(|value| value / maximum);
    let sum = a * a + b * b + c * c + d * d;
    let determinant = a * d - b * c;
    let result = maximum
        * ((sum
            + (sum * sum - 4.0 * determinant * determinant)
                .max(0.0)
                .sqrt())
            * 0.5)
            .sqrt();
    result.is_finite().then_some(result)
}

fn surface(
    raster: Raster,
    draw: impl FnOnce(&mut DrawingContext<'_>) -> Result<(), Failure>,
) -> Result<Rc<RenderTargetBitmap>, Failure> {
    let target = Rc::new(RenderTargetBitmap::with_dpi(
        raster.size,
        Vector::new(96.0 * raster.scale, 96.0 * raster.scale),
    ));
    let mut implementation = target.create_platform_drawing_context(true);
    if implementation
        .as_drawing_context_impl_with_effects()
        .is_none()
    {
        implementation.dispose();
        return Err(Failure::Unsupported);
    }
    let mut context = DrawingContext::owned(Box::new(PlatformDrawingContext::new(implementation)));
    let Some(probe) = context.try_push_compositing_layer(
        BitmapBlendingMode::SourceOver,
        1.0,
        ui_rect(raster.sample),
    ) else {
        return Err(Failure::Unsupported);
    };
    context.pop(probe);
    let _origin = context.push_transform(Matrix::create_translation(
        -raster.sample.x,
        -raster.sample.y,
    ));
    let result = draw(&mut context);
    // Dispose balances all scopes even when a nested capability check declined.
    context.dispose();
    result.map(|_| target)
}

fn composite_image(
    context: &mut DrawingContext<'_>,
    image: &RenderTargetBitmap,
    bounds: Rect,
    blend: BitmapBlendingMode,
    opacity: f64,
) -> Result<(), Failure> {
    let layer = context
        .try_push_compositing_layer(blend, opacity, ui_rect(bounds))
        .ok_or(Failure::Unsupported)?;
    draw_surface(context, image, bounds);
    context.pop(layer);
    Ok(())
}

fn replace_surface(context: &mut DrawingContext<'_>, image: &RenderTargetBitmap, bounds: Rect) {
    // Source on an isolated group also replaces its transparent surroundings;
    // a clip on its contents does not bound that final group composition. This
    // operation replaces one bitmap instead, so use the public bitmap blend:
    // its image rectangle and the active clips supply destructive coverage.
    // Keep Source rather than SourceOver so transparent/premultiplied pixels
    // replace the previous prefix without increasing its alpha.
    let options = context.push_render_options(RenderOptions {
        bitmap_blending_mode: BitmapBlendingMode::Source,
        ..RenderOptions::default()
    });
    draw_surface(context, image, bounds);
    context.pop(options);
}

fn draw_surface(context: &mut DrawingContext<'_>, image: &RenderTargetBitmap, bounds: Rect) {
    // The platform bitmap source rectangle addresses pixels. A high-DPI
    // target's logical size is smaller, so state the full pixel rectangle
    // explicitly instead of cropping it through IImage::size().
    let size = image.pixel_size();
    context.draw_image_with_rects(
        image,
        UiRect::new(0.0, 0.0, size.width as f64, size.height as f64),
        ui_rect(bounds),
    );
}

#[cfg(all(test, not(target_os = "emscripten")))]
mod tests {
    use super::*;
    use ferrodesign_core::Fill;
    use ferroui_base::platform::{IPlatformRenderInterface, PixelFormat};
    use ferroui_base::{FerroLocator, PixelRect};

    fn backend() -> Rc<dyn ferroui_base::reactive::IDisposable> {
        let scope = FerroLocator::enter_scope();
        FerroLocator::current_mutable()
            .bind::<dyn IPlatformRenderInterface>()
            .to_constant(Rc::new(ferroui_vello::PlatformRenderInterface::default()));
        scope
    }

    fn blur(radius: f64) -> Effect {
        Effect {
            kind: EffectKind::BackgroundBlur,
            radius,
            color: Color::TRANSPARENT,
            offset: Point::new(0.0, 0.0),
            spread: 0.0,
            visible: true,
        }
    }

    fn glass(x: f64, y: f64, width: f64, height: f64) -> Node {
        let mut node = Node::new(NodeKind::Rectangle, "Glass", x, y, width, height);
        node.fills.clear();
        node.effects = vec![blur(6.0)];
        node
    }

    fn divided_background() -> (Document, NodeId, NodeId) {
        let mut document = Document::new("Backdrop pixels");
        let left = document
            .insert_node(
                1,
                Node::new(NodeKind::Rectangle, "White", 0.0, 0.0, 64.0, 128.0)
                    .with_fill(Color::WHITE),
            )
            .unwrap();
        document
            .insert_node(
                1,
                Node::new(NodeKind::Rectangle, "Black", 64.0, 0.0, 64.0, 128.0)
                    .with_fill(Color::BLACK),
            )
            .unwrap();
        let glass = document
            .insert_node(1, glass(32.0, 32.0, 64.0, 64.0))
            .unwrap();
        (document, left, glass)
    }

    fn render(
        painter: &mut ScenePainter,
        document: &Document,
        scale: f64,
        pan: f64,
        host: Option<Color>,
    ) -> (RenderTargetBitmap, RenderStats) {
        let bitmap = RenderTargetBitmap::new(PixelSize::new(
            (128.0 * scale) as i32,
            (128.0 * scale) as i32,
        ));
        let stats;
        {
            let mut context = bitmap.create_drawing_context();
            let transform = context.push_transform(
                Matrix::create_translation(-pan, 0.0) * Matrix::create_scale(scale, scale),
            );
            if let Some(host) = host {
                context.fill_rectangle(
                    &solid_brush(host, 1.0),
                    UiRect::new(pan, 0.0, 128.0, 128.0),
                    0.0,
                );
            }
            painter.set_raster_scale(scale);
            stats = painter.render(
                &mut context,
                document,
                &document.pages[0].children,
                Rect::new(pan, 0.0, 128.0, 128.0),
            );
            context.pop(transform);
        }
        (bitmap, stats)
    }

    fn pixel(bitmap: &RenderTargetBitmap, x: i32, y: i32) -> [u8; 4] {
        let mut value = [0; 4];
        bitmap.copy_pixels(PixelRect::new(x, y, 1, 1), &mut value, 4);
        if bitmap.format() == Some(PixelFormat::BGRA8888) {
            value.swap(0, 2);
        }
        value
    }

    fn near(actual: [u8; 4], expected: [u8; 4]) {
        assert!(
            actual.iter().zip(expected).all(|(a, e)| a.abs_diff(e) <= 2),
            "{actual:?} != {expected:?}"
        );
    }

    #[test]
    fn backdrop_blur_samples_previous_pixels_and_preserves_rounded_corners() {
        let scope = backend();
        let (mut document, _, glass) = divided_background();
        document.nodes.get_mut(&glass).unwrap().corner_radius = 16.0;
        let (image, stats) = render(&mut ScenePainter::default(), &document, 1.0, 0.0, None);
        assert_eq!(stats.unsupported_effects, 0);
        assert_eq!(stats.backdrop_rasterizations, 1);
        let left = pixel(&image, 61, 64);
        let right = pixel(&image, 66, 64);
        assert!(left[0] < 255 && left[0] > 128, "left transition: {left:?}");
        assert!(
            right[0] > 0 && right[0] < 128,
            "right transition: {right:?}"
        );
        assert_eq!(left, [left[0], left[0], left[0], 255]);
        assert_eq!(right, [right[0], right[0], right[0], 255]);
        assert_eq!(pixel(&image, 33, 33), [255, 255, 255, 255]);
        assert_eq!(pixel(&image, 61, 20), [255, 255, 255, 255]);
        scope.dispose();
    }

    #[test]
    fn backdrop_blur_preserves_transparent_export_and_applies_ancestor_opacity_once() {
        let scope = backend();
        let mut document = Document::new("Transparent groups");
        let mut group = Node::new(
            NodeKind::Group,
            "Half-opacity group",
            0.0,
            0.0,
            128.0,
            128.0,
        );
        group.opacity = 0.5;
        let group = document.insert_node(1, group).unwrap();
        document
            .insert_node(
                group,
                Node::new(
                    NodeKind::Rectangle,
                    "Translucent red",
                    0.0,
                    0.0,
                    128.0,
                    128.0,
                )
                .with_fill(Color::rgba(255, 0, 0, 128)),
            )
            .unwrap();
        let mut filter = glass(40.0, 40.0, 48.0, 48.0);
        filter.opacity = 0.4;
        document.insert_node(group, filter).unwrap();
        let mut painter = ScenePainter::default();
        let (transparent, stats) = render(&mut painter, &document, 1.0, 0.0, None);
        assert_eq!(stats.unsupported_effects, 0);
        near(pixel(&transparent, 64, 64), [64, 0, 0, 64]);
        near(pixel(&transparent, 20, 20), [64, 0, 0, 64]);
        // The identical cached document pixels are composed over the host;
        // neither that host nor its opacity was captured into the prefix.
        let (host, stats) = render(
            &mut painter,
            &document,
            1.0,
            0.0,
            Some(Color::rgb(0, 255, 0)),
        );
        assert_eq!(stats.backdrop_rasterizations, 0);
        assert_eq!(stats.backdrop_cache_hits, 1);
        near(pixel(&host, 64, 64), [64, 191, 0, 255]);
        near(pixel(&host, 20, 20), [64, 191, 0, 255]);
        scope.dispose();
    }

    #[test]
    fn backdrop_blur_node_opacity_weights_the_complete_source_and_filtered_backdrop_once() {
        let scope = backend();
        let (mut document, _, glass) = divided_background();
        let filter = document.nodes.get_mut(&glass).unwrap();
        filter.opacity = 0.5;
        filter.fills = vec![Fill::solid(Color::rgb(255, 0, 0))];
        let (image, stats) = render(&mut ScenePainter::default(), &document, 1.0, 0.0, None);
        assert_eq!(stats.unsupported_effects, 0);
        // The opaque source covers the filtered backdrop before node opacity.
        // Blurring B first in the live scene would incorrectly soften this edge.
        near(pixel(&image, 61, 64), [255, 128, 128, 255]);
        near(pixel(&image, 66, 64), [128, 0, 0, 255]);
        scope.dispose();
    }

    #[test]
    fn backdrop_blur_uses_parent_local_prefix_under_world_scale_and_frame_clip() {
        let scope = backend();
        let mut document = Document::new("Transformed clipped scope");
        let mut frame = Node::new(NodeKind::Frame, "Frame", 0.0, 0.0, 100.0, 100.0);
        frame.set_local_transform(Transform::translation(8.0, 12.0) * Transform::scale(0.75, 0.75));
        let frame = document.insert_node(1, frame).unwrap();
        document
            .insert_node(
                frame,
                Node::new(NodeKind::Rectangle, "Overflow", 50.0, 0.0, 80.0, 100.0)
                    .with_fill(Color::BLACK),
            )
            .unwrap();
        document
            .insert_node(frame, glass(36.0, 30.0, 44.0, 44.0))
            .unwrap();
        let (image, stats) = render(&mut ScenePainter::default(), &document, 1.0, 0.0, None);
        assert_eq!(stats.unsupported_effects, 0);
        let left = pixel(&image, 43, 51);
        let right = pixel(&image, 48, 51);
        assert!(left[0] > 128 && left[0] < 255, "left: {left:?}");
        assert!(right[0] > 0 && right[0] < 128, "right: {right:?}");
        assert_eq!(pixel(&image, 90, 51), [0, 0, 0, 0]);
        assert_eq!(pixel(&image, 15, 18), [255, 255, 255, 255]);
        scope.dispose();
    }

    #[test]
    fn backdrop_blur_reuses_pan_frames_and_invalidates_after_prefix_changes() {
        let scope = backend();
        let (mut document, left, _) = divided_background();
        let mut painter = ScenePainter::default();
        let (first, first_stats) = render(&mut painter, &document, 1.0, 0.0, None);
        assert_eq!(first_stats.backdrop_rasterizations, 1);
        let (panned, panned_stats) = render(&mut painter, &document, 1.0, 5.0, None);
        assert_eq!(panned_stats.backdrop_rasterizations, 0);
        assert_eq!(panned_stats.backdrop_cache_hits, 1);
        assert_eq!(pixel(&first, 61, 64), pixel(&panned, 56, 64));
        document.nodes.get_mut(&left).unwrap().fills = vec![Fill::solid(Color::rgb(255, 0, 0))];
        document.revision += 1;
        let (changed, stats) = render(&mut painter, &document, 1.0, 5.0, None);
        assert_eq!(stats.backdrop_rasterizations, 1);
        let changed = pixel(&changed, 56, 64);
        assert!(changed[0] > 0 && changed[0] < 255);
        assert_eq!((changed[1], changed[2], changed[3]), (0, 0, 255));
        scope.dispose();
    }

    #[test]
    fn backdrop_blur_high_resolution_surfaces_use_the_complete_source_pixel_rectangle() {
        let scope = backend();
        let (document, _, _) = divided_background();
        let mut painter = ScenePainter::default();
        render(&mut painter, &document, 1.0, 0.0, None);
        let (twice, stats) = render(&mut painter, &document, 2.0, 0.0, None);
        assert_eq!(stats.backdrop_rasterizations, 1);
        let left = pixel(&twice, 122, 128);
        let right = pixel(&twice, 132, 128);
        assert!(left[0] < 255 && left[0] > 128, "left: {left:?}");
        assert!(right[0] > 0 && right[0] < 128, "right: {right:?}");
        assert_eq!(right[3], 255);
        let (_, stats) = render(&mut painter, &document, 1.0, 0.0, None);
        assert_eq!(stats.backdrop_rasterizations, 0);
        assert_eq!(stats.backdrop_cache_hits, 1);
        scope.dispose();
    }

    #[test]
    fn backdrop_blur_oversized_regions_are_declined_before_allocation_and_reported() {
        let scope = backend();
        let (mut document, _, glass) = divided_background();
        document.nodes.get_mut(&glass).unwrap().width = MAX_DIMENSION as f64 + 1.0;
        let mut painter = ScenePainter::default();
        let (image, stats) = render(&mut painter, &document, 1.0, 0.0, None);
        assert_eq!(stats.backdrop_rasterizations, 0);
        assert_eq!(stats.backdrop_skipped, 1);
        assert_eq!(stats.backdrop_too_large, 1);
        assert_eq!(stats.unsupported_effects, 1);
        assert!(painter.backdrops.entries.is_empty());
        assert_eq!(pixel(&image, 61, 64), [255, 255, 255, 255]);
        scope.dispose();
    }
}

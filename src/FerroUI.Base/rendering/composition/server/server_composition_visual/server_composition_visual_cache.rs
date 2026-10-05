use super::ServerCompositionVisual;
use crate::media::Colors;
use crate::platform::{IDrawingContextImpl, IDrawingContextLayerImpl, IPlatformRenderInterfaceContext, LtrbRect};
use crate::rendering::composition::server::{
    IDirtyRectCollector, IDirtyRectTracker, ServerCompositionBitmapCache, SingleDirtyRectTracker,
};
use crate::{Matrix, PixelSize, Rect, Vector};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// The bitmap cache of a visual: an offscreen layer the subtree of the
/// visual is rendered into, redrawn only where it changed.
///
/// The cache is its own dirty rect collector (`DirtyRectCollector`
/// upstream).
pub struct ServerCompositionVisualCache {
    cache_mode: Rc<ServerCompositionBitmapCache>,
    target_visual: Weak<ServerCompositionVisual>,
    needs_full_re_render: Cell<bool>,
    layer: RefCell<Option<Rc<dyn IDrawingContextLayerImpl>>>,
    layer_created_with_context: RefCell<Option<Rc<dyn IPlatformRenderInterfaceContext>>>,
    layer_has_text_antialiasing: Cell<bool>,
    desired_layer_size: Cell<PixelSize>,
    scale_x: Cell<f64>,
    scale_y: Cell<f64>,
    draw_at_offset: Cell<Vector>,
    need_to_finalize_frame: Cell<bool>,
    dirty_rect_tracker: Rc<SingleDirtyRectTracker>,
}

impl ServerCompositionVisualCache {
    pub fn new(
        visual: Weak<ServerCompositionVisual>,
        cache_mode: Rc<ServerCompositionBitmapCache>,
    ) -> Rc<ServerCompositionVisualCache> {
        let cache = Rc::new(ServerCompositionVisualCache {
            cache_mode,
            target_visual: visual,
            needs_full_re_render: Cell::new(false),
            layer: RefCell::new(None),
            layer_created_with_context: RefCell::new(None),
            layer_has_text_antialiasing: Cell::new(false),
            desired_layer_size: Cell::new(PixelSize::default()),
            scale_x: Cell::new(0.0),
            scale_y: Cell::new(0.0),
            draw_at_offset: Cell::new(Vector::default()),
            need_to_finalize_frame: Cell::new(true),
            dirty_rect_tracker: Rc::new(SingleDirtyRectTracker::new()),
        });
        cache.mark_for_full_re_render();
        cache
    }

    pub fn is_dirty(&self) -> bool {
        !self.dirty_rect_tracker.is_empty()
    }

    pub fn target_visual(&self) -> Option<Rc<ServerCompositionVisual>> {
        self.target_visual.upgrade()
    }

    fn render_at_scale(&self) -> f64 {
        self.cache_mode.render_at_scale()
    }

    fn snaps_to_device_pixels(&self) -> bool {
        self.cache_mode.snaps_to_device_pixels()
    }

    fn enable_clear_type(&self) -> bool {
        self.cache_mode.enable_clear_type()
    }

    pub fn free_resources(&self) {
        if let Some(layer) = self.layer.borrow_mut().take() {
            layer.dispose();
        }
        *self.layer_created_with_context.borrow_mut() = None;
    }

    pub fn invalidate_properties(&self) {
        self.mark_for_full_re_render();
    }

    fn reset_dirty_rects(&self) {
        self.need_to_finalize_frame.set(true);
        self.dirty_rect_tracker.initialize(LtrbRect::INFINITE);
    }

    fn mark_for_full_re_render(&self) {
        self.needs_full_re_render.set(true);
        self.reset_dirty_rects();
    }

    fn is_close_real(a: f64, b: f64) -> bool {
        // Underlying rendering platform is using floats anyway, so we use float epsilon here
        ((a - b) / (if b == 0.0 { 1.0 } else { b })).abs() < 10.0 * f32::EPSILON as f64
    }

    fn update_realization_dimensions(&self) -> bool {
        let Some(target_visual) = self.target_visual() else { return false };
        let (Some(root), Some(visual_bounds), Some(compositor)) =
            (target_visual.root(), target_visual.sub_tree_bounds(), target_visual.compositor())
        else {
            return false;
        };

        // Since the cache relies only on local space bounds, the DPI isn't taken into account (as it's the root
        // transform of the visual tree).  Scale for DPI if needed here.
        let scale = root.scaling() * self.render_at_scale();

        // Caches are not clipped to the window bounds, they use local space bounds,
        // so (especially in combination with RenderScale) a very large intermediate
        // surface could be requested.  Instead of failing in this case, we clamp the
        // surface to the max texture size, which can cause some pixelation but will
        // allow the app to render in hardware and still benefit from a cache.
        let max_size = compositor
            .render_interface()
            .value()
            .max_offscreen_render_target_pixel_size()
            .unwrap_or(PixelSize::new(16384, 16384));

        // We round our bounds up to integral values for consistency here, since we need to do so when creating the surface anyway.
        // This also ensures that our content will always be drawn in its entirety in the texture.
        let f_width = visual_bounds.width() * scale;
        let mut u_width = f_width as i32;
        // If our width was non-integer, round up.
        if !Self::is_close_real(f_width, f_width) {
            u_width += 1;
        }

        let f_height = visual_bounds.height() * scale;
        let mut u_height = f_height as i32;
        // If our height was non-integer, round up.
        if !Self::is_close_real(f_height, f_height) {
            u_height += 1;
        }

        self.scale_x.set(scale);
        self.scale_y.set(scale);
        if u_width > max_size.width {
            self.scale_x.set(self.scale_x.get() * max_size.width as f64 / u_width as f64);
            u_width = max_size.width;
        }

        if u_height > max_size.height {
            self.scale_y.set(self.scale_y.get() * max_size.height as f64 / u_height as f64);
            u_height = max_size.height;
        }

        self.draw_at_offset.set(Vector::new(-visual_bounds.left, -visual_bounds.top));
        self.desired_layer_size.set(PixelSize::new(u_width, u_height));
        true
    }

    /// Draws the cached subtree onto `outer_canvas`, re-rendering the dirty
    /// part of the cache first. Returns the numbers of visited and rendered
    /// visuals.
    pub fn draw(self: &Rc<Self>, outer_canvas: &mut dyn IDrawingContextImpl) -> (i32, i32) {
        let Some(target_visual) = self.target_visual() else { return (0, 0) };
        let Some(sub_tree_bounds) = target_visual.sub_tree_bounds() else { return (0, 0) };
        let Some(compositor) = target_visual.compositor() else { return (0, 0) };

        self.update_realization_dimensions();

        let render_context = compositor.render_interface().value();

        // Re-create layer if needed
        let recreate = {
            let layer = self.layer.borrow();
            match layer.as_ref() {
                None => true,
                Some(layer) => {
                    self.layer_has_text_antialiasing.get() != self.enable_clear_type()
                        || layer.pixel_size() != self.desired_layer_size.get()
                        || !self
                            .layer_created_with_context
                            .borrow()
                            .as_ref()
                            .is_some_and(|context| Rc::ptr_eq(context, &render_context))
                }
            }
        };
        if recreate {
            if let Some(layer) = self.layer.borrow_mut().take() {
                layer.dispose();
            }
            *self.layer_created_with_context.borrow_mut() = None;

            let desired = self.desired_layer_size.get();
            if desired.width < 1 || desired.height < 1 {
                self.reset_dirty_rects();
                return (0, 0);
            }

            *self.layer.borrow_mut() = Some(render_context.create_offscreen_render_target(
                desired,
                Vector::new(self.scale_x.get(), self.scale_x.get()),
                self.enable_clear_type(),
            ));
            self.layer_has_text_antialiasing.set(self.enable_clear_type());
            *self.layer_created_with_context.borrow_mut() = Some(render_context);
            self.needs_full_re_render.set(true);
        }
        let Some(layer) = self.layer.borrow().clone() else { return (0, 0) };
        let layer_size = layer.pixel_size();

        let full_frame_rect = LtrbRect::new(0.0, 0.0, layer_size.width as f64, layer_size.height as f64);

        // Extend the dirty rect area if needed
        if self.needs_full_re_render.get() {
            self.reset_dirty_rects();
            self.add_rect(LtrbRect::INFINITE);
        }

        // Compute the final dirty rect set that accounts for antialiasing effects
        if self.need_to_finalize_frame.get() {
            self.dirty_rect_tracker.finalize_frame(full_frame_rect);
            self.need_to_finalize_frame.set(false);
        }

        let visual_local_bounds = sub_tree_bounds.to_rect();
        let mut rv = (0, 0);
        // Render to layer if needed
        if !self.dirty_rect_tracker.is_empty() {
            let mut ctx = layer.create_drawing_context();
            let clipped = !self.needs_full_re_render.get();
            if clipped {
                self.dirty_rect_tracker.begin_draw(&mut *ctx);
            }
            ctx.clear(Colors::TRANSPARENT);
            let offset = self.draw_at_offset.get();
            ctx.set_transform(
                Matrix::create_translation(offset.x, offset.y)
                    * Matrix::create_scale(self.scale_x.get(), self.scale_y.get()),
            );
            let tracker: Rc<dyn IDirtyRectTracker> = self.dirty_rect_tracker.clone();
            rv = target_visual.render(
                &mut *ctx,
                self.dirty_rect_tracker.combined_rect(),
                Some(tracker),
                true,
                false,
                true,
            );
            if clipped {
                self.dirty_rect_tracker.end_draw(&mut *ctx);
            }
            ctx.dispose();
        }

        self.needs_full_re_render.set(false);

        let original_transform = outer_canvas.transform();
        if self.snaps_to_device_pixels() {
            let world_bounds = visual_local_bounds.transform_to_aabb(original_transform);
            let snap_offset_x = world_bounds.x - world_bounds.x.floor();
            let snap_offset_y = world_bounds.y - world_bounds.y.floor();
            outer_canvas.set_transform(original_transform * Matrix::create_translation(-snap_offset_x, -snap_offset_y));
        }

        //TODO: Maybe adjust for that extra pixel added due to rounding?
        outer_canvas.draw_bitmap(
            &*layer,
            1.0,
            Rect::new(0.0, 0.0, layer_size.width as f64, layer_size.height as f64),
            visual_local_bounds,
        );
        if self.snaps_to_device_pixels() {
            outer_canvas.set_transform(original_transform);
        }

        // Set empty dirty rects for next frame
        self.reset_dirty_rects();
        rv
    }
}

impl IDirtyRectCollector for ServerCompositionVisualCache {
    fn add_rect(&self, rect: LtrbRect) {
        self.need_to_finalize_frame.set(true);
        // scale according to our render transform, since those values come in local space of the visual
        let offset = self.draw_at_offset.get();
        let (scale_x, scale_y) = (self.scale_x.get(), self.scale_y.get());
        self.dirty_rect_tracker.add_rect(LtrbRect::new(
            (rect.left + offset.x) * scale_x,
            (rect.top + offset.y) * scale_y,
            (rect.right + offset.x) * scale_x,
            (rect.bottom + offset.y) * scale_y,
        ));
    }
}

//! Native image crop gestures and retained DrawingContext adorners.
use super::*;

#[derive(Clone, Copy)]
pub(super) struct ImageCropMode {
    node: NodeId,
    fill: usize,
    image_size: Point,
    resize: bool,
}

pub(super) struct ImageCropGesture {
    session: core::ImageCropSession,
    inverse_world: core::Transform,
    start_local: Point,
    corner: Option<usize>,
    changed: bool,
}

impl DesignCanvas {
    /// Enter crop mode for one selected image paint. Decoding happens once at
    /// entry (or returns the retained bitmap cache); pointer updates retain it.
    pub fn edit_image_crop(&self, fill: Option<usize>) -> bool {
        self.finish_pen();
        self.cancel_image_crop();
        self.cancel_active_edit();
        let mode = {
            let editor = self.editor.borrow();
            if editor.selection.len() != 1 { return false; }
            let node_id = editor.selection[0];
            if editor.document.effective_locked(node_id) || !editor.document.effective_visible(node_id) {
                return false;
            }
            let Some(node) = editor.document.nodes.get(&node_id) else { return false; };
            let fill = fill.or_else(|| node.fills.iter().enumerate().rev().find_map(|(index, fill)| {
                (fill.visible && fill.opacity > 0.0 && matches!(fill.paint, core::Paint::Image { .. })).then_some(index)
            }));
            let Some(fill) = fill else { return false; };
            let Some(core::Fill { paint: core::Paint::Image { href, .. }, .. }) = node.fills.get(fill) else {
                return false;
            };
            let Ok((width, height)) = self.register_image_data(href) else { return false; };
            let image_size = Point::new(width, height);
            if core::ImageCropSession::new(node, fill, image_size).is_none() { return false; }
            ImageCropMode { node: node_id, fill, image_size, resize: editor.can_resize_image_crop(node_id) }
        };
        self.leave_vector_edit();
        self.tool.set(Tool::Select);
        self.image_crop.set(Some(mode));
        self.hover.set(None);
        self.focus();
        self.invalidate_visual();
        self.emit(CanvasEvent::Selection);
        true
    }

    /// This is observable state for the actual native mode, never a parallel
    /// document or a synthetic browser editor.
    pub fn image_crop_state(&self) -> Option<(NodeId, usize, bool)> {
        self.image_crop.get().map(|mode| (mode.node, mode.fill, self.image_crop_gesture.borrow().is_some()))
    }

    pub fn finish_image_crop(&self) -> bool {
        let mode = self.image_crop.take().is_some();
        let gesture = self.finish_image_crop_gesture(true);
        if mode {
            self.invalidate_visual();
            self.emit(CanvasEvent::Selection);
        }
        mode || gesture
    }

    pub fn cancel_image_crop(&self) -> bool {
        // Clear the mode before callbacks can refresh and reconcile selection.
        let mode = self.image_crop.take().is_some();
        let gesture = self.finish_image_crop_gesture(false);
        if mode {
            self.invalidate_visual();
            self.emit(CanvasEvent::Selection);
        }
        mode || gesture
    }

    pub fn cancel_image_crop_gesture(&self) -> bool {
        self.finish_image_crop_gesture(false)
    }

    fn finish_image_crop_gesture(&self, commit: bool) -> bool {
        // Pointer capture-lost and shell refresh can be synchronous. Take the
        // gesture before committing history or raising document callbacks.
        let gesture = self.image_crop_gesture.borrow_mut().take();
        let Some(gesture) = gesture else { return false; };
        {
            let mut editor = self.editor.borrow_mut();
            if commit && gesture.changed {
                editor.commit_transaction();
            } else {
                editor.cancel_transaction();
            }
        }
        self.invalidate_visual();
        self.emit(CanvasEvent::Document);
        true
    }

    pub fn reconcile_image_crop(&self) {
        let Some(mut mode) = self.image_crop.get() else { return; };
        let valid = {
            let editor = self.editor.borrow();
            mode.resize = editor.can_resize_image_crop(mode.node);
            editor.selection.as_slice() == [mode.node]
                && !editor.document.effective_locked(mode.node)
                && editor.document.effective_visible(mode.node)
                && editor.document.nodes.get(&mode.node).is_some_and(|node| {
                    core::ImageCropSession::new(node, mode.fill, mode.image_size).is_some()
                })
        };
        if valid { self.image_crop.set(Some(mode)); }
        else { self.cancel_image_crop(); }
    }

    pub(super) fn image_crop_press(&self, world: Point) -> bool {
        // A replacement press must snapshot the restored document, rather
        // than geometry from a preview that cancellation is about to discard.
        self.cancel_image_crop_gesture();
        self.reconcile_image_crop();
        let Some(mode) = self.image_crop.get() else { return false; };
        let prepared = {
            let editor = self.editor.borrow();
            let Some(node) = editor.document.nodes.get(&mode.node) else { return false; };
            let Some(transform) = editor.document.world_transform(mode.node) else { return false; };
            let Some(inverse_world) = transform.inverse() else { return false; };
            let local = inverse_world.apply(world);
            let viewport = self.viewport.get();
            let pointer = viewport.world_to_screen(world);
            let corner = if mode.resize {
                node.local_bounds().corners().iter().position(|corner| {
                    viewport.world_to_screen(transform.apply(*corner)).distance(pointer) <= 8.0
                })
            } else { None };
            if corner.is_none() && !node.local_bounds().contains(local) {
                None
            } else {
                core::ImageCropSession::new(node, mode.fill, mode.image_size).map(|session| ImageCropGesture {
                    session, inverse_world, start_local: local, corner, changed: false,
                })
            }
        };
        let Some(gesture) = prepared else {
            self.finish_image_crop();
            return false;
        };
        self.editor.borrow_mut().begin_node_transaction(&[mode.node], "Crop image");
        *self.image_crop_gesture.borrow_mut() = Some(gesture);
        self.invalidate_visual();
        true
    }

    pub(super) fn image_crop_move(&self, world: Point) -> bool {
        let mut active = self.image_crop_gesture.borrow_mut();
        let Some(gesture) = active.as_mut() else { return false; };
        let local = gesture.inverse_world.apply(world);
        let delta = local - gesture.start_local;
        let original_bounds = Rect::new(0.0, 0.0, gesture.session.width, gesture.session.height);
        // Offset the corner by the actual pointer displacement, so clicking a
        // handle a few pixels off center never jumps or records a crop.
        let point = gesture.corner.map(|corner| original_bounds.corners()[corner] + delta);
        let window = if let (Some(corner), Some(point)) = (gesture.corner, point) {
            gesture.session.resize(corner, point)
        } else {
            gesture.session.pan(delta).map(|crop| (crop, original_bounds))
        };
        let Some((crop, bounds)) = window else { return true; };
        let mut editor = self.editor.borrow_mut();
        if gesture.session.is_original_window(crop, bounds) {
            if gesture.changed {
                // Restore the exact style/binding metadata on a drag back to
                // its start. Start a fresh snapshot if the pointer moves again.
                editor.cancel_transaction();
                editor.begin_node_transaction(&[gesture.session.node], "Crop image");
                gesture.changed = false;
            }
        } else {
            let changed = if let (Some(corner), Some(point)) = (gesture.corner, point) {
                editor.resize_image_crop(&gesture.session, corner, point)
            } else {
                editor.set_image_crop(gesture.session.node, gesture.session.fill, crop)
            };
            gesture.changed |= changed;
        }
        drop(editor);
        drop(active);
        self.invalidate_visual();
        true
    }

    /// Call before releasing native pointer capture.
    pub(super) fn image_crop_release(&self) -> bool {
        self.finish_image_crop_gesture(true)
    }

    pub(super) fn image_crop_key(&self, key: Key) -> bool {
        if self.image_crop.get().is_none() { return false; }
        match key {
            Key::Escape => {
                if !self.cancel_image_crop_gesture() { self.cancel_image_crop(); }
                true
            }
            Key::Enter => { self.finish_image_crop(); true }
            _ => false,
        }
    }

    pub(super) fn render_image_crop(
        &self, ctx: &mut DrawingContext, node: &Node, transform: core::Transform, viewport: Viewport,
    ) -> bool {
        let Some(mode) = self.image_crop.get().filter(|mode| mode.node == node.id) else { return false; };
        let Some(session) = core::ImageCropSession::new(node, mode.fill, mode.image_size) else { return false; };
        let screen = |point| viewport.world_to_screen(transform.apply(point));
        let corners = node.local_bounds().corners().map(screen);
        let border = pen(0xFF0D99FF, 1.5);
        let guides = pen(0xBFFFFFFF, 1.0);
        let full = Rect::new(
            -session.source.x * node.width / session.source.width,
            -session.source.y * node.height / session.source.height,
            node.width / session.source.width, node.height / session.source.height,
        ).corners().map(screen);
        let source_outline = pen(0x80737373, 1.0);
        for index in 0..4 {
            ctx.draw_line(&source_outline, ui(full[index]), ui(full[(index + 1) % 4]));
            ctx.draw_line(&border, ui(corners[index]), ui(corners[(index + 1) % 4]));
        }
        for fraction in [1.0 / 3.0, 2.0 / 3.0] {
            ctx.draw_line(&guides,
                ui(screen(Point::new(node.width * fraction, 0.0))),
                ui(screen(Point::new(node.width * fraction, node.height))));
            ctx.draw_line(&guides,
                ui(screen(Point::new(0.0, node.height * fraction))),
                ui(screen(Point::new(node.width, node.height * fraction))));
        }
        if mode.resize {
            let white = brush("#FFFFFF");
            for point in corners {
                ctx.draw_rectangle(Some(&white), Some(&border),
                    UiRect::new(point.x - 4.0, point.y - 4.0, 8.0, 8.0),
                    0.0, 0.0, &BoxShadows::default());
            }
        }
        let instruction = if mode.resize {
            "Crop image · Drag image or corners · Enter done · Esc cancel drag"
        } else {
            "Crop image · Drag image · Flow layout owns bounds · Enter done"
        };
        let left = corners.iter().map(|point| point.x).fold(f64::INFINITY, f64::min).max(8.0);
        let top = corners.iter().map(|point| point.y).fold(f64::INFINITY, f64::min);
        label(ctx, instruction, Point::new(left, (top - 23.0).max(8.0)), 11.0, "#0D99FF");
        true
    }
}

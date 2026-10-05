//! Support for adorners is a rather invasive thing, so all the related code
//! is isolated in this file and prefixed with `adorner_helper_`.

use super::render::RenderContext;
use super::ServerCompositionVisual;
use crate::rendering::composition::MatrixUtils;
use crate::{Matrix, Rect};
use std::rc::Rc;

const OP_POP_CLIP: i32 = 0;
const OP_POP_GEOMETRY_CLIP: i32 = 1;
const OP_STOP: i32 = 2;

impl ServerCompositionVisual {
    pub(super) fn adorner_helper_attached_to_root(&self) {
        if self.adorned_visual().is_some() {
            self.adorner_helper_enqueue_for_adorner_update();
        }
    }

    pub fn adorner_helper_enqueue_for_adorner_update(&self) {
        if self.att_helper.enqueued_for_adorner_update.get() {
            return;
        }
        if let (Some(compositor), Some(this)) = (self.compositor(), self.rc()) {
            compositor.enqueue_adorner_update(this);
            self.att_helper.enqueued_for_adorner_update.set(true);
        }
    }

    pub(super) fn on_adorned_visual_changing_core(&self) {
        if let Some(adorned) = self.adorned_visual() {
            adorned.att_helper_unsubscribe_from_act_notification(&self.adorned_visual_act_subscription_action());
        }
    }

    pub(super) fn on_adorned_visual_changed_core(&self) {
        if let Some(adorned) = self.adorned_visual() {
            adorned.att_helper_subscribe_to_act_notification(self.adorned_visual_act_subscription_action());
        }
        self.adorner_helper_enqueue_for_adorner_update();
    }

    fn adorner_layer_get_expected_shared_ancestor(adorner: &ServerCompositionVisual) -> Option<Rc<ServerCompositionVisual>> {
        // This is hardcoded to VisualLayerManager -> AdornerLayer -> adorner
        // Since AdornedVisual is a private API that's only supposed to be accessible from AdornerLayer
        // it's a safe assumption to make
        adorner.parent()?.parent()
    }

    pub fn update_adorner(&self) {
        self.att_helper.enqueued_for_adorner_update.set(false);
        let adorned = self.adorned_visual();
        if let (Some(adorned), true) = (&adorned, self.parent().is_some()) {
            // We ignore Visual's RenderTransform completely since it's set by AdornerLayer and can be out of sync
            // with compositor-driver animations
            let own_transform = MatrixUtils::compute_transform(
                self.size(),
                self.anchor_point(),
                self.center_point(),
                Matrix::IDENTITY,
                self.scale(),
                self.rotation_angle(),
                self.orientation(),
                self.offset() + self.translation(),
            );

            let adorner_layer_to_adorned_visual = Self::adorner_layer_get_expected_shared_ancestor(self)
                .and_then(|shared_ancestor| Self::compute_transform_from_ancestor(adorned, &shared_ancestor));
            match adorner_layer_to_adorned_visual {
                Some(transform) => {
                    self.own_transform.set(Some(own_transform.unwrap_or(Matrix::IDENTITY) * transform))
                }
                // Don't render, something is broken
                None => self.own_transform.set(Some(Matrix::new_3x3(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0))),
            }
        } else {
            self.own_transform.set(MatrixUtils::compute_transform(
                self.size(),
                self.anchor_point(),
                self.center_point(),
                self.transform_matrix(),
                self.scale(),
                self.rotation_angle(),
                self.orientation(),
                self.offset() + self.translation(),
            ));
        }

        self.propagate_flags(true, true, false);
    }
}

impl RenderContext<'_> {
    fn adorner_layer_walk_adorner_parent_clip_recursive(&mut self, visual: Option<Rc<ServerCompositionVisual>>) -> bool {
        // AdornedVisual is a part of a different subtree, this is not supported
        let Some(visual) = visual else { return false };
        let is_layer = self.current_adorner_layer.as_ref().is_some_and(|layer| Rc::ptr_eq(layer, &visual));
        if !is_layer && !self.adorner_layer_walk_adorner_parent_clip_recursive(visual.parent()) {
            return false;
        }

        if let Some(own_transform) = visual.own_transform.get() {
            let transform = own_transform * self.canvas.transform();
            self.canvas.set_transform(transform);
        }

        if visual.clip_to_bounds() {
            let size = visual.size();
            self.canvas.push_clip(Rect::new(0.0, 0.0, size.x, size.y));
            if let Some(stack) = &mut self.adorner_pushed_clip_stack {
                stack.push(OP_POP_CLIP);
            }
        }

        if let Some(clip) = visual.clip() {
            self.canvas.push_geometry_clip(&*clip);
            if let Some(stack) = &mut self.adorner_pushed_clip_stack {
                stack.push(OP_POP_GEOMETRY_CLIP);
            }
        }

        true
    }

    fn skip_adorner_clip(&self, visual: &Rc<ServerCompositionVisual>) -> bool {
        !visual.adorner_is_clipped()
            || Rc::ptr_eq(visual, &self.root_visual)
            // Root visual is AdornerLayer
            || visual.parent().is_some_and(|parent| Rc::ptr_eq(&parent, &self.root_visual))
            || ServerCompositionVisual::adorner_layer_get_expected_shared_ancestor(visual).is_none()
    }

    pub(super) fn adorner_helper_render_pre_graph_push_adorner_clip(&mut self, visual: &Rc<ServerCompositionVisual>) {
        if self.skip_adorner_clip(visual) {
            return;
        }

        let pools = self.pools;
        self.adorner_pushed_clip_stack.get_or_insert_with(|| pools.int_stack_pool.rent()).push(OP_STOP);

        let original_transform = self.canvas.transform();
        let mut transform = original_transform;
        if let Some(own_transform) = visual.own_transform.get() {
            let Some(transform_to_adorner_layer) = own_transform.try_invert() else { return };
            transform = transform_to_adorner_layer * transform;
        }

        self.canvas.set_transform(transform);

        self.current_adorner_layer = ServerCompositionVisual::adorner_layer_get_expected_shared_ancestor(visual);
        self.adorner_layer_walk_adorner_parent_clip_recursive(visual.adorned_visual());

        self.canvas.set_transform(original_transform);
    }

    pub(super) fn adorner_helper_render_post_graph_push_adorner_clip(&mut self, visual: &Rc<ServerCompositionVisual>) {
        if self.skip_adorner_clip(visual) {
            return;
        }

        loop {
            let Some(op) = self.adorner_pushed_clip_stack.as_mut().and_then(Vec::pop) else { break };
            if op == OP_STOP {
                break;
            }
            if op == OP_POP_GEOMETRY_CLIP {
                self.canvas.pop_geometry_clip();
            } else if op == OP_POP_CLIP {
                self.canvas.pop_clip();
            }
        }
    }

    pub(super) fn adorner_helper_dispose(&mut self) {
        if let Some(stack) = self.adorner_pushed_clip_stack.take() {
            self.pools.int_stack_pool.return_stack(stack);
        }
    }
}

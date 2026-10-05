use super::ZIndexComparer;
use crate::media::{DrawingContext, PushedState, RenderOptions, TextOptions};
use crate::{Matrix, Rect, Ref, RoundedRect, Visual};

/// Renders a visual tree directly to a drawing context, without the
/// compositor. Used for rendering visuals to bitmaps and visual brushes.
pub struct ImmediateRenderer;

impl ImmediateRenderer {
    /// Renders a visual to a drawing context.
    pub fn render(context: &mut DrawingContext<'_>, visual: &Ref<Visual>) {
        Self::render_clipped(context, visual, Rect::from_size(visual.bounds().size()));
    }

    /// Renders the part of a visual inside `clip_rect` to a drawing context.
    pub fn render_clipped(context: &mut DrawingContext<'_>, visual: &Ref<Visual>, clip_rect: Rect) {
        let transform =
            context.push_transform(Matrix::create_translation(-clip_rect.position().x, -clip_rect.position().y));
        let clip = context.push_clip(clip_rect);
        Self::render_core(
            context,
            visual,
            Rect::from_size(visual.bounds().size()),
            Matrix::IDENTITY,
            Rect::from_size(clip_rect.size()),
        );
        context.pop(clip);
        context.pop(transform);
    }

    fn render_core(
        context: &mut DrawingContext<'_>,
        visual: &Ref<Visual>,
        bounds: Rect,
        parent_transform: Matrix,
        mut clip_rect: Rect,
    ) {
        let opacity = visual.opacity();
        // NaN opacity is not rendered either (`Opacity is not > 0`).
        if !visual.is_visible() || !(opacity > 0.0) {
            return;
        }

        let rect = Rect::from_size(bounds.size());
        let transform = match visual.render_transform().map(|t| t.value()) {
            Some(rt) => {
                let origin = visual.render_transform_origin().to_pixels(visual.bounds().size());
                let offset = Matrix::create_translation(origin.x, origin.y);
                (-offset) * rt * offset * Matrix::create_translation(bounds.position().x, bounds.position().y)
            }
            None => Matrix::create_translation(bounds.position().x, bounds.position().y),
        };

        let text_options = visual.text_options();
        let s_text_options = if text_options != TextOptions::default() {
            context.push_text_options(text_options)
        } else {
            PushedState::NONE
        };
        let render_options = visual.render_options();
        let s_render_options = if render_options != RenderOptions::default() {
            context.push_render_options(render_options)
        } else {
            PushedState::NONE
        };
        let s_transform = context.push_transform(transform);
        let s_mirror = if visual.has_mirror_transform() {
            context.push_transform(Matrix::new(-1.0, 0.0, 0.0, 1.0, visual.bounds().width, 0.0))
        } else {
            PushedState::NONE
        };
        let s_opacity = context.push_opacity(opacity);
        let clip_to_bounds = visual.clip_to_bounds();
        let s_clip = if clip_to_bounds {
            match visual.clip_to_bounds_radius() {
                Some(radius) => context.push_rounded_clip(RoundedRect::from_corner_radius(rect, radius)),
                None => context.push_clip(rect),
            }
        } else {
            PushedState::NONE
        };
        let s_geometry_clip = match visual.clip() {
            Some(clip) => context.push_geometry_clip(&clip),
            None => PushedState::NONE,
        };
        let s_opacity_mask = match visual.opacity_mask() {
            Some(mask) => context.push_opacity_mask(&mask, rect),
            None => PushedState::NONE,
        };
        let effect = visual.effect();
        let s_effect = match &effect {
            Some(effect) => context.push_effect(effect, rect),
            None => PushedState::NONE,
        };

        {
            let mut total_transform = transform * parent_transform;
            let effect_rect = match &effect {
                Some(effect) => rect.inflate_thickness(crate::media::EffectExtensions::get_effect_output_padding(Some(&**effect))),
                None => rect,
            };
            let visual_bounds = effect_rect.transform_to_aabb(total_transform);

            if visual_bounds.intersects(clip_rect) {
                visual.render(context);
            }

            if clip_to_bounds {
                total_transform = Matrix::IDENTITY;
                clip_rect = rect;
            }

            if let Some(children) = visual.visual_children_snapshot() {
                if visual.has_non_uniform_z_index_children() {
                    let mut sorted: Vec<Ref<Visual>> = children.iter().cloned().collect();
                    // Stable, like `OrderBy`.
                    sorted.sort_by(ZIndexComparer::compare);
                    for child in &sorted {
                        Self::render_core(context, child, child.bounds(), total_transform, clip_rect);
                    }
                } else {
                    for child in children.iter() {
                        Self::render_core(context, child, child.bounds(), total_transform, clip_rect);
                    }
                }
            }
        }

        context.pop(s_effect);
        context.pop(s_opacity_mask);
        context.pop(s_geometry_clip);
        context.pop(s_clip);
        context.pop(s_opacity);
        context.pop(s_mirror);
        context.pop(s_transform);
        context.pop(s_render_options);
        context.pop(s_text_options);
    }
}

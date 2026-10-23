//! Sibling mask scopes and vector coverage use FerroUI geometry and public
//! compositing. Pixel masks are cached with the backdrop they interpolate.
use super::*;

pub(super) fn paint_has_alpha(paint: &Paint) -> bool {
    match paint {
        Paint::Solid(color) => color.a != 0,
        Paint::LinearGradient { stops, .. } | Paint::RadialGradient { stops, .. }
            | Paint::AngularGradient { stops, .. } => {
            stops.iter().any(|stop| stop.color.a != 0)
        }
        Paint::Image { .. } => true,
    }
}

pub(super) fn add_vector_children(
    own: Ref<Geometry>,
    children: &[PreparedNode],
    base: usize,
    node: &Node,
) -> Ref<Geometry> {
    let mut content: Ref<Geometry> = StreamGeometry::new().upcast();
    let mut active_mask: Option<Ref<Geometry>> = None;
    let mut cursor = 0;
    while cursor < children.len() {
        let child = &children[cursor];
        let geometry = if child.opacity <= 0.0 {
            StreamGeometry::new().upcast::<Geometry>()
        } else {
            child.visual.vector_mask.clone_geometry()
        };
        geometry.set_transform(MatrixTransform::with_matrix(ui_matrix(child.local)));
        if child.mask.is_some() {
            active_mask = Some(match active_mask {
                Some(previous) => combined(GeometryCombineMode::Intersect, previous, geometry),
                None => geometry,
            });
        } else {
            let geometry = if let Some(mask) = &active_mask {
                combined(GeometryCombineMode::Intersect, geometry, mask.clone())
            } else {
                geometry
            };
            content = combined(GeometryCombineMode::Union, content, geometry);
        }
        cursor = child.end.saturating_sub(base);
    }
    if node.clip_content {
        content = combined(
            GeometryCombineMode::Intersect,
            content,
            corner_geometry(node),
        );
    }
    combined(GeometryCombineMode::Union, own, content)
}

/// `end` can stop inside a pass-through descendant when reconstructing a
/// backdrop prefix. Full paint uses the containing parent's actual end.
pub(super) fn paint_siblings(
    context: &mut DrawingContext<'_>,
    scene: &[PreparedNode],
    mut index: usize,
    end: usize,
    viewport: Rect,
    scene_reused: bool,
    stats: &mut RenderStats,
    backdrops: &mut backdrop::BackdropCache,
    roots: &[usize],
) {
    // Sibling mask counts are independent of document nesting depth. Keep
    // vector scopes iterative so a long mask chain cannot exhaust the native
    // or WebAssembly stack; bounded pixel-mask recursion lives in the cache.
    let mut vector_clips = Vec::new();
    while index < end {
        let node = &scene[index];
        if node.end > end {
            paint_partial(
                context,
                scene,
                index,
                end,
                viewport,
                scene_reused,
                stats,
                backdrops,
                roots,
            );
            break;
        }
        if let Some(kind) = node.mask {
            // Prefix reconstruction happens inside any already-open mask's
            // source. Reapplying that outer mask here would square its alpha.
            if backdrops.mask_is_open(index) {
                index = node.end;
                continue;
            }
            stats.visible_nodes += 1;
            stats.cached_nodes += usize::from(scene_reused || node.reused.replace(true));
            stats.missing_images += node.visual.missing_images;
            if node.end == end {
                break;
            }
            if node.opacity <= 0.0
                || node.local_subtree_bounds.width <= 0.0
                || node.local_subtree_bounds.height <= 0.0
                || !node.world_subtree_bounds.intersects(viewport)
            {
                stats.culled_nodes += end - node.end;
                break;
            }
            if kind == MaskType::Vector {
                let clip = context
                    .push_geometry_clip(node.vector_clip.as_ref().expect("prepared vector mask"));
                backdrops.push_mask_scope(index);
                vector_clips.push((index, clip));
                index = node.end;
                continue;
            } else {
                let transform = context.push_transform(ui_matrix(node.local));
                let painted = backdrops.try_paint_mask(
                    context,
                    scene,
                    roots,
                    index,
                    end,
                    scene_reused,
                    stats,
                );
                context.pop(transform);
                if !painted {
                    // A declined mask must never expose content outside its
                    // authored coverage. The bounded fallback reports failure.
                    stats.culled_nodes += end - node.end;
                }
            }
            break;
        }
        paint_subtree(
            context,
            scene,
            index,
            viewport,
            scene_reused,
            stats,
            backdrops,
            roots,
        );
        index = node.end;
    }
    for (index, clip) in vector_clips.into_iter().rev() {
        backdrops.pop_mask_scope(index);
        context.pop(clip);
    }
}

/// Replay only the open ancestor's source before `stop`. Its completed group
/// opacity and effects belong outside the current backdrop's isolation scope.
pub(super) fn paint_partial(
    context: &mut DrawingContext<'_>,
    scene: &[PreparedNode],
    index: usize,
    stop: usize,
    viewport: Rect,
    scene_reused: bool,
    stats: &mut RenderStats,
    backdrops: &mut backdrop::BackdropCache,
    roots: &[usize],
) {
    let node = &scene[index];
    let transform = context.push_transform(ui_matrix(node.local));
    paint_shadows(context, &node.visual, false);
    paint_body(context, &node.visual);
    let clip = push_content_clip(context, node);
    paint_siblings(
        context,
        scene,
        index + 1,
        stop,
        viewport,
        scene_reused,
        stats,
        backdrops,
        roots,
    );
    context.pop(clip);
    context.pop(transform);
}

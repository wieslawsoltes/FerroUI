//! What the render walk tells the brushes it paints: the visual that paints
//! them and the root of the render, which a backdrop brush draws again up to
//! that visual, and the XAML lights a brush is targeted by
//! (winui-materials.md, the backdrop).

use super::ServerCompositionVisual;
use crate::platform::{IDrawingContextImpl, LtrbRect};
use crate::{Matrix, Rect};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::sync::Arc;

/// How deep a backdrop may be drawn inside another backdrop: a backdrop of
/// a deeper level is empty.
pub const MAX_BACKDROP_DEPTH: usize = 2;

/// A render of a subtree that is in progress: its root and how it was
/// begun, which a backdrop render repeats.
#[derive(Clone)]
pub(crate) struct RenderRoot {
    pub(crate) visual: Rc<ServerCompositionVisual>,
    /// The transform of the canvas when the render began.
    pub(crate) transform: Matrix,
    /// The clip of the render, in device pixels.
    pub(crate) clip: LtrbRect,
    pub(crate) skip_root_visual_transform: bool,
    pub(crate) rendering_to_bitmap_cache: bool,
}

crate::ferro_thread_local! {
    /// The renders in progress on this (render) thread, innermost last.
    static RENDER_ROOTS: RefCell<Vec<RenderRoot>> = const { RefCell::new(Vec::new()) };
    /// The visuals whose own content is being drawn, innermost last.
    static PAINTING_VISUALS: RefCell<Vec<Rc<ServerCompositionVisual>>> = const { RefCell::new(Vec::new()) };
    /// The ids of the XAML lights that target the XAML brush being painted,
    /// innermost last.
    static BRUSH_LIGHT_IDS: RefCell<Vec<Arc<[String]>>> = const { RefCell::new(Vec::new()) };
    /// How many backdrops are being drawn inside each other.
    static BACKDROP_DEPTH: Cell<usize> = const { Cell::new(0) };
    /// The backdrops drawn since the last call of `take_backdrop_regions`:
    /// the visual that painted each and the area it covers, in device
    /// pixels.
    static BACKDROP_REGIONS: RefCell<Vec<(Weak<ServerCompositionVisual>, LtrbRect)>> = const { RefCell::new(Vec::new()) };
}

/// Runs `f` with `root` as the innermost render.
pub(crate) fn with_render_root<R>(root: RenderRoot, f: impl FnOnce() -> R) -> R {
    RENDER_ROOTS.with(|roots| roots.borrow_mut().push(root));
    let result = f();
    RENDER_ROOTS.with(|roots| roots.borrow_mut().pop());
    result
}

/// Runs `f` while the own content of `visual` is drawn.
pub(crate) fn with_painting_visual<R>(visual: &Rc<ServerCompositionVisual>, f: impl FnOnce() -> R) -> R {
    PAINTING_VISUALS.with(|visuals| visuals.borrow_mut().push(visual.clone()));
    let result = f();
    PAINTING_VISUALS.with(|visuals| visuals.borrow_mut().pop());
    result
}

/// The visual whose own content is being drawn.
pub fn painting_visual() -> Option<Rc<ServerCompositionVisual>> {
    PAINTING_VISUALS.try_with(|visuals| visuals.borrow().last().cloned()).ok().flatten()
}

/// Runs `f` while a XAML brush targeted by the lights `ids` is painted.
pub(crate) fn with_brush_light_ids<R>(ids: Arc<[String]>, f: impl FnOnce() -> R) -> R {
    BRUSH_LIGHT_IDS.with(|stack| stack.borrow_mut().push(ids));
    let result = f();
    BRUSH_LIGHT_IDS.with(|stack| stack.borrow_mut().pop());
    result
}

/// The ids of the XAML lights that target the XAML brush being painted
/// (empty outside a XAML brush).
pub fn brush_light_ids() -> Arc<[String]> {
    BRUSH_LIGHT_IDS
        .try_with(|stack| stack.borrow().last().cloned())
        .ok()
        .flatten()
        .unwrap_or_else(|| Arc::from(Vec::<String>::new()))
}

/// Whether `ancestor` is `visual` or one of its ancestors.
pub(crate) fn is_self_or_ancestor(ancestor: &ServerCompositionVisual, visual: &Rc<ServerCompositionVisual>) -> bool {
    let mut current = Some(visual.clone());
    let mut depth = 0;
    while let Some(candidate) = current {
        if std::ptr::eq(&*candidate, ancestor) {
            return true;
        }
        depth += 1;
        if depth > 1024 {
            return false;
        }
        current = candidate.parent();
    }
    false
}

/// The device rectangle (the axis-aligned bounds) of `rect` under
/// `transform`.
fn device_rect(rect: Rect, transform: Matrix) -> LtrbRect {
    let corners = [
        transform.transform(rect.top_left()),
        transform.transform(rect.top_right()),
        transform.transform(rect.bottom_left()),
        transform.transform(rect.bottom_right()),
    ];
    let (mut left, mut top, mut right, mut bottom) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for corner in corners {
        left = left.min(corner.x);
        top = top.min(corner.y);
        right = right.max(corner.x);
        bottom = bottom.max(corner.y);
    }
    LtrbRect::new(left.floor(), top.floor(), right.ceil(), bottom.ceil())
}

/// Draws the backdrop of the visual being painted into `rect` (in the
/// coordinates of `canvas`): the tree of the innermost render, from its
/// root, with the transform the render began with, cut to `rect`, up to
/// (and without) the visual. Returns whether anything was drawn.
///
/// Nothing is drawn outside a render, for a visual that is not in the tree
/// of the render, or deeper than [`MAX_BACKDROP_DEPTH`] backdrops.
pub fn draw_backdrop(canvas: &mut dyn IDrawingContextImpl, rect: Rect) -> bool {
    if rect.width <= 0.0 || rect.height <= 0.0 {
        return false;
    }
    let depth = BACKDROP_DEPTH.with(Cell::get);
    if depth >= MAX_BACKDROP_DEPTH {
        return false;
    }
    let Some(visual) = painting_visual() else { return false };
    let Some(root) = RENDER_ROOTS.with(|roots| roots.borrow().last().cloned()) else { return false };
    if !is_self_or_ancestor(&root.visual, &visual) || Rc::ptr_eq(&root.visual, &visual) {
        return false;
    }

    let saved = canvas.transform();
    let device = device_rect(rect, saved);
    BACKDROP_REGIONS.with(|regions| {
        let mut regions = regions.borrow_mut();
        regions.retain(|(existing, _)| existing.upgrade().is_some_and(|existing| !Rc::ptr_eq(&existing, &visual)));
        regions.push((Rc::downgrade(&visual), device));
    });

    canvas.push_clip(rect);
    canvas.set_transform(root.transform);
    BACKDROP_DEPTH.with(|value| value.set(depth + 1));
    let clip = root.clip.intersect_or_empty(device);
    root.visual.render_up_to(
        canvas,
        clip,
        &visual,
        root.skip_root_visual_transform,
        root.rendering_to_bitmap_cache,
    );
    BACKDROP_DEPTH.with(|value| value.set(depth));
    canvas.set_transform(saved);
    canvas.pop_clip();
    true
}

/// Takes the backdrops drawn since the last call: the visual that painted
/// each and its area in device pixels. A visual that painted several times
/// keeps its last area.
pub(crate) fn take_backdrop_regions() -> Vec<(Weak<ServerCompositionVisual>, LtrbRect)> {
    BACKDROP_REGIONS.try_with(|regions| std::mem::take(&mut *regions.borrow_mut())).unwrap_or_default()
}

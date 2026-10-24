//! The visual that draws the theme shadow of an element (phase 7, not from
//! upstream): the counterpart of the drop shadow visual of
//! `HWCompTreeNodeWinRT` in microsoft-ui-xaml
//! (`dxaml/xcp/components/comptree/HWCompNodeWinRT.cpp`: `NeedsDropShadowVisual`,
//! `UpdateDropShadowVisual`, `CreateDropShadowVisual`).
//!
//! WinUI draws the shadow of a caster on a sprite visual parented under the
//! prepend visual of the caster, so that it escapes the caster's own clips
//! but not those of its ancestors, with the caster's primary visual as its
//! `ParentForTransform` and the caster's opacity. Its brush is a nine-grid of
//! an image of the recipe: a rounded rectangle (one pixel smaller than the
//! caster on each side) with an ambient and a directional drop shadow, the
//! rectangle itself cut out. Here the visual is a custom visual inserted just
//! before the caster's own visual among the visuals of the caster's parent,
//! with the same transform parent and opacity; it draws the two shadows of
//! the recipe as the box shadows of that rounded rectangle, which the drawing
//! contexts draw outside the rectangle only: the same image, without the
//! nine-grid.

use super::{
    CompositionCustomVisual, CompositionCustomVisualHandler, CompositionVisual, ICompositionCustomVisualHandler,
    ICompositionObjectAnimations,
};
use crate::media::{BoxShadow, BoxShadows, DropShadowRecipe, ImmediateDrawingContext};
use crate::{Rect, RoundedRect, Vector};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;

/// What the visual draws: the box shadows of the rounded rectangle and its
/// radii.
#[derive(Clone, Debug, PartialEq)]
struct ShadowShape {
    shadows: Vec<BoxShadow>,
    radius_x: f64,
    radius_y: f64,
}

impl ShadowShape {
    fn of(recipe: &DropShadowRecipe) -> ShadowShape {
        ShadowShape { shadows: recipe.box_shadows(), radius_x: recipe.radius_x, radius_y: recipe.radius_y }
    }
}

/// The shadow visual of a caster: created by the caster's
/// [`Visual`](crate::Visual), shown by its parent before the caster's own
/// visual.
pub struct ThemeShadowVisual {
    visual: CompositionCustomVisual,
    /// The shape last sent to the handler.
    shape: RefCell<Option<ShadowShape>>,
    size: Cell<Vector>,
}

impl ThemeShadowVisual {
    /// Creates the shadow visual of `caster` (the composition visual of the
    /// caster): its transform is taken in the space of the caster
    /// (`ParentForTransform`), its opacity follows the caster's through an
    /// expression (as `UpdateDropShadowVisualOpacity` does for an animated
    /// opacity), and it is not hit-testable.
    ///
    /// The handler starts with the shape of `recipe`, so that the first frame that shows the
    /// visual draws the shadow (a message reaches the handler a frame later).
    pub fn new(caster: &Rc<CompositionVisual>, recipe: &DropShadowRecipe) -> ThemeShadowVisual {
        let compositor = caster.compositor().clone();
        let shape = ShadowShape::of(recipe);
        let initial = shape.clone();
        let visual = compositor.create_custom_visual(move || {
            Rc::new(ThemeShadowHandler { base: CompositionCustomVisualHandler::default(), shape: RefCell::new(Some(initial)) })
                as Rc<dyn ICompositionCustomVisualHandler>
        });
        visual.set_is_hit_test_visible(false);
        // The shadow lies outside the bounds of the caster.
        visual.set_clip_to_bounds(false);
        visual.set_parent_for_transform(Some(caster.clone()));
        let opacity = compositor.create_expression_animation_with("caster.Opacity");
        opacity.set_reference_parameter("caster", caster.clone());
        visual.start_animation("Opacity", &*opacity);
        ThemeShadowVisual { visual, shape: RefCell::new(Some(shape)), size: Cell::new(Vector::default()) }
    }

    /// The composition visual.
    pub fn visual(&self) -> &Rc<CompositionVisual> {
        &self.visual
    }

    /// Shows `recipe` around a caster of `size`, or nothing when `visible` is
    /// false or the recipe draws nothing.
    pub fn update(&self, recipe: &DropShadowRecipe, size: Vector, visible: bool) {
        let visible = visible && recipe.is_visible() && size.x > 0.0 && size.y > 0.0;
        self.visual.set_visible(visible);
        if self.size.get() != size {
            self.size.set(size);
            self.visual.set_size(size);
        }
        let shape = ShadowShape::of(recipe);
        if self.shape.borrow().as_ref() == Some(&shape) {
            return;
        }
        *self.shape.borrow_mut() = Some(shape.clone());
        self.visual.send_handler_message(Arc::new(shape));
    }

    /// The box shadows the visual draws (for tests).
    pub fn box_shadows(&self) -> Vec<BoxShadow> {
        self.shape.borrow().as_ref().map(|shape| shape.shadows.clone()).unwrap_or_default()
    }

    /// Releases the visual.
    pub fn dispose(&self) {
        self.visual.dispose();
    }
}

/// Draws the shape of the shadow on the server.
#[derive(Default)]
struct ThemeShadowHandler {
    base: CompositionCustomVisualHandler,
    shape: RefCell<Option<ShadowShape>>,
}

impl ThemeShadowHandler {
    /// The rounded rectangle the shadows are cast by: the caster one pixel
    /// smaller on each side, so that the antialiased edge of the caster lies
    /// over the shadow (`CreateDropShadowVisual`).
    fn caster_rect(&self) -> Option<RoundedRect> {
        let size = self.base.effective_size();
        if size.x <= 2.0 || size.y <= 2.0 {
            return None;
        }
        let shape = self.shape.borrow();
        let shape = shape.as_ref()?;
        let rect = Rect::new(1.0, 1.0, size.x - 2.0, size.y - 2.0);
        Some(RoundedRect::from_radius_xy(rect, shape.radius_x, shape.radius_y))
    }

    fn box_shadows(&self) -> Option<BoxShadows> {
        let shape = self.shape.borrow();
        let shadows = &shape.as_ref()?.shadows;
        let (first, rest) = shadows.split_first()?;
        Some(BoxShadows::with_rest(*first, rest))
    }
}

impl ICompositionCustomVisualHandler for ThemeShadowHandler {
    fn handler_base(&self) -> &CompositionCustomVisualHandler {
        &self.base
    }

    fn on_message(&self, message: Arc<dyn Any + Send + Sync>) {
        if let Some(shape) = message.downcast_ref::<ShadowShape>() {
            *self.shape.borrow_mut() = Some(shape.clone());
            self.base.invalidate();
        }
    }

    fn on_render(&self, drawing_context: &mut ImmediateDrawingContext<'_>) {
        let (Some(rect), Some(shadows)) = (self.caster_rect(), self.box_shadows()) else { return };
        drawing_context.platform_impl().draw_rectangle(None, None, rect, &shadows);
    }

    /// The caster with the reach of its shadows.
    fn get_render_bounds(&self) -> Rect {
        let size = self.base.effective_size();
        let caster = Rect::new(0.0, 0.0, size.x, size.y);
        match (self.caster_rect(), self.box_shadows()) {
            (Some(rect), Some(shadows)) => caster.union(shadows.transform_bounds(rect.rect)),
            _ => caster,
        }
    }
}

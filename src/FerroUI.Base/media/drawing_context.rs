//! The drawing context: the surface a visual draws itself onto.
//!
//! # Shape
//!
//! Upstream this is an abstract class: public drawing members validate
//! their arguments and forward to protected abstract `*Core` members, which
//! the platform context and the recording (render data) context implement;
//! pushed states are RAII tokens kept on a per-context stack.
//!
//! Here the abstract part is the [`IDrawingContextCore`] trait and
//! [`DrawingContext`] is one concrete struct over a `dyn` core:
//!
//! * `Visual::render` receives `&mut DrawingContext`, a nameable type that
//!   does not depend on which core is behind it.
//! * The core is either borrowed ([`DrawingContext::new`]) or owned
//!   ([`DrawingContext::owned`]). A borrowed core lets the caller keep the
//!   concrete core (a recording context is reused for many visuals and
//!   queried for its result after each one); an owned core is disposed with
//!   the context, like `using var context = ...` upstream.
//! * A push returns a [`PushedState`] token, a plain `Copy` value recording
//!   the stack level. It is popped explicitly with [`DrawingContext::pop`]
//!   (C# `state.Dispose()`), which checks the order exactly as upstream.
//!   [`PushedState::NONE`] is C#'s `default(PushedState)`: popping it does
//!   nothing, which makes conditional pushes straightforward. For lexical
//!   scoping there is [`DrawingContext::scope`], a guard that derefs to the
//!   context and pops on drop.
//! * Text (formatted text, text layouts, lines and runs) draws through
//!   `ITextDrawingSink`, whose members are the drawing context members text
//!   needs. The context implements the sink, so text is drawn by passing
//!   the context itself; the sink's unpaired `pop_transform` pops the state
//!   pushed last.
//! * No draw call allocates: brushes and pens are passed as borrowed `Rc`
//!   handles (the platform core derefs them, the recording core clones the
//!   handle), and the state stack is taken from a per-thread pool.

use crate::media::immutable::ImmutableSolidColorBrush;
use crate::media::text_formatting::ITextDrawingSink;
use crate::media::IEffect;
use crate::media::{
    BoxShadows, FormattedText, Geometry, GlyphRun, IBrush, IExperimentalAcrylicMaterial, IImage, IPen, RenderOptions,
    TextOptions,
};
use crate::platform::{IBitmapImpl, IGeometryImpl};
use crate::rendering::scene_graph::ICustomDrawOperation;
use crate::utilities::MathUtilities;
use crate::{Matrix, Point, Rect, Ref, RoundedRect};
use std::cell::RefCell;
use std::ops::{Deref, DerefMut};
use std::rc::Rc;
use std::sync::Arc;

/// The overridable part of a drawing context: the `*Core` members.
///
/// Implemented by the platform drawing context and by the recording
/// context. The arguments have been validated by [`DrawingContext`].
pub trait IDrawingContextCore {
    /// Draws a line. The pen is visible.
    fn draw_line_core(&mut self, pen: &Rc<dyn IPen>, p1: Point, p2: Point);

    /// Draws a geometry object. The default implementation draws its
    /// platform implementation, if it has one.
    fn draw_geometry_core(
        &mut self,
        brush: Option<&Rc<dyn IBrush>>,
        pen: Option<&Rc<dyn IPen>>,
        geometry: &Ref<Geometry>,
    ) {
        if let Some(geometry_impl) = geometry.platform_impl() {
            self.draw_geometry_impl_core(brush, pen, &geometry_impl);
        }
    }

    /// Draws a platform geometry.
    fn draw_geometry_impl_core(
        &mut self,
        brush: Option<&Rc<dyn IBrush>>,
        pen: Option<&Rc<dyn IPen>>,
        geometry: &Arc<dyn IGeometryImpl>,
    );

    /// Draws a rectangle with the specified brush and pen.
    fn draw_rectangle_core(
        &mut self,
        brush: Option<&Rc<dyn IBrush>>,
        pen: Option<&Rc<dyn IPen>>,
        rrect: RoundedRect,
        box_shadows: &BoxShadows,
    );

    /// Draws an ellipse with the specified brush and pen.
    fn draw_ellipse_core(&mut self, brush: Option<&Rc<dyn IBrush>>, pen: Option<&Rc<dyn IPen>>, rect: Rect);

    /// Draws a platform bitmap.
    fn draw_bitmap(&mut self, source: &std::sync::Arc<crate::platform::SharedBitmapImpl>, opacity: f64, source_rect: Rect, dest_rect: Rect);

    /// Draws a custom drawing operation.
    fn custom(&mut self, custom: &Rc<dyn ICustomDrawOperation>);

    /// Draws a glyph run.
    fn draw_glyph_run(&mut self, foreground: Option<&Rc<dyn IBrush>>, glyph_run: &Rc<GlyphRun>);

    /// Draws a rectangle filled with an acrylic-like material.
    ///
    /// Upstream only the platform drawing context has this member and
    /// callers cast the context to reach it. Here it is a core member so
    /// that it is reachable through [`DrawingContext`]: the platform core
    /// uses the backend's acrylic support, every other core gets the
    /// rectangle filled with the fallback color of the material.
    fn draw_rectangle_acrylic_core(&mut self, material: &dyn IExperimentalAcrylicMaterial, rect: RoundedRect) {
        let brush: Rc<dyn IBrush> = Rc::new(ImmutableSolidColorBrush::new(material.fallback_color()));
        self.draw_rectangle_core(Some(&brush), None, rect, &BoxShadows::default());
    }

    fn push_clip_core(&mut self, rect: Rect);
    fn push_rounded_clip_core(&mut self, rect: RoundedRect);
    fn push_geometry_clip_core(&mut self, clip: &Ref<Geometry>);
    fn push_opacity_core(&mut self, opacity: f64);
    fn push_opacity_mask_core(&mut self, mask: &Rc<dyn IBrush>, bounds: Rect);
    fn push_transform_core(&mut self, matrix: Matrix);
    fn push_render_options_core(&mut self, render_options: RenderOptions);
    fn push_text_options_core(&mut self, text_options: TextOptions);
    fn push_effect_core(&mut self, effect: &Rc<dyn IEffect>, bounds: Rect);

    fn pop_clip_core(&mut self);
    fn pop_geometry_clip_core(&mut self);
    fn pop_opacity_core(&mut self);
    fn pop_opacity_mask_core(&mut self);
    fn pop_transform_core(&mut self);
    fn pop_render_options_core(&mut self);
    fn pop_text_options_core(&mut self);
    fn pop_effect_core(&mut self);

    /// Releases the core. Called once, after every pushed state has been
    /// popped, when an owned context is disposed.
    fn dispose_core(&mut self);
}

/// The kind of a pushed state; decides which `pop_*_core` restores it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PushedStateType {
    Transform,
    Opacity,
    Clip,
    GeometryClip,
    OpacityMask,
    RenderOptions,
    TextOptions,
    Effect,
}

/// A token for a pushed state. Pop it with [`DrawingContext::pop`], in the
/// reverse order of the pushes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[must_use = "a pushed state must be popped with `DrawingContext::pop` (or left to the context's disposal)"]
pub struct PushedState {
    /// The state stack depth right after the push; zero for [`Self::NONE`].
    level: usize,
}

impl PushedState {
    /// The state that represents "nothing was pushed"; popping it does
    /// nothing.
    pub const NONE: PushedState = PushedState { level: 0 };
}

impl Default for PushedState {
    fn default() -> Self {
        Self::NONE
    }
}

enum CoreRef<'a> {
    Borrowed(&'a mut dyn IDrawingContextCore),
    Owned(Box<dyn IDrawingContextCore + 'a>),
}

thread_local! {
    static STATE_STACK_POOL: RefCell<Vec<Vec<PushedStateType>>> = const { RefCell::new(Vec::new()) };
}

const STATE_STACK_POOL_LIMIT: usize = 16;

/// The surface a visual draws itself onto during rendering.
pub struct DrawingContext<'a> {
    core: CoreRef<'a>,
    states: Option<Vec<PushedStateType>>,
    disposed: bool,
}

impl<'a> DrawingContext<'a> {
    /// Creates a context over a borrowed core. Dropping (or disposing) the
    /// context pops the states still pushed but leaves the core alive.
    pub fn new(core: &'a mut dyn IDrawingContextCore) -> Self {
        Self { core: CoreRef::Borrowed(core), states: None, disposed: false }
    }

    /// Creates a context that owns its core: disposing (or dropping) the
    /// context pops the states still pushed and disposes the core.
    pub fn owned(core: Box<dyn IDrawingContextCore + 'a>) -> Self {
        Self { core: CoreRef::Owned(core), states: None, disposed: false }
    }

    #[inline]
    fn core(&mut self) -> &mut dyn IDrawingContextCore {
        match &mut self.core {
            CoreRef::Borrowed(core) => &mut **core,
            CoreRef::Owned(core) => &mut **core,
        }
    }

    /// Pops every state still pushed and releases the context (and the core,
    /// if the context owns it).
    pub fn dispose(mut self) {
        self.dispose_in_place();
    }

    fn dispose_in_place(&mut self) {
        if self.disposed {
            return;
        }
        self.disposed = true;
        if let Some(mut states) = self.states.take() {
            while let Some(state) = states.pop() {
                self.restore(state);
            }
            STATE_STACK_POOL.with(|pool| {
                let mut pool = pool.borrow_mut();
                if pool.len() < STATE_STACK_POOL_LIMIT {
                    pool.push(states);
                }
            });
        }
        if let CoreRef::Owned(core) = &mut self.core {
            core.dispose_core();
        }
    }

    /// Draws an image into `rect`.
    pub fn draw_image(&mut self, source: &dyn IImage, rect: Rect) {
        self.draw_image_with_rects(source, Rect::from_size(source.size()), rect);
    }

    /// Draws the part of an image inside `source_rect` into `dest_rect`.
    pub fn draw_image_with_rects(&mut self, source: &dyn IImage, source_rect: Rect, dest_rect: Rect) {
        source.draw(self, source_rect, dest_rect);
    }

    /// Draws a platform bitmap.
    pub fn draw_bitmap(&mut self, source: &std::sync::Arc<crate::platform::SharedBitmapImpl>, opacity: f64, source_rect: Rect, dest_rect: Rect) {
        self.core().draw_bitmap(source, opacity, source_rect, dest_rect);
    }

    /// Draws a line.
    pub fn draw_line(&mut self, pen: &Rc<dyn IPen>, p1: Point, p2: Point) {
        if pen_is_visible(Some(pen)) {
            self.core().draw_line_core(pen, p1, p2);
        }
    }

    /// Draws a geometry.
    pub fn draw_geometry(
        &mut self,
        brush: Option<&Rc<dyn IBrush>>,
        pen: Option<&Rc<dyn IPen>>,
        geometry: &Ref<Geometry>,
    ) {
        if brush.is_some() || pen_is_visible(pen) {
            self.core().draw_geometry_core(brush, pen, geometry);
        }
    }

    /// Draws a platform geometry.
    pub fn draw_geometry_impl(
        &mut self,
        brush: Option<&Rc<dyn IBrush>>,
        pen: Option<&Rc<dyn IPen>>,
        geometry: &Arc<dyn IGeometryImpl>,
    ) {
        if brush.is_some() || pen_is_visible(pen) {
            self.core().draw_geometry_impl_core(brush, pen, geometry);
        }
    }

    /// Draws a rectangle with the specified brush and pen.
    ///
    /// The brush and the pen can both be `None`. If the brush is `None`, no
    /// fill is performed. If the pen is `None`, no stroke is performed. If
    /// both are `None` and there are no box shadows, the call is a no-op.
    pub fn draw_rectangle(
        &mut self,
        brush: Option<&Rc<dyn IBrush>>,
        pen: Option<&Rc<dyn IPen>>,
        rect: Rect,
        mut radius_x: f64,
        mut radius_y: f64,
        box_shadows: &BoxShadows,
    ) {
        if brush.is_none() && !pen_is_visible(pen) && box_shadows.count() == 0 {
            return;
        }
        if !MathUtilities::is_zero(radius_x) {
            radius_x = radius_x.min(rect.width / 2.0);
        }
        if !MathUtilities::is_zero(radius_y) {
            radius_y = radius_y.min(rect.height / 2.0);
        }
        self.core().draw_rectangle_core(brush, pen, RoundedRect::from_radius_xy(rect, radius_x, radius_y), box_shadows);
    }

    /// Draws a rounded rectangle with the specified brush and pen.
    pub fn draw_rounded_rectangle(
        &mut self,
        brush: Option<&Rc<dyn IBrush>>,
        pen: Option<&Rc<dyn IPen>>,
        rrect: RoundedRect,
        box_shadows: &BoxShadows,
    ) {
        if brush.is_none() && !pen_is_visible(pen) && box_shadows.count() == 0 {
            return;
        }
        self.core().draw_rectangle_core(brush, pen, rrect, box_shadows);
    }

    /// Draws the outline of a rectangle.
    pub fn draw_rectangle_outline(&mut self, pen: &Rc<dyn IPen>, rect: Rect, corner_radius: f32) {
        let radius = f64::from(corner_radius);
        self.draw_rectangle(None, Some(pen), rect, radius, radius, &BoxShadows::default());
    }

    /// Draws a filled rectangle.
    pub fn fill_rectangle(&mut self, brush: &Rc<dyn IBrush>, rect: Rect, corner_radius: f32) {
        let radius = f64::from(corner_radius);
        self.draw_rectangle(Some(brush), None, rect, radius, radius, &BoxShadows::default());
    }

    /// Draws an ellipse given its center and radii.
    pub fn draw_ellipse_at(
        &mut self,
        brush: Option<&Rc<dyn IBrush>>,
        pen: Option<&Rc<dyn IPen>>,
        center: Point,
        radius_x: f64,
        radius_y: f64,
    ) {
        if brush.is_some() || pen_is_visible(pen) {
            let origin_x = center.x - radius_x;
            let origin_y = center.y - radius_y;
            let width = radius_x * 2.0;
            let height = radius_y * 2.0;
            self.core().draw_ellipse_core(brush, pen, Rect::new(origin_x, origin_y, width, height));
        }
    }

    /// Draws an ellipse inscribed in a rectangle.
    pub fn draw_ellipse(&mut self, brush: Option<&Rc<dyn IBrush>>, pen: Option<&Rc<dyn IPen>>, rect: Rect) {
        if brush.is_some() || pen_is_visible(pen) {
            self.core().draw_ellipse_core(brush, pen, rect);
        }
    }

    /// Draws a custom drawing operation.
    pub fn custom(&mut self, custom: &Rc<dyn ICustomDrawOperation>) {
        self.core().custom(custom);
    }

    /// Draws text. `origin` is the upper-left corner of the text.
    pub fn draw_text(&mut self, text: &FormattedText, origin: Point) {
        text.draw(self, origin);
    }

    /// Draws a glyph run.
    pub fn draw_glyph_run(&mut self, foreground: Option<&Rc<dyn IBrush>>, glyph_run: &Rc<GlyphRun>) {
        self.core().draw_glyph_run(foreground, glyph_run);
    }

    /// Draws a rectangle filled with an acrylic-like material. A core
    /// without acrylic support fills the rectangle with the fallback color
    /// of the material; see
    /// [`IDrawingContextCore::draw_rectangle_acrylic_core`].
    pub fn draw_rectangle_acrylic(&mut self, material: &dyn IExperimentalAcrylicMaterial, rect: RoundedRect) {
        self.core().draw_rectangle_acrylic_core(material, rect);
    }

    // --- pushed states ----------------------------------------------------

    fn pushed(&mut self, state: PushedStateType) -> PushedState {
        let states = self
            .states
            .get_or_insert_with(|| STATE_STACK_POOL.with(|pool| pool.borrow_mut().pop()).unwrap_or_default());
        states.push(state);
        PushedState { level: states.len() }
    }

    fn restore(&mut self, state: PushedStateType) {
        let core = self.core();
        match state {
            PushedStateType::Transform => core.pop_transform_core(),
            PushedStateType::Clip => core.pop_clip_core(),
            PushedStateType::Opacity => core.pop_opacity_core(),
            PushedStateType::GeometryClip => core.pop_geometry_clip_core(),
            PushedStateType::OpacityMask => core.pop_opacity_mask_core(),
            PushedStateType::RenderOptions => core.pop_render_options_core(),
            PushedStateType::TextOptions => core.pop_text_options_core(),
            PushedStateType::Effect => core.pop_effect_core(),
        }
    }

    /// Pops a pushed state (C# `PushedState.Dispose`).
    ///
    /// Panics if the state is not the most recently pushed one. Popping
    /// [`PushedState::NONE`], or any state after the context has been
    /// disposed, does nothing.
    pub fn pop(&mut self, state: PushedState) {
        if state.level == 0 {
            return;
        }
        let Some(states) = self.states.as_mut() else { return };
        if states.len() != state.level {
            panic!("Wrong Push/Pop state order");
        }
        let restored = states.pop().expect("level is non-zero");
        self.restore(restored);
    }

    /// Turns a pushed state into a guard that derefs to the context and pops
    /// the state when dropped.
    pub fn scope(&mut self, state: PushedState) -> PushedScope<'_, 'a> {
        PushedScope { context: self, state }
    }

    /// Pushes a rounded clip rectangle.
    pub fn push_rounded_clip(&mut self, clip: RoundedRect) -> PushedState {
        self.core().push_rounded_clip_core(clip);
        self.pushed(PushedStateType::Clip)
    }

    /// Pushes a clip rectangle.
    pub fn push_clip(&mut self, clip: Rect) -> PushedState {
        self.core().push_clip_core(clip);
        self.pushed(PushedStateType::Clip)
    }

    /// Pushes a clip geometry.
    pub fn push_geometry_clip(&mut self, clip: &Ref<Geometry>) -> PushedState {
        self.core().push_geometry_clip_core(clip);
        self.pushed(PushedStateType::GeometryClip)
    }

    /// Pushes an opacity value.
    pub fn push_opacity(&mut self, opacity: f64) -> PushedState {
        self.core().push_opacity_core(opacity);
        self.pushed(PushedStateType::Opacity)
    }

    /// Pushes an opacity mask.
    ///
    /// `bounds` are the bounds of the control, used to place brushes
    /// correctly.
    pub fn push_opacity_mask(&mut self, mask: &Rc<dyn IBrush>, bounds: Rect) -> PushedState {
        self.core().push_opacity_mask_core(mask, bounds);
        self.pushed(PushedStateType::OpacityMask)
    }

    /// Pushes a matrix transformation.
    pub fn push_transform(&mut self, matrix: Matrix) -> PushedState {
        self.core().push_transform_core(matrix);
        self.pushed(PushedStateType::Transform)
    }

    /// Pushes render options.
    pub fn push_render_options(&mut self, render_options: RenderOptions) -> PushedState {
        self.core().push_render_options_core(render_options);
        self.pushed(PushedStateType::RenderOptions)
    }

    /// Pushes text options.
    pub fn push_text_options(&mut self, text_options: TextOptions) -> PushedState {
        self.core().push_text_options_core(text_options);
        self.pushed(PushedStateType::TextOptions)
    }

    /// Pushes an effect applied to everything drawn until the state is
    /// popped. `bounds` are the bounds of the affected content.
    pub fn push_effect(&mut self, effect: &Rc<dyn IEffect>, bounds: Rect) -> PushedState {
        self.core().push_effect_core(effect, bounds);
        self.pushed(PushedStateType::Effect)
    }
}

/// Text draws itself through the sink members, which are the corresponding
/// drawing context members. A sink push has no state token: its pop restores
/// the state pushed last, which is the transform the matching push pushed
/// because text balances its pushes.
impl ITextDrawingSink for DrawingContext<'_> {
    fn draw_glyph_run(&mut self, foreground: Option<&Rc<dyn IBrush>>, glyph_run: &Rc<GlyphRun>) {
        DrawingContext::draw_glyph_run(self, foreground, glyph_run);
    }

    fn draw_rectangle(&mut self, brush: Option<&Rc<dyn IBrush>>, pen: Option<&Rc<dyn IPen>>, rect: Rect) {
        DrawingContext::draw_rectangle(self, brush, pen, rect, 0.0, 0.0, &BoxShadows::default());
    }

    fn draw_line(&mut self, pen: &Rc<dyn IPen>, p1: Point, p2: Point) {
        DrawingContext::draw_line(self, pen, p1, p2);
    }

    fn push_transform(&mut self, matrix: Matrix) {
        let _ = DrawingContext::push_transform(self, matrix);
    }

    fn pop_transform(&mut self) {
        let Some(states) = self.states.as_mut() else { return };
        match states.pop() {
            Some(PushedStateType::Transform) => self.restore(PushedStateType::Transform),
            Some(_) => panic!("Wrong Push/Pop state order"),
            None => {}
        }
    }
}

impl Drop for DrawingContext<'_> {
    fn drop(&mut self) {
        // The equivalent of leaving a `using` block. When unwinding, the
        // core is left alone: popping could panic again.
        if !std::thread::panicking() {
            self.dispose_in_place();
        }
    }
}

/// A pushed state that is popped when the guard goes out of scope. Derefs
/// to the drawing context.
pub struct PushedScope<'c, 'a> {
    context: &'c mut DrawingContext<'a>,
    state: PushedState,
}

impl<'a> Deref for PushedScope<'_, 'a> {
    type Target = DrawingContext<'a>;

    fn deref(&self) -> &Self::Target {
        self.context
    }
}

impl DerefMut for PushedScope<'_, '_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.context
    }
}

impl Drop for PushedScope<'_, '_> {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            self.context.pop(self.state);
        }
    }
}

/// Whether a pen draws anything: it has a brush and a positive thickness.
pub(crate) fn pen_is_visible(pen: Option<&Rc<dyn IPen>>) -> bool {
    pen.is_some_and(|pen| pen.brush().is_some() && pen.thickness() > 0.0)
}

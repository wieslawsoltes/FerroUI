use super::platform_drawing_context::ImplRef;
use crate::media::imaging::Bitmap;
use crate::media::immutable::ImmutablePen;
use crate::media::{BoxShadows, IBrush, IImmutableBrush, IImmutableGlyphRunReference, IPen};
use crate::platform::{IBitmapImpl, IDrawingContextImpl};
use crate::utilities::MathUtilities;
use crate::{Matrix, Point, Rect, RoundedRect};
use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone, Copy)]
struct TransformContainer {
    local_transform: Matrix,
    container_transform: Matrix,
}

#[derive(Clone, Copy)]
enum PushedStateType {
    Matrix(Matrix),
    Opacity,
    Clip,
    MatrixContainer,
    #[allow(dead_code)]
    GeometryClip,
    OpacityMask,
}

/// A token for a state pushed on an [`ImmediateDrawingContext`]. Pop it with
/// [`ImmediateDrawingContext::pop`], in the reverse order of the pushes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[must_use = "a pushed state must be popped with `ImmediateDrawingContext::pop`"]
pub struct ImmediatePushedState {
    level: usize,
}

impl ImmediatePushedState {
    /// The state that represents "nothing was pushed"; popping it does
    /// nothing.
    pub const NONE: ImmediatePushedState = ImmediatePushedState { level: 0 };
}

thread_local! {
    static STATE_STACK_POOL: RefCell<Vec<Vec<PushedStateType>>> = const { RefCell::new(Vec::new()) };
    static TRANSFORM_STACK_POOL: RefCell<Vec<Vec<TransformContainer>>> = const { RefCell::new(Vec::new()) };
}

const STACK_POOL_LIMIT: usize = 16;

/// A drawing context that draws immediately to a platform drawing context
/// implementation, using immutable resources only. This is what custom draw
/// operations render with.
pub struct ImmediateDrawingContext<'a> {
    platform_impl: ImplRef<'a>,
    states: Option<Vec<PushedStateType>>,
    transform_containers: Option<Vec<TransformContainer>>,
    current_transform: Matrix,
    current_container_transform: Matrix,
}

impl<'a> ImmediateDrawingContext<'a> {
    /// Creates a context that owns `platform_impl` and disposes it with the
    /// context.
    pub fn new(platform_impl: Box<dyn IDrawingContextImpl + 'a>) -> Self {
        let transform = platform_impl.transform();
        Self::create(ImplRef::Owned(platform_impl), transform)
    }

    /// Creates a context over a borrowed implementation.
    pub fn borrowed(platform_impl: &'a mut dyn IDrawingContextImpl) -> Self {
        let transform = platform_impl.transform();
        Self::create(ImplRef::Borrowed(platform_impl), transform)
    }

    /// Creates a context over a borrowed implementation with an explicit
    /// container transform.
    pub fn borrowed_with_transform(platform_impl: &'a mut dyn IDrawingContextImpl, transform: Matrix) -> Self {
        Self::create(ImplRef::Borrowed(platform_impl), transform)
    }

    fn create(platform_impl: ImplRef<'a>, transform: Matrix) -> Self {
        Self {
            platform_impl,
            states: Some(STATE_STACK_POOL.with(|pool| pool.borrow_mut().pop()).unwrap_or_default()),
            transform_containers: Some(TRANSFORM_STACK_POOL.with(|pool| pool.borrow_mut().pop()).unwrap_or_default()),
            current_transform: Matrix::IDENTITY,
            current_container_transform: transform,
        }
    }

    /// The platform drawing context implementation.
    pub fn platform_impl(&mut self) -> &mut (dyn IDrawingContextImpl + 'a) {
        self.platform_impl.get()
    }

    /// The current transform of the drawing context.
    pub fn current_transform(&self) -> Matrix {
        self.current_transform
    }

    fn set_current_transform(&mut self, value: Matrix) {
        self.current_transform = value;
        let transform = self.current_transform * self.current_container_transform;
        self.platform_impl.get().set_transform(transform);
    }

    /// Draws a bitmap into `rect`.
    pub fn draw_bitmap(&mut self, source: &Bitmap, rect: Rect) {
        self.draw_bitmap_rects(source, Rect::from_size(source.size()), rect);
    }

    /// Draws the part of a bitmap inside `source_rect` into `dest_rect`.
    pub fn draw_bitmap_rects(&mut self, source: &Bitmap, source_rect: Rect, dest_rect: Rect) {
        self.platform_impl.get().draw_bitmap(&*source.platform_impl().item(), 1.0, source_rect, dest_rect);
    }

    /// Draws a platform bitmap: [`draw_bitmap_rects`](Self::draw_bitmap_rects)
    /// for callers that hold the platform implementation only.
    pub fn draw_bitmap_impl(&mut self, source: &dyn IBitmapImpl, source_rect: Rect, dest_rect: Rect) {
        self.platform_impl.get().draw_bitmap(source, 1.0, source_rect, dest_rect);
    }

    /// Draws a line.
    pub fn draw_line(&mut self, pen: &ImmutablePen, p1: Point, p2: Point) {
        if pen_is_visible(Some(pen)) {
            self.platform_impl.get().draw_line(Some(pen), p1, p2);
        }
    }

    /// Draws a rectangle with the specified brush and pen.
    pub fn draw_rectangle(
        &mut self,
        brush: Option<&dyn IImmutableBrush>,
        pen: Option<&ImmutablePen>,
        rect: Rect,
        mut radius_x: f64,
        mut radius_y: f64,
        box_shadows: &BoxShadows,
    ) {
        if brush.is_none() && !pen_is_visible(pen) {
            return;
        }
        if !MathUtilities::is_zero(radius_x) {
            radius_x = radius_x.min(rect.width / 2.0);
        }
        if !MathUtilities::is_zero(radius_y) {
            radius_y = radius_y.min(rect.height / 2.0);
        }
        self.platform_impl.get().draw_rectangle(
            brush.map(|b| b as &dyn IBrush),
            pen.map(|p| p as &dyn IPen),
            RoundedRect::from_radius_xy(rect, radius_x, radius_y),
            box_shadows,
        );
    }

    /// Draws the outline of a rectangle.
    pub fn draw_rectangle_outline(&mut self, pen: &ImmutablePen, rect: Rect, corner_radius: f32) {
        let radius = f64::from(corner_radius);
        self.draw_rectangle(None, Some(pen), rect, radius, radius, &BoxShadows::default());
    }

    /// Draws an ellipse with the specified brush and pen.
    pub fn draw_ellipse(
        &mut self,
        brush: Option<&dyn IImmutableBrush>,
        pen: Option<&ImmutablePen>,
        center: Point,
        radius_x: f64,
        radius_y: f64,
    ) {
        if brush.is_none() && !pen_is_visible(pen) {
            return;
        }
        let origin_x = center.x - radius_x;
        let origin_y = center.y - radius_y;
        let width = radius_x * 2.0;
        let height = radius_y * 2.0;
        self.platform_impl.get().draw_ellipse(
            brush.map(|b| b as &dyn IBrush),
            pen.map(|p| p as &dyn IPen),
            Rect::new(origin_x, origin_y, width, height),
        );
    }

    /// Draws a glyph run, if the reference still holds its platform glyph
    /// run.
    pub fn draw_glyph_run(&mut self, foreground: &dyn IImmutableBrush, glyph_run: &dyn IImmutableGlyphRunReference) {
        if let Some(glyph_run) = glyph_run.glyph_run() {
            self.platform_impl.get().draw_glyph_run(Some(foreground as &dyn IBrush), &*glyph_run);
        }
    }

    /// Draws a filled rectangle.
    pub fn fill_rectangle(&mut self, brush: &dyn IImmutableBrush, rect: Rect, corner_radius: f32) {
        let radius = f64::from(corner_radius);
        self.draw_rectangle(Some(brush), None, rect, radius, radius, &BoxShadows::default());
    }

    fn pushed(&mut self, state: PushedStateType) -> ImmediatePushedState {
        let Some(states) = self.states.as_mut() else {
            panic!("the drawing context has been disposed");
        };
        states.push(state);
        ImmediatePushedState { level: states.len() }
    }

    /// Pops a pushed state (C# `PushedState.Dispose`). Panics if the state
    /// is not the most recently pushed one.
    pub fn pop(&mut self, state: ImmediatePushedState) {
        if state.level == 0 {
            return;
        }
        let (Some(states), Some(_)) = (self.states.as_mut(), self.transform_containers.as_ref()) else {
            panic!("the drawing context has been disposed");
        };
        if states.len() != state.level {
            panic!("Wrong Push/Pop state order");
        }
        let restored = states.pop().expect("level is non-zero");
        self.restore(restored);
    }

    fn restore(&mut self, state: PushedStateType) {
        match state {
            PushedStateType::Matrix(matrix) => self.set_current_transform(matrix),
            PushedStateType::Clip => self.platform_impl.get().pop_clip(),
            PushedStateType::Opacity => self.platform_impl.get().pop_opacity(),
            PushedStateType::GeometryClip => self.platform_impl.get().pop_geometry_clip(),
            PushedStateType::OpacityMask => self.platform_impl.get().pop_opacity_mask(),
            PushedStateType::MatrixContainer => {
                let container = self
                    .transform_containers
                    .as_mut()
                    .and_then(Vec::pop)
                    .expect("a transform container was pushed");
                self.current_container_transform = container.container_transform;
                self.set_current_transform(container.local_transform);
            }
        }
    }

    /// Pushes a rounded clip rectangle.
    pub fn push_rounded_clip(&mut self, clip: RoundedRect) -> ImmediatePushedState {
        self.platform_impl.get().push_clip_rounded(clip);
        self.pushed(PushedStateType::Clip)
    }

    /// Pushes a clip rectangle.
    pub fn push_clip(&mut self, clip: Rect) -> ImmediatePushedState {
        self.platform_impl.get().push_clip(clip);
        self.pushed(PushedStateType::Clip)
    }

    /// Pushes an opacity value.
    pub fn push_opacity(&mut self, opacity: f64, bounds: Rect) -> ImmediatePushedState {
        self.platform_impl.get().push_opacity(opacity, Some(bounds));
        self.pushed(PushedStateType::Opacity)
    }

    /// Pushes an opacity mask.
    pub fn push_opacity_mask(&mut self, mask: &dyn IImmutableBrush, bounds: Rect) -> ImmediatePushedState {
        self.platform_impl.get().push_opacity_mask(mask as &dyn IBrush, bounds);
        self.pushed(PushedStateType::OpacityMask)
    }

    /// Pushes a matrix post-transformation.
    pub fn push_post_transform(&mut self, matrix: Matrix) -> ImmediatePushedState {
        self.push_set_transform(self.current_transform * matrix)
    }

    /// Pushes a matrix pre-transformation.
    pub fn push_pre_transform(&mut self, matrix: Matrix) -> ImmediatePushedState {
        self.push_set_transform(matrix * self.current_transform)
    }

    /// Sets the current matrix transformation.
    pub fn push_set_transform(&mut self, matrix: Matrix) -> ImmediatePushedState {
        let old_matrix = self.current_transform;
        self.set_current_transform(matrix);
        self.pushed(PushedStateType::Matrix(old_matrix))
    }

    /// Pushes a new transform context.
    pub fn push_transform_container(&mut self) -> ImmediatePushedState {
        let Some(containers) = self.transform_containers.as_mut() else {
            panic!("the drawing context has been disposed");
        };
        containers.push(TransformContainer {
            local_transform: self.current_transform,
            container_transform: self.current_container_transform,
        });
        self.current_container_transform = self.current_transform * self.current_container_transform;
        self.current_transform = Matrix::IDENTITY;
        self.pushed(PushedStateType::MatrixContainer)
    }

    /// Pops every state still pushed and releases the context (and the
    /// platform implementation, if the context owns it).
    pub fn dispose(mut self) {
        self.dispose_in_place();
    }

    fn dispose_in_place(&mut self) {
        let (Some(mut states), Some(containers)) = (self.states.take(), self.transform_containers.take()) else {
            return;
        };
        // Restoring needs the container stack in place.
        self.transform_containers = Some(containers);
        while let Some(state) = states.pop() {
            self.restore(state);
        }
        let containers = self.transform_containers.take().expect("just set");
        STATE_STACK_POOL.with(|pool| {
            let mut pool = pool.borrow_mut();
            if pool.len() < STACK_POOL_LIMIT {
                pool.push(states);
            }
        });
        if !containers.is_empty() {
            panic!("Transform container stack is non-empty");
        }
        TRANSFORM_STACK_POOL.with(|pool| {
            let mut pool = pool.borrow_mut();
            if pool.len() < STACK_POOL_LIMIT {
                pool.push(containers);
            }
        });
        if self.platform_impl.is_owned() {
            self.platform_impl.get().dispose();
        }
    }

    /// Queries the platform drawing context implementation for an optional
    /// feature.
    pub fn try_get_feature(&mut self, feature_type: TypeId) -> Option<Rc<dyn Any>> {
        self.platform_impl.get().get_feature(feature_type)
    }
}

impl Drop for ImmediateDrawingContext<'_> {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            self.dispose_in_place();
        }
    }
}

fn pen_is_visible(pen: Option<&ImmutablePen>) -> bool {
    pen.is_some_and(|pen| pen.brush().is_some() && pen.thickness() > 0.0)
}

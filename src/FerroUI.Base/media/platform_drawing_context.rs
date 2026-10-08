use super::drawing_context::IDrawingContextCore;
use super::immediate_drawing_context::ImmediateDrawingContext;
use crate::media::immutable::ImmutableSolidColorBrush;
use crate::media::{
    BoxShadows, EffectExtensions, Geometry, GlyphRun, IBrush, IEffect, IExperimentalAcrylicMaterial, IPen,
    RenderOptions, TextOptions,
};
use crate::platform::{IDrawingContextImpl, IGeometryImpl};
use crate::rendering::scene_graph::ICustomDrawOperation;
use crate::{Matrix, Point, Rect, Ref, RoundedRect};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

/// A platform drawing context implementation, borrowed or owned.
pub(crate) enum ImplRef<'a> {
    Borrowed(&'a mut dyn IDrawingContextImpl),
    Owned(Box<dyn IDrawingContextImpl + 'a>),
}

impl<'a> ImplRef<'a> {
    #[inline]
    pub(crate) fn get(&mut self) -> &mut (dyn IDrawingContextImpl + 'a) {
        match self {
            ImplRef::Borrowed(value) => reborrow_impl(value),
            ImplRef::Owned(value) => &mut **value,
        }
    }

    pub(crate) fn is_owned(&self) -> bool {
        matches!(self, ImplRef::Owned(_))
    }
}

/// Reborrows the borrowed implementation for the duration of one call.
#[inline]
fn reborrow_impl<'s, 'a>(value: &'s mut &'a mut dyn IDrawingContextImpl) -> &'s mut (dyn IDrawingContextImpl + 'a) {
    &mut **value
}

thread_local! {
    static TRANSFORM_STACK_POOL: RefCell<Vec<Vec<Matrix>>> = const { RefCell::new(Vec::new()) };
}

const TRANSFORM_STACK_POOL_LIMIT: usize = 16;

/// The drawing context core that draws straight to a platform drawing
/// context implementation.
pub struct PlatformDrawingContext<'a> {
    platform_impl: ImplRef<'a>,
    transforms: Option<Vec<Matrix>>,
}

impl<'a> PlatformDrawingContext<'a> {
    /// Creates a core that draws to `platform_impl` and disposes it when the
    /// core is disposed (`ownsImpl: true`).
    pub fn new(platform_impl: Box<dyn IDrawingContextImpl + 'a>) -> Self {
        Self { platform_impl: ImplRef::Owned(platform_impl), transforms: None }
    }

    /// Creates a core that draws to `platform_impl` without taking ownership
    /// of it (`ownsImpl: false`).
    pub fn borrowed(platform_impl: &'a mut dyn IDrawingContextImpl) -> Self {
        Self { platform_impl: ImplRef::Borrowed(platform_impl), transforms: None }
    }

    /// The platform drawing context implementation.
    pub fn platform_impl(&mut self) -> &mut (dyn IDrawingContextImpl + 'a) {
        self.platform_impl.get()
    }

    /// Draws a rectangle filled with an acrylic-like material. A backend
    /// without acrylic support gets the rectangle filled with the fallback
    /// color of the material.
    pub fn draw_rectangle_acrylic(&mut self, material: &dyn IExperimentalAcrylicMaterial, rect: RoundedRect) {
        let platform_impl = self.platform_impl.get();
        if let Some(acrylic_impl) = platform_impl.as_drawing_context_with_acrylic_like_support() {
            acrylic_impl.draw_rectangle_with_material(material, rect);
        } else {
            let brush = ImmutableSolidColorBrush::new(material.fallback_color());
            platform_impl.draw_rectangle(Some(&brush), None, rect, &BoxShadows::default());
        }
    }
}

impl IDrawingContextCore for PlatformDrawingContext<'_> {
    fn draw_line_core(&mut self, pen: &Rc<dyn IPen>, p1: Point, p2: Point) {
        self.platform_impl.get().draw_line(Some(&**pen), p1, p2);
    }

    fn draw_geometry_impl_core(
        &mut self,
        brush: Option<&Rc<dyn IBrush>>,
        pen: Option<&Rc<dyn IPen>>,
        geometry: &Arc<dyn IGeometryImpl>,
    ) {
        self.platform_impl.get().draw_geometry(brush.map(|b| &**b), pen.map(|p| &**p), &**geometry);
    }

    fn draw_rectangle_core(
        &mut self,
        brush: Option<&Rc<dyn IBrush>>,
        pen: Option<&Rc<dyn IPen>>,
        rrect: RoundedRect,
        box_shadows: &BoxShadows,
    ) {
        self.platform_impl.get().draw_rectangle(brush.map(|b| &**b), pen.map(|p| &**p), rrect, box_shadows);
    }

    fn draw_ellipse_core(&mut self, brush: Option<&Rc<dyn IBrush>>, pen: Option<&Rc<dyn IPen>>, rect: Rect) {
        self.platform_impl.get().draw_ellipse(brush.map(|b| &**b), pen.map(|p| &**p), rect);
    }

    fn draw_bitmap(&mut self, source: &std::sync::Arc<crate::platform::SharedBitmapImpl>, opacity: f64, source_rect: Rect, dest_rect: Rect) {
        self.platform_impl.get().draw_bitmap(&**source, opacity, source_rect, dest_rect);
    }

    fn custom(&mut self, custom: &std::sync::Arc<dyn ICustomDrawOperation>) {
        let mut immediate = ImmediateDrawingContext::borrowed(self.platform_impl.get());
        custom.render(&mut immediate);
        immediate.dispose();
    }

    fn draw_glyph_run(&mut self, foreground: Option<&Rc<dyn IBrush>>, glyph_run: &Rc<GlyphRun>) {
        if let Some(foreground) = foreground {
            self.platform_impl.get().draw_glyph_run(Some(&**foreground), &*glyph_run.platform_impl());
        }
    }

    fn draw_rectangle_acrylic_core(&mut self, material: &dyn IExperimentalAcrylicMaterial, rect: RoundedRect) {
        self.draw_rectangle_acrylic(material, rect);
    }

    fn push_clip_core(&mut self, rect: Rect) {
        self.platform_impl.get().push_clip(rect);
    }

    fn push_rounded_clip_core(&mut self, rect: RoundedRect) {
        self.platform_impl.get().push_clip_rounded(rect);
    }

    fn push_geometry_clip_core(&mut self, clip: &Ref<Geometry>) {
        let Some(platform_impl) = clip.platform_impl() else {
            panic!("the clip geometry has no platform implementation");
        };
        self.platform_impl.get().push_geometry_clip(&*platform_impl);
    }

    fn push_opacity_core(&mut self, opacity: f64) {
        self.platform_impl.get().push_opacity(opacity, None);
    }

    fn push_opacity_mask_core(&mut self, mask: &Rc<dyn IBrush>, bounds: Rect) {
        self.platform_impl.get().push_opacity_mask(&**mask, bounds);
    }

    fn push_transform_core(&mut self, matrix: Matrix) {
        let transforms = self
            .transforms
            .get_or_insert_with(|| TRANSFORM_STACK_POOL.with(|pool| pool.borrow_mut().pop()).unwrap_or_default());
        let platform_impl = self.platform_impl.get();
        let current = platform_impl.transform();
        transforms.push(current);
        platform_impl.set_transform(matrix * current);
    }

    fn push_render_options_core(&mut self, render_options: RenderOptions) {
        self.platform_impl.get().push_render_options(render_options);
    }

    fn push_text_options_core(&mut self, text_options: TextOptions) {
        self.platform_impl.get().push_text_options(text_options);
    }

    fn push_effect_core(&mut self, effect: &Rc<dyn IEffect>, bounds: Rect) {
        if let Some(effect_impl) = self.platform_impl.get().as_drawing_context_impl_with_effects() {
            let padding = EffectExtensions::get_effect_output_padding(Some(&**effect));
            effect_impl.push_effect(Some(bounds.inflate_thickness(padding)), &**effect);
        }
    }

    fn pop_clip_core(&mut self) {
        self.platform_impl.get().pop_clip();
    }

    fn pop_geometry_clip_core(&mut self) {
        self.platform_impl.get().pop_geometry_clip();
    }

    fn pop_opacity_core(&mut self) {
        self.platform_impl.get().pop_opacity();
    }

    fn pop_opacity_mask_core(&mut self) {
        self.platform_impl.get().pop_opacity_mask();
    }

    fn pop_transform_core(&mut self) {
        let Some(transform) = self.transforms.as_mut().and_then(Vec::pop) else {
            panic!("the drawing context has been disposed");
        };
        self.platform_impl.get().set_transform(transform);
    }

    fn pop_render_options_core(&mut self) {
        self.platform_impl.get().pop_render_options();
    }

    fn pop_text_options_core(&mut self) {
        self.platform_impl.get().pop_text_options();
    }

    fn pop_effect_core(&mut self) {
        if let Some(effect_impl) = self.platform_impl.get().as_drawing_context_impl_with_effects() {
            effect_impl.pop_effect();
        }
    }

    fn dispose_core(&mut self) {
        if self.platform_impl.is_owned() {
            self.platform_impl.get().dispose();
        }
        if let Some(transforms) = self.transforms.take() {
            if !transforms.is_empty() {
                panic!("Not all states are disposed");
            }
            TRANSFORM_STACK_POOL.with(|pool| {
                let mut pool = pool.borrow_mut();
                if pool.len() < TRANSFORM_STACK_POOL_LIMIT {
                    pool.push(transforms);
                }
            });
        }
    }
}

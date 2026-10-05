//! `CompositorDrawingContextProxy` (upstream `DrawingContextProxy.cs` and
//! `DrawingContextProxy.PendingCommands.cs`).

use crate::media::effects::IEffect;
use crate::media::immutable::ImmutableSolidColorBrush;
use crate::media::{BoxShadows, Color, IBrush, IExperimentalAcrylicMaterial, IPen, RenderOptions, TextOptions};
use crate::platform::{
    IBitmapImpl, IDrawingContextImpl, IDrawingContextImplWithEffects, IDrawingContextLayerImpl,
    IDrawingContextWithAcrylicLikeSupport, IGeometryImpl, IGlyphRunImpl, IPlatformRenderInterfaceRegion,
};
use crate::{Matrix, PixelSize, Point, Rect, RoundedRect};
use std::any::{Any, TypeId};
use std::rc::Rc;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum PendingCommandType {
    SetTransform,
    PushClip,
    PushOpacity,
    PushOpacityMask,
    PushGeometryClip,
    PushRenderOptions,
    PushTextOptions,
    PushEffect,
}

/// A command whose execution is postponed until something is drawn, so
/// that a push that is popped before anything was drawn costs nothing.
///
/// The opacity mask, geometry clip and effect pushes carry a borrowed
/// object and cannot be postponed here: they flush and execute at once.
#[derive(Clone, Copy)]
enum PendingCommand {
    SetTransform(Matrix),
    PushClip(Rect),
    PushRoundedClip(RoundedRect),
    PushOpacity(f64, Option<Rect>),
    PushRenderOptions(RenderOptions),
    PushTextOptions(TextOptions),
}

impl PendingCommand {
    fn command_type(&self) -> PendingCommandType {
        match self {
            PendingCommand::SetTransform(_) => PendingCommandType::SetTransform,
            PendingCommand::PushClip(_) | PendingCommand::PushRoundedClip(_) => PendingCommandType::PushClip,
            PendingCommand::PushOpacity(..) => PendingCommandType::PushOpacity,
            PendingCommand::PushRenderOptions(_) => PendingCommandType::PushRenderOptions,
            PendingCommand::PushTextOptions(_) => PendingCommandType::PushTextOptions,
        }
    }
}

/// A drawing context that forwards to another one, postponing state
/// changes until they are needed and applying an optional post transform.
pub struct CompositorDrawingContextProxy {
    inner: Box<dyn IDrawingContextImpl>,
    transform_stack: Vec<Matrix>,
    post_transform: Option<Matrix>,
    // Transform that was most recently passed to set_transform or restored by a pop operation.
    // We use it to report the transform that would correspond to the current state if all commands were executed
    reported_transform: Matrix,
    // Transform that was most recently passed to set_impl_transform or restored by a pop operation.
    // We use it to save the effective transform before executing a push operation
    effective_transform: Matrix,
    commands: Vec<PendingCommand>,
    auto_flush: bool,
}

impl CompositorDrawingContextProxy {
    pub fn new(inner: Box<dyn IDrawingContextImpl>) -> Self {
        Self {
            inner,
            transform_stack: Vec::new(),
            post_transform: None,
            reported_transform: Matrix::IDENTITY,
            effective_transform: Matrix::IDENTITY,
            commands: Vec::new(),
            auto_flush: false,
        }
    }

    /// Flushes the pending commands and returns the wrapped context.
    pub fn into_inner(mut self) -> Box<dyn IDrawingContextImpl> {
        self.flush();
        self.inner
    }

    pub fn post_transform(&self) -> Option<Matrix> {
        self.post_transform
    }

    pub fn set_post_transform(&mut self, value: Option<Matrix>) {
        self.post_transform = value;
    }

    fn set_impl_transform(&mut self, mut m: Matrix) {
        self.effective_transform = m;
        if let Some(post_transform) = self.post_transform {
            m = m * post_transform;
        }
        self.inner.set_transform(m);
    }

    fn save_transform(&mut self) {
        self.transform_stack.push(self.effective_transform);
    }

    fn restore_transform(&mut self) {
        let transform = self.transform_stack.pop().expect("a transform was saved");
        self.reported_transform = transform;
        self.effective_transform = transform;
    }

    // --- pending commands ------------------------------------------------------

    pub fn auto_flush(&self) -> bool {
        self.auto_flush
    }

    pub fn set_auto_flush(&mut self, value: bool) {
        self.auto_flush = value;
        if value {
            self.flush();
        }
    }

    fn set_transform_command(&mut self, m: Matrix) {
        if self.auto_flush {
            self.set_impl_transform(m);
            return;
        }

        let cmd = PendingCommand::SetTransform(m);
        match self.commands.last_mut() {
            Some(last) if last.command_type() == PendingCommandType::SetTransform => *last = cmd,
            _ => self.commands.push(cmd),
        }
    }

    fn try_discard_or_flush(&mut self, command_type: PendingCommandType) -> bool {
        for c in (0..self.commands.len()).rev() {
            let current = self.commands[c].command_type();
            if current == PendingCommandType::SetTransform {
                continue;
            }
            if current == command_type {
                self.commands.truncate(c);
                return true;
            }
            break;
        }

        // We've failed to collapse PushX,SetTransform,PopX stack, so we need to execute any pending commands
        self.flush();
        false
    }

    fn add_command(&mut self, command: PendingCommand) {
        if self.auto_flush {
            self.exec_command(command);
        } else {
            self.commands.push(command);
        }
    }

    fn exec_command(&mut self, cmd: PendingCommand) {
        if let PendingCommand::SetTransform(transform) = cmd {
            self.set_impl_transform(transform);
            return;
        }

        self.save_transform();

        match cmd {
            PendingCommand::PushOpacity(opacity, bounds) => self.inner.push_opacity(opacity, bounds),
            PendingCommand::PushClip(rect) => self.inner.push_clip(rect),
            PendingCommand::PushRoundedClip(rect) => self.inner.push_clip_rounded(rect),
            PendingCommand::PushRenderOptions(options) => self.inner.push_render_options(options),
            PendingCommand::PushTextOptions(options) => self.inner.push_text_options(options),
            PendingCommand::SetTransform(_) => {}
        }
    }

    pub fn flush(&mut self) {
        let commands = std::mem::take(&mut self.commands);
        for command in &commands {
            self.exec_command(*command);
        }
        let mut commands = commands;
        commands.clear();
        self.commands = commands;
    }

    fn pop(&mut self, command_type: PendingCommandType, pop: impl FnOnce(&mut dyn IDrawingContextImpl)) {
        if !self.try_discard_or_flush(command_type) {
            pop(&mut *self.inner);
            self.restore_transform();
        }
    }
}

impl IDrawingContextImpl for CompositorDrawingContextProxy {
    fn transform(&self) -> Matrix {
        self.reported_transform
    }

    fn set_transform(&mut self, value: Matrix) {
        self.reported_transform = value;
        self.set_transform_command(value);
    }

    fn clear(&mut self, color: Color) {
        self.flush();
        self.inner.clear(color);
    }

    fn draw_bitmap(&mut self, source: &dyn IBitmapImpl, opacity: f64, source_rect: Rect, dest_rect: Rect) {
        self.flush();
        self.inner.draw_bitmap(source, opacity, source_rect, dest_rect);
    }

    fn draw_bitmap_with_mask(
        &mut self,
        source: &dyn IBitmapImpl,
        opacity_mask: &dyn IBrush,
        opacity_mask_rect: Rect,
        dest_rect: Rect,
    ) {
        self.flush();
        self.inner.draw_bitmap_with_mask(source, opacity_mask, opacity_mask_rect, dest_rect);
    }

    fn draw_line(&mut self, pen: Option<&dyn IPen>, p1: Point, p2: Point) {
        self.flush();
        self.inner.draw_line(pen, p1, p2);
    }

    fn draw_geometry(&mut self, brush: Option<&dyn IBrush>, pen: Option<&dyn IPen>, geometry: &dyn IGeometryImpl) {
        self.flush();
        self.inner.draw_geometry(brush, pen, geometry);
    }

    fn draw_rectangle(
        &mut self,
        brush: Option<&dyn IBrush>,
        pen: Option<&dyn IPen>,
        rect: RoundedRect,
        box_shadows: &BoxShadows,
    ) {
        self.flush();
        self.inner.draw_rectangle(brush, pen, rect, box_shadows);
    }

    fn draw_region(
        &mut self,
        brush: Option<&dyn IBrush>,
        pen: Option<&dyn IPen>,
        region: &dyn IPlatformRenderInterfaceRegion,
    ) {
        self.flush();
        self.inner.draw_region(brush, pen, region);
    }

    fn draw_ellipse(&mut self, brush: Option<&dyn IBrush>, pen: Option<&dyn IPen>, rect: Rect) {
        self.flush();
        self.inner.draw_ellipse(brush, pen, rect);
    }

    fn draw_glyph_run(&mut self, foreground: Option<&dyn IBrush>, glyph_run: &dyn IGlyphRunImpl) {
        self.flush();
        self.inner.draw_glyph_run(foreground, glyph_run);
    }

    fn create_layer(&mut self, size: PixelSize) -> Rc<dyn IDrawingContextLayerImpl> {
        self.inner.create_layer(size)
    }

    fn push_clip(&mut self, clip: Rect) {
        self.add_command(PendingCommand::PushClip(clip));
    }

    fn push_clip_rounded(&mut self, clip: RoundedRect) {
        self.add_command(PendingCommand::PushRoundedClip(clip));
    }

    fn push_clip_region(&mut self, region: &dyn IPlatformRenderInterfaceRegion) {
        self.flush();
        self.inner.push_clip_region(region);
    }

    fn pop_clip(&mut self) {
        self.pop(PendingCommandType::PushClip, |inner| inner.pop_clip());
    }

    fn push_layer(&mut self, bounds: Rect) {
        self.flush();
        self.inner.push_layer(bounds);
    }

    fn pop_layer(&mut self) {
        self.flush();
        self.inner.pop_layer();
    }

    fn push_opacity(&mut self, opacity: f64, bounds: Option<Rect>) {
        self.add_command(PendingCommand::PushOpacity(opacity, bounds));
    }

    fn pop_opacity(&mut self) {
        self.pop(PendingCommandType::PushOpacity, |inner| inner.pop_opacity());
    }

    fn push_opacity_mask(&mut self, mask: &dyn IBrush, bounds: Rect) {
        self.flush();
        self.save_transform();
        self.inner.push_opacity_mask(mask, bounds);
    }

    fn pop_opacity_mask(&mut self) {
        self.pop(PendingCommandType::PushOpacityMask, |inner| inner.pop_opacity_mask());
    }

    fn push_geometry_clip(&mut self, clip: &dyn IGeometryImpl) {
        self.flush();
        self.save_transform();
        self.inner.push_geometry_clip(clip);
    }

    fn pop_geometry_clip(&mut self) {
        self.pop(PendingCommandType::PushGeometryClip, |inner| inner.pop_geometry_clip());
    }

    fn push_render_options(&mut self, render_options: RenderOptions) {
        self.add_command(PendingCommand::PushRenderOptions(render_options));
    }

    fn pop_render_options(&mut self) {
        self.pop(PendingCommandType::PushRenderOptions, |inner| inner.pop_render_options());
    }

    fn push_text_options(&mut self, text_options: TextOptions) {
        self.add_command(PendingCommand::PushTextOptions(text_options));
    }

    fn pop_text_options(&mut self) {
        self.pop(PendingCommandType::PushTextOptions, |inner| inner.pop_text_options());
    }

    fn get_feature(&mut self, feature_type: TypeId) -> Option<Rc<dyn Any>> {
        self.flush();
        self.inner.get_feature(feature_type)
    }

    fn as_drawing_context_impl_with_effects(&mut self) -> Option<&mut dyn IDrawingContextImplWithEffects> {
        Some(self)
    }

    fn as_drawing_context_with_acrylic_like_support(
        &mut self,
    ) -> Option<&mut dyn IDrawingContextWithAcrylicLikeSupport> {
        Some(self)
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn dispose(&mut self) {
        self.flush();
        debug_assert!(self.transform_stack.is_empty());
        self.transform_stack.clear();
    }
}

impl IDrawingContextWithAcrylicLikeSupport for CompositorDrawingContextProxy {
    fn draw_rectangle_with_material(&mut self, material: &dyn IExperimentalAcrylicMaterial, rect: RoundedRect) {
        self.flush();
        match self.inner.as_drawing_context_with_acrylic_like_support() {
            Some(acrylic) => acrylic.draw_rectangle_with_material(material, rect),
            None => self.inner.draw_rectangle(
                Some(&ImmutableSolidColorBrush::new(material.fallback_color())),
                None,
                rect,
                &BoxShadows::default(),
            ),
        }
    }
}

impl IDrawingContextImplWithEffects for CompositorDrawingContextProxy {
    fn push_effect(&mut self, clip_rect: Option<Rect>, effect: &dyn IEffect) {
        self.flush();
        self.save_transform();
        if let Some(effects) = self.inner.as_drawing_context_impl_with_effects() {
            effects.push_effect(clip_rect, effect);
        }
    }

    fn pop_effect(&mut self) {
        if !self.try_discard_or_flush(PendingCommandType::PushEffect) {
            if let Some(effects) = self.inner.as_drawing_context_impl_with_effects() {
                effects.pop_effect();
            }
            self.restore_transform();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rendering::testing::{DrawingLog, MockDrawingContextImpl};

    fn proxy() -> (CompositorDrawingContextProxy, DrawingLog) {
        let log = DrawingLog::new();
        (CompositorDrawingContextProxy::new(Box::new(MockDrawingContextImpl::new(log.clone()))), log)
    }

    #[test]
    fn a_push_popped_before_drawing_never_reaches_the_context() {
        let (mut proxy, log) = proxy();
        proxy.push_clip(Rect::new(0.0, 0.0, 10.0, 10.0));
        proxy.set_transform(Matrix::create_translation(5.0, 5.0));
        proxy.push_opacity(0.5, None);
        proxy.pop_opacity();
        proxy.pop_clip();
        assert!(log.entries().is_empty(), "{:?}", log.entries());
        assert_eq!(proxy.transform(), Matrix::create_translation(5.0, 5.0));
    }

    #[test]
    fn drawing_flushes_pending_commands_in_order_and_pops_reach_the_context() {
        let (mut proxy, log) = proxy();
        proxy.push_clip(Rect::new(0.0, 0.0, 10.0, 10.0));
        proxy.set_transform(Matrix::create_translation(5.0, 5.0));
        proxy.draw_ellipse(None, None, Rect::new(0.0, 0.0, 1.0, 1.0));
        let before_pop = log.entries().len();
        assert_eq!(before_pop, 3, "{:?}", log.entries());
        proxy.pop_clip();
        assert_eq!(log.entries().len(), before_pop + 1);
        // The transform saved by the push is restored.
        assert_eq!(proxy.transform(), Matrix::IDENTITY);
    }

    #[test]
    fn auto_flush_executes_at_once_and_post_transform_is_applied() {
        let (mut proxy, log) = proxy();
        proxy.set_post_transform(Some(Matrix::create_scale(2.0, 2.0)));
        proxy.set_auto_flush(true);
        proxy.set_transform(Matrix::create_translation(1.0, 1.0));
        assert_eq!(log.entries().len(), 1);
        assert_eq!(proxy.transform(), Matrix::create_translation(1.0, 1.0));
        let mut inner = proxy.into_inner();
        assert_eq!(inner.transform(), Matrix::create_translation(1.0, 1.0) * Matrix::create_scale(2.0, 2.0));
        inner.dispose();
    }
}

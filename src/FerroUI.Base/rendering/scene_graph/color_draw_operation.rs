//! A solid compositing command, recorded through the native custom-draw path.
use super::ICustomDrawOperation;
use crate::media::{Color, ImmediateDrawingContext, imaging::BitmapBlendingMode};
use crate::{Point, Rect};

pub(crate) struct ColorDrawOperation {
    pub color: Color,
    pub blend: BitmapBlendingMode,
    pub bounds: Rect,
}

impl ICustomDrawOperation for ColorDrawOperation {
    fn bounds(&self) -> Rect {
        self.bounds
    }
    fn hit_test(&self, _point: Point) -> bool {
        false
    }
    fn render(&self, context: &mut ImmediateDrawingContext<'_>) {
        context.platform_impl().draw_color(self.color, self.blend);
    }
    fn equals(&self, other: &dyn ICustomDrawOperation) -> bool {
        std::ptr::eq(self as &dyn ICustomDrawOperation, other)
    }
    fn dispose(&self) {
        // Only Copy values are retained. The native draw list owns this
        // operation through Arc; dropping it releases all of its storage.
    }
}

#[cfg(test)]
mod tests {
    use crate::Rect;
    use crate::media::{Color, DrawingContext, PlatformDrawingContext, imaging::BitmapBlendingMode};
    use crate::media::{ISceneBrushContent, ITileBrush, immutable::ImmutableImageBrush};
    use crate::rendering::composition::drawing::RenderDataDrawingContext;
    use crate::rendering::composition::server::CompositorDrawingContextProxy;
    use crate::rendering::testing::{DrawingLog, MockDrawingContextImpl, MockPlatformRenderInterface};

    #[test]
    fn color_composition_flushes_pending_clip_and_layer_state_in_order() {
        let log = DrawingLog::new();
        let mut mock = MockDrawingContextImpl::new(log.clone());
        mock.supports_compositing_layers = true;
        mock.log_transforms = false;
        let proxy = CompositorDrawingContextProxy::new(Box::new(mock));
        let mut context =
            DrawingContext::owned(Box::new(PlatformDrawingContext::new(Box::new(proxy))));
        let bounds = Rect::new(0., 0., 3., 2.);
        let layer = context
            .try_push_compositing_layer(BitmapBlendingMode::SourceOver, 1.0, bounds)
            .unwrap();
        let clip = context.push_clip(bounds);
        context.draw_color(Color::from_uint32(0x80402010), BitmapBlendingMode::SourceIn, bounds);
        let entries = log.entries();
        assert_eq!(entries.len(), 3, "{entries:?}");
        assert!(entries[0].starts_with("PushCompositingLayer"));
        assert!(entries[1].starts_with("PushClip"));
        assert_eq!(entries[2], "DrawColor #80402010 SourceIn");
        context.pop(clip);
        context.pop(layer);
    }

    #[test]
    fn recorded_color_stays_inside_its_compositing_layer() {
        let (_scope, backend) = MockPlatformRenderInterface::install();
        backend.set_supports_compositing_layers(true);
        let bounds = Rect::new(0., 0., 3., 2.);
        let mut recorder = RenderDataDrawingContext::new(None);
        {
            let mut context = DrawingContext::new(&mut recorder);
            let layer = context
                .try_push_compositing_layer(BitmapBlendingMode::SourceOver, 1.0, bounds)
                .unwrap();
            context.draw_color(Color::from_uint32(0xffff0000), BitmapBlendingMode::SourceIn, bounds);
            context.pop(layer);
        }
        let brush: std::rc::Rc<dyn ITileBrush> =
            std::rc::Rc::new(ImmutableImageBrush::from_bitmap(None));
        let content = recorder.get_immediate_scene_brush_content(brush, None, true).unwrap();
        let log = DrawingLog::new();
        let mut target = MockDrawingContextImpl::new(log.clone());
        target.supports_compositing_layers = true;
        content.render(&mut target, None);
        assert_eq!(log.count("PushCompositingLayer"), 1, "{:?}", log.entries());
        assert_eq!(log.count("PopCompositingLayer"), 1, "{:?}", log.entries());
    }
}

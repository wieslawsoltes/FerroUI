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
    use crate::media::{
        Color, DrawingContext, PlatformDrawingContext, RenderOptions, imaging::BitmapBlendingMode,
    };
    use crate::media::{ISceneBrushContent, ITileBrush, immutable::ImmutableImageBrush};
    use crate::rendering::composition::drawing::{RenderDataDrawingContext, RenderDataStream};
    use crate::rendering::composition::server::CompositorDrawingContextProxy;
    use crate::rendering::testing::{DrawingLog, MockDrawingContextImpl};

    #[test]
    fn color_composition_flushes_pending_clip_and_layer_state_in_order() {
        let log = DrawingLog::new();
        let mut mock = MockDrawingContextImpl::new(log.clone());
        mock.log_transforms = false;
        let proxy = CompositorDrawingContextProxy::new(Box::new(mock));
        let mut context =
            DrawingContext::owned(Box::new(PlatformDrawingContext::new(Box::new(proxy))));
        let options = context.push_render_options(RenderOptions {
            requires_full_opacity_handling: Some(true),
            ..Default::default()
        });
        let opacity = context.push_opacity(1.0);
        let clip = context.push_clip(Rect::new(0., 0., 3., 2.));
        assert!(log.entries().is_empty());
        context.draw_color(
            Color::from_uint32(0x80402010),
            BitmapBlendingMode::SourceIn,
            Rect::new(0., 0., 3., 2.),
        );
        let entries = log.entries();
        assert_eq!(entries.len(), 4, "{entries:?}");
        assert!(entries[0].starts_with("PushRenderOptions"));
        assert!(entries[1].starts_with("PushOpacity"));
        assert!(entries[2].starts_with("PushClip"));
        assert_eq!(entries[3], "DrawColor #80402010 SourceIn");
        context.pop(clip);
        context.pop(opacity);
        context.pop(options);
    }

    #[test]
    fn full_opacity_recording_preserves_unit_layer() {
        let mut recorder = RenderDataDrawingContext::new(None);
        {
            let mut context = DrawingContext::new(&mut recorder);
            let options = context.push_render_options(RenderOptions {
                requires_full_opacity_handling: Some(true),
                ..Default::default()
            });
            let layer = context.push_opacity(1.0);
            context.draw_color(
                Color::from_uint32(0xffff0000),
                BitmapBlendingMode::SourceIn,
                Rect::new(0., 0., 3., 2.),
            );
            context.pop(layer);
            context.pop(options);
        }
        let brush: std::rc::Rc<dyn ITileBrush> =
            std::rc::Rc::new(ImmutableImageBrush::from_bitmap(None));
        let content = recorder
            .get_immediate_scene_brush_content(brush, None, true)
            .unwrap();
        let log = DrawingLog::new();
        content.render(&mut MockDrawingContextImpl::new(log.clone()), None);
        assert_eq!(log.count("PushOpacity"), 1, "{:?}", log.entries());
        assert_eq!(log.count("PopOpacity"), 1, "{:?}", log.entries());
    }

    #[test]
    fn full_opacity_replay_preserves_unit_layer() {
        let mut stream = RenderDataStream::new();
        stream.push_render_options(RenderOptions {
            requires_full_opacity_handling: Some(true),
            ..Default::default()
        });
        stream.push_opacity(1.0);
        stream.draw_custom(Some(std::sync::Arc::new(super::ColorDrawOperation {
            color: Color::from_uint32(0xffff0000),
            blend: BitmapBlendingMode::SourceIn,
            bounds: Rect::new(0., 0., 3., 2.),
        })));
        stream.pop();
        stream.pop();
        let log = DrawingLog::new();
        stream.replay(&mut MockDrawingContextImpl::new(log.clone()));
        assert_eq!(log.count("PushOpacity"), 1, "{:?}", log.entries());
        assert_eq!(log.count("PopOpacity"), 1, "{:?}", log.entries());
    }
}

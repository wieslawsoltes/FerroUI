use crate::media::ref_adapter::RefAdapter;
use crate::media::{DrawingContext, ISceneBrushContent, TileBrush};
use crate::rendering::composition::drawing::RenderDataDrawingContext;
use crate::rendering::ImmediateRenderer;
use crate::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, FerroObjectImpl, FerroProperty, Nullable, Rect, Ref,
    StyledProperty, Visual,
};
use std::rc::Rc;

/// Paints an area with a [`Visual`].
#[repr(C)]
pub struct VisualBrush {
    base: TileBrush,
}

ferro_class!(VisualBrush: TileBrush);
crate::ferro_class_info!(VisualBrush { new: VisualBrush::new });
ferro_impl_classes!(VisualBrush: FerroObjectImpl, crate::media::BrushImpl);

crate::ferro_properties! { impl VisualBrush {
    ferro_property!(pub fn visual_property() -> StyledProperty<Option<Ref<Visual>>> {
        FerroProperty::register::<VisualBrush, _>("Visual", None)
    });
} }

impl VisualBrush {
    /// Creates the class data.
    pub fn construct() -> Self {
        Self { base: TileBrush::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Creates a brush that draws `visual`.
    pub fn with_visual(visual: impl Into<Nullable<Visual>>) -> Ref<Self> {
        let result = Self::new();
        result.set_visual(visual);
        result
    }

    /// The visual to draw.
    pub fn visual(&self) -> Option<Ref<Visual>> {
        self.get_value(Self::visual_property())
    }

    pub fn set_visual(&self, value: impl Into<Nullable<Visual>>) {
        self.set_value(Self::visual_property(), value.into().0)
    }

    /// Records the current content of the brush: the rendering of the
    /// visual. `None` when there is no visual or it draws nothing.
    pub fn create_content(&self) -> Option<Rc<dyn ISceneBrushContent>> {
        let visual = self.visual()?;

        let mut recorder = RenderDataDrawingContext::new(None);
        {
            let mut context = DrawingContext::new(&mut recorder);
            ImmediateRenderer::render(&mut context, &visual);
        }
        let content = recorder.get_immediate_scene_brush_content(
            Rc::new(RefAdapter(self.to_ref())),
            Some(Rect::from_size(visual.bounds().size())),
            true,
        );
        recorder.reset();
        content.map(|content| content as Rc<dyn ISceneBrushContent>)
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream: the upstream tests of this class need the compositor.
    use super::*;
    use crate::media::{IBrush, Stretch};
    use std::cell::Cell;

    #[test]
    fn changing_visual_raises_invalidated() {
        let target = VisualBrush::new();
        let raised = Rc::new(Cell::new(0));
        let r = raised.clone();
        target.invalidated(move || r.set(r.get() + 1));
        let visual = Visual::new();
        target.set_visual(&visual);
        assert_eq!(1, raised.get());
        assert!(target.visual().unwrap() == visual);
        target.set_stretch(Stretch::Fill);
        assert_eq!(2, raised.get());

        let brush: Rc<dyn IBrush> = VisualBrush::with_visual(&visual).into();
        assert_eq!(Stretch::Uniform, brush.as_tile_brush().unwrap().stretch());
        assert!(brush.as_image_brush().is_none());
        assert!(brush.as_mutable_brush().is_none());
    }

    #[repr(C)]
    struct Painted {
        base: Visual,
    }

    crate::ferro_class!(Painted: Visual);
    crate::ferro_impl_classes!(Painted: FerroObjectImpl, crate::StyledElementImpl);

    impl crate::VisualImpl for Painted {
        fn render(this: &Self, context: &mut DrawingContext) {
            let brush: Rc<dyn IBrush> = crate::media::Brushes::red();
            context.fill_rectangle(&brush, Rect::from_size(this.bounds().size()), 0.0);
        }
    }

    #[test]
    fn create_content_records_the_rendering_of_the_visual() {
        use crate::media::ISceneBrush;
        use crate::rendering::testing::{DrawingLog, MockDrawingContextImpl};

        let target = VisualBrush::new();
        assert!(target.create_content().is_none());

        // A visual that draws nothing has no content.
        target.set_visual(&Visual::new());
        assert!(target.create_content().is_none());

        let visual: Ref<Painted> = instantiate(Painted { base: Visual::construct() });
        visual.set_bounds(Rect::new(30.0, 40.0, 20.0, 10.0));
        let child: Ref<Painted> = instantiate(Painted { base: Visual::construct() });
        child.set_bounds(Rect::new(5.0, 5.0, 2.0, 2.0));
        visual.visual_children().add(child.upcast());
        target.set_visual(&visual.clone().upcast::<Visual>());
        target.set_stretch(Stretch::None);

        let adapter = RefAdapter(target.clone());
        let content = ISceneBrush::create_content(&adapter).expect("the visual draws something");
        // The content covers the visual's own area, wherever its parent
        // placed it.
        assert_eq!(Rect::new(0.0, 0.0, 20.0, 10.0), content.rect());
        assert!(content.use_scalable_rasterization());
        assert_eq!(Stretch::None, content.brush().stretch());

        let log = DrawingLog::new();
        let mut context = MockDrawingContextImpl::new(log.clone());
        context.log_transforms = false;
        content.render(&mut context, None);
        assert_eq!(
            log.entries(),
            [
                "PushRoundedClip 0, 0, 20, 10",
                "DrawRectangle Red none 0, 0, 20, 10 shadows=0",
                "DrawRectangle Red none 0, 0, 2, 2 shadows=0",
                "PopClip"
            ]
        );
        content.dispose();
    }
}

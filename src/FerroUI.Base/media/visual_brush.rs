use crate::media::ref_adapter::RefAdapter;
use crate::media::{BrushImpl, BrushImplExt, DrawingContext, ISceneBrushContent, ServerBrushFactory, TileBrush};
use crate::rendering::composition::drawing::{
    CompositionRenderData, CompositionRenderDataSceneBrushContentProperties, RenderDataDrawingContext,
};
use crate::rendering::composition::server::ServerCompositionSimpleContentBrush;
use crate::rendering::composition::transport::BatchStreamWriter;
use crate::rendering::composition::Compositor;
use crate::rendering::ImmediateRenderer;
use crate::{
    ferro_class, ferro_property, instantiate, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, Nullable, Rect, Ref, StyledProperty, Visual,
};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// The content of the brush recorded for one compositor.
struct RenderDataItem {
    data: Rc<CompositionRenderData>,
    rect: Rect,
    is_dirty: Cell<bool>,
}

impl RenderDataItem {
    fn dispose(&self) {
        self.data.dispose();
    }
}

/// Paints an area with a [`Visual`].
#[repr(C)]
pub struct VisualBrush {
    base: TileBrush,
    /// The recorded content per compositor the brush is used with; `None`
    /// for a compositor the content of which is empty.
    render_data_dictionary: RefCell<Vec<(Weak<Compositor>, Option<Rc<RenderDataItem>>)>>,
}

ferro_class!(VisualBrush: TileBrush);
crate::ferro_class_info!(VisualBrush { new: VisualBrush::new });

impl FerroObjectImpl for VisualBrush {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        // We are supposed to be only calling this when content is actually changed,
        // but instead we are calling this on brush property change for backwards compatibility
        this.invalidate_content();
        Self::parent_on_property_changed(this, change);
    }
}

impl BrushImpl for VisualBrush {
    fn factory(_this: &Self) -> Option<ServerBrushFactory> {
        Some(|c| ServerCompositionSimpleContentBrush::new(c))
    }

    fn on_unreferenced_from_compositor(this: &Self, c: &Rc<Compositor>) {
        let removed = {
            let mut dictionary = this.render_data_dictionary.borrow_mut();
            dictionary.iter().position(|(key, _)| is_compositor(key, c)).map(|index| dictionary.remove(index).1)
        };
        if let Some(Some(content)) = removed {
            content.dispose();
        }
        Self::parent_on_unreferenced_from_compositor(this, c);
    }

    /// The first half of the serialization of upstream: records the content
    /// again when there is none or it is out of date. The render data is a
    /// new server object, which has to be created before the changes of the
    /// brush refer to it.
    fn prepare_serialization(this: &Self, c: &Rc<Compositor>) {
        Self::parent_prepare_serialization(this, c);
        // Should always be true here, but just in case do this check
        if this.is_on_compositor(c) {
            let data = this.render_data(c);
            if data.as_ref().is_none_or(|data| data.is_dirty.get()) {
                let created = this.create_server_content(c);
                // Dispose the old render list _after_ creating a new one to avoid unnecessary detach/attach
                // sequence for referenced resources
                if let Some(data) = data {
                    data.dispose();
                }

                this.set_render_data(c, created);
            }
        }
    }

    fn serialize_changes(this: &Self, c: &Compositor, writer: &mut BatchStreamWriter<'_>) {
        Self::parent_serialize_changes(this, c, writer);
        let mut content = None;
        // Should always be true here, but just in case do this check
        if this.is_on_compositor(c) {
            if let Some(data) = this.render_data(c) {
                content = Some((data.data.server(), Some(data.rect), true));
            }
        }

        CompositionRenderDataSceneBrushContentProperties::serialize(writer, content);
    }
}

fn is_compositor(key: &Weak<Compositor>, c: &Compositor) -> bool {
    std::ptr::eq(key.as_ptr(), c)
}

crate::ferro_properties! { impl VisualBrush {
    ferro_property!(pub fn visual_property() -> StyledProperty<Option<Ref<Visual>>> {
        FerroProperty::register::<VisualBrush, _>("Visual", None)
    });
} }

impl VisualBrush {
    /// Creates the class data.
    pub fn construct() -> Self {
        Self { base: TileBrush::construct(), render_data_dictionary: RefCell::new(Vec::new()) }
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

        visual.ensure_initialized_for_visual_brush();

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

    /// The content recorded for compositor `c`, if there is any.
    fn render_data(&self, c: &Compositor) -> Option<Rc<RenderDataItem>> {
        self.render_data_dictionary.borrow().iter().find(|(key, _)| is_compositor(key, c)).and_then(|(_, item)| item.clone())
    }

    fn set_render_data(&self, c: &Rc<Compositor>, item: Option<Rc<RenderDataItem>>) {
        let mut dictionary = self.render_data_dictionary.borrow_mut();
        match dictionary.iter_mut().find(|(key, _)| is_compositor(key, c)) {
            Some(entry) => entry.1 = item,
            None => dictionary.push((Rc::downgrade(c), item)),
        }
    }

    fn invalidate_content(&self) {
        for (_, item) in self.render_data_dictionary.borrow().iter() {
            if let Some(item) = item {
                item.is_dirty.set(true);
            }
        }
        self.register_for_serialization();
    }

    fn create_server_content(&self, c: &Rc<Compositor>) -> Option<Rc<RenderDataItem>> {
        let visual = self.visual()?;

        visual.ensure_initialized_for_visual_brush();

        let mut recorder = RenderDataDrawingContext::new(Some(c.clone()));
        {
            let mut context = DrawingContext::new(&mut recorder);
            ImmediateRenderer::render(&mut context, &visual);
        }
        let render_data = recorder.get_render_results();
        recorder.reset();
        let render_data = render_data?;

        Some(Rc::new(RenderDataItem {
            data: render_data,
            rect: Rect::from_size(visual.bounds().size()),
            is_dirty: Cell::new(false),
        }))
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

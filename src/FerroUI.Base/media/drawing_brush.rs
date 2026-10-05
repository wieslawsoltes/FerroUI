use crate::media::ref_adapter::RefAdapter;
use crate::media::{Drawing, DrawingContext, ISceneBrushContent, TileBrush};
use crate::rendering::composition::drawing::RenderDataDrawingContext;
use crate::reactive::IDisposable;
use crate::{
    ferro_class, ferro_property, instantiate, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, Nullable, Ref, StyledProperty,
};
use std::cell::RefCell;
use std::rc::Rc;

/// Paints an area with a [`Drawing`].
#[repr(C)]
pub struct DrawingBrush {
    base: TileBrush,
    drawing_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
}

ferro_class!(DrawingBrush: TileBrush);
crate::ferro_class_info!(DrawingBrush { new: DrawingBrush::new });
crate::ferro_impl_classes!(DrawingBrush: crate::media::BrushImpl);

impl FerroObjectImpl for DrawingBrush {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        // Replacing the drawing, or a structural change inside it (e.g. a
        // replaced brush, pen or geometry) changes the recorded content.
        if change.property() == Self::drawing_property().as_property() {
            let new_value = change.get_new_value::<Option<Ref<Drawing>>>();
            if let Some(subscription) = this.drawing_subscription.take() {
                subscription.dispose();
            }
            if let Some(new_value) = new_value {
                let weak = this.to_ref().downgrade();
                let subscription = new_value.invalidated(move || {
                    if let Some(this) = weak.upgrade() {
                        this.invalidate_content();
                    }
                });
                *this.drawing_subscription.borrow_mut() = Some(subscription);
            }
            this.invalidate_content();
        }

        Self::parent_on_property_changed(this, change);
    }
}

crate::ferro_properties! { impl DrawingBrush {
    ferro_property!(pub fn drawing_property() -> StyledProperty<Option<Ref<Drawing>>> {
        FerroProperty::register::<DrawingBrush, _>("Drawing", None)
    });
} }

impl DrawingBrush {
    /// Creates the class data.
    pub fn construct() -> Self {
        Self { base: TileBrush::construct(), drawing_subscription: RefCell::new(None) }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Creates a brush that draws `drawing`.
    pub fn with_drawing(drawing: impl Into<Nullable<Drawing>>) -> Ref<Self> {
        let result = Self::new();
        result.set_drawing(drawing);
        result
    }

    /// The drawing to draw.
    pub fn drawing(&self) -> Option<Ref<Drawing>> {
        self.get_value(Self::drawing_property())
    }

    pub fn set_drawing(&self, value: impl Into<Nullable<Drawing>>) {
        self.set_value(Self::drawing_property(), value.into().0)
    }

    /// Records the current content of the brush: what the drawing draws.
    /// `None` when there is no drawing or it draws nothing.
    pub fn create_content(&self) -> Option<Rc<dyn ISceneBrushContent>> {
        let drawing = self.drawing()?;

        let mut recorder = RenderDataDrawingContext::new(None);
        {
            let mut context = DrawingContext::new(&mut recorder);
            drawing.draw(&mut context);
        }
        let content = recorder.get_immediate_scene_brush_content(Rc::new(RefAdapter(self.to_ref())), None, true);
        recorder.reset();
        content.map(|content| content as Rc<dyn ISceneBrushContent>)
    }

    /// Marks the recorded content of the brush as out of date.
    fn invalidate_content(&self) {
        self.raise_invalidated();
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream: the upstream tests of this class need the compositor.
    use super::*;
    use crate::media::{Brushes, GeometryDrawing};
    use std::cell::Cell;

    #[test]
    fn drawing_changes_raise_invalidated() {
        let target = DrawingBrush::new();
        let raised = Rc::new(Cell::new(0));
        let r = raised.clone();
        target.invalidated(move || r.set(r.get() + 1));

        let drawing = GeometryDrawing::new();
        target.set_drawing(&drawing);
        assert!(raised.get() >= 1);

        let before = raised.get();
        drawing.set_brush(Some(Brushes::red()));
        assert_eq!(before + 1, raised.get());

        target.set_drawing(None);
        let before = raised.get();
        drawing.set_brush(Some(Brushes::blue()));
        assert_eq!(before, raised.get());
    }

    #[test]
    fn create_content_records_the_drawing() {
        use crate::media::{ISceneBrush, RectangleGeometry, Stretch};
        use crate::rendering::testing::{DrawingLog, MockDrawingContextImpl, MockPlatformRenderInterface};
        use crate::{Matrix, Rect};

        let (scope, _) = MockPlatformRenderInterface::install();
        let target = DrawingBrush::new();
        assert!(target.create_content().is_none());

        let drawing = GeometryDrawing::new();
        target.set_drawing(&drawing);
        // A drawing that draws nothing has no content.
        assert!(target.create_content().is_none());

        drawing.set_brush(Some(Brushes::red()));
        drawing.set_geometry(RectangleGeometry::with_rect(Rect::new(1.5, 2.5, 10.0, 20.0)).upcast::<crate::media::Geometry>());
        target.set_stretch(Stretch::Fill);
        target.set_opacity(0.25);

        // Through the scene brush interface of the class handle.
        let adapter = RefAdapter(target.clone());
        let content = ISceneBrush::create_content(&adapter).expect("the drawing draws something");
        // The bounds of what was drawn, rounded outwards to whole units.
        assert_eq!(Rect::new(1.0, 2.0, 11.0, 21.0), content.rect());
        assert!(content.use_scalable_rasterization());
        assert_eq!(Stretch::Fill, content.brush().stretch());
        assert_eq!(0.25, content.opacity());

        let log = DrawingLog::new();
        let mut context = MockDrawingContextImpl::new(log.clone());
        content.render(&mut context, None);
        assert_eq!(log.entries(), ["DrawGeometry Red none 1.5, 2.5, 10, 20"]);

        // With a transform: applied around the replay and restored after.
        log.clear();
        let transform = Matrix::create_scale(2.0, 2.0);
        content.render(&mut context, Some(transform));
        assert_eq!(
            log.entries(),
            [
                format!("SetTransform {transform}"),
                "DrawGeometry Red none 1.5, 2.5, 10, 20".to_string(),
                format!("SetTransform {}", Matrix::IDENTITY),
            ]
        );

        // Disposed content draws nothing; disposing twice is fine.
        content.dispose();
        content.dispose();
        log.clear();
        content.render(&mut context, None);
        assert!(log.entries().is_empty());

        // The content is a snapshot: it is recorded again on every call.
        drawing.set_brush(Some(Brushes::blue()));
        let content = target.create_content().unwrap();
        content.render(&mut context, None);
        assert_eq!(log.entries(), ["DrawGeometry Blue none 1.5, 2.5, 10, 20"]);
        scope.dispose();
    }
}

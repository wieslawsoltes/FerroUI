use crate::{Control, ControlImpl};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{Layoutable, LayoutableImpl};
use ferroui_base::media::imaging::BitmapBlendingMode;
use ferroui_base::media::{
    DrawingContext, DrawingImage, IImage, MediaExtensions, RenderOptions, Stretch, StretchDirection,
};
use ferroui_base::reactive::IDisposable;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, Rect, Ref, Size, StyledElementImpl, StyledProperty, Upcast, Visual, VisualImpl,
    VisualImplExt, VisualTreeAttachmentEventArgs,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Displays an [`IImage`].
#[repr(C)]
pub struct Image {
    base: Control,
    current_drawing_bounds: Cell<Rect>,
    subscribed_to_drawing_image_source: Cell<bool>,
    source_invalidated_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
}

ferro_class!(Image: Control);
ferroui_base::ferro_class_info!(Image { new: Image::new });
ferro_impl_classes!(Image: StyledElementImpl, InteractiveImpl, InputElementImpl);

impl ControlImpl for Image {
    fn on_create_automation_peer(this: &Self) -> Ref<crate::automation::peers::AutomationPeer> {
        crate::automation::peers::ImageAutomationPeer::new(this).upcast()
    }
}

impl FerroObjectImpl for Image {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        if change.property() == Self::source_property().as_property() {
            let (old_value, new_value) = change.get_old_and_new_value::<Option<Rc<dyn IImage>>>();

            this.current_drawing_bounds.set(Rect::default());
            if Self::as_drawing_image(old_value.as_ref()).is_some() && this.subscribed_to_drawing_image_source.get() {
                this.unsubscribe_from_source();
            }

            if let Some(new_drawing_image) = Self::as_drawing_image(new_value.as_ref()) {
                if this.is_attached_to_visual_tree() {
                    this.subscribe_to_source(&new_drawing_image);
                }
            }
        }
    }
}

impl VisualImpl for Image {
    /// Renders the control.
    fn render(this: &Self, context: &mut DrawingContext) {
        let source = this.source();
        let bounds = this.bounds();

        if let Some(source) = source {
            if bounds.width > 0.0 && bounds.height > 0.0 {
                let view_port = Rect::from_size(bounds.size());
                let source_size = source.size();

                let scale = MediaExtensions::calculate_scaling(
                    this.stretch(),
                    bounds.size(),
                    source_size,
                    this.stretch_direction(),
                );
                let scaled_size = source_size * scale;
                let dest_rect = view_port.center_rect(Rect::from_size(scaled_size)).intersect(view_port);
                let source_rect =
                    Rect::from_size(source_size).center_rect(Rect::from_size(dest_rect.size() / scale));

                let visual: &Visual = this.upcast();
                let render_options = RenderOptions {
                    text_rendering_mode: RenderOptions::get_text_rendering_mode(visual),
                    bitmap_interpolation_mode: RenderOptions::get_bitmap_interpolation_mode(visual),
                    edge_mode: RenderOptions::get_edge_mode(visual),
                    bitmap_blending_mode: this.blend_mode(),
                    requires_full_opacity_handling: RenderOptions::get_requires_full_opacity_handling(visual),
                };

                let state = context.push_render_options(render_options);
                context.draw_image_with_rects(&*source, source_rect, dest_rect);
                context.pop(state);
            }
        }
    }

    fn bypass_flow_direction_policies(_this: &Self) -> bool {
        true
    }

    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);

        if !this.subscribed_to_drawing_image_source.get() {
            if let Some(drawing_image) = Self::as_drawing_image(this.source().as_ref()) {
                this.subscribe_to_source(&drawing_image);
            }
        }
    }

    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_detached_from_visual_tree(this, e);

        if this.subscribed_to_drawing_image_source.get() && Self::as_drawing_image(this.source().as_ref()).is_some() {
            this.unsubscribe_from_source();
        }
    }
}

impl LayoutableImpl for Image {
    /// Measures the control.
    fn measure_override(this: &Self, available_size: Size) -> Size {
        let source = this.source();
        let mut result = Size::default();

        if let Some(source) = source {
            result = MediaExtensions::calculate_size(
                this.stretch(),
                available_size,
                source.size(),
                this.stretch_direction(),
            );
        }

        result
    }

    fn arrange_override(this: &Self, final_size: Size) -> Size {
        let source = this.source();

        if let Some(source) = source {
            let source_size = source.size();
            MediaExtensions::calculate_size(this.stretch(), final_size, source_size, StretchDirection::Both)
        } else {
            Size::default()
        }
    }
}

ferroui_base::ferro_properties! { impl Image {
    ferro_property!(
        /// Defines the `Source` property.
        pub fn source_property() -> StyledProperty<Option<Rc<dyn IImage>>> {
            FerroProperty::register::<Image, _>("Source", None)
        }
    );

    ferro_property!(
        /// Defines the `BlendMode` property.
        pub fn blend_mode_property() -> StyledProperty<BitmapBlendingMode> {
            FerroProperty::register::<Image, _>("BlendMode", BitmapBlendingMode::Unspecified)
        }
    );

    ferro_property!(
        /// Defines the `Stretch` property.
        pub fn stretch_property() -> StyledProperty<Stretch> {
            FerroProperty::register::<Image, _>("Stretch", Stretch::Uniform)
        }
    );

    ferro_property!(
        /// Defines the `StretchDirection` property.
        pub fn stretch_direction_property() -> StyledProperty<StretchDirection> {
            FerroProperty::register::<Image, _>("StretchDirection", StretchDirection::Both)
        }
    );
} }

impl Image {
    fn static_constructor() {
        Visual::affects_render::<Image>(&[
            Self::source_property().as_property(),
            Self::stretch_property().as_property(),
            Self::stretch_direction_property().as_property(),
            Self::blend_mode_property().as_property(),
        ]);
        Layoutable::affects_measure::<Image>(&[
            Self::source_property().as_property(),
            Self::stretch_property().as_property(),
            Self::stretch_direction_property().as_property(),
        ]);
        crate::automation::AutomationProperties::control_type_override_property()
            .override_default_value::<Image>(Some(crate::automation::peers::AutomationControlType::Image));
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: Control::construct(),
            current_drawing_bounds: Cell::new(Rect::default()),
            subscribed_to_drawing_image_source: Cell::new(false),
            source_invalidated_subscription: RefCell::new(None),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets the image that will be displayed.
    pub fn source(&self) -> Option<Rc<dyn IImage>> {
        self.get_value(Self::source_property())
    }

    /// Sets the image that will be displayed.
    pub fn set_source(&self, value: Option<Rc<dyn IImage>>) {
        self.set_value(Self::source_property(), value)
    }

    /// Gets the blend mode for the image.
    pub fn blend_mode(&self) -> BitmapBlendingMode {
        self.get_value(Self::blend_mode_property())
    }

    /// Sets the blend mode for the image.
    pub fn set_blend_mode(&self, value: BitmapBlendingMode) {
        self.set_value(Self::blend_mode_property(), value)
    }

    /// Gets a value controlling how the image will be stretched.
    pub fn stretch(&self) -> Stretch {
        self.get_value(Self::stretch_property())
    }

    /// Sets a value controlling how the image will be stretched.
    pub fn set_stretch(&self, value: Stretch) {
        self.set_value(Self::stretch_property(), value)
    }

    /// Gets a value controlling in what direction the image will be
    /// stretched.
    pub fn stretch_direction(&self) -> StretchDirection {
        self.get_value(Self::stretch_direction_property())
    }

    /// Sets a value controlling in what direction the image will be
    /// stretched.
    pub fn set_stretch_direction(&self, value: StretchDirection) {
        self.set_value(Self::stretch_direction_property(), value)
    }

    /// The image as a drawing image, when it is one.
    fn as_drawing_image(image: Option<&Rc<dyn IImage>>) -> Option<Ref<DrawingImage>> {
        image?.as_object()?.downcast_ref::<DrawingImage>().map(DrawingImage::to_ref)
    }

    fn subscribe_to_source(&self, drawing_image: &Ref<DrawingImage>) {
        self.subscribed_to_drawing_image_source.set(true);

        let weak = self.to_ref().downgrade();
        let subscription = drawing_image.invalidated(move || {
            if let Some(this) = weak.upgrade() {
                this.on_source_invalidated();
            }
        });

        let previous = self.source_invalidated_subscription.replace(Some(subscription));
        if let Some(previous) = previous {
            previous.dispose();
        }
    }

    fn unsubscribe_from_source(&self) {
        self.subscribed_to_drawing_image_source.set(false);

        let subscription = self.source_invalidated_subscription.take();
        if let Some(subscription) = subscription {
            subscription.dispose();
        }
    }

    fn on_source_invalidated(&self) {
        if !self.is_attached_to_visual_tree() {
            return;
        }

        let Some(drawing_image) = Self::as_drawing_image(self.source().as_ref()) else {
            return;
        };

        if let Some(drawing) = drawing_image.drawing() {
            let bounds = drawing.get_bounds();
            if bounds != self.current_drawing_bounds.get() {
                self.invalidate_measure();
            }
            self.current_drawing_bounds.set(bounds);
        }
    }
}

use crate::{Control, ControlImpl, TopLevel};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::{
    IRoutedEventArgs, Interactive, InteractiveImpl, RoutedEvent, RoutedEventArgs,
    RoutedEventHandlerToken, RoutingStrategies,
};
use ferroui_base::layout::{Layoutable, LayoutableImpl};
use ferroui_base::media::imaging::BitmapBlendingMode;
use ferroui_base::media::{
    DrawingContext, DrawingImage, IImage, ImageLoadResult, ImageScaleVariant, ImageVariantContext, MediaExtensions,
    RenderOptions, Stretch, StretchDirection,
};
use ferroui_base::metadata::{MarkupType, MarkupTyped};
use ferroui_base::platform::IPlatformSettings;
use ferroui_base::reactive::Subscription;
use ferroui_base::rendering::composition::{AlphaMask, CompositionBrush};
use ferroui_base::styling::ThemeVariant;
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{FerroLocator, LocatorExtensions};
use ferroui_base::{
    FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Rect, Ref,
    Size, StyledElementImpl, StyledProperty, Upcast, Visual, VisualImpl, VisualImplExt,
    VisualTreeAttachmentEventArgs, ferro_class, ferro_impl_classes, ferro_property,
    ferro_routed_event, ferro_routed_event_args, instantiate,
};
use std::cell::{Cell, RefCell};
use std::error::Error;
use std::rc::{Rc, Weak};

/// Data for an [`ImageFailed`](Image::image_failed_event) event.
#[derive(Clone)]
pub struct ImageFailedEventArgs {
    base: RoutedEventArgs,
    error: Rc<dyn Error>,
}

ferro_routed_event_args!(ImageFailedEventArgs: RoutedEventArgs);

impl ImageFailedEventArgs {
    /// The error reported by the current image source.
    pub fn error(&self) -> &(dyn Error + 'static) {
        self.error.as_ref()
    }

    /// The message reported by the current image source.
    pub fn error_message(&self) -> String {
        self.error.to_string()
    }
}

ferroui_base::ferro_markup_type!(class ImageFailedEventArgs {
    namespace: "FerroUI.Controls",
    handles: [ImageFailedEventArgs],
    this: Rc<dyn IRoutedEventArgs>,
    base: Rc<dyn IRoutedEventArgs>,
    properties: [
        ErrorMessage: String {
            try_get: |args: &Rc<dyn IRoutedEventArgs>| args.downcast_ref::<ImageFailedEventArgs>()
                .map(ImageFailedEventArgs::error_message)
                .ok_or_else(|| "The arguments are not ImageFailedEventArgs.".to_owned())
        },
    ],
});

pub(crate) const MARKUP_TYPES: &[&MarkupType] = &[<ImageFailedEventArgs as MarkupTyped>::MARKUP];

/// Displays an [`IImage`].
#[repr(C)]
pub struct Image {
    base: Control,
    current_drawing_size: Cell<Size>,
    subscribed_to_drawing_image_source: Cell<bool>,
    source_invalidated_subscription: RefCell<Option<Subscription>>,
    source_load: RefCell<Option<ImageLoadWatch>>,
    scale_variant: RefCell<Option<ImageScaleVariant>>,
    scale_changed: RefCell<Option<Subscription>>,
    palette_changed: RefCell<Option<Subscription>>,
    /// The alpha mask of [`get_alpha_mask`](Image::get_alpha_mask), created
    /// on the first request.
    alpha_mask: std::cell::OnceCell<Box<AlphaMask>>,
}

struct ImageLoadWatch {
    lease: Rc<()>,
    subscription: Option<Subscription>,
    pending: Option<ImageLoadResult>,
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

        if change.property() == Self::source_property().as_property()
            && this.source() == change.get_new_value::<Option<Rc<dyn IImage>>>()
        {
            this.scale_variant.take();
            this.select_variant();
            this.watch_displayed_source();
            this.watch_variant_context();
        } else if change.property() == ThemeVariant::actual_theme_variant_property().as_property()
            && this.scale_variant.borrow().is_some()
        {
            this.refresh_variant();
        }
    }
}

impl VisualImpl for Image {
    /// Renders the control.
    fn render(this: &Self, context: &mut DrawingContext) {
        let source = this.displayed_source();
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

        // The mask follows what the image draws (WinUI's
        // `CImage::NotifyRenderContent`).
        if let Some(mask) = this.alpha_mask.get() {
            mask.update_if_available(this.upcast());
        }
    }

    fn bypass_flow_direction_policies(_this: &Self) -> bool {
        true
    }

    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);

        if this.select_variant() {
            this.watch_displayed_source();
            this.invalidate_measure();
            this.invalidate_visual();
        } else if !this.subscribed_to_drawing_image_source.get() {
            if let Some(drawing_image) = Self::as_drawing_image(this.displayed_source().as_ref()) {
                this.subscribe_to_source(&drawing_image);
            }
        }
        this.watch_variant_context();
        let pending = this.source_load.borrow_mut().as_mut().and_then(|watch| {
            watch.pending.take().map(|result| (Rc::downgrade(&watch.lease), result))
        });
        if let Some((lease, result)) = pending {
            Self::queue_source_load(this.to_ref().downgrade(), lease, result);
        }
    }

    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_detached_from_visual_tree(this, e);

        drop(this.scale_changed.take());
        drop(this.palette_changed.take());
        if this.subscribed_to_drawing_image_source.get() {
            this.unsubscribe_from_source();
        }
    }
}

impl LayoutableImpl for Image {
    /// Measures the control.
    fn measure_override(this: &Self, available_size: Size) -> Size {
        let source = this.displayed_source();
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
        let source = this.displayed_source();

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
    ferro_routed_event!(
        /// Defines the `ImageOpened` event.
        pub fn image_opened_event() -> RoutedEvent<RoutedEventArgs> {
            RoutedEvent::register::<Image, _>("ImageOpened", RoutingStrategies::DIRECT)
        }
    );

    ferro_routed_event!(
        /// Defines the `ImageFailed` event.
        pub fn image_failed_event() -> RoutedEvent<ImageFailedEventArgs> {
            RoutedEvent::register::<Image, _>("ImageFailed", RoutingStrategies::DIRECT)
        }
    );

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
            current_drawing_size: Cell::new(Size::default()),
            subscribed_to_drawing_image_source: Cell::new(false),
            source_invalidated_subscription: RefCell::new(None),
            source_load: RefCell::new(None),
            scale_variant: RefCell::new(None),
            scale_changed: RefCell::new(None),
            palette_changed: RefCell::new(None),
            alpha_mask: std::cell::OnceCell::new(),
        }
    }

    /// `Image.GetAlphaMask`: a composition brush that shows the alpha of the
    /// image as it is drawn (its source, stretched and placed), for the
    /// `Mask` of a [`DropShadow`](ferroui_base::rendering::composition::DropShadow)
    /// or another composition effect. The same brush is returned on every
    /// call; it follows the image as it is drawn again (its source, stretch,
    /// size and visibility). `None` when no compositor is available.
    ///
    /// See [`AlphaMask`] for how the mask is drawn.
    pub fn get_alpha_mask(&self) -> Option<Rc<CompositionBrush>> {
        self.alpha_mask.get_or_init(|| Box::new(AlphaMask::new())).get(self.upcast())
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets the image that will be displayed.
    pub fn source(&self) -> Option<Rc<dyn IImage>> {
        self.get_value(Self::source_property())
    }

    /// Gets the concrete image selected for this control's display scale.
    pub fn displayed_source(&self) -> Option<Rc<dyn IImage>> {
        self.scale_variant
            .borrow()
            .as_ref()
            .map(|variant| variant.image.clone())
            .or_else(|| self.source())
    }

    /// Sets the image that will be displayed.
    pub fn set_source(&self, value: Option<Rc<dyn IImage>>) {
        self.set_value(Self::source_property(), value)
    }

    /// Observes successful loads of the source displayed by this image.
    pub fn image_opened(
        &self,
        handler: impl Fn(&Interactive, &RoutedEventArgs) + 'static,
    ) -> RoutedEventHandlerToken {
        self.add_handler(Self::image_opened_event(), handler)
    }

    /// Observes failed loads of the source displayed by this image.
    pub fn image_failed(
        &self,
        handler: impl Fn(&Interactive, &ImageFailedEventArgs) + 'static,
    ) -> RoutedEventHandlerToken {
        self.add_handler(Self::image_failed_event(), handler)
    }

    fn watch_source_load(&self, source: Option<&Rc<dyn IImage>>) {
        drop(self.source_load.take());
        let Some(source) = source else { return };
        let lease = Rc::new(());
        self.source_load.replace(Some(ImageLoadWatch { lease: lease.clone(), subscription: None, pending: None }));
        let active = Rc::downgrade(&lease);
        let target = self.to_ref().downgrade();
        let subscription = source.subscribe_load(Rc::new(move |result| {
            Self::queue_source_load(target.clone(), active.clone(), result.clone());
        }));
        let mut current = self.source_load.borrow_mut();
        if let Some(watch) = current.as_mut().filter(|watch| Rc::ptr_eq(&watch.lease, &lease)) {
            watch.subscription = subscription;
        }
    }

    fn queue_source_load(target: ferroui_base::WeakRef<Self>, lease: Weak<()>, result: ImageLoadResult) {
        Dispatcher::ui_thread().post_local(
            move || {
                if let (Some(target), Some(lease)) = (target.upgrade(), lease.upgrade()) {
                    target.report_source_load(&lease, result);
                }
            },
            DispatcherPriority::NORMAL,
        );
    }

    fn report_source_load(&self, lease: &Rc<()>, result: ImageLoadResult) {
        {
            let mut current = self.source_load.borrow_mut();
            let Some(watch) = current.as_mut().filter(|watch| Rc::ptr_eq(&watch.lease, lease)) else { return };
            if !self.is_attached_to_visual_tree() {
                watch.pending = Some(result);
                return;
            }
        }
        match result {
            Ok(()) => self.raise_event(&RoutedEventArgs::with_event(Self::image_opened_event())),
            Err(error) => self.raise_event(&ImageFailedEventArgs {
                base: RoutedEventArgs::with_event(Self::image_failed_event()),
                error,
            }),
        }
    }

    fn render_scaling(&self) -> f64 {
        TopLevel::get_top_level(Some(self)).map_or(1., |root| root.render_scaling())
    }

    fn select_variant(&self) -> bool {
        let Some(source) = self.source() else { return false };
        let current = self.scale_variant.borrow().as_ref().map(|variant| variant.id);
        let platform_colors = FerroLocator::current()
            .get_service::<dyn IPlatformSettings>()
            .map(|settings| settings.get_color_values())
            .unwrap_or_default();
        let context = ImageVariantContext {
            scaling: self.render_scaling(),
            theme: self.actual_theme_variant(),
            platform_colors,
        };
        let Some(variant) = source.resolve_for_context(&context, current) else { return false };
        if self.source().as_ref() != Some(&source) {
            return false;
        }
        self.scale_variant.replace(Some(variant));
        true
    }

    fn refresh_variant(&self) {
        if self.select_variant() {
            self.watch_displayed_source();
            self.invalidate_measure();
            self.invalidate_visual();
        }
    }

    fn watch_variant_context(&self) {
        drop(self.scale_changed.take());
        drop(self.palette_changed.take());
        if self.scale_variant.borrow().is_none() || !self.is_attached_to_visual_tree() {
            return;
        }
        let Some(root) = TopLevel::get_top_level(Some(self)) else { return };
        let target = self.to_ref().downgrade();
        let refresh = move || {
            if let Some(image) = target.upgrade().filter(|image| image.is_attached_to_visual_tree()) {
                image.refresh_variant();
            }
        };
        self.scale_changed.replace(Some(root.scaling_changed(refresh.clone())));
        if let Some(settings) = FerroLocator::current().get_service::<dyn IPlatformSettings>() {
            let subscription = settings.color_values_changed(Rc::new(move |_| refresh()));
            self.palette_changed.replace(Some(Subscription::new(subscription)));
        }
    }

    fn watch_displayed_source(&self) {
        self.unsubscribe_from_source();
        self.current_drawing_size.set(Size::default());
        let source = self.displayed_source();
        if self.is_attached_to_visual_tree() {
            if let Some(drawing_image) = Self::as_drawing_image(source.as_ref()) {
                self.subscribe_to_source(&drawing_image);
            }
        }
        self.watch_source_load(source.as_ref());
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
        self.current_drawing_size.set(drawing_image.size());

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

        let Some(drawing_image) = Self::as_drawing_image(self.displayed_source().as_ref()) else {
            return;
        };

        let size = drawing_image.size();
        if self.current_drawing_size.replace(size) != size {
            self.invalidate_measure();
        }
    }
}

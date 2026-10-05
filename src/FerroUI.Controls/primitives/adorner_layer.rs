use super::{AdornerHelper, OverlayLayer, VisualLayerManager};
use crate::templates::ITemplateOf;
use crate::{Canvas, CanvasImpl, Control, ControlImpl, PanelImpl, TopLevel};
use ferroui_base::collections::{NotifyCollectionChangedAction, NotifyCollectionChangedEventArgs};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::input::text_input::TransformTrackingHelper;
use ferroui_base::ElementRef;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::{ITransform, MatrixTransform};
use ferroui_base::reactive::IDisposable;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, AttachedProperty, FerroObjectImpl,
    FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Matrix, Nullable, Point, Rect, Ref,
    RelativePoint, RelativeUnit, Size, StyledElement, StyledElementImpl, StyledProperty, Visual, VisualImpl, WeakRef,
};
use std::cell::RefCell;
use std::rc::Rc;

/// Represents a surface for showing adorners.
/// Adorners are always on top of the adorned element and are positioned to
/// stay relative to the adorned element.
#[repr(C)]
pub struct AdornerLayer {
    base: Canvas,
    tracking_helper: Rc<TransformTrackingHelper>,
}

ferro_class!(AdornerLayer: Canvas);
ferro_impl_classes!(AdornerLayer: StyledElementImpl, VisualImpl, InteractiveImpl, InputElementImpl, ControlImpl, PanelImpl, CanvasImpl);

impl FerroObjectImpl for AdornerLayer {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);

        let weak = this.to_ref().downgrade();
        this.children().add_collection_changed(Rc::new(
            move |e: &NotifyCollectionChangedEventArgs<'_, Ref<Control>>| {
                if let Some(this) = weak.upgrade() {
                    this.children_collection_changed(e);
                }
            },
        ));

        this.tracking_helper.set_visual(Some(&this.to_ref().upcast()));
        let weak = this.to_ref().downgrade();
        this.tracking_helper.matrix_changed(move || {
            if let Some(this) = weak.upgrade() {
                this.invalidate_measure();
            }
        });
    }
}

impl LayoutableImpl for AdornerLayer {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        for child in this.children().snapshot().iter() {
            let info = child.get_value(Self::adorned_element_info_property());

            match info.and_then(|info| info.0.adorned_element()) {
                Some(adorned_element) => child.measure(adorned_element.bounds().size()),
                None => child.measure(available_size),
            }
        }

        Size::default()
    }

    fn arrange_override(this: &Self, final_size: Size) -> Size {
        for child in this.children().snapshot().iter() {
            let info = child.get_value(Self::adorned_element_info_property());
            let is_clip_enabled = child.get_value(Self::is_clip_enabled_property());

            let adorned = info.and_then(|info| info.0.adorned_element());

            if let Some(adorned) = adorned {
                child.arrange(Rect::from_size(adorned.bounds().size()));
                let mut transform = adorned.transform_to_visual(this);
                // If somebody decides that having Margin on an adorner is a good idea,
                // we need to compensate for element being positioned at non-(0,0) coords.
                let position = child.bounds().position();
                if let Some(value) = transform {
                    if position != Point::default() {
                        transform = Some(
                            Matrix::create_translation(position.x, position.y)
                                * value
                                * Matrix::create_translation(-position.x, -position.y),
                        );
                    }
                }
                let render_transform: Rc<dyn ITransform> =
                    MatrixTransform::with_matrix(transform.unwrap_or(ZERO_MATRIX)).into();
                child.set_render_transform(Some(render_transform));
                child.set_render_transform_origin(RelativePoint::new(0.0, 0.0, RelativeUnit::Absolute));
                this.update_clip(child, &adorned, is_clip_enabled);
            } else {
                this.arrange_child(child, final_size);
            }
        }

        final_size
    }
}

/// The value the reference uses when the adorned element has no transform
/// to the layer: the all-zero matrix, which collapses the adorner.
const ZERO_MATRIX: Matrix = Matrix::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0);

ferroui_base::ferro_properties! { impl AdornerLayer {
    ferro_property!(
        /// Allows for getting and setting of the adorned element.
        pub fn adorned_element_property() -> AttachedProperty<Option<ElementRef<Visual>>> {
            ValueTypes::register_element_ref::<Visual>();
            let property = FerroProperty::register_attached::<AdornerLayer, Visual, _>("AdornedElement", None);
            property.changed().subscribe(Self::adorned_element_changed);
            property
        }
    );

    ferro_property!(
        /// Allows for controlling clipping of the adorner.
        pub fn is_clip_enabled_property() -> AttachedProperty<bool> {
            let property = FerroProperty::register_attached::<AdornerLayer, Visual, _>("IsClipEnabled", true);
            property.changed().subscribe(Self::adorner_is_clip_enabled_changed);
            property
        }
    );

    ferro_property!(
        /// Allows for getting and setting of the adorner for control.
        pub fn adorner_property() -> AttachedProperty<Option<Ref<Control>>> {
            let property = FerroProperty::register_attached::<AdornerLayer, Visual, _>("Adorner", None);
            property.changed().subscribe(Self::adorner_changed);
            property
        }
    );

    ferro_property!(
        /// Defines the `DefaultFocusAdorner` property.
        pub fn default_focus_adorner_property() -> StyledProperty<Option<Rc<dyn ITemplateOf<Option<Ref<Control>>>>>> {
            FerroProperty::register::<AdornerLayer, _>("DefaultFocusAdorner", None)
        }
    );

    ferro_property!(
        fn adorned_element_info_property() -> AttachedProperty<Option<AdornedElementInfoHandle>> {
            FerroProperty::register_attached::<AdornerLayer, Visual, _>("AdornedElementInfo", None)
        }
    );

    ferro_property!(
        fn saved_adorner_layer_property() -> AttachedProperty<Option<SavedAdornerLayer>> {
            FerroProperty::register_attached::<Visual, Visual, _>("SavedAdornerLayer", None)
        }
    );

    ferro_property!(
        /// The subscriptions to the attachment events of a visual that has
        /// an adorner (the static event handlers of the reference).
        fn adorner_attachment_subscriptions_property() -> AttachedProperty<Option<AttachmentSubscriptions>> {
            FerroProperty::register_attached::<AdornerLayer, Visual, _>("AdornerAttachmentSubscriptions", None)
        }
    );
} }

impl AdornerLayer {
    /// Creates the layer (internal upstream: the layers are created by
    /// their [`VisualLayerManager`]).
    pub(crate) fn new() -> Ref<Self> {
        instantiate(Self { base: Canvas::construct(), tracking_helper: TransformTrackingHelper::new(false) })
    }

    pub fn get_adorned_element(adorner: &Visual) -> Option<Ref<Visual>> {
        ElementRef::resolve(&adorner.get_value(Self::adorned_element_property()))
    }

    pub fn set_adorned_element(adorner: &Visual, adorned: impl Into<Nullable<Visual>>) {
        adorner.set_value(Self::adorned_element_property(), ElementRef::from_nullable(adorned.into().0))
    }

    /// Gets the adorner layer of a visual: the dedicated layer of the
    /// overlay layer the visual is in, otherwise the layer of the nearest
    /// layer manager, otherwise the first one found in its top-level.
    pub fn get_adorner_layer(visual: &Visual) -> Option<Ref<AdornerLayer>> {
        // Check if the visual is inside an OverlayLayer with a dedicated AdornerLayer
        for ancestor in visual.get_self_and_visual_ancestors() {
            if let Some(adorner_layer) = Self::get_direct_adorner_layer(&ancestor) {
                return Some(adorner_layer);
            }
        }

        if let Some(top_level) = TopLevel::get_top_level(Some(visual)) {
            for descendant in top_level.get_visual_descendants() {
                if let Some(adorner_layer) = Self::get_direct_adorner_layer(&descendant) {
                    return Some(adorner_layer);
                }
            }
        }

        None
    }

    fn get_direct_adorner_layer(visual: &Ref<Visual>) -> Option<Ref<AdornerLayer>> {
        if let Some(adorner_layer) = visual.cast::<OverlayLayer>().and_then(|overlay| overlay.adorner_layer()) {
            return Some(adorner_layer);
        }
        if let Some(vlm) = visual.cast::<VisualLayerManager>() {
            return vlm.adorner_layer();
        }
        None
    }

    pub fn get_is_clip_enabled(adorner: &Visual) -> bool {
        adorner.get_value(Self::is_clip_enabled_property())
    }

    pub fn set_is_clip_enabled(adorner: &Visual, is_clip_enabled: bool) {
        adorner.set_value(Self::is_clip_enabled_property(), is_clip_enabled)
    }

    pub fn get_adorner(visual: &Visual) -> Option<Ref<Control>> {
        visual.get_value(Self::adorner_property())
    }

    pub fn set_adorner(visual: &Visual, adorner: impl Into<Nullable<Control>>) {
        visual.set_value(Self::adorner_property(), adorner.into().0)
    }

    /// The default control's focus adorner.
    pub fn default_focus_adorner(&self) -> Option<Rc<dyn ITemplateOf<Option<Ref<Control>>>>> {
        self.get_value(Self::default_focus_adorner_property())
    }

    pub fn set_default_focus_adorner(&self, value: Option<Rc<dyn ITemplateOf<Option<Ref<Control>>>>>) {
        self.set_value(Self::default_focus_adorner_property(), value)
    }

    fn adorner_changed(e: &FerroPropertyChangedEventArgs<'_>) {
        let Some(visual) = e.sender().downcast_ref::<Visual>() else { return };
        let (old_adorner, new_adorner) = e.get_old_and_new_value::<Option<Ref<Control>>>();

        if old_adorner == new_adorner {
            return;
        }

        if let Some(old_adorner) = old_adorner {
            if let Some(subscriptions) = visual.get_value(Self::adorner_attachment_subscriptions_property()) {
                for subscription in subscriptions.0.iter() {
                    subscription.dispose();
                }
            }
            visual.clear_value(Self::adorner_attachment_subscriptions_property());
            Self::detach(visual, &old_adorner);
        }

        if let Some(new_adorner) = new_adorner {
            let weak = visual.to_ref().downgrade();
            let attached = visual.attached_to_visual_tree(move |_| {
                if let Some(visual) = weak.upgrade() {
                    Self::visual_on_attached_to_visual_tree(&visual);
                }
            });
            let weak = visual.to_ref().downgrade();
            let detached = visual.detached_from_visual_tree(move |_| {
                if let Some(visual) = weak.upgrade() {
                    Self::visual_on_detached_from_visual_tree(&visual);
                }
            });
            visual.set_value(
                Self::adorner_attachment_subscriptions_property(),
                Some(AttachmentSubscriptions(Rc::new([attached, detached]))),
            );
            Self::attach(visual, &new_adorner);
        }
    }

    fn visual_on_attached_to_visual_tree(visual: &Visual) {
        if let Some(adorner) = Self::get_adorner(visual) {
            Self::attach(visual, &adorner);
        }
    }

    fn visual_on_detached_from_visual_tree(visual: &Visual) {
        if let Some(adorner) = Self::get_adorner(visual) {
            Self::detach(visual, &adorner);
        }
    }

    fn attach(visual: &Visual, adorner: &Ref<Control>) {
        let layer = AdornerLayer::get_adorner_layer(visual);
        Self::add_visual_adorner(visual, adorner, layer.as_ref());
        visual.set_value(
            Self::saved_adorner_layer_property(),
            layer.map(|layer| SavedAdornerLayer(layer.downgrade())),
        );
    }

    fn detach(visual: &Visual, adorner: &Ref<Control>) {
        let layer = visual.get_value(Self::saved_adorner_layer_property()).and_then(|layer| layer.0.upgrade());
        Self::remove_visual_adorner(adorner, layer.as_ref());
        visual.clear_value(Self::saved_adorner_layer_property());
    }

    fn add_visual_adorner(visual: &Visual, adorner: &Ref<Control>, layer: Option<&Ref<AdornerLayer>>) {
        let Some(layer) = layer else { return };
        if layer.children().contains(adorner) {
            return;
        }

        Self::set_adorned_element(adorner, visual.to_ref());

        adorner.set_parent(visual.to_ref().upcast::<StyledElement>());
        layer.children().add(adorner.clone());
    }

    fn remove_visual_adorner(adorner: &Ref<Control>, layer: Option<&Ref<AdornerLayer>>) {
        let Some(layer) = layer else { return };
        if !layer.children().contains(adorner) {
            return;
        }

        layer.children().remove(adorner.clone());
        adorner.set_parent(None);
    }

    fn adorned_element_changed(e: &FerroPropertyChangedEventArgs<'_>) {
        let Some(adorner) = e.sender().downcast_ref::<Visual>() else { return };
        let adorned = Self::get_adorned_element(adorner);
        if let Some(layer) = adorner.get_visual_parent_of_type::<AdornerLayer>() {
            layer.update_adorned_element(adorner, adorned);
        }
    }

    fn adorner_is_clip_enabled_changed(e: &FerroPropertyChangedEventArgs<'_>) {
        let Some(adorner) = e.sender().downcast_ref::<Visual>() else { return };
        if let Some(info) = adorner.get_value(Self::adorned_element_info_property()) {
            info.0.update_subscription();
            if let Some(layer) = info.0.layer() {
                layer.invalidate_measure();
            }
        }
    }

    fn update_clip(&self, control: &Control, adorned: &Visual, is_enabled: bool) {
        if !is_enabled {
            control.set_clip(None);
            return;
        }

        control.set_clip(AdornerHelper::calculate_adorner_clip(adorned));
    }

    fn children_collection_changed(&self, e: &NotifyCollectionChangedEventArgs<'_, Ref<Control>>) {
        if e.action == NotifyCollectionChangedAction::Add {
            for i in e.new_items.iter() {
                self.update_adorned_element(i, Self::get_adorned_element(i));
            }
        } else {
            // An adorner that left the layer stops watching its adorned
            // element: the subscription references the element and its
            // ancestors, which would keep an adorned visual and the adorner
            // it owns alive through each other. It is renewed when the
            // adorner is added to a layer again.
            for i in e.old_items.iter() {
                if let Some(info) = i.get_value(Self::adorned_element_info_property()) {
                    if let Some(subscription) = info.0.subscription.borrow_mut().take() {
                        subscription.dispose();
                    }
                }
            }
        }

        self.invalidate_arrange();
    }

    fn update_adorned_element(&self, adorner: &Visual, adorned: Option<Ref<Visual>>) {
        if let Some(composition_visual) = adorner.composition_visual() {
            let adorned_visual = adorned.as_ref().and_then(|adorned| adorned.composition_visual());
            composition_visual.set_adorned_visual(adorned_visual.map(|visual| (*visual).clone()));
            composition_visual.set_adorner_is_clipped(Self::get_is_clip_enabled(adorner));
        }

        let mut info = adorner.get_value(Self::adorned_element_info_property());

        if let Some(existing) = &info {
            if let Some(subscription) = existing.0.subscription.borrow().clone() {
                subscription.dispose();
            }

            if adorned.is_none() {
                adorner.clear_value(Self::adorned_element_info_property());
            }
        }

        if let Some(adorned) = adorned {
            let info = match info.take() {
                Some(info) => info,
                None => {
                    let info = AdornedElementInfoHandle(Rc::new(AdornedElementInfo {
                        adorner: adorner.to_ref().downgrade(),
                        layer: RefCell::new(None),
                        subscription: RefCell::new(None),
                        adorned_element: RefCell::new(None),
                    }));
                    adorner.set_value(Self::adorned_element_info_property(), Some(info.clone()));
                    info
                }
            };

            *info.0.layer.borrow_mut() = Some(self.to_ref().downgrade());
            *info.0.adorned_element.borrow_mut() = Some(adorned.downgrade());
            info.0.update_subscription();
        }
    }
}

/// What the layer knows about an adorner: its adorned element and the
/// subscription to the changes that move it. The back references are weak.
struct AdornedElementInfo {
    adorner: WeakRef<Visual>,
    layer: RefCell<Option<WeakRef<AdornerLayer>>>,
    subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    adorned_element: RefCell<Option<WeakRef<Visual>>>,
}

impl AdornedElementInfo {
    fn layer(&self) -> Option<Ref<AdornerLayer>> {
        self.layer.borrow().as_ref().and_then(WeakRef::upgrade)
    }

    fn adorned_element(&self) -> Option<Ref<Visual>> {
        self.adorned_element.borrow().as_ref().and_then(WeakRef::upgrade)
    }

    fn update_subscription(self: &Rc<Self>) {
        let subscription = self.subscription.borrow_mut().take();
        if let Some(subscription) = subscription {
            subscription.dispose();
        }

        if let Some(adorned_element) = self.adorned_element() {
            let include_clip =
                self.adorner.upgrade().map(|adorner| AdornerLayer::get_is_clip_enabled(&adorner)).unwrap_or(true);
            let weak = Rc::downgrade(self);
            let subscription =
                AdornerHelper::subscribe_to_ancestor_property_changes(&adorned_element, include_clip, move || {
                    if let Some(layer) = weak.upgrade().and_then(|info| info.layer()) {
                        layer.invalidate_measure();
                    }
                });
            *self.subscription.borrow_mut() = Some(subscription);
        }
    }
}

#[derive(Clone)]
struct AdornedElementInfoHandle(Rc<AdornedElementInfo>);

impl PartialEq for AdornedElementInfoHandle {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

#[derive(Clone)]
struct SavedAdornerLayer(WeakRef<AdornerLayer>);

impl PartialEq for SavedAdornerLayer {
    fn eq(&self, other: &Self) -> bool {
        self.0.upgrade() == other.0.upgrade()
    }
}

#[derive(Clone)]
struct AttachmentSubscriptions(Rc<[Rc<dyn IDisposable>; 2]>);

impl PartialEq for AttachmentSubscriptions {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

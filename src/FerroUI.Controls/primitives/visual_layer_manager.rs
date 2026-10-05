use super::{AdornerLayer, LightDismissOverlayLayer, OverlayLayer, PopupOverlayLayer, TextSelectorLayer};
use crate::{Control, ControlImpl, Decorator, Panel};
use ferroui_base::controls::ResourcesChangedEventArgs;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{LayoutableImpl, LayoutableImplExt};
use ferroui_base::logical_tree::LogicalTreeAttachmentEventArgs;
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, ObjectType, Rect, Ref, Size, StyledElement,
    StyledElementImpl, StyledElementImplExt, VisualImpl, WeakRef,
};
use std::cell::{Cell, RefCell};

const ADORNER_Z_INDEX: i32 = i32::MAX - 100;
const OVERLAY_Z_INDEX: i32 = i32::MAX - 98;
const LIGHT_DISMISS_OVERLAY_Z_INDEX: i32 = i32::MAX - 97;
const TEXT_SELECTOR_LAYER_Z_INDEX: i32 = i32::MAX - 96;
const POPUP_OVERLAY_Z_INDEX: i32 = i32::MAX - 95;

/// A control that manages multiple layers such as adorners, overlays, text
/// selectors, and popups.
#[repr(C)]
pub struct VisualLayerManager {
    base: Decorator,
    logical_root: RefCell<Option<WeakRef<StyledElement>>>,
    layers: RefCell<Vec<Ref<Control>>>,
    overlay_layer: RefCell<Option<Ref<OverlayLayer>>>,
    enable_adorner_layer: Cell<bool>,
    enable_overlay_layer: Cell<bool>,
    enable_popup_overlay_layer: Cell<bool>,
    enable_text_selector_layer: Cell<bool>,
}

ferro_class!(VisualLayerManager: Decorator);
ferroui_base::ferro_class_info!(VisualLayerManager { new: VisualLayerManager::new });
ferro_impl_classes!(VisualLayerManager: FerroObjectImpl, VisualImpl, InteractiveImpl, InputElementImpl, ControlImpl);

impl StyledElementImpl for VisualLayerManager {
    fn notify_child_resources_changed(this: &Self, e: ResourcesChangedEventArgs) {
        for layer in this.layers_snapshot() {
            layer.notify_resources_changed(e, true);
        }

        Self::parent_notify_child_resources_changed(this, e);
    }

    fn on_attached_to_logical_tree(this: &Self, e: &LogicalTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_logical_tree(this, e);
        *this.logical_root.borrow_mut() = Some(e.root().downgrade());

        for layer in this.layers_snapshot() {
            layer.notify_attached_to_logical_tree(e);
        }
    }

    fn on_detached_from_logical_tree(this: &Self, e: &LogicalTreeAttachmentEventArgs) {
        *this.logical_root.borrow_mut() = None;
        Self::parent_on_detached_from_logical_tree(this, e);

        for layer in this.layers_snapshot() {
            layer.notify_detached_from_logical_tree(e);
        }
    }
}

impl LayoutableImpl for VisualLayerManager {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        for layer in this.layers_snapshot() {
            layer.measure(available_size);
        }

        Self::parent_measure_override(this, available_size)
    }

    fn arrange_override(this: &Self, final_size: Size) -> Size {
        for layer in this.layers_snapshot() {
            layer.arrange(Rect::from_size(final_size));
        }

        Self::parent_arrange_override(this, final_size)
    }
}

impl VisualLayerManager {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: Decorator::construct(),
            logical_root: RefCell::new(None),
            layers: RefCell::new(Vec::new()),
            overlay_layer: RefCell::new(None),
            enable_adorner_layer: Cell::new(true),
            enable_overlay_layer: Cell::new(false),
            enable_popup_overlay_layer: Cell::new(false),
            enable_text_selector_layer: Cell::new(false),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Whether an [`AdornerLayer`] is created for this manager. When
    /// enabled, the adorner layer is added to the visual tree, providing a
    /// dedicated layer for rendering adorners.
    pub fn enable_adorner_layer(&self) -> bool {
        self.enable_adorner_layer.get()
    }

    pub fn set_enable_adorner_layer(&self, value: bool) {
        self.enable_adorner_layer.set(value)
    }

    /// Whether an [`OverlayLayer`] is created for this manager. When
    /// enabled, the overlay layer is added to the visual tree, providing a
    /// dedicated layer for rendering overlay visuals.
    pub fn enable_overlay_layer(&self) -> bool {
        self.enable_overlay_layer.get()
    }

    pub fn set_enable_overlay_layer(&self, value: bool) {
        self.enable_overlay_layer.set(value)
    }

    /// Whether a [`PopupOverlayLayer`] is created for this manager
    /// (internal upstream: set by the top-levels that host their popups in
    /// an overlay).
    pub fn enable_popup_overlay_layer(&self) -> bool {
        self.enable_popup_overlay_layer.get()
    }

    pub fn set_enable_popup_overlay_layer(&self, value: bool) {
        self.enable_popup_overlay_layer.set(value)
    }

    /// Whether a [`TextSelectorLayer`] is created for this manager. When
    /// enabled, the layer is added to the visual tree, providing a
    /// dedicated layer for rendering text selection handles.
    pub fn enable_text_selector_layer(&self) -> bool {
        self.enable_text_selector_layer.get()
    }

    pub fn set_enable_text_selector_layer(&self, value: bool) {
        self.enable_text_selector_layer.set(value)
    }

    /// The adorner layer, created on first use; `None` when the layer is
    /// not enabled (internal upstream).
    pub fn adorner_layer(&self) -> Option<Ref<AdornerLayer>> {
        if !self.enable_adorner_layer() {
            return None;
        }

        let mut rv = self.find_layer::<AdornerLayer>();
        if rv.is_none() {
            let layer = AdornerLayer::new();
            self.add_layer(layer.clone().upcast(), ADORNER_Z_INDEX);
            rv = Some(layer);
        }
        rv
    }

    /// The layer that hosts popups shown inside the top-level, created on
    /// first use; `None` when the layer is not enabled (internal upstream).
    pub fn popup_overlay_layer(&self) -> Option<Ref<PopupOverlayLayer>> {
        if !self.enable_popup_overlay_layer() {
            return None;
        }

        let mut rv = self.find_layer::<PopupOverlayLayer>();
        if rv.is_none() {
            let layer = PopupOverlayLayer::new();
            self.add_layer(layer.clone().upcast(), POPUP_OVERLAY_Z_INDEX);
            rv = Some(layer);
        }
        rv
    }

    /// The overlay layer, created on first use together with its dedicated
    /// adorner layer; `None` when the layer is not enabled (internal
    /// upstream).
    pub fn overlay_layer(&self) -> Option<Ref<OverlayLayer>> {
        if !self.enable_overlay_layer() {
            return None;
        }

        let existing = self.overlay_layer.borrow().clone();
        if existing.is_some() {
            return existing;
        }

        let overlay_layer = OverlayLayer::new();
        *self.overlay_layer.borrow_mut() = Some(overlay_layer.clone());
        let adorner = AdornerLayer::new();
        overlay_layer.set_adorner_layer(Some(adorner.clone()));

        let panel = Panel::new();
        panel.children().add(overlay_layer.clone());
        panel.children().add(adorner);

        self.add_layer(panel.upcast(), OVERLAY_Z_INDEX);

        Some(overlay_layer)
    }

    /// The layer of the text selection handles, created on first use;
    /// `None` when the layer is not enabled (internal upstream).
    pub fn text_selector_layer(&self) -> Option<Ref<TextSelectorLayer>> {
        if !self.enable_text_selector_layer() {
            return None;
        }

        let mut rv = self.find_layer::<TextSelectorLayer>();
        if rv.is_none() {
            let layer = TextSelectorLayer::new();
            self.add_layer(layer.clone().upcast(), TEXT_SELECTOR_LAYER_Z_INDEX);
            rv = Some(layer);
        }
        rv
    }

    /// The layer that dismisses popups when the user clicks outside them,
    /// created on first use (internal upstream).
    pub fn light_dismiss_overlay_layer(&self) -> Ref<LightDismissOverlayLayer> {
        match self.find_layer::<LightDismissOverlayLayer>() {
            Some(rv) => rv,
            None => {
                let rv = LightDismissOverlayLayer::new();
                self.add_layer(rv.clone().upcast(), LIGHT_DISMISS_OVERLAY_Z_INDEX);
                rv
            }
        }
    }

    fn layers_snapshot(&self) -> Vec<Ref<Control>> {
        self.layers.borrow().clone()
    }

    fn find_layer<T: ObjectType>(&self) -> Option<Ref<T>> {
        self.layers.borrow().iter().find_map(|layer| layer.cast::<T>())
    }

    fn add_layer(&self, layer: Ref<Control>, z_index: i32) {
        self.layers.borrow_mut().push(layer.clone());
        layer.set_parent(self.to_ref());
        layer.set_z_index(z_index);
        self.visual_children().add(layer.clone().upcast());
        if self.is_attached_to_logical_tree() {
            let logical_root = self
                .logical_root
                .borrow()
                .as_ref()
                .and_then(WeakRef::upgrade)
                .expect("an attached layer manager knows its logical root");
            layer.notify_attached_to_logical_tree(&LogicalTreeAttachmentEventArgs::new(
                logical_root,
                layer.clone().upcast(),
                Some(self.to_ref().upcast()),
            ));
        }
        self.invalidate_arrange();
    }
}

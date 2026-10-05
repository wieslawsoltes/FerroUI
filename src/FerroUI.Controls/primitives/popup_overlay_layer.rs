use super::VisualLayerManager;
use crate::{Canvas, CanvasImpl, ControlImpl, PanelImpl, TopLevel};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{LayoutableImpl, LayoutableImplExt};
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Ref, Size, StyledElementImpl, Visual, VisualImpl,
};
use std::cell::Cell;

/// The surface that hosts the popups shown inside their top-level (internal
/// upstream).
#[repr(C)]
pub struct PopupOverlayLayer {
    base: Canvas,
    available_size: Cell<Size>,
}

ferro_class!(PopupOverlayLayer: Canvas);
ferroui_base::ferro_class_info!(PopupOverlayLayer { new: PopupOverlayLayer::new });
ferro_impl_classes!(
    PopupOverlayLayer: FerroObjectImpl,
    StyledElementImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    PanelImpl,
    CanvasImpl
);

impl VisualImpl for PopupOverlayLayer {
    fn bypass_flow_direction_policies(_this: &Self) -> bool {
        true
    }
}

impl LayoutableImpl for PopupOverlayLayer {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        for child in this.children().snapshot().iter() {
            child.measure(available_size);
        }
        available_size
    }

    fn arrange_override(this: &Self, final_size: Size) -> Size {
        // We are saving it here since child controls might need to know the entire size of the overlay
        // and Bounds won't be updated in time
        this.available_size.set(final_size);
        Self::parent_arrange_override(this, final_size)
    }
}

impl PopupOverlayLayer {
    pub fn new() -> Ref<Self> {
        instantiate(Self { base: Canvas::construct(), available_size: Cell::new(Size::default()) })
    }

    /// The size the layer was last arranged with.
    pub fn available_size(&self) -> Size {
        self.available_size.get()
    }

    /// Retrieves the popup overlay layer associated with the specified
    /// visual, if any.
    pub fn get_popup_overlay_layer(visual: &Visual) -> Option<Ref<PopupOverlayLayer>> {
        for v in visual.get_self_and_visual_ancestors() {
            if let Some(layer) = v.cast::<VisualLayerManager>().and_then(|manager| manager.popup_overlay_layer()) {
                return Some(layer);
            }
        }

        if let Some(tl) = TopLevel::get_top_level(Some(visual)) {
            let layers = tl.get_visual_descendants().find_map(|v| v.cast::<VisualLayerManager>());
            return layers.and_then(|layers| layers.popup_overlay_layer());
        }

        None
    }
}

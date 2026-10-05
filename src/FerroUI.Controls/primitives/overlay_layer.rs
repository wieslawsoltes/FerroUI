use super::{AdornerLayer, VisualLayerManager};
use crate::{Canvas, CanvasImpl, ControlImpl, PanelImpl, TopLevel};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{LayoutableImpl, LayoutableImplExt};
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Ref, Size, StyledElementImpl, Visual, VisualImpl,
};
use std::cell::{Cell, RefCell};

/// Represents a surface for showing overlays.
/// Overlays are displayed on top of other elements, but behind popups.
#[repr(C)]
pub struct OverlayLayer {
    base: Canvas,
    available_size: Cell<Size>,
    adorner_layer: RefCell<Option<Ref<AdornerLayer>>>,
}

ferro_class!(OverlayLayer: Canvas);
ferro_impl_classes!(
    OverlayLayer: FerroObjectImpl,
    StyledElementImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    PanelImpl,
    CanvasImpl
);

impl VisualImpl for OverlayLayer {
    fn bypass_flow_direction_policies(_this: &Self) -> bool {
        true
    }
}

impl LayoutableImpl for OverlayLayer {
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

impl OverlayLayer {
    /// Creates the layer (internal upstream: the layer is created by its
    /// [`VisualLayerManager`]).
    pub(crate) fn new() -> Ref<Self> {
        instantiate(Self {
            base: Canvas::construct(),
            available_size: Cell::new(Size::default()),
            adorner_layer: RefCell::new(None),
        })
    }

    /// The size the layer was last arranged with.
    pub fn available_size(&self) -> Size {
        self.available_size.get()
    }

    /// The dedicated adorner layer for this overlay layer (internal
    /// upstream).
    pub fn adorner_layer(&self) -> Option<Ref<AdornerLayer>> {
        self.adorner_layer.borrow().clone()
    }

    pub(crate) fn set_adorner_layer(&self, value: Option<Ref<AdornerLayer>>) {
        *self.adorner_layer.borrow_mut() = value;
    }

    /// Retrieves the overlay layer associated with the specified visual, if
    /// any.
    pub fn get_overlay_layer(visual: &Visual) -> Option<Ref<OverlayLayer>> {
        for v in visual.get_self_and_visual_ancestors() {
            if let Some(layer) = v.cast::<VisualLayerManager>().and_then(|manager| manager.overlay_layer()) {
                return Some(layer);
            }
        }

        if let Some(tl) = TopLevel::get_top_level(Some(visual)) {
            let layers = tl.get_visual_descendants().find_map(|v| v.cast::<VisualLayerManager>());
            return layers.and_then(|layers| layers.overlay_layer());
        }

        None
    }
}

use super::VisualLayerManager;
use crate::{Canvas, CanvasImpl, Control, ControlImpl, PanelImpl, TopLevel};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{LayoutableImpl, LayoutableImplExt};
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, IntoRef, Ref, Size, StyledElementImpl, Visual,
    VisualImpl,
};
use std::cell::Cell;

/// The surface that hosts the text selection handles.
#[repr(C)]
pub struct TextSelectorLayer {
    base: Canvas,
    available_size: Cell<Size>,
}

ferro_class!(TextSelectorLayer: Canvas);
ferroui_base::ferro_class_info!(TextSelectorLayer { new: TextSelectorLayer::new });
ferro_impl_classes!(
    TextSelectorLayer: FerroObjectImpl,
    StyledElementImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    PanelImpl,
    CanvasImpl
);

impl VisualImpl for TextSelectorLayer {
    fn bypass_flow_direction_policies(_this: &Self) -> bool {
        true
    }
}

impl LayoutableImpl for TextSelectorLayer {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        for child in this.children().snapshot().iter() {
            child.measure(available_size);
        }
        Size::default()
    }

    fn arrange_override(this: &Self, final_size: Size) -> Size {
        this.available_size.set(final_size);
        Self::parent_arrange_override(this, final_size)
    }
}

impl TextSelectorLayer {
    pub fn new() -> Ref<Self> {
        instantiate(Self { base: Canvas::construct(), available_size: Cell::new(Size::default()) })
    }

    /// The size the layer was last arranged with.
    pub fn available_size(&self) -> Size {
        self.available_size.get()
    }

    /// Retrieves the text selector layer associated with the specified
    /// visual, if any.
    pub fn get_text_selector_layer(visual: &Visual) -> Option<Ref<TextSelectorLayer>> {
        for v in visual.get_self_and_visual_ancestors() {
            if let Some(layer) = v.cast::<VisualLayerManager>().and_then(|manager| manager.text_selector_layer()) {
                return Some(layer);
            }
        }

        if let Some(tl) = TopLevel::get_top_level(Some(visual)) {
            let layers = tl.get_visual_descendants().find_map(|v| v.cast::<VisualLayerManager>());
            return layers.and_then(|layers| layers.text_selector_layer());
        }

        None
    }

    /// Adds a control to the layer.
    pub fn add(&self, control: impl IntoRef<Control>) {
        self.children().add(control);
    }

    /// Removes a control from the layer.
    pub fn remove(&self, control: impl IntoRef<Control>) {
        self.children().remove(control);
    }
}

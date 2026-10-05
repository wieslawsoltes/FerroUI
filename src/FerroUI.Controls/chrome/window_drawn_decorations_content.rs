use crate::Control;
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Nullable, Ref, StyledElement, StyledElementImpl,
};
use std::cell::RefCell;

/// Holds the template content for
/// [`WindowDrawnDecorations`](super::WindowDrawnDecorations). Contains three
/// visual slots: underlay, overlay, and fullscreen popover.
#[repr(C)]
pub struct WindowDrawnDecorationsContent {
    base: StyledElement,
    overlay: RefCell<Option<Ref<Control>>>,
    underlay: RefCell<Option<Ref<Control>>>,
    fullscreen_popover: RefCell<Option<Ref<Control>>>,
}

ferro_class!(WindowDrawnDecorationsContent: StyledElement);
ferroui_base::ferro_class_info!(WindowDrawnDecorationsContent { new: WindowDrawnDecorationsContent::new });
ferro_impl_classes!(WindowDrawnDecorationsContent: FerroObjectImpl, StyledElementImpl);

impl WindowDrawnDecorationsContent {
    /// Field initialisation only.
    pub fn construct() -> Self {
        Self {
            base: StyledElement::construct(),
            overlay: RefCell::new(None),
            underlay: RefCell::new(None),
            fullscreen_popover: RefCell::new(None),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The overlay layer content (titlebar, caption buttons). Positioned
    /// above the client area.
    pub fn overlay(&self) -> Option<Ref<Control>> {
        self.overlay.borrow().clone()
    }

    pub fn set_overlay(&self, value: impl Into<Nullable<Control>>) {
        self.handle_logical_child(&self.overlay, value.into().0)
    }

    /// The underlay layer content (borders, background, shadow area).
    /// Positioned below the client area.
    pub fn underlay(&self) -> Option<Ref<Control>> {
        self.underlay.borrow().clone()
    }

    pub fn set_underlay(&self, value: impl Into<Nullable<Control>>) {
        self.handle_logical_child(&self.underlay, value.into().0)
    }

    /// The fullscreen popover content. Shown when the user hovers the
    /// pointer at the top of the window in fullscreen mode.
    pub fn fullscreen_popover(&self) -> Option<Ref<Control>> {
        self.fullscreen_popover.borrow().clone()
    }

    pub fn set_fullscreen_popover(&self, value: impl Into<Nullable<Control>>) {
        self.handle_logical_child(&self.fullscreen_popover, value.into().0)
    }

    fn handle_logical_child(&self, field: &RefCell<Option<Ref<Control>>>, value: Option<Ref<Control>>) {
        if *field.borrow() == value {
            return;
        }
        let old = field.replace(value.clone());
        if let Some(old) = old {
            self.logical_children().remove(&old.clone().upcast());
            old.set_parent(None);
        }

        if let Some(new) = value {
            self.logical_children().add(new.clone().upcast());
            new.set_parent(self.to_ref().upcast::<StyledElement>());
        }
    }
}

//! The drawn decorations part of the host of a top-level.

use crate::automation::AutomationProperties;
use crate::chrome::{DrawnWindowDecorationParts, ResizeGripLayer, WindowDrawnDecorations};
use crate::top_level_host::TopLevelHost;
use crate::{Control, ControlImpl, Window, WindowState};
use ferroui_base::input::{InputElementImpl, InputElementImplExt, PointerEventArgs};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::reactive::{CompositeDisposable, IDisposable, ObservableExt};
use ferroui_base::styling::ControlTheme;
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroObject, FerroObjectExtensions, FerroObjectImpl, Ref,
    StyledElementImpl, Thickness, Visual, VisualImpl,
};
use std::cell::RefCell;
use std::rc::Rc;

/// Wrapper that holds a single visual child, used to host decoration layer
/// content extracted from the decorations template.
#[repr(C)]
pub(crate) struct LayerWrapper {
    base: Control,
    inner: RefCell<Option<Ref<Control>>>,
}

ferro_class!(LayerWrapper: Control);
ferro_impl_classes!(
    LayerWrapper: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

impl LayerWrapper {
    fn new() -> Ref<Self> {
        instantiate(Self { base: Control::construct(), inner: RefCell::new(None) })
    }

    #[allow(dead_code)]
    pub(crate) fn inner(&self) -> Option<Ref<Control>> {
        self.inner.borrow().clone()
    }

    fn set_inner(&self, value: Option<Ref<Control>>) {
        if *self.inner.borrow() == value {
            return;
        }
        let old = self.inner.replace(value.clone());
        if let Some(old) = old {
            self.visual_children().remove(&old.upcast());
        }
        if let Some(new) = value {
            self.visual_children().add(new.upcast());
        }
    }
}

const POPOVER_TRIGGER_ZONE_HEIGHT: f64 = 1.0;

impl InputElementImpl for TopLevelHost {
    fn on_pointer_moved(this: &Self, e: &PointerEventArgs) {
        Self::parent_on_pointer_moved(this, e);

        let fullscreen_popover = this.fullscreen_popover.borrow().clone();
        if let (true, Some(fullscreen_popover)) = (this.fullscreen_popover_enabled.get(), fullscreen_popover) {
            let visual: &Visual = this;
            let pos = e.get_position(Some(visual));
            // Use the default title bar height since the title bar height is 0 in fullscreen
            let title_bar_height =
                this.decorations().map(|decorations| decorations.default_title_bar_height()).unwrap_or(30.0);

            if !fullscreen_popover.is_visible() && pos.y <= POPOVER_TRIGGER_ZONE_HEIGHT {
                fullscreen_popover.set_is_visible(true);
                this.invalidate_decorations_overlay_peer_children();
            } else if pos.y > title_bar_height && fullscreen_popover.is_visible() {
                fullscreen_popover.set_is_visible(false);
                this.invalidate_decorations_overlay_peer_children();
            }
        }
    }
}

impl TopLevelHost {
    /// The current drawn decorations instance, if active.
    pub(crate) fn decorations(&self) -> Option<Ref<WindowDrawnDecorations>> {
        self.decorations.borrow().clone()
    }

    /// The layer below the top-level that hosts the underlay of the drawn
    /// decorations, if active.
    pub(crate) fn underlay_layer(&self) -> Option<Ref<LayerWrapper>> {
        self.underlay.borrow().clone()
    }

    /// The layer above the top-level that hosts the overlay of the drawn
    /// decorations, if active.
    pub(crate) fn overlay_layer(&self) -> Option<Ref<LayerWrapper>> {
        self.overlay.borrow().clone()
    }

    /// The layer that hosts the fullscreen popover of the drawn decorations,
    /// if active.
    pub(crate) fn fullscreen_popover_layer(&self) -> Option<Ref<LayerWrapper>> {
        self.fullscreen_popover.borrow().clone()
    }

    /// The resize grips of the drawn decorations, if active.
    pub(crate) fn resize_grips(&self) -> Option<Ref<ResizeGripLayer>> {
        self.resize_grips.borrow().clone()
    }

    fn host_window(&self) -> Option<Ref<Window>> {
        self.top_level().and_then(|top_level| top_level.cast::<Window>())
    }

    /// Updates drawn window decorations with the specified parts and window
    /// state. When `parts` is `None`, decorations are removed entirely. When
    /// present (including [`DrawnWindowDecorationParts::NONE`]), the
    /// decoration infrastructure is kept alive and the parts and fullscreen
    /// state are updated.
    pub(crate) fn update_drawn_decorations(
        &self,
        parts: Option<DrawnWindowDecorationParts>,
        window_state: WindowState,
        theme: Option<Ref<ControlTheme>>,
    ) {
        let Some(enabled_parts) = parts else {
            self.remove_decorations();
            return;
        };

        if let Some(decorations) = self.decorations() {
            // Layers persist across part changes; pseudo-classes driven by the enabled parts
            // control visibility of individual decoration elements in the theme.
            decorations.set_enabled_parts(enabled_parts);

            let old_theme = decorations.theme();
            if old_theme != theme {
                decorations.set_theme(theme);
                decorations.apply_styling();
            }

            if let Some(resize_grips) = self.resize_grips() {
                resize_grips.set_is_visible(enabled_parts.contains(DrawnWindowDecorationParts::RESIZE_GRIPS));
            }
        } else {
            let decorations = WindowDrawnDecorations::new();
            decorations.set_enabled_parts(enabled_parts);
            decorations.set_theme(theme);
            *self.decorations.borrow_mut() = Some(decorations.clone());

            // Set up logical parenting
            self.logical_children().add(decorations.clone().upcast());

            // Create layer wrappers
            let underlay = LayerWrapper::new();
            AutomationProperties::set_automation_id(&underlay, Some("WindowChromeUnderlay"));
            let overlay = LayerWrapper::new();
            AutomationProperties::set_automation_id(&overlay, Some("WindowChromeOverlay"));
            let fullscreen_popover = LayerWrapper::new();
            fullscreen_popover.set_is_visible(false);
            AutomationProperties::set_automation_id(&fullscreen_popover, Some("PopoverWindowChrome"));
            *self.underlay.borrow_mut() = Some(underlay.clone());
            *self.overlay.borrow_mut() = Some(overlay.clone());
            *self.fullscreen_popover.borrow_mut() = Some(fullscreen_popover.clone());

            // Insert layers: underlay below the top-level, overlay and popover above
            // Visual order: underlay(0), top-level(1), overlay(2), fullscreen popover(3), resize grips(4)
            self.visual_children().insert(0, underlay.upcast());
            self.visual_children().add(overlay.upcast());
            self.visual_children().add(fullscreen_popover.upcast());

            // Always create resize grips; visibility is controlled by the enabled parts
            let resize_grips = ResizeGripLayer::new();
            resize_grips.set_is_visible(enabled_parts.contains(DrawnWindowDecorationParts::RESIZE_GRIPS));
            *self.resize_grips.borrow_mut() = Some(resize_grips.clone());
            self.visual_children().add(resize_grips.upcast());

            // Attach to window if available
            if let Some(window) = self.host_window() {
                decorations.attach(&window);
            }

            // Subscribe to template changes to re-apply and geometry changes for resize grips
            let weak = self.to_ref().downgrade();
            let geometry_subscription = decorations.effective_geometry_changed({
                let weak = weak.clone();
                move || {
                    if let Some(this) = weak.upgrade() {
                        this.on_decorations_geometry_changed();
                    }
                }
            });
            let object: &FerroObject = &decorations;
            let template_subscription =
                FerroObjectExtensions::get_observable(object, WindowDrawnDecorations::template_property())
                    .subscribe_fn(move |_| {
                        if let Some(this) = weak.upgrade() {
                            this.apply_decorations_template();
                        }
                    });
            let subscriptions: Rc<dyn IDisposable> =
                Rc::new(CompositeDisposable::from_disposables([template_subscription, geometry_subscription]));
            *self.decorations_subscriptions.borrow_mut() = Some(subscriptions);

            self.apply_decorations_template();
            self.invalidate_measure();
        }

        self.apply_fullscreen_state(window_state == WindowState::FullScreen);
    }

    /// Removes drawn window decorations and all associated layers.
    fn remove_decorations(&self) {
        let Some(decorations) = self.decorations() else { return };

        let subscriptions = self.decorations_subscriptions.borrow_mut().take();
        if let Some(subscriptions) = subscriptions {
            subscriptions.dispose();
        }

        decorations.detach();

        // Remove layers
        let underlay = self.underlay.borrow_mut().take();
        if let Some(underlay) = underlay {
            self.visual_children().remove(&underlay.upcast());
        }
        let overlay = self.overlay.borrow_mut().take();
        if let Some(overlay) = overlay {
            self.visual_children().remove(&overlay.upcast());
        }
        let fullscreen_popover = self.fullscreen_popover.borrow_mut().take();
        if let Some(fullscreen_popover) = fullscreen_popover {
            self.visual_children().remove(&fullscreen_popover.upcast());
        }
        let resize_grips = self.resize_grips.borrow_mut().take();
        if let Some(resize_grips) = resize_grips {
            self.visual_children().remove(&resize_grips.upcast());
        }

        // Clean up logical tree
        self.logical_children().remove(&decorations.upcast());
        *self.decorations.borrow_mut() = None;
        self.invalidate_decorations_overlay_peer_children();
    }

    fn apply_decorations_template(&self) {
        let Some(decorations) = self.decorations() else { return };

        decorations.apply_styling();
        decorations.apply_template();

        let content = decorations.content();
        if let Some(underlay) = self.underlay_layer() {
            underlay.set_inner(content.as_ref().and_then(|content| content.underlay()));
        }
        if let Some(overlay) = self.overlay_layer() {
            overlay.set_inner(content.as_ref().and_then(|content| content.overlay()));
        }
        if let Some(fullscreen_popover) = self.fullscreen_popover_layer() {
            fullscreen_popover.set_inner(content.as_ref().and_then(|content| content.fullscreen_popover()));
        }

        self.update_resize_grip_thickness();
    }

    fn update_resize_grip_thickness(&self) {
        let (Some(resize_grips), Some(decorations)) = (self.resize_grips(), self.decorations()) else { return };

        let frame = decorations.frame_thickness();
        let shadow = decorations.shadow_thickness();
        // Grips strictly cover frame + shadow area, never client area
        resize_grips.set_grip_thickness(Thickness::new(
            frame.left + shadow.left,
            frame.top + shadow.top,
            frame.right + shadow.right,
            frame.bottom + shadow.bottom,
        ));
    }

    fn on_decorations_geometry_changed(&self) {
        self.update_resize_grip_thickness();

        // Notify the window to update margins
        if let Some(window) = self.host_window() {
            window.on_drawn_decorations_geometry_changed();
        }
    }

    /// Applies fullscreen-specific layer visibility: hides overlay/underlay
    /// and enables popover hover detection, or restores normal state.
    fn apply_fullscreen_state(&self, is_fullscreen: bool) {
        let Some(fullscreen_popover) = self.fullscreen_popover_layer() else { return };

        if is_fullscreen {
            // In fullscreen mode, hide overlay and underlay, enable popover hover detection
            if let Some(overlay) = self.overlay_layer() {
                overlay.set_is_visible(false);
            }
            if let Some(underlay) = self.underlay_layer() {
                underlay.set_is_visible(false);
            }
            // Popover starts hidden, will show on hover at top edge
            fullscreen_popover.set_is_visible(false);
            self.fullscreen_popover_enabled.set(true);
        } else {
            // Not fullscreen: show overlay and underlay, hide popover
            if let Some(overlay) = self.overlay_layer() {
                overlay.set_is_visible(true);
            }
            if let Some(underlay) = self.underlay_layer() {
                underlay.set_is_visible(true);
            }
            fullscreen_popover.set_is_visible(false);
            self.fullscreen_popover_enabled.set(false);
        }
        self.invalidate_decorations_overlay_peer_children();
    }

    /// Invalidates the children of the peer of the decoration layers, if
    /// the peer exists.
    fn invalidate_decorations_overlay_peer_children(&self) {
        let peer = self.decorations_overlay_peer.borrow().clone();
        if let Some(peer) = peer {
            peer.invalidate_children();
        }
    }
}

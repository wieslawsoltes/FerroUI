use crate::{Control, TopLevel};
use ferroui_base::input::{KeyboardNavigation, KeyboardNavigationMode};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{Layoutable, LayoutableImpl};
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Rect, Ref, Size,
    StyledElementImpl, Thickness, VisualImpl, WeakRef,
};
use std::cell::{Cell, RefCell};

/// Hosts the top-level and, when enabled, the drawn decoration layers
/// (underlay, overlay, fullscreen popover). Serves as the visual root of the
/// presentation source.
#[repr(C)]
pub struct TopLevelHost {
    base: Control,
    pub(crate) top_level: RefCell<WeakRef<TopLevel>>,
    decoration_inset: Cell<Thickness>,
    pub(crate) decorations: RefCell<Option<Ref<crate::chrome::WindowDrawnDecorations>>>,
    pub(crate) underlay: RefCell<Option<Ref<crate::top_level_host_decorations::LayerWrapper>>>,
    pub(crate) overlay: RefCell<Option<Ref<crate::top_level_host_decorations::LayerWrapper>>>,
    pub(crate) fullscreen_popover: RefCell<Option<Ref<crate::top_level_host_decorations::LayerWrapper>>>,
    pub(crate) resize_grips: RefCell<Option<Ref<crate::chrome::ResizeGripLayer>>>,
    pub(crate) decorations_subscriptions: RefCell<Option<std::rc::Rc<dyn ferroui_base::reactive::IDisposable>>>,
    pub(crate) fullscreen_popover_enabled: Cell<bool>,
    pub(crate) decorations_overlay_peer:
        RefCell<Option<Ref<crate::top_level_host_peers::DecorationsOverlaysAutomationPeer>>>,
}

ferro_class!(TopLevelHost: Control);
ferro_impl_classes!(TopLevelHost: StyledElementImpl, InteractiveImpl);
// With the drawn decorations the pointer moved override lives in `top_level_host_decorations.rs`.
// The automation peer override lives in `top_level_host_peers.rs`.
impl FerroObjectImpl for TopLevelHost {}

impl VisualImpl for TopLevelHost {
    fn bypass_flow_direction_policies(_this: &Self) -> bool {
        true
    }
}

impl LayoutableImpl for TopLevelHost {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        let inset = this.decoration_inset.get();
        let has_inset = inset != Thickness::default();
        let mut desired_size = Size::default();
        let top_level = this.top_level();

        for child in this.visual_children().snapshot().iter() {
            let Some(l) = child.downcast_ref::<Layoutable>() else { continue };
            if has_inset && top_level.as_ref().is_some_and(|tl| is_same(l, tl)) {
                // In forced mode, measure the TopLevel with reduced size
                let content_size = Size::new(
                    (available_size.width - inset.left - inset.right).max(0.0),
                    (available_size.height - inset.top - inset.bottom).max(0.0),
                );
                l.measure(content_size);

                // Add inset back so the host's desired size represents the
                // full frame. This ensures the arrange pass receives the full
                // frame size and can correctly position the TopLevel within
                // the inset area.
                let desired = l.desired_size();
                desired_size = Size::new(
                    desired_size.width.max(desired.width + inset.left + inset.right),
                    desired_size.height.max(desired.height + inset.top + inset.bottom),
                );
            } else {
                l.measure(available_size);

                let desired = l.desired_size();
                desired_size =
                    Size::new(desired_size.width.max(desired.width), desired_size.height.max(desired.height));
            }
        }

        desired_size
    }

    fn arrange_override(this: &Self, final_size: Size) -> Size {
        let inset = this.decoration_inset.get();
        let has_inset = inset != Thickness::default();
        let top_level = this.top_level();

        for child in this.visual_children().snapshot().iter() {
            let Some(l) = child.downcast_ref::<Layoutable>() else { continue };
            if has_inset && top_level.as_ref().is_some_and(|tl| is_same(l, tl)) {
                // In forced mode, arrange the TopLevel within the inset area
                let content_size = Size::new(
                    (final_size.width - inset.left - inset.right).max(0.0),
                    (final_size.height - inset.top - inset.bottom).max(0.0),
                );

                l.arrange(Rect::new(inset.left, inset.top, content_size.width, content_size.height));
            } else {
                l.arrange(Rect::from_size(final_size));
            }
        }

        final_size
    }
}

fn is_same(layoutable: &Layoutable, top_level: &Ref<TopLevel>) -> bool {
    let top_level: &Layoutable = top_level;
    std::ptr::eq(layoutable, top_level)
}

impl TopLevelHost {
    fn static_constructor() {
        KeyboardNavigation::tab_navigation_property()
            .override_default_value::<TopLevelHost>(KeyboardNavigationMode::Cycle);
    }

    /// Creates the host of a top-level and makes the top-level its visual
    /// child.
    pub(crate) fn new(tl: &Ref<TopLevel>) -> Ref<Self> {
        let host = instantiate(Self {
            base: Control::construct(),
            top_level: RefCell::new(tl.downgrade()),
            decoration_inset: Cell::new(Thickness::default()),
            decorations: RefCell::new(None),
            underlay: RefCell::new(None),
            overlay: RefCell::new(None),
            fullscreen_popover: RefCell::new(None),
            resize_grips: RefCell::new(None),
            decorations_subscriptions: RefCell::new(None),
            fullscreen_popover_enabled: Cell::new(false),
            decorations_overlay_peer: RefCell::new(None),
        });
        host.visual_children().add(tl.clone().upcast());
        host
    }

    /// The hosted top-level.
    pub(crate) fn top_level(&self) -> Option<Ref<TopLevel>> {
        self.top_level.borrow().upgrade()
    }

    /// The decoration inset applied to the top-level child in forced
    /// decoration mode. When non-zero, the top-level is measured and
    /// arranged within the inset area while decoration layers use the full
    /// available size.
    pub(crate) fn decoration_inset(&self) -> Thickness {
        self.decoration_inset.get()
    }

    pub(crate) fn set_decoration_inset(&self, value: Thickness) {
        if self.decoration_inset.get() == value {
            return;
        }
        self.decoration_inset.set(value);
        self.invalidate_measure();
    }
}

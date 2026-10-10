//! The port of `RootAtSpiNode.cs`: what the node of a window has beyond
//! a node. The reference derives a class; here it is a part of the node
//! (`AtSpiNode::as_root`).

use super::application_at_spi_node::ApplicationAtSpiNode;
use super::at_spi_node::AtSpiNode;
use ferroui_base::reactive::IDisposable;
use ferroui_base::{PixelPoint, Point, Rect};
use ferroui_controls::automation::provider::IRootProvider;
use ferroui_controls::platform::IWindowBaseImpl;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

pub(crate) struct RootAtSpiNode {
    node: Weak<AtSpiNode>,
    root_provider: Rc<dyn IRootProvider>,
    app_root: RefCell<Option<Rc<ApplicationAtSpiNode>>>,
    focus_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    /// Whether the activation callbacks of the window still reach the node.
    window_subscribed: Rc<Cell<bool>>,
}

impl RootAtSpiNode {
    pub(crate) fn new(root_provider: Rc<dyn IRootProvider>, node: Weak<AtSpiNode>) -> Self {
        let weak = node.clone();
        let focus_subscription = root_provider.focus_changed(Rc::new(move || {
            if let Some(node) = weak.upgrade() {
                if let Some(root) = node.as_root() {
                    root.on_root_focus_changed(&node);
                }
            }
        }));

        let this = Self {
            node,
            root_provider,
            app_root: RefCell::new(None),
            focus_subscription: RefCell::new(Some(focus_subscription)),
            window_subscribed: Rc::new(Cell::new(false)),
        };

        // `impl.Activated += OnWindowActivated`: the callbacks of a window
        // are one slot each in the port, so the node's part is called
        // after whatever the slot held, for as long as the node is
        // attached (docs/porting/DEVIATIONS.md).
        let subscribed = this.window_subscribed.clone();
        let hooked = this.with_window_impl(|window| {
            let (previous, weak, flag) = (window.activated(), this.node.clone(), subscribed.clone());
            window.set_activated(Some(Rc::new(move || {
                if let Some(previous) = &previous {
                    previous();
                }
                Self::on_window_activation(&weak, &flag, true);
            })));
            let (previous, weak, flag) = (window.deactivated(), this.node.clone(), subscribed.clone());
            window.set_deactivated(Some(Rc::new(move || {
                if let Some(previous) = &previous {
                    previous();
                }
                Self::on_window_activation(&weak, &flag, false);
            })));
        });
        this.window_subscribed.set(hooked.is_some());
        this
    }

    #[allow(dead_code)] // A member of the reference that nothing of it reads.
    pub(crate) fn root_provider(&self) -> &Rc<dyn IRootProvider> {
        &self.root_provider
    }

    /// Calls `f` with the window of the root (`WindowImpl`); `None`
    /// when the root has no platform window.
    pub(crate) fn with_window_impl<R>(&self, f: impl FnOnce(&dyn IWindowBaseImpl) -> R) -> Option<R> {
        let platform_impl = self.root_provider.platform_impl()?;
        platform_impl.as_window_base_impl().map(f)
    }

    pub(crate) fn app_root(&self) -> Option<Rc<ApplicationAtSpiNode>> {
        self.app_root.borrow().clone()
    }

    pub(crate) fn set_app_root(&self, app_root: Option<Rc<ApplicationAtSpiNode>>) {
        *self.app_root.borrow_mut() = app_root;
    }

    pub(crate) fn to_screen(&self, rect: Rect) -> Rect {
        self.with_window_impl(|window| {
            let top_left = window.point_to_screen(rect.top_left());
            let bottom_right = window.point_to_screen(rect.bottom_right());
            Rect::new(
                f64::from(top_left.x),
                f64::from(top_left.y),
                f64::from(bottom_right.x - top_left.x),
                f64::from(bottom_right.y - top_left.y),
            )
        })
        .unwrap_or_default()
    }

    #[allow(dead_code)] // A member of the reference that nothing of it calls.
    pub(crate) fn point_to_client(&self, point: PixelPoint) -> Point {
        self.with_window_impl(|window| window.point_to_client(point)).unwrap_or_default()
    }

    fn on_root_focus_changed(&self, node: &Rc<AtSpiNode>) {
        let Some(server) = node.server() else { return };
        let focused = self.root_provider.get_focus();
        let mut focused_node = server.try_get_attached_node(focused.as_ref());
        if focused_node.is_none() {
            // Focus can shift before children are queried;
            // refresh visible root children lazily.
            node.ensure_children();
            focused_node = server.try_get_attached_node(focused.as_ref());
        }

        server.emit_focus_change(focused_node.as_ref());
    }

    fn on_window_activation(node: &Weak<AtSpiNode>, subscribed: &Cell<bool>, active: bool) {
        if !subscribed.get() {
            return;
        }
        if let Some(node) = node.upgrade() {
            if let Some(server) = node.server() {
                server.emit_window_activation_change(&node, active);
            }
        }
    }

    /// The part of `Detach` that belongs to the root.
    pub(crate) fn detach(&self) {
        if let Some(subscription) = self.focus_subscription.borrow_mut().take() {
            subscription.dispose();
        }
        self.window_subscribed.set(false);
    }
}

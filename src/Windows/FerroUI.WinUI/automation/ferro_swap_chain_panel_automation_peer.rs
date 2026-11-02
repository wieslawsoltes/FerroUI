//! The automation peer of the panel: a `FrameworkElementAutomationPeer`
//! (composed, `interop/composable.rs`) whose children are the peers of the
//! framework's content, wrapped for XAML's automation tree.

#[cfg(windows)]
pub(crate) use imp::FerroSwapChainPanelAutomationPeer;

#[cfg(windows)]
mod imp {
    use super::super::ferro_to_xaml_peer_proxy::FerroToXamlPeerProxy;
    use crate::ferro_swap_chain_panel::FerroSwapChainPanel;
    use crate::interop::collections::AutomationPeerVector;
    use crate::interop::composable::{compose, Composed, ComposedBase};
    use crate::interop::win_rt::{client_origin_on_screen, factory, flag, get_window_from_window_id, new_string};
    use crate::interop::*;
    use ferroui_base::reactive::Subscription;
    use ferroui_base::{Point as FerroPoint, Rect as FerroRect, Ref};
    use ferroui_controls::automation::peers::{AutomationPeer as FerroPeer, ControlAutomationPeer as FerroControlPeer};
    use ferroui_controls::automation::provider::IEmbeddedRootProvider;
    use ferroui_microcom::{ComPtr, HResult};
    use std::cell::{Cell, RefCell};
    use std::ffi::c_void;
    use std::rc::{Rc, Weak};

    pub(crate) struct FerroSwapChainPanelAutomationPeer {
        this: Weak<FerroSwapChainPanelAutomationPeer>,
        base: ComposedBase,
        /// The outer object, uncounted: it holds this.
        outer: Cell<*mut IAutomationPeer>,
        // The reference holds the panel; the panel's element holds this
        // peer, so the panel is held weakly (docs/porting/winui.md).
        panel: Weak<FerroSwapChainPanel>,
        embedded_root: RefCell<Option<Rc<dyn IEmbeddedRootProvider>>>,
        embedded_root_peer: RefCell<Option<Ref<FerroPeer>>>,
        /// The subscription to `FocusChanged` of the embedded root.
        focus_changed: RefCell<Option<Subscription>>,
    }

    /// The outer object of the composed peer.
    struct PeerOuter(Rc<FerroSwapChainPanelAutomationPeer>);

    impl Composed for PeerOuter {
        fn composed_base(&self) -> &ComposedBase {
            &self.0.base
        }
    }

    inspectable!(PeerOuter, "FerroUI.WinUI.Automation.FerroSwapChainPanelAutomationPeer", [IAutomationPeerOverrides]);

    impl FerroSwapChainPanelAutomationPeer {
        pub(crate) fn new(panel: Rc<FerroSwapChainPanel>) -> Result<ComPtr<IAutomationPeer>, HResult> {
            let owner = panel.as_ui_element().cast::<IFrameworkElement>()?;
            let peer = Rc::new_cyclic(|this| FerroSwapChainPanelAutomationPeer {
                this: this.clone(),
                base: ComposedBase::default(),
                outer: Cell::new(std::ptr::null_mut()),
                panel: Rc::downgrade(&panel),
                embedded_root: RefCell::new(None),
                embedded_root_peer: RefCell::new(None),
                focus_changed: RefCell::new(None),
            });
            let peer_factory = factory::<IFrameworkElementAutomationPeerFactory>(
                "Microsoft.UI.Xaml.Automation.Peers.FrameworkElementAutomationPeer",
            )?;
            let outer = compose::<IAutomationPeerOverrides, PeerOuter, 0>(PeerOuter(peer.clone()), [], |outer, inner| {
                // SAFETY: the owner, the outer object and a place of the
                // caller's frame.
                unsafe { peer_factory.create_instance_with_owner(Some(&owner), Some(outer), inner) }.map(|_| ())
            })?;
            let outer = outer.cast::<IAutomationPeer>()?;
            peer.outer.set(outer.as_ptr());
            Ok(outer)
        }

        /// The outer object as `IAutomationPeer`, uncounted.
        fn this_peer(&self) -> &IAutomationPeer {
            // SAFETY: the outer object holds this peer, so it is alive while
            // a method of the peer runs.
            unsafe { &*self.outer.get() }
        }

        #[allow(dead_code)] // As in the reference, which has no caller of it either.
        pub(crate) fn panel(&self) -> Option<Rc<FerroSwapChainPanel>> {
            self.panel.upgrade()
        }

        fn get_class_name_core(&self) -> Result<isize, HResult> {
            new_string("FerroSwapChainPanel")
        }

        fn get_automation_control_type_core(&self) -> Result<AutomationControlType, HResult> {
            Ok(AutomationControlType::Pane)
        }

        fn get_children_core(&self) -> Result<Option<ComPtr<IVectorAutomationPeer>>, HResult> {
            super::super::trace("panel peer: GetChildrenCore of the base");
            let list = self
                .base
                .base::<IAutomationPeerOverrides>()?
                .get_children_core()?
                .unwrap_or_else(|| AutomationPeerVector::new(Vec::new()));

            if let Some(root_peer) = self.try_get_embedded_root_peer() {
                super::super::trace("panel peer: the proxy of the embedded root");
                let proxy = FerroToXamlPeerProxy::get_or_create(&root_peer, &self.this.upgrade().ok_or(HResult::POINTER)?)?;
                super::super::trace("panel peer: SetParent of the proxy");
                proxy.set_parent(Some(self.this_peer()))?;
                super::super::trace("panel peer: Append of the proxy");
                list.append(Some(&proxy))?;
            }

            super::super::trace("panel peer: GetChildrenCore returns");
            Ok(Some(list))
        }

        fn get_focused_element_core(&self) -> Result<Option<ComPtr<IInspectable>>, HResult> {
            self.ensure_subscribed();
            let focused = self.embedded_root.borrow().as_ref().and_then(|embedded_root| embedded_root.get_focus());
            match focused {
                None => self.base.base::<IAutomationPeerOverrides>()?.get_focused_element_core(),
                Some(focused) => {
                    let host = self.this.upgrade().ok_or(HResult::POINTER)?;
                    Ok(Some(FerroToXamlPeerProxy::get_or_create(&focused, &host)?.cast::<IInspectable>()?))
                }
            }
        }

        fn get_element_from_point_core(&self, point: Point) -> Result<Option<ComPtr<IInspectable>>, HResult> {
            self.ensure_subscribed();
            let hit = self.embedded_root.borrow().as_ref().and_then(|embedded_root| {
                embedded_root.get_peer_from_point(FerroPoint::new(f64::from(point.x), f64::from(point.y)))
            });
            match hit {
                None => self.base.base::<IAutomationPeerOverrides>()?.get_element_from_point_core(point),
                Some(hit) => {
                    let host = self.this.upgrade().ok_or(HResult::POINTER)?;
                    Ok(Some(FerroToXamlPeerProxy::get_or_create(&hit, &host)?.cast::<IInspectable>()?))
                }
            }
        }

        /// Translate a rectangle of the framework (in embedded-root
        /// coordinates) to screen pixels.
        pub(crate) fn transform_ferro_rect_to_screen(&self, rect: FerroRect) -> Result<(f64, f64, f64, f64), HResult> {
            let panel = self.panel.upgrade().ok_or(HResult::POINTER)?;
            let element = panel.as_ui_element();
            let transform = element.transform_to_visual(None)?.ok_or(HResult::POINTER)?;
            let top_left = transform.transform_point(Point { x: rect.x as f32, y: rect.y as f32 })?;
            let bottom_right = transform.transform_point(Point { x: rect.right() as f32, y: rect.bottom() as f32 })?;

            let xaml_root = element.xaml_root().ok().flatten();
            let scale = xaml_root.as_ref().and_then(|xaml_root| xaml_root.rasterization_scale().ok()).unwrap_or(1.0);

            let mut x = f64::from(top_left.x) * scale;
            let mut y = f64::from(top_left.y) * scale;
            let w = f64::from(bottom_right.x - top_left.x) * scale;
            let h = f64::from(bottom_right.y - top_left.y) * scale;

            let island = xaml_root
                .and_then(|xaml_root| xaml_root.cast::<IXamlRoot2>().ok())
                .and_then(|xaml_root| xaml_root.content_island_environment().ok().flatten());
            if let Some(island) = island {
                let hwnd = island.app_window_id().map(get_window_from_window_id).unwrap_or(0);
                if hwnd != 0 {
                    if let Some((origin_x, origin_y)) = client_origin_on_screen(hwnd) {
                        x += f64::from(origin_x);
                        y += f64::from(origin_y);
                    }
                }
            }

            Ok((x, y, w, h))
        }

        fn try_get_embedded_root_peer(&self) -> Option<Ref<FerroPeer>> {
            self.ensure_subscribed();
            self.embedded_root_peer.borrow().clone()
        }

        fn ensure_subscribed(&self) {
            if self.embedded_root_peer.borrow().is_some() {
                return;
            }

            let Some(root) = self.panel.upgrade().and_then(|panel| panel.get_embedded_root_for_automation()) else {
                return;
            };

            let embedded_root_peer = FerroControlPeer::create_peer_for_element(&root);
            let embedded_root = embedded_root_peer.get_provider::<dyn IEmbeddedRootProvider>();
            *self.embedded_root_peer.borrow_mut() = Some(embedded_root_peer);

            if let Some(embedded_root) = &embedded_root {
                let this = self.this.clone();
                *self.focus_changed.borrow_mut() = Some(Subscription::new(embedded_root.focus_changed(Rc::new(move || {
                    if let Some(this) = this.upgrade() {
                        this.on_embedded_focus_changed();
                    }
                }))));
            }
            *self.embedded_root.borrow_mut() = embedded_root;
        }

        fn on_embedded_focus_changed(&self) {
            let listener_exists = factory::<IAutomationPeerStatics>("Microsoft.UI.Xaml.Automation.Peers.AutomationPeer")
                .map(|statics| flag(statics.listener_exists(AutomationEvents::AutomationFocusChanged)))
                .unwrap_or(false);
            if !listener_exists {
                return;
            }

            let focused = self.embedded_root.borrow().as_ref().and_then(|embedded_root| embedded_root.get_focus());
            let (Some(focused), Some(host)) = (focused, self.this.upgrade()) else {
                return;
            };

            if let Ok(proxy) = FerroToXamlPeerProxy::get_or_create(&focused, &host) {
                let _ = proxy.raise_automation_event(AutomationEvents::AutomationFocusChanged);
            }
        }
    }


    macro_rules! base {
        ($self:ident . $method:ident ( $($arg:expr),* )) => {
            $self.0.base.base::<IAutomationPeerOverrides>()?.$method($($arg),*)
        };
    }

    impl IAutomationPeerOverridesImpl for PeerOuter {
        fn get_pattern_core(&self, pattern_interface: PatternInterface) -> Result<Option<ComPtr<IInspectable>>, HResult> {
            base!(self.get_pattern_core(pattern_interface))
        }
        fn get_accelerator_key_core(&self) -> Result<isize, HResult> {
            base!(self.get_accelerator_key_core())
        }
        fn get_access_key_core(&self) -> Result<isize, HResult> {
            base!(self.get_access_key_core())
        }
        fn get_automation_control_type_core(&self) -> Result<AutomationControlType, HResult> {
            self.0.get_automation_control_type_core()
        }
        fn get_automation_id_core(&self) -> Result<isize, HResult> {
            base!(self.get_automation_id_core())
        }
        fn get_bounding_rectangle_core(&self) -> Result<Rect, HResult> {
            base!(self.get_bounding_rectangle_core())
        }
        fn get_children_core(&self) -> Result<Option<ComPtr<IVectorAutomationPeer>>, HResult> {
            self.0.get_children_core()
        }
        fn navigate_core(&self, direction: i32) -> Result<Option<ComPtr<IInspectable>>, HResult> {
            base!(self.navigate_core(direction))
        }
        fn get_class_name_core(&self) -> Result<isize, HResult> {
            self.0.get_class_name_core()
        }
        fn get_clickable_point_core(&self) -> Result<Point, HResult> {
            base!(self.get_clickable_point_core())
        }
        fn get_help_text_core(&self) -> Result<isize, HResult> {
            base!(self.get_help_text_core())
        }
        fn get_item_status_core(&self) -> Result<isize, HResult> {
            base!(self.get_item_status_core())
        }
        fn get_item_type_core(&self) -> Result<isize, HResult> {
            base!(self.get_item_type_core())
        }
        fn get_labeled_by_core(&self) -> Result<Option<ComPtr<IAutomationPeer>>, HResult> {
            base!(self.get_labeled_by_core())
        }
        fn get_localized_control_type_core(&self) -> Result<isize, HResult> {
            base!(self.get_localized_control_type_core())
        }
        fn get_name_core(&self) -> Result<isize, HResult> {
            base!(self.get_name_core())
        }
        fn get_orientation_core(&self) -> Result<i32, HResult> {
            base!(self.get_orientation_core())
        }
        fn has_keyboard_focus_core(&self) -> Result<u8, HResult> {
            base!(self.has_keyboard_focus_core())
        }
        fn is_content_element_core(&self) -> Result<u8, HResult> {
            base!(self.is_content_element_core())
        }
        fn is_control_element_core(&self) -> Result<u8, HResult> {
            base!(self.is_control_element_core())
        }
        fn is_enabled_core(&self) -> Result<u8, HResult> {
            base!(self.is_enabled_core())
        }
        fn is_keyboard_focusable_core(&self) -> Result<u8, HResult> {
            base!(self.is_keyboard_focusable_core())
        }
        fn is_offscreen_core(&self) -> Result<u8, HResult> {
            base!(self.is_offscreen_core())
        }
        fn is_password_core(&self) -> Result<u8, HResult> {
            base!(self.is_password_core())
        }
        fn is_required_for_form_core(&self) -> Result<u8, HResult> {
            base!(self.is_required_for_form_core())
        }
        fn set_focus_core(&self) -> Result<(), HResult> {
            base!(self.set_focus_core())
        }
        fn get_peer_from_point_core(&self, point: Point) -> Result<Option<ComPtr<IAutomationPeer>>, HResult> {
            base!(self.get_peer_from_point_core(point))
        }
        fn get_element_from_point_core(&self, point_in_window_coordinates: Point) -> Result<Option<ComPtr<IInspectable>>, HResult> {
            self.0.get_element_from_point_core(point_in_window_coordinates)
        }
        fn get_focused_element_core(&self) -> Result<Option<ComPtr<IInspectable>>, HResult> {
            self.0.get_focused_element_core()
        }
        fn get_live_setting_core(&self) -> Result<i32, HResult> {
            base!(self.get_live_setting_core())
        }
        fn show_context_menu_core(&self) -> Result<(), HResult> {
            base!(self.show_context_menu_core())
        }
        fn get_controlled_peers_core(&self) -> Result<Option<ComPtr<IVectorViewAutomationPeer>>, HResult> {
            base!(self.get_controlled_peers_core())
        }
        fn get_annotations_core(&self) -> Result<*mut c_void, HResult> {
            base!(self.get_annotations_core())
        }
        fn get_position_in_set_core(&self) -> Result<i32, HResult> {
            base!(self.get_position_in_set_core())
        }
        fn get_size_of_set_core(&self) -> Result<i32, HResult> {
            base!(self.get_size_of_set_core())
        }
        fn get_level_core(&self) -> Result<i32, HResult> {
            base!(self.get_level_core())
        }
        fn get_landmark_type_core(&self) -> Result<i32, HResult> {
            base!(self.get_landmark_type_core())
        }
        fn get_localized_landmark_type_core(&self) -> Result<isize, HResult> {
            base!(self.get_localized_landmark_type_core())
        }
        fn is_peripheral_core(&self) -> Result<u8, HResult> {
            base!(self.is_peripheral_core())
        }
        fn is_data_valid_for_form_core(&self) -> Result<u8, HResult> {
            base!(self.is_data_valid_for_form_core())
        }
        fn get_full_description_core(&self) -> Result<isize, HResult> {
            base!(self.get_full_description_core())
        }
        fn get_described_by_core(&self) -> Result<Option<ComPtr<IIterableAutomationPeer>>, HResult> {
            base!(self.get_described_by_core())
        }
        fn get_flows_to_core(&self) -> Result<Option<ComPtr<IIterableAutomationPeer>>, HResult> {
            base!(self.get_flows_to_core())
        }
        fn get_flows_from_core(&self) -> Result<Option<ComPtr<IIterableAutomationPeer>>, HResult> {
            base!(self.get_flows_from_core())
        }
        fn get_culture_core(&self) -> Result<i32, HResult> {
            base!(self.get_culture_core())
        }
        fn get_heading_level_core(&self) -> Result<i32, HResult> {
            base!(self.get_heading_level_core())
        }
        fn is_dialog_core(&self) -> Result<u8, HResult> {
            base!(self.is_dialog_core())
        }
    }
}

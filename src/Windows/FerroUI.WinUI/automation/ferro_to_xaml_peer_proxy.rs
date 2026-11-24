//! Wraps an automation peer of the framework so it can be exposed to
//! Microsoft UI Automation through WinUI XAML's automation infrastructure.
//! Mirrors the role of the automation node of the Windows backend
//! (`ferroui_win32::automation`) but for WinUI hosts.
//!
//! The patterns are in the files beside this one, one per pattern, as the
//! partial class of the reference is.

#[cfg(windows)]
pub(crate) use imp::FerroToXamlPeerProxy;

#[cfg(windows)]
mod imp {
    use super::super::control_type_map::ControlTypeMap;
    use super::super::ferro_swap_chain_panel_automation_peer::FerroSwapChainPanelAutomationPeer;
    use crate::interop::collections::AutomationPeerVector;
    use crate::interop::composable::{compose, Composed, ComposedBase};
    use crate::interop::win_rt::{factory, new_string, to_abi};
    use crate::interop::*;
    use ferroui_base::Ref;
    use ferroui_controls::automation::peers::{AutomationPeer as FerroPeer, EmbeddableControlRootAutomationPeer};
    use ferroui_controls::automation::provider as ferro;
    use ferroui_microcom::{ComPtr, HResult, InterfaceEntry};
    use std::cell::{Cell, RefCell};
    use std::collections::HashMap;
    use std::ffi::c_void;
    use std::rc::{Rc, Weak};

    ferroui_base::ferro_thread_local! {
        /// The proxy of each peer of the framework, while XAML holds it: the
        /// reference keys a weak table by the peer, whose entry lives as long
        /// as the peer does; a counted proxy that holds its peer cannot be
        /// held by a table that lets the peer go, so here the table does not
        /// count the proxy and a proxy leaves the table when it is destroyed
        /// (docs/porting/winui.md, section 5).
        static S_CACHE: RefCell<HashMap<usize, usize>> = RefCell::new(HashMap::new());
        static S_NEXT_ID: Cell<i64> = const { Cell::new(0) };
    }

    fn peer_key(peer: &Ref<FerroPeer>) -> usize {
        std::ptr::from_ref::<FerroPeer>(peer) as usize
    }

    pub(crate) struct FerroToXamlPeerProxy {
        base: ComposedBase,
        /// The outer object, uncounted: it holds this value.
        this: Cell<*mut IAutomationPeer>,
        pub(crate) host: Weak<FerroSwapChainPanelAutomationPeer>,
        pub(crate) peer: Ref<FerroPeer>,
        is_embedded_root: bool,
        #[allow(dead_code)] // As in the reference, which keeps it and does not read it.
        unique_id: String,
    }

    impl Composed for FerroToXamlPeerProxy {
        fn composed_base(&self) -> &ComposedBase {
            &self.base
        }
    }

    inspectable!(
        FerroToXamlPeerProxy,
        "FerroUI.WinUI.Automation.FerroToXamlPeerProxy",
        [
            IAutomationPeerOverrides,
            IInvokeProvider,
            IToggleProvider,
            IValueProvider,
            IRangeValueProvider,
            IScrollProvider,
            IScrollItemProvider,
            ISelectionProvider,
            ISelectionItemProvider,
            IExpandCollapseProvider,
        ]
    );

    impl FerroToXamlPeerProxy {
        fn new(peer: &Ref<FerroPeer>, host: &Rc<FerroSwapChainPanelAutomationPeer>) -> Result<ComPtr<IAutomationPeer>, HResult> {
            let id = S_NEXT_ID.with(|next| {
                next.set(next.get() + 1);
                next.get()
            });
            let proxy = FerroToXamlPeerProxy {
                base: ComposedBase::default(),
                this: Cell::new(std::ptr::null_mut()),
                host: Rc::downgrade(host),
                peer: peer.clone(),
                is_embedded_root: peer.is::<EmbeddableControlRootAutomationPeer>(),
                unique_id: format!("FerroUI.WinUI.Peer:{id}"),
            };
            let peer_factory = factory::<IAutomationPeerFactory>("Microsoft.UI.Xaml.Automation.Peers.AutomationPeer")?;
            let composed = compose::<IAutomationPeerOverrides, FerroToXamlPeerProxy, 9>(
                proxy,
                [
                    InterfaceEntry::of::<IInvokeProvider>(),
                    InterfaceEntry::of::<IToggleProvider>(),
                    InterfaceEntry::of::<IValueProvider>(),
                    InterfaceEntry::of::<IRangeValueProvider>(),
                    InterfaceEntry::of::<IScrollProvider>(),
                    InterfaceEntry::of::<IScrollItemProvider>(),
                    InterfaceEntry::of::<ISelectionProvider>(),
                    InterfaceEntry::of::<ISelectionItemProvider>(),
                    InterfaceEntry::of::<IExpandCollapseProvider>(),
                ],
                |outer, inner| {
                    // SAFETY: the outer object and a place of the caller's
                    // frame.
                    unsafe { peer_factory.create_instance(Some(outer), inner) }.map(|_| ())
                },
            )?;
            // SAFETY: `composed` is the first interface of the outer object
            // `compose` made with a `FerroToXamlPeerProxy` (the interface the
            // runtime of the port made, not one of the inner object), alive
            // through `composed`. The value is read before the query below:
            // `IAutomationPeer` is an interface of the inner object, an object
            // of XAML, whose memory is not a `ComObject`.
            let value = unsafe { ferroui_microcom::ComObject::<FerroToXamlPeerProxy>::value(composed.as_ptr().cast()) };
            let outer = composed.cast::<IAutomationPeer>()?;
            value.this.set(outer.as_ptr());
            super::super::trace(&format!("proxy created for a peer of class {:?}", value.peer.get_class_name()));
            S_CACHE.with(|cache| cache.borrow_mut().insert(peer_key(peer), outer.as_ptr() as usize));
            Ok(outer)
        }

        #[allow(dead_code)] // As in the reference: the patterns read the field.
        pub(crate) fn peer(&self) -> &Ref<FerroPeer> {
            &self.peer
        }

        pub(crate) fn get_or_create(
            peer: &Ref<FerroPeer>,
            host: &Rc<FerroSwapChainPanelAutomationPeer>,
        ) -> Result<ComPtr<IAutomationPeer>, HResult> {
            match Self::try_get(Some(peer)) {
                Some(proxy) => Ok(proxy),
                None => Self::new(peer, host),
            }
        }

        pub(crate) fn try_get(peer: Option<&Ref<FerroPeer>>) -> Option<ComPtr<IAutomationPeer>> {
            let peer = peer?;
            let pointer = S_CACHE.with(|cache| cache.borrow().get(&peer_key(peer)).copied())?;
            // SAFETY: a proxy of the table is alive: it leaves the table when
            // it is destroyed.
            unsafe { ComPtr::from_raw_add_ref(pointer as *mut IAutomationPeer) }
        }

        /// The outer object as `IAutomationPeer`, uncounted.
        pub(crate) fn this_peer(&self) -> &IAutomationPeer {
            // SAFETY: the outer object holds this value, so it is alive while
            // a method of it runs.
            unsafe { &*self.this.get() }
        }

        /// The outer object as an `IInspectable` the caller owns.
        fn this_object(&self) -> Result<Option<ComPtr<IInspectable>>, HResult> {
            Ok(Some(ComPtr::from_ref(self.this_peer()).cast::<IInspectable>()?))
        }

        /// `ProviderFromPeer` (protected on `AutomationPeer`): the provider
        /// XAML makes of a peer.
        pub(crate) fn provider_from_peer(&self, peer: &IAutomationPeer) -> Option<ComPtr<IIRawElementProviderSimple>> {
            self.base.base::<IAutomationPeerProtected>().ok()?.provider_from_peer(Some(peer)).ok().flatten()
        }

        fn get_children_core(&self) -> Result<Option<ComPtr<IVectorAutomationPeer>>, HResult> {
            let list = AutomationPeerVector::new(Vec::new());
            let children = self.peer.get_children();
            let Some(host) = self.host.upgrade() else {
                return Ok(Some(list));
            };

            super::super::trace(&format!("proxy of {:?}: GetChildrenCore, {} children", self.peer.get_class_name(), children.len()));
            for child in children.iter() {
                let child_proxy = Self::get_or_create(child, &host)?;
                child_proxy.set_parent(Some(self.this_peer()))?;
                list.append(Some(&child_proxy))?;
            }

            super::super::trace(&format!("proxy of {:?}: GetChildrenCore returns", self.peer.get_class_name()));
            Ok(Some(list))
        }

        fn get_class_name_core(&self) -> Result<isize, HResult> {
            super::super::trace(&format!("proxy: GetClassNameCore {:?}", self.peer.get_class_name()));
            new_string(&self.peer.get_class_name())
        }

        fn get_name_core(&self) -> Result<isize, HResult> {
            new_string(&self.peer.get_name())
        }

        fn get_automation_id_core(&self) -> Result<isize, HResult> {
            new_string(&self.peer.get_automation_id().unwrap_or_default())
        }

        fn get_help_text_core(&self) -> Result<isize, HResult> {
            new_string(&self.peer.get_help_text())
        }

        fn get_accelerator_key_core(&self) -> Result<isize, HResult> {
            new_string(&self.peer.get_accelerator_key().unwrap_or_default())
        }

        fn get_access_key_core(&self) -> Result<isize, HResult> {
            new_string(&self.peer.get_access_key().unwrap_or_default())
        }

        fn get_item_status_core(&self) -> Result<isize, HResult> {
            new_string(&self.peer.get_item_status().unwrap_or_default())
        }

        fn get_item_type_core(&self) -> Result<isize, HResult> {
            new_string(&self.peer.get_item_type().unwrap_or_default())
        }

        fn get_localized_control_type_core(&self) -> Result<isize, HResult> {
            let localized_control_type = self.peer.get_localized_control_type();
            if localized_control_type.is_empty() {
                return self.base.base::<IAutomationPeerOverrides>()?.get_localized_control_type_core();
            }
            new_string(&localized_control_type)
        }

        fn get_automation_control_type_core(&self) -> Result<AutomationControlType, HResult> {
            Ok(ControlTypeMap::to_xaml(self.peer.get_automation_control_type()))
        }

        fn get_bounding_rectangle_core(&self) -> Result<Rect, HResult> {
            let rect = self.peer.get_bounding_rectangle();
            if rect.width <= 0.0 && rect.height <= 0.0 {
                return Ok(Rect { x: 0.0, y: 0.0, width: 0.0, height: 0.0 });
            }

            let host = self.host.upgrade().ok_or(HResult::POINTER)?;
            let (screen_x, screen_y, w, h) = host.transform_ferro_rect_to_screen(rect)?;
            Ok(Rect { x: screen_x as f32, y: screen_y as f32, width: w as f32, height: h as f32 })
        }

        fn has_keyboard_focus_core(&self) -> Result<u8, HResult> {
            Ok(to_abi(self.peer.has_keyboard_focus()))
        }

        fn is_keyboard_focusable_core(&self) -> Result<u8, HResult> {
            Ok(to_abi(self.peer.is_keyboard_focusable()))
        }

        fn is_enabled_core(&self) -> Result<u8, HResult> {
            Ok(to_abi(self.peer.is_enabled()))
        }

        // Collapse the redundant Pane → Pane: the embedded root sits beneath
        // the panel's own peer and adds nothing to the screen reader's view.
        fn is_content_element_core(&self) -> Result<u8, HResult> {
            Ok(to_abi(!self.is_embedded_root && self.peer.is_content_element()))
        }

        fn is_control_element_core(&self) -> Result<u8, HResult> {
            Ok(to_abi(!self.is_embedded_root && self.peer.is_control_element()))
        }

        fn is_offscreen_core(&self) -> Result<u8, HResult> {
            Ok(to_abi(self.peer.is_offscreen()))
        }

        fn set_focus_core(&self) -> Result<(), HResult> {
            self.peer.set_focus();
            Ok(())
        }

        fn get_pattern_core(&self, pattern_interface: PatternInterface) -> Result<Option<ComPtr<IInspectable>>, HResult> {
            let provides = match pattern_interface {
                PatternInterface::Invoke => self.peer.get_provider::<dyn ferro::IInvokeProvider>().is_some(),
                PatternInterface::Toggle => self.peer.get_provider::<dyn ferro::IToggleProvider>().is_some(),
                PatternInterface::Value => self.peer.get_provider::<dyn ferro::IValueProvider>().is_some(),
                PatternInterface::RangeValue => self.peer.get_provider::<dyn ferro::IRangeValueProvider>().is_some(),
                PatternInterface::Scroll => self.peer.get_provider::<dyn ferro::IScrollProvider>().is_some(),
                // Every peer supports BringIntoView, so ScrollItem is always available.
                PatternInterface::ScrollItem => true,
                PatternInterface::Selection => self.peer.get_provider::<dyn ferro::ISelectionProvider>().is_some(),
                PatternInterface::SelectionItem => self.peer.get_provider::<dyn ferro::ISelectionItemProvider>().is_some(),
                PatternInterface::ExpandCollapse => self.peer.get_provider::<dyn ferro::IExpandCollapseProvider>().is_some(),
                _ => false,
            };
            if provides {
                self.this_object()
            } else {
                Ok(None)
            }
        }
    }

    impl Drop for FerroToXamlPeerProxy {
        fn drop(&mut self) {
            super::super::trace("proxy destroyed");
            let key = peer_key(&self.peer);
            let this = self.this.get() as usize;
            S_CACHE.with(|cache| {
                let mut cache = cache.borrow_mut();
                if cache.get(&key) == Some(&this) {
                    cache.remove(&key);
                }
            });
        }
    }

    macro_rules! base {
        ($self:ident . $method:ident ( $($arg:expr),* )) => {
            $self.base.base::<IAutomationPeerOverrides>()?.$method($($arg),*)
        };
    }

    impl IAutomationPeerOverridesImpl for FerroToXamlPeerProxy {
        fn get_pattern_core(&self, pattern_interface: PatternInterface) -> Result<Option<ComPtr<IInspectable>>, HResult> {
            FerroToXamlPeerProxy::get_pattern_core(self, pattern_interface)
        }
        fn get_accelerator_key_core(&self) -> Result<isize, HResult> {
            FerroToXamlPeerProxy::get_accelerator_key_core(self)
        }
        fn get_access_key_core(&self) -> Result<isize, HResult> {
            FerroToXamlPeerProxy::get_access_key_core(self)
        }
        fn get_automation_control_type_core(&self) -> Result<AutomationControlType, HResult> {
            FerroToXamlPeerProxy::get_automation_control_type_core(self)
        }
        fn get_automation_id_core(&self) -> Result<isize, HResult> {
            FerroToXamlPeerProxy::get_automation_id_core(self)
        }
        fn get_bounding_rectangle_core(&self) -> Result<Rect, HResult> {
            FerroToXamlPeerProxy::get_bounding_rectangle_core(self)
        }
        fn get_children_core(&self) -> Result<Option<ComPtr<IVectorAutomationPeer>>, HResult> {
            FerroToXamlPeerProxy::get_children_core(self)
        }
        fn navigate_core(&self, direction: i32) -> Result<Option<ComPtr<IInspectable>>, HResult> {
            base!(self.navigate_core(direction))
        }
        fn get_class_name_core(&self) -> Result<isize, HResult> {
            FerroToXamlPeerProxy::get_class_name_core(self)
        }
        fn get_clickable_point_core(&self) -> Result<Point, HResult> {
            base!(self.get_clickable_point_core())
        }
        fn get_help_text_core(&self) -> Result<isize, HResult> {
            FerroToXamlPeerProxy::get_help_text_core(self)
        }
        fn get_item_status_core(&self) -> Result<isize, HResult> {
            FerroToXamlPeerProxy::get_item_status_core(self)
        }
        fn get_item_type_core(&self) -> Result<isize, HResult> {
            FerroToXamlPeerProxy::get_item_type_core(self)
        }
        fn get_labeled_by_core(&self) -> Result<Option<ComPtr<IAutomationPeer>>, HResult> {
            base!(self.get_labeled_by_core())
        }
        fn get_localized_control_type_core(&self) -> Result<isize, HResult> {
            FerroToXamlPeerProxy::get_localized_control_type_core(self)
        }
        fn get_name_core(&self) -> Result<isize, HResult> {
            FerroToXamlPeerProxy::get_name_core(self)
        }
        fn get_orientation_core(&self) -> Result<i32, HResult> {
            base!(self.get_orientation_core())
        }
        fn has_keyboard_focus_core(&self) -> Result<u8, HResult> {
            FerroToXamlPeerProxy::has_keyboard_focus_core(self)
        }
        fn is_content_element_core(&self) -> Result<u8, HResult> {
            FerroToXamlPeerProxy::is_content_element_core(self)
        }
        fn is_control_element_core(&self) -> Result<u8, HResult> {
            FerroToXamlPeerProxy::is_control_element_core(self)
        }
        fn is_enabled_core(&self) -> Result<u8, HResult> {
            FerroToXamlPeerProxy::is_enabled_core(self)
        }
        fn is_keyboard_focusable_core(&self) -> Result<u8, HResult> {
            FerroToXamlPeerProxy::is_keyboard_focusable_core(self)
        }
        fn is_offscreen_core(&self) -> Result<u8, HResult> {
            FerroToXamlPeerProxy::is_offscreen_core(self)
        }
        fn is_password_core(&self) -> Result<u8, HResult> {
            base!(self.is_password_core())
        }
        fn is_required_for_form_core(&self) -> Result<u8, HResult> {
            base!(self.is_required_for_form_core())
        }
        fn set_focus_core(&self) -> Result<(), HResult> {
            FerroToXamlPeerProxy::set_focus_core(self)
        }
        fn get_peer_from_point_core(&self, point: Point) -> Result<Option<ComPtr<IAutomationPeer>>, HResult> {
            base!(self.get_peer_from_point_core(point))
        }
        fn get_element_from_point_core(&self, point_in_window_coordinates: Point) -> Result<Option<ComPtr<IInspectable>>, HResult> {
            base!(self.get_element_from_point_core(point_in_window_coordinates))
        }
        fn get_focused_element_core(&self) -> Result<Option<ComPtr<IInspectable>>, HResult> {
            base!(self.get_focused_element_core())
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
            Ok(ControlTypeMap::landmark_to_xaml(self.peer.get_landmark_type()))
        }
        fn get_localized_landmark_type_core(&self) -> Result<isize, HResult> {
            let landmark = self.peer.get_landmark_type();
            match self.peer.get_localized_landmark_type() {
                Some(name) => new_string(&name),
                None => match ControlTypeMap::landmark_name(landmark) {
                    Some(name) => new_string(name),
                    None => base!(self.get_localized_landmark_type_core()),
                },
            }
        }
        fn is_peripheral_core(&self) -> Result<u8, HResult> {
            base!(self.is_peripheral_core())
        }
        fn is_data_valid_for_form_core(&self) -> Result<u8, HResult> {
            base!(self.is_data_valid_for_form_core())
        }
        fn get_full_description_core(&self) -> Result<isize, HResult> {
            match self.peer.get_full_description() {
                Some(description) => new_string(&description),
                None => base!(self.get_full_description_core()),
            }
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

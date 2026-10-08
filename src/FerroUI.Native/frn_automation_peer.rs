//! The automation peers of the toolkit exposed to native code: the wrapper
//! the accessibility elements of the native side read and drive.

use crate::helpers::*;
use crate::interop::*;
use crate::popup_impl::PopupImpl;
use crate::window_impl::WindowImpl;
use crate::window_impl_base::WindowBaseParent;
use ferroui_base::reactive::IDisposable;
use ferroui_base::Ref;
use ferroui_controls::automation::peers::{
    AutomationPeer, ControlAutomationPeer, InteropAutomationPeer, ScrollViewerAutomationPeer,
};
use ferroui_controls::automation::provider::{
    IEmbeddedRootProvider, IExpandCollapseProvider, IInvokeProvider, IRangeValueProvider, IRootProvider,
    ISelectionItemProvider, IToggleProvider, IValueProvider,
};
use ferroui_controls::automation::{
    AutomationElementIdentifiers, AutomationProperty, AutomationPropertyChangedEventArgs,
    ElementNotEnabledException, ExpandCollapsePatternIdentifiers, ExpandCollapseState,
    RangeValuePatternIdentifiers, SelectionItemPatternIdentifiers, SelectionPatternIdentifiers,
    TogglePatternIdentifiers, ValuePatternIdentifiers,
};
use ferroui_controls::Control;
use ferroui_microcom::{ComPtr, HResult};
use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::{c_void, CStr};
use std::rc::{Rc, Weak};

/// The automation properties whose changes native code is told about, with
/// the identifier native code knows them by.
fn property_map() -> [(&'static AutomationProperty, FrnAutomationProperty); 10] {
    [
        (AutomationElementIdentifiers::automation_id_property(), FrnAutomationProperty::AutomationPeer_AutomationId),
        (
            AutomationElementIdentifiers::bounding_rectangle_property(),
            FrnAutomationProperty::AutomationPeer_BoundingRectangle,
        ),
        (AutomationElementIdentifiers::class_name_property(), FrnAutomationProperty::AutomationPeer_ClassName),
        (AutomationElementIdentifiers::name_property(), FrnAutomationProperty::AutomationPeer_Name),
        (RangeValuePatternIdentifiers::value_property(), FrnAutomationProperty::RangeValueProvider_Value),
        (ValuePatternIdentifiers::value_property(), FrnAutomationProperty::ValueProvider_Value),
        (TogglePatternIdentifiers::toggle_state_property(), FrnAutomationProperty::ToggleProvider_ToggleState),
        (
            ExpandCollapsePatternIdentifiers::expand_collapse_state_property(),
            FrnAutomationProperty::ExpandCollapseProvider_ExpandCollapseState,
        ),
        (
            SelectionItemPatternIdentifiers::is_selected_property(),
            FrnAutomationProperty::SelectionItemProvider_IsSelected,
        ),
        (SelectionPatternIdentifiers::selection_property(), FrnAutomationProperty::SelectionProvider_Selection),
    ]
}

thread_local! {
    /// The wrapper of each wrapped peer, by the address of the peer.
    ///
    /// The reference keeps the wrapper for as long as the peer lives (a
    /// conditional weak table); here the table holds the wrapper weakly and
    /// the wrapper lives as long as native code or the toolkit holds it. A
    /// native accessibility element holds the wrapper of its peer for its
    /// whole life, so a peer that has a node always maps to the same
    /// wrapper, and with it to the same node.
    ///
    /// Once native code has set a node, the wrapper is part of a reference
    /// cycle, as in the reference: the wrapper holds its node, the node holds
    /// its element strongly (`FrnAutomationNode` in `FrnAutomationNode.h`)
    /// and the element holds the wrapper (`FrnAccessibilityElement` in
    /// `automation.mm`). Such a wrapper is therefore never dropped and its
    /// node never disposed; the native window holds the wrapper of its root
    /// peer the same way, which keeps the peers of the whole tree alive after
    /// the window is closed. The reference has the same cycle through the
    /// object the managed runtime hands to native code.
    static WRAPPERS: RefCell<HashMap<usize, Weak<FrnAutomationPeer>>> = RefCell::new(HashMap::new());
}

fn peer_key(peer: &AutomationPeer) -> usize {
    peer as *const AutomationPeer as usize
}

/// Wraps an automation peer for native code.
pub(crate) struct FrnAutomationPeer {
    inner: Ref<AutomationPeer>,
    node: RefCell<Option<ComPtr<IFrnAutomationNode>>>,
    /// The handlers on the events of the peer, removed with the wrapper.
    subscriptions: Vec<Rc<dyn IDisposable>>,
}

impl FrnAutomationPeer {
    fn new(inner: Ref<AutomationPeer>) -> Rc<Self> {
        Rc::new_cyclic(|this: &Weak<Self>| {
            let mut subscriptions = Vec::new();

            let weak = this.clone();
            subscriptions.push(inner.children_changed(move || {
                if let Some(node) = weak.upgrade().and_then(|this| this.node()) {
                    node.children_changed();
                }
            }));

            let weak = this.clone();
            subscriptions.push(inner.property_changed(move |e| {
                if let Some(this) = weak.upgrade() {
                    this.on_peer_property_changed(e);
                }
            }));

            // The reference tests the class of the peer for the contract;
            // the provider lookup of a peer answers the same question.
            if let Some(root) = inner.get_provider::<dyn IRootProvider>() {
                let weak = this.clone();
                subscriptions.push(root.focus_changed(Rc::new(move || {
                    if let Some(node) = weak.upgrade().and_then(|this| this.node()) {
                        node.focus_changed();
                    }
                })));
            }

            Self { inner, node: RefCell::new(None), subscriptions }
        })
    }

    /// The node native code set on the wrapper, if any.
    pub(crate) fn node(&self) -> Option<ComPtr<IFrnAutomationNode>> {
        self.node.borrow().clone()
    }

    /// The wrapper of `peer`: the same wrapper for the same peer for as long
    /// as the wrapper is alive.
    pub(crate) fn wrap(peer: Option<Ref<AutomationPeer>>) -> Option<Rc<FrnAutomationPeer>> {
        let peer = peer?;
        let key = peer_key(&peer);
        if let Some(wrapper) = WRAPPERS.with(|wrappers| wrappers.borrow().get(&key).and_then(Weak::upgrade)) {
            return Some(wrapper);
        }
        // Created outside the borrow: the constructor subscribes to the
        // events of the peer.
        let wrapper = FrnAutomationPeer::new(peer);
        WRAPPERS.with(|wrappers| wrappers.borrow_mut().insert(key, Rc::downgrade(&wrapper)));
        Some(wrapper)
    }

    /// [`wrap`](Self::wrap) as the interface pointer native code receives.
    pub(crate) fn wrap_native(peer: Option<Ref<AutomationPeer>>) -> Option<ComPtr<IFrnAutomationPeer>> {
        Self::wrap(peer).map(IFrnAutomationPeer::from_impl)
    }

    fn provider<T: ?Sized + 'static>(&self) -> Rc<T> {
        self.inner.get_provider::<T>().unwrap_or_else(|| {
            panic!(
                "The peer {} does not implement {}.",
                self.inner.get_type().name(),
                std::any::type_name::<T>()
            )
        })
    }

    fn is_provider<T: ?Sized + 'static>(&self) -> bool {
        self.inner.get_provider::<T>().is_some()
    }

    fn on_peer_property_changed(&self, e: &AutomationPropertyChangedEventArgs) {
        let property = property_map().into_iter().find(|(key, _)| *key == e.property()).map(|(_, value)| value);
        if let Some(property) = property {
            if let Some(node) = self.node() {
                node.property_changed(property);
            }
        }
    }
}

impl Drop for FrnAutomationPeer {
    fn drop(&mut self) {
        for subscription in self.subscriptions.drain(..) {
            subscription.dispose();
        }
        let key = peer_key(&self.inner);
        let _ = WRAPPERS.try_with(|wrappers| {
            if let Ok(mut wrappers) = wrappers.try_borrow_mut() {
                if wrappers.get(&key).is_some_and(|wrapper| wrapper.strong_count() == 0) {
                    wrappers.remove(&key);
                }
            }
        });
        // The finalizer of the reference. Only a wrapper whose node was never
        // set, or was set by something other than a native element, gets
        // here: an element keeps the wrapper of its node alive (see
        // `WRAPPERS`).
        if let Some(node) = self.node.get_mut().take() {
            node.dispose();
        }
    }
}

/// Hands the error of a provider member to the dispatcher, as an exception
/// thrown by a callback is in the reference.
fn raise_on_error(result: Result<(), ElementNotEnabledException>) {
    if let Err(e) = result {
        crate::callback_base::raise_exception(Box::new(e));
    }
}

/// The peer native code expects from a hit test: the nearest control
/// element at or above `result`.
fn control_element_for_hit_test(result: Option<Ref<AutomationPeer>>) -> Option<Ref<AutomationPeer>> {
    let mut result = result?;

    // The OSX accessibility APIs expect non-ignored elements when hit-testing.
    while !result.is_control_element() {
        match result.get_parent() {
            Some(parent) => result = parent,
            None => break,
        }
    }

    Some(result)
}

/// The native window of a window or popup implementation of this backend.
fn window_base_native(platform_impl: &dyn ferroui_controls::platform::ITopLevelImpl) -> Option<ComPtr<IFrnWindowBase>> {
    let any = platform_impl.as_any();
    if let Some(window) = any.downcast_ref::<WindowImpl>() {
        return window.window_base().native();
    }
    if let Some(popup) = any.downcast_ref::<PopupImpl>() {
        return popup.window_base().native();
    }
    None
}

impl IFrnAutomationPeerImpl for FrnAutomationPeer {
    fn get_node(&self) -> Option<ComPtr<IFrnAutomationNode>> {
        crate::callback_base::guard(None, || self.node())
    }

    fn set_node(&self, node: Option<&IFrnAutomationNode>) {
        crate::callback_base::guard((), || {
            if self.node.borrow().is_some() {
                panic!("The FrnAutomationPeer already has a node.");
            }
            // A counted reference, as in the reference. The element that owns
            // the node deletes it in its `dealloc` without regard to its
            // reference count (`automation.mm`), so the pointer would dangle
            // if the element were ever deallocated while the wrapper lives;
            // the reference cycle described at `WRAPPERS` keeps that from
            // happening, in the reference as here.
            *self.node.borrow_mut() = node.map(ComPtr::from_ref);
        })
    }

    fn get_accelerator_key(&self) -> Option<ComPtr<IFrnString>> {
        crate::callback_base::guard(None, || to_frn_string(self.inner.get_accelerator_key().as_deref()))
    }

    fn get_access_key(&self) -> Option<ComPtr<IFrnString>> {
        crate::callback_base::guard(None, || to_frn_string(self.inner.get_access_key().as_deref()))
    }

    fn get_automation_control_type(&self) -> FrnAutomationControlType {
        crate::callback_base::guard(FrnAutomationControlType::default(), || {
            FrnAutomationControlType(self.inner.get_automation_control_type() as i32)
        })
    }

    fn get_automation_id(&self) -> Option<ComPtr<IFrnString>> {
        crate::callback_base::guard(None, || to_frn_string(self.inner.get_automation_id().as_deref()))
    }

    fn get_bounding_rectangle(&self) -> FrnRect {
        crate::callback_base::guard(FrnRect { x: 0.0, y: 0.0, width: 0.0, height: 0.0 }, || {
            to_frn_rect(self.inner.get_bounding_rectangle())
        })
    }

    fn get_children(&self) -> Option<ComPtr<IFrnAutomationPeerArray>> {
        crate::callback_base::guard(None, || {
            Some(IFrnAutomationPeerArray::from_impl(FrnAutomationPeerArray::new(&self.inner.get_children())))
        })
    }

    fn get_class_name(&self) -> Option<ComPtr<IFrnString>> {
        crate::callback_base::guard(None, || to_frn_string(Some(&self.inner.get_class_name())))
    }

    fn get_labeled_by(&self) -> Option<ComPtr<IFrnAutomationPeer>> {
        crate::callback_base::guard(None, || Self::wrap_native(self.inner.get_labeled_by()))
    }

    fn get_name(&self) -> Option<ComPtr<IFrnString>> {
        crate::callback_base::guard(None, || to_frn_string(Some(&self.inner.get_name())))
    }

    fn get_parent(&self) -> Option<ComPtr<IFrnAutomationPeer>> {
        crate::callback_base::guard(None, || Self::wrap_native(self.inner.get_parent()))
    }

    fn get_templated_parent(&self) -> Option<ComPtr<IFrnAutomationPeer>> {
        crate::callback_base::guard(None, || {
            let peer = self.inner.cast::<ControlAutomationPeer>()?;
            let templated_parent = peer.owner().templated_parent()?.cast::<Control>()?;
            Self::wrap_native(Some(ControlAutomationPeer::create_peer_for_element(&templated_parent)))
        })
    }

    fn get_visual_root(&self) -> Option<ComPtr<IFrnAutomationPeer>> {
        crate::callback_base::guard(None, || Self::wrap_native(self.inner.get_automation_root()))
    }

    fn has_keyboard_focus(&self) -> bool {
        crate::callback_base::guard(false, || self.inner.has_keyboard_focus())
    }

    fn is_content_element(&self) -> bool {
        crate::callback_base::guard(false, || self.inner.is_content_element())
    }

    fn is_control_element(&self) -> bool {
        crate::callback_base::guard(false, || self.inner.is_control_element())
    }

    fn is_enabled(&self) -> bool {
        crate::callback_base::guard(false, || self.inner.is_enabled())
    }

    fn is_keyboard_focusable(&self) -> bool {
        crate::callback_base::guard(false, || self.inner.is_keyboard_focusable())
    }

    fn set_focus(&self) {
        crate::callback_base::guard((), || self.inner.set_focus())
    }

    fn show_context_menu(&self) -> bool {
        crate::callback_base::guard(false, || self.inner.show_context_menu())
    }

    fn bring_into_view(&self) {
        crate::callback_base::guard((), || self.inner.bring_into_view())
    }

    fn get_root_peer(&self) -> Option<ComPtr<IFrnAutomationPeer>> {
        crate::callback_base::guard(None, || {
            let mut peer = self.inner.clone();
            let mut parent = peer.get_parent();

            while peer.get_provider::<dyn IRootProvider>().is_none() {
                let Some(p) = parent else { break };
                peer = p;
                parent = peer.get_parent();
            }

            Self::wrap_native(Some(peer))
        })
    }

    fn is_interop_peer(&self) -> bool {
        crate::callback_base::guard(false, || self.inner.is::<InteropAutomationPeer>())
    }

    fn interop_peer_get_native_control_handle(&self) -> *mut c_void {
        crate::callback_base::guard(std::ptr::null_mut(), || {
            let peer = self
                .inner
                .cast::<InteropAutomationPeer>()
                .unwrap_or_else(|| panic!("The peer {} is not an interop peer.", self.inner.get_type().name()));
            peer.native_control_handle().handle() as *mut c_void
        })
    }

    fn is_root_provider(&self) -> bool {
        crate::callback_base::guard(false, || self.is_provider::<dyn IRootProvider>())
    }

    fn root_provider_get_window(&self) -> Option<ComPtr<IFrnWindowBase>> {
        crate::callback_base::guard(None, || {
            let platform_impl = self.provider::<dyn IRootProvider>().platform_impl()?;
            window_base_native(&*platform_impl)
        })
    }

    fn root_provider_get_focus(&self) -> Option<ComPtr<IFrnAutomationPeer>> {
        crate::callback_base::guard(None, || Self::wrap_native(self.provider::<dyn IRootProvider>().get_focus()))
    }

    fn root_provider_get_peer_from_point(&self, point: FrnPoint) -> Option<ComPtr<IFrnAutomationPeer>> {
        crate::callback_base::guard(None, || {
            let result = self.provider::<dyn IRootProvider>().get_peer_from_point(to_ferro_point(point));
            Self::wrap_native(control_element_for_hit_test(result))
        })
    }

    fn is_embedded_root_provider(&self) -> bool {
        crate::callback_base::guard(false, || self.is_provider::<dyn IEmbeddedRootProvider>())
    }

    fn embedded_root_provider_get_focus(&self) -> Option<ComPtr<IFrnAutomationPeer>> {
        crate::callback_base::guard(None, || {
            Self::wrap_native(self.provider::<dyn IEmbeddedRootProvider>().get_focus())
        })
    }

    fn embedded_root_provider_get_peer_from_point(&self, point: FrnPoint) -> Option<ComPtr<IFrnAutomationPeer>> {
        crate::callback_base::guard(None, || {
            let result = self.provider::<dyn IEmbeddedRootProvider>().get_peer_from_point(to_ferro_point(point));
            Self::wrap_native(control_element_for_hit_test(result))
        })
    }

    fn is_expand_collapse_provider(&self) -> bool {
        crate::callback_base::guard(false, || self.is_provider::<dyn IExpandCollapseProvider>())
    }

    fn expand_collapse_provider_get_is_expanded(&self) -> bool {
        crate::callback_base::guard(false, || {
            matches!(
                self.provider::<dyn IExpandCollapseProvider>().expand_collapse_state(),
                ExpandCollapseState::Expanded | ExpandCollapseState::PartiallyExpanded
            )
        })
    }

    fn expand_collapse_provider_get_shows_menu(&self) -> bool {
        crate::callback_base::guard(false, || self.provider::<dyn IExpandCollapseProvider>().shows_menu())
    }

    fn expand_collapse_provider_expand(&self) {
        crate::callback_base::guard((), || raise_on_error(self.provider::<dyn IExpandCollapseProvider>().expand()))
    }

    fn expand_collapse_provider_collapse(&self) {
        crate::callback_base::guard((), || raise_on_error(self.provider::<dyn IExpandCollapseProvider>().collapse()))
    }

    fn is_invoke_provider(&self) -> bool {
        crate::callback_base::guard(false, || self.is_provider::<dyn IInvokeProvider>())
    }

    fn invoke_provider_invoke(&self) {
        crate::callback_base::guard((), || raise_on_error(self.provider::<dyn IInvokeProvider>().invoke()))
    }

    fn is_range_value_provider(&self) -> bool {
        crate::callback_base::guard(false, || self.is_provider::<dyn IRangeValueProvider>())
    }

    fn range_value_provider_get_value(&self) -> f64 {
        crate::callback_base::guard(0.0, || self.provider::<dyn IRangeValueProvider>().value())
    }

    fn range_value_provider_get_minimum(&self) -> f64 {
        crate::callback_base::guard(0.0, || self.provider::<dyn IRangeValueProvider>().minimum())
    }

    fn range_value_provider_get_maximum(&self) -> f64 {
        crate::callback_base::guard(0.0, || self.provider::<dyn IRangeValueProvider>().maximum())
    }

    fn range_value_provider_get_small_change(&self) -> f64 {
        crate::callback_base::guard(0.0, || self.provider::<dyn IRangeValueProvider>().small_change())
    }

    fn range_value_provider_get_large_change(&self) -> f64 {
        crate::callback_base::guard(0.0, || self.provider::<dyn IRangeValueProvider>().large_change())
    }

    fn range_value_provider_set_value(&self, value: f64) {
        crate::callback_base::guard((), || {
            raise_on_error(self.provider::<dyn IRangeValueProvider>().set_value(value))
        })
    }

    fn range_value_provider_is_read_only(&self) -> bool {
        crate::callback_base::guard(false, || self.provider::<dyn IRangeValueProvider>().is_read_only())
    }

    fn is_selection_item_provider(&self) -> bool {
        crate::callback_base::guard(false, || self.is_provider::<dyn ISelectionItemProvider>())
    }

    fn selection_item_provider_is_selected(&self) -> bool {
        crate::callback_base::guard(false, || self.provider::<dyn ISelectionItemProvider>().is_selected())
    }

    fn selection_item_provider_select(&self) {
        crate::callback_base::guard((), || raise_on_error(self.provider::<dyn ISelectionItemProvider>().select()))
    }

    fn selection_item_provider_add_to_selection(&self) {
        crate::callback_base::guard((), || {
            raise_on_error(self.provider::<dyn ISelectionItemProvider>().add_to_selection())
        })
    }

    fn selection_item_provider_remove_from_selection(&self) {
        crate::callback_base::guard((), || {
            raise_on_error(self.provider::<dyn ISelectionItemProvider>().remove_from_selection())
        })
    }

    fn scroll_provider_get_horizontal_scroll_bar(&self) -> Option<ComPtr<IFrnAutomationPeer>> {
        crate::callback_base::guard(None, || {
            let scroll_viewer = self.inner.cast::<ScrollViewerAutomationPeer>()?;
            Self::wrap_native(scroll_viewer.get_horizontal_scroll_bar_peer())
        })
    }

    fn scroll_provider_get_vertical_scroll_bar(&self) -> Option<ComPtr<IFrnAutomationPeer>> {
        crate::callback_base::guard(None, || {
            let scroll_viewer = self.inner.cast::<ScrollViewerAutomationPeer>()?;
            Self::wrap_native(scroll_viewer.get_vertical_scroll_bar_peer())
        })
    }

    fn is_toggle_provider(&self) -> bool {
        crate::callback_base::guard(false, || self.is_provider::<dyn IToggleProvider>())
    }

    fn toggle_provider_get_toggle_state(&self) -> i32 {
        crate::callback_base::guard(0, || self.provider::<dyn IToggleProvider>().toggle_state() as i32)
    }

    fn toggle_provider_toggle(&self) {
        crate::callback_base::guard((), || raise_on_error(self.provider::<dyn IToggleProvider>().toggle()))
    }

    fn is_value_provider(&self) -> bool {
        crate::callback_base::guard(false, || self.is_provider::<dyn IValueProvider>())
    }

    fn value_provider_get_value(&self) -> Option<ComPtr<IFrnString>> {
        crate::callback_base::guard(None, || to_frn_string(self.provider::<dyn IValueProvider>().value().as_deref()))
    }

    fn value_provider_set_value(&self, value: Option<&CStr>) {
        crate::callback_base::guard((), || {
            let value = value.map(|value| value.to_string_lossy().into_owned());
            if let Err(e) = self.provider::<dyn IValueProvider>().set_value(value.as_deref()) {
                // The value provider reports any error its setter throws in the reference.
                crate::callback_base::raise_exception(Box::new(e));
            }
        })
    }

    fn value_provider_is_read_only(&self) -> bool {
        crate::callback_base::guard(false, || self.provider::<dyn IValueProvider>().is_read_only())
    }

    fn get_help_text(&self) -> Option<ComPtr<IFrnString>> {
        crate::callback_base::guard(None, || to_frn_string(Some(&self.inner.get_help_text())))
    }

    fn get_placeholder_text(&self) -> Option<ComPtr<IFrnString>> {
        crate::callback_base::guard(None, || to_frn_string(Some(&self.inner.get_placeholder_text())))
    }

    fn get_landmark_type(&self) -> FrnLandmarkType {
        crate::callback_base::guard(FrnLandmarkType::LandmarkNone, || {
            self.inner.get_landmark_type().map_or(FrnLandmarkType::LandmarkNone, |t| FrnLandmarkType(t as i32))
        })
    }

    fn get_heading_level(&self) -> i32 {
        crate::callback_base::guard(0, || self.inner.get_heading_level())
    }

    fn get_live_setting(&self) -> FrnLiveSetting {
        crate::callback_base::guard(FrnLiveSetting::default(), || FrnLiveSetting(self.inner.get_live_setting() as i32))
    }
}

/// A list of automation peers exposed to native code.
pub(crate) struct FrnAutomationPeerArray {
    items: Vec<Rc<FrnAutomationPeer>>,
}

impl FrnAutomationPeerArray {
    pub(crate) fn new(items: &[Ref<AutomationPeer>]) -> Self {
        Self { items: items.iter().filter_map(|x| FrnAutomationPeer::wrap(Some(x.clone()))).collect() }
    }
}

impl IFrnAutomationPeerArrayImpl for FrnAutomationPeerArray {
    fn get_count(&self) -> u32 {
        self.items.len() as u32
    }

    fn get(&self, index: u32) -> Result<Option<ComPtr<IFrnAutomationPeer>>, HResult> {
        match self.items.get(index as usize) {
            Some(item) => Ok(Some(IFrnAutomationPeer::from_impl(item.clone()))),
            None => Err(HResult::INVALIDARG),
        }
    }
}

// Not from upstream: the reference has no tests of its native automation
// wrapper. These drive the wrapper through its interface pointers, as native
// code does, with a recording node in place of an accessibility element.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::frn_string::frn_string_to_string;
    use ferroui_base::threading::Dispatcher;
    use ferroui_controls::automation::AutomationProperties;
    use ferroui_controls::{Border, Button, CheckBox, Expander, Panel};
    use std::cell::Cell;

    struct TestScope {
        _dispatcher: ferroui_base::threading::UnitTestDispatcherScope,
    }

    fn test_scope() -> TestScope {
        Control::reset_loaded_queue_for_unit_tests();
        TestScope { _dispatcher: Dispatcher::unit_test_scope() }
    }

    impl Drop for TestScope {
        fn drop(&mut self) {
            Control::reset_loaded_queue_for_unit_tests();
        }
    }

    #[derive(Default)]
    struct NodeEvents {
        disposed: Cell<u32>,
        children_changed: Cell<u32>,
        focus_changed: Cell<u32>,
        properties: RefCell<Vec<FrnAutomationProperty>>,
    }

    struct RecordingNode(Rc<NodeEvents>);

    impl IFrnAutomationNodeImpl for RecordingNode {
        fn dispose(&self) {
            self.0.disposed.set(self.0.disposed.get() + 1);
        }

        fn children_changed(&self) {
            self.0.children_changed.set(self.0.children_changed.get() + 1);
        }

        fn property_changed(&self, property: FrnAutomationProperty) {
            self.0.properties.borrow_mut().push(property);
        }

        fn focus_changed(&self) {
            self.0.focus_changed.set(self.0.focus_changed.get() + 1);
        }
    }

    fn peer_of(control: &Control) -> Ref<AutomationPeer> {
        ControlAutomationPeer::create_peer_for_element(control)
    }

    fn native(control: &Control) -> ComPtr<IFrnAutomationPeer> {
        FrnAutomationPeer::wrap_native(Some(peer_of(control))).expect("a wrapper")
    }

    fn text(s: Option<ComPtr<IFrnString>>) -> Option<String> {
        s.and_then(|s| frn_string_to_string(&s))
    }

    /// Native code finds the accessibility element of a peer through the
    /// node of its wrapper, so every wrapping of a peer must lead to it.
    #[test]
    fn wrap_returns_the_same_wrapper_for_the_same_peer() {
        let _scope = test_scope();
        let button = Button::new();
        let peer = peer_of(&button);

        let first = FrnAutomationPeer::wrap(Some(peer.clone())).unwrap();
        let second = FrnAutomationPeer::wrap(Some(peer.clone())).unwrap();
        assert!(Rc::ptr_eq(&first, &second));

        let other = FrnAutomationPeer::wrap(Some(peer_of(&Border::new()))).unwrap();
        assert!(!Rc::ptr_eq(&first, &other));

        assert!(FrnAutomationPeer::wrap(None).is_none());
    }

    #[test]
    fn node_set_through_one_wrapping_is_seen_through_another() {
        let _scope = test_scope();
        let button = Button::new();
        let events = Rc::new(NodeEvents::default());
        let node = IFrnAutomationNode::from_impl(RecordingNode(events.clone()));

        let first = native(&button);
        assert!(first.get_node().is_none());
        first.set_node(Some(&node));

        let second = native(&button);
        let found = second.get_node().expect("the node");
        assert_eq!(found.as_ptr(), node.as_ptr());
    }

    #[test]
    fn a_second_node_is_refused() {
        let _scope = test_scope();
        let button = Button::new();
        let first_node = IFrnAutomationNode::from_impl(RecordingNode(Rc::default()));
        let second_node = IFrnAutomationNode::from_impl(RecordingNode(Rc::default()));
        let peer = native(&button);
        peer.set_node(Some(&first_node));

        // The refusal is handed to the dispatcher implementation (there is
        // none on a test thread); the first node stays.
        let hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        peer.set_node(Some(&second_node));
        std::panic::set_hook(hook);

        assert_eq!(peer.get_node().unwrap().as_ptr(), first_node.as_ptr());
    }

    #[test]
    fn mapped_property_changes_reach_the_node() {
        let _scope = test_scope();
        let button = Button::new();
        let events = Rc::new(NodeEvents::default());
        let peer = native(&button);
        peer.set_node(Some(&IFrnAutomationNode::from_impl(RecordingNode(events.clone()))));

        AutomationProperties::set_automation_id(&button, Some("ok-button"));
        peer_of(&button).raise_property_changed_event(AutomationElementIdentifiers::name_property(), None, None);

        assert_eq!(
            *events.properties.borrow(),
            vec![FrnAutomationProperty::AutomationPeer_AutomationId, FrnAutomationProperty::AutomationPeer_Name]
        );
    }

    #[test]
    fn unmapped_property_changes_do_not_reach_the_node() {
        let _scope = test_scope();
        let button = Button::new();
        let events = Rc::new(NodeEvents::default());
        let peer = native(&button);
        peer.set_node(Some(&IFrnAutomationNode::from_impl(RecordingNode(events.clone()))));

        peer_of(&button).raise_property_changed_event(AutomationElementIdentifiers::help_text_property(), None, None);

        assert!(events.properties.borrow().is_empty());
    }

    #[test]
    fn children_changes_reach_the_node() {
        let _scope = test_scope();
        let panel = Panel::new();
        let events = Rc::new(NodeEvents::default());
        let peer = native(&panel);
        peer.set_node(Some(&IFrnAutomationNode::from_impl(RecordingNode(events.clone()))));

        peer_of(&panel).raise_children_changed_event();

        assert_eq!(events.children_changed.get(), 1);
    }

    #[test]
    fn handlers_are_removed_with_the_wrapper() {
        let _scope = test_scope();
        let panel = Panel::new();
        let peer = peer_of(&panel);
        let events = Rc::new(NodeEvents::default());
        let node = IFrnAutomationNode::from_impl(RecordingNode(events.clone()));
        let wrapper = FrnAutomationPeer::wrap_native(Some(peer.clone())).unwrap();
        wrapper.set_node(Some(&node));

        drop(wrapper);
        // The last reference took the wrapper and released its node, as the
        // finalizer of the reference does.
        assert_eq!(events.disposed.get(), 1);

        peer.raise_children_changed_event();
        assert_eq!(events.children_changed.get(), 0);
        assert!(native(&panel).get_node().is_none());
    }

    #[test]
    fn children_are_wrapped_in_order() {
        let _scope = test_scope();
        let panel = Panel::new();
        let first = Button::new();
        AutomationProperties::set_name(&first, Some("first"));
        let second = Button::new();
        AutomationProperties::set_name(&second, Some("second"));
        panel.children().add(first.clone());
        panel.children().add(second.clone());

        let children = native(&panel).get_children().expect("children");

        assert_eq!(children.get_count(), 2);
        let names: Vec<_> =
            (0..2).map(|i| text(children.get(i).unwrap().unwrap().get_name()).unwrap_or_default()).collect();
        assert_eq!(names, ["first", "second"]);
        assert_eq!(children.get(2).err(), Some(HResult::INVALIDARG));

        let parent = children.get(0).unwrap().unwrap().get_parent().expect("a parent");
        assert_eq!(text(parent.get_class_name()), Some("Panel".to_string()));
    }

    #[test]
    fn element_members_come_from_the_peer() {
        let _scope = test_scope();
        let button = Button::new();
        AutomationProperties::set_automation_id(&button, Some("ok-button"));
        AutomationProperties::set_name(&button, Some("OK"));
        AutomationProperties::set_help_text(&button, Some("Confirms"));
        let peer = native(&button);

        assert_eq!(peer.get_automation_control_type(), FrnAutomationControlType::AutomationButton);
        assert_eq!(text(peer.get_automation_id()), Some("ok-button".to_string()));
        assert_eq!(text(peer.get_name()), Some("OK".to_string()));
        assert_eq!(text(peer.get_help_text()), Some("Confirms".to_string()));
        assert_eq!(text(peer.get_class_name()), Some("Button".to_string()));
        assert!(text(peer.get_accelerator_key()).is_none());
        assert_eq!(peer.get_landmark_type(), FrnLandmarkType::LandmarkNone);
        assert_eq!(peer.get_live_setting(), FrnLiveSetting::LiveSettingOff);
        assert!(peer.is_control_element());
        assert!(peer.is_enabled());
        assert!(!peer.is_interop_peer());
        assert!(peer.get_templated_parent().is_none());
        assert!(peer.get_parent().is_none());

        // Without a root provider above it, the peer is its own root.
        let root = peer.get_root_peer().expect("a root");
        assert_eq!(text(root.get_name()), Some("OK".to_string()));
    }

    #[test]
    fn invoke_provider_clicks_the_button() {
        let _scope = test_scope();
        let button = Button::new();
        let clicks = Rc::new(Cell::new(0));
        let count = clicks.clone();
        button.click(move |_, _| count.set(count.get() + 1));
        let peer = native(&button);

        assert!(peer.is_invoke_provider());
        assert!(!peer.is_toggle_provider());
        assert!(!peer.is_range_value_provider());
        assert!(!peer.is_root_provider());
        peer.invoke_provider_invoke();

        assert_eq!(clicks.get(), 1);
    }

    #[test]
    fn invoking_a_disabled_button_does_nothing() {
        let _scope = test_scope();
        let button = Button::new();
        button.set_is_enabled(false);
        let clicks = Rc::new(Cell::new(0));
        let count = clicks.clone();
        button.click(move |_, _| count.set(count.get() + 1));

        native(&button).invoke_provider_invoke();

        assert_eq!(clicks.get(), 0);
    }

    #[test]
    fn toggle_provider_reports_and_changes_the_state() {
        let _scope = test_scope();
        let check_box = CheckBox::new();
        let events = Rc::new(NodeEvents::default());
        let peer = native(&check_box);
        peer.set_node(Some(&IFrnAutomationNode::from_impl(RecordingNode(events.clone()))));

        assert!(peer.is_toggle_provider());
        assert_eq!(peer.toggle_provider_get_toggle_state(), 0);
        peer.toggle_provider_toggle();

        assert_eq!(check_box.is_checked(), Some(true));
        assert_eq!(peer.toggle_provider_get_toggle_state(), 1);
        assert!(events.properties.borrow().contains(&FrnAutomationProperty::ToggleProvider_ToggleState));
    }

    #[test]
    fn expand_collapse_provider_drives_the_expander() {
        let _scope = test_scope();
        let expander = Expander::new();
        let peer = native(&expander);

        assert!(peer.is_expand_collapse_provider());
        assert!(!peer.expand_collapse_provider_get_is_expanded());
        peer.expand_collapse_provider_expand();
        assert!(expander.is_expanded());
        assert!(peer.expand_collapse_provider_get_is_expanded());
        peer.expand_collapse_provider_collapse();
        assert!(!expander.is_expanded());
    }

    #[test]
    fn a_missing_provider_answers_with_the_default() {
        let _scope = test_scope();
        let peer = native(&Border::new());

        // The reference throws; the failure is handed to the dispatcher
        // implementation and native code gets the default value.
        let hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        let value = peer.range_value_provider_get_value();
        std::panic::set_hook(hook);

        assert_eq!(value, 0.0);
    }

    #[test]
    fn hit_test_result_moves_up_to_a_control_element() {
        let _scope = test_scope();
        let panel = Panel::new();
        let decorator = Border::new();
        let inner = Border::new();
        decorator.set_child(inner.clone());
        panel.children().add(decorator.clone());
        AutomationProperties::set_is_control_element_override(&inner, Some(false));
        AutomationProperties::set_is_control_element_override(&decorator, Some(false));

        // Neither border is a control element: the walk passes both and stops
        // at the panel.
        let result = control_element_for_hit_test(Some(peer_of(&inner)));
        assert_eq!(result, Some(peer_of(&panel)));

        // A control element is the answer itself.
        let result = control_element_for_hit_test(Some(peer_of(&panel)));
        assert_eq!(result, Some(peer_of(&panel)));

        // Without a control element above it, the walk ends at the top.
        AutomationProperties::set_is_control_element_override(&panel, Some(false));
        let result = control_element_for_hit_test(Some(peer_of(&inner)));
        assert_eq!(result, Some(peer_of(&panel)));

        assert!(control_element_for_hit_test(None).is_none());
    }
}

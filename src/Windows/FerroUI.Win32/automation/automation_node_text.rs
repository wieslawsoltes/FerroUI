//! The native text pattern of a node. The provider methods follow WinUI's
//! `CUIATextProviderWrapper` at `fe244ca6ac4d482712b234a9de3cf771cc6481de`
//! (Microsoft Corporation, MIT), using the existing FerroUI automation node's
//! dispatcher and identity instead of a parallel native document.

use super::automation_node::{AutomationNode, SendProvider, ELEMENT_NOT_AVAILABLE};
use super::interop::{
    IRawElementProviderSimple, ITextChildProviderImpl, ITextEditProviderImpl, ITextProvider2Impl, ITextProviderImpl,
    ITextRangeProvider, SupportedTextSelection, UiaCoreProviderApi, UiaEventId, UiaPoint,
};
use super::marshalling::{SafeArrayMarshaller, SafeArrayRef};
use super::root_automation_node::RootAutomationNode;
use super::text_pattern_marshalling::{identity_of, provider_error};
use super::text_range_node::{SendTextRange, TextRangeNode};
use ferroui_base::reactive::IDisposable;
use ferroui_base::{PixelPoint, PixelRect, Point, Rect, Ref};
use ferroui_controls::automation::peers::{AutomationPeer, ControlAutomationPeer};
use ferroui_controls::automation::provider::{ITextChildProvider, ITextEditProvider, ITextProvider, ITextProvider2, TextEditTextChangedEventArgs};
use ferroui_microcom::{ComPtr, HResult, E_FAIL, E_INVALIDARG};
use std::rc::Rc;

impl AutomationNode {
    /// The subscriptions use only the node identifier. They do not retain the
    /// peer, its control, or the provider adapter.
    pub(crate) fn text_subscriptions(
        peer: &Ref<AutomationPeer>,
        id: i32,
    ) -> Vec<Rc<dyn IDisposable>> {
        let Some(provider) = peer.get_provider::<dyn ITextProvider>() else {
            return Vec::new();
        };
        let mut subscriptions = Vec::new();
        if let Ok(subscription) = provider.text_changed(Rc::new(move || {
            Self::raise_text_event(id, UiaEventId::Text_TextChanged)
        })) {
            subscriptions.push(subscription);
        }
        if let Ok(subscription) = provider.text_selection_changed(Rc::new(move || {
            Self::raise_text_event(id, UiaEventId::Text_TextSelectionChanged)
        })) {
            subscriptions.push(subscription);
        }
        if let Some(provider) = peer.get_provider::<dyn ITextEditProvider>() {
            if !provider.raises_text_edit_events_through_peer() {
                if let Ok(subscription) = provider.text_edit_text_changed(Rc::new(move |event| {
                    Self::raise_text_edit_event(id, event)
                })) {
                    subscriptions.push(subscription);
                }
            }
        }
        subscriptions
    }

    fn raise_text_edit_event(id: i32, event: &TextEditTextChangedEventArgs) {
        if let Some(peer) = Self::peer_of(id) {
            peer.raise_text_edit_text_changed_event(event.change_type, event.changed_data.clone());
        }
    }

    fn raise_text_event(id: i32, event: UiaEventId) {
        // Events run on the UI thread, like the existing property and selection
        // events. A released/disconnected peer cannot raise a new native event.
        if Self::peer_of(id).is_none() {
            return;
        }
        if let Some(node) = Self::provider_of(id) {
            UiaCoreProviderApi::uia_raise_automation_event(Some(&node), event.0);
        }
    }
}

impl ITextProviderImpl for AutomationNode {
    fn get_selection(&self) -> Result<SafeArrayRef, HResult> {
        let owner = self.id();
        let ranges = self.invoke_sync_provider(move |provider: &Rc<dyn ITextProvider>| {
            provider
                .get_selection()
                .map(|ranges| {
                    ranges
                        .into_iter()
                        .map(|range| TextRangeNode::new(owner, range))
                        .collect::<Vec<_>>()
                })
                .map_err(provider_error)
        })?;
        range_array(ranges)
    }

    fn get_visible_ranges(&self) -> Result<SafeArrayRef, HResult> {
        let owner = self.id();
        let ranges = self.invoke_sync_provider(move |provider: &Rc<dyn ITextProvider>| {
            provider
                .get_visible_ranges()
                .map(|ranges| {
                    ranges
                        .into_iter()
                        .map(|range| TextRangeNode::new(owner, range))
                        .collect::<Vec<_>>()
                })
                .map_err(provider_error)
        })?;
        range_array(ranges)
    }

    fn range_from_child(
        &self,
        child_element: Option<&IRawElementProviderSimple>,
    ) -> Result<Option<ComPtr<ITextRangeProvider>>, HResult> {
        let identity = identity_of(child_element.ok_or(HResult(E_INVALIDARG))?)?;
        let key = identity.as_ptr() as usize;
        let owner = self.id();
        let range = self.invoke_sync_provider(move |provider: &Rc<dyn ITextProvider>| {
            // Foreign native objects cannot be cast to a Rust node. Resolve only
            // identities registered by this backend and let the provider check
            // that the peer belongs to its own text document.
            let child = Self::peer_from_identity(key).ok_or(HResult(E_INVALIDARG))?;
            provider
                .range_from_child(&child)
                .map(|range| TextRangeNode::new(owner, range))
                .map_err(provider_error)
        })?;
        Ok(Some(range.into_inner()))
    }

    fn range_from_point(
        &self,
        point: UiaPoint,
    ) -> Result<Option<ComPtr<ITextRangeProvider>>, HResult> {
        if !point.x.is_finite()
            || !point.y.is_finite()
            || point.x < i32::MIN as f64
            || point.x > i32::MAX as f64
            || point.y < i32::MIN as f64
            || point.y > i32::MAX as f64
        {
            return Err(HResult(E_INVALIDARG));
        }
        let owner = self.id();
        let range = self.invoke_sync_provider(move |provider: &Rc<dyn ITextProvider>| {
            let point = from_screen(&provider.peer(), point)?;
            provider
                .range_from_point(point)
                .map(|range| TextRangeNode::new(owner, range))
                .map_err(provider_error)
        })?;
        Ok(Some(range.into_inner()))
    }

    fn get_document_range(&self) -> Result<Option<ComPtr<ITextRangeProvider>>, HResult> {
        let owner = self.id();
        let range = self.invoke_sync_provider(move |provider: &Rc<dyn ITextProvider>| {
            provider
                .document_range()
                .map(|range| TextRangeNode::new(owner, range))
                .map_err(provider_error)
        })?;
        Ok(Some(range.into_inner()))
    }

    fn get_supported_text_selection(&self) -> Result<SupportedTextSelection, HResult> {
        self.invoke_sync_provider(|provider: &Rc<dyn ITextProvider>| {
            provider
                .supported_text_selection()
                .map(|value| SupportedTextSelection(value as i32))
                .map_err(provider_error)
        })
    }
}

impl ITextProvider2Impl for AutomationNode {
    fn range_from_annotation(
        &self,
        annotation_element: Option<&IRawElementProviderSimple>,
    ) -> Result<Option<ComPtr<ITextRangeProvider>>, HResult> {
        let identity = identity_of(annotation_element.ok_or(HResult(E_INVALIDARG))?)?;
        let key = identity.as_ptr() as usize;
        let owner = self.id();
        let range = self.invoke_sync_provider(move |provider: &Rc<dyn ITextProvider2>| {
            let annotation = Self::peer_from_identity(key).ok_or(HResult(E_INVALIDARG))?;
            provider
                .range_from_annotation(&annotation)
                .map(|range| TextRangeNode::new(owner, range))
                .map_err(provider_error)
        })?;
        Ok(Some(range.into_inner()))
    }

    fn get_caret_range(
        &self,
        is_active: *mut i32,
    ) -> Result<Option<ComPtr<ITextRangeProvider>>, HResult> {
        if is_active.is_null() {
            return Err(HResult(E_INVALIDARG));
        }
        // SAFETY: UIA supplies writable BOOL storage for this synchronous COM
        // call. Null is rejected above. The pointer stays on the caller's thread
        // and never enters the dispatched closure; only owned results cross it.
        unsafe {
            *is_active = 0;
        }
        let owner = self.id();
        let (active, range) =
            self.invoke_sync_provider(move |provider: &Rc<dyn ITextProvider2>| {
                let caret = provider.get_caret_range().map_err(provider_error)?;
                Ok((
                    caret.is_active,
                    caret.range.map(|range| TextRangeNode::new(owner, range)),
                ))
            })?;
        // SAFETY: same caller-owned BOOL storage, still live until this call
        // returns. A failed dispatched operation leaves its initialized false.
        unsafe {
            *is_active = i32::from(active);
        }
        Ok(range.map(SendTextRange::into_inner))
    }
}

impl ITextEditProviderImpl for AutomationNode {
    fn get_active_composition(&self) -> Result<Option<ComPtr<ITextRangeProvider>>, HResult> {
        let owner = self.id();
        let range = self.invoke_sync_provider(move |provider: &Rc<dyn ITextEditProvider>| {
            provider
                .get_active_composition()
                .map(|range| range.map(|range| TextRangeNode::new(owner, range)))
                .map_err(provider_error)
        })?;
        Ok(range.map(SendTextRange::into_inner))
    }

    fn get_conversion_target(&self) -> Result<Option<ComPtr<ITextRangeProvider>>, HResult> {
        let owner = self.id();
        let range = self.invoke_sync_provider(move |provider: &Rc<dyn ITextEditProvider>| {
            provider
                .get_conversion_target()
                .map(|range| range.map(|range| TextRangeNode::new(owner, range)))
                .map_err(provider_error)
        })?;
        Ok(range.map(SendTextRange::into_inner))
    }
}

impl ITextChildProviderImpl for AutomationNode {
    fn get_text_container(&self) -> Result<Option<ComPtr<IRawElementProviderSimple>>, HResult> {
        let container = self.invoke_sync_provider(|provider: &Rc<dyn ITextChildProvider>| {
            let peer = provider.text_container().map_err(provider_error)?;
            Self::get_or_create(Some(&peer))
                .map(SendProvider)
                .ok_or(ELEMENT_NOT_AVAILABLE)
        })?;
        Ok(Some(container.into_inner()))
    }

    fn get_text_range(&self) -> Result<Option<ComPtr<ITextRangeProvider>>, HResult> {
        let range = self.invoke_sync_provider(|provider: &Rc<dyn ITextChildProvider>| {
            let range = provider.text_range().map_err(provider_error)?;
            let peer = range.peer().map_err(provider_error)?;
            let owner = Self::id_for_peer(&peer).ok_or(ELEMENT_NOT_AVAILABLE)?;
            Ok(TextRangeNode::new(owner, range))
        })?;
        Ok(Some(range.into_inner()))
    }
}

fn range_array(ranges: Vec<SendTextRange>) -> Result<SafeArrayRef, HResult> {
    let ranges: Vec<_> = ranges.into_iter().map(SendTextRange::into_inner).collect();
    SafeArrayMarshaller::convert_to_unmanaged(Some(&ranges)).map_err(|_| HResult(E_FAIL))
}

fn from_screen(peer: &Ref<AutomationPeer>, point: UiaPoint) -> Result<Point, HResult> {
    let pixel = PixelPoint::new(point.x.round() as i32, point.y.round() as i32);
    if let Some(control_peer) = peer.cast::<ControlAutomationPeer>() {
        // Use exactly the presentation root that ControlAutomationPeer uses
        // for `to_screen`, including popup and embedded presentation roots.
        let owner = control_peer.try_owner().ok_or(ELEMENT_NOT_AVAILABLE)?;
        let source = owner.presentation_source().ok_or(HResult(E_FAIL))?;
        source.root_visual().ok_or(HResult(E_FAIL))?;
        return source.point_to_client(pixel).ok_or(HResult(E_FAIL));
    }
    let root = peer.get_visual_root().ok_or(HResult(E_FAIL))?;
    let platform = RootAutomationNode::window_impl(&root).ok_or(HResult(E_FAIL))?;
    Ok(platform.point_to_client(pixel))
}

pub(crate) fn to_screen(peer: &Ref<AutomationPeer>, rect: Rect) -> Result<Rect, HResult> {
    if let Some(control_peer) = peer.cast::<ControlAutomationPeer>() {
        let owner = control_peer.try_owner().ok_or(ELEMENT_NOT_AVAILABLE)?;
        let source = owner.presentation_source().ok_or(HResult(E_FAIL))?;
        source.root_visual().ok_or(HResult(E_FAIL))?;
        let first = source
            .point_to_screen(rect.top_left())
            .ok_or(HResult(E_FAIL))?;
        let last = source
            .point_to_screen(rect.bottom_right())
            .ok_or(HResult(E_FAIL))?;
        return Ok(PixelRect::from_points(first, last).to_rect(1.0));
    }
    peer.to_screen(rect).ok_or(HResult(E_FAIL))
}

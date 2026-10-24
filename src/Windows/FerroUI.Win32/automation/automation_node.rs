//! The provider of an automation peer: what a client of UI Automation
//! holds of an element (the port of `AutomationNode.cs`).
//!
//! # Threads
//!
//! A client calls a provider on a thread of the automation core, and the
//! peers are objects of the UI thread. The reference runs every call on the
//! UI thread (`InvokeSync`) and holds the peer in the provider. Here the two
//! halves are apart, so that the compiler holds the rule:
//!
//! - [`AutomationNode`] is the COM object. It holds nothing of the UI
//!   thread: its number, its kind and the dispatcher of the thread it was
//!   made on. It is `Send` and `Sync`, and may be called and released on
//!   any thread.
//! - the table of the UI thread has, for every number, the peer and the
//!   provider. `invoke_sync` runs a function on the UI thread with the peer
//!   of the node, directly when the call is on that thread already and
//!   through the dispatcher (waiting for the result) otherwise; only values
//!   that are `Send` come back.
//!
//! # Lifetimes
//!
//! The reference keeps its nodes in a table with weak keys: a node lives as
//! long as its peer, and a provider a client holds keeps the peer alive.
//! Here the table holds a peer weakly and the provider strongly; an entry
//! goes when its peer is gone (the table is swept as it grows). A provider
//! a client still holds then answers `UIA_E_ELEMENTNOTAVAILABLE`, where the
//! reference would keep the peer, and with it the control, alive for the
//! client (docs/porting/DEVIATIONS.md).

use super::interop::{
    IExpandCollapseProvider, IInvokeProvider, IInvokeProviderImpl, IRangeValueProvider, IRawElementProviderFragment,
    IRawElementProviderFragmentImpl, IRawElementProviderFragmentRoot, IRawElementProviderFragmentRootImpl,
    IRawElementProviderSimple, IRawElementProviderSimple2, IRawElementProviderSimple2Impl, IRawElementProviderSimpleImpl,
    IScrollItemProvider, IScrollProvider, ISelectionItemProvider, ISelectionProvider, IToggleProvider, IValueProvider,
    NavigateDirection, ProviderOptions, StructureChangeType, UiaControlTypeId, UiaCoreProviderApi, UiaCoreTypesApi,
    UiaEventId, UiaHeadingLevel, UiaLandmarkType, UiaLiveSetting, UiaPatternId, UiaPropertyId, UiaRect,
};
use super::interop_automation_node::InteropAutomationNode;
use super::marshalling::{ComVariant, ComVariantMarshaller, SafeArrayMarshaller, SafeArrayRef, VariantArray, VariantValue};
use super::root_automation_node::RootAutomationNode;
use ferroui_base::reactive::IDisposable;
use ferroui_base::threading::Dispatcher;
use ferroui_base::utilities::CultureInfo;
use ferroui_base::{Rect, Ref, WeakRef};
use ferroui_controls::automation::peers::{
    AutomationControlType, AutomationLandmarkType, AutomationPeer, InteropAutomationPeer,
};
use ferroui_controls::automation::provider as aap;
use ferroui_controls::automation::{
    AutomationElementIdentifiers, AutomationLiveSetting, AutomationProperty, AutomationPropertyChangedEventArgs,
    ElementNotEnabledException, ExpandCollapsePatternIdentifiers, ExpandCollapseState, RangeValuePatternIdentifiers,
    ScrollPatternIdentifiers, SelectionItemPatternIdentifiers, SelectionPatternIdentifiers, TogglePatternIdentifiers,
};
use ferroui_microcom::{make_com_with, ComPtr, HResult, IUnknown, InterfaceEntry, E_ABORT, E_FAIL, E_INVALIDARG};
use std::cell::RefCell;
use std::collections::HashMap;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Arc, OnceLock};

/// The element of the provider is gone (`UIA_E_ELEMENTNOTAVAILABLE`).
pub(crate) const ELEMENT_NOT_AVAILABLE: HResult = HResult(UiaCoreTypesApi::UIA_E_ELEMENTNOTAVAILABLE as u32);
/// The element is not enabled (`UIA_E_ELEMENTNOTENABLED`).
pub(crate) const ELEMENT_NOT_ENABLED: HResult = HResult(UiaCoreProviderApi::UIA_E_ELEMENTNOTENABLED as u32);
/// The peer has no provider of the pattern (the result code of the
/// `NotSupportedException` of the reference).
pub(crate) const NOT_SUPPORTED: HResult = HResult(0x8013_1515);

/// What a node is: the classes of the reference that derive from the node
/// are kinds of the one COM class here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NodeKind {
    /// `AutomationNode`.
    Node,
    /// `RootAutomationNode`: the root of the tree of a window.
    Root,
    /// `InteropAutomationNode`: the root of the tree of an embedded native
    /// control, with the handle of its window.
    Interop { handle: isize },
}

/// A COM pointer of a node that travels between threads.
pub(crate) struct SendProvider(pub(crate) ComPtr<IRawElementProviderSimple>);

// SAFETY: the pointer is one of an `AutomationNode`, which is `Send` and
// `Sync` (see the assertion below) behind the atomic reference count of
// the COM runtime of the port: it may be called and released on any thread.
unsafe impl Send for SendProvider {}

impl SendProvider {
    /// The pointer, in place.
    #[allow(dead_code)] // For tests.
    pub(crate) fn get(&self) -> &ComPtr<IRawElementProviderSimple> {
        &self.0
    }

    pub(crate) fn into_inner(self) -> ComPtr<IRawElementProviderSimple> {
        self.0
    }
}

/// The half of a node that lives on the UI thread.
struct NodeEntry {
    peer: WeakRef<AutomationPeer>,
    kind: NodeKind,
    provider: ComPtr<IRawElementProviderSimple>,
    subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
}

#[derive(Default)]
struct NodeTable {
    /// The number of the node of a peer, by the address of the peer.
    by_peer: HashMap<usize, i32>,
    by_id: HashMap<i32, Rc<NodeEntry>>,
    /// The size at which the entries of peers that are gone are removed.
    sweep_at: usize,
}

ferroui_base::ferro_thread_local! {
    static S_NODES: RefCell<NodeTable> = RefCell::new(NodeTable::default());
}

static S_NEXT_ID: AtomicI32 = AtomicI32::new(1);

fn peer_key(peer: &Ref<AutomationPeer>) -> usize {
    std::ptr::from_ref::<AutomationPeer>(peer) as usize
}

/// The provider of an automation peer. One COM object with the interfaces
/// of the class of the reference: the element itself, a fragment, and the
/// patterns, of which a client gets the ones the peer provides.
pub(crate) struct AutomationNode {
    id: i32,
    runtime_id: [i32; 2],
    kind: NodeKind,
    dispatcher: Arc<Dispatcher>,
    /// The object itself, not counted: it is read while a method of the
    /// object runs, when the object is alive.
    this: OnceLock<usize>,
}

const _: () = {
    const fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<AutomationNode>();
};

impl AutomationNode {
    /// The map of the reference from the properties of the peers to the
    /// property identifiers of the system (`s_propertyMap`).
    fn property_map(property: &'static AutomationProperty) -> Option<UiaPropertyId> {
        ferroui_base::ferro_thread_local! {
            static MAP: Vec<(&'static AutomationProperty, UiaPropertyId)> = vec![
                (AutomationElementIdentifiers::automation_id_property(), UiaPropertyId::AutomationId),
                (AutomationElementIdentifiers::bounding_rectangle_property(), UiaPropertyId::BoundingRectangle),
                (AutomationElementIdentifiers::class_name_property(), UiaPropertyId::ClassName),
                (AutomationElementIdentifiers::name_property(), UiaPropertyId::Name),
                (AutomationElementIdentifiers::help_text_property(), UiaPropertyId::HelpText),
                (AutomationElementIdentifiers::landmark_type_property(), UiaPropertyId::LandmarkType),
                (AutomationElementIdentifiers::heading_level_property(), UiaPropertyId::HeadingLevel),
                (AutomationElementIdentifiers::item_status_property(), UiaPropertyId::ItemStatus),
                (ExpandCollapsePatternIdentifiers::expand_collapse_state_property(), UiaPropertyId::ExpandCollapseExpandCollapseState),
                (RangeValuePatternIdentifiers::is_read_only_property(), UiaPropertyId::RangeValueIsReadOnly),
                (RangeValuePatternIdentifiers::maximum_property(), UiaPropertyId::RangeValueMaximum),
                (RangeValuePatternIdentifiers::minimum_property(), UiaPropertyId::RangeValueMinimum),
                (RangeValuePatternIdentifiers::value_property(), UiaPropertyId::RangeValueValue),
                (ScrollPatternIdentifiers::horizontally_scrollable_property(), UiaPropertyId::ScrollHorizontallyScrollable),
                (ScrollPatternIdentifiers::horizontal_scroll_percent_property(), UiaPropertyId::ScrollHorizontalScrollPercent),
                (ScrollPatternIdentifiers::horizontal_view_size_property(), UiaPropertyId::ScrollHorizontalViewSize),
                (ScrollPatternIdentifiers::vertically_scrollable_property(), UiaPropertyId::ScrollVerticallyScrollable),
                (ScrollPatternIdentifiers::vertical_scroll_percent_property(), UiaPropertyId::ScrollVerticalScrollPercent),
                (ScrollPatternIdentifiers::vertical_view_size_property(), UiaPropertyId::ScrollVerticalViewSize),
                (SelectionPatternIdentifiers::can_select_multiple_property(), UiaPropertyId::SelectionCanSelectMultiple),
                (SelectionPatternIdentifiers::is_selection_required_property(), UiaPropertyId::SelectionIsSelectionRequired),
                (SelectionPatternIdentifiers::selection_property(), UiaPropertyId::SelectionSelection),
                (SelectionItemPatternIdentifiers::is_selected_property(), UiaPropertyId::SelectionItemIsSelected),
                (SelectionItemPatternIdentifiers::selection_container_property(), UiaPropertyId::SelectionItemSelectionContainer),
                (TogglePatternIdentifiers::toggle_state_property(), UiaPropertyId::ToggleToggleState),
            ];
        }
        MAP.with(|map| map.iter().find(|(key, _)| *key == property).map(|(_, id)| *id))
    }

    /// The constructor of the reference: the node of `peer` with its place
    /// in the table and its subscriptions to the events of the peer.
    fn new(peer: &Ref<AutomationPeer>, kind: NodeKind) -> Rc<NodeEntry> {
        let id = S_NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let node = AutomationNode {
            id,
            // The reference: `{ 3, GetHashCode() }`; the first number is
            // the one that tells the core to append the rest to the
            // identifier of the window (`UiaAppendRuntimeId`).
            runtime_id: [3, id],
            kind,
            dispatcher: Dispatcher::ui_thread(),
            this: OnceLock::new(),
        };

        macro_rules! patterns {
            () => {
                [
                    InterfaceEntry::of::<IRawElementProviderFragment>(),
                    InterfaceEntry::of::<IInvokeProvider>(),
                    InterfaceEntry::of::<IExpandCollapseProvider>(),
                    InterfaceEntry::of::<IRangeValueProvider>(),
                    InterfaceEntry::of::<IScrollProvider>(),
                    InterfaceEntry::of::<IScrollItemProvider>(),
                    InterfaceEntry::of::<ISelectionProvider>(),
                    InterfaceEntry::of::<ISelectionItemProvider>(),
                    InterfaceEntry::of::<IToggleProvider>(),
                    InterfaceEntry::of::<IValueProvider>(),
                ]
            };
        }
        // The first interface is the identity of the object, and answers
        // for the simple provider it derives from.
        let provider: ComPtr<IRawElementProviderSimple2> = match kind {
            NodeKind::Node => make_com_with(node, patterns!()),
            NodeKind::Root | NodeKind::Interop { .. } => {
                let [a, b, c, d, e, f, g, h, i, j] = patterns!();
                make_com_with(
                    node,
                    [a, b, c, d, e, f, g, h, i, j, InterfaceEntry::of::<IRawElementProviderFragmentRoot>()],
                )
            }
        };
        let provider = provider.cast::<IRawElementProviderSimple>().expect("a node is a simple provider");
        // SAFETY: the object that was just made, whose value is the node.
        let value = unsafe { ferroui_microcom::ComObject::<AutomationNode>::value(provider.as_ptr().cast()) };
        let _ = value.this.set(provider.as_ptr() as usize);

        let entry = Rc::new(NodeEntry { peer: peer.downgrade(), kind, provider, subscriptions: RefCell::new(Vec::new()) });
        S_NODES.with(|nodes| {
            let mut nodes = nodes.borrow_mut();
            nodes.sweep();
            nodes.by_peer.insert(peer_key(peer), id);
            nodes.by_id.insert(id, entry.clone());
        });

        let mut subscriptions = vec![
            peer.children_changed(move || Self::on_peer_children_changed(id)),
            peer.property_changed(move |e| Self::on_peer_property_changed(id, e)),
        ];
        if let Some(embedded_root) = peer.get_provider::<dyn aap::IEmbeddedRootProvider>() {
            subscriptions.push(embedded_root.focus_changed(Rc::new(move || Self::on_embedded_root_focus_changed(id))));
        }
        if kind == NodeKind::Root {
            subscriptions.extend(RootAutomationNode::new(peer, id));
        }
        *entry.subscriptions.borrow_mut() = subscriptions;
        entry
    }

    fn entry(id: i32) -> Option<Rc<NodeEntry>> {
        S_NODES.with(|nodes| nodes.borrow().by_id.get(&id).cloned())
    }

    /// The peer of the node with the number `id`, on the UI thread; `None`
    /// when the peer is gone.
    pub(crate) fn peer_of(id: i32) -> Option<Ref<AutomationPeer>> {
        // A peer whose control is gone is not an element any more: every call of the node
        // answers `UIA_E_ELEMENTNOTAVAILABLE` (`invoke_sync`), as for a peer that is gone,
        // instead of reaching the control (DEVIATIONS.md, Automation peers).
        Self::entry(id)?.peer.upgrade().filter(|peer| peer.is_element_available())
    }

    /// The provider of the node with the number `id`, on the UI thread.
    pub(crate) fn provider_of(id: i32) -> Option<ComPtr<IRawElementProviderSimple>> {
        Self::entry(id).map(|entry| entry.provider.clone())
    }

    /// The number of nodes the table of the calling thread holds.
    #[allow(dead_code)] // For tests.
    pub(crate) fn count() -> usize {
        S_NODES.with(|nodes| nodes.borrow().by_id.len())
    }

    /// `Peer`: the peer of the node. On the UI thread only: `None` on any
    /// other thread, and when the peer is gone.
    #[allow(dead_code)] // The reference reads it from outside the node in its tests only, as here.
    pub fn peer(&self) -> Option<Ref<AutomationPeer>> {
        if !self.dispatcher.check_access() {
            return None;
        }
        Self::peer_of(self.id)
    }

    #[allow(dead_code)]
    pub(crate) fn id(&self) -> i32 {
        self.id
    }

    #[allow(dead_code)] // For tests.
    pub(crate) fn kind(&self) -> NodeKind {
        self.kind
    }

    /// The object itself as its first interface.
    pub(crate) fn this(&self) -> ComPtr<IRawElementProviderSimple> {
        let this = *self.this.get().expect("the node knows itself from its creation") as *mut IRawElementProviderSimple;
        // SAFETY: the pointer of the object this value belongs to, which
        // is alive while a method of it runs.
        unsafe { ComPtr::from_raw_add_ref(this) }.expect("the pointer of the object is not null")
    }

    pub fn get_bounding_rectangle(&self) -> Result<Rect, HResult> {
        if let NodeKind::Interop { .. } = self.kind {
            return Ok(InteropAutomationNode::get_bounding_rectangle());
        }
        self.invoke_sync(|peer| peer.to_screen(peer.get_bounding_rectangle()).unwrap_or_default())
    }

    pub fn get_fragment_root(&self) -> Result<Option<ComPtr<IRawElementProviderFragmentRoot>>, HResult> {
        match self.kind {
            NodeKind::Root => Ok(RootAutomationNode::get_fragment_root(self)),
            NodeKind::Interop { .. } => Ok(InteropAutomationNode::get_fragment_root()),
            NodeKind::Node => {
                let root = self.invoke_sync(|peer| Self::get_root(peer).map(SendProvider))?;
                Ok(root.and_then(|root| root.into_inner().cast::<IRawElementProviderFragmentRoot>().ok()))
            }
        }
    }

    pub fn get_host_raw_element_provider(&self) -> Result<Option<ComPtr<IRawElementProviderSimple>>, HResult> {
        match self.kind {
            NodeKind::Node => Ok(None),
            NodeKind::Root => RootAutomationNode::get_host_raw_element_provider(self),
            NodeKind::Interop { handle } => InteropAutomationNode::get_host_raw_element_provider(handle),
        }
    }

    pub fn get_provider_options(&self) -> ProviderOptions {
        match self.kind {
            NodeKind::Interop { .. } => InteropAutomationNode::get_provider_options(),
            _ => ProviderOptions::ServerSideProvider,
        }
    }

    /// The node itself where the peer provides the pattern; the reference
    /// answers its object, of which the client asks the interface.
    pub fn get_pattern_provider(&self, pattern_id: i32) -> Result<Option<ComPtr<IUnknown>>, HResult> {
        if let NodeKind::Interop { .. } = self.kind {
            return Ok(InteropAutomationNode::get_pattern_provider(pattern_id));
        }

        fn has<P: ?Sized + 'static>(node: &AutomationNode) -> Result<bool, HResult> {
            node.invoke_sync(|peer| peer.get_provider::<P>().is_some())
        }
        let this_if_peer_implements_provider = match UiaPatternId(pattern_id) {
            UiaPatternId::ExpandCollapse => has::<dyn aap::IExpandCollapseProvider>(self)?,
            UiaPatternId::Invoke => has::<dyn aap::IInvokeProvider>(self)?,
            UiaPatternId::RangeValue => has::<dyn aap::IRangeValueProvider>(self)?,
            UiaPatternId::Scroll => has::<dyn aap::IScrollProvider>(self)?,
            UiaPatternId::ScrollItem => true,
            UiaPatternId::Selection => has::<dyn aap::ISelectionProvider>(self)?,
            UiaPatternId::SelectionItem => has::<dyn aap::ISelectionItemProvider>(self)?,
            UiaPatternId::Toggle => has::<dyn aap::IToggleProvider>(self)?,
            UiaPatternId::Value => has::<dyn aap::IValueProvider>(self)?,
            _ => false,
        };
        Ok(if this_if_peer_implements_provider { self.this().cast::<IUnknown>().ok() } else { None })
    }

    pub fn get_property_value(&self, property_id: i32) -> Result<Option<VariantValue>, HResult> {
        if let NodeKind::Interop { .. } = self.kind {
            return Ok(InteropAutomationNode::get_property_value(property_id));
        }

        /// A value of a peer on its way from the UI thread: a variant
        /// value may hold a COM object, which does not travel.
        enum Plain {
            None,
            Text(Option<String>),
            I4(i32),
            Bool(bool),
        }
        let property_id = UiaPropertyId(property_id);
        match property_id {
            UiaPropertyId::ClickablePoint => {
                let center = self.get_bounding_rectangle()?.center();
                return Ok(Some(VariantValue::Array(VariantArray::R8(vec![center.x, center.y]))));
            }
            UiaPropertyId::Culture => return Ok(Some(VariantValue::I4(CultureInfo::current_culture().lcid()))),
            // The reference answers the name of its framework.
            UiaPropertyId::FrameworkId => return Ok(Some(VariantValue::Bstr("FerroUI".to_owned()))),
            UiaPropertyId::ProcessId => return Ok(Some(VariantValue::I4(Self::get_process_id()))),
            UiaPropertyId::RuntimeId => return Ok(Some(VariantValue::Array(VariantArray::I4(self.runtime_id.to_vec())))),
            _ => {}
        }
        // An enumeration is its number (`Convert.ToInt32`).
        let value = self.invoke_sync(move |peer| match property_id {
            UiaPropertyId::AcceleratorKey => Plain::Text(peer.get_accelerator_key()),
            UiaPropertyId::AccessKey => Plain::Text(peer.get_access_key()),
            UiaPropertyId::AutomationId => Plain::Text(peer.get_automation_id()),
            UiaPropertyId::ClassName => Plain::Text(Some(peer.get_class_name())),
            UiaPropertyId::ControlType => Plain::I4(Self::to_uia_control_type(peer.get_automation_control_type()).0),
            UiaPropertyId::HasKeyboardFocus => Plain::Bool(peer.has_keyboard_focus()),
            UiaPropertyId::IsContentElement => Plain::Bool(peer.is_content_element()),
            UiaPropertyId::IsControlElement => Plain::Bool(peer.is_control_element()),
            UiaPropertyId::IsEnabled => Plain::Bool(peer.is_enabled()),
            UiaPropertyId::IsKeyboardFocusable => Plain::Bool(peer.is_keyboard_focusable()),
            UiaPropertyId::IsOffscreen => Plain::Bool(peer.is_offscreen()),
            UiaPropertyId::ItemType => Plain::Text(peer.get_item_type()),
            UiaPropertyId::ItemStatus => Plain::Text(peer.get_item_status()),
            UiaPropertyId::LocalizedControlType => Plain::Text(Some(peer.get_localized_control_type())),
            UiaPropertyId::Name => Plain::Text(Some(peer.get_name())),
            UiaPropertyId::HelpText => Plain::Text(Some(peer.get_help_text())),
            UiaPropertyId::LandmarkType => match Self::to_uia_landmark_type(peer.get_landmark_type()) {
                Some(landmark) => Plain::I4(landmark.0),
                None => Plain::None,
            },
            UiaPropertyId::LocalizedLandmarkType => {
                Plain::Text(Self::to_uia_localized_landmark_type(peer.get_landmark_type()).map(str::to_owned))
            }
            UiaPropertyId::HeadingLevel => Plain::I4(Self::to_uia_heading_level(peer.get_heading_level()).0),
            UiaPropertyId::LiveSetting => Plain::I4(Self::to_uia_live_setting(peer.get_live_setting()).0),
            _ => Plain::None,
        })?;
        Ok(match value {
            Plain::None | Plain::Text(None) => None,
            Plain::Text(Some(text)) => Some(VariantValue::Bstr(text)),
            Plain::I4(value) => Some(VariantValue::I4(value)),
            Plain::Bool(value) => Some(VariantValue::Bool(value)),
        })
    }

    pub fn get_runtime_id(&self) -> [i32; 2] {
        self.runtime_id
    }

    pub fn navigate(&self, direction: NavigateDirection) -> Result<Option<ComPtr<IRawElementProviderFragment>>, HResult> {
        if let NodeKind::Interop { .. } = self.kind {
            if !InteropAutomationNode::navigates(direction) {
                return Ok(None);
            }
        }

        fn get_sibling(peer: &Ref<AutomationPeer>, direction: isize) -> Option<ComPtr<IRawElementProviderSimple>> {
            let children = peer.get_parent()?.get_children();
            let i = children.iter().position(|child| child.ptr_eq(peer))?;
            let j = i.checked_add_signed(direction)?;
            AutomationNode::get_or_create(children.get(j))
        }

        let result = self.invoke_sync(move |peer| {
            match direction {
                NavigateDirection::Parent => Self::get_or_create(peer.get_parent().as_ref()),
                NavigateDirection::NextSibling => get_sibling(peer, 1),
                NavigateDirection::PreviousSibling => get_sibling(peer, -1),
                NavigateDirection::FirstChild => Self::get_or_create(peer.get_children().first()),
                NavigateDirection::LastChild => Self::get_or_create(peer.get_children().last()),
                _ => None,
            }
            .map(SendProvider)
        })?;
        Ok(result.and_then(|node| node.into_inner().cast::<IRawElementProviderFragment>().ok()))
    }

    pub fn set_focus(&self) -> Result<(), HResult> {
        self.invoke_sync(|peer| peer.set_focus())
    }

    /// `GetOrCreate`: the node of a peer, on the UI thread: the one the
    /// table has for it, or a new one.
    pub fn get_or_create(peer: Option<&Ref<AutomationPeer>>) -> Option<ComPtr<IRawElementProviderSimple>> {
        let peer = peer?;
        let existing = S_NODES.with(|nodes| {
            let nodes = nodes.borrow();
            let entry = nodes.by_id.get(nodes.by_peer.get(&peer_key(peer))?)?;
            // The address of a peer that is gone may be the one of a new
            // peer: the entry has to be the one of this peer.
            entry.peer.upgrade().filter(|known| known.ptr_eq(peer)).map(|_| entry.provider.clone())
        });
        Some(existing.unwrap_or_else(|| Self::create(peer).provider.clone()))
    }

    /// `Release`: forgets the node of a peer.
    #[allow(dead_code)] // The reference declares it and does not call it either.
    pub fn release(peer: &Ref<AutomationPeer>) {
        let entry = S_NODES.with(|nodes| {
            let mut nodes = nodes.borrow_mut();
            let id = nodes.by_peer.remove(&peer_key(peer))?;
            nodes.by_id.remove(&id)
        });
        if let Some(entry) = entry {
            for subscription in entry.subscriptions.take() {
                subscription.dispose();
            }
        }
    }

    pub fn show_context_menu(&self) -> Result<(), HResult> {
        self.invoke_sync(|peer| {
            peer.show_context_menu();
        })
    }

    /// `InvokeSync`: runs `func` on the UI thread with the peer of the
    /// node and waits for its value.
    ///
    /// A panic of `func` is the failure of the call (the exception of the
    /// reference, which its runtime turns into a result code): a call of a
    /// client must not end the thread that serves it.
    pub(crate) fn invoke_sync<R: Send + 'static>(
        &self,
        func: impl FnOnce(&Ref<AutomationPeer>) -> R + Send + 'static,
    ) -> Result<R, HResult> {
        let id = self.id;
        let run = move || -> Result<R, HResult> {
            let peer = Self::peer_of(id).ok_or(ELEMENT_NOT_AVAILABLE)?;
            catch_unwind(AssertUnwindSafe(|| func(&peer))).map_err(|_| HResult(E_FAIL))
        };
        if self.dispatcher.check_access() {
            run()
        } else {
            self.dispatcher.invoke_async(run).result().map_err(|_| HResult(E_ABORT))?
        }
    }

    /// `InvokeSync<TInterface>`: runs `func` on the UI thread with the
    /// provider `P` of the peer; the peer not having it is the failure of
    /// the call.
    pub(crate) fn invoke_sync_provider<P: ?Sized + 'static, R: Send + 'static>(
        &self,
        func: impl FnOnce(&Rc<P>) -> Result<R, HResult> + Send + 'static,
    ) -> Result<R, HResult> {
        self.invoke_sync(move |peer| match peer.get_provider::<P>() {
            Some(provider) => func(&provider),
            None => Err(NOT_SUPPORTED),
        })?
    }

    /// The failure of an element that is not enabled as the result code of
    /// the system (`UIA_E_ELEMENTNOTENABLED`).
    pub(crate) fn enabled<T>(result: Result<T, ElementNotEnabledException>) -> Result<T, HResult> {
        result.map_err(|_| ELEMENT_NOT_ENABLED)
    }

    fn raise(id: i32, raise: impl FnOnce(Option<&IRawElementProviderSimple>)) {
        if let Some(provider) = Self::provider_of(id) {
            raise(Some(&provider));
        }
    }

    pub(crate) fn raise_children_changed(id: i32) {
        Self::raise(id, |provider| {
            UiaCoreProviderApi::uia_raise_structure_changed_event(provider, StructureChangeType::ChildrenInvalidated, None);
        });
    }

    pub(crate) fn raise_focus_changed(focused: Option<&ComPtr<IRawElementProviderSimple>>) {
        UiaCoreProviderApi::uia_raise_automation_event(focused.map(|focused| &**focused), UiaEventId::AutomationFocusChanged.0);
    }

    pub(crate) fn raise_live_region_changed(id: i32) {
        Self::raise(id, |provider| {
            UiaCoreProviderApi::uia_raise_automation_event(provider, UiaEventId::LiveRegionChanged.0);
        });
    }

    /// `GetRoot`: the node of the root of the tree of `peer`, when it is
    /// the node of a window.
    fn get_root(peer: &Ref<AutomationPeer>) -> Option<ComPtr<IRawElementProviderSimple>> {
        Dispatcher::ui_thread().verify_access();
        let root = peer.get_automation_root()?;
        let node = Self::get_or_create(Some(&root))?;
        let is_root = S_NODES.with(|nodes| {
            let nodes = nodes.borrow();
            nodes.by_peer.get(&peer_key(&root)).and_then(|id| nodes.by_id.get(id)).is_some_and(|entry| entry.kind == NodeKind::Root)
        });
        is_root.then_some(node)
    }

    fn on_peer_children_changed(id: i32) {
        Self::raise_children_changed(id);
    }

    fn on_peer_property_changed(id: i32, e: &AutomationPropertyChangedEventArgs) {
        // The states of expanding have other numbers in the system; any
        // other value is passed when it is one the variant carries
        // (`IConvertible` in the reference).
        fn value(value: Option<&ferroui_base::BoxedValue>) -> Option<VariantValue> {
            let value = value?;
            match value.downcast_ref::<ExpandCollapseState>() {
                Some(state) => Some(VariantValue::I4(AutomationNode::to_uia_expand_collapse_state(*state))),
                None => VariantValue::from_any(&**value),
            }
        }

        let property_id = Self::property_map(e.property());
        if let Some(property_id) = property_id {
            let (old_value, new_value) = (value(e.old_value()), value(e.new_value()));
            Self::raise(id, |provider| {
                UiaCoreProviderApi::uia_raise_automation_property_changed_event(
                    provider,
                    property_id.0,
                    old_value.as_ref(),
                    new_value.as_ref(),
                );
            });
        }

        if property_id == Some(UiaPropertyId::Name)
            && Self::peer_of(id).is_some_and(|peer| peer.get_live_setting() != AutomationLiveSetting::Off)
        {
            Self::raise_live_region_changed(id);
        }
    }

    fn on_embedded_root_focus_changed(id: i32) {
        let Some(peer) = Self::peer_of(id) else { return };
        if let Some(embedded_root) = peer.get_provider::<dyn aap::IEmbeddedRootProvider>() {
            Self::raise_focus_changed(Self::get_or_create(embedded_root.get_focus().as_ref()).as_ref());
        }
    }

    /// `Create`: the node of the kind of the peer.
    fn create(peer: &Ref<AutomationPeer>) -> Rc<NodeEntry> {
        if let Some(interop) = peer.cast::<InteropAutomationPeer>() {
            Self::new(peer, NodeKind::Interop { handle: InteropAutomationNode::new(&interop) })
        } else if peer.get_provider::<dyn aap::IRootProvider>().is_some() {
            Self::new(peer, NodeKind::Root)
        } else {
            Self::new(peer, NodeKind::Node)
        }
    }

    pub(crate) fn to_uia_control_type(role: AutomationControlType) -> UiaControlTypeId {
        match role {
            AutomationControlType::None => UiaControlTypeId::Group,
            AutomationControlType::Button => UiaControlTypeId::Button,
            AutomationControlType::Calendar => UiaControlTypeId::Calendar,
            AutomationControlType::CheckBox => UiaControlTypeId::CheckBox,
            AutomationControlType::ComboBox => UiaControlTypeId::ComboBox,
            AutomationControlType::ComboBoxItem => UiaControlTypeId::ListItem,
            AutomationControlType::Edit => UiaControlTypeId::Edit,
            AutomationControlType::Hyperlink => UiaControlTypeId::Hyperlink,
            AutomationControlType::Image => UiaControlTypeId::Image,
            AutomationControlType::ListItem => UiaControlTypeId::ListItem,
            AutomationControlType::List => UiaControlTypeId::List,
            AutomationControlType::Menu => UiaControlTypeId::Menu,
            AutomationControlType::MenuBar => UiaControlTypeId::MenuBar,
            AutomationControlType::MenuItem => UiaControlTypeId::MenuItem,
            AutomationControlType::ProgressBar => UiaControlTypeId::ProgressBar,
            AutomationControlType::RadioButton => UiaControlTypeId::RadioButton,
            AutomationControlType::ScrollBar => UiaControlTypeId::ScrollBar,
            AutomationControlType::Slider => UiaControlTypeId::Slider,
            AutomationControlType::Spinner => UiaControlTypeId::Spinner,
            AutomationControlType::StatusBar => UiaControlTypeId::StatusBar,
            AutomationControlType::Tab => UiaControlTypeId::Tab,
            AutomationControlType::TabItem => UiaControlTypeId::TabItem,
            AutomationControlType::Text => UiaControlTypeId::Text,
            AutomationControlType::ToolBar => UiaControlTypeId::ToolBar,
            AutomationControlType::ToolTip => UiaControlTypeId::ToolTip,
            AutomationControlType::Tree => UiaControlTypeId::Tree,
            AutomationControlType::TreeItem => UiaControlTypeId::TreeItem,
            AutomationControlType::Custom => UiaControlTypeId::Custom,
            AutomationControlType::Group => UiaControlTypeId::Group,
            AutomationControlType::Thumb => UiaControlTypeId::Thumb,
            AutomationControlType::DataGrid => UiaControlTypeId::DataGrid,
            AutomationControlType::DataItem => UiaControlTypeId::DataItem,
            AutomationControlType::Document => UiaControlTypeId::Document,
            AutomationControlType::SplitButton => UiaControlTypeId::SplitButton,
            AutomationControlType::Window => UiaControlTypeId::Window,
            AutomationControlType::Pane => UiaControlTypeId::Pane,
            AutomationControlType::Header => UiaControlTypeId::Header,
            AutomationControlType::HeaderItem => UiaControlTypeId::HeaderItem,
            AutomationControlType::Table => UiaControlTypeId::Table,
            AutomationControlType::TitleBar => UiaControlTypeId::TitleBar,
            AutomationControlType::Separator => UiaControlTypeId::Separator,
            AutomationControlType::Expander => UiaControlTypeId::Group,
            AutomationControlType::ScrollViewer => UiaControlTypeId::Pane,
        }
    }

    pub(crate) fn to_uia_landmark_type(landmark_type: Option<AutomationLandmarkType>) -> Option<UiaLandmarkType> {
        Some(match landmark_type? {
            AutomationLandmarkType::Banner
            | AutomationLandmarkType::Complementary
            | AutomationLandmarkType::ContentInfo
            | AutomationLandmarkType::Region => UiaLandmarkType::Custom,
            AutomationLandmarkType::Form => UiaLandmarkType::Form,
            AutomationLandmarkType::Main => UiaLandmarkType::Main,
            AutomationLandmarkType::Navigation => UiaLandmarkType::Navigation,
            AutomationLandmarkType::Search => UiaLandmarkType::Search,
        })
    }

    pub(crate) fn to_uia_localized_landmark_type(landmark_type: Option<AutomationLandmarkType>) -> Option<&'static str> {
        Some(match landmark_type? {
            AutomationLandmarkType::Banner => "banner",
            AutomationLandmarkType::Complementary => "complementary",
            AutomationLandmarkType::ContentInfo => "content information",
            AutomationLandmarkType::Region => "region",
            _ => return None,
        })
    }

    pub(crate) fn to_uia_heading_level(level: i32) -> UiaHeadingLevel {
        match level {
            1 => UiaHeadingLevel::Level1,
            2 => UiaHeadingLevel::Level2,
            3 => UiaHeadingLevel::Level3,
            4 => UiaHeadingLevel::Level4,
            5 => UiaHeadingLevel::Level5,
            6 => UiaHeadingLevel::Level6,
            7 => UiaHeadingLevel::Level7,
            8 => UiaHeadingLevel::Level8,
            9 => UiaHeadingLevel::Level9,
            _ => UiaHeadingLevel::None,
        }
    }

    pub(crate) fn to_uia_live_setting(live_setting: AutomationLiveSetting) -> UiaLiveSetting {
        UiaLiveSetting(live_setting as i32)
    }

    fn get_process_id() -> i32 {
        std::process::id() as i32
    }
}

impl NodeTable {
    /// Removes the entries of peers that are gone, once the table has
    /// grown to twice what the last removal left.
    fn sweep(&mut self) {
        if self.by_id.len() < self.sweep_at.max(64) {
            return;
        }
        self.by_id.retain(|_, entry| entry.peer.upgrade().is_some());
        let by_id = &self.by_id;
        self.by_peer.retain(|_, id| by_id.contains_key(id));
        self.sweep_at = self.by_id.len() * 2;
    }
}

// The interfaces of the element and of the fragment: the values of the
// methods above as the system passes them.

impl IRawElementProviderSimpleImpl for AutomationNode {
    fn get_provider_options(&self) -> Result<ProviderOptions, HResult> {
        Ok(AutomationNode::get_provider_options(self))
    }

    fn get_pattern_provider(&self, pattern_id: i32) -> Result<Option<ComPtr<IUnknown>>, HResult> {
        AutomationNode::get_pattern_provider(self, pattern_id)
    }

    fn get_property_value(&self, property_id: i32) -> Result<ComVariant, HResult> {
        let value = AutomationNode::get_property_value(self, property_id)?;
        ComVariantMarshaller::convert_to_unmanaged(value.as_ref()).map_err(|_| HResult(E_INVALIDARG))
    }

    fn get_host_raw_element_provider(&self) -> Result<Option<ComPtr<IRawElementProviderSimple>>, HResult> {
        AutomationNode::get_host_raw_element_provider(self)
    }
}

impl IRawElementProviderSimple2Impl for AutomationNode {
    fn show_context_menu(&self) -> Result<(), HResult> {
        AutomationNode::show_context_menu(self)
    }
}

impl IRawElementProviderFragmentImpl for AutomationNode {
    fn navigate(&self, direction: NavigateDirection) -> Result<Option<ComPtr<IRawElementProviderFragment>>, HResult> {
        AutomationNode::navigate(self, direction)
    }

    fn get_runtime_id(&self) -> Result<SafeArrayRef, HResult> {
        SafeArrayMarshaller::<i32>::convert_to_unmanaged(Some(&AutomationNode::get_runtime_id(self))).map_err(|_| HResult(E_FAIL))
    }

    fn get_bounding_rectangle(&self) -> Result<UiaRect, HResult> {
        let rect = AutomationNode::get_bounding_rectangle(self)?;
        Ok(UiaRect { left: rect.x, top: rect.y, width: rect.width, height: rect.height })
    }

    fn get_embedded_fragment_roots(&self) -> Result<SafeArrayRef, HResult> {
        // No node has embedded roots: the reference answers no array.
        let roots = match self.kind {
            NodeKind::Interop { .. } => InteropAutomationNode::get_embedded_fragment_roots(),
            _ => None,
        };
        SafeArrayMarshaller::convert_to_unmanaged(roots.as_deref()).map_err(|_| HResult(E_FAIL))
    }

    fn set_focus(&self) -> Result<(), HResult> {
        AutomationNode::set_focus(self)
    }

    fn get_fragment_root(&self) -> Result<Option<ComPtr<IRawElementProviderFragmentRoot>>, HResult> {
        AutomationNode::get_fragment_root(self)
    }
}

impl IRawElementProviderFragmentRootImpl for AutomationNode {
    fn element_provider_from_point(&self, x: f64, y: f64) -> Result<Option<ComPtr<IRawElementProviderFragment>>, HResult> {
        match self.kind {
            NodeKind::Root => RootAutomationNode::element_provider_from_point(self, x, y),
            _ => Ok(InteropAutomationNode::element_provider_from_point(x, y)),
        }
    }

    fn get_focus(&self) -> Result<Option<ComPtr<IRawElementProviderFragment>>, HResult> {
        match self.kind {
            NodeKind::Root => RootAutomationNode::get_focus(self),
            _ => Ok(InteropAutomationNode::get_focus()),
        }
    }
}

impl IInvokeProviderImpl for AutomationNode {
    fn invoke(&self) -> Result<(), HResult> {
        self.invoke_sync_provider(|x: &Rc<dyn aap::IInvokeProvider>| Self::enabled(x.invoke()))
    }
}

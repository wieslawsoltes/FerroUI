//! The selection pattern and the selection item pattern of a node (the
//! port of `AutomationNode.Selection.cs`).

use super::automation_node::{AutomationNode, SendProvider};
use super::interop::{IRawElementProviderSimple, ISelectionItemProviderImpl, ISelectionProviderImpl};
use super::marshalling::{SafeArrayMarshaller, SafeArrayRef};
use ferroui_controls::automation::provider::{ISelectionItemProvider, ISelectionProvider};
use ferroui_microcom::{ComPtr, HResult, E_FAIL};
use std::rc::Rc;

impl ISelectionProviderImpl for AutomationNode {
    fn get_selection(&self) -> Result<SafeArrayRef, HResult> {
        let nodes = self.invoke_sync_provider(|x: &Rc<dyn ISelectionProvider>| {
            let peers = x.get_selection();
            Ok(peers.iter().filter_map(|x| AutomationNode::get_or_create(Some(x))).map(SendProvider).collect::<Vec<_>>())
        })?;
        let nodes: Vec<ComPtr<IRawElementProviderSimple>> = nodes.into_iter().map(SendProvider::into_inner).collect();
        SafeArrayMarshaller::convert_to_unmanaged(Some(&nodes)).map_err(|_| HResult(E_FAIL))
    }

    fn can_select_multiple(&self) -> Result<i32, HResult> {
        self.invoke_sync_provider(|x: &Rc<dyn ISelectionProvider>| Ok(i32::from(x.can_select_multiple())))
    }

    fn is_selection_required(&self) -> Result<i32, HResult> {
        self.invoke_sync_provider(|x: &Rc<dyn ISelectionProvider>| Ok(i32::from(x.is_selection_required())))
    }
}

impl ISelectionItemProviderImpl for AutomationNode {
    fn select(&self) -> Result<(), HResult> {
        self.invoke_sync_provider(|x: &Rc<dyn ISelectionItemProvider>| Self::enabled(x.select()))
    }

    fn add_to_selection(&self) -> Result<(), HResult> {
        self.invoke_sync_provider(|x: &Rc<dyn ISelectionItemProvider>| Self::enabled(x.add_to_selection()))
    }

    fn remove_from_selection(&self) -> Result<(), HResult> {
        self.invoke_sync_provider(|x: &Rc<dyn ISelectionItemProvider>| Self::enabled(x.remove_from_selection()))
    }

    fn get_is_selected(&self) -> Result<i32, HResult> {
        self.invoke_sync_provider(|x: &Rc<dyn ISelectionItemProvider>| Ok(i32::from(x.is_selected())))
    }

    fn get_selection_container(&self) -> Result<Option<ComPtr<IRawElementProviderSimple>>, HResult> {
        let node = self.invoke_sync_provider(|x: &Rc<dyn ISelectionItemProvider>| {
            // The container is a provider of a peer: its node is the node
            // of that peer.
            let peer = x.selection_container_peer();
            Ok(AutomationNode::get_or_create(peer.as_ref()).map(SendProvider))
        })?;
        Ok(node.map(SendProvider::into_inner))
    }
}

use crate::automation::peers::AutomationPeer;
use ferroui_base::Ref;
use super::ISelectionProvider;
use crate::automation::AutomationError;
use std::rc::Rc;

/// Exposes methods and properties to support access by a UI Automation client to individual,
/// selectable child controls of containers that implement `ISelectionProvider`.
pub trait ISelectionItemProvider {
    /// The automation peer that provides this contract.
    fn peer(&self) -> Ref<AutomationPeer>;

    /// Gets a value that indicates whether an item is selected.
    fn is_selected(&self) -> bool;

    /// Gets the UI Automation provider that implements `ISelectionProvider` and acts as the
    /// container for the calling object.
    fn selection_container(&self) -> Option<Rc<dyn ISelectionProvider>>;

    /// Gets the raw selection-container peer. A container need not expose the
    /// Selection pattern. Platform raw-provider bridges use this accessor;
    /// existing implementations retain their typed-container behavior by default.
    fn selection_container_peer(&self) -> Option<Ref<AutomationPeer>> {
        self.selection_container().map(|container| container.peer())
    }

    /// Adds the current element to the collection of selected items.
    ///
    /// # Errors
    ///
    /// An [`AutomationError`] when the state of the element refuses the call
    /// (the exception the member throws in the original).
    fn add_to_selection(&self) -> Result<(), AutomationError>;

    /// Removes the current element from the collection of selected items.
    ///
    /// # Errors
    ///
    /// An [`AutomationError`] when the state of the element refuses the call
    /// (the exception the member throws in the original).
    fn remove_from_selection(&self) -> Result<(), AutomationError>;

    /// Clears any existing selection and then selects the current element.
    ///
    /// # Errors
    ///
    /// An [`AutomationError`] when the state of the element refuses the call
    /// (the exception the member throws in the original).
    fn select(&self) -> Result<(), AutomationError>;
}

impl PartialEq for dyn ISelectionItemProvider {
    /// Providers are equal if they are provided by the same automation peer.
    fn eq(&self, other: &Self) -> bool {
        self.peer() == other.peer()
    }
}

#[cfg(test)]
mod raw_container_regression {
// Framework-only contract regression: a raw selection container need not support Selection.
use ferroui_base::Ref;
use crate::{Border, automation::{AutomationError, peers::{AutomationPeer, ControlAutomationPeer}, provider::{ISelectionItemProvider, ISelectionProvider}}};
use std::rc::Rc;
struct Item { peer: Ref<AutomationPeer>, container: Ref<AutomationPeer> }
impl ISelectionItemProvider for Item {
 fn peer(&self)->Ref<AutomationPeer>{self.peer.clone()}
 fn is_selected(&self)->bool{true}
 fn selection_container(&self)->Option<Rc<dyn ISelectionProvider>>{self.container.get_provider::<dyn ISelectionProvider>()}
 fn selection_container_peer(&self)->Option<Ref<AutomationPeer>>{Some(self.container.clone())}
 fn add_to_selection(&self)->Result<(),AutomationError>{Ok(())}
 fn remove_from_selection(&self)->Result<(),AutomationError>{Ok(())}
 fn select(&self)->Result<(),AutomationError>{Ok(())}
}
#[test]
fn raw_container_does_not_require_selection_pattern() {
 let _scope=crate::test_support::test_scope();
 crate::register_types();
 let control=Border::new(); let container=ControlAutomationPeer::create_peer_for_element(&control);
 let item_control=Border::new(); let peer=ControlAutomationPeer::create_peer_for_element(&item_control);
 let provider: Rc<dyn ISelectionItemProvider> = Rc::new(Item{peer,container:container.clone()});
 assert!(container.get_provider::<dyn ISelectionProvider>().is_none());
 assert!(provider.selection_container().is_none());
 assert_eq!(provider.selection_container_peer(),Some(container));
}

struct Legacy {peer:Ref<AutomationPeer>,container:Rc<dyn ISelectionProvider>}
impl ISelectionItemProvider for Legacy {
 fn peer(&self)->Ref<AutomationPeer>{self.peer.clone()}
 fn is_selected(&self)->bool{true}
 fn selection_container(&self)->Option<Rc<dyn ISelectionProvider>>{Some(self.container.clone())}
 fn add_to_selection(&self)->Result<(),AutomationError>{Ok(())}
 fn remove_from_selection(&self)->Result<(),AutomationError>{Ok(())}
 fn select(&self)->Result<(),AutomationError>{Ok(())}
}
#[test]
fn existing_selection_pattern_implementations_keep_their_container(){
 let _scope=crate::test_support::test_scope();
 crate::register_types();
 let root=crate::ListBox::new();
 let peer=ControlAutomationPeer::create_peer_for_element(&root);
 let pattern=peer.get_provider::<dyn ISelectionProvider>().unwrap();
 let item=Legacy{peer:peer.clone(),container:pattern};
 assert_eq!(item.selection_container_peer(),Some(peer));
}

}

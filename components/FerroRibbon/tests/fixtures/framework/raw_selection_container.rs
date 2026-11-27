//! Framework-only contract regression: a raw selection container need not support Selection.
use ferroui_base::Ref;
use ferroui_controls::{
    Border,
    automation::{
        AutomationError,
        peers::{AutomationPeer, ControlAutomationPeer},
        provider::{ISelectionItemProvider, ISelectionProvider},
    },
};
use std::rc::Rc;
struct Item {
    peer: Ref<AutomationPeer>,
    container: Ref<AutomationPeer>,
}
impl ISelectionItemProvider for Item {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.peer.clone()
    }
    fn is_selected(&self) -> bool {
        true
    }
    fn selection_container(&self) -> Option<Rc<dyn ISelectionProvider>> {
        self.container.get_provider::<dyn ISelectionProvider>()
    }
    fn selection_container_peer(&self) -> Option<Ref<AutomationPeer>> {
        Some(self.container.clone())
    }
    fn add_to_selection(&self) -> Result<(), AutomationError> {
        Ok(())
    }
    fn remove_from_selection(&self) -> Result<(), AutomationError> {
        Ok(())
    }
    fn select(&self) -> Result<(), AutomationError> {
        Ok(())
    }
}
#[test]
fn raw_container_does_not_require_selection_pattern() {
    ferroui_controls::register_types();
    let control = Border::new();
    let container = ControlAutomationPeer::create_peer_for_element(&control);
    let item_control = Border::new();
    let peer = ControlAutomationPeer::create_peer_for_element(&item_control);
    let provider: Rc<dyn ISelectionItemProvider> = Rc::new(Item {
        peer,
        container: container.clone(),
    });
    assert!(container.get_provider::<dyn ISelectionProvider>().is_none());
    assert!(provider.selection_container().is_none());
    assert_eq!(provider.selection_container_peer(), Some(container));
}

struct Legacy {
    peer: Ref<AutomationPeer>,
    container: Rc<dyn ISelectionProvider>,
}
impl ISelectionItemProvider for Legacy {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.peer.clone()
    }
    fn is_selected(&self) -> bool {
        true
    }
    fn selection_container(&self) -> Option<Rc<dyn ISelectionProvider>> {
        Some(self.container.clone())
    }
    fn add_to_selection(&self) -> Result<(), AutomationError> {
        Ok(())
    }
    fn remove_from_selection(&self) -> Result<(), AutomationError> {
        Ok(())
    }
    fn select(&self) -> Result<(), AutomationError> {
        Ok(())
    }
}
#[test]
fn existing_selection_pattern_implementations_keep_their_container() {
    ferroui_controls::register_types();
    let root = ferroui_controls::ListBox::new();
    let peer = ControlAutomationPeer::create_peer_for_element(&root);
    let pattern = peer.get_provider::<dyn ISelectionProvider>().unwrap();
    let item = Legacy {
        peer: peer.clone(),
        container: pattern,
    };
    assert_eq!(item.selection_container_peer(), Some(peer));
}

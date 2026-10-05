//! Port of the reference `NativeMenuBarAutomationPeerTests`.

use super::{AutomationControlType, ControlAutomationPeer, NativeMenuBarAutomationPeer};
use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions};
use crate::test_support::{boxed_str, test_scope};
use crate::{Control, Menu, MenuItem, NativeMenuBar};
use std::rc::Rc;

fn create_template() -> Rc<FuncControlTemplate> {
    FuncControlTemplate::new(|_, ns| {
        let menu = Menu::new();
        menu.set_name(Some("PART_NativeMenuPresenter".to_string()));
        for header in ["File", "Edit"] {
            let item = MenuItem::new();
            item.set_header(boxed_str(header));
            menu.items().add(Some(Control::boxed(item)));
        }
        menu.register_in_name_scope(&**ns).upcast()
    })
}

mod peer_creation {
    use super::*;

    #[test]
    fn creates_native_menu_bar_automation_peer() {
        let _scope = test_scope();
        let control = NativeMenuBar::new();
        control.set_template(Some(create_template()));
        let peer = ControlAutomationPeer::create_peer_for_element(&control);

        assert!(std::ptr::eq(peer.get_type(), NativeMenuBarAutomationPeer::TYPE));
    }

    #[test]
    fn control_type_is_menu_bar() {
        let _scope = test_scope();
        let control = NativeMenuBar::new();
        control.set_template(Some(create_template()));
        let peer = ControlAutomationPeer::create_peer_for_element(&control)
            .cast::<NativeMenuBarAutomationPeer>()
            .expect("a native menu bar automation peer");

        assert_eq!(AutomationControlType::MenuBar, peer.get_automation_control_type());
    }
}

//! Public FerroUI-only reproducer of native masked text automation behavior.
#[path = "../tests/support/headless.rs"]
mod headless;
use ferroui_controls::{
    TextBox,
    automation::{
        ValuePatternIdentifiers,
        peers::{ControlAutomationPeer, TextBoxAutomationPeer},
        provider::IValueProvider,
    },
};
use std::{cell::Cell, rc::Rc};
fn main() {
    headless::run(|| {
        let field = TextBox::new();
        let automation = ControlAutomationPeer::create_peer_for_element(&field);
        let retained = automation.get_provider::<dyn IValueProvider>().unwrap();
        field.set_text(Some("public fixture"));
        assert_eq!(retained.value().as_deref(), Some("public fixture"));
        let notifications = Rc::new(Cell::new(0));
        let count = notifications.clone();
        let _subscription = automation.property_changed(move |change| {
            if std::ptr::eq(change.property(), ValuePatternIdentifiers::value_property()) {
                count.set(count.get() + 1);
            }
        });
        field.set_password_char('●');
        field.set_reveal_password(true);
        field.set_text(Some("synthetic secret"));
        assert!(automation.get_provider::<dyn IValueProvider>().is_none());
        assert_eq!(retained.value(), None);
        assert_eq!(
            automation.cast::<TextBoxAutomationPeer>().unwrap().value(),
            None
        );
        assert!(automation.get_children().is_empty());
        assert_eq!(notifications.get(), 0);
        assert_eq!(field.text().as_deref(), Some("synthetic secret"));
        drop(field);
        ferroui_base::threading::Dispatcher::current_dispatcher().run_jobs(None);
        assert_eq!(retained.value(), None);
        println!(
            "Masked/revealed automation values, retained providers, secret value notifications and closed-owner value reads protected; native text remains editable."
        );
    });
}

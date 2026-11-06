#[path = "support/headless.rs"]
mod headless;
use ferroboard_controls::input::PasswordBoxInputTarget;
use ferroboard_core::editor::ITextInputTarget;
use ferroui_base::{Ref, WeakRef, threading::Dispatcher};
use ferroui_controls::{
    Grid, TextBlock, TextBox,
    automation::{
        ValuePatternIdentifiers,
        peers::{AutomationPeer, ControlAutomationPeer, TextBoxAutomationPeer},
        provider::IValueProvider,
    },
    presenters::TextPresenter,
    templates::FuncControlTemplate,
};
use std::{cell::RefCell, rc::Rc};
fn peer(field: &TextBox) -> Ref<AutomationPeer> {
    ControlAutomationPeer::create_peer_for_element(field)
}
#[test]
fn masked_and_revealed_passwords_have_no_automation_value_provider() {
    headless::run(|| {
        let field = TextBox::new();
        field.set_password_char('●');
        field.set_text(Some("synthetic password"));
        let automation = peer(&field);
        assert!(automation.get_provider::<dyn IValueProvider>().is_none());
        assert_eq!(
            automation.cast::<TextBoxAutomationPeer>().unwrap().value(),
            None
        );
        field.set_reveal_password(true);
        assert!(automation.get_provider::<dyn IValueProvider>().is_none());
        assert_eq!(
            automation.cast::<TextBoxAutomationPeer>().unwrap().value(),
            None
        );
        assert_eq!(field.text().as_deref(), Some("synthetic password"));
    });
}
#[test]
fn retained_plain_provider_redacts_masked_reads_and_suppresses_value_payloads() {
    headless::run(|| {
        let field = TextBox::new();
        let automation = peer(&field);
        let retained = automation.get_provider::<dyn IValueProvider>().unwrap();
        let observed = Rc::new(RefCell::new(Vec::new()));
        let log = observed.clone();
        let _subscription = automation.property_changed(move |change| {
            if std::ptr::eq(change.property(), ValuePatternIdentifiers::value_property()) {
                let extract = |value: Option<&ferroui_base::BoxedValue>| {
                    value.and_then(|v| v.downcast_ref::<String>().cloned())
                };
                log.borrow_mut()
                    .push((extract(change.old_value()), extract(change.new_value())));
            }
        });
        field.set_text(Some("public fixture"));
        assert_eq!(retained.value().as_deref(), Some("public fixture"));
        assert_eq!(
            &*observed.borrow(),
            &[(None, Some("public fixture".to_owned()))]
        );
        observed.borrow_mut().clear();
        field.set_password_char('●');
        field.set_text(Some("synthetic secret"));
        assert_eq!(retained.value(), None);
        field.set_reveal_password(true);
        field.set_text(Some("second synthetic secret"));
        assert_eq!(retained.value(), None);
        assert!(observed.borrow().is_empty());
        // Existing automation write behavior remains; only secret reads/notifications are hidden.
        retained.set_value(Some("replacement fixture")).unwrap();
        assert_eq!(field.text().as_deref(), Some("replacement fixture"));
        assert!(observed.borrow().is_empty());
        field.set_password_char('\0');
        assert!(automation.get_provider::<dyn IValueProvider>().is_some());
        assert_eq!(retained.value().as_deref(), Some("replacement fixture"));
    });
}
#[test]
fn cached_template_children_are_hidden_when_password_mode_changes() {
    headless::run(|| {
        let field = TextBox::new();
        field.set_template(Some(FuncControlTemplate::new(|_, scope| {
            let root = Grid::new();
            let presenter = TextPresenter::new();
            presenter.set_name(Some("PART_TextPresenter".into()));
            scope.register("PART_TextPresenter", presenter.clone().upcast());
            root.children().add(presenter);
            let label = TextBlock::new();
            label.set_text(Some("template fixture"));
            root.children().add(label);
            scope.complete();
            root.upcast()
        })));
        field.apply_template();
        let automation = peer(&field);
        assert!(!automation.get_children().is_empty());
        field.set_password_char('●');
        assert!(automation.get_children().is_empty());
        field.set_reveal_password(true);
        assert!(automation.get_children().is_empty());
        field.set_password_char('\0');
        assert!(!automation.get_children().is_empty());
    });
}
#[test]
fn earlier_mask_change_observers_cannot_read_a_stale_public_provider() {
    headless::run(|| {
        let field = TextBox::new();
        field.set_text(Some("fixture"));
        let slot = Rc::new(RefCell::new(None::<WeakRef<AutomationPeer>>));
        let lookup = slot.clone();
        let observed = Rc::new(RefCell::new(Vec::new()));
        let log = observed.clone();
        let _subscription = field.property_changed(move |change| {
            if change.property() == TextBox::password_char_property().as_property() {
                let automation = lookup.borrow().as_ref().unwrap().upgrade().unwrap();
                log.borrow_mut().push((
                    automation.get_provider::<dyn IValueProvider>().is_some(),
                    automation.cast::<TextBoxAutomationPeer>().unwrap().value(),
                ));
            }
        });
        let automation = peer(&field);
        *slot.borrow_mut() = Some(automation.downgrade());
        field.set_password_char('●');
        assert_eq!(&*observed.borrow(), &[(false, None)]);
    });
}
#[test]
fn password_input_target_edits_remain_available_without_automation_disclosure() {
    headless::run(|| {
        let field = TextBox::new();
        field.set_password_char('●');
        let automation = peer(&field);
        let observed = Rc::new(RefCell::new(0));
        let log = observed.clone();
        let _subscription = automation.property_changed(move |change| {
            if std::ptr::eq(change.property(), ValuePatternIdentifiers::value_property()) {
                *log.borrow_mut() += 1;
            }
        });
        let target = PasswordBoxInputTarget::new(field.clone(), None).unwrap();
        target.replace(0, 0, Some(&"a😀".into())).unwrap();
        assert_eq!(target.text(), "a😀".into());
        assert_eq!(target.selection_start(), 3);
        assert!(automation.get_provider::<dyn IValueProvider>().is_none());
        assert_eq!(
            automation.cast::<TextBoxAutomationPeer>().unwrap().value(),
            None
        );
        assert_eq!(*observed.borrow(), 0);
        Dispatcher::current_dispatcher().run_jobs(None);
    });
}

#[test]
fn closed_native_controls_leave_no_readable_value_or_provider() {
    headless::run(|| {
        let field = TextBox::new();
        field.set_text(Some("public fixture"));
        let weak = field.downgrade();
        let automation = peer(&field);
        let retained = automation.get_provider::<dyn IValueProvider>().unwrap();
        drop(field);
        // TextChanged queues a framework-owned callback; finish it before testing owner loss.
        Dispatcher::current_dispatcher().run_jobs(None);
        assert!(weak.upgrade().is_none());
        assert!(automation.get_provider::<dyn IValueProvider>().is_none());
        assert_eq!(retained.value(), None);
        assert!(automation.get_children().is_empty());
    });
}

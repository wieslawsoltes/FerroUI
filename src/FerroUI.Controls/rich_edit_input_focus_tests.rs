//! Accepted keyboard focus and semantic input ownership stay synchronized
//! through real template changes and nested focus callbacks.
use super::*;
use ferroui_base::input::KeyboardDevice;

fn outside(f: &Fixture) -> Ref<Border> {
    let outside = Border::new();
    outside.set_focusable(true);
    outside.set_height(30.0);
    f.panel.children().add(outside.clone());
    f.layout();
    outside
}

fn focused_flyout(f: &Fixture) -> (Ref<Flyout>, Ref<Border>) {
    let flyout = f.flyout();
    let child = Border::new();
    child.set_focusable(true);
    child.set_width(50.0);
    child.set_height(24.0);
    flyout.set_content(Some(Control::boxed(child.clone())));
    flyout.set_show_mode(FlyoutShowMode::Standard);
    assert!(f.keyboard(Key::Apps, KeyModifiers::NONE).handled());
    assert!(child.is_focused());
    (flyout, child)
}

#[test]
fn rich_input_template_replacement_during_focus_return_preserves_session() {
    for overlay in [false, true] {
        let f = Fixture::new(overlay);
        let connection = f.native.connection.borrow().clone().unwrap();
        let session = connection.begin_composition().unwrap();
        session.update("candidate", Some(4)).unwrap();
        let snapshot = f.owner.document().snapshot();
        let selection = (f.owner.selection_start(), f.owner.selection_end());
        let revision = f
            .owner
            .text_input_method_client()
            .input_method_state_revision();
        let (flyout, _) = focused_flyout(&f);
        assert!(session.is_current());
        let owner = f.owner.downgrade();
        let calls = Rc::new(Cell::new(0));
        let observed = calls.clone();
        f.owner
            .add_handler(InputElement::got_focus_event(), move |_, args| {
                let owner = owner.upgrade().unwrap();
                if args.is_source(&owner) {
                    observed.set(observed.get() + 1);
                    owner.set_template(Some(crate::rich_edit_box_tests::rich_edit_box_template()));
                    owner.apply_template();
                }
            });
        flyout.hide();
        assert_eq!(calls.get(), 1);
        assert!(f.owner.is_focused());
        assert_eq!(f.owner.document().snapshot(), snapshot);
        assert!(f.owner.editor().is_composing());
        assert!(connection.is_current(), "after remount; overlay={overlay}");
        assert!(session.is_current(), "after remount; overlay={overlay}");
        assert!(Rc::ptr_eq(
            &connection,
            f.native.connection.borrow().as_ref().unwrap()
        ));
        f.layout();
        assert!(session.is_current(), "after layout; overlay={overlay}");
        assert_eq!(f.owner.document().snapshot(), snapshot);
        assert_eq!(
            (f.owner.selection_start(), f.owner.selection_end()),
            selection
        );
        assert_eq!(
            f.owner
                .text_input_method_client()
                .input_method_state_revision(),
            revision
        );
        assert!(!f.owner.can_undo());
        assert!(session.cancel().unwrap());
    }
}

#[test]
fn rich_input_nested_focus_during_popup_dismissal_owns_the_final_binding() {
    for overlay in [false, true] {
        for from_lost_focus in [false, true] {
            for return_to_owner in [false, true] {
                let f = Fixture::new(overlay);
                let outside = outside(&f);
                let before = f.owner.document().snapshot();
                let connection = f.native.connection.borrow().clone().unwrap();
                let session = connection.begin_composition().unwrap();
                session.update("candidate", Some(4)).unwrap();
                let (flyout, child) = focused_flyout(&f);
                assert!(session.is_current());
                let armed = Rc::new(Cell::new(true));
                let changed = Rc::new(Cell::new(0));
                let got_focus = Rc::new(Cell::new(0));
                let owner = f.owner.downgrade();
                let destination = outside.downgrade();
                let observed = changed.clone();
                let original = connection.clone();
                let change_focus: Rc<dyn Fn()> = Rc::new(move || {
                    if !armed.replace(false) {
                        return;
                    }
                    observed.set(observed.get() + 1);
                    let owner = owner.upgrade().unwrap();
                    owner.set_template(Some(crate::rich_edit_box_tests::rich_edit_box_template()));
                    owner.apply_template();
                    assert!(
                        original.is_current(),
                        "remount precedes the nested focus change"
                    );
                    assert!(destination.upgrade().unwrap().focus());
                    if return_to_owner {
                        assert!(owner.focus());
                    }
                });
                let owner = f.owner.downgrade();
                let calls = got_focus.clone();
                let change = change_focus.clone();
                f.owner
                    .add_handler(InputElement::got_focus_event(), move |_, args| {
                        if args.is_source(&owner.upgrade().unwrap()) {
                            calls.set(calls.get() + 1);
                            if !from_lost_focus {
                                change();
                            }
                        }
                    });
                if from_lost_focus {
                    let focused_child = child.downgrade();
                    child.add_handler(InputElement::lost_focus_event(), move |_, args| {
                        if args.is_source(&focused_child.upgrade().unwrap()) {
                            change_focus();
                        }
                    });
                }
                flyout.hide();
                assert_eq!(changed.get(), 1);
                assert_eq!(
                    got_focus.get(),
                    usize::from(!from_lost_focus) + usize::from(return_to_owner)
                );
                let expected: Ref<InputElement> = if return_to_owner {
                    f.owner.clone().upcast()
                } else {
                    outside.clone().upcast()
                };
                assert_eq!(
                    KeyboardDevice::instance().unwrap().focused_element(),
                    Some(expected)
                );
                assert_eq!(f.owner.is_focused(), return_to_owner);
                assert_eq!(outside.is_focused(), !return_to_owner);
                assert!(!connection.is_current());
                assert!(!session.is_current());
                assert!(!f.owner.editor().is_composing());
                assert_eq!(f.owner.document().snapshot(), before);
                assert!(!f.owner.can_undo());
                let accepted = f.native.connection.borrow().clone();
                if return_to_owner {
                    let accepted = accepted.unwrap();
                    assert!(accepted.is_current());
                    assert!(!Rc::ptr_eq(&connection, &accepted));
                    assert_eq!(
                        accepted.owner(),
                        Some(f.owner.clone().upcast::<InputElement>())
                    );
                } else {
                    assert!(accepted.is_none());
                    assert!(f.native.client.borrow().is_none());
                }
            }
        }
    }
}

#[test]
fn rich_input_canceled_focus_with_template_change_preserves_current_connection() {
    for cancel_at_source in [false, true] {
        let f = Fixture::new(false);
        let outside = outside(&f);
        let connection = f.native.connection.borrow().clone().unwrap();
        let session = connection.begin_composition().unwrap();
        session.update("candidate", Some(4)).unwrap();
        let snapshot = f.owner.document().snapshot();
        let calls = Rc::new(Cell::new(0));
        let observed = calls.clone();
        let owner = f.owner.downgrade();
        let cancel =
            move |_: &ferroui_base::interactivity::Interactive,
                  args: &ferroui_base::input::FocusChangingEventArgs| {
                observed.set(observed.get() + 1);
                let owner = owner.upgrade().unwrap();
                owner.set_template(Some(crate::rich_edit_box_tests::rich_edit_box_template()));
                owner.apply_template();
                assert!(args.try_cancel());
            };
        let handler = if cancel_at_source {
            f.owner
                .add_handler(InputElement::losing_focus_event(), cancel)
        } else {
            outside.add_handler(InputElement::getting_focus_event(), cancel)
        };
        assert!(!outside.focus());
        assert_eq!(calls.get(), 1);
        assert!(f.owner.is_focused());
        assert!(!outside.is_focused());
        assert!(connection.is_current());
        assert!(session.is_current());
        assert!(Rc::ptr_eq(
            &connection,
            f.native.connection.borrow().as_ref().unwrap()
        ));
        assert_eq!(f.owner.document().snapshot(), snapshot);
        assert!(!f.owner.can_undo());
        // Window teardown clears focus without allowing cancellation. This
        // handler belongs only to the explicit cancellable request above.
        if cancel_at_source {
            f.owner
                .remove_handler(InputElement::losing_focus_event(), handler);
        } else {
            outside.remove_handler(InputElement::getting_focus_event(), handler);
        }
        assert!(session.cancel().unwrap());
    }
}

#[test]
fn rich_input_canceled_nested_focus_keeps_post_event_client_installation() {
    let f = Fixture::new(false);
    let outside = outside(&f);
    assert!(outside.focus());
    assert!(f.native.client.borrow().is_none());
    let canceled = Rc::new(Cell::new(0));
    let observed = canceled.clone();
    outside.add_handler(InputElement::getting_focus_event(), move |_, args| {
        observed.set(observed.get() + 1);
        assert!(args.try_cancel());
    });
    let native = f.native.clone();
    let destination = outside.downgrade();
    let owner = f.owner.downgrade();
    let calls = Rc::new(Cell::new(0));
    let observed = calls.clone();
    f.owner
        .add_handler(InputElement::got_focus_event(), move |_, args| {
            if args.is_source(&owner.upgrade().unwrap()) {
                observed.set(observed.get() + 1);
                assert!(native.client.borrow().is_none());
                assert!(!destination.upgrade().unwrap().focus());
                assert!(native.client.borrow().is_none());
            }
        });
    assert!(f.owner.focus());
    assert_eq!(calls.get(), 1);
    assert_eq!(canceled.get(), 1);
    assert!(f.owner.is_focused());
    assert!(!outside.is_focused());
    let connection = f.native.connection.borrow().clone().unwrap();
    assert!(connection.is_current());
    assert_eq!(
        connection.owner(),
        Some(f.owner.clone().upcast::<InputElement>())
    );
}

//! Public native callback ownership through the reusable library's markup API.
use ferroboard_core::editor::{
    EnterAction, EnterActionHandler, ITextInputTarget, TextBufferTarget,
};
use ferroboard_core_ferro::{TextBufferTargetMetadata, register_types};
use ferroui_base::metadata::{MarkupDelegate, MarkupTyped, from_markup_value, into_markup_value};
use std::{cell::Cell, rc::Rc};

fn read(target: &Rc<TextBufferTarget>) -> Option<MarkupDelegate> {
    let property = TextBufferTargetMetadata::MARKUP
        .find_property("EnterActionHandler")
        .unwrap();
    let value = (property.get.unwrap())(&[into_markup_value(target.clone())]).unwrap();
    from_markup_value(&value).unwrap()
}
fn write(target: &Rc<TextBufferTarget>, callback: Option<MarkupDelegate>) {
    let property = TextBufferTargetMetadata::MARKUP
        .find_property("EnterActionHandler")
        .unwrap();
    (property.set.unwrap())(&[
        into_markup_value(target.clone()),
        into_markup_value(callback),
    ])
    .unwrap();
}

#[test]
fn callback_snapshots_and_targets_release_captures_without_registry_cleanup() {
    register_types();
    let target = Rc::new(TextBufferTarget::default());
    let target_weak = Rc::downgrade(&target);
    let old_state = Rc::new(Cell::new(0));
    let old_weak = Rc::downgrade(&old_state);
    target.set_enter_action_handler(Some(Rc::new(move |action| {
        old_state.set(old_state.get() + 1);
        action == EnterAction::Go
    })));
    let snapshot = read(&target).unwrap();
    write(&target, None);
    assert!(read(&target).is_none());
    assert!(old_weak.upgrade().is_some());
    assert_eq!(
        from_markup_value::<bool>(
            &snapshot
                .try_invoke(&[into_markup_value(EnterAction::Go)])
                .unwrap()
        ),
        Some(true)
    );
    assert_eq!(old_weak.upgrade().unwrap().get(), 1);
    drop(snapshot);
    assert!(old_weak.upgrade().is_none());

    let state = Rc::new(Cell::new(0));
    let weak = Rc::downgrade(&state);
    write(
        &target,
        Some(MarkupDelegate::new(move |args| {
            state.set(state.get() + 1);
            into_markup_value(
                from_markup_value::<EnterAction>(&args[0]) == Some(EnterAction::Search),
            )
        })),
    );
    assert!(target.perform_enter_action(EnterAction::Search));
    assert!(!target.perform_enter_action(EnterAction::Done));
    assert_eq!(weak.upgrade().unwrap().get(), 2);
    drop(target);
    assert!(weak.upgrade().is_none());
    assert!(target_weak.upgrade().is_none());
}

struct DropAction(Option<Box<dyn FnOnce()>>);
impl DropAction {
    fn keep_alive(&self) {}
}
impl Drop for DropAction {
    fn drop(&mut self) {
        self.0.take().unwrap()();
    }
}

#[test]
fn replacing_a_callback_allows_capture_destructors_to_replace_it_again() {
    register_types();
    let target = Rc::new(TextBufferTarget::default());
    let weak = Rc::downgrade(&target);
    let drops = Rc::new(Cell::new(0));
    let observed = drops.clone();
    let captured = DropAction(Some(Box::new(move || {
        observed.set(observed.get() + 1);
        let target = weak.upgrade().unwrap();
        // The requested replacement is installed before the old capture retires.
        assert!(target.enter_action_handler().is_some());
        target.set_enter_action_handler(Some(Rc::new(|action| action == EnterAction::Done)));
    })));
    write(
        &target,
        Some(MarkupDelegate::new(move |_| {
            captured.keep_alive();
            into_markup_value(false)
        })),
    );
    write(
        &target,
        Some(MarkupDelegate::new(|_| into_markup_value(false))),
    );
    assert_eq!(drops.get(), 1);
    assert!(target.perform_enter_action(EnterAction::Done));
    assert!(!target.perform_enter_action(EnterAction::Go));
}

#[test]
fn capture_drop_panics_propagate_once_after_publishing_the_replacement() {
    let target = Rc::new(TextBufferTarget::default());
    let drops = Rc::new(Cell::new(0));
    let observed = drops.clone();
    let captured = DropAction(Some(Box::new(move || {
        observed.set(observed.get() + 1);
        std::panic::panic_any("capture drop marker");
    })));
    let callback: EnterActionHandler = Rc::new(move |_| {
        captured.keep_alive();
        false
    });
    target.set_enter_action_handler(Some(callback));
    let failure = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        target.set_enter_action_handler(Some(Rc::new(|action| action == EnterAction::Go)));
    }))
    .unwrap_err();
    assert_eq!(failure.downcast_ref::<&str>(), Some(&"capture drop marker"));
    assert_eq!(drops.get(), 1);
    assert!(target.perform_enter_action(EnterAction::Go));
    drop(target);
    assert_eq!(drops.get(), 1);
}

//! Public-peer contracts derived from AutomationPeer_Partial and UIAWindow.
//! The recorder substitutes only the platform event transport.

use super::*;
use crate::automation::peers::ControlAutomationPeer;
use crate::automation::provider::{ITextEditProvider, ITextProvider};
use crate::test_support::test_scope;
use crate::Control;
use std::cell::Cell;

#[derive(Default)]
struct RecordingSink {
    events: RefCell<Vec<(usize, TextEditTextChangedEventArgs)>>,
    callback: RefCell<Option<Rc<dyn Fn()>>>,
}

impl IAutomationEventSink for RecordingSink {
    fn raise_text_edit_text_changed(
        &self,
        source: &Ref<AutomationPeer>,
        event: &TextEditTextChangedEventArgs,
    ) {
        self.events.borrow_mut().push((
            std::ptr::from_ref::<AutomationPeer>(source) as usize,
            event.clone(),
        ));
        let callback = self.callback.borrow().clone();
        if let Some(callback) = callback {
            callback();
        }
    }
}

fn install_sink() -> Rc<RecordingSink> {
    let sink = Rc::new(RecordingSink::default());
    FerroLocator::current_mutable()
        .bind::<dyn IAutomationEventSink>()
        .to_constant(sink.clone());
    sink
}

fn peer() -> (Ref<Control>, Ref<AutomationPeer>) {
    let control = Control::new();
    let peer = ControlAutomationPeer::create_peer_for_element(&control);
    (control, peer)
}

fn identity(peer: &Ref<AutomationPeer>) -> usize {
    std::ptr::from_ref::<AutomationPeer>(peer) as usize
}

#[test]
fn public_text_edit_metadata_preserves_existing_peer_events_and_calls_typed_members() {
    use ferroui_base::metadata::{from_markup_value, into_markup_value};
    use ferroui_base::StaticType;

    let _scope = test_scope();
    let sink = install_sink();
    let (_owner, original) = peer();
    let (_source_owner, source) = peer();
    let markup = AutomationPeer::TYPE.markup().unwrap();
    assert!(markup
        .events
        .iter()
        .any(|event| event.name == "ChildrenChanged"));
    let property = markup.find_property("EventsSource").unwrap();
    let instance = into_markup_value(original.clone());
    (property.set.unwrap())(&[instance.clone(), into_markup_value(Some(source.clone()))]).unwrap();
    let read = (property.get.unwrap())(&[instance.clone()]).unwrap();
    assert_eq!(
        from_markup_value::<Option<Ref<AutomationPeer>>>(&read).unwrap(),
        Some(source.clone())
    );
    let raise = markup
        .methods
        .iter()
        .find(|method| method.name == "RaiseTextEditTextChangedEvent")
        .unwrap();
    (raise.invoke)(&[
        instance,
        into_markup_value(TextEditChangeType::Composition),
        into_markup_value(vec!["typed 😀".to_string()]),
    ])
    .unwrap();
    let events = sink.events.borrow();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].0, identity(&source));
    assert_eq!(events[0].1.changed_data, ["typed 😀"]);
}

#[test]
fn public_text_edit_event_works_for_an_arbitrary_peer_without_text_patterns() {
    let _scope = test_scope();
    let sink = install_sink();
    let (_control, peer) = peer();
    assert!(peer.get_provider::<dyn ITextProvider>().is_none());
    assert!(peer.get_provider::<dyn ITextEditProvider>().is_none());
    for change_type in [
        TextEditChangeType::AutoCorrect,
        TextEditChangeType::Composition,
        TextEditChangeType::CompositionFinalized,
    ] {
        let values = vec!["候\0😀".to_owned(), String::new()];
        peer.raise_text_edit_text_changed_event(change_type, values.clone());
        let events = sink.events.borrow();
        assert_eq!(
            events.last(),
            Some(&(
                identity(&peer),
                TextEditTextChangedEventArgs {
                    change_type,
                    changed_data: values,
                }
            ))
        );
    }
    assert_eq!(sink.events.borrow().len(), 3);
}

#[test]
fn public_text_edit_events_source_is_substituted_once_even_with_cycles() {
    let _scope = test_scope();
    let sink = install_sink();
    let (_a, a) = peer();
    let (_b, b) = peer();
    let (_c, c) = peer();
    a.set_events_source(Some(b.clone()));
    b.set_events_source(Some(c.clone()));
    a.raise_text_edit_text_changed_event(TextEditChangeType::AutoCorrect, vec!["one".into()]);
    b.set_events_source(Some(a.clone()));
    a.raise_text_edit_text_changed_event(TextEditChangeType::Composition, vec!["two".into()]);
    a.set_events_source(Some(a.clone()));
    a.raise_text_edit_text_changed_event(TextEditChangeType::CompositionFinalized, Vec::new());
    let events = sink.events.borrow();
    assert_eq!(
        events.iter().map(|event| event.0).collect::<Vec<_>>(),
        [identity(&b), identity(&b), identity(&a)]
    );
}

#[test]
fn public_text_edit_events_source_does_not_retain_other_peers_or_owners() {
    let _scope = test_scope();
    let sink = install_sink();
    let (owner, original) = peer();
    let weak_original = original.downgrade();
    let (target_owner, target) = peer();
    let weak_target = target.downgrade();
    original.set_events_source(Some(target.clone()));
    target.set_events_source(Some(original.clone()));
    drop(target);
    drop(target_owner);
    assert!(weak_target.upgrade().is_none());
    assert!(original.events_source().is_none());
    original.raise_text_edit_text_changed_event(TextEditChangeType::AutoCorrect, Vec::new());
    assert_eq!(sink.events.borrow()[0].0, identity(&original));
    original.set_events_source(Some(original.clone()));
    drop(original);
    drop(owner);
    assert!(weak_original.upgrade().is_none());
}

#[test]
fn public_text_edit_observation_is_disposable_and_its_handle_is_weak() {
    let _scope = test_scope();
    let sink = install_sink();
    let (owner, peer) = peer();
    let weak = peer.downgrade();
    let count = Rc::new(Cell::new(0));
    let observed = count.clone();
    let subscription = peer.text_edit_text_changed(move |event| {
        assert_eq!(event.change_type, TextEditChangeType::Composition);
        observed.set(observed.get() + 1);
    });
    peer.raise_text_edit_text_changed_event(TextEditChangeType::None, vec!["ignored".into()]);
    assert!(sink.events.borrow().is_empty());
    assert_eq!(count.get(), 0);
    peer.raise_text_edit_text_changed_event(TextEditChangeType::Composition, Vec::new());
    subscription.dispose();
    subscription.dispose();
    peer.raise_text_edit_text_changed_event(TextEditChangeType::Composition, Vec::new());
    assert_eq!(count.get(), 1);
    assert_eq!(sink.events.borrow().len(), 2);
    drop(peer);
    drop(owner);
    assert!(weak.upgrade().is_none());
    subscription.dispose();
}

#[test]
fn public_text_edit_delivery_captures_source_once_across_reentrant_callbacks() {
    let _scope = test_scope();
    let sink = install_sink();
    let (_owner, original) = peer();
    let (_b, b) = peer();
    let (_c, c) = peer();
    original.set_events_source(Some(b.clone()));
    let original_weak = original.downgrade();
    let c_weak = c.downgrade();
    let entered = Rc::new(Cell::new(false));
    *sink.callback.borrow_mut() = Some(Rc::new(move || {
        if !entered.replace(true) {
            let original = original_weak.upgrade().unwrap();
            original.set_events_source(c_weak.upgrade());
            original.raise_text_edit_text_changed_event(
                TextEditChangeType::CompositionFinalized,
                vec!["nested".into()],
            );
        }
    }));
    original
        .raise_text_edit_text_changed_event(TextEditChangeType::Composition, vec!["outer".into()]);
    let events = sink.events.borrow();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].0, identity(&b));
    assert_eq!(events[0].1.changed_data, ["outer"]);
    assert_eq!(events[1].0, identity(&c));
    assert_eq!(events[1].1.changed_data, ["nested"]);
}

#[test]
fn public_text_edit_observers_can_rewire_and_dispose_during_delivery() {
    let _scope = test_scope();
    let (_owner, peer) = peer();
    let count = Rc::new(Cell::new(0));
    let subscription = Rc::new(RefCell::new(None::<Rc<dyn IDisposable>>));
    let observed = count.clone();
    let handle = subscription.clone();
    let weak = peer.downgrade();
    *subscription.borrow_mut() = Some(peer.text_edit_text_changed(move |_| {
        observed.set(observed.get() + 1);
        let subscription = handle.borrow_mut().take();
        subscription.unwrap().dispose();
        weak.upgrade().unwrap().raise_text_edit_text_changed_event(
            TextEditChangeType::CompositionFinalized,
            Vec::new(),
        );
    }));
    peer.raise_text_edit_text_changed_event(TextEditChangeType::Composition, Vec::new());
    peer.raise_text_edit_text_changed_event(TextEditChangeType::Composition, Vec::new());
    assert_eq!(count.get(), 1);
}

#[test]
fn rich_text_first_composition_creates_its_peer_before_forwarding_once() {
    use crate::RichEditBox;
    let _scope = test_scope();
    let sink = install_sink();
    for through_model in [false, true] {
        let editor = RichEditBox::new();
        let model = editor.editor();
        assert!(ControlAutomationPeer::from_element(&editor).is_none());
        if through_model {
            model.set_preedit("first", None).unwrap();
        } else {
            editor.set_preedit_text(Some("first"), None).unwrap();
        }
        let peer = ControlAutomationPeer::from_element(&editor).unwrap();
        assert_eq!(sink.events.borrow().len(), 1);
        assert_eq!(sink.events.borrow()[0].0, identity(&peer));
        assert_eq!(sink.events.borrow()[0].1.changed_data, ["first"]);
        model.set_preedit("next", None).unwrap();
        model.cancel_composition().unwrap();
        let events = sink.events.take();
        assert_eq!(events.len(), 3);
        assert!(events.iter().all(|event| event.0 == identity(&peer)));
        assert_eq!(
            events[2].1.change_type,
            TextEditChangeType::CompositionFinalized
        );
        assert_eq!(events[2].1.changed_data, [""]);
    }
}

#[test]
fn rich_text_automatic_events_ignore_detached_and_retained_editors() {
    use crate::documents::RichTextDocument;
    use crate::RichEditBox;
    let _scope = test_scope();
    let sink = install_sink();
    let (detached, retained, weak_owner, weak_peer) = {
        let owner = RichEditBox::new();
        let detached = owner.editor();
        owner.set_document(RichTextDocument::from_text("replacement"));
        detached.set_preedit("detached", None).unwrap();
        assert!(sink.events.borrow().is_empty());
        assert!(ControlAutomationPeer::from_element(&owner).is_none());
        let retained = owner.editor();
        retained.set_preedit("live", None).unwrap();
        let peer = ControlAutomationPeer::from_element(&owner).unwrap();
        assert_eq!(sink.events.borrow().len(), 1);
        assert_eq!(sink.events.borrow()[0].0, identity(&peer));
        (detached, retained, owner.downgrade(), peer.downgrade())
    };
    assert!(weak_owner.upgrade().is_none());
    assert!(weak_peer.upgrade().is_none());
    detached.set_preedit("still detached", None).unwrap();
    retained.set_preedit("owner gone", None).unwrap();
    retained.cancel_composition().unwrap();
    assert_eq!(sink.events.borrow().len(), 1);
}

//! `ChangeSetExtensionsTests` (`ReactiveUI.Tests/ChangeSets/ChangeSetExtensionsTests.cs`).
//!
//! The collection that raises chosen notifications implements [`ChangeSetSource`], the port's form
//! of upstream's `INotifyCollectionChanged, IEnumerable<string>`.

use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::*;
use ferroui_system::collections::object_model::CollectionChangedHandler;
use ferroui_system::collections::specialized::{NotifyCollectionChangedAction, NotifyCollectionChangedEventArgs};
use ferroui_system::IDisposable;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// A collection that raises the notifications a test gives it (`RaisingCollection`).
#[derive(Default)]
struct RaisingCollection {
    items: Vec<String>,
    handlers: RefCell<Vec<(u64, Rc<CollectionChangedHandler<String>>)>>,
    next_token: Cell<u64>,
}

impl RaisingCollection {
    fn new(items: &[&str]) -> Rc<Self> {
        Rc::new(Self { items: items.iter().map(|item| (*item).to_owned()).collect(), ..Self::default() })
    }

    /// `Raise`.
    fn raise(&self, args: &NotifyCollectionChangedEventArgs<'_, String>) {
        let handlers: Vec<Rc<CollectionChangedHandler<String>>> = self.handlers.borrow().iter().map(|(_, handler)| handler.clone()).collect();
        for handler in handlers {
            handler(args);
        }
    }
}

impl ChangeSetSource<String> for RaisingCollection {
    fn current_items(&self) -> Vec<String> {
        self.items.clone()
    }

    fn add_collection_changed(&self, handler: Rc<CollectionChangedHandler<String>>) -> u64 {
        let token = self.next_token.get() + 1;
        self.next_token.set(token);
        self.handlers.borrow_mut().push((token, handler));
        token
    }

    fn remove_collection_changed(&self, token: u64) -> bool {
        let mut handlers = self.handlers.borrow_mut();
        let before = handlers.len();
        handlers.retain(|(candidate, _)| *candidate != token);
        handlers.len() != before
    }
}

fn args<'a>(
    action: NotifyCollectionChangedAction,
    new_items: &'a [String],
    old_items: &'a [String],
    new_starting_index: i32,
    old_starting_index: i32,
) -> NotifyCollectionChangedEventArgs<'a, String> {
    NotifyCollectionChangedEventArgs { action, new_items, old_items, new_starting_index, old_starting_index }
}

fn subscribe(collection: &Rc<RaisingCollection>) -> (Rc<RefCell<Vec<ReactiveChangeSet<String>>>>, Rc<dyn IDisposable>) {
    let sets = Rc::new(RefCell::new(Vec::new()));
    let sink = sets.clone();
    let subscription = collection.to_reactive_change_set().subscribe_next(move |set| sink.borrow_mut().push(set));
    (sets, subscription)
}

#[test]
fn add_with_unknown_index_appends() {
    let collection = RaisingCollection::new(&["a", "b"]);
    let (sets, subscription) = subscribe(&collection);

    let added = ["c".to_owned()];
    collection.raise(&args(NotifyCollectionChangedAction::Add, &added, &[], -1, -1));

    let sets = sets.borrow();
    let last = sets.last().unwrap();
    assert_eq!(last[0].reason(), ReactiveChangeReason::Add);
    assert_eq!(last[0].current(), "c");
    subscription.dispose();
}

#[test]
fn remove_with_unknown_index_removes_by_value() {
    let collection = RaisingCollection::new(&["a", "b"]);
    let (sets, subscription) = subscribe(&collection);

    let removed = ["a".to_owned()];
    collection.raise(&args(NotifyCollectionChangedAction::Remove, &[], &removed, -1, -1));

    let sets = sets.borrow();
    let last = sets.last().unwrap();
    assert_eq!(last[0].reason(), ReactiveChangeReason::Remove);
    assert_eq!(last[0].current(), "a");
    subscription.dispose();
}

#[test]
fn move_emits_move_change() {
    let collection = RaisingCollection::new(&["a", "b"]);
    let (sets, subscription) = subscribe(&collection);

    // `new NotifyCollectionChangedEventArgs(Move, "b", 0, 1)`: "b" moves from 1 to 0.
    let moved = ["b".to_owned()];
    collection.raise(&args(NotifyCollectionChangedAction::Move, &moved, &moved, 0, 1));

    let sets = sets.borrow();
    let last = sets.last().unwrap();
    assert_eq!(last[0].reason(), ReactiveChangeReason::Move);
    subscription.dispose();
}

#[test]
fn reset_re_emits_removes_then_adds() {
    let collection = RaisingCollection::new(&["a", "b"]);
    let (sets, subscription) = subscribe(&collection);

    collection.raise(&args(NotifyCollectionChangedAction::Reset, &[], &[], -1, -1));

    let sets = sets.borrow();
    let last = sets.last().unwrap();
    assert!(last.removes() > 0);
    assert!(last.adds() > 0);
    subscription.dispose();
}

#[test]
fn reset_on_empty_collection_emits_no_further_change_set() {
    let collection = RaisingCollection::new(&[]);
    let (sets, subscription) = subscribe(&collection);

    // Only the first (empty) change set.
    let initial_count = sets.borrow().len();

    // A reset with an empty shadow and an empty collection makes no change, so nothing is
    // notified.
    collection.raise(&args(NotifyCollectionChangedAction::Reset, &[], &[], -1, -1));

    assert_eq!(sets.borrow().len(), initial_count);
    subscription.dispose();
}

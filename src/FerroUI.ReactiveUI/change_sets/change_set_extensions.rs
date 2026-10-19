//! `ChangeSetExtensions` (`ReactiveUI.Core/ChangeSets/ChangeSetExtensions.cs`).

use super::{ReactiveChange, ReactiveChangeReason, ReactiveChangeSet};
use ferroui_system::collections::object_model::{CollectionChangedHandler, ObservableCollection};
use ferroui_system::collections::specialized::{NotifyCollectionChangedAction, NotifyCollectionChangedEventArgs};
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// A collection whose changes can be observed as change sets: it raises the typed
/// `CollectionChanged` notification and enumerates its items (upstream's constraint
/// `INotifyCollectionChanged, IEnumerable<T>`).
///
/// Deviation (DEVIATIONS.md, ReactiveUI): the `INotifyCollectionChanged` of `ferroui-system`
/// raises an untyped description of a change, without the items; the typed notification that
/// carries them is the one of each collection type, which this trait names.
pub trait ChangeSetSource<T> {
    /// A copy of the items, in order (`IEnumerable<T>`).
    fn current_items(&self) -> Vec<T>;

    /// Adds a handler of the typed change notifications; returns the token that removes it
    /// (`CollectionChanged +=`).
    fn add_collection_changed(&self, handler: Rc<CollectionChangedHandler<T>>) -> u64;

    /// Removes a handler (`CollectionChanged -=`).
    fn remove_collection_changed(&self, token: u64) -> bool;
}

impl<T: Clone + PartialEq + 'static> ChangeSetSource<T> for ObservableCollection<T> {
    fn current_items(&self) -> Vec<T> {
        self.to_vec()
    }

    fn add_collection_changed(&self, handler: Rc<CollectionChangedHandler<T>>) -> u64 {
        ObservableCollection::add_collection_changed(self, handler)
    }

    fn remove_collection_changed(&self, token: u64) -> bool {
        ObservableCollection::remove_collection_changed(self, token)
    }
}

impl<T, C: ChangeSetSource<T> + ?Sized> ChangeSetSource<T> for Rc<C> {
    fn current_items(&self) -> Vec<T> {
        (**self).current_items()
    }

    fn add_collection_changed(&self, handler: Rc<CollectionChangedHandler<T>>) -> u64 {
        (**self).add_collection_changed(handler)
    }

    fn remove_collection_changed(&self, token: u64) -> bool {
        (**self).remove_collection_changed(token)
    }
}

/// The changes of a collection as a stream of change sets (`ChangeSetExtensions`), the light
/// replacement of DynamicData's change sets inside ReactiveUI.
pub trait ChangeSetExtensions<T: 'static> {
    /// A stream that gives, at subscription, a change set adding the current items (empty for an
    /// empty collection), then one change set per change of the collection
    /// (`ToReactiveChangeSet`).
    fn to_reactive_change_set(&self) -> Rc<dyn IObservable<ReactiveChangeSet<T>>>;
}

impl<T: Clone + PartialEq + 'static, C: ChangeSetSource<T> + Clone + 'static> ChangeSetExtensions<T> for C {
    fn to_reactive_change_set(&self) -> Rc<dyn IObservable<ReactiveChangeSet<T>>> {
        Rc::new(ChangeSetObservable { collection: self.clone(), _items: std::marker::PhantomData })
    }
}

/// `ChangeSetObservable<TCollection, T>`: translates each change of the collection into a change
/// set of one change per item, keeping a shadow copy of the items for the first change set and
/// the resets.
struct ChangeSetObservable<C, T> {
    collection: C,
    _items: std::marker::PhantomData<fn() -> T>,
}

impl<T: Clone + PartialEq + 'static, C: ChangeSetSource<T> + Clone + 'static> IObservable<ReactiveChangeSet<T>>
    for ChangeSetObservable<C, T>
{
    fn subscribe(&self, observer: Rc<dyn IObserver<ReactiveChangeSet<T>>>) -> Rc<dyn IDisposable> {
        let subscription = Rc::new(Subscription {
            collection: self.collection.clone(),
            observer,
            shadow: RefCell::new(Vec::new()),
            token: Cell::new(None),
        });
        subscription.start();
        subscription
    }
}

/// `ChangeSetObservable.Subscription`.
struct Subscription<C, T> {
    collection: C,
    observer: Rc<dyn IObserver<ReactiveChangeSet<T>>>,
    /// `_shadow`.
    shadow: RefCell<Vec<T>>,
    /// The token of the handler, while hooked.
    token: Cell<Option<u64>>,
}

impl<T: Clone + PartialEq + 'static, C: ChangeSetSource<T> + Clone + 'static> Subscription<C, T> {
    /// `Start`: the first change set, for the current items, then the hook of the changes.
    fn start(self: &Rc<Self>) {
        self.shadow.borrow_mut().extend(self.collection.current_items());

        // A first change set is given even for an empty collection, as DynamicData's
        // `ToObservableChangeSet` does; it adds nothing, so the consumers of count changes ignore
        // it.
        let initial: Vec<ReactiveChange<T>> = self
            .shadow
            .borrow()
            .iter()
            .enumerate()
            .map(|(index, item)| ReactiveChange::new(ReactiveChangeReason::Add, item.clone(), None, index as i32, -1))
            .collect();
        self.observer.on_next(ReactiveChangeSet::new(initial));

        let this = self.clone();
        let handler: Rc<CollectionChangedHandler<T>> =
            Rc::new(move |args: &NotifyCollectionChangedEventArgs<'_, T>| this.on_collection_changed(args));
        self.token.set(Some(self.collection.add_collection_changed(handler)));
    }

    /// `OnCollectionChanged`.
    fn on_collection_changed(&self, args: &NotifyCollectionChangedEventArgs<'_, T>) {
        let mut changes = Vec::with_capacity(self.change_capacity(args));
        match args.action {
            NotifyCollectionChangedAction::Add => self.apply_add(args, &mut changes),
            NotifyCollectionChangedAction::Remove => self.apply_remove(args, &mut changes),
            NotifyCollectionChangedAction::Replace => self.apply_replace(args, &mut changes),
            NotifyCollectionChangedAction::Move => self.apply_move(args, &mut changes),
            NotifyCollectionChangedAction::Reset => self.apply_reset(&mut changes),
        }

        if changes.is_empty() {
            return;
        }

        self.observer.on_next(ReactiveChangeSet::new(changes));
    }

    /// `GetChangeCapacity`: the items of the change, or twice the shadow for a reset, at least 4.
    fn change_capacity(&self, args: &NotifyCollectionChangedEventArgs<'_, T>) -> usize {
        const DEFAULT_CHANGE_CAPACITY: usize = 4;
        const RESET_CHANGE_CAPACITY_MULTIPLIER: usize = 2;
        let count = if args.action == NotifyCollectionChangedAction::Reset {
            self.shadow.borrow().len() * RESET_CHANGE_CAPACITY_MULTIPLIER
        } else {
            args.new_items.len() + args.old_items.len()
        };
        count.max(DEFAULT_CHANGE_CAPACITY)
    }

    /// `ApplyAdd`.
    fn apply_add(&self, args: &NotifyCollectionChangedEventArgs<'_, T>, changes: &mut Vec<ReactiveChange<T>>) {
        let start = args.new_starting_index;
        let mut shadow = self.shadow.borrow_mut();
        for (offset, item) in args.new_items.iter().enumerate() {
            let index = if start < 0 { shadow.len() } else { start as usize + offset };
            shadow.insert(index, item.clone());
            changes.push(ReactiveChange::new(ReactiveChangeReason::Add, item.clone(), None, index as i32, -1));
        }
    }

    /// `ApplyRemove`.
    fn apply_remove(&self, args: &NotifyCollectionChangedEventArgs<'_, T>, changes: &mut Vec<ReactiveChange<T>>) {
        let start = args.old_starting_index;
        for item in args.old_items {
            let index = if start < 0 { -1 } else { start };
            self.remove_from_shadow(index, item);
            changes.push(ReactiveChange::new(ReactiveChangeReason::Remove, item.clone(), None, index, -1));
        }
    }

    /// `ApplyReplace`.
    fn apply_replace(&self, args: &NotifyCollectionChangedEventArgs<'_, T>, changes: &mut Vec<ReactiveChange<T>>) {
        let start = args.new_starting_index;
        let mut shadow = self.shadow.borrow_mut();
        for (offset, (current, previous)) in args.new_items.iter().zip(args.old_items).enumerate() {
            let index = if start < 0 { -1 } else { start + offset as i32 };
            if index >= 0 && (index as usize) < shadow.len() {
                shadow[index as usize] = current.clone();
            }
            changes.push(ReactiveChange::new(ReactiveChangeReason::Replace, current.clone(), Some(previous.clone()), index, -1));
        }
    }

    /// `ApplyMove`.
    fn apply_move(&self, args: &NotifyCollectionChangedEventArgs<'_, T>, changes: &mut Vec<ReactiveChange<T>>) {
        let Some(item) = args.new_items.first() else { return };
        let mut shadow = self.shadow.borrow_mut();
        let old_index = args.old_starting_index;
        if old_index >= 0 && (old_index as usize) < shadow.len() {
            shadow.remove(old_index as usize);
        }

        let index = if args.new_starting_index < 0 { shadow.len() } else { args.new_starting_index as usize };
        shadow.insert(index, item.clone());
        changes.push(ReactiveChange::new(ReactiveChangeReason::Move, item.clone(), None, index as i32, old_index));
    }

    /// `ApplyReset`: a removal for each former item, then an addition for each item the
    /// collection has now, so that the items still there are subscribed again rather than lost.
    fn apply_reset(&self, changes: &mut Vec<ReactiveChange<T>>) {
        let mut shadow = self.shadow.borrow_mut();
        for (index, item) in shadow.iter().enumerate() {
            changes.push(ReactiveChange::new(ReactiveChangeReason::Remove, item.clone(), None, index as i32, -1));
        }

        shadow.clear();
        shadow.extend(self.collection.current_items());
        for (index, item) in shadow.iter().enumerate() {
            changes.push(ReactiveChange::new(ReactiveChangeReason::Add, item.clone(), None, index as i32, -1));
        }
    }

    /// `RemoveFromShadow`: by index when it is valid, else by value.
    fn remove_from_shadow(&self, index: i32, item: &T) {
        let mut shadow = self.shadow.borrow_mut();
        if index >= 0 && (index as usize) < shadow.len() {
            shadow.remove(index as usize);
            return;
        }

        if let Some(position) = shadow.iter().position(|candidate| candidate == item) {
            shadow.remove(position);
        }
    }
}

impl<C: ChangeSetSource<T>, T> IDisposable for Subscription<C, T> {
    /// Unhooks the changes of the collection (`Dispose`).
    fn dispose(&self) {
        if let Some(token) = self.token.take() {
            let _ = self.collection.remove_collection_changed(token);
        }
    }
}

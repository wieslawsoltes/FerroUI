//! The helpers of the ported tests: `TestCollection.cs` and `ResettingCollection.cs` of the reference's tests, and
//! the assertions the tests share (`Assert.Throws`, `Assert.Same`, `Assert.Single`).

#![allow(dead_code)]

use crate::model_collection::{reset_args, ModelCollection, Models, ModelsChangedArgs, ModelsChangedHandler, MutableModelCollection};
use crate::models::{IRows, RowRef};
use ferroui_base::collections::{NotifyCollectionChangedAction, NotifyCollectionChangedEventArgs};
use ferroui_base::BoxedValue;
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// `Assert.Throws<T>(() => ...)`: the reference's exceptions are panics.
pub fn assert_panics(f: impl FnOnce()) {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
    assert!(result.is_err(), "expected the action to panic");
}

/// A column header (`"Name"`).
pub fn header(text: &str) -> Option<BoxedValue> {
    Some(Rc::new(text.to_string()) as BoxedValue)
}

/// Whether two shared values are the same object (`ReferenceEquals`).
pub fn same_rc<T: ?Sized, U: ?Sized>(a: &Rc<T>, b: &Rc<U>) -> bool {
    std::ptr::addr_eq(Rc::as_ptr(a), Rc::as_ptr(b))
}

/// `Assert.Same(expected, actual)`.
pub fn assert_same<T: ?Sized, U: ?Sized>(expected: &Rc<T>, actual: &Rc<U>) {
    assert!(same_rc(expected, actual), "expected the same object");
}

/// `Assert.Equal(expected, actual)` of sequences of objects compared by identity.
pub fn assert_same_items<T: ?Sized, U: ?Sized>(expected: &[Rc<T>], actual: &[Rc<U>]) {
    assert_eq!(expected.len(), actual.len(), "the sequences have different lengths");
    for (i, (e, a)) in expected.iter().zip(actual.iter()).enumerate() {
        assert!(same_rc(e, a), "the items at {i} are not the same object");
    }
}

/// `Assert.Single(items)`: the only item.
pub fn single<T>(items: Vec<T>) -> T {
    assert_eq!(1, items.len(), "expected a single item");
    items.into_iter().next().expect("a single item")
}

/// The items of a list of nullable items, none of which is null.
pub fn some_items<M: ?Sized>(items: Vec<Option<Rc<M>>>) -> Vec<Rc<M>> {
    items.into_iter().map(|item| item.expect("an item that is not null")).collect()
}

/// An untyped model as its model type (`(Node)x.Model!`).
pub fn downcast_model<M: 'static>(value: Option<BoxedValue>) -> Rc<M> {
    let value = value.expect("a model");
    let any: Rc<dyn Any> = value;
    any.downcast::<M>().unwrap_or_else(|_| panic!("a model of type {}", std::any::type_name::<M>()))
}

/// The model of a row.
pub fn row_model<M: 'static>(row: &RowRef) -> Rc<M> {
    downcast_model(row.model())
}

/// The models of the rows (`source.Rows.Select(x => x.Model)`); each row is read before the next is
/// fetched, as the reused row of a flat source requires.
pub fn row_models<M: 'static>(rows: &dyn IRows) -> Vec<Rc<M>> {
    (0..rows.count()).map(|i| row_model::<M>(&rows.get(i))).collect()
}

/// A C# array of models: fixed and not notifying.
pub fn array<M: 'static>(items: Vec<Rc<M>>) -> Models<M> {
    Rc::new(items)
}

struct Subscription<M: 'static> {
    token: u64,
    handler: Rc<ModelsChangedHandler<M>>,
    // Subscribed through the interface (`INotifyCollectionChanged.CollectionChanged`), which the
    // reference's test collection counts.
    counted: bool,
}

/// The handlers of the change notifications of a test collection.
struct Handlers<M: 'static> {
    subscriptions: RefCell<Vec<Subscription<M>>>,
    next_token: Cell<u64>,
}

impl<M: 'static> Handlers<M> {
    fn new() -> Self {
        Self { subscriptions: RefCell::new(Vec::new()), next_token: Cell::new(1) }
    }

    fn add(&self, handler: Rc<ModelsChangedHandler<M>>, counted: bool) -> u64 {
        let token = self.next_token.get();
        self.next_token.set(token + 1);
        self.subscriptions.borrow_mut().push(Subscription { token, handler, counted });
        token
    }

    fn remove(&self, token: u64) {
        self.subscriptions.borrow_mut().retain(|s| s.token != token);
    }

    fn counted(&self) -> usize {
        self.subscriptions.borrow().iter().filter(|s| s.counted).count()
    }

    fn raise(&self, args: &ModelsChangedArgs<'_, M>) {
        let handlers: Vec<Rc<ModelsChangedHandler<M>>> = self.subscriptions.borrow().iter().map(|s| s.handler.clone()).collect();
        for handler in handlers {
            handler(args);
        }
    }
}

fn args<'a, M>(
    action: NotifyCollectionChangedAction,
    new_items: &'a [Rc<M>],
    old_items: &'a [Rc<M>],
    new_starting_index: i32,
    old_starting_index: i32,
) -> ModelsChangedArgs<'a, M> {
    NotifyCollectionChangedEventArgs { action, new_items, old_items, new_starting_index, old_starting_index }
}

/// `TestCollection<T>`: an observable collection that counts the subscribers of its interface event and
/// raises the removal and the move of ranges.
pub struct TestCollection<M: 'static> {
    items: RefCell<Vec<Rc<M>>>,
    handlers: Handlers<M>,
}

impl<M: 'static> TestCollection<M> {
    pub fn new() -> Rc<Self> {
        Self::from_items(Vec::new())
    }

    pub fn from_items(items: impl IntoIterator<Item = Rc<M>>) -> Rc<Self> {
        Rc::new(Self { items: RefCell::new(items.into_iter().collect()), handlers: Handlers::new() })
    }

    pub fn count(&self) -> usize {
        self.items.borrow().len()
    }

    pub fn get(&self, index: usize) -> Rc<M> {
        self.items.borrow()[index].clone()
    }

    pub fn to_vec(&self) -> Vec<Rc<M>> {
        self.items.borrow().clone()
    }

    /// The class's own `CollectionChanged` (not counted by
    /// [`collection_changed_subscriber_count`](Self::collection_changed_subscriber_count)).
    pub fn on_collection_changed(&self, handler: Rc<ModelsChangedHandler<M>>) -> u64 {
        self.handlers.add(handler, false)
    }

    /// `GetCollectionChangedSubscribers()?.Length ?? 0`: the subscribers of the interface event.
    pub fn collection_changed_subscriber_count(&self) -> usize {
        self.handlers.counted()
    }

    pub fn add(&self, item: Rc<M>) {
        let index = self.count();
        self.insert(index, item);
    }

    pub fn insert(&self, index: usize, item: Rc<M>) {
        self.items.borrow_mut().insert(index, item.clone());
        self.handlers.raise(&args(NotifyCollectionChangedAction::Add, std::slice::from_ref(&item), &[], index as i32, -1));
    }

    pub fn remove_at(&self, index: usize) {
        let item = self.items.borrow_mut().remove(index);
        self.handlers.raise(&args(NotifyCollectionChangedAction::Remove, &[], std::slice::from_ref(&item), -1, index as i32));
    }

    /// `this[index] = item`.
    pub fn set(&self, index: usize, item: Rc<M>) {
        let old = std::mem::replace(&mut self.items.borrow_mut()[index], item.clone());
        self.handlers.raise(&args(
            NotifyCollectionChangedAction::Replace,
            std::slice::from_ref(&item),
            std::slice::from_ref(&old),
            index as i32,
            index as i32,
        ));
    }

    /// `Move(oldIndex, newIndex)`.
    pub fn move_item(&self, old_index: usize, new_index: usize) {
        let item = {
            let mut items = self.items.borrow_mut();
            let item = items.remove(old_index);
            items.insert(new_index, item.clone());
            item
        };
        self.handlers.raise(&args(
            NotifyCollectionChangedAction::Move,
            std::slice::from_ref(&item),
            std::slice::from_ref(&item),
            new_index as i32,
            old_index as i32,
        ));
    }

    /// `Clear()`: a reset.
    pub fn clear(&self) {
        self.items.borrow_mut().clear();
        self.handlers.raise(&reset_args::<M>());
    }

    /// `RemoveRange(index, count)`: one removal of the range.
    pub fn remove_range(&self, index: usize, count: usize) {
        let removed: Vec<Rc<M>> = self.items.borrow_mut().drain(index..index + count).collect();
        self.handlers.raise(&args(NotifyCollectionChangedAction::Remove, &[], &removed, -1, index as i32));
    }

    /// `MoveRange(oldIndex, newIndex, count)`: removes `count` items at `old_index` and inserts them at
    /// `new_index` of the remaining items; one move of the range.
    pub fn move_range(&self, old_index: usize, new_index: usize, count: usize) {
        let moved: Vec<Rc<M>> = {
            let mut items = self.items.borrow_mut();
            let moved: Vec<Rc<M>> = items.drain(old_index..old_index + count).collect();
            for (i, item) in moved.iter().enumerate() {
                items.insert(new_index + i, item.clone());
            }
            moved
        };
        self.handlers.raise(&args(NotifyCollectionChangedAction::Move, &moved, &moved, new_index as i32, old_index as i32));
    }
}

impl<M: 'static> ModelCollection<M> for TestCollection<M> {
    fn count(&self) -> usize {
        TestCollection::count(self)
    }

    fn get(&self, index: usize) -> Rc<M> {
        TestCollection::get(self, index)
    }

    fn add_collection_changed(&self, handler: Rc<ModelsChangedHandler<M>>) -> Option<u64> {
        Some(self.handlers.add(handler, true))
    }

    fn remove_collection_changed(&self, token: u64) {
        self.handlers.remove(token);
    }

    fn is_notifying(&self) -> bool {
        true
    }

    fn as_mutable(&self) -> Option<&dyn MutableModelCollection<M>> {
        Some(self)
    }

    fn to_vec(&self) -> Vec<Rc<M>> {
        TestCollection::to_vec(self)
    }
}

// The `IList<T>` of an observable collection raises the changes.
impl<M: 'static> MutableModelCollection<M> for TestCollection<M> {
    fn insert(&self, index: usize, model: Rc<M>) {
        TestCollection::insert(self, index, model);
    }

    fn remove_at(&self, index: usize) {
        TestCollection::remove_at(self, index);
    }

    fn set(&self, index: usize, model: Rc<M>) {
        TestCollection::set(self, index, model);
    }
}

/// `ResettingCollection<T>`: a list that raises the changes it is told to, among them resets and changes
/// without indexes.
pub struct ResettingCollection<M: 'static> {
    items: RefCell<Vec<Rc<M>>>,
    handlers: Handlers<M>,
}

impl<M: 'static> ResettingCollection<M> {
    pub fn new(items: impl IntoIterator<Item = Rc<M>>) -> Rc<Self> {
        Rc::new(Self { items: RefCell::new(items.into_iter().collect()), handlers: Handlers::new() })
    }

    pub fn count(&self) -> usize {
        self.items.borrow().len()
    }

    pub fn get(&self, index: usize) -> Rc<M> {
        self.items.borrow()[index].clone()
    }

    pub fn to_vec(&self) -> Vec<Rc<M>> {
        self.items.borrow().clone()
    }

    /// `RemoveAt(index)`: a removal with its index.
    pub fn remove_at(&self, index: usize) {
        let item = self.items.borrow_mut().remove(index);
        self.handlers.raise(&args(NotifyCollectionChangedAction::Remove, &[], std::slice::from_ref(&item), -1, index as i32));
    }

    /// `Reset(items)`: replaces the items and raises a reset.
    pub fn reset(&self, items: impl IntoIterator<Item = Rc<M>>) {
        let items: Vec<Rc<M>> = items.into_iter().collect();
        *self.items.borrow_mut() = items;
        self.handlers.raise(&reset_args::<M>());
    }

    /// `AddWithoutIndex(item)`: an addition without its index.
    pub fn add_without_index(&self, item: Rc<M>) {
        self.items.borrow_mut().push(item.clone());
        self.handlers.raise(&args(NotifyCollectionChangedAction::Add, std::slice::from_ref(&item), &[], -1, -1));
    }

    /// `RemoveAtWithoutIndex(index)`: a removal without its index.
    pub fn remove_at_without_index(&self, index: usize) {
        let item = self.items.borrow_mut().remove(index);
        self.handlers.raise(&args(NotifyCollectionChangedAction::Remove, &[], std::slice::from_ref(&item), -1, -1));
    }

    /// `ReplaceWithoutIndex(index, item)`: a replacement without its index.
    pub fn replace_without_index(&self, index: usize, item: Rc<M>) {
        let old = std::mem::replace(&mut self.items.borrow_mut()[index], item.clone());
        self.handlers.raise(&args(
            NotifyCollectionChangedAction::Replace,
            std::slice::from_ref(&item),
            std::slice::from_ref(&old),
            -1,
            -1,
        ));
    }

    /// `ReplaceRange(index, count, items)`: one replacement of `count` items by `items`, which may be more
    /// or fewer.
    pub fn replace_range(&self, index: usize, count: usize, items: impl IntoIterator<Item = Rc<M>>) {
        let new_items: Vec<Rc<M>> = items.into_iter().collect();
        let old_items: Vec<Rc<M>> = {
            let mut list = self.items.borrow_mut();
            list.splice(index..index + count, new_items.iter().cloned()).collect()
        };
        self.handlers.raise(&args(NotifyCollectionChangedAction::Replace, &new_items, &old_items, index as i32, index as i32));
    }
}

impl<M: 'static> ModelCollection<M> for ResettingCollection<M> {
    fn count(&self) -> usize {
        ResettingCollection::count(self)
    }

    fn get(&self, index: usize) -> Rc<M> {
        ResettingCollection::get(self, index)
    }

    fn add_collection_changed(&self, handler: Rc<ModelsChangedHandler<M>>) -> Option<u64> {
        Some(self.handlers.add(handler, true))
    }

    fn remove_collection_changed(&self, token: u64) {
        self.handlers.remove(token);
    }

    fn is_notifying(&self) -> bool {
        true
    }

    fn as_mutable(&self) -> Option<&dyn MutableModelCollection<M>> {
        Some(self)
    }

    fn to_vec(&self) -> Vec<Rc<M>> {
        ResettingCollection::to_vec(self)
    }
}

// The `IList<T>` of the reference's collection is the one of `List<T>`, which raises nothing.
impl<M: 'static> MutableModelCollection<M> for ResettingCollection<M> {
    fn insert(&self, index: usize, model: Rc<M>) {
        self.items.borrow_mut().insert(index, model);
    }

    fn remove_at(&self, index: usize) {
        self.items.borrow_mut().remove(index);
    }

    fn set(&self, index: usize, model: Rc<M>) {
        self.items.borrow_mut()[index] = model;
    }
}

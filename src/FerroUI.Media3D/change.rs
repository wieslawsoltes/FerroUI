//! How the scene objects tell their owners that they changed.
//!
//! Every scene object (a transform, a rotation, a camera, a model, a geometry, a material, a
//! visual) has a [`Notifier`]. A change of one of its properties raises a [`Scene3DChange`] that
//! says what the change touches in a compiled scene; an object forwards the changes of the
//! objects it holds (its transform, its children, its geometry) to its own notifier, so a change
//! anywhere in a tree reaches the `Viewport3DVisual` at its root, which bumps the generation of
//! that kind and drops its snapshot. WPF raises `Changed` on its freezables for the same
//! purpose; the kind of the change is what the port adds, for the generations of the snapshot
//! (`docs/porting/media3d.md`, section 3.7).

use ferroui_base::media::MediaCollection;
use ferroui_base::reactive::{Disposable, Subscription};
use ferroui_base::{ObjectType, Ref};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// What a change of a scene object changes in a compiled scene: which generation of the
/// snapshot it bumps.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Scene3DChange {
    /// The structure: a mesh, which materials (and brushes) a model has, or the objects of the
    /// tree (a model, a light or a visual added, removed or replaced).
    Structure,
    /// A record: a transform, a light, or a value of a material (a colour, the specular power,
    /// the values of a brush).
    Record,
    /// The camera or the viewport.
    Camera,
}

/// The changes of a child as its owner sees them: unchanged.
pub(crate) fn same(change: Scene3DChange) -> Scene3DChange {
    change
}

/// The changes of a child of a camera (its transform): changes of the camera.
pub(crate) fn camera(_change: Scene3DChange) -> Scene3DChange {
    Scene3DChange::Camera
}

type Handler = Rc<dyn Fn(Scene3DChange)>;

/// The handlers of the changes of one object.
#[derive(Default)]
pub(crate) struct Notifier {
    handlers: RefCell<Vec<(u64, Handler)>>,
    next: Cell<u64>,
}

impl Notifier {
    pub(crate) fn new() -> Rc<Notifier> {
        Rc::new(Notifier::default())
    }

    /// Calls `handler` for every change until the returned guard is dropped.
    pub(crate) fn subscribe(self: &Rc<Self>, handler: Handler) -> Subscription {
        let id = self.next.get();
        self.next.set(id + 1);
        self.handlers.borrow_mut().push((id, handler));
        let weak = Rc::downgrade(self);
        Subscription::new(Disposable::create(move || {
            if let Some(notifier) = weak.upgrade() {
                notifier.handlers.borrow_mut().retain(|(other, _)| *other != id);
            }
        }))
    }

    /// Calls the handlers, in the order they subscribed. A handler may subscribe or unsubscribe.
    pub(crate) fn raise(&self, change: Scene3DChange) {
        let handlers: Vec<Handler> = self.handlers.borrow().iter().map(|(_, handler)| handler.clone()).collect();
        for handler in handlers {
            handler(change);
        }
    }

    /// Forwards the changes of this object to `owner`, mapped by `map`, until the returned guard
    /// is dropped. The owner is held weakly.
    pub(crate) fn forward_to(
        self: &Rc<Self>,
        owner: &Rc<Notifier>,
        map: fn(Scene3DChange) -> Scene3DChange,
    ) -> Subscription {
        let owner = Rc::downgrade(owner);
        self.subscribe(Rc::new(move |change: Scene3DChange| {
            if let Some(owner) = owner.upgrade() {
                owner.raise(map(change));
            }
        }))
    }
}

/// A scene object: it has a notifier. Implemented by the base classes (`Transform3D`,
/// `Rotation3D`, `Camera`, `Model3D`, `Geometry3D`, `Material`, `Visual3D`); the derived classes
/// reach it through them.
pub(crate) trait Notifying {
    fn notifier(&self) -> &Rc<Notifier>;
}

/// The subscription of an owner to the changes of one child (the value of a property).
#[derive(Default)]
pub(crate) struct Link(RefCell<Option<Subscription>>);

impl Link {
    /// Forwards the changes of `child` (none: nothing) to `owner`, mapped by `map`, and stops
    /// forwarding those of the previous child.
    pub(crate) fn set<T: ObjectType + Notifying>(
        &self,
        child: Option<&Ref<T>>,
        owner: &Rc<Notifier>,
        map: fn(Scene3DChange) -> Scene3DChange,
    ) {
        let subscription = child.map(|child| child.notifier().forward_to(owner, map));
        let previous = self.0.replace(subscription);
        drop(previous);
    }
}

type Items<T> = Rc<RefCell<Vec<(Ref<T>, Subscription)>>>;

/// The subscription of an owner to a collection of children (the value of a property): to the
/// changes of each child, and to the collection, whose additions and removals are changes of the
/// owner of the kind given (the structure for models, materials and visuals, a record for the
/// transforms of a group).
pub(crate) struct ListLink<T: ObjectType> {
    collection: RefCell<Option<Subscription>>,
    items: Items<T>,
}

impl<T: ObjectType> Default for ListLink<T> {
    fn default() -> Self {
        Self { collection: RefCell::new(None), items: Rc::new(RefCell::new(Vec::new())) }
    }
}

impl<T: ObjectType + Notifying> ListLink<T> {
    /// Follows `list` (none: nothing) for `owner`, an addition or a removal being a change of
    /// the kind `membership`, and stops following the previous list.
    pub(crate) fn set(&self, list: Option<&MediaCollection<Ref<T>>>, owner: &Rc<Notifier>, membership: Scene3DChange) {
        let previous = self.collection.replace(None);
        drop(previous);
        let previous_items = std::mem::take(&mut *self.items.borrow_mut());
        drop(previous_items);
        let Some(list) = list else { return };
        // A cleared list reports what it removed, so the children are let go one by one.
        list.set_reset_behavior(ferroui_base::media::ResetBehavior::Remove);
        let (items_added, owner_added) = (self.items.clone(), Rc::downgrade(owner));
        let (items_removed, owner_removed) = (self.items.clone(), Rc::downgrade(owner));
        let (items_reset, owner_reset) = (self.items.clone(), Rc::downgrade(owner));
        let subscription = list.for_each_item(
            move |_, item: &Ref<T>| {
                if let Some(owner) = owner_added.upgrade() {
                    let forwarded = item.notifier().forward_to(&owner, same);
                    items_added.borrow_mut().push((item.clone(), forwarded));
                    owner.raise(membership);
                }
            },
            move |_, item: &Ref<T>| {
                let position = items_removed.borrow().iter().position(|(other, _)| other.ptr_eq(item));
                if let Some(position) = position {
                    let (_, forwarded) = items_removed.borrow_mut().remove(position);
                    drop(forwarded);
                }
                if let Some(owner) = owner_removed.upgrade() {
                    owner.raise(membership);
                }
            },
            move || {
                let removed = std::mem::take(&mut *items_reset.borrow_mut());
                drop(removed);
                if let Some(owner) = owner_reset.upgrade() {
                    owner.raise(membership);
                }
            },
        );
        *self.collection.borrow_mut() = Some(subscription);
    }
}

/// The subscription of an owner to a collection of values (the positions of a mesh): every
/// change of the collection is a change of the owner's structure.
#[derive(Default)]
pub(crate) struct ValueListLink(RefCell<Option<Subscription>>);

impl ValueListLink {
    /// Calls `changed` on every change of `list` (none: nothing), and stops following the
    /// previous list.
    pub(crate) fn set<T: Clone + 'static>(&self, list: Option<&MediaCollection<T>>, changed: impl Fn() + 'static) {
        let subscription = list.map(|list| list.collection_changed(move |_| changed()));
        let previous = self.0.replace(subscription);
        drop(previous);
    }
}

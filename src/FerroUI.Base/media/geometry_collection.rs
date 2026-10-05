use crate::media::{Geometry, GeometryGroup, MediaCollection, ResetBehavior};
use crate::reactive::IDisposable;
use crate::{Ref, WeakRef};
use std::cell::RefCell;
use std::ops::Deref;
use std::rc::Rc;

type ChildSubscriptions = Rc<RefCell<Vec<(Ref<Geometry>, Rc<dyn IDisposable>)>>>;

/// A collection of [`Geometry`] objects that notifies its parent
/// [`GeometryGroup`] of changes.
///
/// Cloning the handle shares the collection; equality is by identity.
#[derive(Clone)]
pub struct GeometryCollection {
    list: MediaCollection<Ref<Geometry>>,
    parent: Rc<RefCell<Option<WeakRef<GeometryGroup>>>>,
}

impl PartialEq for GeometryCollection {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.list == other.list
    }
}

impl Default for GeometryCollection {
    fn default() -> Self {
        Self::new()
    }
}

impl Deref for GeometryCollection {
    type Target = MediaCollection<Ref<Geometry>>;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.list
    }
}

impl FromIterator<Ref<Geometry>> for GeometryCollection {
    fn from_iter<I: IntoIterator<Item = Ref<Geometry>>>(iter: I) -> Self {
        Self::from_items(iter)
    }
}

impl GeometryCollection {
    /// Creates an empty collection.
    pub fn new() -> Self {
        Self::from_items([])
    }

    /// Creates a collection holding `items`.
    pub fn from_items(items: impl IntoIterator<Item = Ref<Geometry>>) -> Self {
        let list = MediaCollection::from_items(items);
        list.set_reset_behavior(ResetBehavior::Remove);
        let parent: Rc<RefCell<Option<WeakRef<GeometryGroup>>>> = Rc::new(RefCell::new(None));
        let subscriptions: ChildSubscriptions = Rc::new(RefCell::new(Vec::new()));

        let invalidate_parent = {
            let parent = parent.clone();
            move || {
                let parent = parent.borrow().as_ref().and_then(WeakRef::upgrade);
                if let Some(parent) = parent {
                    parent.invalidate();
                }
            }
        };

        let subscriptions_added = subscriptions.clone();
        let (invalidate_added, invalidate_removed) = (invalidate_parent.clone(), invalidate_parent);
        // The subscription lives as long as the list.
        let _ = list.for_each_item(
            move |_, x: &Ref<Geometry>| {
                let child_changed = invalidate_added.clone();
                let subscription = x.changed(child_changed);
                subscriptions_added.borrow_mut().push((x.clone(), subscription));
                invalidate_added();
            },
            move |_, x: &Ref<Geometry>| {
                let position = subscriptions.borrow().iter().position(|(g, _)| g.ptr_eq(x));
                if let Some(position) = position {
                    let (_, subscription) = subscriptions.borrow_mut().remove(position);
                    subscription.dispose();
                }
                invalidate_removed();
            },
            || panic!("Collection reset is not supported."),
        );

        Self { list, parent }
    }

    /// The group that owns the collection.
    pub fn parent(&self) -> Option<Ref<GeometryGroup>> {
        self.parent.borrow().as_ref().and_then(WeakRef::upgrade)
    }

    /// Sets the group that owns the collection.
    pub fn set_parent(&self, value: Option<&Ref<GeometryGroup>>) {
        *self.parent.borrow_mut() = value.map(Ref::downgrade);
    }
}

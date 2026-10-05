use crate::{Control, ItemsControl};
use ferroui_base::{BoxedValue, Ref, WeakRef};
use std::any::TypeId;
use std::cell::Cell;

/// A key that locates previously recycled containers of the right kind for
/// an item.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RecycleKey {
    /// The default key, for an items control that supports a single
    /// container type.
    Default,
    /// A key identifying containers by a Rust type.
    Type(TypeId),
    /// A key allocated with [`RecycleKey::new_unique`] or chosen by the
    /// items control.
    Id(u64),
}

impl RecycleKey {
    /// Allocates a key that is different from every other key on the
    /// current thread.
    pub fn new_unique() -> Self {
        thread_local! {
            static NEXT: Cell<u64> = const { Cell::new(1 << 63) };
        }
        RecycleKey::Id(NEXT.with(|next| next.replace(next.get() + 1)))
    }

    /// The key identifying containers by the type `T`.
    pub fn of<T: 'static>() -> Self {
        RecycleKey::Type(TypeId::of::<T>())
    }
}

/// Generates containers for an [`ItemsControl`].
///
/// When creating a container for an item from a `VirtualizingPanel`, the
/// following process should be followed:
///
/// - [`needs_container`](Self::needs_container) should first be called to
///   determine whether the item needs a container. This method will return
///   true if the item should be wrapped in a container control, or false if
///   the item itself can be used as a container.
/// - If `needs_container` returns true then the
///   [`create_container`](Self::create_container) method should be called
///   to create a new container, passing the recycle key returned from
///   `needs_container`.
/// - If the panel supports recycling and the recycle key is non-null then
///   the recycle key should be recorded for the container (e.g. in an
///   attached property or the realized container list).
/// - [`prepare_item_container`](Self::prepare_item_container) method should
///   be called for the container.
/// - The container should then be added to the panel using
///   `VirtualizingPanel::add_internal_child`.
/// - Finally, [`item_container_prepared`](Self::item_container_prepared)
///   should be called.
///
/// NOTE: If `needs_container` in the first step above returns false then
/// the above steps should be carried out a single time: the first time the
/// item is displayed. Otherwise the steps should be carried out each time a
/// new container is realized for an item.
///
/// When unrealizing a container, the following process should be followed:
///
/// - If `needs_container` for the item returned false then the item cannot
///   be unrealized or recycled.
/// - Otherwise, [`clear_item_container`](Self::clear_item_container) should
///   be called for the container.
/// - If recycling is supported by the panel and the container then the
///   container should be added to a recycle pool keyed on the recycle key
///   returned from `needs_container`. It is assumed that recycled
///   containers will not be removed from the panel but instead hidden from
///   view using e.g. `container.set_is_visible(false)`.
/// - If recycling is not supported then the container should be removed
///   from the panel.
///
/// When recycling an unrealized container, the following process should be
/// followed:
///
/// - `needs_container` should be called to determine whether the item needs
///   a container, and if so, the recycle key.
/// - A container should be taken from the recycle pool keyed on the
///   returned recycle key.
/// - The container should be made visible.
/// - `prepare_item_container` method should be called for the container.
/// - `item_container_prepared` should be called.
///
/// NOTE: Although this class is similar to that found in other XAML
/// frameworks, here this class only concerns itself with generating and
/// clearing item containers; it does not maintain a record of the currently
/// realized containers, that responsibility is delegated to the items
/// panel.
pub struct ItemContainerGenerator {
    owner: WeakRef<ItemsControl>,
}

impl ItemContainerGenerator {
    pub(crate) fn new(owner: &ItemsControl) -> Self {
        Self { owner: owner.to_ref().downgrade() }
    }

    #[inline]
    fn owner(&self) -> Ref<ItemsControl> {
        self.owner.upgrade().expect("the items control of the generator has been released")
    }

    /// Determines whether the specified item needs to be wrapped in a
    /// container control.
    ///
    /// Returns true if the item needs a container; otherwise false if the
    /// item can itself be used as a container. The second value is a key
    /// that can be used to locate a previously recycled container of the
    /// correct type, or `None` if the item cannot be recycled.
    pub fn needs_container(&self, item: &Option<BoxedValue>, index: i32) -> (bool, Option<RecycleKey>) {
        self.owner().needs_container_override(item, index)
    }

    /// Creates a new container control.
    ///
    /// Before calling this method, [`needs_container`](Self::needs_container)
    /// should be called to determine whether the item itself should be used
    /// as a container. After calling this method,
    /// [`prepare_item_container`](Self::prepare_item_container) must be
    /// called to prepare the container to display the specified item.
    ///
    /// If the panel supports recycling then the returned recycle key should
    /// be stored alongside the container and when container becomes
    /// eligible for recycling the container should be placed in a recycle
    /// pool using this key. If the returned recycle key is `None` then the
    /// container cannot be recycled.
    pub fn create_container(
        &self,
        item: &Option<BoxedValue>,
        index: i32,
        recycle_key: Option<RecycleKey>,
    ) -> Ref<Control> {
        self.owner().create_container_for_item_override(item, index, recycle_key)
    }

    /// Prepares the specified element as the container for the
    /// corresponding item.
    ///
    /// If [`needs_container`](Self::needs_container) is false for an item,
    /// then this method must only be called a single time; otherwise this
    /// method must be called after the container is created, and each
    /// subsequent time the container is recycled to display a new item.
    pub fn prepare_item_container(&self, container: &Ref<Control>, item: &Option<BoxedValue>, index: i32) {
        self.owner().prepare_item_container(container, item, index)
    }

    /// Notifies the [`ItemsControl`] that a container has been fully
    /// prepared to display an item.
    ///
    /// This method must be called when a container has been fully prepared
    /// and added to the logical and visual trees, but may be called before
    /// a layout pass has completed. It must be called regardless of the
    /// result of [`needs_container`](Self::needs_container) but if that
    /// method returned false then must be called only a single time.
    pub fn item_container_prepared(&self, container: &Ref<Control>, item: &Option<BoxedValue>, index: i32) {
        self.owner().item_container_prepared(container, item, index)
    }

    /// Called when the index for a container changes due to an insertion or
    /// removal in the items collection.
    pub fn item_container_index_changed(&self, container: &Ref<Control>, old_index: i32, new_index: i32) {
        self.owner().item_container_index_changed(container, old_index, new_index)
    }

    /// Undoes the effects of the
    /// [`prepare_item_container`](Self::prepare_item_container) method.
    ///
    /// This method must be called when a container is unrealized. The
    /// container must have already have been removed from the virtualizing
    /// panel's list of realized containers before this method is called.
    /// This method must not be called if
    /// [`needs_container`](Self::needs_container) returned false for the
    /// item.
    pub fn clear_item_container(&self, container: &Ref<Control>) {
        self.owner().clear_item_container(container)
    }
}

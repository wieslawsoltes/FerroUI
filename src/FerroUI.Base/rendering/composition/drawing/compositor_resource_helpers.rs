use super::ICompositionRenderResource;
use crate::rendering::composition::server::ServerObjectId;
use crate::rendering::composition::{Compositor, ICompositorSerializable};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// A server-side resource shared by reference counting.
pub struct CompositorRefCountableResource {
    value: ServerObjectId,
    ref_count: i32,
}

impl CompositorRefCountableResource {
    pub fn new(value: ServerObjectId) -> Self {
        Self { value, ref_count: 1 }
    }

    pub fn value(&self) -> ServerObjectId {
        self.value
    }

    pub fn ref_count(&self) -> i32 {
        self.ref_count
    }

    pub fn add_ref(&mut self) {
        if self.ref_count <= 0 {
            panic!("This resource is disposed");
        }
        self.ref_count += 1;
    }

    /// Drops one reference; when it was the last one the server object is
    /// disposed with the next batch and `true` is returned.
    pub fn release(&mut self, c: &Compositor) -> bool {
        if self.ref_count <= 0 {
            panic!("This resource is disposed");
        }
        self.ref_count -= 1;
        if self.ref_count == 0 {
            c.dispose_on_next_batch(self.value);
            return true;
        }
        false
    }
}

struct Entry {
    compositor: Weak<Compositor>,
    resource: CompositorRefCountableResource,
}

/// Holds the server-side counterparts of a media object, one per compositor
/// the object is used with.
#[derive(Default)]
pub struct CompositorResourceHolder {
    dictionary: RefCell<Vec<Entry>>,
}

fn is_compositor(entry: &Entry, compositor: &Compositor) -> bool {
    std::ptr::eq(entry.compositor.as_ptr(), compositor)
}

impl CompositorResourceHolder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether the object is attached to any compositor.
    pub fn is_attached(&self) -> bool {
        !self.dictionary.borrow().is_empty()
    }

    /// Adds a reference to the counterpart on `compositor`, creating it
    /// with `factory` if there is none. Returns the counterpart and whether
    /// it was created; a newly created counterpart's `owner` is registered
    /// for serialization.
    pub fn create_or_add_ref(
        &self,
        compositor: &Rc<Compositor>,
        owner: Option<Rc<dyn ICompositorSerializable>>,
        factory: impl FnOnce(&Rc<Compositor>) -> ServerObjectId,
    ) -> (ServerObjectId, bool) {
        {
            let mut dictionary = self.dictionary.borrow_mut();
            if let Some(entry) = dictionary.iter_mut().find(|e| is_compositor(e, compositor)) {
                entry.resource.add_ref();
                return (entry.resource.value(), false);
            }
        }
        let resource = factory(compositor);
        self.dictionary
            .borrow_mut()
            .push(Entry { compositor: Rc::downgrade(compositor), resource: CompositorRefCountableResource::new(resource) });
        if let Some(owner) = owner {
            compositor.register_for_serialization(owner);
        }
        (resource, true)
    }

    pub fn try_get_for_compositor(&self, compositor: &Compositor) -> Option<ServerObjectId> {
        self.dictionary.borrow().iter().find(|e| is_compositor(e, compositor)).map(|e| e.resource.value())
    }

    pub fn get_for_compositor(&self, compositor: &Compositor) -> ServerObjectId {
        match self.try_get_for_compositor(compositor) {
            Some(value) => value,
            None => panic!("This resource doesn't exist on that compositor"),
        }
    }

    /// Drops one reference to the counterpart on `compositor`. Returns
    /// whether it was the last one.
    pub fn release(&self, compositor: &Compositor) -> bool {
        let mut dictionary = self.dictionary.borrow_mut();
        let Some(index) = dictionary.iter().position(|e| is_compositor(e, compositor)) else {
            panic!("This resource doesn't exist on that compositor");
        };
        if dictionary[index].resource.release(compositor) {
            dictionary.remove(index);
            return true;
        }
        false
    }

    fn compositors(&self) -> Vec<Rc<Compositor>> {
        self.dictionary.borrow().iter().filter_map(|e| e.compositor.upgrade()).collect()
    }

    /// Moves the references this object holds on a property value from the
    /// old value to the new one, on every compositor.
    pub fn process_property_change_notification(
        &self,
        old_value: Option<&dyn ICompositionRenderResource>,
        new_value: Option<&dyn ICompositionRenderResource>,
    ) {
        if let Some(old_resource) = old_value {
            self.transitive_release_all(old_resource);
        }
        if let Some(new_resource) = new_value {
            self.transitive_add_ref_all(new_resource);
        }
    }

    pub fn transitive_release_all(&self, old_resource: &dyn ICompositionRenderResource) {
        for compositor in self.compositors() {
            old_resource.release_on_compositor(&compositor);
        }
    }

    pub fn transitive_add_ref_all(&self, new_resource: &dyn ICompositionRenderResource) {
        for compositor in self.compositors() {
            new_resource.add_ref_on_compositor(&compositor);
        }
    }

    pub fn register_for_invalidation_on_all_compositors(&self, serializable: &Rc<dyn ICompositorSerializable>) {
        for compositor in self.compositors() {
            compositor.register_for_serialization(serializable.clone());
        }
    }
}

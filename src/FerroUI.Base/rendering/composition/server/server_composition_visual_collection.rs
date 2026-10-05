use super::{impl_animated_server_object, IAnimatedServerObject, IServerObject, ServerCompositionVisual, ServerCompositor, ServerObject};
use crate::rendering::composition::generated::{
    ServerCompositionVisualCollectionHooks, ServerCompositionVisualCollectionProps,
};
use crate::rendering::composition::transport::BatchStreamReader;
use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::rc::{Rc, Weak};
use std::time::Duration;

/// Server-side counterpart of the children collection of a visual.
pub struct ServerCompositionVisualCollection {
    object: ServerObject,
    props: ServerCompositionVisualCollectionProps,
    /// The items as visuals: what the tree walks iterate.
    typed: RefCell<Rc<Vec<Rc<ServerCompositionVisual>>>>,
}

impl ServerCompositionVisualCollection {
    pub fn new(compositor: &Rc<ServerCompositor>) -> Rc<ServerCompositionVisualCollection> {
        Rc::new_cyclic(|this: &Weak<ServerCompositionVisualCollection>| {
            let owner: Weak<dyn IAnimatedServerObject> = this.clone();
            ServerCompositionVisualCollection {
                object: ServerObject::new(compositor, owner),
                props: ServerCompositionVisualCollectionProps::new(),
                typed: RefCell::new(Rc::new(Vec::new())),
            }
        })
    }

    /// A snapshot of the items.
    pub fn list(&self) -> Rc<Vec<Rc<ServerCompositionVisual>>> {
        self.typed.borrow().clone()
    }

    pub fn count(&self) -> usize {
        self.typed.borrow().len()
    }
}

impl ServerCompositionVisualCollectionHooks for ServerCompositionVisualCollection {}

impl_animated_server_object!(ServerCompositionVisualCollection, object);

impl IServerObject for ServerCompositionVisualCollection {
    fn deserialize_changes_core(&self, reader: &mut BatchStreamReader<'_>, committed_at: Duration) {
        self.props.deserialize_changes_core(self, reader, committed_at);
        *self.typed.borrow_mut() = Rc::new(self.props.list().items::<ServerCompositionVisual>());
    }

    fn dispose(&self) {
        self.props.list().clear();
        *self.typed.borrow_mut() = Rc::new(Vec::new());
    }

    fn get_props(&self, type_id: TypeId) -> Option<&dyn Any> {
        self.props.find_props(type_id)
    }

    fn as_animated(self: Rc<Self>) -> Option<Rc<dyn IAnimatedServerObject>> {
        Some(self)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn into_any_rc(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }
}

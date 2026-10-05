use super::generated::{CompositionVisualCollectionHooks, CompositionVisualCollectionProps};
use super::server::{CompositionProperty, ServerObjectId};
use super::transport::{BatchStreamWriter, IRegisterForSerialization};
use super::{
    AsCompositionObject, CompositionObject, ICompositionObjectAnimations, CompositionVisual, Compositor, ICompositionObject, ICompositionObjectHost,
    ICompositorSerializable, PendingAnimations,
};
use crate::rendering::composition::animations::{ICompositionAnimation, ICompositionAnimationBase};
use crate::rendering::composition::expressions::ExpressionVariant;
use std::any::Any;
use std::rc::{Rc, Weak};

/// The children of a composition visual.
pub struct CompositionVisualCollection {
    this: Weak<CompositionVisualCollection>,
    object: CompositionObject,
    owner: Weak<CompositionVisual>,
    props: CompositionVisualCollectionProps<CompositionVisual>,
}

impl CompositionVisualCollection {
    pub(crate) fn new(
        compositor: &Rc<Compositor>,
        parent: Weak<CompositionVisual>,
        server: ServerObjectId,
    ) -> Rc<CompositionVisualCollection> {
        let collection = Rc::new_cyclic(|this: &Weak<CompositionVisualCollection>| CompositionVisualCollection {
            this: this.clone(),
            object: CompositionObject::new(compositor, Some(server)),
            owner: parent,
            props: CompositionVisualCollectionProps::new(),
        });
        collection.props.initialize_defaults(&*collection);
        collection
    }

    fn owner(&self) -> Option<Rc<CompositionVisual>> {
        self.owner.upgrade()
    }

    pub fn insert_above(&self, new_child: Rc<CompositionVisual>, sibling: &Rc<CompositionVisual>) {
        let Some(idx) = self.props.index_of(sibling) else { panic!("the sibling is not in the collection") };
        self.insert(idx + 1, new_child);
    }

    pub fn insert_below(&self, new_child: Rc<CompositionVisual>, sibling: &Rc<CompositionVisual>) {
        let Some(idx) = self.props.index_of(sibling) else { panic!("the sibling is not in the collection") };
        self.insert(idx, new_child);
    }

    pub fn insert_at_top(&self, new_child: Rc<CompositionVisual>) {
        self.insert(self.props.count(), new_child);
    }

    pub fn insert_at_bottom(&self, new_child: Rc<CompositionVisual>) {
        self.insert(0, new_child);
    }

    pub fn remove_all(&self) {
        self.clear();
    }

    // --- the list ----------------------------------------------------------------

    /// A snapshot of the items, for enumeration.
    pub fn items(&self) -> Vec<Rc<CompositionVisual>> {
        self.props.items()
    }

    pub fn add(&self, item: Rc<CompositionVisual>) {
        self.props.add(self, item);
    }

    pub fn clear(&self) {
        self.props.clear(self);
    }

    pub fn contains(&self, item: &Rc<CompositionVisual>) -> bool {
        self.props.contains(item)
    }

    pub fn remove(&self, item: &Rc<CompositionVisual>) -> bool {
        self.props.remove(self, item)
    }

    pub fn count(&self) -> usize {
        self.props.count()
    }

    pub fn is_read_only(&self) -> bool {
        self.props.is_read_only()
    }

    pub fn index_of(&self, item: &Rc<CompositionVisual>) -> Option<usize> {
        self.props.index_of(item)
    }

    pub fn insert(&self, index: usize, item: Rc<CompositionVisual>) {
        self.props.insert(self, index, item);
    }

    pub fn remove_at(&self, index: usize) {
        self.props.remove_at(self, index);
    }

    pub fn get(&self, index: usize) -> Rc<CompositionVisual> {
        self.props.get(index)
    }

    pub fn set(&self, index: usize, value: Rc<CompositionVisual>) {
        self.props.set(self, index, value);
    }
}

impl CompositionVisualCollectionHooks<CompositionVisual> for CompositionVisualCollection {
    fn on_added(&self, item: &Rc<CompositionVisual>) {
        let owner = self.owner();
        item.set_parent(owner.clone());
        if let Some(owner) = owner {
            owner.add_hit_test_child(item);
        }
    }

    fn on_before_replace(&self, old_item: &Rc<CompositionVisual>, new_item: &Rc<CompositionVisual>) {
        if !Rc::ptr_eq(old_item, new_item) {
            self.on_before_added(new_item);
        }
    }

    fn on_replace(&self, old_item: &Rc<CompositionVisual>, new_item: &Rc<CompositionVisual>) {
        if !Rc::ptr_eq(old_item, new_item) {
            self.on_removed(old_item);
            self.on_added(new_item);
        }
    }

    fn on_removed(&self, item: &Rc<CompositionVisual>) {
        item.set_parent(None);
        if let Some(owner) = self.owner() {
            owner.remove_hit_test_child(item);
        }
    }

    fn on_before_clear(&self) {
        for item in self.items() {
            item.set_parent(None);
        }
    }

    fn on_clear(&self) {
        if let Some(owner) = self.owner() {
            owner.clear_hit_test_children();
        }
    }

    fn on_before_added(&self, item: &Rc<CompositionVisual>) {
        if item.parent().is_some() {
            panic!("Visual already has a parent");
        }
        item.set_parent(self.owner());
    }
}

impl IRegisterForSerialization for CompositionVisualCollection {
    fn register_for_serialization(&self) {
        self.object
            .register_for_serialization(|| self.this.upgrade().map(|this| this as Rc<dyn ICompositorSerializable>));
    }
}

impl ICompositionObjectHost for CompositionVisualCollection {
    fn server(&self) -> ServerObjectId {
        self.object.required_server()
    }

    fn pending_animations(&self) -> &PendingAnimations {
        self.object.pending_animations()
    }

    fn implicit_animation(&self, property_name: &str) -> Option<Rc<dyn ICompositionAnimationBase>> {
        self.object.implicit_animation(property_name)
    }

    fn start_animation_group(
        &self,
        grp: &Rc<dyn ICompositionAnimationBase>,
        target: &str,
        final_value: ExpressionVariant,
    ) -> bool {
        self.start_animation_group_for(&**grp, target, final_value)
    }
}

/// The class has no animated property: upstream does not override
/// `StartAnimation`, whose base implementation rejects every property.
impl ICompositionObjectAnimations for CompositionVisualCollection {
    fn try_start_animation(
        &self,
        _property_name: &str,
        _animation: &dyn ICompositionAnimation,
        _final_value: Option<ExpressionVariant>,
    ) -> bool {
        false
    }

    fn get_composition_property(&self, property_name: &str) -> Option<&'static CompositionProperty> {
        let _ = property_name;
        None
    }

    fn composition_object(&self) -> &CompositionObject {
        &self.object
    }
}

impl AsCompositionObject for CompositionVisualCollection {
    fn as_composition_object(&self) -> &CompositionObject {
        &self.object
    }

    fn composition_type_name(&self) -> &'static str {
        "CompositionVisualCollection"
    }
}

impl ICompositionObject for CompositionVisualCollection {
    fn server(&self) -> ServerObjectId {
        self.object.required_server()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn into_any_rc(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }
}

impl ICompositorSerializable for CompositionVisualCollection {
    fn try_get_server(&self, c: &Compositor) -> Option<ServerObjectId> {
        self.object.try_get_server(c)
    }

    fn serialize_changes(&self, c: &Compositor, writer: &mut BatchStreamWriter<'_>) {
        self.object.begin_serialize_changes(c);
        self.props.serialize_changes_core(self, writer);
    }
}

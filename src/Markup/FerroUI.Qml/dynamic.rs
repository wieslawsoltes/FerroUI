//! Object identity and deferred component definitions live in a single engine.
use super::*;

#[derive(Clone)]
pub(crate) struct Template {
    pub object: Rc<ir::Object>,
    pub imports: Rc<[ir::Import]>,
    pub scopes: enums::Scopes,
}
#[derive(Default)]
pub(crate) struct Components {
    pub serial:u64,
    pub revision:u64,
    pub registry:Option<Rc<TypeRegistry>>,
    pub templates:BTreeMap<ObjectId,Template>,
}
impl Instance{
    pub(crate) fn next_object(&self)->ObjectId{
        ObjectId{instance:self.identity,index:self.nodes.len(),serial:self.dynamic.serial}
    }
    pub fn objects(&self)->impl Iterator<Item=ObjectId>+'_ { self.nodes.iter().map(|node|node.id) }
    pub fn structure_revision(&self)->u64 { self.dynamic.revision }
    pub fn is_component(&self,id:ObjectId)->bool{self.node(id).is_some() && self.dynamic.templates.contains_key(&id)}
}

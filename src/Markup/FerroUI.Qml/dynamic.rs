//! Object identity and deferred component definitions live in a single engine.
use super::*;

#[derive(Clone)]
pub(crate) struct Template {
    pub object: Rc<ir::Object>,
    pub imports: Rc<[ir::Import]>,
    pub scopes: enums::Scopes,
}
pub(crate) struct Context {
    pub declaration: ObjectId,
    pub ids: BTreeMap<String, ObjectId>,
}
#[derive(Default)]
pub(crate) struct Components {
    pub loaders: BTreeMap<ObjectId, crate::loader::State>,
    pub serial: u64,
    pub revision: u64,
    pub registry: Option<Rc<TypeRegistry>>,
    pub contexts: BTreeMap<ObjectId, Context>,
    pub created: Vec<ObjectId>,
    pub deadlines: BTreeMap<ObjectId, std::time::Duration>,
    pub catalog: BTreeMap<String, Template>,
    pub file_errors: BTreeMap<ObjectId, String>,
    pub loading: BTreeSet<ObjectId>,
    pub templates: BTreeMap<ObjectId, Template>,
}
impl Instance {
    pub(crate) fn next_object(&self) -> ObjectId {
        ObjectId {
            instance: self.identity,
            index: self.nodes.len(),
            serial: self.dynamic.serial,
        }
    }
    pub fn objects(&self) -> impl Iterator<Item = ObjectId> + '_ {
        self.nodes
            .iter()
            .filter(|node| node.live)
            .map(|node| node.id)
    }
    pub fn structure_revision(&self) -> u64 {
        self.dynamic.revision
    }
    pub fn is_component(&self, id: ObjectId) -> bool {
        self.node(id)
            .is_some_and(|n| n.module == "QtQml" && n.name == "Component")
    }
}
pub(crate) struct Checkpoint {
    pub(crate) loaded: BTreeSet<ObjectId>,
    pub(crate) removed: BTreeSet<ObjectId>,
    pub(crate) children: BTreeMap<ObjectId, Vec<ObjectId>>,
    pub(crate) deadlines: BTreeMap<ObjectId, Option<std::time::Duration>>,
    pub(crate) aliases: Vec<(SlotId, Alias)>,
    nodes: usize,
    slots: usize,
    revision: u64,
    created: usize,
}
impl Instance {
    pub(crate) fn context_root(&self, id: ObjectId) -> ObjectId {
        self.node(id)
            .and_then(|n| n.context)
            .unwrap_or_else(|| self.root())
    }
    pub(crate) fn insert_scoped_id(
        &mut self,
        context: Option<ObjectId>,
        name: String,
        id: ObjectId,
    ) -> Option<ObjectId> {
        match context {
            Some(owner) => self
                .dynamic
                .contexts
                .get_mut(&owner)
                .expect("creation context")
                .ids
                .insert(name, id),
            None => self.ids.insert(name, id),
        }
    }
    pub(crate) fn lexical_id(&self, owner: ObjectId, name: &str) -> Option<ObjectId> {
        let mut current = self.node(owner)?.context;
        while let Some(root) = current {
            let context = self.dynamic.contexts.get(&root)?;
            if let Some(id) = context.ids.get(name) {
                return Some(*id);
            }
            current = self.node(context.declaration)?.context;
        }
        self.ids.get(name).copied()
    }
    pub(crate) fn lexical_names(&self, owner: ObjectId, names: &mut BTreeSet<String>) {
        let mut current = self.node(owner).and_then(|n| n.context);
        while let Some(root) = current {
            let context = &self.dynamic.contexts[&root];
            names.extend(
                context
                    .ids
                    .keys()
                    .filter(|n| !n.starts_with("$qml:"))
                    .cloned(),
            );
            current = self.node(context.declaration).and_then(|n| n.context);
        }
        names.extend(self.ids.keys().filter(|n| !n.starts_with("$qml:")).cloned());
    }
    pub(crate) fn checkpoint(&self) -> Checkpoint {
        Checkpoint {
            loaded: Default::default(),
            removed: Default::default(),
            children: Default::default(),
            deadlines: Default::default(),
            aliases: vec![],
            nodes: self.nodes.len(),
            slots: self.slots.len(),
            revision: self.dynamic.revision,
            created: self.dynamic.created.len(),
        }
    }
    pub(crate) fn rollback_creation(&mut self, c: Checkpoint) {
        self.dynamic.loading.extend(c.loaded);
        for (id, old) in c.deadlines {
            match old {
                Some(when) => {
                    self.dynamic.deadlines.insert(id, when);
                }
                None => {
                    self.dynamic.deadlines.remove(&id);
                }
            }
        }
        for id in c.removed {
            self.nodes[id.index].live = true;
        }
        for (id, children) in c.children {
            self.nodes[id.index].children = children;
        }
        for (slot, alias) in c.aliases {
            self.properties[slot].alias = Some(alias);
        }
        if self.nodes.len() == c.nodes && self.dynamic.revision == c.revision {
            return;
        }
        for i in c.nodes..self.nodes.len() {
            if let Some(parent) = self.nodes[i].parent.filter(|p| p.index < c.nodes) {
                self.nodes[parent.index]
                    .children
                    .retain(|id| id.index < c.nodes);
            }
        }
        if self.nodes.len() > c.nodes {
            if let Some(engine) = &self.javascript.engine {
                let abandoned = self.nodes[c.nodes..].iter().map(|node| node.id).collect();
                engine.release_objects(self.script_instance(), &abandoned);
            }
        }
        self.nodes.truncate(c.nodes);
        self.properties.truncate(c.slots);
        self.slots.truncate(c.slots);
        self.dynamic.loading.retain(|id| id.index < c.nodes);
        self.dynamic.file_errors.retain(|id, _| id.index < c.nodes);
        self.dynamic.templates.retain(|id, _| id.index < c.nodes);
        self.dynamic.contexts.retain(|id, _| id.index < c.nodes);
        self.dynamic.created.truncate(c.created);
        self.dynamic.revision = c.revision;
        self.dynamic.loaders.retain(|id, _| id.index < c.nodes);
        self.models.retain(|id, _| id.index < c.nodes);
        self.bindings.retain(|id| id.index < c.nodes);
        self.timers.truncate(c.nodes);
        self.workers.truncate(c.nodes);
        self.listeners.retain(|_, handlers| {
            handlers.retain(|h| h.receiver.index < c.nodes);
            !handlers.is_empty()
        });
    }
    pub fn drain_created(&mut self) -> Vec<ObjectId> {
        std::mem::take(&mut self.dynamic.created)
    }
    pub fn create_object(
        &mut self,
        component: ObjectId,
        parent: Option<ObjectId>,
        initial: BTreeMap<String, Value>,
    ) -> Result<ObjectId, Error> {
        self.transaction(|tx| tx.create_object(component, parent, initial))
    }
}
impl crate::transaction::Transaction<'_> {
    pub(crate) fn create_object(
        &mut self,
        component: ObjectId,
        parent: Option<ObjectId>,
        initial: BTreeMap<String, Value>,
    ) -> Result<ObjectId, Error> {
        self.create_with_properties(
            component,
            parent,
            initial
                .into_iter()
                .map(|(k, v)| (k, InitialProperty::Value(v)))
                .collect(),
        )
    }
    pub(crate) fn create_with_properties(
        &mut self,
        component: ObjectId,
        parent: Option<ObjectId>,
        properties: BTreeMap<String, InitialProperty>,
    ) -> Result<ObjectId, Error> {
        let template = self
            .model
            .dynamic
            .templates
            .get(&component)
            .ok_or_else(|| Error::new("not a ready Component"))?
            .clone();
        if self.model.get(component, "status")? != Value::Number(1.) {
            return Err(Error::new("not a ready Component"));
        }
        self.build_object(component, parent, properties, &template)
    }
    pub(crate) fn try_create_with_properties(
        &mut self,
        component: ObjectId,
        parent: Option<ObjectId>,
        properties: BTreeMap<String, InitialProperty>,
    ) -> Result<Option<ObjectId>, Error> {
        let rev = self.model.structure_revision();
        match self.create_with_properties(component, parent, properties) {
            Ok(id) => Ok(Some(id)),
            Err(_) if self.model.structure_revision() == rev => Ok(None),
            Err(err) => Err(err),
        }
    }
    pub(crate) fn build_object(
        &mut self,
        declaration: ObjectId,
        parent: Option<ObjectId>,
        initial: BTreeMap<String, InitialProperty>,
        template: &Template,
    ) -> Result<ObjectId, Error> {
        if self.in_binding() {
            return Err(Error::new("a binding cannot create objects"));
        }
        if parent.is_some_and(|id| self.model.node(id).is_none()) {
            return Err(Error::new("invalid component parent"));
        }
        let before = self.model.checkpoint();
        let registry = self.model.dynamic.registry.clone().expect("type registry");
        let root = self.model.next_object();
        let first = self.model.slots.len();
        self.model.dynamic.revision = self
            .model
            .dynamic
            .revision
            .checked_add(1)
            .ok_or_else(|| Error::new("tree revision exhausted"))?;
        self.model.dynamic.contexts.insert(
            root,
            Context {
                declaration,
                ids: Default::default(),
            },
        );
        let prepared = self
            .model
            .allocate_template(root, parent, template, &registry)
            .and_then(|assigned| self.apply_initial(root, first, assigned, initial));
        if let Err(error) = prepared {
            self.model.rollback_creation(before);
            return Err(error);
        }
        if let Some(parent) = parent {
            if self.model.nodes[root.index].visual_children.is_some() {
                if let Some(name) = self.model.nodes[parent.index].visual_children.clone() {
                    let key = self.model.key(parent, &name)?;
                    let Value::List(mut children) = self.read(key, &mut BTreeSet::new())? else {
                        return Err(Error::new("visual children must be a list"));
                    };
                    children.push(Value::Object(root));
                    self.write(key, Value::List(children))?;
                }
            }
        }
        self.model.dynamic.created.push(root);
        self.initialize_range(root.index, first)?;
        Ok(root)
    }
    pub(crate) fn create_method(
        &mut self,
        component: ObjectId,
        args: &[Value],
    ) -> Result<Value, Error> {
        let parent = match args.first() {
            None | Some(Value::Undefined | Value::Null) => None,
            Some(Value::Object(p)) => Some(*p),
            _ => return Err(Error::new("invalid component parent")),
        };
        let initial = match args.get(1) {
            None | Some(Value::Undefined) => BTreeMap::new(),
            Some(Value::Record(r)) if !r.is_array() => self.model.record_entries(r)?.clone(),
            _ => return Err(Error::new("invalid initial property map")),
        };
        if self.model.get(component, "status")? != Value::Number(1.) {
            return Ok(Value::Null);
        }
        if args.len() > 2 {
            return Err(Error::new(
                "createObject expects a parent and initial property map",
            ));
        }
        self.create_object(component, parent, initial)
            .map(Value::Object)
    }
}

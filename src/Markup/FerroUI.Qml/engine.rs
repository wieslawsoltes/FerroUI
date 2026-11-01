//! Independent document roots in one owner-thread object graph and JavaScript realm.
use super::*;

pub(crate) struct Unit {
    pub functions: &'static [compiled::Evaluator],
    pub scripts: &'static [compiled::Script],
    pub first_reads: &'static [compiled::FirstRead],
    catalog: BTreeMap<String, dynamic::Template>,
}

#[derive(Default)]
pub(crate) struct Roots {
    pub units: BTreeMap<ObjectId, Rc<Unit>>,
    pub singletons: BTreeMap<u32, ObjectId>,
    documents: BTreeSet<ObjectId>,
    types: BTreeMap<String, u32>,
}

impl Instance {
    #[cfg(feature = "diagnostics")]
    pub(crate) fn declaration_key(&self, scope: u32) -> Option<&str> {
        self.debug_type_keys
            .get(scope as usize)
            .map(String::as_str)
            .or_else(|| {
                self.roots
                    .types
                    .iter()
                    .find(|(_, index)| **index == scope)
                    .map(|(key, _)| key.as_str())
            })
    }
    pub(crate) fn unit(&self, owner: ObjectId) -> Option<&Unit> {
        let document = self.node(owner)?.document?;
        self.roots.units.get(&document).map(Rc::as_ref)
    }

    pub(crate) fn document_root(&self, owner: ObjectId) -> ObjectId {
        self.node(owner)
            .and_then(|n| n.document)
            .unwrap_or_else(|| self.root())
    }

    pub(crate) fn component_catalog(
        &self,
        owner: ObjectId,
    ) -> &BTreeMap<String, dynamic::Template> {
        self.unit(owner)
            .map_or(&self.dynamic.catalog, |unit| &unit.catalog)
    }
}

/// Loads independent documents into one engine. Object handles, singleton instances,
/// signal delivery and property dependencies belong to this engine.
pub struct Engine {
    model: Instance,
}

impl Engine {
    pub fn new(registry: &TypeRegistry) -> Result<Self, Error> {
        Self::create(registry, None)
    }
    pub fn with_script_engine(
        registry: &TypeRegistry,
        engine: Rc<dyn ScriptEngine>,
    ) -> Result<Self, Error> {
        Self::create(registry, Some(engine))
    }
    fn create(
        registry: &TypeRegistry,
        engine: Option<Rc<dyn ScriptEngine>>,
    ) -> Result<Self, Error> {
        // Host continuations need a receiver even when no application root is loaded.
        let shell = ir::parse("import QtQml\nQtObject {}").expect("engine shell");
        let mut model = Instance::new(&shell, registry, BTreeMap::new())?;
        model.javascript.engine = engine;
        Ok(Self { model })
    }

    /// Runtime APIs for object handles from any loaded document.
    pub fn instance(&self) -> &Instance {
        &self.model
    }
    pub fn instance_mut(&mut self) -> &mut Instance {
        &mut self.model
    }

    pub fn roots(&self) -> impl Iterator<Item = ObjectId> + '_ {
        self.model.roots.documents.iter().copied()
    }

    /// A public lexical ID from this document. Other documents have independent IDs.
    pub fn id(&self, root: ObjectId, name: &str) -> Option<ObjectId> {
        if name.starts_with("$qml:") || !self.model.roots.documents.contains(&root) {
            return None;
        }
        self.model
            .lexical_id(root, name)
            .filter(|id| self.model.node(*id).is_some())
    }

    pub fn load_root(
        &mut self,
        source: &impl ComponentSource,
        initial: BTreeMap<String, Value>,
    ) -> Result<ObjectId, Error> {
        let first_object = self.model.nodes.len();
        let first_type = self.model.roots.types.len();
        let resources = self.model.javascript.resources.clone();
        let result = self.model.transaction(|tx| {
            let registry = tx.model.dynamic.registry.clone().expect("type registry");
            let mut doc = source.document().clone();
            registry.validate_imports(&doc.imports)?;
            if let Some(p) = doc.pragmas.first() {
                return Err(Error::new(format!(
                    "unsupported document pragma '{}'",
                    p.name
                )));
            }
            let root = tx.model.next_object();
            let first = tx.model.slots.len();
            // Linker ordinals belong to one document. Intern their stable type keys
            // before objects, method signatures or property shadows enter shared storage.
            let mut intern = |key| {
                let index = tx.model.roots.types.len() as u32;
                *tx.model.roots.types.entry(key).or_insert(index)
            };
            let mapped = std::mem::take(&mut doc.type_keys)
                .into_iter()
                .map(&mut intern)
                .collect::<Vec<_>>();
            let mut scope = |n: u32| {
                mapped
                    .get(n as usize)
                    .copied()
                    .unwrap_or_else(|| intern(format!("$unit:{}:{n}", root.serial)))
            };
            for object in std::iter::once(&mut doc.root)
                .chain(&mut doc.singletons)
                .chain(doc.components.values_mut())
            {
                relocate_object(object, &mut scope);
            }
            let catalog = doc
                .components
                .iter()
                .map(|(url, shape)| {
                    Ok((
                        url.clone(),
                        dynamic::Template::from_object(shape.clone(), &doc.imports)?,
                    ))
                })
                .collect::<Result<_, Error>>()?;
            let unit = Rc::new(Unit {
                functions: source.evaluators(),
                scripts: source.scripts(),
                first_reads: source.first_reads(),
                catalog,
            });
            tx.model.dynamic.revision = tx
                .model
                .dynamic
                .revision
                .checked_add(1)
                .ok_or_else(|| Error::new("tree revision exhausted"))?;
            let assigned = tx.model.allocate_document(&doc, &registry, unit)?;
            let initial = initial
                .into_iter()
                .map(|(k, v)| (k, InitialProperty::Value(v)))
                .collect();
            tx.apply_initial(root, first, assigned, initial)?;
            if !doc.scripts.is_empty() {
                let all = std::sync::Arc::make_mut(&mut tx.model.javascript.resources);
                for (url, mut resource) in doc.scripts {
                    if let Some(old) = all.get(&url) {
                        let preload = resource.preload || old.preload;
                        resource.preload = old.preload;
                        if *old != resource {
                            return Err(Error::new(format!("conflicting script resource '{url}'")));
                        }
                        resource.preload = preload;
                    }
                    all.insert(url, resource);
                }
                let guest = tx.script_engine()?;
                tx.with_receiver_context(root, &mut BTreeSet::new(), |ctx| {
                    guest.prepare_script_resources(ctx)
                })?;
            }
            tx.initialize_range(root.index, first)?;
            tx.model.roots.documents.insert(root);
            Ok(root)
        });
        if result.is_err() {
            self.model
                .roots
                .units
                .retain(|id, _| id.index < first_object);
            self.model
                .roots
                .singletons
                .retain(|_, id| id.index < first_object);
            self.model
                .roots
                .documents
                .retain(|id| id.index < first_object);
            self.model
                .roots
                .types
                .retain(|_, index| (*index as usize) < first_type);
            if self.model.javascript.resources != resources {
                self.model.javascript.resources = resources;
                let shell = self.model.root();
                let _ = self.model.transaction(|tx| {
                    let guest = tx.script_engine()?;
                    tx.with_receiver_context(shell, &mut BTreeSet::new(), |ctx| {
                        guest.prepare_script_resources(ctx)
                    })
                });
            }
        }
        result
    }

    /// Run destruction handlers and retire this document and its dynamic objects.
    /// Engine singletons stay live across unloads, including when no roots remain.
    pub fn unload_root(&mut self, root: ObjectId) -> Result<(), Error> {
        if !self.model.roots.documents.contains(&root) {
            return Err(Error::new("unknown engine document root"));
        }
        self.model.transaction(|tx| tx.destroy_documents(&[root]))?;
        self.model.roots.documents.remove(&root);
        self.model.roots.units.remove(&root);
        Ok(())
    }
    /// Destroy application objects and singletons, then start a fresh realm with the same registry.
    /// An error in a destruction handler rolls back the teardown.
    pub fn reset(&mut self) -> Result<(), Error> {
        let registry = self.model.dynamic.registry.as_ref().expect("type registry");
        let next = Self::create(registry, self.model.javascript.engine.clone())?;
        let documents = self.roots().collect::<Vec<_>>();
        let singletons = self
            .model
            .roots
            .singletons
            .values()
            .copied()
            .collect::<Vec<_>>();
        self.model.transaction(|tx| {
            tx.destroy_documents(&documents)?;
            tx.destroy_documents(&singletons)
        })?;
        #[cfg(feature = "diagnostics")]
        let next = {
            let mut next = next;
            next.model.diagnostics.clone_from(&self.model.diagnostics);
            next
        };
        *self = next;
        Ok(())
    }
}

impl crate::transaction::Transaction<'_> {
    fn destroy_documents(&mut self, documents: &[ObjectId]) -> Result<(), Error> {
        let owned = documents.iter().copied().collect::<BTreeSet<_>>();
        loop {
            // A factory can create a parentless object or give it a parent in
            // another document. Destruction handlers may create more objects.
            let roots = self
                .model
                .objects()
                .filter(|&id| {
                    let node = &self.model.nodes[id.index];
                    node.document.is_some_and(|d| owned.contains(&d))
                        && !node
                            .parent
                            .and_then(|p| self.model.node(p)?.document)
                            .is_some_and(|d| owned.contains(&d))
                })
                .collect::<Vec<_>>();
            if roots.is_empty() {
                return Ok(());
            }
            self.destroy_subtrees(&roots)?;
        }
    }
}

fn relocate_object(object: &mut ir::Object, scope: &mut impl FnMut(u32) -> u32) {
    for ty in &mut object.composite_types {
        relocate_type(ty, scope);
    }
    for shadow in &mut object.shadows {
        shadow.scope = scope(shadow.scope);
    }
    for layer in &mut object.source_layers {
        layer.scope = layer.scope.map(&mut *scope);
    }
    relocate_members(&mut object.members, scope);
}

fn relocate_type(ty: &mut String, scope: &mut impl FnMut(u32) -> u32) {
    let list = ty.strip_prefix("list<").and_then(|s| s.strip_suffix('>'));
    if let Some(id) = list
        .unwrap_or(ty)
        .strip_prefix("Qml$Type")
        .and_then(|n| n.parse().ok())
    {
        let id = scope(id);
        *ty = if list.is_some() {
            format!("list<Qml$Type{id}>")
        } else {
            format!("Qml$Type{id}")
        };
    }
}

fn relocate_members(members: &mut [ir::Member], scope: &mut impl FnMut(u32) -> u32) {
    use ir::MemberKind::*;
    for member in members {
        member.scope = member.scope.map(&mut *scope);
        match &mut member.kind {
            Property { ty, value, .. } => {
                relocate_type(ty, scope);
                if let Some(value) = value {
                    relocate_binding(value, scope);
                }
            }
            Binding { value, .. } => relocate_binding(value, scope),
            Child(child) | OnProperty { object: child, .. } | Component { object: child, .. } => {
                relocate_object(child, scope)
            }
            Group { members, .. } => relocate_members(members, scope),
            Signal { parameters, .. } | Function { parameters, .. } => {
                for ty in parameters.iter_mut().filter_map(|p| p.ty.as_mut()) {
                    relocate_type(ty, scope);
                }
                if let Function {
                    return_type: Some(ty),
                    ..
                } = &mut member.kind
                {
                    relocate_type(ty, scope);
                }
            }
            Id(_) | Enum { .. } | Handler { .. } => (),
        }
    }
}
fn relocate_binding(value: &mut Binding, scope: &mut impl FnMut(u32) -> u32) {
    match value {
        Binding::Object(child) => relocate_object(child, scope),
        Binding::Objects(children) => {
            for child in children {
                relocate_object(child, scope);
            }
        }
        _ => (),
    }
}

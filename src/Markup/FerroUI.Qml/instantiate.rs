use super::*;
use ferroui_qml_compiler::{Expr, Import, Member, MemberKind, Object};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_INSTANCE: AtomicU64 = AtomicU64::new(1);

struct Builder<'a, I = Box<Instance>> {
    prepared: incubation::PreparedNodes,
    context: Option<ObjectId>,
    scopes: enums::Scopes,
    instance: I,
    registry: &'a TypeRegistry,
    imports: &'a [Import],
    assigned: BTreeSet<SlotId>,
    aliases: Vec<(SlotId, Expr, Option<u32>)>,
    implicit: Vec<(SlotId, Vec<Value>)>,
    binding_targets: Vec<(SlotId, String)>,
    read_candidates: Vec<SlotId>,
}

impl Slot {
    pub(super) fn initial(value: Value) -> Self {
        Self {
            scope: None,
            delegate_local: false,
            overlay: None,
            active_target: None,
            assigned: false,
            value,
            binding: None,
            dirty: false,
            reads: BTreeSet::new(),
            dependents: BTreeSet::new(),
        }
    }
}

impl Instance {
    pub(crate) fn allocate_property(
        &mut self,
        property: Property,
        value: Value,
    ) -> Result<SlotId, Error> {
        let key = self.properties.len();
        let node = &mut self.nodes[property.owner.index];
        if node.properties.insert(property.name.clone(), key).is_some() {
            return Err(Error::new(format!(
                "duplicate property '{}'",
                property.name
            )));
        }
        self.properties.push(property);
        self.slots.push(Slot::initial(value));
        Ok(key)
    }

    /// Allocates the complete object and id table before evaluating any binding.
    /// Required root properties can be supplied by the Rust caller.
    pub fn new(
        source: &impl ComponentSource,
        registry: &TypeRegistry,
        initial: BTreeMap<String, Value>,
    ) -> Result<Self, Error> {
        Self::construct(None, source, registry, initial, None)
    }

    /// Load an in-memory document with the type name normally supplied by its
    /// filename, so its own enumerations can be accessed from expressions.
    pub fn new_named(
        name: &str,
        source: &impl ComponentSource,
        registry: &TypeRegistry,
        initial: BTreeMap<String, Value>,
    ) -> Result<Self, Error> {
        Self::construct(Some(name), source, registry, initial, None)
    }

    /// Attach a JavaScript runtime before initial bindings and completion handlers execute.
    pub fn with_engine(
        source: &impl ComponentSource,
        registry: &TypeRegistry,
        initial: BTreeMap<String, Value>,
        engine: Rc<dyn ScriptEngine>,
    ) -> Result<Self, Error> {
        Self::construct(None, source, registry, initial, Some(engine))
    }

    fn construct(
        name: Option<&str>,
        source: &impl ComponentSource,
        registry: &TypeRegistry,
        initial: BTreeMap<String, Value>,
        engine: Option<Rc<dyn ScriptEngine>>,
    ) -> Result<Self, Error> {
        let mut builder = Builder::prepare(name, source, registry)?;
        if engine.is_some() {
            builder.instance.javascript.resources = source.document().scripts.clone().into();
        }
        builder.instance.javascript.engine = engine;
        let root = builder.instance.root();
        for (name, value) in initial {
            let key = builder.instance.public_key(root, &name)?;
            let key = builder.instance.write_target(key)?;
            builder.instance.slots[key].value = builder
                .instance
                .typed(&builder.instance.properties[key].ty, value)?;
            builder.instance.slots[key].binding = None;
            builder.instance.slots[key].dirty = false;
            builder.assigned.insert(key);
        }
        for (key, property) in builder.instance.properties.iter().enumerate() {
            if property.required && !builder.assigned.contains(&key) {
                return Err(Error::new(format!(
                    "required property '{}' was not initialized",
                    property.name
                )));
            }
        }
        for key in builder.assigned {
            builder.instance.slots[key].assigned = true;
        }
        let mut instance = *builder.instance;
        // A module initializer can have side effects or fail. Keep the normal
        // first evaluation when static imports still need the guest engine.
        let preload = source.document().scripts.values().any(|r| r.preload);
        if !preload {
            instance.specialize_reads(&builder.read_candidates);
        }
        instance.transaction(|tx| tx.initialize())?;
        if preload {
            instance.specialize_reads(&builder.read_candidates);
        }
        instance.changes.clear();
        Ok(instance)
    }

    fn specialize_reads(&mut self, candidates: &[SlotId]) {
        for &key in candidates {
            let Some(InstalledBinding::Expression(binding)) = &self.slots[key].binding else {
                continue;
            };
            let Binding::Expr(Expr::JavaScript(script)) = binding.as_ref() else {
                continue;
            };
            if let Some(source) = self.scalar_read(key, script) {
                self.slots[key].binding = Some(InstalledBinding::PropertyRead(source));
            }
        }
    }

    fn scalar_read(&self, key: SlotId, script: &ir::JavaScript) -> Option<SlotId> {
        let read = script.property_read.as_ref()?;
        let owner = self.properties[key].owner;
        let name = read.property.as_str();
        let id = |name: &str| {
            if let Some(local) = script.ids.get(name) {
                self.ids.get(local).copied()
            } else if script.root.is_none() {
                self.id(name)
            } else {
                None
            }
        };
        let receiver = match read.receiver.as_deref() {
            Some("this") => owner,
            Some("parent") => self.parent(owner)?,
            Some(name) if !script.imports.contains_key(name) => id(name)?,
            None if id(name).is_none()
                && self.id(name).is_none()
                && !script.imports.contains_key(name)
                && !matches!(
                    name,
                    "this" | "parent" | "undefined" | "globalThis" | "eval" | "console"
                ) =>
            {
                owner
            }
            _ => return None,
        };
        if self.script_method(receiver, name) {
            return None;
        }
        let source = self.key_in(receiver, name, self.slots[key].scope).ok()?;
        matches!(
            self.properties[source].ty.as_str(),
            "bool" | "int" | "real" | "double" | "float" | "string" | "color" | "url"
        )
        .then_some(source)
    }

    pub(crate) fn write_target(&self, mut key: SlotId) -> Result<SlotId, Error> {
        loop {
            let property = &self.properties[key];
            if self.node(property.owner).is_none() {
                return Err(Error::new("property belongs to a destroyed object"));
            }
            if property.readonly {
                return Err(Error::new(format!(
                    "property '{}' is readonly",
                    property.name
                )));
            }
            match property.alias {
                Some(Alias::Slot(target)) => key = target,
                Some(Alias::Object(_)) => {
                    return Err(Error::new("an alias to an object is readonly"))
                }
                None => return Ok(key),
            }
        }
    }

    pub(crate) fn validate_handles(&self, value: &Value) -> Result<(), Error> {
        match value {
            Value::External(value) if value.instance() != self.script_instance() => {
                Err(Error::new("foreign JavaScript value"))
            }
            Value::ModelRow(row) => self.model_role(*row, "").map(|_| ()),
            Value::DelegateRow(owner) => self.delegate_row(*owner).map(|_| ()),
            Value::AnchorGroup(id) | Value::PropertyGroup(id, _) | Value::AnchorLine(id, _)
                if self.node(*id).is_none() =>
            {
                Err(Error::new("stale anchor target"))
            }
            Value::Method(method) if self.node(method.receiver()).is_none() => {
                Err(Error::new("foreign method receiver"))
            }
            Value::Object(id) if self.node(*id).is_none() => {
                Err(Error::new("foreign object handle"))
            }
            Value::Record(record) => self.record_entries(record).map(|_| ()),
            Value::List(values) => values.iter().try_for_each(|v| self.validate_handles(v)),
            _ => Ok(()),
        }
    }

    pub(crate) fn typed(&self, ty: &str, value: Value) -> Result<Value, Error> {
        let value = if let Some(shape) = self.geometry_snapshot(&value)? {
            Value::Geometry(shape)
        } else {
            value
        };
        match (ty, &value) {
            (ty, Value::Geometry(shape)) if shape.type_name() == ty => (),
            (ty, Value::String(text)) if Geometry::default_for(ty).is_some() => {
                return Geometry::parse(ty, text).map(Value::Geometry)
            }
            ("var" | "variant" | "alias", _) => {
                self.validate_handles(&value)?;
            }
            ("$qml:anchorLine", Value::AnchorLine(_, _) | Value::Undefined | Value::Null) => {
                self.validate_handles(&value)?
            }
            ("bool", Value::Bool(_))
            | ("real" | "double" | "float", Value::Number(_))
            | ("string" | "url" | "color", Value::String(_)) => (),
            ("int", Value::Number(n))
                if n.is_finite() && (i32::MIN as f64..=i32::MAX as f64).contains(&n.trunc()) =>
            {
                return Ok(Value::Number(n.trunc()))
            }
            (list, Value::Record(r)) if list_element(list).is_some() && r.is_array() => {
                return self.typed(list, Value::List(self.array_values(r)?));
            }
            (list, Value::List(items)) if list_element(list).is_some() => {
                let element = list_element(list).unwrap();
                return items
                    .iter()
                    .cloned()
                    .map(|v| self.typed(element, v))
                    .collect::<Result<Vec<_>, _>>()
                    .map(Value::List);
            }
            (object, Value::Null) if is_object_type(object) => (),
            (ty, Value::Object(id))
                if self.node(*id).is_some_and(|node| {
                    matches!(ty, "QtObject" | "QtQml.QtObject")
                        || node.composite_types.iter().any(|candidate| ty == candidate)
                        || ty == node.name
                        || ty == format!("{}.{}", node.module, node.name)
                        || node
                            .lineage
                            .iter()
                            .any(|(module, name)| ty == name || ty == format!("{module}.{name}"))
                }) => {}
            _ => return Err(Error::new(format!("cannot assign {value:?} to '{ty}'"))),
        }
        Ok(value)
    }
}

fn list_element(ty: &str) -> Option<&str> {
    ty.strip_prefix("list<")?.strip_suffix('>')
}
fn is_object_type(ty: &str) -> bool {
    ty.rsplit('.')
        .next()
        .and_then(|part| part.chars().next())
        .is_some_and(char::is_uppercase)
}

impl TypeRegistry {
    /// Check native types, member names, handlers, aliases, object-valued properties
    /// and required child properties without executing a binding, method or callback.
    /// Required properties on the root can be supplied by the eventual host.
    pub fn check(&self, document: &Document) -> Result<(), Error> {
        let mut builder = Builder::prepare(None, document, self)?;
        let root = builder.instance.root();
        verify_required(&builder.instance, 0, root, &builder.assigned)?;
        let mut pending: Vec<_> = builder
            .instance
            .dynamic
            .templates
            .iter()
            .map(|(&owner, template)| (owner, template.clone()))
            .collect();
        pending.extend(
            builder
                .instance
                .dynamic
                .catalog
                .values()
                .cloned()
                .map(|template| (root, template)),
        );
        while let Some((declaration, template)) = pending.pop() {
            let id = builder.instance.next_object();
            let first = builder.instance.slots.len();
            builder.instance.dynamic.contexts.insert(
                id,
                dynamic::Context {
                    row_revision: None,
                    delegate_inputs: None,
                    declaration,
                    ids: Default::default(),
                },
            );
            let supplied = builder
                .instance
                .allocate_template(id, None, &template, self)?;
            verify_required(&builder.instance, first, id, &supplied)?;
            pending.extend(
                builder
                    .instance
                    .dynamic
                    .templates
                    .range(id..)
                    .map(|(&id, template)| (id, template.clone())),
            );
        }
        Ok(())
    }
}

fn verify_required(
    instance: &Instance,
    first: usize,
    root: ObjectId,
    assigned: &BTreeSet<SlotId>,
) -> Result<(), Error> {
    for (offset, property) in instance.properties[first..].iter().enumerate() {
        if property.required && property.owner != root && !assigned.contains(&(first + offset)) {
            return Err(Error::new(format!(
                "required property '{}' was not initialized",
                property.name
            )));
        }
    }
    Ok(())
}

impl<'a> Builder<'a> {
    fn prepare(
        name: Option<&str>,
        source: &'a impl ComponentSource,
        registry: &'a TypeRegistry,
    ) -> Result<Self, Error> {
        let document = source.document();
        registry.validate_imports(&document.imports)?;
        if let Some(pragma) = document.pragmas.first() {
            return Err(Error::new(format!(
                "unsupported document pragma '{}'",
                pragma.name
            )));
        }
        let mut builder = Self {
            prepared: Default::default(),
            context: None,
            scopes: enums::Scopes::named(name, &document.root)?,
            instance: Box::new(Instance {
                roots: Default::default(),
                dynamic: dynamic::Components {
                    registry: Some(Rc::new(registry.clone())),
                    ..Default::default()
                },
                javascript: Default::default(),
                records: Default::default(),
                models: BTreeMap::new(),
                bindings: BTreeMap::new(),
                native_functions: source.evaluators(),
                native_first_reads: source.first_reads(),
                native_scripts: source.scripts(),
                network: Default::default(),
                workers: Default::default(),
                timers: timers::Timers::default(),
                listeners: BTreeMap::new(),
                validator: None,
                watched: BTreeSet::new(),
                emissions: Vec::new(),
                nodes: Vec::new(),
                ids: BTreeMap::new(),
                properties: Vec::new(),
                slots: Vec::new(),
                changes: BTreeMap::new(),
                identity: NEXT_INSTANCE.fetch_add(1, Ordering::Relaxed),
            }),
            registry,
            imports: &document.imports,
            assigned: BTreeSet::new(),
            aliases: Vec::new(),
            implicit: Vec::new(),
            binding_targets: vec![],
            read_candidates: Vec::new(),
        };
        for (url, shape) in &document.components {
            builder.instance.dynamic.catalog.insert(
                url.clone(),
                dynamic::Template::from_object(shape.clone(), &document.imports)?,
            );
        }
        builder.object(&document.root, None)?;
        // Module objects share this engine's slots and dependency graph. They
        // have no visual parent and are allocated before any binding runs.
        for singleton in &document.singletons {
            builder.object(singleton, None)?;
        }
        builder.link_aliases()?;
        builder.link_binding_targets()?;
        builder.default_children()?;
        Ok(builder)
    }
}

impl TypeRegistry {
    pub(super) fn wraps_component(
        &self,
        property_type: &str,
        imports: &[Import],
        object_type: &str,
    ) -> bool {
        let component = |ty: &str| {
            self.resolve(imports, ty)
                .is_ok_and(|(module, name, _)| module == "QtQml" && name == "Component")
        };
        (property_type == "QtQml.Component" || component(property_type)) && !component(object_type)
    }

    pub(super) fn canonical_type(&self, imports: &[Import], ty: &str) -> Result<String, Error> {
        if let Some(element) = list_element(ty) {
            return Ok(format!("list<{}>", self.canonical_type(imports, element)?));
        }
        if is_object_type(ty) && !ty.starts_with("Qml$Type") {
            let (module, name, _) = self.resolve(imports, ty)?;
            return Ok(format!("{module}.{name}"));
        }
        Ok(ty.into())
    }

    pub(super) fn parameters(
        &self,
        imports: &[Import],
        params: &[Parameter],
    ) -> Result<Vec<Parameter>, Error> {
        params
            .iter()
            .map(|p| {
                Ok(Parameter {
                    name: p.name.clone(),
                    ty: p
                        .ty
                        .as_deref()
                        .map(|s| self.canonical_type(imports, s))
                        .transpose()?,
                })
            })
            .collect()
    }

    pub(super) fn default_value(&self, imports: &[Import], ty: &str) -> Result<Value, Error> {
        Ok(match ty {
            "alias" | "var" | "variant" => Value::Undefined,
            "bool" => Value::Bool(false),
            "int" | "real" | "double" | "float" => Value::Number(0.0),
            "string" | "url" => Value::String(String::new()),
            "color" => Value::String("#000000".into()),
            name if Geometry::default_for(name).is_some() => {
                Value::Geometry(Geometry::default_for(name).unwrap())
            }
            list if list_element(list).is_some() => {
                self.default_value(imports, list_element(list).unwrap())?;
                Value::List(Vec::new())
            }
            name if is_object_type(name) => {
                if !name.starts_with("Qml$Type") {
                    self.resolve(imports, name)?;
                }
                Value::Null
            }
            _ => return Err(Error::new(format!("unsupported property type '{ty}'"))),
        })
    }
}

impl<I: std::ops::DerefMut<Target = Instance>> Builder<'_, I> {
    fn object(&mut self, source: &Object, parent: Option<ObjectId>) -> Result<ObjectId, Error> {
        let imports = source.type_imports.as_deref().unwrap_or(self.imports);
        let mut live_members = None;
        let (mut shell, module, name, definition) = if let Some(mut shell) =
            self.prepared.remove(&std::ptr::from_ref(source))
        {
            live_members = Some(std::mem::take(&mut shell.live_members));
            // Module and type names were resolved against this immutable registry
            // during preparation, including validation of versioned imports.
            let (module, types) = self
                .registry
                .modules
                .get_key_value(&shell.node.module)
                .expect("prepared module");
            let (name, definition) = types
                .get_key_value(&shell.node.name)
                .expect("prepared type");
            (shell, module.as_str(), name.as_str(), definition)
        } else {
            if source.type_imports.is_some() {
                self.registry.validate_imports(imports)?;
            }
            let (module, name, definition) = self.registry.resolve(imports, &source.type_name)?;
            let scope = self.scopes.imports(imports, self.registry);
            let mut shell = incubation::PreparedNode::new(source, module, name, definition, scope);
            for member in &source.members {
                shell.declare(self.registry, imports, member)?;
            }
            (shell, module, name, definition)
        };
        let id = self.instance.next_object();
        self.instance.dynamic.serial += 1;
        let offset = self.instance.slots.len();
        for slot in shell.node.properties.values_mut() {
            *slot += offset;
        }
        self.aliases.extend(
            shell
                .aliases
                .into_iter()
                .map(|(key, expr, scope)| (offset + key, expr, scope)),
        );
        if let Some(name) = shell.id {
            if self
                .instance
                .insert_scoped_id(self.context, name.clone(), id)
                .is_some()
            {
                return Err(Error::new(format!("duplicate id '{name}'")));
            }
        }
        shell.node.document = self.context.and_then(|context| {
            if self.instance.roots.units.contains_key(&context) {
                Some(context)
            } else {
                let declaration = self.instance.dynamic.contexts.get(&context)?.declaration;
                self.instance.node(declaration)?.document
            }
        });
        shell.node.context = self.context;
        shell.node.parent = parent;
        self.instance.nodes.push(NodeEntry {
            id,
            live: true,
            data: Some(shell.node),
        });
        if let Some(parent) = parent {
            self.instance.nodes[parent.index].children.push(id);
        }
        for mut property in shell.properties {
            if property.ty == "$qml:propertyGroup" {
                property.slot.value = Value::PropertyGroup(id, Rc::from(property.name.as_str()));
            }
            if property.ty == "$qml:anchorGroup" {
                property.slot.value = Value::AnchorGroup(id);
            }
            if property.slot.assigned {
                self.assigned.insert(self.instance.slots.len());
                if property.slot.binding.is_none() {
                    property.slot.value = self.instance.typed(&property.ty, property.slot.value)?;
                }
            }
            if property.ty == "$qml:anchorLine" && property.readonly {
                if let Some(edge) = AnchorEdge::ALL
                    .into_iter()
                    .find(|e| e.name() == property.name)
                {
                    property.slot.value = Value::AnchorLine(id, edge);
                }
            }
            self.instance.properties.push(Property {
                owner: id,
                name: property.name,
                ty: property.ty,
                readonly: property.readonly,
                required: property.required,
                alias: None,
            });
            self.instance.slots.push(property.slot);
        }
        if (module == "QtQuick" && name == "Loader")
            || definition
                .lineage
                .iter()
                .any(|(m, n)| m == "QtQuick" && n == "Loader")
        {
            self.instance.dynamic.loaders.insert(id, Default::default());
        }
        let is_repeater = (module == "QtQuick" && name == "Repeater")
            || definition
                .lineage
                .iter()
                .any(|(m, n)| m == "QtQuick" && n == "Repeater");
        if (module == "QtQuick" && name == "ListView")
            || definition
                .lineage
                .iter()
                .any(|(m, n)| m == "QtQuick" && n == "ListView")
        {
            self.instance
                .dynamic
                .repeaters
                .insert(id, repeater::State::list_view());
        }
        if is_repeater {
            self.instance
                .dynamic
                .repeaters
                .insert(id, Default::default());
        }
        let model = module == list_model::MODULE && name == "ListModel";
        if model {
            self.instance.models.insert(id, Default::default());
        }
        if module == list_model::MODULE && name == "ListElement" {
            return Err(Error::new("ListElement must appear inside ListModel"));
        }
        if module == "QtQml" && name == "Binding" {
            self.instance.bindings.insert(id, Vec::new());
        }
        let connections = module == "QtQml" && name == "Connections";
        if connections {
            let target = self.instance.key(id, "target")?;
            self.instance.slots[target].value = parent.map(Value::Object).unwrap_or(Value::Null);
        }
        if module == "QtQml.WorkerScript" && name == "WorkerScript" {
            self.instance.workers.register(
                id,
                source
                    .source_url
                    .as_deref()
                    .unwrap_or("https://qml.invalid/components/project/main.qml"),
            );
        }
        if module == "QtQml" && name == "Timer" {
            self.instance.timers.register(id);
        }
        let mut implicit = Vec::new();
        let count = live_members.as_ref().map_or(source.members.len(), Vec::len);
        for index in 0..count {
            let index = live_members
                .as_ref()
                .map_or(index, |members| members[index]);
            let member = &source.members[index];
            match &member.kind {
                MemberKind::Child(child) if module == "QtQml" && name == "Component" => {
                    let definition = dynamic::Template {
                        object: Rc::new(child.clone()),
                        imports: Rc::from(imports),
                        scopes: self.scopes.clone(),
                    };
                    if self
                        .instance
                        .dynamic
                        .templates
                        .insert(id, definition)
                        .is_some()
                    {
                        return Err(Error::new(
                            "Component requires exactly one object definition",
                        ));
                    }
                }
                MemberKind::Child(child) if is_repeater => {
                    self.bind(
                        id,
                        "delegate",
                        &Binding::Object(child.clone()),
                        member.scope,
                    )?;
                }
                MemberKind::Child(child) if model => {
                    let child_imports = child.type_imports.as_deref().unwrap_or(imports);
                    let (module, name, _) =
                        self.registry.resolve(child_imports, &child.type_name)?;
                    if (module, name) != (list_model::MODULE, "ListElement") {
                        return Err(Error::new("ListModel children must be ListElement objects"));
                    }
                    self.list_element(id, child, child_imports)?;
                }
                MemberKind::Child(child) => {
                    implicit.push(Value::Object(self.object(child, Some(id))?))
                }
                MemberKind::OnProperty { name, object } => {
                    let child = self.object(object, Some(id))?;
                    if self.instance.quick_kind(child, "Behavior") {
                        let key = self.instance.key_in(id, name, member.scope)?;
                        if self.instance.dynamic.behaviors.insert(key, child).is_some() {
                            return Err(Error::new(format!("multiple Behaviors on '{name}'")));
                        }
                        for (property, value) in [
                            ("targetProperty.name", Value::String(name.clone())),
                            ("targetProperty.object", Value::Object(id)),
                        ] {
                            let key = self.instance.key(child, property)?;
                            self.instance.slots[key].value = value;
                        }
                        continue;
                    }
                    if self.instance.is_animation(child) {
                        let (target_name, property_name) =
                            if self.instance.key(child, "$qml:onTarget").is_ok() {
                                ("$qml:onTarget", "$qml:onProperty")
                            } else {
                                ("target", "property")
                            };
                        let target = self.instance.key(child, target_name)?;
                        self.instance.slots[target].value = Value::Object(id);
                        self.bind(
                            child,
                            property_name,
                            &Binding::Expr(Expr::String(name.clone())),
                            member.scope,
                        )?;
                        let running = self.instance.key(child, "running")?;
                        if !self.assigned.contains(&running) {
                            self.instance.slots[running].value = Value::Bool(true);
                        }
                        continue;
                    }
                    if !self.instance.bindings.contains_key(&child) {
                        return Err(Error::new(format!(
                            "'{}' is not a supported property value source",
                            object.type_name
                        )));
                    }
                    self.bind(
                        child,
                        "target",
                        &Binding::Expr(Expr::Name("parent".into())),
                        member.scope,
                    )?;
                    self.bind(
                        child,
                        "property",
                        &Binding::Expr(Expr::String(name.clone())),
                        member.scope,
                    )?;
                }
                MemberKind::Binding { name, .. } if definition.readonly.contains(name) => {
                    return Err(Error::new(format!(
                        "cannot initialize readonly native property '{name}'"
                    )))
                }
                MemberKind::Binding { name, value } => self.bind(id, name, value, member.scope)?,
                MemberKind::Group { name, members } => self.group(id, name, members)?,
                MemberKind::Property {
                    name,
                    value: Some(value),
                    ..
                } => self.bind(id, name, value, member.scope)?,
                MemberKind::Handler { name, .. }
                    if connections && !name.starts_with("Component.on") => {}
                MemberKind::Handler {
                    name,
                    parameters,
                    body,
                } => {
                    self.instance.nodes[id.index].install_handler(name, parameters, body, member)?
                }
                _ => (),
            }
        }
        if module == "QtQml"
            && name == "Component"
            && !self.instance.dynamic.templates.contains_key(&id)
            && !self.instance.dynamic.file_errors.contains_key(&id)
        {
            return Err(Error::new("Component requires an object definition"));
        }
        if model {
            let count = self.instance.key(id, "count")?;
            self.instance.slots[count].value =
                Value::Number(self.instance.models[&id].len() as f64);
        }
        if connections {
            self.instance.connect_handlers(id, source)?;
        }
        if !implicit.is_empty() {
            let name = self.instance.nodes[id.index]
                .default_property
                .clone()
                .ok_or_else(|| {
                    Error::new(format!("'{}' has no default property", source.type_name))
                })?;
            let key = self.instance.key(id, &name)?;
            self.implicit.push((key, implicit));
        }
        if module == "QtQuick" && matches!(name, "Row" | "Flow") {
            if let Ok(key) = self.instance.key(id, "effectiveLayoutDirection") {
                let direction = self.instance.key(id, "layoutDirection")?;
                let mirror = self.instance.key(id, "LayoutMirroring.enabled")?;
                self.instance.slots[key].binding =
                    Some(InstalledBinding::PositionerDirection { direction, mirror });
                self.instance.slots[key].dirty = true;
            }
        }
        if let Ok(key) = self.instance.key(id, "LayoutMirroring.enabled") {
            if !self.assigned.contains(&key) && parent.is_some() {
                self.instance.slots[key].binding = Some(InstalledBinding::InheritedMirroring);
                self.instance.slots[key].dirty = true;
            }
        }
        Ok(id)
    }

    fn list_element(
        &mut self,
        parent: ObjectId,
        object: &Object,
        imports: &[Import],
    ) -> Result<(), Error> {
        let mut fields = BTreeMap::new();
        let mut lists = BTreeSet::new();
        for member in &object.members {
            let MemberKind::Binding { name, value } = &member.kind else {
                return Err(Error::new("ListElement expects constant role assignments"));
            };
            let items = match value {
                Binding::Object(item) => Some(vec![item.clone()]),
                Binding::Objects(items) => Some(items.clone()),
                _ => None,
            };
            let value = if let Some(items) = items {
                let shape = list_model::list_model_object(imports, items);
                let child = self.object(&shape, Some(parent))?;
                self.instance
                    .models
                    .get_mut(&child)
                    .expect("nested list model")
                    .role_owner = Some(parent);
                lists.insert(name.clone());
                Value::Object(child)
            } else if let Binding::Expr(expr) = value {
                Value::from(expr.as_literal().ok_or_else(||Error::new(format!("ListElement role '{name}' requires a scalar constant or ListElement definitions")))?)
            } else {
                return Err(Error::new("ListElement expects constant role assignments"));
            };
            fields.insert(name.clone(), value);
        }
        self.instance
            .models
            .get_mut(&parent)
            .unwrap()
            .static_row(fields, &lists)
    }

    fn default_children(&mut self) -> Result<(), Error> {
        for (key, mut children) in std::mem::take(&mut self.implicit) {
            let key = self.instance.write_target(key)?;
            self.assign_once(key)?;
            let property = &self.instance.properties[key];
            let value = if list_element(&property.ty).is_some() {
                Value::List(children)
            } else if children.len() == 1 {
                children.pop().unwrap()
            } else {
                return Err(Error::new(format!(
                    "default property '{}' accepts only one child",
                    property.name
                )));
            };
            self.instance.slots[key].value = self.instance.typed(&property.ty, value)?;
        }
        Ok(())
    }

    fn group(&mut self, owner: ObjectId, prefix: &str, members: &[Member]) -> Result<(), Error> {
        for member in members {
            match &member.kind {
                MemberKind::Binding { name, value } => {
                    self.bind(owner, &format!("{prefix}.{name}"), value, member.scope)?
                }
                MemberKind::Group { name, members } => {
                    self.group(owner, &format!("{prefix}.{name}"), members)?
                }
                _ => return Err(Error::new("a property group only accepts bindings")),
            }
        }
        Ok(())
    }

    fn assign_once(&mut self, key: SlotId) -> Result<(), Error> {
        if !self.assigned.insert(key) {
            return Err(Error::new(format!(
                "duplicate assignment to '{}'",
                self.instance.properties[key].name
            )));
        }
        Ok(())
    }

    fn bind(
        &mut self,
        owner: ObjectId,
        name: &str,
        binding: &Binding,
        scope: Option<u32>,
    ) -> Result<(), Error> {
        if self.instance.nodes[owner.index].module == "QtQuick"
            && self.instance.nodes[owner.index].name == "PropertyChanges"
            && self.instance.key(owner, name).is_err()
        {
            return self.instance.state_input(owner, name, binding, scope);
        }
        if name.contains('.')
            && self.instance.bindings.contains_key(&owner)
            && self.instance.key(owner, name).is_err()
        {
            return self.binding_input(owner, name, binding, scope);
        }
        if self.instance.key(owner, name).is_err() {
            if let Some((key, field)) = self.instance.geometry_path(owner, name) {
                let Value::Geometry(base) = self.instance.slots[key].value else {
                    unreachable!()
                };
                base.with(field, 0.)?;
                let fields = match &self.instance.slots[key].binding {
                    Some(InstalledBinding::GeometryFields { fields, .. }) => fields.clone(),
                    _ if self.assigned.contains(&key) => {
                        return Err(Error::new(format!(
                            "geometry '{}' already has an initializer",
                            self.instance.properties[key].name
                        )))
                    }
                    _ => Rc::new(Vec::new()),
                };
                let hidden = format!("$qml:geometry:{key}:{field}");
                let input = self.instance.allocate_property(
                    Property {
                        owner,
                        name: hidden.clone(),
                        ty: "real".into(),
                        readonly: false,
                        required: false,
                        alias: None,
                    },
                    Value::Number(base.field(field).unwrap()),
                )?;
                let mut fields = fields;
                Rc::make_mut(&mut fields).push((field, input));
                self.instance.slots[key].binding =
                    Some(InstalledBinding::GeometryFields { base, fields });
                self.instance.slots[key].dirty = true;
                self.assigned.insert(key);
                return self.bind(owner, &hidden, binding, scope);
            }
        }
        let key = self.instance.key(owner, name)?;
        self.assign_once(key)?;
        self.instance.slots[key].scope = scope;
        let value = match binding {
            Binding::Object(child) => {
                let ty = &self.instance.properties[key].ty;
                let imports = child.type_imports.as_deref().unwrap_or(self.imports);
                if self.registry.wraps_component(ty, imports, &child.type_name) {
                    // A Component-valued property owns a definition here. Its object and
                    // private ids come into existence each time the factory is invoked.
                    let mut scope = imports.to_vec();
                    scope.push(Import {
                        uri: "QtQml".into(),
                        is_path: false,
                        version: None,
                        alias: Some("$qml_component".into()),
                        span: child.span,
                    });
                    let wrapper = Object {
                        type_name: "$qml_component.Component".into(),
                        source_url: child.source_url.clone(),
                        source_layers: child.source_layers.clone(),
                        shadows: Vec::new(),
                        composite_types: Vec::new(),
                        type_imports: Some(scope),
                        span: child.span,
                        members: vec![Member {
                            kind: MemberKind::Child(child.clone()),
                            scope: None,
                            span: child.span,
                        }],
                    };
                    Value::Object(self.object(&wrapper, Some(owner))?)
                } else {
                    let value = Value::Object(self.object(child, Some(owner))?);
                    if list_element(&self.instance.properties[key].ty).is_some() {
                        Value::List(vec![value])
                    } else {
                        value
                    }
                }
            }
            Binding::Objects(children) => Value::List(
                children
                    .iter()
                    .map(|c| self.object(c, Some(owner)).map(Value::Object))
                    .collect::<Result<_, _>>()?,
            ),
            expression => {
                let slot = &mut self.instance.slots[key];
                let Some(value) = slot.install_expression(expression) else {
                    if matches!(expression, Binding::Expr(Expr::JavaScript(script)) if script.property_read.is_some())
                    {
                        self.read_candidates.push(key);
                    }
                    return Ok(());
                };
                value
            }
        };
        self.instance.slots[key].value = self
            .instance
            .typed(&self.instance.properties[key].ty, value)?;
        Ok(())
    }

    fn binding_input(
        &mut self,
        owner: ObjectId,
        path: &str,
        binding: &Binding,
        scope: Option<u32>,
    ) -> Result<(), Error> {
        if !matches!(binding, Binding::Expr(_) | Binding::Script(_)) {
            return Err(Error::new(
                "a declarative Binding target requires an expression or script",
            ));
        }
        let when = self.instance.key(owner, "when")?;
        let input = self.instance.allocate_property(
            Property {
                owner,
                name: format!("$qml:binding:{path}"),
                ty: "var".into(),
                readonly: true,
                required: false,
                alias: None,
            },
            Value::Undefined,
        )?;
        let slot = &mut self.instance.slots[input];
        slot.scope = scope;
        slot.binding = Some(InstalledBinding::Guarded {
            when,
            expression: Rc::new(binding.clone()),
        });
        slot.dirty = true;
        self.binding_targets.push((input, path.to_owned()));
        Ok(())
    }

    fn link_binding_targets(&mut self) -> Result<(), Error> {
        for (input, path) in &self.binding_targets {
            let owner = self.instance.properties[*input].owner;
            let (id, property) = path.split_once('.').expect("qualified binding target");
            let object = self
                .instance
                .lexical_id(owner, id)
                .ok_or_else(|| Error::new(format!("unknown Binding target id '{id}'")))?;
            let slot = self
                .instance
                .key_in(object, property, self.instance.slots[*input].scope)?;
            let target = self.instance.write_target(slot)?;
            let target_owner = self.instance.properties[target].owner;
            let declarations = self
                .instance
                .bindings
                .get_mut(&owner)
                .expect("Binding controller");
            declarations.push(binding_object::Declaration {
                input: *input,
                target,
                target_owner,
            });
        }
        Ok(())
    }

    fn link_aliases(&mut self) -> Result<(), Error> {
        for (key, expr, scope) in &self.aliases {
            let alias = match expr {
                Expr::Name(name) => Alias::Object(
                    self.instance
                        .lexical_id(self.instance.properties[*key].owner, name)
                        .ok_or_else(|| Error::new(format!("unknown alias id '{name}'")))?,
                ),
                Expr::Member(base, name) => {
                    let (id, path) = match base.as_ref() {
                        Expr::Name(id) => (id, name.clone()),
                        Expr::Member(group, part) => match group.as_ref() {
                            Expr::Name(id) => (id, format!("{part}.{name}")),
                            _ => return Err(Error::new("alias target is too deeply nested")),
                        },
                        _ => return Err(Error::new("alias requires id.property")),
                    };
                    let obj = self
                        .instance
                        .lexical_id(self.instance.properties[*key].owner, id)
                        .ok_or_else(|| Error::new(format!("unknown alias id '{id}'")))?;
                    let key = if let Ok(key) = self.instance.key_in(obj, &path, *scope) {
                        key
                    } else if let Some((first, second)) = path.split_once('.') {
                        let key = self.instance.key_in(obj, first, *scope)?;
                        let Value::Object(nested) = self.instance.slots[key].value else {
                            return Err(Error::new(format!(
                                "unresolved nested alias target '{id}.{path}'"
                            )));
                        };
                        self.instance.key_in(nested, second, *scope)?
                    } else {
                        return Err(Error::new(format!("unknown alias target '{id}.{path}'")));
                    };
                    Alias::Slot(key)
                }
                _ => return Err(Error::new("alias requires an id or id.property")),
            };
            self.instance.properties[*key].alias = Some(alias);
        }
        for (key, _, _) in &self.aliases {
            let mut visited = BTreeSet::new();
            let mut current = *key;
            while let Some(Alias::Slot(next)) = self.instance.properties[current].alias {
                if !visited.insert(current) {
                    return Err(Error::new("alias cycle"));
                }
                current = next;
            }
        }
        Ok(())
    }
}

impl Instance {
    pub(crate) fn allocate_template(
        &mut self,
        root: ObjectId,
        parent: Option<ObjectId>,
        template: &dynamic::Template,
        registry: &TypeRegistry,
    ) -> Result<BTreeSet<SlotId>, Error> {
        self.allocate_prepared(root, parent, template, registry, Default::default())
    }

    pub(crate) fn allocate_prepared(
        &mut self,
        root: ObjectId,
        parent: Option<ObjectId>,
        template: &dynamic::Template,
        registry: &TypeRegistry,
        prepared: incubation::PreparedNodes,
    ) -> Result<BTreeSet<SlotId>, Error> {
        let mut builder = Builder {
            prepared,
            context: Some(root),
            scopes: template.scopes.clone(),
            instance: self,
            registry,
            imports: &template.imports,
            assigned: Default::default(),
            aliases: vec![],
            implicit: vec![],
            binding_targets: vec![],
            read_candidates: vec![],
        };
        builder.object(&template.object, parent)?;
        builder.link_aliases()?;
        builder.link_binding_targets()?;
        builder.default_children()?;
        Ok(builder.assigned)
    }
}

impl Instance {
    pub(crate) fn allocate_document(
        &mut self,
        doc: &Document,
        registry: &TypeRegistry,
        unit: Rc<engine::Unit>,
    ) -> Result<BTreeSet<SlotId>, Error> {
        let mut builder = Builder {
            prepared: Default::default(),
            context: None,
            scopes: enums::Scopes::named(None, &doc.root)?,
            instance: self,
            registry,
            imports: &doc.imports,
            assigned: Default::default(),
            aliases: vec![],
            implicit: vec![],
            binding_targets: vec![],
            read_candidates: vec![],
        };
        let root = builder.document_object(&doc.root, &unit)?;
        let mut fresh = vec![root];
        let mut aliases = BTreeMap::new();
        for definition in &doc.singletons {
            let (name, scope) = definition
                .members
                .iter()
                .find_map(|member| match &member.kind {
                    MemberKind::Id(name) => Some((name, member.scope?)),
                    _ => None,
                })
                .ok_or_else(|| Error::new("linked singleton needs a declaration identity"))?;
            let id = if let Some(id) = builder.instance.roots.singletons.get(&scope) {
                *id
            } else {
                let id = builder.document_object(definition, &unit)?;
                builder.instance.roots.singletons.insert(scope, id);
                fresh.push(id);
                id
            };
            aliases.insert(name.clone(), id);
        }
        for id in fresh {
            builder
                .instance
                .dynamic
                .contexts
                .get_mut(&id)
                .expect("document context")
                .ids
                .extend(aliases.clone());
        }
        builder.link_aliases()?;
        builder.link_binding_targets()?;
        builder.default_children()?;
        Ok(builder.assigned)
    }
}

impl<I: std::ops::DerefMut<Target = Instance>> Builder<'_, I> {
    fn document_object(
        &mut self,
        object: &Object,
        unit: &Rc<engine::Unit>,
    ) -> Result<ObjectId, Error> {
        let id = self.instance.next_object();
        let declaration = self.instance.root();
        self.instance.dynamic.contexts.insert(
            id,
            dynamic::Context {
                declaration,
                row_revision: None,
                delegate_inputs: None,
                ids: BTreeMap::new(),
            },
        );
        self.instance.roots.units.insert(id, unit.clone());
        self.context = Some(id);
        self.object(object, None)
    }
}

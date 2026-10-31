use super::*;
use ferroui_qml_compiler::{Expr, Import, Member, MemberKind, Object};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_INSTANCE: AtomicU64 = AtomicU64::new(1);

struct Builder<'a> {
    instance: Instance,
    registry: &'a TypeRegistry,
    imports: &'a [Import],
    assigned: BTreeSet<SlotId>,
    aliases: Vec<(SlotId, Expr)>,
}

impl Instance {
    /// Allocates the complete object and id table before evaluating any binding.
    /// Required root properties can be supplied by the Rust caller.
    pub fn new(
        document: &Document,
        registry: &TypeRegistry,
        initial: BTreeMap<String, Value>,
    ) -> Result<Self, Error> {
        registry.validate_imports(&document.imports)?;
        if let Some(pragma) = document.pragmas.first() {
            return Err(Error::new(format!(
                "unsupported document pragma '{}'",
                pragma.name
            )));
        }
        let mut builder = Builder {
            instance: Self {
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
            },
            registry,
            imports: &document.imports,
            assigned: BTreeSet::new(),
            aliases: Vec::new(),
        };
        let root = builder.object(&document.root, None)?;
        builder.link_aliases()?;
        for (name, value) in initial {
            let key = builder.instance.key(root, &name)?;
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
        let mut instance = builder.instance;
        instance.transaction(|tx| tx.initialize())?;
        instance.changes.clear();
        Ok(instance)
    }

    pub(crate) fn write_target(&self, mut key: SlotId) -> Result<SlotId, Error> {
        loop {
            let property = &self.properties[key];
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

    fn validate_handles(&self, value: &Value) -> Result<(), Error> {
        match value {
            Value::Object(id) if self.node(*id).is_none() => {
                Err(Error::new("foreign object handle"))
            }
            Value::List(values) => values.iter().try_for_each(|v| self.validate_handles(v)),
            _ => Ok(()),
        }
    }

    pub(crate) fn typed(&self, ty: &str, value: Value) -> Result<Value, Error> {
        match (ty, &value) {
            ("var" | "variant" | "alias", _) => {
                self.validate_handles(&value)?;
            }
            ("bool", Value::Bool(_))
            | ("real" | "double" | "float", Value::Number(_))
            | ("string" | "url" | "color", Value::String(_)) => (),
            ("int", Value::Number(n))
                if n.is_finite() && (i32::MIN as f64..=i32::MAX as f64).contains(&n.trunc()) =>
            {
                return Ok(Value::Number(n.trunc()))
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
                    ty == "QtObject"
                        || node.composite_types.iter().any(|candidate| ty == candidate)
                        || ty == node.name
                        || ty == format!("{}.{}", node.module, node.name)
                        || node
                            .lineage
                            .iter()
                            .any(|(module, name)| ty == name || ty == format!("{module}.{name}"))
                }) =>
            {
                ()
            }
            _ => return Err(Error::new(format!("cannot assign {value:?} to '{ty}'"))),
        }
        Ok(value)
    }
}

fn list_element(ty: &str) -> Option<&str> {
    ty.strip_prefix("list<")?.strip_suffix('>')
}
fn is_object_type(ty: &str) -> bool {
    ty.chars().next().is_some_and(char::is_uppercase)
}

impl Builder<'_> {
    fn default_value(&self, imports: &[Import], ty: &str) -> Result<Value, Error> {
        Ok(match ty {
            "alias" | "var" | "variant" => Value::Undefined,
            "bool" => Value::Bool(false),
            "int" | "real" | "double" | "float" => Value::Number(0.0),
            "string" | "url" => Value::String(String::new()),
            "color" => Value::String("#000000".into()),
            list if list_element(list).is_some() => {
                self.default_value(imports, list_element(list).unwrap())?;
                Value::List(Vec::new())
            }
            name if is_object_type(name) => {
                if !name.starts_with("Qml$Type") {
                    self.registry.resolve(imports, name)?;
                }
                Value::Null
            }
            _ => return Err(Error::new(format!("unsupported property type '{ty}'"))),
        })
    }

    fn property(&mut self, property: Property, value: Value) -> Result<SlotId, Error> {
        let key = self.instance.properties.len();
        let node = &mut self.instance.nodes[property.owner.index];
        if node.properties.insert(property.name.clone(), key).is_some() {
            return Err(Error::new(format!(
                "duplicate property '{}'",
                property.name
            )));
        }
        self.instance.properties.push(property);
        self.instance.slots.push(Slot {
            assigned: false,
            value,
            binding: None,
            dirty: false,
            reads: BTreeSet::new(),
            dependents: BTreeSet::new(),
        });
        Ok(key)
    }

    fn object(&mut self, source: &Object, parent: Option<ObjectId>) -> Result<ObjectId, Error> {
        let imports = source.type_imports.as_deref().unwrap_or(self.imports);
        if source.type_imports.is_some() {
            self.registry.validate_imports(imports)?;
        }
        let (module, name, definition) = self.registry.resolve(imports, &source.type_name)?;
        let id = ObjectId {
            index: self.instance.nodes.len(),
            instance: self.instance.identity,
        };
        self.instance.nodes.push(Node {
            composite_types: source.composite_types.clone(),
            lineage: definition.lineage.clone(),
            module: module.into(),
            name: name.into(),
            parent,
            children: Vec::new(),
            properties: BTreeMap::new(),
            default_property: definition.default_property.clone(),
            native: definition.native.clone(),
            signals: definition.signals.clone(),
            handlers: BTreeMap::new(),
            methods: BTreeMap::new(),
        });
        if let Some(parent) = parent {
            self.instance.nodes[parent.index].children.push(id);
        }
        for (name, (ty, default)) in &definition.properties {
            self.property(
                Property {
                    owner: id,
                    name: name.clone(),
                    ty: ty.clone(),
                    readonly: false,
                    required: false,
                    alias: None,
                },
                default.clone(),
            )?;
        }
        let connections = module == "QtQml" && name == "Connections";
        if connections {
            let target = self.instance.key(id, "target")?;
            self.instance.slots[target].value = parent.map(Value::Object).unwrap_or(Value::Null);
        }
        let mut has_id = false;
        for member in &source.members {
            match &member.kind {
                MemberKind::Id(name) => {
                    if has_id || self.instance.ids.insert(name.clone(), id).is_some() {
                        return Err(Error::new(format!("duplicate id '{name}'")));
                    }
                    has_id = true;
                }
                MemberKind::Property {
                    name,
                    ty,
                    modifiers,
                    value,
                } => {
                    if modifiers.readonly && value.is_none() {
                        return Err(Error::new(format!(
                            "readonly property '{name}' requires an initializer"
                        )));
                    }
                    let key = self.property(
                        Property {
                            owner: id,
                            name: name.clone(),
                            ty: ty.clone(),
                            readonly: modifiers.readonly,
                            required: modifiers.required,
                            alias: None,
                        },
                        self.default_value(imports, ty)?,
                    )?;
                    if ty == "alias" {
                        let Some(Binding::Expr(expr)) = value else {
                            return Err(Error::new("alias requires an id or id.property"));
                        };
                        self.aliases.push((key, expr.clone()));
                    }
                    if modifiers.default {
                        self.instance.nodes[id.index].default_property = Some(name.clone());
                    }
                }
                MemberKind::Signal { name, parameters } => {
                    if self.instance.nodes[id.index]
                        .signals
                        .insert(name.clone(), parameters.clone())
                        .is_some()
                    {
                        return Err(Error::new(format!("duplicate signal '{name}'")));
                    }
                }
                MemberKind::Function {
                    name,
                    parameters,
                    return_type,
                    body,
                } => {
                    let proc = Procedure {
                        parameters: parameters.clone(),
                        result: return_type.clone(),
                        body: body.clone(),
                    };
                    if self.instance.nodes[id.index]
                        .methods
                        .insert(name.clone(), Rc::new(proc))
                        .is_some()
                    {
                        return Err(Error::new(format!("duplicate method '{name}'")));
                    }
                }
                MemberKind::Enum { name, .. } => {
                    return Err(Error::new(format!("unresolved enumeration '{name}'")))
                }
                MemberKind::Component { name, .. } => {
                    return Err(Error::new(format!(
                        "inline component '{name}' must be linked before construction"
                    )))
                }
                _ => (),
            }
        }
        let mut implicit = Vec::new();
        for member in &source.members {
            match &member.kind {
                MemberKind::Child(child) => {
                    implicit.push(Value::Object(self.object(child, Some(id))?))
                }
                MemberKind::Binding { name, value } => self.bind(id, name, value)?,
                MemberKind::Group { name, members } => self.group(id, name, members)?,
                MemberKind::Property {
                    name,
                    value: Some(value),
                    ..
                } => self.bind(id, name, value)?,
                MemberKind::Handler { name, .. }
                    if connections && name != "Component.onCompleted" =>
                {
                    ()
                }
                MemberKind::Handler {
                    name,
                    parameters,
                    body,
                } => self.handler(id, name, parameters, body)?,
                _ => (),
            }
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
            self.assign_once(key)?;
            let value = if list_element(&self.instance.properties[key].ty).is_some() {
                Value::List(implicit)
            } else if implicit.len() == 1 {
                implicit.pop().unwrap()
            } else {
                return Err(Error::new(format!(
                    "default property '{name}' accepts only one child"
                )));
            };
            self.instance.slots[key].value = self
                .instance
                .typed(&self.instance.properties[key].ty, value)?;
        }
        Ok(id)
    }

    fn group(&mut self, owner: ObjectId, prefix: &str, members: &[Member]) -> Result<(), Error> {
        for member in members {
            match &member.kind {
                MemberKind::Binding { name, value } => {
                    self.bind(owner, &format!("{prefix}.{name}"), value)?
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

    fn bind(&mut self, owner: ObjectId, name: &str, binding: &Binding) -> Result<(), Error> {
        let key = self.instance.key(owner, name)?;
        self.assign_once(key)?;
        let value = match binding {
            Binding::Object(child) => Value::Object(self.object(child, Some(owner))?),
            Binding::Objects(children) => Value::List(
                children
                    .iter()
                    .map(|c| self.object(c, Some(owner)).map(Value::Object))
                    .collect::<Result<_, _>>()?,
            ),
            expr => {
                self.instance.slots[key].binding = Some(Rc::new(expr.clone()));
                self.instance.slots[key].dirty = true;
                return Ok(());
            }
        };
        self.instance.slots[key].value = self
            .instance
            .typed(&self.instance.properties[key].ty, value)?;
        Ok(())
    }

    fn handler(
        &mut self,
        owner: ObjectId,
        name: &str,
        parameters: &[String],
        body: &[Stmt],
    ) -> Result<(), Error> {
        let node = &mut self.instance.nodes[owner.index];
        let (signal, formal) = if name == "Component.onCompleted" {
            ("Component.completed".to_owned(), Vec::new())
        } else {
            let raw = name
                .strip_prefix("on")
                .filter(|s| !s.is_empty())
                .ok_or_else(|| Error::new(format!("invalid handler '{name}'")))?;
            let mut chars = raw.chars();
            let signal = format!("{}{}", chars.next().unwrap().to_lowercase(), chars.as_str());
            let formal = if let Some(parameters) = node.signals.get(&signal) {
                parameters.clone()
            } else if signal
                .strip_suffix("Changed")
                .is_some_and(|p| node.properties.contains_key(p))
            {
                Vec::new()
            } else {
                return Err(Error::new(format!("unknown signal for handler '{name}'")));
            };
            (signal, formal)
        };
        if parameters.len() > formal.len() {
            return Err(Error::new(format!("too many parameters for '{name}'")));
        }
        let parameters = if parameters.is_empty() {
            formal
        } else {
            parameters
                .iter()
                .map(|name| Parameter {
                    name: name.clone(),
                    ty: None,
                })
                .collect()
        };
        if node
            .handlers
            .insert(
                signal,
                Rc::new(Procedure {
                    parameters,
                    result: None,
                    body: body.to_vec(),
                }),
            )
            .is_some()
        {
            return Err(Error::new(format!("duplicate handler '{name}'")));
        }
        Ok(())
    }

    fn link_aliases(&mut self) -> Result<(), Error> {
        for (key, expr) in &self.aliases {
            let alias = match expr {
                Expr::Name(name) => Alias::Object(
                    self.instance
                        .ids
                        .get(name)
                        .copied()
                        .ok_or_else(|| Error::new(format!("unknown alias id '{name}'")))?,
                ),
                Expr::Member(base, name) => {
                    let Expr::Name(id) = base.as_ref() else {
                        return Err(Error::new("alias requires id.property"));
                    };
                    let obj = self
                        .instance
                        .ids
                        .get(id)
                        .copied()
                        .ok_or_else(|| Error::new(format!("unknown alias id '{id}'")))?;
                    Alias::Slot(self.instance.key(obj, name)?)
                }
                _ => return Err(Error::new("alias requires an id or id.property")),
            };
            self.instance.properties[*key].alias = Some(alias);
        }
        for (key, _) in &self.aliases {
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

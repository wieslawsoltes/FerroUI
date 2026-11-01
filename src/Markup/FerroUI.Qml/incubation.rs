//! Prepare native node storage off the live object arena. Private IDs, values
//! and completion handlers are linked in a single transaction once it is ready.
use super::*;
use crate::ir::{Expr, Import, Member, MemberKind, Object};

// A slice bounds both wide node lists and objects with many non-child members.
const SLICE_STEPS: usize = 64;

// The key identifies an immutable object inside the retained template. No raw
// pointer is dereferenced. Synthetic wrappers take the ordinary allocation path.
pub(crate) type PreparedNodes = BTreeMap<*const Object, PreparedNode>;

pub(crate) struct PreparedNode {
    pub node: Box<Node>,
    pub properties: Vec<PropertySlot>,
    pub id: Option<String>,
    pub aliases: Vec<(usize, Expr, Option<u32>)>,
    // These entries still need an owner ID or a neighboring object at publication.
    pub live_members: Vec<usize>,
    native_properties: usize,
    eager_children: bool,
}

pub(crate) struct PropertySlot {
    pub name: String,
    pub ty: String,
    pub readonly: bool,
    pub required: bool,
    pub slot: Slot,
}

impl PreparedNode {
    pub fn new(
        source: &Object,
        module: &str,
        name: &str,
        definition: &TypeDefinition,
        type_scope: Rc<Namespace>,
    ) -> Self {
        let slots = definition
            .properties
            .iter()
            .map(|(name, (ty, value))| PropertySlot {
                name: name.clone(),
                ty: ty.clone(),
                readonly: definition.readonly.contains(name),
                required: false,
                slot: Slot::initial(value.clone()),
            })
            .collect();
        let properties = definition.properties.keys().cloned().zip(0..).collect();
        Self {
            node: Box::new(Node {
                document: None,
                source_url: source.source_url.clone(),
                source_layers: source.source_layers.clone(),
                context: None,
                visual_children: definition.visual_children.clone(),
                shadows: source.shadows.clone(),
                type_scope,
                composite_types: source.composite_types.clone(),
                lineage: definition.lineage.clone(),
                module: module.into(),
                name: name.into(),
                parent: None,
                children: Vec::new(),
                properties,
                default_property: definition.default_property.clone(),
                native: definition.native.clone(),
                signals: definition.signals.clone(),
                handlers: BTreeMap::new(),
                methods: BTreeMap::new(),
            }),
            properties: slots,
            id: None,
            aliases: Vec::new(),
            live_members: Vec::new(),
            native_properties: definition.properties.len(),
            eager_children: !matches!(
                (module, name),
                ("QtQml", "Component")
                    | (list_model::MODULE, "ListModel")
                    | ("QtQuick", "Repeater")
            ) && !definition
                .lineage
                .iter()
                .any(|(m, n)| m == "QtQuick" && n == "Repeater"),
        }
    }

    pub(crate) fn declare(
        &mut self,
        registry: &TypeRegistry,
        imports: &[Import],
        member: &Member,
    ) -> Result<(), Error> {
        match &member.kind {
            MemberKind::Id(name) => {
                if self.id.replace(name.clone()).is_some() {
                    return Err(Error::new(format!("duplicate id '{name}'")));
                }
            }
            MemberKind::Property {
                name,
                ty,
                modifiers,
                value,
            } => {
                if modifiers.override_ {
                    return Err(Error::new(format!(
                        "no virtual source property '{name}' to override"
                    )));
                }
                if modifiers.readonly && value.is_none() {
                    return Err(Error::new(format!(
                        "readonly property '{name}' requires an initializer"
                    )));
                }
                let key = self.properties.len();
                let property = PropertySlot {
                    name: name.clone(),
                    ty: registry.canonical_type(imports, ty)?,
                    readonly: modifiers.readonly,
                    required: modifiers.required,
                    slot: Slot::initial(registry.default_value(imports, ty)?),
                };
                if self.node.properties.insert(name.clone(), key).is_some() {
                    return Err(Error::new(format!("duplicate property '{name}'")));
                }
                self.properties.push(property);
                if ty == "alias" {
                    let Some(Binding::Expr(expr)) = value else {
                        return Err(Error::new("alias requires an id or id.property"));
                    };
                    self.aliases.push((key, expr.clone(), member.scope));
                }
                if modifiers.default {
                    self.node.default_property = Some(name.clone());
                }
            }
            MemberKind::Signal { name, parameters } => {
                let signature = registry.parameters(imports, parameters)?;
                if self.node.signals.insert(name.clone(), signature).is_some() {
                    return Err(Error::new(format!("duplicate signal '{name}'")));
                }
            }
            MemberKind::Function {
                name,
                parameters,
                return_type,
                body,
            } => {
                let procedure = Procedure {
                    scope: member.scope,
                    parameters: registry.parameters(imports, parameters)?,
                    result: return_type
                        .as_deref()
                        .map(|s| registry.canonical_type(imports, s))
                        .transpose()?,
                    body: body.clone(),
                };
                if self
                    .node
                    .methods
                    .insert(name.clone(), Rc::new(procedure))
                    .is_some()
                {
                    return Err(Error::new(format!("duplicate method '{name}'")));
                }
            }
            MemberKind::Component { name, .. } => {
                return Err(Error::new(format!(
                    "inline component '{name}' must be linked before construction"
                )))
            }
            _ => {}
        }
        Ok(())
    }
}

impl Slot {
    /// Install an expression without reading the creation context. The final
    /// transaction coerces literal values and evaluates dirty bindings.
    pub(super) fn install_expression(&mut self, binding: &Binding) -> Option<Value> {
        if let Binding::Expr(expr) = binding {
            if let Some(value) = expr.as_literal() {
                return Some(Value::from(value));
            }
        }
        self.binding = Some(InstalledBinding::Expression(Rc::new(binding.clone())));
        self.dirty = true;
        None
    }
}

impl PreparedNode {
    fn stage(&mut self, index: usize, member: &Member) -> Result<(), Error> {
        let (name, binding) = match &member.kind {
            MemberKind::Handler {
                name,
                parameters,
                body,
            } => {
                if self.node.module != "QtQml"
                    || self.node.name != "Connections"
                    || name.starts_with("Component.on")
                {
                    self.node.install_handler(name, parameters, body, member)?;
                }
                return Ok(());
            }
            MemberKind::Property {
                name,
                value: Some(binding),
                ..
            }
            | MemberKind::Binding {
                name,
                value: binding,
            } => (name, binding),
            MemberKind::Child(_) | MemberKind::Group { .. } | MemberKind::OnProperty { .. } => {
                self.live_members.push(index);
                return Ok(());
            }
            _ => return Ok(()),
        };
        let Some(&local) = self.node.properties.get(name) else {
            self.live_members.push(index);
            return Ok(());
        };
        let property = &mut self.properties[local];
        if !matches!(binding, Binding::Expr(_) | Binding::Script(_))
            || property.ty.starts_with("$qml:")
            || (self.node.module == "QtQml" && self.node.name == "Connections" && name == "target")
        {
            self.live_members.push(index);
            return Ok(());
        }
        if local < self.native_properties
            && property.readonly
            && matches!(member.kind, MemberKind::Binding { .. })
        {
            return Err(Error::new(format!(
                "cannot initialize readonly native property '{name}'"
            )));
        }
        if property.slot.assigned {
            return Err(Error::new(format!("duplicate assignment to '{name}'")));
        }
        property.slot.scope = member.scope;
        property.slot.assigned = true;
        if let Some(value) = property.slot.install_expression(binding) {
            property.slot.value = value;
        }
        Ok(())
    }
}

impl Node {
    pub(super) fn install_handler(
        &mut self,
        name: &str,
        parameters: &[String],
        body: &[Stmt],
        member: &Member,
    ) -> Result<(), Error> {
        let (signal, formal) = if name == "Component.onDestruction" {
            ("Component.destruction".to_owned(), Vec::new())
        } else if name == "ListView.onPooled" {
            ("ListView.pooled".to_owned(), Vec::new())
        } else if name == "ListView.onReused" {
            ("ListView.reused".to_owned(), Vec::new())
        } else if name == "Component.onCompleted" {
            ("Component.completed".to_owned(), Vec::new())
        } else {
            let (group, handler) = name.rsplit_once('.').unwrap_or(("", name));
            let raw = handler
                .strip_prefix("on")
                .filter(|s| !s.is_empty())
                .ok_or_else(|| Error::new(format!("invalid handler '{name}'")))?;
            let mut chars = raw.chars();
            let mut signal = format!("{}{}", chars.next().unwrap().to_lowercase(), chars.as_str());
            if !group.is_empty() {
                signal = format!("{group}.{signal}");
            }
            let formal = if let Some(parameters) = self.signals.get(&signal) {
                parameters.clone()
            } else if signal
                .strip_suffix("Changed")
                .is_some_and(|p| self.properties.contains_key(p))
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
        if self
            .handlers
            .insert(
                signal,
                Rc::new(Procedure {
                    scope: member.scope,
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
}

#[derive(Clone, Copy)]
struct Edge {
    member: usize,
    element: usize,
}

impl Edge {
    fn object(self, source: &Object) -> Option<&Object> {
        match &source.members.get(self.member)?.kind {
            MemberKind::Child(child) | MemberKind::OnProperty { object: child, .. }
                if self.element == 0 =>
            {
                Some(child)
            }
            MemberKind::Property {
                value: Some(binding),
                ..
            }
            | MemberKind::Binding { value: binding, .. } => match binding {
                Binding::Object(child) if self.element == 0 => Some(child),
                Binding::Objects(items) => items.get(self.element),
                _ => None,
            },
            _ => None,
        }
    }

    fn eager<'a>(
        self,
        source: &'a Object,
        shell: &PreparedNode,
        registry: &TypeRegistry,
        template_imports: &[Import],
    ) -> Option<&'a Object> {
        let child = self.object(source)?;
        match &source.members[self.member].kind {
            MemberKind::Child(_) if !shell.eager_children => None,
            MemberKind::Property {
                name,
                value: Some(Binding::Object(_)),
                ..
            }
            | MemberKind::Binding {
                name,
                value: Binding::Object(_),
            } => {
                // A Component-valued slot may synthesize a factory around the
                // definition. The factory does not construct its defined child.
                let slot = *shell.node.properties.get(name)?;
                let imports = child.type_imports.as_deref().unwrap_or(template_imports);
                (!registry.wraps_component(&shell.properties[slot].ty, imports, &child.type_name))
                    .then_some(child)
            }
            _ => Some(child),
        }
    }
}

enum Cursor {
    Start,
    Declare(usize),
    Stage(Edge),
}

pub(crate) struct Preparation {
    template: dynamic::Template,
    // Member indices from the root to the cursor. The sibling position for
    // each ancestor lets the traversal resume without rescanning visited nodes.
    path: Vec<Edge>,
    cursors: Vec<Cursor>,
    prepared: PreparedNodes,
}

impl Preparation {
    pub(crate) fn new(template: dynamic::Template) -> Self {
        Self {
            template,
            path: Vec::new(),
            cursors: vec![Cursor::Start],
            prepared: PreparedNodes::new(),
        }
    }

    pub(crate) fn matches(&self, template: &dynamic::Template) -> bool {
        Rc::ptr_eq(&self.template.object, &template.object)
    }

    /// Prepare eager children, value sources, object-valued properties and
    /// object lists. A separate declaration pass handles forward-declared slots.
    pub(crate) fn advance(
        &mut self,
        registry: &TypeRegistry,
        has_time: &mut impl FnMut() -> bool,
    ) -> Result<bool, Error> {
        // Resolve the saved path once per slice, then descend and unwind the
        // borrowed branch in linear time even for deeply nested layouts.
        let mut source = self.template.object.as_ref();
        let mut branch = vec![source];
        for &edge in &self.path {
            source = edge.object(source).expect("saved incubation child");
            branch.push(source);
        }
        for step in 0..SLICE_STEPS {
            if step > 0 && !has_time() {
                return Ok(false);
            }
            let Some(cursor) = self.cursors.last_mut() else {
                return Ok(true);
            };
            let source = *branch.last().expect("current incubation node");
            let imports = source
                .type_imports
                .as_deref()
                .unwrap_or(&self.template.imports);
            let key = std::ptr::from_ref(source);
            match cursor {
                Cursor::Start => {
                    if source.type_imports.is_some() {
                        registry.validate_imports(imports)?;
                    }
                    let (module, name, definition) =
                        registry.resolve(imports, &source.type_name)?;
                    let scope = self.template.scopes.imports(imports, registry);
                    self.prepared.insert(
                        key,
                        PreparedNode::new(source, module, name, definition, scope),
                    );
                    if !source.members.is_empty() {
                        *cursor = Cursor::Declare(0);
                        continue;
                    }
                }
                Cursor::Declare(member) => {
                    let shell = self
                        .prepared
                        .get_mut(&key)
                        .expect("prepared incubation node");
                    shell.declare(registry, imports, &source.members[*member])?;
                    *member += 1;
                    if *member == source.members.len() {
                        *cursor = Cursor::Stage(Edge {
                            member: 0,
                            element: 0,
                        });
                    }
                    continue;
                }
                Cursor::Stage(edge) => {
                    if edge.member < source.members.len() {
                        let shell = self
                            .prepared
                            .get_mut(&key)
                            .expect("prepared incubation node");
                        if edge.element == 0 {
                            shell.stage(edge.member, &source.members[edge.member])?;
                        }
                        if let Some(child) =
                            edge.eager(source, shell, registry, &self.template.imports)
                        {
                            self.path.push(*edge);
                            edge.element += 1;
                            branch.push(child);
                            self.cursors.push(Cursor::Start);
                        } else {
                            edge.member += 1;
                            edge.element = 0;
                        }
                        continue;
                    }
                }
            }
            branch.pop();
            self.path.pop();
            self.cursors.pop();
        }
        // If preparation finished on the final step, ask the host before
        // entering the indivisible live transaction in this same turn.
        Ok(self.cursors.is_empty() && has_time())
    }

    pub(crate) fn into_nodes(self) -> PreparedNodes {
        self.prepared
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eager_object_properties_and_lists_prepare_without_entering_component_definitions() {
        let doc = ir::parse(r#"import QtQml
            QtObject {
                property QtObject child: QtObject { id: one }
                property list<QtObject> rows: [QtObject { id: two }, QtObject { id: three }]
                property Component implicitFactory: QtObject { id: deferredImplicit }
                property Component explicitFactory: Component { id: factory; QtObject { id: deferredExplicit } }
            }"#).unwrap();
        let template = dynamic::Template::from_object(doc.root, &doc.imports).unwrap();
        let mut preparation = Preparation::new(template);
        let registry = TypeRegistry::new();
        while !preparation.advance(&registry, &mut || true).unwrap() {}
        let ids = preparation
            .prepared
            .values()
            .filter_map(|shell| shell.id.as_deref())
            .collect::<BTreeSet<_>>();
        assert_eq!(ids, BTreeSet::from(["one", "two", "three", "factory"]));
        assert_eq!(preparation.prepared.len(), 5);
    }

    #[test]
    fn a_budget_boundary_suspends_binding_and_handler_installation_in_the_private_node() {
        let properties = (0..80)
            .map(|n| format!("property int p{n}: p0 + 1\n"))
            .collect::<String>();
        let doc = ir::parse(&format!(
            "import QtQml\nQtObject {{ onPulse: {{ p1 = 4 }}; signal pulse(); {properties} }}"
        ))
        .unwrap();
        let template = dynamic::Template::from_object(doc.root, &doc.imports).unwrap();
        let root = std::ptr::from_ref(template.object.as_ref());
        let mut job = Preparation::new(template);
        let registry = TypeRegistry::new();
        loop {
            assert!(!job.advance(&registry, &mut || false).unwrap());
            let node = &job.prepared[&root];
            if node.node.handlers.contains_key("pulse")
                && node.properties[node.node.properties["p1"]].slot.dirty
            {
                assert!(!node.properties[node.node.properties["p79"]].slot.assigned);
                break;
            }
        }
        while !job.advance(&registry, &mut || true).unwrap() {}
        let node = &job.prepared[&root];
        assert!(node.properties[node.node.properties["p79"]].slot.dirty);
        assert!(node.live_members.is_empty());
    }

    #[test]
    fn a_budget_expiring_on_the_last_staged_member_defers_publication() {
        // One entry step, 31 declarations, 31 initializers, one unwind.
        let members = (0..31)
            .map(|n| format!("property int p{n}: {n}\n"))
            .collect::<String>();
        let doc = ir::parse(&format!("import QtQml\nQtObject {{ {members} }}")).unwrap();
        let mut job =
            Preparation::new(dynamic::Template::from_object(doc.root, &doc.imports).unwrap());
        let registry = TypeRegistry::new();
        let mut checks = 0;
        assert!(!job
            .advance(&registry, &mut || {
                checks += 1;
                checks < SLICE_STEPS
            })
            .unwrap());
        assert!(job.cursors.is_empty());
        assert!(job.advance(&registry, &mut || true).unwrap());
    }

    #[test]
    fn a_property_heavy_node_yields_with_real_declarations_in_private_storage() {
        let properties = (0..100)
            .map(|n| format!("property int value{n}: {n}\n"))
            .collect::<String>();
        let doc = ir::parse(&format!(
            "import QtQml\nQtObject {{ id: object; {properties}
                property alias first: object.value0
                required property string caption
                function result(): int {{ return value99 }}
                signal ready(int total)
            }}"
        ))
        .unwrap();
        let template = dynamic::Template::from_object(doc.root, &doc.imports).unwrap();
        let root = std::ptr::from_ref(template.object.as_ref());
        let mut preparation = Preparation::new(template);
        let registry = TypeRegistry::new();
        assert!(!preparation.advance(&registry, &mut || true).unwrap());
        let partial = &preparation.prepared[&root];
        assert!(partial.node.properties.contains_key("value0"));
        assert!(!partial.node.properties.contains_key("value99"));
        while !preparation.advance(&registry, &mut || true).unwrap() {}
        let completed = &preparation.prepared[&root];
        let first = completed.node.properties["first"];
        assert_eq!(completed.aliases[0].0, first);
        assert!(completed.properties[completed.node.properties["caption"]].required);
        assert!(completed.node.methods.contains_key("result"));
        assert_eq!(completed.node.signals["ready"][0].name, "total");
    }
}

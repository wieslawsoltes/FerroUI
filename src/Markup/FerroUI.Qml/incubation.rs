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

pub(crate) struct Preparation {
    template: dynamic::Template,
    // Member indices from the root to the cursor. The sibling position for
    // each ancestor lets the traversal resume without rescanning visited nodes.
    path: Vec<usize>,
    next_member: Vec<Option<usize>>,
    prepared: PreparedNodes,
}

impl Preparation {
    pub(crate) fn new(template: dynamic::Template) -> Self {
        Self {
            template,
            path: Vec::new(),
            next_member: vec![None],
            prepared: PreparedNodes::new(),
        }
    }

    pub(crate) fn matches(&self, template: &dynamic::Template) -> bool {
        Rc::ptr_eq(&self.template.object, &template.object)
    }

    /// Visit the static default-child tree. Component definitions and ListModel
    /// rows are data rather than live children. Object-valued property bindings
    /// may implicitly wrap a definition and use the regular allocator at activation.
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
            let MemberKind::Child(child) = &source.members[edge].kind else {
                unreachable!("incubation cursor follows default children")
            };
            source = child;
            branch.push(source);
        }
        for step in 0..SLICE_STEPS {
            if step > 0 && !has_time() {
                return Ok(false);
            }
            let Some(next) = self.next_member.last_mut() else {
                return Ok(true);
            };
            let source = *branch.last().expect("current incubation node");
            let imports = source
                .type_imports
                .as_deref()
                .unwrap_or(&self.template.imports);
            let key = std::ptr::from_ref(source);
            if let Some(index) = *next {
                if let Some(member) = source.members.get(index) {
                    *next = Some(index + 1);
                    let shell = self
                        .prepared
                        .get_mut(&key)
                        .expect("prepared incubation node");
                    shell.declare(registry, imports, member)?;
                    if shell.eager_children {
                        if let MemberKind::Child(child) = &member.kind {
                            branch.push(child);
                            self.path.push(index);
                            self.next_member.push(None);
                        }
                    }
                    continue;
                }
            } else {
                if source.type_imports.is_some() {
                    registry.validate_imports(imports)?;
                }
                let (module, name, definition) = registry.resolve(imports, &source.type_name)?;
                let scope = self.template.scopes.imports(imports, registry);
                let shell = PreparedNode::new(source, module, name, definition, scope);
                self.prepared.insert(key, shell);
                if !source.members.is_empty() {
                    *next = Some(0);
                    continue;
                }
            }
            branch.pop();
            self.path.pop();
            self.next_member.pop();
        }
        Ok(self.next_member.is_empty())
    }

    pub(crate) fn into_nodes(self) -> PreparedNodes {
        self.prepared
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

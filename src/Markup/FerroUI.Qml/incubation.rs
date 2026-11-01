//! Prepare native node storage off the live object arena. Private IDs, values
//! and completion handlers are linked in a single transaction once it is ready.
use super::*;
use crate::ir::{MemberKind, Object};

// A slice bounds both wide node lists and objects with many non-child members.
const SLICE_STEPS: usize = 64;

// The key identifies an immutable object inside the retained template. No raw
// pointer is dereferenced. Synthetic wrappers take the ordinary allocation path.
pub(crate) type PreparedNodes = BTreeMap<*const Object, PreparedNode>;

pub(crate) struct PreparedNode {
    pub node: Box<Node>,
    pub defaults: Vec<(String, String, bool, Slot)>,
}

impl PreparedNode {
    pub fn new(
        source: &Object,
        module: &str,
        name: &str,
        definition: &TypeDefinition,
        type_scope: Rc<Namespace>,
    ) -> Self {
        let defaults = definition
            .properties
            .iter()
            .map(|(name, (ty, value))| {
                (
                    name.clone(),
                    ty.clone(),
                    definition.readonly.contains(name),
                    Slot::initial(value.clone()),
                )
            })
            .collect();
        let properties = definition.properties.keys().cloned().zip(0..).collect();
        Self {
            node: Box::new(Node {
                source_url: source.source_url.clone(),
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
            defaults,
        }
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
        for step in 0..SLICE_STEPS {
            if step > 0 && !has_time() {
                return Ok(false);
            }
            let Some(next) = self.next_member.last_mut() else {
                return Ok(true);
            };
            let mut source = self.template.object.as_ref();
            for &edge in &self.path {
                let MemberKind::Child(child) = &source.members[edge].kind else {
                    unreachable!("incubation cursor follows default children")
                };
                source = child;
            }
            if let Some(index) = *next {
                if let Some(member) = source.members.get(index) {
                    *next = Some(index + 1);
                    if matches!(member.kind, MemberKind::Child(_)) {
                        self.path.push(index);
                        self.next_member.push(None);
                    }
                    continue;
                }
            } else {
                let imports = source
                    .type_imports
                    .as_deref()
                    .unwrap_or(&self.template.imports);
                if source.type_imports.is_some() {
                    registry.validate_imports(imports)?;
                }
                let (module, name, definition) = registry.resolve(imports, &source.type_name)?;
                let scope = self.template.scopes.imports(imports, registry);
                let shell = PreparedNode::new(source, module, name, definition, scope);
                self.prepared.insert(std::ptr::from_ref(source), shell);
                if (module, name) != ("QtQml", "Component")
                    && (module, name) != (list_model::MODULE, "ListModel")
                    && !((module, name) == ("QtQuick", "Repeater")
                        || definition
                            .lineage
                            .iter()
                            .any(|(m, n)| m == "QtQuick" && n == "Repeater"))
                    && !source.members.is_empty()
                {
                    *next = Some(0);
                    continue;
                }
            }
            self.path.pop();
            self.next_member.pop();
        }
        Ok(self.next_member.is_empty())
    }

    pub(crate) fn into_nodes(self) -> PreparedNodes {
        self.prepared
    }
}

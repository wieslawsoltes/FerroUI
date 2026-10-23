//! Component sets, variant reconciliation, and typed exposed properties.
//! Materialized instances retain stable layer IDs when their source identities
//! or corresponding named sibling positions survive a master/variant change.
use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub const COMPONENT_SET_METADATA_KEY: &str = "component_set";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum ComponentPropertyValue {
    Text(String),
    Boolean(bool),
    InstanceSwap(NodeId),
}
impl ComponentPropertyValue {
    pub fn kind_name(&self) -> &'static str {
        match self {
            Self::Text(_) => "Text",
            Self::Boolean(_) => "Boolean",
            Self::InstanceSwap(_) => "Instance swap",
        }
    }
    pub fn same_kind(&self, other: &Self) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
    fn direct_override(&self) -> &'static str {
        match self {
            Self::Text(_) => "text",
            Self::Boolean(_) => "visible",
            Self::InstanceSwap(_) => "component_id",
        }
    }
    fn manual_value(&self, node: &Node) -> Option<Self> {
        if !node.overridden_properties.contains(self.direct_override()) { return None; }
        match self {
            Self::Text(_) if node.kind == NodeKind::Text => Some(Self::Text(node.text.clone())),
            Self::Boolean(_) => Some(Self::Boolean(node.visible)),
            Self::InstanceSwap(_) if node.kind == NodeKind::Instance => node.component_id.map(Self::InstanceSwap),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComponentPropertyDefinition {
    /// The map key is the stable property identity; this name can be renamed.
    pub name: String,
    pub default_value: ComponentPropertyValue,
    /// Source layer IDs in the component, including the component itself.
    pub targets: Vec<NodeId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComponentPropertyView {
    pub key: String,
    pub definition: ComponentPropertyDefinition,
    pub value: ComponentPropertyValue,
    pub overridden: bool,
}

fn invalid(message: impl Into<String>) -> DocumentError {
    DocumentError::Invalid(message.into())
}

fn dependencies(document: &Document, component: NodeId) -> BTreeSet<NodeId> {
    document.descendants(component).into_iter()
        .filter_map(|id| document.nodes.get(&id).and_then(|node| node.component_id))
        .collect()
}

fn depends_on(document: &Document, component: NodeId, target: NodeId) -> bool {
    let mut stack = vec![component];
    let mut seen = BTreeSet::new();
    while let Some(id) = stack.pop() {
        if id == target {
            return true;
        }
        if seen.insert(id) {
            stack.extend(dependencies(document, id));
        }
    }
    false
}

fn target_accepts(node: &Node, value: &ComponentPropertyValue) -> bool {
    match value {
        ComponentPropertyValue::Text(_) => node.kind == NodeKind::Text,
        ComponentPropertyValue::Boolean(_) => true,
        ComponentPropertyValue::InstanceSwap(_) => node.kind == NodeKind::Instance,
    }
}

fn property_definition<'a>(
    document: &'a Document,
    component: NodeId,
    key: &str,
) -> Option<&'a ComponentPropertyDefinition> {
    let master = document.nodes.get(&component)?;
    if let Some(definition) = master.component_property_definitions.get(key) {
        return Some(definition);
    }
    // Keep a compatible exposed override while another variant omits its layer.
    let set = document.component_set_of(component)?;
    document.component_variants(set).into_iter()
        .filter_map(|id| document.nodes.get(&id))
        .find_map(|node| node.component_property_definitions.get(key))
}

impl Document {
    pub fn is_component_set(&self, id: NodeId) -> bool {
        self.nodes.get(&id).is_some_and(|node| node.kind == NodeKind::Frame
            && node.metadata.get(COMPONENT_SET_METADATA_KEY).is_some_and(|value| value == "true"))
    }

    pub fn component_variants(&self, set: NodeId) -> Vec<NodeId> {
        if !self.is_component_set(set) {
            return Vec::new();
        }
        self.nodes[&set].children.iter().copied()
            .filter(|id| self.nodes.get(id).is_some_and(|node| node.kind == NodeKind::Component))
            .collect()
    }

    pub fn component_set_of(&self, id: NodeId) -> Option<NodeId> {
        if self.is_component_set(id) {
            return Some(id);
        }
        let node = self.nodes.get(&id)?;
        let master = if node.kind == NodeKind::Instance {
            self.nodes.get(&node.component_id?)?
        } else {
            node
        };
        master.parent.filter(|parent| self.is_component_set(*parent))
    }

    /// Find the nearest component or instance containing an inspector selection.
    pub fn component_owner(&self, mut id: NodeId) -> Option<NodeId> {
        for _ in 0..MAX_DOCUMENT_DEPTH {
            let node = self.nodes.get(&id)?;
            if matches!(node.kind, NodeKind::Component | NodeKind::Instance) || self.is_component_set(id) {
                return Some(id);
            }
            id = node.parent?;
        }
        None
    }

    pub fn component_property_views(&self, instance: NodeId) -> Vec<ComponentPropertyView> {
        let Some(node) = self.nodes.get(&instance).filter(|node| node.kind == NodeKind::Instance) else {
            return Vec::new();
        };
        let Some(master) = node.component_id.and_then(|id| self.nodes.get(&id)) else {
            return Vec::new();
        };
        let plan = InstancePlan::new(self);
        master.component_property_definitions.iter().map(|(key, definition)| {
            let stored = node.component_property_values.get(key);
            let manual = definition.targets.iter().find_map(|source| {
                let target = plan.materialized_target(instance, master.id, *source)?;
                definition.default_value.manual_value(self.nodes.get(&target)?)
            });
            let overridden = manual.is_some() || stored.is_some();
            ComponentPropertyView {
                key: key.clone(),
                definition: definition.clone(),
                value: manual.or_else(|| stored.cloned()).unwrap_or_else(|| definition.default_value.clone()),
                overridden,
            }
        }).collect()
    }

    pub fn validate_component_properties(&self) -> Result<(), DocumentError> {
        for node in self.nodes.values() {
            if node.component_source.is_some_and(|source| !self.nodes.contains_key(&source)) {
                return Err(invalid(format!("Layer {} has a missing component source", node.id)));
            }
            if !node.component_property_definitions.is_empty() && node.kind != NodeKind::Component {
                return Err(invalid(format!("Property definitions require a main component ({})", node.id)));
            }
            for (key, definition) in &node.component_property_definitions {
                self.validate_component_property_definition(node.id, key, definition)?;
            }
            if !node.component_property_values.is_empty() && node.kind != NodeKind::Instance {
                return Err(invalid(format!("Exposed values require an instance ({})", node.id)));
            }
            for (key, value) in &node.component_property_values {
                let component = node.component_id.ok_or_else(|| invalid("Instance has no main component"))?;
                let definition = property_definition(self, component, key)
                    .ok_or_else(|| invalid(format!("Unknown component property {key} on {}", node.id)))?;
                self.validate_component_property_value(component, value)?;
                if !value.same_kind(&definition.default_value) {
                    return Err(invalid(format!("Wrong type for component property {key}")));
                }
            }
            if node.kind == NodeKind::Component {
                if dependencies(self, node.id).into_iter().any(|id| depends_on(self, id, node.id)) {
                    return Err(invalid(format!("Recursive component reference on {}", node.id)));
                }
            }
            if self.is_component_set(node.id) {
                let mut combinations = BTreeSet::new();
                let mut property_kinds = BTreeMap::new();
                for id in &node.children {
                    let variant = self.nodes.get(id).filter(|child| child.kind == NodeKind::Component)
                        .ok_or_else(|| invalid("A component set can contain only main components"))?;
                    if variant.variant_properties.iter().any(|(key, value)| key.trim().is_empty() || value.trim().is_empty())
                        || !combinations.insert(variant.variant_properties.clone()) {
                        return Err(invalid(format!("Duplicate or empty variant properties in {}", node.name)));
                    }
                    for (key, definition) in &variant.component_property_definitions {
                        let kind = definition.default_value.kind_name();
                        if property_kinds.insert(key, kind).is_some_and(|old| old != kind) {
                            return Err(invalid(format!("Variant property {key} changes type")));
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn validate_component_property_value(&self, component: NodeId, value: &ComponentPropertyValue) -> Result<(), DocumentError> {
        if let ComponentPropertyValue::InstanceSwap(target) = value {
            if !self.nodes.get(target).is_some_and(|node| node.kind == NodeKind::Component) {
                return Err(invalid("Instance swap must refer to a main component"));
            }
            if depends_on(self, *target, component) {
                return Err(invalid("Instance swap would introduce a recursive component"));
            }
        }
        Ok(())
    }

    fn validate_component_property_definition(
        &self,
        component: NodeId,
        key: &str,
        definition: &ComponentPropertyDefinition,
    ) -> Result<(), DocumentError> {
        if !self.nodes.get(&component).is_some_and(|node| node.kind == NodeKind::Component)
            || key.trim().is_empty() || definition.name.trim().is_empty() || definition.targets.is_empty() {
            return Err(invalid("A component property needs a name and at least one source layer"));
        }
        self.validate_component_property_value(component, &definition.default_value)?;
        let mut seen = BTreeSet::new();
        for target in &definition.targets {
            let node = self.nodes.get(target).ok_or_else(|| invalid("Component property target is missing"))?;
            if !seen.insert(*target) || (*target != component && !self.is_ancestor(component, *target))
                || !target_accepts(node, &definition.default_value) {
                return Err(invalid("Component property target is incompatible or outside its main component"));
            }
        }
        Ok(())
    }
}

/// Remap references when duplicating component source trees. Materializing an
/// instance keeps definitions on its source instead and must clear copied ones.
pub(crate) fn remap_component_references(node: &mut Node, remap: &BTreeMap<NodeId, NodeId>) {
    if let Some(mapped) = node.component_id.and_then(|id| remap.get(&id)) {
        node.component_id = Some(*mapped);
    }
    for definition in node.component_property_definitions.values_mut() {
        for target in &mut definition.targets {
            if let Some(mapped) = remap.get(target) { *target = *mapped; }
        }
        if let ComponentPropertyValue::InstanceSwap(target) = &mut definition.default_value {
            if let Some(mapped) = remap.get(target) { *target = *mapped; }
        }
    }
    for value in node.component_property_values.values_mut() {
        if let ComponentPropertyValue::InstanceSwap(target) = value {
            if let Some(mapped) = remap.get(target) { *target = *mapped; }
        }
    }
    if let Some(source) = node.component_source.and_then(|id| remap.get(&id)) {
        node.component_source = Some(*source);
    }
}

impl Document {
    /// Remove bindings made invalid by source deletion, reparenting, conversion,
    /// or detachment. A deleted swap default removes its exposed definition.
    pub fn repair_component_references(&mut self) {
        let ids: BTreeSet<_> = self.nodes.keys().copied().collect();
        let components: BTreeSet<_> = self.nodes.values()
            .filter(|node| node.kind == NodeKind::Component).map(|node| node.id).collect();
        for node in self.nodes.values_mut() {
            if node.component_source.is_some_and(|source| !ids.contains(&source)) {
                node.component_source = None;
            }
            if node.component_id.is_some_and(|component| !components.contains(&component)) {
                node.component_id = None;
                node.component_source = None;
                if node.kind == NodeKind::Instance { node.kind = NodeKind::Frame; }
            }
            if node.kind != NodeKind::Component {
                node.component_property_definitions.clear();
            }
            if node.kind != NodeKind::Instance {
                node.component_property_values.clear();
            }
        }
        let definitions: Vec<_> = self.nodes.values()
            .filter(|node| !node.component_property_definitions.is_empty())
            .map(|node| {
                let definitions = node.component_property_definitions.iter().filter_map(|(key, definition)| {
                    let mut definition = definition.clone();
                    let default_value = definition.default_value.clone();
                    definition.targets.retain(|target| self.nodes.get(target).is_some_and(|target_node| {
                        (*target == node.id || self.is_ancestor(node.id, *target))
                            && target_accepts(target_node, &default_value)
                    }));
                    definition.targets.sort_unstable();
                    definition.targets.dedup();
                    self.validate_component_property_definition(node.id, key, &definition).is_ok()
                        .then_some((key.clone(), definition))
                }).collect();
                (node.id, definitions)
            }).collect();
        for (id, definitions) in definitions {
            self.nodes.get_mut(&id).unwrap().component_property_definitions = definitions;
        }
        let values: Vec<_> = self.nodes.values()
            .filter(|node| !node.component_property_values.is_empty())
            .map(|node| {
                let values = node.component_property_values.iter().filter_map(|(key, value)| {
                    let component = node.component_id?;
                    let definition = property_definition(self, component, key)?;
                    (value.same_kind(&definition.default_value)
                        && self.validate_component_property_value(component, value).is_ok())
                        .then_some((key.clone(), value.clone()))
                }).collect();
                (node.id, values)
            }).collect();
        for (id, values) in values {
            self.nodes.get_mut(&id).unwrap().component_property_values = values;
        }
    }
}

fn preserve_variable_binding(node: &mut Node, old: &Node, property: &str) {
    if let Some(variable) = old.variable_bindings.get(property) {
        node.variable_bindings.insert(property.into(), *variable);
    } else {
        node.variable_bindings.remove(property);
    }
}

fn preserve_overrides(node: &mut Node, old: &Node) {
    node.overridden_properties = old.overridden_properties.clone();
    for property in &old.overridden_properties {
        match property.as_str() {
            "text" => { node.text = old.text.clone(); node.text_runs = old.text_runs.clone(); node.text_paragraphs = old.text_paragraphs.clone(); }
            "text_paragraphs" => {
                // A paragraph-only override follows edits to the master's text.
                // Rebase its byte ranges through the same split/merge rules as
                // a direct text edit; copying the old ranges can invalidate the
                // refreshed instance. A simultaneous text override is already
                // preserved above, making this replacement a no-op in that case.
                let mut rebased = old.clone();
                rebased.replace_text_preserving_runs(node.text.clone());
                node.text_paragraphs = rebased.text_paragraphs;
            }
            "fills" => {
                node.fills = old.fills.clone();
                node.fill_style = old.fill_style;
                preserve_variable_binding(node, old, "fill");
            }
            "strokes" => node.strokes = old.strokes.clone(),
            "effects" => { node.effects = old.effects.clone(); node.effect_style = old.effect_style; }
            "layout_grid" => {
                node.layout_grid = old.layout_grid; node.layout_grid_overlays.clone_from(&old.layout_grid_overlays); node.grid_style = old.grid_style;
                crate::layout_guides::copy_layout_guide_bindings(old, node, false);
            }
            "layout_grid_visible" => {
                if let (Some(grid), Some(old_grid)) = (&mut node.layout_grid, old.layout_grid) {
                    grid.visible = old_grid.visible;
                }
                for (grid, old_grid) in node.layout_grid_overlays.iter_mut().zip(&old.layout_grid_overlays) {
                    grid.visible = old_grid.visible;
                }
                crate::layout_guides::copy_layout_guide_bindings(old, node, true);
            }
            "visible" => node.visible = old.visible,
            "opacity" => node.opacity = old.opacity,
            "blend_mode" => node.blend_mode = old.blend_mode,
            "is_mask" => node.is_mask = old.is_mask,
            "mask_type" => node.mask_type = old.mask_type,
            "width" => node.width = old.width,
            "height" => node.height = old.height,
            "position" => { node.x = old.x; node.y = old.y; }
            "rotation" => node.rotation = old.rotation,
            "flip_x" => node.flip_x = old.flip_x,
            "flip_y" => node.flip_y = old.flip_y,
            "affine_transform" => node.affine_transform = old.affine_transform,
            "corner_radius" => { node.corner_radius = old.corner_radius; node.corner_radii = old.corner_radii; }
            "corner_smoothing" => node.corner_smoothing = old.corner_smoothing,
            "font_family" => node.font_family = old.font_family.clone(),
            "font_size" => {
                node.font_size = old.font_size;
                preserve_variable_binding(node, old, "font_size");
            }
            "font_weight" => node.font_weight = old.font_weight,
            "italic" => node.italic = old.italic,
            "line_height" => node.line_height = old.line_height,
            "letter_spacing" => node.letter_spacing = old.letter_spacing,
            "text_align" => node.text_align = old.text_align,
            "text_vertical_align" => node.text_vertical_align = old.text_vertical_align,
            "text_decoration" => node.text_decoration = old.text_decoration,
            "text_auto_resize" => node.text_auto_resize = old.text_auto_resize,
            "text_direction" => node.text_direction = old.text_direction,
            "font_features" => node.font_features = old.font_features.clone(),
            "text_style" => node.text_style = old.text_style,
            "name" => node.name = old.name.clone(),
            "path" => { node.path = old.path.clone(); node.vector_network = old.vector_network.clone(); }
            "component_id" => node.component_id = old.component_id,
            "layout" => {
                node.layout = old.layout.clone();
                crate::layout_variables::copy_layout_variable_bindings(old, node);
            }
            "layout_child" => node.layout_child = old.layout_child,
            "constraints" => node.constraints = old.constraints,
            _ => {}
        }
    }
}

#[derive(Default)]
struct ComponentChanges {
    nodes: BTreeMap<NodeId, Node>,
    removed: BTreeSet<NodeId>,
    pages: Option<Vec<Page>>,
    next_id: u64,
}
impl ComponentChanges {
    fn apply(self, document: &mut Document) {
        for id in &self.removed { document.nodes.remove(id); }
        document.nodes.extend(self.nodes);
        if let Some(pages) = self.pages { document.pages = pages; }
        document.next_id = self.next_id;
        if !self.removed.is_empty() {
            for node in document.nodes.values_mut() {
                node.interactions.retain(|interaction| match interaction.action {
                    PrototypeAction::NavigateTo { target }
                    | PrototypeAction::OpenOverlay { target, .. }
                    | PrototypeAction::ScrollTo { target } => !self.removed.contains(&target),
                    _ => true,
                });
            }
            for page in &mut document.pages {
                if page.prototype_start.is_some_and(|id| self.removed.contains(&id)) {
                    page.prototype_start = None;
                }
            }
            document.comments.retain(|thread| !thread.node.is_some_and(|id| self.removed.contains(&id)));
            document.guides.retain(|id, _| !self.removed.contains(id));
        }
        document.repair_component_references();
    }
}

/// An overlay of changed instance nodes keeps failed swaps atomic without
/// copying unrelated pages, image payloads, paths, or document metadata.
struct InstancePlan<'a> {
    document: &'a Document,
    changes: ComponentChanges,
}
impl<'a> InstancePlan<'a> {
    fn new(document: &'a Document) -> Self {
        Self { document, changes: ComponentChanges { next_id: document.next_id, ..ComponentChanges::default() } }
    }
    fn node(&self, id: NodeId) -> Option<&Node> {
        if self.changes.removed.contains(&id) { return None; }
        self.changes.nodes.get(&id).or_else(|| self.document.nodes.get(&id))
    }
    fn node_mut(&mut self, id: NodeId) -> Option<&mut Node> {
        if self.changes.removed.contains(&id) { return None; }
        if !self.changes.nodes.contains_key(&id) {
            let node = self.document.nodes.get(&id)?.clone();
            self.changes.nodes.insert(id, node);
        }
        self.changes.nodes.get_mut(&id)
    }
    fn allocate(&mut self) -> Result<NodeId, DocumentError> {
        loop {
            let id = self.changes.next_id;
            self.changes.next_id = id.checked_add(1).ok_or_else(|| invalid("Document ID space exhausted"))?;
            if self.node(id).is_none() && !self.changes.removed.contains(&id) && self.document.page(id).is_none()
                && !self.document.styles.contains_key(&id) && !self.document.variables.contains_key(&id)
                && !self.document.comments.iter().any(|comment| comment.id == id) {
                return Ok(id);
            }
        }
    }
    fn remove_tree(&mut self, root: NodeId) {
        let mut stack = vec![root];
        while let Some(id) = stack.pop() {
            if self.changes.removed.contains(&id) { continue; }
            if let Some(node) = self.node(id) { stack.extend(node.children.iter().copied()); }
            self.changes.removed.insert(id);
            self.changes.nodes.remove(&id);
        }
    }
    fn finish(self) -> Result<ComponentChanges, DocumentError> {
        let added = self.changes.nodes.keys().filter(|id| !self.document.nodes.contains_key(id)).count();
        let removed = self.changes.removed.iter().filter(|id| self.document.nodes.contains_key(id)).count();
        if self.document.nodes.len() - removed + added > MAX_DOCUMENT_NODES {
            return Err(invalid("Component change exceeds the document layer limit"));
        }
        Ok(self.changes)
    }

    fn layer_key(&self, node: &Node) -> (String, u8) {
        let source = node.component_source.and_then(|id| self.node(id)).unwrap_or(node);
        (source.name.clone(), materialized_kind(node.kind) as u8)
    }

    fn reconcile(&mut self, instance: NodeId, component: NodeId) -> Result<(), DocumentError> {
        let old = self.node(instance).filter(|node| node.kind == NodeKind::Instance)
            .ok_or_else(|| invalid("Select an instance"))?;
        let master = self.node(component).filter(|node| node.kind == NodeKind::Component)
            .ok_or_else(|| invalid("Select a main component or variant"))?;
        if instance == component || self.document.is_ancestor(instance, component)
            || self.document.is_ancestor(component, instance) {
            return Err(invalid("A component cannot be materialized inside itself"));
        }
        let mut depth = 0;
        let mut parent = old.parent;
        while let Some(id) = parent {
            let Some(node) = self.node(id) else { break };
            depth += 1;
            if depth >= MAX_DOCUMENT_DEPTH { return Err(invalid("Component hierarchy is too deep")); }
            if node.kind == NodeKind::Component && depends_on(self.document, master.id, node.id) {
                return Err(invalid("Instance swap would introduce a recursive component"));
            }
            parent = node.parent;
        }
        self.reconcile_node(instance, component, true, depth)?;
        self.remap_links(instance, component);
        Ok(())
    }

    fn reconcile_node(&mut self, target: NodeId, source: NodeId, root: bool, depth: usize) -> Result<(), DocumentError> {
        if depth >= MAX_DOCUMENT_DEPTH { return Err(invalid("Component hierarchy is too deep")); }
        let template = self.node(source).ok_or_else(|| invalid("Component source layer is missing"))?;
        let old = self.node(target);
        let swapped = old.filter(|old| !root && template.kind == NodeKind::Instance
            && old.overridden_properties.contains("component_id")
            && old.component_id != template.component_id)
            .and_then(|old| old.component_id).and_then(|id| self.node(id));
        // Borrow the old node and clone the effective source once. In particular,
        // do not copy image/path payloads to inspect a node or its matching key.
        let mut node = swapped.unwrap_or(template).clone();
        let source_children = std::mem::take(&mut node.children);
        node.id = target;
        node.parent = old.and_then(|old| old.parent);
        node.component_source = Some(source);
        node.component_property_definitions.clear();
        node.overridden_properties.clear();
        node.revision = old.map(|old| old.revision).unwrap_or(0);
        if template.kind == NodeKind::Component || swapped.is_some() {
            node.kind = NodeKind::Instance;
            node.component_id = if let Some(swapped) = swapped { Some(swapped.id) } else { Some(source) };
        }
        if let Some(old) = old {
            preserve_overrides(&mut node, old);
            if root {
                node.kind = NodeKind::Instance;
                node.component_id = Some(source);
                // A nested instance also has an identity in its containing
                // component. Preserve that outer source link across swaps.
                if old.component_source != old.component_id {
                    if let Some(outer_source) = old.component_source.and_then(|id| self.node(id))
                        .filter(|source| source.kind == NodeKind::Instance) {
                        node.component_source = old.component_source;
                        node.component_property_values.extend(outer_source.component_property_values.clone());
                    }
                }
                node.x = old.x;
                node.y = old.y;
                node.rotation = old.rotation;
                node.flip_x = old.flip_x;
                node.flip_y = old.flip_y;
                node.affine_transform = old.affine_transform;
                node.constraints = old.constraints;
                node.layout_child = old.layout_child;
            }
            if node.kind == NodeKind::Instance {
                for (key, value) in &old.component_property_values {
                    if old.overridden_properties.contains(&format!("component_property:{key}"))
                        || (root && old.component_source == old.component_id) {
                        node.component_property_values.insert(key.clone(), value.clone());
                    }
                }
            }
        }
        let old_children = old.map(|old| old.children.clone()).unwrap_or_default();
        let mut by_source: BTreeMap<NodeId, VecDeque<NodeId>> = BTreeMap::new();
        let mut by_name: BTreeMap<(String, u8), VecDeque<NodeId>> = BTreeMap::new();
        for id in &old_children {
            if let Some(child) = self.node(*id) {
                if let Some(source) = child.component_source { by_source.entry(source).or_default().push_back(*id); }
                by_name.entry(self.layer_key(child)).or_default().push_back(*id);
            }
        }
        self.changes.nodes.insert(target, node);
        let mut used = BTreeSet::new();
        let mut children = Vec::with_capacity(source_children.len());
        for source_child in source_children {
            let key = self.node(source_child).map(|node| (node.name.clone(), materialized_kind(node.kind) as u8))
                .ok_or_else(|| invalid("Component child is missing"))?;
            let pop = |queue: Option<&mut VecDeque<NodeId>>, used: &BTreeSet<NodeId>| {
                let queue = queue?;
                while let Some(id) = queue.pop_front() {
                    if !used.contains(&id) { return Some(id); }
                }
                None
            };
            let matched = pop(by_source.get_mut(&source_child), &used)
                .or_else(|| pop(by_name.get_mut(&key), &used));
            let child = if let Some(id) = matched { id } else { self.allocate()? };
            used.insert(child);
            self.reconcile_node(child, source_child, false, depth + 1)?;
            self.changes.nodes.get_mut(&child).unwrap().parent = Some(target);
            children.push(child);
        }
        for child in old_children {
            if !used.contains(&child) { self.remove_tree(child); }
        }
        self.changes.nodes.get_mut(&target).unwrap().children = children;
        Ok(())
    }
}

fn materialized_kind(kind: NodeKind) -> NodeKind {
    if kind == NodeKind::Component { NodeKind::Instance } else { kind }
}

impl InstancePlan<'_> {
    fn descendants(&self, root: NodeId) -> Vec<NodeId> {
        let mut stack = vec![root];
        let mut seen = BTreeSet::new();
        let mut out = Vec::new();
        while let Some(id) = stack.pop() {
            if !seen.insert(id) { continue; }
            if let Some(node) = self.node(id) {
                out.push(id);
                stack.extend(node.children.iter().rev().copied());
            }
        }
        out
    }

    fn materialized_target(&self, instance: NodeId, component: NodeId, source: NodeId) -> Option<NodeId> {
        let mut path = Vec::new();
        let mut current = source;
        while current != component {
            if path.len() >= MAX_DOCUMENT_DEPTH { return None; }
            path.push(current);
            current = self.node(current)?.parent?;
        }
        let mut current = instance;
        for source in path.into_iter().rev() {
            let expected = self.node(source)?;
            let key = (expected.name.clone(), materialized_kind(expected.kind) as u8);
            let children = &self.node(current)?.children;
            if let Some(found) = children.iter().find(|id| {
                self.node(**id).is_some_and(|node| node.component_source == Some(source))
            }) {
                current = *found;
                continue;
            }
            let siblings = &self.node(expected.parent?)?.children;
            let occurrence = siblings.iter().take_while(|id| **id != source)
                .filter(|id| self.node(**id).is_some_and(|node| {
                    (node.name.clone(), materialized_kind(node.kind) as u8) == key
                })).count();
            current = children.iter().filter(|id| self.node(**id).is_some_and(|node| self.layer_key(node) == key))
                .nth(occurrence).copied()?;
        }
        Some(current)
    }

    fn remap_links(&mut self, instance: NodeId, component: NodeId) {
        for id in self.descendants(instance) {
            let Some(node) = self.node(id) else { continue };
            if node.interactions.is_empty() { continue; }
            let mut interactions = node.interactions.clone();
            interactions.retain_mut(|interaction| {
                let target = match &mut interaction.action {
                    PrototypeAction::NavigateTo { target }
                    | PrototypeAction::OpenOverlay { target, .. }
                    | PrototypeAction::ScrollTo { target } => target,
                    _ => return true,
                };
                if *target == component || self.document.is_ancestor(component, *target) {
                    if let Some(mapped) = self.materialized_target(instance, component, *target) {
                        *target = mapped;
                    } else {
                        return false;
                    }
                }
                true
            });
            if self.node(id).is_some_and(|node| node.interactions != interactions) {
                self.node_mut(id).unwrap().interactions = interactions;
            }
        }
    }

    fn apply_properties(&mut self, instance: NodeId, component: NodeId) -> Result<(), DocumentError> {
        self.apply_properties_inner(instance, component, &mut Vec::new())
    }

    fn apply_properties_inner(&mut self, instance: NodeId, component: NodeId, stack: &mut Vec<NodeId>) -> Result<(), DocumentError> {
        if stack.len() >= MAX_DOCUMENT_DEPTH || stack.contains(&component) {
            return Err(invalid("Exposed properties would create a recursive component instance"));
        }
        stack.push(component);
        let definitions = self.node(component).ok_or_else(|| invalid("Missing main component"))?
            .component_property_definitions.clone();
        let values = self.node(instance).ok_or_else(|| invalid("Missing instance"))?
            .component_property_values.clone();
        for (key, definition) in &definitions {
            let value = values.get(key).filter(|value| value.same_kind(&definition.default_value));
            let ComponentPropertyValue::InstanceSwap(swap_component) = value.unwrap_or(&definition.default_value) else { continue };
            for source in &definition.targets {
                let Some(target) = self.materialized_target(instance, component, *source) else { continue };
                let Some(node) = self.node(target) else { continue };
                // Direct layer editing takes precedence until this exposed
                // property is authored or reset, which clears its direct flag.
                if node.overridden_properties.contains("component_id") { continue; }
                self.reconcile(target, *swap_component)?;
                if value.is_some() {
                    self.node_mut(target).unwrap().overridden_properties.insert("component_id".into());
                }
            }
        }
        // Nested instances inherit their own typed properties before a binding
        // exposed by the containing component applies to their visible layers.
        let mut pending = self.node(instance).map(|node| node.children.clone()).unwrap_or_default();
        let mut nested = Vec::new();
        while let Some(id) = pending.pop() {
            let Some(node) = self.node(id) else { continue };
            if node.kind == NodeKind::Instance {
                if let Some(component) = node.component_id { nested.push((id, component)); }
            } else {
                pending.extend(node.children.iter().copied());
            }
        }
        for (nested, component) in nested {
            self.apply_properties_inner(nested, component, stack)?;
        }
        for (key, definition) in &definitions {
            let explicit = values.get(key).filter(|value| value.same_kind(&definition.default_value));
            let value = explicit.unwrap_or(&definition.default_value);
            for source in &definition.targets {
                let Some(target) = self.materialized_target(instance, component, *source) else { continue };
                let Some(node) = self.node(target) else { continue };
                if node.overridden_properties.contains(value.direct_override()) { continue; }
                match value {
                    ComponentPropertyValue::Text(text) if node.kind == NodeKind::Text => {
                        if node.text != *text {
                            self.node_mut(target).unwrap().replace_text_preserving_runs(text.clone());
                        }
                    }
                    ComponentPropertyValue::Boolean(visible) => {
                        if node.visible != *visible { self.node_mut(target).unwrap().visible = *visible; }
                    }
                    _ => continue,
                }
            }
        }
        stack.pop();
        Ok(())
    }
}

impl Document {
    /// Materialize only the selected source subtree; unrelated document nodes
    /// are borrowed throughout planning, including embedded image payloads.
    pub fn create_component_instance(&mut self, component: NodeId, parent: NodeId, x: f64, y: f64) -> Result<NodeId, DocumentError> {
        if !x.is_finite() || !y.is_finite() || self.children(parent).is_none()
            || self.nodes.get(&parent).is_some_and(|node| !node.kind.is_container()) || self.is_component_set(parent) {
            return Err(invalid("Choose a valid container and finite instance position"));
        }
        if !self.nodes.get(&component).is_some_and(|node| node.kind == NodeKind::Component) {
            return Err(invalid("Choose a main component"));
        }
        let changes;
        let instance;
        {
            let mut plan = InstancePlan::new(self);
            instance = plan.allocate()?;
            let mut node = self.nodes[&component].clone();
            node.id = instance;
            node.kind = NodeKind::Instance;
            node.x = x;
            node.y = y;
            node.parent = Some(parent);
            node.children.clear();
            node.component_property_definitions.clear();
            node.component_property_values.clear();
            node.overridden_properties.clear();
            node.component_id = Some(component);
            node.component_source = Some(component);
            plan.changes.nodes.insert(instance, node);
            plan.reconcile(instance, component)?;
            plan.apply_properties(instance, component)?;
            if let Some(mut container) = plan.node(parent).cloned() {
                container.children.push(instance);
                plan.changes.nodes.insert(parent, container);
            } else {
                let mut pages = self.pages.clone();
                pages.iter_mut().find(|page| page.id == parent).unwrap().children.push(instance);
                plan.changes.pages = Some(pages);
            }
            changes = plan.finish()?;
        }
        changes.apply(self);
        self.layout_all();
        Ok(instance)
    }

    /// Combine existing main components without replacing their IDs or changing
    /// their world transforms. Components from different parents use their page.
    pub fn create_component_set(&mut self, variants: &[NodeId], name: &str) -> Result<NodeId, DocumentError> {
        if variants.is_empty() || name.trim().is_empty() {
            return Err(invalid("Select main components and enter a component set name"));
        }
        let selected: BTreeSet<_> = variants.iter().copied().collect();
        if selected.len() != variants.len() { return Err(invalid("A variant can appear only once in a set")); }
        let page = self.page_of(variants[0]).ok_or_else(|| invalid("Main component has no page"))?;
        for id in variants {
            if !self.nodes.get(id).is_some_and(|node| node.kind == NodeKind::Component)
                || self.effective_locked(*id) || self.page_of(*id) != Some(page)
                || self.component_set_of(*id).is_some()
                || variants.iter().any(|other| *id != *other && self.is_ancestor(*id, *other)) {
                return Err(invalid("Choose unlocked main components on one page outside an existing set"));
            }
        }
        let first_parent = self.nodes[&variants[0]].parent.unwrap_or(page);
        let parent = if variants.iter().all(|id| self.nodes[id].parent == Some(first_parent)) { first_parent } else { page };
        let parent_depth = self.nodes.get(&parent).map(|_| {
            let mut depth = 0;
            let mut current = Some(parent);
            while let Some(id) = current {
                let Some(node) = self.nodes.get(&id) else { break };
                depth += 1;
                if depth >= MAX_DOCUMENT_DEPTH { break; }
                current = node.parent;
            }
            depth
        }).unwrap_or(0);
        for root in variants {
            let mut stack = vec![(*root, parent_depth + 1)];
            while let Some((id, depth)) = stack.pop() {
                if depth >= MAX_DOCUMENT_DEPTH { return Err(invalid("Component set would exceed the document depth limit")); }
                stack.extend(self.nodes[&id].children.iter().map(|child| (*child, depth + 1)));
            }
        }
        let inverse = self.world_transform(parent).and_then(Transform::inverse)
            .ok_or_else(|| invalid("Component parent has a singular transform"))?;
        let transforms: Vec<_> = variants.iter().map(|id| {
            self.world_transform(*id).map(|transform| (*id, inverse * transform))
                .ok_or_else(|| invalid("Component transform is unavailable"))
        }).collect::<Result<_, _>>()?;
        let bounds = transforms.iter().map(|(id, transform)| transform.transform_rect(self.nodes[id].local_bounds()))
            .reduce(Rect::union).ok_or_else(|| invalid("Component bounds are unavailable"))?.inflate(24.0);
        let changes;
        let set;
        {
            let mut plan = InstancePlan::new(self);
            set = plan.allocate()?;
            let mut group = Node::new(NodeKind::Frame, name.trim(), bounds.x, bounds.y, bounds.width, bounds.height);
            group.id = set;
            group.parent = Some(parent);
            group.children = variants.to_vec();
            group.fills.clear();
            group.clip_content = false;
            group.metadata.insert(COMPONENT_SET_METADATA_KEY.into(), "true".into());
            let mut combinations = BTreeSet::new();
            let mut kinds: BTreeMap<String, &'static str> = BTreeMap::new();
            for (index, (id, transform)) in transforms.into_iter().enumerate() {
                let mut variant = plan.node(id).unwrap().clone();
                variant.parent = Some(set);
                variant.set_local_transform(Transform::translation(-bounds.x, -bounds.y) * transform);
                if variant.variant_properties.is_empty() {
                    let base = if variant.name.trim().is_empty() { format!("Variant {}", index + 1) } else { variant.name.clone() };
                    let mut value = base.clone();
                    let mut suffix = 2;
                    while combinations.contains(&BTreeMap::from([("State".into(), value.clone())])) {
                        value = format!("{base} {suffix}"); suffix += 1;
                    }
                    variant.variant_properties.insert("State".into(), value);
                }
                if variant.variant_properties.iter().any(|(key, value)| key.trim().is_empty() || value.trim().is_empty())
                    || !combinations.insert(variant.variant_properties.clone()) {
                    return Err(invalid("Each variant needs a unique, nonempty property combination"));
                }
                for (key, definition) in &variant.component_property_definitions {
                    let kind = definition.default_value.kind_name();
                    if kinds.insert(key.clone(), kind).is_some_and(|old| old != kind) {
                        return Err(invalid("An exposed property must have the same type in every variant"));
                    }
                }
                plan.changes.nodes.insert(id, variant);
            }
            let parents: BTreeSet<_> = variants.iter().filter_map(|id| self.nodes[id].parent).chain([parent]).collect();
            let mut pages = self.pages.clone();
            for owner in parents {
                let original = self.children(owner).ok_or_else(|| invalid("Missing component parent"))?;
                let insertion = original.iter().position(|id| selected.contains(id)).unwrap_or(original.len());
                let mut children: Vec<_> = original.iter().copied().filter(|id| !selected.contains(id)).collect();
                if owner == parent { children.insert(insertion.min(children.len()), set); }
                if let Some(mut node) = plan.node(owner).cloned() {
                    node.children = children;
                    plan.changes.nodes.insert(owner, node);
                } else if let Some(page) = pages.iter_mut().find(|page| page.id == owner) {
                    page.children = children;
                }
            }
            plan.changes.pages = Some(pages);
            plan.changes.nodes.insert(set, group);
            changes = plan.finish()?;
        }
        changes.apply(self);
        self.layout_all();
        Ok(set)
    }

    pub fn add_component_variant(&mut self, set: NodeId, template: NodeId) -> Result<NodeId, DocumentError> {
        if !self.component_variants(set).contains(&template) || self.effective_locked(set) {
            return Err(invalid("Choose a main component inside an unlocked component set"));
        }
        let changes;
        let variant;
        {
            let mut plan = InstancePlan::new(self);
            let source = self.descendants(template);
            let remap: BTreeMap<_, _> = source.iter().map(|id| Ok((*id, plan.allocate()?)))
                .collect::<Result<_, DocumentError>>()?;
            variant = remap[&template];
            let right = self.component_variants(set).iter().filter_map(|id| self.nodes.get(id))
                .map(|node| node.local_transform().transform_rect(node.local_bounds()).right())
                .fold(0.0, f64::max);
            let origin = self.nodes[&template].local_transform();
            let offset = right + 32.0 - origin.transform_rect(self.nodes[&template].local_bounds()).x;
            for id in source {
                let mut node = self.nodes[&id].clone();
                node.id = remap[&id];
                node.revision = 0;
                node.parent = if id == template { Some(set) } else { node.parent.and_then(|parent| remap.get(&parent).copied()) };
                node.children = node.children.iter().map(|child| remap[child]).collect();
                remap_component_references(&mut node, &remap);
                for interaction in &mut node.interactions {
                    match &mut interaction.action {
                        PrototypeAction::NavigateTo { target } | PrototypeAction::OpenOverlay { target, .. }
                        | PrototypeAction::ScrollTo { target } => {
                            if let Some(mapped) = remap.get(target) { *target = *mapped; }
                        }
                        _ => {}
                    }
                }
                if id == template {
                    node.kind = NodeKind::Component;
                    node.component_id = None;
                    node.component_source = None;
                    node.component_property_values.clear();
                    node.overridden_properties.clear();
                    node.set_local_transform(Transform::translation(offset, 0.0) * origin);
                    let axis = if node.variant_properties.contains_key("State") { "State".to_string() }
                        else { node.variant_properties.keys().next().cloned().unwrap_or_else(|| "State".into()) };
                    let mut number = self.component_variants(set).len() + 1;
                    loop {
                        node.variant_properties.insert(axis.clone(), format!("Variant {number}"));
                        if !self.component_variants(set).iter().any(|id| self.nodes[id].variant_properties == node.variant_properties) { break; }
                        number += 1;
                    }
                    node.name = variant_label(&node.variant_properties);
                }
                plan.changes.nodes.insert(node.id, node);
            }
            let mut group = plan.node(set).unwrap().clone();
            group.children.push(variant);
            let bounds = plan.node(variant).unwrap().local_transform().transform_rect(plan.node(variant).unwrap().local_bounds());
            group.width = group.width.max(bounds.right() + 24.0);
            group.height = group.height.max(bounds.bottom() + 24.0);
            plan.changes.nodes.insert(set, group);
            changes = plan.finish()?;
        }
        changes.apply(self);
        self.layout_all();
        Ok(variant)
    }

    pub fn set_variant_properties(&mut self, component: NodeId, properties: BTreeMap<String, String>) -> Result<(), DocumentError> {
        let set = self.component_set_of(component).filter(|_| self.nodes.get(&component).is_some_and(|node| node.kind == NodeKind::Component))
            .ok_or_else(|| invalid("Choose a main variant inside a component set"))?;
        if properties.is_empty() || properties.iter().any(|(key, value)| key.trim().is_empty() || value.trim().is_empty()) {
            return Err(invalid("Variant property names and values cannot be empty"));
        }
        if self.component_variants(set).iter().any(|id| *id != component && self.nodes[id].variant_properties == properties) {
            return Err(invalid("Another variant already has this property combination"));
        }
        self.nodes.get_mut(&component).unwrap().variant_properties = properties.clone();
        for node in self.nodes.values_mut().filter(|node| node.kind == NodeKind::Instance && node.component_id == Some(component)) {
            node.variant_properties = properties.clone();
        }
        Ok(())
    }

    /// Replace an instance's main component or variant while preserving matching
    /// materialized layer IDs, placement, and explicitly authored overrides.
    pub fn swap_instance_component(&mut self, instance: NodeId, component: NodeId) -> Result<(), DocumentError> {
        if self.nodes.get(&instance).is_some_and(|node| node.kind == NodeKind::Instance && node.component_id == Some(component))
            && self.nodes.get(&component).is_some_and(|node| node.kind == NodeKind::Component) {
            return Ok(());
        }
        let changes = {
            let mut plan = InstancePlan::new(self);
            plan.reconcile(instance, component)?;
            plan.changes.nodes.get_mut(&instance).unwrap().overridden_properties.insert("component_id".into());
            plan.apply_properties(instance, component)?;
            plan.finish()?
        };
        changes.apply(self);
        self.layout_all();
        Ok(())
    }

    pub fn set_instance_variant(&mut self, instance: NodeId, properties: &BTreeMap<String, String>) -> Result<NodeId, DocumentError> {
        let set = self.component_set_of(instance).ok_or_else(|| invalid("This instance has no component set"))?;
        let variants: Vec<_> = self.component_variants(set).into_iter().filter(|id| {
            properties.iter().all(|(key, value)| self.nodes[id].variant_properties.get(key) == Some(value))
        }).collect();
        if variants.len() != 1 {
            return Err(invalid("Variant properties must identify exactly one variant"));
        }
        self.swap_instance_component(instance, variants[0])?;
        Ok(variants[0])
    }

    pub fn set_instance_property(
        &mut self,
        instance: NodeId,
        key: &str,
        value: Option<ComponentPropertyValue>,
    ) -> Result<(), DocumentError> {
        let node = self.nodes.get(&instance).filter(|node| node.kind == NodeKind::Instance)
            .ok_or_else(|| invalid("Select an instance to change exposed properties"))?;
        let component = node.component_id.ok_or_else(|| invalid("Instance has no main component"))?;
        let definition = property_definition(self, component, key).cloned()
            .ok_or_else(|| invalid("Exposed property does not exist"))?;
        if let Some(value) = &value {
            if !value.same_kind(&definition.default_value) { return Err(invalid("Exposed property type does not match")); }
            self.validate_component_property_value(component, value)?;
        }
        let changes = {
            let mut plan = InstancePlan::new(self);
            let mut node = plan.node(instance).unwrap().clone();
            let marker = format!("component_property:{key}");
            if let Some(value) = value {
                node.component_property_values.insert(key.into(), value);
                node.overridden_properties.insert(marker);
            } else {
                node.component_property_values.remove(key);
                node.overridden_properties.remove(&marker);
            }
            plan.changes.nodes.insert(instance, node);
            for source in &definition.targets {
                if let Some(target) = plan.materialized_target(instance, component, *source) {
                    let property = definition.default_value.direct_override();
                    if plan.node(target).is_some_and(|node| node.overridden_properties.contains(property)) {
                        plan.node_mut(target).unwrap().overridden_properties.remove(property);
                    }
                }
            }
            plan.reconcile(instance, component)?;
            plan.apply_properties(instance, component)?;
            plan.finish()?
        };
        changes.apply(self);
        self.layout_all();
        Ok(())
    }

    pub fn set_component_property_definition(
        &mut self,
        component: NodeId,
        key: &str,
        definition: ComponentPropertyDefinition,
    ) -> Result<(), DocumentError> {
        self.validate_component_property_definition(component, key, &definition)?;
        if let Some(set) = self.component_set_of(component) {
            for id in self.component_variants(set) {
                if id != component && self.nodes[&id].component_property_definitions.get(key)
                    .is_some_and(|other| !other.default_value.same_kind(&definition.default_value)) {
                    return Err(invalid("An exposed property must have the same type in every variant"));
                }
            }
        }
        let changes = {
            let mut plan = InstancePlan::new(self);
            let mut master = plan.node(component).unwrap().clone();
            master.component_property_definitions.insert(key.into(), definition.clone());
            plan.changes.nodes.insert(component, master);
            for target in &definition.targets {
                match &definition.default_value {
                    ComponentPropertyValue::InstanceSwap(value) => {
                        plan.reconcile(*target, *value)?;
                        plan.changes.nodes.get_mut(target).unwrap().overridden_properties.insert("component_id".into());
                    }
                    value => {
                        let mut node = plan.node(*target).unwrap().clone();
                        match value {
                            ComponentPropertyValue::Text(text) => {
                                node.replace_text_preserving_runs(text.clone());
                                if node.component_source.is_some() { node.overridden_properties.insert("text".into()); }
                            }
                            ComponentPropertyValue::Boolean(visible) => {
                                node.visible = *visible;
                                if node.component_source.is_some() { node.overridden_properties.insert("visible".into()); }
                            }
                            _ => {}
                        }
                        plan.changes.nodes.insert(*target, node);
                    }
                }
            }
            plan.sync_component_and_dependents(component)?;
            plan.finish()?
        };
        changes.apply(self);
        self.layout_all();
        Ok(())
    }

    pub fn remove_component_property(&mut self, component: NodeId, key: &str) -> Result<bool, DocumentError> {
        let master = self.nodes.get(&component).filter(|node| node.kind == NodeKind::Component)
            .ok_or_else(|| invalid("Select a main component"))?;
        if !master.component_property_definitions.contains_key(key) { return Ok(false); }
        let changes = {
            let mut plan = InstancePlan::new(self);
            let mut master = plan.node(component).unwrap().clone();
            master.component_property_definitions.remove(key);
            plan.changes.nodes.insert(component, master);
            plan.sync_component_and_dependents(component)?;
            plan.finish()?
        };
        changes.apply(self);
        self.layout_all();
        Ok(true)
    }

    pub fn refresh_component_instances(&mut self, component: NodeId) -> Result<usize, DocumentError> {
        if !self.nodes.get(&component).is_some_and(|node| node.kind == NodeKind::Component) {
            return Err(invalid("Select a main component to update instances"));
        }
        let (changes, count) = {
            let mut plan = InstancePlan::new(self);
            let count = plan.sync_component_and_dependents(component)?;
            (plan.finish()?, count)
        };
        changes.apply(self);
        self.layout_all();
        Ok(count)
    }
}

impl InstancePlan<'_> {
    fn sync_component_and_dependents(&mut self, component: NodeId) -> Result<usize, DocumentError> {
        let mut remaining: BTreeSet<_> = self.document.nodes.values()
            .filter(|node| node.kind == NodeKind::Component && depends_on(self.document, node.id, component))
            .map(|node| node.id).collect();
        remaining.insert(component);
        let mut count = 0;
        while !remaining.is_empty() {
            let ready = remaining.iter().copied().find(|id| {
                dependencies(self.document, *id).iter().all(|dependency| !remaining.contains(dependency))
            }).ok_or_else(|| invalid("Recursive component dependency"))?;
            remaining.remove(&ready);
            let instances: Vec<_> = self.document.nodes.keys().chain(self.changes.nodes.keys())
                .copied().collect::<BTreeSet<_>>().into_iter()
                .filter(|id| self.node(*id).is_some_and(|node| {
                    node.kind == NodeKind::Instance && node.component_id == Some(ready)
                })).collect();
            for instance in instances {
                if !self.node(instance).is_some_and(|node| node.kind == NodeKind::Instance && node.component_id == Some(ready)) {
                    continue;
                }
                self.reconcile(instance, ready)?;
                self.apply_properties(instance, ready)?;
                count += 1;
            }
        }
        Ok(count)
    }
}

impl Editor {
    pub fn create_component_set(&mut self, variants: &[NodeId], name: &str) -> Result<NodeId, DocumentError> {
        self.edit("Combine as component variants", |editor| {
            let id = editor.document.create_component_set(variants, name)?;
            editor.selection = vec![id];
            Ok(id)
        })
    }
    pub fn add_component_variant(&mut self, set: NodeId, template: NodeId) -> Result<NodeId, DocumentError> {
        self.edit("Add component variant", |editor| {
            let id = editor.document.add_component_variant(set, template)?;
            editor.selection = vec![id];
            Ok(id)
        })
    }
    pub fn set_variant_properties(&mut self, component: NodeId, properties: BTreeMap<String, String>) -> Result<(), DocumentError> {
        if self.document.effective_locked(component) { return Err(invalid("Component is locked")); }
        self.transact("Edit variant properties", |document| document.set_variant_properties(component, properties))
    }
    pub fn swap_instance_component(&mut self, instance: NodeId, component: NodeId) -> Result<(), DocumentError> {
        if self.document.effective_locked(instance) { return Err(invalid("Instance is locked")); }
        self.transact("Swap component variant", |document| document.swap_instance_component(instance, component))
    }
    pub fn set_instance_variant(&mut self, instance: NodeId, properties: &BTreeMap<String, String>) -> Result<NodeId, DocumentError> {
        if self.document.effective_locked(instance) { return Err(invalid("Instance is locked")); }
        self.transact("Change variant properties", |document| document.set_instance_variant(instance, properties))
    }
    pub fn set_instance_property(&mut self, instance: NodeId, key: &str, value: Option<ComponentPropertyValue>) -> Result<(), DocumentError> {
        if self.document.effective_locked(instance) { return Err(invalid("Instance is locked")); }
        self.transact("Change exposed component property", |document| document.set_instance_property(instance, key, value))
    }
    pub fn set_component_property_definition(&mut self, component: NodeId, key: &str, definition: ComponentPropertyDefinition) -> Result<(), DocumentError> {
        if self.document.effective_locked(component) { return Err(invalid("Component is locked")); }
        self.transact("Edit component property", |document| document.set_component_property_definition(component, key, definition))
    }
    pub fn remove_component_property(&mut self, component: NodeId, key: &str) -> Result<bool, DocumentError> {
        if self.document.effective_locked(component) { return Err(invalid("Component is locked")); }
        self.transact("Remove component property", |document| document.remove_component_property(component, key))
    }
    pub fn refresh_component_instances(&mut self, component: NodeId) -> Result<usize, DocumentError> {
        self.transact("Update component instances", |document| document.refresh_component_instances(component))
    }
}

pub fn variant_label(properties: &BTreeMap<String, String>) -> String {
    properties.iter().map(|(key, value)| format!("{key}={value}")).collect::<Vec<_>>().join(", ")
}

pub(crate) fn unique_variant_properties(document: &Document, set: NodeId, properties: &BTreeMap<String, String>) -> BTreeMap<String, String> {
    let mut properties = properties.clone();
    let axis = if properties.contains_key("State") { "State".to_string() }
        else { properties.keys().next().cloned().unwrap_or_else(|| "State".into()) };
    let base = properties.get(&axis).cloned().unwrap_or_else(|| "Variant".into());
    let mut number = 2;
    while properties.is_empty() || document.component_variants(set).iter().any(|id| document.nodes[id].variant_properties == properties) {
        properties.insert(axis.clone(), format!("{base} {number}"));
        number += 1;
    }
    properties
}

#[cfg(test)]
mod tests {
    use super::*;

    fn component(document: &mut Document, parent: NodeId, name: &str, x: f64) -> (NodeId, NodeId) {
        let root = document.insert_node(parent, Node::new(NodeKind::Component, name, x, 20.0, 160.0, 48.0)).unwrap();
        let mut label = Node::new(NodeKind::Text, "Label", 12.0, 8.0, 130.0, 32.0);
        label.text = name.into();
        let label = document.insert_node(root, label).unwrap();
        (root, label)
    }
    fn expose(document: &mut Document, component: NodeId, key: &str, value: ComponentPropertyValue, target: NodeId) {
        document.set_component_property_definition(component, key, ComponentPropertyDefinition {
            name: key.into(), default_value: value, targets: vec![target],
        }).unwrap();
    }
    fn label(document: &Document, instance: NodeId) -> NodeId {
        document.descendants(instance).into_iter().find(|id| document.nodes[id].kind == NodeKind::Text && document.nodes[id].name == "Label").unwrap()
    }
    fn roundtrip(document: &Document) {
        document.validate().unwrap();
        let json = document.to_json().unwrap();
        assert_eq!(Document::from_json(&json).unwrap().to_json().unwrap(), json);
    }

    #[test]
    fn component_set_grouping_and_variant_copy_preserve_geometry_and_bindings() {
        let mut document = Document::new("Components");
        let page = document.pages[0].id;
        let parent = document.insert_node(page, Node::new(NodeKind::Frame, "Rotated", 80.0, 50.0, 700.0, 500.0)).unwrap();
        document.nodes.get_mut(&parent).unwrap().rotation = 19.0;
        let (a, a_label) = component(&mut document, parent, "Default", 20.0);
        let (b, _) = component(&mut document, parent, "Hover", 230.0);
        document.nodes.get_mut(&a).unwrap().rotation = -8.0;
        expose(&mut document, a, "title", ComponentPropertyValue::Text("Hello".into()), a_label);
        let world = [document.world_transform(a).unwrap(), document.world_transform(b).unwrap()];
        let set = document.create_component_set(&[a, b], "Button").unwrap();
        for (id, original) in [a, b].into_iter().zip(world) {
            let current = document.world_transform(id).unwrap();
            for point in [Point::default(), Point::new(17.0, 23.0)] {
                assert!(current.apply(point).distance(original.apply(point)) < 1e-7);
            }
        }
        let copy = document.add_component_variant(set, a).unwrap();
        let copy_label = label(&document, copy);
        assert_ne!(copy_label, a_label);
        assert_eq!(document.nodes[&copy].component_property_definitions["title"].targets, vec![copy_label]);
        assert_ne!(document.nodes[&copy].variant_properties, document.nodes[&a].variant_properties);
        roundtrip(&document);
    }

    #[test]
    fn variant_switch_preserves_matching_ids_manual_overrides_and_exact_undo() {
        let mut document = Document::new("Variants");
        let page = document.pages[0].id;
        let (a, _) = component(&mut document, page, "Default", 0.0);
        let (b, b_label) = component(&mut document, page, "Hover", 200.0);
        let removed = document.insert_node(a, Node::new(NodeKind::Rectangle, "Removed", 1.0, 1.0, 3.0, 3.0)).unwrap();
        document.nodes.get_mut(&b).unwrap().interactions.push(Interaction {
            trigger: Trigger::Click, action: PrototypeAction::ScrollTo { target: b_label },
            transition: Transition::default(), delay_ms: 0, key: None,
        });
        let set = document.create_component_set(&[a, b], "Button").unwrap();
        let mut editor = Editor::new(document);
        let instance = editor.create_instance(a, 50.0, 200.0).unwrap();
        let old_label = label(&editor.document, instance);
        let old_removed = editor.document.nodes[&instance].children.iter().copied()
            .find(|id| editor.document.nodes[id].component_source == Some(removed)).unwrap();
        editor.update_node(old_label, "Manual title", |node| {
            node.text = "Keep this 用户".into();
            node.fills = vec![Fill::solid(Color::rgb(10, 20, 30))];
            node.overridden_properties.extend(["text".into(), "fills".into()]);
        });
        let before = editor.document.to_json().unwrap();
        let properties = editor.document.nodes[&b].variant_properties.clone();
        assert_eq!(editor.set_instance_variant(instance, &properties).unwrap(), b);
        assert_eq!(editor.document.component_set_of(instance), Some(set));
        assert_eq!(label(&editor.document, instance), old_label);
        assert_eq!(editor.document.nodes[&old_label].text, "Keep this 用户");
        assert_eq!(editor.document.nodes[&old_label].fills[0], Fill::solid(Color::rgb(10, 20, 30)));
        assert!(!editor.document.nodes.contains_key(&old_removed));
        assert_eq!(editor.document.nodes[&instance].x, 50.0);
        assert_eq!(editor.document.nodes[&instance].interactions[0].action, PrototypeAction::ScrollTo { target: old_label });
        let after = editor.document.to_json().unwrap();
        assert!(editor.undo());
        assert_eq!(editor.document.to_json().unwrap(), before);
        assert!(editor.redo());
        assert_eq!(editor.document.to_json().unwrap(), after);
        roundtrip(&editor.document);
    }

    #[test]
    fn exposed_text_boolean_reset_and_invalid_types_are_atomic() {
        let mut document = Document::new("Properties");
        let page = document.pages[0].id;
        let (master, source_label) = component(&mut document, page, "Default", 0.0);
        document.nodes.get_mut(&source_label).unwrap().text_runs.push(TextRun {
            start: 0, end: 7, font_family: None, font_size: Some(22.0), font_weight: Some(700),
            color: None, italic: None, decoration: None,
        });
        expose(&mut document, master, "title", ComponentPropertyValue::Text("Default".into()), source_label);
        expose(&mut document, master, "show", ComponentPropertyValue::Boolean(true), source_label);
        let instance = document.create_component_instance(master, page, 0.0, 150.0).unwrap();
        let target = label(&document, instance);
        document.set_instance_property(instance, "title", Some(ComponentPropertyValue::Text("東京 👩🏽‍💻".into()))).unwrap();
        document.set_instance_property(instance, "show", Some(ComponentPropertyValue::Boolean(false))).unwrap();
        assert_eq!(document.nodes[&target].text, "東京 👩🏽‍💻");
        assert!(!document.nodes[&target].visible);
        assert_eq!(document.nodes[&target].text_runs[0].font_weight, Some(700));
        let unchanged = document.to_json().unwrap();
        assert!(document.set_instance_property(instance, "show", Some(ComponentPropertyValue::Text("wrong".into()))).is_err());
        assert_eq!(document.to_json().unwrap(), unchanged);
        document.set_instance_property(instance, "title", None).unwrap();
        document.set_instance_property(instance, "show", None).unwrap();
        assert_eq!(document.nodes[&target].text, "Default");
        assert!(document.nodes[&target].visible);
        assert!(document.nodes[&instance].component_property_values.is_empty());
        roundtrip(&document);
    }

    #[test]
    fn nested_instance_swap_preserves_ids_overrides_and_outer_source_identity() {
        let mut document = Document::new("Swap");
        let page = document.pages[0].id;
        let (icon_a, _) = component(&mut document, page, "A", 0.0);
        let (icon_b, _) = component(&mut document, page, "B", 200.0);
        let (button, _) = component(&mut document, page, "Button", 400.0);
        let source_icon = document.create_component_instance(icon_a, button, 4.0, 4.0).unwrap();
        document.nodes.get_mut(&source_icon).unwrap().name = "Icon".into();
        expose(&mut document, button, "icon", ComponentPropertyValue::InstanceSwap(icon_a), source_icon);
        let instance = document.create_component_instance(button, page, 20.0, 300.0).unwrap();
        let target_icon = document.nodes[&instance].children.iter().copied()
            .find(|id| document.nodes[id].component_source == Some(source_icon)).unwrap();
        let target_label = label(&document, target_icon);
        let layer = document.nodes.get_mut(&target_label).unwrap();
        layer.text = "Custom icon label".into();
        layer.overridden_properties.insert("text".into());
        document.set_instance_property(instance, "icon", Some(ComponentPropertyValue::InstanceSwap(icon_b))).unwrap();
        assert_eq!(document.nodes[&target_icon].component_id, Some(icon_b));
        assert_eq!(document.nodes[&target_icon].component_source, Some(source_icon));
        assert_eq!(label(&document, target_icon), target_label);
        assert_eq!(document.nodes[&target_label].text, "Custom icon label");
        let unchanged = document.to_json().unwrap();
        assert!(document.set_instance_property(instance, "icon", Some(ComponentPropertyValue::InstanceSwap(button))).is_err());
        assert_eq!(document.to_json().unwrap(), unchanged);
        document.set_instance_property(instance, "icon", None).unwrap();
        assert_eq!(document.nodes[&target_icon].component_id, Some(icon_a));
        assert_eq!(document.nodes[&target_icon].component_source, Some(source_icon));
        assert_eq!(label(&document, target_icon), target_label);
        roundtrip(&document);
    }

    #[test]
    fn dependent_component_sync_updates_nested_defaults_but_keeps_user_overrides() {
        let mut document = Document::new("Nested sync");
        let page = document.pages[0].id;
        let (inner, inner_label) = component(&mut document, page, "Before", 0.0);
        let (outer, _) = component(&mut document, page, "Outer", 200.0);
        let nested = document.create_component_instance(inner, outer, 5.0, 5.0).unwrap();
        let first = document.create_component_instance(outer, page, 0.0, 300.0).unwrap();
        let second = document.create_component_instance(outer, page, 200.0, 300.0).unwrap();
        let find_nested = |document: &Document, root: NodeId| document.nodes[&root].children.iter().copied()
            .find(|id| document.nodes[id].component_source == Some(nested)).unwrap();
        let first_label = label(&document, find_nested(&document, first));
        let second_label = label(&document, find_nested(&document, second));
        document.nodes.get_mut(&first_label).unwrap().text = "Keep".into();
        document.nodes.get_mut(&first_label).unwrap().overridden_properties.insert("text".into());
        document.nodes.get_mut(&inner_label).unwrap().text = "After".into();
        assert!(document.refresh_component_instances(inner).unwrap() >= 3);
        assert_eq!(document.nodes[&label(&document, nested)].text, "After");
        assert_eq!(document.nodes[&first_label].text, "Keep");
        assert_eq!(document.nodes[&second_label].text, "After");
        roundtrip(&document);
    }

    #[test]
    fn manual_nested_swap_survives_containing_component_refresh() {
        let mut document = Document::new("Manual nested swap");
        let page = document.pages[0].id;
        let (a, _) = component(&mut document, page, "A", 0.0);
        let (b, _) = component(&mut document, page, "B", 200.0);
        let (outer, _) = component(&mut document, page, "Outer", 400.0);
        let nested = document.create_component_instance(a, outer, 0.0, 0.0).unwrap();
        let instance = document.create_component_instance(outer, page, 0.0, 200.0).unwrap();
        let target = document.nodes[&instance].children.iter().copied()
            .find(|id| document.nodes[id].component_source == Some(nested)).unwrap();
        let original_label = label(&document, target);
        document.swap_instance_component(target, b).unwrap();
        document.refresh_component_instances(outer).unwrap();
        assert_eq!(document.nodes[&target].component_id, Some(b));
        assert_eq!(document.nodes[&target].component_source, Some(nested));
        assert_eq!(label(&document, target), original_label);
        assert_eq!(document.nodes[&original_label].text, "B");
        roundtrip(&document);
    }

    #[test]
    fn exposed_views_and_reset_follow_direct_edits_over_stale_stored_values() {
        let mut document = Document::new("Effective exposed values");
        let page = document.pages[0].id;
        let (icon_a, _) = component(&mut document, page, "A", 0.0);
        let (icon_b, _) = component(&mut document, page, "B", 200.0);
        let (master, source_label) = component(&mut document, page, "Default", 400.0);
        let source_icon = document.create_component_instance(icon_a, master, 0.0, 0.0).unwrap();
        expose(&mut document, master, "title", ComponentPropertyValue::Text("Default".into()), source_label);
        expose(&mut document, master, "show", ComponentPropertyValue::Boolean(true), source_label);
        expose(&mut document, master, "icon", ComponentPropertyValue::InstanceSwap(icon_a), source_icon);
        let instance = document.create_component_instance(master, page, 0.0, 300.0).unwrap();
        let target_label = label(&document, instance);
        let target_icon = document.nodes[&instance].children.iter().copied()
            .find(|id| document.nodes[id].component_source == Some(source_icon)).unwrap();
        let icon_label = label(&document, target_icon);
        document.set_instance_property(instance, "title", Some(ComponentPropertyValue::Text("Earlier value".into()))).unwrap();
        document.set_instance_property(instance, "show", Some(ComponentPropertyValue::Boolean(true))).unwrap();
        document.set_instance_property(instance, "icon", Some(ComponentPropertyValue::InstanceSwap(icon_b))).unwrap();
        let direct = document.nodes.get_mut(&target_label).unwrap();
        direct.text = "Direct 東京 🦀".into();
        direct.visible = false;
        direct.overridden_properties.extend(["text".into(), "visible".into()]);
        document.swap_instance_component(target_icon, icon_a).unwrap();
        let effective: BTreeMap<_, _> = document.component_property_views(instance).into_iter()
            .map(|view| { assert!(view.overridden); (view.key, view.value) }).collect();
        assert_eq!(effective["title"], ComponentPropertyValue::Text("Direct 東京 🦀".into()));
        assert_eq!(effective["show"], ComponentPropertyValue::Boolean(false));
        assert_eq!(effective["icon"], ComponentPropertyValue::InstanceSwap(icon_a));
        document.refresh_component_instances(master).unwrap();
        assert_eq!(document.nodes[&target_label].text, "Direct 東京 🦀");
        assert!(!document.nodes[&target_label].visible);
        assert_eq!(document.nodes[&target_icon].component_id, Some(icon_a));
        assert_eq!(label(&document, target_icon), icon_label);
        let mut editor = Editor::new(document);
        let before = editor.document.to_json().unwrap();
        for key in ["title", "show", "icon"] { editor.set_instance_property(instance, key, None).unwrap(); }
        let after = editor.document.to_json().unwrap();
        assert_eq!(editor.document.nodes[&target_label].text, "Default");
        assert!(editor.document.nodes[&target_label].visible);
        assert_eq!(editor.document.nodes[&target_icon].component_id, Some(icon_a));
        assert!(!editor.document.nodes[&target_label].overridden_properties.contains("text"));
        assert!(!editor.document.nodes[&target_label].overridden_properties.contains("visible"));
        assert!(!editor.document.nodes[&target_icon].overridden_properties.contains("component_id"));
        assert!(editor.document.component_property_views(instance).iter().all(|view| !view.overridden));
        for _ in 0..3 { assert!(editor.undo()); }
        assert_eq!(editor.document.to_json().unwrap(), before);
        for _ in 0..3 { assert!(editor.redo()); }
        assert_eq!(editor.document.to_json().unwrap(), after);
        roundtrip(&editor.document);
    }

    #[test]
    fn authoring_exposed_value_clears_only_its_direct_flags_on_all_bound_layers() {
        let mut document = Document::new("Bound field ownership");
        let page = document.pages[0].id;
        let (master, first) = component(&mut document, page, "Default", 0.0);
        let second = document.insert_node(master, Node::new(NodeKind::Text, "Secondary", 0.0, 24.0, 80.0, 24.0).with_text("Default", 14.0, Color::BLACK)).unwrap();
        document.set_component_property_definition(master, "title", ComponentPropertyDefinition {
            name: "Title".into(), default_value: ComponentPropertyValue::Text("Default".into()), targets: vec![first, second],
        }).unwrap();
        expose(&mut document, master, "show", ComponentPropertyValue::Boolean(true), master);
        let instance = document.create_component_instance(master, page, 0.0, 150.0).unwrap();
        let children = document.nodes[&instance].children.clone();
        for child in &children {
            let node = document.nodes.get_mut(child).unwrap();
            node.text = "Manual".into();
            node.visible = false;
            node.overridden_properties.extend(["text".into(), "visible".into()]);
        }
        let root = document.nodes.get_mut(&instance).unwrap();
        root.visible = false;
        root.overridden_properties.insert("visible".into());
        document.set_instance_property(instance, "title", Some(ComponentPropertyValue::Text("Typed Ω".into()))).unwrap();
        for child in &children {
            let node = &document.nodes[child];
            assert_eq!(node.text, "Typed Ω");
            assert!(!node.overridden_properties.contains("text"));
            assert!(node.overridden_properties.contains("visible"));
            assert!(!node.visible);
        }
        assert!(!document.nodes[&instance].visible);
        document.set_instance_property(instance, "show", None).unwrap();
        assert!(document.nodes[&instance].visible);
        assert!(!document.nodes[&instance].overridden_properties.contains("visible"));
        assert_eq!(document.nodes[&instance].children, children);
        roundtrip(&document);
    }

    #[test]
    fn duplication_deletion_and_detach_keep_component_references_valid() {
        let mut document = Document::new("Reference cleanup");
        let page = document.pages[0].id;
        let (master, source_label) = component(&mut document, page, "Default", 0.0);
        expose(&mut document, master, "title", ComponentPropertyValue::Text("Default".into()), source_label);
        let set = document.create_component_set(&[master], "Button").unwrap();
        let mut editor = Editor::new(document);
        editor.selection = vec![master];
        let copied = editor.duplicate_selection()[0];
        assert_eq!(editor.document.nodes[&copied].component_property_definitions["title"].targets, vec![label(&editor.document, copied)]);
        assert_ne!(editor.document.nodes[&copied].variant_properties, editor.document.nodes[&master].variant_properties);
        let instance = editor.create_instance(master, 0.0, 200.0).unwrap();
        editor.set_instance_property(instance, "title", Some(ComponentPropertyValue::Text("User".into()))).unwrap();
        assert!(editor.detach_instance(instance));
        assert!(editor.document.nodes[&instance].component_property_values.is_empty());
        assert_eq!(editor.document.nodes[&instance].kind, NodeKind::Frame);
        editor.document.remove_subtree(source_label);
        assert!(editor.document.nodes[&master].component_property_definitions.is_empty());
        assert!(editor.document.is_component_set(set));
        roundtrip(&editor.document);
    }

    #[test]
    fn duplicate_named_siblings_keep_occurrence_overrides_across_variants() {
        let mut document = Document::new("Matching");
        let page = document.pages[0].id;
        let (a, _) = component(&mut document, page, "A", 0.0);
        let (b, _) = component(&mut document, page, "B", 200.0);
        for component in [a, b] {
            let mut second = Node::new(NodeKind::Text, "Label", 0.0, 20.0, 60.0, 20.0);
            second.text = "Second".into();
            document.insert_node(component, second).unwrap();
        }
        document.create_component_set(&[a, b], "Labels").unwrap();
        let instance = document.create_component_instance(a, page, 0.0, 200.0).unwrap();
        let ids = document.nodes[&instance].children.clone();
        for (index, id) in ids.iter().enumerate() {
            let node = document.nodes.get_mut(id).unwrap();
            node.text = format!("Override {index}");
            node.overridden_properties.insert("text".into());
        }
        document.swap_instance_component(instance, b).unwrap();
        assert_eq!(document.nodes[&instance].children, ids);
        for (index, id) in ids.iter().enumerate() { assert_eq!(document.nodes[id].text, format!("Override {index}")); }
        roundtrip(&document);
    }
}

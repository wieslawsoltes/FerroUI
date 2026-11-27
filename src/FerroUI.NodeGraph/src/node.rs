//! Extensible [`NodeComponent`] trait, evaluation context, and [`NodeRegistry`] with fuzzy search.

use crate::data_tree::{lace_item_inputs, DataTree, Value};
use crate::types::{DataMatching, NodeCategory, NodeKind, ParamAccess, ParamWidget, PortSpec};
use indexmap::IndexMap;
use std::sync::Arc;

/// Context passed into [`NodeComponent::evaluate`] during incremental DAG execution.
pub struct EvalContext<'a> {
    pub inputs: &'a [DataTree],
    pub outputs: Vec<DataTree>,
    pub widget: &'a ParamWidget,
    pub matching: DataMatching,
    pub time_secs: f64,
    pub warnings: Vec<String>,
}

impl<'a> EvalContext<'a> {
    pub fn new(
        inputs: &'a [DataTree],
        num_outputs: usize,
        widget: &'a ParamWidget,
        matching: DataMatching,
        time_secs: f64,
    ) -> Self {
        Self {
            inputs,
            outputs: (0..num_outputs).map(|_| DataTree::new()).collect(),
            widget,
            matching,
            time_secs,
            warnings: Vec::new(),
        }
    }

    pub fn input(&self, idx: usize) -> &DataTree {
        &self.inputs[idx]
    }

    pub fn set_output(&mut self, idx: usize, tree: DataTree) {
        if idx < self.outputs.len() {
            self.outputs[idx] = tree;
        }
    }

    pub fn set_output_single(&mut self, idx: usize, val: Value) {
        self.set_output(idx, DataTree::single(val));
    }

    pub fn warn(&mut self, msg: impl Into<String>) {
        self.warnings.push(msg.into());
    }

    /// Helper that automatically executes Grasshopper item lacing over all input ports.
    pub fn map_items(
        &mut self,
        f: impl FnMut(&[Value]) -> Result<Vec<Value>, String>,
    ) -> Result<(), String> {
        let out_count = self.outputs.len();
        self.outputs = lace_item_inputs(self.inputs, self.matching, f, out_count)?;
        Ok(())
    }
}

/// Implement this trait to add any custom node component to `ferroui-nodegraph`.
pub trait NodeComponent: Send + Sync {
    /// Stable globally unique identifier used for serialization (e.g. `"math.add"`, `"physics.rigid_body"`).
    fn type_id(&self) -> &str;
    /// Full human-readable name shown in palettes & tooltips (e.g. `"Addition"`).
    fn display_name(&self) -> &str;
    /// Compact Grasshopper capsule label (e.g. `"Add"`, `"Pt"`, `"Body"`).
    fn nickname(&self) -> &str;
    /// Ribbon tab category.
    fn category(&self) -> NodeCategory;
    /// Short description shown in search popup & inspector.
    fn description(&self) -> &str;
    /// Visual archetype on the canvas.
    fn kind(&self) -> NodeKind {
        NodeKind::Component
    }
    /// Default inline widget state when instantiated.
    fn default_widget(&self) -> ParamWidget {
        ParamWidget::None
    }
    /// Input port specifications.
    fn inputs(&self) -> Vec<PortSpec>;
    /// Output port specifications.
    fn outputs(&self) -> Vec<PortSpec>;
    /// Evaluates this component's inputs & widget into its outputs.
    fn evaluate(&self, ctx: &mut EvalContext<'_>) -> Result<(), String>;
}

/// Functional convenience wrapper for defining compact nodes with a closure!
pub struct FnComponent {
    pub type_id: String,
    pub display_name: String,
    pub nickname: String,
    pub category: NodeCategory,
    pub description: String,
    pub kind: NodeKind,
    pub default_widget: ParamWidget,
    pub inputs: Vec<PortSpec>,
    pub outputs: Vec<PortSpec>,
    pub eval_fn: Box<dyn Fn(&mut EvalContext<'_>) -> Result<(), String> + Send + Sync>,
}

impl NodeComponent for FnComponent {
    fn type_id(&self) -> &str {
        &self.type_id
    }
    fn display_name(&self) -> &str {
        &self.display_name
    }
    fn nickname(&self) -> &str {
        &self.nickname
    }
    fn category(&self) -> NodeCategory {
        self.category.clone()
    }
    fn description(&self) -> &str {
        &self.description
    }
    fn kind(&self) -> NodeKind {
        self.kind
    }
    fn default_widget(&self) -> ParamWidget {
        self.default_widget.clone()
    }
    fn inputs(&self) -> Vec<PortSpec> {
        self.inputs.clone()
    }
    fn outputs(&self) -> Vec<PortSpec> {
        self.outputs.clone()
    }
    fn evaluate(&self, ctx: &mut EvalContext<'_>) -> Result<(), String> {
        (self.eval_fn)(ctx)
    }
}

/// Metadata snapshot for palette & fuzzy search UI.
#[derive(Clone, Debug)]
pub struct ComponentMeta {
    pub type_id: String,
    pub display_name: String,
    pub nickname: String,
    pub category: NodeCategory,
    pub description: String,
    pub kind: NodeKind,
}

/// Central component registry supporting Grasshopper-style fuzzy double-click search and plugins.
#[derive(Clone, Default)]
pub struct NodeRegistry {
    components: IndexMap<String, Arc<dyn NodeComponent>>,
}

impl NodeRegistry {
    pub fn new() -> Self {
        Self { components: IndexMap::new() }
    }

    /// Creates a registry pre-populated with all built-in Grasshopper standard library components
    /// (and physics components when `feature = "physics"` is enabled).
    pub fn with_standard_library() -> Self {
        let mut reg = Self::new();
        crate::stdlib::register_standard_nodes(&mut reg);
        #[cfg(feature = "physics")]
        crate::physics_nodes::register_physics_nodes(&mut reg);
        reg
    }

    pub fn register<C: NodeComponent + 'static>(&mut self, component: C) {
        let id = component.type_id().to_string();
        self.components.insert(id, Arc::new(component));
    }

    pub fn register_arc(&mut self, component: Arc<dyn NodeComponent>) {
        let id = component.type_id().to_string();
        self.components.insert(id, component);
    }

    pub fn get(&self, type_id: &str) -> Option<&Arc<dyn NodeComponent>> {
        self.components.get(type_id)
    }

    pub fn len(&self) -> usize {
        self.components.len()
    }

    pub fn is_empty(&self) -> bool {
        self.components.is_empty()
    }

    pub fn all_meta(&self) -> Vec<ComponentMeta> {
        self.components
            .values()
            .map(|c| ComponentMeta {
                type_id: c.type_id().to_string(),
                display_name: c.display_name().to_string(),
                nickname: c.nickname().to_string(),
                category: c.category(),
                description: c.description().to_string(),
                kind: c.kind(),
            })
            .collect()
    }

    pub fn categories(&self) -> Vec<NodeCategory> {
        let mut cats = Vec::new();
        for c in self.components.values() {
            let cat = c.category();
            if !cats.contains(&cat) {
                cats.push(cat);
            }
        }
        cats
    }

    pub fn by_category(&self, category: &NodeCategory) -> Vec<ComponentMeta> {
        self.all_meta().into_iter().filter(|m| &m.category == category).collect()
    }

    /// Grasshopper-style fuzzy search across display name, nickname, type_id, category, and description.
    pub fn search(&self, query: &str) -> Vec<ComponentMeta> {
        let q = query.trim().to_ascii_lowercase();
        if q.is_empty() {
            return self.all_meta();
        }
        let mut scored: Vec<(i32, ComponentMeta)> = self
            .all_meta()
            .into_iter()
            .filter_map(|m| {
                let name = m.display_name.to_ascii_lowercase();
                let nick = m.nickname.to_ascii_lowercase();
                let tid = m.type_id.to_ascii_lowercase();
                let cat = m.category.label().to_ascii_lowercase();
                let desc = m.description.to_ascii_lowercase();

                let score = if nick == q || name == q {
                    100
                } else if name.starts_with(&q) || nick.starts_with(&q) {
                    80
                } else if name.contains(&q) || nick.contains(&q) {
                    60
                } else if tid.contains(&q) || cat.contains(&q) {
                    40
                } else if desc.contains(&q) {
                    20
                } else if subsequence_match(&q, &name) {
                    15
                } else {
                    return None;
                };
                Some((score, m))
            })
            .collect();
        scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.display_name.cmp(&b.1.display_name)));
        scored.into_iter().map(|(_, m)| m).collect()
    }

    /// Helper: returns true if a component uses only `ParamAccess::Item` on all inputs.
    pub fn all_inputs_are_item_access(&self, type_id: &str) -> bool {
        self.get(type_id)
            .map(|c| c.inputs().iter().all(|p| p.access == ParamAccess::Item))
            .unwrap_or(true)
    }
}

fn subsequence_match(needle: &str, haystack: &str) -> bool {
    let mut chars = needle.chars();
    let Some(mut target) = chars.next() else { return true };
    for ch in haystack.chars() {
        if ch == target {
            match chars.next() {
                Some(next) => target = next,
                None => return true,
            }
        }
    }
    false
}

//! Rhino Grasshopper-style **Cluster / Sub-Graph** encapsulation.
//!
//! Package any [`NodeGraph`] into a reusable [`NodeComponent`] with exposed input & output
//! parameter mappings so complex parametric recipes become single clean nodes!

use crate::data_tree::Value;
use crate::eval::evaluate_graph;
use crate::graph::NodeGraph;
use crate::node::{EvalContext, NodeComponent, NodeRegistry};
use crate::types::{NodeCategory, NodeId, ParamAccess, PortSpec, PortType};
use std::sync::Arc;

/// Maps an external input port of a [`ClusterComponent`] to an internal `(node_id, input_port)`.
#[derive(Clone, Debug)]
pub struct ClusterInputBinding {
    pub spec: PortSpec,
    pub target_node: NodeId,
    pub target_input_port: usize,
}

/// Maps an internal `(node_id, output_port)` to an external output port of a [`ClusterComponent`].
#[derive(Clone, Debug)]
pub struct ClusterOutputBinding {
    pub spec: PortSpec,
    pub source_node: NodeId,
    pub source_output_port: usize,
}

/// A reusable sub-graph component that evaluates an encapsulated [`NodeGraph`] on demand.
pub struct ClusterComponent {
    pub type_id: String,
    pub display_name: String,
    pub nickname: String,
    pub description: String,
    pub inner_graph: NodeGraph,
    pub inner_registry: Arc<NodeRegistry>,
    pub input_bindings: Vec<ClusterInputBinding>,
    pub output_bindings: Vec<ClusterOutputBinding>,
}

impl ClusterComponent {
    pub fn new(
        type_id: impl Into<String>,
        display_name: impl Into<String>,
        nickname: impl Into<String>,
        description: impl Into<String>,
        inner_graph: NodeGraph,
        inner_registry: Arc<NodeRegistry>,
        input_bindings: Vec<ClusterInputBinding>,
        output_bindings: Vec<ClusterOutputBinding>,
    ) -> Self {
        Self {
            type_id: type_id.into(),
            display_name: display_name.into(),
            nickname: nickname.into(),
            description: description.into(),
            inner_graph,
            inner_registry,
            input_bindings,
            output_bindings,
        }
    }
}

impl NodeComponent for ClusterComponent {
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
        NodeCategory::Cluster
    }
    fn description(&self) -> &str {
        &self.description
    }
    fn inputs(&self) -> Vec<PortSpec> {
        self.input_bindings
            .iter()
            .map(|b| {
                let mut s = b.spec.clone();
                s.access = ParamAccess::Tree;
                s
            })
            .collect()
    }
    fn outputs(&self) -> Vec<PortSpec> {
        self.output_bindings
            .iter()
            .map(|b| {
                let mut s = b.spec.clone();
                s.access = ParamAccess::Tree;
                s
            })
            .collect()
    }

    fn evaluate(&self, ctx: &mut EvalContext<'_>) -> Result<(), String> {
        let mut g = self.inner_graph.clone();
        // Inject outer input DataTrees into bound inner nodes' default overrides (first item) or widget
        for (i, bind) in self.input_bindings.iter().enumerate() {
            let tree = ctx.input(i);
            if let Some(first) = tree.first_item() {
                g.set_input_default(bind.target_node, bind.target_input_port, first.clone());
                if let Some(node) = g.nodes.get_mut(&bind.target_node) {
                    if let crate::types::ParamWidget::Slider { ref mut value, min, max, .. } = node.widget {
                        if let Some(f) = first.as_float() {
                            *value = f.clamp(min, max);
                        }
                    }
                }
            }
        }
        g.mark_all_dirty();
        let report = evaluate_graph(&mut g, &self.inner_registry, ctx.time_secs);
        if report.errors > 0 {
            return Err(format!("Cluster `{}` failed with {} inner errors", self.display_name, report.errors));
        }
        for (o, bind) in self.output_bindings.iter().enumerate() {
            if let Some(src) = g.nodes.get(&bind.source_node) {
                if let Some(tree) = src.output_tree(bind.source_output_port) {
                    ctx.set_output(o, tree.clone());
                }
            }
        }
        Ok(())
    }
}

/// Convenience helper for building a quick [`PortSpec`] for cluster boundaries.
pub fn cluster_port(nick: &str, name: &str, port_type: PortType) -> PortSpec {
    PortSpec::new(nick, name, port_type, Value::Null).with_access(ParamAccess::Tree)
}

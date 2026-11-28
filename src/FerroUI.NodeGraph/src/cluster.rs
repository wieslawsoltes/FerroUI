//! Rhino Grasshopper-style **Cluster / Sub-Graph** encapsulation.
//!
//! Package any [`NodeGraph`] into a reusable [`NodeComponent`] with exposed input & output
//! parameter mappings so complex parametric recipes become single clean nodes!

use crate::data_tree::Value;
use crate::eval::evaluate_graph_with_inputs;
use crate::graph::NodeGraph;
use crate::node::{EvalContext, NodeComponent, NodeRegistry};
use crate::types::{NodeCategory, NodeId, ParamAccess, PortSpec, PortType};
use rustc_hash::FxHashMap;
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

    fn is_time_dependent(&self, _: &crate::types::ParamWidget) -> bool {
        self.inner_graph.nodes.values().any(|node| {
            !node.disabled
                && self
                    .inner_registry
                    .get(&node.type_id)
                    .is_some_and(|component| component.is_time_dependent(&node.widget))
        })
    }

    fn evaluate(&self, ctx: &mut EvalContext<'_>) -> Result<(), String> {
        let mut graph = self.inner_graph.clone_definition();
        graph.solution_locked = false;
        let mut bound_inputs = FxHashMap::default();
        for (port, binding) in self.input_bindings.iter().enumerate() {
            let Some(target) = graph.nodes.get(&binding.target_node) else {
                return Err(format!("Cluster input {port} references a missing node"));
            };
            if binding.target_input_port >= target.input_modifiers.len() {
                return Err(format!("Cluster input {port} references an invalid port"));
            }
            bound_inputs.insert(
                (binding.target_node, binding.target_input_port),
                ctx.input(port),
            );
        }
        let report = evaluate_graph_with_inputs(
            &mut graph,
            &self.inner_registry,
            ctx.time_secs,
            &bound_inputs,
        );
        if report.errors > 0 {
            return Err(format!(
                "Cluster `{}` failed with {} inner errors",
                self.display_name, report.errors
            ));
        }
        for (o, bind) in self.output_bindings.iter().enumerate() {
            let tree = graph
                .nodes
                .get(&bind.source_node)
                .and_then(|source| source.output_tree(bind.source_output_port))
                .ok_or_else(|| format!("Cluster output {o} references an invalid port"))?;
            ctx.set_output(o, tree.clone());
        }
        Ok(())
    }
}

/// Convenience helper for building a quick [`PortSpec`] for cluster boundaries.
pub fn cluster_port(nick: &str, name: &str, port_type: PortType) -> PortSpec {
    PortSpec::new(nick, name, port_type, Value::Null).with_access(ParamAccess::Tree)
}

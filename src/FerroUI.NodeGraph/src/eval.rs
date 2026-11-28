//! Incremental topological DAG evaluator with Grasshopper DataTree modifiers,
//! multi-wire stream merging, error/warning propagation, and per-node profiling.

use crate::data_tree::DataTree;
use crate::graph::NodeGraph;
use crate::node::{EvalContext, NodeRegistry};
use crate::types::{NodeId, NodeState, ParamWidget};
use rustc_hash::FxHashMap;
use std::collections::VecDeque;
use std::time::Instant;

/// Summary returned by [`evaluate_graph`].
#[derive(Clone, Debug, Default)]
pub struct EvalReport {
    pub evaluated_nodes: usize,
    pub total_nodes: usize,
    pub total_us: u64,
    pub warnings: usize,
    pub errors: usize,
}

/// Edge indices are rebuilt once per solution because the document's maps are public.
/// Keeping them local avoids stale caches after callers edit those maps directly.
#[derive(Default)]
struct EdgeIndex {
    incoming: FxHashMap<(NodeId, usize), Vec<(NodeId, usize)>>,
    outgoing: FxHashMap<NodeId, Vec<NodeId>>,
    in_degree: FxHashMap<NodeId, usize>,
}

impl EdgeIndex {
    fn new(graph: &NodeGraph) -> Self {
        let mut index = Self {
            in_degree: graph.nodes.keys().map(|&id| (id, 0)).collect(),
            ..Self::default()
        };
        for wire in graph.wires.values() {
            if graph.nodes.contains_key(&wire.from_node) && graph.nodes.contains_key(&wire.to_node)
            {
                index
                    .incoming
                    .entry((wire.to_node, wire.to_port))
                    .or_default()
                    .push((wire.from_node, wire.from_port));
                index
                    .outgoing
                    .entry(wire.from_node)
                    .or_default()
                    .push(wire.to_node);
                *index.in_degree.get_mut(&wire.to_node).unwrap() += 1;
            }
        }
        index
    }

    fn order(&self, graph: &NodeGraph) -> (Vec<NodeId>, Vec<NodeId>) {
        let mut degrees = self.in_degree.clone();
        let mut queue: VecDeque<NodeId> = graph
            .nodes
            .keys()
            .copied()
            .filter(|id| degrees[id] == 0)
            .collect();
        let mut order = Vec::with_capacity(graph.nodes.len());
        while let Some(id) = queue.pop_front() {
            order.push(id);
            if let Some(children) = self.outgoing.get(&id) {
                for child in children {
                    let degree = degrees.get_mut(child).unwrap();
                    *degree -= 1;
                    if *degree == 0 {
                        queue.push_back(*child);
                    }
                }
            }
        }
        let blocked = graph
            .nodes
            .keys()
            .copied()
            .filter(|id| degrees[id] != 0)
            .collect();
        (order, blocked)
    }
}

/// Computes a deterministic topological execution order of all nodes in `graph`.
/// On failure, returns the nodes in or downstream of a cycle.
pub fn topological_order(graph: &NodeGraph) -> Result<Vec<NodeId>, Vec<NodeId>> {
    let (order, blocked) = EdgeIndex::new(graph).order(graph);
    if blocked.is_empty() {
        Ok(order)
    } else {
        Err(blocked)
    }
}

/// Incrementally evaluates dirty and time-dependent nodes, including their dependents.
/// Graph traversal and input lookup are O(nodes + wires), excluding component evaluation
/// and the data copied into input streams. Clean nodes do not clone widgets or port metadata.
pub fn evaluate_graph(
    graph: &mut NodeGraph,
    registry: &NodeRegistry,
    time_secs: f64,
) -> EvalReport {
    evaluate_graph_with_inputs(graph, registry, time_secs, &FxHashMap::default())
}

/// Cluster boundaries inject whole trees instead of discarding every item after the first.
pub(crate) fn evaluate_graph_with_inputs(
    graph: &mut NodeGraph,
    registry: &NodeRegistry,
    time_secs: f64,
    bound_inputs: &FxHashMap<(NodeId, usize), &DataTree>,
) -> EvalReport {
    let start = Instant::now();
    let mut report = EvalReport {
        total_nodes: graph.nodes.len(),
        ..Default::default()
    };
    if graph.solution_locked {
        report.total_us = start.elapsed().as_micros() as u64;
        return report;
    }

    let index = EdgeIndex::new(graph);
    let (order, blocked) = index.order(graph);
    for id in blocked {
        let node = graph.nodes.get_mut(&id).unwrap();
        node.cached_outputs
            .iter_mut()
            .for_each(|tree| *tree = DataTree::new());
        node.state = NodeState::Error("Cyclic wire dependency".into());
        node.eval_us = 0;
        // Keep blocked nodes dirty so removing an externally edited cycle recovers.
        node.dirty = true;
        report.errors += 1;
    }

    for id in order {
        let node = &graph.nodes[&id];
        let component = registry.get(&node.type_id);
        let time_dependent = component.is_some_and(|c| c.is_time_dependent(&node.widget));
        if !node.dirty && (!time_dependent || node.disabled) {
            match &node.state {
                NodeState::Warning(_) => report.warnings += 1,
                NodeState::Error(_) => report.errors += 1,
                _ => {}
            }
            continue;
        }

        let node_start = Instant::now();
        let (outputs, state) = if node.disabled {
            (
                vec![DataTree::new(); node.output_modifiers.len()],
                NodeState::Disabled,
            )
        } else if let Some(component) = component {
            let input_specs = component.inputs();
            let output_count = node.output_modifiers.len();
            let mut inputs = Vec::with_capacity(input_specs.len());
            let mut missing_required = None;
            for (port, spec) in input_specs.iter().enumerate() {
                let mut merged = DataTree::new();
                if let Some(bound) = bound_inputs.get(&(id, port)) {
                    merged.merge_from(bound);
                } else if let Some(sources) = index.incoming.get(&(id, port)) {
                    for &(source_id, source_port) in sources {
                        if let Some(tree) = graph.nodes[&source_id].cached_outputs.get(source_port)
                        {
                            merged.merge_from(tree);
                        }
                    }
                } else if let Some(Some(value)) = node.input_defaults.get(port) {
                    merged = DataTree::single(value.clone());
                } else if !spec.default_value.is_null() {
                    merged = DataTree::single(spec.default_value.clone());
                }
                if merged.is_empty() && !spec.optional {
                    missing_required =
                        Some(format!("Input `{}` failed to collect data", spec.name));
                }
                if let Some(&modifier) = node.input_modifiers.get(port) {
                    if !modifier.is_empty() {
                        merged = merged.apply_modifier(modifier);
                    }
                }
                inputs.push(merged);
            }

            if let Some(message) = missing_required {
                (
                    vec![DataTree::new(); output_count],
                    NodeState::Warning(message),
                )
            } else {
                let mut ctx = EvalContext::new(
                    &inputs,
                    output_count,
                    &node.widget,
                    node.matching,
                    time_secs,
                );
                match component.evaluate(&mut ctx) {
                    Ok(()) => {
                        for (port, output) in ctx.outputs.iter_mut().enumerate() {
                            if let Some(&modifier) = node.output_modifiers.get(port) {
                                if !modifier.is_empty() {
                                    *output = output.apply_modifier(modifier);
                                }
                            }
                        }
                        let state = ctx
                            .warnings
                            .into_iter()
                            .next()
                            .map(NodeState::Warning)
                            .unwrap_or(NodeState::Normal);
                        (ctx.outputs, state)
                    }
                    Err(message) => (
                        vec![DataTree::new(); output_count],
                        NodeState::Error(message),
                    ),
                }
            }
        } else {
            (
                vec![DataTree::new(); node.output_modifiers.len()],
                NodeState::Error(format!("Unknown component `{}`", node.type_id)),
            )
        };

        match &state {
            NodeState::Warning(_) => report.warnings += 1,
            NodeState::Error(_) => report.errors += 1,
            _ => {}
        }
        let node = graph.nodes.get_mut(&id).unwrap();
        node.cached_outputs = outputs;
        node.state = state;
        node.eval_us = node_start.elapsed().as_micros() as u64;
        node.dirty = false;
        if let ParamWidget::Button { pressed } = &mut node.widget {
            if *pressed {
                *pressed = false;
                node.dirty = true;
            }
        }
        report.evaluated_nodes += 1;

        // Topological order guarantees that dependents have not yet been visited.
        // This also propagates empty/error/disabled outputs, clearing stale previews.
        if let Some(children) = index.outgoing.get(&id) {
            for child in children {
                graph.nodes.get_mut(child).unwrap().dirty = true;
            }
        }
    }
    report.total_us = start.elapsed().as_micros() as u64;
    report
}

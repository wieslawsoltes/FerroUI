//! Incremental topological DAG evaluator with Grasshopper DataTree modifiers,
//! multi-wire stream merging, error/warning propagation, and per-node profiling.

use crate::data_tree::DataTree;
use crate::graph::NodeGraph;
use crate::node::{EvalContext, NodeRegistry};
use crate::types::{NodeId, NodeState};
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

/// Computes a deterministic topological execution order of all nodes in `graph`.
pub fn topological_order(graph: &NodeGraph) -> Result<Vec<NodeId>, Vec<NodeId>> {
    let mut in_deg: FxHashMap<NodeId, usize> = graph.nodes.keys().map(|&id| (id, 0)).collect();
    let mut adj: FxHashMap<NodeId, Vec<NodeId>> = FxHashMap::default();
    for w in graph.wires.values() {
        if graph.nodes.contains_key(&w.from_node) && graph.nodes.contains_key(&w.to_node) {
            adj.entry(w.from_node).or_default().push(w.to_node);
            *in_deg.entry(w.to_node).or_default() += 1;
        }
    }

    let mut queue: VecDeque<NodeId> = graph
        .nodes
        .keys()
        .copied()
        .filter(|id| in_deg[id] == 0)
        .collect();
    let mut order = Vec::with_capacity(graph.nodes.len());

    while let Some(u) = queue.pop_front() {
        order.push(u);
        if let Some(nbrs) = adj.get(&u) {
            for &v in nbrs {
                let d = in_deg.get_mut(&v).unwrap();
                *d -= 1;
                if *d == 0 {
                    queue.push_back(v);
                }
            }
        }
    }

    if order.len() == graph.nodes.len() {
        Ok(order)
    } else {
        let cyclic: Vec<NodeId> = graph
            .nodes
            .keys()
            .copied()
            .filter(|id| !order.contains(id))
            .collect();
        Err(cyclic)
    }
}

/// Incrementally evaluates all dirty nodes in topological order, updating each node's
/// `cached_outputs`, `state`, and `eval_us`.
pub fn evaluate_graph(graph: &mut NodeGraph, registry: &NodeRegistry, time_secs: f64) -> EvalReport {
    let start = Instant::now();
    let mut report = EvalReport { total_nodes: graph.nodes.len(), ..Default::default() };
    if graph.solution_locked {
        report.total_us = start.elapsed().as_micros() as u64;
        return report;
    }

    let order = match topological_order(graph) {
        Ok(o) => o,
        Err(cyclic) => {
            for id in cyclic {
                if let Some(n) = graph.nodes.get_mut(&id) {
                    n.state = NodeState::Error("Cyclic wire dependency".into());
                    report.errors += 1;
                }
            }
            report.total_us = start.elapsed().as_micros() as u64;
            return report;
        }
    };

    for id in order {
        let (type_id, is_dirty, is_disabled, matching, widget, in_mods, out_mods, in_defaults) = {
            let Some(n) = graph.nodes.get(&id) else { continue };
            (
                n.type_id.clone(),
                n.dirty,
                n.disabled,
                n.matching,
                n.widget.clone(),
                n.input_modifiers.clone(),
                n.output_modifiers.clone(),
                n.input_defaults.clone(),
            )
        };

        // Always re-evaluate time-dependent nodes
        let force_eval = type_id == "params.time";
        if !is_dirty && !force_eval {
            if let Some(n) = graph.nodes.get(&id) {
                match &n.state {
                    NodeState::Warning(_) => report.warnings += 1,
                    NodeState::Error(_) => report.errors += 1,
                    _ => {}
                }
            }
            continue;
        }

        let Some(comp) = registry.get(&type_id) else {
            if let Some(n) = graph.nodes.get_mut(&id) {
                n.state = NodeState::Error(format!("Unknown component `{type_id}`"));
                n.dirty = false;
                report.errors += 1;
            }
            continue;
        };

        let in_specs = comp.inputs();
        let out_specs = comp.outputs();

        if is_disabled {
            if let Some(n) = graph.nodes.get_mut(&id) {
                n.cached_outputs = vec![DataTree::new(); out_specs.len()];
                n.state = NodeState::Disabled;
                n.eval_us = 0;
                n.dirty = false;
            }
            continue;
        }

        // Gather input DataTrees from all incoming wires (supporting multi-wire stream merge!)
        let mut input_trees = Vec::with_capacity(in_specs.len());
        let mut missing_required: Option<String> = None;

        for (port_idx, spec) in in_specs.iter().enumerate() {
            let mut merged = DataTree::new();
            let mut had_wire = false;
            for w in graph.wires.values() {
                if w.to_node == id && w.to_port == port_idx {
                    had_wire = true;
                    if let Some(src_node) = graph.nodes.get(&w.from_node) {
                        if let Some(tree) = src_node.cached_outputs.get(w.from_port) {
                            merged.merge_from(tree);
                        }
                    }
                }
            }
            if !had_wire {
                if let Some(Some(override_val)) = in_defaults.get(port_idx) {
                    merged = DataTree::single(override_val.clone());
                } else if !spec.default_value.is_null() {
                    merged = DataTree::single(spec.default_value.clone());
                } else if !spec.optional {
                    missing_required = Some(format!("Input `{}` failed to collect data", spec.name));
                }
            }
            if let Some(&modifier) = in_mods.get(port_idx) {
                merged = merged.apply_modifier(modifier);
            }
            input_trees.push(merged);
        }

        let node_start = Instant::now();
        let (outputs, state) = if let Some(warn_msg) = missing_required {
            (vec![DataTree::new(); out_specs.len()], NodeState::Warning(warn_msg))
        } else {
            let mut ctx = EvalContext::new(&input_trees, out_specs.len(), &widget, matching, time_secs);
            match comp.evaluate(&mut ctx) {
                Ok(()) => {
                    let mut final_outs = ctx.outputs;
                    for (idx, out_tree) in final_outs.iter_mut().enumerate() {
                        if let Some(&m) = out_mods.get(idx) {
                            *out_tree = out_tree.apply_modifier(m);
                        }
                    }
                    let st = if let Some(first_warn) = ctx.warnings.into_iter().next() {
                        NodeState::Warning(first_warn)
                    } else {
                        NodeState::Normal
                    };
                    (final_outs, st)
                }
                Err(err) => (vec![DataTree::new(); out_specs.len()], NodeState::Error(err)),
            }
        };
        let elapsed_us = node_start.elapsed().as_micros() as u64;

        match &state {
            NodeState::Warning(_) => report.warnings += 1,
            NodeState::Error(_) => report.errors += 1,
            _ => {}
        }

        if let Some(n) = graph.nodes.get_mut(&id) {
            n.cached_outputs = outputs;
            n.state = state;
            n.eval_us = elapsed_us;
            n.dirty = false;
        }
        report.evaluated_nodes += 1;
    }

    report.total_us = start.elapsed().as_micros() as u64;
    report
}

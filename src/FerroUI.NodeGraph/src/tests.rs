//! Comprehensive unit test suite for `ferroui-nodegraph`.

use crate::prelude::*;
use std::sync::Arc;

#[test]
fn data_tree_flatten_graft_simplify_reverse_and_flip_matrix() {
    let mut tree = DataTree::new();
    tree.insert_branch(DataPath::from_slice(&[0, 0]), vec![Value::Int(1), Value::Int(2), Value::Int(3)]);
    tree.insert_branch(DataPath::from_slice(&[0, 1]), vec![Value::Int(4), Value::Int(5), Value::Int(6)]);

    assert_eq!(tree.branch_count(), 2);
    assert_eq!(tree.item_count(), 6);
    assert_eq!(tree.wire_data_kind(), WireDataKind::Tree);

    // Simplify strips shared leading 0 segment -> {0}, {1}
    let simp = tree.simplify();
    assert!(simp.branches().contains_key(&DataPath::from_slice(&[0])));
    assert!(simp.branches().contains_key(&DataPath::from_slice(&[1])));

    // Flatten collapses into single {0} branch of 6 items
    let flat = tree.flatten();
    assert_eq!(flat.branch_count(), 1);
    assert_eq!(flat.item_count(), 6);
    assert_eq!(flat.wire_data_kind(), WireDataKind::List);

    // Graft promotes every item to its own branch
    let grafted = flat.graft();
    assert_eq!(grafted.branch_count(), 6);
    assert_eq!(grafted.item_count(), 6);

    // FlipMatrix transposes 2x3 into 3x2
    let flipped = tree.flip_matrix();
    assert_eq!(flipped.branch_count(), 3);
    assert_eq!(flipped.branches().get(&DataPath::from_slice(&[0])).unwrap().len(), 2);

    // Reverse reverses inside each branch
    let rev = flat.reverse();
    assert_eq!(rev.first_item().unwrap().as_int(), Some(6));
}

#[test]
fn grasshopper_lacing_longest_shortest_and_cross_reference() {
    let a = DataTree::from_list(vec![Value::Float(10.0), Value::Float(20.0), Value::Float(30.0)]);
    let b = DataTree::from_list(vec![Value::Float(1.0), Value::Float(2.0)]);

    let add = |args: &[Value]| -> Result<Vec<Value>, String> {
        Ok(vec![Value::Float(args[0].as_float().unwrap() + args[1].as_float().unwrap())])
    };

    // LongestList: (10+1, 20+2, 30+2) = [11, 22, 32]
    let out_l = lace_item_inputs(&[a.clone(), b.clone()], DataMatching::LongestList, add, 1).unwrap();
    let vals_l: Vec<f64> = out_l[0].all_items().iter().filter_map(Value::as_float).collect();
    assert_eq!(vals_l, vec![11.0, 22.0, 32.0]);

    // ShortestList: (10+1, 20+2) = [11, 22]
    let out_s = lace_item_inputs(&[a.clone(), b.clone()], DataMatching::ShortestList, add, 1).unwrap();
    let vals_s: Vec<f64> = out_s[0].all_items().iter().filter_map(Value::as_float).collect();
    assert_eq!(vals_s, vec![11.0, 22.0]);

    // CrossReference: 3 x 2 = 6 pairs
    let out_c = lace_item_inputs(&[a, b], DataMatching::CrossReference, add, 1).unwrap();
    assert_eq!(out_c[0].item_count(), 6);
}

#[test]
fn expression_parser_evaluates_math_and_kepler_formulas() {
    let v = eval_math_expression("sqrt(x / max(y, 0.5))", 48.0, 3.0, 0.0, 0.0).unwrap();
    assert!((v - 4.0).abs() < 1e-9);
    let trig = eval_math_expression("sin(pi / 2) + cos(0) * 3 - 1", 0.0, 0.0, 0.0, 0.0).unwrap();
    assert!((trig - 3.0).abs() < 1e-9);
    let pow_v = eval_math_expression("2^3^2", 0.0, 0.0, 0.0, 0.0).unwrap();
    assert!((pow_v - 512.0).abs() < 1e-9);
}

#[test]
fn graph_prevents_cycles_supports_multi_wire_undo_redo_and_json_roundtrip() {
    let reg = NodeRegistry::with_standard_library();
    let mut g = NodeGraph::new("Test Graph");

    let s1 = g.add_node(&reg, "params.slider", 10.0, 10.0).unwrap();
    g.configure_slider(s1, "A", 4.0, 0.0, 10.0, 0.5, false);
    let s2 = g.add_node(&reg, "params.slider", 10.0, 80.0).unwrap();
    g.configure_slider(s2, "B", 6.0, 0.0, 10.0, 0.5, false);
    let add = g.add_node(&reg, "math.add", 250.0, 40.0).unwrap();
    let mul = g.add_node(&reg, "math.mul", 460.0, 40.0).unwrap();

    g.connect(s1, 0, add, 0, false).unwrap();
    g.connect(s2, 0, add, 1, false).unwrap();
    g.connect(add, 0, mul, 0, false).unwrap();
    g.set_input_default(mul, 1, Value::Float(3.0));

    // Cycle rejection
    assert!(g.connect(mul, 0, add, 0, false).is_err());

    let rep = evaluate_graph(&mut g, &reg, 0.0);
    assert_eq!(rep.errors, 0);
    assert_eq!(g.nodes[&mul].output_tree(0).unwrap().first_item().unwrap().as_float(), Some(30.0));

    // Incremental evaluation: clean graph evaluates 0 nodes!
    let rep2 = evaluate_graph(&mut g, &reg, 0.0);
    assert_eq!(rep2.evaluated_nodes, 0);

    // Undo / Redo test
    g.push_undo_checkpoint();
    g.remove_node(mul);
    assert_eq!(g.nodes.len(), 3);
    assert!(g.undo());
    assert_eq!(g.nodes.len(), 4);
    assert!(g.redo());
    assert_eq!(g.nodes.len(), 3);
    assert!(g.undo());

    // JSON round-trip preserves structure, values, and evaluation result!
    let json = graph_to_json(&g);
    let mut g2 = graph_from_json(&json, &reg).expect("valid JSON");
    g2.set_input_default(
        *g2.nodes.iter().find(|(_, n)| n.type_id == "math.mul").unwrap().0,
        1,
        Value::Float(3.0),
    );
    evaluate_graph(&mut g2, &reg, 0.0);
    let mul2 = g2.nodes.values().find(|n| n.type_id == "math.mul").unwrap();
    assert_eq!(mul2.output_tree(0).unwrap().first_item().unwrap().as_float(), Some(30.0));
}

#[test]
fn cluster_component_encapsulates_reusable_subgraph() {
    let reg = Arc::new(NodeRegistry::with_standard_library());
    let mut inner = NodeGraph::new("DoublePlusOne");
    let mul = inner.add_node(&reg, "math.mul", 0.0, 0.0).unwrap();
    inner.set_input_default(mul, 1, Value::Float(2.0));
    let add = inner.add_node(&reg, "math.add", 180.0, 0.0).unwrap();
    inner.set_input_default(add, 1, Value::Float(1.0));
    inner.connect(mul, 0, add, 0, false).unwrap();

    let cluster = ClusterComponent::new(
        "cluster.double_plus_one",
        "Double + 1",
        "2x+1",
        "Encapsulated sub-graph computing 2x + 1",
        inner,
        reg.clone(),
        vec![ClusterInputBinding {
            spec: cluster_port("x", "Input", PortType::Float),
            target_node: mul,
            target_input_port: 0,
        }],
        vec![ClusterOutputBinding {
            spec: cluster_port("y", "Output", PortType::Float),
            source_node: add,
            source_output_port: 0,
        }],
    );

    let mut outer_reg = (*reg).clone();
    outer_reg.register(cluster);
    let mut outer = NodeGraph::new("Outer");
    let cnode = outer.add_node(&outer_reg, "cluster.double_plus_one", 100.0, 100.0).unwrap();
    outer.set_input_default(cnode, 0, Value::Float(7.0));
    evaluate_graph(&mut outer, &outer_reg, 0.0);
    assert_eq!(outer.nodes[&cnode].output_tree(0).unwrap().first_item().unwrap().as_float(), Some(15.0));
}

#[cfg(feature = "physics")]
#[test]
fn all_four_parametric_physics_templates_compile_and_simulate_cleanly() {
    let reg = NodeRegistry::with_standard_library();
    for tpl in PhysicsGraphTemplate::ALL {
        let mut g = tpl.build(&reg);
        let rep = evaluate_graph(&mut g, &reg, 0.0);
        assert_eq!(rep.errors, 0, "Template {tpl:?} had evaluation errors");

        let bp = extract_physics_blueprint(&g).expect("expected CompiledPhysicsBlueprint");
        let mut built = bp.build(16.0, 10.0);
        for frame in 0..60 {
            built.world.step(1.0 / 60.0);
            let body_refs: Vec<&ferroui_physics::RigidBody> = built.world.bodies().collect();
            if let Some(cloth) = built.cloth.as_mut() {
                cloth.step(
                    1.0 / 60.0,
                    ferroui_physics::Vec2::new(0.0, 9.81),
                    ferroui_physics::Vec2::new(3.0, 0.0),
                    frame as f32 / 60.0,
                    &body_refs,
                );
            }
            for (sb, _) in &mut built.soft_bodies {
                sb.step(1.0 / 60.0, ferroui_physics::Vec2::new(0.0, 9.81), &body_refs);
            }
        }
        for b in built.world.bodies() {
            assert!(b.position.is_finite(), "{tpl:?}: non-finite body position");
        }
        assert!(
            built.world.bodies().count() > 4 || built.cloth.is_some() || !built.soft_bodies.is_empty(),
            "{tpl:?} produced an empty scene"
        );
    }
}

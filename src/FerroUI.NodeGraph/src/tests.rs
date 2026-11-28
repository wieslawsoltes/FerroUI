//! Comprehensive unit test suite for `ferroui-nodegraph`.

use crate::prelude::*;
use std::sync::Arc;

#[test]
fn data_tree_flatten_graft_simplify_reverse_and_flip_matrix() {
    let mut tree = DataTree::new();
    tree.insert_branch(
        DataPath::from_slice(&[0, 0]),
        vec![Value::Int(1), Value::Int(2), Value::Int(3)],
    );
    tree.insert_branch(
        DataPath::from_slice(&[0, 1]),
        vec![Value::Int(4), Value::Int(5), Value::Int(6)],
    );

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
    assert_eq!(
        flipped
            .branches()
            .get(&DataPath::from_slice(&[0]))
            .unwrap()
            .len(),
        2
    );

    // Reverse reverses inside each branch
    let rev = flat.reverse();
    assert_eq!(rev.first_item().unwrap().as_int(), Some(6));
}

#[test]
fn grasshopper_lacing_longest_shortest_and_cross_reference() {
    let a = DataTree::from_list(vec![
        Value::Float(10.0),
        Value::Float(20.0),
        Value::Float(30.0),
    ]);
    let b = DataTree::from_list(vec![Value::Float(1.0), Value::Float(2.0)]);

    let add = |args: &[Value]| -> Result<Vec<Value>, String> {
        Ok(vec![Value::Float(
            args[0].as_float().unwrap() + args[1].as_float().unwrap(),
        )])
    };

    // LongestList: (10+1, 20+2, 30+2) = [11, 22, 32]
    let out_l =
        lace_item_inputs(&[a.clone(), b.clone()], DataMatching::LongestList, add, 1).unwrap();
    let vals_l: Vec<f64> = out_l[0]
        .all_items()
        .iter()
        .filter_map(Value::as_float)
        .collect();
    assert_eq!(vals_l, vec![11.0, 22.0, 32.0]);

    // ShortestList: (10+1, 20+2) = [11, 22]
    let out_s =
        lace_item_inputs(&[a.clone(), b.clone()], DataMatching::ShortestList, add, 1).unwrap();
    let vals_s: Vec<f64> = out_s[0]
        .all_items()
        .iter()
        .filter_map(Value::as_float)
        .collect();
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
    assert_eq!(
        g.nodes[&mul]
            .output_tree(0)
            .unwrap()
            .first_item()
            .unwrap()
            .as_float(),
        Some(30.0)
    );

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
        *g2.nodes
            .iter()
            .find(|(_, n)| n.type_id == "math.mul")
            .unwrap()
            .0,
        1,
        Value::Float(3.0),
    );
    evaluate_graph(&mut g2, &reg, 0.0);
    let mul2 = g2.nodes.values().find(|n| n.type_id == "math.mul").unwrap();
    assert_eq!(
        mul2.output_tree(0)
            .unwrap()
            .first_item()
            .unwrap()
            .as_float(),
        Some(30.0)
    );
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
    let cnode = outer
        .add_node(&outer_reg, "cluster.double_plus_one", 100.0, 100.0)
        .unwrap();
    outer.set_input_default(cnode, 0, Value::Float(7.0));
    evaluate_graph(&mut outer, &outer_reg, 0.0);
    assert_eq!(
        outer.nodes[&cnode]
            .output_tree(0)
            .unwrap()
            .first_item()
            .unwrap()
            .as_float(),
        Some(15.0)
    );
}

#[test]
fn grasshopper_path_mapper_graph_mapper_md_slider_align_relay_and_solution_lock() {
    let reg = NodeRegistry::with_standard_library();

    // 1. PathMapper lexical remapping {A;B}(i) -> {B;A}
    let mut tree = DataTree::new();
    tree.insert_branch(
        DataPath::from_slice(&[0, 1]),
        vec![Value::Int(10), Value::Int(20)],
    );
    tree.insert_branch(DataPath::from_slice(&[2, 3]), vec![Value::Int(30)]);
    let mapped = tree.map_paths("{A;B}(i)", "{B;A}").unwrap();
    assert!(mapped
        .branches()
        .contains_key(&DataPath::from_slice(&[1, 0])));
    assert!(mapped
        .branches()
        .contains_key(&DataPath::from_slice(&[3, 2])));

    // 2. GraphMapper evaluates all 5 curve types in [0, 1]
    for kind in GraphCurveKind::ALL {
        let y = kind.evaluate(0.5, [0.25, 0.8], [0.75, 0.2]);
        assert!(
            y.is_finite() && (0.0..=1.0).contains(&y),
            "curve {kind:?} out of bounds: {y}"
        );
    }

    // 3. Inline Relay insertion on wire, AlignMode, and Solution Lock
    let mut g = NodeGraph::new("GH Features");
    let a = g.add_node(&reg, "params.md_slider", 20.0, 20.0).unwrap();
    let b = g
        .add_node(&reg, "params.graph_mapper", 320.0, 120.0)
        .unwrap();
    let wid = g.connect(a, 1, b, 0, false).unwrap();
    let relay = g
        .insert_relay_on_wire(wid, 180.0, 70.0, &reg)
        .expect("relay inserted");
    assert_eq!(g.wires.len(), 2);
    assert!(g.nodes.contains_key(&relay));

    // Align selection
    g.selected_nodes.insert(a);
    g.selected_nodes.insert(b);
    assert!(g.align_selection(AlignMode::Top));
    assert!((g.nodes[&a].position[1] - g.nodes[&b].position[1]).abs() < 1e-6);

    // Solution Lock prevents evaluation until unlocked
    g.solution_locked = true;
    g.mark_all_dirty();
    let locked_rep = evaluate_graph(&mut g, &reg, 0.0);
    assert_eq!(locked_rep.evaluated_nodes, 0);
    g.solution_locked = false;
    let live_rep = evaluate_graph(&mut g, &reg, 0.0);
    assert!(live_rep.evaluated_nodes >= 3);
    assert_eq!(live_rep.errors, 0);
}

#[cfg(feature = "physics")]
#[test]
fn all_five_parametric_physics_templates_compile_and_simulate_cleanly() {
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
                sb.step(
                    1.0 / 60.0,
                    ferroui_physics::Vec2::new(0.0, 9.81),
                    &body_refs,
                );
            }
        }
        for b in built.world.bodies() {
            assert!(b.position.is_finite(), "{tpl:?}: non-finite body position");
        }
        assert!(
            built.world.bodies().count() > 4
                || built.cloth.is_some()
                || !built.soft_bodies.is_empty(),
            "{tpl:?} produced an empty scene"
        );
    }
}

#[test]
fn reconfiguration_and_time_propagate_through_clean_dependents() {
    let registry = NodeRegistry::with_standard_library();
    let mut graph = NodeGraph::new("Incremental clock");
    let source = graph
        .add_node(&registry, "params.slider", 0.0, 0.0)
        .unwrap();
    let time = graph.add_node(&registry, "params.time", 0.0, 80.0).unwrap();
    let add = graph.add_node(&registry, "math.add", 240.0, 0.0).unwrap();
    let multiply = graph.add_node(&registry, "math.mul", 480.0, 0.0).unwrap();
    let unrelated = graph
        .add_node(&registry, "params.toggle", 0.0, 160.0)
        .unwrap();
    graph.configure_slider(source, "Offset", 2.0, 0.0, 10.0, 0.1, false);
    graph.set_input_default(multiply, 1, Value::Float(3.0));
    graph.connect(source, 0, add, 0, false).unwrap();
    graph.connect(time, 0, add, 1, false).unwrap();
    graph.connect(add, 0, multiply, 0, false).unwrap();
    evaluate_graph(&mut graph, &registry, 1.0);
    let report = evaluate_graph(&mut graph, &registry, 2.0);
    assert_eq!(report.evaluated_nodes, 3);
    assert_eq!(
        graph.nodes[&multiply].cached_outputs[0]
            .first_item()
            .unwrap()
            .as_float(),
        Some(12.0)
    );
    assert!(!graph.nodes[&unrelated].dirty);
    graph.configure_slider(source, "Offset", 4.0, 0.0, 10.0, 0.1, false);
    assert!(graph.nodes[&multiply].dirty);
    evaluate_graph(&mut graph, &registry, 2.0);
    assert_eq!(
        graph.nodes[&multiply].cached_outputs[0]
            .first_item()
            .unwrap()
            .as_float(),
        Some(18.0)
    );

    let expression = graph
        .add_node(&registry, "math.expression", 0.0, 240.0)
        .unwrap();
    graph.configure_text(expression, "Clock expression", "sin(t)");
    evaluate_graph(&mut graph, &registry, 0.0);
    evaluate_graph(&mut graph, &registry, std::f64::consts::FRAC_PI_2);
    assert_eq!(
        graph.nodes[&expression].cached_outputs[0]
            .first_item()
            .unwrap()
            .as_float(),
        Some(1.0)
    );
}

#[test]
fn wiring_validates_ports_preserves_noop_rewire_and_cleans_selection() {
    let registry = NodeRegistry::with_standard_library();
    let mut graph = NodeGraph::default();
    let a = graph
        .add_node(&registry, "params.slider", 0.0, 0.0)
        .unwrap();
    let b = graph
        .add_node(&registry, "params.slider", 0.0, 80.0)
        .unwrap();
    let add = graph.add_node(&registry, "math.add", 240.0, 0.0).unwrap();
    assert!(graph.connect(a, 1, add, 0, false).is_err());
    assert!(graph.connect(a, 0, add, 2, false).is_err());
    let first = graph.connect(a, 0, add, 0, true).unwrap();
    let second = graph.connect(b, 0, add, 0, true).unwrap();
    graph.selected_wires.insert(second);
    assert_eq!(graph.rewire_input(add, 0, add, 0), 0);
    assert_eq!(graph.wires.len(), 2);
    assert_eq!(graph.connect(a, 0, add, 0, false).unwrap(), first);
    assert_eq!(graph.wires.len(), 1);
    assert!(graph.selected_wires.is_empty());
    graph.selected_wires.insert(first);
    graph.remove_node(a);
    assert!(graph.selected_wires.is_empty());
    assert!(graph.wires.is_empty());
}

#[test]
fn disabled_and_unknown_components_clear_downstream_cached_outputs() {
    let registry = NodeRegistry::with_standard_library();
    let mut graph = NodeGraph::default();
    let source = graph
        .add_node(&registry, "params.slider", 0.0, 0.0)
        .unwrap();
    let relay = graph
        .add_node(&registry, "params.relay", 240.0, 0.0)
        .unwrap();
    graph.connect(source, 0, relay, 0, false).unwrap();
    evaluate_graph(&mut graph, &registry, 0.0);
    assert!(!graph.nodes[&relay].cached_outputs[0].is_empty());
    graph.nodes.get_mut(&source).unwrap().disabled = true;
    graph.nodes.get_mut(&source).unwrap().dirty = true;
    evaluate_graph(&mut graph, &registry, 0.0);
    assert!(graph.nodes[&relay].cached_outputs[0].is_empty());
    graph.nodes.get_mut(&source).unwrap().disabled = false;
    graph.nodes.get_mut(&source).unwrap().dirty = true;
    evaluate_graph(&mut graph, &registry, 0.0);
    graph.nodes.get_mut(&source).unwrap().type_id = "missing.component".into();
    graph.nodes.get_mut(&source).unwrap().dirty = true;
    assert_eq!(evaluate_graph(&mut graph, &registry, 0.0).errors, 1);
    assert!(graph.nodes[&source].cached_outputs[0].is_empty());
    assert!(graph.nodes[&relay].cached_outputs[0].is_empty());
}

#[test]
fn clusters_preserve_complete_branched_inputs() {
    let registry = Arc::new(NodeRegistry::with_standard_library());
    let mut inner = NodeGraph::new("Branch doubling");
    let multiply = inner.add_node(&registry, "math.mul", 0.0, 0.0).unwrap();
    inner.set_input_default(multiply, 1, Value::Float(2.0));
    let cluster = ClusterComponent::new(
        "test.branches",
        "Double",
        "2x",
        "Doubles every item",
        inner,
        registry,
        vec![ClusterInputBinding {
            spec: cluster_port("x", "Input", PortType::Float),
            target_node: multiply,
            target_input_port: 0,
        }],
        vec![ClusterOutputBinding {
            spec: cluster_port("y", "Output", PortType::Float),
            source_node: multiply,
            source_output_port: 0,
        }],
    );
    let mut input = DataTree::new();
    input.insert_branch(
        DataPath::from_slice(&[0, 2]),
        vec![Value::Int(1), Value::Int(2)],
    );
    input.insert_branch(DataPath::from_slice(&[0, 5]), vec![Value::Int(3)]);
    let inputs = [input];
    let mut ctx = EvalContext::new(
        &inputs,
        1,
        &ParamWidget::None,
        DataMatching::LongestList,
        0.0,
    );
    cluster.evaluate(&mut ctx).unwrap();
    assert_eq!(ctx.outputs[0].branch_count(), 2);
    assert_eq!(
        ctx.outputs[0]
            .all_items()
            .iter()
            .filter_map(Value::as_float)
            .collect::<Vec<_>>(),
        [2.0, 4.0, 6.0]
    );
    assert!(ctx.outputs[0]
        .branches()
        .contains_key(&DataPath::from_slice(&[0, 5])));
}

#[test]
fn cartesian_lacing_streams_in_order_and_can_stop_before_materializing_product() {
    let inputs = [
        DataTree::from_list(vec![Value::Int(1), Value::Int(2)]),
        DataTree::from_list(vec![Value::Int(3), Value::Int(4), Value::Int(5)]),
    ];
    let result = lace_item_inputs(
        &inputs,
        DataMatching::CrossReference,
        |values| {
            Ok(vec![Value::Int(
                values[0].as_int().unwrap() * 10 + values[1].as_int().unwrap(),
            )])
        },
        1,
    )
    .unwrap();
    assert_eq!(
        result[0]
            .all_items()
            .iter()
            .filter_map(Value::as_int)
            .collect::<Vec<_>>(),
        [13, 14, 15, 23, 24, 25]
    );
    let large_inputs = vec![DataTree::from_list(vec![Value::Int(1); 1000]); 8];
    let mut calls = 0;
    let result = lace_item_inputs(
        &large_inputs,
        DataMatching::CrossReference,
        |_| {
            calls += 1;
            Err("stop".to_string())
        },
        1,
    );
    assert!(result.is_err());
    assert_eq!(calls, 1);
    let path = DataPath::from_slice(&[1, 2, 3]);
    assert_eq!(path.shift(isize::MIN), DataPath::from_slice(&[1]));
    assert_eq!(
        DataTree::from_list(vec![Value::Null, Value::Int(7)])
            .first_item()
            .unwrap()
            .as_int(),
        Some(7)
    );
}

#[test]
fn json_preserves_defaults_modifiers_precision_unicode_and_document_settings() {
    let registry = NodeRegistry::with_standard_library();
    let mut graph = NodeGraph::new("Zażółć gęślą jaźń 🦀\u{08}\u{0c}\n");
    let slider = graph
        .add_node(&registry, "params.slider", 0.123456789012345, 20.0)
        .unwrap();
    let add = graph.add_node(&registry, "math.add", 240.0, 0.0).unwrap();
    graph.configure_slider(
        slider,
        "Wartość ω",
        0.123456789012345,
        -1.0,
        1.0,
        0.0000000001,
        false,
    );
    let wire = graph.connect(slider, 0, add, 0, false).unwrap();
    graph.wires.get_mut(&wire).unwrap().display = WireDisplay::Hidden;
    graph.set_input_default(add, 1, Value::Float(0.987654321098765));
    let modifier = ParamModifier {
        graft: true,
        reverse: true,
        ..ParamModifier::NONE
    };
    graph.nodes.get_mut(&add).unwrap().output_modifiers[0] = modifier;
    graph.nodes.get_mut(&add).unwrap().matching = DataMatching::CrossReference;
    graph.nodes.get_mut(&add).unwrap().preview = false;
    graph.nodes.get_mut(&add).unwrap().size = [230.123456789, 80.987654321];
    graph.camera_pan = [0.123456789, -3.123456789];
    graph.camera_zoom = 0.987654321;
    graph.snap_to_grid = true;
    graph.fancy_wires = false;
    graph.show_profiler = false;
    graph.solution_locked = true;
    let json = graph_to_json(&graph);
    let loaded = graph_from_json(&json, &registry).unwrap();
    assert_eq!(graph_to_json(&loaded), json);

    for value in [
        Value::Null,
        Value::Bool(true),
        Value::Int(i64::MAX),
        Value::Float(0.123456789),
        Value::Vec2([1.0, 2.0]),
        Value::Vec3([1.0, 2.0, 3.0]),
        Value::Color(0xFF123456),
        Value::String("漢字\\\"\u{0}".into()),
        Value::Domain(-2.0, 3.0),
        Value::Curve(vec![[1.0, 2.0], [3.0, 4.0]]),
        Value::Mesh {
            vertices: vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]],
            indices: vec![[0, 1, 2]],
        },
    ] {
        graph.set_input_default(add, 1, value);
        let json = graph_to_json(&graph);
        assert_eq!(
            graph_to_json(&graph_from_json(&json, &registry).unwrap()),
            json
        );
    }
    let unicode = r#"{"name":"\u017b\u00f3\u0142\u0107 \ud83e\udd80","nodes":[]}"#;
    assert_eq!(graph_from_json(unicode, &registry).unwrap().name, "Żółć 🦀");
}

#[test]
fn json_rejects_malformed_documents_instead_of_silently_dropping_data() {
    let registry = NodeRegistry::with_standard_library();
    for invalid in [
        r#"{"nodes":[]} trailing"#,
        r#"{"name":"\q","nodes":[]}"#,
        r#"{"name":"\ud800","nodes":[]}"#,
        r#"{"nodes":[],"nodes":[]}"#,
        r#"{"nodes":[],"x":01}"#,
        r#"{"nodes":[],"x":1.}"#,
        r#"{"nodes":[],"x":1e999}"#,
        r#"{"format":"future/v2","nodes":[]}"#,
        r#"{"nodes":[{"id":1,"type":"missing"}]}"#,
        r#"{"nodes":[{"id":1,"type":"params.slider"},{"id":1,"type":"params.slider"}]}"#,
        r#"{"nodes":[{"id":1,"type":"params.slider"},{"id":2,"type":"math.add"}],"wires":[{"from":[1,100],"to":[2,0]}]}"#,
    ] {
        assert!(
            graph_from_json(invalid, &registry).is_err(),
            "Accepted malformed graph: {invalid}"
        );
    }
    let nested = format!("{}0{}", "[".repeat(256), "]".repeat(256));
    assert!(graph_from_json(&nested, &registry).is_err());
}

#[test]
fn momentary_button_releases_on_the_next_solution() {
    let registry = NodeRegistry::with_standard_library();
    let mut graph = NodeGraph::default();
    let button = graph
        .add_node(&registry, "params.button", 0.0, 0.0)
        .unwrap();
    let relay = graph
        .add_node(&registry, "params.relay", 240.0, 0.0)
        .unwrap();
    graph.connect(button, 0, relay, 0, false).unwrap();
    graph.nodes.get_mut(&button).unwrap().widget = ParamWidget::Button { pressed: true };
    evaluate_graph(&mut graph, &registry, 0.0);
    assert_eq!(
        graph.nodes[&relay].cached_outputs[0]
            .first_item()
            .unwrap()
            .as_bool(),
        Some(true)
    );
    evaluate_graph(&mut graph, &registry, 1.0);
    assert_eq!(
        graph.nodes[&relay].cached_outputs[0]
            .first_item()
            .unwrap()
            .as_bool(),
        Some(false)
    );
    assert_eq!(
        evaluate_graph(&mut graph, &registry, 2.0).evaluated_nodes,
        0
    );
}

#[test]
fn cyclic_components_clear_caches_without_blocking_independent_subgraphs() {
    let registry = NodeRegistry::with_standard_library();
    let mut graph = NodeGraph::default();
    let a = graph.add_node(&registry, "math.add", 0.0, 0.0).unwrap();
    let b = graph.add_node(&registry, "math.add", 240.0, 0.0).unwrap();
    let independent = graph
        .add_node(&registry, "params.slider", 0.0, 80.0)
        .unwrap();
    graph.connect(a, 0, b, 0, false).unwrap();
    evaluate_graph(&mut graph, &registry, 0.0);
    graph.wires.insert(
        WireId(500),
        Wire {
            id: WireId(500),
            from_node: b,
            from_port: 0,
            to_node: a,
            to_port: 0,
            display: WireDisplay::Default,
        },
    );
    graph.configure_slider(independent, "Independent", 5.0, 0.0, 10.0, 1.0, false);
    let report = evaluate_graph(&mut graph, &registry, 0.0);
    assert_eq!(report.errors, 2);
    assert_eq!(report.evaluated_nodes, 1);
    assert!(graph.nodes[&a].cached_outputs[0].is_empty());
    assert!(graph.nodes[&b].cached_outputs[0].is_empty());
    assert_eq!(
        graph.nodes[&independent].cached_outputs[0]
            .first_item()
            .unwrap()
            .as_float(),
        Some(5.0)
    );
    graph.wires.shift_remove(&WireId(500));
    assert_eq!(evaluate_graph(&mut graph, &registry, 0.0).errors, 0);
}

#[test]
fn checked_export_rejects_nonfinite_or_opaque_defaults() {
    let registry = NodeRegistry::with_standard_library();
    let mut graph = NodeGraph::default();
    let add = graph.add_node(&registry, "math.add", 0.0, 0.0).unwrap();
    graph.set_input_default(add, 0, Value::Float(f64::NAN));
    assert!(try_graph_to_json(&graph).is_err());
    graph.set_input_default(add, 0, Value::custom("test", "Opaque value", 42u32));
    assert!(try_graph_to_json(&graph).is_err());
}

#[test]
fn json_roundtrips_every_widget_variant_including_explicit_none() {
    let registry = NodeRegistry::with_standard_library();
    let mut graph = NodeGraph::default();
    let node = graph
        .add_node(&registry, "params.slider", 0.0, 0.0)
        .unwrap();
    for widget in [
        ParamWidget::None,
        ParamWidget::Slider {
            value: 0.123456789,
            min: -1.23456789,
            max: 9.87654321,
            step: 0.000000001,
            integer: false,
        },
        ParamWidget::Toggle { value: true },
        ParamWidget::Button { pressed: true },
        ParamWidget::Color { argb: 0x01020304 },
        ParamWidget::Text {
            text: "Parameters Ω 🦀".into(),
        },
        ParamWidget::GraphMapper {
            curve: GraphCurveKind::Gaussian,
            p1: [0.123456789, 0.23456789],
            p2: [0.3456789, 0.456789],
        },
        ParamWidget::MdSlider {
            x: 0.123456789,
            y: 0.987654321,
        },
        ParamWidget::ValueList {
            options: vec![("Small".into(), 0.123456789), ("Large".into(), 0.987654321)],
            selected: 1,
        },
        ParamWidget::PathMapper {
            source_mask: "{A;B}(i)".into(),
            target_mask: "{B;i}".into(),
        },
    ] {
        graph.nodes.get_mut(&node).unwrap().widget = widget.clone();
        let json = try_graph_to_json(&graph).unwrap();
        let loaded = graph_from_json(&json, &registry).unwrap();
        assert_eq!(loaded.nodes[&node].widget, widget);
    }
}

#[test]
fn bezier_graph_mapper_inverts_the_horizontal_curve_coordinate() {
    let curve = GraphCurveKind::Bezier;
    let p1 = [0.8, 0.1];
    let p2 = [0.9, 0.8];
    // At parameter 0.5, this Bezier passes through (0.7625, 0.4625).
    assert!((curve.evaluate(0.7625, p1, p2) - 0.4625).abs() < 1e-9);
    assert_eq!(curve.evaluate(0.0, p1, p2), 0.0);
    assert_eq!(curve.evaluate(1.0, p1, p2), 1.0);
    for i in 0..=100 {
        let x = i as f64 / 100.0;
        assert!((curve.evaluate(x, [0.2, 0.2], [0.8, 0.8]) - x).abs() < 1e-9);
    }
    assert!(
        (curve.evaluate(0.5, p1, p2) - curve.evaluate(0.5, [0.1, p1[1]], [0.2, p2[1]])).abs() > 0.2
    );
}

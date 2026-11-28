//! Reproducible graph invalidation/evaluation microbenchmark.
//! Run with `cargo bench -p ferroui-nodegraph --no-default-features --bench evaluation`.
//! Construction and initial evaluation are outside the timed regions. Every edit checks
//! the full expected output before contributing to the checksum.

use ferroui_nodegraph::prelude::*;
use std::time::Instant;

fn run(fanout: bool, iterations: usize) -> (f64, f64, f64) {
    let registry = NodeRegistry::with_standard_library();
    let mut graph = NodeGraph::new("Evaluation benchmark");
    let source = graph
        .add_node(&registry, "params.slider", 0.0, 0.0)
        .unwrap();
    let mut results = Vec::new();
    if fanout {
        for _ in 0..1000 {
            let add = graph.add_node(&registry, "math.add", 0.0, 0.0).unwrap();
            graph.set_input_default(add, 1, Value::Float(1.0));
            graph.connect(source, 0, add, 0, false).unwrap();
            let multiply = graph.add_node(&registry, "math.mul", 0.0, 0.0).unwrap();
            graph.set_input_default(multiply, 1, Value::Float(2.0));
            graph.connect(add, 0, multiply, 0, false).unwrap();
            results.push(multiply);
        }
    } else {
        let mut previous = source;
        for _ in 0..2000 {
            let add = graph.add_node(&registry, "math.add", 0.0, 0.0).unwrap();
            graph.set_input_default(add, 1, Value::Float(1.0));
            graph.connect(previous, 0, add, 0, false).unwrap();
            previous = add;
        }
        results.push(previous);
    }
    evaluate_graph(&mut graph, &registry, 0.0);
    let (mut dirty_time, mut evaluation_time, mut checksum) = (0.0, 0.0, 0.0);
    for iteration in 0..iterations {
        let ParamWidget::Slider { value, .. } = &mut graph.nodes.get_mut(&source).unwrap().widget
        else {
            unreachable!()
        };
        *value = iteration as f64 + 1.0;
        let start = Instant::now();
        graph.mark_dirty(source);
        dirty_time += start.elapsed().as_secs_f64();
        let start = Instant::now();
        let report = evaluate_graph(&mut graph, &registry, 0.0);
        evaluation_time += start.elapsed().as_secs_f64();
        assert_eq!(report.errors, 0);
        assert_eq!(report.evaluated_nodes, 2001);
        let sum = results
            .iter()
            .map(|id| {
                graph.nodes[id].cached_outputs[0]
                    .first_item()
                    .unwrap()
                    .as_float()
                    .unwrap()
            })
            .sum::<f64>();
        let expected = if fanout {
            (iteration as f64 + 2.0) * 2.0 * 1000.0
        } else {
            iteration as f64 + 2001.0
        };
        assert_eq!(sum, expected);
        checksum += std::hint::black_box(sum);
    }
    (dirty_time * 1000.0, evaluation_time * 1000.0, checksum)
}

fn main() {
    println!("workload,run,iterations,dirty_ms,evaluate_ms,checksum");
    for fanout in [false, true] {
        let case = if fanout {
            "fanout-1000x2"
        } else {
            "chain-2001"
        };
        for repetition in 1..=5 {
            let (dirty, evaluate, checksum) = run(fanout, 32);
            println!("{case},{repetition},32,{dirty:.4},{evaluate:.4},{checksum:.0}");
        }
    }
}

//! Dependency-free release microbenchmarks for replica indexing and local editing.
//! Run with `cargo bench -p ferroui-designer-collaboration --bench replication`.
use ferroui_designer_collaboration::{
    DocumentReplica, Operation, OperationId, SharedEdit, SharedNodeId,
};
use std::hint::black_box;
use std::time::{Duration, Instant};

fn operations(count: u64, distinct_actors: bool) -> Vec<Operation> {
    (1..=count)
        .map(|sequence| Operation {
            id: OperationId {
                actor: if distinct_actors {
                    format!("writer-{sequence:05}")
                } else {
                    "writer".into()
                },
                lamport: if distinct_actors { 1 } else { sequence },
                sequence: if distinct_actors { 1 } else { sequence },
            },
            document_id: "benchmark".into(),
            label: "Retained dependency".into(),
            // Missing dependencies keep the document small so this measures indexing/replay
            // overhead without conflating the result with growing XAML payloads.
            edits: vec![SharedEdit::RemoveNode {
                node: SharedNodeId::Baseline(999),
            }],
        })
        .collect()
}

fn report(case: &str, iterations: usize, elapsed: Duration) {
    println!(
        "{case},{iterations},{:.3},{:.1}",
        elapsed.as_secs_f64() * 1_000.0,
        elapsed.as_secs_f64() * 1_000_000_000.0 / iterations as f64,
    );
}

fn main() {
    println!("case,iterations,total_ms,ns_per_iteration");

    let mut replica = DocumentReplica::new("local", "benchmark", "<Panel/>").unwrap();
    let message = replica.message(operations(4_000, true));
    let started = Instant::now();
    replica.merge(message).unwrap();
    report("merge_distinct_writers", 4_000, started.elapsed());
    assert_eq!(replica.operations().count(), 4_000);

    let mut replica = DocumentReplica::new("local", "benchmark", "<Panel/>").unwrap();
    replica
        .merge(replica.message(operations(4_000, false)))
        .unwrap();
    let started = Instant::now();
    for _ in 0..2_000 {
        black_box(replica.version());
    }
    report("read_contiguous_vector", 2_000, started.elapsed());

    let source = format!("<Panel>{}</Panel>", "<Button Width='10'/>".repeat(100));
    let mut replica = DocumentReplica::new("local", "benchmark", source).unwrap();
    let started = Instant::now();
    for width in 0..500 {
        replica
            .set_attribute(replica.root(), "Width", width.to_string())
            .unwrap();
    }
    report("local_property_edits_100_controls", 500, started.elapsed());
    assert_eq!(replica.version()["local"], 500);
}

//! Reproducible, dependency-free document-engine microbenchmarks.
//!
//! `cargo bench -p ferroui-designer-core --bench document_engine`
//! `FERROUI_BENCH_NODES` (default20,000) and `FERROUI_BENCH_SAMPLES` (default5)
//! can increase the workload. The public API used here also exists before the
//! infrastructure review so the same harness can measure the baseline checkout.

use ferroui_designer_core::{Document, Edit, MarkupValue};
use std::hint::black_box;
use std::time::Instant;

fn main() {
    let nodes = setting("FERROUI_BENCH_NODES", 20_000).max(15_000);
    let samples = setting("FERROUI_BENCH_SAMPLES", 5).max(5);
    let mut source =
        String::from("<Canvas xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>\n");
    let mut offsets = Vec::with_capacity(nodes);
    for index in 0..nodes {
        offsets.push(source.len() + 2);
        source.push_str(&format!(
            "  <Button x:Name='item_{index}' Width='120' Content='Item {index}'/>\n"
        ));
    }
    source.push_str("</Canvas>");
    println!(
        "document_engine nodes={nodes} bytes={} samples={samples} profile=bench",
        source.len()
    );
    println!("Each case reports the median of {samples} samples; setup is outside timing, one warm-up operation per case.");
    println!("case,operations_per_sample,median_ns_per_operation,sample_ns_per_operation");

    measure("parse_and_drop", 3, samples, || {
        black_box(Document::new(black_box(source.clone())).unwrap());
    });

    let mut document = Document::new(source.clone()).unwrap();
    document.set_history_limit(0);
    let target = document.ast().element_children(document.ast().root())[nodes / 2];
    let mut step = 0;
    measure("attribute_edit", 5, samples, || {
        step += 1;
        black_box(
            document
                .apply(
                    "Resize",
                    vec![Edit::SetAttribute {
                        node: target,
                        name: "Width".into(),
                        value: if step % 2 == 0 { "400" } else { "401" }.into(),
                    }],
                )
                .unwrap(),
        );
    });

    let mut document = Document::new(source.clone()).unwrap();
    document.set_history_limit(0);
    measure("unchanged_source", 10, samples, || {
        black_box(document.update_source("Unchanged editor buffer", black_box(source.clone())));
    });

    let document = Document::new(source.clone()).unwrap();
    let mut query = 0;
    measure("source_hit_test", 4_096, samples, || {
        let offset = offsets[(query * 7_919) % offsets.len()];
        query += 1;
        black_box(document.ast().node_at(black_box(offset)));
    });

    let mut markup = String::from("{Extension ");
    for index in 0..2_000 {
        if index > 0 {
            markup.push(',');
        }
        markup.push_str(&format!("Property{index}=Value{index}"));
    }
    markup.push('}');
    measure("markup_2000_arguments", 3, samples, || {
        black_box(MarkupValue::parse(black_box(&markup)).unwrap());
    });
}

fn setting(name: &str, default: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

fn measure(name: &str, operations: usize, samples: usize, mut operation: impl FnMut()) {
    // Feed results through black_box in each operation. Setup allocations and
    // compilation are excluded; source clones and returned-value drops are part
    // of the operation because real callers also own these values.
    operation();
    let mut elapsed = Vec::with_capacity(samples);
    for _ in 0..samples {
        let started = Instant::now();
        for _ in 0..operations {
            operation();
        }
        elapsed.push(started.elapsed().as_nanos() / operations as u128);
    }
    let mut sorted = elapsed.clone();
    sorted.sort_unstable();
    let median = sorted[sorted.len() / 2];
    println!("{name},{operations},{median},{elapsed:?}");
}

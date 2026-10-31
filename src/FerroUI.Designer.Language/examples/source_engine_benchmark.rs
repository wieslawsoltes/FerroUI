//! Reproducible source-coordinate/refactoring workloads, without a UI backend.
//! Run with `cargo run --release -p ferroui-designer-language --example source_engine_benchmark`.
use ferroui_designer_language::{apply_text_edits, LineIndex, Position, Range, TextEdit};
use std::hint::black_box;
use std::time::Instant;

fn main() {
    println!("scalars,index_us,coordinate_queries_us,batch_edits_us");
    for count in [1_000_u32, 4_000, 16_000] {
        let source = "aα🦀".repeat(count as usize);
        let edits: Vec<_> = (0..count)
            .map(|i| TextEdit {
                range: Range {
                    start: Position {
                        line: 0,
                        character: i * 4 + 1,
                    },
                    end: Position {
                        line: 0,
                        character: i * 4 + 2,
                    },
                },
                new_text: "界".into(),
            })
            .collect();
        let start = Instant::now();
        let index = LineIndex::new(black_box(&source));
        let indexed = start.elapsed();
        let start = Instant::now();
        for i in 0..count {
            black_box(index.position(black_box(i as usize * 7)).unwrap());
            black_box(
                index
                    .offset(black_box(Position {
                        line: 0,
                        character: i * 4,
                    }))
                    .unwrap(),
            );
        }
        let queried = start.elapsed();
        let start = Instant::now();
        let output = apply_text_edits(black_box(&source), black_box(&edits)).unwrap();
        let edited = start.elapsed();
        assert_eq!(output, "a界🦀".repeat(count as usize));
        println!(
            "{},{},{},{}",
            count * 3,
            indexed.as_micros(),
            queried.as_micros(),
            edited.as_micros()
        );
    }
}

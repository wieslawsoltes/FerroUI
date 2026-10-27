//! `cargo bench -p ferroui-designer-tools --bench snapping`
//! Compare one-off queries with preparing immutable peer geometry once per drag.
use ferroui_designer_tools::{Rect, SnapSettings};
use std::hint::black_box;
use std::time::Instant;

fn main() {
    let peers: Vec<_> = (0..10_000)
        .map(|i| Rect::new((i % 100) as f64 * 40.0, (i / 100) as f64 * 30.0, 32.0, 24.0))
        .collect();
    let queries: Vec<_> = (0..2_000)
        .map(|i| Rect::new(i as f64 * 0.73, i as f64 * 0.41, 120.0, 60.0))
        .collect();
    let settings = SnapSettings::default();
    let start = Instant::now();
    let prepared = settings.prepare(black_box(&peers)).unwrap();
    let prepare = start.elapsed();
    for rect in &queries {
        assert_eq!(prepared.snap(*rect).unwrap(), settings.snap(*rect, &peers).unwrap());
    }
    println!("case,peers,queries,microseconds");
    println!("prepare,{},1,{}", peers.len(), prepare.as_micros());
    for _ in 0..5 {
        let start = Instant::now();
        for rect in &queries {
            black_box(settings.snap(black_box(*rect), black_box(&peers)).unwrap());
        }
        println!("one_off,{},{},{}", peers.len(), queries.len(), start.elapsed().as_micros());
        let start = Instant::now();
        for rect in &queries {
            black_box(prepared.snap(black_box(*rect)).unwrap());
        }
        println!("prepared,{},{},{}", peers.len(), queries.len(), start.elapsed().as_micros());
    }
}

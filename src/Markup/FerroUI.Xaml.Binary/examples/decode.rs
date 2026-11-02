//! Measure validation and expansion of a set of compiled FXB leaves already in memory.
//! Pass only the blocks referenced by the generated module being measured.
use ferroui_xaml_binary::Batch;
use std::{env, error::Error, fs, hint::black_box, time::Instant};

fn main() -> Result<(), Box<dyn Error>> {
    let files: Vec<Vec<u8>> = env::args_os()
        .skip(1)
        .map(fs::read)
        .collect::<Result<_, _>>()?;
    if files.is_empty() {
        return Err("pass one or more compiled .fxb files".into());
    }
    let bytes: usize = files.iter().map(Vec::len).sum();
    let mut records = 0;
    for data in &files {
        records += Batch::decode(data)?.records().len();
    }

    let mut samples = Vec::with_capacity(101);
    for _ in 0..101 {
        let start = Instant::now();
        for data in &files {
            black_box(Batch::decode(black_box(data))?);
        }
        samples.push(start.elapsed());
    }
    samples.sort_unstable();
    println!(
        "blocks={} records={records} bytes={bytes} p50_us={} p95_us={} wire_MiB_s={:.1}",
        files.len(),
        samples[50].as_micros(),
        samples[95].as_micros(),
        bytes as f64 / (1024. * 1024.) / samples[50].as_secs_f64()
    );
    Ok(())
}

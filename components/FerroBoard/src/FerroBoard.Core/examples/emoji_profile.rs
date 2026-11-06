//! Opt-in local emoji-search diagnostic. No UI, network or personal data.
#[path = "../tests/support/rust_allocation_counter.rs"]
mod allocations;

use allocations::counted;
use ferroboard_core::{emoji::EmojiCatalog, text::Utf16Text};
use serde_json::{Value, json};
use std::{
    error::Error,
    fs::OpenOptions,
    hint::black_box,
    io::{self, Write},
    path::PathBuf,
    time::Instant,
};
fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args_os().skip(1);
    let output = args.next().map(PathBuf::from).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "Provide one new output JSON path",
        )
    })?;
    if args.next().is_some() || output.exists() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Expected one new output JSON file",
        )
        .into());
    }
    let mut workloads = Vec::new();
    for (language, face) in [
        ("en", "face"),
        ("de", "gesicht"),
        ("es", "cara"),
        ("fr", "visage"),
        ("it", "viso"),
        ("pt", "rosto"),
    ] {
        let catalog = EmojiCatalog::load(&language.into())?;
        assert_eq!(catalog.locale(), Utf16Text::from(language));
        let entry_count = catalog.all().count();
        assert!(entry_count > 1800);
        let queries = ["pizza", face, "red heart", "zzzzzzzz", "👋"]
            .map(Utf16Text::from)
            .into_iter()
            .chain([Utf16Text::from_units(vec![0xd800])]);
        for query in queries {
            for _ in 0..3 {
                black_box(catalog.search(&query, 8));
            }
            let (result, allocation) = counted(|| catalog.search(&query, 8));
            let mut times = Vec::with_capacity(32);
            for _ in 0..32 {
                let start = Instant::now();
                let next = black_box(catalog.search(&query, 8));
                let duration = u64::try_from(start.elapsed().as_nanos())?;
                assert!(
                    next.iter()
                        .map(|item| &item.value)
                        .eq(result.iter().map(|item| &item.value))
                );
                times.push(duration);
            }
            let raw_times = times.clone();
            times.sort_unstable();
            workloads.push(json!({
                "language":language,"catalog_entries":entry_count,"query_units":query.units(),
                "max_results":8,"result_units":result.iter().map(|item|item.value.units()).collect::<Vec<_>>(),
                "rust_allocation_calls":allocation.calls,"rust_requested_bytes":allocation.requested_bytes,
                "samples_ns":raw_times,"p50_ns":times[15],"p95_ns":times[30]
            }));
        }
    }
    let report: Value = json!({"format":1,"profile":"emoji_search","os":std::env::consts::OS,"arch":std::env::consts::ARCH,
        "allocation_scope":"Current-thread Rust alloc/alloc_zeroed/realloc requests during warmed search, including returned results, excluding setup/assertions/result drops. Bytes sum requested sizes, not live/peak heap; native allocation calls are not counted.",
        "timing_scope":"32 warmed individual searches, excluding setup/assertions/result drops. Nearest-rank quantiles; shared-host scheduling noise; no latency budget or device/UI claim.","workloads":workloads});
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)?;
    serde_json::to_writer_pretty(&mut file, &report)?;
    file.write_all(b"\n")?;
    println!(
        "Recorded 36 workloads across all six bundled emoji locales; local Core diagnostic only."
    );
    Ok(())
}

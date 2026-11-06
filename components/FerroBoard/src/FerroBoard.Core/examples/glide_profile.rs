//! Opt-in full-dictionary glide ranking diagnostic against frozen source traces.
#[path = "../tests/support/rust_allocation_counter.rs"]
mod allocations;

use allocations::counted;
use ferroboard_core::{
    glide::{GesturePoint, GlideTypingClassifier},
    layouts::KeyRect,
    text::{Utf16Text, WordDictionary},
};
use serde_json::{Value, json};
use std::{
    error::Error,
    fs::OpenOptions,
    hint::black_box,
    io::{self, Write},
    path::PathBuf,
    sync::Arc,
    time::Instant,
};

fn number(value: &Value) -> f64 {
    f64::from_bits(u64::from_str_radix(value.as_str().unwrap(), 16).unwrap())
}
fn verify(result: &[Utf16Text], case: &Value, classifier: &GlideTypingClassifier) {
    assert_eq!(
        json!(result.iter().map(|word| word.units()).collect::<Vec<_>>()),
        case["ranked"]
    );
    assert_eq!(
        classifier.point_count(),
        case["pointCount"].as_u64().unwrap() as usize
    );
}
fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args_os().skip(1);
    let output = args.next().map(PathBuf::from).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "Provide one new JSON output path",
        )
    })?;
    if args.next().is_some() || output.exists() {
        return Err(
            io::Error::new(io::ErrorKind::InvalidInput, "Expected a new output file").into(),
        );
    }
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/native-android-glide.json"
    ))?;
    let start = Instant::now();
    let dictionary = WordDictionary::load_bundled_english()?;
    let dictionary_load_ns = start.elapsed().as_nanos();
    assert_eq!(
        dictionary.count(),
        fixture["dictionaryWords"].as_u64().unwrap() as usize
    );
    let classifier = GlideTypingClassifier::default();
    classifier.set_layout(fixture["layout"].as_array().unwrap().iter().map(|key| {
        let r = &key["r"];
        (
            key["c"].as_u64().unwrap() as u16,
            KeyRect::new(number(&r[0]), number(&r[1]), number(&r[2]), number(&r[3])),
        )
    }))?;
    let start = Instant::now();
    let (loaded, loading_allocations) = counted(|| classifier.set_dictionary(&dictionary));
    let dictionary_install_ns = start.elapsed().as_nanos();
    loaded?;
    let cases = fixture["cases"].as_array().unwrap();
    let traces = cases
        .iter()
        .map(|case| {
            case["points"]
                .as_array()
                .unwrap()
                .iter()
                .map(|p| GesturePoint {
                    x: number(&p[0]),
                    y: number(&p[1]),
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    for _ in 0..3 {
        for (case, points) in cases.iter().zip(&traces) {
            verify(
                &classifier.classify(points.iter().copied(), 3)?,
                case,
                &classifier,
            );
        }
    }
    let mut results = Vec::new();
    for (index, (case, points)) in cases.iter().zip(&traces).enumerate() {
        let other = &traces[(index + 1) % traces.len()];
        black_box(classifier.classify(other.iter().copied(), 3)?);
        let (ranked, allocation) = counted(|| classifier.classify(points.iter().copied(), 3));
        let ranked = ranked?;
        verify(&ranked, case, &classifier);
        let mut times = Vec::with_capacity(32);
        for _ in 0..32 {
            // A different retained trace forces a ranking-cache miss without rebuilding
            // the layout/dictionary or discarding the source's per-word length cache.
            black_box(classifier.classify(other.iter().copied(), 3)?);
            let start = Instant::now();
            let next = black_box(classifier.classify(points.iter().copied(), 3)?);
            times.push(u64::try_from(start.elapsed().as_nanos())?);
            verify(&next, case, &classifier);
        }
        let (cached, cache_allocations) =
            counted(|| classifier.classify(points.iter().copied(), 3));
        let cached = cached?;
        verify(&cached, case, &classifier);
        let again = classifier.classify(points.iter().copied(), 3)?;
        assert!(Arc::ptr_eq(&cached, &again));
        assert_eq!(cache_allocations.calls, 0);
        let raw = times.clone();
        times.sort_unstable();
        results.push(json!({
            "case":case["name"],"input_points":points.len(),"retained_points":classifier.point_count(),
            "ranked_utf16":case["ranked"],"cache_miss_samples_ns":raw,
            "p50_ns":times[15],"p95_ns":times[30],
            "uncached_rust_allocations":allocation.calls,"uncached_rust_requested_bytes":allocation.requested_bytes,
            "cached_rust_allocations":cache_allocations.calls,"cached_rust_requested_bytes":cache_allocations.requested_bytes,
            "cached_result_identity_preserved":true
        }));
    }
    let report = json!({
        "schema":1,"profile":"full_dictionary_glide","debug_assertions":cfg!(debug_assertions),
        "os":std::env::consts::OS,"arch":std::env::consts::ARCH,
        "source_revision":fixture["sourceRevision"],"dictionary_words":dictionary.count(),
        "dictionary_first_object_ns":dictionary_load_ns,"dictionary_install_ns":dictionary_install_ns,
        "dictionary_install_rust_allocations":loading_allocations.calls,
        "dictionary_install_rust_requested_bytes":loading_allocations.requested_bytes,
        "cases":results,
        "scope":"32 warmed cache-miss classifications per trace; alternate trace setup, assertions and result drops excluded from time/allocation samples. Standard Rust allocation/reallocation requests on this thread only; requested byte totals are not live/peak heap. Native allocator calls are excluded.",
        "limits":["Three saved native traces and the bundled English dictionary; not every language/path or live preview session.","Shared-host timing noise; no latency budget, device, UI rendering, pointer queue or source-runtime speed comparison.","First dictionary object in process, not guaranteed cold OS caches. Frozen rankings/point counts are exact; no floating-point tolerance."]
    });
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(output)?;
    serde_json::to_writer_pretty(&mut file, &report)?;
    writeln!(file)?;
    println!(
        "Recorded three exact-ranked full-dictionary glide workloads and zero-allocation cache hits."
    );
    Ok(())
}

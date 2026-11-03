//! Opt-in release measurements, not a timing-sensitive test or platform certification.
//! Run: cargo run --release --locked -p ferroboard-core --example performance
use ferroboard_core::{
    engine::KeyboardEngine,
    glide::{GesturePoint, GlideTypingClassifier},
    input::{IKeyboardScheduler, KeyboardTouchProcessor, ScheduledAction},
    keys::KeyboardMode,
    layouts::{KeyComputeContext, KeyboardResources, LayoutComputer, Subtype},
    resources::EmbeddedResources,
    settings::KeyboardSettings,
    text::{Utf16Text, WordDictionary},
};
use serde_json::{Value, json};
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
    error::Error,
    hint::black_box,
    io::Read,
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

// This allocator belongs only to this executable. Timing runs leave counting off;
// a separate pass counts Rust allocation requests, not C/OS allocations or RSS.
struct CountedSystem;
static COUNTING: AtomicBool = AtomicBool::new(false);
static ALLOCATIONS: AtomicU64 = AtomicU64::new(0);
static BYTES: AtomicU64 = AtomicU64::new(0);
fn count(bytes: usize) {
    if COUNTING.load(Ordering::Relaxed) {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        BYTES.fetch_add(bytes as u64, Ordering::Relaxed);
    }
}
// SAFETY: every operation forwards its unchanged allocation contract to System.
unsafe impl GlobalAlloc for CountedSystem {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count(layout.size());
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        count(layout.size());
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        count(size);
        unsafe { System.realloc(ptr, layout, size) }
    }
}
#[global_allocator]
static ALLOCATOR: CountedSystem = CountedSystem;
type Result<T> = std::result::Result<T, Box<dyn Error>>;

// A stopped clock for the subthreshold pointer-move workload. Scheduled actions
// remain owned until cancellation; timer expiry is deliberately outside this probe.
#[derive(Default)]
struct HeldScheduler(Rc<Cell<usize>>);
impl IKeyboardScheduler for HeldScheduler {
    fn schedule(&self, _delay: Duration, action: Rc<dyn Fn()>) -> ScheduledAction {
        self.0.set(self.0.get() + 1);
        let pending = self.0.clone();
        ScheduledAction::new(move || {
            pending.set(pending.get() - 1);
            drop(action);
        })
    }
}

fn measure(
    name: &str,
    iterations: usize,
    mut operation: impl FnMut() -> Result<()>,
) -> Result<Value> {
    assert!(iterations > 0);
    operation()?;
    let mut samples = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        let start = Instant::now();
        operation()?;
        samples.push(start.elapsed().as_secs_f64() * 1_000_000.0);
    }
    samples.sort_by(f64::total_cmp);
    ALLOCATIONS.store(0, Ordering::Relaxed);
    BYTES.store(0, Ordering::Relaxed);
    COUNTING.store(true, Ordering::Relaxed);
    let allocation_result = operation();
    COUNTING.store(false, Ordering::Relaxed);
    allocation_result?;
    Ok(json!({
        "name":name,"iterations":iterations,
        "microseconds":{"min":samples[0],"p50":samples[(iterations-1)/2],
            "p95":samples[(iterations*95).div_ceil(100)-1],"max":samples[iterations-1]},
        "allocation_probe":{"operations":1,"requests":ALLOCATIONS.load(Ordering::Relaxed),
            "requested_bytes":BYTES.load(Ordering::Relaxed)}
    }))
}

fn main() -> Result<()> {
    if cfg!(debug_assertions) {
        return Err("Use --release; debug measurements are not accepted".into());
    }
    let mut results = Vec::new();
    let first = Instant::now();
    let mut decoded_bytes = 0;
    for path in EmbeddedResources::paths() {
        let mut bytes = Vec::new();
        EmbeddedResources::open(path)
            .ok_or("missing bundled resource")?
            .read_to_end(&mut bytes)?;
        decoded_bytes += bytes.len();
        black_box(bytes);
    }
    let first_decode_us = first.elapsed().as_secs_f64() * 1_000_000.0;
    results.push(measure("bundled_registry_construction", 20, || {
        black_box(KeyboardResources::create_default()?);
        Ok(())
    })?);
    let resources = Rc::new(KeyboardResources::create_default()?);
    let computer = LayoutComputer::new(resources);
    results.push(measure(
        "qwerty_layout_composition_and_selectors",
        100,
        || {
            let keyboard =
                computer.compute(KeyboardMode::Characters, Subtype::shared_default(), false)?;
            keyboard.compute(Rc::new(KeyComputeContext::default()))?;
            black_box(keyboard);
            Ok(())
        },
    )?);
    let keyboard = computer.compute(KeyboardMode::Characters, Subtype::shared_default(), false)?;
    keyboard.compute(Rc::new(KeyComputeContext::default()))?;
    let mut size = 0;
    results.push(measure("fractional_layout_resize", 500, || {
        let width = [320.5, 634.25, 1000.75][size % 3];
        size += 1;
        keyboard.layout(width, 240.5, None)?;
        Ok(())
    })?);
    keyboard.layout(1000.0, 240.0, None)?;
    let q = keyboard.find_key(113)?.ok_or("Q key not computed")?;
    let bounds = q.touch_bounds();
    assert!(Rc::ptr_eq(
        &q,
        &keyboard
            .hit_test(bounds.center_x(), bounds.center_y())?
            .ok_or("Q not hit")?
    ));
    let mut hit = measure("hit_test_q_batch_100", 1_000, || {
        for _ in 0..100 {
            black_box(keyboard.hit_test(bounds.center_x(), bounds.center_y())?);
        }
        Ok(())
    })?;
    hit["inner_operations_per_sample"] = json!(100);
    results.push(hit);
    assert!(keyboard.hit_test(-1.0, -1.0)?.is_none());
    let mut miss = measure("hit_test_outside_all_rows_batch_100", 1_000, || {
        for _ in 0..100 {
            black_box(keyboard.hit_test(black_box(-1.0), black_box(-1.0))?);
        }
        Ok(())
    })?;
    miss["inner_operations_per_sample"] = json!(100);
    results.push(miss);
    let settings = Rc::new(KeyboardSettings::new());
    settings.set_learn_words(false);
    settings.set_clipboard_history_enabled(false);
    settings.set_clipboard_suggestion(false);
    let engine = KeyboardEngine::new(Some(settings), Some(computer.resources()), None)?;
    engine.update_layout(1000.0, 240.0, None)?;
    let first_key = engine
        .keyboard()?
        .character_keys()?
        .first()
        .cloned()
        .ok_or("no engine character key")?;
    let rect = first_key.touch_bounds();
    let scheduler = Rc::new(HeldScheduler::default());
    let touch = KeyboardTouchProcessor::new(engine.clone(), scheduler.clone());
    touch.pointer_down(1, rect.center_x(), rect.center_y())?;
    assert!(scheduler.0.get() > 0);
    let mut step = 0;
    let mut moves = measure("stationary_character_pointer_move_batch_100", 1000, || {
        for _ in 0..100 {
            let jitter = if step % 2 == 0 { 0.125 } else { -0.125 };
            step += 1;
            touch.pointer_move(1, rect.center_x() + jitter, rect.center_y())?;
        }
        Ok(())
    })?;
    moves["inner_operations_per_sample"] = json!(100);
    moves["scope"] = json!(
        "One active character pointer, subthreshold jitter, no editor target or timer expiry; no native device/rendering work"
    );
    results.push(moves);
    touch.cancel_all()?;
    assert_eq!(scheduler.0.get(), 0);
    engine.dispose()?;
    results.push(measure("bundled_english_dictionary_load", 5, || {
        black_box(WordDictionary::load_bundled_english()?);
        Ok(())
    })?);
    let dictionary = WordDictionary::load_bundled_english()?;
    let bundled_count = dictionary.count();
    let prefix = Utf16Text::from("hel");
    let typo = Utf16Text::from("helol");
    assert!(!dictionary.complete(&prefix, 3)?.is_empty());
    assert!(!dictionary.find_similar(&typo, 2.0, 3, None)?.is_empty());
    results.push(measure("bundled_completion_hel", 100, || {
        black_box(dictionary.complete(&prefix, 3)?);
        Ok(())
    })?);
    results.push(measure("bundled_fuzzy_helol", 100, || {
        black_box(dictionary.find_similar(&typo, 2.0, 3, None)?);
        Ok(())
    })?);
    let synthetic: Vec<_> = (0..100_000u32)
        .map(|i| {
            // Base-26 digits spread the prefix branches; these are synthetic tokens,
            // not a language corpus or evidence of correction quality.
            let mut n = i;
            let mut word = String::with_capacity(6);
            for _ in 0..6 {
                word.push(char::from(b'a' + (n % 26) as u8));
                n /= 26;
            }
            (Utf16Text::from(word.as_str()), (i % 10_000 + 1) as i32)
        })
        .collect();
    results.push(measure("synthetic_100k_dictionary_build", 3, || {
        black_box(WordDictionary::from_words(
            synthetic.iter().cloned(),
            "en-US".into(),
        )?);
        Ok(())
    })?);
    let large = WordDictionary::from_words(synthetic, "en-US".into())?;
    assert_eq!(large.count(), 100_000);
    let query: Utf16Text = "abc".into();
    results.push(measure("synthetic_100k_completion_abc", 100, || {
        black_box(large.complete(&query, 3)?);
        Ok(())
    })?);
    let fuzzy_query: Utf16Text = "abcaaa".into();
    assert!(large.contains(&fuzzy_query)?);
    results.push(measure("synthetic_100k_fuzzy_abcaaa", 30, || {
        black_box(large.find_similar(&fuzzy_query, 2.0, 3, None)?);
        Ok(())
    })?);
    let classifier = GlideTypingClassifier::default();
    let keys = keyboard.character_keys()?;
    classifier.set_computed_layout(keys.iter().map(|k| k.as_ref()))?;
    results.push(measure("bundled_glide_dictionary_install", 5, || {
        classifier.set_dictionary(&dictionary)?;
        Ok(())
    })?);
    let traces: Vec<Value> = include_str!("../../../tests/fixtures/glide.jsonl")
        .lines()
        .map(serde_json::from_str::<Value>)
        .collect::<std::result::Result<Vec<_>, _>>()?
        .into_iter()
        .filter(|row| row["kind"] == "computed")
        .collect();
    assert!(traces.len() >= 2);
    let point_sets: Vec<Vec<GesturePoint>> = traces
        .iter()
        .map(|trace| {
            trace["points"]
                .as_array()
                .ok_or("invalid glide points")?
                .iter()
                .map(|p| {
                    let number = |v: &Value| -> Result<f64> {
                        Ok(f64::from_bits(u64::from_str_radix(
                            v.as_str().ok_or("invalid float bits")?,
                            16,
                        )?))
                    };
                    Ok(GesturePoint {
                        x: number(&p[0])?,
                        y: number(&p[1])?,
                    })
                })
                .collect::<Result<Vec<_>>>()
        })
        .collect::<Result<_>>()?;
    for (trace, points) in traces.iter().zip(&point_sets) {
        let ranked = classifier.classify(points.iter().copied(), 3)?;
        let units: Vec<_> = ranked.iter().map(|word| word.units()).collect();
        assert_eq!(
            json!(units),
            trace["result"],
            "unchanged C# rankings must match before measuring"
        );
    }
    let cached = classifier.classify(point_sets[0].iter().copied(), 3)?;
    assert!(Arc::ptr_eq(
        &cached,
        &classifier.classify(point_sets[0].iter().copied(), 3)?
    ));
    results.push(measure(
        "bundled_glide_identical_trace_cache_hit",
        100,
        || {
            black_box(classifier.classify(point_sets[0].iter().copied(), 3)?);
            Ok(())
        },
    )?);
    let mut previous = cached;
    let mut next = 1;
    results.push(measure(
        "bundled_glide_alternating_saved_traces",
        100,
        || {
            let ranked =
                classifier.classify(point_sets[next % point_sets.len()].iter().copied(), 3)?;
            assert!(
                !Arc::ptr_eq(&previous, &ranked),
                "recognition measurement must miss the result cache"
            );
            previous = ranked;
            next += 1;
            Ok(())
        },
    )?);
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "schema_version":1,"scope":"Core CPU and Rust allocation observations; not UI, device latency, RSS or certification",
            "os":std::env::consts::OS,"arch":std::env::consts::ARCH,
            "resource_first_decode":{"process_first_pass_us":first_decode_us,"entries":EmbeddedResources::paths().count(),"decoded_bytes":decoded_bytes},
            "bundled_dictionary_words":bundled_count,"synthetic_dictionary_words":large.count(),
            "glide_traces":traces.iter().zip(&point_sets).map(|(trace,points)|json!({"word":trace["word"],"points":points.len(),"expected_rankings_matched":true})).collect::<Vec<_>>(),
            "method":"One warmup, per-operation timing with counting disabled; separate allocation pass. Timings include the disabled allocator flag check. Realloc records its requested new size. No timing thresholds.",
            "measurements":results
        }))?
    );
    Ok(())
}

//! Opt-in desktop Core timing probe. No production behavior, UI, network or user data access.
#[path = "../tests/support/scheduler.rs"]
mod scheduler;

use ferroboard_core::{
    editor::{ITextInputTarget, TextBufferTarget},
    engine::{KeyboardEngine, LanguageModels},
    input::KeyboardTouchProcessor,
    keys::KeyboardMode,
    layouts::KeyboardResources,
    layouts::{KeyComputeContext, KeyComputeContextValues, LayoutComputer, LayoutType, Subtype},
    settings::KeyboardSettings,
    text::{FuzzyMatch, Utf16Text, WordDictionary},
};
use scheduler::ManualScheduler;
use serde_json::{Value, json};
use std::{
    error::Error,
    fs::OpenOptions,
    hint::black_box,
    io::{self, Write},
    path::PathBuf,
    rc::Rc,
    sync::Arc,
    time::Instant,
};

fn timing(mut values: Vec<u64>) -> Value {
    assert!(!values.is_empty());
    values.sort_unstable();
    let rank = |percent: usize| (values.len() * percent).div_ceil(100) - 1;
    json!({
        "samples": values.len(), "min_ns": values[0],
        "p50_ns": values[rank(50)], "p95_ns": values[rank(95)],
        "max_ns": values.last().unwrap(), "sum_measured_ns": values.iter().copied().sum::<u64>()
    })
}
fn ns(start: Instant) -> u64 {
    u64::try_from(start.elapsed().as_nanos()).expect("probe duration fits u64 nanoseconds")
}
fn measure<T>(
    samples: usize,
    mut setup: impl FnMut(usize),
    mut operation: impl FnMut(usize) -> T,
    mut check: impl FnMut(&T),
) -> Value {
    let mut elapsed = Vec::with_capacity(samples);
    for index in 0..samples {
        setup(index);
        let start = Instant::now();
        let result = black_box(operation(index));
        let duration = ns(start);
        check(&result);
        elapsed.push(duration);
    }
    timing(elapsed)
}
fn batches<T>(
    samples: usize,
    operations_per_sample: usize,
    mut operation: impl FnMut(usize) -> T,
    mut check: impl FnMut(),
) -> Value {
    let mut result = measure(
        samples,
        |_| {},
        |batch| {
            for offset in 0..operations_per_sample {
                black_box(operation(batch * operations_per_sample + offset));
            }
        },
        |_| check(),
    );
    result["operations_per_sample"] = json!(operations_per_sample);
    result["scope"] = json!(
        "whole batch nanoseconds, including loop and returned-result drops; checks outside timing"
    );
    result
}
fn pairs(values: &[(Utf16Text, i32)]) -> Value {
    json!(
        values
            .iter()
            .map(|(word, frequency)| json!({"word":word.units(),"frequency":frequency}))
            .collect::<Vec<_>>()
    )
}
fn fuzzy(values: &[FuzzyMatch]) -> Value {
    json!(values.iter().map(|value| json!({"word":value.word.units(),"frequency":value.frequency,"distance":format!("{:016x}",value.distance.to_bits())})).collect::<Vec<_>>())
}
fn candidates(engine: &KeyboardEngine) -> Value {
    json!(
        engine
            .suggestions()
            .snapshot()
            .unwrap()
            .iter()
            .map(|value| {
                let value = value
                    .as_ref()
                    .expect("built-in provider returns nonnull suggestions");
                json!({"text":value.text().map(|text| text.units().to_vec()),
            "kind":format!("{:?}",value.kind()),
            "confidence_bits":format!("{:016x}",value.confidence().to_bits()),
            "auto_commit":value.is_auto_commit()})
            })
            .collect::<Vec<_>>()
    )
}
fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args_os().skip(1);
    let output = args.next().map(PathBuf::from).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "Provide a new JSON output path for this opt-in probe.",
        )
    })?;
    if args.next().is_some() || output.exists() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Expected one new output file; existing reports are never overwritten.",
        )
        .into());
    }
    let source: Vec<Value> = include_str!("../../../tests/fixtures/dictionary.jsonl")
        .lines()
        .map(|line| serde_json::from_str(line).expect("frozen source fixture JSON"))
        .collect();
    let reference = |word: &str| {
        let units: Vec<u16> = word.encode_utf16().collect();
        source
            .iter()
            .find(|row| row["kind"] == "english-query" && row["word"] == json!(units))
            .expect("English source query fixture")
    };
    let registry: Value = serde_json::from_str(
        include_str!("../../../tests/fixtures/registry.jsonl")
            .lines()
            .next()
            .expect("default source registry fixture"),
    )
    .expect("source registry JSON");
    let mut timings = serde_json::Map::new();
    timings.insert(
        "timer_floor".into(),
        measure(10_000, |_| {}, |_| black_box(()), |_| {}),
    );
    timings.insert(
        "batch_loop_floor".into(),
        batches(200, 256, black_box, || {}),
    );

    let start = Instant::now();
    let resources = Rc::new(KeyboardResources::create_default().expect("bundled registry"));
    timings.insert(
        "first_registry_object".into(),
        json!({"samples":1,"elapsed_ns":ns(start)}),
    );
    let mut first_layouts = Vec::new();
    for kind in LayoutType::ALL {
        for name in resources.get_layout_names(kind) {
            let start = Instant::now();
            let layout = resources
                .get_layout(kind, &name)
                .expect("bundled layout load")
                .expect("registered layout");
            let elapsed_ns = ns(start);
            assert!(Rc::ptr_eq(
                &layout,
                &resources.get_layout(kind, &name).unwrap().unwrap()
            ));
            first_layouts.push(json!({"kind":kind.to_id().unwrap(),"name":name.to_text().to_utf8().unwrap(),"elapsed_ns":elapsed_ns}));
        }
    }
    assert_eq!(
        first_layouts
            .iter()
            .map(|row| json!({"type":row["kind"],"name":row["name"]}))
            .collect::<Vec<_>>(),
        registry["layouts"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| json!({"type":row["type"],"name":row["name"]}))
            .collect::<Vec<_>>()
    );
    let name = resources
        .get_layout_names(LayoutType::Characters)
        .into_iter()
        .next()
        .expect("character layouts");
    let cached = resources
        .get_layout(LayoutType::Characters, &name)
        .unwrap()
        .unwrap();
    for _ in 0..32 {
        black_box(
            resources
                .get_layout(LayoutType::Characters, &name)
                .unwrap()
                .unwrap(),
        );
    }
    timings.insert(
        "cached_layout_lookup".into(),
        measure(
            5_000,
            |_| {},
            |_| {
                resources
                    .get_layout(LayoutType::Characters, &name)
                    .unwrap()
                    .unwrap()
            },
            |layout| assert!(Rc::ptr_eq(layout, &cached)),
        ),
    );
    timings.insert(
        "cached_layout_lookup_and_release_batch".into(),
        batches(
            200,
            256,
            |_| {
                resources
                    .get_layout(LayoutType::Characters, &name)
                    .unwrap()
                    .unwrap()
            },
            || {
                assert!(Rc::ptr_eq(
                    &cached,
                    &resources
                        .get_layout(LayoutType::Characters, &name)
                        .unwrap()
                        .unwrap()
                ))
            },
        ),
    );

    let start = Instant::now();
    let dictionary =
        Arc::new(WordDictionary::load_bundled_english().expect("bundled English dictionary"));
    timings.insert(
        "first_english_dictionary_object".into(),
        json!({"samples":1,"elapsed_ns":ns(start)}),
    );
    let english = source.iter().find(|row| row["kind"] == "english").unwrap();
    assert_eq!(
        dictionary.count(),
        english["count"].as_u64().unwrap() as usize
    );
    assert_eq!(
        dictionary.max_frequency(),
        english["max"].as_i64().unwrap() as i32
    );
    let hello = Utf16Text::from("Hello");
    let th = Utf16Text::from("th");
    let helo = Utf16Text::from("helo");
    for _ in 0..32 {
        assert!(dictionary.contains(&hello).unwrap());
    }
    timings.insert(
        "dictionary_contains_hello".into(),
        measure(
            5_000,
            |_| {},
            |_| dictionary.contains(&hello).unwrap(),
            |found| assert!(*found),
        ),
    );
    let expected_complete = &reference("th")["complete"];
    let expected_fuzzy = &reference("helo")["fuzzy"];
    for _ in 0..8 {
        assert_eq!(
            pairs(&dictionary.complete(&th, 5).unwrap()),
            *expected_complete
        );
        assert_eq!(
            fuzzy(&dictionary.find_similar(&helo, 1.0, 10, None).unwrap()),
            *expected_fuzzy
        );
    }
    timings.insert(
        "dictionary_complete_th".into(),
        measure(
            200,
            |_| {},
            |_| dictionary.complete(&th, 5).unwrap(),
            |result| assert_eq!(pairs(result), *expected_complete),
        ),
    );
    timings.insert(
        "dictionary_fuzzy_helo".into(),
        measure(
            200,
            |_| {},
            |_| dictionary.find_similar(&helo, 1.0, 10, None).unwrap(),
            |result| assert_eq!(fuzzy(result), *expected_fuzzy),
        ),
    );

    let computer = LayoutComputer::new(resources.clone());
    for (name, mode) in [
        ("characters", KeyboardMode::Characters),
        ("numeric", KeyboardMode::Numeric),
        ("phone", KeyboardMode::Phone),
    ] {
        let context = Rc::new(KeyComputeContext::from_values(KeyComputeContextValues {
            mode,
            ..Default::default()
        }));
        let build = || {
            let keyboard = computer
                .compute(mode, Subtype::shared_default(), false)
                .unwrap();
            keyboard.compute(context.clone()).unwrap();
            keyboard.layout(1000.25, 240.5, None).unwrap();
            keyboard
        };
        for _ in 0..4 {
            black_box(build());
        }
        timings.insert(
            format!("new_computed_layout_{name}"),
            measure(
                100,
                |_| {},
                |_| build(),
                |keyboard| {
                    assert_eq!(keyboard.width(), 1000.25);
                    assert!(
                        keyboard
                            .visible_keys()
                            .collect::<Result<Vec<_>, _>>()
                            .unwrap()
                            .len()
                            >= 10
                    );
                },
            ),
        );
    }

    let settings = Rc::new(KeyboardSettings::default());
    settings.set_auto_capitalization(false);
    settings.set_show_suggestions(false);
    settings.set_auto_correct(false);
    settings.set_learn_words(false);
    settings.set_glide_typing(false);
    settings.set_clipboard_history_enabled(false);
    settings.set_clipboard_suggestion(false);
    settings.set_emoji_history_enabled(false);
    let models = LanguageModels::new();
    models.register_dictionary(&"en".into(), Some(dictionary.clone()));
    let start = Instant::now();
    let engine = KeyboardEngine::new_explicit(settings, resources.clone(), models).unwrap();
    timings.insert(
        "engine_with_warm_resources_and_dictionary".into(),
        json!({"samples":1,"elapsed_ns":ns(start)}),
    );
    let clock = ManualScheduler::new();
    let engine_clock = clock.clone();
    engine.set_clock(Rc::new(move || {
        std::time::UNIX_EPOCH
            .checked_add(std::time::Duration::from_nanos(
                u64::try_from(engine_clock.now()).expect("nonnegative profiling clock") * 100,
            ))
            .expect("profiling clock within native range")
    }));
    let target = Rc::new(TextBufferTarget::default());
    engine.attach(target.clone()).unwrap();
    engine.update_layout(1000.0, 240.0, None).unwrap();
    let keyboard = engine.keyboard().unwrap();
    let key = keyboard
        .visible_keys()
        .map(|key| key.unwrap().unwrap())
        .find(|key| key.label() == Some("h".into()))
        .unwrap();
    let bounds = key.touch_bounds();
    let (x, y) = (
        bounds.x + bounds.width / 2.0,
        bounds.y + bounds.height / 2.0,
    );
    timings.insert(
        "computed_hit_test_center".into(),
        measure(
            10_000,
            |_| {},
            |_| keyboard.hit_test(x, y).unwrap().expect("key hit"),
            |hit| assert!(Rc::ptr_eq(hit, &key)),
        ),
    );
    timings.insert(
        "computed_hit_test_and_release_batch".into(),
        batches(
            200,
            256,
            |_| keyboard.hit_test(x, y).unwrap().unwrap(),
            || assert!(Rc::ptr_eq(&keyboard.hit_test(x, y).unwrap().unwrap(), &key)),
        ),
    );
    let touch = KeyboardTouchProcessor::new(engine.clone(), clock.clone());
    touch.pointer_down(1, x, y).unwrap();
    for index in 0..32 {
        touch
            .pointer_move(1, x + if index % 2 == 0 { 0.125 } else { -0.125 }, y)
            .unwrap();
    }
    timings.insert(
        "held_subthreshold_pointer_move".into(),
        measure(
            10_000,
            |_| {},
            |index| {
                touch
                    .pointer_move(1, x + if index % 2 == 0 { 0.125 } else { -0.125 }, y)
                    .unwrap()
            },
            |_| assert!(target.text().is_empty()),
        ),
    );
    timings.insert(
        "held_subthreshold_pointer_move_batch".into(),
        batches(
            200,
            256,
            |index| {
                touch
                    .pointer_move(1, x + if index % 2 == 0 { 0.125 } else { -0.125 }, y)
                    .unwrap()
            },
            || assert!(target.text().is_empty()),
        ),
    );
    assert_eq!(touch.pressed_keys().into_iter().count(), 1);
    assert!(touch.popup().is_none());
    touch.cancel_all().unwrap();
    clock.advance(50_000_000);
    assert!(target.text().is_empty());
    let empty = Utf16Text::default();
    let typed = Utf16Text::from("h");
    for _ in 0..32 {
        target.set_text(Some(empty.clone()));
        touch.pointer_down(2, x, y).unwrap();
        touch.pointer_up(2, x, y).unwrap();
        assert_eq!(target.text(), typed);
    }
    timings.insert(
        "full_pointer_tap_empty_buffer".into(),
        measure(
            1_000,
            |_| target.set_text(Some(empty.clone())),
            |_| {
                touch.pointer_down(2, x, y).unwrap();
                touch.pointer_up(2, x, y).unwrap();
            },
            |_| assert_eq!(target.text(), typed),
        ),
    );
    engine.settings().set_show_suggestions(true);
    engine.settings().set_auto_correct(true);
    assert!(engine.settings().emoji_suggestions());
    let o = engine
        .keyboard()
        .unwrap()
        .visible_keys()
        .map(|key| key.unwrap().unwrap())
        .find(|key| key.label() == Some("o".into()))
        .unwrap();
    let bounds = o.touch_bounds();
    let (ox, oy) = (
        bounds.x + bounds.width / 2.0,
        bounds.y + bounds.height / 2.0,
    );
    let prefix = Utf16Text::from("hel");
    let misspelling = Utf16Text::from("helo");
    let prepare_word = || {
        target.set_text(Some(prefix.clone()));
        target.select(3, 0);
    };
    for _ in 0..8 {
        prepare_word();
        touch.pointer_down(3, ox, oy).unwrap();
        touch.pointer_up(3, ox, oy).unwrap();
        assert_eq!(target.text(), misspelling);
        assert!(engine.suggestions().len().unwrap() > 0);
    }
    timings.insert(
        "full_pointer_tap_helo_with_suggestions".into(),
        measure(
            200,
            |_| prepare_word(),
            |_| {
                touch.pointer_down(3, ox, oy).unwrap();
                touch.pointer_up(3, ox, oy).unwrap();
            },
            |_| {
                assert_eq!(target.text(), misspelling);
                assert!(engine.suggestions().len().unwrap() > 0);
            },
        ),
    );
    let correction_candidates = engine.suggestions().len().unwrap();
    let with_emoji = candidates(&engine);
    let catalog = engine.emoji_catalog().unwrap();
    for _ in 0..8 {
        assert!(catalog.search(&misspelling, 1).is_empty());
    }
    timings.insert(
        "emoji_search_helo_no_match".into(),
        measure(
            200,
            |_| {},
            |_| catalog.search(&misspelling, 1),
            |result| assert!(result.is_empty()),
        ),
    );
    engine.settings().set_emoji_suggestions(false);
    for _ in 0..8 {
        prepare_word();
        touch.pointer_down(3, ox, oy).unwrap();
        touch.pointer_up(3, ox, oy).unwrap();
        assert_eq!(candidates(&engine), with_emoji);
    }
    timings.insert(
        "full_pointer_tap_helo_without_emoji_suggestions".into(),
        measure(
            200,
            |_| prepare_word(),
            |_| {
                touch.pointer_down(3, ox, oy).unwrap();
                touch.pointer_up(3, ox, oy).unwrap();
            },
            |_| {
                assert_eq!(target.text(), misspelling);
                assert_eq!(candidates(&engine), with_emoji);
            },
        ),
    );
    touch.cancel_all().unwrap();
    assert_eq!(touch.pressed_keys().into_iter().count(), 0);
    engine.dispose().unwrap();

    let report = json!({
        "schema":2,"status":"passed_behavioral_checks","profile":"see recorded Cargo command; use --release","debug_assertions":cfg!(debug_assertions),
        "os":std::env::consts::OS,"arch":std::env::consts::ARCH,"pointer_width_bits":usize::BITS,
        "quantiles":"nearest rank; raw Instant timings; baseline not subtracted",
        "measurement_scope":"individual calls exclude setup, checks and returned-result drops; marked batches include loop and drops, with whole-batch nanosecond statistics",
        "first_object_scope":"first objects in this process, not guaranteed cold OS caches",
        "warmups":{"cached_layout":32,"dictionary_contains":32,"dictionary_complete_fuzzy":8,"each_computed_layout":4,"held_moves":32,"full_taps":32,"correction_word_taps":8,"emoji_search":8,"without_emoji_word_taps":8},
        "correction_workload":{"prefix_utf16":prefix.units(),"result_utf16":misspelling.units(),"candidate_count":correction_candidates,"show_suggestions":true,"auto_correct":true,"candidates_equal_with_and_without_emoji":true,"ranked_candidates":with_emoji},
        "dictionary":{"words":dictionary.count(),"max_frequency":dictionary.max_frequency(),"completion_th":expected_complete,"fuzzy_helo":expected_fuzzy},
        "cached_layout_name":name.to_text().to_utf8().unwrap(),"registry_names_match_source":true,"first_layout_loads":first_layouts,"timings":timings,
        "limits":["Host load/frequency/OS caches affect timings; no hard timing thresholds.","No per-operation allocations, live bytes or peak heap measurement. External process RSS includes all setup and verification.","Core-only: excludes native input queues, pointer devices, rendering, focus, text shaping, timer callbacks and live glide.","Pointer workloads use a fixed manual clock, disabled learning/glide/history, a warmed English layout and an in-memory target. Only the correction-word tap enables suggestions/autocorrect.","Representative layout computation and dictionary queries only; not every language/mode/input scenario."]
    });
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)?;
    serde_json::to_writer_pretty(&mut file, &report)?;
    writeln!(file)?;
    Ok(())
}

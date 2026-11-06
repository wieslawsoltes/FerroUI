use ferroboard_core::{
    editor::{ITextInputTarget, TextBufferTarget},
    engine::{KeyboardEngine, LanguageModels},
    keys::ShiftState,
    layouts::{KeyRect, KeyboardResources},
    settings::KeyboardSettings,
};
use serde_json::{Value, json};
use std::rc::Rc;

fn number(v: &Value) -> f64 {
    f64::from_bits(u64::from_str_radix(v.as_str().unwrap(), 16).unwrap())
}

#[test]
fn native_shift_caps_and_live_transform_traces_match_unchanged_glide_engine() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/native-glide-casing.json"
    ))
    .unwrap();
    let resources = Rc::new(KeyboardResources::create_default().unwrap());
    for case in fixture["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| matches!(c["name"].as_str().unwrap(), "shift" | "caps" | "geometry"))
    {
        let settings = Rc::new(KeyboardSettings::default());
        settings.set_auto_capitalization(false);
        settings.set_auto_correct(false);
        settings.set_glide_typing(true);
        settings.set_glide_preview(true);
        settings.set_suggestion_count(3);
        settings.set_learn_words(false);
        let engine =
            KeyboardEngine::new_explicit(settings, resources.clone(), LanguageModels::new())
                .unwrap();
        let target = Rc::new(TextBufferTarget::default());
        engine.attach(target.clone()).unwrap();
        engine
            .set_shift_state(ShiftState(case["shift"].as_i64().unwrap() as i32))
            .unwrap();
        let classifier = engine.glide_classifier();
        classifier
            .set_layout(fixture["layout"].as_array().unwrap().iter().map(|key| {
                let r = &key["r"];
                (
                    key["c"].as_u64().unwrap() as u16,
                    KeyRect::new(number(&r[0]), number(&r[1]), number(&r[2]), number(&r[3])),
                )
            }))
            .unwrap();
        let points = case["points"].as_array().unwrap();
        engine
            .begin_glide(number(&points[0][0]), number(&points[0][1]))
            .unwrap();
        for p in &points[1..] {
            engine
                .add_glide_point(number(&p[0]), number(&p[1]))
                .unwrap();
        }
        assert_eq!(
            classifier.point_count(),
            case["pointCount"].as_u64().unwrap() as usize
        );
        engine.finish_glide(true).unwrap();
        assert_eq!(
            json!(target.text().units()),
            case["committed"],
            "{}",
            case["name"]
        );
        assert_eq!(
            engine.shift_state().0,
            case["shiftAfter"].as_i64().unwrap() as i32
        );
        let ranked: Vec<_> = engine
            .suggestions()
            .snapshot()
            .unwrap()
            .into_iter()
            .map(|s| s.unwrap().text().unwrap().units().to_vec())
            .collect();
        assert_eq!(json!(ranked), case["ranked"], "{}", case["name"]);
    }
}

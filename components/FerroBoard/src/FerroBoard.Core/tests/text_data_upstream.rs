//! Exact request limits and finger interpolation from the pinned TextAndDataTests.cs.
//! Remaining methods map to audited existing tests in docs/upstream-core-suite.json.
use ferroboard_core::{glide::*, keys::*, layouts::*, text::*};
use std::{rc::Rc, sync::Arc};
fn english() -> Arc<WordDictionary> {
    Arc::new(WordDictionary::load_bundled_english().unwrap())
}
#[test]
fn provider_suggests_corrections_and_marks_autocorrect() {
    let provider = DictionarySuggestionProvider::new(english(), None);
    let result = provider
        .suggest(&SuggestionRequest::from_values(SuggestionRequestValues {
            word: Some("becuase".into()),
            max_count: 3,
            ..Default::default()
        }))
        .unwrap()
        .snapshot()
        .unwrap();
    assert!(
        result
            .iter()
            .any(|s| s.as_ref().unwrap().text() == Some("because".into()))
    );
    assert!(result.iter().any(|s| {
        let s = s.as_ref().unwrap();
        s.is_auto_commit() && s.text() == Some("because".into())
    }));
}
#[test]
fn provider_keeps_case_pattern() {
    let provider = DictionarySuggestionProvider::new(english(), None);
    let result = provider
        .suggest(&SuggestionRequest::from_values(SuggestionRequestValues {
            word: Some("Tha".into()),
            max_count: 3,
            ..Default::default()
        }))
        .unwrap()
        .snapshot()
        .unwrap();
    for suggestion in result {
        let text = suggestion.unwrap().text().unwrap();
        // The source indexes a UTF-16 char. These English results have BMP uppercase initials.
        assert!(
            char::from_u32(text.units()[0] as u32)
                .unwrap()
                .is_uppercase()
        );
    }
}
#[test]
fn glide_classifier_recognizes_word_from_ideal_path() {
    let keyboard = LayoutComputer::new(Rc::new(KeyboardResources::create_default().unwrap()))
        .compute(KeyboardMode::Characters, Subtype::shared_default(), false)
        .unwrap();
    keyboard
        .compute(Rc::new(KeyComputeContext::default()))
        .unwrap();
    keyboard.layout(1000.0, 240.0, None).unwrap();
    let classifier = GlideTypingClassifier::default();
    let keys = keyboard.character_keys().unwrap();
    classifier
        .set_computed_layout(keys.iter().map(|k| k.as_ref()))
        .unwrap();
    classifier.set_dictionary(&english()).unwrap();
    for word in ["hello", "world", "keyboard", "the"] {
        let mut points = Vec::new();
        let mut previous: Option<Rc<ComputedKey>> = None;
        for c in word.chars() {
            let key = keys
                .iter()
                .find(|k| k.data().as_string(false).unwrap() == Some(c.to_string().as_str().into()))
                .unwrap()
                .clone();
            if let Some(from) = previous {
                let mut t = 0.1;
                while t < 1.0 {
                    points.push(GesturePoint {
                        x: from.visible_bounds().center_x()
                            + (key.visible_bounds().center_x() - from.visible_bounds().center_x())
                                * t,
                        y: from.visible_bounds().center_y()
                            + (key.visible_bounds().center_y() - from.visible_bounds().center_y())
                                * t,
                    });
                    t += 0.1;
                }
            }
            points.push(GesturePoint {
                x: key.visible_bounds().center_x(),
                y: key.visible_bounds().center_y(),
            });
            previous = Some(key);
        }
        let result = classifier.classify(points, 5).unwrap();
        assert!(
            result.iter().take(3).any(|v| *v == word.into()),
            "{word}: {result:?}"
        );
    }
}

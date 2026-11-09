//! Process-wide identities measured from the unchanged C# implementation on fresh OS threads.
use ferroboard_core::{
    layouts::*,
    text::{AppenderComposer, IComposer},
};
use serde_json::json;
use std::sync::{Arc, Barrier};
fn assert_send_sync<T: Send + Sync>() {}
#[test]
fn unchanged_csharp_default_objects_are_shared_across_fresh_threads() {
    assert_send_sync::<Subtype>();
    assert_send_sync::<SubtypeLayoutMap>();
    assert_send_sync::<PunctuationRule>();
    assert_send_sync::<SharedKeyEvaluator>();
    let mut expected: serde_json::Value =
        serde_json::from_str(include_str!("../../../tests/fixtures/shared-defaults.json")).unwrap();
    for key in ["evaluatorCloneHash", "subtypeCloneHash", "mapCloneHash"] {
        expected["equality"].as_object_mut().unwrap().remove(key);
    }
    let count = expected["threads"].as_u64().unwrap() as usize;
    let barrier = Arc::new(Barrier::new(count + 1));
    let jobs = (0..count)
        .map(|_| {
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                let subtype = Subtype::shared_default();
                let registry = KeyboardResources::create_empty();
                let missing = ComponentName::new("no", "rule");
                let rule = registry.get_punctuation_rule(&missing);
                let evaluator = KeyEvaluator::shared_default();
                let map = subtype.layouts().unwrap().clone();
                let appender = AppenderComposer::instance();
                let registered = registry.get_composer(&ComponentName::new("no", "composer"));
                let expected: Arc<dyn IComposer> = appender.clone();
                assert!(Arc::ptr_eq(&expected, &registered));
                (subtype, rule, evaluator, map, appender)
            })
        })
        .collect::<Vec<_>>();
    barrier.wait();
    let values = jobs
        .into_iter()
        .map(|j| j.join().unwrap())
        .collect::<Vec<_>>();
    let (subtype, rule, evaluator, map, appender) = &values[0];
    let evaluator_clone = KeyEvaluator::from_values(evaluator.values());
    let invariant_override = KeyEvaluator::from_values(KeyEvaluatorValues {
        culture_override: Some(ferroboard_core::text::TextCulture::invariant()),
        ..KeyEvaluator::from_values(evaluator.values()).values()
    });
    let subtype_clone = Arc::new((**subtype).clone());
    let map_clone = Arc::new((**map).clone());
    let changed_map = Arc::new(SubtypeLayoutMap::from_values(
        ferroboard_core::layouts::SubtypeLayoutMapValues {
            characters: ComponentName::new("custom", "other"),
            ..(**map).values()
        },
    ));
    let english = |name| {
        KeyEvaluator::from_values(KeyEvaluatorValues {
            culture_override: Some(ferroboard_core::text::TextCulture::get(name).unwrap()),
            ..KeyEvaluatorValues::default()
        })
    };
    let actual = json!({"threads":count,
 "equality":{
 "evaluatorClone":evaluator.as_evaluator()==evaluator_clone,"evaluatorCloneIdentity":evaluator.as_evaluator().ptr_eq(&evaluator_clone),"evaluatorExplicitInvariant":evaluator.as_ref()==&invariant_override,"evaluatorSameComputedCulture":evaluator.culture()==invariant_override.culture(),"evaluatorCultureName":english("en-US")==english("EN-us"),"evaluatorDifferentCulture":english("en-US")==english("en-GB"),
 "subtypeClone":subtype==&subtype_clone,"subtypeCloneIdentity":Arc::ptr_eq(subtype,&subtype_clone),"subtypeClonedMap":subtype.as_ref()==&Subtype::from_values(ferroboard_core::layouts::SubtypeValues{layouts:Some(map_clone.clone()),..(**subtype).values()}),"subtypeChangedMap":subtype.as_ref()==&Subtype::from_values(ferroboard_core::layouts::SubtypeValues{layouts:Some(changed_map.clone()),..(**subtype).values()}),"subtypeSameIdDifferentFields":subtype.as_ref()==&Subtype::new("en-US".into()),
 "mapClone":map==&map_clone,"mapCloneIdentity":Arc::ptr_eq(map,&map_clone),"mapChanged":map==&changed_map,"punctuationSameFields":rule.as_ref()==&PunctuationRule::default()
 },
 "subtype":values.iter().all(|v|Arc::ptr_eq(&v.0,subtype)),"rule":values.iter().all(|v|Arc::ptr_eq(&v.1,rule)),"evaluator":values.iter().all(|v|Arc::ptr_eq(&v.2,evaluator)),"map":values.iter().all(|v|Arc::ptr_eq(&v.3,map)),"appender":values.iter().all(|v|Arc::ptr_eq(&v.4,appender)),
 "subtypeId":subtype.id().to_utf8().unwrap(),"popup":subtype.popup_mapping().to_text().to_utf8().unwrap(),"terminating":rule.symbols_terminating_sentence().unwrap().to_utf8().unwrap(),"mode":evaluator.mode().0,"shift":evaluator.shift_state().0,"variation":evaluator.variation().0,"direction":evaluator.direction().0,"halfWidth":evaluator.is_char_half_width(),"kana":evaluator.is_kana_kata(),"culture":evaluator.culture().unwrap().name(),
 "independentSubtype":!Arc::ptr_eq(&Arc::new(Subtype::new("en-US".into())),subtype),"independentMap":!Arc::ptr_eq(&Arc::new(SubtypeLayoutMap::default()),map),"independentRule":!Arc::ptr_eq(&Arc::new(PunctuationRule::default()),rule),"independentEvaluator":!evaluator.as_evaluator().ptr_eq(&KeyEvaluator::default())});
    assert_eq!(actual, expected);
    // The calling thread and separately-created local registries observe the same original objects.
    assert!(Arc::ptr_eq(subtype, &Subtype::shared_default()));
    assert!(Arc::ptr_eq(evaluator, &KeyEvaluator::shared_default()));
    assert!(Arc::ptr_eq(rule, &PunctuationRule::shared_default()));
}

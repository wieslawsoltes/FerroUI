//! Actual source record values, clones, runtime init accessors and shared Default identity.
use ferroboard_core::editor::*;
use serde_json::{Value, json};
use std::{
    cell::Cell,
    hash::{Hash, Hasher},
    rc::Rc,
    sync::Arc,
};
fn rows() -> Vec<Value> {
    include_str!("../../../tests/fixtures/input-record.jsonl")
        .lines()
        .map(|s| {
            let mut v: Value = serde_json::from_str(s).unwrap();
            strip_runtime_fields(&mut v);
            v
        })
        .collect()
}
fn strip_runtime_fields(v: &mut Value) {
    match v {
        Value::Object(fields) => {
            for key in ["text", "objectEqual", "nullEqual", "wrongEqual"] {
                fields.remove(key);
            }
            for value in fields.values_mut() {
                strip_runtime_fields(value);
            }
        }
        Value::Array(values) => {
            for value in values {
                strip_runtime_fields(value);
            }
        }
        _ => {}
    }
}
fn values(s: &Value) -> InputAttributesValues {
    InputAttributesValues {
        kind: InputKind(s["kind"].as_i64().unwrap() as i32),
        enter_action: EnterAction(s["action"].as_i64().unwrap() as i32),
        capitalization: CapitalizationMode(s["cap"].as_i64().unwrap() as i32),
        is_multiline: s["multi"].as_bool().unwrap(),
        allow_suggestions: s["suggest"].as_bool().unwrap(),
        allow_auto_correct: s["correct"].as_bool().unwrap(),
        is_private: s["privateField"].as_bool().unwrap(),
    }
}
fn state(a: &InputAttributes) -> Value {
    json!({"kind":a.kind().0,"action":a.enter_action().0,"cap":a.capitalization().0,"multi":a.is_multiline(),"suggest":a.allow_suggestions(),"correct":a.allow_auto_correct(),"privateField":a.is_private(),"password":a.is_password(),"variation":a.variation().0,"mode":a.initial_mode().0,"effective":a.effective_enter_action().0,"nlp":a.supports_nlp()})
}
fn hash(a: &InputAttributes) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    a.hash(&mut h);
    h.finish()
}
#[test]
fn original_record_clone_equality_text_and_constructor_values() {
    for row in rows().into_iter().filter(|v| v["tag"] == "record") {
        let a = Arc::new(InputAttributes::from_values(values(&row["state"])));
        let clone = Arc::new(a.as_ref().clone());
        let same = a.with_values(|v| v.kind = a.kind());
        let changed = a.with_values(|v| v.kind = InputKind(12345));
        let target = TextBufferTarget::new("target".into(), Some(a.clone()));
        assert_eq!(
            json!({"tag":"record","i":row["i"],"profile":row["profile"],"state":state(&a),"clone":state(&clone),"cloneSame":Arc::ptr_eq(&a,&clone),"sameValueSame":Arc::ptr_eq(&a,&same),"equal":a==clone,"operatorEqual":a==clone,"hashEqual":hash(&a)==hash(&clone),"changedEqual":a==changed,"targetSame":Arc::ptr_eq(&a,&target.attributes())}),
            row
        );
        assert_eq!(format!("{a:?}"), a.to_string());
    }
}
#[test]
fn original_runtime_init_retains_alias_target_and_independent_clone_without_notifications() {
    let data = rows();
    for base in data.iter().filter(|v| v["tag"] == "record") {
        let i = base["i"].as_i64().unwrap();
        let profile = base["profile"].as_i64().unwrap();
        let a = Arc::new(InputAttributes::from_values(values(&base["state"])));
        let clone = Arc::new(a.as_ref().clone());
        let alias = a.clone();
        let target = Rc::new(TextBufferTarget::new("target".into(), Some(a.clone())));
        let events = Rc::new(Cell::new(0));
        let count = events.clone();
        target
            .changed()
            .add(Rc::new(move |_| count.set(count.get() + 1)));
        let initial = a.values();
        for row in data
            .iter()
            .filter(|v| v["tag"] == "init" && v["i"] == i && v["profile"] == profile)
        {
            let step = row["step"].as_i64().unwrap();
            match step {
                0 => alias.set_kind(InputKind(i32::MIN)),
                1 => alias.set_enter_action(EnterAction(i32::MAX)),
                2 => alias.set_capitalization(CapitalizationMode(-1)),
                3 => alias.set_is_multiline(!initial.is_multiline),
                4 => alias.set_allow_suggestions(!initial.allow_suggestions),
                5 => alias.set_allow_auto_correct(!initial.allow_auto_correct),
                6 => alias.set_is_private(!initial.is_private),
                _ => panic!("unknown initializer"),
            };
            assert_eq!(
                json!({"tag":"init","i":i,"profile":profile,"step":step,"state":state(&a),"clone":state(&clone),"aliasSame":Arc::ptr_eq(&a,&alias),"targetSame":Arc::ptr_eq(&a,&target.attributes()),"target":state(&target.attributes()),"equal":a==clone,"events":events.get()}),
                *row
            );
        }
    }
}
struct DefaultRestore(Arc<InputAttributes>, InputAttributesValues);
impl Drop for DefaultRestore {
    fn drop(&mut self) {
        let a = &self.0;
        let v = self.1;
        a.set_kind(v.kind);
        a.set_enter_action(v.enter_action);
        a.set_capitalization(v.capitalization);
        a.set_is_multiline(v.is_multiline);
        a.set_allow_suggestions(v.allow_suggestions);
        a.set_allow_auto_correct(v.allow_auto_correct);
        a.set_is_private(v.is_private);
    }
}
#[test]
fn original_default_retains_process_worker_target_identity_after_runtime_init() {
    let row = rows().into_iter().find(|v| v["tag"] == "default").unwrap();
    let a = InputAttributes::shared_default();
    let _restore = DefaultRestore(a.clone(), a.values());
    let original = Arc::new(a.as_ref().clone());
    let target = TextBufferTarget::new("default".into(), None);
    a.set_kind(InputKind(99));
    let worker = std::thread::spawn(InputAttributes::shared_default)
        .join()
        .unwrap();
    assert_eq!(
        json!({"tag":"default","same":Arc::ptr_eq(&a,&InputAttributes::shared_default()),"workerSame":Arc::ptr_eq(&a,&worker),"targetSame":Arc::ptr_eq(&a,&target.attributes()),"state":state(&worker),"cloneSame":Arc::ptr_eq(&a,&original),"clone":state(&original)}),
        row
    );
}

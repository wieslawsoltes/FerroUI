use ferroboard_core::{
    layouts::{ComposerDefinition, KeyboardResources, LayoutJson},
    text::{
        AppenderComposer, ComposerRules, HangulComposer, IComposer, KanaComposer, RulesComposer,
        Utf16Text,
    },
};
use serde_json::{Value, json};
use std::{sync::Arc, thread};

fn text(value: &Value) -> Utf16Text {
    Utf16Text::from_units(
        value
            .as_array()
            .unwrap()
            .iter()
            .map(|n| n.as_u64().unwrap() as u16)
            .collect::<Vec<_>>(),
    )
}
fn rows() -> Vec<Value> {
    include_str!("../../../tests/fixtures/composer-ownership.jsonl")
        .lines()
        .map(|row| {
            let mut value: Value = serde_json::from_str(row).unwrap();
            // Native HashMap iteration is unspecified. Compare map contents canonically while
            // retaining exact source actions, identity, threading and snapshot observations.
            value.as_object_mut().unwrap().remove("ordered");
            value["rules"]
                .as_array_mut()
                .unwrap()
                .sort_by(|left, right| text(&left["key"]).units().cmp(text(&right["key"]).units()));
            value
        })
        .collect()
}
fn capture(
    phase: &str,
    composer: &RulesComposer,
    definition: &ComposerDefinition,
    cases: &[(Utf16Text, Utf16Text)],
    main: thread::ThreadId,
) -> Value {
    let rules = composer.rules();
    let mut entries = rules
        .read()
        .iter()
        .map(|(k, v)| json!({"key":k.units(),"value":v.units()}))
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| text(&left["key"]).units().cmp(text(&right["key"]).units()));
    let actions = cases
        .iter()
        .map(|(before, insert)| {
            let (delete, insert) = composer.get_actions(before, insert);
            json!({"delete":delete,"insert":insert.units()})
        })
        .collect::<Vec<_>>();
    json!({
        "phase":phase,"worker":thread::current().id()!=main,
        "sameRules":Arc::ptr_eq(&rules,&definition.rules),
        "id":composer.id().units(),"label":composer.label().units(),"toRead":composer.to_read(),
        "rules":entries,"actions":actions,
    })
}
fn construct(definition: &ComposerDefinition) -> Arc<RulesComposer> {
    Arc::new(RulesComposer::new(
        definition.id.clone(),
        definition.label.clone(),
        definition.rules.clone(),
    ))
}

#[test]
fn source_original_rules_identity_worker_changes_snapshot_and_bundled_definitions() {
    let rows = rows();
    let input = &rows[0];
    assert_eq!(input["phase"], "input");
    let cases = input["cases"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| (text(&c["before"]), text(&c["insert"])))
        .collect::<Vec<_>>();
    let rules = ComposerRules::default();
    for entry in input["rules"].as_array().unwrap() {
        rules
            .write()
            .insert(text(&entry["key"]), text(&entry["value"]));
    }
    let definition = Arc::new(ComposerDefinition {
        label: "Source probe".into(),
        rules: rules.clone(),
        ..ComposerDefinition::new("with-rules".into(), "source-probe".into())
    });
    let composer = construct(&definition);
    let main = thread::current().id();
    assert_eq!(
        capture("initial", &composer, &definition, &cases, main),
        rows[1]
    );
    let worker = {
        let composer = composer.clone();
        let definition = definition.clone();
        let cases = cases.clone();
        thread::spawn(move || {
            {
                let mut rules = definition.rules.write();
                rules.insert("aa".into(), "changed".into());
                rules.remove(&Utf16Text::from("aaa"));
                rules.remove(&Utf16Text::from("zz"));
                rules.insert("longer".into(), "new-long".into());
                rules.insert("qq".into(), "new-q".into());
            }
            capture("worker-mutated", &composer, &definition, &cases, main)
        })
    };
    assert_eq!(worker.join().unwrap(), rows[2]);
    {
        let mut rules = rules.write();
        rules.clear();
        rules.insert(
            Utf16Text::from_units(vec![b'x' as u16, 0xd800]),
            Utf16Text::from_units(vec![0xd800]),
        );
        rules.insert("ss".into(), "ß".into());
        rules.insert("".into(), "still-ignored".into());
    }
    assert_eq!(
        capture("cleared", &composer, &definition, &cases, main),
        rows[3]
    );
    let worker = {
        let definition = definition.clone();
        let cases = cases.clone();
        thread::spawn(move || {
            let rebuilt = construct(&definition);
            let factory = KeyboardResources::create_composer(&definition);
            for (before, insert) in &cases {
                assert_eq!(
                    factory.get_actions(before, insert),
                    rebuilt.get_actions(before, insert)
                );
            }
            capture("worker-rebuilt", &rebuilt, &definition, &cases, main)
        })
    };
    assert_eq!(worker.join().unwrap(), rows[4]);
    let resources = KeyboardResources::create_default().unwrap();
    let mut bundled = 0;
    for manifest in resources.extensions() {
        for definition in manifest.composers.borrow().iter() {
            if definition.kind != Utf16Text::from("with-rules") {
                continue;
            }
            let phase = format!("bundled:{}", definition.id.to_utf8().unwrap());
            let expected = rows.iter().find(|r| r["phase"] == phase).unwrap();
            assert_eq!(
                capture(&phase, &construct(definition), definition, &cases, main),
                *expected
            );
            // Parsed definitions are the same original Arc object when handed to a worker.
            let definition = definition.clone();
            let original = definition.clone();
            let cases = cases.clone();
            thread::spawn(move || {
                assert!(Arc::ptr_eq(&definition, &original));
                let factory = KeyboardResources::create_composer(&definition);
                let concrete = construct(&definition);
                for (before, insert) in cases {
                    assert_eq!(
                        factory.get_actions(&before, &insert),
                        concrete.get_actions(&before, &insert)
                    );
                }
            })
            .join()
            .unwrap();
            bundled += 1;
        }
    }
    assert!(bundled > 0);
    assert_eq!(bundled + 5, rows.len());
}

#[test]
fn builtin_composers_are_shareable_and_actions_do_not_lock_the_retained_rules() {
    fn send_sync<T: Send + Sync>() {}
    send_sync::<ComposerDefinition>();
    send_sync::<ComposerRules>();
    send_sync::<RulesComposer>();
    send_sync::<AppenderComposer>();
    send_sync::<HangulComposer>();
    send_sync::<KanaComposer>();
    let rules = ComposerRules::default();
    rules.write().insert("aa".into(), "â".into());
    let composer = Arc::new(RulesComposer::new(
        "id".into(),
        "Label".into(),
        rules.clone(),
    ));
    let mut writer = rules.write();
    writer.insert("aa".into(), "changed".into());
    let (sender, receiver) = std::sync::mpsc::channel();
    let worker = {
        let composer = composer.clone();
        thread::spawn(move || {
            sender
                .send(composer.get_actions(&"a".into(), &"a".into()))
                .unwrap()
        })
    };
    let result = receiver.recv_timeout(std::time::Duration::from_secs(5));
    drop(writer);
    worker.join().unwrap();
    assert_eq!(result.unwrap(), (1, "â".into()));
    let barrier = Arc::new(std::sync::Barrier::new(9));
    let workers = (0..8)
        .map(|_| {
            let composer = composer.clone();
            let barrier = barrier.clone();
            thread::spawn(move || {
                barrier.wait();
                assert_eq!(
                    composer.get_actions(&"A".into(), &"a".into()),
                    (1, "Â".into())
                );
            })
        })
        .collect::<Vec<_>>();
    barrier.wait();
    for worker in workers {
        worker.join().unwrap();
    }

    // The IComposer contract still admits local custom implementations; no Send/Sync bound
    // has been added to the open interface merely to make builtin storage shareable.
    struct Local(std::rc::Rc<std::cell::Cell<i32>>);
    impl IComposer for Local {
        fn id(&self) -> Utf16Text {
            "local".into()
        }
        fn label(&self) -> Utf16Text {
            "Local".into()
        }
        fn to_read(&self) -> i32 {
            0
        }
        fn get_actions(&self, _: &Utf16Text, insert: &Utf16Text) -> (i32, Utf16Text) {
            self.0.set(self.0.get() + 1);
            (0, insert.clone())
        }
    }
    let calls = std::rc::Rc::new(std::cell::Cell::new(0));
    let resources = KeyboardResources::create_empty();
    #[allow(
        clippy::arc_with_non_send_sync,
        reason = "Preserve the unrestricted original custom IComposer contract"
    )]
    let local = Arc::new(Local(calls.clone()));
    resources.add_composer(local, Some("custom".into()));
    assert_eq!(
        resources
            .get_composer(&"custom:local".into())
            .get_actions(&"".into(), &"x".into()),
        (0, "x".into())
    );
    assert_eq!(calls.get(), 1);

    // Parser defaults retain a shared, originally mutable dictionary, not a detached copy.
    let manifest = LayoutJson::read_extension_manifest(
        r#"{"meta":{"id":"custom"},"composers":[{"$":"with-rules","id":"blank"}]}"#,
    )
    .unwrap();
    let definition = manifest.composers.borrow()[0].clone();
    assert!(definition.rules.read().is_empty());
    let empty = construct(&definition);
    definition.rules.write().insert("ab".into(), "value".into());
    assert_eq!(empty.to_read(), 0);
    assert_eq!(empty.get_actions(&"a".into(), &"b".into()), (0, "b".into()));
    assert!(Arc::ptr_eq(&empty.rules(), &definition.rules));
}

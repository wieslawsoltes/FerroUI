use ferroboard_core::{keys::*, layouts::*, text::Utf16Text};
use serde_json::{Value, json};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
};
fn rows() -> Vec<Value> {
    include_str!("../../../tests/fixtures/row-reentrancy.jsonl")
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect()
}
fn mutate(rows: &ComputedRows, op: &str) {
    match op {
        "none" => {}
        "outer-clear" => rows.clear(),
        "outer-set-same" => rows.set(0, rows.get(0).unwrap()).unwrap(),
        "outer-remove-last" => {
            rows.remove(rows.len() - 1).unwrap();
        }
        "outer-add" => rows.push(ComputedRow::new(vec![rows.get(1).unwrap().get(0).unwrap()])),
        "inner-clear" => rows.get(0).unwrap().clear(),
        "inner-set-same" => {
            let r = rows.get(0).unwrap();
            r.set(0, r.get(0).unwrap()).unwrap();
        }
        "inner-remove-last" => {
            let r = rows.get(0).unwrap();
            r.remove(r.len() - 1).unwrap();
        }
        _ => panic!("unknown mutation {op}"),
    }
}
struct Probe {
    name: &'static str,
    enabled: Rc<Cell<bool>>,
    fired: Rc<Cell<bool>>,
    hide_on_mutate: bool,
    always_hidden: bool,
    mutation: &'static str,
    owner: Rc<RefCell<Option<ComputedRows>>>,
    trace: Rc<RefCell<Vec<String>>>,
}
impl AbstractKeyData for Probe {
    fn compute_nullable(
        self: Rc<Self>,
        e: Option<&dyn IKeyEvaluator>,
    ) -> Result<Option<TextKey>, KeyDataError> {
        self.trace.borrow_mut().push(format!(
            "{}:{}",
            self.name,
            e.ok_or(KeyDataError::NullReference)?.shift_state().0
        ));
        if self.name == "a" && self.enabled.get() && !self.fired.replace(true) {
            self.trace
                .borrow_mut()
                .push(format!("mutate:{}", self.mutation));
            let owner = self.owner.borrow().as_ref().unwrap().clone();
            mutate(&owner, self.mutation);
        }
        if self.always_hidden || (self.hide_on_mutate && self.enabled.get()) {
            return Ok(None);
        }
        let label = if self.name == "a" && self.enabled.get() {
            "x"
        } else {
            self.name
        };
        Ok(Some(TextKeyData::char(label.into(), None)?))
    }
    fn as_string(&self, _: bool) -> Result<Option<Utf16Text>, KeyDataError> {
        Ok(Some(self.name.into()))
    }
}
fn build(keys: Vec<Vec<Key>>) -> ComputedKeyboard {
    let resources = Rc::new(KeyboardResources::create_empty());
    resources.add_layout(Rc::new(KeyboardLayout::new(
        LayoutType::Characters,
        "probe".into(),
        "main".into(),
        Rc::new(LayoutArrangement::from_rows(keys)),
    )));
    let subtype = Arc::new(Subtype::from_values(
        ferroboard_core::layouts::SubtypeValues {
            layouts: Some(Arc::new(SubtypeLayoutMap::from_values(
                ferroboard_core::layouts::SubtypeLayoutMapValues {
                    characters: ComponentName::new("probe", "main"),
                    ..(*Subtype::shared_default().layouts().unwrap()).values()
                },
            ))),
            ..(*Subtype::shared_default()).values()
        },
    ));
    LayoutComputer::new(resources)
        .compute(KeyboardMode::Characters, subtype, false)
        .unwrap()
}
const OPS: [&str; 8] = [
    "none",
    "outer-clear",
    "outer-set-same",
    "outer-remove-last",
    "outer-add",
    "inner-clear",
    "inner-set-same",
    "inner-remove-last",
];
#[test]
fn original_compute_row_mutation_order_context_partial_state_version_and_recovery() {
    let expected = rows();
    for mutation in OPS {
        for hidden in [false, true] {
            let enabled = Rc::new(Cell::new(false));
            let fired = Rc::new(Cell::new(false));
            let owner = Rc::new(RefCell::new(None));
            let trace = Rc::new(RefCell::new(Vec::new()));
            let make = |name| {
                Rc::new(Probe {
                    name,
                    enabled: enabled.clone(),
                    fired: fired.clone(),
                    hide_on_mutate: name == "a" && hidden,
                    always_hidden: false,
                    mutation,
                    owner: owner.clone(),
                    trace: trace.clone(),
                }) as Key
            };
            let keyboard = build(vec![vec![make("a"), make("b")], vec![make("c")]]);
            owner.replace(Some(keyboard.rows()));
            let original = keyboard
                .keys()
                .map(|key| key.unwrap().unwrap())
                .collect::<Vec<_>>();
            for phase in ["warm", "mutate", "recover"] {
                enabled.set(phase == "mutate");
                trace.borrow_mut().clear();
                let context = Rc::new(KeyComputeContext::from_values(
                    ferroboard_core::layouts::KeyComputeContextValues {
                        show_number_hints: false,
                        show_symbol_hints: false,
                        ..Default::default()
                    },
                ));
                let error = match keyboard.compute(context.clone()) {
                    Ok(()) => None,
                    Err(KeyDataError::Collection(ResourceTextListError::Modified)) => {
                        Some("InvalidOperationException")
                    }
                    Err(e) => panic!("unexpected {e:?}"),
                };
                let row_ids = keyboard
                    .rows()
                    .snapshot()
                    .unwrap()
                    .into_iter()
                    .map(|r| {
                        r.snapshot()
                            .unwrap()
                            .iter()
                            .map(|key| original.iter().position(|v| Rc::ptr_eq(v, key)).unwrap())
                            .collect::<Vec<_>>()
                    })
                    .collect::<Vec<_>>();
                let state=original.iter().map(|k|json!({"code":k.code(),"label":k.label().map(|s|s.to_utf8().unwrap()),"visible":k.is_visible()})).collect::<Vec<_>>();
                let actual = json!({"kind":"compute","mutation":mutation,"hidden":hidden,"phase":phase,"error":error,"trace":*trace.borrow(),"version":keyboard.version(),"contextSet":Rc::ptr_eq(&keyboard.context().unwrap(),&context),"rows":row_ids,"state":state});
                assert_eq!(
                    actual,
                    *expected
                        .iter()
                        .find(|r| r["kind"] == "compute"
                            && r["mutation"] == mutation
                            && r["hidden"] == hidden
                            && r["phase"] == phase)
                        .unwrap()
                );
            }
            owner.take();
        }
    }
}
fn moved(result: Result<Option<Rc<ComputedKey>>, KeyDataError>) -> Value {
    match result {
        Ok(v) => json!({"value":v.is_some()}),
        Err(KeyDataError::Collection(ResourceTextListError::Modified)) => {
            json!({"error":"InvalidOperationException"})
        }
        Err(e) => panic!("unexpected {e:?}"),
    }
}
#[test]
fn native_deferred_keys_preserve_source_values_errors_and_mutation_order() {
    let expected = rows();
    let mut retained = 0;
    for query in ["keys", "visible"] {
        for timing in ["before-first", "after-first", "after-end"] {
            for mutation in OPS {
                let key: Key = Rc::new(Probe {
                    name: "a",
                    enabled: Rc::new(Cell::new(false)),
                    fired: Rc::new(Cell::new(false)),
                    hide_on_mutate: false,
                    always_hidden: query == "visible",
                    mutation: "none",
                    owner: Rc::new(RefCell::new(None)),
                    trace: Rc::new(RefCell::new(Vec::new())),
                });
                let keyboard = build(vec![
                    vec![key, TextKeyData::char("b".into(), None).unwrap()],
                    vec![TextKeyData::char("c".into(), None).unwrap()],
                ]);
                keyboard
                    .compute(Rc::new(KeyComputeContext::default()))
                    .unwrap();
                let originals = keyboard
                    .keys()
                    .map(|key| key.unwrap().unwrap())
                    .collect::<Vec<_>>();
                let mut iterator = if query == "keys" {
                    keyboard.keys()
                } else {
                    keyboard.visible_keys()
                };
                if timing == "after-first" {
                    iterator.try_next().unwrap();
                }
                if timing == "after-end" {
                    while iterator.try_next().unwrap().is_some() {}
                }
                mutate(&keyboard.rows(), mutation);
                for step in 0..5 {
                    let row = expected
                        .iter()
                        .find(|r| {
                            r["kind"] == "enumerator"
                                && r["query"] == query
                                && r["timing"] == timing
                                && r["mutation"] == mutation
                                && r["step"] == step
                        })
                        .unwrap();
                    let item = iterator.try_next();
                    if let Ok(Some(value)) = &item {
                        assert_eq!(
                            json!({"value":originals.iter().position(|key|Rc::ptr_eq(key,value)).unwrap()}),
                            row["current"]
                        );
                    }
                    assert_eq!(moved(item), row["moved"], "{row}");
                    retained += 1;
                }
            }
        }
    }
    // Reset, post-Dispose calls and cached Current are intentionally retired.
    assert_eq!(retained, 240);
}

#[test]
fn native_query_results_fresh_iteration_and_drop_keep_failures_explicit() {
    let keyboard = build(vec![vec![
        TextKeyData::char("a".into(), None).unwrap(),
        TextKeyData::char("b".into(), None).unwrap(),
    ]]);
    keyboard
        .compute(Rc::new(KeyComputeContext::default()))
        .unwrap();
    let query = keyboard.keys();
    let mut first = query.iter();
    let mut second = query.iter();
    let a = first.try_next().unwrap().unwrap();
    assert!(Rc::ptr_eq(&a, &second.try_next().unwrap().unwrap()));
    let weak = Rc::downgrade(&a);
    keyboard.rows().get(0).unwrap().clear();
    assert!(matches!(
        first.next(),
        Some(Err(KeyDataError::Collection(
            ResourceTextListError::Modified
        )))
    ));
    assert!(matches!(
        second.try_next(),
        Err(KeyDataError::Collection(ResourceTextListError::Modified))
    ));
    assert!(query.iter().try_next().unwrap().is_none());
    drop(a);
    drop(first);
    drop(second);
    drop(query);
    drop(keyboard);
    assert!(weak.upgrade().is_none());
}

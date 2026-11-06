//! Original CurrencySet IReadOnlyList identity, nullable values, callbacks and compute failures.
use ferroboard_core::{keys::*, layouts::*};
use serde_json::{Value, json};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
};
type Result<T> = std::result::Result<T, ResourceTextListError>;
fn key(s: &str) -> TextKey {
    TextKeyData::char(s.into(), None).unwrap()
}
fn text(k: TextKey) -> String {
    k.as_string(false).unwrap().unwrap().to_utf8().unwrap()
}
fn observe<T: serde::Serialize>(r: Result<T>) -> Value {
    match r {
        Ok(v) => json!({"value":v}),
        Err(e) => json!({"error":match e {
                            ResourceTextListError::IndexOutOfRange=>"ArgumentOutOfRangeException",
                            ResourceTextListError::NotSupported=>"NotSupportedException",
                ResourceTextListError::NullReference=>"NullReferenceException",
                ResourceTextListError::ArgumentNull=>"ArgumentNullException",
        ResourceTextListError::KeyNotFound=>"KeyNotFoundException",
        ResourceTextListError::DuplicateKey=>"ArgumentException",
                            ResourceTextListError::Modified=>"InvalidOperationException",
                        }}),
    }
}
fn collection_error(e: KeyDataError) -> ResourceTextListError {
    if let KeyDataError::Collection(e) = e {
        e
    } else {
        panic!("unexpected {e:?}")
    }
}
fn lookup(set: &CurrencySet, code: i32) -> Result<Option<TextKey>> {
    set.get_slot(code).map_err(collection_error)
}
fn rows() -> Vec<Value> {
    include_str!("../../../tests/fixtures/currency-slots.jsonl")
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect()
}
const CODES: [i32; 8] = [-801, -802, -800, 0, i32::MIN, i32::MAX, -1000, -803];
const MODES: [&str; 10] = [
    "none",
    "count-clear",
    "count-add",
    "count-negative",
    "count-throw",
    "index-clear",
    "index-replace",
    "index-null",
    "index-throw",
    "count-max",
];
fn texts(values: &ResourceObjectList<Option<TextKey>>) -> Vec<Option<String>> {
    values.snapshot().into_iter().map(|k| k.map(text)).collect()
}
struct Probe {
    values: ResourceObjectList<Option<TextKey>>,
    trace: Rc<RefCell<Vec<String>>>,
    mode: &'static str,
    enabled: Cell<bool>,
}
impl Probe {
    fn new(mode: &'static str) -> Rc<Self> {
        Rc::new(Self {
            values: ResourceObjectList::new(vec![Some(key("x")), Some(key("y"))]),
            trace: Rc::default(),
            mode,
            enabled: Cell::new(false),
        })
    }
}
impl IResourceReadOnlyList<TextKey> for Probe {
    fn count(&self) -> Result<i32> {
        self.trace.borrow_mut().push("count".into());
        let count = self.values.len() as i32;
        if self.enabled.get() {
            match self.mode {
                "count-clear" => self.values.clear(),
                "count-add" => self.values.push(Some(key("z"))),
                "count-negative" => return Ok(-1),
                "count-throw" => return Err(ResourceTextListError::IndexOutOfRange),
                "count-max" => return Ok(i32::MAX),
                _ => {}
            }
        }
        Ok(count)
    }
    fn get(&self, index: i32) -> Result<Option<TextKey>> {
        self.trace.borrow_mut().push(format!("get:{index}"));
        if self.enabled.get() {
            match self.mode {
                "index-clear" => self.values.clear(),
                "index-replace" => self.values.set(0, Some(key("z")))?,
                "index-null" => return Ok(None),
                "index-throw" => return Err(ResourceTextListError::IndexOutOfRange),
                _ => {}
            }
        }
        self.values
            .get(usize::try_from(index).map_err(|_| ResourceTextListError::IndexOutOfRange)?)
    }
    fn iter(&self) -> Result<ResourceItems<TextKey>> {
        ResourceReadOnlyList::from_nullable(self.values.clone()).iter()
    }
}
#[test]
fn original_custom_currency_count_index_short_circuit_null_failures_and_order() {
    let expected = rows();
    let mut n = 44;
    for mode in MODES {
        for code in CODES {
            let probe = Probe::new(mode);
            probe.enabled.set(true);
            let slots = ResourceReadOnlyList::from_custom(probe.clone());
            let set = CurrencySet::new("probe".into(), slots);
            let result = observe(lookup(&set, code).map(|k| k.map(text)));
            let actual = json!({"kind":"custom","mode":mode,"code":code,"retained":set.slots().unwrap().ptr_eq(&ResourceReadOnlyList::from_custom(probe.clone())),"result":result,"trace":*probe.trace.borrow(),"text":texts(&probe.values)});
            assert_eq!(actual, expected[n], "{mode}/{code}");
            n += 1;
        }
    }
    assert_eq!(n, 124);
}
#[test]
fn original_custom_currency_compute_partial_state_context_version_and_recovery() {
    let expected = rows();
    let mut n = 124;
    for mode in MODES {
        let probe = Probe::new(mode);
        let set = Rc::new(CurrencySet::new(
            "probe".into(),
            ResourceReadOnlyList::from_custom(probe.clone()),
        ));
        let resources = Rc::new(KeyboardResources::create_empty());
        resources.add_layout(Rc::new(KeyboardLayout::new(
            LayoutType::Characters,
            "probe".into(),
            "main".into(),
            Rc::new(LayoutArrangement::from_rows(vec![vec![
                Rc::new(TextKeyData::from_fields(TextKeyDataFields {
                    code: KeyCode::CurrencySlot1,
                    label: Some("slot".into()),
                    ..Default::default()
                })) as Key,
                key("b") as Key,
            ]])),
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
        let keyboard = LayoutComputer::new(resources)
            .compute(KeyboardMode::Characters, subtype, false)
            .unwrap();
        let k = keyboard
            .keys()
            .map(|key| key.unwrap().unwrap())
            .next()
            .unwrap();
        for phase in ["warm", "mutate", "recover"] {
            probe.enabled.set(phase == "mutate");
            probe.trace.borrow_mut().clear();
            let ctx = Rc::new(KeyComputeContext::from_values(
                ferroboard_core::layouts::KeyComputeContextValues {
                    currency_set: Some(set.clone()),
                    show_number_hints: false,
                    show_symbol_hints: false,
                    ..Default::default()
                },
            ));
            let error = keyboard.compute(ctx.clone()).err().map(|e| {
                assert_eq!(collection_error(e), ResourceTextListError::IndexOutOfRange);
                "ArgumentOutOfRangeException"
            });
            let actual = json!({"kind":"compute","mode":mode,"phase":phase,"error":error,"trace":*probe.trace.borrow(),"contextSet":Rc::ptr_eq(&ctx,&keyboard.context().unwrap()),"version":keyboard.version(),"code":k.code(),"label":k.label().map(|s|s.to_utf8().unwrap()),"output":k.output().unwrap().unwrap().to_utf8().unwrap(),"text":texts(&probe.values)});
            assert_eq!(actual, expected[n], "{mode}/{phase}");
            n += 1;
        }
    }
    assert_eq!(n, 154);
}

#[test]
fn native_resource_buffers_retain_currency_values_from_source_fixtures() {
    let expected = rows();
    let mut compared = 0;
    for storage in ["empty-list", "list", "nullable-list"] {
        for mutation in ["none", "add", "clear", "set-same", "replace", "remove-last"] {
            let x = key("x");
            let values = ResourceObjectList::new(if storage == "empty-list" {
                vec![]
            } else {
                vec![
                    Some(x.clone()),
                    if storage == "nullable-list" {
                        None
                    } else {
                        Some(key("y"))
                    },
                ]
            });
            let slots = ResourceReadOnlyList::from_nullable(values.clone());
            let set = CurrencySet::new("test".into(), slots.clone());
            let edit: Result<()> = match mutation {
                "none" => Ok(()),
                "add" => {
                    values.push(Some(key("z")));
                    Ok(())
                }
                "clear" => {
                    values.clear();
                    Ok(())
                }
                "set-same" => values.get(0).and_then(|v| values.set(0, v)),
                "replace" => values.set(0, Some(key("z"))),
                "remove-last" => values
                    .len()
                    .checked_sub(1)
                    .ok_or(ResourceTextListError::IndexOutOfRange)
                    .and_then(|i| values.remove(i).map(|_| ())),
                _ => unreachable!(),
            };
            let actual = json!({"id":set.id().unwrap().to_utf8().unwrap(),"extensionId":set.extension_id().unwrap().to_utf8().unwrap(),"label":set.label().unwrap().to_utf8().unwrap(),"retained":slots.ptr_eq(&set.slots().unwrap()),"count":slots.count().unwrap(),"text":texts(&values),"lookup":CODES.map(|code|json!({"code":code,"result":observe(lookup(&set,code).map(|k|k.map(text))),"sameFirst":lookup(&set,code).unwrap().is_some_and(|k|Rc::ptr_eq(&x,&k))})),"edit":observe(edit)});
            let row = expected
                .iter()
                .find(|row| {
                    row["kind"] == "collection"
                        && row["storage"] == storage
                        && row["mutation"] == mutation
                })
                .unwrap();
            for (field, value) in actual.as_object().unwrap() {
                assert_eq!(value, &row[field], "{storage}/{mutation}/{field}");
            }
            compared += 1;
        }
    }
    assert_eq!(compared, 18);
}

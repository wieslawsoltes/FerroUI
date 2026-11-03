//! Original dictionary interfaces, mutable cursor contracts, nested lookup and engine precedence.
use ferroboard_core::{keys::*, layouts::*, text::Utf16Text};
use serde_json::{Value, json};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
};
type Result<T> = std::result::Result<T, ResourceTextListError>;
fn name(e: ResourceTextListError) -> &'static str {
    match e {
        ResourceTextListError::IndexOutOfRange => "ArgumentOutOfRangeException",
        ResourceTextListError::NotSupported => "NotSupportedException",
        ResourceTextListError::Modified => "InvalidOperationException",
        ResourceTextListError::ArgumentNull => "ArgumentNullException",
        ResourceTextListError::NullReference => "NullReferenceException",
        ResourceTextListError::KeyNotFound => "KeyNotFoundException",
        ResourceTextListError::DuplicateKey => "ArgumentException",
    }
}
fn observe<T: serde::Serialize>(v: Result<T>) -> Value {
    match v {
        Ok(v) => json!({"value":v}),
        Err(e) => json!({"error":name(e)}),
    }
}
fn text(s: Utf16Text) -> String {
    s.to_utf8().unwrap()
}
fn pair(p: ResourceDictionaryEntry<Utf16Text, Utf16Text>) -> Value {
    json!({"key":p.key.map(text),"value":p.value.map(text)})
}
fn entries(d: &ResourceDictionary<Utf16Text, Utf16Text>) -> Vec<Value> {
    d.iter()
        .unwrap()
        .map(|p| pair(p.unwrap().unwrap()))
        .collect()
}
fn rows() -> Vec<Value> {
    include_str!("../../../tests/fixtures/popup-dictionary.jsonl")
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect()
}
trait TraceKey: ResourceDictionaryKey {
    fn trace(&self) -> String;
}
impl TraceKey for Utf16Text {
    fn trace(&self) -> String {
        text(self.clone())
    }
}
impl TraceKey for KeyVariation {
    fn trace(&self) -> String {
        self.0.to_string()
    }
}
type Lookup<K, V> = Rc<dyn Fn(Option<&K>) -> Result<(bool, Option<V>)>>;
struct Probe<K: TraceKey, V: Clone + 'static> {
    id: &'static str,
    data: ResourceDictionary<K, V>,
    lookup: Lookup<K, V>,
    trace: Rc<RefCell<Vec<String>>>,
}
impl<K: TraceKey, V: Clone + 'static> Probe<K, V> {
    fn wrap(
        id: &'static str,
        data: ResourceDictionary<K, V>,
        lookup: impl Fn(Option<&K>) -> Result<(bool, Option<V>)> + 'static,
        trace: Rc<RefCell<Vec<String>>>,
    ) -> ResourceReadOnlyDictionary<K, V> {
        ResourceReadOnlyDictionary::from_custom(Rc::new(Self {
            id,
            data,
            lookup: Rc::new(lookup),
            trace,
        }))
    }
    fn step(&self, member: &str, key: Option<Option<&K>>) {
        self.trace.borrow_mut().push(format!(
            "{}.{}{}",
            self.id,
            member,
            key.map_or_else(String::new, |k| format!(
                ":{}",
                k.map_or_else(|| "<null>".into(), TraceKey::trace)
            ))
        ));
    }
}
impl<K: TraceKey, V: Clone + 'static> IResourceReadOnlyDictionary<K, V> for Probe<K, V> {
    fn count(&self) -> Result<i32> {
        self.step("count", None);
        self.data.count()
    }
    fn get(&self, key: Option<&K>) -> Result<Option<V>> {
        self.step("get", Some(key));
        self.data.get(key)
    }
    fn contains_key(&self, key: Option<&K>) -> Result<bool> {
        self.step("contains", Some(key));
        self.data.contains_key(key)
    }
    fn try_get_value(&self, key: Option<&K>) -> Result<(bool, Option<V>)> {
        self.step("try", Some(key));
        (self.lookup)(key)
    }
    fn keys(&self) -> Result<Option<ResourceEnumerable<K>>> {
        self.step("keys", None);
        Ok(Some(self.data.keys()))
    }
    fn values(&self) -> Result<Option<ResourceEnumerable<V>>> {
        self.step("values", None);
        Ok(Some(self.data.values()))
    }
    fn iter(&self) -> Result<ResourceItems<ResourceDictionaryEntry<K, V>>> {
        self.step("enum", None);
        self.data.iter()
    }
}
const BEHAVIORS: [&str; 6] = ["hit", "miss", "miss-out", "null-hit", "throw", "mutate-hit"];
fn popup(label: &str) -> Rc<PopupSet> {
    Rc::new(PopupSet::from_parts(
        Some(TextKeyData::char(label.into(), None).unwrap()),
        ResourceReadOnlyList::<Key>::default(),
    ))
}
fn key_error(e: KeyDataError) -> &'static str {
    if let KeyDataError::Collection(e) = e {
        name(e)
    } else {
        panic!("unexpected {e:?}")
    }
}
#[test]
fn original_popup_get_custom_true_false_out_null_labels_and_null_maps() {
    let expected = rows();
    let mut n = 337;
    for outer_mode in
        BEHAVIORS
            .into_iter()
            .chain(["null-outer", "empty-outer", "null-map", "null-set"])
    {
        let inner_modes = if outer_mode.starts_with("null-") && outer_mode != "null-hit"
            || outer_mode == "empty-outer"
        {
            &BEHAVIORS[..1]
        } else {
            &BEHAVIORS[..]
        };
        for &inner_mode in inner_modes {
            for label in [Some("a"), Some(""), None] {
                for variation in [KeyVariation::Normal, KeyVariation::All] {
                    let trace = Rc::new(RefCell::new(vec![]));
                    let set = popup("p");
                    let data = ResourceDictionary::default();
                    data.set("a".into(), Some(set.clone()));
                    let (values, p) = (data.clone(), set.clone());
                    let inner = Probe::wrap(
                        "i",
                        data.clone(),
                        move |_| {
                            if inner_mode == "throw" {
                                return Err(ResourceTextListError::IndexOutOfRange);
                            }
                            if inner_mode == "mutate-hit" {
                                values.clear();
                            }
                            Ok(match inner_mode {
                                "miss" => (false, None),
                                "miss-out" => (false, Some(p.clone())),
                                "null-hit" => (true, None),
                                _ => (true, Some(p.clone())),
                            })
                        },
                        trace.clone(),
                    );
                    let raw = ResourceDictionary::default();
                    raw.set(KeyVariation::Normal, Some(inner.clone()));
                    let (values, inner_value) = (data.clone(), inner.clone());
                    let outer = Probe::wrap(
                        "o",
                        raw,
                        move |_| {
                            if outer_mode == "throw" {
                                return Err(ResourceTextListError::IndexOutOfRange);
                            }
                            if outer_mode == "mutate-hit" {
                                values.clear();
                            }
                            Ok(match outer_mode {
                                "miss" => (false, None),
                                "miss-out" => (false, Some(inner_value.clone())),
                                "null-hit" => (true, None),
                                _ => (true, Some(inner_value.clone())),
                            })
                        },
                        trace.clone(),
                    );
                    let mappings = match outer_mode {
                        "null-outer" => None,
                        "empty-outer" => Some(PopupVariationMappings::default()),
                        "null-map" => {
                            let d = ResourceDictionary::default();
                            d.set(KeyVariation::Normal, None);
                            Some(d.into())
                        }
                        "null-set" => {
                            let i = ResourceDictionary::default();
                            i.set("a".into(), None);
                            let o = ResourceDictionary::default();
                            o.set(KeyVariation::Normal, Some(i.into()));
                            Some(o.into())
                        }
                        _ => Some(outer),
                    };
                    let map = PopupMapping::from_values(PopupMappingValues {
                        mappings,
                        ..Default::default()
                    });
                    let label_value = label.map(Utf16Text::from);
                    let result = map.get_nullable(variation, label_value.as_ref());
                    let failure = result.as_ref().err().cloned().map(key_error);
                    let result = result.ok().flatten();
                    let actual = json!({"kind":"lookup","outerMode":outer_mode,"innerMode":inner_mode,"label":label,"variation":(variation).0,"error":failure,"trace":*trace.borrow(),"isNull":result.is_none(),"same":result.is_some_and(|r|Rc::ptr_eq(&r,&set)),"remaining":data.count().unwrap()});
                    assert_eq!(
                        actual, expected[n],
                        "{outer_mode}/{inner_mode}/{label:?}/{variation:?}"
                    );
                    n += 1;
                }
            }
        }
    }
    assert_eq!(n, 577);
}
fn behavior(
    mode: &str,
    key: Option<&KeyVariation>,
    inner: &PopupLabelMappings,
) -> Result<(bool, Option<PopupLabelMappings>)> {
    Ok(match mode {
        "throw" => return Err(ResourceTextListError::IndexOutOfRange),
        "null-hit" => (true, None),
        "miss" => (false, None),
        "miss-out" => (false, Some(inner.clone())),
        "hit-uri" => (key == Some(&KeyVariation::Uri), Some(inner.clone())),
        "hit-all" => (key == Some(&KeyVariation::All), Some(inner.clone())),
        _ => (true, Some(inner.clone())),
    })
}
#[test]
fn original_computed_popup_lookup_fallback_repeated_uri_alias_precedence_errors_and_recovery() {
    let expected = rows();
    let mut n = 577;
    for primary in [
        "hit",
        "miss",
        "miss-out",
        "hit-uri",
        "hit-all",
        "null-hit",
        "throw",
        "inner-throw",
    ] {
        for fallback in ["miss", "hit", "hit-uri", "hit-all"] {
            for variation in [
                KeyVariation::All,
                KeyVariation::Normal,
                KeyVariation::EmailAddress,
                KeyVariation::Uri,
                KeyVariation::Password,
            ] {
                for grouped in [false, true] {
                    let trace = Rc::new(RefCell::new(vec![]));
                    let enabled = Rc::new(Cell::new(false));
                    let p = popup("p");
                    let d = popup("d");
                    let flag = enabled.clone();
                    let pi = Probe::wrap(
                        "p.i",
                        ResourceDictionary::default(),
                        move |_| {
                            if flag.get() && primary == "inner-throw" {
                                Err(ResourceTextListError::IndexOutOfRange)
                            } else {
                                Ok((true, Some(p.clone())))
                            }
                        },
                        trace.clone(),
                    );
                    let di = Probe::wrap(
                        "d.i",
                        ResourceDictionary::default(),
                        move |_| Ok((true, Some(d.clone()))),
                        trace.clone(),
                    );
                    let flag = enabled.clone();
                    let po = Probe::wrap(
                        "p.o",
                        ResourceDictionary::default(),
                        move |key| behavior(if flag.get() { primary } else { "hit" }, key, &pi),
                        trace.clone(),
                    );
                    let ds = Probe::wrap(
                        "d.o",
                        ResourceDictionary::default(),
                        move |key| behavior(fallback, key, &di),
                        trace.clone(),
                    );
                    let resources = Rc::new(KeyboardResources::create_empty());
                    resources.add_popup_mapping(Rc::new(PopupMapping::from_values(
                        PopupMappingValues {
                            id: Some("local".into()),
                            extension_id: Some("probe".into()),
                            mappings: Some(po),
                            ..Default::default()
                        },
                    )));
                    resources.add_popup_mapping(Rc::new(PopupMapping::from_values(
                        PopupMappingValues {
                            id: Some("default".into()),
                            extension_id: Some(Subtype::CORE_LOCALIZATION.into()),
                            mappings: Some(ds),
                            ..Default::default()
                        },
                    )));
                    resources.add_layout(Rc::new(KeyboardLayout::new(
                        LayoutType::Numeric,
                        "probe".into(),
                        "main".into(),
                        Rc::new(LayoutArrangement::from_rows(vec![vec![Rc::new(
                            TextKeyData::from_fields(TextKeyDataFields {
                                code: 97,
                                label: Some("a".into()),
                                group_id: if grouped { TextKeyData::GroupLeft } else { 0 },
                                ..Default::default()
                            }),
                        )
                            as Key]])),
                    )));
                    let subtype = Arc::new(Subtype::from_values(
                        ferroboard_core::layouts::SubtypeValues {
                            popup_mapping: ComponentName::new("probe", "local"),
                            layouts: Some(Arc::new(SubtypeLayoutMap::from_values(
                                ferroboard_core::layouts::SubtypeLayoutMapValues {
                                    numeric: ComponentName::new("probe", "main"),
                                    ..(*Subtype::shared_default().layouts().unwrap()).values()
                                },
                            ))),
                            ..(*Subtype::shared_default()).values()
                        },
                    ));
                    let keyboard = LayoutComputer::new(resources)
                        .compute(KeyboardMode::Numeric, subtype, false)
                        .unwrap();
                    let key = keyboard
                        .keys()
                        .map(|key| key.unwrap().unwrap())
                        .next()
                        .unwrap();
                    for phase in ["warm", "mutate", "recover"] {
                        enabled.set(phase == "mutate");
                        trace.borrow_mut().clear();
                        let previous = key.popup_keys();
                        let ctx = Rc::new(KeyComputeContext::from_values(
                            ferroboard_core::layouts::KeyComputeContextValues {
                                mode: KeyboardMode::Numeric,
                                variation,
                                show_number_hints: false,
                                show_symbol_hints: false,
                                ..Default::default()
                            },
                        ));
                        let failure = keyboard.compute(ctx.clone()).err().map(key_error);
                        let actual = json!({"kind":"compute","primary":primary,"fallback":fallback,"variation":(variation).0,"grouped":grouped,"phase":phase,"error":failure,"trace":*trace.borrow(),"version":keyboard.version(),"contextSet":Rc::ptr_eq(&ctx,&keyboard.context().unwrap()),"popupSame":previous.ptr_eq(&key.popup_keys()),"popups":key.popup_keys().snapshot().unwrap().into_iter().map(|p|text(p.as_string(false).unwrap().unwrap())).collect::<Vec<_>>(),"defaultIndex":key.popup_default_index(),"code":key.code(),"label":key.label().map(text)});
                        assert_eq!(
                            actual, expected[n],
                            "{primary}/{fallback}/{variation:?}/{grouped}/{phase}"
                        );
                        n += 1;
                    }
                }
            }
        }
    }
    assert_eq!(n, 1537);
}
#[test]
fn original_popup_defaults_ordinal_duplicates_aliases_and_dictionary_public_dispatch() {
    let expected = rows();
    let a = PopupMapping::default();
    let b = PopupMapping::default();
    let parsed=LayoutJson::read_popup_mapping(r#"{"normal":{"x":["a"],"X":["b"],"x":["c"]},"email":{"a":["e"]},"emailAddress":{"b":["f"]}}"#,"id".into(),"ext".into(),None).unwrap();
    let actual = json!({"kind":"popup-shape","Id":text(a.id().unwrap()),"ExtensionId":text(a.extension_id().unwrap()),"mapsShared":a.mappings().as_ref().unwrap().ptr_eq(b.mappings().as_ref().unwrap()),"count":a.mappings().as_ref().unwrap().count().unwrap(),"mutable":a.mappings().as_ref().unwrap().as_concrete().is_some(),"parsed":parsed.mappings().as_ref().unwrap().iter().unwrap().map(|p|{let p=p.unwrap().unwrap();json!({"variation":(p.key.unwrap()).0,"labels":p.value.unwrap().iter().unwrap().map(|p|{let p=p.unwrap().unwrap();json!({"key":text(p.key.unwrap()),"popups":p.value.unwrap().relevant().unwrap().as_concrete().unwrap().snapshot().into_iter().map(|k|text(k.as_string(false).unwrap().unwrap())).collect::<Vec<_>>()})}).collect::<Vec<_>>()})}).collect::<Vec<_>>()});
    let mut native_expected = expected[1537].clone();
    native_expected
        .as_object_mut()
        .unwrap()
        .remove("authorsShared");
    assert!(!a.authors().unwrap().ptr_eq(&b.authors().unwrap()));
    assert_eq!(actual, native_expected);
    let mut n = 1538;
    for custom in [false, true] {
        for operation in [
            "count",
            "get-a",
            "get-b",
            "get-missing",
            "get-null",
            "contains-a",
            "contains-missing",
            "contains-null",
            "try-a",
            "try-b",
            "try-missing",
            "try-null",
            "keys",
            "values",
            "entries",
            "duplicate",
            "set-existing",
            "remove-missing",
            "remove-a",
            "clear",
        ] {
            let trace = Rc::new(RefCell::new(vec![]));
            let data = ResourceDictionary::default();
            data.set("a".into(), Some("x".into()));
            data.set("b".into(), None);
            let map = if custom {
                Probe::wrap(
                    "c",
                    data.clone(),
                    |_| Ok((true, Some("q".into()))),
                    trace.clone(),
                )
            } else {
                data.clone().into()
            };
            let key = if operation.ends_with("-a") {
                Some(Utf16Text::from("a"))
            } else if operation.ends_with("-b") {
                Some("b".into())
            } else if operation.ends_with("-missing") {
                Some("z".into())
            } else {
                None
            };
            let result = observe(match operation {
                "count" => map.count().map(|v| json!(v)),
                "get-a" | "get-b" | "get-missing" | "get-null" => {
                    map.get(key.as_ref()).map(|v| json!(v.map(text)))
                }
                "contains-a" | "contains-missing" | "contains-null" => {
                    map.contains_key(key.as_ref()).map(|v| json!(v))
                }
                "try-a" | "try-b" | "try-missing" | "try-null" => map
                    .try_get_value(key.as_ref())
                    .map(|(found, value)| json!({"found":found,"value":value.map(text)})),
                "keys" => map.keys().map(|v| {
                    json!(
                        v.unwrap()
                            .iter()
                            .unwrap()
                            .map(|p| p.unwrap().map(text))
                            .collect::<Vec<_>>()
                    )
                }),
                "values" => map.values().map(|v| {
                    json!(
                        v.unwrap()
                            .iter()
                            .unwrap()
                            .map(|p| p.unwrap().map(text))
                            .collect::<Vec<_>>()
                    )
                }),
                "entries" => map
                    .iter()
                    .map(|v| json!(v.map(|p| pair(p.unwrap().unwrap())).collect::<Vec<_>>())),
                "duplicate" => data.add("a".into(), Some("z".into())).map(|()| Value::Null),
                "set-existing" => {
                    data.set("a".into(), Some("z".into()));
                    Ok(Value::Null)
                }
                "remove-missing" => data.remove(Some(&"z".into())).map(|v| json!(v)),
                "remove-a" => data.remove(Some(&"a".into())).map(|v| json!(v)),
                "clear" => {
                    data.clear();
                    Ok(Value::Null)
                }
                _ => unreachable!(),
            });
            let actual = json!({"kind":"api","custom":custom,"operation":operation,"result":result,"trace":*trace.borrow(),"text":entries(&data)});
            assert_eq!(actual, expected[n], "{custom}/{operation}");
            n += 1;
        }
    }
    assert_eq!(n, 1578);
    assert_eq!(expected.len(), 1579);
}

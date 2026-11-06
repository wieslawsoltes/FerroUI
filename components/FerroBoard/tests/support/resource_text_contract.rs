//! Source oracle routing only. Core-only types; native consumers supply real metadata operations.
use ferroboard_core::{
    engine::{LanguageModelError, LanguageModels},
    layouts::{ComponentName, KeyboardResources, PunctuationRule, ResourceError},
    text::{Utf16Text, WordDictionary},
};
use serde_json::{Value, json};
use std::{
    cell::{Cell, RefCell},
    panic::{AssertUnwindSafe, catch_unwind, resume_unwind},
    rc::Rc,
    sync::Arc,
};
pub const NAMES: [&str; 8] = [
    "ExtensionId",
    "Id",
    "Label",
    "SymbolsPrecedingAutoSpace",
    "SymbolsFollowingAutoSpace",
    "SymbolsPrecedingPhantomSpace",
    "SymbolsFollowingPhantomSpace",
    "SymbolsTerminatingSentence",
];
pub type Hook = Rc<dyn Fn()>;
pub type Cleanup = Box<dyn FnOnce()>;
#[derive(Debug)]
pub struct HandlerFailure;
pub trait Driver {
    type Snapshot;
    fn read(r: &Arc<PunctuationRule>, field: usize) -> Option<Utf16Text>;
    fn write(r: &Arc<PunctuationRule>, field: usize, value: Option<Utf16Text>);
    fn add(
        r: &Rc<KeyboardResources>,
        rule: Option<Arc<PunctuationRule>>,
    ) -> Result<(), &'static str>;
    fn get(r: &Rc<KeyboardResources>, name: ComponentName) -> Arc<PunctuationRule>;
    fn subscribe(r: &Rc<KeyboardResources>, hook: Hook) -> Cleanup;
    fn has(m: &Arc<LanguageModels>, query: Option<&Utf16Text>) -> Result<bool, &'static str>;
    fn loaded(m: &Arc<LanguageModels>, query: Option<&Utf16Text>) -> Result<bool, &'static str>;
    fn dictionary(
        m: &Arc<LanguageModels>,
        query: Option<&Utf16Text>,
    ) -> Result<Option<Arc<WordDictionary>>, &'static str>;
    fn languages(m: &Arc<LanguageModels>) -> Self::Snapshot;
    fn values(snapshot: &Self::Snapshot) -> Vec<Utf16Text>;
    fn same(a: &Self::Snapshot, b: &Self::Snapshot) -> bool;
}
pub struct Core;
fn language_error(e: Arc<LanguageModelError>) -> &'static str {
    match &*e {
        LanguageModelError::NullReference => "NullReferenceException",
        other => panic!("unexpected language failure: {other:?}"),
    }
}
impl Driver for Core {
    type Snapshot = Vec<Utf16Text>;
    fn read(r: &Arc<PunctuationRule>, field: usize) -> Option<Utf16Text> {
        match field {
            0 => r.extension_id(),
            1 => r.id(),
            2 => r.label(),
            3 => r.symbols_preceding_auto_space(),
            4 => r.symbols_following_auto_space(),
            5 => r.symbols_preceding_phantom_space(),
            6 => r.symbols_following_phantom_space(),
            7 => r.symbols_terminating_sentence(),
            _ => unreachable!(),
        }
    }
    fn write(r: &Arc<PunctuationRule>, field: usize, value: Option<Utf16Text>) {
        match field {
            0 => r.set_extension_id(value),
            1 => r.set_id(value),
            2 => r.set_label(value),
            3 => r.set_symbols_preceding_auto_space(value),
            4 => r.set_symbols_following_auto_space(value),
            5 => r.set_symbols_preceding_phantom_space(value),
            6 => r.set_symbols_following_phantom_space(value),
            7 => r.set_symbols_terminating_sentence(value),
            _ => unreachable!(),
        }
    }
    fn add(
        r: &Rc<KeyboardResources>,
        rule: Option<Arc<PunctuationRule>>,
    ) -> Result<(), &'static str> {
        r.add_punctuation_rule_nullable(rule).map_err(|e| match e {
            ResourceError::NullReference => "NullReferenceException",
            other => panic!("unexpected resource failure: {other:?}"),
        })
    }
    fn get(r: &Rc<KeyboardResources>, name: ComponentName) -> Arc<PunctuationRule> {
        r.get_punctuation_rule(&name)
    }
    fn subscribe(r: &Rc<KeyboardResources>, hook: Hook) -> Cleanup {
        let token = r.changed().add(Rc::new(move |_| hook()));
        let weak = Rc::downgrade(r);
        Box::new(move || {
            if let Some(r) = weak.upgrade() {
                assert!(r.changed().remove(token));
            }
        })
    }
    fn has(m: &Arc<LanguageModels>, q: Option<&Utf16Text>) -> Result<bool, &'static str> {
        m.has_dictionary_nullable(q).map_err(language_error)
    }
    fn loaded(m: &Arc<LanguageModels>, q: Option<&Utf16Text>) -> Result<bool, &'static str> {
        m.is_loaded_nullable(q).map_err(language_error)
    }
    fn dictionary(
        m: &Arc<LanguageModels>,
        q: Option<&Utf16Text>,
    ) -> Result<Option<Arc<WordDictionary>>, &'static str> {
        m.get_dictionary_nullable(q).map_err(language_error)
    }
    fn languages(m: &Arc<LanguageModels>) -> Self::Snapshot {
        m.languages()
    }
    fn values(s: &Self::Snapshot) -> Vec<Utf16Text> {
        s.clone()
    }
    fn same(a: &Self::Snapshot, b: &Self::Snapshot) -> bool {
        std::ptr::eq(a.as_ptr(), b.as_ptr())
    }
}
fn text(v: &Value) -> Option<Utf16Text> {
    v.as_array().map(|v| {
        Utf16Text::from_units(
            v.iter()
                .map(|n| u16::try_from(n.as_u64().unwrap()).unwrap())
                .collect::<Vec<_>>(),
        )
    })
}
fn state<D: Driver>(r: &Arc<PunctuationRule>) -> Value {
    json!(
        (0..8)
            .map(|i| D::read(r, i).map(|s| s.units().to_vec()))
            .collect::<Vec<_>>()
    )
}
fn values() -> [Option<Utf16Text>; 8] {
    [
        None,
        Some("".into()),
        Some("custom".into()),
        Some(" \t\r\n".into()),
        Some(".?!".into()),
        Some("。！？".into()),
        Some("😀".into()),
        Some(Utf16Text::from_units(vec![0xd800, 120, 0xdfff])),
    ]
}
fn key<D: Driver>(rule: &Arc<PunctuationRule>) -> ComponentName {
    ComponentName::new_nullable(D::read(rule, 0), D::read(rule, 1))
}
fn snapshot<D: Driver>(
    r: &Rc<KeyboardResources>,
    first: &Arc<PunctuationRule>,
    second: &Arc<PunctuationRule>,
) -> Value {
    let old = D::get(r, ComponentName::new("custom", "id"));
    json!({"state":state::<D>(first),"oldIsFirst":Arc::ptr_eq(first,&old),"oldIsSecond":Arc::ptr_eq(second,&old),"currentIsFirst":Arc::ptr_eq(first,&D::get(r,key::<D>(first)))})
}
fn bool_result(value: Result<bool, &'static str>) -> Value {
    match value {
        Ok(value) => json!({"value":value,"error":null}),
        Err(error) => json!({"value":null,"error":error}),
    }
}
pub fn models() -> Arc<LanguageModels> {
    let m = LanguageModels::new();
    let tags = [
        "xx-YY",
        "EN_GB",
        "placeholder",
        "İ-TR",
        "-x",
        "ß-de",
        "𐐀-X",
        "nil",
        " a\nb",
    ];
    for (i, tag) in tags.iter().enumerate() {
        let tag = if i == 2 {
            Utf16Text::from_units(vec![0xd800, 116, 97, 103, 95, 109, 111, 114, 101])
        } else {
            Utf16Text::from(*tag)
        };
        m.register_dictionary(
            &tag,
            if i == 7 {
                None
            } else {
                Some(Arc::new(WordDictionary::new(
                    format!("d{i}").as_str().into(),
                )))
            },
        );
    }
    m
}
pub fn verify<D: Driver>(corpus: &str) {
    let mut behavior = 0;
    let mut metadata = 0;
    let m = models();
    let before_snapshot = D::languages(&m);
    for line in corpus.lines() {
        let row: Value = serde_json::from_str(line).unwrap();
        match row["tag"].as_str().unwrap() {
            "metadata" => {
                assert!(NAMES.contains(&row["name"].as_str().unwrap()));
                metadata += 1;
                continue;
            }
            "default" => {
                let p = Arc::new(PunctuationRule::default());
                assert_eq!(state::<D>(&p), row["state"]);
                assert_eq!(
                    Arc::ptr_eq(
                        &PunctuationRule::shared_default(),
                        &PunctuationRule::shared_default()
                    ),
                    row["same"]
                );
                assert_eq!(
                    PunctuationRule::default() == PunctuationRule::default(),
                    row["equal"]
                );
            }
            "mutation" => {
                let p = Arc::new(PunctuationRule::default());
                D::write(&p, 0, Some("custom".into()));
                D::write(&p, 1, Some("id".into()));
                let alias = p.clone();
                let r = Rc::new(KeyboardResources::create_empty());
                let count = Rc::new(Cell::new(0));
                let counter = count.clone();
                let cleanup = D::subscribe(&r, Rc::new(move || counter.set(counter.get() + 1)));
                D::add(&r, Some(p.clone())).unwrap();
                let field = row["field"].as_u64().unwrap() as usize;
                let vi = row["vi"].as_u64().unwrap() as usize;
                D::write(&p, field, values()[vi].clone());
                let k = key::<D>(&p);
                let old_same = Arc::ptr_eq(&p, &D::get(&r, ComponentName::new("custom", "id")));
                let new_same = Arc::ptr_eq(&p, &D::get(&r, k.clone()));
                let before = count.get();
                D::add(&r, Some(p.clone())).unwrap();
                assert_eq!(
                    json!({"tag":"mutation","field":field,"vi":vi,"state":state::<D>(&p),"aliasSame":Arc::ptr_eq(&p,&alias),"oldSame":old_same,"newSame":new_same,"before":before,"after":count.get(),"oldAfterSame":Arc::ptr_eq(&p,&D::get(&r,ComponentName::new("custom", "id"))),"newAfterSame":Arc::ptr_eq(&p,&D::get(&r,k))}),
                    row
                );
                cleanup();
            }
            "null-rule" => {
                let r = Rc::new(KeyboardResources::create_empty());
                let count = Rc::new(Cell::new(0));
                let counter = count.clone();
                let cleanup = D::subscribe(&r, Rc::new(move || counter.set(counter.get() + 1)));
                let error = D::add(&r, None).unwrap_err();
                assert_eq!(
                    json!({"tag":"null-rule","error":error,"events":count.get(),"fallbackSame":Arc::ptr_eq(&PunctuationRule::shared_default(),&D::get(&r,ComponentName::new("missing", "missing")))}),
                    row
                );
                cleanup();
            }
            "event" => {
                let reaction = row["reaction"].as_str().unwrap().to_owned();
                let r = Rc::new(KeyboardResources::create_empty());
                let first = Arc::new(PunctuationRule::default());
                D::write(&first, 0, Some("custom".into()));
                D::write(&first, 1, Some("id".into()));
                let second = Arc::new(PunctuationRule::default());
                D::write(&second, 0, Some("custom".into()));
                D::write(&second, 1, Some("id".into()));
                D::write(&second, 2, Some(Utf16Text::from_units(vec![0xd800])));
                let log = Rc::new(RefCell::new(Vec::<Value>::new()));
                let once = Rc::new(Cell::new(true));
                let tail_id = Rc::new(Cell::new(0));
                let weak = Rc::downgrade(&r);
                let a = first.clone();
                let b = second.clone();
                let journal = log.clone();
                let tail = tail_id.clone();
                let mode = reaction.clone();
                let cleanup = D::subscribe(
                    &r,
                    Rc::new(move || {
                        let r = weak.upgrade().unwrap();
                        journal
                            .borrow_mut()
                            .push(json!({"kind":"head","state":snapshot::<D>(&r,&a,&b)}));
                        if once.replace(false) {
                            match mode.as_str() {
                                "nested" => D::add(&r, Some(b.clone())).unwrap(),
                                "throw" => std::panic::panic_any(HandlerFailure),
                                "remove-tail" => {
                                    assert!(r.changed().remove(tail.get()));
                                }
                                "mutate" => {
                                    D::write(&a, 0, None);
                                    D::write(&a, 2, Some(Utf16Text::from_units(vec![0xdfff])));
                                    D::add(&r, Some(a.clone())).unwrap();
                                }
                                _ => unreachable!(),
                            };
                            journal
                                .borrow_mut()
                                .push(json!({"kind":"after","state":snapshot::<D>(&r,&a,&b)}));
                        }
                    }),
                );
                let weak = Rc::downgrade(&r);
                let a = first.clone();
                let b = second.clone();
                let journal = log.clone();
                let token=r.changed().add(Rc::new(move|_|journal.borrow_mut().push(json!({"kind":"tail","state":snapshot::<D>(&weak.upgrade().unwrap(),&a,&b)}))));
                tail_id.set(token);
                let error = match catch_unwind(AssertUnwindSafe(|| D::add(&r, Some(first.clone()))))
                {
                    Ok(result) => result.err(),
                    Err(p) => {
                        if p.is::<HandlerFailure>() {
                            Some("ProbeFailure")
                        } else {
                            resume_unwind(p)
                        }
                    }
                };
                assert_eq!(
                    json!({"tag":"event","reaction":reaction,"error":error,"state":snapshot::<D>(&r,&first,&second),"log":*log.borrow()}),
                    row
                );
                cleanup();
                r.changed().remove(token);
            }
            "language" => {
                let query = text(&row["query"]);
                let has = bool_result(D::has(&m, query.as_ref()));
                let before = bool_result(D::loaded(&m, query.as_ref()));
                let (found, error) = match D::dictionary(&m, query.as_ref()) {
                    Ok(d) => {
                        if let Some(d) = &d {
                            assert!(Arc::ptr_eq(
                                d,
                                &m.get_dictionary_nullable(query.as_ref()).unwrap().unwrap()
                            ));
                        }
                        (
                            d.and_then(|d| d.language_tag())
                                .map(|s| s.to_utf8().expect("fixture labels are valid UTF8")),
                            None,
                        )
                    }
                    Err(e) => (None, Some(e)),
                };
                assert_eq!(
                    json!({"tag":"language","query":row["query"],"has":has,"before":before,"found":found,"error":error,"after":bool_result(D::loaded(&m,query.as_ref())),"languages":D::values(&D::languages(&m)).iter().map(Utf16Text::units).collect::<Vec<_>>()}),
                    row
                );
            }
            "language-snapshot" => {
                m.register_dictionary(&"late".into(), Some(Arc::new(WordDictionary::default())));
                let after = D::languages(&m);
                assert_eq!(
                    json!({"tag":"language-snapshot","same":D::same(&before_snapshot,&after),"before":D::values(&before_snapshot).iter().map(Utf16Text::units).collect::<Vec<_>>(),"after":D::values(&after).iter().map(Utf16Text::units).collect::<Vec<_>>()}),
                    row
                );
            }
            other => panic!("Unexpected source row: {other}"),
        }
        behavior += 1;
    }
    assert_eq!((behavior, metadata), (96, 8));
}

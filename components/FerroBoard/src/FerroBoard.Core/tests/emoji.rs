use ferroboard_core::{emoji::*, text::Utf16Text};
use serde::Serialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{cell::Cell, rc::Rc, sync::Arc};
fn rows() -> Vec<Value> {
    include_str!("../../../tests/fixtures/emoji.jsonl")
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}
fn text(v: &Value) -> Utf16Text {
    Utf16Text::from_units(
        v.as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap() as u16)
            .collect::<Vec<_>>(),
    )
}
struct Info {
    value: Vec<u16>,
    name: Vec<u16>,
    keywords: Vec<Vec<u16>>,
    category: i32,
    variants: Vec<Vec<u16>>,
}
impl Serialize for Info {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("Info", 5)?;
        s.serialize_field("value", &self.value)?;
        s.serialize_field("name", &self.name)?;
        s.serialize_field("keywords", &self.keywords)?;
        s.serialize_field("category", &self.category)?;
        s.serialize_field("variants", &self.variants)?;
        s.end()
    }
}
struct Behavior {
    lookup: Vec<Option<Info>>,
    tones: Vec<Vec<u16>>,
}
impl Serialize for Behavior {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("Behavior", 2)?;
        s.serialize_field("lookup", &self.lookup)?;
        s.serialize_field("tones", &self.tones)?;
        s.end()
    }
}
fn info(i: &EmojiInfo) -> Info {
    Info {
        value: i.value.units().to_vec(),
        name: i.name.units().to_vec(),
        keywords: i
            .keywords
            .lock()
            .clone()
            .iter()
            .map(|s| s.units().to_vec())
            .collect(),
        category: i.category.0,
        variants: i
            .variants()
            .lock()
            .clone()
            .iter()
            .map(|s| s.units().to_vec())
            .collect(),
    }
}
fn values(v: Vec<Arc<EmojiInfo>>) -> Value {
    json!(
        v.iter()
            .map(|v| v.value.units().to_vec())
            .collect::<Vec<_>>()
    )
}
fn error(e: EmojiError) -> &'static str {
    match e {
        EmojiError::InvalidScalar | EmojiError::Index => "ArgumentOutOfRangeException",
        EmojiError::Json(ferroboard_core::serialization::JsonBridgeError::Syntax { .. }) => {
            "JsonReaderException"
        }
        EmojiError::Json(ferroboard_core::serialization::JsonBridgeError::NumberFormat) => {
            "FormatException"
        }
        EmojiError::Json(_) => "InvalidOperationException",
        _ => panic!("unexpected {e:?}"),
    }
}
fn result<T>(r: EmojiResult<T>, f: impl FnOnce(T) -> Value) -> Value {
    match r {
        Ok(v) => json!({"value":f(v)}),
        Err(e) => json!({"error":error(e)}),
    }
}
#[test]
fn all_bundled_catalog_records_and_localized_search_match_source() {
    let mut count = 0;
    for row in rows() {
        match row["kind"].as_str().unwrap() {
            "catalog" => {
                let c = EmojiCatalog::load(&text(&row["lang"])).unwrap();
                assert_eq!(c.locale().to_utf8().unwrap(), row["locale"]);
                assert_eq!(
                    json!(c.categories().iter().map(|c| c.0).collect::<Vec<_>>()),
                    row["categories"]
                );
                let mut hash = Sha256::new();
                let mut n = 0;
                for i in c.all() {
                    hash.update(serde_json::to_vec(&info(&i)).unwrap());
                    hash.update(b"\n");
                    n += 1;
                }
                assert_eq!(json!(n), row["count"]);
                assert_eq!(
                    format!("{:x}", hash.finalize()),
                    row["sha256"].as_str().unwrap()
                );
                let mut behavior = Sha256::new();
                for entry in c.all() {
                    let variants = entry.variants().lock().clone();
                    let lookup = std::iter::once(entry.value.clone())
                        .chain(variants)
                        .map(|value| c.get(&value).map(|i| info(&i)))
                        .collect();
                    let tones = (0..6)
                        .map(|t| {
                            entry
                                .with_skin_tone(EmojiSkinTone(t))
                                .unwrap()
                                .units()
                                .to_vec()
                        })
                        .collect();
                    behavior.update(serde_json::to_vec(&Behavior { lookup, tones }).unwrap());
                    behavior.update(b"\n");
                }
                assert_eq!(
                    format!("{:x}", behavior.finalize()),
                    row["behavior"].as_str().unwrap()
                );
                assert!(Arc::ptr_eq(
                    &c,
                    &EmojiCatalog::load(&text(&row["lang"])).unwrap()
                ));
            }
            "search" => {
                let c = EmojiCatalog::load(&text(&row["lang"])).unwrap();
                assert_eq!(
                    values(c.search(&text(&row["query"]), row["max"].as_i64().unwrap() as i32)),
                    row["result"],
                    "{row}"
                );
                count += 1;
            }
            _ => {}
        }
    }
    assert!(count > 1000);
}
#[test]
fn source_custom_parser_variants_lookup_and_emoticons() {
    for row in rows() {
        match row["kind"].as_str().unwrap() {
            "parse" => {
                let c = EmojiCatalog::parse(&text(&row["input"]), "custom".into()).unwrap();
                assert_eq!(
                    json!(c.all().map(|i| info(&i)).collect::<Vec<_>>()),
                    row["all"]
                );
                assert_eq!(
                    json!(c.categories().iter().map(|c| c.0).collect::<Vec<_>>()),
                    row["categories"]
                );
                for get in row["gets"].as_array().unwrap() {
                    assert_eq!(
                        json!(c.get(&text(&get["value"])).map(|i| info(&i))),
                        get["info"]
                    );
                }
                for category in c.categories() {
                    assert!(Arc::ptr_eq(
                        &c.get_emojis(category),
                        &c.get_emojis(category)
                    ));
                }
            }
            "lookup" => {
                let c = EmojiCatalog::load(&"en".into()).unwrap();
                let i = c.get(&text(&row["value"]));
                assert_eq!(json!(i.as_ref().map(|i| info(i))), row["info"]);
                if let Some(i) = i {
                    assert!(Arc::ptr_eq(&i, &c.get(&i.value).unwrap()));
                    for tone in row["tones"].as_array().unwrap() {
                        assert_eq!(
                            result(
                                i.with_skin_tone(EmojiSkinTone(
                                    tone["tone"].as_i64().unwrap() as i32
                                )),
                                |s| json!(s.units())
                            ),
                            tone["result"]
                        );
                    }
                }
            }
            "emoticons" => {
                let v = EmojiCatalog::load_emoticons().unwrap();
                assert_eq!(json!(v.iter().map(|e|json!({"text":e.text.units(),"meanings":e.meanings.lock().clone().iter().map(|s|s.units().to_vec()).collect::<Vec<_>>()})).collect::<Vec<_>>()),row["all"]);
            }
            _ => {}
        }
    }
}
#[test]
fn history_source_operations_partial_failures_and_live_collection_identity() {
    let h = EmojiHistory::default();
    let events = Rc::new(Cell::new(0));
    let n = events.clone();
    h.changed().add(Rc::new(move |_| n.set(n.get() + 1)));
    let recent = h.recent();
    let pinned = h.pinned();
    for row in rows().iter().filter(|r| r["kind"] == "history") {
        let op = row["op"].as_array().unwrap();
        let command = text(&op[0]).to_utf8().unwrap();
        let r = match command.as_str() {
            "max" => {
                h.set_max_recent(op[1].as_i64().unwrap() as i32);
                Ok(())
            }
            "pin" => h.pin(&text(&op[1])),
            "add" => h.add(&text(&op[1])),
            "remove" => h.remove(&text(&op[1])),
            "unpin" => h.unpin(&text(&op[1])),
            "clear" => h.clear_recent(),
            "import" => h.import(&text(&op[1]).to_utf8().unwrap()),
            _ => unreachable!(),
        };
        assert_eq!(result(r, |_| json!(true)), row["result"], "{row}");
        assert!(Arc::ptr_eq(&recent, &h.recent()));
        assert!(Arc::ptr_eq(&pinned, &h.pinned()));
        let array = |v: Vec<Option<Utf16Text>>| {
            json!(
                v.iter()
                    .map(|s| s.as_ref().map(Utf16Text::units))
                    .collect::<Vec<_>>()
            )
        };
        assert_eq!(
            json!({"recent":array(h.recent().lock().clone()),"pinned":array(h.pinned().lock().clone()),"combined":array(h.combined()),"export":h.export(),"events":events.get(),"preferred":h.get_preferred_tone(&"👋".into()).map(|t|t.0)}),
            row["state"],
            "{row}"
        );
    }
}
#[test]
fn shared_cache_and_owned_snapshots_use_native_collections() {
    let c = EmojiCatalog::load(&"en-US".into()).unwrap();
    let other = std::thread::spawn(|| EmojiCatalog::load(&"EN".into()).unwrap())
        .join()
        .unwrap();
    assert!(Arc::ptr_eq(&c, &other));
    let custom = EmojiCatalog::parse(&"a;one;x\nb;two;y\n".into(), "x".into()).unwrap();
    let category = custom.get_emojis(EmojiCategory::SmileysEmotion);
    let mut iter = custom.all();
    assert_eq!(iter.next().unwrap().value, Utf16Text::from("a"));
    let added = Arc::new(EmojiInfo::new("c".into()));
    category.lock().push(added.clone());
    // Existing iterators own Arcs and remain stable after a native Vec mutation.
    assert_eq!(iter.next().unwrap().value, Utf16Text::from("b"));
    assert!(iter.next().is_none());
    assert!(Arc::ptr_eq(&custom.all().last().unwrap(), &added));
    let keywords = added.keywords.clone();
    std::thread::spawn(move || keywords.lock().push("updated".into()))
        .join()
        .unwrap();
    assert!(Arc::ptr_eq(&custom.search(&"updated".into(), 1)[0], &added));
    let a = EmojiInfo::new("a".into());
    let b = EmojiInfo::new("b".into());
    a.variants().lock().push("a-tone".into());
    assert!(a.has_skin_tones());
    assert!(!b.has_skin_tones());
    assert!(a.keywords.lock().is_empty());
}

#[test]
fn upstream_emoji_catalog_loads_categories_and_searches() {
    let catalog = EmojiCatalog::load(&"en".into()).unwrap();
    assert_eq!(catalog.categories().len(), 9);
    assert!(catalog.all().count() > 1800);
    assert_eq!(
        catalog.search(&"pizza".into(), 50).first().unwrap().value,
        Utf16Text::from("🍕")
    );
    let wave = catalog.get(&"👋".into()).unwrap();
    assert!(wave.has_skin_tones());
    assert_eq!(
        wave.with_skin_tone(EmojiSkinTone::Medium).unwrap(),
        Utf16Text::from("👋🏽")
    );
    assert!(Arc::ptr_eq(&wave, &catalog.get(&"👋🏿".into()).unwrap()));
    assert!(catalog.search(&"".into(), 50).is_empty());
    assert!(catalog.search(&" ".into(), 50).is_empty());
}
#[test]
fn upstream_emoji_catalog_supports_other_languages() {
    let de = EmojiCatalog::load(&"de-DE".into()).unwrap();
    assert_eq!(de.locale(), Utf16Text::from("de"));
    assert!(!de.search(&"Pizza".into(), 50).is_empty());
    assert_eq!(
        EmojiCatalog::load(&"xx".into()).unwrap().locale(),
        Utf16Text::from("en")
    );
}
#[test]
fn upstream_emoji_history_orders_and_persists() {
    let history = EmojiHistory::default();
    history.set_max_recent(3);
    for s in ["😀", "👋🏽", "😀", "🍕", "🎉"] {
        history.add(&s.into()).unwrap();
    }
    assert_eq!(
        history.recent().lock().clone(),
        ["🎉", "🍕", "😀"].map(|v| Some(Utf16Text::from(v)))
    );
    assert_eq!(
        history.get_preferred_tone(&"👋".into()),
        Some(EmojiSkinTone::Medium)
    );
    history.pin(&"❤️".into()).unwrap();
    let copy = EmojiHistory::default();
    copy.import(&history.export()).unwrap();
    assert_eq!(history.combined(), copy.combined());
}
#[test]
fn upstream_emoticons_load() {
    assert!(
        EmojiCatalog::load_emoticons()
            .unwrap()
            .iter()
            .any(|e| e.text == Utf16Text::from(":-)"))
    );
}

#[test]
fn keyword_prefix_unicode_boundaries_and_live_mutations_match_source() {
    for line in include_str!("../../../tests/fixtures/native-emoji-search.jsonl").lines() {
        let row: Value = serde_json::from_str(line).unwrap();
        let catalog = EmojiCatalog::parse(&text(&row["input"]), "custom".into()).unwrap();
        if row["mutate"].as_bool().unwrap() {
            // Search before mutation, so a future stale keyword cache cannot hide the edit.
            catalog.search(&"al".into(), 50);
            catalog.all().next().unwrap().keywords.lock()[0] = "updated".into();
        }
        assert_eq!(
            values(catalog.search(&text(&row["query"]), row["max"].as_i64().unwrap() as i32)),
            row["result"],
            "query={:?}, mutate={}, max={}",
            row["query"],
            row["mutate"],
            row["max"]
        );
    }
}

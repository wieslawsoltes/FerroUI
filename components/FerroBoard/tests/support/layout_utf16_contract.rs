//! Exact source text units and underlying object/list aliasing; not a CLR box/GC identity claim.
use ferroboard_core::{layouts::*, text::Utf16Text};
use serde_json::{Value, json};
use std::{rc::Rc, sync::Arc};
pub const FIELDS: [&str; 3] = ["ExtensionId", "Id", "Label"];
pub fn values() -> Vec<Option<Utf16Text>> {
    vec![
        None,
        Some("".into()),
        Some("a".into()),
        Some("a:b:c".into()),
        Some(":leading".into()),
        Some("trailing:".into()),
        Some(Utf16Text::from_units(vec![0xd800, 120, 0xdfff])),
        Some("😀".into()),
        Some("ßİéالعربية".into()),
        Some("\0".into()),
        Some("\r\n".into()),
        Some("a\u{301}".into()),
    ]
}
pub trait Driver {
    fn component(a: Option<Utf16Text>, b: Option<Utf16Text>) -> ComponentName;
    fn default_component() -> ComponentName;
    fn parse(
        value: Option<Utf16Text>,
        extension: Option<Utf16Text>,
        omitted: bool,
    ) -> Result<ComponentName, &'static str>;
    fn name_fields(n: &ComponentName) -> [Option<Utf16Text>; 2];
    fn format(n: &ComponentName) -> Utf16Text;
    fn get(l: &Rc<KeyboardLayout>, field: usize) -> Option<Utf16Text>;
    fn set(l: &Rc<KeyboardLayout>, field: usize, v: Option<Utf16Text>);
    fn name(l: &Rc<KeyboardLayout>) -> ComponentName;
    fn metadata(m: &Arc<LayoutMetadata>) -> [Option<Utf16Text>; 2];
    fn authors_same(m: &Arc<LayoutMetadata>, l: &Rc<KeyboardLayout>) -> bool;
    fn author(m: &Arc<LayoutMetadata>, index: i32) -> Result<Option<Utf16Text>, &'static str>;
    fn author_count(m: &Arc<LayoutMetadata>) -> i32;
    fn key(k: Key, display: bool) -> Result<Option<Utf16Text>, &'static str>;
}
pub struct Core;
impl Driver for Core {
    fn component(a: Option<Utf16Text>, b: Option<Utf16Text>) -> ComponentName {
        ComponentName::new_nullable(a, b)
    }
    fn default_component() -> ComponentName {
        ComponentName::default()
    }
    fn parse(
        v: Option<Utf16Text>,
        e: Option<Utf16Text>,
        omitted: bool,
    ) -> Result<ComponentName, &'static str> {
        let empty = Utf16Text::default();
        ComponentName::parse_nullable(v.as_ref(), if omitted { Some(&empty) } else { e.as_ref() })
            .map_err(|e| {
                assert_eq!(e, ResourceTextListError::NullReference);
                "NullReferenceException"
            })
    }
    fn name_fields(n: &ComponentName) -> [Option<Utf16Text>; 2] {
        [n.extension_id.clone(), n.component_id.clone()]
    }
    fn format(n: &ComponentName) -> Utf16Text {
        n.to_text()
    }
    fn get(l: &Rc<KeyboardLayout>, f: usize) -> Option<Utf16Text> {
        match f {
            0 => l.extension_id(),
            1 => l.id(),
            2 => l.label(),
            _ => unreachable!(),
        }
    }
    fn set(l: &Rc<KeyboardLayout>, f: usize, v: Option<Utf16Text>) {
        match f {
            0 => l.set_extension_id(v),
            1 => l.set_id(v),
            2 => l.set_label(v),
            _ => unreachable!(),
        }
    }
    fn name(l: &Rc<KeyboardLayout>) -> ComponentName {
        l.name()
    }
    fn metadata(m: &Arc<LayoutMetadata>) -> [Option<Utf16Text>; 2] {
        [m.id.clone(), m.label.clone()]
    }
    fn authors_same(m: &Arc<LayoutMetadata>, l: &Rc<KeyboardLayout>) -> bool {
        m.authors
            .as_ref()
            .unwrap()
            .ptr_eq(l.authors().as_ref().unwrap())
    }
    fn author(m: &Arc<LayoutMetadata>, i: i32) -> Result<Option<Utf16Text>, &'static str> {
        usize::try_from(i)
            .map_err(|_| ResourceTextListError::IndexOutOfRange)
            .and_then(|i| m.authors.as_ref().unwrap().get(i))
            .map_err(|e| {
                assert_eq!(e, ResourceTextListError::IndexOutOfRange);
                "ArgumentOutOfRangeException"
            })
    }
    fn author_count(m: &Arc<LayoutMetadata>) -> i32 {
        m.authors.as_ref().unwrap().len() as i32
    }
    fn key(k: Key, display: bool) -> Result<Option<Utf16Text>, &'static str> {
        k.as_string(display).map_err(|e| match e {
            KeyDataError::InvalidCodePoint(_) => "ArgumentOutOfRangeException",
            e => panic!("unexpected key failure {e:?}"),
        })
    }
}
fn units(v: Option<Utf16Text>) -> Value {
    v.map(|s| json!(s.units())).unwrap_or(Value::Null)
}
fn name<D: Driver>(n: ComponentName) -> Value {
    let [a, b] = D::name_fields(&n);
    json!({"extension":units(a),"component":units(b),"text":units(Some(D::format(&n)))})
}
fn state<D: Driver>(l: &Rc<KeyboardLayout>) -> Value {
    json!({"extension":units(D::get(l,0)),"id":units(D::get(l,1)),"label":units(D::get(l,2)),"name":name::<D>(D::name(l))})
}
struct CustomKey(Utf16Text);
impl AbstractKeyData for CustomKey {
    fn compute_nullable(
        self: Rc<Self>,
        _: Option<&dyn IKeyEvaluator>,
    ) -> Result<Option<TextKey>, KeyDataError> {
        Ok(None)
    }
    fn as_string(&self, display: bool) -> Result<Option<Utf16Text>, KeyDataError> {
        if display {
            Ok(Some(self.0.clone()))
        } else {
            let mut v = self.0.units().to_vec();
            v.push(0xd800);
            Ok(Some(Utf16Text::from_units(v)))
        }
    }
}
pub fn verify<D: Driver>(fixture: &str) -> (usize, usize, usize) {
    let v = values();
    let (mut checked, mut facts, missing) = (0, 0, 0);
    for line in fixture.lines() {
        let row: Value = serde_json::from_str(line).unwrap();
        let tag = row["tag"].as_str().unwrap();
        let index = |n: &str| row[n].as_u64().unwrap() as usize;
        match tag {
            "initializer" => {
                assert!(FIELDS.contains(&row["name"].as_str().unwrap()));
                facts += 1;
                continue;
            }
            "null-key-label-audit" => {
                let key: Key = Rc::new(TextKeyData::from_fields(TextKeyDataFields {
                    code: row["code"].as_i64().unwrap() as i32,
                    label: None,
                    ..Default::default()
                }));
                assert_eq!(
                    units(D::key(key, row["display"].as_bool().unwrap()).unwrap()),
                    row["value"]
                );
            }
            "component-default" => assert_eq!(name::<D>(D::default_component()), row["state"]),
            "component" => assert_eq!(
                name::<D>(D::component(v[index("a")].clone(), v[index("b")].clone())),
                row["state"]
            ),
            "parse" => {
                let b = row["b"].as_i64().unwrap();
                let result = D::parse(
                    v[index("a")].clone(),
                    if b < 0 { None } else { v[b as usize].clone() },
                    b < 0,
                );
                match result {
                    Ok(n) => {
                        assert!(row["error"].is_null());
                        assert_eq!(name::<D>(n), row["state"], "{row}");
                    }
                    Err(e) => assert_eq!(e, row["error"].as_str().unwrap()),
                }
            }
            "layout" => {
                let l = Rc::new(KeyboardLayout::default());
                l.set_extension_id(Some("e".into()));
                l.set_id(Some("i".into()));
                l.set_label(Some("label".into()));
                let alias = l.clone();
                let value = v[index("v")].clone();
                let f = index("f");
                D::set(&l, f, value.clone());
                if let Some(value) = value {
                    assert!(value.ptr_eq(&D::get(&l, f).unwrap()));
                }
                assert_eq!(state::<D>(&l), row["state"]);
                assert_eq!(Rc::ptr_eq(&alias, &l), row["same"]);
            }
            "metadata" | "author-index" => {
                let value = v[index("v")].clone();
                let authors =
                    ResourceTexts::from_nullable(vec![None, Some("author".into()), value.clone()]);
                let m = Arc::new(LayoutMetadata {
                    layout_type: LayoutType::Characters,
                    id: value.clone(),
                    label: value.clone(),
                    authors: Some(authors.clone()),
                    direction: Default::default(),
                    modifier: None,
                });
                let l = Rc::new(KeyboardLayout::default());
                l.set_authors(Some(authors.clone()));
                if tag == "metadata" {
                    let [id, label] = D::metadata(&m);
                    assert_eq!(units(id), row["id"]);
                    assert_eq!(units(label), row["label"]);
                    assert_eq!(D::authors_same(&m, &l), row["authorsSame"]);
                    let actual = (0..D::author_count(&m))
                        .map(|i| units(D::author(&m, i).unwrap()))
                        .collect::<Vec<_>>();
                    assert_eq!(json!(actual), row["authors"]);
                } else {
                    authors.set(1, value).unwrap();
                    let result = D::author(&m, row["index"].as_i64().unwrap() as i32);
                    match result {
                        Ok(v) => {
                            assert!(row["error"].is_null());
                            assert_eq!(units(v), row["value"]);
                        }
                        Err(e) => assert_eq!(e, row["error"].as_str().unwrap()),
                    };
                    assert_eq!(D::author_count(&m), row["count"]);
                    assert_eq!(D::authors_same(&m, &l), row["same"]);
                    assert_eq!(
                        json!(
                            authors
                                .snapshot()
                                .into_iter()
                                .map(units)
                                .collect::<Vec<_>>()
                        ),
                        row["state"]
                    );
                }
            }
            "key" | "custom-key" => {
                let text = v[index("v")].clone().unwrap();
                let key: Key = if tag == "key" {
                    Rc::new(TextKeyData::from_fields(TextKeyDataFields {
                        code: row["code"].as_i64().unwrap() as i32,
                        label: Some(text),
                        ..Default::default()
                    }))
                } else {
                    Rc::new(CustomKey(text))
                };
                match D::key(key, row["display"].as_bool().unwrap()) {
                    Ok(v) => {
                        assert!(row["error"].is_null());
                        assert_eq!(units(v), row["value"], "{row}");
                    }
                    Err(e) => assert_eq!(e, row["error"].as_str().unwrap()),
                }
            }
            _ => panic!("unexpected row {tag}"),
        }
        checked += 1;
    }
    (checked, facts, missing)
}

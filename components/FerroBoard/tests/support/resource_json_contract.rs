//! Original JSON-string boundary and registry corpus. Native tests use actual method/event metadata;
//! state snapshots read Core handles, so this does not prove remaining layout property metadata.
use ferroboard_core::{layouts::*, text::Utf16Text};
use serde_json::{Value, json};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
pub type Hook = Rc<dyn Fn()>;
pub type Cleanup = Box<dyn FnOnce()>;
pub trait Driver {
    const DOCUMENTS: bool;
    fn add(
        r: &Rc<KeyboardResources>,
        json: Option<Utf16Text>,
        extension: Option<Utf16Text>,
        omitted: bool,
    ) -> Result<Rc<KeyboardLayout>, String>;
    fn subscribe(r: &Rc<KeyboardResources>, hook: Hook) -> Cleanup;
    fn found(r: &Rc<KeyboardResources>, l: &Rc<KeyboardLayout>) -> bool;
}
pub struct Core;
impl Driver for Core {
    const DOCUMENTS: bool = true;
    fn add(
        r: &Rc<KeyboardResources>,
        json: Option<Utf16Text>,
        extension: Option<Utf16Text>,
        omitted: bool,
    ) -> Result<Rc<KeyboardLayout>, String> {
        r.add_layout_json_utf16(
            json.as_ref(),
            if omitted {
                Some("custom".into())
            } else {
                extension
            },
        )
        .map_err(|e| e.to_string())
    }
    fn subscribe(r: &Rc<KeyboardResources>, hook: Hook) -> Cleanup {
        let token = r.changed().add(Rc::new(move |_| hook()));
        let weak = Rc::downgrade(r);
        Box::new(move || {
            if let Some(r) = weak.upgrade() {
                r.changed().remove(token);
            }
        })
    }
    fn found(r: &Rc<KeyboardResources>, l: &Rc<KeyboardLayout>) -> bool {
        r.get_layout(l.layout_type(), &l.name())
            .unwrap()
            .is_some_and(|v| Rc::ptr_eq(&v, l))
    }
}
pub fn text(v: &Value) -> Option<Utf16Text> {
    (!v.is_null()).then(|| {
        Utf16Text::from_units(
            v.as_array()
                .unwrap()
                .iter()
                .map(|n| n.as_u64().unwrap() as u16)
                .collect::<Vec<_>>(),
        )
    })
}
fn units(v: Option<Utf16Text>) -> Value {
    v.map(|s| json!(s.units())).unwrap_or(Value::Null)
}
fn state(l: &KeyboardLayout) -> Value {
    let authors = l.authors().unwrap().snapshot();
    json!({"type":l.layout_type().0,"extension":units(l.extension_id()),"id":units(l.id()),"label":units(l.label()),"authors":authors.into_iter().map(units).collect::<Vec<_>>(),"direction":l.direction().0,"modifier":l.modifier().map(|m|json!([units(m.extension_id),units(m.component_id)])),"arrangement":serde_json::from_str::<Value>(&LayoutJson::write_arrangement(&l.arrangement().unwrap(),false).unwrap()).unwrap()})
}
#[derive(Debug)]
pub struct ProbeFailure;
fn names(r: &KeyboardResources) -> Value {
    json!(
        r.get_layout_names(LayoutType::Characters)
            .iter()
            .map(|n| json!([units(n.extension_id.clone()), units(n.component_id.clone())]))
            .collect::<Vec<_>>()
    )
}
pub fn verify<D: Driver>(fixture: &str) -> (usize, usize) {
    let (mut count, source_only) = (0, 0);
    for line in fixture.lines() {
        let row: Value = serde_json::from_str(line).unwrap();
        match row["tag"].as_str().unwrap() {
            "escaped-surrogate-audit" => {
                let value = text(&row["json"]);
                let result = if D::DOCUMENTS {
                    LayoutJson::read_layout_document_utf16(value.as_ref(), Some("custom".into()))
                        .map(Rc::new)
                        .map_err(|e| e.0)
                } else {
                    D::add(
                        &Rc::new(KeyboardResources::create_empty()),
                        value,
                        Some("custom".into()),
                        true,
                    )
                };
                match result {
                    Ok(l) => {
                        assert!(row["error"].is_null());
                        let mut expected = row["state"].clone();
                        expected["arrangement"] =
                            serde_json::from_str(expected["arrangement"].as_str().unwrap())
                                .unwrap();
                        assert_eq!(state(&l), expected);
                    }
                    Err(e) => {
                        assert_eq!(row["error"], "InvalidOperationException");
                        assert_eq!(e, "Invalid JSON string encoding");
                    }
                }
            }
            "parse" => {
                let resource = row["action"] == "resource";
                if !resource && !D::DOCUMENTS {
                    continue;
                }
                let r = Rc::new(KeyboardResources::create_empty());
                let events = Rc::new(Cell::new(0));
                let e = events.clone();
                let cleanup = D::subscribe(&r, Rc::new(move || e.set(e.get() + 1)));
                let value = text(&row["json"]);
                let extension = text(&row["extension"]);
                let omitted = row["omitted"].as_bool().unwrap();
                let result = if resource {
                    D::add(&r, value, extension, omitted)
                } else {
                    LayoutJson::read_layout_document_utf16(value.as_ref(), extension)
                        .map(Rc::new)
                        .map_err(|e| e.0)
                };
                match result {
                    Ok(l) => {
                        assert!(row["error"].is_null(), "{row}");
                        let mut expected = row["state"].clone();
                        expected["arrangement"] =
                            serde_json::from_str(expected["arrangement"].as_str().unwrap())
                                .unwrap();
                        assert_eq!(state(&l), expected, "{row}");
                        assert_eq!(resource && D::found(&r, &l), row["registeredSame"]);
                    }
                    Err(error) => {
                        let expected = match row["error"].as_str().unwrap() {
                            "ArgumentNullException" => "Layout JSON input is missing",
                            "ArgumentException" => {
                                "Layout JSON contains unpaired UTF-16 code units"
                            }
                            e => panic!("uncategorized source failure {e}: {row}"),
                        };
                        assert_eq!(error, expected, "{row}");
                        assert_eq!(names(&r), json!([]));
                    }
                }
                assert_eq!(events.get(), row["events"].as_i64().unwrap());
                cleanup();
            }
            "event" => {
                let r = Rc::new(KeyboardResources::create_empty());
                let log = Rc::new(RefCell::new(Vec::new()));
                let once = Rc::new(Cell::new(true));
                let throw = row["mode"] == "throw";
                let weak = Rc::downgrade(&r);
                let out = log.clone();
                let first = once.clone();
                let head = D::subscribe(
                    &r,
                    Rc::new(move || {
                        let r = weak.upgrade().unwrap();
                        out.borrow_mut().push(format!(
                            "head:{}",
                            r.get_layout_names(LayoutType::Characters).len()
                        ));
                        if first.replace(false) {
                            if throw {
                                std::panic::panic_any(ProbeFailure);
                            }
                            D::add(
                                &r,
                                Some("{\"id\":\"nested\"}".into()),
                                Some(Utf16Text::from_units(vec![0xd800])),
                                false,
                            )
                            .unwrap();
                        }
                        out.borrow_mut().push(format!(
                            "after:{}",
                            r.get_layout_names(LayoutType::Characters).len()
                        ));
                    }),
                );
                // Tail removal uses Core tokens, not unimplemented native removal metadata.
                let weak = Rc::downgrade(&r);
                let out = log.clone();
                let token = r.changed().add(Rc::new(move |_| {
                    out.borrow_mut().push(format!(
                        "tail:{}",
                        weak.upgrade()
                            .unwrap()
                            .get_layout_names(LayoutType::Characters)
                            .len()
                    ))
                }));
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    D::add(
                        &r,
                        Some("{\"id\":\"outer😀\"}".into()),
                        Some("custom".into()),
                        true,
                    )
                }));
                let error = match result {
                    Ok(result) => {
                        result.unwrap();
                        Value::Null
                    }
                    Err(payload) => {
                        if !payload.is::<ProbeFailure>() {
                            std::panic::resume_unwind(payload);
                        }
                        json!("ProbeFailure")
                    }
                };
                assert_eq!(error, row["error"]);
                assert_eq!(json!(*log.borrow()), row["log"]);
                assert_eq!(names(&r), row["names"]);
                head();
                r.changed().remove(token);
            }
            tag => panic!("unexpected row {tag}"),
        }
        count += 1;
    }
    (count, source_only)
}

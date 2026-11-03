//! Same actual source operations through direct Core and independently registered native APIs.
use ferroboard_core::{layouts::*, text::Utf16Text};
use serde_json::{Value, json};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
pub trait Driver {
    fn make() -> Rc<CurrencySet>;
    fn set_text(c: &Rc<CurrencySet>, name: &str, value: Option<Utf16Text>);
    fn get_text(c: &Rc<CurrencySet>, name: &str) -> Option<Utf16Text>;
    fn set_slots(c: &Rc<CurrencySet>, value: Option<ResourceReadOnlyList<TextKey>>);
    fn slots(c: &Rc<CurrencySet>) -> Option<ResourceReadOnlyList<TextKey>>;
    fn lookup(c: &Rc<CurrencySet>, code: i32) -> Result<Option<TextKey>, String>;
}
fn units(v: Option<Utf16Text>) -> Value {
    json!(v.map(|v| v.units().to_vec()))
}
fn text(v: &Value) -> Option<Utf16Text> {
    v.as_array().map(|v| {
        Utf16Text::from_units(
            v.iter()
                .map(|v| v.as_u64().unwrap() as u16)
                .collect::<Vec<_>>(),
        )
    })
}
fn key(v: &str) -> TextKey {
    TextKeyData::char(v.into(), None).unwrap()
}
struct Probe {
    mode: String,
    trace: Rc<RefCell<Vec<String>>>,
    action: Rc<dyn Fn()>,
    data: ResourceObjectList<TextKey>,
}
impl IResourceReadOnlyList<TextKey> for Probe {
    fn count(&self) -> Result<i32, ResourceTextListError> {
        self.trace.borrow_mut().push("count".into());
        if self.mode == "throw-count" {
            return Err(ResourceTextListError::IndexOutOfRange);
        }
        if self.mode.ends_with("-count") {
            (self.action)();
        }
        Ok(if self.mode == "negative" {
            -1
        } else {
            self.data.len() as i32
        })
    }
    fn get(&self, index: i32) -> Result<Option<TextKey>, ResourceTextListError> {
        self.trace.borrow_mut().push(format!("get:{index}"));
        if self.mode == "throw-get" {
            return Err(ResourceTextListError::Modified);
        }
        if self.mode == "replace-get" {
            (self.action)();
        }
        self.data
            .get(usize::try_from(index).map_err(|_| ResourceTextListError::IndexOutOfRange)?)
            .map(Some)
    }
    fn iter(&self) -> Result<ResourceItems<TextKey>, ResourceTextListError> {
        ResourceReadOnlyList::from(self.data.clone()).iter()
    }
}
pub fn verify<D: Driver>(fixture: &str) -> usize {
    let mut count = 0;
    for row in fixture
        .lines()
        .map(|v| serde_json::from_str::<Value>(v).unwrap())
    {
        match row["tag"].as_str().unwrap() {
            "property" => {
                let c = D::make();
                let name = row["name"].as_str().unwrap();
                if name == "Slots" {
                    assert!(D::slots(&c).is_none());
                } else {
                    let actual = D::get_text(&c, name);
                    assert_eq!(actual.is_none(), row["isNull"]);
                    assert_eq!(units(actual), row["defaults"]);
                }
            }
            "type" => {}
            "text-init" => {
                let c = D::make();
                let alias = c.clone();
                let name = row["property"].as_str().unwrap();
                let value = text(&row["value"]);
                D::set_text(&alias, name, value.clone());
                assert_eq!(
                    json!({"tag":"text-init","property":name,"value":units(value),"observed":units(D::get_text(&c,name)),"same":Rc::ptr_eq(&alias,&c)}),
                    row
                );
            }
            "lookup" => {
                let c = D::make();
                let trace = Rc::new(RefCell::new(Vec::new()));
                let mode = row["mode"].as_str().unwrap();
                let data = ResourceObjectList::new(vec![key("x"), key("y")]);
                let original = match mode {
                    "null" => None,
                    "list" => Some(data.clone().into()),
                    "empty" => Some(ResourceReadOnlyList::default()),
                    _ => {
                        let replacement = ResourceObjectList::new(vec![key("z")]);
                        let weak = Rc::downgrade(&c);
                        let mode = mode.to_string();
                        let action = Rc::new({
                            let mode = mode.clone();
                            move || {
                                let c = weak.upgrade().unwrap();
                                D::set_slots(
                                    &c,
                                    match mode.as_str() {
                                        "null-count" => None,
                                        "short-count" => Some(
                                            ResourceObjectList::<TextKey>::new(Vec::new()).into(),
                                        ),
                                        _ => Some(replacement.clone().into()),
                                    },
                                );
                            }
                        });
                        Some(ResourceReadOnlyList::from_custom(Rc::new(Probe {
                            mode,
                            trace: trace.clone(),
                            action,
                            data,
                        })))
                    }
                };
                D::set_slots(&c, original.clone());
                let code = row["code"].as_i64().unwrap() as i32;
                let result = match D::lookup(&c, code) {
                    Ok(v) => {
                        json!({"value":units(v.and_then(|v|v.as_string(false).unwrap())),"error":null})
                    }
                    Err(e) => json!({"value":null,"error":e}),
                };
                let retained = match (original, D::slots(&c)) {
                    (None, None) => true,
                    (Some(a), Some(b)) => a.ptr_eq(&b),
                    _ => false,
                };
                assert_eq!(
                    json!({"tag":"lookup","mode":mode,"code":code,"result":result,"trace":trace.borrow().clone(),"originalRetained":retained}),
                    row
                );
            }
            "identity" => {
                let c = D::make();
                D::set_slots(&c, Some(ResourceObjectList::new(vec![key("x")]).into()));
                D::set_text(&c, "Id", Some("new".into()));
                D::set_text(&c, "Label", None);
                D::set_slots(&c, None);
                assert_eq!(std::ptr::eq(&*c, &*c), row["alias"]);
                assert_eq!(std::ptr::eq(&*c, &*D::make()), row["other"]);
                assert_eq!(row["text"], "ScreenKeyboard.Layouts.CurrencySet");
                assert_eq!(
                    c.to_text(),
                    Utf16Text::from("FerroBoard.Layouts.CurrencySet")
                );
            }
            "registry" => {
                let r = KeyboardResources::create_empty();
                let changed = Rc::new(Cell::new(0));
                r.changed().add(Rc::new({
                    let changed = changed.clone();
                    move |_| changed.set(changed.get() + 1)
                }));
                let c = D::make();
                D::set_text(&c, "ExtensionId", None);
                r.add_currency_set(c.clone()).unwrap();
                let old = ComponentName::new_nullable(None, None);
                let retained = Rc::ptr_eq(&c, &r.get_currency_set(&old).unwrap());
                D::set_text(&c, "ExtensionId", Some("new".into()));
                D::set_text(&c, "Id", Some("id".into()));
                let next = ComponentName::new("new", "id");
                let old_still = Rc::ptr_eq(&c, &r.get_currency_set(&old).unwrap());
                let missing = r.get_currency_set(&next).is_none();
                r.add_currency_set(c.clone()).unwrap();
                assert_eq!(
                    json!({"tag":"registry","retained":retained,"oldStill":old_still,"missingNew":missing,"newSame":Rc::ptr_eq(&c,&r.get_currency_set(&next).unwrap()),"count":r.currency_sets().len(),"changes":changed.get()}),
                    row
                );
            }
            _ => panic!("{row}"),
        }
        count += 1;
    }
    count
}

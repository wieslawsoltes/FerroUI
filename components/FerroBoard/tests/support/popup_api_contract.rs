//! Live source popup init/access, enumeration/finally and computed-popup callback order.
use ferroboard_core::{keys::*, layouts::*, text::Utf16Text};
use serde_json::{Value, json};
use std::{cell::RefCell, rc::Rc, sync::Arc};
type Error = ResourceTextListError;
type Row = ResourceReadOnlyList<Key>;
pub trait Driver {
    fn construct() -> Rc<PopupSet>;
    fn main(p: &Rc<PopupSet>) -> Option<Key>;
    fn relevant(p: &Rc<PopupSet>) -> Option<Row>;
    fn set_main(p: &Rc<PopupSet>, v: Option<Key>);
    fn set_relevant(p: &Rc<PopupSet>, v: Option<Row>);
    fn is_empty(p: &Rc<PopupSet>) -> Result<bool, Error>;
    fn count(row: &Row) -> Result<i32, Error>;
    fn get(row: &Row, index: i32) -> Result<Option<Key>, Error>;
    fn enumerate(row: &Row) -> Result<Vec<Option<Key>>, Error>;
}
pub struct Core;
impl Driver for Core {
    fn construct() -> Rc<PopupSet> {
        Rc::new(PopupSet::default())
    }
    fn main(p: &Rc<PopupSet>) -> Option<Key> {
        p.main()
    }
    fn relevant(p: &Rc<PopupSet>) -> Option<Row> {
        p.relevant()
    }
    fn set_main(p: &Rc<PopupSet>, v: Option<Key>) {
        p.set_main(v)
    }
    fn set_relevant(p: &Rc<PopupSet>, v: Option<Row>) {
        p.set_relevant(v)
    }
    fn is_empty(p: &Rc<PopupSet>) -> Result<bool, Error> {
        p.is_empty()
    }
    fn count(row: &Row) -> Result<i32, Error> {
        row.count()
    }
    fn get(row: &Row, index: i32) -> Result<Option<Key>, Error> {
        row.get(index)
    }
    fn enumerate(row: &Row) -> Result<Vec<Option<Key>>, Error> {
        let mut v = Vec::new();
        row.visit::<Error>(|key| {
            v.push(key);
            Ok(())
        })?;
        Ok(v)
    }
}
pub fn error(e: Error) -> &'static str {
    match e {
        Error::NullReference => "NullReferenceException",
        Error::IndexOutOfRange => "ArgumentOutOfRangeException",
        Error::NotSupported => "NotSupportedException",
        Error::Modified => "InvalidOperationException",
        other => panic!("unexpected {other:?}"),
    }
}
fn result(v: Result<Value, Error>) -> Value {
    match v {
        Ok(value) => json!({"value":value,"error":null}),
        Err(e) => json!({"value":null,"error":error(e)}),
    }
}
fn units(v: Option<Utf16Text>) -> Value {
    json!(v.as_ref().map(Utf16Text::units))
}
fn key(text: &str) -> Key {
    TextKeyData::char(text.into(), None).unwrap()
}
fn text(k: Option<Key>) -> Value {
    units(k.and_then(|k| k.as_string(false).unwrap()))
}
fn same(a: Option<Key>, b: Option<Key>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => Rc::ptr_eq(&a, &b),
        _ => false,
    }
}
fn same_list(a: Option<Row>, b: Option<Row>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => a.ptr_eq(&b),
        _ => false,
    }
}
fn json_popup(p: Rc<PopupSet>) -> Result<Value, Error> {
    let a =
        LayoutArrangement::from_rows(vec![vec![TextKeyData::char("x".into(), Some(p)).unwrap()]]);
    LayoutJson::write_arrangement(&a, false)
        .map(|v| serde_json::from_str(&v).unwrap())
        .map_err(|e| match e.0.as_str() {
            "object reference was null" | "Required layout data is missing" => Error::NullReference,
            "resource data changed while being read" => Error::Modified,
            "resource operation is unsupported" => Error::NotSupported,
            other => panic!("unexpected JSON {other}"),
        })
}
pub struct TraceList {
    pub data: Vec<Option<Key>>,
    pub trace: Rc<RefCell<Vec<String>>>,
    pub failure: String,
    pub count: Option<i32>,
    pub action: Option<Rc<dyn Fn()>>,
}
impl IResourceReadOnlyList<Key> for TraceList {
    fn count(&self) -> Result<i32, Error> {
        self.trace.borrow_mut().push("count".into());
        if let Some(a) = &self.action {
            a();
        }
        if self.failure == "count" {
            return Err(Error::Modified);
        }
        Ok(self.count.unwrap_or(self.data.len() as i32))
    }
    fn get(&self, index: i32) -> Result<Option<Key>, Error> {
        self.trace.borrow_mut().push(format!("index:{index}"));
        self.data
            .get(usize::try_from(index).map_err(|_| Error::IndexOutOfRange)?)
            .cloned()
            .ok_or(Error::IndexOutOfRange)
    }
    fn iter(&self) -> Result<ResourceItems<Key>, Error> {
        self.trace.borrow_mut().push("get".into());
        if self.failure == "get" {
            return Err(Error::Modified);
        }
        Ok(Box::new(Cursor {
            data: self.data.clone(),
            trace: self.trace.clone(),
            failure: self.failure.clone(),
            index: 0,
        }))
    }
}
struct Cursor {
    data: Vec<Option<Key>>,
    trace: Rc<RefCell<Vec<String>>>,
    failure: String,
    index: usize,
}
impl Iterator for Cursor {
    type Item = Result<Option<Key>, Error>;
    fn next(&mut self) -> Option<Self::Item> {
        self.trace.borrow_mut().push(format!("move:{}", self.index));
        if self.failure == "move" {
            return Some(Err(Error::Modified));
        }
        let value = self.data.get(self.index)?.clone();
        self.trace
            .borrow_mut()
            .push(format!("current:{}", self.index));
        self.index += 1;
        Some(if self.failure == "current" {
            Err(Error::Modified)
        } else {
            Ok(value)
        })
    }
}
impl Drop for Cursor {
    fn drop(&mut self) {
        self.trace.borrow_mut().push("dispose".into());
    }
}
struct Probe {
    name: &'static str,
    trace: Rc<RefCell<Vec<String>>>,
    action: Rc<dyn Fn()>,
    fail: bool,
}
impl AbstractKeyData for Probe {
    fn compute_nullable(
        self: Rc<Self>,
        _: Option<&dyn IKeyEvaluator>,
    ) -> Result<Option<TextKey>, KeyDataError> {
        self.trace
            .borrow_mut()
            .push(format!("compute:{}", self.name));
        (self.action)();
        if self.fail {
            return Err(KeyDataError::Collection(Error::NotSupported));
        }
        Ok(Some(TextKeyData::char(self.name.into(), None)?))
    }
    fn as_string(&self, _: bool) -> Result<Option<Utf16Text>, KeyDataError> {
        Ok(Some(self.name.into()))
    }
}
pub fn verify<D: Driver + 'static>(fixture: &str) -> (usize, usize) {
    let (mut checked, mut facts) = (0, 0);
    for line in fixture.lines() {
        let row: Value = serde_json::from_str(line).unwrap();
        let tag = row["tag"].as_str().unwrap();
        if tag == "compute"
            && matches!(
                row["failure"].as_str(),
                Some("null-enumerator" | "dispose" | "body-dispose")
            )
        {
            continue;
        }
        let trace = Rc::new(RefCell::new(Vec::<String>::new()));
        match tag {
            "access" => {
                let storage = row["storage"].as_str().unwrap();
                let a = key("a");
                let b = key("b");
                let list: Option<Row> = match storage {
                    "null" => None,
                    "default" => Some(Row::default()),
                    "list" => Some(ResourceObjectList::new(vec![a.clone(), b.clone()]).into()),
                    "array" => Some(ResourceObjectList::new(vec![a.clone(), b.clone()]).into()),
                    "null-item" => Some(Row::from_nullable(ResourceObjectList::new(vec![
                        None,
                        Some(a.clone()),
                    ]))),
                    _ => Some(Row::from_custom(Rc::new(TraceList {
                        data: vec![Some(a.clone()), Some(b)],
                        trace: trace.clone(),
                        failure: if storage == "count-error" {
                            "count"
                        } else {
                            "none"
                        }
                        .into(),
                        count: (storage == "negative").then_some(-1),
                        action: None,
                    }))),
                };
                let main = if row["main"] == true { Some(a) } else { None };
                let p = D::construct();
                D::set_main(&p, main.clone());
                D::set_relevant(&p, list.clone());
                let alias = p.clone();
                let op = row["op"].as_str().unwrap();
                let value = match op {
                    "empty" => D::is_empty(&p).map(|v| json!(v)),
                    "json" => json_popup(p.clone()),
                    "enumerate" => list
                        .as_ref()
                        .ok_or(Error::NullReference)
                        .and_then(D::enumerate)
                        .map(|v| json!(v.into_iter().map(text).collect::<Vec<_>>())),
                    _ => list
                        .as_ref()
                        .ok_or(Error::NullReference)
                        .and_then(|r| D::get(r, op[6..].parse().unwrap()))
                        .and_then(|v| v.ok_or(Error::NullReference))
                        .map(|k| text(Some(k))),
                };
                let actual = json!({"tag":"access","storage":storage,"main":row["main"],"op":op,"result":result(value),"trace":*trace.borrow(),"aliasSame":Rc::ptr_eq(&p,&alias),"listSame":same_list(list,D::relevant(&p)),"mainSame":same(main,D::main(&p))});
                assert_eq!(actual, row, "{row}");
            }
            "init" => {
                let p = D::construct();
                let alias = p.clone();
                let old = D::relevant(&p).unwrap();
                let field = row["field"].as_str().unwrap();
                let v = row["v"].as_u64().unwrap() as usize;
                let a = key("a");
                let b = key("b");
                let values: Row = ResourceObjectList::new(vec![a.clone()]).into();
                let fixed: Row = ResourceObjectList::new(vec![b.clone()]).into();
                let expected = if field == "Main" {
                    D::set_main(
                        &p,
                        match v {
                            0 => None,
                            1 => Some(a),
                            _ => Some(b),
                        },
                    );
                    None
                } else {
                    let value = match v {
                        0 => None,
                        1 => Some(values),
                        _ => Some(fixed),
                    };
                    D::set_relevant(&p, value.clone());
                    value
                };
                let actual = json!({"tag":"init","field":field,"v":v,"main":text(D::main(&p)),"same":Rc::ptr_eq(&p,&alias),"listSame":same_list(expected,D::relevant(&p)),"oldRetained":old.count().unwrap(),"isEmpty":result(D::is_empty(&p).map(|v|json!(v))),"count":result(D::relevant(&p).ok_or(Error::NullReference).and_then(|v|D::count(&v)).map(|v|json!(v)))});
                assert_eq!(actual, row, "{row}");
            }
            "compute" => verify_compute::<D>(&row, trace),
            "count-reentrant" => {
                let p = D::construct();
                let replacement: Row = ResourceObjectList::new(vec![key("b")]).into();
                let reaction = row["reaction"].as_str().unwrap().to_owned();
                let w = Rc::downgrade(&p);
                let r = replacement.clone();
                let log = trace.clone();
                let act = reaction.clone();
                let list = Row::from_custom(Rc::new(TraceList {
                    data: vec![Some(key("a"))],
                    trace: trace.clone(),
                    failure: "none".into(),
                    count: Some(row["countValue"].as_i64().unwrap() as i32),
                    action: Some(Rc::new(move || {
                        log.borrow_mut().push(format!("react:{act}"));
                        let p = w.upgrade().unwrap();
                        match act.as_str() {
                            "main" => D::set_main(&p, Some(key("a"))),
                            "swap" => D::set_relevant(&p, Some(r.clone())),
                            "null" => D::set_relevant(&p, None),
                            _ => {}
                        }
                    })),
                }));
                D::set_relevant(&p, Some(list.clone()));
                if row["initialMain"] == true {
                    D::set_main(&p, Some(key("a")));
                }
                let operation = row["operation"].as_str().unwrap();
                let value = if operation == "empty" {
                    D::is_empty(&p).map(|v| json!(v))
                } else {
                    json_popup(p.clone())
                };
                let current = D::relevant(&p);
                let name = match current {
                    None => "null",
                    Some(value) if value.ptr_eq(&list) => "original",
                    Some(_) => "replacement",
                };
                let actual = json!({"tag":"count-reentrant","reaction":reaction,"operation":operation,"initialMain":row["initialMain"],"countValue":row["countValue"],"result":result(value),"trace":*trace.borrow(),"mainNull":D::main(&p).is_none(),"relevant":name});
                assert_eq!(actual, row, "{row}");
            }
            "property" => {
                facts += 1;
                continue;
            }
            _ => panic!("unexpected {tag}"),
        }
        checked += 1;
    }
    (checked, facts)
}
fn verify_compute<D: Driver + 'static>(row: &Value, trace: Rc<RefCell<Vec<String>>>) {
    let placement = row["placement"].as_str().unwrap();
    let failure = row["failure"].as_str().unwrap();
    let mutation = row["mutation"].as_str().unwrap().to_owned();
    let p = D::construct();
    let replacement: Row = ResourceObjectList::new(vec![key("n")]).into();
    let target = Rc::downgrade(&p);
    let repl = replacement.clone();
    let action = Rc::new(move || {
        let p = target.upgrade().unwrap();
        match mutation.as_str() {
            "swap" => D::set_relevant(&p, Some(repl.clone())),
            "null" => D::set_relevant(&p, None),
            "clear-main" => D::set_main(&p, None),
            _ => {}
        }
    });
    let probe: Key = Rc::new(Probe {
        name: if placement == "main" { "m" } else { "r" },
        trace: trace.clone(),
        action,
        fail: matches!(failure, "body" | "body-dispose"),
    });
    let data = vec![
        if failure == "null-item" {
            None
        } else if placement == "relevant" {
            Some(probe.clone())
        } else {
            Some(key("r"))
        },
        Some(key("t")),
    ];
    let list = Row::from_custom(Rc::new(TraceList {
        data,
        trace: trace.clone(),
        failure: failure.into(),
        count: None,
        action: None,
    }));
    D::set_main(
        &p,
        if placement == "main" {
            Some(probe)
        } else {
            None
        },
    );
    D::set_relevant(&p, Some(list.clone()));
    let resources = Rc::new(KeyboardResources::create_empty());
    resources.add_layout(Rc::new(KeyboardLayout::new(
        LayoutType::Characters,
        "p".into(),
        "keys".into(),
        Rc::new(LayoutArrangement::from_rows(vec![vec![
            TextKeyData::char("a".into(), Some(p.clone())).unwrap(),
        ]])),
    )));
    let subtype = Arc::new(Subtype::from_values(
        ferroboard_core::layouts::SubtypeValues {
            layouts: Some(Arc::new(SubtypeLayoutMap::from_values(
                ferroboard_core::layouts::SubtypeLayoutMapValues {
                    characters: ComponentName::new("p", "keys"),
                    ..(*Subtype::shared_default().layouts().unwrap()).values()
                },
            ))),
            ..(*Subtype::shared_default()).values()
        },
    ));
    let keyboard = LayoutComputer::new(resources)
        .compute(KeyboardMode::Characters, subtype, false)
        .unwrap();
    let key = keyboard
        .keys()
        .map(|key| key.unwrap().unwrap())
        .next()
        .unwrap();
    let context = Rc::new(KeyComputeContext::from_values(
        ferroboard_core::layouts::KeyComputeContextValues {
            show_number_hints: false,
            show_symbol_hints: false,
            ..Default::default()
        },
    ));
    let failure = match keyboard.compute(context.clone()) {
        Ok(()) => None,
        Err(KeyDataError::Collection(e)) => Some(error(e)),
        Err(KeyDataError::NullReference) => Some("NullReferenceException"),
        Err(e) => panic!("unexpected {e:?}"),
    };
    let current = D::relevant(&p);
    let name = match current {
        None => "null",
        Some(value) if value.ptr_eq(&list) => "original",
        Some(_) => "replacement",
    };
    let actual = json!({"tag":"compute","placement":placement,"failure":row["failure"],"mutation":row["mutation"],"error":failure,"trace":*trace.borrow(),"version":keyboard.version(),"contextSame":keyboard.context().is_some_and(|c|Rc::ptr_eq(&c,&context)),"mainNull":D::main(&p).is_none(),"relevant":name,"code":key.code(),"label":units(key.label()),"visible":key.is_visible(),"enabled":key.is_enabled(),"popups":key.popup_keys().snapshot().unwrap().into_iter().map(|k|units(k.as_string(false).unwrap())).collect::<Vec<_>>(),"index":key.popup_default_index()});
    assert_eq!(actual, *row, "{row}");
}

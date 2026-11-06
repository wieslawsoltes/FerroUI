//! Original derived-key scalar-array and replacement behavior; native virtual calls are injectable.
use ferroboard_core::{layouts::*, text::Utf16Text};
use serde_json::{Value, json};
use std::rc::Rc;
pub trait Driver {
    fn make() -> Rc<MultiTextKeyData>;
    fn array(k: &Rc<MultiTextKeyData>) -> Option<CodePointArray> {
        k.code_points()
    }
    fn init_array(k: &Rc<MultiTextKeyData>, array: Option<CodePointArray>) {
        k.set_code_points(array);
    }

    fn produces(k: &Rc<MultiTextKeyData>) -> Result<bool, KeyDataError>;
    fn text(k: &Rc<MultiTextKeyData>, display: bool) -> Result<Option<Utf16Text>, KeyDataError>;
    fn with(k: &Rc<MultiTextKeyData>, code: Option<i32>) -> TextKey;
    fn compute(k: Rc<MultiTextKeyData>) -> Result<Option<TextKey>, KeyDataError>;
}
pub struct Core;
impl Driver for Core {
    fn make() -> Rc<MultiTextKeyData> {
        Rc::default()
    }
    fn produces(k: &Rc<MultiTextKeyData>) -> Result<bool, KeyDataError> {
        k.produces_text()
    }
    fn text(k: &Rc<MultiTextKeyData>, display: bool) -> Result<Option<Utf16Text>, KeyDataError> {
        k.as_string(display)
    }
    fn with(k: &Rc<MultiTextKeyData>, code: Option<i32>) -> TextKey {
        k.with(KeyChanges {
            code,
            ..Default::default()
        })
    }
    fn compute(k: Rc<MultiTextKeyData>) -> Result<Option<TextKey>, KeyDataError> {
        k.compute_nullable(None)
    }
}
fn points(a: usize) -> Option<CodePointArray> {
    match a {
        0 => None,
        1 => Some(CodePointArray::default()),
        2 => Some(CodePointArray::new(vec![])),
        _ => Some(CodePointArray::new(match a {
            3 => vec![97],
            4 => vec![0x1f600, 0x301],
            5 => vec![0],
            6 => vec![0xd800],
            7 => vec![0xdfff],
            8 => vec![-1],
            9 => vec![0x110000],
            10 => vec![i32::MIN, i32::MAX],
            _ => panic!("unknown source array"),
        })),
    }
}
fn error(e: KeyDataError) -> (&'static str, Option<&'static str>) {
    match e {
        KeyDataError::NullReference => ("NullReferenceException", None),
        KeyDataError::ArrayIndexOutOfRange => ("IndexOutOfRangeException", None),
        KeyDataError::InvalidCodePoint(_) => ("ArgumentOutOfRangeException", Some("utf32")),
        other => panic!("unexpected {other:?}"),
    }
}
fn result(f: impl FnOnce() -> Result<Value, KeyDataError>) -> Value {
    match f() {
        Ok(v) => json!({"value":v,"error":null,"param":null}),
        Err(e) => {
            let (name, param) = error(e);
            json!({"value":null,"error":name,"param":param})
        }
    }
}

fn units(text: Option<Utf16Text>) -> Value {
    text.map_or(Value::Null, |v| json!(v.units()))
}
fn same_array(a: Option<CodePointArray>, b: Option<CodePointArray>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => a.ptr_eq(&b),
        _ => false,
    }
}
fn operation<D: Driver>(row: &Value) -> Value {
    let a = row["a"].as_u64().unwrap() as usize;
    let array = points(a);
    let k = D::make();
    D::init_array(&k, array.clone());
    k.base.set_code(row["code"].as_i64().unwrap() as i32);
    k.base.set_label(Some("label".into()));
    k.base.set_group_id(3);
    let popup = Rc::new(PopupSet::default());
    k.base.set_popup(Some(popup.clone()));
    let op = row["op"].as_str().unwrap();
    let outcome = result(|| match op {
        "produces" => D::produces(&k).map(|v| json!(v)),
        "string" | "display" => D::text(&k, op == "display").map(units),
        "compute-null" => D::compute(k.clone())
            .map(|v| json!(v.is_some_and(|v| std::ptr::addr_eq(Rc::as_ptr(&v), Rc::as_ptr(&k))))),
        "with" => {
            let copy = D::with(&k, Some(65));
            let original: Rc<dyn std::any::Any> = copy.clone();
            let multi = original
                .downcast::<MultiTextKeyData>()
                .unwrap_or_else(|_| panic!("source With must retain Multi runtime class"));
            Ok(
                json!({"code":copy.data().code(),"label":units(copy.data().label()),"group":copy.data().group_id(),"popup":copy.data().popup().as_ref().is_some_and(|p|Rc::ptr_eq(p,&popup)),"same":std::ptr::addr_eq(Rc::as_ptr(&copy),Rc::as_ptr(&k)),"array":same_array(D::array(&multi),array.clone())}),
            )
        }
        "json" => {
            match LayoutJson::write_arrangement(
                &LayoutArrangement::from_rows(vec![vec![k.clone()]]),
                false,
            ) {
                Ok(v) => Ok(serde_json::from_str(&v).unwrap()),
                Err(e) if e.0 == "Required layout data is missing" => {
                    Err(KeyDataError::NullReference)
                }
                Err(e) => panic!("unexpected {e:?}"),
            }
        }
        _ if op.starts_with("index:") => array
            .as_ref()
            .ok_or(KeyDataError::NullReference)?
            .get(op[6..].parse().unwrap())
            .map(|v| json!(v)),
        _ => {
            let array = array.as_ref().ok_or(KeyDataError::NullReference)?;
            array.set(op[4..].parse().unwrap(), 98)?;
            Ok(json!(array.snapshot()))
        }
    });
    json!({"tag":"operation","a":a,"code":row["code"],"op":op,"result":outcome,"points":D::array(&k).map(|v|v.snapshot()),"codeAfter":k.base.code(),"sameArray":same_array(D::array(&k),array)})
}
fn replace<D: Driver>(row: &Value) -> Value {
    let a = row["a"].as_u64().unwrap() as usize;
    // A native copy owns its storage; CLR empty-array interning is retired.
    let original = points(a).map(|points| CodePointArray::new(points.snapshot()));
    let k = D::make();
    D::init_array(&k, original.clone());
    let alias = k.clone();
    let copy = D::with(&k, None);
    let erased: Rc<dyn std::any::Any> = copy;
    let copy = erased
        .downcast::<MultiTextKeyData>()
        .unwrap_or_else(|_| panic!("source With must retain class"));
    let replacement = row["replacement"].as_u64().unwrap();
    let next = match replacement {
        0 => None,
        1 => Some(CodePointArray::default()),
        _ => Some(CodePointArray::new(vec![122])),
    };
    D::init_array(&k, next.clone());
    if let Some(points) = &original
        && !points.is_empty()
    {
        points.set(0, 98).unwrap();
    }
    json!({"tag":"replace","a":a,"replacement":replacement,"same":Rc::ptr_eq(&k,&alias),"copySame":Rc::ptr_eq(&k,&copy),"keyArraySame":same_array(D::array(&k),next),"copyArraySame":same_array(D::array(&copy),original),"key":result(||D::text(&k,false).map(units)),"copy":result(||D::text(&copy,false).map(units)),"keyProduces":result(||D::produces(&k).map(|v|json!(v))),"copyProduces":result(||D::produces(&copy).map(|v|json!(v)))})
}
pub fn verify<D: Driver>(fixture: &str) -> (usize, usize) {
    let (mut behavior, mut facts) = (0, 0);
    for line in fixture.lines() {
        let mut row: Value = serde_json::from_str(line).unwrap();
        if row["tag"] == "defaults" {
            // Native code-point buffers do not intern empty arrays.
            row.as_object_mut().unwrap().remove("sharedEmpty");
            row.as_object_mut().unwrap().remove("sharedArrayEmpty");
        }
        let actual = match row["tag"].as_str().unwrap() {
            "operation" => operation::<D>(&row),
            "replace" => replace::<D>(&row),
            "defaults" => {
                let a = D::make();
                let points = D::array(&a).unwrap();
                json!({"tag":"defaults","code":a.base.code(),"label":units(a.base.label()),"group":a.base.group_id(),"popupNull":a.base.popup().is_none(),"produces":D::produces(&a).unwrap(),"freshEmptyDifferent":!points.ptr_eq(&CodePointArray::new(vec![]))})
            }
            "carrier" => {
                let original = CodePointArray::new(vec![97]);
                let first = original.clone();
                let second = original.clone();
                first.set(0, 98).unwrap();
                json!({"tag":"carrier","sameStorage":first.ptr_eq(&second),"original":original.get(0).unwrap()})
            }
            "class" | "method" => {
                facts += 1;
                continue;
            }
            other => panic!("unknown {other}"),
        };
        assert_eq!(actual, row, "{line}");
        behavior += 1;
    }
    (behavior, facts)
}

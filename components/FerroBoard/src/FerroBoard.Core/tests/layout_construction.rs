//! Direct unchanged-source record/value arithmetic and retained collection construction.
use ferroboard_core::layouts::*;
use serde_json::{Value, json};
use std::{cell::RefCell, rc::Rc};
fn rows() -> Vec<Value> {
    include_str!("../../../tests/fixtures/layout-construction.jsonl")
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect()
}
fn number(v: &Value) -> f64 {
    f64::from_bits(u64::from_str_radix(v.as_str().unwrap(), 16).unwrap())
}
fn bits(v: f64) -> String {
    format!("{:016x}", v.to_bits())
}
fn arithmetic(v: f64) -> String {
    if v.is_nan() { "nan".into() } else { bits(v) }
}
fn fields(r: KeyRect) -> [String; 4] {
    [bits(r.x), bits(r.y), bits(r.width), bits(r.height)]
}
#[test]
fn source_key_rect_values_pairs_boundaries_and_arithmetic() {
    let rows = rows();
    let rects = rows
        .iter()
        .filter(|v| v["kind"] == "record")
        .map(|v| {
            let f = &v["fields"];
            KeyRect::new(number(&f[0]), number(&f[1]), number(&f[2]), number(&f[3]))
        })
        .collect::<Vec<_>>();
    assert_eq!(rects.len(), 142);
    let mut count = 0;
    for row in &rows {
        let kind = row["kind"].as_str().unwrap();
        if kind == "arrangement" {
            continue;
        }
        let actual = if kind == "equality" {
            let a = rects[row["a"].as_u64().unwrap() as usize];
            let b = rects[row["b"].as_u64().unwrap() as usize];
            if a == b {
                use std::hash::{Hash, Hasher};
                let mut ah = std::collections::hash_map::DefaultHasher::new();
                let mut bh = std::collections::hash_map::DefaultHasher::new();
                a.hash(&mut ah);
                b.hash(&mut bh);
                assert_eq!(ah.finish(), bh.finish());
            }
            json!({"kind":kind,"a":row["a"],"b":row["b"],"equal":a==b,"different":a!=b,"typed":a==b})
        } else {
            let r = rects[row["index"].as_u64().unwrap() as usize];
            match kind {
                "record" => {
                    let KeyRect {
                        x,
                        y,
                        width: w,
                        height: h,
                    } = r;
                    let mut changed = r;
                    changed.width = -changed.width;
                    json!({"kind":kind,"index":row["index"],"fields":fields(r),"deconstruct":[bits(x),bits(y),bits(w),bits(h)],"changed":fields(changed),"self":r.eq(&r),"left":bits(r.left()),"top":bits(r.top()),"right":arithmetic(r.right()),"bottom":arithmetic(r.bottom()),"centerX":arithmetic(r.center_x()),"centerY":arithmetic(r.center_y()),"empty":r.is_empty()})
                }
                "point" => {
                    let a = &row["input"];
                    let x = number(&a[0]);
                    let y = number(&a[1]);
                    json!({"kind":kind,"index":row["index"],"point":row["point"],"input":a,"contains":r.contains(x,y),"distance":arithmetic(r.center_distance_squared(x,y))})
                }
                "deflate" => {
                    let a = &row["input"];
                    let d = r.deflate(number(&a[0]), number(&a[1]), number(&a[2]), number(&a[3]));
                    json!({"kind":kind,"index":row["index"],"margin":row["margin"],"input":a,"value":[arithmetic(d.x),arithmetic(d.y),arithmetic(d.width),arithmetic(d.height)]})
                }
                _ => panic!("unexpected {kind}"),
            }
        };
        // CLR boxing and exact hash numbers are not geometry behavior. The
        // unchanged fixture still supplies all arithmetic and hit-test values.
        let mut expected = row.clone();
        for field in [
            "hash",
            "hashEqual",
            "boxed",
            "nullObject",
            "otherObject",
            "nullableSome",
            "nullableNone",
        ] {
            expected.as_object_mut().unwrap().remove(field);
        }
        assert_eq!(actual, expected, "{row}");
        count += 1;
    }
    assert_eq!(count, 21726);
}
struct ProbeRows {
    values: ResourceObjectList<ArrangementRow>,
    count_fault: bool,
    enum_fault: bool,
    trace: Rc<RefCell<Vec<String>>>,
}
impl IResourceReadOnlyList<ArrangementRow> for ProbeRows {
    fn count(&self) -> Result<i32, ResourceTextListError> {
        self.trace.borrow_mut().push("count".into());
        if self.count_fault {
            Err(ResourceTextListError::NotSupported)
        } else {
            Ok(self.values.len() as i32)
        }
    }
    fn get(&self, index: i32) -> Result<Option<ArrangementRow>, ResourceTextListError> {
        self.trace.borrow_mut().push(format!("index:{index}"));
        self.values
            .get(usize::try_from(index).map_err(|_| ResourceTextListError::IndexOutOfRange)?)
            .map(Some)
    }
    fn iter(&self) -> Result<ResourceItems<ArrangementRow>, ResourceTextListError> {
        self.trace.borrow_mut().push("enumerator".into());
        if self.enum_fault {
            Err(ResourceTextListError::NotSupported)
        } else {
            ArrangementRows::from(self.values.clone()).iter()
        }
    }
}
#[test]
fn unchanged_arrangement_constructor_retains_null_and_custom_rows_without_callbacks() {
    for row in rows().into_iter().filter(|v| v["kind"] == "arrangement") {
        let profile = row["profile"].as_str().unwrap();
        let trace = Rc::new(RefCell::new(Vec::<String>::new()));
        let parsed = LayoutJson::read_arrangement("[[\"a\"]]").unwrap();
        let inner = parsed.rows.as_ref().unwrap().get(0).unwrap().unwrap();
        let values = ResourceObjectList::new(vec![inner]);
        let rows = match profile {
            "null" => None,
            "empty-array" => Some(ArrangementRows::default()),
            "list" => Some(values.clone().into()),
            _ => Some(ArrangementRows::from_custom(Rc::new(ProbeRows {
                values,
                count_fault: profile == "count-fault",
                enum_fault: profile == "enum-fault",
                trace: trace.clone(),
            }))),
        };
        let a = LayoutArrangement::new_nullable(rows.clone());
        let cached = a.rows.clone();
        let before = trace.borrow().clone();
        let count = match cached.as_ref().map(ArrangementRows::count).transpose() {
            Ok(v) => json!({"value":v}),
            Err(ResourceTextListError::NotSupported) => json!({"error":"NotSupportedException"}),
            Err(e) => panic!("{e:?}"),
        };
        let written = match LayoutJson::write_arrangement(&a, false) {
            Ok(v) => json!({"value":v}),
            Err(e) => {
                let category = match e.0.as_str() {
                    "Collection::NullReference" => "NullReferenceException",
                    "Collection::NotSupported" => "NotSupportedException",
                    _ => panic!("{e:?}"),
                };
                json!({"error":category})
            }
        };
        let same = match (&rows, &cached) {
            (None, None) => true,
            (Some(a), Some(b)) => a.ptr_eq(b),
            _ => false,
        };
        assert_eq!(
            json!({"kind":"arrangement","profile":profile,"same":same,"isNull":cached.is_none(),"before":before,"count":count,"written":written,"trace":*trace.borrow()}),
            row
        );
    }
}

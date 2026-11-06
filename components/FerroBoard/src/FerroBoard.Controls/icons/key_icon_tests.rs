//! Public typed factory calls against original C#; data containers are not native WinUI evidence.
use super::*;
use ferroui_base::media::{Colors, immutable::ImmutableSolidColorBrush};
use serde_json::{Value, json};
type Slot = (i32, Option<Option<Utf16Text>>, Option<Arc<[Command]>>);
struct CacheScope(Vec<Slot>);
impl CacheScope {
    fn pristine() -> Self {
        let mut ids = (0..68).collect::<Vec<_>>();
        ids.extend([i32::MIN, -1, 68, 99, i32::MAX, 930001]);
        let mut c = cache().lock().unwrap();
        Self(
            ids.into_iter()
                .map(|id| (id, c.custom.remove(&id), c.parsed.remove(&id)))
                .collect(),
        )
    }
}
impl Drop for CacheScope {
    fn drop(&mut self) {
        let mut c = cache()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        for (id, custom, parsed) in self.0.drain(..) {
            c.custom.remove(&id);
            c.parsed.remove(&id);
            if let Some(v) = custom {
                c.custom.insert(id, v);
            }
            if let Some(v) = parsed {
                c.parsed.insert(id, v);
            }
        }
    }
}
fn bits(v: f64) -> String {
    format!("{:016x}", v.to_bits())
}
fn p(v: Point) -> Value {
    json!([bits(v.x), bits(v.y)])
}
fn geometry(g: &Ref<Geometry>) -> Value {
    let g = g.clone().downcast::<PathGeometry>().unwrap();
    let figures=g.figures().unwrap().iter().map(|f|{let segments=f.segments().unwrap().iter().map(|s|{if let Some(l)=s.downcast_ref::<LineSegment>(){json!({"kind":"L","points":[p(l.point())]})}else{let b=s.downcast_ref::<BezierSegment>().unwrap();json!({"kind":"C","points":[p(b.point1()),p(b.point2()),p(b.point3())]})}}).collect::<Vec<_>>();json!({"start":p(f.start_point()),"closed":f.is_closed(),"filled":f.is_filled(),"segments":segments})}).collect::<Vec<_>>();
    json!({"fill":g.fill_rule()as i32,"figures":figures})
}
fn snapshot(icon: KeyIcon, size: f64) -> Result<Value, IconError> {
    if size < 0.0 || !size.is_finite() {
        let a = IconFactory::create_geometry(icon, size)?;
        let b = IconFactory::create_geometry(icon, size)?;
        let fresh_figures = a.clone().downcast::<PathGeometry>().unwrap().figures()
            != b.clone().downcast::<PathGeometry>().unwrap().figures();
        return Ok(
            json!({"value":{"geometry":geometry(&a),"fresh":a!=b,"freshFigures":fresh_figures,"again":geometry(&b)}}),
        );
    }
    let fill: Rc<dyn IBrush> = Rc::new(ImmutableSolidColorBrush::new(Colors::BLACK));
    let path = IconFactory::create(icon, size, Some(fill.clone()))?;
    let data = path.data().unwrap();
    let next = IconFactory::create_geometry(icon, size)?;
    let fresh_figures = data.clone().downcast::<PathGeometry>().unwrap().figures()
        != next.clone().downcast::<PathGeometry>().unwrap().figures();
    Ok(
        json!({"value":{"geometry":geometry(&data),"width":bits(path.width()),"height":bits(path.height()),"stretch":path.stretch()as i32,"hit":path.is_hit_test_visible(),"fillSame":Rc::ptr_eq(&fill,&path.fill().unwrap()),"fresh":data!=next,"freshFigures":fresh_figures,"again":geometry(&next)}}),
    )
}
fn result(icon: KeyIcon, size: f64) -> Value {
    match snapshot(icon, size) {
        Ok(v) => v,
        Err(e) => {
            json!({"error":match e{IconError::NullPath=>"NullReferenceException",IconError::MissingNumber{..}=>"IndexOutOfRangeException",IconError::InvalidNumber{..}=>"FormatException"}})
        }
    }
}
#[test]
fn original_typed_icon_registration_errors_worker_handoff_and_fresh_native_geometry() {
    crate::test_support::run(|| {
        let _scope = CacheScope::pristine();
        let mut count = 0;
        for line in include_str!("../../../tests/fixtures/key-icon.jsonl").lines() {
            let row: Value = serde_json::from_str(line).unwrap();
            let raw = row["raw"].as_i64().unwrap() as i32;
            let icon = KeyIcon(raw);
            let kind = row["kind"].as_str().unwrap();
            let mut actual = row.clone();
            if kind == "name" {
                actual["name"] = json!(icon.to_string());
            } else {
                let size = if kind == "worker" {
                    3.25
                } else {
                    f64::from_bits(u64::from_str_radix(row["size"].as_str().unwrap(), 16).unwrap())
                };
                if kind == "register" {
                    let path: Option<Vec<u16>> =
                        serde_json::from_value(row["path"].clone()).unwrap();
                    IconFactory::register_nullable(icon, path.map(Utf16Text::from_units));
                } else if kind == "worker" {
                    std::thread::spawn(move || IconFactory::register(icon, "M 2 3 L 4 5 Z"))
                        .join()
                        .unwrap();
                }
                actual["result"] = result(icon, size);
            }
            actual["has"] = json!(IconFactory::has_icon(icon));
            assert_eq!(actual, row, "{kind} {raw}");
            count += 1;
        }
        assert_eq!(count, 308);
    });
}

#[test]
fn icon_path_dimension_validation_reproducer_keeps_geometry_scope_distinct() {
    crate::test_support::run(|| {
        let icon = KeyIcon(930002);
        IconFactory::register(icon, "M 1 2 L 3 4");
        for size in [-1.0, f64::INFINITY] {
            assert!(IconFactory::create_geometry(icon, size).is_ok());
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                IconFactory::create(icon, size, None)
            }));
            match result {
                Err(payload) => {
                    let message = payload
                        .downcast_ref::<String>()
                        .map(String::as_str)
                        .or_else(|| payload.downcast_ref::<&str>().copied());
                    if !message.is_some_and(|m| m.contains("not valid for property 'Width'")) {
                        std::panic::resume_unwind(payload);
                    }
                }
                Ok(_) => panic!(
                    "Pinned FerroUI width constraint changed; inspect actual native source and update the reproducer"
                ),
            }
        }
    });
}

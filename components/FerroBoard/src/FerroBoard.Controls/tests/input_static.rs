#[path = "support/headless.rs"]
mod headless;
#[path = "../../../tests/support/input_static_values.rs"]
mod values;
use ferroboard_controls::input::{InputOverrideError, ScreenKeyboardInput as Input};
use ferroui_base::{BoxedValue, FerroObject, Ref, media::SolidColorBrush};
use ferroui_controls::TextBox;
use serde_json::{Value, json};
use std::rc::Rc;
use values::*;
fn get(n: &str, e: Option<&FerroObject>) -> Result<Option<BoxedValue>, InputOverrideError> {
    match n {
        "InputKind" => Input::get_input_kind_nullable(e),
        "EnterAction" => Input::get_enter_action_nullable(e),
        "Capitalization" => Input::get_capitalization_nullable(e),
        "SuggestionsEnabled" => {
            Input::get_suggestions_enabled_nullable(e).map(|v| Some(Rc::new(v) as BoxedValue))
        }
        "IsPrivate" => Input::get_is_private_nullable(e).map(|v| Some(Rc::new(v) as BoxedValue)),
        "IsEnabled" => Input::get_is_enabled_nullable(e).map(|v| Some(Rc::new(v) as BoxedValue)),
        _ => panic!("name"),
    }
}
fn read(n: &str, e: Option<&FerroObject>, v: &Option<BoxedValue>) -> Value {
    match get(n, e) {
        Ok(value) => output(n, value, v),
        Err(InputOverrideError::NullReference) => json!({"error":"NullReferenceException"}),
        Err(e) => panic!("{e}"),
    }
}
fn write(n: &str, e: Option<&FerroObject>, v: Option<BoxedValue>) -> Value {
    let boolean = || *v.as_ref().unwrap().downcast_ref::<bool>().unwrap();
    let result = match n {
        "InputKind" => Input::set_input_kind_nullable(e, v.clone()),
        "EnterAction" => Input::set_enter_action_nullable(e, v.clone()),
        "Capitalization" => Input::set_capitalization_nullable(e, v.clone()),
        "SuggestionsEnabled" => Input::set_suggestions_enabled_nullable(e, boolean()),
        "IsPrivate" => Input::set_is_private_nullable(e, boolean()),
        "IsEnabled" => Input::set_is_enabled_nullable(e, boolean()),
        _ => panic!("name"),
    };
    match result {
        Ok(()) => read(n, e, &v),
        Err(InputOverrideError::NullReference) => json!({"error":"NullReferenceException"}),
        Err(e) => panic!("{e}"),
    }
}
fn owner(kind: u8) -> Ref<FerroObject> {
    match kind {
        0 => FerroObject::new(),
        1 => TextBox::new().upcast(),
        2 => SolidColorBrush::new().upcast(),
        _ => panic!("kind"),
    }
}
#[test]
fn actual_native_static_helpers_retain_arbitrary_objects_and_support_property_root_owners() {
    headless::run(|| {
        for kind in 0..3 {
            for line in include_str!("../../../tests/fixtures/input-static.jsonl").lines() {
                let row: Value = serde_json::from_str(line).unwrap();
                if row["tag"] != "roundtrip" {
                    continue;
                }
                let object = owner(kind);
                let e = if row["exists"] == true {
                    Some(&*object)
                } else {
                    None
                };
                let n = row["name"].as_str().unwrap();
                assert_eq!(read(n, e, &None), row["initial"], "owner {kind}: {row}");
                assert_eq!(
                    write(n, e, boxed(&row["input"])),
                    row["first"],
                    "owner {kind}: {row}"
                );
                assert_eq!(
                    write(n, e, boxed(&row["input"])),
                    row["second"],
                    "owner {kind}: {row}"
                );
                let all = e.map(|e| NAMES.map(|n| describe(get(n, Some(e)).unwrap().as_ref())));
                assert_eq!(json!(all), row["all"], "owner {kind}: {row}");
            }
        }
    });
}

// A dependency reproducer, not a passing cross-thread-parity test. Only leaked-definition
// addresses cross the thread boundary; no non-Sync property reference is sent or dereferenced.
#[test]
fn pinned_framework_static_property_identity_is_per_thread_unlike_original_csharp_fields() {
    headless::run(|| {
        fn addresses() -> [usize; 6] {
            let fields: [&'static ferroui_base::FerroProperty; 6] = [
                Input::input_kind_property(),
                Input::enter_action_property(),
                Input::capitalization_property(),
                Input::suggestions_enabled_property(),
                Input::is_private_property(),
                Input::is_enabled_property(),
            ];
            fields.map(|v| v as *const ferroui_base::FerroProperty as usize)
        }
        let owner = addresses();
        let worker = std::thread::spawn(addresses).join().unwrap();
        let rows = include_str!("../../../tests/fixtures/input-static.jsonl")
            .lines()
            .map(|v| serde_json::from_str::<Value>(v).unwrap())
            .filter(|v| v["tag"] == "worker")
            .collect::<Vec<_>>();
        assert_eq!(rows.len(), 6);
        for (i, row) in rows.iter().enumerate() {
            assert_eq!(row["name"], format!("{}Property", NAMES[i]));
            assert_eq!(row["same"], true);
            assert_ne!(
                owner[i], worker[i],
                "Selected framework now preserves shared property identity; re-audit the dependency limitation"
            );
        }
    });
}

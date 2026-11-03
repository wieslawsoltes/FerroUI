//! Unchanged native-locale cases from the C# evaluator corpus; no virtual BCL objects.
use ferroboard_core::{keys::*, layouts::*, text::*};
use serde_json::{Value, json};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
type Trace = Rc<RefCell<Vec<String>>>;
type Action = Rc<dyn Fn()>;
ferroui_system::ferro_thread_local! {static AMBIENT:Cell<TextCulture>=Cell::new(TextCulture::invariant());}
fn ambient() -> Result<String, locale_text::CollationError> {
    Ok(AMBIENT.with(|v| v.get().name().to_owned()))
}
struct Native;
impl Native {
    fn make_culture(kind: &str) -> Option<TextCulture> {
        match kind {
            "null" => None,
            "invariant" => Some(TextCulture::invariant()),
            "turkish" => Some(TextCulture::get("tr-TR").unwrap()),
            _ => panic!("excluded virtual culture: {kind}"),
        }
    }
    fn set_ambient(value: TextCulture) -> TextCulture {
        AMBIENT.with(|slot| slot.replace(value))
    }
    fn restore_ambient(previous: TextCulture) {
        AMBIENT.with(|slot| slot.set(previous));
    }
}
struct Evaluator {
    shift: ShiftState,
    trace: Trace,
    value: Option<TextCulture>,
    action: Action,
}
impl IKeyEvaluator for Evaluator {
    fn mode(&self) -> KeyboardMode {
        KeyboardMode::Characters
    }
    fn shift_state(&self) -> ShiftState {
        self.trace.borrow_mut().push("shift".into());
        self.shift
    }
    fn variation(&self) -> KeyVariation {
        KeyVariation::Normal
    }
    fn direction(&self) -> LayoutDirection {
        LayoutDirection::Ltr
    }
    fn is_char_half_width(&self) -> bool {
        false
    }
    fn is_kana_kata(&self) -> bool {
        false
    }
    fn culture(&self) -> Option<TextCulture> {
        self.trace.borrow_mut().push("culture".into());
        (self.action)();
        self.value
    }
}
fn capture(
    action: impl FnOnce() -> Result<Option<TextKey>, KeyDataError>,
) -> (Option<TextKey>, Option<&'static str>, Option<&'static str>) {
    match action() {
        Ok(v) => (v, None, None),
        Err(KeyDataError::NullReference) => (None, Some("NullReferenceException"), None),
        Err(e) => panic!("unexpected native casing error: {e}"),
    }
}
fn labels() -> Vec<Option<Utf16Text>> {
    vec![
        None,
        Some("".into()),
        Some("a".into()),
        Some("A".into()),
        Some("i".into()),
        Some("ß".into()),
        Some("a\u{301}".into()),
        Some(Utf16Text::from_units(vec![0xd800])),
        Some(Utf16Text::from_units(vec![0xdfff])),
        Some("😀".into()),
        Some(Utf16Text::from_units(vec![0xd800, 97])),
        Some("\r\n".into()),
    ]
}
fn units(text: Option<Utf16Text>) -> Value {
    text.map_or(Value::Null, |v| json!(v.units()))
}
fn auto(row: &Value) -> Value {
    let trace: Trace = Rc::default();
    let original_popup = Rc::new(PopupSet::default());
    let replacement = Rc::new(PopupSet::default());
    let label = row["label"].as_u64().unwrap() as usize;
    let key = Rc::new(AutoTextKeyData {
        base: TextKeyData::from_fields(TextKeyDataFields {
            code: row["code"].as_i64().unwrap() as i32,
            label: labels()[label].clone(),
            group_id: 3,
            popup: Some(original_popup.clone()),
            ..Default::default()
        }),
    });
    let mutation = |name: &str| -> Action {
        let weak = Rc::downgrade(&key);
        let trace = trace.clone();
        let replacement = replacement.clone();
        let name = name.to_string();
        Rc::new(move || {
            if name == "none" {
                return;
            }
            trace.borrow_mut().push(format!("mutate:{name}"));
            let key = weak.upgrade().unwrap();
            match name.as_str() {
                "label-null" => key.base.set_label(None),
                "label-upper" => key.base.set_label(Some("A".into())),
                "fields" => {
                    key.base.set_type(KeyType(17));
                    key.base.set_code(32);
                    key.base.set_group_id(97);
                    key.base.set_popup(Some(replacement.clone()));
                }
                _ => panic!("unknown mutation {name}"),
            }
        })
    };
    let culture = Native::make_culture(row["kind"].as_str().unwrap());
    let current = Native::make_culture(row["ambient"].as_str().unwrap()).unwrap();
    let previous = Native::set_ambient(current);
    let evaluator = Rc::new(Evaluator {
        shift: ShiftState(row["shift"].as_i64().unwrap() as i32),
        trace: trace.clone(),
        value: culture,
        action: mutation(row["evaluatorMutation"].as_str().unwrap()),
    });
    let (result, error, param) = capture(|| key.clone().compute(evaluator.as_ref()));
    let state = |data: &TextKeyData| json!({"code":data.code(),"label":units(data.label()),"type":data.key_type().0,"group":data.group_id(),"popup":if data.popup().as_ref().is_some_and(|p|Rc::ptr_eq(p,&original_popup)){"original"}else if data.popup().as_ref().is_some_and(|p|Rc::ptr_eq(p,&replacement)){"replacement"}else{"other"}});
    let same = result
        .as_ref()
        .is_some_and(|value| std::ptr::addr_eq(Rc::as_ptr(value), Rc::as_ptr(&key)));
    let actual = json!({"tag":"auto","label":label,"shift":row["shift"],"code":row["code"],"kind":row["kind"],"ambient":row["ambient"],"evaluatorMutation":row["evaluatorMutation"],"infoMutation":row["infoMutation"],"error":error,"param":param,"trace":*trace.borrow(),"result":result.as_ref().map(|k|state(k.data())),"same":same,"source":state(&key.base),"multi":result.is_some_and(|k|k.is_multi_text())});
    Native::restore_ambient(previous);
    actual
}
fn context(row: &Value) -> Value {
    let trace: Trace = Rc::default();
    let culture = Native::make_culture(row["kind"].as_str().unwrap());
    let previous = Native::set_ambient(Native::make_culture("invariant").unwrap());
    let context = Rc::new(KeyComputeContext::from_values(
        ferroboard_core::layouts::KeyComputeContextValues {
            culture,
            shift_state: ShiftState::ShiftedManual,
            ..Default::default()
        },
    ));
    let alias = context.clone();
    let copy = context.unshifted();
    let key = Rc::new(AutoTextKeyData {
        base: TextKeyData::from_fields(TextKeyDataFields {
            code: 97,
            label: labels()[row["label"].as_u64().unwrap() as usize].clone(),
            ..Default::default()
        }),
    });
    let (result, error, param) = capture(|| key.clone().compute(context.as_ref()));
    let same_culture = |a: Option<TextCulture>, b: Option<TextCulture>| match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => a == b,
        _ => false,
    };
    let actual = json!({"tag":"context","kind":row["kind"],"label":row["label"],"error":error,"param":param,"trace":*trace.borrow(),"result":units(result.as_ref().and_then(|k|k.data().label())),"same":result.is_some_and(|k|std::ptr::addr_eq(Rc::as_ptr(&k),Rc::as_ptr(&key))),"cultureSame":same_culture(context.culture(),culture),"cloneCultureSame":same_culture(copy.culture(),culture),"aliasSame":Rc::ptr_eq(&context,&alias),"cloneSame":false,"cloneShift":copy.shift_state().0});
    Native::restore_ambient(previous);
    actual
}

pub fn verify(fixture: &str) -> usize {
    locale_text::install_locale_provider(ambient).unwrap();
    let native = |kind: &str| matches!(kind, "null" | "invariant" | "turkish");
    let mut count = 0;
    for line in fixture.lines() {
        let row: Value = serde_json::from_str(line).unwrap();
        let actual = match row["tag"].as_str().unwrap() {
            "auto"
                if native(row["kind"].as_str().unwrap())
                    && native(row["ambient"].as_str().unwrap()) =>
            {
                auto(&row)
            }
            "context" if native(row["kind"].as_str().unwrap()) => context(&row),
            "literal" => {
                let text: Utf16Text = "ß".into();
                let a = AutoTextKeyData::to_upper_with_culture(
                    Some(&text),
                    Some(&TextCulture::get("tr").unwrap()),
                )
                .unwrap();
                let b = AutoTextKeyData::to_upper_with_culture(Some(&text), None).unwrap();
                json!({"tag":"literal","value":a.units(),"same":a.ptr_eq(&b),"trace":[]})
            }
            _ => continue,
        };
        assert_eq!(actual, row, "{line}");
        count += 1;
    }
    count
}

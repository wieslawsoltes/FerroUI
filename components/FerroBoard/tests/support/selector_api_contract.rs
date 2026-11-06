//! Unchanged-source selector/evaluator oracle shared by Core and independent native consumers.
use ferroboard_core::{
    keys::*,
    layouts::*,
    text::{TextCulture, Utf16Text},
};
use serde_json::{Value, json};
use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};
pub trait Driver {
    type Handle: Clone + 'static;
    fn construct(name: &str) -> Self::Handle;
    fn key(handle: &Self::Handle) -> Key;
    fn get(handle: &Self::Handle, name: &str) -> Option<Key>;
    fn set(handle: &Self::Handle, name: &str, value: Option<Key>);
}
pub struct Core;
#[derive(Clone)]
pub enum CoreHandle {
    Case(Rc<CaseSelector>),
    ShiftState(Rc<ShiftStateSelector>),
    Variation(Rc<VariationSelector>),
    Direction(Rc<LayoutDirectionSelector>),
    Width(Rc<CharWidthSelector>),
    Kana(Rc<KanaSelector>),
}
impl Driver for Core {
    type Handle = CoreHandle;
    fn construct(name: &str) -> Self::Handle {
        match name {
            "CaseSelector" => CoreHandle::Case(Rc::default()),
            "ShiftStateSelector" => CoreHandle::ShiftState(Rc::default()),
            "VariationSelector" => CoreHandle::Variation(Rc::default()),
            "LayoutDirectionSelector" => CoreHandle::Direction(Rc::default()),
            "CharWidthSelector" => CoreHandle::Width(Rc::default()),
            "KanaSelector" => CoreHandle::Kana(Rc::default()),
            _ => panic!("unknown selector {name}"),
        }
    }
    fn key(handle: &Self::Handle) -> Key {
        match handle {
            CoreHandle::Case(k) => k.clone(),
            CoreHandle::ShiftState(k) => k.clone(),
            CoreHandle::Variation(k) => k.clone(),
            CoreHandle::Direction(k) => k.clone(),
            CoreHandle::Width(k) => k.clone(),
            CoreHandle::Kana(k) => k.clone(),
        }
    }
    fn get(handle: &Self::Handle, name: &str) -> Option<Key> {
        match handle {
            CoreHandle::Case(k) => match name {
                "Lower" => k.lower(),
                "Upper" => k.upper(),
                _ => panic!("unknown arm {name}"),
            },
            CoreHandle::ShiftState(k) => match name {
                "Unshifted" => k.unshifted(),
                "Shifted" => k.shifted(),
                "ShiftedManual" => k.shifted_manual(),
                "ShiftedAutomatic" => k.shifted_automatic(),
                "CapsLock" => k.caps_lock(),
                "Default" => k.default_key(),
                _ => panic!("unknown arm {name}"),
            },
            CoreHandle::Variation(k) => match name {
                "Default" => k.default_key(),
                "Email" => k.email(),
                "Uri" => k.uri(),
                "Normal" => k.normal(),
                "Password" => k.password(),
                _ => panic!("unknown arm {name}"),
            },
            CoreHandle::Direction(k) => match name {
                "Ltr" => k.ltr(),
                "Rtl" => k.rtl(),
                _ => panic!("unknown arm {name}"),
            },
            CoreHandle::Width(k) => match name {
                "Full" => k.full(),
                "Half" => k.half(),
                _ => panic!("unknown arm {name}"),
            },
            CoreHandle::Kana(k) => match name {
                "Hira" => k.hira(),
                "Kata" => k.kata(),
                _ => panic!("unknown arm {name}"),
            },
        }
    }
    fn set(handle: &Self::Handle, name: &str, value: Option<Key>) {
        match handle {
            CoreHandle::Case(k) => match name {
                "Lower" => k.set_lower(value),
                "Upper" => k.set_upper(value),
                _ => panic!("unknown arm {name}"),
            },
            CoreHandle::ShiftState(k) => match name {
                "Unshifted" => k.set_unshifted(value),
                "Shifted" => k.set_shifted(value),
                "ShiftedManual" => k.set_shifted_manual(value),
                "ShiftedAutomatic" => k.set_shifted_automatic(value),
                "CapsLock" => k.set_caps_lock(value),
                "Default" => k.set_default(value),
                _ => panic!("unknown arm {name}"),
            },
            CoreHandle::Variation(k) => match name {
                "Default" => k.set_default(value),
                "Email" => k.set_email(value),
                "Uri" => k.set_uri(value),
                "Normal" => k.set_normal(value),
                "Password" => k.set_password(value),
                _ => panic!("unknown arm {name}"),
            },
            CoreHandle::Direction(k) => match name {
                "Ltr" => k.set_ltr(value),
                "Rtl" => k.set_rtl(value),
                _ => panic!("unknown arm {name}"),
            },
            CoreHandle::Width(k) => match name {
                "Full" => k.set_full(value),
                "Half" => k.set_half(value),
                _ => panic!("unknown arm {name}"),
            },
            CoreHandle::Kana(k) => match name {
                "Hira" => k.set_hira(value),
                "Kata" => k.set_kata(value),
                _ => panic!("unknown arm {name}"),
            },
        }
    }
}
pub fn fields(name: &str) -> &'static [&'static str] {
    match name {
        "CaseSelector" => &["Lower", "Upper"],
        "ShiftStateSelector" => &[
            "Unshifted",
            "Shifted",
            "ShiftedManual",
            "ShiftedAutomatic",
            "CapsLock",
            "Default",
        ],
        "VariationSelector" => &["Default", "Email", "Uri", "Normal", "Password"],
        "LayoutDirectionSelector" => &["Ltr", "Rtl"],
        "CharWidthSelector" => &["Full", "Half"],
        "KanaSelector" => &["Hira", "Kata"],
        _ => panic!("unknown selector {name}"),
    }
}
type Trace = Rc<RefCell<Vec<String>>>;
struct Evaluator {
    raw: i32,
    trace: Trace,
    action: Rc<dyn Fn()>,
}
impl Evaluator {
    fn read<T>(&self, name: &str, value: T) -> T {
        self.trace.borrow_mut().push(name.into());
        (self.action)();
        value
    }
}
impl IKeyEvaluator for Evaluator {
    fn mode(&self) -> KeyboardMode {
        self.read("mode", KeyboardMode(self.raw))
    }
    fn shift_state(&self) -> ShiftState {
        self.read("shift", ShiftState(self.raw))
    }
    fn variation(&self) -> KeyVariation {
        self.read("variation", KeyVariation(self.raw))
    }
    fn direction(&self) -> LayoutDirection {
        self.read("direction", LayoutDirection(self.raw))
    }
    fn is_char_half_width(&self) -> bool {
        self.read("half", self.raw != 0)
    }
    fn is_kana_kata(&self) -> bool {
        self.read("kata", self.raw != 0)
    }
    fn culture(&self) -> Option<TextCulture> {
        self.read("culture", Some(TextCulture::invariant()))
    }
}
struct Probe {
    base: TextKeyData,
    index: usize,
    trace: Trace,
    expected: Rc<RefCell<Weak<Evaluator>>>,
    action: String,
    clear: Rc<dyn Fn()>,
}
impl AbstractKeyData for Probe {
    fn compute_nullable(
        self: Rc<Self>,
        e: Option<&dyn IKeyEvaluator>,
    ) -> Result<Option<TextKey>, KeyDataError> {
        let expected = self.expected.borrow().upgrade();
        let same = match (e, expected.as_ref()) {
            (None, None) => true,
            (Some(a), Some(b)) => std::ptr::addr_eq(a, Rc::as_ptr(b) as *const dyn IKeyEvaluator),
            _ => false,
        };
        self.trace.borrow_mut().push(format!(
            "child:{}:{}",
            self.index,
            if same { "same" } else { "other" }
        ));
        if self.action == "clear" {
            (self.clear)();
        }
        if self.action == "fail" {
            return Err(KeyDataError::NullReference);
        }
        if self.action == "hide" {
            Ok(None)
        } else {
            Ok(Some(self))
        }
    }
    fn as_string(&self, _: bool) -> Result<Option<Utf16Text>, KeyDataError> {
        Ok(Some("probe".into()))
    }
}
impl ITextKeyData for Probe {
    fn data(&self) -> &TextKeyData {
        &self.base
    }
}
fn code_key(code: i32) -> TextKey {
    Rc::new(TextKeyData::from_fields(TextKeyDataFields {
        code,
        ..Default::default()
    }))
}
fn identity(key: Option<&Key>, children: &[Key], replacements: &[Key]) -> Value {
    let Some(key) = key else { return Value::Null };
    for (prefix, keys) in [("child", children), ("replacement", replacements)] {
        if let Some(i) = keys.iter().position(|k| Rc::ptr_eq(k, key)) {
            return json!(format!("{prefix}:{i}"));
        }
    }
    json!("other")
}
fn error_name(error: KeyDataError) -> &'static str {
    match error {
        KeyDataError::NullReference => "NullReferenceException",
        other => panic!("unexpected {other:?}"),
    }
}
fn compute<D: Driver>(row: &Value) -> Value {
    let name = row["type"].as_str().unwrap();
    let fields = fields(name);
    let handle = D::construct(name);
    let alias = handle.clone();
    let owner = Rc::new(RefCell::new(Some(handle.clone())));
    let trace: Trace = Rc::default();
    let expected = Rc::new(RefCell::new(Weak::new()));
    let replacements: Vec<Key> = (0..fields.len())
        .map(|i| code_key(300 + i as i32) as Key)
        .collect();
    let mutation = |kind: &str| -> Rc<dyn Fn()> {
        let owner = Rc::downgrade(&owner);
        let trace = trace.clone();
        let replacements = replacements.clone();
        let kind = kind.to_string();
        Rc::new(move || {
            if kind == "none" {
                return;
            }
            trace.borrow_mut().push(format!("mutate:{kind}"));
            let owner = owner.upgrade().unwrap();
            let handle = owner.borrow().as_ref().unwrap().clone();
            for (i, name) in fields.iter().enumerate() {
                D::set(
                    &handle,
                    name,
                    if kind == "clear" {
                        None
                    } else {
                        Some(replacements[i].clone())
                    },
                );
            }
        })
    };
    let mask = row["mask"].as_u64().unwrap();
    let children: Vec<Key> = (0..fields.len())
        .map(|index| {
            Rc::new(Probe {
                base: TextKeyData::from_fields(TextKeyDataFields {
                    code: 100 + index as i32,
                    ..Default::default()
                }),
                index,
                trace: trace.clone(),
                expected: expected.clone(),
                action: row["child"].as_str().unwrap().into(),
                clear: mutation("clear"),
            }) as Key
        })
        .collect();
    for (i, name) in fields.iter().enumerate() {
        D::set(
            &handle,
            name,
            if mask & (1 << i) != 0 {
                Some(children[i].clone())
            } else {
                None
            },
        );
    }
    let kind = row["evaluator"].as_str().unwrap();
    let evaluator = (kind != "null").then(|| {
        Rc::new(Evaluator {
            raw: row["raw"].as_i64().unwrap() as i32,
            trace: trace.clone(),
            action: mutation(if matches!(kind, "clear" | "replace") {
                kind
            } else {
                "none"
            }),
        })
    });
    *expected.borrow_mut() = evaluator.as_ref().map(Rc::downgrade).unwrap_or_default();
    let original = D::key(&handle);
    let result = original
        .clone()
        .compute_nullable(evaluator.as_deref().map(|e| e as &dyn IKeyEvaluator));
    let (key, error) = match result {
        Ok(k) => (k, None),
        Err(e) => (None, Some(error_name(e))),
    };
    let output = key.as_ref().map(|k| k.clone() as Key);
    let actual = json!({"tag":"compute","type":name,"mask":mask,"raw":row["raw"],"evaluator":kind,"child":row["child"],"error":error,"code":key.map(|k|k.data().code()),"identity":identity(output.as_ref(),&children,&replacements),"trace":*trace.borrow(),"arms":fields.iter().map(|n|identity(D::get(&alias,n).as_ref(),&children,&replacements)).collect::<Vec<_>>(),"aliasSame":Rc::ptr_eq(&original,&D::key(&alias)),"text":original.as_string(false).unwrap().unwrap().to_utf8().unwrap(),"display":original.as_string(true).unwrap().unwrap().to_utf8().unwrap()});
    owner.borrow_mut().take();
    actual
}
fn nullable(row: &Value) -> Value {
    let name = row["type"].as_str().unwrap();
    let is_null = row["nullable"].as_bool().unwrap();
    let trace: Trace = Rc::default();
    let eval = Rc::new(Evaluator {
        raw: 0,
        trace: trace.clone(),
        action: Rc::new(|| {}),
    });
    let key: Key = match name {
        "text" => Rc::new(TextKeyData::default()),
        "multi" => Rc::new(MultiTextKeyData::default()),
        "auto-character" | "auto-function" => Rc::new(AutoTextKeyData {
            base: TextKeyData::from_fields(TextKeyDataFields {
                code: if name == "auto-character" { 97 } else { -7 },
                label: Some(if name == "auto-character" { "a" } else { "" }.into()),
                ..Default::default()
            }),
        }),
        "custom" => Rc::new(Probe {
            base: TextKeyData::from_fields(TextKeyDataFields {
                code: 100,
                ..Default::default()
            }),
            index: 0,
            trace: trace.clone(),
            expected: Rc::new(RefCell::new(if is_null {
                Weak::new()
            } else {
                Rc::downgrade(&eval)
            })),
            action: "return".into(),
            clear: Rc::new(|| {}),
        }),
        _ => panic!("unknown nullable key {name}"),
    };
    let result = key
        .clone()
        .compute_nullable(if is_null { None } else { Some(eval.as_ref()) });
    let (output, error) = match result {
        Ok(v) => (v, None),
        Err(e) => (None, Some(error_name(e))),
    };
    json!({"tag":"nullable","type":name,"nullable":is_null,"error":error,"code":output.as_ref().map(|k|k.data().code()),"same":output.as_ref().is_some_and(|k|Rc::ptr_eq(&(k.clone() as Key),&key)),"trace":*trace.borrow()})
}
fn json_row<D: Driver>(row: &Value) -> Value {
    let name = row["type"].as_str().unwrap();
    let mask = row["mask"].as_u64().unwrap();
    let handle = D::construct(name);
    for (i, arm) in fields(name).iter().enumerate() {
        D::set(
            &handle,
            arm,
            if mask & (1 << i) == 0 {
                None
            } else {
                Some(code_key(100 + i as i32) as Key)
            },
        );
    }
    let text = LayoutJson::write_arrangement(
        &LayoutArrangement::from_rows(vec![vec![D::key(&handle)]]),
        false,
    )
    .unwrap();
    json!({"tag":"json","type":name,"mask":mask,"value":serde_json::from_str::<Value>(&text).unwrap()})
}
pub fn verify<D: Driver>(fixture: &str) -> (usize, usize) {
    let (mut behavior, mut facts) = (0, 0);
    for line in fixture.lines() {
        let row: Value = serde_json::from_str(line).unwrap();
        match row["tag"].as_str().unwrap() {
            "compute" => {
                assert_eq!(compute::<D>(&row), row, "{line}");
                behavior += 1;
            }
            "json" => {
                assert_eq!(json_row::<D>(&row), row, "{line}");
                behavior += 1;
            }
            "nullable" => {
                assert_eq!(nullable(&row), row, "{line}");
                behavior += 1;
            }
            "property" | "class" => facts += 1,
            other => panic!("unknown tag {other}"),
        }
    }
    (behavior, facts)
}

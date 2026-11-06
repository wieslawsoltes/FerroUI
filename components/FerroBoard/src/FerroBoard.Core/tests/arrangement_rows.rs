//! Public original nested layout list dispatch, nulls, merge/writer disposal and identity.
use ferroboard_core::{keys::*, layouts::*};
use serde_json::{Value, json};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
};
type Result<T> = std::result::Result<T, ResourceTextListError>;
const MODES: [&str; 16] = [
    "none",
    "count-clear",
    "count-add",
    "count-negative",
    "count-throw",
    "get-clear",
    "get-null",
    "get-throw",
    "enum-factory-throw",
    "enum-move-throw",
    "enum-current-throw",
    "enum-dispose-throw",
    "enum-move-clear",
    "enum-current-clear",
    "enum-dispose-clear",
    "enum-double-throw",
];
fn error(e: ResourceTextListError) -> &'static str {
    match e {
        ResourceTextListError::IndexOutOfRange => "ArgumentOutOfRangeException",
        ResourceTextListError::Modified => "InvalidOperationException",
        ResourceTextListError::NotSupported => "NotSupportedException",
        ResourceTextListError::NullReference => "NullReferenceException",
        ResourceTextListError::ArgumentNull => "ArgumentNullException",
        ResourceTextListError::KeyNotFound => "KeyNotFoundException",
        ResourceTextListError::DuplicateKey => "ArgumentException",
    }
}
fn resource_error(e: ResourceError) -> &'static str {
    if let ResourceError::Collection(e) = e {
        error(e)
    } else {
        panic!("unexpected {e:?}")
    }
}
fn json_error(e: LayoutJsonError) -> &'static str {
    match e.0.as_str() {
        "Unsupported key serialization" => "NotSupportedException",
        "Collection::IndexOutOfRange" => "ArgumentOutOfRangeException",
        "Collection::Modified" => "InvalidOperationException",
        "Collection::NotSupported" => "NotSupportedException",
        "Collection::NullReference" => "NullReferenceException",
        "Collection::ArgumentNull" => "ArgumentNullException",
        _ => panic!("unexpected {e:?}"),
    }
}
struct State {
    mode: &'static str,
    trace: RefCell<Vec<String>>,
    fired: Cell<bool>,
    remaining: RefCell<Option<Rc<dyn Fn() -> i32>>>,
}
impl State {
    fn new(mode: &'static str) -> Rc<Self> {
        Rc::new(Self {
            mode,
            trace: RefCell::new(vec![]),
            fired: Cell::new(false),
            remaining: RefCell::new(None),
        })
    }
    fn step<T: Clone + 'static>(
        &self,
        step: &str,
        values: &ResourceObjectList<Option<T>>,
        additional: &dyn Fn() -> T,
    ) -> Result<()> {
        self.trace.borrow_mut().push(step.into());
        if self.mode == format!("{step}-throw")
            || self.mode == "enum-double-throw" && step == "enum-move"
        {
            return Err(ResourceTextListError::IndexOutOfRange);
        }
        if self.mode == "enum-double-throw" && step == "enum-dispose" {
            return Err(ResourceTextListError::NotSupported);
        }
        if !self.fired.get()
            && (self.mode == format!("{step}-clear") || self.mode == format!("{step}-add"))
        {
            self.fired.set(true);
            if self.mode.ends_with("clear") {
                values.clear();
            } else {
                values.push(Some(additional()));
            }
        }
        Ok(())
    }
    fn remaining(&self) -> i32 {
        self.remaining.borrow().as_ref().map_or(-1, |f| f())
    }
}
struct Probe<T: Clone + 'static> {
    values: ResourceObjectList<Option<T>>,
    additional: Rc<dyn Fn() -> T>,
    state: Rc<State>,
}
impl<T: Clone + 'static> Probe<T> {
    fn wrap(
        values: Vec<T>,
        additional: impl Fn() -> T + 'static,
        state: Rc<State>,
    ) -> ResourceReadOnlyList<T> {
        let values = ResourceObjectList::new(values.into_iter().map(Some).collect());
        let retained = values.clone();
        state
            .remaining
            .replace(Some(Rc::new(move || retained.len() as i32)));
        ResourceReadOnlyList::from_custom(Rc::new(Self {
            values,
            additional: Rc::new(additional),
            state,
        }))
    }
}
impl<T: Clone + 'static> IResourceReadOnlyList<T> for Probe<T> {
    fn count(&self) -> Result<i32> {
        let count = self.values.len() as i32;
        self.state
            .step("count", &self.values, self.additional.as_ref())?;
        Ok(if self.state.mode == "count-negative" {
            -1
        } else {
            count
        })
    }
    fn get(&self, index: i32) -> Result<Option<T>> {
        self.state
            .step("get", &self.values, self.additional.as_ref())?;
        if self.state.mode == "get-null" {
            Ok(None)
        } else {
            self.values
                .get(usize::try_from(index).map_err(|_| ResourceTextListError::IndexOutOfRange)?)
        }
    }
    fn iter(&self) -> Result<ResourceItems<T>> {
        self.state
            .step("enum-factory", &self.values, self.additional.as_ref())?;
        Ok(Box::new(self.values.iter().map(Ok)))
    }
}
fn key(s: &str) -> Key {
    TextKeyData::char(s.into(), None).unwrap()
}
fn row(s: &str) -> ArrangementRow {
    ResourceObjectList::new(vec![key(s)]).into()
}
fn outer(rows: Vec<ArrangementRow>) -> ArrangementRows {
    ResourceObjectList::new(rows).into()
}
fn list_layout(
    layout_type: LayoutType,
    id: &str,
    rows: Option<ArrangementRows>,
) -> Rc<KeyboardLayout> {
    Rc::new(KeyboardLayout::new(
        layout_type,
        "probe".into(),
        id.into(),
        Rc::new(LayoutArrangement::new_nullable(rows)),
    ))
}
fn run(placement: &str, mode: &'static str, operation: &str) -> Value {
    let state = State::new(mode);
    let resources = Rc::new(KeyboardResources::create_empty());
    let main_base = vec![row("a"), row("b")];
    let mod_first: ArrangementRow = ResourceObjectList::new(vec![
        key("m"),
        Rc::new(TextKeyData::default()) as Key,
        key("n"),
    ])
    .into();
    let mod_base = vec![mod_first.clone(), row("c")];
    let mut main_rows = Some(outer(main_base.clone()));
    let mut mod_rows = Some(outer(mod_base.clone()));
    let mut ext_rows = Some(outer(vec![row("e")]));
    match placement {
        "main-outer" => main_rows = Some(Probe::wrap(main_base, || row("z"), state.clone())),
        "main-inner" => {
            main_rows = Some(outer(vec![
                Probe::wrap(vec![key("x"), key("y")], || key("z"), state.clone()),
                row("b"),
            ]))
        }
        "modifier-outer" => mod_rows = Some(Probe::wrap(mod_base, || row("z"), state.clone())),
        "modifier-inner" => {
            mod_rows = Some(outer(vec![
                Probe::wrap(
                    mod_first.as_concrete().unwrap().snapshot(),
                    || key("z"),
                    state.clone(),
                ),
                row("c"),
            ]))
        }
        "extension-outer" => {
            ext_rows = Some(Probe::wrap(vec![row("e")], || row("z"), state.clone()))
        }
        "extension-inner" => {
            ext_rows = Some(outer(vec![Probe::wrap(
                vec![key("e")],
                || key("z"),
                state.clone(),
            )]))
        }
        "hint-outer" | "hint-inner" => {}
        "null-hint-empty" => {
            main_rows = Some(outer(vec![ResourceObjectList::<Key>::new(vec![]).into()]));
            mod_rows = None;
        }
        "null-main" => main_rows = None,
        "null-modifier" => mod_rows = None,
        "null-extension" => ext_rows = None,
        _ => unreachable!(),
    }
    let engine_mode = if placement.contains("hint") {
        KeyboardMode::Characters
    } else if placement.starts_with("main") || placement == "null-main" {
        KeyboardMode::Numeric
    } else if placement.starts_with("extension") || placement == "null-extension" {
        KeyboardMode::Symbols
    } else {
        KeyboardMode::Symbols2
    };
    let main_type = match engine_mode {
        KeyboardMode::Characters => LayoutType::Characters,
        KeyboardMode::Numeric => LayoutType::Numeric,
        KeyboardMode::Symbols => LayoutType::Symbols,
        _ => LayoutType::Symbols2,
    };
    let mod_type = if engine_mode == KeyboardMode::Characters {
        LayoutType::CharactersMod
    } else if engine_mode == KeyboardMode::Symbols {
        LayoutType::SymbolsMod
    } else {
        LayoutType::Symbols2Mod
    };
    let main = KeyboardLayout::new(
        main_type,
        "probe".into(),
        "main".into(),
        Rc::new(LayoutArrangement::new_nullable(main_rows)),
    );
    main.set_modifier(Some(ComponentName::new("probe", "mod")));
    let main = Rc::new(main);
    let modifier = list_layout(mod_type, "mod", mod_rows);
    let extension = list_layout(LayoutType::NumericRow, "ext", ext_rows);
    let hints = match placement {
        "null-hint-empty" => ArrangementRows::from_nullable(ResourceObjectList::new(vec![None])),
        "hint-outer" => Probe::wrap(vec![row("x"), row("y")], || row("z"), state.clone()),
        "hint-inner" => outer(vec![Probe::wrap(
            vec![key("x"), key("y")],
            || key("z"),
            state.clone(),
        )]),
        _ => outer(vec![row("x"), row("y")]),
    };
    let hint = list_layout(LayoutType::Symbols, "hint", Some(hints));
    resources.add_layout(hint.clone());
    resources.add_layout(main.clone());
    resources.add_layout(modifier.clone());
    if placement != "null-hint-empty" {
        resources.add_layout(extension.clone());
    }
    let subtype = Arc::new(Subtype::from_values(
        ferroboard_core::layouts::SubtypeValues {
            layouts: Some(Arc::new(SubtypeLayoutMap::from_values(
                ferroboard_core::layouts::SubtypeLayoutMapValues {
                    characters: ComponentName::new("probe", "main"),
                    numeric: ComponentName::new("probe", "main"),
                    symbols: ComponentName::new(
                        "probe",
                        if placement.contains("hint") {
                            "hint"
                        } else {
                            "main"
                        },
                    ),
                    symbols2: ComponentName::new("probe", "main"),
                    numeric_row: ComponentName::new("probe", "ext"),
                    ..(*Subtype::shared_default().layouts().unwrap()).values()
                },
            ))),
            ..(*Subtype::shared_default()).values()
        },
    ));
    let target = if placement.contains("hint") {
        hint
    } else if placement.contains("modifier") {
        modifier
    } else if placement.contains("extension") {
        extension
    } else {
        main
    };
    let ctx = Rc::new(KeyComputeContext::from_values(
        ferroboard_core::layouts::KeyComputeContextValues {
            mode: engine_mode,
            show_number_hints: false,
            show_symbol_hints: false,
            ..Default::default()
        },
    ));
    let mut keyboard = None;
    let mut result = Value::Null;
    let failure = if operation == "layout" {
        match LayoutComputer::new(resources).compute(engine_mode, subtype, false) {
            Err(e) => Some(resource_error(e)),
            Ok(k) => {
                let e = k.compute(ctx.clone()).err().map(|e| {
                    if let KeyDataError::Collection(e) = e {
                        error(e)
                    } else {
                        panic!("unexpected {e:?}")
                    }
                });
                keyboard = Some(k);
                e
            }
        }
    } else {
        let value = if operation == "write-arrangement" {
            LayoutJson::write_arrangement(target.arrangement().as_ref().unwrap(), false)
        } else {
            LayoutJson::write_layout_document(&target, false)
        };
        match value {
            Err(e) => Some(json_error(e)),
            Ok(v) => {
                result = serde_json::from_str(&v).unwrap();
                None
            }
        }
    };
    if let Some(k) = keyboard {
        result = json!({"rows":k.rows().snapshot().unwrap().iter().map(|r|r.len()).collect::<Vec<_>>(),"keys":k.keys().map(|key| key.unwrap().unwrap()).map(|key|json!({"sourceNull":key.source().is_none(),"code":key.code()})).collect::<Vec<_>>(),"version":k.version(),"contextSet":k.context().is_some_and(|c|Rc::ptr_eq(&ctx,&c))});
    }
    json!({"kind":"case","placement":placement,"mode":mode,"operation":operation,"error":failure,"trace":*state.trace.borrow(),"remaining":state.remaining(),"result":result})
}
fn rows() -> Vec<Value> {
    include_str!("../../../tests/fixtures/arrangement-rows.jsonl")
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect()
}
#[test]
fn original_nested_row_custom_counts_indices_enumeration_disposal_mutation_nulls_and_serialization()
{
    let expected = rows();
    let selected = &MODES[..9];
    let mut compared = 0;
    for row in expected.iter().filter(|v| v["kind"] == "case") {
        let mode = row["mode"].as_str().unwrap();
        if !selected.contains(&mode) {
            continue;
        }
        let mode = *selected.iter().find(|m| **m == mode).unwrap();
        let mut actual = run(
            row["placement"].as_str().unwrap(),
            mode,
            row["operation"].as_str().unwrap(),
        );
        let mut wanted = row.clone();
        actual.as_object_mut().unwrap().remove("trace");
        wanted.as_object_mut().unwrap().remove("trace");
        assert_eq!(actual, wanted, "{row}");
        compared += 1;
    }
    assert_eq!(compared, 228);
}

#[test]
fn native_empty_resource_buffers_are_independent_mutable_and_snapshotted() {
    let a = ResourceObjectList::<Key>::default();
    let b = ResourceObjectList::<Key>::default();
    assert!(!a.ptr_eq(&b));
    a.push(key("a"));
    let snapshot = a.iter();
    a.clear();
    assert_eq!(snapshot.count(), 1);
    assert!(a.is_empty());
    assert!(b.is_empty());
    let constant = LayoutArrangement::empty();
    assert!(constant.rows.as_ref().unwrap().as_concrete().is_none());
    assert_eq!(constant.rows.as_ref().unwrap().count().unwrap(), 0);
}

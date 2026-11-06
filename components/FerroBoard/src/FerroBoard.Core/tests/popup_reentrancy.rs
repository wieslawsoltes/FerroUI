use ferroboard_core::{keys::*, layouts::*, text::Utf16Text};
use serde_json::{Value, json};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
};
struct Probe {
    name: &'static str,
    mutation: &'static str,
    hidden: bool,
    enabled: Rc<Cell<bool>>,
    relevant: Rc<RefCell<ResourceObjectList<Key>>>,
    trace: Rc<RefCell<Vec<String>>>,
}
impl AbstractKeyData for Probe {
    fn compute_nullable(
        self: Rc<Self>,
        _: Option<&dyn IKeyEvaluator>,
    ) -> Result<Option<TextKey>, KeyDataError> {
        self.trace
            .borrow_mut()
            .push(format!("{}:compute", self.name));
        if self.enabled.get() {
            self.trace
                .borrow_mut()
                .push(format!("mutate:{}", self.mutation));
            let relevant = self.relevant.borrow().clone();
            match self.mutation {
                "none" => {}
                "add" => relevant.push(TextKeyData::char("n".into(), None)?),
                "clear" => relevant.clear(),
                "replace" => relevant.set(0, TextKeyData::char("r".into(), None)?)?,
                "set-same" => relevant.set(0, relevant.get(0)?)?,
                "remove-last" => {
                    relevant.remove(relevant.len() - 1)?;
                }
                "clear-add" => {
                    relevant.clear();
                    relevant.push(TextKeyData::char("n".into(), None)?);
                }
                _ => unreachable!(),
            }
        }
        Ok(if self.hidden {
            None
        } else {
            Some(TextKeyData::char(self.name.into(), None)?)
        })
    }
    fn as_string(&self, _: bool) -> Result<Option<Utf16Text>, KeyDataError> {
        Ok(Some(self.name.into()))
    }
}
#[test]
fn original_popup_main_and_relevant_mutation_precedence_errors_partial_state_and_recovery() {
    let rows = include_str!("../../../tests/fixtures/popup-reentrancy.jsonl")
        .lines()
        .map(|s| serde_json::from_str::<Value>(s).unwrap())
        .collect::<Vec<_>>();
    for storage in ["list", "array"] {
        for placement in ["main", "relevant"] {
            for mutation in [
                "none",
                "add",
                "clear",
                "replace",
                "set-same",
                "remove-last",
                "clear-add",
            ] {
                for hidden in [false, true] {
                    let trace = Rc::new(RefCell::new(Vec::new()));
                    let enabled = Rc::new(Cell::new(false));
                    let relevant = ResourceObjectList::new(Vec::<Key>::new());
                    let holder = Rc::new(RefCell::new(relevant.clone()));
                    let probe: Key = Rc::new(Probe {
                        name: if placement == "main" { "m" } else { "p" },
                        mutation,
                        hidden,
                        enabled: enabled.clone(),
                        relevant: holder.clone(),
                        trace: trace.clone(),
                    });
                    if placement == "relevant" {
                        relevant.push(probe.clone());
                    }
                    relevant.push(TextKeyData::char("t".into(), None).unwrap());
                    let relevant = if storage == "array" {
                        ResourceObjectList::new(relevant.snapshot())
                    } else {
                        relevant
                    };
                    holder.replace(relevant.clone());
                    let popup = Rc::new(PopupSet::from_parts(
                        if placement == "main" {
                            Some(probe)
                        } else {
                            None
                        },
                        (relevant.clone()).into(),
                    ));
                    let parent: Key = TextKeyData::char("a".into(), Some(popup)).unwrap();
                    let resources = Rc::new(KeyboardResources::create_empty());
                    resources.add_layout(Rc::new(KeyboardLayout::new(
                        LayoutType::Characters,
                        "probe".into(),
                        "main".into(),
                        Rc::new(LayoutArrangement::from_rows(vec![vec![parent]])),
                    )));
                    let subtype = Arc::new(Subtype::from_values(
                        ferroboard_core::layouts::SubtypeValues {
                            layouts: Some(Arc::new(SubtypeLayoutMap::from_values(
                                ferroboard_core::layouts::SubtypeLayoutMapValues {
                                    characters: ComponentName::new("probe", "main"),
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
                    for phase in ["warm", "mutate", "recover"] {
                        enabled.set(phase == "mutate");
                        trace.borrow_mut().clear();
                        let previous = key.popup_keys();
                        let context = Rc::new(KeyComputeContext::from_values(
                            ferroboard_core::layouts::KeyComputeContextValues {
                                show_number_hints: false,
                                show_symbol_hints: false,
                                ..KeyComputeContextValues::default()
                            },
                        ));
                        let error = match keyboard.compute(context.clone()) {
                            Ok(()) => None,
                            Err(KeyDataError::Collection(ResourceTextListError::Modified)) => {
                                Some("InvalidOperationException")
                            }
                            Err(KeyDataError::Collection(ResourceTextListError::NotSupported)) => {
                                Some("NotSupportedException")
                            }
                            Err(e) => panic!("unexpected {e:?}"),
                        };
                        let mut actual = json!({
                            "storage":storage,"placement":placement,"mutation":mutation,"hidden":hidden,"phase":phase,"error":error,"trace":*trace.borrow(),
                            "version":keyboard.version(),"contextSet":Rc::ptr_eq(&keyboard.context().unwrap(),&context),
                            "popupListSame":previous.ptr_eq(&key.popup_keys()),"sourceRelevantCount":relevant.len(),
                            "code":key.code(),"label":key.label().map(|s|s.to_utf8().unwrap()),"hint":key.hint_label().map(|s|s.to_utf8().unwrap()),
                            "visible":key.is_visible(),"enabled":key.is_enabled(),"popups":key.popup_keys().snapshot().unwrap().iter().map(|k|k.as_string(false).unwrap().unwrap().to_utf8().unwrap()).collect::<Vec<_>>(),
                            "defaultIndex":key.popup_default_index(),
                        });
                        let expected = rows
                            .iter()
                            .find(|r| {
                                r["storage"] == storage
                                    && r["placement"] == placement
                                    && r["mutation"] == mutation
                                    && r["hidden"] == hidden
                                    && r["phase"] == phase
                            })
                            .unwrap();
                        assert!(
                            error.is_none(),
                            "native snapshot mutation must compute: {actual}"
                        );
                        let source_compatible = storage == "list"
                            && (placement == "main" || mutation == "none")
                            || storage == "array"
                                && ["none", "replace", "set-same"].contains(&mutation);
                        if source_compatible {
                            let mut expected = expected.clone();
                            actual.as_object_mut().unwrap().remove("popupListSame");
                            expected.as_object_mut().unwrap().remove("popupListSame");
                            assert_eq!(actual, expected);
                        }
                    }
                    // The probe itself keeps the original mutable list as a normal managed-style
                    // reference. Release that deliberate reference cycle explicitly in this test.
                    holder.replace(ResourceObjectList::new(Vec::new()));
                    relevant.clear();
                }
            }
        }
    }
    assert_eq!(rows.len(), 183);
}

#[test]
fn native_resource_snapshots_preserve_shared_handles_across_worker_mutation() {
    let values = ResourceObjectList::<Arc<LayoutMetadata>>::default();
    let mut before = values.iter();
    let item = Arc::new(LayoutMetadata::new(LayoutType::Characters, "shared".into()));
    let worker = {
        let values = values.clone();
        let item = item.clone();
        std::thread::spawn(move || values.push(item))
    };
    worker.join().unwrap();
    assert!(before.next().is_none());
    let mut snapshot = values.iter();
    values.clear();
    assert!(Arc::ptr_eq(&snapshot.next().unwrap(), &item));
    assert!(snapshot.next().is_none());
    assert!(!values.ptr_eq(&ResourceObjectList::default()));
}
#[test]
fn native_popup_defaults_and_parsed_resources_use_fresh_mutable_buffers() {
    let defaults = PopupSet::default();
    let other = PopupSet::default();
    assert!(defaults.is_empty().unwrap());
    assert!(
        !defaults
            .relevant()
            .unwrap()
            .ptr_eq(&other.relevant().unwrap())
    );
    defaults
        .relevant()
        .unwrap()
        .as_concrete()
        .unwrap()
        .push(TextKeyData::char("a".into(), None).unwrap());
    assert!(!defaults.is_empty().unwrap());
    assert!(other.is_empty().unwrap());
    let a = LayoutJson::read_popup_set(&LayoutJson::parse("{}").unwrap()).unwrap();
    let b = LayoutJson::read_popup_set(&LayoutJson::parse("[]").unwrap()).unwrap();
    assert!(!a.relevant().unwrap().ptr_eq(&b.relevant().unwrap()));
}

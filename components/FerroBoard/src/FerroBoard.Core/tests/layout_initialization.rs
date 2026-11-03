//! Actual unchanged-source auto-property initialization and retained resource observations.
use ferroboard_core::{keys::LayoutDirection, layouts::*, text::Utf16Text};
use serde_json::{Value, json};
use std::{cell::Cell, rc::Rc};
fn rows() -> Vec<Value> {
    include_str!("../../../tests/fixtures/layout-initialization.jsonl")
        .lines()
        .map(|v| serde_json::from_str(v).unwrap())
        .collect()
}
fn units(value: Option<Utf16Text>) -> Value {
    json!(value.map(|v| v.units().to_vec()))
}
fn state(l: &KeyboardLayout) -> Value {
    json!({"type":l.layout_type().0,"extension":units(l.extension_id()),"id":units(l.id()),"label":units(l.label()),"authors":l.authors().map(|a|a.snapshot().into_iter().map(units).collect::<Vec<_>>()),"direction":l.direction().0,"modifier":l.modifier().map(|m|m.to_text().to_utf8().unwrap()),"arrangement":l.arrangement().map(|a|LayoutJson::write_arrangement(&a,false).unwrap()),"name":l.name().to_text().units()})
}
#[test]
fn source_default_and_initialization_orders_preserve_values_and_original_references() {
    let mut expected = rows();
    expected[9].as_object_mut().unwrap().remove("authorsShared");
    let a = Rc::new(KeyboardLayout::default());
    let b = Rc::new(KeyboardLayout::default());
    assert_eq!(
        json!({"kind":"defaults","state":state(&a),"freshClass":!Rc::ptr_eq(&a,&b)}),
        expected[9]
    );
    assert!(!a.authors().unwrap().ptr_eq(&b.authors().unwrap()));
    let mut at = 10;
    for profile in ["normal", "nulls", "empty", "surrogates"] {
        for reverse in [false, true] {
            let l = Rc::new(KeyboardLayout::default());
            let authors = ResourceTexts::from_nullable(vec![Some("first".into()), None]);
            let a = LayoutJson::read_arrangement("[[\"a\"]]").unwrap();
            let text = match profile {
                "nulls" => None,
                "empty" => Some("".into()),
                "surrogates" => Some(Utf16Text::from_units(vec![0xd800])),
                _ => Some("fixture".into()),
            };
            let mut names = vec![
                "Type",
                "ExtensionId",
                "Id",
                "Label",
                "Authors",
                "Direction",
                "Modifier",
                "Arrangement",
            ];
            if reverse {
                names.reverse();
            }
            for name in names {
                match name {
                    "Type" => l.set_type(LayoutType::Numeric),
                    "ExtensionId" => l.set_extension_id(text.clone()),
                    "Id" => l.set_id(text.clone()),
                    "Label" => l.set_label(text.clone()),
                    "Authors" => l.set_authors(if profile == "nulls" {
                        None
                    } else {
                        Some(authors.clone())
                    }),
                    "Direction" => l.set_direction(LayoutDirection::Rtl),
                    "Modifier" => l.set_modifier(if profile == "nulls" {
                        None
                    } else {
                        Some(ComponentName::new_nullable(None, Some("m".into())))
                    }),
                    "Arrangement" => l.set_arrangement(if profile == "nulls" {
                        None
                    } else {
                        Some(a.clone())
                    }),
                    _ => unreachable!(),
                }
                assert_eq!(
                    json!({"kind":"step","profile":profile,"reverse":reverse,"property":name,"state":state(&l),"authorsSame":l.authors().is_some_and(|v|v.ptr_eq(&authors)),"arrangementSame":l.arrangement().is_some_and(|v|Rc::ptr_eq(&v,&a))}),
                    expected[at]
                );
                at += 1;
            }
        }
    }
    assert_eq!(at, 74);
}
#[test]
fn source_raw_init_accessors_keep_cached_class_and_original_metadata_snapshot() {
    let expected = rows();
    let l = Rc::new(KeyboardLayout::new(
        LayoutType::Numeric,
        "ext".into(),
        "old".into(),
        LayoutJson::read_arrangement("[[\"a\"]]").unwrap(),
    ));
    let authors = ResourceTexts::new(vec!["old".into()]);
    l.set_label(Some("old".into()));
    l.set_authors(Some(authors.clone()));
    let resources = KeyboardResources::create_empty();
    let events = Rc::new(Cell::new(0));
    let changed = events.clone();
    resources
        .changed()
        .add(Rc::new(move |_| changed.set(changed.get() + 1)));
    resources.add_layout(l.clone());
    let m = resources.get_layout_metadata(LayoutType::Numeric).unwrap()[0].clone();
    for (i, name) in [
        "Type",
        "ExtensionId",
        "Id",
        "Label",
        "Authors",
        "Direction",
        "Modifier",
        "Arrangement",
    ]
    .into_iter()
    .enumerate()
    {
        match name {
            "Type" => l.set_type(LayoutType::Phone),
            "ExtensionId" => l.set_extension_id(Some("next".into())),
            "Id" => l.set_id(Some("next".into())),
            "Label" => l.set_label(Some("next".into())),
            "Authors" => l.set_authors(Some(ResourceTexts::new(vec!["new".into()]))),
            "Direction" => l.set_direction(LayoutDirection::Rtl),
            "Modifier" => l.set_modifier(Some(ComponentName::new("ext", "mod"))),
            "Arrangement" => {
                l.set_arrangement(Some(LayoutJson::read_arrangement("[[\"b\"]]").unwrap()))
            }
            _ => unreachable!(),
        }
        assert_eq!(
            json!({"kind":"retained","property":name,"state":state(&l),"oldKeySame":Rc::ptr_eq(&l,&resources.get_layout(LayoutType::Numeric,&ComponentName::new("ext","old")).unwrap().unwrap()),"newKeyMissing":resources.get_layout(LayoutType::Phone,&ComponentName::new("next","next")).unwrap().is_none(),"metadataId":units(m.id.clone()),"metadataLabel":units(m.label.clone()),"metadataAuthorsOld":m.authors.as_ref().unwrap().ptr_eq(&authors),"metadataDirection":m.direction.0,"metadataModifier":m.modifier.as_ref().map(|m|m.to_text().to_utf8().unwrap()),"events":events.get()}),
            expected[74 + i]
        );
    }
}

#[test]
fn replacing_a_retained_interface_drops_it_without_a_layout_borrow() {
    use std::{cell::RefCell, rc::Weak};
    struct DropRows {
        layout: Weak<KeyboardLayout>,
        observed: Rc<RefCell<Vec<Option<Utf16Text>>>>,
    }
    impl Drop for DropRows {
        fn drop(&mut self) {
            self.observed
                .borrow_mut()
                .push(self.layout.upgrade().unwrap().label());
        }
    }
    impl IResourceReadOnlyList<ArrangementRow> for DropRows {
        fn count(&self) -> Result<i32, ResourceTextListError> {
            Ok(0)
        }
        fn get(&self, _: i32) -> Result<Option<ArrangementRow>, ResourceTextListError> {
            Err(ResourceTextListError::IndexOutOfRange)
        }
        fn iter(&self) -> Result<ResourceItems<ArrangementRow>, ResourceTextListError> {
            ArrangementRows::default().iter()
        }
    }
    let layout = Rc::new(KeyboardLayout::default());
    layout.set_label(Some("visible".into()));
    let seen = Rc::new(RefCell::new(Vec::new()));
    layout.set_arrangement(Some(Rc::new(LayoutArrangement::new(
        ArrangementRows::from_custom(Rc::new(DropRows {
            layout: Rc::downgrade(&layout),
            observed: seen.clone(),
        })),
    ))));
    layout.set_arrangement(None);
    assert_eq!(*seen.borrow(), vec![Some("visible".into())]);
}

use ferroboard_core::{layouts::*, text::Utf16Text};
use serde_json::{Value, json};
use std::{cell::Cell, rc::Rc};
fn rows() -> impl Iterator<Item = Value> {
    include_str!("../../../tests/fixtures/popup-mapping-state.jsonl")
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
}
fn units(v: Option<Utf16Text>) -> Value {
    json!(v.map(|v| v.units().to_vec()))
}
fn text(v: &Value) -> Option<Utf16Text> {
    v.as_array().map(|v| {
        Utf16Text::from_units(
            v.iter()
                .map(|v| v.as_u64().unwrap() as u16)
                .collect::<Vec<_>>(),
        )
    })
}
#[test]
fn source_mapping_defaults_nullable_init_original_authors_and_registry_keys() {
    let mut count = 0;
    for mut row in rows() {
        // Native code-unit buffers do not promise CLR empty-string interning.
        for field in ["hashStable", "emptyShared", "newEmptyShared"] {
            row.as_object_mut().unwrap().remove(field);
        }
        match row["tag"].as_str().unwrap() {
            "property" => {
                assert_eq!(row["init"], true);
                assert_eq!(row["required"], false);
                count += 1;
            }
            "text" => {
                let m = Rc::new(PopupMapping::default());
                let alias = m.clone();
                let field = row["field"].as_str().unwrap();
                let value = text(&row["value"]);
                let got = match field {
                    "Id" => {
                        alias.set_id(value);
                        m.id()
                    }
                    "ExtensionId" => {
                        alias.set_extension_id(value);
                        m.extension_id()
                    }
                    _ => panic!("{field}"),
                };
                assert_eq!(
                    json!({"tag":"text","field":field,"value":units(got),"same":Rc::ptr_eq(&m,&alias)}),
                    row
                );
                count += 1;
            }
            "default" => {
                row.as_object_mut().unwrap().remove("authorsShared");
                let a = Rc::new(PopupMapping::default());
                let b = Rc::new(PopupMapping::default());
                let authors = a.authors().unwrap();
                assert!(!authors.ptr_eq(&b.authors().unwrap()));
                let maps = a.mappings().unwrap();
                let actual = json!({"tag":"default","extension":units(a.extension_id()),"id":units(a.id()),"authors":authors.count().unwrap(),"maps":maps.count().unwrap(),"mapsShared":maps.ptr_eq(&b.mappings().unwrap()),"mapsMutable":maps.as_concrete().is_some(),"identity":std::ptr::eq(&*a,&*a),"other":std::ptr::eq(&*a,&*b),"text":row["text"]});
                assert_eq!(
                    a.to_text(),
                    Utf16Text::from("FerroBoard.Layouts.PopupMapping")
                );
                assert_eq!(row["text"], "ScreenKeyboard.Layouts.PopupMapping");
                assert_eq!(actual, {
                    let mut row = row.clone();
                    if row["tag"] == "default" {
                        row.as_object_mut().unwrap().remove("wrong");
                        row.as_object_mut().unwrap().remove("nullEqual");
                    }
                    row
                });
                count += 1;
            }
            "null-fields" => {
                let a = Rc::new(PopupMapping::default());
                let old_authors = a.authors().unwrap();
                let old_maps = a.mappings().unwrap();
                let alias = a.clone();
                alias.set_authors(None);
                alias.set_mappings(None);
                assert_eq!(
                    json!({"tag":"null-fields","authorsNull":a.authors().is_none(),"mapsNull":a.mappings().is_none(),"oldAuthorsCount":old_authors.count().unwrap(),"oldMapsCount":old_maps.count().unwrap()}),
                    row
                );
                count += 1;
            }
            "authors" => {
                let m = Rc::new(PopupMapping::default());
                let list = ResourceTexts::from_nullable(vec![
                    Some("a".into()),
                    None,
                    Some(Utf16Text::from_units(vec![120, 0xd800])),
                ]);
                m.set_authors(Some(list.clone()));
                let got = m.authors().unwrap().as_concrete().unwrap();
                list.push(Some("later".into()));
                assert_eq!(
                    json!({"tag":"authors","same":got.ptr_eq(&list),"values":got.snapshot().into_iter().map(units).collect::<Vec<_>>()}),
                    row
                );
                count += 1;
            }
            "registry" => {
                let resources = KeyboardResources::create_empty();
                let changed = Rc::new(Cell::new(0));
                let counter = changed.clone();
                let _subscription = resources
                    .changed()
                    .add(Rc::new(move |_| counter.set(counter.get() + 1)));
                let m = Rc::new(PopupMapping::default());
                m.set_id(None);
                m.set_extension_id(None);
                resources.add_popup_mapping(m.clone());
                let old = ComponentName::new_nullable(None, None);
                let retained = Rc::ptr_eq(&m, &resources.get_popup_mapping(&old).unwrap().unwrap());
                m.set_id(Some("id".into()));
                m.set_extension_id(Some("new".into()));
                let next = ComponentName::new("new", "id");
                let old_still =
                    Rc::ptr_eq(&m, &resources.get_popup_mapping(&old).unwrap().unwrap());
                let missing = resources.get_popup_mapping(&next).unwrap().is_none();
                resources.add_popup_mapping(m.clone());
                assert_eq!(
                    json!({"tag":"registry","retained":retained,"oldStill":old_still,"missingNew":missing,"newSame":Rc::ptr_eq(&m,&resources.get_popup_mapping(&next).unwrap().unwrap()),"changed":changed.get()}),
                    row
                );
                count += 1;
            }
            _ => {}
        }
    }
    assert_eq!(count, 18);
}
#[path = "../../../tests/support/popup_mapping_lookup_contract.rs"]
mod lookup;
#[test]
fn source_mapping_lookup_short_circuit_out_values_and_reentrant_property_replacement() {
    lookup::verify(|m, v, label| m.get_nullable(v, label));
}

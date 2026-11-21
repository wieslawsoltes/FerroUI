//! Source combo factory state plus shared collection/native-control integration.
use super::*;
use crate::controls::*;
use ferroribbon_core::{commands::RibbonRelayCommand, model::*};
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_controls::Control;
use ferroui_system::component_model::INotifyPropertyChanged;
use serde_json::{Value, json};
use std::{cell::RefCell, rc::Rc};
fn check(name: &str, mut actual: Value) {
    let fixture: Value =
        serde_json::from_str(include_str!("../../../tests/fixtures/menu-factory.json")).unwrap();
    actual["name"] = json!(name);
    let mut expected = fixture["corpus"]["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["name"] == name)
        .unwrap()
        .clone();
    // C# serializes integral double widths without a decimal. Compare actual
    // numeric values; every integer in these two cases is an exact Int32.
    fn numbers(v: &mut Value) {
        match v {
            Value::Number(n) => *v = json!(n.as_f64().unwrap()),
            Value::Array(a) => a.iter_mut().for_each(numbers),
            Value::Object(o) => o.values_mut().for_each(numbers),
            _ => {}
        }
    }
    numbers(&mut actual);
    numbers(&mut expected);
    assert_eq!(actual, expected);
}
fn object(s: &str) -> RibbonObject {
    RibbonObject::value(s.to_owned())
}
#[test]
fn original_combo_factory_uses_same_items_and_two_way_null_notification_order() {
    crate::group_tests::run(|| {
        let f = RibbonElementFactory::new();
        let b = RibbonModelBindings::new();
        let first = object("first");
        let second = object("second");
        let m = RibbonComboBoxModel::with_items(
            Some("combo".into()),
            Some("Combo".into()),
            [Some(first.clone()), Some(second.clone())],
        );
        m.set_is_editable(true);
        m.set_input_width(144.);
        m.set_placeholder(Some("hint".into()));
        m.set_max_drop_down_height(188.);
        m.set_selected_item(Some(first.clone()));
        m.set_text(Some("initial".into()));
        let e = f
            .create_combo_box(m.clone(), b.clone())
            .cast::<RibbonComboBox>()
            .unwrap();
        let selected = crate::metadata::model_objects::to_native(Some(first))
            .unwrap()
            .unwrap();
        let initial = json!({"same":e.items_source().unwrap().downcast_ref::<RibbonObjectItems>().unwrap()==&m.items(),"own":e.items().count(),"selected":Rc::ptr_eq(&selected,&e.selected_item().unwrap()),"text":e.text(),"editable":e.is_editable(),"width":e.input_width(),"placeholder":e.placeholder_text(),"max":e.max_drop_down_height()});
        let trace = Rc::new(RefCell::new(vec![]));
        let t = trace.clone();
        m.property_changed()
            .add(Rc::new(move |p| t.borrow_mut().push(p.to_owned())));
        e.set_selected_item(
            crate::metadata::model_objects::to_native(Some(second.clone())).unwrap(),
        );
        let changed = json!({"selected":m.selected_item().unwrap().ptr_eq(&second),"text":m.text().map(|s|s.to_utf8().unwrap()),"trace":trace.borrow().clone()});
        m.set_selected_item(None);
        let keeps = e.text();
        m.items().add(Some(object("third")));
        let count = e.effective_items().len();
        m.set_text(None);
        let null_text = e.text();
        b.dispose();
        e.set_text(Some("disposed".into()));
        m.set_input_width(200.);
        check(
            "combo_factory",
            json!({"initial":initial,"changed":changed,"nullKeeps":keeps,"count":count,"nullText":null_text,"disposedModel":m.text().map(|s|s.to_utf8().unwrap()),"disposedWidth":e.input_width()}),
        );
    });
}
#[test]
fn original_preview_font_family_recreates_native_container_child_and_disposes_old_bindings() {
    crate::group_tests::run(|| {
        let f = RibbonElementFactory::new();
        let b = RibbonModelBindings::new();
        let m = RibbonComboBoxModel::new(Some("font".into()), Some("Font".into()));
        let group = RibbonGroupModel::new(None, None);
        group.items().add(Some(m.clone()));
        let e = f.create_group(group, b.clone());
        let get = || e.items().get(0).unwrap().cast::<RibbonComboBox>().unwrap();
        let first = get();
        m.set_preview_font_family(true);
        let second = get();
        let changed = json!({"replaced":!first.ptr_eq(&second),"font":second.is::<RibbonFontComboBox>(),"own":second.items().count(),"width":second.input_width(),"editable":second.is_editable()});
        m.set_input_width(155.);
        let widths = [first.input_width(), second.input_width()];
        b.suspend();
        m.set_preview_font_family(false);
        let suspended = get().ptr_eq(&second);
        b.resume();
        let resumed =
            json!({"replaced":!get().ptr_eq(&second),"font":get().is::<RibbonFontComboBox>()});
        b.dispose();
        check(
            "combo_recreate",
            json!({"changed":changed,"widths":widths,"suspended":suspended,"resumed":resumed}),
        );
    });
}
#[test]
fn model_combo_command_and_linked_copy_commit_once_with_actual_selected_payload() {
    crate::group_tests::run(|| {
        let f = RibbonElementFactory::new();
        let b = RibbonModelBindings::new();
        let m = RibbonComboBoxModel::new(None, None);
        let one = object("one");
        m.items().add(Some(one.clone()));
        let log = Rc::new(RefCell::new(vec![]));
        let l = log.clone();
        m.set_command(Some(RibbonRelayCommand::new_with_parameter(
            move |p| {
                l.borrow_mut()
                    .push(p.unwrap().downcast_ref::<String>().unwrap().clone())
            },
            None,
        )));
        let e = f
            .create_item(Some(m.clone()), b.clone())
            .unwrap()
            .cast::<RibbonComboBox>()
            .unwrap();
        let item = ValueTypes::try_cast(
            &Control::boxed(e.clone()),
            ValueType::of::<Rc<dyn IRibbonItem>>(),
        )
        .unwrap();
        let copy = item
            .downcast_ref::<Rc<dyn IRibbonItem>>()
            .unwrap()
            .create_linked_copy()
            .unwrap()
            .cast::<RibbonComboBox>()
            .unwrap();
        let payload = crate::metadata::model_objects::to_native(Some(one.clone())).unwrap();
        copy.commit(payload);
        assert_eq!(&*log.borrow(), &["one"]);
        assert!(m.selected_item().unwrap().ptr_eq(&one));
        assert_eq!(m.text().unwrap().to_utf8().unwrap(), "one");
        m.items().add(Some(object("two")));
        assert_eq!(copy.effective_items().len(), 2);
        assert!(ValueTypes::identity_equals(
            e.items_source().as_ref(),
            copy.items_source().as_ref()
        ));
        b.dispose();
    });
}
#[test]
fn standalone_factory_disposal_releases_model_but_keeps_actual_collection_alive() {
    crate::group_tests::run(|| {
        let f = RibbonElementFactory::new();
        let b = RibbonModelBindings::new();
        let m = RibbonComboBoxModel::new(None, None);
        let weak = Rc::downgrade(&m);
        let items = m.items();
        let e = f.create_combo_box(m.clone(), b.clone());
        b.dispose();
        drop(m);
        assert!(weak.upgrade().is_none());
        let combo = e.cast::<RibbonComboBox>().unwrap();
        assert!(
            combo
                .items_source()
                .unwrap()
                .downcast_ref::<RibbonObjectItems>()
                .unwrap()
                == &items
        );
        items.add(Some(object("still shared")));
        assert_eq!(combo.effective_items().len(), 1);
    });
}

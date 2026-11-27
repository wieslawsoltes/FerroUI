use ferroribbon_core::{RibbonString, model::*};
use ferroui_system::component_model::INotifyPropertyChanged;
use serde_json::{Value, json};
use std::{cell::RefCell, rc::Rc};
fn corpus() -> Value {
    serde_json::from_str(include_str!("../../../tests/fixtures/item-inputs.json")).unwrap()
}
fn s(v: &Value) -> Option<RibbonString> {
    v.as_array().map(|a| {
        RibbonString::from_utf16(
            a.iter()
                .map(|n| n.as_u64().unwrap() as u16)
                .collect::<Vec<_>>(),
        )
    })
}
fn es(v: Option<RibbonString>) -> Value {
    v.map_or(Value::Null, |s| json!(s.as_utf16()))
}
fn o(v: &Value) -> Option<RibbonObject> {
    s(v).map(RibbonObject::value)
}
fn eo(v: Option<RibbonObject>) -> Value {
    v.map_or(Value::Null, |v| {
        es(Some(v.downcast_ref::<RibbonString>().unwrap().clone()))
    })
}
fn b(v: &Value) -> bool {
    v.as_bool().unwrap()
}
fn eb(v: bool) -> Value {
    json!(v)
}
fn ob(v: &Value) -> Option<bool> {
    v.as_bool()
}
fn eob(v: Option<bool>) -> Value {
    json!(v)
}
fn d(v: &Value) -> f64 {
    f64::from_bits(u64::from_str_radix(v.as_str().unwrap(), 16).unwrap())
}
fn ed(v: f64) -> Value {
    json!(format!("{:016x}", v.to_bits()))
}
fn i(v: &Value) -> i32 {
    v.as_i64().unwrap() as i32
}
fn ei(v: i32) -> Value {
    json!(v)
}

#[test]
fn source_ribbon_check_box_model_defaults_and_setter_traces() {
    let fixture = corpus();
    let model = RibbonCheckBoxModel::new(Some("x".into()), Some("X".into()));
    let mut properties = serde_json::Map::new();
    properties.insert("IsChecked".into(), eob(model.is_checked()));
    properties.insert("IsThreeState".into(), eb(model.is_three_state()));
    assert_eq!(
        json!({"size":model.size().raw(),"qat":model.can_add_to_quick_access(),"label":es(model.label()),"properties":properties}),
        fixture["defaults"]["RibbonCheckBoxModel"]
    );
    {
        let model = RibbonCheckBoxModel::new(Some("x".into()), Some("X".into()));
        let trace = Rc::new(RefCell::new(Vec::<String>::new()));
        let log = trace.clone();
        model
            .property_changed()
            .add(Rc::new(move |n| log.borrow_mut().push(n.into())));
        let case = fixture["mutations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["type"] == "RibbonCheckBoxModel" && c["property"] == "IsChecked")
            .unwrap();
        let mut actual = vec![];
        for value in case["values"].as_array().unwrap() {
            model.set_is_checked(ob(value));
            actual.push(json!({"value":eob(model.is_checked()),"events":*trace.borrow()}));
            trace.borrow_mut().clear();
        }
        assert_eq!(
            json!(actual),
            case["states"],
            "RibbonCheckBoxModel.IsChecked"
        );
    }
    {
        let model = RibbonCheckBoxModel::new(Some("x".into()), Some("X".into()));
        let trace = Rc::new(RefCell::new(Vec::<String>::new()));
        let log = trace.clone();
        model
            .property_changed()
            .add(Rc::new(move |n| log.borrow_mut().push(n.into())));
        let case = fixture["mutations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["type"] == "RibbonCheckBoxModel" && c["property"] == "IsThreeState")
            .unwrap();
        let mut actual = vec![];
        for value in case["values"].as_array().unwrap() {
            model.set_is_three_state(b(value));
            actual.push(json!({"value":eb(model.is_three_state()),"events":*trace.borrow()}));
            trace.borrow_mut().clear();
        }
        assert_eq!(
            json!(actual),
            case["states"],
            "RibbonCheckBoxModel.IsThreeState"
        );
    }
}
#[test]
fn source_ribbon_combo_box_model_defaults_and_setter_traces() {
    let fixture = corpus();
    let model = RibbonComboBoxModel::new(Some("x".into()), Some("X".into()));
    let mut properties = serde_json::Map::new();
    properties.insert("SelectedItem".into(), eo(model.selected_item()));
    properties.insert("Text".into(), es(model.text()));
    properties.insert("IsEditable".into(), eb(model.is_editable()));
    properties.insert("InputWidth".into(), ed(model.input_width()));
    properties.insert("Placeholder".into(), es(model.placeholder()));
    properties.insert("DisplayMemberPath".into(), es(model.display_member_path()));
    properties.insert("PreviewFontFamily".into(), eb(model.preview_font_family()));
    properties.insert("MaxDropDownHeight".into(), ed(model.max_drop_down_height()));
    assert_eq!(
        json!({"size":model.size().raw(),"qat":model.can_add_to_quick_access(),"label":es(model.label()),"properties":properties}),
        fixture["defaults"]["RibbonComboBoxModel"]
    );
    {
        let model = RibbonComboBoxModel::new(Some("x".into()), Some("X".into()));
        let trace = Rc::new(RefCell::new(Vec::<String>::new()));
        let log = trace.clone();
        model
            .property_changed()
            .add(Rc::new(move |n| log.borrow_mut().push(n.into())));
        let case = fixture["mutations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["type"] == "RibbonComboBoxModel" && c["property"] == "SelectedItem")
            .unwrap();
        let mut actual = vec![];
        for value in case["values"].as_array().unwrap() {
            model.set_selected_item(o(value));
            actual.push(json!({"value":eo(model.selected_item()),"events":*trace.borrow()}));
            trace.borrow_mut().clear();
        }
        assert_eq!(
            json!(actual),
            case["states"],
            "RibbonComboBoxModel.SelectedItem"
        );
    }
    {
        let model = RibbonComboBoxModel::new(Some("x".into()), Some("X".into()));
        let trace = Rc::new(RefCell::new(Vec::<String>::new()));
        let log = trace.clone();
        model
            .property_changed()
            .add(Rc::new(move |n| log.borrow_mut().push(n.into())));
        let case = fixture["mutations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["type"] == "RibbonComboBoxModel" && c["property"] == "Text")
            .unwrap();
        let mut actual = vec![];
        for value in case["values"].as_array().unwrap() {
            model.set_text(s(value));
            actual.push(json!({"value":es(model.text()),"events":*trace.borrow()}));
            trace.borrow_mut().clear();
        }
        assert_eq!(json!(actual), case["states"], "RibbonComboBoxModel.Text");
    }
    {
        let model = RibbonComboBoxModel::new(Some("x".into()), Some("X".into()));
        let trace = Rc::new(RefCell::new(Vec::<String>::new()));
        let log = trace.clone();
        model
            .property_changed()
            .add(Rc::new(move |n| log.borrow_mut().push(n.into())));
        let case = fixture["mutations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["type"] == "RibbonComboBoxModel" && c["property"] == "IsEditable")
            .unwrap();
        let mut actual = vec![];
        for value in case["values"].as_array().unwrap() {
            model.set_is_editable(b(value));
            actual.push(json!({"value":eb(model.is_editable()),"events":*trace.borrow()}));
            trace.borrow_mut().clear();
        }
        assert_eq!(
            json!(actual),
            case["states"],
            "RibbonComboBoxModel.IsEditable"
        );
    }
    {
        let model = RibbonComboBoxModel::new(Some("x".into()), Some("X".into()));
        let trace = Rc::new(RefCell::new(Vec::<String>::new()));
        let log = trace.clone();
        model
            .property_changed()
            .add(Rc::new(move |n| log.borrow_mut().push(n.into())));
        let case = fixture["mutations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["type"] == "RibbonComboBoxModel" && c["property"] == "InputWidth")
            .unwrap();
        let mut actual = vec![];
        for value in case["values"].as_array().unwrap() {
            model.set_input_width(d(value));
            actual.push(json!({"value":ed(model.input_width()),"events":*trace.borrow()}));
            trace.borrow_mut().clear();
        }
        assert_eq!(
            json!(actual),
            case["states"],
            "RibbonComboBoxModel.InputWidth"
        );
    }
    {
        let model = RibbonComboBoxModel::new(Some("x".into()), Some("X".into()));
        let trace = Rc::new(RefCell::new(Vec::<String>::new()));
        let log = trace.clone();
        model
            .property_changed()
            .add(Rc::new(move |n| log.borrow_mut().push(n.into())));
        let case = fixture["mutations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["type"] == "RibbonComboBoxModel" && c["property"] == "Placeholder")
            .unwrap();
        let mut actual = vec![];
        for value in case["values"].as_array().unwrap() {
            model.set_placeholder(s(value));
            actual.push(json!({"value":es(model.placeholder()),"events":*trace.borrow()}));
            trace.borrow_mut().clear();
        }
        assert_eq!(
            json!(actual),
            case["states"],
            "RibbonComboBoxModel.Placeholder"
        );
    }
    {
        let model = RibbonComboBoxModel::new(Some("x".into()), Some("X".into()));
        let trace = Rc::new(RefCell::new(Vec::<String>::new()));
        let log = trace.clone();
        model
            .property_changed()
            .add(Rc::new(move |n| log.borrow_mut().push(n.into())));
        let case = fixture["mutations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["type"] == "RibbonComboBoxModel" && c["property"] == "DisplayMemberPath")
            .unwrap();
        let mut actual = vec![];
        for value in case["values"].as_array().unwrap() {
            model.set_display_member_path(s(value));
            actual.push(json!({"value":es(model.display_member_path()),"events":*trace.borrow()}));
            trace.borrow_mut().clear();
        }
        assert_eq!(
            json!(actual),
            case["states"],
            "RibbonComboBoxModel.DisplayMemberPath"
        );
    }
    {
        let model = RibbonComboBoxModel::new(Some("x".into()), Some("X".into()));
        let trace = Rc::new(RefCell::new(Vec::<String>::new()));
        let log = trace.clone();
        model
            .property_changed()
            .add(Rc::new(move |n| log.borrow_mut().push(n.into())));
        let case = fixture["mutations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["type"] == "RibbonComboBoxModel" && c["property"] == "PreviewFontFamily")
            .unwrap();
        let mut actual = vec![];
        for value in case["values"].as_array().unwrap() {
            model.set_preview_font_family(b(value));
            actual.push(json!({"value":eb(model.preview_font_family()),"events":*trace.borrow()}));
            trace.borrow_mut().clear();
        }
        assert_eq!(
            json!(actual),
            case["states"],
            "RibbonComboBoxModel.PreviewFontFamily"
        );
    }
    {
        let model = RibbonComboBoxModel::new(Some("x".into()), Some("X".into()));
        let trace = Rc::new(RefCell::new(Vec::<String>::new()));
        let log = trace.clone();
        model
            .property_changed()
            .add(Rc::new(move |n| log.borrow_mut().push(n.into())));
        let case = fixture["mutations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["type"] == "RibbonComboBoxModel" && c["property"] == "MaxDropDownHeight")
            .unwrap();
        let mut actual = vec![];
        for value in case["values"].as_array().unwrap() {
            model.set_max_drop_down_height(d(value));
            actual.push(json!({"value":ed(model.max_drop_down_height()),"events":*trace.borrow()}));
            trace.borrow_mut().clear();
        }
        assert_eq!(
            json!(actual),
            case["states"],
            "RibbonComboBoxModel.MaxDropDownHeight"
        );
    }
}
#[test]
fn source_ribbon_spinner_model_defaults_and_setter_traces() {
    let fixture = corpus();
    let model = RibbonSpinnerModel::new(Some("x".into()), Some("X".into()));
    let mut properties = serde_json::Map::new();
    properties.insert("Value".into(), ed(model.value()));
    properties.insert("Minimum".into(), ed(model.minimum()));
    properties.insert("Maximum".into(), ed(model.maximum()));
    properties.insert("Increment".into(), ed(model.increment()));
    properties.insert("InputWidth".into(), ed(model.input_width()));
    properties.insert("Format".into(), es(model.format()));
    properties.insert("Unit".into(), es(model.unit()));
    assert_eq!(
        json!({"size":model.size().raw(),"qat":model.can_add_to_quick_access(),"label":es(model.label()),"properties":properties}),
        fixture["defaults"]["RibbonSpinnerModel"]
    );
    {
        let model = RibbonSpinnerModel::new(Some("x".into()), Some("X".into()));
        let trace = Rc::new(RefCell::new(Vec::<String>::new()));
        let log = trace.clone();
        model
            .property_changed()
            .add(Rc::new(move |n| log.borrow_mut().push(n.into())));
        let case = fixture["mutations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["type"] == "RibbonSpinnerModel" && c["property"] == "Value")
            .unwrap();
        let mut actual = vec![];
        for value in case["values"].as_array().unwrap() {
            model.set_value(d(value));
            actual.push(json!({"value":ed(model.value()),"events":*trace.borrow()}));
            trace.borrow_mut().clear();
        }
        assert_eq!(json!(actual), case["states"], "RibbonSpinnerModel.Value");
    }
    {
        let model = RibbonSpinnerModel::new(Some("x".into()), Some("X".into()));
        let trace = Rc::new(RefCell::new(Vec::<String>::new()));
        let log = trace.clone();
        model
            .property_changed()
            .add(Rc::new(move |n| log.borrow_mut().push(n.into())));
        let case = fixture["mutations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["type"] == "RibbonSpinnerModel" && c["property"] == "Minimum")
            .unwrap();
        let mut actual = vec![];
        for value in case["values"].as_array().unwrap() {
            model.set_minimum(d(value));
            actual.push(json!({"value":ed(model.minimum()),"events":*trace.borrow()}));
            trace.borrow_mut().clear();
        }
        assert_eq!(json!(actual), case["states"], "RibbonSpinnerModel.Minimum");
    }
    {
        let model = RibbonSpinnerModel::new(Some("x".into()), Some("X".into()));
        let trace = Rc::new(RefCell::new(Vec::<String>::new()));
        let log = trace.clone();
        model
            .property_changed()
            .add(Rc::new(move |n| log.borrow_mut().push(n.into())));
        let case = fixture["mutations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["type"] == "RibbonSpinnerModel" && c["property"] == "Maximum")
            .unwrap();
        let mut actual = vec![];
        for value in case["values"].as_array().unwrap() {
            model.set_maximum(d(value));
            actual.push(json!({"value":ed(model.maximum()),"events":*trace.borrow()}));
            trace.borrow_mut().clear();
        }
        assert_eq!(json!(actual), case["states"], "RibbonSpinnerModel.Maximum");
    }
    {
        let model = RibbonSpinnerModel::new(Some("x".into()), Some("X".into()));
        let trace = Rc::new(RefCell::new(Vec::<String>::new()));
        let log = trace.clone();
        model
            .property_changed()
            .add(Rc::new(move |n| log.borrow_mut().push(n.into())));
        let case = fixture["mutations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["type"] == "RibbonSpinnerModel" && c["property"] == "Increment")
            .unwrap();
        let mut actual = vec![];
        for value in case["values"].as_array().unwrap() {
            model.set_increment(d(value));
            actual.push(json!({"value":ed(model.increment()),"events":*trace.borrow()}));
            trace.borrow_mut().clear();
        }
        assert_eq!(
            json!(actual),
            case["states"],
            "RibbonSpinnerModel.Increment"
        );
    }
    {
        let model = RibbonSpinnerModel::new(Some("x".into()), Some("X".into()));
        let trace = Rc::new(RefCell::new(Vec::<String>::new()));
        let log = trace.clone();
        model
            .property_changed()
            .add(Rc::new(move |n| log.borrow_mut().push(n.into())));
        let case = fixture["mutations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["type"] == "RibbonSpinnerModel" && c["property"] == "InputWidth")
            .unwrap();
        let mut actual = vec![];
        for value in case["values"].as_array().unwrap() {
            model.set_input_width(d(value));
            actual.push(json!({"value":ed(model.input_width()),"events":*trace.borrow()}));
            trace.borrow_mut().clear();
        }
        assert_eq!(
            json!(actual),
            case["states"],
            "RibbonSpinnerModel.InputWidth"
        );
    }
    {
        let model = RibbonSpinnerModel::new(Some("x".into()), Some("X".into()));
        let trace = Rc::new(RefCell::new(Vec::<String>::new()));
        let log = trace.clone();
        model
            .property_changed()
            .add(Rc::new(move |n| log.borrow_mut().push(n.into())));
        let case = fixture["mutations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["type"] == "RibbonSpinnerModel" && c["property"] == "Format")
            .unwrap();
        let mut actual = vec![];
        for value in case["values"].as_array().unwrap() {
            model.set_format(s(value));
            actual.push(json!({"value":es(model.format()),"events":*trace.borrow()}));
            trace.borrow_mut().clear();
        }
        assert_eq!(json!(actual), case["states"], "RibbonSpinnerModel.Format");
    }
    {
        let model = RibbonSpinnerModel::new(Some("x".into()), Some("X".into()));
        let trace = Rc::new(RefCell::new(Vec::<String>::new()));
        let log = trace.clone();
        model
            .property_changed()
            .add(Rc::new(move |n| log.borrow_mut().push(n.into())));
        let case = fixture["mutations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["type"] == "RibbonSpinnerModel" && c["property"] == "Unit")
            .unwrap();
        let mut actual = vec![];
        for value in case["values"].as_array().unwrap() {
            model.set_unit(s(value));
            actual.push(json!({"value":es(model.unit()),"events":*trace.borrow()}));
            trace.borrow_mut().clear();
        }
        assert_eq!(json!(actual), case["states"], "RibbonSpinnerModel.Unit");
    }
}
#[test]
fn source_ribbon_text_box_model_defaults_and_setter_traces() {
    let fixture = corpus();
    let model = RibbonTextBoxModel::new(Some("x".into()), Some("X".into()));
    let mut properties = serde_json::Map::new();
    properties.insert("Text".into(), es(model.text()));
    properties.insert("Placeholder".into(), es(model.placeholder()));
    properties.insert("InputWidth".into(), ed(model.input_width()));
    assert_eq!(
        json!({"size":model.size().raw(),"qat":model.can_add_to_quick_access(),"label":es(model.label()),"properties":properties}),
        fixture["defaults"]["RibbonTextBoxModel"]
    );
    {
        let model = RibbonTextBoxModel::new(Some("x".into()), Some("X".into()));
        let trace = Rc::new(RefCell::new(Vec::<String>::new()));
        let log = trace.clone();
        model
            .property_changed()
            .add(Rc::new(move |n| log.borrow_mut().push(n.into())));
        let case = fixture["mutations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["type"] == "RibbonTextBoxModel" && c["property"] == "Text")
            .unwrap();
        let mut actual = vec![];
        for value in case["values"].as_array().unwrap() {
            model.set_text(s(value));
            actual.push(json!({"value":es(model.text()),"events":*trace.borrow()}));
            trace.borrow_mut().clear();
        }
        assert_eq!(json!(actual), case["states"], "RibbonTextBoxModel.Text");
    }
    {
        let model = RibbonTextBoxModel::new(Some("x".into()), Some("X".into()));
        let trace = Rc::new(RefCell::new(Vec::<String>::new()));
        let log = trace.clone();
        model
            .property_changed()
            .add(Rc::new(move |n| log.borrow_mut().push(n.into())));
        let case = fixture["mutations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["type"] == "RibbonTextBoxModel" && c["property"] == "Placeholder")
            .unwrap();
        let mut actual = vec![];
        for value in case["values"].as_array().unwrap() {
            model.set_placeholder(s(value));
            actual.push(json!({"value":es(model.placeholder()),"events":*trace.borrow()}));
            trace.borrow_mut().clear();
        }
        assert_eq!(
            json!(actual),
            case["states"],
            "RibbonTextBoxModel.Placeholder"
        );
    }
    {
        let model = RibbonTextBoxModel::new(Some("x".into()), Some("X".into()));
        let trace = Rc::new(RefCell::new(Vec::<String>::new()));
        let log = trace.clone();
        model
            .property_changed()
            .add(Rc::new(move |n| log.borrow_mut().push(n.into())));
        let case = fixture["mutations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["type"] == "RibbonTextBoxModel" && c["property"] == "InputWidth")
            .unwrap();
        let mut actual = vec![];
        for value in case["values"].as_array().unwrap() {
            model.set_input_width(d(value));
            actual.push(json!({"value":ed(model.input_width()),"events":*trace.borrow()}));
            trace.borrow_mut().clear();
        }
        assert_eq!(
            json!(actual),
            case["states"],
            "RibbonTextBoxModel.InputWidth"
        );
    }
}
#[test]
fn source_ribbon_slider_model_defaults_and_setter_traces() {
    let fixture = corpus();
    let model = RibbonSliderModel::new(Some("x".into()), Some("X".into()));
    let mut properties = serde_json::Map::new();
    properties.insert("Value".into(), ed(model.value()));
    properties.insert("Minimum".into(), ed(model.minimum()));
    properties.insert("Maximum".into(), ed(model.maximum()));
    properties.insert("StepFrequency".into(), ed(model.step_frequency()));
    properties.insert("SliderWidth".into(), ed(model.slider_width()));
    assert_eq!(
        json!({"size":model.size().raw(),"qat":model.can_add_to_quick_access(),"label":es(model.label()),"properties":properties}),
        fixture["defaults"]["RibbonSliderModel"]
    );
    {
        let model = RibbonSliderModel::new(Some("x".into()), Some("X".into()));
        let trace = Rc::new(RefCell::new(Vec::<String>::new()));
        let log = trace.clone();
        model
            .property_changed()
            .add(Rc::new(move |n| log.borrow_mut().push(n.into())));
        let case = fixture["mutations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["type"] == "RibbonSliderModel" && c["property"] == "Value")
            .unwrap();
        let mut actual = vec![];
        for value in case["values"].as_array().unwrap() {
            model.set_value(d(value));
            actual.push(json!({"value":ed(model.value()),"events":*trace.borrow()}));
            trace.borrow_mut().clear();
        }
        assert_eq!(json!(actual), case["states"], "RibbonSliderModel.Value");
    }
    {
        let model = RibbonSliderModel::new(Some("x".into()), Some("X".into()));
        let trace = Rc::new(RefCell::new(Vec::<String>::new()));
        let log = trace.clone();
        model
            .property_changed()
            .add(Rc::new(move |n| log.borrow_mut().push(n.into())));
        let case = fixture["mutations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["type"] == "RibbonSliderModel" && c["property"] == "Minimum")
            .unwrap();
        let mut actual = vec![];
        for value in case["values"].as_array().unwrap() {
            model.set_minimum(d(value));
            actual.push(json!({"value":ed(model.minimum()),"events":*trace.borrow()}));
            trace.borrow_mut().clear();
        }
        assert_eq!(json!(actual), case["states"], "RibbonSliderModel.Minimum");
    }
    {
        let model = RibbonSliderModel::new(Some("x".into()), Some("X".into()));
        let trace = Rc::new(RefCell::new(Vec::<String>::new()));
        let log = trace.clone();
        model
            .property_changed()
            .add(Rc::new(move |n| log.borrow_mut().push(n.into())));
        let case = fixture["mutations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["type"] == "RibbonSliderModel" && c["property"] == "Maximum")
            .unwrap();
        let mut actual = vec![];
        for value in case["values"].as_array().unwrap() {
            model.set_maximum(d(value));
            actual.push(json!({"value":ed(model.maximum()),"events":*trace.borrow()}));
            trace.borrow_mut().clear();
        }
        assert_eq!(json!(actual), case["states"], "RibbonSliderModel.Maximum");
    }
    {
        let model = RibbonSliderModel::new(Some("x".into()), Some("X".into()));
        let trace = Rc::new(RefCell::new(Vec::<String>::new()));
        let log = trace.clone();
        model
            .property_changed()
            .add(Rc::new(move |n| log.borrow_mut().push(n.into())));
        let case = fixture["mutations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["type"] == "RibbonSliderModel" && c["property"] == "StepFrequency")
            .unwrap();
        let mut actual = vec![];
        for value in case["values"].as_array().unwrap() {
            model.set_step_frequency(d(value));
            actual.push(json!({"value":ed(model.step_frequency()),"events":*trace.borrow()}));
            trace.borrow_mut().clear();
        }
        assert_eq!(
            json!(actual),
            case["states"],
            "RibbonSliderModel.StepFrequency"
        );
    }
    {
        let model = RibbonSliderModel::new(Some("x".into()), Some("X".into()));
        let trace = Rc::new(RefCell::new(Vec::<String>::new()));
        let log = trace.clone();
        model
            .property_changed()
            .add(Rc::new(move |n| log.borrow_mut().push(n.into())));
        let case = fixture["mutations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["type"] == "RibbonSliderModel" && c["property"] == "SliderWidth")
            .unwrap();
        let mut actual = vec![];
        for value in case["values"].as_array().unwrap() {
            model.set_slider_width(d(value));
            actual.push(json!({"value":ed(model.slider_width()),"events":*trace.borrow()}));
            trace.borrow_mut().clear();
        }
        assert_eq!(
            json!(actual),
            case["states"],
            "RibbonSliderModel.SliderWidth"
        );
    }
}
#[test]
fn source_ribbon_grid_picker_model_defaults_and_setter_traces() {
    let fixture = corpus();
    let model = RibbonGridPickerModel::new(Some("x".into()), Some("X".into()));
    let mut properties = serde_json::Map::new();
    properties.insert("Rows".into(), ei(model.rows()));
    properties.insert("Columns".into(), ei(model.columns()));
    assert_eq!(
        json!({"size":model.size().raw(),"qat":model.can_add_to_quick_access(),"label":es(model.label()),"properties":properties}),
        fixture["defaults"]["RibbonGridPickerModel"]
    );
    {
        let model = RibbonGridPickerModel::new(Some("x".into()), Some("X".into()));
        let trace = Rc::new(RefCell::new(Vec::<String>::new()));
        let log = trace.clone();
        model
            .property_changed()
            .add(Rc::new(move |n| log.borrow_mut().push(n.into())));
        let case = fixture["mutations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["type"] == "RibbonGridPickerModel" && c["property"] == "Rows")
            .unwrap();
        let mut actual = vec![];
        for value in case["values"].as_array().unwrap() {
            model.set_rows(i(value));
            actual.push(json!({"value":ei(model.rows()),"events":*trace.borrow()}));
            trace.borrow_mut().clear();
        }
        assert_eq!(json!(actual), case["states"], "RibbonGridPickerModel.Rows");
    }
    {
        let model = RibbonGridPickerModel::new(Some("x".into()), Some("X".into()));
        let trace = Rc::new(RefCell::new(Vec::<String>::new()));
        let log = trace.clone();
        model
            .property_changed()
            .add(Rc::new(move |n| log.borrow_mut().push(n.into())));
        let case = fixture["mutations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["type"] == "RibbonGridPickerModel" && c["property"] == "Columns")
            .unwrap();
        let mut actual = vec![];
        for value in case["values"].as_array().unwrap() {
            model.set_columns(i(value));
            actual.push(json!({"value":ei(model.columns()),"events":*trace.borrow()}));
            trace.borrow_mut().clear();
        }
        assert_eq!(
            json!(actual),
            case["states"],
            "RibbonGridPickerModel.Columns"
        );
    }
}
#[test]
fn source_ribbon_custom_item_model_defaults_and_setter_traces() {
    let fixture = corpus();
    let model = RibbonCustomItemModel::new(Some("x".into()), None);
    let mut properties = serde_json::Map::new();
    properties.insert("Content".into(), eo(model.content()));
    properties.insert("TemplateKey".into(), es(model.template_key()));
    assert_eq!(
        json!({"size":model.size().raw(),"qat":model.can_add_to_quick_access(),"label":es(model.label()),"properties":properties}),
        fixture["defaults"]["RibbonCustomItemModel"]
    );
    {
        let model = RibbonCustomItemModel::new(Some("x".into()), None);
        let trace = Rc::new(RefCell::new(Vec::<String>::new()));
        let log = trace.clone();
        model
            .property_changed()
            .add(Rc::new(move |n| log.borrow_mut().push(n.into())));
        let case = fixture["mutations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["type"] == "RibbonCustomItemModel" && c["property"] == "Content")
            .unwrap();
        let mut actual = vec![];
        for value in case["values"].as_array().unwrap() {
            model.set_content(o(value));
            actual.push(json!({"value":eo(model.content()),"events":*trace.borrow()}));
            trace.borrow_mut().clear();
        }
        assert_eq!(
            json!(actual),
            case["states"],
            "RibbonCustomItemModel.Content"
        );
    }
    {
        let model = RibbonCustomItemModel::new(Some("x".into()), None);
        let trace = Rc::new(RefCell::new(Vec::<String>::new()));
        let log = trace.clone();
        model
            .property_changed()
            .add(Rc::new(move |n| log.borrow_mut().push(n.into())));
        let case = fixture["mutations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["type"] == "RibbonCustomItemModel" && c["property"] == "TemplateKey")
            .unwrap();
        let mut actual = vec![];
        for value in case["values"].as_array().unwrap() {
            model.set_template_key(s(value));
            actual.push(json!({"value":es(model.template_key()),"events":*trace.borrow()}));
            trace.borrow_mut().clear();
        }
        assert_eq!(
            json!(actual),
            case["states"],
            "RibbonCustomItemModel.TemplateKey"
        );
    }
}
#[test]
fn source_ribbon_label_model_defaults_and_setter_traces() {
    let fixture = corpus();
    let model = RibbonLabelModel::new(Some("x".into()), Some("X".into()));
    let properties = serde_json::Map::new();
    assert_eq!(
        json!({"size":model.size().raw(),"qat":model.can_add_to_quick_access(),"label":es(model.label()),"properties":properties}),
        fixture["defaults"]["RibbonLabelModel"]
    );
}
#[test]
fn source_ribbon_separator_model_defaults_and_setter_traces() {
    let fixture = corpus();
    let model = RibbonSeparatorModel::new();
    let properties = serde_json::Map::new();
    assert_eq!(
        json!({"size":model.size().raw(),"qat":model.can_add_to_quick_access(),"label":es(model.label()),"properties":properties}),
        fixture["defaults"]["RibbonSeparatorModel"]
    );
}

struct DerivedCombo {
    base: RibbonComboBoxModel,
    trace: Rc<RefCell<Vec<String>>>,
}
impl INotifyPropertyChanged for DerivedCombo {
    fn property_changed(&self) -> &ferroui_system::events::Event<str> {
        self.base.property_changed()
    }
}
impl ferroribbon_core::ObservableObjectImpl for DerivedCombo {
    fn on_property_changed(&self, name: &str) {
        self.trace.borrow_mut().push(name.into());
        if name == "Size" {
            let log = self.trace.clone();
            self.items()
                .add_collection_changed(Rc::new(move |_| log.borrow_mut().push("add".into())));
        }
        self.property_changed().raise(name);
    }
}
impl RibbonNodeModelImpl for DerivedCombo {
    fn node_model(&self) -> &RibbonNodeModel {
        self.base.item_model().node_model()
    }
}
impl RibbonItemModelImpl for DerivedCombo {
    fn item_model(&self) -> &RibbonItemModel {
        self.base.item_model()
    }
}
impl RibbonComboBoxModelImpl for DerivedCombo {
    fn ribbon_combo_box_model(&self) -> &RibbonComboBoxModel {
        &self.base
    }
}
#[test]
fn lazy_combo_constructor_keeps_source_order_and_partial_collection_on_iterator_failure() {
    let trace = Rc::new(RefCell::new(Vec::<String>::new()));
    let model = DerivedCombo {
        base: RibbonComboBoxModel::construct(Some("x".into()), Some("X".into())),
        trace: trace.clone(),
    };
    let mut step = 0;
    let log = trace.clone();
    let iterator = std::iter::from_fn(move || {
        step += 1;
        if step == 1 {
            log.borrow_mut().push("enumerate".into());
            Some(Some(RibbonObject::value(RibbonString::from("one"))))
        } else {
            panic!("iteration failure")
        }
    });
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            RibbonComboBoxModel::initialize(&model, iterator)
        }))
        .is_err()
    );
    trace.borrow_mut().push("throw".into());
    assert_eq!(json!(*trace.borrow()), corpus()["comboTrace"]);
    assert_eq!(json!(model.items().count()), corpus()["partialCount"]);
}
#[test]
fn selection_and_content_retain_payload_and_collection_identity() {
    let payload = RibbonObject::reference(Rc::new(42_i32));
    let combo = RibbonComboBoxModel::with_items(
        None,
        None,
        [Some(payload.clone()), None, Some(payload.clone())],
    );
    assert_eq!(combo.items().count(), 3);
    assert!(combo.items().ptr_eq(&combo.items()));
    assert!(combo.items().get(0).unwrap().ptr_eq(&payload));
    assert!(combo.items().get(1).is_none());
    combo.set_selected_item(Some(payload.clone()));
    assert!(combo.selected_item().unwrap().ptr_eq(&payload));
    let custom = RibbonCustomItemModel::new(None, Some(payload.clone()));
    assert!(custom.content().unwrap().ptr_eq(&payload));
    assert!(custom.label().is_none());
    assert_eq!(
        RibbonCheckBoxModel::with_checked(None, None, true).is_checked(),
        Some(true)
    );
    assert_eq!(
        RibbonSpinnerModel::with_value(None, None, -42.0).value(),
        -42.0
    );
}

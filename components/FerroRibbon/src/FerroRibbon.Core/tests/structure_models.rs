use ferroribbon_core::{
    RibbonBackstagePlacement, RibbonContextualActivation, RibbonGroupItemsLayout,
    RibbonSimplifiedVisibility, RibbonString, model::*, theming::RibbonColor,
};
use ferroui_system::component_model::INotifyPropertyChanged;
use serde_json::{Value, json};
use std::{cell::RefCell, rc::Rc};
fn corpus() -> Value {
    serde_json::from_str(include_str!("../../../tests/fixtures/structures.json")).unwrap()
}
fn s(v: &Value) -> Option<RibbonString> {
    v.as_array().map(|a| {
        RibbonString::from_utf16(
            a.iter()
                .map(|v| v.as_u64().unwrap() as u16)
                .collect::<Vec<_>>(),
        )
    })
}
fn es(v: Option<RibbonString>) -> Value {
    v.map_or(Value::Null, |v| json!(v.as_utf16()))
}
fn o(v: &Value) -> Option<RibbonObject> {
    s(v).map(RibbonObject::value)
}
fn eo(v: Option<RibbonObject>) -> Value {
    v.map_or(Value::Null, |v| {
        es(Some(v.downcast_ref::<RibbonString>().unwrap().clone()))
    })
}
fn i(v: &Value) -> i32 {
    v.as_i64().unwrap() as i32
}
fn ei(v: i32) -> Value {
    json!(v)
}
fn b(v: &Value) -> bool {
    v.as_bool().unwrap()
}
fn eb(v: bool) -> Value {
    json!(v)
}
fn d(v: &Value) -> f64 {
    f64::from_bits(u64::from_str_radix(v.as_str().unwrap(), 16).unwrap())
}
fn ed(v: f64) -> Value {
    json!(format!("{:016x}", v.to_bits()))
}
fn c(v: &Value) -> RibbonColor {
    RibbonColor::parse(Some(&v.as_str().unwrap().into())).unwrap()
}
fn ec(v: RibbonColor) -> Value {
    json!(v.to_hex(true))
}
fn oc(v: &Value) -> Option<RibbonColor> {
    if v.is_null() { None } else { Some(c(v)) }
}
fn eoc(v: Option<RibbonColor>) -> Value {
    v.map_or(Value::Null, ec)
}

fn ge(v: &Value) -> RibbonGroupItemsLayout {
    RibbonGroupItemsLayout::from_raw(i(v))
}
fn ege(v: RibbonGroupItemsLayout) -> Value {
    json!(v.raw())
}
fn se(v: &Value) -> RibbonSimplifiedVisibility {
    RibbonSimplifiedVisibility::from_raw(i(v))
}
fn ese(v: RibbonSimplifiedVisibility) -> Value {
    json!(v.raw())
}
fn ae(v: &Value) -> RibbonContextualActivation {
    RibbonContextualActivation::from_raw(i(v))
}
fn eae(v: RibbonContextualActivation) -> Value {
    json!(v.raw())
}
fn pe(v: &Value) -> RibbonBackstagePlacement {
    RibbonBackstagePlacement::from_raw(i(v))
}
fn epe(v: RibbonBackstagePlacement) -> Value {
    json!(v.raw())
}
#[test]
fn source_ribbon_group_model_defaults_and_setters() {
    let source = corpus();
    let model = RibbonGroupModel::new(Some("x".into()), Some("X".into()));
    let mut props = serde_json::Map::new();
    props.insert("ItemsLayout".into(), ege(model.items_layout()));
    props.insert("RowCount".into(), ei(model.row_count()));
    props.insert(
        "DialogLauncherCommandId".into(),
        es(model.dialog_launcher_command_id()),
    );
    props.insert("ReductionOrder".into(), ei(model.reduction_order()));
    props.insert("CanCollapse".into(), eb(model.can_collapse()));
    props.insert(
        "SimplifiedVisibility".into(),
        ese(model.simplified_visibility()),
    );
    assert!(model.dialog_launcher_command().is_none());
    props.insert("DialogLauncherCommand".into(), Value::Null);
    assert!(model.dialog_launcher_screen_tip().is_none());
    props.insert("DialogLauncherScreenTip".into(), Value::Null);
    assert_eq!(
        json!({"size":Value::Null,"visible":json!(model.is_visible()),"properties":props}),
        source["defaults"]["RibbonGroupModel"]
    );
    {
        let model = RibbonGroupModel::new(Some("x".into()), Some("X".into()));
        let trace = Rc::new(RefCell::new(Vec::<String>::new()));
        let log = trace.clone();
        model
            .property_changed()
            .add(Rc::new(move |n| log.borrow_mut().push(n.into())));
        let case = source["mutations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["type"] == "RibbonGroupModel" && r["property"] == "ItemsLayout")
            .unwrap();
        let mut states = vec![];
        for value in case["values"].as_array().unwrap() {
            model.set_items_layout(ge(value));
            states.push(json!({"value":ege(model.items_layout()),"events":*trace.borrow()}));
            trace.borrow_mut().clear();
        }
        assert_eq!(
            json!(states),
            case["states"],
            "RibbonGroupModel.ItemsLayout"
        );
    }
    {
        let model = RibbonGroupModel::new(Some("x".into()), Some("X".into()));
        let trace = Rc::new(RefCell::new(Vec::<String>::new()));
        let log = trace.clone();
        model
            .property_changed()
            .add(Rc::new(move |n| log.borrow_mut().push(n.into())));
        let case = source["mutations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["type"] == "RibbonGroupModel" && r["property"] == "RowCount")
            .unwrap();
        let mut states = vec![];
        for value in case["values"].as_array().unwrap() {
            model.set_row_count(i(value));
            states.push(json!({"value":ei(model.row_count()),"events":*trace.borrow()}));
            trace.borrow_mut().clear();
        }
        assert_eq!(json!(states), case["states"], "RibbonGroupModel.RowCount");
    }
    {
        let model = RibbonGroupModel::new(Some("x".into()), Some("X".into()));
        let trace = Rc::new(RefCell::new(Vec::<String>::new()));
        let log = trace.clone();
        model
            .property_changed()
            .add(Rc::new(move |n| log.borrow_mut().push(n.into())));
        let case = source["mutations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["type"] == "RibbonGroupModel" && r["property"] == "DialogLauncherCommandId")
            .unwrap();
        let mut states = vec![];
        for value in case["values"].as_array().unwrap() {
            model.set_dialog_launcher_command_id(s(value));
            states.push(
                json!({"value":es(model.dialog_launcher_command_id()),"events":*trace.borrow()}),
            );
            trace.borrow_mut().clear();
        }
        assert_eq!(
            json!(states),
            case["states"],
            "RibbonGroupModel.DialogLauncherCommandId"
        );
    }
    {
        let model = RibbonGroupModel::new(Some("x".into()), Some("X".into()));
        let trace = Rc::new(RefCell::new(Vec::<String>::new()));
        let log = trace.clone();
        model
            .property_changed()
            .add(Rc::new(move |n| log.borrow_mut().push(n.into())));
        let case = source["mutations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["type"] == "RibbonGroupModel" && r["property"] == "ReductionOrder")
            .unwrap();
        let mut states = vec![];
        for value in case["values"].as_array().unwrap() {
            model.set_reduction_order(i(value));
            states.push(json!({"value":ei(model.reduction_order()),"events":*trace.borrow()}));
            trace.borrow_mut().clear();
        }
        assert_eq!(
            json!(states),
            case["states"],
            "RibbonGroupModel.ReductionOrder"
        );
    }
    {
        let model = RibbonGroupModel::new(Some("x".into()), Some("X".into()));
        let trace = Rc::new(RefCell::new(Vec::<String>::new()));
        let log = trace.clone();
        model
            .property_changed()
            .add(Rc::new(move |n| log.borrow_mut().push(n.into())));
        let case = source["mutations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["type"] == "RibbonGroupModel" && r["property"] == "CanCollapse")
            .unwrap();
        let mut states = vec![];
        for value in case["values"].as_array().unwrap() {
            model.set_can_collapse(b(value));
            states.push(json!({"value":eb(model.can_collapse()),"events":*trace.borrow()}));
            trace.borrow_mut().clear();
        }
        assert_eq!(
            json!(states),
            case["states"],
            "RibbonGroupModel.CanCollapse"
        );
    }
    {
        let model = RibbonGroupModel::new(Some("x".into()), Some("X".into()));
        let trace = Rc::new(RefCell::new(Vec::<String>::new()));
        let log = trace.clone();
        model
            .property_changed()
            .add(Rc::new(move |n| log.borrow_mut().push(n.into())));
        let case = source["mutations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["type"] == "RibbonGroupModel" && r["property"] == "SimplifiedVisibility")
            .unwrap();
        let mut states = vec![];
        for value in case["values"].as_array().unwrap() {
            model.set_simplified_visibility(se(value));
            states
                .push(json!({"value":ese(model.simplified_visibility()),"events":*trace.borrow()}));
            trace.borrow_mut().clear();
        }
        assert_eq!(
            json!(states),
            case["states"],
            "RibbonGroupModel.SimplifiedVisibility"
        );
    }
}
#[test]
fn source_ribbon_tab_model_defaults_and_setters() {
    let source = corpus();
    let model = RibbonTabModel::new(Some("x".into()), Some("X".into()));
    let mut props = serde_json::Map::new();
    props.insert("ContextualGroupId".into(), es(model.contextual_group_id()));
    assert_eq!(
        json!({"size":Value::Null,"visible":json!(model.is_visible()),"properties":props}),
        source["defaults"]["RibbonTabModel"]
    );
    {
        let model = RibbonTabModel::new(Some("x".into()), Some("X".into()));
        let trace = Rc::new(RefCell::new(Vec::<String>::new()));
        let log = trace.clone();
        model
            .property_changed()
            .add(Rc::new(move |n| log.borrow_mut().push(n.into())));
        let case = source["mutations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["type"] == "RibbonTabModel" && r["property"] == "ContextualGroupId")
            .unwrap();
        let mut states = vec![];
        for value in case["values"].as_array().unwrap() {
            model.set_contextual_group_id(s(value));
            states.push(json!({"value":es(model.contextual_group_id()),"events":*trace.borrow()}));
            trace.borrow_mut().clear();
        }
        assert_eq!(
            json!(states),
            case["states"],
            "RibbonTabModel.ContextualGroupId"
        );
    }
}
#[test]
fn source_ribbon_contextual_group_model_defaults_and_setters() {
    let source = corpus();
    let model = RibbonContextualGroupModel::new(Some("x".into()), Some("X".into()));
    let mut props = serde_json::Map::new();
    props.insert("Color".into(), ec(model.color()));
    props.insert("Activation".into(), eae(model.activation()));
    assert_eq!(
        json!({"size":Value::Null,"visible":json!(model.is_visible()),"properties":props}),
        source["defaults"]["RibbonContextualGroupModel"]
    );
    {
        let model = RibbonContextualGroupModel::new(Some("x".into()), Some("X".into()));
        let trace = Rc::new(RefCell::new(Vec::<String>::new()));
        let log = trace.clone();
        model
            .property_changed()
            .add(Rc::new(move |n| log.borrow_mut().push(n.into())));
        let case = source["mutations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["type"] == "RibbonContextualGroupModel" && r["property"] == "Color")
            .unwrap();
        let mut states = vec![];
        for value in case["values"].as_array().unwrap() {
            model.set_color(c(value));
            states.push(json!({"value":ec(model.color()),"events":*trace.borrow()}));
            trace.borrow_mut().clear();
        }
        assert_eq!(
            json!(states),
            case["states"],
            "RibbonContextualGroupModel.Color"
        );
    }
    {
        let model = RibbonContextualGroupModel::new(Some("x".into()), Some("X".into()));
        let trace = Rc::new(RefCell::new(Vec::<String>::new()));
        let log = trace.clone();
        model
            .property_changed()
            .add(Rc::new(move |n| log.borrow_mut().push(n.into())));
        let case = source["mutations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["type"] == "RibbonContextualGroupModel" && r["property"] == "Activation")
            .unwrap();
        let mut states = vec![];
        for value in case["values"].as_array().unwrap() {
            model.set_activation(ae(value));
            states.push(json!({"value":eae(model.activation()),"events":*trace.borrow()}));
            trace.borrow_mut().clear();
        }
        assert_eq!(
            json!(states),
            case["states"],
            "RibbonContextualGroupModel.Activation"
        );
    }
}
#[test]
fn source_ribbon_backstage_item_model_defaults_and_setters() {
    let source = corpus();
    let model = RibbonBackstageItemModel::new(Some("x".into()), Some("X".into()));
    let mut props = serde_json::Map::new();
    props.insert("Content".into(), eo(model.content()));
    props.insert("CommandId".into(), es(model.command_id()));
    props.insert("CommandParameter".into(), eo(model.command_parameter()));
    props.insert("Placement".into(), epe(model.placement()));
    props.insert("ClosesBackstage".into(), eb(model.closes_backstage()));
    props.insert(
        "HasSeparatorBefore".into(),
        eb(model.has_separator_before()),
    );
    assert!(model.command().is_none());
    props.insert("Command".into(), Value::Null);
    assert_eq!(
        json!({"size":Value::Null,"visible":json!(model.is_visible()),"properties":props}),
        source["defaults"]["RibbonBackstageItemModel"]
    );
    {
        let model = RibbonBackstageItemModel::new(Some("x".into()), Some("X".into()));
        let trace = Rc::new(RefCell::new(Vec::<String>::new()));
        let log = trace.clone();
        model
            .property_changed()
            .add(Rc::new(move |n| log.borrow_mut().push(n.into())));
        let case = source["mutations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["type"] == "RibbonBackstageItemModel" && r["property"] == "Content")
            .unwrap();
        let mut states = vec![];
        for value in case["values"].as_array().unwrap() {
            model.set_content(o(value));
            states.push(json!({"value":eo(model.content()),"events":*trace.borrow()}));
            trace.borrow_mut().clear();
        }
        assert_eq!(
            json!(states),
            case["states"],
            "RibbonBackstageItemModel.Content"
        );
    }
    {
        let model = RibbonBackstageItemModel::new(Some("x".into()), Some("X".into()));
        let trace = Rc::new(RefCell::new(Vec::<String>::new()));
        let log = trace.clone();
        model
            .property_changed()
            .add(Rc::new(move |n| log.borrow_mut().push(n.into())));
        let case = source["mutations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["type"] == "RibbonBackstageItemModel" && r["property"] == "CommandId")
            .unwrap();
        let mut states = vec![];
        for value in case["values"].as_array().unwrap() {
            model.set_command_id(s(value));
            states.push(json!({"value":es(model.command_id()),"events":*trace.borrow()}));
            trace.borrow_mut().clear();
        }
        assert_eq!(
            json!(states),
            case["states"],
            "RibbonBackstageItemModel.CommandId"
        );
    }
    {
        let model = RibbonBackstageItemModel::new(Some("x".into()), Some("X".into()));
        let trace = Rc::new(RefCell::new(Vec::<String>::new()));
        let log = trace.clone();
        model
            .property_changed()
            .add(Rc::new(move |n| log.borrow_mut().push(n.into())));
        let case = source["mutations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| {
                r["type"] == "RibbonBackstageItemModel" && r["property"] == "CommandParameter"
            })
            .unwrap();
        let mut states = vec![];
        for value in case["values"].as_array().unwrap() {
            model.set_command_parameter(o(value));
            states.push(json!({"value":eo(model.command_parameter()),"events":*trace.borrow()}));
            trace.borrow_mut().clear();
        }
        assert_eq!(
            json!(states),
            case["states"],
            "RibbonBackstageItemModel.CommandParameter"
        );
    }
    {
        let model = RibbonBackstageItemModel::new(Some("x".into()), Some("X".into()));
        let trace = Rc::new(RefCell::new(Vec::<String>::new()));
        let log = trace.clone();
        model
            .property_changed()
            .add(Rc::new(move |n| log.borrow_mut().push(n.into())));
        let case = source["mutations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["type"] == "RibbonBackstageItemModel" && r["property"] == "Placement")
            .unwrap();
        let mut states = vec![];
        for value in case["values"].as_array().unwrap() {
            model.set_placement(pe(value));
            states.push(json!({"value":epe(model.placement()),"events":*trace.borrow()}));
            trace.borrow_mut().clear();
        }
        assert_eq!(
            json!(states),
            case["states"],
            "RibbonBackstageItemModel.Placement"
        );
    }
    {
        let model = RibbonBackstageItemModel::new(Some("x".into()), Some("X".into()));
        let trace = Rc::new(RefCell::new(Vec::<String>::new()));
        let log = trace.clone();
        model
            .property_changed()
            .add(Rc::new(move |n| log.borrow_mut().push(n.into())));
        let case = source["mutations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["type"] == "RibbonBackstageItemModel" && r["property"] == "ClosesBackstage")
            .unwrap();
        let mut states = vec![];
        for value in case["values"].as_array().unwrap() {
            model.set_closes_backstage(b(value));
            states.push(json!({"value":eb(model.closes_backstage()),"events":*trace.borrow()}));
            trace.borrow_mut().clear();
        }
        assert_eq!(
            json!(states),
            case["states"],
            "RibbonBackstageItemModel.ClosesBackstage"
        );
    }
    {
        let model = RibbonBackstageItemModel::new(Some("x".into()), Some("X".into()));
        let trace = Rc::new(RefCell::new(Vec::<String>::new()));
        let log = trace.clone();
        model
            .property_changed()
            .add(Rc::new(move |n| log.borrow_mut().push(n.into())));
        let case = source["mutations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| {
                r["type"] == "RibbonBackstageItemModel" && r["property"] == "HasSeparatorBefore"
            })
            .unwrap();
        let mut states = vec![];
        for value in case["values"].as_array().unwrap() {
            model.set_has_separator_before(b(value));
            states.push(json!({"value":eb(model.has_separator_before()),"events":*trace.borrow()}));
            trace.borrow_mut().clear();
        }
        assert_eq!(
            json!(states),
            case["states"],
            "RibbonBackstageItemModel.HasSeparatorBefore"
        );
    }
}
#[test]
fn source_ribbon_backstage_model_defaults_and_setters() {
    let source = corpus();
    let model = RibbonBackstageModel::new();
    let mut props = serde_json::Map::new();
    props.insert("IsOpen".into(), eb(model.is_open()));
    props.insert("Title".into(), es(model.title()));
    assert!(model.selected_item().is_none());
    props.insert("SelectedItem".into(), Value::Null);
    assert_eq!(
        json!({"size":Value::Null,"visible":Value::Null,"properties":props}),
        source["defaults"]["RibbonBackstageModel"]
    );
    {
        let model = RibbonBackstageModel::new();
        let trace = Rc::new(RefCell::new(Vec::<String>::new()));
        let log = trace.clone();
        model
            .property_changed()
            .add(Rc::new(move |n| log.borrow_mut().push(n.into())));
        let case = source["mutations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["type"] == "RibbonBackstageModel" && r["property"] == "IsOpen")
            .unwrap();
        let mut states = vec![];
        for value in case["values"].as_array().unwrap() {
            model.set_is_open(b(value));
            states.push(json!({"value":eb(model.is_open()),"events":*trace.borrow()}));
            trace.borrow_mut().clear();
        }
        assert_eq!(json!(states), case["states"], "RibbonBackstageModel.IsOpen");
    }
    {
        let model = RibbonBackstageModel::new();
        let trace = Rc::new(RefCell::new(Vec::<String>::new()));
        let log = trace.clone();
        model
            .property_changed()
            .add(Rc::new(move |n| log.borrow_mut().push(n.into())));
        let case = source["mutations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["type"] == "RibbonBackstageModel" && r["property"] == "Title")
            .unwrap();
        let mut states = vec![];
        for value in case["values"].as_array().unwrap() {
            model.set_title(s(value));
            states.push(json!({"value":es(model.title()),"events":*trace.borrow()}));
            trace.borrow_mut().clear();
        }
        assert_eq!(json!(states), case["states"], "RibbonBackstageModel.Title");
    }
}
#[test]
fn source_ribbon_gallery_item_model_defaults_and_setters() {
    let source = corpus();
    let model = RibbonGalleryItemModel::new(Some("x".into()), Some("X".into()));
    let mut props = serde_json::Map::new();
    props.insert("Category".into(), es(model.category()));
    props.insert("Content".into(), eo(model.content()));
    props.insert("Value".into(), eo(model.value()));
    assert_eq!(
        json!({"size":Value::Null,"visible":json!(model.is_visible()),"properties":props}),
        source["defaults"]["RibbonGalleryItemModel"]
    );
    {
        let model = RibbonGalleryItemModel::new(Some("x".into()), Some("X".into()));
        let trace = Rc::new(RefCell::new(Vec::<String>::new()));
        let log = trace.clone();
        model
            .property_changed()
            .add(Rc::new(move |n| log.borrow_mut().push(n.into())));
        let case = source["mutations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["type"] == "RibbonGalleryItemModel" && r["property"] == "Category")
            .unwrap();
        let mut states = vec![];
        for value in case["values"].as_array().unwrap() {
            model.set_category(s(value));
            states.push(json!({"value":es(model.category()),"events":*trace.borrow()}));
            trace.borrow_mut().clear();
        }
        assert_eq!(
            json!(states),
            case["states"],
            "RibbonGalleryItemModel.Category"
        );
    }
    {
        let model = RibbonGalleryItemModel::new(Some("x".into()), Some("X".into()));
        let trace = Rc::new(RefCell::new(Vec::<String>::new()));
        let log = trace.clone();
        model
            .property_changed()
            .add(Rc::new(move |n| log.borrow_mut().push(n.into())));
        let case = source["mutations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["type"] == "RibbonGalleryItemModel" && r["property"] == "Content")
            .unwrap();
        let mut states = vec![];
        for value in case["values"].as_array().unwrap() {
            model.set_content(o(value));
            states.push(json!({"value":eo(model.content()),"events":*trace.borrow()}));
            trace.borrow_mut().clear();
        }
        assert_eq!(
            json!(states),
            case["states"],
            "RibbonGalleryItemModel.Content"
        );
    }
    {
        let model = RibbonGalleryItemModel::new(Some("x".into()), Some("X".into()));
        let trace = Rc::new(RefCell::new(Vec::<String>::new()));
        let log = trace.clone();
        model
            .property_changed()
            .add(Rc::new(move |n| log.borrow_mut().push(n.into())));
        let case = source["mutations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["type"] == "RibbonGalleryItemModel" && r["property"] == "Value")
            .unwrap();
        let mut states = vec![];
        for value in case["values"].as_array().unwrap() {
            model.set_value(o(value));
            states.push(json!({"value":eo(model.value()),"events":*trace.borrow()}));
            trace.borrow_mut().clear();
        }
        assert_eq!(
            json!(states),
            case["states"],
            "RibbonGalleryItemModel.Value"
        );
    }
}
#[test]
fn source_ribbon_gallery_model_defaults_and_setters() {
    let source = corpus();
    let model = RibbonGalleryModel::new(Some("x".into()), Some("X".into()));
    let mut props = serde_json::Map::new();
    props.insert("MinColumns".into(), ei(model.min_columns()));
    props.insert("MaxColumns".into(), ei(model.max_columns()));
    props.insert("DropDownColumns".into(), ei(model.drop_down_columns()));
    props.insert("ItemWidth".into(), ed(model.item_width()));
    props.insert("ItemHeight".into(), ed(model.item_height()));
    props.insert("IsFilterEnabled".into(), eb(model.is_filter_enabled()));
    props.insert("ShowLabels".into(), eb(model.show_labels()));
    props.insert("Rows".into(), ei(model.rows()));
    assert!(model.selected_item().is_none());
    props.insert("SelectedItem".into(), Value::Null);
    assert!(model.preview_command().is_none());
    props.insert("PreviewCommand".into(), Value::Null);
    assert_eq!(
        json!({"size":json!(model.size().raw()),"visible":json!(model.is_visible()),"properties":props}),
        source["defaults"]["RibbonGalleryModel"]
    );
    {
        let model = RibbonGalleryModel::new(Some("x".into()), Some("X".into()));
        let trace = Rc::new(RefCell::new(Vec::<String>::new()));
        let log = trace.clone();
        model
            .property_changed()
            .add(Rc::new(move |n| log.borrow_mut().push(n.into())));
        let case = source["mutations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["type"] == "RibbonGalleryModel" && r["property"] == "MinColumns")
            .unwrap();
        let mut states = vec![];
        for value in case["values"].as_array().unwrap() {
            model.set_min_columns(i(value));
            states.push(json!({"value":ei(model.min_columns()),"events":*trace.borrow()}));
            trace.borrow_mut().clear();
        }
        assert_eq!(
            json!(states),
            case["states"],
            "RibbonGalleryModel.MinColumns"
        );
    }
    {
        let model = RibbonGalleryModel::new(Some("x".into()), Some("X".into()));
        let trace = Rc::new(RefCell::new(Vec::<String>::new()));
        let log = trace.clone();
        model
            .property_changed()
            .add(Rc::new(move |n| log.borrow_mut().push(n.into())));
        let case = source["mutations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["type"] == "RibbonGalleryModel" && r["property"] == "MaxColumns")
            .unwrap();
        let mut states = vec![];
        for value in case["values"].as_array().unwrap() {
            model.set_max_columns(i(value));
            states.push(json!({"value":ei(model.max_columns()),"events":*trace.borrow()}));
            trace.borrow_mut().clear();
        }
        assert_eq!(
            json!(states),
            case["states"],
            "RibbonGalleryModel.MaxColumns"
        );
    }
    {
        let model = RibbonGalleryModel::new(Some("x".into()), Some("X".into()));
        let trace = Rc::new(RefCell::new(Vec::<String>::new()));
        let log = trace.clone();
        model
            .property_changed()
            .add(Rc::new(move |n| log.borrow_mut().push(n.into())));
        let case = source["mutations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["type"] == "RibbonGalleryModel" && r["property"] == "DropDownColumns")
            .unwrap();
        let mut states = vec![];
        for value in case["values"].as_array().unwrap() {
            model.set_drop_down_columns(i(value));
            states.push(json!({"value":ei(model.drop_down_columns()),"events":*trace.borrow()}));
            trace.borrow_mut().clear();
        }
        assert_eq!(
            json!(states),
            case["states"],
            "RibbonGalleryModel.DropDownColumns"
        );
    }
    {
        let model = RibbonGalleryModel::new(Some("x".into()), Some("X".into()));
        let trace = Rc::new(RefCell::new(Vec::<String>::new()));
        let log = trace.clone();
        model
            .property_changed()
            .add(Rc::new(move |n| log.borrow_mut().push(n.into())));
        let case = source["mutations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["type"] == "RibbonGalleryModel" && r["property"] == "ItemWidth")
            .unwrap();
        let mut states = vec![];
        for value in case["values"].as_array().unwrap() {
            model.set_item_width(d(value));
            states.push(json!({"value":ed(model.item_width()),"events":*trace.borrow()}));
            trace.borrow_mut().clear();
        }
        assert_eq!(
            json!(states),
            case["states"],
            "RibbonGalleryModel.ItemWidth"
        );
    }
    {
        let model = RibbonGalleryModel::new(Some("x".into()), Some("X".into()));
        let trace = Rc::new(RefCell::new(Vec::<String>::new()));
        let log = trace.clone();
        model
            .property_changed()
            .add(Rc::new(move |n| log.borrow_mut().push(n.into())));
        let case = source["mutations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["type"] == "RibbonGalleryModel" && r["property"] == "ItemHeight")
            .unwrap();
        let mut states = vec![];
        for value in case["values"].as_array().unwrap() {
            model.set_item_height(d(value));
            states.push(json!({"value":ed(model.item_height()),"events":*trace.borrow()}));
            trace.borrow_mut().clear();
        }
        assert_eq!(
            json!(states),
            case["states"],
            "RibbonGalleryModel.ItemHeight"
        );
    }
    {
        let model = RibbonGalleryModel::new(Some("x".into()), Some("X".into()));
        let trace = Rc::new(RefCell::new(Vec::<String>::new()));
        let log = trace.clone();
        model
            .property_changed()
            .add(Rc::new(move |n| log.borrow_mut().push(n.into())));
        let case = source["mutations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["type"] == "RibbonGalleryModel" && r["property"] == "IsFilterEnabled")
            .unwrap();
        let mut states = vec![];
        for value in case["values"].as_array().unwrap() {
            model.set_is_filter_enabled(b(value));
            states.push(json!({"value":eb(model.is_filter_enabled()),"events":*trace.borrow()}));
            trace.borrow_mut().clear();
        }
        assert_eq!(
            json!(states),
            case["states"],
            "RibbonGalleryModel.IsFilterEnabled"
        );
    }
    {
        let model = RibbonGalleryModel::new(Some("x".into()), Some("X".into()));
        let trace = Rc::new(RefCell::new(Vec::<String>::new()));
        let log = trace.clone();
        model
            .property_changed()
            .add(Rc::new(move |n| log.borrow_mut().push(n.into())));
        let case = source["mutations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["type"] == "RibbonGalleryModel" && r["property"] == "ShowLabels")
            .unwrap();
        let mut states = vec![];
        for value in case["values"].as_array().unwrap() {
            model.set_show_labels(b(value));
            states.push(json!({"value":eb(model.show_labels()),"events":*trace.borrow()}));
            trace.borrow_mut().clear();
        }
        assert_eq!(
            json!(states),
            case["states"],
            "RibbonGalleryModel.ShowLabels"
        );
    }
    {
        let model = RibbonGalleryModel::new(Some("x".into()), Some("X".into()));
        let trace = Rc::new(RefCell::new(Vec::<String>::new()));
        let log = trace.clone();
        model
            .property_changed()
            .add(Rc::new(move |n| log.borrow_mut().push(n.into())));
        let case = source["mutations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["type"] == "RibbonGalleryModel" && r["property"] == "Rows")
            .unwrap();
        let mut states = vec![];
        for value in case["values"].as_array().unwrap() {
            model.set_rows(i(value));
            states.push(json!({"value":ei(model.rows()),"events":*trace.borrow()}));
            trace.borrow_mut().clear();
        }
        assert_eq!(json!(states), case["states"], "RibbonGalleryModel.Rows");
    }
}
#[test]
fn source_ribbon_color_picker_model_defaults_and_setters() {
    let source = corpus();
    let model = RibbonColorPickerModel::new(Some("x".into()), Some("X".into()));
    let mut props = serde_json::Map::new();
    props.insert("SelectedColor".into(), eoc(model.selected_color()));
    props.insert("ShowAutomatic".into(), eb(model.show_automatic()));
    props.insert("ShowNoColor".into(), eb(model.show_no_color()));
    props.insert("ShowMoreColors".into(), eb(model.show_more_colors()));
    props.insert("IsSplit".into(), eb(model.is_split()));
    props.insert("AutomaticColor".into(), ec(model.automatic_color()));
    assert_eq!(
        json!({"size":json!(model.size().raw()),"visible":json!(model.is_visible()),"properties":props}),
        source["defaults"]["RibbonColorPickerModel"]
    );
    {
        let model = RibbonColorPickerModel::new(Some("x".into()), Some("X".into()));
        let trace = Rc::new(RefCell::new(Vec::<String>::new()));
        let log = trace.clone();
        model
            .property_changed()
            .add(Rc::new(move |n| log.borrow_mut().push(n.into())));
        let case = source["mutations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["type"] == "RibbonColorPickerModel" && r["property"] == "SelectedColor")
            .unwrap();
        let mut states = vec![];
        for value in case["values"].as_array().unwrap() {
            model.set_selected_color(oc(value));
            states.push(json!({"value":eoc(model.selected_color()),"events":*trace.borrow()}));
            trace.borrow_mut().clear();
        }
        assert_eq!(
            json!(states),
            case["states"],
            "RibbonColorPickerModel.SelectedColor"
        );
    }
    {
        let model = RibbonColorPickerModel::new(Some("x".into()), Some("X".into()));
        let trace = Rc::new(RefCell::new(Vec::<String>::new()));
        let log = trace.clone();
        model
            .property_changed()
            .add(Rc::new(move |n| log.borrow_mut().push(n.into())));
        let case = source["mutations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["type"] == "RibbonColorPickerModel" && r["property"] == "ShowAutomatic")
            .unwrap();
        let mut states = vec![];
        for value in case["values"].as_array().unwrap() {
            model.set_show_automatic(b(value));
            states.push(json!({"value":eb(model.show_automatic()),"events":*trace.borrow()}));
            trace.borrow_mut().clear();
        }
        assert_eq!(
            json!(states),
            case["states"],
            "RibbonColorPickerModel.ShowAutomatic"
        );
    }
    {
        let model = RibbonColorPickerModel::new(Some("x".into()), Some("X".into()));
        let trace = Rc::new(RefCell::new(Vec::<String>::new()));
        let log = trace.clone();
        model
            .property_changed()
            .add(Rc::new(move |n| log.borrow_mut().push(n.into())));
        let case = source["mutations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["type"] == "RibbonColorPickerModel" && r["property"] == "ShowNoColor")
            .unwrap();
        let mut states = vec![];
        for value in case["values"].as_array().unwrap() {
            model.set_show_no_color(b(value));
            states.push(json!({"value":eb(model.show_no_color()),"events":*trace.borrow()}));
            trace.borrow_mut().clear();
        }
        assert_eq!(
            json!(states),
            case["states"],
            "RibbonColorPickerModel.ShowNoColor"
        );
    }
    {
        let model = RibbonColorPickerModel::new(Some("x".into()), Some("X".into()));
        let trace = Rc::new(RefCell::new(Vec::<String>::new()));
        let log = trace.clone();
        model
            .property_changed()
            .add(Rc::new(move |n| log.borrow_mut().push(n.into())));
        let case = source["mutations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["type"] == "RibbonColorPickerModel" && r["property"] == "ShowMoreColors")
            .unwrap();
        let mut states = vec![];
        for value in case["values"].as_array().unwrap() {
            model.set_show_more_colors(b(value));
            states.push(json!({"value":eb(model.show_more_colors()),"events":*trace.borrow()}));
            trace.borrow_mut().clear();
        }
        assert_eq!(
            json!(states),
            case["states"],
            "RibbonColorPickerModel.ShowMoreColors"
        );
    }
    {
        let model = RibbonColorPickerModel::new(Some("x".into()), Some("X".into()));
        let trace = Rc::new(RefCell::new(Vec::<String>::new()));
        let log = trace.clone();
        model
            .property_changed()
            .add(Rc::new(move |n| log.borrow_mut().push(n.into())));
        let case = source["mutations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["type"] == "RibbonColorPickerModel" && r["property"] == "IsSplit")
            .unwrap();
        let mut states = vec![];
        for value in case["values"].as_array().unwrap() {
            model.set_is_split(b(value));
            states.push(json!({"value":eb(model.is_split()),"events":*trace.borrow()}));
            trace.borrow_mut().clear();
        }
        assert_eq!(
            json!(states),
            case["states"],
            "RibbonColorPickerModel.IsSplit"
        );
    }
    {
        let model = RibbonColorPickerModel::new(Some("x".into()), Some("X".into()));
        let trace = Rc::new(RefCell::new(Vec::<String>::new()));
        let log = trace.clone();
        model
            .property_changed()
            .add(Rc::new(move |n| log.borrow_mut().push(n.into())));
        let case = source["mutations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["type"] == "RibbonColorPickerModel" && r["property"] == "AutomaticColor")
            .unwrap();
        let mut states = vec![];
        for value in case["values"].as_array().unwrap() {
            model.set_automatic_color(c(value));
            states.push(json!({"value":ec(model.automatic_color()),"events":*trace.borrow()}));
            trace.borrow_mut().clear();
        }
        assert_eq!(
            json!(states),
            case["states"],
            "RibbonColorPickerModel.AutomaticColor"
        );
    }
}

#[test]
fn contextual_reentry_and_selection_reference_notifications_match_source() {
    let source = corpus();
    let tab = RibbonTabModel::new(Some("tab".into()), None);
    let trace = Rc::new(RefCell::new(Vec::<String>::new()));
    let log = trace.clone();
    let weak = Rc::downgrade(&tab);
    tab.property_changed().add(Rc::new(move |name| {
        let tab = weak.upgrade().unwrap();
        log.borrow_mut().push(format!(
            "{name}:{}",
            if tab.is_contextual() { "True" } else { "False" }
        ));
        if name == "ContextualGroupId" && tab.contextual_group_id().is_some() {
            tab.set_contextual_group_id(None);
        }
    }));
    tab.set_contextual_group_id(Some("ctx".into()));
    assert_eq!(json!(*trace.borrow()), source["reentry"]);
    let gallery = RibbonGalleryModel::new(None, None);
    let log = Rc::new(RefCell::new(Vec::<String>::new()));
    let out = log.clone();
    gallery
        .property_changed()
        .add(Rc::new(move |n| out.borrow_mut().push(n.into())));
    let item: Rc<dyn RibbonGalleryItemModelImpl> =
        RibbonGalleryItemModel::new(Some("item".into()), None);
    gallery.set_selected_item(Some(item.clone()));
    gallery.set_selected_item(Some(item));
    gallery.set_selected_item(Some(RibbonGalleryItemModel::new(Some("item".into()), None)));
    gallery.set_selected_item(None);
    assert_eq!(json!(*log.borrow()), source["gallerySelection"]);
    let backstage = RibbonBackstageModel::new();
    let log = Rc::new(RefCell::new(Vec::<String>::new()));
    let out = log.clone();
    backstage
        .property_changed()
        .add(Rc::new(move |n| out.borrow_mut().push(n.into())));
    let item: Rc<dyn RibbonBackstageItemModelImpl> =
        RibbonBackstageItemModel::new(Some("entry".into()), None);
    backstage.set_selected_item(Some(item.clone()));
    backstage.set_selected_item(Some(item));
    backstage.set_selected_item(Some(RibbonBackstageItemModel::new(
        Some("entry".into()),
        None,
    )));
    backstage.set_selected_item(None);
    assert_eq!(json!(*log.borrow()), source["backstageSelection"]);
}
#[test]
fn structure_collections_keep_sharing_and_core_setters_do_not_invoke_actions() {
    use ferroribbon_core::commands::RibbonRelayCommand;
    use std::cell::Cell;
    let group = RibbonGroupModel::new(None, None);
    let item: Rc<dyn RibbonItemModelImpl> = RibbonButtonModel::new(None, None);
    group.items().add(Some(item.clone()));
    group.slide_out_items().add(Some(item.clone()));
    group.items().add(None);
    assert!(group.items().ptr_eq(&group.items()));
    assert!(Rc::ptr_eq(
        &group.items().get(0).unwrap(),
        &group.slide_out_items().get(0).unwrap()
    ));
    let tab = RibbonTabModel::new(None, None);
    let group_ref: Rc<dyn RibbonGroupModelImpl> = group.clone();
    tab.groups().add(Some(group_ref.clone()));
    assert!(Rc::ptr_eq(&tab.groups().get(0).unwrap(), &group_ref));
    let calls = Rc::new(Cell::new(0));
    let out = calls.clone();
    let command = RibbonRelayCommand::new(move || out.set(out.get() + 1));
    group.set_dialog_launcher_command(Some(command.clone()));
    let gallery = RibbonGalleryModel::new(None, None);
    gallery.set_command(Some(command.clone()));
    gallery.set_preview_command(Some(command.clone()));
    let chosen: Rc<dyn RibbonGalleryItemModelImpl> = RibbonGalleryItemModel::new(None, None);
    gallery.items().add(Some(chosen.clone()));
    gallery.set_selected_item(Some(chosen.clone()));
    assert!(Rc::ptr_eq(&gallery.selected_item().unwrap(), &chosen));
    let picker = RibbonColorPickerModel::new(None, None);
    picker.set_command(Some(command));
    picker.set_selected_color(None);
    assert!(picker.theme_colors().is_empty());
    assert!(picker.standard_colors().is_empty());
    assert!(picker.recent_colors().is_empty());
    picker.recent_colors().add(RibbonColor::BLACK);
    assert_eq!(picker.recent_colors().get(0), RibbonColor::BLACK);
    assert_eq!(calls.get(), 0);
    let tip = RibbonScreenTip::new();
    let trace = Rc::new(RefCell::new(0));
    let out = trace.clone();
    group
        .property_changed()
        .add(Rc::new(move |_| *out.borrow_mut() += 1));
    group.set_dialog_launcher_screen_tip(Some(tip.clone()));
    group.set_dialog_launcher_screen_tip(Some(tip));
    group.set_dialog_launcher_screen_tip(Some(RibbonScreenTip::new()));
    assert_eq!(*trace.borrow(), 2);
}

struct DerivedPicker {
    base: RibbonColorPickerModel,
    trace: RefCell<Vec<String>>,
}
impl INotifyPropertyChanged for DerivedPicker {
    fn property_changed(&self) -> &ferroui_system::events::Event<str> {
        self.base.property_changed()
    }
}
impl ferroribbon_core::ObservableObjectImpl for DerivedPicker {
    fn on_property_changed(&self, n: &str) {
        self.trace.borrow_mut().push(format!(
            "{n}:{}:{}",
            self.size(),
            if self.command().is_none() {
                "null"
            } else {
                "command"
            }
        ));
        self.property_changed().raise(n);
    }
}
impl RibbonNodeModelImpl for DerivedPicker {
    fn node_model(&self) -> &RibbonNodeModel {
        self.base.item_model().node_model()
    }
}
impl RibbonItemModelImpl for DerivedPicker {
    fn item_model(&self) -> &RibbonItemModel {
        self.base.item_model()
    }
}
impl RibbonColorPickerModelImpl for DerivedPicker {
    fn ribbon_color_picker_model(&self) -> &RibbonColorPickerModel {
        &self.base
    }
}
struct DerivedGalleryItem {
    base: RibbonGalleryItemModel,
    trace: RefCell<Vec<String>>,
}
impl INotifyPropertyChanged for DerivedGalleryItem {
    fn property_changed(&self) -> &ferroui_system::events::Event<str> {
        self.base.property_changed()
    }
}
impl ferroribbon_core::ObservableObjectImpl for DerivedGalleryItem {
    fn on_property_changed(&self, n: &str) {
        self.trace.borrow_mut().push(format!(
            "{n}:{}:{}",
            self.category()
                .map_or("null".into(), |v| v.to_utf8().unwrap()),
            if self.value().is_none() {
                "null"
            } else {
                "value"
            }
        ));
        self.property_changed().raise(n);
    }
}
impl RibbonNodeModelImpl for DerivedGalleryItem {
    fn node_model(&self) -> &RibbonNodeModel {
        self.base.node_model()
    }
}
impl RibbonGalleryItemModelImpl for DerivedGalleryItem {
    fn ribbon_gallery_item_model(&self) -> &RibbonGalleryItemModel {
        &self.base
    }
}
struct DerivedContextual {
    base: RibbonContextualGroupModel,
    trace: RefCell<Vec<String>>,
}
impl INotifyPropertyChanged for DerivedContextual {
    fn property_changed(&self) -> &ferroui_system::events::Event<str> {
        self.base.property_changed()
    }
}
impl ferroribbon_core::ObservableObjectImpl for DerivedContextual {
    fn on_property_changed(&self, n: &str) {
        self.trace.borrow_mut().push(format!(
            "{n}:{}:{}",
            if self.is_visible() { "True" } else { "False" },
            self.color().to_hex(true)
        ));
        self.property_changed().raise(n);
    }
}
impl RibbonNodeModelImpl for DerivedContextual {
    fn node_model(&self) -> &RibbonNodeModel {
        self.base.node_model()
    }
}
impl RibbonContextualGroupModelImpl for DerivedContextual {
    fn ribbon_contextual_group_model(&self) -> &RibbonContextualGroupModel {
        &self.base
    }
}
#[test]
fn original_constructor_calls_dispatch_before_later_direct_field_assignments() {
    use ferroribbon_core::commands::RibbonRelayCommand;
    let picker = DerivedPicker {
        base: RibbonColorPickerModel::construct(Some("x".into()), Some("X".into())),
        trace: RefCell::default(),
    };
    RibbonColorPickerModel::initialize(
        &picker,
        Some(RibbonIcon::glyph(Some("a".into()))),
        Some(RibbonRelayCommand::new(|| {})),
    );
    assert_eq!(json!(*picker.trace.borrow()), corpus()["pickerConstructor"]);
    let item = DerivedGalleryItem {
        base: RibbonGalleryItemModel::construct(Some("x".into()), Some("X".into())),
        trace: RefCell::default(),
    };
    RibbonGalleryItemModel::initialize(
        &item,
        Some(RibbonIcon::glyph(Some("a".into()))),
        Some("category".into()),
        Some(RibbonObject::value(RibbonString::from("value"))),
    );
    assert_eq!(json!(*item.trace.borrow()), corpus()["itemConstructor"]);
    assert_eq!(item.category(), Some("category".into()));
    assert_eq!(
        item.value().unwrap().downcast_ref::<RibbonString>(),
        Some(&RibbonString::from("value"))
    );
    let context = DerivedContextual {
        base: RibbonContextualGroupModel::construct(Some("x".into()), Some("X".into())),
        trace: RefCell::default(),
    };
    RibbonContextualGroupModel::initialize(&context);
    assert_eq!(
        json!(*context.trace.borrow()),
        corpus()["contextualConstructor"]
    );
}

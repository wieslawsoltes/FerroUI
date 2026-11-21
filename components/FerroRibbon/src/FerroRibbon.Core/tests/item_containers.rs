use ferroribbon_core::{
    RibbonString,
    commands::{RibbonGestureNumberFormat, RibbonKeyGesture},
    model::*,
};
use ferroui_system::component_model::INotifyPropertyChanged;
use serde_json::{Value, json};
use std::{cell::RefCell, rc::Rc};
fn corpus() -> Value {
    serde_json::from_str(include_str!("../../../tests/fixtures/item-containers.json")).unwrap()
}
fn text(v: &Value) -> RibbonString {
    RibbonString::from_utf16(
        v.as_array()
            .unwrap()
            .iter()
            .map(|n| n.as_u64().unwrap() as u16)
            .collect::<Vec<_>>(),
    )
}
#[test]
fn groups_and_rows_retain_nested_shared_items_nulls_and_duplicates() {
    let child: Rc<dyn RibbonItemModelImpl> = RibbonButtonModel::new(Some("child".into()), None);
    let group = RibbonButtonGroupModel::with_items(
        Some("g".into()),
        [Some(child.clone()), None, Some(child.clone())],
    );
    let row = RibbonRowModel::with_items(
        Some("r".into()),
        [Some(child.clone()), None, Some(child.clone())],
    );
    let bases: [Rc<dyn RibbonButtonGroupModelImpl>; 2] = [group.clone(), row.clone()];
    let actual=bases.iter().map(|g|json!({"type":g.model_type_name(),"size":g.size().raw(),"qat":g.can_add_to_quick_access(),"label":g.label().map(|s|s.as_utf16().to_vec()),"count":g.items().count(),"same":Rc::ptr_eq(&g.items().get(0).unwrap(),&g.items().get(2).unwrap()),"hasNull":g.items().get(1).is_none()})).collect::<Vec<_>>();
    assert_eq!(json!(actual), corpus()["containers"]);
    assert!(group.items().ptr_eq(&group.items()));
    row.items().get(0).unwrap().set_label(Some("shared".into()));
    assert_eq!(group.items().get(0).unwrap().label(), Some("shared".into()));
}
#[test]
fn segment_default_uses_original_id_argument_and_selection_has_no_implicit_pick() {
    let nodes = [
        RibbonSegmentModel::new(None, None),
        RibbonSegmentModel::new(Some("".into()), None),
        RibbonSegmentModel::new(Some("a".into()), None),
    ];
    let actual=nodes.iter().map(|s|json!({"hasId":s.id().is_some(),"value":s.value().map(|v|v.downcast_ref::<RibbonString>().unwrap().as_utf16().to_vec())})).collect::<Vec<_>>();
    assert_eq!(json!(actual), corpus()["segmentDefaults"]);
    let selected = RibbonSegmentedModel::new(Some("s".into()), None);
    let trace = Rc::new(RefCell::new(Vec::<String>::new()));
    let log = trace.clone();
    selected
        .property_changed()
        .add(Rc::new(move |n| log.borrow_mut().push(n.into())));
    selected
        .segments()
        .add(Some(RibbonSegmentModel::new(Some("inside".into()), None)));
    assert_eq!(
        selected.selected_segment().is_some(),
        corpus()["autoSelected"].as_bool().unwrap()
    );
    let external: Rc<dyn RibbonSegmentModelImpl> =
        RibbonSegmentModel::new(Some("outside".into()), None);
    selected.set_selected_segment(Some(external.clone()));
    assert!(Rc::ptr_eq(&selected.selected_segment().unwrap(), &external));
    selected.set_selected_segment(Some(external));
    selected.set_selected_segment(None);
    assert_eq!(json!(*trace.borrow()), corpus()["selectionChanges"]);
    assert_eq!(json!(selected.size().raw()), corpus()["selectedSize"]);
    assert_eq!(
        json!(selected.can_add_to_quick_access()),
        corpus()["selectedQat"]
    );
    let id: RibbonString = "shared".into();
    let segment = RibbonSegmentModel::new(Some(id.clone()), None);
    assert!(
        segment
            .value()
            .unwrap()
            .downcast_ref::<RibbonString>()
            .unwrap()
            .ptr_eq(&id)
    );
}
#[test]
fn grid_size_formats_negative_integers_using_live_culture_and_lossless_utf16() {
    let old = RibbonKeyGesture::set_number_format_provider(None);
    for row in corpus()["grids"].as_array().unwrap() {
        let negative = text(&row["negative"]);
        RibbonKeyGesture::set_number_format_provider(Some(Rc::new(move || {
            RibbonGestureNumberFormat {
                positive_sign: "+".into(),
                negative_sign: negative.clone(),
            }
        })));
        let grid = RibbonGridSize::new(
            row["rows"].as_i64().unwrap() as i32,
            row["columns"].as_i64().unwrap() as i32,
        );
        assert_eq!(grid.to_ribbon_string(), text(&row["text"]));
        assert_eq!(grid, grid);
    }
    RibbonKeyGesture::set_number_format_provider(old);
}

struct DerivedGroup {
    base: RibbonButtonGroupModel,
    trace: RefCell<Vec<String>>,
}
impl INotifyPropertyChanged for DerivedGroup {
    fn property_changed(&self) -> &ferroui_system::events::Event<str> {
        self.base.property_changed()
    }
}
impl ferroribbon_core::ObservableObjectImpl for DerivedGroup {
    fn on_property_changed(&self, name: &str) {
        self.trace.borrow_mut().push(name.into());
        self.property_changed().raise(name);
    }
}
impl RibbonNodeModelImpl for DerivedGroup {
    fn node_model(&self) -> &RibbonNodeModel {
        self.base.item_model().node_model()
    }
}
impl RibbonItemModelImpl for DerivedGroup {
    fn item_model(&self) -> &RibbonItemModel {
        self.base.item_model()
    }
}
impl RibbonButtonGroupModelImpl for DerivedGroup {
    fn button_group_model(&self) -> &RibbonButtonGroupModel {
        &self.base
    }
}
#[test]
fn null_params_array_fails_after_source_constructor_state_changes() {
    let model = DerivedGroup {
        base: RibbonButtonGroupModel::construct(Some("x".into())),
        trace: RefCell::default(),
    };
    let failure = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        RibbonButtonGroupModel::initialize_nullable(&model, None)
    }));
    assert!(failure.is_err());
    assert_eq!(json!(*model.trace.borrow()), corpus()["nullTrace"]);
    assert_eq!(corpus()["nullError"], "NullReferenceException");
    assert!(model.items().is_empty());
    assert!(!model.can_add_to_quick_access());
}

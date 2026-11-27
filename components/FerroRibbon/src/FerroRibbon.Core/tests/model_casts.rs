use ferroribbon_core::{ObservableObjectImpl, model::*};
use ferroui_system::{component_model::INotifyPropertyChanged, events::Event};
use std::{any::Any, cell::RefCell, rc::Rc};
fn corpus() -> serde_json::Value {
    serde_json::from_str(include_str!("../../../tests/fixtures/model-casts.json")).unwrap()
}
fn check<Target: ?Sized + RibbonModelView>(source: &Rc<dyn Any>, expected: &serde_json::Value) {
    let cast = try_cast_model::<Target>(Some(source.clone())).unwrap();
    assert_eq!(cast.is_some(), expected.as_bool().unwrap());
    if let Some(cast) = cast {
        assert!(
            std::ptr::addr_eq(Rc::as_ptr(&cast), Rc::as_ptr(source)),
            "cast changed the object allocation"
        );
    }
}
#[test]
fn original_model_assignability_matrix_retains_identity() {
    let sources: Vec<(&str, Rc<dyn Any>)> = vec![
        (
            "RibbonBackstageItemModel",
            RibbonBackstageItemModel::new(None, None),
        ),
        ("RibbonBackstageModel", RibbonBackstageModel::new()),
        ("RibbonButtonGroupModel", RibbonButtonGroupModel::new(None)),
        ("RibbonButtonModel", RibbonButtonModel::new(None, None)),
        ("RibbonCheckBoxModel", RibbonCheckBoxModel::new(None, None)),
        (
            "RibbonColorPickerModel",
            RibbonColorPickerModel::new(None, None),
        ),
        ("RibbonComboBoxModel", RibbonComboBoxModel::new(None, None)),
        (
            "RibbonContextualGroupModel",
            RibbonContextualGroupModel::new(None, None),
        ),
        (
            "RibbonCustomItemModel",
            RibbonCustomItemModel::new(None, None),
        ),
        (
            "RibbonDropDownButtonModel",
            RibbonDropDownButtonModel::new(None, None),
        ),
        (
            "RibbonGalleryItemModel",
            RibbonGalleryItemModel::new(None, None),
        ),
        ("RibbonGalleryModel", RibbonGalleryModel::new(None, None)),
        (
            "RibbonGridPickerModel",
            RibbonGridPickerModel::new(None, None),
        ),
        ("RibbonGroupModel", RibbonGroupModel::new(None, None)),
        ("RibbonLabelModel", RibbonLabelModel::new(None, None)),
        ("RibbonMenuHeaderModel", RibbonMenuHeaderModel::new(None)),
        ("RibbonMenuItemModel", RibbonMenuItemModel::new(None, None)),
        ("RibbonMenuSeparatorModel", RibbonMenuSeparatorModel::new()),
        ("RibbonModel", RibbonModel::new()),
        ("RibbonRowModel", RibbonRowModel::new(None)),
        ("RibbonSegmentModel", RibbonSegmentModel::new(None, None)),
        (
            "RibbonSegmentedModel",
            RibbonSegmentedModel::new(None, None),
        ),
        ("RibbonSeparatorModel", RibbonSeparatorModel::new()),
        ("RibbonSliderModel", RibbonSliderModel::new(None, None)),
        ("RibbonSpinnerModel", RibbonSpinnerModel::new(None, None)),
        (
            "RibbonSplitButtonModel",
            RibbonSplitButtonModel::new(None, None),
        ),
        ("RibbonTabModel", RibbonTabModel::new(None, None)),
        ("RibbonTextBoxModel", RibbonTextBoxModel::new(None, None)),
        (
            "RibbonToggleButtonModel",
            RibbonToggleButtonModel::new(None, None),
        ),
    ];
    let data = corpus();
    assert_eq!(sources.len(), data["rows"].as_array().unwrap().len());
    for (name, source) in sources {
        let row = data["rows"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["type"] == name)
            .unwrap();
        let expected = &row["casts"];
        // Object erasure and inherited views are ordinary Rust operations.
        assert_eq!(expected["Object"], true);
        let observable: Rc<dyn ObservableObjectImpl> = if let Some(node) =
            try_cast_model::<dyn RibbonNodeModelImpl>(Some(source.clone())).unwrap()
        {
            node
        } else if let Some(root) =
            try_cast_model::<dyn RibbonModelImpl>(Some(source.clone())).unwrap()
        {
            root
        } else {
            try_cast_model::<dyn RibbonBackstageModelImpl>(Some(source.clone()))
                .unwrap()
                .unwrap()
        };
        let notify: Rc<dyn INotifyPropertyChanged> = observable.clone();
        assert_eq!(expected["ObservableObject"], true);
        assert_eq!(expected["INotifyPropertyChanged"], true);
        assert!(std::ptr::addr_eq(
            Rc::as_ptr(&observable),
            Rc::as_ptr(&source)
        ));
        assert!(std::ptr::addr_eq(Rc::as_ptr(&notify), Rc::as_ptr(&source)));
        check::<dyn RibbonNodeModelImpl>(&source, &expected["RibbonNodeModel"]);
        check::<dyn RibbonModelImpl>(&source, &expected["RibbonModel"]);
        check::<dyn RibbonItemModelImpl>(&source, &expected["RibbonItemModel"]);
        check::<dyn RibbonBackstageItemModelImpl>(&source, &expected["RibbonBackstageItemModel"]);
        check::<dyn RibbonBackstageModelImpl>(&source, &expected["RibbonBackstageModel"]);
        check::<dyn RibbonButtonGroupModelImpl>(&source, &expected["RibbonButtonGroupModel"]);
        check::<dyn RibbonButtonModelImpl>(&source, &expected["RibbonButtonModel"]);
        check::<dyn RibbonCheckBoxModelImpl>(&source, &expected["RibbonCheckBoxModel"]);
        check::<dyn RibbonColorPickerModelImpl>(&source, &expected["RibbonColorPickerModel"]);
        check::<dyn RibbonComboBoxModelImpl>(&source, &expected["RibbonComboBoxModel"]);
        check::<dyn RibbonContextualGroupModelImpl>(
            &source,
            &expected["RibbonContextualGroupModel"],
        );
        check::<dyn RibbonCustomItemModelImpl>(&source, &expected["RibbonCustomItemModel"]);
        check::<dyn RibbonDropDownButtonModelImpl>(&source, &expected["RibbonDropDownButtonModel"]);
        check::<dyn RibbonGalleryItemModelImpl>(&source, &expected["RibbonGalleryItemModel"]);
        check::<dyn RibbonGalleryModelImpl>(&source, &expected["RibbonGalleryModel"]);
        check::<dyn RibbonGridPickerModelImpl>(&source, &expected["RibbonGridPickerModel"]);
        check::<dyn RibbonGroupModelImpl>(&source, &expected["RibbonGroupModel"]);
        check::<dyn RibbonLabelModelImpl>(&source, &expected["RibbonLabelModel"]);
        let concrete = source.clone().downcast::<RibbonMenuHeaderModel>().ok();
        assert_eq!(
            concrete.is_some(),
            expected["RibbonMenuHeaderModel"].as_bool().unwrap()
        );
        if let Some(concrete) = concrete {
            assert!(std::ptr::addr_eq(
                Rc::as_ptr(&concrete),
                Rc::as_ptr(&source)
            ));
        }
        check::<dyn RibbonMenuItemModelImpl>(&source, &expected["RibbonMenuItemModel"]);
        let concrete = source.clone().downcast::<RibbonMenuSeparatorModel>().ok();
        assert_eq!(
            concrete.is_some(),
            expected["RibbonMenuSeparatorModel"].as_bool().unwrap()
        );
        if let Some(concrete) = concrete {
            assert!(std::ptr::addr_eq(
                Rc::as_ptr(&concrete),
                Rc::as_ptr(&source)
            ));
        }
        check::<dyn RibbonRowModelImpl>(&source, &expected["RibbonRowModel"]);
        check::<dyn RibbonSegmentModelImpl>(&source, &expected["RibbonSegmentModel"]);
        check::<dyn RibbonSegmentedModelImpl>(&source, &expected["RibbonSegmentedModel"]);
        check::<dyn RibbonSeparatorModelImpl>(&source, &expected["RibbonSeparatorModel"]);
        check::<dyn RibbonSliderModelImpl>(&source, &expected["RibbonSliderModel"]);
        check::<dyn RibbonSpinnerModelImpl>(&source, &expected["RibbonSpinnerModel"]);
        check::<dyn RibbonSplitButtonModelImpl>(&source, &expected["RibbonSplitButtonModel"]);
        check::<dyn RibbonTabModelImpl>(&source, &expected["RibbonTabModel"]);
        check::<dyn RibbonTextBoxModelImpl>(&source, &expected["RibbonTextBoxModel"]);
        check::<dyn RibbonToggleButtonModelImpl>(&source, &expected["RibbonToggleButtonModel"]);
    }
}

struct DerivedToggle {
    base: RibbonToggleButtonModel,
    trace: RefCell<Vec<String>>,
}
impl INotifyPropertyChanged for DerivedToggle {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}
impl ObservableObjectImpl for DerivedToggle {
    fn on_property_changed(&self, name: &str) {
        self.trace.borrow_mut().push(name.into());
        self.property_changed().raise(name);
    }
}
impl RibbonNodeModelImpl for DerivedToggle {
    fn node_model(&self) -> &RibbonNodeModel {
        self.base.button_model().item_model().node_model()
    }
}
impl RibbonItemModelImpl for DerivedToggle {
    fn item_model(&self) -> &RibbonItemModel {
        self.base.button_model().item_model()
    }
}
impl RibbonButtonModelImpl for DerivedToggle {
    fn button_model(&self) -> &RibbonButtonModel {
        self.base.button_model()
    }
}
impl RibbonToggleButtonModelImpl for DerivedToggle {
    fn toggle_button_model(&self) -> &RibbonToggleButtonModel {
        &self.base
    }
}
#[test]
fn declared_downstream_type_keeps_original_allocation_and_virtual_dispatch() {
    register_ribbon_toggle_button_model::<DerivedToggle>().unwrap();
    let concrete = Rc::new(DerivedToggle {
        base: RibbonToggleButtonModel::construct(Some("x".into()), Some("X".into())),
        trace: RefCell::default(),
    });
    let node: Rc<dyn RibbonNodeModelImpl> = concrete.clone();
    let button = try_cast_node::<dyn RibbonButtonModelImpl>(Some(node.clone()))
        .unwrap()
        .unwrap();
    let item = try_cast_node::<dyn RibbonItemModelImpl>(Some(node.clone()))
        .unwrap()
        .unwrap();
    let notify: Rc<dyn INotifyPropertyChanged> = node.clone();
    assert!(std::ptr::addr_eq(
        Rc::as_ptr(&button),
        Rc::as_ptr(&concrete)
    ));
    assert!(std::ptr::addr_eq(Rc::as_ptr(&item), Rc::as_ptr(&concrete)));
    assert!(std::ptr::addr_eq(
        Rc::as_ptr(&notify),
        Rc::as_ptr(&concrete)
    ));
    button.set_label(Some("changed".into()));
    assert_eq!(
        serde_json::json!(*concrete.trace.borrow()),
        corpus()["derived"]["trace"]
    );
    assert_eq!(
        try_cast_node::<dyn RibbonToggleButtonModelImpl>(Some(node.clone()))
            .unwrap()
            .is_some(),
        corpus()["derived"]["toggle"].as_bool().unwrap()
    );
    assert_eq!(
        try_cast_node::<dyn RibbonSplitButtonModelImpl>(Some(node))
            .unwrap()
            .is_some(),
        corpus()["derived"]["split"].as_bool().unwrap()
    );
    let erased: Rc<dyn Any> = item;
    let recovered = erased.downcast::<DerivedToggle>().unwrap();
    assert!(Rc::ptr_eq(&concrete, &recovered));
}
struct Undeclared {
    base: RibbonButtonModel,
}
impl INotifyPropertyChanged for Undeclared {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}
impl ObservableObjectImpl for Undeclared {}
impl RibbonNodeModelImpl for Undeclared {
    fn node_model(&self) -> &RibbonNodeModel {
        self.base.item_model().node_model()
    }
}
impl RibbonItemModelImpl for Undeclared {
    fn item_model(&self) -> &RibbonItemModel {
        self.base.item_model()
    }
}
impl RibbonButtonModelImpl for Undeclared {
    fn button_model(&self) -> &RibbonButtonModel {
        &self.base
    }
}
#[test]
fn null_unknown_and_conflicting_declarations_are_distinguished() {
    assert!(
        try_cast_node::<dyn RibbonItemModelImpl>(None)
            .unwrap()
            .is_none()
    );
    let node: Rc<dyn RibbonNodeModelImpl> = Rc::new(Undeclared {
        base: RibbonButtonModel::construct(None, None),
    });
    assert!(matches!(
        try_cast_node::<dyn RibbonButtonModelImpl>(Some(node)),
        Err(ModelCastError::UnregisteredSource(_))
    ));
    assert!(register_ribbon_button_model::<RibbonToggleButtonModel>().is_err());
    let toggle: Rc<dyn RibbonNodeModelImpl> = RibbonToggleButtonModel::new(None, None);
    assert!(
        try_cast_node::<dyn RibbonToggleButtonModelImpl>(Some(toggle))
            .unwrap()
            .is_some()
    );
}
#[test]
fn registry_does_not_retain_models_and_sibling_types_are_not_assignable() {
    let model = RibbonRowModel::new(None);
    let weak = Rc::downgrade(&model);
    let node: Rc<dyn RibbonNodeModelImpl> = model.clone();
    let group = try_cast_node::<dyn RibbonButtonGroupModelImpl>(Some(node.clone()))
        .unwrap()
        .unwrap();
    assert!(
        try_cast_node::<dyn RibbonSplitButtonModelImpl>(Some(node.clone()))
            .unwrap()
            .is_none()
    );
    drop(node);
    drop(model);
    assert!(weak.upgrade().is_some());
    drop(group);
    assert!(weak.upgrade().is_none());
    let item = RibbonButtonModel::new(None, None);
    let erased: Rc<dyn Any> = item.clone();
    assert!(erased.downcast::<String>().is_err());
    assert_eq!(Rc::strong_count(&item), 1);
}

//! Marker and generated-container metadata use source interfaces without UI projections.
use crate::*;
use ferrodock_model::core::*;
use ferrodock_model_reactiveui::controls::*;
use ferroui_base::{
    data::core::{ValueType, ValueTypes},
    metadata::{MarkupAssembly, MarkupTyped},
    BoxedValue,
};
use std::{cell::RefCell, rc::Rc};

#[test]
fn original_target_markers_keep_core_owner_checked_membership_and_enclosing_model_identity() {
    register_types();
    for metadata in [
        <dyn ILocalTargetMetadata>::MARKUP,
        <dyn IGlobalTargetMetadata>::MARKUP,
        <dyn IDockItemContainerMetadataDescriptor>::MARKUP,
    ] {
        assert_eq!(
            MarkupAssembly::of_module(metadata.module_path)
                .unwrap()
                .name,
            "FerroDock.Model"
        );
    }
    let cases: Vec<(Rc<dyn IDockable>, bool, bool)> = vec![
        (Document::new(), true, false),
        (Tool::new(), true, false),
        (DocumentDock::new(), true, false),
        (ToolDock::new(), true, false),
        (ProportionalDock::new(), false, true),
        (RootDock::new(), false, false),
        (GridDock::new(), false, false),
        (ProportionalDockSplitter::new(), false, false),
    ];
    for (model, local, global) in cases {
        let value: BoxedValue = Rc::new(model);
        for (target, expected) in [
            (ValueType::of::<Rc<dyn ILocalTarget>>(), local),
            (ValueType::of::<Rc<dyn IGlobalTarget>>(), global),
        ] {
            let actual = ValueTypes::try_cast(&value, target);
            assert_eq!(actual.is_some(), expected);
            if let Some(actual) = actual {
                assert!(ValueTypes::reference_equals(&value, &actual));
            }
        }
    }
}

#[derive(Default)]
struct Container {
    theme: RefCell<Option<ferrodock_model::BoxedValue>>,
    selector: RefCell<Option<ferrodock_model::BoxedValue>>,
}
impl IDockItemContainerMetadata for Container {
    fn item_container_theme(&self) -> Option<ferrodock_model::BoxedValue> {
        self.theme.borrow().clone()
    }
    fn set_item_container_theme(&self, value: Option<ferrodock_model::BoxedValue>) {
        *self.theme.borrow_mut() = value;
    }
    fn item_template_selector(&self) -> Option<ferrodock_model::BoxedValue> {
        self.selector.borrow().clone()
    }
    fn set_item_template_selector(&self, value: Option<ferrodock_model::BoxedValue>) {
        *self.selector.borrow_mut() = value;
    }
}
#[derive(PartialEq)]
struct ApplicationValue(u32);
#[test]
fn core_container_metadata_retains_original_arbitrary_objects_and_nulls_through_shared_runtime() {
    register_types();
    let source = Rc::new(Container::default());
    let contract: Rc<dyn IDockItemContainerMetadata> = source.clone();
    let boxed: BoxedValue = Rc::new(contract.clone());
    let metadata = <dyn IDockItemContainerMetadataDescriptor>::MARKUP;
    assert!(ValueTypes::reference_equals(
        &boxed,
        &(Rc::new(contract) as BoxedValue)
    ));
    for name in ["ItemContainerTheme", "ItemTemplateSelector"] {
        let property = metadata.find_property(name).unwrap();
        assert!((property.get.unwrap())(&[Some(boxed.clone())])
            .unwrap()
            .is_none());
        for value in [
            Rc::new("resource key".to_owned()) as BoxedValue,
            Rc::new(ApplicationValue(42)),
        ] {
            (property.set.unwrap())(&[Some(boxed.clone()), Some(value.clone())]).unwrap();
            let actual = (property.get.unwrap())(&[Some(boxed.clone())])
                .unwrap()
                .unwrap();
            assert!(Rc::ptr_eq(&value, &actual));
        }
        (property.set.unwrap())(&[Some(boxed.clone()), None]).unwrap();
        assert!((property.get.unwrap())(&[Some(boxed.clone())])
            .unwrap()
            .is_none());
    }
    assert!(source.item_container_theme().is_none());
    assert!(source.item_template_selector().is_none());
}

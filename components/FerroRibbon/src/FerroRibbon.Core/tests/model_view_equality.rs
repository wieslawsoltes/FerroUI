//! Original CLR virtual Equals through typed model class references.
use ferroribbon_core::{ObservableObjectImpl, model::*};
use ferroui_system::{component_model::INotifyPropertyChanged, events::Event};
use serde_json::json;
use std::rc::Rc;
struct EqualButton {
    base: RibbonButtonModel,
}
impl EqualButton {
    fn new(label: &str) -> Rc<Self> {
        Rc::new(Self {
            base: RibbonButtonModel::construct(Some("id".into()), Some(label.into())),
        })
    }
}
impl INotifyPropertyChanged for EqualButton {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}
impl ObservableObjectImpl for EqualButton {}
impl RibbonNodeModelImpl for EqualButton {
    fn node_model(&self) -> &RibbonNodeModel {
        self.base.node_model()
    }
    fn equals_node(&self, other: &dyn RibbonNodeModelImpl) -> bool {
        self.label() == other.label()
    }
}
impl RibbonItemModelImpl for EqualButton {
    fn item_model(&self) -> &RibbonItemModel {
        self.base.item_model()
    }
}
impl RibbonButtonModelImpl for EqualButton {
    fn button_model(&self) -> &RibbonButtonModel {
        &self.base
    }
}
#[test]
fn source_virtual_equality_dispatches_through_specific_and_base_model_views() {
    let expected: serde_json::Value =
        serde_json::from_str(include_str!("../../../tests/fixtures/model-metadata.json")).unwrap();
    let first: Rc<dyn RibbonButtonModelImpl> = EqualButton::new("same");
    let same: Rc<dyn RibbonButtonModelImpl> = EqualButton::new("same");
    let other: Rc<dyn RibbonButtonModelImpl> = EqualButton::new("other");
    let ordinary: Rc<dyn RibbonButtonModelImpl> =
        RibbonButtonModel::new(Some("x".into()), Some("same".into()));
    let first_node: Rc<dyn RibbonNodeModelImpl> = first.clone();
    let same_node: Rc<dyn RibbonNodeModelImpl> = same.clone();
    let actual = json!({"same":first.as_ref()==same.as_ref(),"different":first.as_ref()==other.as_ref(),"sameThroughNode":first_node==same_node,"ordinary":first.as_ref()==ordinary.as_ref(),"reverse":ordinary.as_ref()==first.as_ref(),"erased":first.equals_node(same.as_ref()),"erasedOther":first.equals_node(other.as_ref()),"erasedOrdinary":first.equals_node(ordinary.as_ref())});
    assert_eq!(actual, expected["corpus"]["equality"]);
}

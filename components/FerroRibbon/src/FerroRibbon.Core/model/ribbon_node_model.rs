use super::{RibbonIcon, RibbonObject, RibbonScreenTip};
use crate::{ObservableObject, ObservableObjectImpl, RibbonMergeAction, RibbonString};
use ferroui_system::{
    collections::object_model::ObservableCollection, component_model::INotifyPropertyChanged,
    events::Event,
};
use std::{any::Any, cell::RefCell, rc::Rc};

/// Storage of the abstract source base. Setters dispatch through RibbonNodeModelImpl
/// on the owning concrete model, including when called through a base trait object.
pub struct RibbonNodeModel {
    base: ObservableObject,
    id: RefCell<Option<RibbonString>>,
    label: RefCell<Option<RibbonString>>,
    icon: RefCell<Option<Rc<RibbonIcon>>>,
    large_icon: RefCell<Option<Rc<RibbonIcon>>>,
    is_visible: RefCell<bool>,
    is_enabled: RefCell<bool>,
    key_tip: RefCell<Option<RibbonString>>,
    screen_tip: RefCell<Option<Rc<RibbonScreenTip>>>,
    description: RefCell<Option<RibbonString>>,
    order: RefCell<i32>,
    tag: RefCell<Option<RibbonObject>>,
    automation_id: RefCell<Option<RibbonString>>,
    merge_action: RefCell<RibbonMergeAction>,
    keywords: ObservableCollection<Option<RibbonString>>,
}
macro_rules! property {
    ($get:ident,$set:ident,$ty:ty,$name:literal) => {
        fn $get(&self) -> $ty {
            self.node_model().$get.borrow().to_owned()
        }
        fn $set(&self, value: $ty) {
            ObservableObject::set_property(self, &self.node_model().$get, value, $name);
        }
    };
}
pub trait RibbonNodeModelImpl: ObservableObjectImpl + Any {
    fn node_model(&self) -> &RibbonNodeModel;
    fn model_type_name(&self) -> &'static str {
        std::any::type_name::<Self>().rsplit("::").next().unwrap()
    }
    fn to_ribbon_string(&self) -> RibbonString {
        let mut out = self.model_type_name().encode_utf16().collect::<Vec<_>>();
        out.push(40);
        if let Some(id) = self.id() {
            out.extend_from_slice(id.as_utf16());
        }
        out.extend([44, 32]);
        if let Some(label) = self.label() {
            out.extend_from_slice(label.as_utf16());
        }
        out.push(41);
        RibbonString::from_utf16(out)
    }
    /// Default object identity hash for query membership. Downstream Equals overrides
    /// can pair it with a matching hash, as with CLR GetHashCode. Numeric CLR object
    /// identity hash values are not reproduced by this native allocation hash.
    fn node_hash_code(&self) -> i32 {
        self as *const Self as *const () as usize as i32
    }
    fn equals_node(&self, other: &dyn RibbonNodeModelImpl) -> bool {
        std::ptr::addr_eq(
            self as *const Self as *const (),
            other as *const dyn RibbonNodeModelImpl as *const (),
        )
    }
}
pub trait RibbonNodeModelExt: RibbonNodeModelImpl {
    property!(id, set_id, Option<RibbonString>, "Id");
    property!(label, set_label, Option<RibbonString>, "Label");
    property!(icon, set_icon, Option<Rc<RibbonIcon>>, "Icon");
    property!(
        large_icon,
        set_large_icon,
        Option<Rc<RibbonIcon>>,
        "LargeIcon"
    );
    property!(is_visible, set_is_visible, bool, "IsVisible");
    property!(is_enabled, set_is_enabled, bool, "IsEnabled");
    property!(key_tip, set_key_tip, Option<RibbonString>, "KeyTip");
    property!(
        screen_tip,
        set_screen_tip,
        Option<Rc<RibbonScreenTip>>,
        "ScreenTip"
    );
    property!(
        description,
        set_description,
        Option<RibbonString>,
        "Description"
    );
    property!(order, set_order, i32, "Order");
    property!(tag, set_tag, Option<RibbonObject>, "Tag");
    property!(
        automation_id,
        set_automation_id,
        Option<RibbonString>,
        "AutomationId"
    );
    property!(
        merge_action,
        set_merge_action,
        RibbonMergeAction,
        "MergeAction"
    );
    fn keywords(&self) -> ObservableCollection<Option<RibbonString>> {
        self.node_model().keywords.clone()
    }
}
impl<T: RibbonNodeModelImpl + ?Sized> RibbonNodeModelExt for T {}
impl PartialEq for dyn RibbonNodeModelImpl {
    fn eq(&self, other: &Self) -> bool {
        self.equals_node(other)
    }
}
impl RibbonNodeModel {
    pub fn construct(id: Option<RibbonString>, label: Option<RibbonString>) -> Self {
        Self {
            base: ObservableObject::construct(),
            id: RefCell::new(Some(
                id.unwrap_or_else(|| uuid::Uuid::new_v4().simple().to_string().into()),
            )),
            label: RefCell::new(label),
            icon: RefCell::default(),
            large_icon: RefCell::default(),
            is_visible: RefCell::new(true),
            is_enabled: RefCell::new(true),
            key_tip: RefCell::default(),
            screen_tip: RefCell::default(),
            description: RefCell::default(),
            order: RefCell::new(0),
            tag: RefCell::default(),
            automation_id: RefCell::default(),
            merge_action: RefCell::default(),
            keywords: ObservableCollection::new_managed(),
        }
    }
}
impl INotifyPropertyChanged for RibbonNodeModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}
pub fn downcast_node<T: RibbonNodeModelImpl>(
    node: Rc<dyn RibbonNodeModelImpl>,
) -> Result<Rc<T>, Rc<dyn RibbonNodeModelImpl>> {
    let value: Rc<dyn Any> = node.clone();
    value.downcast::<T>().map_err(|_| node)
}

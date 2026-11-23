use super::{RibbonIcon, RibbonNodeModel, RibbonNodeModelExt, RibbonNodeModelImpl, RibbonObject};
use crate::{ObservableObject, ObservableObjectImpl, RibbonString, commands::ICommand};
use crate::collections::ObservableCollection;
use ferroui_system::{
    component_model::INotifyPropertyChanged,
    events::Event,
};
use std::{cell::RefCell, rc::Rc};
pub struct RibbonMenuItemModel {
    base: RibbonNodeModel,
    command_id: RefCell<Option<RibbonString>>,
    command: RefCell<Option<Rc<dyn ICommand>>>,
    command_parameter: RefCell<Option<RibbonObject>>,
    is_checkable: RefCell<bool>,
    is_checked: RefCell<bool>,
    group_name: RefCell<Option<RibbonString>>,
    shortcut: RefCell<Option<RibbonString>>,
    items: ObservableCollection<Option<Rc<dyn RibbonNodeModelImpl>>>,
}
macro_rules! property {
    ($get:ident,$set:ident,$ty:ty,$name:literal) => {
        fn $get(&self) -> $ty {
            self.menu_item_model().$get.borrow().to_owned()
        }
        fn $set(&self, value: $ty) {
            ObservableObject::set_property(self, &self.menu_item_model().$get, value, $name);
        }
    };
}
pub trait RibbonMenuItemModelImpl: RibbonNodeModelImpl {
    fn menu_item_model(&self) -> &RibbonMenuItemModel;
}
pub trait RibbonMenuItemModelExt: RibbonMenuItemModelImpl {
    property!(
        command_id,
        set_command_id,
        Option<RibbonString>,
        "CommandId"
    );
    property!(
        command_parameter,
        set_command_parameter,
        Option<RibbonObject>,
        "CommandParameter"
    );
    property!(is_checkable, set_is_checkable, bool, "IsCheckable");
    property!(is_checked, set_is_checked, bool, "IsChecked");
    property!(
        group_name,
        set_group_name,
        Option<RibbonString>,
        "GroupName"
    );
    property!(shortcut, set_shortcut, Option<RibbonString>, "Shortcut");
    fn command(&self) -> Option<Rc<dyn ICommand>> {
        self.menu_item_model().command.borrow().clone()
    }
    fn set_command(&self, value: Option<Rc<dyn ICommand>>) {
        let equal = match (&*self.menu_item_model().command.borrow(), &value) {
            (None, None) => true,
            (Some(a), Some(b)) => Rc::ptr_eq(a, b),
            _ => false,
        };
        if !equal {
            let old = self.menu_item_model().command.replace(value);
            drop(old);
            self.on_property_changed("Command");
        }
    }
    fn items(&self) -> ObservableCollection<Option<Rc<dyn RibbonNodeModelImpl>>> {
        self.menu_item_model().items.clone()
    }
}
impl<T: RibbonMenuItemModelImpl + ?Sized> RibbonMenuItemModelExt for T {}
impl RibbonMenuItemModel {
    pub fn construct(id: Option<RibbonString>, label: Option<RibbonString>) -> Self {
        Self {
            base: RibbonNodeModel::construct(id, label),
            command_id: RefCell::default(),
            command: RefCell::default(),
            command_parameter: RefCell::default(),
            is_checkable: RefCell::new(false),
            is_checked: RefCell::new(false),
            group_name: RefCell::default(),
            shortcut: RefCell::default(),
            items: ObservableCollection::new_managed(),
        }
    }
    pub fn initialize<O: RibbonMenuItemModelImpl + ?Sized>(
        owner: &O,
        icon: Option<Rc<RibbonIcon>>,
        command: Option<Rc<dyn ICommand>>,
    ) {
        owner.set_icon(icon);
        owner.menu_item_model().command.replace(command);
    }
    pub fn new(id: Option<RibbonString>, label: Option<RibbonString>) -> Rc<Self> {
        Self::with_options(id, label, None, None)
    }
    pub fn with_options(
        id: Option<RibbonString>,
        label: Option<RibbonString>,
        icon: Option<Rc<RibbonIcon>>,
        command: Option<Rc<dyn ICommand>>,
    ) -> Rc<Self> {
        let model = Rc::new(Self::construct(id, label));
        Self::initialize(model.as_ref(), icon, command);
        model
    }
    pub fn node_model(&self) -> &RibbonNodeModel {
        &self.base
    }
}
impl INotifyPropertyChanged for RibbonMenuItemModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}
impl ObservableObjectImpl for RibbonMenuItemModel {}
impl RibbonNodeModelImpl for RibbonMenuItemModel {
    fn node_model(&self) -> &RibbonNodeModel {
        &self.base
    }
}
impl RibbonMenuItemModelImpl for RibbonMenuItemModel {
    fn menu_item_model(&self) -> &RibbonMenuItemModel {
        self
    }
}
impl PartialEq for RibbonMenuItemModel {
    fn eq(&self, other: &Self) -> bool {
        self.equals_node(other)
    }
}

pub struct RibbonMenuSeparatorModel {
    base: RibbonNodeModel,
}
impl RibbonMenuSeparatorModel {
    pub fn new() -> Rc<Self> {
        Rc::new(Self {
            base: RibbonNodeModel::construct(None, None),
        })
    }
}
impl INotifyPropertyChanged for RibbonMenuSeparatorModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}
impl ObservableObjectImpl for RibbonMenuSeparatorModel {}
impl RibbonNodeModelImpl for RibbonMenuSeparatorModel {
    fn node_model(&self) -> &RibbonNodeModel {
        &self.base
    }
}
impl PartialEq for RibbonMenuSeparatorModel {
    fn eq(&self, other: &Self) -> bool {
        self.equals_node(other)
    }
}

pub struct RibbonMenuHeaderModel {
    base: RibbonNodeModel,
}
impl RibbonMenuHeaderModel {
    pub fn new(label: Option<RibbonString>) -> Rc<Self> {
        Rc::new(Self {
            base: RibbonNodeModel::construct(None, label),
        })
    }
}
impl INotifyPropertyChanged for RibbonMenuHeaderModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}
impl ObservableObjectImpl for RibbonMenuHeaderModel {}
impl RibbonNodeModelImpl for RibbonMenuHeaderModel {
    fn node_model(&self) -> &RibbonNodeModel {
        &self.base
    }
}
impl PartialEq for RibbonMenuHeaderModel {
    fn eq(&self, other: &Self) -> bool {
        self.equals_node(other)
    }
}

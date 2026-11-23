use super::{RibbonIcon, RibbonNodeModel, RibbonNodeModelExt, RibbonNodeModelImpl, RibbonObject};
use crate::{
    ObservableObject, ObservableObjectImpl, RibbonItemSize, RibbonSimplifiedVisibility,
    RibbonString, commands::ICommand, layout::RibbonSizeDefinition,
};
use ferroui_system::{component_model::INotifyPropertyChanged, events::Event};
use std::{cell::RefCell, rc::Rc};
pub struct RibbonItemModel {
    base: RibbonNodeModel,
    size: RefCell<RibbonItemSize>,
    size_definition: RefCell<Option<RibbonSizeDefinition>>,
    simplified_visibility: RefCell<RibbonSimplifiedVisibility>,
    show_label_in_simplified: RefCell<Option<bool>>,
    command_id: RefCell<Option<RibbonString>>,
    command: RefCell<Option<Rc<dyn ICommand>>>,
    command_parameter: RefCell<Option<RibbonObject>>,
    shortcut: RefCell<Option<RibbonString>>,
    can_add_to_quick_access: RefCell<bool>,
    show_label: RefCell<bool>,
}
macro_rules! property {
    ($get:ident,$set:ident,$ty:ty,$name:literal) => {
        fn $get(&self) -> $ty {
            self.item_model().$get.borrow().to_owned()
        }
        fn $set(&self, value: $ty) {
            ObservableObject::set_property(self, &self.item_model().$get, value, $name);
        }
    };
}
pub trait RibbonItemModelImpl: RibbonNodeModelImpl {
    fn item_model(&self) -> &RibbonItemModel;
}
pub trait RibbonItemModelExt: RibbonItemModelImpl {
    fn size(&self) -> RibbonItemSize {
        *self.item_model().size.borrow()
    }
    fn set_size(&self, value: RibbonItemSize) {
        if ObservableObject::set_property(self, &self.item_model().size, value, "Size")
            && self.size_definition().is_none()
        {
            self.on_property_changed("EffectiveSizeDefinition");
        }
    }
    fn size_definition(&self) -> Option<RibbonSizeDefinition> {
        *self.item_model().size_definition.borrow()
    }
    fn set_size_definition(&self, value: Option<RibbonSizeDefinition>) {
        if ObservableObject::set_property(
            self,
            &self.item_model().size_definition,
            value,
            "SizeDefinition",
        ) {
            self.on_property_changed("EffectiveSizeDefinition");
        }
    }
    fn effective_size_definition(&self) -> RibbonSizeDefinition {
        self.size_definition()
            .unwrap_or_else(|| RibbonSizeDefinition::for_preferred_size(self.size()))
    }
    property!(
        simplified_visibility,
        set_simplified_visibility,
        RibbonSimplifiedVisibility,
        "SimplifiedVisibility"
    );
    property!(
        show_label_in_simplified,
        set_show_label_in_simplified,
        Option<bool>,
        "ShowLabelInSimplified"
    );
    property!(show_label, set_show_label, bool, "ShowLabel");
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
    property!(shortcut, set_shortcut, Option<RibbonString>, "Shortcut");
    property!(
        can_add_to_quick_access,
        set_can_add_to_quick_access,
        bool,
        "CanAddToQuickAccess"
    );
    fn command(&self) -> Option<Rc<dyn ICommand>> {
        self.item_model().command.borrow().clone()
    }
    fn set_command(&self, value: Option<Rc<dyn ICommand>>) {
        let equal = match (&*self.item_model().command.borrow(), &value) {
            (None, None) => true,
            (Some(a), Some(b)) => Rc::ptr_eq(a, b),
            _ => false,
        };
        if !equal {
            let old = self.item_model().command.replace(value);
            drop(old);
            self.on_property_changed("Command");
        }
    }
}
impl<T: RibbonItemModelImpl + ?Sized> RibbonItemModelExt for T {}
impl PartialEq for dyn RibbonItemModelImpl {
    fn eq(&self, other: &Self) -> bool {
        self.equals_node(other)
    }
}
impl RibbonItemModel {
    pub fn construct(id: Option<RibbonString>, label: Option<RibbonString>) -> Self {
        Self {
            base: RibbonNodeModel::construct(id, label),
            size: RefCell::new(RibbonItemSize::Medium),
            size_definition: RefCell::default(),
            simplified_visibility: RefCell::default(),
            show_label_in_simplified: RefCell::default(),
            command_id: RefCell::default(),
            command: RefCell::default(),
            command_parameter: RefCell::default(),
            shortcut: RefCell::default(),
            can_add_to_quick_access: RefCell::new(true),
            show_label: RefCell::new(true),
        }
    }
    pub fn node_model(&self) -> &RibbonNodeModel {
        &self.base
    }
}
impl INotifyPropertyChanged for RibbonItemModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}

pub struct RibbonButtonModel {
    base: RibbonItemModel,
}
pub trait RibbonButtonModelImpl: RibbonItemModelImpl {
    fn button_model(&self) -> &RibbonButtonModel;
}
impl RibbonButtonModel {
    pub fn construct(id: Option<RibbonString>, label: Option<RibbonString>) -> Self {
        Self {
            base: RibbonItemModel::construct(id, label),
        }
    }
    /// Run the original constructor body on the complete owner, preserving virtual notifications.
    pub fn initialize<O: RibbonButtonModelImpl + ?Sized>(
        owner: &O,
        icon: Option<Rc<RibbonIcon>>,
        command: Option<Rc<dyn ICommand>>,
        size: RibbonItemSize,
    ) {
        owner.set_icon(icon);
        owner.set_command(command);
        owner.set_size(size);
    }
    pub fn new(id: Option<RibbonString>, label: Option<RibbonString>) -> Rc<Self> {
        Self::with_options(id, label, None, None, RibbonItemSize::Medium)
    }
    pub fn with_options(
        id: Option<RibbonString>,
        label: Option<RibbonString>,
        icon: Option<Rc<RibbonIcon>>,
        command: Option<Rc<dyn ICommand>>,
        size: RibbonItemSize,
    ) -> Rc<Self> {
        let model = Rc::new(Self::construct(id, label));
        Self::initialize(model.as_ref(), icon, command, size);
        model
    }
    pub fn item_model(&self) -> &RibbonItemModel {
        &self.base
    }
}
impl INotifyPropertyChanged for RibbonButtonModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}
impl ObservableObjectImpl for RibbonButtonModel {}
impl RibbonNodeModelImpl for RibbonButtonModel {
    fn node_model(&self) -> &RibbonNodeModel {
        self.base.node_model()
    }
}
impl RibbonItemModelImpl for RibbonButtonModel {
    fn item_model(&self) -> &RibbonItemModel {
        &self.base
    }
}
impl RibbonButtonModelImpl for RibbonButtonModel {
    fn button_model(&self) -> &RibbonButtonModel {
        self
    }
}
impl PartialEq for RibbonButtonModel {
    fn eq(&self, other: &Self) -> bool {
        self.equals_node(other)
    }
}

pub struct RibbonToggleButtonModel {
    base: RibbonButtonModel,
    is_checked: RefCell<bool>,
    group_name: RefCell<Option<RibbonString>>,
}
pub trait RibbonToggleButtonModelImpl: RibbonButtonModelImpl {
    fn toggle_button_model(&self) -> &RibbonToggleButtonModel;
}
pub trait RibbonToggleButtonModelExt: RibbonToggleButtonModelImpl {
    fn is_checked(&self) -> bool {
        *self.toggle_button_model().is_checked.borrow()
    }
    fn set_is_checked(&self, value: bool) {
        ObservableObject::set_property(
            self,
            &self.toggle_button_model().is_checked,
            value,
            "IsChecked",
        );
    }
    fn group_name(&self) -> Option<RibbonString> {
        self.toggle_button_model().group_name.borrow().clone()
    }
    fn set_group_name(&self, value: Option<RibbonString>) {
        ObservableObject::set_property(
            self,
            &self.toggle_button_model().group_name,
            value,
            "GroupName",
        );
    }
}
impl<T: RibbonToggleButtonModelImpl + ?Sized> RibbonToggleButtonModelExt for T {}
impl RibbonToggleButtonModel {
    pub fn construct(id: Option<RibbonString>, label: Option<RibbonString>) -> Self {
        Self {
            base: RibbonButtonModel::construct(id, label),
            is_checked: RefCell::new(false),
            group_name: RefCell::default(),
        }
    }
    pub fn new(id: Option<RibbonString>, label: Option<RibbonString>) -> Rc<Self> {
        Self::with_options(id, label, None, None, RibbonItemSize::Small)
    }
    pub fn with_options(
        id: Option<RibbonString>,
        label: Option<RibbonString>,
        icon: Option<Rc<RibbonIcon>>,
        command: Option<Rc<dyn ICommand>>,
        size: RibbonItemSize,
    ) -> Rc<Self> {
        let model = Rc::new(Self::construct(id, label));
        RibbonButtonModel::initialize(model.as_ref(), icon, command, size);
        model
    }
    pub fn button_model(&self) -> &RibbonButtonModel {
        &self.base
    }
}
impl INotifyPropertyChanged for RibbonToggleButtonModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}
impl ObservableObjectImpl for RibbonToggleButtonModel {}
impl RibbonNodeModelImpl for RibbonToggleButtonModel {
    fn node_model(&self) -> &RibbonNodeModel {
        self.base.item_model().node_model()
    }
}
impl RibbonItemModelImpl for RibbonToggleButtonModel {
    fn item_model(&self) -> &RibbonItemModel {
        self.base.item_model()
    }
}
impl RibbonButtonModelImpl for RibbonToggleButtonModel {
    fn button_model(&self) -> &RibbonButtonModel {
        &self.base
    }
}
impl RibbonToggleButtonModelImpl for RibbonToggleButtonModel {
    fn toggle_button_model(&self) -> &RibbonToggleButtonModel {
        self
    }
}
impl PartialEq for RibbonToggleButtonModel {
    fn eq(&self, other: &Self) -> bool {
        self.equals_node(other)
    }
}

pub struct RibbonDropDownButtonModel {
    base: RibbonItemModel,
    menu_items: crate::collections::ObservableCollection<
        Option<Rc<dyn RibbonNodeModelImpl>>,
    >,
    drop_down_content: RefCell<Option<RibbonObject>>,
}
pub trait RibbonDropDownButtonModelImpl: RibbonItemModelImpl {
    fn drop_down_button_model(&self) -> &RibbonDropDownButtonModel;
}
pub trait RibbonDropDownButtonModelExt: RibbonDropDownButtonModelImpl {
    fn menu_items(
        &self,
    ) -> crate::collections::ObservableCollection<
        Option<Rc<dyn RibbonNodeModelImpl>>,
    > {
        self.drop_down_button_model().menu_items.clone()
    }
    fn drop_down_content(&self) -> Option<RibbonObject> {
        self.drop_down_button_model()
            .drop_down_content
            .borrow()
            .clone()
    }
    fn set_drop_down_content(&self, value: Option<RibbonObject>) {
        ObservableObject::set_property(
            self,
            &self.drop_down_button_model().drop_down_content,
            value,
            "DropDownContent",
        );
    }
}
impl<T: RibbonDropDownButtonModelImpl + ?Sized> RibbonDropDownButtonModelExt for T {}
impl RibbonDropDownButtonModel {
    pub fn construct(id: Option<RibbonString>, label: Option<RibbonString>) -> Self {
        Self {
            base: RibbonItemModel::construct(id, label),
            menu_items: Default::default(),
            drop_down_content: RefCell::default(),
        }
    }
    pub fn initialize<O: RibbonDropDownButtonModelImpl + ?Sized>(
        owner: &O,
        icon: Option<Rc<RibbonIcon>>,
        size: RibbonItemSize,
    ) {
        owner.set_icon(icon);
        owner.set_size(size);
    }
    pub fn new(id: Option<RibbonString>, label: Option<RibbonString>) -> Rc<Self> {
        Self::with_options(id, label, None, RibbonItemSize::Medium)
    }
    pub fn with_options(
        id: Option<RibbonString>,
        label: Option<RibbonString>,
        icon: Option<Rc<RibbonIcon>>,
        size: RibbonItemSize,
    ) -> Rc<Self> {
        let value = Rc::new(Self::construct(id, label));
        Self::initialize(value.as_ref(), icon, size);
        value
    }
    pub fn item_model(&self) -> &RibbonItemModel {
        &self.base
    }
}
impl INotifyPropertyChanged for RibbonDropDownButtonModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}
impl ObservableObjectImpl for RibbonDropDownButtonModel {}
impl RibbonNodeModelImpl for RibbonDropDownButtonModel {
    fn node_model(&self) -> &RibbonNodeModel {
        self.base.node_model()
    }
}
impl RibbonItemModelImpl for RibbonDropDownButtonModel {
    fn item_model(&self) -> &RibbonItemModel {
        &self.base
    }
}
impl RibbonDropDownButtonModelImpl for RibbonDropDownButtonModel {
    fn drop_down_button_model(&self) -> &RibbonDropDownButtonModel {
        self
    }
}
impl PartialEq for RibbonDropDownButtonModel {
    fn eq(&self, other: &Self) -> bool {
        self.equals_node(other)
    }
}

pub struct RibbonSplitButtonModel {
    base: RibbonDropDownButtonModel,
    is_checkable: RefCell<bool>,
    is_checked: RefCell<bool>,
    follow_last_choice: RefCell<bool>,
}
pub trait RibbonSplitButtonModelImpl: RibbonDropDownButtonModelImpl {
    fn split_button_model(&self) -> &RibbonSplitButtonModel;
}
pub trait RibbonSplitButtonModelExt: RibbonSplitButtonModelImpl {
    fn is_checkable(&self) -> bool {
        *self.split_button_model().is_checkable.borrow()
    }
    fn set_is_checkable(&self, value: bool) {
        ObservableObject::set_property(
            self,
            &self.split_button_model().is_checkable,
            value,
            "IsCheckable",
        );
    }
    fn is_checked(&self) -> bool {
        *self.split_button_model().is_checked.borrow()
    }
    fn set_is_checked(&self, value: bool) {
        ObservableObject::set_property(
            self,
            &self.split_button_model().is_checked,
            value,
            "IsChecked",
        );
    }
    fn follow_last_choice(&self) -> bool {
        *self.split_button_model().follow_last_choice.borrow()
    }
    fn set_follow_last_choice(&self, value: bool) {
        ObservableObject::set_property(
            self,
            &self.split_button_model().follow_last_choice,
            value,
            "FollowLastChoice",
        );
    }
}
impl<T: RibbonSplitButtonModelImpl + ?Sized> RibbonSplitButtonModelExt for T {}
impl RibbonSplitButtonModel {
    pub fn construct(id: Option<RibbonString>, label: Option<RibbonString>) -> Self {
        Self {
            base: RibbonDropDownButtonModel::construct(id, label),
            is_checkable: RefCell::new(false),
            is_checked: RefCell::new(false),
            follow_last_choice: RefCell::new(false),
        }
    }
    pub fn initialize<O: RibbonSplitButtonModelImpl + ?Sized>(
        owner: &O,
        icon: Option<Rc<RibbonIcon>>,
        command: Option<Rc<dyn ICommand>>,
        size: RibbonItemSize,
    ) {
        RibbonDropDownButtonModel::initialize(owner, icon, size);
        owner.set_command(command);
    }
    pub fn new(id: Option<RibbonString>, label: Option<RibbonString>) -> Rc<Self> {
        Self::with_options(id, label, None, None, RibbonItemSize::Large)
    }
    pub fn with_options(
        id: Option<RibbonString>,
        label: Option<RibbonString>,
        icon: Option<Rc<RibbonIcon>>,
        command: Option<Rc<dyn ICommand>>,
        size: RibbonItemSize,
    ) -> Rc<Self> {
        let value = Rc::new(Self::construct(id, label));
        Self::initialize(value.as_ref(), icon, command, size);
        value
    }
    pub fn drop_down_button_model(&self) -> &RibbonDropDownButtonModel {
        &self.base
    }
}
impl INotifyPropertyChanged for RibbonSplitButtonModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}
impl ObservableObjectImpl for RibbonSplitButtonModel {}
impl RibbonNodeModelImpl for RibbonSplitButtonModel {
    fn node_model(&self) -> &RibbonNodeModel {
        self.base.item_model().node_model()
    }
}
impl RibbonItemModelImpl for RibbonSplitButtonModel {
    fn item_model(&self) -> &RibbonItemModel {
        self.base.item_model()
    }
}
impl RibbonDropDownButtonModelImpl for RibbonSplitButtonModel {
    fn drop_down_button_model(&self) -> &RibbonDropDownButtonModel {
        &self.base
    }
}
impl RibbonSplitButtonModelImpl for RibbonSplitButtonModel {
    fn split_button_model(&self) -> &RibbonSplitButtonModel {
        self
    }
}
impl PartialEq for RibbonSplitButtonModel {
    fn eq(&self, other: &Self) -> bool {
        self.equals_node(other)
    }
}

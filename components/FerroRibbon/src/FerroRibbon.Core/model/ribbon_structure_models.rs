// Port of the same-named pinned C# file
use super::*;
use crate::{
    ObservableObject, ObservableObjectImpl, RibbonBackstagePlacement, RibbonContextualActivation,
    RibbonGroupItemsLayout, RibbonSimplifiedVisibility, RibbonString, commands::ICommand,
    theming::RibbonColor,
};
use ferroui_system::{
    collections::object_model::ObservableCollection, component_model::INotifyPropertyChanged,
    events::Event,
};
use std::{cell::RefCell, rc::Rc};
pub struct RibbonGroupModel {
    base: RibbonNodeModel,
    items_layout: RefCell<RibbonGroupItemsLayout>,
    row_count: RefCell<i32>,
    dialog_launcher_command_id: RefCell<Option<RibbonString>>,
    dialog_launcher_screen_tip: RefCell<Option<Rc<RibbonScreenTip>>>,
    reduction_order: RefCell<i32>,
    can_collapse: RefCell<bool>,
    simplified_visibility: RefCell<RibbonSimplifiedVisibility>,
    items: ObservableCollection<Option<Rc<dyn RibbonItemModelImpl>>>,
    slide_out_items: ObservableCollection<Option<Rc<dyn RibbonItemModelImpl>>>,
    dialog_launcher_command: RefCell<Option<Rc<dyn ICommand>>>,
}
pub trait RibbonGroupModelImpl: RibbonNodeModelImpl {
    fn ribbon_group_model(&self) -> &RibbonGroupModel;
}
pub trait RibbonGroupModelExt: RibbonGroupModelImpl {
    fn items_layout(&self) -> RibbonGroupItemsLayout {
        self.ribbon_group_model().items_layout.borrow().to_owned()
    }
    fn set_items_layout(&self, value: RibbonGroupItemsLayout) {
        ObservableObject::set_property(
            self,
            &self.ribbon_group_model().items_layout,
            value,
            "ItemsLayout",
        );
    }
    fn row_count(&self) -> i32 {
        self.ribbon_group_model().row_count.borrow().to_owned()
    }
    fn set_row_count(&self, value: i32) {
        ObservableObject::set_property(
            self,
            &self.ribbon_group_model().row_count,
            value.clamp(1, 3),
            "RowCount",
        );
    }
    fn dialog_launcher_command_id(&self) -> Option<RibbonString> {
        self.ribbon_group_model()
            .dialog_launcher_command_id
            .borrow()
            .to_owned()
    }
    fn set_dialog_launcher_command_id(&self, value: Option<RibbonString>) {
        ObservableObject::set_property(
            self,
            &self.ribbon_group_model().dialog_launcher_command_id,
            value,
            "DialogLauncherCommandId",
        );
    }
    fn dialog_launcher_screen_tip(&self) -> Option<Rc<RibbonScreenTip>> {
        self.ribbon_group_model()
            .dialog_launcher_screen_tip
            .borrow()
            .to_owned()
    }
    fn set_dialog_launcher_screen_tip(&self, value: Option<Rc<RibbonScreenTip>>) {
        ObservableObject::set_property(
            self,
            &self.ribbon_group_model().dialog_launcher_screen_tip,
            value,
            "DialogLauncherScreenTip",
        );
    }
    fn reduction_order(&self) -> i32 {
        self.ribbon_group_model()
            .reduction_order
            .borrow()
            .to_owned()
    }
    fn set_reduction_order(&self, value: i32) {
        ObservableObject::set_property(
            self,
            &self.ribbon_group_model().reduction_order,
            value,
            "ReductionOrder",
        );
    }
    fn can_collapse(&self) -> bool {
        self.ribbon_group_model().can_collapse.borrow().to_owned()
    }
    fn set_can_collapse(&self, value: bool) {
        ObservableObject::set_property(
            self,
            &self.ribbon_group_model().can_collapse,
            value,
            "CanCollapse",
        );
    }
    fn simplified_visibility(&self) -> RibbonSimplifiedVisibility {
        self.ribbon_group_model()
            .simplified_visibility
            .borrow()
            .to_owned()
    }
    fn set_simplified_visibility(&self, value: RibbonSimplifiedVisibility) {
        ObservableObject::set_property(
            self,
            &self.ribbon_group_model().simplified_visibility,
            value,
            "SimplifiedVisibility",
        );
    }
    fn items(&self) -> ObservableCollection<Option<Rc<dyn RibbonItemModelImpl>>> {
        self.ribbon_group_model().items.clone()
    }
    fn slide_out_items(&self) -> ObservableCollection<Option<Rc<dyn RibbonItemModelImpl>>> {
        self.ribbon_group_model().slide_out_items.clone()
    }
    fn dialog_launcher_command(&self) -> Option<Rc<dyn ICommand>> {
        self.ribbon_group_model()
            .dialog_launcher_command
            .borrow()
            .clone()
    }
    fn set_dialog_launcher_command(&self, value: Option<Rc<dyn ICommand>>) {
        let equal = match (
            &*self.ribbon_group_model().dialog_launcher_command.borrow(),
            &value,
        ) {
            (None, None) => true,
            (Some(a), Some(b)) => Rc::ptr_eq(a, b),
            _ => false,
        };
        if !equal {
            let old = self
                .ribbon_group_model()
                .dialog_launcher_command
                .replace(value);
            drop(old);
            self.on_property_changed("DialogLauncherCommand");
        }
    }
}
impl<T: RibbonGroupModelImpl + ?Sized> RibbonGroupModelExt for T {}
impl PartialEq for dyn RibbonGroupModelImpl {
    fn eq(&self, other: &Self) -> bool {
        self.equals_node(other)
    }
}
impl RibbonGroupModel {
    pub fn construct(id: Option<RibbonString>, label: Option<RibbonString>) -> Self {
        Self {
            base: RibbonNodeModel::construct(id, label),
            items_layout: RefCell::new(Default::default()),
            row_count: RefCell::new(3),
            dialog_launcher_command_id: RefCell::new(None),
            dialog_launcher_screen_tip: RefCell::new(None),
            reduction_order: RefCell::new(0),
            can_collapse: RefCell::new(true),
            simplified_visibility: RefCell::new(Default::default()),
            items: ObservableCollection::new_managed(),
            slide_out_items: ObservableCollection::new_managed(),
            dialog_launcher_command: RefCell::default(),
        }
    }
    pub fn initialize<O: RibbonGroupModelImpl + ?Sized>(owner: &O, icon: Option<Rc<RibbonIcon>>) {
        owner.set_icon(icon);
    }
    pub fn new(id: Option<RibbonString>, label: Option<RibbonString>) -> Rc<Self> {
        Self::with_options(id, label, None)
    }
    pub fn with_options(
        id: Option<RibbonString>,
        label: Option<RibbonString>,
        icon: Option<Rc<RibbonIcon>>,
    ) -> Rc<Self> {
        let model = Rc::new(Self::construct(id, label));
        Self::initialize(model.as_ref(), icon);
        model
    }
    pub fn node_model(&self) -> &RibbonNodeModel {
        &self.base
    }
}
impl INotifyPropertyChanged for RibbonGroupModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}
impl ObservableObjectImpl for RibbonGroupModel {}
impl RibbonNodeModelImpl for RibbonGroupModel {
    fn node_model(&self) -> &RibbonNodeModel {
        &self.base
    }
}
impl RibbonGroupModelImpl for RibbonGroupModel {
    fn ribbon_group_model(&self) -> &RibbonGroupModel {
        self
    }
}
impl PartialEq for RibbonGroupModel {
    fn eq(&self, other: &Self) -> bool {
        self.equals_node(other)
    }
}
pub struct RibbonTabModel {
    base: RibbonNodeModel,
    contextual_group_id: RefCell<Option<RibbonString>>,
    groups: ObservableCollection<Option<Rc<dyn RibbonGroupModelImpl>>>,
}
pub trait RibbonTabModelImpl: RibbonNodeModelImpl {
    fn ribbon_tab_model(&self) -> &RibbonTabModel;
}
pub trait RibbonTabModelExt: RibbonTabModelImpl {
    fn contextual_group_id(&self) -> Option<RibbonString> {
        self.ribbon_tab_model()
            .contextual_group_id
            .borrow()
            .to_owned()
    }
    fn set_contextual_group_id(&self, value: Option<RibbonString>) {
        if ObservableObject::set_property(
            self,
            &self.ribbon_tab_model().contextual_group_id,
            value,
            "ContextualGroupId",
        ) {
            self.on_property_changed("IsContextual");
        }
    }
    fn is_contextual(&self) -> bool {
        self.contextual_group_id().is_some()
    }
    fn groups(&self) -> ObservableCollection<Option<Rc<dyn RibbonGroupModelImpl>>> {
        self.ribbon_tab_model().groups.clone()
    }
}
impl<T: RibbonTabModelImpl + ?Sized> RibbonTabModelExt for T {}
impl PartialEq for dyn RibbonTabModelImpl {
    fn eq(&self, other: &Self) -> bool {
        self.equals_node(other)
    }
}
impl RibbonTabModel {
    pub fn construct(id: Option<RibbonString>, label: Option<RibbonString>) -> Self {
        Self {
            base: RibbonNodeModel::construct(id, label),
            contextual_group_id: RefCell::new(None),
            groups: ObservableCollection::new_managed(),
        }
    }
    pub fn new(id: Option<RibbonString>, label: Option<RibbonString>) -> Rc<Self> {
        Rc::new(Self::construct(id, label))
    }
    pub fn node_model(&self) -> &RibbonNodeModel {
        &self.base
    }
}
impl INotifyPropertyChanged for RibbonTabModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}
impl ObservableObjectImpl for RibbonTabModel {}
impl RibbonNodeModelImpl for RibbonTabModel {
    fn node_model(&self) -> &RibbonNodeModel {
        &self.base
    }
}
impl RibbonTabModelImpl for RibbonTabModel {
    fn ribbon_tab_model(&self) -> &RibbonTabModel {
        self
    }
}
impl PartialEq for RibbonTabModel {
    fn eq(&self, other: &Self) -> bool {
        self.equals_node(other)
    }
}
pub struct RibbonContextualGroupModel {
    base: RibbonNodeModel,
    color: RefCell<RibbonColor>,
    activation: RefCell<RibbonContextualActivation>,
}
pub trait RibbonContextualGroupModelImpl: RibbonNodeModelImpl {
    fn ribbon_contextual_group_model(&self) -> &RibbonContextualGroupModel;
}
pub trait RibbonContextualGroupModelExt: RibbonContextualGroupModelImpl {
    fn color(&self) -> RibbonColor {
        self.ribbon_contextual_group_model()
            .color
            .borrow()
            .to_owned()
    }
    fn set_color(&self, value: RibbonColor) {
        ObservableObject::set_property(
            self,
            &self.ribbon_contextual_group_model().color,
            value,
            "Color",
        );
    }
    fn activation(&self) -> RibbonContextualActivation {
        self.ribbon_contextual_group_model()
            .activation
            .borrow()
            .to_owned()
    }
    fn set_activation(&self, value: RibbonContextualActivation) {
        ObservableObject::set_property(
            self,
            &self.ribbon_contextual_group_model().activation,
            value,
            "Activation",
        );
    }
}
impl<T: RibbonContextualGroupModelImpl + ?Sized> RibbonContextualGroupModelExt for T {}
impl PartialEq for dyn RibbonContextualGroupModelImpl {
    fn eq(&self, other: &Self) -> bool {
        self.equals_node(other)
    }
}
impl RibbonContextualGroupModel {
    pub fn construct(id: Option<RibbonString>, label: Option<RibbonString>) -> Self {
        Self {
            base: RibbonNodeModel::construct(id, label),
            color: RefCell::new(RibbonColor::new(255, 15, 108, 189)),
            activation: RefCell::new(Default::default()),
        }
    }
    pub fn initialize<O: RibbonContextualGroupModelImpl + ?Sized>(owner: &O) {
        owner.set_is_visible(false);
    }
    pub fn new(id: Option<RibbonString>, label: Option<RibbonString>) -> Rc<Self> {
        let model = Rc::new(Self::construct(id, label));
        Self::initialize(model.as_ref());
        model
    }
    pub fn node_model(&self) -> &RibbonNodeModel {
        &self.base
    }
}
impl INotifyPropertyChanged for RibbonContextualGroupModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}
impl ObservableObjectImpl for RibbonContextualGroupModel {}
impl RibbonNodeModelImpl for RibbonContextualGroupModel {
    fn node_model(&self) -> &RibbonNodeModel {
        &self.base
    }
}
impl RibbonContextualGroupModelImpl for RibbonContextualGroupModel {
    fn ribbon_contextual_group_model(&self) -> &RibbonContextualGroupModel {
        self
    }
}
impl PartialEq for RibbonContextualGroupModel {
    fn eq(&self, other: &Self) -> bool {
        self.equals_node(other)
    }
}
pub struct RibbonBackstageItemModel {
    base: RibbonNodeModel,
    content: RefCell<Option<RibbonObject>>,
    command_id: RefCell<Option<RibbonString>>,
    command_parameter: RefCell<Option<RibbonObject>>,
    placement: RefCell<RibbonBackstagePlacement>,
    closes_backstage: RefCell<bool>,
    has_separator_before: RefCell<bool>,
    command: RefCell<Option<Rc<dyn ICommand>>>,
}
pub trait RibbonBackstageItemModelImpl: RibbonNodeModelImpl {
    fn ribbon_backstage_item_model(&self) -> &RibbonBackstageItemModel;
}
pub trait RibbonBackstageItemModelExt: RibbonBackstageItemModelImpl {
    fn content(&self) -> Option<RibbonObject> {
        self.ribbon_backstage_item_model()
            .content
            .borrow()
            .to_owned()
    }
    fn set_content(&self, value: Option<RibbonObject>) {
        ObservableObject::set_property(
            self,
            &self.ribbon_backstage_item_model().content,
            value,
            "Content",
        );
    }
    fn command_id(&self) -> Option<RibbonString> {
        self.ribbon_backstage_item_model()
            .command_id
            .borrow()
            .to_owned()
    }
    fn set_command_id(&self, value: Option<RibbonString>) {
        ObservableObject::set_property(
            self,
            &self.ribbon_backstage_item_model().command_id,
            value,
            "CommandId",
        );
    }
    fn command_parameter(&self) -> Option<RibbonObject> {
        self.ribbon_backstage_item_model()
            .command_parameter
            .borrow()
            .to_owned()
    }
    fn set_command_parameter(&self, value: Option<RibbonObject>) {
        ObservableObject::set_property(
            self,
            &self.ribbon_backstage_item_model().command_parameter,
            value,
            "CommandParameter",
        );
    }
    fn placement(&self) -> RibbonBackstagePlacement {
        self.ribbon_backstage_item_model()
            .placement
            .borrow()
            .to_owned()
    }
    fn set_placement(&self, value: RibbonBackstagePlacement) {
        ObservableObject::set_property(
            self,
            &self.ribbon_backstage_item_model().placement,
            value,
            "Placement",
        );
    }
    fn closes_backstage(&self) -> bool {
        self.ribbon_backstage_item_model()
            .closes_backstage
            .borrow()
            .to_owned()
    }
    fn set_closes_backstage(&self, value: bool) {
        ObservableObject::set_property(
            self,
            &self.ribbon_backstage_item_model().closes_backstage,
            value,
            "ClosesBackstage",
        );
    }
    fn has_separator_before(&self) -> bool {
        self.ribbon_backstage_item_model()
            .has_separator_before
            .borrow()
            .to_owned()
    }
    fn set_has_separator_before(&self, value: bool) {
        ObservableObject::set_property(
            self,
            &self.ribbon_backstage_item_model().has_separator_before,
            value,
            "HasSeparatorBefore",
        );
    }
    fn command(&self) -> Option<Rc<dyn ICommand>> {
        self.ribbon_backstage_item_model().command.borrow().clone()
    }
    fn set_command(&self, value: Option<Rc<dyn ICommand>>) {
        let equal = match (
            &*self.ribbon_backstage_item_model().command.borrow(),
            &value,
        ) {
            (None, None) => true,
            (Some(a), Some(b)) => Rc::ptr_eq(a, b),
            _ => false,
        };
        if !equal {
            let old = self.ribbon_backstage_item_model().command.replace(value);
            drop(old);
            self.on_property_changed("Command");
        }
    }
}
impl<T: RibbonBackstageItemModelImpl + ?Sized> RibbonBackstageItemModelExt for T {}
impl PartialEq for dyn RibbonBackstageItemModelImpl {
    fn eq(&self, other: &Self) -> bool {
        self.equals_node(other)
    }
}
impl RibbonBackstageItemModel {
    pub fn construct(id: Option<RibbonString>, label: Option<RibbonString>) -> Self {
        Self {
            base: RibbonNodeModel::construct(id, label),
            content: RefCell::new(None),
            command_id: RefCell::new(None),
            command_parameter: RefCell::new(None),
            placement: RefCell::new(Default::default()),
            closes_backstage: RefCell::new(true),
            has_separator_before: RefCell::new(false),
            command: RefCell::default(),
        }
    }
    pub fn initialize<O: RibbonBackstageItemModelImpl + ?Sized>(
        owner: &O,
        icon: Option<Rc<RibbonIcon>>,
        content: Option<RibbonObject>,
    ) {
        owner.set_icon(icon);
        owner.ribbon_backstage_item_model().content.replace(content);
    }
    pub fn new(id: Option<RibbonString>, label: Option<RibbonString>) -> Rc<Self> {
        Self::with_options(id, label, None, None)
    }
    pub fn with_options(
        id: Option<RibbonString>,
        label: Option<RibbonString>,
        icon: Option<Rc<RibbonIcon>>,
        content: Option<RibbonObject>,
    ) -> Rc<Self> {
        let model = Rc::new(Self::construct(id, label));
        Self::initialize(model.as_ref(), icon, content);
        model
    }
    pub fn node_model(&self) -> &RibbonNodeModel {
        &self.base
    }
}
impl INotifyPropertyChanged for RibbonBackstageItemModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}
impl ObservableObjectImpl for RibbonBackstageItemModel {}
impl RibbonNodeModelImpl for RibbonBackstageItemModel {
    fn node_model(&self) -> &RibbonNodeModel {
        &self.base
    }
}
impl RibbonBackstageItemModelImpl for RibbonBackstageItemModel {
    fn ribbon_backstage_item_model(&self) -> &RibbonBackstageItemModel {
        self
    }
}
impl PartialEq for RibbonBackstageItemModel {
    fn eq(&self, other: &Self) -> bool {
        self.equals_node(other)
    }
}
pub struct RibbonBackstageModel {
    base: ObservableObject,
    selected_item: RefCell<Option<Rc<dyn RibbonBackstageItemModelImpl>>>,
    is_open: RefCell<bool>,
    title: RefCell<Option<RibbonString>>,
    items: ObservableCollection<Option<Rc<dyn RibbonBackstageItemModelImpl>>>,
}
pub trait RibbonBackstageModelImpl: ObservableObjectImpl + std::any::Any {
    fn ribbon_backstage_model(&self) -> &RibbonBackstageModel;
    fn equals_backstage(&self, other: &dyn RibbonBackstageModelImpl) -> bool {
        std::ptr::addr_eq(
            self as *const Self as *const (),
            other as *const dyn RibbonBackstageModelImpl as *const (),
        )
    }
}
pub trait RibbonBackstageModelExt: RibbonBackstageModelImpl {
    fn selected_item(&self) -> Option<Rc<dyn RibbonBackstageItemModelImpl>> {
        self.ribbon_backstage_model()
            .selected_item
            .borrow()
            .to_owned()
    }
    fn set_selected_item(&self, value: Option<Rc<dyn RibbonBackstageItemModelImpl>>) {
        ObservableObject::set_property(
            self,
            &self.ribbon_backstage_model().selected_item,
            value,
            "SelectedItem",
        );
    }
    fn is_open(&self) -> bool {
        self.ribbon_backstage_model().is_open.borrow().to_owned()
    }
    fn set_is_open(&self, value: bool) {
        ObservableObject::set_property(
            self,
            &self.ribbon_backstage_model().is_open,
            value,
            "IsOpen",
        );
    }
    fn title(&self) -> Option<RibbonString> {
        self.ribbon_backstage_model().title.borrow().to_owned()
    }
    fn set_title(&self, value: Option<RibbonString>) {
        ObservableObject::set_property(self, &self.ribbon_backstage_model().title, value, "Title");
    }
    fn items(&self) -> ObservableCollection<Option<Rc<dyn RibbonBackstageItemModelImpl>>> {
        self.ribbon_backstage_model().items.clone()
    }
}
impl<T: RibbonBackstageModelImpl + ?Sized> RibbonBackstageModelExt for T {}
impl PartialEq for dyn RibbonBackstageModelImpl {
    fn eq(&self, other: &Self) -> bool {
        self.equals_backstage(other)
    }
}

impl RibbonBackstageModel {
    pub fn construct() -> Self {
        Self {
            base: ObservableObject::construct(),
            selected_item: RefCell::new(None),
            is_open: RefCell::new(false),
            title: RefCell::new(None),
            items: ObservableCollection::new_managed(),
        }
    }
    pub fn new() -> Rc<Self> {
        Rc::new(Self::construct())
    }
}
impl INotifyPropertyChanged for RibbonBackstageModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}
impl ObservableObjectImpl for RibbonBackstageModel {}
impl RibbonBackstageModelImpl for RibbonBackstageModel {
    fn ribbon_backstage_model(&self) -> &RibbonBackstageModel {
        self
    }
}
impl PartialEq for RibbonBackstageModel {
    fn eq(&self, other: &Self) -> bool {
        self.equals_backstage(other)
    }
}

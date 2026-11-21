//! Application-defined editable item for verifying public interface dispatch.
//! This is not a replacement for any missing library RibbonButton/control.
use ferroribbon::controls::{
    IRibbonCheckable, IRibbonCommandSource, IRibbonItem, IRibbonItemEditable, RibbonItemHelper,
    RibbonItemLayout, RibbonKeyTipResult, RibbonOverflowMenuItems, RibbonSimplifiedLabel,
};
use ferroribbon_core::{
    RibbonItemSize, RibbonSimplifiedVisibility, RibbonString, layout::RibbonSizeDefinition,
    model::RibbonItemsSource,
};
use ferroui_base::{
    BoxedValue, FerroObject, FerroObjectImpl, FerroProperty, Ref, StyledElementImpl,
    StyledProperty, VisualImpl, ferro_class, ferro_class_info, ferro_impl_classes,
    ferro_properties, input::InputElementImpl, instantiate, interactivity::InteractiveImpl,
    layout::LayoutableImpl,
};
use ferroui_controls::{Control, ControlImpl};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
#[repr(C)]
pub struct ContractItem {
    base: Control,
    pub invocations: Cell<usize>,
    pub layouts: RefCell<Vec<RibbonItemLayout>>,
    pub malformed: Cell<bool>,
    pub enumerations: Rc<Cell<usize>>,
    pub command_reads: Cell<usize>,
}
ferro_class!(ContractItem:Control);
ferro_impl_classes!(ContractItem:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl);
ferro_class_info!(ContractItem {new:ContractItem::new,markup:{namespace:"FerroRibbon.Consumer"},interfaces:[
    Rc<dyn IRibbonItem> => |v|Rc::new(ItemView(v)) as Rc<dyn IRibbonItem>,
    Rc<dyn IRibbonCommandSource> => |v|Rc::new(ItemView(v)) as Rc<dyn IRibbonCommandSource>,
    Rc<dyn IRibbonCheckable> => |v|Rc::new(ItemView(v)) as Rc<dyn IRibbonCheckable>,
    Rc<dyn IRibbonItemEditable> => |v|Rc::new(ItemView(v)) as Rc<dyn IRibbonItemEditable>
]});
ferro_properties! {impl ContractItem {
pub fn id_property()->StyledProperty<Option<String>>{FerroProperty::register::<Self,_>("Id",None)}
pub fn label_property()->StyledProperty<Option<String>>{FerroProperty::register::<Self,_>("Label",None)}
pub fn icon_property()->StyledProperty<Option<BoxedValue>>{FerroProperty::register::<Self,_>("Icon",None)}
pub fn large_icon_property()->StyledProperty<Option<BoxedValue>>{FerroProperty::register::<Self,_>("LargeIcon",None)}
pub fn key_tip_property()->StyledProperty<Option<String>>{FerroProperty::register::<Self,_>("KeyTip",None)}
pub fn command_id_property()->StyledProperty<Option<String>>{FerroProperty::register::<Self,_>("CommandId",None)}
pub fn command_property()->StyledProperty<Option<Rc<dyn ferroui_base::input::ICommand>>>{FerroProperty::register::<Self,_>("Command",None)}
pub fn command_parameter_property()->StyledProperty<Option<BoxedValue>>{FerroProperty::register::<Self,_>("CommandParameter",None)}
pub fn is_checked_property()->StyledProperty<Option<bool>>{FerroProperty::register::<Self,_>("IsChecked",None)}
pub fn shortcut_property()->StyledProperty<Option<String>>{FerroProperty::register::<Self,_>("Shortcut",None)}
pub fn screen_tip_property()->StyledProperty<Option<BoxedValue>>{FerroProperty::register::<Self,_>("ScreenTip",None)}
pub fn size_property()->StyledProperty<RibbonItemSize>{FerroProperty::register::<Self,_>("Size",RibbonItemSize::Medium)}
pub fn size_definition_property()->StyledProperty<Option<String>>{FerroProperty::register::<Self,_>("SizeDefinition",None)}
pub fn simplified_visibility_property()->StyledProperty<RibbonSimplifiedVisibility>{FerroProperty::register::<Self,_>("SimplifiedVisibility",RibbonSimplifiedVisibility::Auto)}
pub fn simplified_label_property()->StyledProperty<RibbonSimplifiedLabel>{FerroProperty::register::<Self,_>("SimplifiedLabel",RibbonSimplifiedLabel::Auto)}
pub fn show_label_property()->StyledProperty<bool>{FerroProperty::register::<Self,_>("ShowLabel",true)}
pub fn can_add_to_quick_access_property()->StyledProperty<bool>{FerroProperty::register::<Self,_>("CanAddToQuickAccess",true)}
}}
impl ContractItem {
    pub fn command(&self) -> Option<Rc<dyn ferroui_base::input::ICommand>> {
        self.command_reads.set(self.command_reads.get() + 1);
        self.get_value(Self::command_property())
    }
    pub fn set_command(&self, value: Option<Rc<dyn ferroui_base::input::ICommand>>) {
        self.set_value(Self::command_property(), value);
    }
    pub fn command_parameter(&self) -> Option<BoxedValue> {
        self.get_value(Self::command_parameter_property())
    }
    pub fn set_command_parameter(&self, value: Option<BoxedValue>) {
        self.set_value(Self::command_parameter_property(), value);
    }
    pub fn is_checked(&self) -> Option<bool> {
        self.get_value(Self::is_checked_property())
    }
    pub fn set_is_checked(&self, value: Option<bool>) {
        self.set_value(Self::is_checked_property(), value);
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self {
            base: Control::construct(),
            invocations: Cell::new(0),
            layouts: RefCell::new(Vec::new()),
            malformed: Cell::new(false),
            enumerations: Rc::new(Cell::new(0)),
            command_reads: Cell::new(0),
        })
    }
    pub fn id(&self) -> Option<String> {
        self.get_value(Self::id_property())
    }
    pub fn set_id(&self, v: Option<String>) {
        self.set_value(Self::id_property(), v)
    }
    pub fn label(&self) -> Option<String> {
        self.get_value(Self::label_property())
    }
    pub fn set_label(&self, v: Option<String>) {
        self.set_value(Self::label_property(), v)
    }
    pub fn icon(&self) -> Option<BoxedValue> {
        self.get_value(Self::icon_property())
    }
    pub fn set_icon(&self, v: Option<BoxedValue>) {
        self.set_value(Self::icon_property(), v)
    }
    pub fn large_icon(&self) -> Option<BoxedValue> {
        self.get_value(Self::large_icon_property())
    }
    pub fn set_large_icon(&self, v: Option<BoxedValue>) {
        self.set_value(Self::large_icon_property(), v)
    }
    pub fn key_tip(&self) -> Option<String> {
        self.get_value(Self::key_tip_property())
    }
    pub fn set_key_tip(&self, v: Option<String>) {
        self.set_value(Self::key_tip_property(), v)
    }
    pub fn command_id(&self) -> Option<String> {
        self.get_value(Self::command_id_property())
    }
    pub fn set_command_id(&self, v: Option<String>) {
        self.set_value(Self::command_id_property(), v)
    }
    pub fn shortcut(&self) -> Option<String> {
        self.get_value(Self::shortcut_property())
    }
    pub fn set_shortcut(&self, v: Option<String>) {
        self.set_value(Self::shortcut_property(), v)
    }
    pub fn screen_tip(&self) -> Option<BoxedValue> {
        self.get_value(Self::screen_tip_property())
    }
    pub fn set_screen_tip(&self, v: Option<BoxedValue>) {
        self.set_value(Self::screen_tip_property(), v)
    }
    pub fn size(&self) -> RibbonItemSize {
        self.get_value(Self::size_property())
    }
    pub fn set_size(&self, v: RibbonItemSize) {
        self.set_value(Self::size_property(), v)
    }
    pub fn size_definition(&self) -> Option<String> {
        self.get_value(Self::size_definition_property())
    }
    pub fn set_size_definition(&self, v: Option<String>) {
        self.set_value(Self::size_definition_property(), v)
    }
    pub fn simplified_visibility(&self) -> RibbonSimplifiedVisibility {
        self.get_value(Self::simplified_visibility_property())
    }
    pub fn set_simplified_visibility(&self, v: RibbonSimplifiedVisibility) {
        self.set_value(Self::simplified_visibility_property(), v)
    }
    pub fn simplified_label(&self) -> RibbonSimplifiedLabel {
        self.get_value(Self::simplified_label_property())
    }
    pub fn set_simplified_label(&self, v: RibbonSimplifiedLabel) {
        self.set_value(Self::simplified_label_property(), v)
    }
    pub fn show_label(&self) -> bool {
        self.get_value(Self::show_label_property())
    }
    pub fn set_show_label(&self, v: bool) {
        self.set_value(Self::show_label_property(), v)
    }
    pub fn can_add_to_quick_access(&self) -> bool {
        self.get_value(Self::can_add_to_quick_access_property())
    }
    pub fn set_can_add_to_quick_access(&self, v: bool) {
        self.set_value(Self::can_add_to_quick_access_property(), v)
    }
}
struct ItemView(Ref<ContractItem>);
impl IRibbonCommandSource for ItemView {
    fn as_object(&self) -> Ref<FerroObject> {
        self.0.clone().upcast()
    }
    fn command(&self) -> Option<Rc<dyn ferroui_base::input::ICommand>> {
        self.0.command()
    }
    fn set_command(&self, value: Option<Rc<dyn ferroui_base::input::ICommand>>) {
        self.0.set_command(value);
    }
    fn command_parameter(&self) -> Option<BoxedValue> {
        self.0.command_parameter()
    }
    fn set_command_parameter(&self, value: Option<BoxedValue>) {
        self.0.set_command_parameter(value);
    }
}
impl IRibbonCheckable for ItemView {
    fn as_object(&self) -> Ref<FerroObject> {
        self.0.clone().upcast()
    }
    fn is_checked(&self) -> Option<bool> {
        self.0.is_checked()
    }
    fn set_is_checked(&self, value: Option<bool>) {
        self.0.set_is_checked(value);
    }
}
impl IRibbonItem for ItemView {
    fn as_object(&self) -> Ref<FerroObject> {
        self.0.clone().upcast()
    }
    fn id(&self) -> Option<RibbonString> {
        RibbonItemHelper::get_id(
            &self.0,
            self.0.id().map(Into::into),
            self.0.command_id().map(Into::into),
        )
    }
    fn label(&self) -> Option<RibbonString> {
        if self.0.malformed.get() {
            Some(RibbonString::from_utf16(vec![0xd800]))
        } else {
            self.0.label().map(Into::into)
        }
    }
    fn icon(&self) -> Option<BoxedValue> {
        self.0.icon()
    }
    fn key_tip(&self) -> Option<RibbonString> {
        self.0.key_tip().map(Into::into)
    }
    fn command_id(&self) -> Option<RibbonString> {
        self.0.command_id().map(Into::into)
    }
    fn size(&self) -> RibbonItemSize {
        self.0.size()
    }
    fn simplified_visibility(&self) -> RibbonSimplifiedVisibility {
        self.0.simplified_visibility()
    }
    fn can_add_to_quick_access(&self) -> bool {
        self.0.can_add_to_quick_access()
    }
    fn effective_size_definition(&self) -> RibbonSizeDefinition {
        RibbonSizeDefinition::try_parse(self.0.size_definition().as_deref())
            .unwrap_or_else(|| RibbonSizeDefinition::for_preferred_size(self.0.size()))
    }
    fn current_size(&self) -> RibbonItemSize {
        self.0
            .layouts
            .borrow()
            .last()
            .map_or(self.0.size(), |v| v.size)
    }
    fn show_label_in_simplified_mode(&self) -> bool {
        match self.0.simplified_label() {
            RibbonSimplifiedLabel::Show => true,
            RibbonSimplifiedLabel::Hide => false,
            _ => self.0.size() == RibbonItemSize::Large,
        }
    }
    fn apply_layout(&self, layout: RibbonItemLayout) {
        self.0.layouts.borrow_mut().push(layout)
    }
    fn create_linked_copy(&self) -> Option<Ref<Control>> {
        let copy = ContractItem::new();
        RibbonItemHelper::set_source_item(&copy, &self.0);
        RibbonItemHelper::link(&self.0, &copy, ContractItem::label_property(), true);
        Some(copy.upcast())
    }
    fn create_overflow_menu_items(&self) -> RibbonOverflowMenuItems {
        let item = self.0.clone();
        let count = self.0.enumerations.clone();
        RibbonItemsSource::new(move || {
            count.set(count.get() + 1);
            Box::new(
                vec![
                    Some(item.clone().upcast()),
                    None,
                    Some(item.clone().upcast()),
                ]
                .into_iter(),
            )
        })
    }
    fn on_key_tip(&self) -> Rc<RibbonKeyTipResult> {
        RibbonKeyTipResult::close()
    }
    fn invoke(&self) -> bool {
        if !self.0.is_enabled() {
            return false;
        }
        self.0.invocations.set(self.0.invocations.get() + 1);
        true
    }
}
impl IRibbonItemEditable for ItemView {
    fn id(&self) -> Option<RibbonString> {
        self.0.id().map(Into::into)
    }
    fn label(&self) -> Option<RibbonString> {
        self.0.label().map(Into::into)
    }
    fn icon(&self) -> Option<BoxedValue> {
        self.0.icon()
    }
    fn size(&self) -> RibbonItemSize {
        self.0.size()
    }
    fn key_tip(&self) -> Option<RibbonString> {
        self.0.key_tip().map(Into::into)
    }
    fn command_id(&self) -> Option<RibbonString> {
        self.0.command_id().map(Into::into)
    }
    fn simplified_visibility(&self) -> RibbonSimplifiedVisibility {
        self.0.simplified_visibility()
    }
    fn can_add_to_quick_access(&self) -> bool {
        self.0.can_add_to_quick_access()
    }
    fn set_id(&self, v: Option<RibbonString>) {
        self.0.set_id(v.map(|v| {
            v.to_utf8()
                .expect("consumer text setter only accepts valid native text")
        }))
    }
    fn set_label(&self, v: Option<RibbonString>) {
        self.0.set_label(v.map(|v| {
            v.to_utf8()
                .expect("consumer text setter only accepts valid native text")
        }))
    }
    fn set_icon(&self, v: Option<BoxedValue>) {
        self.0.set_icon(v)
    }
    fn large_icon(&self) -> Option<BoxedValue> {
        self.0.large_icon()
    }
    fn set_large_icon(&self, v: Option<BoxedValue>) {
        self.0.set_large_icon(v)
    }
    fn set_key_tip(&self, v: Option<RibbonString>) {
        self.0.set_key_tip(v.map(|v| {
            v.to_utf8()
                .expect("consumer text setter only accepts valid native text")
        }))
    }
    fn set_command_id(&self, v: Option<RibbonString>) {
        self.0.set_command_id(v.map(|v| {
            v.to_utf8()
                .expect("consumer text setter only accepts valid native text")
        }))
    }
    fn shortcut(&self) -> Option<RibbonString> {
        self.0.shortcut().map(Into::into)
    }
    fn set_shortcut(&self, v: Option<RibbonString>) {
        self.0.set_shortcut(v.map(|v| {
            v.to_utf8()
                .expect("consumer text setter only accepts valid native text")
        }))
    }
    fn screen_tip(&self) -> Option<BoxedValue> {
        self.0.screen_tip()
    }
    fn set_screen_tip(&self, v: Option<BoxedValue>) {
        self.0.set_screen_tip(v)
    }
    fn set_size(&self, v: RibbonItemSize) {
        self.0.set_size(v)
    }
    fn size_definition(&self) -> Option<RibbonString> {
        self.0.size_definition().map(Into::into)
    }
    fn set_size_definition(&self, v: Option<RibbonString>) {
        self.0.set_size_definition(v.map(|v| {
            v.to_utf8()
                .expect("consumer text setter only accepts valid native text")
        }))
    }
    fn set_simplified_visibility(&self, v: RibbonSimplifiedVisibility) {
        self.0.set_simplified_visibility(v)
    }
    fn simplified_label(&self) -> RibbonSimplifiedLabel {
        self.0.simplified_label()
    }
    fn set_simplified_label(&self, v: RibbonSimplifiedLabel) {
        self.0.set_simplified_label(v)
    }
    fn show_label(&self) -> bool {
        self.0.show_label()
    }
    fn set_show_label(&self, v: bool) {
        self.0.set_show_label(v)
    }
    fn set_can_add_to_quick_access(&self, v: bool) {
        self.0.set_can_add_to_quick_access(v)
    }
}
pub struct ItemContractViewModel {
    pub item: Ref<ContractItem>,
}
impl PartialEq for ItemContractViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl ItemContractViewModel {
    pub fn new() -> Rc<Self> {
        let item = ContractItem::new();
        item.set_label(Some("Original".into()));
        item.set_id(Some("public-item".into()));
        Rc::new(Self { item })
    }
    pub fn item(&self) -> Rc<dyn IRibbonItemEditable> {
        Rc::new(ItemView(self.item.clone()))
    }
}
ferroui_base::ferro_markup_type!(class ItemContractViewModel {
 namespace:"FerroRibbon.Consumer",this:Rc<ItemContractViewModel>,handles:[Rc<ItemContractViewModel>,Option<Rc<ItemContractViewModel>>],constructors:[()=>ItemContractViewModel::new],
 properties:[Item:Rc<dyn IRibbonItemEditable>{get:|v:&Rc<ItemContractViewModel>|v.item()}],
});

pub fn register() {
    use ferroui_base::{
        TypeInfo,
        metadata::{MarkupType, MarkupTyped},
    };
    TypeInfo::register_namespaces(&[(
        "ferroribbon_markup_consumer::item_contract_consumer",
        "FerroRibbon.Consumer",
    )]);
    TypeInfo::register_all(&[ContractItem::TYPE]);
    MarkupType::register_all(&[ItemContractViewModel::MARKUP]);
}

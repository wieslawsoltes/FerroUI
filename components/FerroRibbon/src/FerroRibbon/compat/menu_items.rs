//! Explicit source menu-platform adapters over native MenuItem. Native ToggleType
//! stays None: source toggling occurs inside Invoke, not in the menu interaction handler.
use ferroui_base::{
    BoxedValue, FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs,
    Ref, StaticType, StyledElementImpl, StyledProperty, TypeInfo, VisualImpl, ferro_class,
    ferro_class_info, ferro_impl_classes, ferro_properties, instantiate,
};
use ferroui_base::{
    input::InputElementImpl,
    interactivity::{InteractiveImpl, RoutedEventArgs},
    layout::LayoutableImpl,
    metadata::{MarkupDelegate, MarkupTyped},
};
use ferroui_controls::{
    Control, ControlImpl, ItemsControl, ItemsControlImpl, MenuItem, MenuItemImpl, TextBlock,
    automation::{AccessibilityView, AutomationProperties},
    primitives::{
        HeaderedSelectingItemsControlImpl, SelectingItemsControlImpl, TemplateAppliedEventArgs,
        TemplatedControlImpl, TemplatedControlImplExt,
    },
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
#[repr(C)]
pub struct MenuFlyoutItem {
    base: MenuItem,
    label: Ref<TextBlock>,
    clicks: RefCell<Option<MarkupDelegate>>,
    invoker: RefCell<Option<MarkupDelegate>>,
    gesture: RefCell<Option<ferroui_base::WeakRef<TextBlock>>>,
}
ferro_class! {MenuFlyoutItem:MenuItem,virtuals MenuFlyoutItemImpl:MenuItemImpl{fn invoke_menu_core(this);}}
ferro_class_info!(MenuFlyoutItem{new:MenuFlyoutItem::new,markup:{namespace:"FerroRibbon.Compatibility",events:[Click(Option<BoxedValue>,RoutedEventArgs){add:MenuFlyoutItem::add_click,remove:MenuFlyoutItem::remove_click}]}});
ferro_impl_classes!(MenuFlyoutItem:VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,ItemsControlImpl,SelectingItemsControlImpl,HeaderedSelectingItemsControlImpl);
ferro_properties! {impl MenuFlyoutItem{
 pub fn text_property()->StyledProperty<Option<String>>{FerroProperty::register::<Self,_>("Text",Some(String::new()))}
 pub fn keyboard_accelerator_text_override_property()->StyledProperty<Option<String>>{FerroProperty::register::<Self,_>("KeyboardAcceleratorTextOverride",None)}
}}
impl MenuFlyoutItem {
    pub fn construct() -> Self {
        Self {
            base: MenuItem::construct(),
            label: TextBlock::new(),
            clicks: RefCell::default(),
            invoker: RefCell::default(),
            gesture: RefCell::default(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn text(&self) -> Option<String> {
        self.get_value(Self::text_property())
    }
    pub fn set_text(&self, v: Option<String>) {
        self.set_value(Self::text_property(), v);
    }
    pub fn keyboard_accelerator_text_override(&self) -> String {
        self.get_value(Self::keyboard_accelerator_text_override_property())
            .unwrap_or_default()
    }
    pub fn set_keyboard_accelerator_text_override(&self, v: Option<String>) {
        self.set_value(Self::keyboard_accelerator_text_override_property(), v);
    }
    pub fn add_click(&self, h: Option<MarkupDelegate>) {
        let next = MarkupDelegate::combine(self.clicks.borrow().as_ref(), h.as_ref());
        self.clicks.replace(next);
    }
    pub fn remove_click(&self, h: Option<MarkupDelegate>) {
        let next = MarkupDelegate::remove(self.clicks.borrow().as_ref(), h.as_ref());
        self.clicks.replace(next);
    }
    pub fn add_click_action(&self, f: impl Fn(&MenuFlyoutItem, &RoutedEventArgs) + 'static) {
        ferroui_base::data::core::ValueTypes::register_reference::<ClickAction>();
        self.add_click(Some(MarkupDelegate::for_method(
            Some(Rc::new(ClickAction(Box::new(f)))),
            ClickAction::MARKUP,
            &ClickAction::MARKUP.methods[0],
        )));
    }
    pub(crate) fn invoker(&self) -> Option<MarkupDelegate> {
        self.invoker.borrow().clone()
    }
    pub(crate) fn set_invoker(&self, v: Option<MarkupDelegate>) {
        let old = self.invoker.replace(v);
        drop(old);
    }
    fn update_shortcut(&self) {
        let part = self.gesture.borrow().as_ref().and_then(|g| g.upgrade());
        if let Some(part) = part {
            part.set_text(Some(&self.keyboard_accelerator_text_override()));
        }
    }
}
impl FerroObjectImpl for MenuFlyoutItem {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.label.set_text(this.text().as_deref());
        this.set_header(Some(Control::boxed(this.label.clone())));
        AutomationProperties::set_labeled_by(this, this.label.clone().upcast::<Control>());
        AutomationProperties::set_accessibility_view(&this.label, AccessibilityView::Raw);
    }
    fn on_property_changed(this: &Self, e: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, e);
        if e.property() == Self::text_property().as_property() {
            this.label.set_text(this.text().as_deref());
        } else if e.property() == Self::keyboard_accelerator_text_override_property().as_property()
        {
            this.update_shortcut();
        }
    }
}
impl MenuItemImpl for MenuFlyoutItem {
    fn on_click(this: &Self, e: &RoutedEventArgs) {
        if e.is_source(this) {
            this.invoke_menu_core();
            e.set_handled(true);
        }
    }
}
impl MenuFlyoutItemImpl for MenuFlyoutItem {
    fn invoke_menu_core(this: &Self) {
        let handler = this.clicks.borrow().clone();
        if let Some(handler) = handler {
            let e = RoutedEventArgs::new();
            e.set_source(this.to_ref().upcast::<ferroui_base::FerroObject>());
            handler
                .try_invoke(&[Some(Control::boxed(this.to_ref())), Some(Rc::new(e))])
                .expect("menu Click handler");
        }
        if let Some(command) = this.command() {
            let parameter = this.command_parameter();
            if command.can_execute(parameter.as_ref()) {
                command.execute(parameter.as_ref());
            }
        }
    }
}
impl TemplatedControlImpl for MenuFlyoutItem {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, e);
        this.gesture.replace(
            e.name_scope()
                .find("PART_InputGestureText")
                .and_then(|o| o.cast::<TextBlock>())
                .map(|p| p.downgrade()),
        );
        this.update_shortcut();
    }
}
// The native default theme supplies the menu platform's visuals. Text is a literal
// TextBlock header so '-' and underscores do not become native separator/access-key syntax.
type ClickFn = dyn Fn(&MenuFlyoutItem, &RoutedEventArgs);
struct ClickAction(Box<ClickFn>);
impl PartialEq for ClickAction {
    fn eq(&self, o: &Self) -> bool {
        std::ptr::eq(self, o)
    }
}
ferroui_base::ferro_markup_type!(class ClickAction{this:Rc<ClickAction>,handles:[Rc<ClickAction>],methods:[fn Invoke(Option<BoxedValue>,RoutedEventArgs)=>|h:&Rc<ClickAction>,sender:Option<BoxedValue>,e|{let c=sender.as_ref().and_then(Control::from_boxed).and_then(|c|c.cast::<MenuFlyoutItem>()).expect("menu event source");(h.0)(&c,&e)}]});
#[repr(C)]
pub struct ToggleMenuFlyoutItem {
    base: MenuFlyoutItem,
}
ferro_class!(ToggleMenuFlyoutItem:MenuFlyoutItem);
ferro_class_info!(ToggleMenuFlyoutItem{new:ToggleMenuFlyoutItem::new,markup:{namespace:"FerroRibbon.Compatibility"}});
ferro_impl_classes!(ToggleMenuFlyoutItem:StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl,ItemsControlImpl,SelectingItemsControlImpl,HeaderedSelectingItemsControlImpl,MenuItemImpl);
impl ToggleMenuFlyoutItem {
    pub fn construct() -> Self {
        Self {
            base: MenuFlyoutItem::construct(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
impl FerroObjectImpl for ToggleMenuFlyoutItem {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.pseudo_classes().set(":toggle", true);
    }
}
impl MenuFlyoutItemImpl for ToggleMenuFlyoutItem {
    fn invoke_menu_core(this: &Self) {
        this.set_is_checked(!this.is_checked());
        Self::parent_invoke_menu_core(this);
    }
}
#[repr(C)]
pub struct MenuFlyoutSubItem {
    base: MenuItem,
    label: Ref<TextBlock>,
    items: crate::metadata::control_items::RibbonControlItems,
    items_token: Cell<Option<u64>>,
}
ferro_class!(MenuFlyoutSubItem:MenuItem);
ferro_class_info!(MenuFlyoutSubItem{new:MenuFlyoutSubItem::new,markup:{namespace:"FerroRibbon.Compatibility",content:Items,properties:[Items:crate::metadata::control_items::RibbonControlItems{get:MenuFlyoutSubItem::items}]}});
ferro_impl_classes!(MenuFlyoutSubItem:VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl,ItemsControlImpl,SelectingItemsControlImpl,HeaderedSelectingItemsControlImpl);
ferro_properties! {impl MenuFlyoutSubItem{pub fn text_property()->StyledProperty<Option<String>>{FerroProperty::register::<Self,_>("Text",Some(String::new()))}}}
impl MenuFlyoutSubItem {
    pub fn new() -> Ref<Self> {
        instantiate(Self {
            base: MenuItem::construct(),
            label: TextBlock::new(),
            items: crate::metadata::control_items::RibbonControlItems::new(),
            items_token: Cell::new(None),
        })
    }
    pub fn text(&self) -> Option<String> {
        self.get_value(Self::text_property())
    }
    pub fn set_text(&self, v: Option<String>) {
        self.set_value(Self::text_property(), v);
    }
    pub fn items(&self) -> crate::metadata::control_items::RibbonControlItems {
        self.items.clone()
    }
    fn sync(
        &self,
        e: &ferroui_system::collections::specialized::NotifyCollectionChangedEventArgs<
            '_,
            Option<Ref<Control>>,
        >,
    ) {
        use ferroui_system::collections::specialized::NotifyCollectionChangedAction as A;
        let items = ItemsControl::items(self);
        match e.action {
            A::Add => {
                for (offset, c) in e.new_items.iter().enumerate() {
                    items.insert(
                        usize::try_from(e.new_starting_index).expect("menu add index") + offset,
                        c.clone().map(Control::boxed),
                    );
                }
            }
            A::Remove => {
                for _ in e.old_items {
                    items.remove_at(
                        usize::try_from(e.old_starting_index).expect("menu remove index"),
                    );
                }
            }
            A::Replace => {
                for (offset, c) in e.new_items.iter().enumerate() {
                    items.set(
                        usize::try_from(e.new_starting_index).expect("menu replace index") + offset,
                        c.clone().map(Control::boxed),
                    );
                }
            }
            A::Move => {
                for _ in e.old_items {
                    items
                        .remove_at(usize::try_from(e.old_starting_index).expect("menu move index"));
                }
                for (offset, c) in e.new_items.iter().enumerate() {
                    items.insert(
                        usize::try_from(e.new_starting_index).expect("menu move index") + offset,
                        c.clone().map(Control::boxed),
                    );
                }
            }
            A::Reset => items.clear(),
        }
    }
}
impl FerroObjectImpl for MenuFlyoutSubItem {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.label.set_text(this.text().as_deref());
        this.set_header(Some(Control::boxed(this.label.clone())));
        AutomationProperties::set_labeled_by(this, this.label.clone().upcast::<Control>());
        AutomationProperties::set_accessibility_view(&this.label, AccessibilityView::Raw);
        let weak = this.to_ref().downgrade();
        let token = this.items.add_collection_changed(Rc::new(move |e| {
            if let Some(this) = weak.upgrade() {
                this.sync(e);
            }
        }));
        this.items_token.set(Some(token));
    }
    fn on_property_changed(this: &Self, e: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, e);
        if e.property() == Self::text_property().as_property() {
            this.label.set_text(this.text().as_deref());
        }
    }
}
impl Drop for MenuFlyoutSubItem {
    fn drop(&mut self) {
        if let Some(token) = self.items_token.take() {
            self.items.remove_collection_changed(token);
        }
    }
}
impl MenuItemImpl for MenuFlyoutSubItem {
    fn on_click(_this: &Self, e: &RoutedEventArgs) {
        e.set_handled(true);
    }
}
impl StyledElementImpl for MenuFlyoutSubItem {
    fn style_key_override(_this: &Self) -> &'static TypeInfo {
        <MenuItem as StaticType>::TYPE
    }
}
impl StyledElementImpl for MenuFlyoutItem {
    fn style_key_override(_this: &Self) -> &'static TypeInfo {
        <MenuItem as StaticType>::TYPE
    }
}

#[repr(C)]
pub struct MenuFlyoutSeparator {
    base: Separator,
}

use ferroui_controls::Separator;
ferro_class!(MenuFlyoutSeparator:Separator);
ferro_class_info!(MenuFlyoutSeparator{new:MenuFlyoutSeparator::new,markup:{namespace:"FerroRibbon.Compatibility"}});
ferro_impl_classes!(MenuFlyoutSeparator:FerroObjectImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl);
impl MenuFlyoutSeparator {
    pub fn new() -> Ref<Self> {
        instantiate(Self {
            base: Separator::construct(),
        })
    }
}
impl StyledElementImpl for MenuFlyoutSeparator {
    fn style_key_override(_this: &Self) -> &'static TypeInfo {
        <Separator as StaticType>::TYPE
    }
}

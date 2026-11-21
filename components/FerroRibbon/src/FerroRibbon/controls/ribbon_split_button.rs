//! RibbonSplitButton.cs: source two-part layout, actions, last choice and links.
use super::*;
use crate::{
    compat::{FontIcon, MenuFlyoutItem, MenuFlyoutSubItem},
    primitives::{RibbonItemContent, RibbonItemContentPart},
};
use ferroribbon_core::RibbonItemSize;
use ferroribbon_core::collections::RibbonFixedItems;
use ferroui_base::{
    BoxedValue, FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs,
    Ref, StyledElementImpl, StyledProperty, VisualImpl, WeakRef,
    controls::NameScopeRef,
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties,
    input::{InputElement, InputElementImpl, InputElementImplExt, Key, KeyEventArgs, KeyModifiers},
    instantiate,
    interactivity::{InteractiveImpl, RoutedEventArgs},
    layout::{LayoutableImpl, LayoutableImplExt},
    media::IBrush,
    metadata::{MarkupDelegate, MarkupTyped},
    reactive::IDisposable,
};
use ferroui_controls::{
    Button, ColumnDefinition, Control, ControlImpl, Grid, GridLength, MenuFlyout, RowDefinition,
    automation::AutomationProperties,
    primitives::{
        FlyoutBase, PopupFlyoutBase, TemplateAppliedEventArgs, TemplatedControlImpl,
        TemplatedControlImplExt,
    },
};
use std::{cell::RefCell, rc::Rc};
#[derive(Default)]
struct Parts {
    scope: Option<NameScopeRef>,
    primary: Option<Ref<Button>>,
    secondary: Option<Ref<Button>>,
    primary_token: Option<ferroui_base::interactivity::RoutedEventHandlerToken>,
    secondary_token: Option<ferroui_base::interactivity::RoutedEventHandlerToken>,
}
#[repr(C)]
pub struct RibbonSplitButton {
    base: RibbonControlBase,
    parts: RefCell<Parts>,
    clicks: RefCell<Option<MarkupDelegate>>,
    flyout_subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
    choices: RefCell<Vec<Ref<MenuFlyoutItem>>>,
    choice_handler: RefCell<Option<MarkupDelegate>>,
}
ferro_class! {RibbonSplitButton:RibbonControlBase,virtuals RibbonSplitButtonImpl:RibbonControlBaseImpl{fn invoke_primary(this);}}
ferro_class_info!(RibbonSplitButton{new:RibbonSplitButton::new,markup:{namespace:"FerroRibbon.Controls",content:Flyout,properties:[IsDropDownOpen:bool{get:RibbonSplitButton::is_drop_down_open}],events:[Click(Option<BoxedValue>,RoutedEventArgs){add:RibbonSplitButton::add_click,remove:RibbonSplitButton::remove_click}],methods:[fn OpenDropDown()=>|this:&RibbonSplitButton|this.open_drop_down(),fn CloseDropDown()=>|this:&RibbonSplitButton|this.close_drop_down()]},interfaces:[Rc<dyn IRibbonCheckable> => |v|Rc::new(Checkable(v)) as Rc<dyn IRibbonCheckable>]});
ferro_impl_classes!(RibbonSplitButton:VisualImpl,InteractiveImpl);
ferro_properties! {impl RibbonSplitButton{
 pub fn flyout_property()->StyledProperty<Option<Ref<FlyoutBase>>>{FerroProperty::register::<Self,_>("Flyout",None)}
 pub fn is_checkable_property()->StyledProperty<bool>{FerroProperty::register::<Self,_>("IsCheckable",false)}
 pub fn is_checked_property()->StyledProperty<Option<bool>>{FerroProperty::register::<Self,_>("IsChecked",Some(false))}
 pub fn follow_last_choice_property()->StyledProperty<bool>{FerroProperty::register::<Self,_>("FollowLastChoice",false)}
 pub fn last_choice_property()->StyledProperty<Option<Ref<MenuFlyoutItem>>>{FerroProperty::register::<Self,_>("LastChoice",None)}
 pub fn color_bar_property()->StyledProperty<Option<Rc<dyn IBrush>>>{FerroProperty::register::<Self,_>("ColorBar",None)}
}}
impl RibbonSplitButton {
    fn static_constructor() {
        Self::flyout_property()
            .changed()
            .add_class_handler::<Self>(|this, e| {
                let (old, new) = e.get_old_and_new_value::<Option<Ref<FlyoutBase>>>();
                this.on_flyout_changed(old, new);
            })
            .detach();
        for p in [
            Self::is_checkable_property().as_property(),
            Self::is_checked_property().as_property(),
        ] {
            p.changed()
                .add_class_handler::<Self>(|this, _| this.update_states())
                .detach();
        }
        Self::follow_last_choice_property()
            .changed()
            .add_class_handler::<Self>(|this, _| this.update_contents())
            .detach();
        Self::color_bar_property()
            .changed()
            .add_class_handler::<Self>(|this, _| this.update_contents())
            .detach();
        Self::last_choice_property()
            .changed()
            .add_class_handler::<Self>(|this, _| {
                this.update_contents();
                RibbonItemHelper::invalidate_host_layout(this);
            })
            .detach();
    }
    pub fn construct() -> Self {
        Self {
            base: RibbonControlBase::construct(),
            parts: RefCell::default(),
            clicks: RefCell::default(),
            flyout_subscriptions: RefCell::default(),
            choices: RefCell::default(),
            choice_handler: RefCell::default(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn flyout(&self) -> Option<Ref<FlyoutBase>> {
        self.get_value(Self::flyout_property())
    }
    pub fn set_flyout(&self, v: Option<Ref<FlyoutBase>>) {
        self.set_value(Self::flyout_property(), v);
    }
    pub fn is_checkable(&self) -> bool {
        self.get_value(Self::is_checkable_property())
    }
    pub fn set_is_checkable(&self, v: bool) {
        self.set_value(Self::is_checkable_property(), v);
    }
    pub fn is_checked(&self) -> Option<bool> {
        self.get_value(Self::is_checked_property())
    }
    pub fn set_is_checked(&self, v: Option<bool>) {
        self.set_value(Self::is_checked_property(), v);
    }
    pub fn follow_last_choice(&self) -> bool {
        self.get_value(Self::follow_last_choice_property())
    }
    pub fn set_follow_last_choice(&self, v: bool) {
        self.set_value(Self::follow_last_choice_property(), v);
    }
    pub fn last_choice(&self) -> Option<Ref<MenuFlyoutItem>> {
        self.get_value(Self::last_choice_property())
    }
    pub fn set_last_choice(&self, v: Option<Ref<MenuFlyoutItem>>) {
        self.set_value(Self::last_choice_property(), v);
    }
    pub fn color_bar(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::color_bar_property())
    }
    pub fn set_color_bar(&self, v: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::color_bar_property(), v);
    }
    pub fn is_drop_down_open(&self) -> bool {
        self.flyout().is_some_and(|f| f.is_open())
    }
    pub fn add_click(&self, h: Option<MarkupDelegate>) {
        let next = MarkupDelegate::combine(self.clicks.borrow().as_ref(), h.as_ref());
        self.clicks.replace(next);
    }
    pub fn remove_click(&self, h: Option<MarkupDelegate>) {
        let next = MarkupDelegate::remove(self.clicks.borrow().as_ref(), h.as_ref());
        self.clicks.replace(next);
    }
    pub fn raise_click(&self) {
        let handler = self.clicks.borrow().clone();
        if let Some(handler) = handler {
            handler
                .try_invoke(&[
                    Some(Control::boxed(self.to_ref())),
                    Some(Rc::new(RoutedEventArgs::new())),
                ])
                .expect("split Click handler");
        }
    }
    pub fn open_drop_down(&self) {
        if self.is_effectively_enabled()
            && let Some(f) = self.flyout()
        {
            f.show_at(self);
        }
    }
    pub fn close_drop_down(&self) {
        if let Some(f) = self.flyout() {
            f.hide();
        }
    }
    fn part<T: ferroui_base::ObjectType>(&self, name: &str) -> Option<Ref<T>> {
        self.parts
            .borrow()
            .scope
            .as_ref()
            .and_then(|s| s.find(name))
            .and_then(|o| o.cast())
    }
    fn detach_parts(&self) {
        let parts = std::mem::take(&mut *self.parts.borrow_mut());
        if let (Some(p), Some(t)) = (parts.primary, parts.primary_token) {
            p.remove_handler(Button::click_event(), t);
        }
        if let (Some(p), Some(t)) = (parts.secondary, parts.secondary_token) {
            p.remove_handler(Button::click_event(), t);
        }
    }
    fn apply_part_layout(&self) {
        let (Some(root), Some(primary), Some(secondary)) = (
            self.part::<Grid>("PART_Root"),
            self.part::<Button>("PART_PrimaryButton"),
            self.part::<Button>("PART_SecondaryButton"),
        ) else {
            return;
        };
        let vertical = self.current_size() == RibbonItemSize::Large && !self.is_simplified();
        root.row_definitions().clear();
        root.column_definitions().clear();
        if vertical {
            let r = RowDefinition::new();
            r.set_height(GridLength::AUTO);
            root.row_definitions().add(r);
            let r = RowDefinition::new();
            r.set_height(GridLength::STAR);
            root.row_definitions().add(r);
            Grid::set_row(&primary, 0);
            Grid::set_column(&primary, 0);
            Grid::set_row(&secondary, 1);
            Grid::set_column(&secondary, 0);
        } else {
            for _ in 0..2 {
                let c = ColumnDefinition::new();
                c.set_width(GridLength::AUTO);
                root.column_definitions().add(c);
            }
            Grid::set_row(&primary, 0);
            Grid::set_column(&primary, 0);
            Grid::set_row(&secondary, 0);
            Grid::set_column(&secondary, 1);
        }
        self.pseudo_classes().set(":vertical", vertical);
        self.pseudo_classes().set(":horizontal", !vertical);
        self.update_contents();
        self.invalidate_measure();
    }
    fn update_contents(&self) {
        let (Some(primary), Some(secondary)) = (
            self.part::<RibbonItemContent>("PART_PrimaryContent"),
            self.part::<RibbonItemContent>("PART_SecondaryContent"),
        ) else {
            return;
        };
        let vertical = self.current_size() == RibbonItemSize::Large && !self.is_simplified();
        for content in [&primary, &secondary] {
            content.set_metrics(self.metrics());
            content.set_is_simplified(self.is_simplified());
            content.set_item_size(self.current_size());
        }
        let (label, icon, large) = self.primary_presentation();
        primary.set_icon(icon);
        primary.set_large_icon(large);
        primary.set_color_bar(self.color_bar());
        if vertical {
            primary.set_part(RibbonItemContentPart::IconOnly);
            primary.set_label(label.clone());
            primary.set_show_chevron(false);
            secondary.set_part(RibbonItemContentPart::LabelAndChevron);
            secondary.set_icon(None);
            secondary.set_label(self.label());
            secondary.set_show_chevron(true);
        } else {
            primary.set_part(RibbonItemContentPart::All);
            primary.set_label(label.clone());
            primary.set_show_label(self.actual_show_label());
            primary.set_show_chevron(false);
            secondary.set_part(RibbonItemContentPart::All);
            secondary.set_icon(None);
            secondary.set_label(None);
            secondary.set_show_chevron(true);
        }
        self.update_part_names(label);
    }
    pub(crate) fn primary_presentation(
        &self,
    ) -> (Option<String>, Option<BoxedValue>, Option<BoxedValue>) {
        let Some(choice) = self.last_choice().filter(|_| self.follow_last_choice()) else {
            return (self.label(), self.icon(), self.large_icon());
        };
        let font = choice
            .icon()
            .and_then(|i| ferroui_base::data::core::ValueTypes::as_object(&*i))
            .and_then(|o| o.cast::<FontIcon>());
        let icon = if let Some(font) = &font {
            font.glyph().map(|s| Rc::new(s) as BoxedValue)
        } else {
            choice.tag().or_else(|| self.icon())
        };
        let text = choice
            .text()
            .filter(|s| !s.is_empty())
            .or_else(|| self.label());
        let large = if font.is_some() || choice.tag().is_some() {
            icon.clone()
        } else {
            self.large_icon()
        };
        (text, icon, large)
    }
    fn update_part_names(&self, label: Option<String>) {
        let name = label
            .or_else(|| self.label())
            .unwrap_or_default()
            .replace('\n', " ");
        if let Some(primary) = self.part::<Button>("PART_PrimaryButton") {
            AutomationProperties::set_name(&primary, Some(&name));
        }
        if let Some(secondary) = self.part::<Button>("PART_SecondaryButton") {
            AutomationProperties::set_name(&secondary, Some(&Self::format_options_name(&name)));
        }
    }
    pub(crate) fn format_options_name(label: &str) -> String {
        use ferroribbon_core::{
            RibbonString,
            localization::{RibbonFormatArgument, RibbonStrings, format_composite},
        };
        let strings = RibbonStrings::current();
        let format = strings
            .split_button_options()
            .expect("NullReference: SplitButtonOptions");
        let format = if format.as_utf16().windows(3).any(|w| w == [123, 48, 125]) {
            format
        } else {
            RibbonString::from("{0} options")
        };
        let text = format_composite(
            strings.culture(),
            &format,
            &RibbonFixedItems::new(vec![RibbonFormatArgument::from(label)]),
        )
        .expect("split options composite format");
        RibbonString::from_utf16(ferroribbon_core::text::trim(text.as_utf16()))
            .to_utf8()
            .expect("split options label has unpaired UTF16")
    }
    fn on_flyout_changed(&self, old: Option<Ref<FlyoutBase>>, new: Option<Ref<FlyoutBase>>) {
        let subs = std::mem::take(&mut *self.flyout_subscriptions.borrow_mut());
        for s in subs {
            s.dispose();
        }
        self.unhook_choices();
        RibbonItemHelper::track_flyout_owner(old, new.clone()).expect("split flyout owner");
        if let Some(new) = new {
            let popup = new
                .cast::<PopupFlyoutBase>()
                .expect("split FlyoutBase Opening mapping requires PopupFlyoutBase");
            // Uno FlyoutBase has its own inherited DataContext and synchronizes
            // it to Popup independently of open/closed placement parenting.
            // Native FlyoutBase is a FerroObject: supply that missing bridge,
            // using the framework's weak-source property observable.
            use ferroui_base::{FerroObjectExtensions, StyledElement, data::BindingPriority};
            let source: Ref<ferroui_base::FerroObject> = self.to_ref().upcast();
            let context = popup.popup().bind(
                StyledElement::data_context_property(),
                source.get_observable(StyledElement::data_context_property()),
                BindingPriority::LocalValue,
            );
            if popup
                .get_local_value(PopupFlyoutBase::placement_property())
                .is_none()
            {
                popup.set_placement(ferroui_controls::PlacementMode::BottomEdgeAlignedLeft);
            }
            let weak = self.to_ref().downgrade();
            let opening = popup.opening(move |_| {
                if let Some(this) = weak.upgrade() {
                    this.on_flyout_opening();
                }
            });
            let weak = self.to_ref().downgrade();
            let opened = new.opened(move || {
                if let Some(this) = weak.upgrade() {
                    this.on_flyout_opened_or_closed();
                }
            });
            let weak = self.to_ref().downgrade();
            let closed = new.closed(move || {
                if let Some(this) = weak.upgrade() {
                    this.on_flyout_opened_or_closed();
                }
            });
            self.flyout_subscriptions.replace(vec![
                context,
                opening.detach(),
                opened.detach(),
                closed.detach(),
            ]);
        }
    }
    fn on_flyout_opened_or_closed(&self) {
        self.update_states();
        if let Some(peer) =
            ferroui_controls::automation::peers::ControlAutomationPeer::from_element(self)
                .and_then(|p| p.cast::<super::RibbonSplitButtonAutomationPeer>())
        {
            peer.source_flyout_changed();
        }
    }
    fn on_flyout_opening(&self) {
        let Some(menu) = self
            .flyout()
            .filter(|_| {
                self.follow_last_choice() && RibbonItemHelper::get_source_item(self).is_none()
            })
            .and_then(|f| f.cast::<MenuFlyout>())
        else {
            return;
        };
        self.unhook_choices();
        self.hook_choices(super::ribbon_menu_cloner::native_menu_items(
            menu.items().clone(),
        ));
    }
    fn hook_choices(&self, items: RibbonOverflowMenuItems) {
        for item in items.iter().flatten() {
            if let Some(sub) = item.downcast_ref::<MenuFlyoutSubItem>() {
                self.hook_choices(ferroribbon_core::model::RibbonItemsSource::from_collection(
                    sub.items(),
                ));
            } else if let Some(item) = item.cast::<MenuFlyoutItem>() {
                let handler = self.choice_handler.borrow().clone();
                item.add_click(handler);
                self.choices.borrow_mut().push(item);
            }
        }
    }
    fn unhook_choices(&self) {
        let items = std::mem::take(&mut *self.choices.borrow_mut());
        let handler = self.choice_handler.borrow().clone();
        for item in items {
            item.remove_click(handler.clone());
        }
    }
    fn invoke_primary_from_copy(&self) {
        if self.is_checkable() {
            self.execute_command(self.is_checked().map(|s| Rc::new(s) as BoxedValue))
                .expect("split copy command");
            self.raise_click();
            return;
        }
        self.invoke_primary();
    }
    fn update_states(&self) {
        self.pseudo_classes().set(
            ":checked",
            self.is_checkable() && self.is_checked() == Some(true),
        );
        self.pseudo_classes()
            .set(":splitdisabled", !self.is_effectively_enabled());
        self.pseudo_classes()
            .set(":dropdownopen", self.is_drop_down_open());
    }
}
impl Drop for RibbonSplitButton {
    fn drop(&mut self) {
        self.unhook_choices();
        for s in self.flyout_subscriptions.get_mut().drain(..) {
            s.dispose();
        }
        self.detach_parts();
    }
}
impl FerroObjectImpl for RibbonSplitButton {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.set_size(RibbonItemSize::Large);
        ferroui_base::data::core::ValueTypes::register_reference::<ChoiceHandler>();
        let handler = Rc::new(ChoiceHandler(this.to_ref().downgrade()));
        this.choice_handler.replace(Some(MarkupDelegate::for_method(
            Some(handler),
            ChoiceHandler::MARKUP,
            &ChoiceHandler::MARKUP.methods[0],
        )));
        this.add_property_changed_handler(Rc::new(|e| {
            if e.property() == InputElement::is_effectively_enabled_property().as_property()
                && let Some(this) = e.sender().downcast_ref::<Self>()
            {
                this.update_states();
            }
        }));
    }
}
impl InputElementImpl for RibbonSplitButton {
    fn on_key_down(this: &Self, e: &KeyEventArgs) {
        Self::parent_on_key_down(this, e);
        if e.handled() {
            return;
        }
        if e.key == Key::F4 || (e.key == Key::Down && e.key_modifiers.contains(KeyModifiers::ALT)) {
            this.open_drop_down();
            e.set_handled(true);
        }
    }
}
impl LayoutableImpl for RibbonSplitButton {
    fn apply_template(this: &Self) {
        Self::parent_apply_template(this);
        if this.visual_children().count() == 0 {
            this.detach_parts();
        }
    }
}
impl TemplatedControlImpl for RibbonSplitButton {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        this.detach_parts();
        Self::parent_on_apply_template(this, e);
        let scope = e.name_scope().clone();
        let primary = scope
            .find("PART_PrimaryButton")
            .and_then(|o| o.cast::<Button>());
        let secondary = scope
            .find("PART_SecondaryButton")
            .and_then(|o| o.cast::<Button>());
        let primary_token = primary.as_ref().map(|b| {
            let weak = this.to_ref().downgrade();
            b.click(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    this.invoke_primary();
                }
            })
        });
        let secondary_token = secondary.as_ref().map(|b| {
            let weak = this.to_ref().downgrade();
            b.click(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    this.open_drop_down();
                }
            })
        });
        this.parts.replace(Parts {
            scope: Some(scope),
            primary,
            secondary,
            primary_token,
            secondary_token,
        });
        this.apply_part_layout();
        this.update_states();
    }
}
impl ControlImpl for RibbonSplitButton {
    fn on_create_automation_peer(
        this: &Self,
    ) -> Ref<ferroui_controls::automation::peers::AutomationPeer> {
        super::RibbonSplitButtonAutomationPeer::new(this).upcast()
    }
}
impl RibbonSplitButtonImpl for RibbonSplitButton {
    fn invoke_primary(this: &Self) {
        if this.is_checkable() {
            this.set_is_checked(Some(this.is_checked() != Some(true)));
        }
        if let Some(last) = this.last_choice().filter(|_| this.follow_last_choice()) {
            if let Some(action) = RibbonMenu::get_invoke(&last) {
                action.try_invoke(&[]).expect("split last choice action");
            } else if let Some(command) = last.command()
                && command.can_execute(last.command_parameter().as_ref())
            {
                command.execute(last.command_parameter().as_ref());
            }
            RibbonItemHelper::notify_invoked(this, last.command_parameter());
        } else {
            this.execute_command(if this.is_checkable() {
                this.is_checked().map(|s| Rc::new(s) as BoxedValue)
            } else {
                None
            })
            .expect("split primary command");
        }
        this.raise_click();
    }
}
impl RibbonControlBaseImpl for RibbonSplitButton {
    fn on_ribbon_property_changed(this: &Self, _: &FerroPropertyChangedEventArgs<'_>) {
        this.update_contents();
    }
    fn on_layout_applied_core(this: &Self, _: RibbonItemLayout) {
        this.apply_part_layout();
    }
    fn create_linked_copy_core(this: &Self) -> Option<Ref<Control>> {
        let copy = Self::new();
        RibbonItemHelper::link_common(this, &copy);
        for p in [
            Self::is_checkable_property().as_property(),
            Self::color_bar_property().as_property(),
            Self::follow_last_choice_property().as_property(),
            Self::last_choice_property().as_property(),
        ] {
            RibbonItemHelper::link_one_way(this, &copy, p);
        }
        RibbonItemHelper::link(this, &copy, Self::is_checked_property().as_property(), true);
        copy.set_flyout(this.flyout());
        let source = this.to_ref();
        copy.add_click(Some(super::ribbon_menu::click_delegate(move |_, _| {
            source.invoke_primary_from_copy()
        })));
        copy.set_command(None);
        Some(copy.upcast())
    }
    fn on_unlinked_core(this: &Self) {
        Self::parent_on_unlinked_core(this);
        this.set_flyout(None);
    }
    fn create_overflow_menu_items_core(this: &Self) -> RibbonOverflowMenuItems {
        let entry = if let Some(menu) = this.flyout().and_then(|f| f.cast::<MenuFlyout>()) {
            let sub = MenuFlyoutSubItem::new();
            sub.set_text(Some(this.label().unwrap_or_default()));
            sub.set_icon(
                RibbonItemHelper::create_menu_icon(this.icon())
                    .expect("split overflow icon")
                    .map(Control::boxed),
            );
            let source = this.to_ref();
            let primary = RibbonMenu::item(
                Some(this.label().unwrap_or_default()),
                Some(super::ribbon_menu::action_delegate(move || {
                    source.invoke_primary()
                })),
                this.icon(),
                None,
            )
            .expect("split overflow primary");
            sub.items().add(Some(primary.upcast()));
            sub.items()
                .add(Some(crate::compat::MenuFlyoutSeparator::new().upcast()));
            for entry in RibbonMenuCloner::clone(Some(
                super::ribbon_menu_cloner::native_menu_items(menu.items().clone()),
            ))
            .iter()
            {
                sub.items().add(entry);
            }
            sub.upcast()
        } else {
            let source = this.to_ref();
            super::ribbon_menu::overflow_entry(
                this,
                this.label(),
                this.icon(),
                this.is_checkable().then(|| this.is_checked() == Some(true)),
                move || source.invoke_primary(),
            )
        };
        ferroribbon_core::model::RibbonItemsSource::from_values(vec![Some(entry)])
    }
    fn on_key_tip_core(this: &Self) -> Rc<RibbonKeyTipResult> {
        this.open_drop_down();
        RibbonKeyTipResult::close()
    }
    fn invoke_core(this: &Self) -> bool {
        if !this.is_effectively_enabled() {
            return false;
        }
        this.invoke_primary();
        true
    }
    fn uses_command_can_execute(_: &Self) -> bool {
        true
    }
}
struct Checkable(Ref<RibbonSplitButton>);
impl IRibbonCheckable for Checkable {
    fn as_object(&self) -> Ref<ferroui_base::FerroObject> {
        self.0.clone().upcast()
    }
    fn is_checked(&self) -> Option<bool> {
        self.0.is_checked()
    }
    fn set_is_checked(&self, v: Option<bool>) {
        self.0.set_is_checked(v);
    }
}
struct ChoiceHandler(WeakRef<RibbonSplitButton>);
impl PartialEq for ChoiceHandler {
    fn eq(&self, o: &Self) -> bool {
        std::ptr::eq(self, o)
    }
}
ferroui_base::ferro_markup_type!(class ChoiceHandler{this:Rc<ChoiceHandler>,handles:[Rc<ChoiceHandler>],methods:[fn Invoke(Option<BoxedValue>,RoutedEventArgs)=>|this:&Rc<ChoiceHandler>,sender:Option<BoxedValue>,_|{if let Some(owner)=this.0.upgrade()&&let Some(item)=sender.as_ref().and_then(Control::from_boxed).and_then(|c|c.cast::<MenuFlyoutItem>()){owner.set_last_choice(Some(item));}}]});

impl StyledElementImpl for RibbonSplitButton {
    fn style_key_override(_: &Self) -> &'static ferroui_base::TypeInfo {
        <Self as ferroui_base::StaticType>::TYPE
    }
}

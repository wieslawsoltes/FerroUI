//! Port of shared RibbonBackstage.cs. Copyright Wiesław Šoltés, MIT.
use super::Ribbon;
use crate::{
    metadata::backstage_items::RibbonBackstageItems, primitives::RibbonIconPresenter,
    theming::RibbonTheme,
};
use ferroribbon_core::{
    RibbonBackstagePlacement, localization::RibbonStrings, model::RibbonItemsSource,
};
use ferroui_base::{
    BoxedValue, FerroObject, FerroObjectExtensions, FerroObjectImpl, FerroObjectImplExt,
    FerroProperty, Ref, StyledElementImpl, StyledProperty, Visual, VisualImpl, WeakRef,
    animation::{
        Animation, AnimationTask, Cue, FillMode, KeyFrame, SignedDuration, easings::CubicEaseOut,
    },
    data::{BindingPriority, core::ValueTypes},
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties,
    input::{
        FocusChangedEventArgs, ICommand, InputElementImpl, InputElementImplExt, Key, KeyEventArgs,
        KeyModifiers, KeyboardNavigation, KeyboardNavigationMode, NavigationMethod,
    },
    instantiate,
    interactivity::{InteractiveImpl, RoutedEventHandlerToken},
    layout::{LayoutableImpl, Orientation, VerticalAlignment},
    media::{SolidColorBrush, TranslateTransform},
    metadata::MarkupDelegate,
    styling::{ControlTheme, Setter, ThemeVariant},
    threading::CancellationToken,
};
use ferroui_controls::{
    Application, Border, Button, Control, ControlImpl, StackPanel, TextBlock, ToolTip, TopLevel,
    automation::AutomationProperties,
    presenters::ContentPresenter,
    primitives::{
        TemplateAppliedEventArgs, TemplatedControl, TemplatedControlImpl, TemplatedControlImplExt,
    },
    shapes::{Rectangle, Shape},
    templates::IDataTemplate,
};
use std::{cell::RefCell, rc::Rc};

macro_rules! property {
    ($get:ident,$set:ident,$p:ident,$ty:ty) => {
        pub fn $get(&self) -> $ty {
            self.get_value(Self::$p())
        }
        pub fn $set(&self, value: $ty) {
            self.set_value(Self::$p(), value)
        }
    };
}

#[repr(C)]
pub struct RibbonBackstageItem {
    base: FerroObject,
    owner: RefCell<Option<WeakRef<RibbonBackstage>>>,
    navigation_button: RefCell<Option<WeakRef<Button>>>,
    navigation_icon: RefCell<Option<WeakRef<RibbonIconPresenter>>>,
    navigation_text: RefCell<Option<WeakRef<TextBlock>>>,
}
ferro_class!(RibbonBackstageItem:FerroObject);
ferro_impl_classes!(RibbonBackstageItem:FerroObjectImpl);
ferro_class_info!(RibbonBackstageItem{new:RibbonBackstageItem::new,markup:{namespace:"FerroRibbon.Controls",content:Content,properties:[IsPage:bool{get:RibbonBackstageItem::is_page},NavigationButton:Option<Ref<Button>>{get:RibbonBackstageItem::navigation_button}]}});
ferro_properties! {impl RibbonBackstageItem{
 pub fn header_property()->StyledProperty<Option<String>>{FerroProperty::register::<Self,_>("Header",None)}
 pub fn icon_property()->StyledProperty<Option<BoxedValue>>{FerroProperty::register::<Self,_>("Icon",None)}
 pub fn content_property()->StyledProperty<Option<BoxedValue>>{FerroProperty::register::<Self,_>("Content",None)}
 pub fn content_template_property()->StyledProperty<Option<Rc<dyn IDataTemplate>>>{FerroProperty::register::<Self,_>("ContentTemplate",None)}
 pub fn command_property()->StyledProperty<Option<Rc<dyn ICommand>>>{FerroProperty::register::<Self,_>("Command",None)}
 pub fn command_parameter_property()->StyledProperty<Option<BoxedValue>>{FerroProperty::register::<Self,_>("CommandParameter",None)}
 pub fn placement_property()->StyledProperty<RibbonBackstagePlacement>{FerroProperty::register::<Self,_>("Placement",RibbonBackstagePlacement::Top)}
 pub fn key_tip_property()->StyledProperty<Option<String>>{FerroProperty::register::<Self,_>("KeyTip",None)}
 pub fn has_separator_before_property()->StyledProperty<bool>{FerroProperty::register::<Self,_>("HasSeparatorBefore",false)}
 pub fn closes_backstage_property()->StyledProperty<bool>{FerroProperty::register::<Self,_>("ClosesBackstage",true)}
 pub fn is_enabled_property()->StyledProperty<bool>{FerroProperty::register::<Self,_>("IsEnabled",true)}
 pub fn is_visible_property()->StyledProperty<bool>{FerroProperty::register::<Self,_>("IsVisible",true)}
 pub fn screen_tip_property()->StyledProperty<Option<BoxedValue>>{FerroProperty::register::<Self,_>("ScreenTip",None)}
 pub fn id_property()->StyledProperty<Option<String>>{FerroProperty::register::<Self,_>("Id",None)}
}}
impl RibbonBackstageItem {
    fn static_constructor() {
        macro_rules! changed {($($p:ident),*)=>{$(Self::$p().changed().add_class_handler::<Self>(|item,e|{
            if let Some(owner)=item.owner(){owner.on_item_property_changed(item,e.property());}
        }).detach();)*};}
        changed!(
            header_property,
            icon_property,
            content_property,
            content_template_property,
            command_property,
            placement_property,
            key_tip_property,
            has_separator_before_property,
            is_enabled_property,
            is_visible_property,
            screen_tip_property,
            id_property
        );
    }
    pub fn construct() -> Self {
        Self {
            base: FerroObject::construct(),
            owner: RefCell::default(),
            navigation_button: RefCell::default(),
            navigation_icon: RefCell::default(),
            navigation_text: RefCell::default(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    property!(header, set_header, header_property, Option<String>);
    property!(icon, set_icon, icon_property, Option<BoxedValue>);
    property!(content, set_content, content_property, Option<BoxedValue>);
    property!(
        content_template,
        set_content_template,
        content_template_property,
        Option<Rc<dyn IDataTemplate>>
    );
    property!(
        command,
        set_command,
        command_property,
        Option<Rc<dyn ICommand>>
    );
    property!(
        command_parameter,
        set_command_parameter,
        command_parameter_property,
        Option<BoxedValue>
    );
    property!(
        placement,
        set_placement,
        placement_property,
        RibbonBackstagePlacement
    );
    property!(key_tip, set_key_tip, key_tip_property, Option<String>);
    property!(
        has_separator_before,
        set_has_separator_before,
        has_separator_before_property,
        bool
    );
    property!(
        closes_backstage,
        set_closes_backstage,
        closes_backstage_property,
        bool
    );
    property!(is_enabled, set_is_enabled, is_enabled_property, bool);
    property!(is_visible, set_is_visible, is_visible_property, bool);
    property!(
        screen_tip,
        set_screen_tip,
        screen_tip_property,
        Option<BoxedValue>
    );
    property!(id, set_id, id_property, Option<String>);
    pub fn is_page(&self) -> bool {
        self.content().is_some() || self.content_template().is_some()
    }
    pub fn navigation_button(&self) -> Option<Ref<Button>> {
        self.navigation_button
            .borrow()
            .as_ref()
            .and_then(WeakRef::upgrade)
    }
    pub(crate) fn owner(&self) -> Option<Ref<RibbonBackstage>> {
        self.owner.borrow().as_ref().and_then(WeakRef::upgrade)
    }
}

#[derive(Default)]
struct Parts {
    top: Option<Ref<StackPanel>>,
    bottom: Option<Ref<StackPanel>>,
    content: Option<Ref<ContentPresenter>>,
    back: Option<Ref<Button>>,
    title: Option<Ref<Control>>,
    content_root: Option<Ref<Control>>,
    back_click: Option<RoutedEventHandlerToken>,
}
#[repr(C)]
pub struct RibbonBackstage {
    base: TemplatedControl,
    items: RibbonBackstageItems,
    known_items: RefCell<Vec<Option<Ref<RibbonBackstageItem>>>>,
    parts: RefCell<Parts>,
    ribbon: RefCell<Option<WeakRef<Ribbon>>>,
    ribbon_theme: RefCell<Option<ferroui_base::reactive::Subscription>>,
    close_requested: RefCell<Option<MarkupDelegate>>,
    item_invoked: RefCell<Option<MarkupDelegate>>,
}
ferro_class!(RibbonBackstage:TemplatedControl);
ferro_impl_classes!(RibbonBackstage:VisualImpl,LayoutableImpl,InteractiveImpl,ControlImpl);
ferro_class_info!(RibbonBackstage{new:RibbonBackstage::new,markup:{namespace:"FerroRibbon.Controls",content:Items,properties:[
 Items:RibbonBackstageItems{get:RibbonBackstage::items},Ribbon:Option<Ref<Ribbon>>{get:RibbonBackstage::ribbon},BackButton:Option<Ref<Button>>{get:RibbonBackstage::back_button},PageTitle:Option<String>{get:RibbonBackstage::page_title}],
 methods:[fn EnsureSelection()=>RibbonBackstage::ensure_selection,fn FocusSelectedItem()->bool=>RibbonBackstage::focus_selected_item,fn Invoke(Option<Ref<RibbonBackstageItem>>)=>RibbonBackstage::invoke,fn AnimateContent()=>RibbonBackstage::animate_content],
 events:[CloseRequested(Option<BoxedValue>,ferroui_base::utilities::EventArgs){add:RibbonBackstage::add_close_requested,remove:RibbonBackstage::remove_close_requested},ItemInvoked(Option<BoxedValue>,Ref<RibbonBackstageItem>){add:RibbonBackstage::add_item_invoked,remove:RibbonBackstage::remove_item_invoked}]}});
ferro_properties! {impl RibbonBackstage{
 pub fn selected_item_property()->StyledProperty<Option<Ref<RibbonBackstageItem>>>{FerroProperty::register::<Self,_>("SelectedItem",None)}
 pub fn title_property()->StyledProperty<Option<String>>{FerroProperty::register::<Self,_>("Title",None)}
 pub fn navigation_pane_width_property()->StyledProperty<f64>{FerroProperty::register::<Self,_>("NavigationPaneWidth",220.)}
 pub fn show_page_title_property()->StyledProperty<bool>{FerroProperty::register::<Self,_>("ShowPageTitle",true)}
 pub fn pane_header_property()->StyledProperty<Option<BoxedValue>>{FerroProperty::register::<Self,_>("PaneHeader",None)}
 pub fn page_title_property()->StyledProperty<Option<String>>{FerroProperty::register::<Self,_>("PageTitle",None)}
}}
impl RibbonBackstage {
    fn static_constructor() {
        Self::selected_item_property()
            .changed()
            .add_class_handler::<Self>(|this, _| this.on_selected_item_changed())
            .detach();
        Self::show_page_title_property()
            .changed()
            .add_class_handler::<Self>(|this, _| this.on_selected_item_changed())
            .detach();
    }
    pub fn construct() -> Self {
        Self {
            base: TemplatedControl::construct(),
            items: RibbonBackstageItems::new_managed(),
            known_items: RefCell::default(),
            parts: RefCell::default(),
            ribbon: RefCell::default(),
            ribbon_theme: RefCell::default(),
            close_requested: RefCell::default(),
            item_invoked: RefCell::default(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn items(&self) -> RibbonBackstageItems {
        self.items.clone()
    }
    property!(
        selected_item,
        set_selected_item,
        selected_item_property,
        Option<Ref<RibbonBackstageItem>>
    );
    property!(title, set_title, title_property, Option<String>);
    property!(
        navigation_pane_width,
        set_navigation_pane_width,
        navigation_pane_width_property,
        f64
    );
    property!(
        show_page_title,
        set_show_page_title,
        show_page_title_property,
        bool
    );
    property!(
        pane_header,
        set_pane_header,
        pane_header_property,
        Option<BoxedValue>
    );
    pub fn page_title(&self) -> Option<String> {
        self.get_value(Self::page_title_property())
    }
    pub fn ribbon(&self) -> Option<Ref<Ribbon>> {
        self.ribbon.borrow().as_ref().and_then(WeakRef::upgrade)
    }
    pub(crate) fn set_ribbon(&self, value: Option<Ref<Ribbon>>) {
        if self.ribbon() == value {
            return;
        }
        if let Some(subscription) = self.ribbon_theme.take() {
            subscription.dispose();
        }
        self.ribbon.replace(value.as_ref().map(Ref::downgrade));
        if let Some(ribbon) = value {
            let weak = self.to_ref().downgrade();
            let subscription = ribbon.actual_theme_variant_changed(move || {
                if let Some(this) = weak.upgrade()
                    && this.is_loaded()
                    && let Some(ribbon) = this.ribbon()
                    && this.get_value(ThemeVariant::requested_theme_variant_property())
                        != ribbon.actual_theme_variant()
                {
                    this.set_value(
                        ThemeVariant::requested_theme_variant_property(),
                        ribbon.actual_theme_variant(),
                    );
                }
            });
            self.ribbon_theme.replace(Some(subscription));
        }
    }
    pub fn back_button(&self) -> Option<Ref<Button>> {
        self.parts.borrow().back.clone()
    }
    pub fn add_close_requested(&self, h: Option<MarkupDelegate>) {
        let next = MarkupDelegate::combine(self.close_requested.borrow().as_ref(), h.as_ref());
        self.close_requested.replace(next);
    }
    pub fn remove_close_requested(&self, h: Option<MarkupDelegate>) {
        let next = MarkupDelegate::remove(self.close_requested.borrow().as_ref(), h.as_ref());
        self.close_requested.replace(next);
    }
    pub fn add_item_invoked(&self, h: Option<MarkupDelegate>) {
        let next = MarkupDelegate::combine(self.item_invoked.borrow().as_ref(), h.as_ref());
        self.item_invoked.replace(next);
    }
    pub fn remove_item_invoked(&self, h: Option<MarkupDelegate>) {
        let next = MarkupDelegate::remove(self.item_invoked.borrow().as_ref(), h.as_ref());
        self.item_invoked.replace(next);
    }
    fn raise_close_requested(&self) {
        let h = self.close_requested.borrow().clone();
        if let Some(h) = h {
            h.try_invoke(&[
                Some(Control::boxed(self.to_ref())),
                Some(Rc::new(ferroui_base::utilities::EventArgs::EMPTY)),
            ])
            .expect("CloseRequested handler");
        }
    }
    fn on_items_changed(&self) {
        let removed: Vec<_> = self
            .known_items
            .borrow()
            .iter()
            .filter(|i| !self.items.contains(i))
            .cloned()
            .collect();
        for item in removed {
            let item = item.expect("NullReference: removed backstage item");
            if item
                .owner()
                .as_ref()
                .is_some_and(|o| o.ptr_eq(&self.to_ref()))
            {
                item.owner.take();
            }
            item.navigation_button.take();
            item.navigation_icon.take();
            item.navigation_text.take();
        }
        self.known_items.borrow_mut().clear();
        for item in self.items.iter() {
            self.known_items.borrow_mut().push(item);
        }
        for item in self.items.iter() {
            item.expect("NullReference: backstage item")
                .owner
                .replace(Some(self.to_ref().downgrade()));
        }
        self.build_navigation();
        self.ensure_selection();
    }
    pub fn ensure_selection(&self) {
        if self.selected_item().as_ref().is_none_or(|i| {
            !self.items.contains(&Some(i.clone())) || !i.is_visible() || !i.is_page()
        }) {
            self.set_selected_item(
                self.items
                    .iter()
                    .map(|i| i.expect("NullReference: backstage item"))
                    .find(|i| i.is_page() && i.is_visible()),
            );
        }
    }
    fn on_item_property_changed(&self, item: &RibbonBackstageItem, p: &'static FerroProperty) {
        if p == RibbonBackstageItem::header_property().as_property() {
            let text = item
                .navigation_text
                .borrow()
                .as_ref()
                .and_then(WeakRef::upgrade);
            if let Some(text) = text {
                text.set_text(Some(&item.header().unwrap_or_default()));
            }
            if let Some(button) = item.navigation_button() {
                AutomationProperties::set_name(&button, Some(&item.header().unwrap_or_default()));
            }
            if self
                .selected_item()
                .as_ref()
                .is_some_and(|i| i.ptr_eq(&item.to_ref()))
            {
                self.set_value(Self::page_title_property(), item.header());
            }
        } else if p == RibbonBackstageItem::icon_property().as_property() {
            let icon = item
                .navigation_icon
                .borrow()
                .as_ref()
                .and_then(WeakRef::upgrade);
            if let Some(icon) = icon {
                icon.set_icon(item.icon());
            }
        } else if p == RibbonBackstageItem::is_enabled_property().as_property() {
            if let Some(button) = item.navigation_button() {
                button.set_is_enabled(item.is_enabled());
            }
        } else if p == RibbonBackstageItem::id_property().as_property() {
            if let Some(button) = item.navigation_button() {
                Self::apply_automation_id(&button, item);
            }
        } else if p == RibbonBackstageItem::screen_tip_property().as_property() {
            if let Some(button) = item.navigation_button() {
                ToolTip::set_tip(&button, item.screen_tip());
            }
        } else if p == RibbonBackstageItem::content_property().as_property()
            || p == RibbonBackstageItem::content_template_property().as_property()
        {
            if self
                .selected_item()
                .as_ref()
                .is_some_and(|i| i.ptr_eq(&item.to_ref()))
                && item.is_page()
            {
                self.on_selected_item_changed();
            } else {
                self.ensure_selection();
            }
        } else if p == RibbonBackstageItem::is_visible_property().as_property()
            || p == RibbonBackstageItem::placement_property().as_property()
            || p == RibbonBackstageItem::has_separator_before_property().as_property()
        {
            self.build_navigation();
            self.ensure_selection();
        }
    }
    fn build_navigation(&self) {
        let (Some(top), Some(_bottom)) = ({
            let p = self.parts.borrow();
            (p.top.clone(), p.bottom.clone())
        }) else {
            return;
        };
        top.children().clear();
        let bottom = self
            .parts
            .borrow()
            .bottom
            .clone()
            .expect("NullReference: bottom items");
        bottom.children().clear();
        for item in self.items.iter() {
            let item = item.expect("NullReference: backstage item");
            let host = if item.placement() == RibbonBackstagePlacement::Bottom {
                self.parts.borrow().bottom.clone()
            } else {
                self.parts.borrow().top.clone()
            }
            .expect("NullReference: navigation host");
            if item.has_separator_before() && item.is_visible() {
                let separator = Rectangle::new();
                separator.set_height(1.);
                separator.set_margin(ferroui_base::Thickness::new(16., 6., 16., 6.));
                separator.set_opacity(0.35);
                RibbonTheme::set_theme_brush(
                    Some(&separator),
                    Some(Shape::fill_property().as_property()),
                    Some("RibbonBackstagePaneForegroundBrush"),
                )
                .expect("backstage separator brush");
                host.children().add(separator.upcast::<Control>());
            }
            let button = Button::new();
            button.set_tag(Some(Rc::new(item.clone())));
            if let Some(value) = Application::current().and_then(|app| {
                app.resources()
                    .try_get_resource(&"RibbonBackstageNavButtonStyle".into(), None)
                    .flatten()
            }) {
                let theme = ValueTypes::as_object(&*value)
                    .and_then(|v| v.cast::<ControlTheme>())
                    .expect("InvalidCast: backstage navigation style");
                button.set_theme(Some(theme));
            }
            let icon = RibbonIconPresenter::new();
            icon.set_icon(item.icon());
            icon.set_icon_size(18.);
            icon.set_width(18.);
            icon.set_height(18.);
            icon.bind(
                RibbonIconPresenter::foreground_property(),
                FerroObject::get_observable(&button, TemplatedControl::foreground_property()),
                BindingPriority::LocalValue,
            );
            let text = TextBlock::new();
            text.set_text(item.header().as_deref());
            text.set_vertical_alignment(VerticalAlignment::Center);
            let row = StackPanel::new();
            row.set_orientation(Orientation::Horizontal);
            row.set_spacing(12.);
            let border = Border::new();
            border.set_width(18.);
            border.set_height(18.);
            border.set_child(Some(icon.clone().upcast()));
            row.children().add(border.upcast::<Control>());
            row.children().add(text.clone().upcast::<Control>());
            button.set_content(Some(Control::boxed(row)));
            button.set_is_enabled(item.is_enabled());
            button.set_is_visible(item.is_visible());
            AutomationProperties::set_name(&button, Some(&item.header().unwrap_or_default()));
            Self::apply_automation_id(&button, &item);
            if item.screen_tip().is_some() {
                ToolTip::set_tip(&button, item.screen_tip());
            }
            let weak = self.to_ref().downgrade();
            let invoked = item.clone();
            button.click(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    this.invoke(Some(invoked.clone()));
                }
            });
            item.navigation_button.replace(Some(button.downgrade()));
            item.navigation_icon.replace(Some(icon.downgrade()));
            item.navigation_text.replace(Some(text.downgrade()));
            host.children().add(button.upcast::<Control>());
        }
        self.update_selection_visuals();
    }
    fn apply_automation_id(button: &Button, item: &RibbonBackstageItem) {
        if let Some(id) = item.id() {
            AutomationProperties::set_automation_id(button, Some(&format!("Backstage_{id}")));
        } else {
            button.clear_value(AutomationProperties::automation_id_property());
        }
    }
    pub fn invoke(&self, item: Option<Ref<RibbonBackstageItem>>) {
        let item = item.expect("ArgumentNull: item");
        if !item.is_enabled() || !item.is_visible() {
            return;
        }
        let handler = self.item_invoked.borrow().clone();
        if let Some(h) = handler {
            h.try_invoke(&[
                Some(Control::boxed(self.to_ref())),
                Some(Rc::new(item.clone())),
            ])
            .expect("ItemInvoked handler");
        }
        if item.is_page() {
            self.set_selected_item(Some(item));
            return;
        }
        if let Some(command) = item.command()
            && command.can_execute(item.command_parameter().as_ref())
        {
            command.execute(item.command_parameter().as_ref());
        }
        if let Some(ribbon) = self.ribbon() {
            ribbon.on_item_invoked(
                item.navigation_button()
                    .map(Ref::upcast::<Control>)
                    .unwrap_or_else(|| self.to_ref().upcast()),
                item.id().map(Into::into),
                item.command_parameter(),
            );
        }
        if item.closes_backstage() {
            self.raise_close_requested();
        }
    }
    fn on_selected_item_changed(&self) {
        let item = self.selected_item();
        self.set_value(
            Self::page_title_property(),
            item.as_ref().and_then(|i| i.header()),
        );
        let title = self.parts.borrow().title.clone();
        if let Some(title) = title {
            title.set_is_visible(self.show_page_title() && item.is_some());
        }
        let content = self.parts.borrow().content.clone();
        if let Some(content) = content {
            content.set_content_template(item.as_ref().and_then(|i| i.content_template()));
            let current = self
                .parts
                .borrow()
                .content
                .clone()
                .expect("NullReference: content presenter after template callback");
            current.set_content(item.as_ref().and_then(|i| i.content()));
        }
        self.update_selection_visuals();
        self.animate_content();
    }
    fn update_selection_visuals(&self) {
        let brush = RibbonTheme::get_brush(Some(self), Some("RibbonBackstagePaneSelectedBrush"))
            .expect("backstage selected brush");
        for item in self.items.iter() {
            let item = item.expect("NullReference: backstage item");
            if let Some(button) = item.navigation_button() {
                let selected = self
                    .selected_item()
                    .as_ref()
                    .is_some_and(|i| i.ptr_eq(&item));
                button.pseudo_classes().set(":ribbon-selected", selected);
                button.pseudo_classes().set(":unselected", !selected);
                button.set_tag(Some(Rc::new(item)));
                button.set_background(
                    if selected {
                        brush.clone().map(Rc::from)
                    } else {
                        None
                    }
                    .or_else(|| Some(Rc::from(SolidColorBrush::from_uint32(0x00ffffff)))),
                );
            }
        }
    }
    pub fn animate_content(&self) {
        let root = self.parts.borrow().content_root.clone();
        let Some(root) = root else { return };
        let transform = TranslateTransform::new();
        transform.set_x(24.);
        root.set_render_transform(Some(Rc::from(transform.clone())));
        root.set_opacity(0.);
        let fade = Animation::new();
        fade.set_duration(SignedDuration::from_milliseconds(180.));
        fade.set_fill_mode(FillMode::Forward);
        fade.children().add(KeyFrame::with_cue(
            Cue::new(0.),
            [Setter::new(Visual::opacity_property(), 0.)
                as Rc<dyn ferroui_base::animation::IAnimationSetter>],
        ));
        fade.children().add(KeyFrame::with_cue(
            Cue::new(1.),
            [Setter::new(Visual::opacity_property(), 1.)
                as Rc<dyn ferroui_base::animation::IAnimationSetter>],
        ));
        let slide = Animation::new();
        slide.set_duration(SignedDuration::from_milliseconds(220.));
        slide.set_fill_mode(FillMode::Forward);
        slide.set_easing(CubicEaseOut);
        slide.children().add(KeyFrame::with_cue(
            Cue::new(0.),
            [Setter::new(TranslateTransform::x_property(), 24.)
                as Rc<dyn ferroui_base::animation::IAnimationSetter>],
        ));
        slide.children().add(KeyFrame::with_cue(
            Cue::new(1.),
            [Setter::new(TranslateTransform::x_property(), 0.)
                as Rc<dyn ferroui_base::animation::IAnimationSetter>],
        ));
        let done = AnimationTask::when_all([
            fade.run_async(&root, CancellationToken::default()),
            slide.run_async(&root, CancellationToken::default()),
        ]);
        let weak = self.to_ref().downgrade();
        done.on_completed(move || {
            if let Some(this) = weak.upgrade() {
                let root = this
                    .parts
                    .borrow()
                    .content_root
                    .clone()
                    .expect("NullReference: current content root on animation completion");
                root.set_opacity(1.);
            }
        });
    }
    fn navigation_buttons(&self) -> Vec<Ref<Button>> {
        let mut buttons = Vec::new();
        for bottom in [false, true] {
            for item in self.items.iter() {
                let item = item.expect("NullReference: backstage item");
                if (item.placement() == RibbonBackstagePlacement::Bottom) == bottom
                    && let Some(button) = item.navigation_button()
                {
                    buttons.push(button);
                }
            }
        }
        buttons
    }
    fn is_navigable(button: &Button) -> bool {
        button.is_visible() && button.is_enabled()
    }
    pub fn focus_selected_item(&self) -> bool {
        let target = self
            .selected_item()
            .and_then(|i| i.navigation_button())
            .filter(|b| Self::is_navigable(b))
            .or_else(|| {
                self.navigation_buttons()
                    .into_iter()
                    .find(|b| Self::is_navigable(b))
            });
        target.is_some_and(|b| b.focus_with(NavigationMethod::Directional, KeyModifiers::NONE))
    }
    fn move_navigation_focus(&self, delta: isize) -> bool {
        let Some(root) = TopLevel::get_top_level(Some(self)) else {
            return false;
        };
        let focused = root.focus_manager().get_focused_element();
        let buttons: Vec<_> = self
            .navigation_buttons()
            .into_iter()
            .filter(|b| Self::is_navigable(b))
            .collect();
        let Some(index) = buttons
            .iter()
            .position(|b| focused.as_ref().is_some_and(|f| f.ptr_eq(b)))
        else {
            return false;
        };
        let next = (index as isize + delta + buttons.len() as isize) % buttons.len() as isize;
        buttons[next as usize].focus_with(NavigationMethod::Directional, KeyModifiers::NONE)
    }
    pub(crate) fn key_tip_targets(&self) -> RibbonItemsSource<Option<Ref<Control>>> {
        let owner = self.to_ref();
        RibbonItemsSource::deferred(move || {
            let owner = owner.clone();
            let mut back = false;
            let mut items = None;
            Box::new(std::iter::from_fn(move || {
                if !back {
                    back = true;
                    if let Some(button) = owner.back_button() {
                        return Some(Some(button.upcast()));
                    }
                }
                let iter = items.get_or_insert_with(|| owner.items.iter());
                for item in iter {
                    let Some(item) = item else { continue };
                    if item.is_visible()
                        && let Some(button) = item.navigation_button()
                    {
                        return Some(Some(button.upcast()));
                    }
                }
                None
            }))
        })
    }
}
impl FerroObjectImpl for RibbonBackstage {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        RibbonTheme::ensure_resources();
        this.set_is_tab_stop(true);
        // WinUI's focusable Control capability is separate from native tab
        // traversal eligibility. Opening focuses this control before its
        // OnGotFocus moves focus into navigation.
        this.set_focusable(true);
        KeyboardNavigation::set_tab_navigation(this, KeyboardNavigationMode::Cycle);
        let weak = this.to_ref().downgrade();
        this.items.add_collection_changed(Rc::new(move |_| {
            if let Some(this) = weak.upgrade() {
                this.on_items_changed();
            }
        }));
        let weak = this.to_ref().downgrade();
        this.actual_theme_variant_changed(move || {
            if let Some(this) = weak.upgrade() {
                this.update_selection_visuals();
            }
        })
        .detach();
        let weak = this.to_ref().downgrade();
        this.key_down(move |_, e| {
            if let Some(this) = weak.upgrade() {
                this.on_backstage_key_down(e);
            }
        });
    }
}
impl TemplatedControlImpl for RibbonBackstage {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        let (back, token, top) = {
            let mut p = this.parts.borrow_mut();
            (p.back.clone(), p.back_click.take(), p.top.clone())
        };
        if let (Some(back), Some(token)) = (back, token) {
            back.remove_handler(Button::click_event(), token);
        }
        if let Some(top) = top {
            top.children().clear();
        }
        let bottom = this.parts.borrow().bottom.clone();
        if let Some(bottom) = bottom {
            bottom.children().clear();
        }
        // Native presenters retain a visual child after their template subtree
        // is detached. Release the outgoing part before reusing the SAME page
        // in the new part. No page reconstruction occurs.
        let outgoing_content = this.parts.borrow().content.clone();
        if let Some(content) = outgoing_content {
            content.set_content(None);
            content.set_content_template(None);
        }
        Self::parent_on_apply_template(this, e);
        let scope = e.name_scope();
        let back = scope.find_as::<Button>("PART_BackButton");
        this.parts.replace(Parts {
            top: scope.find_as("PART_TopItems"),
            bottom: scope.find_as("PART_BottomItems"),
            content: scope.find_as("PART_Content"),
            back: back.clone(),
            title: scope.find_as("PART_PageTitle"),
            content_root: scope.find_as("PART_ContentRoot"),
            back_click: None,
        });
        if let Some(b) = back {
            let weak = this.to_ref().downgrade();
            let token = b.click(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    this.raise_close_requested();
                }
            });
            this.parts.borrow_mut().back_click = Some(token);
            let label = RibbonStrings::current()
                .back()
                .map(|s| s.to_utf8().expect("Back label UTF-16"));
            AutomationProperties::set_name(&b, label.as_deref());
            ToolTip::set_tip(&b, label.map(|s| Rc::new(s) as BoxedValue));
        }
        this.build_navigation();
        this.ensure_selection();
        this.on_selected_item_changed();
    }
}
impl InputElementImpl for RibbonBackstage {
    fn on_got_focus(this: &Self, e: &FocusChangedEventArgs) {
        Self::parent_on_got_focus(this, e);
        if e.source()
            .is_some_and(|source| source.ptr_eq(&this.to_ref()))
        {
            this.focus_selected_item();
        }
    }
}
impl RibbonBackstage {
    fn on_backstage_key_down(&self, e: &KeyEventArgs) {
        let this = self;
        match e.key {
            Key::Escape => {
                this.raise_close_requested();
                e.set_handled(true);
            }
            Key::Up | Key::Down
                if this.move_navigation_focus(if e.key == Key::Down { 1 } else { -1 }) =>
            {
                e.set_handled(true);
            }
            _ => {}
        }
    }
}

impl Drop for RibbonBackstage {
    fn drop(&mut self) {
        if let Some(subscription) = self.ribbon_theme.get_mut().take() {
            subscription.dispose();
        }
    }
}

impl StyledElementImpl for RibbonBackstage {
    fn style_key_override(_: &Self) -> &'static ferroui_base::TypeInfo {
        Self::TYPE
    }
}

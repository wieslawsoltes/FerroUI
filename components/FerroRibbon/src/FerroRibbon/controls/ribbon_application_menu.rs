//! Port of shared RibbonApplicationMenu.cs. Copyright Wiesław Šoltés, MIT.
//! The source builds this control's layout programmatically.
use super::{
    Ribbon, RibbonApplicationMenuItem as Item, RibbonApplicationMenuItemEventArgs as ItemArgs,
    RibbonApplicationMenuItems, RibbonApplicationMenuRecentItem as Recent,
    RibbonApplicationMenuRecentItems,
};
use crate::{primitives::RibbonIconPresenter, theming::RibbonTheme};
use ferroribbon_core::{
    RibbonString,
    localization::RibbonStrings,
    search::{RibbonSearchEngine, RibbonSearchEntry, RibbonSearchEntryOptions, RibbonSearchResult},
};
use ferroui_base::{
    BoxedValue, ElementRef, FerroObjectImpl, FerroObjectImplExt, FerroProperty, Ref, StyledElement,
    StyledElementImpl, StyledProperty, Thickness, VisualImpl, WeakRef,
    data::core::ValueTypes,
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties,
    input::{
        InputElementImpl, Key, KeyEventArgs, KeyModifiers, KeyboardNavigation, NavigationMethod,
    },
    instantiate,
    interactivity::InteractiveImpl,
    layout::{HorizontalAlignment, LayoutableImpl, Orientation, VerticalAlignment},
    media::{TextTrimming, TextWrapping},
    metadata::{MarkupDelegate, into_markup_value},
    reactive::IDisposable,
    utilities::HandlerList,
};
use ferroui_controls::{
    Application, Border, Button, ColumnDefinitions, ContentControl, ContentControlImpl, Control,
    ControlImpl, Grid, Panel, RowDefinitions, ScrollViewer, StackPanel, TextBlock, TextBox,
    ToolTip,
    automation::AutomationProperties,
    primitives::{
        Popup, ScrollBarVisibility, TemplatedControl, TemplatedControlImpl, ToggleButton,
    },
};
use ferroui_system::collections::{
    object_model::ObservableCollection, specialized::NotifyCollectionChangedEventArgs,
};
use std::{cell::RefCell, rc::Rc};
type ItemSubscription<T> = (WeakRef<T>, Rc<dyn IDisposable>);
type Invoked = dyn Fn(&RibbonApplicationMenu, &Rc<ItemArgs>);
type RecentInvoked = dyn Fn(&RibbonApplicationMenu, &Ref<Recent>);
pub type RibbonApplicationMenuFooterItems = ObservableCollection<Option<Ref<Control>>>;
struct Parts {
    commands: Ref<StackPanel>,
    pane: Ref<StackPanel>,
    pane_header: Ref<TextBlock>,
    footer: Ref<StackPanel>,
    search: Ref<TextBox>,
    search_row: Ref<Border>,
}
impl Parts {
    fn new() -> Self {
        Self {
            commands: StackPanel::new(),
            pane: StackPanel::new(),
            pane_header: TextBlock::new(),
            footer: StackPanel::new(),
            search: TextBox::new(),
            search_row: Border::new(),
        }
    }
}
#[repr(C)]
pub struct RibbonApplicationMenu {
    base: ContentControl,
    items: RibbonApplicationMenuItems,
    recent_items: RibbonApplicationMenuRecentItems,
    footer_items: RibbonApplicationMenuFooterItems,
    parts: Parts,
    shown_item: RefCell<Option<Ref<Item>>>,
    command_buttons: RefCell<Vec<(Ref<Item>, Ref<Button>)>>,
    item_subscriptions: RefCell<Vec<ItemSubscription<Item>>>,
    recent_subscriptions: RefCell<Vec<ItemSubscription<Recent>>>,
    collection_tokens: RefCell<Option<(u64, u64, u64)>>,
    footer_callbacks: RefCell<
        Vec<(
            WeakRef<Button>,
            ferroui_base::interactivity::RoutedEventHandlerToken,
        )>,
    >,
    invoked: HandlerList<Invoked>,
    recent_invoked: HandlerList<RecentInvoked>,
    markup_invoked: RefCell<Vec<(MarkupDelegate, u64)>>,
    markup_recent: RefCell<Vec<(MarkupDelegate, u64)>>,
}
ferro_class!(RibbonApplicationMenu:ContentControl);
ferro_impl_classes!(RibbonApplicationMenu:StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl,ContentControlImpl);
ferro_class_info!(RibbonApplicationMenu{new:RibbonApplicationMenu::new,markup:{namespace:"FerroRibbon.Controls",content:Items,properties:[Ribbon:Option<Ref<Ribbon>>{get:RibbonApplicationMenu::ribbon,set:RibbonApplicationMenu::set_ribbon},Items:RibbonApplicationMenuItems{get:RibbonApplicationMenu::items},RecentItems:RibbonApplicationMenuRecentItems{get:RibbonApplicationMenu::recent_items},FooterItems:RibbonApplicationMenuFooterItems{get:RibbonApplicationMenu::footer_items},ShownItem:Option<Ref<Item>>{get:RibbonApplicationMenu::shown_item}],methods:[fn Invoke(Ref<Item>)=>RibbonApplicationMenu::invoke,fn Invoke(Ref<Recent>)=>RibbonApplicationMenu::invoke_recent],events:[ItemInvoked(Option<BoxedValue>,Rc<ItemArgs>){add:RibbonApplicationMenu::add_item_invoked,remove:RibbonApplicationMenu::remove_item_invoked},RecentItemInvoked(Option<BoxedValue>,Ref<Recent>){add:RibbonApplicationMenu::add_recent_item_invoked,remove:RibbonApplicationMenu::remove_recent_item_invoked}]}});
ferro_properties! {impl RibbonApplicationMenu{
    pub fn ribbon_property()->StyledProperty<Option<ElementRef<Ribbon>>>{ValueTypes::register_element_ref::<Ribbon>();FerroProperty::register::<Self,_>("Ribbon",None)}
    pub fn recent_header_property()->StyledProperty<Option<String>>{FerroProperty::register::<Self,_>("RecentHeader",None)}
    pub fn is_search_visible_property()->StyledProperty<bool>{FerroProperty::register::<Self,_>("IsSearchVisible",true)}
}}
fn brush(target: &StyledElement, property: &'static FerroProperty, key: &str) {
    RibbonTheme::set_theme_brush(Some(target), Some(property), Some(key))
        .expect("application menu theme brush");
}
fn localized(value: Option<RibbonString>) -> Option<String> {
    value.map(|s| {
        s.to_utf8()
            .expect("localized application menu text requires native UTF-8")
    })
}
impl RibbonApplicationMenu {
    fn static_constructor() {
        Self::recent_header_property()
            .changed()
            .add_class_handler::<Self>(|c, _| c.show_default_pane())
            .detach();
        Self::is_search_visible_property()
            .changed()
            .add_class_handler::<Self>(|c, _| {
                c.parts.search_row.set_is_visible(c.is_search_visible())
            })
            .detach();
    }
    pub fn construct() -> Self {
        Self {
            base: ContentControl::construct(),
            items: ObservableCollection::new_managed(),
            recent_items: ObservableCollection::new_managed(),
            footer_items: ObservableCollection::new_managed(),
            parts: Parts::new(),
            shown_item: RefCell::default(),
            command_buttons: RefCell::default(),
            item_subscriptions: RefCell::default(),
            recent_subscriptions: RefCell::default(),
            collection_tokens: RefCell::default(),
            footer_callbacks: RefCell::default(),
            invoked: HandlerList::new(),
            recent_invoked: HandlerList::new(),
            markup_invoked: RefCell::default(),
            markup_recent: RefCell::default(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn items(&self) -> RibbonApplicationMenuItems {
        self.items.clone()
    }
    pub fn recent_items(&self) -> RibbonApplicationMenuRecentItems {
        self.recent_items.clone()
    }
    pub fn footer_items(&self) -> RibbonApplicationMenuFooterItems {
        self.footer_items.clone()
    }
    pub fn ribbon(&self) -> Option<Ref<Ribbon>> {
        ElementRef::resolve(&self.get_value(Self::ribbon_property()))
    }
    pub fn set_ribbon(&self, value: Option<Ref<Ribbon>>) {
        self.set_value(Self::ribbon_property(), ElementRef::from_nullable(value));
    }
    pub fn recent_header(&self) -> Option<String> {
        self.get_value(Self::recent_header_property())
    }
    pub fn set_recent_header(&self, value: Option<String>) {
        self.set_value(Self::recent_header_property(), value);
    }
    pub fn is_search_visible(&self) -> bool {
        self.get_value(Self::is_search_visible_property())
    }
    pub fn set_is_search_visible(&self, value: bool) {
        self.set_value(Self::is_search_visible_property(), value);
    }
    pub fn shown_item(&self) -> Option<Ref<Item>> {
        self.shown_item.borrow().clone()
    }
    pub fn invoke(&self, item: Ref<Item>) {
        if !item.is_enabled() {
            return;
        }
        if !item.items().is_empty() {
            self.show_sub_items(&item);
            return;
        }
        self.close_host();
        item.raise_click(item.command_parameter());
        if let Some(command) = item.command()
            && command.can_execute(item.command_parameter().as_ref())
        {
            command.execute(item.command_parameter().as_ref());
        }
        let args = ItemArgs::new(item.clone(), item.command_parameter());
        for (_, handler) in self.invoked.snapshot().iter() {
            handler(self, &args);
        }
    }
    pub fn invoke_recent(&self, item: Ref<Recent>) {
        self.close_host();
        item.raise_click();
        if let Some(command) = item.command() {
            let parameter = into_markup_value(item.clone());
            if command.can_execute(parameter.as_ref()) {
                command.execute(parameter.as_ref());
            }
        }
        for (_, handler) in self.recent_invoked.snapshot().iter() {
            handler(self, &item);
        }
    }
    pub fn item_invoked(
        &self,
        handler: impl Fn(&Self, &Rc<ItemArgs>) + 'static,
    ) -> Rc<dyn IDisposable> {
        let token = self.invoked.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        ferroui_base::reactive::Disposable::create(move || {
            if let Some(menu) = weak.upgrade() {
                menu.invoked.remove(token);
            }
        })
    }
    pub fn recent_item_invoked(
        &self,
        handler: impl Fn(&Self, &Ref<Recent>) + 'static,
    ) -> Rc<dyn IDisposable> {
        let token = self.recent_invoked.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        ferroui_base::reactive::Disposable::create(move || {
            if let Some(menu) = weak.upgrade() {
                menu.recent_invoked.remove(token);
            }
        })
    }
    pub fn add_item_invoked(&self, handler: Option<MarkupDelegate>) {
        let Some(handler) = handler else {
            return;
        };
        let callback = handler.clone();
        let token = self.invoked.add(Rc::new(move |menu, args| {
            callback
                .try_invoke(&[
                    into_markup_value(menu.to_ref()),
                    into_markup_value(args.clone()),
                ])
                .expect("ItemInvoked handler");
        }));
        self.markup_invoked.borrow_mut().push((handler, token));
    }
    pub fn remove_item_invoked(&self, handler: Option<MarkupDelegate>) {
        let removed = {
            let mut h = self.markup_invoked.borrow_mut();
            h.iter()
                .rposition(|(h, _)| Some(h) == handler.as_ref())
                .map(|i| h.remove(i))
        };
        if let Some((_, token)) = removed {
            self.invoked.remove(token);
        }
    }
    pub fn add_recent_item_invoked(&self, handler: Option<MarkupDelegate>) {
        let Some(handler) = handler else {
            return;
        };
        let callback = handler.clone();
        let token = self.recent_invoked.add(Rc::new(move |menu, item| {
            callback
                .try_invoke(&[
                    into_markup_value(menu.to_ref()),
                    into_markup_value(item.clone()),
                ])
                .expect("RecentItemInvoked handler");
        }));
        self.markup_recent.borrow_mut().push((handler, token));
    }
    pub fn remove_recent_item_invoked(&self, handler: Option<MarkupDelegate>) {
        let removed = {
            let mut h = self.markup_recent.borrow_mut();
            h.iter()
                .rposition(|(h, _)| Some(h) == handler.as_ref())
                .map(|i| h.remove(i))
        };
        if let Some((_, token)) = removed {
            self.recent_invoked.remove(token);
        }
    }
}
impl RibbonApplicationMenu {
    fn build_layout(&self) {
        use ferroui_base::input::navigation::{XYFocus, XYFocusNavigationModes};
        use ferroui_base::media::FontWeight;
        let p = &self.parts;
        p.commands.set_spacing(0.);
        p.pane.set_spacing(1.);
        p.pane_header.set_font_weight(FontWeight::SemiBold);
        p.pane_header.set_font_size(13.);
        p.pane_header.set_margin(Thickness::new(10., 8., 10., 6.));
        p.footer.set_orientation(Orientation::Horizontal);
        p.footer.set_spacing(6.);
        p.footer
            .set_horizontal_alignment(HorizontalAlignment::Right);
        p.footer.set_margin(Thickness::new(8., 6., 8., 6.));
        p.search.set_min_height(28.);
        p.search.set_margin(Thickness::new(8., 8., 8., 4.));
        let search_label = localized(RibbonStrings::current().search_commands());
        p.search.set_placeholder_text(search_label.as_deref());
        p.search
            .resources()
            .set("TextControlPlaceholderOpacity", Some(Rc::new(1.0_f64)));
        brush(
            &p.search,
            TextBox::placeholder_foreground_property().as_property(),
            "RibbonSecondaryForegroundBrush",
        );
        AutomationProperties::set_name(&p.search, search_label.as_deref());
        p.search_row.set_child(p.search.clone().upcast::<Control>());
        brush(
            &p.search_row,
            Border::background_property().as_property(),
            "RibbonGroupCaptionBackgroundBrush",
        );
        // Native StackPanel has no padding. A decorator preserves the source's
        // padding inside the ScrollViewer's content, including scroll extent.
        let commands_host = Border::new();
        commands_host.set_padding(Thickness::new(0., 4., 0., 4.));
        commands_host.set_child(p.commands.clone().upcast::<Control>());
        let left = ScrollViewer::new();
        left.set_content(Some(Control::boxed(commands_host)));
        left.set_vertical_scroll_bar_visibility(ScrollBarVisibility::Auto);
        left.set_width(220.);
        XYFocus::set_navigation_modes(
            &p.commands,
            XYFocus::get_navigation_modes(&p.commands) | XYFocusNavigationModes::KEYBOARD,
        );
        XYFocus::set_navigation_modes(
            &p.pane,
            XYFocus::get_navigation_modes(&p.pane) | XYFocusNavigationModes::KEYBOARD,
        );
        let divider = Border::new();
        divider.set_width(1.);
        brush(
            &divider,
            Border::background_property().as_property(),
            "RibbonSeparatorBrush",
        );
        let right = Grid::new();
        right.set_row_definitions(RowDefinitions::parse("Auto,*").expect("fixed rows"));
        right
            .children()
            .add(p.pane_header.clone().upcast::<Control>());
        let pane_scroll = ScrollViewer::new();
        pane_scroll.set_content(Some(Control::boxed(p.pane.clone())));
        pane_scroll.set_vertical_scroll_bar_visibility(ScrollBarVisibility::Auto);
        pane_scroll.set_padding(Thickness::new(4., 0., 4., 4.));
        Grid::set_row(&pane_scroll, 1);
        right.children().add(pane_scroll.upcast::<Control>());
        let body = Grid::new();
        body.set_min_height(340.);
        body.set_column_definitions(
            ColumnDefinitions::parse("Auto,Auto,*").expect("fixed columns"),
        );
        body.children().add(left.upcast::<Control>());
        Grid::set_column(&divider, 1);
        body.children().add(divider.upcast::<Control>());
        Grid::set_column(&right, 2);
        body.children().add(right.upcast::<Control>());
        brush(
            &p.pane_header,
            TextBlock::foreground_property().as_property(),
            "RibbonForegroundBrush",
        );
        let footer_row = Border::new();
        footer_row.set_child(p.footer.clone().upcast::<Control>());
        footer_row.set_border_thickness(Thickness::new(0., 1., 0., 0.));
        brush(
            &footer_row,
            Border::border_brush_property().as_property(),
            "RibbonSeparatorBrush",
        );
        brush(
            &footer_row,
            Border::background_property().as_property(),
            "RibbonGroupCaptionBackgroundBrush",
        );
        let root = Grid::new();
        root.set_width(620.);
        root.set_row_definitions(RowDefinitions::parse("Auto,*,Auto").expect("fixed rows"));
        root.children()
            .add(p.search_row.clone().upcast::<Control>());
        Grid::set_row(&body, 1);
        root.children().add(body.upcast::<Control>());
        Grid::set_row(&footer_row, 2);
        root.children().add(footer_row.upcast::<Control>());
        brush(
            &root,
            Panel::background_property().as_property(),
            "RibbonPopupBackgroundBrush",
        );
        self.set_content(Some(Control::boxed(root)));
        self.set_horizontal_content_alignment(HorizontalAlignment::Stretch);
        self.set_vertical_content_alignment(VerticalAlignment::Stretch);
    }
    fn on_items_changed(&self, e: &NotifyCollectionChangedEventArgs<Option<Ref<Item>>>) {
        for item in e.old_items.iter().flatten() {
            let removed = {
                let mut s = self.item_subscriptions.borrow_mut();
                s.iter()
                    .rposition(|(weak, _)| weak.upgrade().as_ref() == Some(item))
                    .map(|i| s.remove(i))
            };
            if let Some((_, h)) = removed {
                h.dispose();
            }
        }
        for item in e.new_items.iter().flatten() {
            let weak = self.to_ref().downgrade();
            let h = item.changed(move |item| {
                if let Some(menu) = weak.upgrade() {
                    menu.build_commands();
                    if menu
                        .shown_item()
                        .as_ref()
                        .is_some_and(|shown| shown.ptr_eq(&item.to_ref()))
                    {
                        menu.show_sub_items(&item.to_ref());
                    }
                }
            });
            self.item_subscriptions
                .borrow_mut()
                .push((item.downgrade(), h));
        }
        self.build_commands();
    }
    fn build_commands(&self) {
        self.parts.commands.children().clear();
        self.command_buttons.borrow_mut().clear();
        for item in self.items.iter() {
            let item = item.expect("application-menu command item must be non-null");
            if item.has_separator_before() && !self.parts.commands.children().is_empty() {
                let line = Border::new();
                line.set_height(1.);
                line.set_margin(Thickness::new(10., 4., 10., 4.));
                brush(
                    &line,
                    Border::background_property().as_property(),
                    "RibbonSeparatorBrush",
                );
                self.parts.commands.children().add(line.upcast::<Control>());
            }
            let button = self.create_row(
                item.icon(),
                item.label().as_deref(),
                None,
                26.,
                !item.items().is_empty(),
            );
            button.set_height(44.);
            button.set_is_enabled(item.is_enabled());
            AutomationProperties::set_name(&button, Some(item.label().as_deref().unwrap_or("")));
            if let Some(id) = item.id() {
                AutomationProperties::set_automation_id(&button, Some(&format!("AppMenu_{id}")));
            }
            let weak = self.to_ref().downgrade();
            let captured = item.clone();
            button.click(move |_, _| {
                if let Some(menu) = weak.upgrade() {
                    menu.invoke(captured.clone());
                }
            });
            let weak = self.to_ref().downgrade();
            let captured = item.clone();
            button.pointer_entered(move |_, _| {
                if let Some(menu) = weak.upgrade() {
                    menu.on_command_hovered(&captured);
                }
            });
            let weak = self.to_ref().downgrade();
            let captured = item.clone();
            button.got_focus(move |_, _| {
                if let Some(menu) = weak.upgrade() {
                    menu.on_command_hovered(&captured);
                }
            });
            let weak = self.to_ref().downgrade();
            let captured = item.clone();
            button.key_down(move |_, e| {
                if e.key == Key::Right
                    && !captured.items().is_empty()
                    && let Some(menu) = weak.upgrade()
                {
                    menu.show_sub_items(&captured);
                    if let Some(first) = menu.parts.pane.children().snapshot().first() {
                        first.focus_with(NavigationMethod::Directional, KeyModifiers::NONE);
                    }
                    e.set_handled(true);
                }
            });
            {
                let mut buttons = self.command_buttons.borrow_mut();
                if let Some((_, old)) = buttons.iter_mut().find(|(key, _)| key.ptr_eq(&item)) {
                    *old = button.clone();
                } else {
                    buttons.push((item, button.clone()));
                }
            }
            self.parts
                .commands
                .children()
                .add(button.upcast::<Control>());
        }
    }
    fn on_command_hovered(&self, item: &Ref<Item>) {
        if !self.parts.search.text().unwrap_or_default().is_empty() {
            return;
        }
        if !item.items().is_empty() {
            self.show_sub_items(item);
        } else if self.shown_item().is_some() {
            self.show_default_pane();
        }
    }
    fn show_sub_items(&self, item: &Ref<Item>) {
        self.shown_item.replace(Some(item.clone()));
        self.parts
            .pane_header
            .set_text(Some(item.label().as_deref().unwrap_or("")));
        self.parts.pane.children().clear();
        for sub in item.items().iter() {
            let sub = sub.expect("application-menu sub-command must be non-null");
            let button = self.create_row(
                sub.icon().or_else(|| item.icon()),
                sub.label().as_deref(),
                sub.description().as_deref(),
                32.,
                false,
            );
            button.set_is_enabled(sub.is_enabled());
            AutomationProperties::set_name(&button, Some(sub.label().as_deref().unwrap_or("")));
            let weak = self.to_ref().downgrade();
            let captured = sub.clone();
            button.click(move |_, _| {
                if let Some(menu) = weak.upgrade() {
                    menu.invoke(captured.clone());
                }
            });
            let weak = self.to_ref().downgrade();
            let owner = item.clone();
            button.key_down(move |_, e| {
                if e.key == Key::Left
                    && let Some(menu) = weak.upgrade()
                {
                    let button = menu
                        .command_buttons
                        .borrow()
                        .iter()
                        .find(|(key, _)| key.ptr_eq(&owner))
                        .map(|(_, button)| button.clone());
                    if let Some(button) = button {
                        button.focus_with(NavigationMethod::Directional, KeyModifiers::NONE);
                        e.set_handled(true);
                    }
                }
            });
            self.parts.pane.children().add(button.upcast::<Control>());
        }
        let buttons = self.command_buttons.borrow().clone();
        for (key, button) in buttons {
            Self::set_selected(&button, key.ptr_eq(item));
        }
    }
    fn on_recent_items_changed(&self) {
        // Source revisits current entries only. Removed entries keep a weak
        // callback while alive; no retained owner graph or GC emulation.
        for item in self.recent_items.iter() {
            let item = item.expect("application-menu recent item must be non-null");
            let removed = {
                let mut s = self.recent_subscriptions.borrow_mut();
                s.iter()
                    .rposition(|(weak, _)| weak.upgrade().as_ref() == Some(&item))
                    .map(|i| s.remove(i))
            };
            if let Some((_, h)) = removed {
                h.dispose();
            }
            let weak = self.to_ref().downgrade();
            let h = item.changed(move |_| {
                if let Some(menu) = weak.upgrade() {
                    menu.on_recent_changed();
                }
            });
            self.recent_subscriptions
                .borrow_mut()
                .push((item.downgrade(), h));
        }
        self.on_recent_changed();
    }
    fn on_recent_changed(&self) {
        if self.shown_item().is_none() && self.parts.search.text().unwrap_or_default().is_empty() {
            self.show_default_pane();
        }
    }
    fn show_default_pane(&self) {
        self.shown_item.replace(None);
        let header = self
            .recent_header()
            .or_else(|| localized(RibbonStrings::current().recent_documents()));
        self.parts.pane_header.set_text(header.as_deref());
        self.parts.pane.children().clear();
        let mut recents = self.recent_items.to_vec();
        recents.sort_by_key(|item| {
            !item
                .as_ref()
                .expect("application-menu recent item must be non-null")
                .is_pinned()
        });
        for item in recents {
            self.parts
                .pane
                .children()
                .add(self.create_recent_row(&item.unwrap()));
        }
        let buttons = self.command_buttons.borrow().clone();
        for (_, button) in buttons {
            Self::set_selected(&button, false);
        }
    }
    fn create_recent_row(&self, item: &Ref<Recent>) -> Ref<Control> {
        let button = self.create_row(
            item.icon().or_else(|| Some(Rc::new("\u{e8a5}".to_owned()))),
            item.title().as_deref(),
            item.path().as_deref(),
            24.,
            false,
        );
        AutomationProperties::set_name(&button, Some(item.title().as_deref().unwrap_or("")));
        let weak = self.to_ref().downgrade();
        let captured = item.clone();
        button.click(move |_, _| {
            if let Some(menu) = weak.upgrade() {
                menu.invoke_recent(captured.clone());
            }
        });
        let pin = ToggleButton::new();
        pin.set_is_checked(Some(item.is_pinned()));
        let icon = crate::compat::FontIcon::new();
        icon.set_glyph(Some(
            if item.is_pinned() {
                "\u{e841}"
            } else {
                "\u{e718}"
            }
            .into(),
        ));
        icon.set_font_size(12.);
        pin.set_content(Some(Control::boxed(icon)));
        pin.set_width(28.);
        pin.set_height(28.);
        pin.set_padding(Thickness::uniform(0.));
        pin.set_min_width(0.);
        pin.set_min_height(0.);
        pin.set_border_thickness(Thickness::uniform(0.));
        pin.set_background(Some(ferroui_base::media::Brushes::transparent()));
        pin.set_vertical_alignment(VerticalAlignment::Center);
        let pin_text = localized(if item.is_pinned() {
            RibbonStrings::current().unpin_panel()
        } else {
            RibbonStrings::current().pin_panel()
        });
        AutomationProperties::set_name(&pin, pin_text.as_deref());
        ToolTip::set_tip(&pin, pin_text.map(|s| Rc::new(s) as BoxedValue));
        let captured = item.clone();
        let weak = pin.downgrade();
        pin.click(move |_, _| {
            if let Some(pin) = weak.upgrade() {
                captured.set_is_pinned(pin.is_checked() == Some(true));
            }
        });
        let row = Grid::new();
        row.set_column_definitions(
            ColumnDefinitions::parse("*,Auto").expect("fixed recent columns"),
        );
        row.children().add(button.upcast::<Control>());
        Grid::set_column(&pin, 1);
        row.children().add(pin.upcast::<Control>());
        row.upcast()
    }
    fn create_row(
        &self,
        icon: Option<BoxedValue>,
        title: Option<&str>,
        subtitle: Option<&str>,
        icon_size: f64,
        chevron: bool,
    ) -> Ref<Button> {
        let text = StackPanel::new();
        text.set_vertical_alignment(VerticalAlignment::Center);
        text.set_spacing(1.);
        let title_block = TextBlock::new();
        title_block.set_text(Some(title.unwrap_or("")));
        title_block.set_font_size(13.);
        title_block.set_text_trimming(<dyn TextTrimming>::character_ellipsis());
        text.children().add(title_block.upcast::<Control>());
        if let Some(subtitle) = subtitle.filter(|s| !s.trim().is_empty()) {
            let sub = TextBlock::new();
            sub.set_text(Some(subtitle));
            sub.set_font_size(11.);
            sub.set_text_wrapping(TextWrapping::Wrap);
            sub.set_max_lines(2);
            sub.set_text_trimming(<dyn TextTrimming>::character_ellipsis());
            brush(
                &sub,
                TextBlock::foreground_property().as_property(),
                "RibbonSecondaryForegroundBrush",
            );
            text.children().add(sub.upcast::<Control>());
        }
        let grid = Grid::new();
        let columns = ColumnDefinitions::parse("Auto,*,Auto").expect("fixed row columns");
        columns.get(0).set_width(ferroui_controls::GridLength::new(
            icon_size,
            ferroui_controls::GridUnitType::Pixel,
        ));
        grid.set_column_definitions(columns);
        grid.set_column_spacing(10.);
        let presenter = RibbonIconPresenter::new();
        presenter.set_icon(icon);
        presenter.set_icon_size(icon_size);
        presenter.set_vertical_alignment(VerticalAlignment::Center);
        brush(
            &presenter,
            RibbonIconPresenter::foreground_property().as_property(),
            "RibbonIconBrush",
        );
        grid.children().add(presenter.upcast::<Control>());
        Grid::set_column(&text, 1);
        grid.children().add(text.upcast::<Control>());
        if chevron {
            let arrow = crate::compat::FontIcon::new();
            arrow.set_glyph(Some("\u{e76c}".into()));
            arrow.set_font_size(10.);
            arrow.set_vertical_alignment(VerticalAlignment::Center);
            Grid::set_column(&arrow, 2);
            grid.children().add(arrow.upcast::<Control>());
        }
        let button = Button::new();
        button.set_content(Some(Control::boxed(grid)));
        button.set_horizontal_alignment(HorizontalAlignment::Stretch);
        button.set_horizontal_content_alignment(HorizontalAlignment::Stretch);
        button.set_padding(Thickness::new(10., 4., 10., 4.));
        button.set_min_height(36.);
        if let Some(theme) = Application::current()
            .and_then(|app| {
                app.resources()
                    .try_get_resource(&"RibbonMenuItemButtonStyle".into(), None)
                    .flatten()
            })
            .and_then(|value| ferroui_base::data::core::ValueTypes::as_object(&*value))
            .and_then(|object| object.cast::<ferroui_base::styling::ControlTheme>())
        {
            button.set_theme(theme);
        }
        button
    }
    fn set_selected(button: &Button, selected: bool) {
        if selected {
            brush(
                button,
                TemplatedControl::background_property().as_property(),
                "RibbonItemHoverBrush",
            );
        } else {
            button.clear_value(TemplatedControl::background_property());
        }
    }
}
impl RibbonApplicationMenu {
    fn build_footer(&self) {
        self.parts.footer.children().clear();
        for item in self.footer_items.iter() {
            let item = item.expect("application-menu footer element must be non-null");
            if let Some(parent) = item.parent().and_then(|p| p.cast::<Panel>())
                && !parent.ptr_eq(&self.parts.footer)
            {
                parent.children().remove(item.clone());
            }
            if let Some(button) = item.cast::<Button>() {
                let previous = {
                    let mut list = self.footer_callbacks.borrow_mut();
                    list.iter()
                        .rposition(|(weak, _)| weak.upgrade().as_ref() == Some(&button))
                        .map(|i| list.remove(i))
                };
                if let Some((_, token)) = previous {
                    button.remove_handler(Button::click_event(), token);
                }
                let weak = self.to_ref().downgrade();
                let token = button.click(move |_, _| {
                    if let Some(menu) = weak.upgrade() {
                        menu.close_host();
                    }
                });
                self.footer_callbacks
                    .borrow_mut()
                    .push((button.downgrade(), token));
            }
            self.parts.footer.children().add(item);
        }
    }
    fn on_search_text_changed(&self) {
        let query = self.parts.search.text().unwrap_or_default();
        if query.trim().is_empty() {
            self.show_default_pane();
            return;
        }
        self.shown_item.replace(None);
        self.parts.pane.children().clear();
        self.parts
            .pane_header
            .set_text(localized(RibbonStrings::current().search_actions()).as_deref());
        let results = self.search_menu(&query);
        for result in &results {
            let entry = result.entry().expect("engine result entry");
            let target = crate::metadata::model_objects::to_native(entry.target())
                .expect("native search target");
            let icon = target.as_ref().and_then(|target| {
                use ferroui_base::data::core::{ValueType, ValueTypes};
                if let Some(item) =
                    ValueTypes::try_cast(target, ValueType::of::<Rc<dyn super::IRibbonItem>>())
                        .and_then(|v| v.downcast_ref::<Rc<dyn super::IRibbonItem>>().cloned())
                {
                    return item.icon();
                }
                ValueTypes::as_object(&**target)
                    .and_then(|value| value.cast::<Item>())
                    .and_then(|item| item.icon())
            });
            let label = localized(entry.label());
            let path = localized(entry.path());
            let button = self.create_row(icon, label.as_deref(), path.as_deref(), 20., false);
            button.set_is_enabled(entry.is_enabled());
            AutomationProperties::set_name(&button, label.as_deref());
            let weak = self.to_ref().downgrade();
            button.click(move |_, _| {
                if let Some(menu) = weak.upgrade() {
                    menu.execute_search_result(&entry);
                }
            });
            self.parts.pane.children().add(button.upcast::<Control>());
        }
        if results.is_empty() {
            let none = TextBlock::new();
            none.set_text(localized(RibbonStrings::current().search_no_results()).as_deref());
            none.set_margin(Thickness::new(10., 4., 10., 4.));
            none.set_opacity(0.7);
            self.parts.pane.children().add(none.upcast::<Control>());
        }
    }
    /// Native vector replaces the source IReadOnlyList. Ranking uses the existing
    /// Core engine; stable merge preserves menu-before-ribbon ties and limits.
    pub fn search_menu(&self, query: &str) -> Vec<Rc<RibbonSearchResult>> {
        fn add(entries: &mut Vec<Option<Rc<RibbonSearchEntry>>>, item: &Item, path: Option<&str>) {
            if let Some(label) = item.label().filter(|s| !s.is_empty())
                && item.items().is_empty()
            {
                let id = format!(
                    "appmenu/{}",
                    item.id()
                        .unwrap_or_else(|| format!("{}/{}", path.unwrap_or(""), label))
                );
                entries.push(Some(RibbonSearchEntry::with_options(
                    Some(id.into()),
                    Some(label.into()),
                    RibbonSearchEntryOptions {
                        path: path
                            .map(RibbonString::from)
                            .or_else(|| RibbonStrings::current().file()),
                        description: item.description().map(Into::into),
                        target: crate::metadata::model_objects::from_native(into_markup_value(
                            item.to_ref(),
                        )),
                        is_enabled: item.is_enabled(),
                        ..Default::default()
                    },
                )));
            }
            for sub in item.items().iter() {
                add(
                    entries,
                    &sub.expect("application-menu sub-command must be non-null"),
                    item.label().as_deref(),
                );
            }
        }
        let mut entries = Vec::new();
        for item in self.items.iter() {
            add(
                &mut entries,
                &item.expect("application-menu item must be non-null"),
                None,
            );
        }
        let engine = RibbonSearchEngine::new();
        engine.replace_entries(
            entries
                .into_iter()
                .map(|entry| entry.expect("menu search entries are non-null")),
        );
        let query = RibbonString::from(query);
        let mut results = engine
            .search(Some(&query), 6)
            .iter()
            .map(|r| r.expect("engine result must be non-null"))
            .collect::<Vec<_>>();
        if let Some(ribbon) = self.ribbon() {
            results.extend(
                ribbon
                    .search(Some(&query), 10)
                    .iter()
                    .map(|r| r.expect("ribbon search result must be non-null")),
            );
        }
        results.sort_by(|a, b| match (a.score().is_nan(), b.score().is_nan()) {
            (true, true) => std::cmp::Ordering::Equal,
            (true, false) => std::cmp::Ordering::Greater,
            (false, true) => std::cmp::Ordering::Less,
            (false, false) => b.score().partial_cmp(&a.score()).unwrap(),
        });
        results.truncate(12);
        results
    }
    fn execute_search_result(&self, entry: &Rc<RibbonSearchEntry>) {
        if !entry.is_enabled() {
            return;
        }
        let target = crate::metadata::model_objects::to_native(entry.target())
            .expect("native search target");
        if let Some(item) = target
            .and_then(|target| ferroui_base::data::core::ValueTypes::as_object(&*target))
            .and_then(|target| target.cast::<Item>())
        {
            self.invoke(item);
            return;
        }
        self.close_host();
        if let Some(ribbon) = self.ribbon() {
            ribbon.execute_search_entry(Some(entry.clone()));
        }
    }
    fn on_search_key_down(&self, e: &KeyEventArgs) {
        let has_query = !self.parts.search.text().unwrap_or_default().is_empty();
        match e.key {
            Key::Enter if has_query => {
                if let Some(button) = self
                    .parts
                    .pane
                    .children()
                    .snapshot()
                    .iter()
                    .filter_map(|c| c.cast::<Button>())
                    .find(|button| button.is_enabled())
                {
                    use ferroui_controls::automation::peers::ButtonAutomationPeer;
                    let peer = ButtonAutomationPeer::new(&button);
                    peer.invoke()
                        .expect("enabled application-menu search result");
                }
                e.set_handled(true);
            }
            Key::Down => {
                let children = if has_query {
                    self.parts.pane.children().snapshot()
                } else {
                    self.parts.commands.children().snapshot()
                };
                if let Some(first) = children.first() {
                    first.focus_with(NavigationMethod::Directional, KeyModifiers::NONE);
                }
                e.set_handled(true);
            }
            _ => {}
        }
    }
    fn close_host(&self) {
        use ferroui_controls::{TopLevel, primitives::PopupRoot};
        let Some(mut root) = TopLevel::get_top_level(Some(self)) else {
            return;
        };
        // Follow the actual popup placement ownership to its containing root.
        while let Some(popup) = root
            .cast::<PopupRoot>()
            .and_then(|r| r.parent())
            .and_then(|p| p.cast::<Popup>())
        {
            let Some(parent) = popup
                .placement_target()
                .and_then(|target| TopLevel::get_top_level(Some(&target)))
            else {
                break;
            };
            if parent.ptr_eq(&root) {
                break;
            }
            root = parent;
        }
        fn contains_menu(popup: &Popup, menu: &RibbonApplicationMenu) -> bool {
            popup.is_inside_popup(menu)
                || popup
                    .opened_popups()
                    .iter()
                    .any(|child| contains_menu(child, menu))
        }
        // Native visual roots can have a logical owner inside their own visual
        // tree (managed window decoration). Mixing the two parent walks loops.
        // Use the SDK's open-popup ownership tree and visual containment instead.
        for popup in root.opened_popups() {
            if contains_menu(&popup, self) {
                popup.set_is_open(false);
                return;
            }
        }
    }
}
impl FerroObjectImpl for RibbonApplicationMenu {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        RibbonTheme::ensure_resources();
        KeyboardNavigation::set_is_tab_stop(this, false);
        let weak = this.to_ref().downgrade();
        let items = this.items.add_collection_changed(Rc::new(move |e| {
            if let Some(menu) = weak.upgrade() {
                menu.on_items_changed(e);
            }
        }));
        let weak = this.to_ref().downgrade();
        let recent = this.recent_items.add_collection_changed(Rc::new(move |_| {
            if let Some(menu) = weak.upgrade() {
                menu.on_recent_items_changed();
            }
        }));
        let weak = this.to_ref().downgrade();
        let footer = this.footer_items.add_collection_changed(Rc::new(move |_| {
            if let Some(menu) = weak.upgrade() {
                menu.build_footer();
            }
        }));
        this.collection_tokens
            .replace(Some((items, recent, footer)));
        // Native TextChanging runs synchronously after the value is updated;
        // TextChanged is asynchronous at this SDK revision.
        let weak = this.to_ref().downgrade();
        this.parts.search.text_changing(move |_, _| {
            if let Some(menu) = weak.upgrade() {
                menu.on_search_text_changed();
            }
        });
        let weak = this.to_ref().downgrade();
        this.parts.search.key_down(move |_, e| {
            if let Some(menu) = weak.upgrade() {
                menu.on_search_key_down(e);
            }
        });
        this.build_layout();
        let weak = this.to_ref().downgrade();
        this.loaded(move |_, _| {
            if let Some(menu) = weak.upgrade() {
                menu.parts.search.set_text(Some(""));
                menu.show_default_pane();
                if menu.is_search_visible() {
                    menu.parts.search.focus();
                }
            }
        });
        AutomationProperties::set_name(this, localized(RibbonStrings::current().file()).as_deref());
    }
}
impl Drop for RibbonApplicationMenu {
    fn drop(&mut self) {
        if let Some((items, recent, footer)) = self.collection_tokens.get_mut().take() {
            self.items.remove_collection_changed(items);
            self.recent_items.remove_collection_changed(recent);
            self.footer_items.remove_collection_changed(footer);
        }
        for (_, h) in self.item_subscriptions.get_mut().drain(..) {
            h.dispose();
        }
        for (_, h) in self.recent_subscriptions.get_mut().drain(..) {
            h.dispose();
        }
        for (weak, token) in self.footer_callbacks.get_mut().drain(..) {
            if let Some(button) = weak.upgrade() {
                button.remove_handler(Button::click_event(), token);
            }
        }
    }
}

#[cfg(test)]
#[path = "ribbon_application_menu_tests.rs"]
mod tests;

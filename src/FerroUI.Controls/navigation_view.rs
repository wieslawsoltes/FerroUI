// Copyright (c) Microsoft Corporation. Licensed under the MIT License. See NOTICE.md.
// Adapted from NavigationView.cpp/.idl at microsoft/microsoft-ui-xaml a71c42bd.

use crate::automation::peers::NavigationViewAutomationPeer;
use super::navigation_view_events::*;
use super::{AutoSuggestBox, NavigationViewItem, NavigationViewItemBase, Symbol, SymbolIcon};
use crate::automation::peers::{AutomationPeer, ControlAutomationPeer};
use crate::automation::{AutomationProperties, SelectionPatternIdentifiers};
use crate::items_source::{reference_equals, ItemsSource};
use crate::presenters::ContentPresenter;
use crate::primitives::{
    TemplateAppliedEventArgs, TemplatedControl, TemplatedControlImpl, TemplatedControlImplExt,
};
use crate::templates::{
    FuncControlTemplate, FuncTemplateNameScopeExtensions, IControlTemplate, IDataTemplate,
};
use crate::{
    Button, ContentControl, ContentControlImpl, Control, ControlImpl, Grid, ItemCollection,
    ItemsSourceView, Panel, ScrollViewer, SplitView, SplitViewDisplayMode, StackPanel, TextBlock,
};
use ferroui_base::controls::ResourceKey;
use ferroui_base::data::{BindingPriority, TemplateBinding};
use ferroui_base::input::{
    InputElement, InputElementImpl, InputElementImplExt, Key, KeyEventArgs, KeyModifiers,
    NavigationMethod,
};
use ferroui_base::interactivity::{InteractiveImpl, RoutedEventHandlerToken};
use ferroui_base::layout::{
    HorizontalAlignment, LayoutableImpl, LayoutableImplExt, Orientation, VerticalAlignment,
};
use ferroui_base::media::{Brushes, FlowDirection, IBrush};
use ferroui_base::metadata::{into_markup_value, MarkupDelegate};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::styling::ControlTheme;
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, BoxedValue,
    DirectProperty, FerroObject, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, Ref, Size, StyledElement, StyledElementImpl, StyledProperty,
    StyledPropertyOptions, Thickness, VisualImpl, WeakRef,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[derive(Clone)]
struct Entry {
    data: Option<BoxedValue>,
    container: Ref<NavigationViewItemBase>,
    ancestors: Vec<Ref<NavigationViewItemBase>>,
    footer: bool,
}

struct AppliedTheme {
    target: WeakRef<TemplatedControl>,
    theme: Ref<ControlTheme>,
    value: Rc<dyn IDisposable>,
}

#[derive(Clone, Default)]
struct Parts {
    split: Option<Ref<SplitView>>,
    pane: Option<Ref<Grid>>,
    top: Option<Ref<ContentPresenter>>,
    menu: Option<Ref<StackPanel>>,
    footer: Option<Ref<StackPanel>>,
    menu_scroll: Option<Ref<ScrollViewer>>,
    pane_header: Option<Ref<ContentPresenter>>,
    pane_custom: Option<Ref<ContentPresenter>>,
    pane_footer: Option<Ref<ContentPresenter>>,
    header: Option<Ref<ContentPresenter>>,
    content: Option<Ref<ContentPresenter>>,
    search: Option<Ref<ContentPresenter>>,
    overlay: Option<Ref<ContentPresenter>>,
    title: Option<Ref<TextBlock>>,
    toggle: Option<Ref<Button>>,
    back: Option<Ref<Button>>,
    buttons: Vec<(WeakRef<Button>, RoutedEventHandlerToken)>,
    subscriptions: Vec<Rc<dyn IDisposable>>,
}
impl Parts {
    fn detach(&mut self) {
        for (button, token) in self.buttons.drain(..) {
            if let Some(button) = button.upgrade() {
                button.remove_handler(Button::click_event(), token);
            }
        }
        for subscription in self.subscriptions.drain(..) {
            subscription.dispose();
        }
        // Release application content before a replacement template adopts it.
        if let Some(split) = &self.split {
            split.set_pane(None);
            split.set_content(None);
        }
        if let Some(top) = &self.top {
            top.set_content_template(None);
            top.set_content(None);
        }
        if let Some(menu) = &self.menu {
            menu.children().clear();
        }
        if let Some(footer) = &self.footer {
            footer.children().clear();
        }
        for presenter in [
            &self.pane_header,
            &self.pane_custom,
            &self.pane_footer,
            &self.header,
            &self.content,
            &self.search,
            &self.overlay,
        ]
        .into_iter()
        .flatten()
        {
            // A custom data template may create a child for null content.
            // Remove that factory before releasing the old presenter's content.
            presenter.set_content_template(None);
            presenter.set_content(None);
        }
    }
}

/// Native navigation containers, responsive panes and source-backed selection.
///
/// The source model and each generated container retain their separate identities.
/// Top navigation uses a native horizontal panel; the WinUI overflow algorithm,
/// transition animations and OS title-bar padding are not implemented by this type.
/// Native ControlTheme/IDataTemplate values are the audited equivalents of styles
/// and templates; WinUI StyleSelector/DataTemplateSelector are separate contracts.
#[repr(C)]
pub struct NavigationView {
    base: ContentControl,
    menu_items: ItemCollection,
    footer_items: ItemCollection,
    menu_view: RefCell<Option<(Rc<ItemsSourceView>, u64)>>,
    footer_view: RefCell<Option<(Rc<ItemsSourceView>, u64)>>,
    roots: RefCell<Vec<Entry>>,
    known_entries: RefCell<Vec<Entry>>,
    item_subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
    applied_themes: RefCell<Vec<AppliedTheme>>,
    settings_item: Ref<NavigationViewItem>,
    display_mode: Cell<NavigationViewDisplayMode>,
    applied_template: RefCell<Option<Rc<dyn IControlTemplate>>>,
    parts: RefCell<Parts>,
    subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
    rebuilding: Cell<bool>,
    rebuild_pending: Cell<bool>,
    rebuild_force: Cell<bool>,
    syncing_selection: Cell<bool>,
    pending_invocation: RefCell<Option<Ref<NavigationViewItemBase>>>,
    pending_selection: Cell<bool>,
    pending_selection_revision: Cell<u64>,
    pane_internal: Cell<bool>,
    pane_event_raised: Cell<bool>,
    was_force_closed: Cell<bool>,
    initial_nonforced_layout: Cell<bool>,
    updating_parts: Cell<bool>,
    update_pending: Cell<bool>,
    last_width: Cell<f64>,
    selection_changed: HandlerList<dyn Fn(&NavigationViewSelectionChangedEventArgs)>,
    item_invoked: HandlerList<dyn Fn(&NavigationViewItemInvokedEventArgs)>,
    display_mode_changed: HandlerList<dyn Fn(&NavigationViewDisplayModeChangedEventArgs)>,
    back_requested: HandlerList<dyn Fn(&NavigationViewBackRequestedEventArgs)>,
    pane_closing: HandlerList<dyn Fn(&NavigationViewPaneClosingEventArgs)>,
    expanding: HandlerList<dyn Fn(&NavigationViewItemExpandingEventArgs)>,
    collapsed: HandlerList<dyn Fn(&NavigationViewItemCollapsedEventArgs)>,
    pane_opening: HandlerList<dyn Fn()>,
    pane_opened: HandlerList<dyn Fn()>,
    pane_closed: HandlerList<dyn Fn()>,
}
ferro_class!(NavigationView: ContentControl);
ferro_class_info!(NavigationView {
    new: NavigationView::new,
    markup: { namespace: "FerroUI.Controls", content: Content,
        events: [
            SelectionChanged(Option<BoxedValue>, NavigationViewSelectionChangedEventArgs) => |this: &Ref<NavigationView>, handler: MarkupDelegate| {
                let weak = this.downgrade();
                this.selection_changed(move |args| {
                    if let Some(sender) = weak.upgrade() { handler.invoke(&[into_markup_value(Some(sender)), into_markup_value(args.clone())]); }
                })
            },
            ItemInvoked(Option<BoxedValue>, NavigationViewItemInvokedEventArgs) => |this: &Ref<NavigationView>, handler: MarkupDelegate| {
                let weak = this.downgrade();
                this.item_invoked(move |args| {
                    if let Some(sender) = weak.upgrade() { handler.invoke(&[into_markup_value(Some(sender)), into_markup_value(args.clone())]); }
                })
            },
            DisplayModeChanged(Option<BoxedValue>, NavigationViewDisplayModeChangedEventArgs) => |this: &Ref<NavigationView>, handler: MarkupDelegate| {
                let weak = this.downgrade();
                this.display_mode_changed(move |args| {
                    if let Some(sender) = weak.upgrade() { handler.invoke(&[into_markup_value(Some(sender)), into_markup_value(args.clone())]); }
                })
            },
            BackRequested(Option<BoxedValue>, NavigationViewBackRequestedEventArgs) => |this: &Ref<NavigationView>, handler: MarkupDelegate| {
                let weak = this.downgrade();
                this.back_requested(move |args| {
                    if let Some(sender) = weak.upgrade() { handler.invoke(&[into_markup_value(Some(sender)), into_markup_value(args.clone())]); }
                })
            },
            PaneClosing(Option<BoxedValue>, NavigationViewPaneClosingEventArgs) => |this: &Ref<NavigationView>, handler: MarkupDelegate| {
                let weak = this.downgrade();
                this.pane_closing(move |args| {
                    if let Some(sender) = weak.upgrade() { handler.invoke(&[into_markup_value(Some(sender)), into_markup_value(args.clone())]); }
                })
            },
            Expanding(Option<BoxedValue>, NavigationViewItemExpandingEventArgs) => |this: &Ref<NavigationView>, handler: MarkupDelegate| {
                let weak = this.downgrade();
                this.expanding(move |args| {
                    if let Some(sender) = weak.upgrade() { handler.invoke(&[into_markup_value(Some(sender)), into_markup_value(args.clone())]); }
                })
            },
            Collapsed(Option<BoxedValue>, NavigationViewItemCollapsedEventArgs) => |this: &Ref<NavigationView>, handler: MarkupDelegate| {
                let weak = this.downgrade();
                this.collapsed(move |args| {
                    if let Some(sender) = weak.upgrade() { handler.invoke(&[into_markup_value(Some(sender)), into_markup_value(args.clone())]); }
                })
            },
            PaneOpening(Option<BoxedValue>, Option<BoxedValue>) => |this: &Ref<NavigationView>, handler: MarkupDelegate| {
                let weak = this.downgrade();
                this.pane_opening(move || {
                    if let Some(sender) = weak.upgrade() { handler.invoke(&[into_markup_value(Some(sender)), into_markup_value(None::<BoxedValue>)]); }
                })
            },
            PaneOpened(Option<BoxedValue>, Option<BoxedValue>) => |this: &Ref<NavigationView>, handler: MarkupDelegate| {
                let weak = this.downgrade();
                this.pane_opened(move || {
                    if let Some(sender) = weak.upgrade() { handler.invoke(&[into_markup_value(Some(sender)), into_markup_value(None::<BoxedValue>)]); }
                })
            },
            PaneClosed(Option<BoxedValue>, Option<BoxedValue>) => |this: &Ref<NavigationView>, handler: MarkupDelegate| {
                let weak = this.downgrade();
                this.pane_closed(move || {
                    if let Some(sender) = weak.upgrade() { handler.invoke(&[into_markup_value(Some(sender)), into_markup_value(None::<BoxedValue>)]); }
                })
            },
        ],
    },
});
ferro_impl_classes!(NavigationView: StyledElementImpl, VisualImpl, InteractiveImpl, ContentControlImpl);

ferro_properties! { impl NavigationView {
    pub fn is_pane_open_property() -> StyledProperty<bool> { FerroProperty::register::<NavigationView, _>("IsPaneOpen", true) }
    pub fn compact_mode_threshold_width_property() -> StyledProperty<f64> { FerroProperty::register_with::<NavigationView, _>("CompactModeThresholdWidth", StyledPropertyOptions::new(641.0).coerce(|_, v| if v < 0.0 { 0.0 } else { v })) }
    pub fn expanded_mode_threshold_width_property() -> StyledProperty<f64> { FerroProperty::register_with::<NavigationView, _>("ExpandedModeThresholdWidth", StyledPropertyOptions::new(1008.0).coerce(|_, v| if v < 0.0 { 0.0 } else { v })) }
    pub fn is_settings_visible_property() -> StyledProperty<bool> { FerroProperty::register::<NavigationView, _>("IsSettingsVisible", true) }
    pub fn is_pane_toggle_button_visible_property() -> StyledProperty<bool> { FerroProperty::register::<NavigationView, _>("IsPaneToggleButtonVisible", true) }
    pub fn always_show_header_property() -> StyledProperty<bool> { FerroProperty::register::<NavigationView, _>("AlwaysShowHeader", true) }
    pub fn compact_pane_length_property() -> StyledProperty<f64> { FerroProperty::register_with::<NavigationView, _>("CompactPaneLength", StyledPropertyOptions::new(48.0).coerce(|_, v| if v < 0.0 { 0.0 } else { v })) }
    pub fn open_pane_length_property() -> StyledProperty<f64> { FerroProperty::register_with::<NavigationView, _>("OpenPaneLength", StyledPropertyOptions::new(320.0).coerce(|_, v| if v < 0.0 { 0.0 } else { v })) }
    pub fn is_pane_visible_property() -> StyledProperty<bool> { FerroProperty::register::<NavigationView, _>("IsPaneVisible", true) }
    pub fn is_back_enabled_property() -> StyledProperty<bool> { FerroProperty::register::<NavigationView, _>("IsBackEnabled", false) }
    pub fn is_back_button_visible_property() -> StyledProperty<NavigationViewBackButtonVisible> { FerroProperty::register::<NavigationView, _>("IsBackButtonVisible", NavigationViewBackButtonVisible::Auto) }
    pub fn pane_display_mode_property() -> StyledProperty<NavigationViewPaneDisplayMode> { FerroProperty::register::<NavigationView, _>("PaneDisplayMode", NavigationViewPaneDisplayMode::Auto) }
    pub fn selection_follows_focus_property() -> StyledProperty<NavigationViewSelectionFollowsFocus> { FerroProperty::register::<NavigationView, _>("SelectionFollowsFocus", NavigationViewSelectionFollowsFocus::Disabled) }
    pub fn pane_title_property() -> StyledProperty<String> { FerroProperty::register::<NavigationView, _>("PaneTitle", String::new()) }
    pub fn header_property() -> StyledProperty<Option<BoxedValue>> { FerroProperty::register::<NavigationView, _>("Header", None) }
    pub fn header_template_property() -> StyledProperty<Option<Rc<dyn IDataTemplate>>> { FerroProperty::register::<NavigationView, _>("HeaderTemplate", None) }
    pub fn selected_item_property() -> StyledProperty<Option<BoxedValue>> { FerroProperty::register::<NavigationView, _>("SelectedItem", None) }
    pub fn pane_header_property() -> StyledProperty<Option<Ref<Control>>> { FerroProperty::register::<NavigationView, _>("PaneHeader", None) }
    pub fn pane_footer_property() -> StyledProperty<Option<Ref<Control>>> { FerroProperty::register::<NavigationView, _>("PaneFooter", None) }
    pub fn pane_custom_content_property() -> StyledProperty<Option<Ref<Control>>> { FerroProperty::register::<NavigationView, _>("PaneCustomContent", None) }
    pub fn content_overlay_property() -> StyledProperty<Option<Ref<Control>>> { FerroProperty::register::<NavigationView, _>("ContentOverlay", None) }
    pub fn auto_suggest_box_property() -> StyledProperty<Option<Ref<AutoSuggestBox>>> { FerroProperty::register::<NavigationView, _>("AutoSuggestBox", None) }
    pub fn menu_items_source_property() -> StyledProperty<Option<ItemsSource>> { FerroProperty::register::<NavigationView, _>("MenuItemsSource", None) }
    pub fn footer_menu_items_source_property() -> StyledProperty<Option<ItemsSource>> { FerroProperty::register::<NavigationView, _>("FooterMenuItemsSource", None) }
    pub fn menu_item_template_property() -> StyledProperty<Option<Rc<dyn IDataTemplate>>> { FerroProperty::register::<NavigationView, _>("MenuItemTemplate", None) }
    pub fn pane_toggle_button_style_property() -> StyledProperty<Option<Ref<ControlTheme>>> { FerroProperty::register::<NavigationView, _>("PaneToggleButtonStyle", None) }
    pub fn menu_item_container_style_property() -> StyledProperty<Option<Ref<ControlTheme>>> { FerroProperty::register::<NavigationView, _>("MenuItemContainerStyle", None) }
    pub fn menu_items_property() -> DirectProperty<NavigationView, ItemCollection> {
        FerroProperty::register_direct::<NavigationView, _>("MenuItems", |o| o.menu_items(), None, ItemCollection::new())
    }
    pub fn footer_menu_items_property() -> DirectProperty<NavigationView, ItemCollection> {
        FerroProperty::register_direct::<NavigationView, _>("FooterMenuItems", |o| o.footer_menu_items(), None, ItemCollection::new())
    }
    pub fn settings_item_property() -> DirectProperty<NavigationView, Option<BoxedValue>> {
        FerroProperty::register_direct::<NavigationView, _>("SettingsItem", |o| Some(Control::boxed(o.settings_item())), None, None)
    }
    pub fn display_mode_property() -> DirectProperty<NavigationView, NavigationViewDisplayMode> {
        FerroProperty::register_direct::<NavigationView, _>("DisplayMode", |o| o.display_mode(), None, NavigationViewDisplayMode::Minimal)
    }
} }

impl NavigationView {
    pub fn construct() -> Self {
        let settings_item = NavigationViewItem::new();
        settings_item.set_content(Some(Rc::new("Settings".to_owned())));
        settings_item.set_icon(Some(SymbolIcon::with_symbol(Symbol::Setting).upcast()));
        Self {
            base: ContentControl::construct(),
            menu_items: ItemCollection::new(),
            footer_items: ItemCollection::new(),
            menu_view: RefCell::new(None),
            footer_view: RefCell::new(None),
            roots: RefCell::new(Vec::new()),
            known_entries: RefCell::new(Vec::new()),
            item_subscriptions: RefCell::new(Vec::new()),
            applied_themes: RefCell::new(Vec::new()),
            settings_item,
            display_mode: Cell::new(NavigationViewDisplayMode::Minimal),
            applied_template: RefCell::new(None),
            parts: RefCell::new(Parts::default()),
            subscriptions: RefCell::new(Vec::new()),
            rebuilding: Cell::new(false),
            rebuild_pending: Cell::new(false),
            rebuild_force: Cell::new(false),
            syncing_selection: Cell::new(false),
            pending_invocation: RefCell::new(None),
            pending_selection: Cell::new(false),
            pending_selection_revision: Cell::new(0),
            pane_internal: Cell::new(false),
            pane_event_raised: Cell::new(false),
            was_force_closed: Cell::new(false),
            initial_nonforced_layout: Cell::new(true),
            updating_parts: Cell::new(false),
            update_pending: Cell::new(false),
            last_width: Cell::new(0.0),
            selection_changed: HandlerList::new(),
            item_invoked: HandlerList::new(),
            display_mode_changed: HandlerList::new(),
            back_requested: HandlerList::new(),
            pane_closing: HandlerList::new(),
            expanding: HandlerList::new(),
            collapsed: HandlerList::new(),
            pane_opening: HandlerList::new(),
            pane_opened: HandlerList::new(),
            pane_closed: HandlerList::new(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn menu_items(&self) -> ItemCollection {
        self.menu_items.clone()
    }
    pub fn footer_menu_items(&self) -> ItemCollection {
        self.footer_items.clone()
    }
    pub fn settings_item(&self) -> Ref<NavigationViewItem> {
        self.settings_item.clone()
    }
    pub fn display_mode(&self) -> NavigationViewDisplayMode {
        self.display_mode.get()
    }
    pub fn is_pane_open(&self) -> bool {
        self.get_value(Self::is_pane_open_property())
    }
    pub fn set_is_pane_open(&self, value: bool) {
        self.set_value(Self::is_pane_open_property(), value);
    }
    pub fn compact_mode_threshold_width(&self) -> f64 {
        self.get_value(Self::compact_mode_threshold_width_property())
    }
    pub fn set_compact_mode_threshold_width(&self, value: f64) {
        self.set_value(Self::compact_mode_threshold_width_property(), value);
    }
    pub fn expanded_mode_threshold_width(&self) -> f64 {
        self.get_value(Self::expanded_mode_threshold_width_property())
    }
    pub fn set_expanded_mode_threshold_width(&self, value: f64) {
        self.set_value(Self::expanded_mode_threshold_width_property(), value);
    }
    pub fn is_settings_visible(&self) -> bool {
        self.get_value(Self::is_settings_visible_property())
    }
    pub fn set_is_settings_visible(&self, value: bool) {
        self.set_value(Self::is_settings_visible_property(), value);
    }
    pub fn is_pane_toggle_button_visible(&self) -> bool {
        self.get_value(Self::is_pane_toggle_button_visible_property())
    }
    pub fn set_is_pane_toggle_button_visible(&self, value: bool) {
        self.set_value(Self::is_pane_toggle_button_visible_property(), value);
    }
    pub fn always_show_header(&self) -> bool {
        self.get_value(Self::always_show_header_property())
    }
    pub fn set_always_show_header(&self, value: bool) {
        self.set_value(Self::always_show_header_property(), value);
    }
    pub fn compact_pane_length(&self) -> f64 {
        self.get_value(Self::compact_pane_length_property())
    }
    pub fn set_compact_pane_length(&self, value: f64) {
        self.set_value(Self::compact_pane_length_property(), value);
    }
    pub fn open_pane_length(&self) -> f64 {
        self.get_value(Self::open_pane_length_property())
    }
    pub fn set_open_pane_length(&self, value: f64) {
        self.set_value(Self::open_pane_length_property(), value);
    }
    pub fn is_pane_visible(&self) -> bool {
        self.get_value(Self::is_pane_visible_property())
    }
    pub fn set_is_pane_visible(&self, value: bool) {
        self.set_value(Self::is_pane_visible_property(), value);
    }
    pub fn is_back_enabled(&self) -> bool {
        self.get_value(Self::is_back_enabled_property())
    }
    pub fn set_is_back_enabled(&self, value: bool) {
        self.set_value(Self::is_back_enabled_property(), value);
    }
    pub fn is_back_button_visible(&self) -> NavigationViewBackButtonVisible {
        self.get_value(Self::is_back_button_visible_property())
    }
    pub fn set_is_back_button_visible(&self, value: NavigationViewBackButtonVisible) {
        self.set_value(Self::is_back_button_visible_property(), value);
    }
    pub fn pane_display_mode(&self) -> NavigationViewPaneDisplayMode {
        self.get_value(Self::pane_display_mode_property())
    }
    pub fn set_pane_display_mode(&self, value: NavigationViewPaneDisplayMode) {
        self.set_value(Self::pane_display_mode_property(), value);
    }
    pub fn selection_follows_focus(&self) -> NavigationViewSelectionFollowsFocus {
        self.get_value(Self::selection_follows_focus_property())
    }
    pub fn set_selection_follows_focus(&self, value: NavigationViewSelectionFollowsFocus) {
        self.set_value(Self::selection_follows_focus_property(), value);
    }
    pub fn pane_title(&self) -> String {
        self.get_value(Self::pane_title_property())
    }
    pub fn set_pane_title(&self, value: String) {
        self.set_value(Self::pane_title_property(), value);
    }
    pub fn header(&self) -> Option<BoxedValue> {
        self.get_value(Self::header_property())
    }
    pub fn set_header(&self, value: Option<BoxedValue>) {
        self.set_value(Self::header_property(), value);
    }
    pub fn header_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.get_value(Self::header_template_property())
    }
    pub fn set_header_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        self.set_value(Self::header_template_property(), value);
    }
    pub fn selected_item(&self) -> Option<BoxedValue> {
        self.get_value(Self::selected_item_property())
    }
    pub fn set_selected_item(&self, value: Option<BoxedValue>) {
        self.set_value(Self::selected_item_property(), value);
    }
    pub fn pane_header(&self) -> Option<Ref<Control>> {
        self.get_value(Self::pane_header_property())
    }
    pub fn set_pane_header(&self, value: Option<Ref<Control>>) {
        self.set_value(Self::pane_header_property(), value);
    }
    pub fn pane_footer(&self) -> Option<Ref<Control>> {
        self.get_value(Self::pane_footer_property())
    }
    pub fn set_pane_footer(&self, value: Option<Ref<Control>>) {
        self.set_value(Self::pane_footer_property(), value);
    }
    pub fn pane_custom_content(&self) -> Option<Ref<Control>> {
        self.get_value(Self::pane_custom_content_property())
    }
    pub fn set_pane_custom_content(&self, value: Option<Ref<Control>>) {
        self.set_value(Self::pane_custom_content_property(), value);
    }
    pub fn content_overlay(&self) -> Option<Ref<Control>> {
        self.get_value(Self::content_overlay_property())
    }
    pub fn set_content_overlay(&self, value: Option<Ref<Control>>) {
        self.set_value(Self::content_overlay_property(), value);
    }
    pub fn auto_suggest_box(&self) -> Option<Ref<AutoSuggestBox>> {
        self.get_value(Self::auto_suggest_box_property())
    }
    pub fn set_auto_suggest_box(&self, value: Option<Ref<AutoSuggestBox>>) {
        self.set_value(Self::auto_suggest_box_property(), value);
    }
    pub fn menu_items_source(&self) -> Option<ItemsSource> {
        self.get_value(Self::menu_items_source_property())
    }
    pub fn set_menu_items_source(&self, value: Option<ItemsSource>) {
        self.set_value(Self::menu_items_source_property(), value);
    }
    pub fn footer_menu_items_source(&self) -> Option<ItemsSource> {
        self.get_value(Self::footer_menu_items_source_property())
    }
    pub fn set_footer_menu_items_source(&self, value: Option<ItemsSource>) {
        self.set_value(Self::footer_menu_items_source_property(), value);
    }
    pub fn menu_item_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.get_value(Self::menu_item_template_property())
    }
    pub fn set_menu_item_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        self.set_value(Self::menu_item_template_property(), value);
    }
    pub fn pane_toggle_button_style(&self) -> Option<Ref<ControlTheme>> {
        self.get_value(Self::pane_toggle_button_style_property())
    }
    pub fn set_pane_toggle_button_style(&self, value: Option<Ref<ControlTheme>>) {
        self.set_value(Self::pane_toggle_button_style_property(), value);
    }
    pub fn menu_item_container_style(&self) -> Option<Ref<ControlTheme>> {
        self.get_value(Self::menu_item_container_style_property())
    }
    pub fn set_menu_item_container_style(&self, value: Option<Ref<ControlTheme>>) {
        self.set_value(Self::menu_item_container_style_property(), value);
    }
    pub fn selection_changed(
        &self,
        handler: impl Fn(&NavigationViewSelectionChangedEventArgs) + 'static,
    ) -> Rc<dyn IDisposable> {
        let token = self.selection_changed.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(owner) = weak.upgrade() {
                owner.selection_changed.remove(token);
            }
        })
    }
    pub fn item_invoked(
        &self,
        handler: impl Fn(&NavigationViewItemInvokedEventArgs) + 'static,
    ) -> Rc<dyn IDisposable> {
        let token = self.item_invoked.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(owner) = weak.upgrade() {
                owner.item_invoked.remove(token);
            }
        })
    }
    pub fn display_mode_changed(
        &self,
        handler: impl Fn(&NavigationViewDisplayModeChangedEventArgs) + 'static,
    ) -> Rc<dyn IDisposable> {
        let token = self.display_mode_changed.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(owner) = weak.upgrade() {
                owner.display_mode_changed.remove(token);
            }
        })
    }
    pub fn back_requested(
        &self,
        handler: impl Fn(&NavigationViewBackRequestedEventArgs) + 'static,
    ) -> Rc<dyn IDisposable> {
        let token = self.back_requested.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(owner) = weak.upgrade() {
                owner.back_requested.remove(token);
            }
        })
    }
    pub fn pane_closing(
        &self,
        handler: impl Fn(&NavigationViewPaneClosingEventArgs) + 'static,
    ) -> Rc<dyn IDisposable> {
        let token = self.pane_closing.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(owner) = weak.upgrade() {
                owner.pane_closing.remove(token);
            }
        })
    }
    pub fn expanding(
        &self,
        handler: impl Fn(&NavigationViewItemExpandingEventArgs) + 'static,
    ) -> Rc<dyn IDisposable> {
        let token = self.expanding.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(owner) = weak.upgrade() {
                owner.expanding.remove(token);
            }
        })
    }
    pub fn collapsed(
        &self,
        handler: impl Fn(&NavigationViewItemCollapsedEventArgs) + 'static,
    ) -> Rc<dyn IDisposable> {
        let token = self.collapsed.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(owner) = weak.upgrade() {
                owner.collapsed.remove(token);
            }
        })
    }
    pub fn pane_opening(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        let token = self.pane_opening.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(owner) = weak.upgrade() {
                owner.pane_opening.remove(token);
            }
        })
    }
    pub fn pane_opened(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        let token = self.pane_opened.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(owner) = weak.upgrade() {
                owner.pane_opened.remove(token);
            }
        })
    }
    pub fn pane_closed(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        let token = self.pane_closed.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(owner) = weak.upgrade() {
                owner.pane_closed.remove(token);
            }
        })
    }
}

impl FerroObjectImpl for NavigationView {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.reset_source(false);
        this.reset_source(true);
        let weak = this.to_ref().downgrade();
        let resources = this.resources_changed(move |_| {
            if let Some(owner) = weak.upgrade() {
                owner.update_parts();
            }
        });
        let weak = this.to_ref().downgrade();
        let theme = this.actual_theme_variant_changed(move || {
            if let Some(owner) = weak.upgrade() {
                owner.update_parts();
            }
        });
        this.subscriptions.borrow_mut().extend([resources, theme]);
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if !change.is_effective_value_change() {
            return;
        }
        let property = change.property();
        if property == Self::menu_items_source_property().as_property() {
            this.reset_source(false);
        } else if property == Self::footer_menu_items_source_property().as_property() {
            this.reset_source(true);
        } else if property == Self::menu_item_template_property().as_property() {
            this.rebuild_items(true);
        } else if property == Self::selected_item_property().as_property() {
            let (old, next) = change.get_old_and_new_value::<Option<BoxedValue>>();
            this.selection_property_changed(old, next);
        } else if property == Self::is_pane_open_property().as_property() {
            this.pane_open_changed();
        } else if property == Self::pane_display_mode_property().as_property() {
            this.was_force_closed.set(false);
            let old = change.get_old_value::<NavigationViewPaneDisplayMode>();
            let next = this.pane_display_mode();
            this.update_adaptive_layout(this.last_width.get(), true);
            if next == this.pane_display_mode()
                && old == Some(NavigationViewPaneDisplayMode::LeftMinimal)
                && next == NavigationViewPaneDisplayMode::Left
            {
                this.set_pane_open_internal(true);
            }
            this.update_parts();
        } else if property == Self::compact_mode_threshold_width_property().as_property()
            || property == Self::expanded_mode_threshold_width_property().as_property()
        {
            this.update_adaptive_layout(this.last_width.get(), false);
        } else if property == Self::is_pane_visible_property().as_property() {
            if !this.is_pane_visible() {
                this.set_pane_open_internal(false);
            } else if this.display_mode() == NavigationViewDisplayMode::Expanded {
                this.set_pane_open_internal(true);
            }
            this.update_parts();
        } else {
            // Inherited content and brush changes share the native template.
            this.update_parts();
        }
    }
}

impl ControlImpl for NavigationView {
    fn on_create_automation_peer(this: &Self) -> Ref<AutomationPeer> {
        NavigationViewAutomationPeer::new(this).upcast()
    }
}

impl LayoutableImpl for NavigationView {
    fn apply_template(this: &Self) {
        let template = this.template();
        let applied = this.applied_template.borrow().clone();
        if applied != template && (template.is_some() || this.is_attached_to_logical_tree()) {
            // The native base removes the old tree before OnApplyTemplate and
            // does not call that hook for a null template. Revoke old parts before
            // either operation so retained buttons and SplitView cannot act later.
            this.parts.take().detach();
            Self::parent_apply_template(this);
            *this.applied_template.borrow_mut() = template;
            this.update_parts();
        } else {
            Self::parent_apply_template(this);
        }
    }
    fn measure_override(this: &Self, available_size: Size) -> Size {
        if available_size.width.is_finite()
            && (available_size.width != this.last_width.get()
                || this.initial_nonforced_layout.get())
        {
            this.update_adaptive_layout(available_size.width, false);
        }
        Self::parent_measure_override(this, available_size)
    }
    fn arrange_override(this: &Self, final_size: Size) -> Size {
        if final_size.width != this.last_width.get() || this.initial_nonforced_layout.get() {
            this.update_adaptive_layout(final_size.width, false);
        }
        Self::parent_arrange_override(this, final_size)
    }
}

impl InputElementImpl for NavigationView {
    fn on_key_down(this: &Self, args: &KeyEventArgs) {
        Self::parent_on_key_down(this, args);
        if args.handled() {
            return;
        }
        if args.key == Key::Escape
            && this.is_pane_open()
            && this.display_mode() != NavigationViewDisplayMode::Expanded
        {
            if this.try_close_pane() {
                args.set_handled(true);
            }
        }
    }
}

impl TemplatedControlImpl for NavigationView {
    fn on_apply_template(this: &Self, args: &TemplateAppliedEventArgs) {
        this.parts.replace(Parts::default()).detach();
        Self::parent_on_apply_template(this, args);
        let scope = args.name_scope();
        let mut parts = Parts {
            split: scope.find_as("RootSplitView"),
            pane: scope.find_as("PaneContentGrid"),
            top: scope.find_as("TopPanePresenter"),
            menu: scope.find_as("MenuItemsHost"),
            footer: scope.find_as("FooterMenuItemsHost"),
            pane_header: scope.find_as("PaneHeaderPresenter"),
            menu_scroll: scope.find_as("MenuItemsScrollViewer"),
            pane_custom: scope.find_as("PaneCustomContentPresenter"),
            pane_footer: scope.find_as("PaneFooterPresenter"),
            header: scope.find_as("HeaderPresenter"),
            content: scope.find_as("PART_ContentPresenter"),
            search: scope.find_as("AutoSuggestBoxPresenter"),
            overlay: scope.find_as("ContentOverlayPresenter"),
            title: scope.find_as("PaneTitleTextBlock"),
            toggle: scope.find_as("TogglePaneButton"),
            back: scope.find_as("NavigationViewBackButton"),
            ..Parts::default()
        };
        if let Some(toggle) = &parts.toggle {
            let weak = this.to_ref().downgrade();
            let token = toggle.click(move |_, _| {
                if let Some(owner) = weak.upgrade() {
                    owner.was_force_closed.set(owner.is_pane_open());
                    owner.set_pane_open_internal(!owner.is_pane_open());
                }
            });
            parts.buttons.push((toggle.downgrade(), token));
        }
        if let Some(back) = &parts.back {
            let weak = this.to_ref().downgrade();
            let token = back.click(move |_, _| {
                if let Some(owner) = weak.upgrade().filter(|owner| owner.is_back_enabled()) {
                    for (_, handler) in owner.back_requested.snapshot().iter() {
                        handler(&NavigationViewBackRequestedEventArgs);
                    }
                }
            });
            parts.buttons.push((back.downgrade(), token));
        }
        if let Some(split) = &parts.split {
            let weak = this.to_ref().downgrade();
            let token = split.pane_closing(move |_, args| {
                if let Some(owner) = weak.upgrade().filter(|owner| !owner.updating_parts.get()) {
                    let event = NavigationViewPaneClosingEventArgs::default();
                    for (_, handler) in owner.pane_closing.snapshot().iter() {
                        handler(&event);
                    }
                    args.set_cancel(event.cancel());
                    owner.pane_event_raised.set(!event.cancel());
                }
            });
            let weak_split = split.downgrade();
            parts.subscriptions.push(Disposable::create(move || {
                if let Some(split) = weak_split.upgrade() {
                    split.remove_handler(SplitView::pane_closing_event(), token);
                }
            }));
            let weak = this.to_ref().downgrade();
            let weak_split = split.downgrade();
            parts
                .subscriptions
                .push(split.property_changed(move |change| {
                    if change.property() == SplitView::is_pane_open_property().as_property() {
                        if let (Some(owner), Some(split)) = (weak.upgrade(), weak_split.upgrade()) {
                            if !owner.updating_parts.get() {
                                owner.set_pane_open_internal(split.is_pane_open());
                                owner.pane_event_raised.set(false);
                            }
                        }
                    }
                }));
        }
        *this.parts.borrow_mut() = parts;
        this.rebuild_items(false);
        this.update_adaptive_layout(this.last_width.get(), true);
        this.update_parts();
    }
}

impl NavigationView {
    fn static_constructor() {
        InputElement::focusable_property().override_default_value::<NavigationView>(false);
        TemplatedControl::template_property().override_default_value::<NavigationView>(Some(
            FuncControlTemplate::for_type::<NavigationView>(|_, scope| {
                let root = Grid::new();
                root.set_name(Some("NavigationViewRoot".into()));
                root.set_row_definitions("Auto,Auto,*".parse().unwrap());
                root.bind_binding(
                    Panel::background_property().as_property(),
                    &TemplateBinding::new(TemplatedControl::background_property().as_property()),
                );
                let toolbar = StackPanel::new();
                toolbar.set_orientation(Orientation::Horizontal);
                let back = Button::new();
                back.set_name(Some("NavigationViewBackButton".into()));
                back.set_width(48.0);
                back.set_height(40.0);
                back.set_content(Some(Control::boxed(SymbolIcon::with_symbol(Symbol::Back))));
                AutomationProperties::set_name(&back, Some("Back".into()));
                toolbar
                    .children()
                    .add(back.register_in_name_scope(&**scope));
                let toggle = Button::new();
                toggle.set_name(Some("TogglePaneButton".into()));
                toggle.set_width(48.0);
                toggle.set_height(40.0);
                toggle.set_content(Some(Control::boxed(SymbolIcon::with_symbol(
                    Symbol::GlobalNavigationButton,
                ))));
                toolbar
                    .children()
                    .add(toggle.register_in_name_scope(&**scope));
                let title = TextBlock::new();
                title.set_name(Some("PaneTitleTextBlock".into()));
                title.set_vertical_alignment(VerticalAlignment::Center);
                title.set_margin(Thickness::new(12.0, 0.0, 12.0, 0.0));
                toolbar
                    .children()
                    .add(title.register_in_name_scope(&**scope));
                root.children().add(toolbar);

                let make_presenter = |name: &str| {
                    let presenter = ContentPresenter::new();
                    presenter.set_name(Some(name.into()));
                    presenter.register_in_name_scope(&**scope)
                };
                let pane = Grid::new();
                pane.set_name(Some("PaneContentGrid".into()));
                pane.set_row_definitions("Auto,Auto,Auto,*,Auto,Auto".parse().unwrap());
                for (name, row) in [
                    ("PaneHeaderPresenter", 0),
                    ("AutoSuggestBoxPresenter", 1),
                    ("PaneCustomContentPresenter", 2),
                    ("PaneFooterPresenter", 5),
                ] {
                    let presenter = make_presenter(name);
                    Grid::set_row(&presenter, row);
                    pane.children().add(presenter);
                }
                let menu = StackPanel::new();
                menu.set_name(Some("MenuItemsHost".into()));
                let scroll = ScrollViewer::new();
                scroll.set_name(Some("MenuItemsScrollViewer".into()));
                scroll.set_content(Some(Control::boxed(menu.register_in_name_scope(&**scope))));
                Grid::set_row(&scroll, 3);
                pane.children().add(scroll.register_in_name_scope(&**scope));
                let footer = StackPanel::new();
                footer.set_name(Some("FooterMenuItemsHost".into()));
                Grid::set_row(&footer, 4);
                pane.children().add(footer.register_in_name_scope(&**scope));
                let pane = pane.register_in_name_scope(&**scope);

                let body = Grid::new();
                body.set_row_definitions("Auto,*".parse().unwrap());
                let header = make_presenter("HeaderPresenter");
                header.set_margin(Thickness::new(16.0, 8.0, 16.0, 8.0));
                body.children().add(header);
                let content = make_presenter("PART_ContentPresenter");
                content.bind_binding(
                    ContentPresenter::content_property().as_property(),
                    &TemplateBinding::new(ContentControl::content_property().as_property()),
                );
                content.bind_binding(
                    ContentPresenter::content_template_property().as_property(),
                    &TemplateBinding::new(
                        ContentControl::content_template_property().as_property(),
                    ),
                );
                Grid::set_row(&content, 1);
                body.children().add(content);
                let overlay = make_presenter("ContentOverlayPresenter");
                Grid::set_row_span(&overlay, 2);
                overlay.set_z_index(1);
                body.children().add(overlay);

                let top = make_presenter("TopPanePresenter");
                Grid::set_row(&top, 1);
                root.children().add(top);
                let split = SplitView::new();
                split.set_name(Some("RootSplitView".into()));
                split.set_template(Some(FuncControlTemplate::for_type::<SplitView>(
                    |_, scope| {
                        let grid = Grid::new();
                        grid.set_name(Some("SplitLayoutRoot".into()));
                        grid.set_column_definitions("Auto,*".parse().unwrap());
                        let content = ContentPresenter::new();
                        content.set_name(Some("PART_ContentPresenter".into()));
                        content.bind_binding(
                            ContentPresenter::content_property().as_property(),
                            &TemplateBinding::new(ContentControl::content_property().as_property()),
                        );
                        Grid::set_column(&content, 1);
                        grid.children()
                            .add(content.register_in_name_scope(&**scope));
                        let pane_root = Panel::new();
                        pane_root.set_name(Some("PART_PaneRoot".into()));
                        pane_root.set_horizontal_alignment(HorizontalAlignment::Left);
                        pane_root.set_clip_to_bounds(true);
                        pane_root.set_z_index(100);
                        pane_root.bind_binding(
                            Panel::background_property().as_property(),
                            &TemplateBinding::new(
                                SplitView::pane_background_property().as_property(),
                            ),
                        );
                        let pane_presenter = ContentPresenter::new();
                        pane_presenter.set_name(Some("PART_PanePresenter".into()));
                        pane_presenter.bind_binding(
                            ContentPresenter::content_property().as_property(),
                            &TemplateBinding::new(SplitView::pane_property().as_property()),
                        );
                        pane_root
                            .children()
                            .add(pane_presenter.register_in_name_scope(&**scope));
                        grid.children()
                            .add(pane_root.register_in_name_scope(&**scope));
                        grid.register_in_name_scope(&**scope).upcast()
                    },
                )));
                split.set_content(Some(Control::boxed(body)));
                split.set_pane(Some(Control::boxed(pane)));
                Grid::set_row(&split, 2);
                root.children().add(split.register_in_name_scope(&**scope));
                root.register_in_name_scope(&**scope).upcast()
            }),
        ));
    }

    fn pane_open_changed(&self) {
        let open = self.is_pane_open();
        if open {
            self.pane_event_raised.set(false);
            self.was_force_closed.set(false);
        } else if !self.pane_internal.get() {
            self.was_force_closed.set(true);
        }
        if open {
            for (_, handler) in self.pane_opening.snapshot().iter() {
                handler();
            }
        } else if !self.pane_event_raised.replace(false) {
            // An explicit or adaptive assignment is a forced close. A light
            // dismissal asks before assigning, through try_close_pane/SplitView.
            let args = NavigationViewPaneClosingEventArgs::default();
            for (_, handler) in self.pane_closing.snapshot().iter() {
                handler(&args);
            }
        }
        if open != self.is_pane_open() {
            return;
        }
        self.update_parts();
        let handlers = if open {
            &self.pane_opened
        } else {
            &self.pane_closed
        };
        for (_, handler) in handlers.snapshot().iter() {
            handler();
        }
    }
    fn set_pane_open_internal(&self, value: bool) {
        let old = self.pane_internal.replace(true);
        self.set_current_value(Self::is_pane_open_property(), value);
        self.pane_internal.set(old);
    }
    fn try_close_pane(&self) -> bool {
        let args = NavigationViewPaneClosingEventArgs::default();
        for (_, handler) in self.pane_closing.snapshot().iter() {
            handler(&args);
        }
        if args.cancel() && !self.was_force_closed.get() {
            self.pane_event_raised.set(false);
            return false;
        }
        // A handler can complete the close itself. Do not leave a suppression
        // flag waiting for a property change which will no longer occur.
        if !self.is_pane_open() {
            self.pane_event_raised.set(false);
            return true;
        }
        self.pane_event_raised.set(true);
        self.set_pane_open_internal(false);
        self.pane_event_raised.set(false);
        true
    }
    fn update_adaptive_layout(&self, width: f64, forced: bool) {
        if self.parts.borrow().split.is_none() {
            return;
        }
        self.last_width.set(width);
        let mode = match self.pane_display_mode() {
            NavigationViewPaneDisplayMode::Top | NavigationViewPaneDisplayMode::LeftMinimal => {
                NavigationViewDisplayMode::Minimal
            }
            NavigationViewPaneDisplayMode::Left => NavigationViewDisplayMode::Expanded,
            NavigationViewPaneDisplayMode::LeftCompact => NavigationViewDisplayMode::Compact,
            NavigationViewPaneDisplayMode::Auto => {
                if width >= self.expanded_mode_threshold_width() {
                    NavigationViewDisplayMode::Expanded
                } else if width > 0.0 && width < self.compact_mode_threshold_width() {
                    NavigationViewDisplayMode::Minimal
                } else {
                    NavigationViewDisplayMode::Compact
                }
            }
        };
        let previous = self.display_mode();
        self.set_and_raise_cell(Self::display_mode_property(), &self.display_mode, mode);
        if previous != mode {
            let args = NavigationViewDisplayModeChangedEventArgs { display_mode: mode };
            for (_, handler) in self.display_mode_changed.snapshot().iter() {
                handler(&args);
            }
            if self.display_mode() != mode {
                return;
            }
        }
        if self.pane_display_mode() == NavigationViewPaneDisplayMode::Top {
            if !forced {
                self.initial_nonforced_layout.set(false);
            }
            self.set_pane_open_internal(false);
        } else {
            if !forced
                && self.initial_nonforced_layout.replace(false)
                && mode != NavigationViewDisplayMode::Expanded
            {
                self.set_pane_open_internal(false);
            }
            if mode == NavigationViewDisplayMode::Expanded
                && self.is_pane_visible()
                && !self.was_force_closed.get()
            {
                self.set_pane_open_internal(true);
            } else if mode == NavigationViewDisplayMode::Minimal
                || (previous == NavigationViewDisplayMode::Expanded
                    && mode == NavigationViewDisplayMode::Compact)
            {
                self.set_pane_open_internal(false);
            }
        }
        self.update_parts();
    }

    fn update_parts(&self) {
        if self.updating_parts.replace(true) {
            self.update_pending.set(true);
            return;
        }
        loop {
            self.update_pending.set(false);
            self.update_parts_once();
            if !self.update_pending.get() {
                break;
            }
        }
        self.updating_parts.set(false);
    }
    fn update_parts_once(&self) {
        let parts = self.parts.borrow().clone();
        let top = self.pane_display_mode() == NavigationViewPaneDisplayMode::Top;
        // Both CompactInline (Expanded/Left) and CompactOverlay leave the
        // compact strip visible when closed. Minimal and Top use Overlay.
        let compact = !top
            && self.is_pane_visible()
            && self.display_mode() != NavigationViewDisplayMode::Minimal
            && !self.is_pane_open();
        let pane_visible = self.is_pane_visible();
        if let (Some(split), Some(pane), Some(top_presenter)) =
            (&parts.split, &parts.pane, &parts.top)
        {
            let pane_value = Some(Control::boxed(pane.clone()));
            if top {
                if split.pane().is_some() {
                    split.set_pane(None);
                }
                if !reference_equals(&top_presenter.content(), &pane_value) {
                    top_presenter.set_content(pane_value);
                }
            } else {
                if top_presenter.content().is_some() {
                    top_presenter.set_content(None);
                }
                if !reference_equals(&split.pane(), &pane_value) {
                    split.set_pane(pane_value);
                }
            }
            top_presenter.set_is_visible(top && pane_visible);
            pane.set_is_visible(pane_visible);
            split.set_display_mode(if top || !pane_visible {
                SplitViewDisplayMode::Overlay
            } else {
                match self.display_mode() {
                    NavigationViewDisplayMode::Expanded => SplitViewDisplayMode::CompactInline,
                    NavigationViewDisplayMode::Compact => SplitViewDisplayMode::CompactOverlay,
                    NavigationViewDisplayMode::Minimal => SplitViewDisplayMode::Overlay,
                }
            });
            split.set_compact_pane_length(finite_length(self.compact_pane_length()));
            split.set_open_pane_length(finite_length(self.open_pane_length()));
            split.set_current_value(
                SplitView::is_pane_open_property(),
                self.is_pane_open() && !top && pane_visible,
            );
            split.set_pane_background(
                self.navigation_brush("NavigationViewDefaultPaneBackground")
                    .or_else(|| self.background())
                    .or_else(|| Some(Brushes::transparent())),
            );
            split.apply_template();
            update_split_geometry(split, !top && pane_visible);
            let rows = if top {
                "Auto"
            } else {
                "Auto,Auto,Auto,*,Auto,Auto"
            };
            let columns = if top {
                "Auto,Auto,*,Auto,Auto,Auto"
            } else {
                "*"
            };
            // Replacing definitions on every unrelated property update can invalidate
            // measure indefinitely. Change orientation only when the shape differs.
            if pane.row_definitions().count() != if top { 1 } else { 6 } {
                pane.set_row_definitions(rows.parse().unwrap());
            }
            if pane.column_definitions().count() != if top { 6 } else { 1 } {
                pane.set_column_definitions(columns.parse().unwrap());
            }
            let placement: Vec<(Option<Ref<Control>>, i32, i32)> = vec![
                (parts.pane_header.clone().map(|v| v.upcast()), 0, 0),
                (parts.search.clone().map(|v| v.upcast()), 1, 4),
                (parts.pane_custom.clone().map(|v| v.upcast()), 2, 1),
                (parts.menu_scroll.clone().map(|v| v.upcast()), 3, 2),
                (parts.footer.clone().map(|v| v.upcast()), 4, 3),
                (parts.pane_footer.clone().map(|v| v.upcast()), 5, 5),
            ];
            for (control, row, column) in placement {
                if let Some(control) = control {
                    Grid::set_row(&control, if top { 0 } else { row });
                    Grid::set_column(&control, if top { column } else { 0 });
                }
            }
        }
        let roots = self.roots.borrow().clone();
        for (panel, footer) in [(&parts.menu, false), (&parts.footer, true)] {
            if let Some(panel) = panel {
                panel.set_orientation(if top {
                    Orientation::Horizontal
                } else {
                    Orientation::Vertical
                });
                let mut controls: Vec<Ref<Control>> = roots
                    .iter()
                    .filter(|entry| entry.footer == footer)
                    .map(|entry| entry.container.clone().upcast())
                    .collect();
                if footer {
                    controls.push(self.settings_item().upcast());
                }
                if panel.children().snapshot().as_ref() != &controls {
                    panel.children().clear();
                    for control in controls {
                        panel.children().add(control);
                    }
                }
            }
        }
        self.settings_item
            .set_is_visible(self.is_settings_visible());
        let mut themes = Vec::new();
        for entry in self.entries() {
            entry
                .container
                .set_top_navigation(top && entry.ancestors.is_empty());
            if entry.ancestors.is_empty() {
                entry.container.set_navigation_context(
                    finite_length(self.compact_pane_length()),
                    compact,
                    0,
                );
                entry.container.set_pane_open(self.is_pane_open() && !top);
            }
            if let Some(theme) = self.menu_item_container_style() {
                themes.push((entry.container.clone().upcast::<TemplatedControl>(), theme));
            }
        }
        if let Some(header) = &parts.header {
            header.set_content(self.header());
            header.set_content_template(self.header_template());
            header.set_is_visible(
                self.header().is_some()
                    && (self.always_show_header()
                        || !top && self.display_mode() == NavigationViewDisplayMode::Minimal),
            );
        }
        if let Some(content) = &parts.content {
            content.set_content(self.content());
            content.set_content_template(self.content_template());
        }
        for (presenter, content) in [
            (&parts.pane_header, self.pane_header()),
            (&parts.pane_custom, self.pane_custom_content()),
            (&parts.pane_footer, self.pane_footer()),
            (&parts.overlay, self.content_overlay()),
        ] {
            if let Some(presenter) = presenter {
                presenter.set_content(content.map(Control::boxed));
            }
        }
        if let Some(search) = &parts.search {
            search.set_content(self.auto_suggest_box().map(Control::boxed));
            search.set_is_visible(self.auto_suggest_box().is_some() && !compact);
        }
        if let Some(title) = &parts.title {
            title.set_text(Some(&self.pane_title()));
            title.set_is_visible(!top && self.is_pane_open() && !self.pane_title().is_empty());
        }
        if let Some(toggle) = &parts.toggle {
            toggle.set_is_visible(self.is_pane_toggle_button_visible() && pane_visible && !top);
            if let Some(theme) = self.pane_toggle_button_style() {
                themes.push((toggle.clone().upcast::<TemplatedControl>(), theme));
            }
            AutomationProperties::set_name(
                toggle,
                Some(
                    if self.is_pane_open() {
                        "Close navigation"
                    } else {
                        "Open navigation"
                    }
                    .into(),
                ),
            );
        }
        if let Some(back) = &parts.back {
            back.set_is_enabled(self.is_back_enabled());
            back.set_is_visible(
                self.is_back_button_visible() != NavigationViewBackButtonVisible::Collapsed
                    && !(self.display_mode() == NavigationViewDisplayMode::Minimal
                        && self.is_pane_open()),
            );
        }
        self.apply_container_themes(themes);
        self.pseudo_classes().set(":top", top);
        self.pseudo_classes().set(":pane-open", self.is_pane_open());
        self.pseudo_classes().set(
            ":compact",
            self.display_mode() == NavigationViewDisplayMode::Compact,
        );
        self.pseudo_classes().set(
            ":expanded",
            self.display_mode() == NavigationViewDisplayMode::Expanded,
        );
        self.pseudo_classes().set(
            ":minimal",
            self.display_mode() == NavigationViewDisplayMode::Minimal,
        );
    }
    fn navigation_brush(&self, key: &str) -> Option<Rc<dyn IBrush>> {
        use ferroui_base::data::core::{ValueType, ValueTypes};
        let value = self
            .try_find_resource(
                &ResourceKey::from(key),
                self.actual_theme_variant().as_ref(),
            )
            .flatten()?;
        let cast = ValueTypes::try_cast(&value, ValueType::of::<Rc<dyn IBrush>>())?;
        cast.downcast_ref::<Rc<dyn IBrush>>().cloned()
    }

    fn apply_container_themes(&self, desired: Vec<(Ref<TemplatedControl>, Ref<ControlTheme>)>) {
        let mut previous = self.applied_themes.replace(Vec::new());
        let mut applied = Vec::new();
        for (target, theme) in desired {
            if let Some(index) = previous.iter().position(|entry| {
                entry
                    .target
                    .upgrade()
                    .is_some_and(|old| old.ptr_eq(&target))
                    && entry.theme == theme
            }) {
                applied.push(previous.remove(index));
            } else if let Some(value) = target.set_value_with_priority(
                StyledElement::theme_property(),
                Some(theme.clone()),
                BindingPriority::Style,
            ) {
                applied.push(AppliedTheme {
                    target: target.downgrade(),
                    theme,
                    value,
                });
            }
        }
        for obsolete in previous {
            obsolete.value.dispose();
        }
        *self.applied_themes.borrow_mut() = applied;
    }
}

fn finite_length(value: f64) -> f64 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

fn update_split_geometry(split: &SplitView, visible: bool) {
    let descendants = split.get_template_descendants();
    let pane = descendants
        .iter()
        .find(|control| control.name().as_deref() == Some("PART_PaneRoot"))
        .and_then(|control| control.cast::<Panel>());
    let content = descendants
        .iter()
        .find(|control| control.name().as_deref() == Some("PART_ContentPresenter"))
        .and_then(|control| control.cast::<ContentPresenter>());
    let inline = matches!(
        split.display_mode(),
        SplitViewDisplayMode::Inline | SplitViewDisplayMode::CompactInline
    );
    let compact = matches!(
        split.display_mode(),
        SplitViewDisplayMode::CompactOverlay | SplitViewDisplayMode::CompactInline
    );
    if let Some(pane) = pane {
        pane.set_width(if !visible {
            0.0
        } else if split.is_pane_open() {
            split.open_pane_length()
        } else if compact {
            split.compact_pane_length()
        } else {
            0.0
        });
        pane.set_is_visible(visible && (split.is_pane_open() || compact));
        Grid::set_column_span(&pane, if inline { 1 } else { 2 });
    }
    if let Some(content) = content {
        Grid::set_column(&content, if inline { 1 } else { 0 });
        Grid::set_column_span(&content, if inline { 1 } else { 2 });
        content.set_margin(Thickness::new(
            if visible && !inline && compact {
                split.compact_pane_length()
            } else {
                0.0
            },
            0.0,
            0.0,
            0.0,
        ));
    }
}

impl Drop for NavigationView {
    fn drop(&mut self) {
        for slot in [&mut self.menu_view, &mut self.footer_view] {
            if let Some((view, token)) = slot.get_mut().take() {
                view.remove_collection_changed(token);
            }
        }
        for subscription in self.item_subscriptions.get_mut().drain(..) {
            subscription.dispose();
        }
        for subscription in self.subscriptions.get_mut().drain(..) {
            subscription.dispose();
        }
        for theme in self.applied_themes.get_mut().drain(..) {
            theme.value.dispose();
        }
        self.parts.get_mut().detach();
    }
}

impl NavigationView {
    fn reset_source(&self, footer: bool) {
        let slot = if footer {
            &self.footer_view
        } else {
            &self.menu_view
        };
        if let Some((view, token)) = slot.borrow_mut().take() {
            view.remove_collection_changed(token);
        }
        let source = if footer {
            self.footer_menu_items_source()
        } else {
            self.menu_items_source()
        };
        let view = if let Some(source) = source {
            ItemsSourceView::get_or_create(Some(&source))
        } else if footer {
            self.footer_items.view().clone()
        } else {
            self.menu_items.view().clone()
        };
        let weak = self.to_ref().downgrade();
        let token = view.add_collection_changed(Rc::new(move |_| {
            if let Some(owner) = weak.upgrade() {
                owner.rebuild_items(false);
            }
        }));
        *slot.borrow_mut() = Some((view, token));
        self.rebuild_items(false);
    }

    fn build_container(&self, data: &Option<BoxedValue>) -> Ref<NavigationViewItemBase> {
        if let Some(container) = data
            .as_ref()
            .and_then(Control::from_boxed)
            .and_then(|control| control.cast::<NavigationViewItemBase>())
        {
            return container;
        }
        let built = self
            .menu_item_template()
            .and_then(|template| template.build(data));
        if let Some(container) = built
            .as_ref()
            .and_then(|control| control.cast::<NavigationViewItemBase>())
        {
            if data.as_ref().and_then(Control::from_boxed).is_none() {
                container.set_data_context(data.clone());
            }
            return container;
        }
        let container = NavigationViewItem::new();
        container.set_content(built.map(Control::boxed).or_else(|| data.clone()));
        if data.as_ref().and_then(Control::from_boxed).is_none() {
            container.set_data_context(data.clone());
        }
        container.upcast()
    }

    fn rebuild_items(&self, force: bool) {
        if force {
            self.rebuild_force.set(true);
        }
        if self.rebuilding.replace(true) {
            self.rebuild_pending.set(true);
            return;
        }
        loop {
            self.rebuild_pending.set(false);
            let force = self.rebuild_force.replace(false);
            for subscription in self.item_subscriptions.replace(Vec::new()) {
                subscription.dispose();
            }
            let previous_known = self.known_entries.borrow().clone();
            let previous = self.roots.borrow().clone();
            let mut available = previous.clone();
            let mut roots = Vec::new();
            for footer in [false, true] {
                let slot = if footer {
                    &self.footer_view
                } else {
                    &self.menu_view
                };
                let view = slot.borrow().as_ref().map(|(view, _)| view.clone());
                let data = view.map(|view| view.to_vec()).unwrap_or_default();
                for item in data {
                    let reuse = if force {
                        None
                    } else {
                        available.iter().position(|old| {
                            old.footer == footer && reference_equals(&old.data, &item)
                        })
                    };
                    let container = if let Some(index) = reuse {
                        available.remove(index).container
                    } else {
                        self.build_container(&item)
                    };
                    if let Some(nvi) = container.cast::<NavigationViewItem>() {
                        nvi.set_menu_item_template(self.menu_item_template());
                    }
                    roots.push(Entry {
                        data: item,
                        container,
                        ancestors: Vec::new(),
                        footer,
                    });
                }
            }
            *self.roots.borrow_mut() = roots;
            let entries = self.entries();
            *self.known_entries.borrow_mut() = entries.clone();
            let was_syncing = self.syncing_selection.replace(true);
            for removed in previous_known.iter().filter(|old| {
                !entries
                    .iter()
                    .any(|entry| entry.container.ptr_eq(&old.container))
            }) {
                removed
                    .container
                    .set_current_value(NavigationViewItemBase::is_selected_property(), false);
                if let Some(item) = removed.container.cast::<NavigationViewItem>() {
                    item.set_current_value(NavigationViewItem::is_child_selected_property(), false);
                }
            }
            self.syncing_selection.set(was_syncing);
            for entry in entries {
                self.subscribe_item(&entry);
            }
            let selected = self.selected_item();
            if selected.is_some()
                && self.container_from_menu_item(&selected).is_none()
                && previous_known
                    .iter()
                    .any(|entry| reference_equals(&entry.data, &selected))
            {
                self.set_current_value(Self::selected_item_property(), None);
            } else if selected.is_none() {
                if let Some(entry) = self
                    .entries()
                    .into_iter()
                    .find(|entry| entry.container.is_selected())
                {
                    self.set_current_value(Self::selected_item_property(), entry.data);
                }
            }
            self.synchronize_selection();
            self.update_parts();
            if self.pending_selection.get()
                && self
                    .container_from_menu_item(&self.selected_item())
                    .is_some()
            {
                self.pending_selection.set(false);
                self.raise_selection_changed();
            }
            if !self.rebuild_pending.get() {
                break;
            }
        }
        self.rebuilding.set(false);
    }

    fn entries(&self) -> Vec<Entry> {
        fn append(entry: Entry, result: &mut Vec<Entry>) {
            if result
                .iter()
                .any(|known| known.container.ptr_eq(&entry.container))
            {
                return;
            }
            let children = entry
                .container
                .cast::<NavigationViewItem>()
                .map(|item| item.child_containers())
                .unwrap_or_default();
            let mut ancestors = entry.ancestors.clone();
            ancestors.push(entry.container.clone());
            let footer = entry.footer;
            result.push(entry);
            for (data, container) in children {
                append(
                    Entry {
                        data,
                        container,
                        ancestors: ancestors.clone(),
                        footer,
                    },
                    result,
                );
            }
        }
        let mut result = Vec::new();
        for entry in self.roots.borrow().iter().cloned() {
            append(entry, &mut result);
        }
        append(
            Entry {
                data: Some(Control::boxed(self.settings_item())),
                container: self.settings_item().upcast(),
                ancestors: Vec::new(),
                footer: true,
            },
            &mut result,
        );
        result
    }

    fn subscribe_item(&self, entry: &Entry) {
        let weak = self.to_ref().downgrade();
        let weak_container = entry.container.downgrade();
        let property = entry.container.property_changed(move |change| {
            let (Some(owner), Some(container)) = (weak.upgrade(), weak_container.upgrade()) else {
                return;
            };
            if !change.is_effective_value_change() {
                return;
            }
            if change.property() == NavigationViewItemBase::is_selected_property().as_property() {
                if owner.syncing_selection.get() {
                    return;
                }
                let data = owner.menu_item_from_container(&container);
                if container.is_selected() {
                    owner.set_current_value(Self::selected_item_property(), data);
                } else if reference_equals(&owner.selected_item(), &data) {
                    owner.set_current_value(Self::selected_item_property(), None);
                }
            } else if change.property() == NavigationViewItem::is_expanded_property().as_property()
            {
                if let Some(item) = container.cast::<NavigationViewItem>() {
                    owner.item_expansion_changed(&item);
                }
            }
        });
        self.item_subscriptions.borrow_mut().push(property);
        if let Some(item) = entry.container.cast::<NavigationViewItem>() {
            // Observe each realized container directly: popup children have their
            // own visual event route, which need not pass through NavigationView.
            // Both captures and the disposable's target are weak. Rebuild/removal
            // and owner disposal use the same subscription cleanup as invocation.
            let weak = self.to_ref().downgrade();
            let weak_item = item.downgrade();
            let focus =
                item.add_disposable_handler(InputElement::got_focus_event(), move |_, args| {
                    let (Some(owner), Some(item)) = (weak.upgrade(), weak_item.upgrade()) else {
                        return;
                    };
                    if owner.selection_follows_focus()
                        != NavigationViewSelectionFollowsFocus::Enabled
                        || !item.selects_on_invoked()
                        || item.is_selected()
                        || !args.new_focused_element.as_ref().is_some_and(|focused| {
                            focused.ptr_eq(&item.clone().upcast::<InputElement>())
                        })
                    {
                        return;
                    }
                    // An ancestor item can also receive the bubbling event. The
                    // identity check above restricts invocation to the focused item.
                    owner.item_invoked_core(&item);
                });
            let weak = self.to_ref().downgrade();
            let weak_item = item.downgrade();
            let keyboard =
                item.add_disposable_handler(InputElement::key_down_event(), move |_, args| {
                    let (Some(owner), Some(item)) = (weak.upgrade(), weak_item.upgrade()) else {
                        return;
                    };
                    if args
                        .source()
                        .is_some_and(|source| source.ptr_eq(&item.to_ref()))
                    {
                        owner.item_key_down(&item, args);
                    }
                });
            let weak = self.to_ref().downgrade();
            let weak_item = item.downgrade();
            let invoked = item.invoked(move || {
                if let (Some(owner), Some(item)) = (weak.upgrade(), weak_item.upgrade()) {
                    owner.item_invoked_core(&item);
                }
            });
            let weak = self.to_ref().downgrade();
            let items = item.items_changed(move || {
                if let Some(owner) = weak.upgrade() {
                    owner.rebuild_items(false);
                }
            });
            self.item_subscriptions
                .borrow_mut()
                .extend([focus, keyboard, invoked, items]);
        }
    }

    fn item_key_down(&self, item: &NavigationViewItem, args: &KeyEventArgs) {
        if args.handled()
            || args
                .key_modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
        {
            return;
        }
        let entries = self.entries();
        let Some(current) = entries
            .iter()
            .find(|entry| entry.container.ptr_eq(&item.to_ref()))
        else {
            return;
        };
        let top = self.pane_display_mode() == NavigationViewPaneDisplayMode::Top;
        let flyout_mode = top
            || self.is_pane_visible()
                && !self.is_pane_open()
                && self.display_mode() != NavigationViewDisplayMode::Minimal;
        let flyout_root = if flyout_mode {
            current
                .ancestors
                .first()
                .cloned()
                .or_else(|| item.is_expanded().then(|| current.container.clone()))
        } else {
            None
        };
        let can_focus = |entry: &&Entry| {
            entry.container.cast::<NavigationViewItem>().is_some()
                && entry.container.focusable()
                && entry.container.is_effectively_visible()
                && entry.container.is_effectively_enabled()
                && entry.ancestors.iter().all(|ancestor| {
                    ancestor
                        .cast::<NavigationViewItem>()
                        .is_some_and(|item| item.is_expanded())
                })
        };
        if matches!(args.key, Key::Home | Key::End) {
            // The pinned implementation uses the root menu/footer repeater, or
            // the direct children of the item whose flyout is open. Expanded
            // descendants are included only by Up/Down traversal below.
            let roots: Vec<_> = entries
                .iter()
                .filter(can_focus)
                .filter(|entry| {
                    if let Some(root) = &flyout_root {
                        entry.ancestors.len() == 1 && entry.ancestors[0].ptr_eq(root)
                    } else {
                        entry.ancestors.is_empty() && entry.footer == current.footer
                    }
                })
                .collect();
            let target = if args.key == Key::Home {
                roots.first()
            } else {
                roots.last()
            };
            args.set_handled(true);
            if let Some(target) = target {
                target
                    .container
                    .focus_with(NavigationMethod::Directional, args.key_modifiers);
            }
            return;
        }
        let horizontal =
            top && current.ancestors.is_empty() && matches!(args.key, Key::Left | Key::Right);
        let forward = if horizontal {
            if item.flow_direction() == FlowDirection::RightToLeft {
                Key::Left
            } else {
                Key::Right
            }
        } else {
            Key::Down
        };
        let backward = if horizontal {
            if item.flow_direction() == FlowDirection::RightToLeft {
                Key::Right
            } else {
                Key::Left
            }
        } else {
            Key::Up
        };
        if args.key != forward && args.key != backward {
            return;
        }
        let candidates: Vec<_> = entries
            .iter()
            .filter(can_focus)
            .filter(|entry| {
                if horizontal {
                    entry.ancestors.is_empty()
                } else if let Some(root) = &flyout_root {
                    entry
                        .ancestors
                        .first()
                        .is_some_and(|ancestor| ancestor.ptr_eq(root))
                } else {
                    !top
                }
            })
            .collect();
        let index = candidates
            .iter()
            .position(|entry| entry.container.ptr_eq(&current.container));
        let target = if let Some(index) = index {
            if args.key == forward {
                candidates.get(index + 1)
            } else {
                index.checked_sub(1).and_then(|index| candidates.get(index))
            }
        } else if args.key == Key::Down
            && flyout_root
                .as_ref()
                .is_some_and(|root| root.ptr_eq(&current.container))
        {
            // Down on an expanded Top/compact root enters its native popup.
            candidates.first()
        } else {
            None
        };
        if let Some(target) = target {
            args.set_handled(
                target
                    .container
                    .focus_with(NavigationMethod::Directional, args.key_modifiers),
            );
        }
    }

    /// Returns the realized native container for a menu, footer or nested model item.
    pub fn container_from_menu_item(
        &self,
        item: &Option<BoxedValue>,
    ) -> Option<Ref<NavigationViewItemBase>> {
        if item.is_none() {
            return None;
        }
        if self.is_settings_item(item) {
            return Some(self.settings_item().upcast());
        }
        self.entries()
            .into_iter()
            .find(|entry| reference_equals(&entry.data, item))
            .map(|entry| entry.container)
    }
    pub fn menu_item_from_container(&self, container: &FerroObject) -> Option<BoxedValue> {
        self.entries()
            .into_iter()
            .find(|entry| entry.container.ptr_eq(&container.to_ref()))
            .and_then(|entry| entry.data)
    }
    pub fn expand(&self, item: &NavigationViewItem) {
        if item.has_children_for_navigation() {
            item.set_is_expanded(true);
        }
    }
    pub fn collapse(&self, item: &NavigationViewItem) {
        if item.has_children_for_navigation() {
            item.set_is_expanded(false);
        }
    }

    fn is_settings_item(&self, item: &Option<BoxedValue>) -> bool {
        reference_equals(item, &Some(Control::boxed(self.settings_item())))
            || item.is_some() && reference_equals(item, &self.settings_item.content())
    }
    fn synchronize_selection(&self) {
        let was_syncing = self.syncing_selection.replace(true);
        loop {
            let selected = self.selected_item();
            let selected_container = self.container_from_menu_item(&selected);
            let entries = self.entries();
            let ancestors = entries
                .iter()
                .find(|entry| reference_equals(&entry.data, &selected))
                .map(|entry| entry.ancestors.clone())
                .unwrap_or_default();
            for entry in entries {
                entry.container.set_current_value(
                    NavigationViewItemBase::is_selected_property(),
                    selected_container
                        .as_ref()
                        .is_some_and(|selected| selected.ptr_eq(&entry.container)),
                );
                if let Some(item) = entry.container.cast::<NavigationViewItem>() {
                    item.set_current_value(
                        NavigationViewItem::is_child_selected_property(),
                        ancestors
                            .iter()
                            .any(|ancestor| ancestor.ptr_eq(&entry.container)),
                    );
                }
            }
            if reference_equals(&selected, &self.selected_item()) {
                break;
            }
        }
        self.syncing_selection.set(was_syncing);
    }
    fn selection_property_changed(&self, old: Option<BoxedValue>, next: Option<BoxedValue>) {
        let was_pending = self.pending_selection.replace(false);
        self.pending_selection_revision
            .set(self.pending_selection_revision.get().wrapping_add(1));
        if self.syncing_selection.get() {
            return;
        }
        if let Some(container) = self.container_from_menu_item(&next) {
            if container
                .cast::<NavigationViewItem>()
                .is_some_and(|item| !item.selects_on_invoked())
            {
                self.syncing_selection.set(true);
                self.set_current_value(Self::selected_item_property(), old.clone());
                self.syncing_selection.set(false);
                self.synchronize_selection();
                self.pending_invocation.borrow_mut().take();
                self.raise_item_invoked(&container);
                // Suppressing an action does not cancel an earlier, still
                // unresolved selection. A handler may redirect it while invoked.
                if was_pending
                    && old.is_some()
                    && reference_equals(&old, &self.selected_item())
                    && self.container_from_menu_item(&old).is_none()
                    && !self.pending_selection.get()
                {
                    self.pending_selection.set(true);
                    self.defer_selection_changed();
                }
                return;
            }
        }
        let pending = self.pending_invocation.borrow_mut().take();
        if let Some(invoked) = pending {
            self.raise_item_invoked(&invoked);
        }
        // The application may redirect selection from inside ItemInvoked.
        if !reference_equals(&next, &self.selected_item()) {
            return;
        }
        self.synchronize_selection();
        let next = self.selected_item();
        self.pending_selection
            .set(next.is_some() && self.container_from_menu_item(&next).is_none());
        if self.pending_selection.get() {
            self.defer_selection_changed();
        } else {
            self.raise_selection_changed();
        }
        if let Some(item) = self
            .container_from_menu_item(&next)
            .and_then(|container| container.cast::<NavigationViewItem>())
        {
            self.close_after_item(&item);
        }
    }
    fn defer_selection_changed(&self) {
        // The pinned control completes pending selection at low dispatcher
        // priority even when no container is realized. A newer selection or
        // synchronous realization invalidates this callback; the owner is weak.
        let revision = self.pending_selection_revision.get();
        let weak = self.to_ref().downgrade();
        Dispatcher::ui_thread().post_local(
            move || {
                if let Some(owner) = weak.upgrade() {
                    if owner.pending_selection_revision.get() == revision
                        && owner.pending_selection.replace(false)
                    {
                        owner.raise_selection_changed();
                    }
                }
            },
            DispatcherPriority::BACKGROUND,
        );
    }
    fn raise_selection_changed(&self) {
        let selected = self.selected_item();
        let args = NavigationViewSelectionChangedEventArgs {
            selected_item_container: self.container_from_menu_item(&selected),
            is_settings_selected: self.is_settings_item(&selected),
            selected_item: selected,
            identity: Rc::new(()),
        };
        for (_, handler) in self.selection_changed.snapshot().iter() {
            handler(&args);
        }
        if let Some(peer) = ControlAutomationPeer::from_element(self)
            .and_then(|peer| peer.cast::<NavigationViewAutomationPeer>())
        {
            peer.raise_property_changed_event(
                SelectionPatternIdentifiers::selection_property(),
                None,
                Some(Rc::new(peer.get_selection())),
            );
        }
    }
    fn raise_item_invoked(&self, container: &Ref<NavigationViewItemBase>) {
        let data = self.menu_item_from_container(container);
        let args = NavigationViewItemInvokedEventArgs {
            invoked_item: container.content(),
            invoked_item_container: Some(container.clone()),
            is_settings_invoked: self.is_settings_item(&data),
            identity: Rc::new(()),
        };
        for (_, handler) in self.item_invoked.snapshot().iter() {
            handler(&args);
        }
    }
    fn item_invoked_core(&self, item: &NavigationViewItem) {
        if !item.is_effectively_enabled() {
            return;
        }
        let container: Ref<NavigationViewItemBase> = item.to_ref().upcast();
        let Some(entry) = self
            .entries()
            .into_iter()
            .find(|entry| entry.container.ptr_eq(&container))
        else {
            return;
        };
        let data = entry.data;
        if item.selects_on_invoked() && !reference_equals(&self.selected_item(), &data) {
            *self.pending_invocation.borrow_mut() = Some(container.clone());
            self.set_current_value(Self::selected_item_property(), data);
            self.pending_invocation.borrow_mut().take();
        } else {
            self.raise_item_invoked(&container);
        }
        if item.has_children_for_navigation() {
            item.set_is_expanded(!item.is_expanded());
        }
        self.close_after_item(item);
        if item.selects_on_invoked() && !item.has_children_for_navigation() {
            if let Some(entry) = self
                .entries()
                .into_iter()
                .find(|entry| entry.container.ptr_eq(&item.to_ref()))
            {
                if let Some(root) = entry
                    .ancestors
                    .first()
                    .and_then(|ancestor| ancestor.cast::<NavigationViewItem>())
                {
                    if self.pane_display_mode() == NavigationViewPaneDisplayMode::Top
                        || self.display_mode() != NavigationViewDisplayMode::Minimal
                            && !self.is_pane_open()
                    {
                        root.set_is_expanded(false);
                    }
                }
            }
        }
    }
    fn close_after_item(&self, item: &NavigationViewItem) {
        if self.is_pane_open()
            && self.display_mode() != NavigationViewDisplayMode::Expanded
            && !item.has_children_for_navigation()
        {
            self.set_pane_open_internal(false);
        }
    }
    fn item_expansion_changed(&self, item: &NavigationViewItem) {
        let container: Ref<NavigationViewItemBase> = item.to_ref().upcast();
        let data = self.menu_item_from_container(&container);
        if item.is_expanded() {
            let args = NavigationViewItemExpandingEventArgs {
                expanding_item: data,
                expanding_item_container: container,
                identity: Rc::new(()),
            };
            for (_, handler) in self.expanding.snapshot().iter() {
                handler(&args);
            }
        } else {
            let args = NavigationViewItemCollapsedEventArgs {
                collapsed_item: data,
                collapsed_item_container: container,
                identity: Rc::new(()),
            };
            for (_, handler) in self.collapsed.snapshot().iter() {
                handler(&args);
            }
        }
    }
}

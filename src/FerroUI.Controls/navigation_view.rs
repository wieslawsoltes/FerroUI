//! WinUI's `NavigationView` (`controls/dev/NavigationView/NavigationView.cpp`, `NavigationView.h`
//! and `NavigationView.idl` of microsoft-ui-xaml, commit `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`;
//! MIT, Copyright (c) Microsoft Corporation, see NOTICE.md).

use crate::automation::peers::{AutomationPeer, ControlAutomationPeer, NavigationViewAutomationPeer};
use crate::automation::{AutomationProperties, SelectionPatternIdentifiers};
use crate::items_source::{reference_equals, ItemsSource};
use crate::navigation_view_events::*;
use crate::primitives::{
    NavigationViewTemplateSettings, TemplateAppliedEventArgs, TemplatedControl, TemplatedControlImpl,
    TemplatedControlImplExt,
};
use crate::templates::{IControlTemplate, IDataTemplate};
use crate::{
    AutoSuggestBox, Button, ContentControl, ContentControlImpl, Control, ControlImpl, ItemCollection, ItemsSourceView,
    NavigationViewItem, NavigationViewItemBase, SplitView, SplitViewDisplayMode, StackPanel, Symbol, SymbolIcon,
    ToolTip,
};
use ferroui_base::data::BindingPriority;
use ferroui_base::input::{
    InputElement, InputElementImpl, InputElementImplExt, Key, KeyEventArgs, KeyModifiers, NavigationMethod,
};
use ferroui_base::interactivity::{InteractiveImpl, RoutedEventHandlerToken};
use ferroui_base::layout::{LayoutableImpl, LayoutableImplExt};
use ferroui_base::media::FlowDirection;
use ferroui_base::metadata::{into_markup_value, MarkupDelegate};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::styling::ControlTheme;
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, BoxedValue, DirectProperty,
    FerroObject, FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref, Size,
    StyledElement, StyledElementImpl, StyledProperty, StyledPropertyOptions, Thickness, VisualImpl, WeakRef,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The size of the back button (`c_backButtonWidth`, `c_backButtonHeight`).
const BACK_BUTTON_WIDTH: f64 = 40.0;
const BACK_BUTTON_HEIGHT: f64 = 40.0;
/// The localized texts of WinUI (`SR_SettingsButtonName`, `SR_NavigationButtonClosedName`,
/// `SR_NavigationButtonOpenName`) in English.
const SETTINGS_NAME: &str = "Settings";
const NAVIGATION_CLOSED_NAME: &str = "Open Navigation";
const NAVIGATION_OPEN_NAME: &str = "Close Navigation";

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

/// The parts of the template that the control looks up, by WinUI's names.
#[derive(Clone, Default)]
struct Parts {
    split: Option<Ref<SplitView>>,
    toggle: Option<Ref<Button>>,
    back: Option<Ref<Button>>,
    close: Option<Ref<Button>>,
    search_button: Option<Ref<Button>>,
    pane_title: Option<Ref<Control>>,
    pane_title_holder: Option<Ref<Control>>,
    pane_title_presenter: Option<Ref<ContentControl>>,
    pane_title_top: Option<Ref<ContentControl>>,
    left_menu: Option<Ref<StackPanel>>,
    left_footer: Option<Ref<StackPanel>>,
    top_menu: Option<Ref<StackPanel>>,
    top_footer: Option<Ref<StackPanel>>,
    left_pane_header: Option<Ref<ContentControl>>,
    top_pane_header: Option<Ref<ContentControl>>,
    left_pane_custom: Option<Ref<ContentControl>>,
    top_pane_custom: Option<Ref<ContentControl>>,
    left_pane_footer: Option<Ref<ContentControl>>,
    top_pane_footer: Option<Ref<ContentControl>>,
    left_search: Option<Ref<ContentControl>>,
    top_search: Option<Ref<ContentControl>>,
    content_left_padding: Option<Ref<Control>>,
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
        // The content of the application leaves the old parts before a new template takes it.
        for host in [&self.left_menu, &self.left_footer, &self.top_menu, &self.top_footer].into_iter().flatten() {
            host.children().clear();
        }
        for presenter in [
            &self.left_pane_header,
            &self.top_pane_header,
            &self.left_pane_custom,
            &self.top_pane_custom,
            &self.left_pane_footer,
            &self.top_pane_footer,
            &self.left_search,
            &self.top_search,
        ]
        .into_iter()
        .flatten()
        {
            presenter.set_content(None);
        }
    }

    fn pane_title_parents(&self) -> [Option<Ref<ContentControl>>; 3] {
        [
            self.toggle.clone().map(|toggle| toggle.upcast::<ContentControl>()),
            self.pane_title_presenter.clone(),
            self.pane_title_top.clone(),
        ]
    }
}

/// A navigation pane with menu items and a content area (WinUI's `NavigationView`).
///
/// The pane is at the left (`Left`, `LeftCompact`, `LeftMinimal`, or chosen by the width in `Auto`:
/// the display modes `Expanded`, `Compact` and `Minimal`) or at the top (`Top`). The template of the
/// WinUI theme is WinUI's (`NavigationView.xaml`): the control finds WinUI's parts (`RootSplitView`,
/// `TogglePaneButton`, `NavigationViewBackButton`, `NavigationViewCloseButton`,
/// `PaneAutoSuggestButton`, `PaneTitleTextBlock`, `PaneTitleHolder`, `PaneTitlePresenter`,
/// `PaneTitleOnTopPane`, `MenuItemsHost`, `FooterMenuItemsHost`, `TopNavMenuItemsHost`,
/// `TopFooterMenuItemsHost`, the content controls of the pane header, custom content, footer and auto
/// suggest box of the left and the top pane, `ContentLeftPadding`), sets their contents and the
/// values of [`template_settings`](Self::template_settings) as WinUI's code does, and sets WinUI's
/// visual states as pseudo-classes (`:compact`, `:expanded`, `:minimal`, `:top-navigation-minimal`,
/// `:minimal-with-back-button`, `:toggle-pane-button-visible`, `:header-collapsed`,
/// `:auto-suggest-box-collapsed`, `:closed-compact`, `:list-size-compact`, `:pane-not-overlaying`,
/// `:back-button-collapsed`, `:pane-collapsed`, and `:back-button-row-visible` for the row WinUI
/// sizes in code). The Fluent and Simple themes have no template.
///
/// The menu items are realized in stack panels, not in `ItemsRepeater`s with a hierarchical
/// `SelectionModel` (follow-up F1). Missing (docs/porting/winui-controls.md, section 10.4): the
/// overflow of the top pane, the animations of the selection indicator and of the pane, the
/// shadow of the pane, the padding of the title bar, the shoulder navigation and the selectors of
/// templates and styles.
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
    template_settings: Ref<NavigationViewTemplateSettings>,
    display_mode: Cell<NavigationViewDisplayMode>,
    applied_template: RefCell<Option<Rc<dyn IControlTemplate>>>,
    parts: RefCell<Parts>,
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
    is_closed_compact: Cell<bool>,
    list_size_compact: Cell<bool>,
    initial_list_size_state_set: Cell<bool>,
    is_left_pane_title_empty: Cell<bool>,
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
    markup: {
        namespace: "FerroUI.Controls",
        content: Content,
        attributes: [
            TemplatePart("RootSplitView", type(Ref<SplitView>)),
            TemplatePart("TogglePaneButton", type(Ref<Button>)),
            TemplatePart("NavigationViewBackButton", type(Ref<Button>)),
            TemplatePart("NavigationViewCloseButton", type(Ref<Button>)),
            TemplatePart("PaneAutoSuggestButton", type(Ref<Button>)),
            TemplatePart("PaneTitleTextBlock", type(Ref<Control>)),
            TemplatePart("PaneTitleHolder", type(Ref<Control>)),
            TemplatePart("PaneTitlePresenter", type(Ref<ContentControl>)),
            TemplatePart("PaneTitleOnTopPane", type(Ref<ContentControl>)),
            TemplatePart("MenuItemsHost", type(Ref<StackPanel>)),
            TemplatePart("FooterMenuItemsHost", type(Ref<StackPanel>)),
            TemplatePart("TopNavMenuItemsHost", type(Ref<StackPanel>)),
            TemplatePart("TopFooterMenuItemsHost", type(Ref<StackPanel>)),
            TemplatePart("PaneHeaderContentBorder", type(Ref<ContentControl>)),
            TemplatePart("PaneHeaderOnTopPane", type(Ref<ContentControl>)),
            TemplatePart("PaneCustomContentBorder", type(Ref<ContentControl>)),
            TemplatePart("PaneCustomContentOnTopPane", type(Ref<ContentControl>)),
            TemplatePart("FooterContentBorder", type(Ref<ContentControl>)),
            TemplatePart("PaneFooterOnTopPane", type(Ref<ContentControl>)),
            TemplatePart("PaneAutoSuggestBoxPresenter", type(Ref<ContentControl>)),
            TemplatePart("TopPaneAutoSuggestBoxPresenter", type(Ref<ContentControl>)),
            TemplatePart("ContentLeftPadding", type(Ref<Control>)),
            PseudoClasses(":compact", ":expanded", ":minimal", ":top-navigation-minimal", ":minimal-with-back-button",
                ":toggle-pane-button-visible", ":header-collapsed", ":auto-suggest-box-collapsed", ":closed-compact",
                ":list-size-compact", ":pane-not-overlaying", ":back-button-collapsed", ":pane-collapsed",
                ":back-button-row-visible"),
        ],
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

/// The coercion of WinUI's lengths and widths (`NavigationView::OnPropertyChanged`'s
/// `CoerceToGreaterThanZero`): a negative length is 0.
fn non_negative(_: &FerroObject, value: f64) -> f64 {
    if value < 0.0 {
        0.0
    } else {
        value
    }
}

ferro_properties! { impl NavigationView {
    /// Defines the `IsPaneOpen` property: whether the pane is open. `true` by default.
    pub fn is_pane_open_property() -> StyledProperty<bool> {
        FerroProperty::register::<NavigationView, _>("IsPaneOpen", true)
    }
    /// Defines the `CompactModeThresholdWidth` property: the width from which the display mode of
    /// `Auto` is `Compact`. 641 by default.
    pub fn compact_mode_threshold_width_property() -> StyledProperty<f64> {
        FerroProperty::register_with::<NavigationView, _>("CompactModeThresholdWidth", StyledPropertyOptions::new(641.0).coerce(non_negative))
    }
    /// Defines the `ExpandedModeThresholdWidth` property: the width from which the display mode of
    /// `Auto` is `Expanded`. 1008 by default.
    pub fn expanded_mode_threshold_width_property() -> StyledProperty<f64> {
        FerroProperty::register_with::<NavigationView, _>("ExpandedModeThresholdWidth", StyledPropertyOptions::new(1008.0).coerce(non_negative))
    }
    /// Defines the `IsSettingsVisible` property: whether the settings item is shown. `true` by
    /// default.
    pub fn is_settings_visible_property() -> StyledProperty<bool> {
        FerroProperty::register::<NavigationView, _>("IsSettingsVisible", true)
    }
    /// Defines the `IsPaneToggleButtonVisible` property. `true` by default.
    pub fn is_pane_toggle_button_visible_property() -> StyledProperty<bool> {
        FerroProperty::register::<NavigationView, _>("IsPaneToggleButtonVisible", true)
    }
    /// Defines the `AlwaysShowHeader` property. `true` by default.
    pub fn always_show_header_property() -> StyledProperty<bool> {
        FerroProperty::register::<NavigationView, _>("AlwaysShowHeader", true)
    }
    /// Defines the `CompactPaneLength` property: the width of the closed compact pane. 48 by
    /// default; the WinUI theme sets `NavigationViewCompactPaneLength`.
    pub fn compact_pane_length_property() -> StyledProperty<f64> {
        FerroProperty::register_with::<NavigationView, _>("CompactPaneLength", StyledPropertyOptions::new(48.0).coerce(non_negative))
    }
    /// Defines the `OpenPaneLength` property: the width of the open pane. 320 by default.
    pub fn open_pane_length_property() -> StyledProperty<f64> {
        FerroProperty::register_with::<NavigationView, _>("OpenPaneLength", StyledPropertyOptions::new(320.0).coerce(non_negative))
    }
    /// Defines the `IsPaneVisible` property: whether the pane is shown at all. `true` by default.
    pub fn is_pane_visible_property() -> StyledProperty<bool> {
        FerroProperty::register::<NavigationView, _>("IsPaneVisible", true)
    }
    /// Defines the `IsBackEnabled` property: whether the back button is enabled.
    pub fn is_back_enabled_property() -> StyledProperty<bool> {
        FerroProperty::register::<NavigationView, _>("IsBackEnabled", false)
    }
    /// Defines the `IsBackButtonVisible` property. `Auto` by default.
    pub fn is_back_button_visible_property() -> StyledProperty<NavigationViewBackButtonVisible> {
        FerroProperty::register::<NavigationView, _>("IsBackButtonVisible", NavigationViewBackButtonVisible::Auto)
    }
    /// Defines the `PaneDisplayMode` property. `Auto` by default.
    pub fn pane_display_mode_property() -> StyledProperty<NavigationViewPaneDisplayMode> {
        FerroProperty::register::<NavigationView, _>("PaneDisplayMode", NavigationViewPaneDisplayMode::Auto)
    }
    /// Defines the `SelectionFollowsFocus` property. `Disabled` by default.
    pub fn selection_follows_focus_property() -> StyledProperty<NavigationViewSelectionFollowsFocus> {
        FerroProperty::register::<NavigationView, _>("SelectionFollowsFocus", NavigationViewSelectionFollowsFocus::Disabled)
    }
    /// Defines the `PaneTitle` property: the title of the pane.
    pub fn pane_title_property() -> StyledProperty<String> {
        FerroProperty::register::<NavigationView, _>("PaneTitle", String::new())
    }
    /// Defines the `Header` property: the header above the content.
    pub fn header_property() -> StyledProperty<Option<BoxedValue>> {
        FerroProperty::register::<NavigationView, _>("Header", None)
    }
    /// Defines the `HeaderTemplate` property.
    pub fn header_template_property() -> StyledProperty<Option<Rc<dyn IDataTemplate>>> {
        FerroProperty::register::<NavigationView, _>("HeaderTemplate", None)
    }
    /// Defines the `SelectedItem` property: the selected menu item (the data item or the
    /// container).
    pub fn selected_item_property() -> StyledProperty<Option<BoxedValue>> {
        FerroProperty::register::<NavigationView, _>("SelectedItem", None)
    }
    /// Defines the `PaneHeader` property.
    pub fn pane_header_property() -> StyledProperty<Option<Ref<Control>>> {
        FerroProperty::register::<NavigationView, _>("PaneHeader", None)
    }
    /// Defines the `PaneFooter` property.
    pub fn pane_footer_property() -> StyledProperty<Option<Ref<Control>>> {
        FerroProperty::register::<NavigationView, _>("PaneFooter", None)
    }
    /// Defines the `PaneCustomContent` property.
    pub fn pane_custom_content_property() -> StyledProperty<Option<Ref<Control>>> {
        FerroProperty::register::<NavigationView, _>("PaneCustomContent", None)
    }
    /// Defines the `ContentOverlay` property: content shown under the top pane.
    pub fn content_overlay_property() -> StyledProperty<Option<Ref<Control>>> {
        FerroProperty::register::<NavigationView, _>("ContentOverlay", None)
    }
    /// Defines the `AutoSuggestBox` property: the search box of the pane.
    pub fn auto_suggest_box_property() -> StyledProperty<Option<Ref<AutoSuggestBox>>> {
        FerroProperty::register::<NavigationView, _>("AutoSuggestBox", None)
    }
    /// Defines the `MenuItemsSource` property.
    pub fn menu_items_source_property() -> StyledProperty<Option<ItemsSource>> {
        FerroProperty::register::<NavigationView, _>("MenuItemsSource", None)
    }
    /// Defines the `FooterMenuItemsSource` property.
    pub fn footer_menu_items_source_property() -> StyledProperty<Option<ItemsSource>> {
        FerroProperty::register::<NavigationView, _>("FooterMenuItemsSource", None)
    }
    /// Defines the `MenuItemTemplate` property: the template of the data items.
    pub fn menu_item_template_property() -> StyledProperty<Option<Rc<dyn IDataTemplate>>> {
        FerroProperty::register::<NavigationView, _>("MenuItemTemplate", None)
    }
    /// Defines the `PaneToggleButtonStyle` property: the control theme of the pane toggle button
    /// (WinUI's style; the WinUI theme sets `PaneToggleButtonStyle`).
    pub fn pane_toggle_button_style_property() -> StyledProperty<Option<Ref<ControlTheme>>> {
        FerroProperty::register::<NavigationView, _>("PaneToggleButtonStyle", None)
    }
    /// Defines the `MenuItemContainerStyle` property: the control theme of the containers.
    pub fn menu_item_container_style_property() -> StyledProperty<Option<Ref<ControlTheme>>> {
        FerroProperty::register::<NavigationView, _>("MenuItemContainerStyle", None)
    }
    /// Defines the `MenuItems` property (read-only; the collection is writable).
    pub fn menu_items_property() -> DirectProperty<NavigationView, ItemCollection> {
        FerroProperty::register_direct::<NavigationView, _>("MenuItems", |o| o.menu_items(), None, ItemCollection::new())
    }
    /// Defines the `FooterMenuItems` property (read-only; the collection is writable).
    pub fn footer_menu_items_property() -> DirectProperty<NavigationView, ItemCollection> {
        FerroProperty::register_direct::<NavigationView, _>("FooterMenuItems", |o| o.footer_menu_items(), None, ItemCollection::new())
    }
    /// Defines the `SettingsItem` property: the settings item (read-only).
    pub fn settings_item_property() -> DirectProperty<NavigationView, Option<BoxedValue>> {
        FerroProperty::register_direct::<NavigationView, _>("SettingsItem", |o| Some(Control::boxed(o.settings_item())), None, None)
    }
    /// Defines the `DisplayMode` property: the display mode (read-only).
    pub fn display_mode_property() -> DirectProperty<NavigationView, NavigationViewDisplayMode> {
        FerroProperty::register_direct::<NavigationView, _>("DisplayMode", |o| o.display_mode(), None, NavigationViewDisplayMode::Minimal)
    }
    /// Defines the `TemplateSettings` property: the values the template binds to (read-only).
    pub fn template_settings_property() -> DirectProperty<NavigationView, Ref<NavigationViewTemplateSettings>> {
        FerroProperty::register_direct::<NavigationView, _>("TemplateSettings", |o| o.template_settings(), None, NavigationViewTemplateSettings::new())
    }
} }

impl NavigationView {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        // CreateAndHookEventsToSettings: the settings item with the Setting symbol (WinUI's animated
        // icon falls back to it) and the localized name.
        let settings_item = NavigationViewItem::new();
        settings_item.set_content(Some(Rc::new(SETTINGS_NAME.to_owned())));
        settings_item.set_icon(Some(SymbolIcon::with_symbol(Symbol::Setting).upcast()));
        AutomationProperties::set_name(&settings_item, Some(SETTINGS_NAME.into()));
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
            template_settings: NavigationViewTemplateSettings::new(),
            display_mode: Cell::new(NavigationViewDisplayMode::Minimal),
            applied_template: RefCell::new(None),
            parts: RefCell::new(Parts::default()),
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
            is_closed_compact: Cell::new(false),
            list_size_compact: Cell::new(false),
            initial_list_size_state_set: Cell::new(false),
            is_left_pane_title_empty: Cell::new(true),
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

    /// Creates a navigation view.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets the menu items (writable, also while `MenuItemsSource` is set).
    pub fn menu_items(&self) -> ItemCollection {
        self.menu_items.clone()
    }

    /// Gets the footer menu items (writable, also while `FooterMenuItemsSource` is set).
    pub fn footer_menu_items(&self) -> ItemCollection {
        self.footer_items.clone()
    }

    /// Gets the settings item.
    pub fn settings_item(&self) -> Ref<NavigationViewItem> {
        self.settings_item.clone()
    }

    /// Gets the values the template binds to.
    pub fn template_settings(&self) -> Ref<NavigationViewTemplateSettings> {
        self.template_settings.clone()
    }

    /// Gets the display mode.
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

    /// Adds a handler of `SelectionChanged`; the disposable removes it.
    pub fn selection_changed(&self, handler: impl Fn(&NavigationViewSelectionChangedEventArgs) + 'static) -> Rc<dyn IDisposable> {
        let token = self.selection_changed.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(owner) = weak.upgrade() {
                owner.selection_changed.remove(token);
            }
        })
    }
    /// Adds a handler of `ItemInvoked`; the disposable removes it.
    pub fn item_invoked(&self, handler: impl Fn(&NavigationViewItemInvokedEventArgs) + 'static) -> Rc<dyn IDisposable> {
        let token = self.item_invoked.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(owner) = weak.upgrade() {
                owner.item_invoked.remove(token);
            }
        })
    }
    /// Adds a handler of `DisplayModeChanged`; the disposable removes it.
    pub fn display_mode_changed(&self, handler: impl Fn(&NavigationViewDisplayModeChangedEventArgs) + 'static) -> Rc<dyn IDisposable> {
        let token = self.display_mode_changed.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(owner) = weak.upgrade() {
                owner.display_mode_changed.remove(token);
            }
        })
    }
    /// Adds a handler of `BackRequested`; the disposable removes it.
    pub fn back_requested(&self, handler: impl Fn(&NavigationViewBackRequestedEventArgs) + 'static) -> Rc<dyn IDisposable> {
        let token = self.back_requested.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(owner) = weak.upgrade() {
                owner.back_requested.remove(token);
            }
        })
    }
    /// Adds a handler of `PaneClosing`; the disposable removes it.
    pub fn pane_closing(&self, handler: impl Fn(&NavigationViewPaneClosingEventArgs) + 'static) -> Rc<dyn IDisposable> {
        let token = self.pane_closing.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(owner) = weak.upgrade() {
                owner.pane_closing.remove(token);
            }
        })
    }
    /// Adds a handler of `Expanding`; the disposable removes it.
    pub fn expanding(&self, handler: impl Fn(&NavigationViewItemExpandingEventArgs) + 'static) -> Rc<dyn IDisposable> {
        let token = self.expanding.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(owner) = weak.upgrade() {
                owner.expanding.remove(token);
            }
        })
    }
    /// Adds a handler of `Collapsed`; the disposable removes it.
    pub fn collapsed(&self, handler: impl Fn(&NavigationViewItemCollapsedEventArgs) + 'static) -> Rc<dyn IDisposable> {
        let token = self.collapsed.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(owner) = weak.upgrade() {
                owner.collapsed.remove(token);
            }
        })
    }
    /// Adds a handler of `PaneOpening`; the disposable removes it.
    pub fn pane_opening(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        let token = self.pane_opening.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(owner) = weak.upgrade() {
                owner.pane_opening.remove(token);
            }
        })
    }
    /// Adds a handler of `PaneOpened`; the disposable removes it.
    pub fn pane_opened(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        let token = self.pane_opened.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(owner) = weak.upgrade() {
                owner.pane_opened.remove(token);
            }
        })
    }
    /// Adds a handler of `PaneClosed`; the disposable removes it.
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
            // UpdatePaneDisplayMode(old, new): from LeftMinimal to Left the pane opens.
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
        } else if property == Self::open_pane_length_property().as_property() {
            this.update_open_pane_length(this.last_width.get());
            this.update_parts();
        } else if property == Self::pane_header_property().as_property()
            || property == Self::pane_footer_property().as_property()
            || property == Self::pane_custom_content_property().as_property()
            || property == Self::auto_suggest_box_property().as_property()
            || property == Self::pane_title_property().as_property()
            || property == Self::header_property().as_property()
            || property == Self::always_show_header_property().as_property()
            || property == Self::is_settings_visible_property().as_property()
            || property == Self::is_pane_toggle_button_visible_property().as_property()
            || property == Self::is_back_button_visible_property().as_property()
            || property == Self::is_back_enabled_property().as_property()
            || property == Self::compact_pane_length_property().as_property()
            || property == Self::menu_item_container_style_property().as_property()
            || property == Self::selection_follows_focus_property().as_property()
        {
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
            // The templated control removes the old tree before OnApplyTemplate and does not call it
            // for no template: the old parts are revoked first, so that a retained button or split
            // view cannot act on the control afterwards.
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
            && (available_size.width != this.last_width.get() || this.initial_nonforced_layout.get())
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
        // OnKeyDown: Escape closes a pane that is not inline.
        if args.key == Key::Escape
            && this.is_pane_open()
            && this.display_mode() != NavigationViewDisplayMode::Expanded
            && this.try_close_pane()
        {
            args.set_handled(true);
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
            toggle: scope.find_as("TogglePaneButton"),
            back: scope.find_as("NavigationViewBackButton"),
            close: scope.find_as("NavigationViewCloseButton"),
            search_button: scope.find_as("PaneAutoSuggestButton"),
            pane_title: scope.find_as("PaneTitleTextBlock"),
            pane_title_holder: scope.find_as("PaneTitleHolder"),
            pane_title_presenter: scope.find_as("PaneTitlePresenter"),
            pane_title_top: scope.find_as("PaneTitleOnTopPane"),
            left_menu: scope.find_as("MenuItemsHost"),
            left_footer: scope.find_as("FooterMenuItemsHost"),
            top_menu: scope.find_as("TopNavMenuItemsHost"),
            top_footer: scope.find_as("TopFooterMenuItemsHost"),
            left_pane_header: scope.find_as("PaneHeaderContentBorder"),
            top_pane_header: scope.find_as("PaneHeaderOnTopPane"),
            left_pane_custom: scope.find_as("PaneCustomContentBorder"),
            top_pane_custom: scope.find_as("PaneCustomContentOnTopPane"),
            left_pane_footer: scope.find_as("FooterContentBorder"),
            top_pane_footer: scope.find_as("PaneFooterOnTopPane"),
            left_search: scope.find_as("PaneAutoSuggestBoxPresenter"),
            top_search: scope.find_as("TopPaneAutoSuggestBoxPresenter"),
            content_left_padding: scope.find_as("ContentLeftPadding"),
            ..Parts::default()
        };
        // OnPaneToggleButtonClick, also of the close button.
        for button in [&parts.toggle, &parts.close].into_iter().flatten() {
            let weak = this.to_ref().downgrade();
            let token = button.click(move |_, _| {
                if let Some(owner) = weak.upgrade() {
                    owner.was_force_closed.set(owner.is_pane_open());
                    if owner.is_pane_open() {
                        owner.try_close_pane();
                    } else {
                        owner.set_pane_open_internal(true);
                    }
                }
            });
            parts.buttons.push((button.downgrade(), token));
        }
        // OnBackButtonClicked.
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
        // OnPaneSearchButtonClick: the pane opens and the auto suggest box gets the focus.
        if let Some(search) = &parts.search_button {
            let weak = this.to_ref().downgrade();
            let token = search.click(move |_, _| {
                if let Some(owner) = weak.upgrade() {
                    owner.was_force_closed.set(false);
                    owner.set_pane_open_internal(true);
                    if let Some(search) = owner.auto_suggest_box() {
                        search.focus_with(NavigationMethod::Directional, KeyModifiers::NONE);
                    }
                }
            });
            parts.buttons.push((search.downgrade(), token));
        }
        if let Some(split) = &parts.split {
            // OnSplitViewPaneClosing: a light dismissal of the split view raises PaneClosing, which
            // can cancel it; a closing pane of a compact mode shows the compact list size.
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
            // The two way binding of IsPaneOpen of WinUI's template: a change of the split view
            // (a light dismissal) is the pane of the navigation view.
            let weak = this.to_ref().downgrade();
            let weak_split = split.downgrade();
            parts.subscriptions.push(split.property_changed(move |change| {
                let property = change.property();
                if property == SplitView::is_pane_open_property().as_property()
                    || property == SplitView::display_mode_property().as_property()
                {
                    if let (Some(owner), Some(split)) = (weak.upgrade(), weak_split.upgrade()) {
                        if !owner.updating_parts.get() {
                            if property == SplitView::is_pane_open_property().as_property() {
                                owner.set_pane_open_internal(split.is_pane_open());
                                owner.pane_event_raised.set(false);
                            }
                            owner.update_parts();
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
        // IsTabStop False of WinUI's style.
        InputElement::focusable_property().override_default_value::<NavigationView>(false);
    }

    fn is_top_navigation_view(&self) -> bool {
        self.pane_display_mode() == NavigationViewPaneDisplayMode::Top
    }

    /// `OnIsPaneOpenChanged` with the events #286 raises from the property: `PaneOpening` and
    /// `PaneOpened` when the pane opens, `PaneClosing` (unless a light dismissal raised it) and
    /// `PaneClosed` when it closes.
    fn pane_open_changed(&self) {
        let open = self.is_pane_open();
        if open {
            self.pane_event_raised.set(false);
            self.was_force_closed.set(false);
            // OnSplitViewPaneOpening: ListSizeFull.
            self.list_size_compact.set(false);
        } else if !self.pane_internal.get() {
            self.was_force_closed.set(true);
        }
        if open {
            for (_, handler) in self.pane_opening.snapshot().iter() {
                handler();
            }
        } else if !self.pane_event_raised.replace(false) {
            // An assignment of the application or of the adaptive layout is a forced close; a light
            // dismissal asks first, through try_close_pane or the split view.
            let args = NavigationViewPaneClosingEventArgs::default();
            for (_, handler) in self.pane_closing.snapshot().iter() {
                handler(&args);
            }
        }
        if open != self.is_pane_open() {
            return;
        }
        if !open && self.is_compact_split_mode() {
            // OnSplitViewPaneClosing: ListSizeCompact for the compact modes.
            self.list_size_compact.set(true);
        }
        self.update_parts();
        let handlers = if open { &self.pane_opened } else { &self.pane_closed };
        for (_, handler) in handlers.snapshot().iter() {
            handler();
        }
    }

    fn set_pane_open_internal(&self, value: bool) {
        let old = self.pane_internal.replace(true);
        self.set_current_value(Self::is_pane_open_property(), value);
        self.pane_internal.set(old);
    }

    /// `ClosePane` after `PaneClosing`, which can cancel it unless the pane was forced closed.
    fn try_close_pane(&self) -> bool {
        let args = NavigationViewPaneClosingEventArgs::default();
        for (_, handler) in self.pane_closing.snapshot().iter() {
            handler(&args);
        }
        if args.cancel() && !self.was_force_closed.get() {
            self.pane_event_raised.set(false);
            return false;
        }
        // A handler can close the pane itself: no flag then waits for a change that does not come.
        if !self.is_pane_open() {
            self.pane_event_raised.set(false);
            return true;
        }
        self.pane_event_raised.set(true);
        self.set_pane_open_internal(false);
        self.pane_event_raised.set(false);
        true
    }

    /// The display mode of the split view: `UpdateVisualStateForDisplayModeGroup`.
    fn split_display_mode(&self) -> SplitViewDisplayMode {
        if !self.is_pane_visible() {
            // The pane collapses with a compact overlay of no width (PaneCollapsed).
            return SplitViewDisplayMode::CompactOverlay;
        }
        if self.is_top_navigation_view() {
            return SplitViewDisplayMode::Overlay;
        }
        match self.visual_state_display_mode() {
            VisualStateDisplayMode::Compact => SplitViewDisplayMode::CompactOverlay,
            VisualStateDisplayMode::Expanded => SplitViewDisplayMode::CompactInline,
            _ => SplitViewDisplayMode::Overlay,
        }
    }

    fn is_compact_split_mode(&self) -> bool {
        matches!(self.split_display_mode(), SplitViewDisplayMode::CompactOverlay | SplitViewDisplayMode::CompactInline)
    }

    /// `IsOverlay`: the split view is in the overlay mode.
    fn is_overlay(&self) -> bool {
        self.split_display_mode() == SplitViewDisplayMode::Overlay
    }

    /// `GetVisualStateDisplayMode`.
    fn visual_state_display_mode(&self) -> VisualStateDisplayMode {
        let pane_display_mode = self.pane_display_mode();
        let display_mode = self.display_mode();
        if self.is_top_navigation_view() {
            return VisualStateDisplayMode::Minimal;
        }
        if pane_display_mode == NavigationViewPaneDisplayMode::Left
            || pane_display_mode == NavigationViewPaneDisplayMode::Auto && display_mode == NavigationViewDisplayMode::Expanded
        {
            return VisualStateDisplayMode::Expanded;
        }
        if pane_display_mode == NavigationViewPaneDisplayMode::LeftCompact
            || pane_display_mode == NavigationViewPaneDisplayMode::Auto && display_mode == NavigationViewDisplayMode::Compact
        {
            return VisualStateDisplayMode::Compact;
        }
        if self.should_show_back_button() || self.should_show_close_button() {
            VisualStateDisplayMode::MinimalWithBackButton
        } else {
            VisualStateDisplayMode::Minimal
        }
    }

    /// `ShouldShowBackOrCloseButton`: `Visible`, or `Auto` (WinUI hides it on Xbox only).
    fn should_show_back_or_close_button(&self) -> bool {
        self.is_back_button_visible() != NavigationViewBackButtonVisible::Collapsed
    }

    /// `ShouldShowBackButton`.
    fn should_show_back_button(&self) -> bool {
        if self.parts.borrow().back.is_none() {
            return false;
        }
        if self.display_mode() == NavigationViewDisplayMode::Minimal && self.is_pane_open() {
            return false;
        }
        self.should_show_back_or_close_button()
    }

    /// `ShouldShowCloseButton`.
    fn should_show_close_button(&self) -> bool {
        {
            let parts = self.parts.borrow();
            if parts.back.is_none() || parts.close.is_none() {
                return false;
            }
        }
        if !self.is_pane_open() {
            return false;
        }
        let pane_display_mode = self.pane_display_mode();
        if pane_display_mode != NavigationViewPaneDisplayMode::LeftMinimal
            && (pane_display_mode != NavigationViewPaneDisplayMode::Auto
                || self.display_mode() != NavigationViewDisplayMode::Minimal)
        {
            return false;
        }
        self.should_show_back_or_close_button()
    }

    /// `UpdateAdaptiveLayout` and `SetDisplayMode`: the display mode from the width and the pane
    /// display mode, and the pane it opens or closes.
    fn update_adaptive_layout(&self, width: f64, forced: bool) {
        if self.parts.borrow().split.is_none() {
            return;
        }
        self.last_width.set(width);
        self.update_open_pane_length(width);
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
        if self.is_top_navigation_view() {
            if !forced {
                self.initial_nonforced_layout.set(false);
            }
            self.set_pane_open_internal(false);
        } else {
            if !forced && self.initial_nonforced_layout.replace(false) && mode != NavigationViewDisplayMode::Expanded {
                self.set_pane_open_internal(false);
            }
            if mode == NavigationViewDisplayMode::Expanded && self.is_pane_visible() && !self.was_force_closed.get() {
                self.set_pane_open_internal(true);
            } else if mode == NavigationViewDisplayMode::Minimal
                || previous == NavigationViewDisplayMode::Expanded && mode == NavigationViewDisplayMode::Compact
            {
                self.set_pane_open_internal(false);
            }
        }
        self.update_parts();
    }

    /// `UpdateOpenPaneLength`: the open pane is at most as wide as the navigation view.
    fn update_open_pane_length(&self, width: f64) {
        let open = finite_length(self.open_pane_length());
        let length = if width > 0.0 && width.is_finite() { width.min(open) } else { open };
        self.template_settings.set(NavigationViewTemplateSettings::open_pane_length_property(), length);
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
        let top = self.is_top_navigation_view();
        let pane_visible = self.is_pane_visible();
        let settings = &self.template_settings;

        // The split view: its display mode and its pane (WinUI binds IsPaneOpen both ways).
        if let Some(split) = &parts.split {
            split.set_display_mode(self.split_display_mode());
            split.set_current_value(SplitView::is_pane_open_property(), self.is_pane_open() && !top && pane_visible);
        }
        // UpdateIsClosedCompact.
        let closed_compact = !self.is_pane_open() && !top && self.is_compact_split_mode();
        self.is_closed_compact.set(closed_compact);
        if !self.initial_list_size_state_set.get() && parts.split.is_some() {
            self.initial_list_size_state_set.set(true);
            self.list_size_compact.set(closed_compact);
        }
        if self.is_pane_open() {
            self.list_size_compact.set(false);
        }

        // UpdatePaneVisibility, UpdatePaneToggleButtonVisibility, UpdatePaneButtonsWidths.
        settings.set(NavigationViewTemplateSettings::top_pane_visibility_property(), pane_visible && top);
        settings.set(NavigationViewTemplateSettings::left_pane_visibility_property(), pane_visible && !top);
        settings.set(
            NavigationViewTemplateSettings::pane_toggle_button_visibility_property(),
            self.is_pane_toggle_button_visible() && !top,
        );
        let compact_length = finite_length(self.compact_pane_length());
        settings.set(NavigationViewTemplateSettings::pane_toggle_button_width_property(), compact_length);
        settings.set(NavigationViewTemplateSettings::smaller_pane_toggle_button_width_property(), (compact_length - 8.0).max(0.0));
        settings.set(
            NavigationViewTemplateSettings::single_selection_follows_focus_property(),
            self.selection_follows_focus() == NavigationViewSelectionFollowsFocus::Enabled,
        );

        // UpdatePaneDisplayMode: SwapPaneHeaderContent and UpdateContentBindingsForPaneDisplayMode.
        let swap = |left: &Option<Ref<ContentControl>>, top_part: &Option<Ref<ContentControl>>, content: Option<BoxedValue>| {
            let (active, inactive) = if top { (top_part, left) } else { (left, top_part) };
            if let Some(inactive) = inactive {
                if inactive.content().is_some() {
                    inactive.set_content(None);
                }
            }
            if let Some(active) = active {
                if !reference_equals(&active.content(), &content) {
                    active.set_content(content);
                }
            }
        };
        swap(&parts.left_pane_header, &parts.top_pane_header, self.pane_header().map(Control::boxed));
        swap(&parts.left_pane_custom, &parts.top_pane_custom, self.pane_custom_content().map(Control::boxed));
        swap(&parts.left_pane_footer, &parts.top_pane_footer, self.pane_footer().map(Control::boxed));
        swap(&parts.left_search, &parts.top_search, self.auto_suggest_box().map(Control::boxed));

        // UpdateRepeaterItemsSource and UpdateFooterRepeaterItemsSource: the containers in the hosts
        // of the pane in use; the settings item is the last footer item.
        let roots = self.roots.borrow().clone();
        for (left_host, top_host, footer) in
            [(&parts.left_menu, &parts.top_menu, false), (&parts.left_footer, &parts.top_footer, true)]
        {
            let mut controls: Vec<Ref<Control>> =
                roots.iter().filter(|entry| entry.footer == footer).map(|entry| entry.container.clone().upcast()).collect();
            if footer {
                controls.push(self.settings_item().upcast());
            }
            let (active, inactive) = if top { (top_host, left_host) } else { (left_host, top_host) };
            if let Some(inactive) = inactive {
                if inactive.children().count() > 0 {
                    inactive.children().clear();
                }
            }
            if let Some(active) = active {
                if active.children().snapshot().as_ref() != &controls {
                    active.children().clear();
                    for control in controls {
                        active.children().add(control);
                    }
                }
            }
        }

        // CreateAndHookEventsToSettings and UpdateSettingsItemToolTip: the settings item has no text
        // in the top pane, and its name as its tool tip unless the left pane is open.
        let settings_item = self.settings_item();
        settings_item.set_is_visible(self.is_settings_visible());
        let settings_content: Option<BoxedValue> = if top { None } else { Some(Rc::new(SETTINGS_NAME.to_owned())) };
        if settings_item.content().is_some() != settings_content.is_some() {
            settings_item.set_content(settings_content);
        }
        let settings_tip: Option<BoxedValue> =
            if !top && self.is_pane_open() { None } else { Some(Rc::new(SETTINGS_NAME.to_owned())) };
        if ToolTip::get_tip(&settings_item).is_some() != settings_tip.is_some() {
            ToolTip::set_tip(&settings_item, settings_tip);
        }

        // The state of the items in the pane.
        let mut themes = Vec::new();
        for entry in self.entries() {
            entry.container.set_top_navigation(top && entry.ancestors.is_empty());
            if entry.ancestors.is_empty() {
                entry.container.set_navigation_context(compact_length, closed_compact, 0);
                entry.container.set_pane_open(self.is_pane_open() && !top);
            }
            if let Some(theme) = self.menu_item_container_style() {
                themes.push((entry.container.clone().upcast::<TemplatedControl>(), theme));
            }
        }
        self.apply_container_themes(themes);

        // UpdateBackAndCloseButtonsVisibility.
        let show_back = self.should_show_back_button();
        let show_close = self.should_show_close_button();
        let visual_state = self.visual_state_display_mode();
        let use_left_padding = visual_state == VisualStateDisplayMode::Minimal && !top
            || visual_state == VisualStateDisplayMode::MinimalWithBackButton;
        settings.set(NavigationViewTemplateSettings::back_button_visibility_property(), show_back);
        let toggle_visible = self.is_pane_toggle_button_visible() && parts.toggle.is_some();
        let toggle_width = settings.pane_toggle_button_width();
        let mut left_padding = if toggle_visible && use_left_padding { toggle_width } else { 0.0 };
        if use_left_padding && show_back {
            left_padding += parts.back.as_ref().map(|back| back.width()).filter(|width| width.is_finite()).unwrap_or(BACK_BUTTON_WIDTH);
        }
        let mut header_padding_for_close = 0.0;
        if let Some(close) = &parts.close {
            close.set_is_visible(show_close);
            if show_close && use_left_padding {
                header_padding_for_close = close.width().is_finite().then(|| close.width()).unwrap_or(BACK_BUTTON_WIDTH);
                left_padding += header_padding_for_close;
            }
        }
        if let Some(padding) = &parts.content_left_padding {
            padding.set_width(left_padding);
        }
        // PaneHeaderToggleButtonColumn and PaneHeaderCloseButtonColumn: the pane header beside the
        // toggle and the close button.
        if let Some(header) = &parts.left_pane_header {
            let offset = if toggle_visible { toggle_width } else { 0.0 } + header_padding_for_close;
            let old = header.margin();
            header.set_margin(Thickness::new(offset, old.top, old.right, old.bottom));
        }

        // UpdateTitleBarPadding: the toggle button (or the pane title holder) below the back button,
        // or beside it in the overlay mode.
        let title_holder_visible = !self.update_pane_title_parents(&parts, top);
        // In the overlay mode the themes of the port place the toggle button (and the pane title
        // holder) in the second column of ButtonHolderGrid, beside the back or close button whatever
        // its width (NVW-17), where WinUI moves them right by c_backButtonWidth.
        let thickness = if show_back && !self.is_overlay() {
            Thickness::new(0.0, BACK_BUTTON_HEIGHT, 0.0, 0.0)
        } else {
            Thickness::new(0.0, 0.0, 0.0, 0.0)
        };
        if title_holder_visible {
            if let Some(holder) = &parts.pane_title_holder {
                holder.set_margin(thickness);
            }
        } else if let Some(toggle) = parts.toggle.as_ref().filter(|toggle| toggle.is_visible()) {
            toggle.set_margin(thickness);
        }
        // UpdatePaneToggleSize: the toggle button spans the open pane when the pane has a title.
        if let Some(toggle) = &parts.toggle {
            let open_length = settings.open_pane_length();
            let mut toggle_button_width = toggle_width;
            if !closed_compact && !self.pane_title().is_empty() {
                if self.is_overlay() && self.is_pane_open() {
                    toggle_button_width = open_length - if show_back || show_close { BACK_BUTTON_WIDTH } else { 0.0 };
                } else if !(self.is_overlay() && !self.is_pane_open()) {
                    toggle_button_width = open_length;
                }
            }
            toggle.set_width(toggle_button_width.max(0.0));
            // SetPaneToggleButtonAutomationName.
            AutomationProperties::set_name(
                toggle,
                Some(if self.is_pane_open() { NAVIGATION_OPEN_NAME } else { NAVIGATION_CLOSED_NAME }.into()),
            );
        }

        // The visual states.
        let classes = self.pseudo_classes();
        classes.set(":compact", visual_state == VisualStateDisplayMode::Compact);
        classes.set(":expanded", visual_state == VisualStateDisplayMode::Expanded);
        classes.set(":minimal", visual_state == VisualStateDisplayMode::Minimal && !top);
        classes.set(":top-navigation-minimal", top);
        classes.set(":minimal-with-back-button", visual_state == VisualStateDisplayMode::MinimalWithBackButton);
        classes.set(
            ":toggle-pane-button-visible",
            !top && (self.is_pane_toggle_button_visible() || !self.is_left_pane_title_empty.get()),
        );
        // UpdateHeaderVisibility.
        let show_header = self.header().is_some()
            && (self.always_show_header() || !top && self.display_mode() == NavigationViewDisplayMode::Minimal);
        classes.set(":header-collapsed", !show_header);
        classes.set(":auto-suggest-box-collapsed", self.auto_suggest_box().is_none());
        classes.set(":closed-compact", closed_compact);
        classes.set(":list-size-compact", self.list_size_compact.get() && !top);
        // UpdatePaneOverlayGroup.
        let overlaying = self.is_pane_open()
            && matches!(self.split_display_mode(), SplitViewDisplayMode::CompactOverlay | SplitViewDisplayMode::Overlay);
        classes.set(":pane-not-overlaying", !overlaying);
        classes.set(":back-button-collapsed", !show_back);
        classes.set(":pane-collapsed", !pane_visible);
        // The row of the back button of PaneContentGrid (c_backButtonRowDefinition): its height when
        // the back button is shown and the pane is not an overlay.
        classes.set(":back-button-row-visible", !self.is_overlay() && show_back && !top);
    }

    /// `UpdatePaneTitleFrameworkElementParents`: the pane title in the toggle button, in the pane
    /// title presenter (no toggle button) or in the top pane. Returns whether the left pane title is
    /// empty (the holder hidden).
    fn update_pane_title_parents(&self, parts: &Parts, top: bool) -> bool {
        let toggle_visible = self.is_pane_toggle_button_visible();
        let title_empty = toggle_visible
            || top
            || self.pane_title().is_empty()
            || self.pane_display_mode() == NavigationViewPaneDisplayMode::LeftMinimal && !self.is_pane_open();
        self.is_left_pane_title_empty.set(title_empty);
        if let Some(holder) = &parts.pane_title_holder {
            holder.set_is_visible(!title_empty);
        }
        let Some(title) = &parts.pane_title else { return title_empty };
        let title_value = Some(Control::boxed(title.clone()));
        let [toggle, presenter, top_title] = parts.pane_title_parents();
        let should_not_contain = [top || !toggle_visible, top || toggle_visible, !top || toggle_visible];
        let mut pending = None;
        for (parent, should_not) in [toggle, presenter, top_title.clone()].into_iter().zip(should_not_contain) {
            let Some(parent) = parent else { continue };
            let contains = reference_equals(&parent.content(), &title_value);
            if contains == should_not {
                if should_not {
                    parent.set_content(None);
                } else if pending.is_none() {
                    pending = Some(parent);
                }
            }
        }
        let third_pending = pending.as_ref().zip(top_title.as_ref()).is_some_and(|(pending, top_title)| pending == top_title);
        if let Some(parent) = pending {
            parent.set_content(title_value.clone());
        }
        if let Some(top_title) = &top_title {
            let in_top = reference_equals(&top_title.content(), &title_value);
            top_title.set_is_visible((third_pending || in_top) && !self.pane_title().is_empty());
        }
        title_empty
    }

    fn apply_container_themes(&self, desired: Vec<(Ref<TemplatedControl>, Ref<ControlTheme>)>) {
        let mut previous = self.applied_themes.replace(Vec::new());
        let mut applied = Vec::new();
        for (target, theme) in desired {
            if let Some(index) = previous.iter().position(|entry| {
                entry.target.upgrade().is_some_and(|old| old.ptr_eq(&target)) && entry.theme == theme
            }) {
                applied.push(previous.remove(index));
            } else if let Some(value) =
                target.set_value_with_priority(StyledElement::theme_property(), Some(theme.clone()), BindingPriority::Style)
            {
                applied.push(AppliedTheme { target: target.downgrade(), theme, value });
            }
        }
        for obsolete in previous {
            obsolete.value.dispose();
        }
        *self.applied_themes.borrow_mut() = applied;
    }
}

/// `NavigationViewVisualStateDisplayMode`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum VisualStateDisplayMode {
    Compact,
    Expanded,
    Minimal,
    MinimalWithBackButton,
}

fn finite_length(value: f64) -> f64 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
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
        for theme in self.applied_themes.get_mut().drain(..) {
            theme.value.dispose();
        }
        self.parts.get_mut().detach();
    }
}

impl NavigationView {
    fn reset_source(&self, footer: bool) {
        let slot = if footer { &self.footer_view } else { &self.menu_view };
        if let Some((view, token)) = slot.borrow_mut().take() {
            view.remove_collection_changed(token);
        }
        let source = if footer { self.footer_menu_items_source() } else { self.menu_items_source() };
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

    /// `NavigationViewItemsFactory::GetElement`: a container item is its own container; a data item
    /// gets the container its template builds, or an item with the built element or the data as
    /// content.
    fn build_container(&self, data: &Option<BoxedValue>) -> Ref<NavigationViewItemBase> {
        if let Some(container) =
            data.as_ref().and_then(Control::from_boxed).and_then(|control| control.cast::<NavigationViewItemBase>())
        {
            return container;
        }
        let built = self.menu_item_template().and_then(|template| template.build(data));
        if let Some(container) = built.as_ref().and_then(|control| control.cast::<NavigationViewItemBase>()) {
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
                let slot = if footer { &self.footer_view } else { &self.menu_view };
                let view = slot.borrow().as_ref().map(|(view, _)| view.clone());
                let data = view.map(|view| view.to_vec()).unwrap_or_default();
                for item in data {
                    let reuse = if force {
                        None
                    } else {
                        available.iter().position(|old| old.footer == footer && reference_equals(&old.data, &item))
                    };
                    let container =
                        if let Some(index) = reuse { available.remove(index).container } else { self.build_container(&item) };
                    if let Some(nvi) = container.cast::<NavigationViewItem>() {
                        nvi.set_menu_item_template(self.menu_item_template());
                    }
                    roots.push(Entry { data: item, container, ancestors: Vec::new(), footer });
                }
            }
            *self.roots.borrow_mut() = roots;
            let entries = self.entries();
            *self.known_entries.borrow_mut() = entries.clone();
            let was_syncing = self.syncing_selection.replace(true);
            for removed in previous_known
                .iter()
                .filter(|old| !entries.iter().any(|entry| entry.container.ptr_eq(&old.container)))
            {
                removed.container.set_current_value(NavigationViewItemBase::is_selected_property(), false);
                if let Some(item) = removed.container.cast::<NavigationViewItem>() {
                    item.set_current_value(NavigationViewItem::is_child_selected_property(), false);
                }
            }
            self.syncing_selection.set(was_syncing);
            for entry in entries {
                self.subscribe_item(&entry);
            }
            // UpdateSelectionForMenuItems: an item with IsSelected is the selection when there is no
            // selected item.
            let selected = self.selected_item();
            if selected.is_some()
                && self.container_from_menu_item(&selected).is_none()
                && previous_known.iter().any(|entry| reference_equals(&entry.data, &selected))
            {
                self.set_current_value(Self::selected_item_property(), None);
            } else if selected.is_none() {
                if let Some(entry) = self.entries().into_iter().find(|entry| entry.container.is_selected()) {
                    self.set_current_value(Self::selected_item_property(), entry.data);
                }
            }
            self.synchronize_selection();
            self.update_parts();
            if self.pending_selection.get() && self.container_from_menu_item(&self.selected_item()).is_some() {
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
            if result.iter().any(|known| known.container.ptr_eq(&entry.container)) {
                return;
            }
            let children =
                entry.container.cast::<NavigationViewItem>().map(|item| item.child_containers()).unwrap_or_default();
            let mut ancestors = entry.ancestors.clone();
            ancestors.push(entry.container.clone());
            let footer = entry.footer;
            result.push(entry);
            for (data, container) in children {
                append(Entry { data, container, ancestors: ancestors.clone(), footer }, result);
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

    /// `SetNavigationViewItemRevokers`: the selection, the expansion, the focus, the keys, the
    /// invocation and the children of each realized container.
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
            } else if change.property() == NavigationViewItem::is_expanded_property().as_property() {
                if let Some(item) = container.cast::<NavigationViewItem>() {
                    owner.item_expansion_changed(&item);
                }
            }
        });
        self.item_subscriptions.borrow_mut().push(property);
        if let Some(item) = entry.container.cast::<NavigationViewItem>() {
            // Each realized container is observed itself: the children in a popup have a route of
            // their own that need not pass the navigation view. The handlers hold the owner and the
            // item weakly, and are disposed with the other subscriptions.
            let weak = self.to_ref().downgrade();
            let weak_item = item.downgrade();
            let focus = item.add_disposable_handler(InputElement::got_focus_event(), move |_, args| {
                let (Some(owner), Some(item)) = (weak.upgrade(), weak_item.upgrade()) else {
                    return;
                };
                if owner.selection_follows_focus() != NavigationViewSelectionFollowsFocus::Enabled
                    || !item.selects_on_invoked()
                    || item.is_selected()
                    || !args
                        .new_focused_element
                        .as_ref()
                        .is_some_and(|focused| focused.ptr_eq(&item.clone().upcast::<InputElement>()))
                {
                    return;
                }
                // An ancestor item gets the bubbling event too: only the focused item is invoked.
                owner.item_invoked_core(&item);
            });
            let weak = self.to_ref().downgrade();
            let weak_item = item.downgrade();
            let keyboard = item.add_disposable_handler(InputElement::key_down_event(), move |_, args| {
                let (Some(owner), Some(item)) = (weak.upgrade(), weak_item.upgrade()) else {
                    return;
                };
                if args.source().is_some_and(|source| source.ptr_eq(&item.to_ref())) {
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
            self.item_subscriptions.borrow_mut().extend([focus, keyboard, invoked, items]);
        }
    }

    /// `OnNavigationViewItemKeyDown`: Home, End and the arrows between the items of the pane, or of
    /// the flyout of an expanded item.
    fn item_key_down(&self, item: &NavigationViewItem, args: &KeyEventArgs) {
        if args.handled() || args.key_modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) {
            return;
        }
        let entries = self.entries();
        let Some(current) = entries.iter().find(|entry| entry.container.ptr_eq(&item.to_ref())) else {
            return;
        };
        let top = self.is_top_navigation_view();
        let flyout_mode = top
            || self.is_pane_visible() && !self.is_pane_open() && self.display_mode() != NavigationViewDisplayMode::Minimal;
        let flyout_root = if flyout_mode {
            current.ancestors.first().cloned().or_else(|| item.is_expanded().then(|| current.container.clone()))
        } else {
            None
        };
        let can_focus = |entry: &&Entry| {
            entry.container.cast::<NavigationViewItem>().is_some()
                && entry.container.focusable()
                && entry.container.is_effectively_visible()
                && entry.container.is_effectively_enabled()
                && entry
                    .ancestors
                    .iter()
                    .all(|ancestor| ancestor.cast::<NavigationViewItem>().is_some_and(|item| item.is_expanded()))
        };
        if matches!(args.key, Key::Home | Key::End) {
            // WinUI moves within the repeater of the pane or of the open flyout; the expanded
            // descendants are reached by Up and Down only.
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
            let target = if args.key == Key::Home { roots.first() } else { roots.last() };
            args.set_handled(true);
            if let Some(target) = target {
                target.container.focus_with(NavigationMethod::Directional, args.key_modifiers);
            }
            return;
        }
        let horizontal = top && current.ancestors.is_empty() && matches!(args.key, Key::Left | Key::Right);
        let rtl = item.flow_direction() == FlowDirection::RightToLeft;
        let forward = if horizontal {
            if rtl {
                Key::Left
            } else {
                Key::Right
            }
        } else {
            Key::Down
        };
        let backward = if horizontal {
            if rtl {
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
                    entry.ancestors.first().is_some_and(|ancestor| ancestor.ptr_eq(root))
                } else {
                    !top
                }
            })
            .collect();
        let index = candidates.iter().position(|entry| entry.container.ptr_eq(&current.container));
        let target = if let Some(index) = index {
            if args.key == forward {
                candidates.get(index + 1)
            } else {
                index.checked_sub(1).and_then(|index| candidates.get(index))
            }
        } else if args.key == Key::Down && flyout_root.as_ref().is_some_and(|root| root.ptr_eq(&current.container)) {
            // Down on an expanded item of the top pane or of a compact pane enters its flyout.
            candidates.first()
        } else {
            None
        };
        if let Some(target) = target {
            args.set_handled(target.container.focus_with(NavigationMethod::Directional, args.key_modifiers));
        }
    }

    /// Gets the realized container of a menu item, a footer item or a nested item
    /// (`ContainerFromMenuItem`).
    pub fn container_from_menu_item(&self, item: &Option<BoxedValue>) -> Option<Ref<NavigationViewItemBase>> {
        if item.is_none() {
            return None;
        }
        if self.is_settings_item(item) {
            return Some(self.settings_item().upcast());
        }
        self.entries().into_iter().find(|entry| reference_equals(&entry.data, item)).map(|entry| entry.container)
    }

    /// Gets the menu item of a container (`MenuItemFromContainer`).
    pub fn menu_item_from_container(&self, container: &FerroObject) -> Option<BoxedValue> {
        self.entries().into_iter().find(|entry| entry.container.ptr_eq(&container.to_ref())).and_then(|entry| entry.data)
    }

    /// Expands an item that has children (`Expand`).
    pub fn expand(&self, item: &NavigationViewItem) {
        if item.has_children_for_navigation() {
            item.set_is_expanded(true);
        }
    }

    /// Collapses an item that has children (`Collapse`).
    pub fn collapse(&self, item: &NavigationViewItem) {
        if item.has_children_for_navigation() {
            item.set_is_expanded(false);
        }
    }

    fn is_settings_item(&self, item: &Option<BoxedValue>) -> bool {
        reference_equals(item, &Some(Control::boxed(self.settings_item())))
            || item.is_some() && reference_equals(item, &self.settings_item.content())
    }

    /// The selection on the containers: `IsSelected` of the selected container, `IsChildSelected`
    /// of its ancestors.
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
                    selected_container.as_ref().is_some_and(|selected| selected.ptr_eq(&entry.container)),
                );
                if let Some(item) = entry.container.cast::<NavigationViewItem>() {
                    item.set_current_value(
                        NavigationViewItem::is_child_selected_property(),
                        ancestors.iter().any(|ancestor| ancestor.ptr_eq(&entry.container)),
                    );
                }
            }
            if reference_equals(&selected, &self.selected_item()) {
                break;
            }
        }
        self.syncing_selection.set(was_syncing);
    }

    /// `OnSelectedItemPropertyChanged`.
    fn selection_property_changed(&self, old: Option<BoxedValue>, next: Option<BoxedValue>) {
        let was_pending = self.pending_selection.replace(false);
        self.pending_selection_revision.set(self.pending_selection_revision.get().wrapping_add(1));
        if self.syncing_selection.get() {
            return;
        }
        if let Some(container) = self.container_from_menu_item(&next) {
            if container.cast::<NavigationViewItem>().is_some_and(|item| !item.selects_on_invoked()) {
                // An item that does not select on invocation is invoked, and the selection stays.
                self.syncing_selection.set(true);
                self.set_current_value(Self::selected_item_property(), old.clone());
                self.syncing_selection.set(false);
                self.synchronize_selection();
                self.pending_invocation.borrow_mut().take();
                self.raise_item_invoked(&container);
                // A suppressed action does not cancel an earlier selection still waiting for its
                // container; a handler may redirect it during the invocation.
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
        // The application may redirect the selection from ItemInvoked.
        if !reference_equals(&next, &self.selected_item()) {
            return;
        }
        self.synchronize_selection();
        let next = self.selected_item();
        self.pending_selection.set(next.is_some() && self.container_from_menu_item(&next).is_none());
        if self.pending_selection.get() {
            self.defer_selection_changed();
        } else {
            self.raise_selection_changed();
        }
        if let Some(item) =
            self.container_from_menu_item(&next).and_then(|container| container.cast::<NavigationViewItem>())
        {
            self.close_after_item(&item);
        }
    }

    fn defer_selection_changed(&self) {
        // WinUI completes a selection whose container is not realized at a low priority of the
        // dispatcher; a newer selection or a realization invalidates the callback, which holds the
        // owner weakly.
        let revision = self.pending_selection_revision.get();
        let weak = self.to_ref().downgrade();
        Dispatcher::ui_thread().post_local(
            move || {
                if let Some(owner) = weak.upgrade() {
                    if owner.pending_selection_revision.get() == revision && owner.pending_selection.replace(false) {
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
        if let Some(peer) =
            ControlAutomationPeer::from_element(self).and_then(|peer| peer.cast::<NavigationViewAutomationPeer>())
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

    /// `OnNavigationViewItemInvoked`: select the item (or raise `ItemInvoked`), toggle its
    /// expansion, close the pane that is not inline, and collapse the flyout of its top level
    /// ancestor.
    fn item_invoked_core(&self, item: &NavigationViewItem) {
        if !item.is_effectively_enabled() {
            return;
        }
        let container: Ref<NavigationViewItemBase> = item.to_ref().upcast();
        let Some(entry) = self.entries().into_iter().find(|entry| entry.container.ptr_eq(&container)) else {
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
            if let Some(entry) = self.entries().into_iter().find(|entry| entry.container.ptr_eq(&item.to_ref())) {
                if let Some(root) = entry.ancestors.first().and_then(|ancestor| ancestor.cast::<NavigationViewItem>()) {
                    if self.is_top_navigation_view()
                        || self.display_mode() != NavigationViewDisplayMode::Minimal && !self.is_pane_open()
                    {
                        root.set_is_expanded(false);
                    }
                }
            }
        }
    }

    /// `ClosePaneIfNeccessaryAfterItemIsClicked`.
    fn close_after_item(&self, item: &NavigationViewItem) {
        if self.is_pane_open() && self.display_mode() != NavigationViewDisplayMode::Expanded && !item.has_children_for_navigation()
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

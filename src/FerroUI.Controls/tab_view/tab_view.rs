//! WinUI's `TabView` (`controls/dev/TabView/TabView.cpp`, `TabView.h` and `TabView.idl` of
//! microsoft-ui-xaml commit `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c)
//! Microsoft Corporation, see NOTICE.md).

use super::{TabViewItem, TabViewListView};
use crate::automation::peers::{AutomationPeer, TabViewAutomationPeer};
use crate::automation::AutomationProperties;
use crate::primitives::{
    ScrollBarVisibility, SelectingItemsControl, TemplateAppliedEventArgs, TemplatedControl, TemplatedControlImpl,
    TemplatedControlImplExt,
};
use crate::presenters::ContentPresenter;
use crate::templates::IDataTemplate;
use crate::{
    Button, ColumnDefinition, Control, ControlImpl, GridLength, GridUnitType, ItemsChangedEventArgs,
    ItemsSource, ListViewItem, RepeatButton, ScrollViewer, SelectionChangedEventArgs, ToolTip,
};
use ferroui_base::collections::{FerroList, NotifyCollectionChangedAction};
use ferroui_base::input::{
    FocusManager, ICommand, InputElement, InputElementImpl, InputElementImplExt, Key, KeyEventArgs, KeyModifiers,
    NavigationMethod,
};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{LayoutableImpl, LayoutableImplExt};
use ferroui_base::media::FlowDirection;
use ferroui_base::metadata::{into_markup_value, MarkupDelegate};
use ferroui_base::reactive::{Disposable, Subscription};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_markup_enum, ferro_markup_type, ferro_properties,
    ferro_property, instantiate, BoxedValue, DirectProperty, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, Ref, Size, StyledElementImpl, StyledProperty, VisualImpl, WeakRef,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// How wide the tabs of a [`TabView`] are (WinUI's `TabViewWidthMode`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum TabViewWidthMode {
    /// All tabs have the same width, between `TabViewItemMinWidth` and `TabViewItemMaxWidth`.
    #[default]
    Equal = 0,
    /// Each tab is as wide as its header.
    SizeToContent = 1,
    /// The tabs that are not selected show their icon only.
    Compact = 2,
}

/// When the close button of a tab shows (WinUI's `TabViewCloseButtonOverlayMode`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum TabViewCloseButtonOverlayMode {
    /// As `Always` (the default).
    #[default]
    Auto = 0,
    /// On the selected tab and the tab under the pointer.
    OnPointerOver = 1,
    /// On every tab.
    Always = 2,
}

ferro_markup_enum!(TabViewWidthMode { Equal, SizeToContent, Compact }, { namespace: "FerroUI.Controls" });
ferro_markup_enum!(
    TabViewCloseButtonOverlayMode { Auto, OnPointerOver, Always },
    { namespace: "FerroUI.Controls" }
);

/// The arguments of [`TabView::tab_close_requested`] and [`TabViewItem::close_requested`]
/// (WinUI's `TabViewTabCloseRequestedEventArgs`).
#[derive(Clone)]
pub struct TabViewTabCloseRequestedEventArgs {
    item: Option<BoxedValue>,
    tab: Ref<TabViewItem>,
}

impl TabViewTabCloseRequestedEventArgs {
    pub(crate) fn new(item: Option<BoxedValue>, tab: Ref<TabViewItem>) -> Self {
        Self { item, tab }
    }

    /// The item of the tab: the tab itself, or the item of `TabItemsSource` it shows.
    pub fn item(&self) -> Option<BoxedValue> {
        self.item.clone()
    }

    /// The tab whose close was requested.
    pub fn tab(&self) -> Ref<TabViewItem> {
        self.tab.clone()
    }
}

impl PartialEq for TabViewTabCloseRequestedEventArgs {
    fn eq(&self, other: &Self) -> bool {
        self.tab == other.tab
    }
}

ferro_markup_type!(class TabViewTabCloseRequestedEventArgs {
    namespace: "FerroUI.Controls",
    handles: [TabViewTabCloseRequestedEventArgs],
    properties: [
        Item: Option<BoxedValue> { get: |args: &TabViewTabCloseRequestedEventArgs| args.item() },
        Tab: Ref<TabViewItem> { get: |args: &TabViewTabCloseRequestedEventArgs| args.tab() },
    ],
});

// The texts WinUI takes from its localized resources (`controls/dev/TabView/Strings/en-us/
// Resources.resw`), in English: FerroUI has no localized resources of its framework.
const ADD_BUTTON_NAME: &str = "Add New Tab";
const ADD_BUTTON_TOOLTIP: &str = "Add new tab";
pub(super) const CLOSE_BUTTON_NAME: &str = "Close Tab";
pub(super) const CLOSE_BUTTON_TOOLTIP_WITH_ACCELERATOR: &str = "Close tab (Ctrl+F4)";
const SCROLL_DECREASE_BUTTON_TOOLTIP: &str = "Scroll tab list backward";
const SCROLL_INCREASE_BUTTON_TOOLTIP: &str = "Scroll tab list forward";

/// The width limits of a tab when the theme has no `TabViewItemMinWidth` and `TabViewItemMaxWidth`.
const TAB_MINIMUM_WIDTH: f64 = 48.0;
const TAB_MAXIMUM_WIDTH: f64 = 200.0;
const TAB_VIEW_ITEM_MIN_WIDTH_KEY: &str = "TabViewItemMinWidth";
const TAB_VIEW_ITEM_MAX_WIDTH_KEY: &str = "TabViewItemMaxWidth";
/// How far a scroll button scrolls the tabs.
const SCROLL_AMOUNT: f64 = 50.0;

const PC_SINGLE_BOTTOM_BORDER_LINE: &str = ":single-bottom-border-line";

/// The parts of the template, looked up when it is applied.
#[derive(Default)]
struct TabViewParts {
    tab_content_presenter: Option<WeakRef<ContentPresenter>>,
    right_content_presenter: Option<WeakRef<ContentPresenter>>,
    left_content_column: Option<WeakRef<ColumnDefinition>>,
    tab_column: Option<WeakRef<ColumnDefinition>>,
    add_button_column: Option<WeakRef<ColumnDefinition>>,
    right_content_column: Option<WeakRef<ColumnDefinition>>,
    list_view: Option<WeakRef<TabViewListView>>,
    add_button: Option<WeakRef<Button>>,
    scroll_viewer: Option<WeakRef<ScrollViewer>>,
    scroll_decrease_button: Option<WeakRef<RepeatButton>>,
    scroll_increase_button: Option<WeakRef<RepeatButton>>,
    /// What removes the handlers of the parts, run when the template is replaced.
    cleanup: Vec<Box<dyn FnOnce()>>,
}

/// A row of tabs over the content of the selected one (WinUI's `TabView`): the tabs are a
/// [`TabViewListView`] of [`TabViewItem`]s, from [`tab_items`](Self::tab_items) or
/// [`tab_items_source`](Self::tab_items_source); the selected tab's content is shown by the
/// template part `TabContentPresenter`.
///
/// The control raises WinUI's events: [`tab_close_requested`](Self::tab_close_requested) when a
/// tab's close button, a middle click or Ctrl+F4 asks to close a tab (the application removes it),
/// [`add_tab_button_click`](Self::add_tab_button_click), [`selection_changed`](Self::selection_changed)
/// and [`tab_items_changed`](Self::tab_items_changed). Ctrl+Tab and Ctrl+Shift+Tab select the next
/// and the previous tab. With [`TabViewWidthMode::Equal`] the tabs share the width of the strip
/// between `TabViewItemMinWidth` and `TabViewItemMaxWidth` (`UpdateTabWidths`), and the strip
/// scrolls with the buttons `ScrollDecreaseButton` and `ScrollIncreaseButton` of its scroll viewer
/// when they do not fit.
///
/// Differences from WinUI: dragging, reordering, dropping and tearing tabs out of the window
/// (`CanDragTabs`, `CanReorderTabs`, `AllowDropTabs`, `CanTearOutTabs` and their events) are not
/// ported; the keyboard shortcuts are handled in `on_key_down` rather than keyboard accelerators;
/// UI Automation notifications are not raised.
#[repr(C)]
pub struct TabView {
    base: TemplatedControl,
    tab_items: FerroList<Option<BoxedValue>>,
    parts: RefCell<TabViewParts>,
    previous_available_width: Cell<f64>,
    pointer_in_tab_strip: Cell<bool>,
    update_tab_width_on_pointer_leave: Cell<bool>,
    /// Set while the selection of the list is copied to the properties, so that the copy does not
    /// select in the list again.
    syncing_selection: Cell<bool>,
    tab_close_requested: HandlerList<dyn Fn(&Ref<TabView>, &TabViewTabCloseRequestedEventArgs)>,
    add_tab_button_click: HandlerList<dyn Fn(&Ref<TabView>)>,
    selection_changed: HandlerList<dyn Fn(&Ref<TabView>, &SelectionChangedEventArgs)>,
    tab_items_changed: HandlerList<dyn Fn(&Ref<TabView>, &ItemsChangedEventArgs<'_>)>,
}

ferro_class!(TabView: TemplatedControl);
ferro_class_info!(TabView {
    new: TabView::new,
    markup: {
        namespace: "FerroUI.Controls",
        content: TabItems,
        properties: [
            TabItems: FerroList<Option<BoxedValue>> { get: TabView::tab_items },
        ],
        events: [
            TabCloseRequested(Option<BoxedValue>, TabViewTabCloseRequestedEventArgs) =>
                |this: &Ref<TabView>, handler: MarkupDelegate| {
                    this.tab_close_requested(move |sender, args| {
                        handler.invoke(&[into_markup_value(Some(sender.clone())), into_markup_value(args.clone())]);
                    })
                },
            AddTabButtonClick(Option<BoxedValue>, Option<BoxedValue>) => |this: &Ref<TabView>, handler: MarkupDelegate| {
                this.add_tab_button_click(move |sender| {
                    handler.invoke(&[into_markup_value(Some(sender.clone())), into_markup_value(None::<BoxedValue>)]);
                })
            },
        ],
        attributes: [
            TemplatePart("TabListView", type(Ref<TabViewListView>)),
            TemplatePart("AddButton", type(Ref<Button>)),
            TemplatePart("TabContentPresenter", type(Ref<ContentPresenter>)),
            TemplatePart("RightContentPresenter", type(Ref<ContentPresenter>)),
            PseudoClasses(":single-bottom-border-line"),
        ],
    },
});
ferro_impl_classes!(TabView: StyledElementImpl, VisualImpl, InteractiveImpl);

ferro_properties! { impl TabView {
    ferro_property!(
        /// Defines the `TabWidthMode` property.
        pub fn tab_width_mode_property() -> StyledProperty<TabViewWidthMode> {
            FerroProperty::register::<TabView, _>("TabWidthMode", TabViewWidthMode::Equal)
        }
    );

    ferro_property!(
        /// Defines the `CloseButtonOverlayMode` property.
        pub fn close_button_overlay_mode_property() -> StyledProperty<TabViewCloseButtonOverlayMode> {
            FerroProperty::register::<TabView, _>("CloseButtonOverlayMode", TabViewCloseButtonOverlayMode::Auto)
        }
    );

    ferro_property!(
        /// Defines the `TabStripHeader` property.
        pub fn tab_strip_header_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<TabView, _>("TabStripHeader", None)
        }
    );

    ferro_property!(
        /// Defines the `TabStripHeaderTemplate` property.
        pub fn tab_strip_header_template_property() -> StyledProperty<Option<Rc<dyn IDataTemplate>>> {
            FerroProperty::register::<TabView, _>("TabStripHeaderTemplate", None)
        }
    );

    ferro_property!(
        /// Defines the `TabStripFooter` property.
        pub fn tab_strip_footer_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<TabView, _>("TabStripFooter", None)
        }
    );

    ferro_property!(
        /// Defines the `TabStripFooterTemplate` property.
        pub fn tab_strip_footer_template_property() -> StyledProperty<Option<Rc<dyn IDataTemplate>>> {
            FerroProperty::register::<TabView, _>("TabStripFooterTemplate", None)
        }
    );

    ferro_property!(
        /// Defines the `IsAddTabButtonVisible` property.
        pub fn is_add_tab_button_visible_property() -> StyledProperty<bool> {
            FerroProperty::register::<TabView, _>("IsAddTabButtonVisible", true)
        }
    );

    ferro_property!(
        /// Defines the `AddTabButtonCommand` property.
        pub fn add_tab_button_command_property() -> StyledProperty<Option<Rc<dyn ICommand>>> {
            FerroProperty::register::<TabView, _>("AddTabButtonCommand", None)
        }
    );

    ferro_property!(
        /// Defines the `AddTabButtonCommandParameter` property.
        pub fn add_tab_button_command_parameter_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<TabView, _>("AddTabButtonCommandParameter", None)
        }
    );

    ferro_property!(
        /// Defines the `TabItemsSource` property: the items the tabs show, in place of `TabItems`.
        pub fn tab_items_source_property() -> StyledProperty<Option<ItemsSource>> {
            FerroProperty::register::<TabView, _>("TabItemsSource", None)
        }
    );

    ferro_property!(
        /// Defines the `TabItemTemplate` property: the template of the items of `TabItemsSource`.
        pub fn tab_item_template_property() -> StyledProperty<Option<Rc<dyn IDataTemplate>>> {
            FerroProperty::register::<TabView, _>("TabItemTemplate", None)
        }
    );

    ferro_property!(
        /// Defines the `CanDragTabs` property. Accepted: dragging tabs is not ported.
        pub fn can_drag_tabs_property() -> StyledProperty<bool> {
            FerroProperty::register::<TabView, _>("CanDragTabs", false)
        }
    );

    ferro_property!(
        /// Defines the `CanReorderTabs` property. Accepted: reordering tabs is not ported.
        pub fn can_reorder_tabs_property() -> StyledProperty<bool> {
            FerroProperty::register::<TabView, _>("CanReorderTabs", true)
        }
    );

    ferro_property!(
        /// Defines the `AllowDropTabs` property. Accepted: dropping tabs is not ported.
        pub fn allow_drop_tabs_property() -> StyledProperty<bool> {
            FerroProperty::register::<TabView, _>("AllowDropTabs", true)
        }
    );

    ferro_property!(
        /// Defines the `CanTearOutTabs` property. Accepted: tearing tabs out is not ported.
        pub fn can_tear_out_tabs_property() -> StyledProperty<bool> {
            FerroProperty::register::<TabView, _>("CanTearOutTabs", false)
        }
    );

    ferro_property!(
        /// Defines the `SelectedIndex` property.
        pub fn selected_index_property() -> StyledProperty<i32> {
            FerroProperty::register::<TabView, _>("SelectedIndex", -1)
        }
    );

    ferro_property!(
        /// Defines the `SelectedItem` property.
        pub fn selected_item_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<TabView, _>("SelectedItem", None)
        }
    );

    ferro_property!(
        /// Defines the `TabItems` property: the items of the tabs, when `TabItemsSource` is not set.
        pub fn tab_items_property() -> DirectProperty<TabView, FerroList<Option<BoxedValue>>> {
            FerroProperty::register_direct::<TabView, _>("TabItems", |o| o.tab_items(), None, FerroList::new())
        }
    );
} }

impl FerroObjectImpl for TabView {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if !change.is_effective_value_change() {
            return;
        }
        let property = change.property();
        if property == Self::selected_index_property().as_property() {
            this.on_selected_index_changed(change.get_old_and_new_value::<i32>().0);
        } else if property == Self::selected_item_property().as_property() {
            this.update_selected_item();
        } else if property == Self::tab_width_mode_property().as_property() {
            this.on_tab_width_mode_changed();
        } else if property == Self::close_button_overlay_mode_property().as_property() {
            this.on_close_button_overlay_mode_changed();
        } else if property == Self::tab_items_source_property().as_property() {
            this.update_list_view_items();
        }
    }
}

impl ControlImpl for TabView {
    fn on_create_automation_peer(this: &Self) -> Ref<AutomationPeer> {
        TabViewAutomationPeer::new(this).upcast()
    }
}

impl TemplatedControlImpl for TabView {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        this.unhook_events_and_clear_fields();
        Self::parent_on_apply_template(this, e);
        let scope = e.name_scope();
        let weak = this.to_ref().downgrade();
        let mut parts = TabViewParts {
            tab_content_presenter: scope.find_as::<ContentPresenter>("TabContentPresenter").map(|p| p.downgrade()),
            right_content_presenter: scope.find_as::<ContentPresenter>("RightContentPresenter").map(|p| p.downgrade()),
            left_content_column: scope.find_as::<ColumnDefinition>("LeftContentColumn").map(|c| c.downgrade()),
            tab_column: scope.find_as::<ColumnDefinition>("TabColumn").map(|c| c.downgrade()),
            add_button_column: scope.find_as::<ColumnDefinition>("AddButtonColumn").map(|c| c.downgrade()),
            right_content_column: scope.find_as::<ColumnDefinition>("RightContentColumn").map(|c| c.downgrade()),
            ..TabViewParts::default()
        };
        if let Some(container) = scope.find_as::<Control>("TabContainerGrid") {
            let (entered, exited) = (weak.clone(), weak.clone());
            let entered = container.add_handler(InputElement::pointer_entered_event(), move |_, _| {
                if let Some(this) = entered.upgrade() {
                    this.pointer_in_tab_strip.set(true);
                }
            });
            let exited = container.add_handler(InputElement::pointer_exited_event(), move |_, _| {
                if let Some(this) = exited.upgrade() {
                    this.on_tab_strip_pointer_exited();
                }
            });
            let target = container.downgrade();
            parts.cleanup.push(Box::new(move || {
                if let Some(container) = target.upgrade() {
                    container.remove_handler(InputElement::pointer_entered_event(), entered);
                    container.remove_handler(InputElement::pointer_exited_event(), exited);
                }
            }));
        }
        if let Some(list_view) = scope.find_as::<TabViewListView>("TabListView") {
            let loaded = weak.clone();
            let loaded = list_view.add_handler(Control::loaded_event(), move |_, _| {
                if let Some(this) = loaded.upgrade() {
                    this.on_list_view_loaded();
                }
            });
            let selection = weak.clone();
            let selection = list_view.add_handler(
                SelectingItemsControl::selection_changed_event(),
                move |_, args: &SelectionChangedEventArgs| {
                    if let Some(this) = selection.upgrade() {
                        this.on_list_view_selection_changed(args);
                    }
                },
            );
            let target = list_view.downgrade();
            parts.cleanup.push(Box::new(move || {
                if let Some(list_view) = target.upgrade() {
                    list_view.remove_handler(Control::loaded_event(), loaded);
                    list_view.remove_handler(SelectingItemsControl::selection_changed_event(), selection);
                }
            }));
            parts.list_view = Some(list_view.downgrade());
        }
        if let Some(add_button) = scope.find_as::<Button>("AddButton") {
            if AutomationProperties::get_name(&add_button).is_none_or(|name| name.is_empty()) {
                AutomationProperties::set_name(&add_button, Some(ADD_BUTTON_NAME));
            }
            if ToolTip::get_tip(&add_button).is_none() {
                ToolTip::set_tip(&add_button, Some(Rc::new(ADD_BUTTON_TOOLTIP.to_string()) as BoxedValue));
            }
            let click = weak.clone();
            let click = add_button.click(move |_, _| {
                if let Some(this) = click.upgrade() {
                    this.on_add_button_click();
                }
            });
            let target = add_button.downgrade();
            parts.cleanup.push(Box::new(move || {
                if let Some(button) = target.upgrade() {
                    button.remove_handler(Button::click_event(), click);
                }
            }));
            parts.add_button = Some(add_button.downgrade());
        }
        *this.parts.borrow_mut() = parts;
        this.update_list_view_items();
    }
}

impl LayoutableImpl for TabView {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        if this.previous_available_width.get() != available_size.width {
            this.previous_available_width.set(available_size.width);
            this.update_tab_widths(true, true);
        }
        Self::parent_measure_override(this, available_size)
    }
}

impl InputElementImpl for TabView {
    fn on_key_down(this: &Self, e: &KeyEventArgs) {
        // The keyboard accelerators of WinUI's constructor: Ctrl+F4, Ctrl+Tab and Ctrl+Shift+Tab.
        if !e.handled() && e.key_modifiers.contains(KeyModifiers::CONTROL) {
            let shift = e.key_modifiers.contains(KeyModifiers::SHIFT);
            let handled = match e.key {
                Key::F4 if !shift => this.request_close_current_tab(),
                Key::Tab => this.move_selection(!shift),
                _ => false,
            };
            if handled {
                e.set_handled(true);
                return;
            }
        }
        Self::parent_on_key_down(this, e);
    }
}

impl TabView {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: TemplatedControl::construct(),
            tab_items: FerroList::new(),
            parts: RefCell::new(TabViewParts::default()),
            previous_available_width: Cell::new(f64::NAN),
            pointer_in_tab_strip: Cell::new(false),
            update_tab_width_on_pointer_leave: Cell::new(false),
            syncing_selection: Cell::new(false),
            tab_close_requested: HandlerList::new(),
            add_tab_button_click: HandlerList::new(),
            selection_changed: HandlerList::new(),
            tab_items_changed: HandlerList::new(),
        }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        let weak = this.downgrade();
        this.loaded(move |_, _| {
            if let Some(this) = weak.upgrade() {
                this.update_tab_content();
            }
        });
        this
    }

    // --- properties -------------------------------------------------------------------------

    /// The items of the tabs (WinUI's `TabItems`): `TabViewItem`s, or other items when
    /// [`tab_item_template`](Self::tab_item_template) makes their tabs.
    pub fn tab_items(&self) -> FerroList<Option<BoxedValue>> {
        self.tab_items.clone()
    }

    pub fn tab_width_mode(&self) -> TabViewWidthMode {
        self.get_value(Self::tab_width_mode_property())
    }

    pub fn set_tab_width_mode(&self, value: TabViewWidthMode) {
        self.set_value(Self::tab_width_mode_property(), value)
    }

    pub fn close_button_overlay_mode(&self) -> TabViewCloseButtonOverlayMode {
        self.get_value(Self::close_button_overlay_mode_property())
    }

    pub fn set_close_button_overlay_mode(&self, value: TabViewCloseButtonOverlayMode) {
        self.set_value(Self::close_button_overlay_mode_property(), value)
    }

    pub fn tab_strip_header(&self) -> Option<BoxedValue> {
        self.get_value(Self::tab_strip_header_property())
    }

    pub fn set_tab_strip_header(&self, value: Option<BoxedValue>) {
        self.set_value(Self::tab_strip_header_property(), value)
    }

    pub fn tab_strip_header_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.get_value(Self::tab_strip_header_template_property())
    }

    pub fn set_tab_strip_header_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        self.set_value(Self::tab_strip_header_template_property(), value)
    }

    pub fn tab_strip_footer(&self) -> Option<BoxedValue> {
        self.get_value(Self::tab_strip_footer_property())
    }

    pub fn set_tab_strip_footer(&self, value: Option<BoxedValue>) {
        self.set_value(Self::tab_strip_footer_property(), value)
    }

    pub fn tab_strip_footer_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.get_value(Self::tab_strip_footer_template_property())
    }

    pub fn set_tab_strip_footer_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        self.set_value(Self::tab_strip_footer_template_property(), value)
    }

    pub fn is_add_tab_button_visible(&self) -> bool {
        self.get_value(Self::is_add_tab_button_visible_property())
    }

    pub fn set_is_add_tab_button_visible(&self, value: bool) {
        self.set_value(Self::is_add_tab_button_visible_property(), value)
    }

    pub fn add_tab_button_command(&self) -> Option<Rc<dyn ICommand>> {
        self.get_value(Self::add_tab_button_command_property())
    }

    pub fn set_add_tab_button_command(&self, value: Option<Rc<dyn ICommand>>) {
        self.set_value(Self::add_tab_button_command_property(), value)
    }

    pub fn add_tab_button_command_parameter(&self) -> Option<BoxedValue> {
        self.get_value(Self::add_tab_button_command_parameter_property())
    }

    pub fn set_add_tab_button_command_parameter(&self, value: Option<BoxedValue>) {
        self.set_value(Self::add_tab_button_command_parameter_property(), value)
    }

    pub fn tab_items_source(&self) -> Option<ItemsSource> {
        self.get_value(Self::tab_items_source_property())
    }

    pub fn set_tab_items_source(&self, value: Option<ItemsSource>) {
        self.set_value(Self::tab_items_source_property(), value)
    }

    pub fn tab_item_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.get_value(Self::tab_item_template_property())
    }

    pub fn set_tab_item_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        self.set_value(Self::tab_item_template_property(), value)
    }

    pub fn can_drag_tabs(&self) -> bool {
        self.get_value(Self::can_drag_tabs_property())
    }

    pub fn set_can_drag_tabs(&self, value: bool) {
        self.set_value(Self::can_drag_tabs_property(), value)
    }

    pub fn can_reorder_tabs(&self) -> bool {
        self.get_value(Self::can_reorder_tabs_property())
    }

    pub fn set_can_reorder_tabs(&self, value: bool) {
        self.set_value(Self::can_reorder_tabs_property(), value)
    }

    pub fn allow_drop_tabs(&self) -> bool {
        self.get_value(Self::allow_drop_tabs_property())
    }

    pub fn set_allow_drop_tabs(&self, value: bool) {
        self.set_value(Self::allow_drop_tabs_property(), value)
    }

    pub fn can_tear_out_tabs(&self) -> bool {
        self.get_value(Self::can_tear_out_tabs_property())
    }

    pub fn set_can_tear_out_tabs(&self, value: bool) {
        self.set_value(Self::can_tear_out_tabs_property(), value)
    }

    pub fn selected_index(&self) -> i32 {
        self.get_value(Self::selected_index_property())
    }

    pub fn set_selected_index(&self, value: i32) {
        self.set_value(Self::selected_index_property(), value)
    }

    pub fn selected_item(&self) -> Option<BoxedValue> {
        self.get_value(Self::selected_item_property())
    }

    pub fn set_selected_item(&self, value: Option<BoxedValue>) {
        self.set_value(Self::selected_item_property(), value)
    }

    // --- events -----------------------------------------------------------------------------

    /// Adds a handler of `TabCloseRequested`. Dropping the returned guard removes the handler;
    /// [`Subscription::detach`] keeps it for the life of the control.
    pub fn tab_close_requested(
        &self,
        handler: impl Fn(&Ref<TabView>, &TabViewTabCloseRequestedEventArgs) + 'static,
    ) -> Subscription {
        let token = self.tab_close_requested.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Subscription::new(Disposable::create(move || {
            if let Some(owner) = weak.upgrade() {
                owner.tab_close_requested.remove(token);
            }
        }))
    }

    /// Adds a handler of `AddTabButtonClick`.
    pub fn add_tab_button_click(&self, handler: impl Fn(&Ref<TabView>) + 'static) -> Subscription {
        let token = self.add_tab_button_click.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Subscription::new(Disposable::create(move || {
            if let Some(owner) = weak.upgrade() {
                owner.add_tab_button_click.remove(token);
            }
        }))
    }

    /// Adds a handler of `SelectionChanged`, raised with the arguments of the tab list.
    pub fn selection_changed(
        &self,
        handler: impl Fn(&Ref<TabView>, &SelectionChangedEventArgs) + 'static,
    ) -> Subscription {
        let token = self.selection_changed.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Subscription::new(Disposable::create(move || {
            if let Some(owner) = weak.upgrade() {
                owner.selection_changed.remove(token);
            }
        }))
    }

    /// Adds a handler of `TabItemsChanged`, raised when the items of the tabs change.
    pub fn tab_items_changed(
        &self,
        handler: impl for<'a> Fn(&Ref<TabView>, &ItemsChangedEventArgs<'a>) + 'static,
    ) -> Subscription {
        let token = self.tab_items_changed.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Subscription::new(Disposable::create(move || {
            if let Some(owner) = weak.upgrade() {
                owner.tab_items_changed.remove(token);
            }
        }))
    }

    // --- the parts ----------------------------------------------------------------------------

    fn list_view(&self) -> Option<Ref<TabViewListView>> {
        self.parts.borrow().list_view.as_ref().and_then(WeakRef::upgrade)
    }

    fn unhook_events_and_clear_fields(&self) {
        let parts = std::mem::take(&mut *self.parts.borrow_mut());
        for cleanup in parts.cleanup {
            cleanup();
        }
    }

    /// The tabs of the list: `TabItemsSource`, or `TabItems` (WinUI moves `TabItems` into the
    /// items of the list when it loads; the port gives the list the collection itself).
    fn update_list_view_items(&self) {
        let Some(list_view) = self.list_view() else { return };
        let source = self.tab_items_source().unwrap_or_else(|| ItemsSource::new(Rc::new(self.tab_items.clone())));
        if list_view.items_source().as_ref() != Some(&source) {
            list_view.set_items_source(Some(source));
        }
        list_view.set_item_template(self.tab_item_template());
        self.update_selected_index();
    }

    // --- WinUI's handlers ---------------------------------------------------------------------

    fn on_list_view_loaded(&self) {
        let Some(list_view) = self.list_view() else { return };
        if self.selected_item().is_some() {
            self.update_selected_item();
        } else {
            self.update_selected_index();
        }
        self.sync_selection_from_list(&list_view);
        // The scroll viewer of the list and its buttons (OnScrollViewerLoaded).
        let scroll_viewer = list_view.scroll_viewer();
        if let Some(scroll_viewer) = &scroll_viewer {
            self.on_scroll_viewer_loaded(scroll_viewer);
        }
        self.parts.borrow_mut().scroll_viewer = scroll_viewer.map(|viewer| viewer.downgrade());
        self.update_bottom_border_line_visual_states();
    }

    fn on_scroll_viewer_loaded(&self, scroll_viewer: &Ref<ScrollViewer>) {
        let weak = self.to_ref().downgrade();
        let decrease = find_named::<RepeatButton>(scroll_viewer, "ScrollDecreaseButton");
        let increase = find_named::<RepeatButton>(scroll_viewer, "ScrollIncreaseButton");
        let mut parts = self.parts.borrow_mut();
        for (button, tooltip, forward) in [
            (&decrease, SCROLL_DECREASE_BUTTON_TOOLTIP, false),
            (&increase, SCROLL_INCREASE_BUTTON_TOOLTIP, true),
        ] {
            let Some(button) = button else { continue };
            if ToolTip::get_tip(button).is_none() {
                ToolTip::set_tip(button, Some(Rc::new(tooltip.to_string()) as BoxedValue));
            }
            let weak = weak.clone();
            let click = button.click(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    this.scroll_tabs(forward);
                }
            });
            let target = button.downgrade();
            parts.cleanup.push(Box::new(move || {
                if let Some(button) = target.upgrade() {
                    button.remove_handler(Button::click_event(), click);
                }
            }));
        }
        parts.scroll_decrease_button = decrease.map(|button| button.downgrade());
        parts.scroll_increase_button = increase.map(|button| button.downgrade());
        let changed = weak.clone();
        let changed = scroll_viewer.scroll_changed(move |_, _| {
            if let Some(this) = changed.upgrade() {
                this.update_scroll_viewer_decrease_and_increase_buttons_view_state();
            }
        });
        let target = scroll_viewer.downgrade();
        parts.cleanup.push(Box::new(move || {
            if let Some(viewer) = target.upgrade() {
                viewer.remove_handler(ScrollViewer::scroll_changed_event(), changed);
            }
        }));
        drop(parts);
        self.update_tab_widths(true, true);
    }

    fn on_tab_strip_pointer_exited(&self) {
        self.pointer_in_tab_strip.set(false);
        if self.update_tab_width_on_pointer_leave.replace(false) {
            self.update_tab_widths(true, true);
        }
    }

    fn on_add_button_click(&self) {
        let this = self.to_ref();
        for (_, handler) in self.add_tab_button_click.snapshot().iter() {
            handler(&this);
        }
    }

    fn scroll_tabs(&self, forward: bool) {
        let Some(viewer) = self.parts.borrow().scroll_viewer.as_ref().and_then(WeakRef::upgrade) else { return };
        let offset = viewer.offset();
        let scrollable = (viewer.extent().width - viewer.viewport().width).max(0.0);
        let x = match forward {
            true => (offset.x + SCROLL_AMOUNT).min(scrollable),
            false => (offset.x - SCROLL_AMOUNT).max(0.0),
        };
        viewer.set_offset(ferroui_base::Vector::new(x, offset.y));
    }

    fn update_scroll_viewer_decrease_and_increase_buttons_view_state(&self) {
        let parts = self.parts.borrow();
        let Some(viewer) = parts.scroll_viewer.as_ref().and_then(WeakRef::upgrade) else { return };
        let decrease = parts.scroll_decrease_button.as_ref().and_then(WeakRef::upgrade);
        let increase = parts.scroll_increase_button.as_ref().and_then(WeakRef::upgrade);
        const MIN_THRESHOLD: f64 = 0.1;
        let horizontal_offset = viewer.offset().x;
        let scrollable_width = (viewer.extent().width - viewer.viewport().width).max(0.0);
        let (decrease_enabled, increase_enabled) = if (horizontal_offset - scrollable_width).abs() < MIN_THRESHOLD {
            (true, false)
        } else if horizontal_offset.abs() < MIN_THRESHOLD {
            (false, true)
        } else {
            (true, true)
        };
        if let Some(decrease) = decrease {
            decrease.set_is_enabled(decrease_enabled);
        }
        if let Some(increase) = increase {
            increase.set_is_enabled(increase_enabled);
        }
    }

    /// TabViewListView::OnItemsChanged calls this (TabView::OnItemsChanged).
    pub(super) fn on_items_changed(&self, e: &ItemsChangedEventArgs<'_>) {
        let this = self.to_ref();
        for (_, handler) in self.tab_items_changed.snapshot().iter() {
            handler(&this, e);
        }
        let Some(list_view) = self.list_view() else { return };
        let item_count = self.get_item_count();
        let list_selected_index = list_view.selected_index();
        let mut selected_index = self.selected_index();
        if selected_index != list_selected_index && list_selected_index != -1 {
            self.set_selected_index(list_selected_index);
            selected_index = list_selected_index;
        }
        if e.action == NotifyCollectionChangedAction::Remove {
            self.update_tab_width_on_pointer_leave.set(true);
            let removed = e.old_starting_index;
            if item_count > 0 && (selected_index == -1 || selected_index == removed) {
                // The next enabled and visible tab after the removed one, around the end.
                let start = removed.clamp(0, item_count - 1);
                let mut index = start;
                loop {
                    let next = self.container_from_index(index).and_then(|container| container.cast::<ListViewItem>());
                    if next.is_some_and(|item| item.is_enabled() && item.is_visible()) {
                        self.set_selected_item(self.item_at(index));
                        break;
                    }
                    index = (index + 1) % item_count;
                    if index == start {
                        break;
                    }
                }
            }
            if self.tab_width_mode() == TabViewWidthMode::Equal
                && (!self.pointer_in_tab_strip.get() || removed == item_count)
            {
                self.update_tab_widths(true, false);
            }
        } else {
            self.update_tab_widths(true, true);
            self.set_tab_separator_opacity_for(item_count - 1);
        }
        self.update_bottom_border_line_visual_states();
    }

    fn on_list_view_selection_changed(&self, args: &SelectionChangedEventArgs) {
        if let Some(list_view) = self.list_view() {
            self.sync_selection_from_list(&list_view);
        }
        self.update_tab_content();
        let this = self.to_ref();
        for (_, handler) in self.selection_changed.snapshot().iter() {
            handler(&this, args);
        }
    }

    fn sync_selection_from_list(&self, list_view: &TabViewListView) {
        self.syncing_selection.set(true);
        self.set_selected_index(list_view.selected_index());
        self.set_selected_item(list_view.selected_item());
        self.syncing_selection.set(false);
    }

    fn on_selected_index_changed(&self, old_index: i32) {
        self.update_selected_index();
        self.set_tab_separator_opacity_for(old_index);
        self.set_tab_separator_opacity_for(old_index - 1);
        self.set_tab_separator_opacity_for(self.selected_index() - 1);
        self.set_tab_separator_opacity_for(self.selected_index());
        self.update_bottom_border_line_visual_states();
    }

    fn update_selected_item(&self) {
        if self.syncing_selection.get() {
            return;
        }
        if let Some(list_view) = self.list_view() {
            list_view.set_selected_item(self.selected_item());
        }
    }

    fn update_selected_index(&self) {
        if self.syncing_selection.get() {
            return;
        }
        if let Some(list_view) = self.list_view() {
            let selected_index = self.selected_index();
            if selected_index < list_view.item_count() {
                list_view.set_selected_index(selected_index);
            }
        }
    }

    fn on_tab_width_mode_changed(&self) {
        self.update_tab_widths(true, true);
        let mode = self.tab_width_mode();
        for index in 0..self.get_item_count() {
            if let Some(tab) = self.tab_at(index) {
                tab.on_tab_view_width_mode_changed(mode);
            }
        }
    }

    fn on_close_button_overlay_mode_changed(&self) {
        let mode = self.close_button_overlay_mode();
        for index in 0..self.get_item_count() {
            if let Some(tab) = self.tab_at(index) {
                tab.on_close_button_overlay_mode_changed(mode);
            }
        }
    }

    /// The separator after the tab at `index` is hidden beside the selected tab (SetTabSeparatorOpacity).
    pub(super) fn set_tab_separator_opacity_for(&self, index: i32) {
        let selected_index = self.selected_index();
        let opacity = if index == selected_index || index + 1 == selected_index { 0.0 } else { 1.0 };
        self.set_tab_separator_opacity(index, opacity);
    }

    pub(super) fn set_tab_separator_opacity(&self, index: i32, opacity: f64) {
        if let Some(tab) = self.tab_at(index) {
            tab.set_separator_opacity(opacity);
        }
    }

    fn update_tab_bottom_border_line_visual_states(&self) {
        let selected_index = self.selected_index();
        for index in 0..self.get_item_count() {
            let state = if selected_index == -1 {
                BottomBorderLine::Normal
            } else if index == selected_index {
                BottomBorderLine::None
            } else if index == selected_index - 1 {
                BottomBorderLine::LeftOfSelectedTab
            } else if index == selected_index + 1 {
                BottomBorderLine::RightOfSelectedTab
            } else {
                BottomBorderLine::Normal
            };
            if let Some(tab) = self.tab_at(index) {
                tab.set_bottom_border_line(state);
            }
        }
    }

    fn update_bottom_border_line_visual_states(&self) {
        self.update_tab_bottom_border_line_visual_states();
        // `m_isItemBeingDragged` is never set: dragging tabs is not ported.
        self.pseudo_classes().set(PC_SINGLE_BOTTOM_BORDER_LINE, false);
    }

    /// Shows the content of the selected tab (UpdateTabContent).
    pub(super) fn update_tab_content(&self) {
        let Some(presenter) = self.parts.borrow().tab_content_presenter.as_ref().and_then(WeakRef::upgrade) else {
            return;
        };
        let tab = self.selected_item().and_then(|item| self.tab_of_item(&Some(item)));
        match tab {
            None => {
                presenter.set_content(None);
                presenter.set_content_template(None);
            }
            Some(tab) => {
                let had_focus = presenter.is_keyboard_focus_within();
                presenter.set_content(tab.content());
                presenter.set_content_template(tab.content_template());
                if had_focus {
                    // The focus moves into the new content, or to the tab (WinUI's LosingFocus).
                    let first = presenter.find_descendant_of_type_where::<InputElement>(false, |element| {
                        element.focusable() && element.is_effectively_enabled() && element.is_effectively_visible()
                    });
                    if !first.is_some_and(|element| element.focus()) {
                        tab.focus();
                    }
                }
            }
        }
    }

    /// Asks the application to close `tab` (RequestCloseTab): raises `TabCloseRequested` and the
    /// tab's `CloseRequested`. A tab with the focus gives it to the next focusable tab, the
    /// previous one, or the add button.
    pub(super) fn request_close_tab(&self, tab: &Ref<TabViewItem>, update_tab_widths: bool) {
        let tab_is_focused = tab.is_keyboard_focus_within();
        let Some(list_view) = self.list_view() else { return };
        let args = TabViewTabCloseRequestedEventArgs::new(list_view.item_from_container(&tab.clone().upcast()), tab.clone());
        let this = self.to_ref();
        for (_, handler) in self.tab_close_requested.snapshot().iter() {
            handler(&this, &args);
        }
        tab.raise_close_requested(&args);
        if tab_is_focused && !tab.is_attached_to_visual_tree() {
            self.focus_after_closed(list_view.index_from_container(&tab.clone().upcast()));
        }
        self.update_tab_widths(update_tab_widths, true);
    }

    fn focus_after_closed(&self, closed_index: i32) {
        let count = self.get_item_count();
        let candidates = (closed_index.max(0)..count).chain((0..closed_index.max(0)).rev());
        for index in candidates {
            if let Some(container) = self.container_from_index(index) {
                if Self::is_focusable(&container, true) {
                    container.focus_with(NavigationMethod::Directional, KeyModifiers::empty());
                    return;
                }
            }
        }
        if let Some(add_button) = self.parts.borrow().add_button.as_ref().and_then(WeakRef::upgrade) {
            add_button.focus();
        }
    }

    /// The widths of the tabs and of the column of the strip (UpdateTabWidths).
    pub(super) fn update_tab_widths(&self, should_update_widths: bool, fill_all_available_space: bool) {
        let resource = |key: &str, fallback: f64| resource_of::<f64>(self, key).unwrap_or(fallback);
        let max_tab_width = resource(TAB_VIEW_ITEM_MAX_WIDTH_KEY, TAB_MAXIMUM_WIDTH);
        let mut tab_width = f64::NAN;
        let tab_count = self.get_item_count().max(0) as f64;
        let parts = self.parts.borrow();
        let column = |part: &Option<WeakRef<ColumnDefinition>>| part.as_ref().and_then(WeakRef::upgrade);
        let mut width_taken = 0.0;
        if let Some(left) = column(&parts.left_content_column) {
            width_taken += left.actual_width();
        }
        if let Some(add) = column(&parts.add_button_column) {
            width_taken += add.actual_width();
        }
        if let Some(right) = column(&parts.right_content_column) {
            if let Some(presenter) = parts.right_content_presenter.as_ref().and_then(WeakRef::upgrade) {
                let width = presenter.desired_size().width;
                right.set_min_width(width);
                width_taken += width;
            }
        }
        let list_view = parts.list_view.as_ref().and_then(WeakRef::upgrade);
        let decrease = parts.scroll_decrease_button.as_ref().and_then(WeakRef::upgrade);
        let increase = parts.scroll_increase_button.as_ref().and_then(WeakRef::upgrade);
        if let Some(tab_column) = column(&parts.tab_column) {
            let available_width = self.previous_available_width.get() - width_taken;
            if available_width > 0.0 {
                if self.tab_width_mode() == TabViewWidthMode::Equal {
                    let min_tab_width = resource(TAB_VIEW_ITEM_MIN_WIDTH_KEY, TAB_MINIMUM_WIDTH);
                    let padding = self.padding();
                    let (header_width, footer_width) = list_view
                        .as_ref()
                        .map_or((0.0, 0.0), |list| (list.header_width(), list.footer_width()));
                    let chrome = padding.left + padding.right + header_width + footer_width;
                    if fill_all_available_space && tab_count > 0.0 {
                        let for_scroller = (available_width - chrome) / tab_count;
                        tab_width = for_scroller.clamp(min_tab_width, max_tab_width);
                    } else if tab_count > 0.0 {
                        let mut space = tab_column.actual_width() - chrome;
                        for button in [&increase, &decrease].into_iter().flatten() {
                            if button.is_visible() {
                                space -= button.bounds().width;
                            }
                        }
                        tab_width = (space / tab_count).clamp(min_tab_width, max_tab_width);
                    }
                    tab_column.set_max_width(available_width + header_width + footer_width);
                    let required_width = tab_width * tab_count + chrome;
                    if required_width > available_width {
                        tab_column.set_width(GridLength::new(available_width, GridUnitType::Pixel));
                        if let Some(list_view) = &list_view {
                            ScrollViewer::set_horizontal_scroll_bar_visibility_on(list_view, ScrollBarVisibility::Visible);
                        }
                        drop(parts);
                        self.update_scroll_viewer_decrease_and_increase_buttons_view_state();
                    } else {
                        tab_column.set_width(GridLength::AUTO);
                        if let Some(list_view) = &list_view {
                            if should_update_widths && fill_all_available_space {
                                ScrollViewer::set_horizontal_scroll_bar_visibility_on(list_view, ScrollBarVisibility::Hidden);
                            } else {
                                for button in [&decrease, &increase].into_iter().flatten() {
                                    button.set_is_enabled(false);
                                }
                            }
                        }
                        drop(parts);
                    }
                } else {
                    tab_column.set_max_width(available_width);
                    tab_column.set_width(GridLength::AUTO);
                    if let Some(list_view) = &list_view {
                        list_view.set_max_width(available_width);
                        let visible = list_view.items_width() > available_width;
                        let visibility = if visible { ScrollBarVisibility::Visible } else { ScrollBarVisibility::Hidden };
                        ScrollViewer::set_horizontal_scroll_bar_visibility_on(list_view, visibility);
                        drop(parts);
                        if visible {
                            self.update_scroll_viewer_decrease_and_increase_buttons_view_state();
                        }
                    } else {
                        drop(parts);
                    }
                }
            } else {
                drop(parts);
            }
        } else {
            drop(parts);
        }
        if should_update_widths || self.tab_width_mode() != TabViewWidthMode::Equal {
            for index in 0..self.get_item_count() {
                if let Some(tab) = self.tab_at(index) {
                    tab.set_width(tab_width);
                }
            }
        }
    }

    // --- containers ---------------------------------------------------------------------------

    /// The container of `item` (WinUI's `ContainerFromItem`).
    pub fn container_from_item(&self, item: &Option<BoxedValue>) -> Option<Ref<Control>> {
        self.list_view()?.container_from_item(item)
    }

    /// The container of the tab at `index` (WinUI's `ContainerFromIndex`).
    pub fn container_from_index(&self, index: i32) -> Option<Ref<Control>> {
        if index < 0 {
            return None;
        }
        self.list_view()?.container_from_index(index)
    }

    fn tab_at(&self, index: i32) -> Option<Ref<TabViewItem>> {
        self.container_from_index(index)?.cast::<TabViewItem>()
    }

    fn tab_of_item(&self, item: &Option<BoxedValue>) -> Option<Ref<TabViewItem>> {
        if let Some(tab) = item.as_ref().and_then(|item| item.downcast_ref::<Ref<TabViewItem>>()) {
            return Some(tab.clone());
        }
        if let Some(control) = item.as_ref().and_then(|item| item.downcast_ref::<Ref<Control>>()) {
            if let Some(tab) = control.cast::<TabViewItem>() {
                return Some(tab);
            }
        }
        self.container_from_item(item)?.cast::<TabViewItem>()
    }

    fn item_at(&self, index: i32) -> Option<BoxedValue> {
        match self.tab_items_source() {
            Some(source) => usize::try_from(index).ok().filter(|&i| i < source.count()).and_then(|i| source.get_at(i)),
            None => usize::try_from(index).ok().filter(|&i| i < self.tab_items.count()).and_then(|i| self.tab_items.get(i)),
        }
    }

    pub(super) fn index_from_container(&self, container: &Ref<Control>) -> i32 {
        self.list_view().map_or(-1, |list| list.index_from_container(container))
    }

    fn get_item_count(&self) -> i32 {
        match self.tab_items_source() {
            Some(source) => source.count() as i32,
            None => self.tab_items.count() as i32,
        }
    }

    // --- the keyboard -------------------------------------------------------------------------

    /// Moves the focus among the tabs, their close buttons and the add button (MoveFocus).
    pub(super) fn move_focus(&self, move_forward: bool) -> bool {
        let Some(focused) = FocusManager::get_focus_manager(self).and_then(|manager| manager.get_focused_element()) else {
            return false;
        };
        let mut order: Vec<Ref<Control>> = Vec::new();
        for index in 0..self.get_item_count() {
            if let Some(tab) = self.tab_at(index) {
                let control: Ref<Control> = tab.clone().upcast();
                if Self::is_focusable(&control, false) {
                    order.push(control);
                    if let Some(close) = tab.close_button() {
                        let close: Ref<Control> = close.upcast();
                        if Self::is_focusable(&close, false) {
                            order.push(close);
                        }
                    }
                }
            }
        }
        if let Some(add_button) = self.parts.borrow().add_button.as_ref().and_then(WeakRef::upgrade) {
            let add_button: Ref<Control> = add_button.upcast();
            if Self::is_focusable(&add_button, false) {
                order.push(add_button);
            }
        }
        let Some(position) = order.iter().position(|control| control.clone().upcast::<InputElement>() == focused) else {
            return false;
        };
        let count = order.len() as i32;
        let next = (position as i32 + if move_forward { 1 } else { -1 }).rem_euclid(count) as usize;
        order[next].focus_with(NavigationMethod::Directional, KeyModifiers::empty())
    }

    /// Selects the next or the previous focusable tab, around the ends (MoveSelection).
    fn move_selection(&self, move_forward: bool) -> bool {
        let original = self.selected_index();
        let count = self.get_item_count();
        if count == 0 {
            return false;
        }
        let increment = if move_forward { 1 } else { -1 };
        let mut current = original + increment;
        while current != original {
            if current < 0 {
                current = count - 1;
            } else if current >= count {
                current = 0;
            }
            if current == original {
                break;
            }
            if self.container_from_index(current).is_some_and(|container| Self::is_focusable(&container, true)) {
                self.set_selected_index(current);
                return true;
            }
            current += increment;
        }
        false
    }

    fn request_close_current_tab(&self) -> bool {
        match self.selected_item().and_then(|item| self.tab_of_item(&Some(item))) {
            Some(tab) if tab.is_closable() => {
                self.request_close_tab(&tab, true);
                true
            }
            _ => false,
        }
    }

    fn is_focusable(control: &Ref<Control>, check_tab_stop: bool) -> bool {
        control.is_visible() && control.is_effectively_enabled() && (control.is_tab_stop() || !check_tab_stop)
    }

    /// The flow direction of the tab strip, for the arrow keys of the tabs.
    pub(super) fn is_left_to_right(element: &Control) -> bool {
        element.flow_direction() == FlowDirection::LeftToRight
    }
}

/// The descendant of `root` of class `T` named `name` (WinUI's `FindInVisualTreeByName`).
pub(super) fn find_named<T: ferroui_base::ObjectType>(root: &ferroui_base::Visual, name: &str) -> Option<Ref<T>> {
    let found = root.find_descendant_of_type_where::<ferroui_base::StyledElement>(false, |element| {
        element.name().as_deref() == Some(name) && element.to_ref().cast::<T>().is_some()
    })?;
    found.cast::<T>()
}

/// A resource of the theme, of type `T`, found from `element`.
pub(super) fn resource_of<T: Clone + 'static>(element: &ferroui_base::StyledElement, key: &str) -> Option<T> {
    let value = element.try_find_resource(&ferroui_base::controls::ResourceKey::from(key), None).flatten()?;
    value.downcast_ref::<T>().cloned()
}

/// The bottom border line of a tab (the states of WinUI's `UpdateTabBottomBorderLineVisualStates`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum BottomBorderLine {
    Normal,
    None,
    LeftOfSelectedTab,
    RightOfSelectedTab,
}

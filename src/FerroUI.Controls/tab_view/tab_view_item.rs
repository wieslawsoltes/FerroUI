//! WinUI's `TabViewItem` and `TabViewItemTemplateSettings` (`controls/dev/TabView/TabViewItem.cpp`,
//! `TabViewItem.h` and `TabViewItemTemplateSettings.h` of microsoft-ui-xaml commit
//! `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c) Microsoft Corporation, see
//! NOTICE.md).

use super::tab_view::{BottomBorderLine, CLOSE_BUTTON_NAME, CLOSE_BUTTON_TOOLTIP_WITH_ACCELERATOR};
use super::{TabView, TabViewCloseButtonOverlayMode, TabViewTabCloseRequestedEventArgs, TabViewWidthMode};
use crate::automation::peers::{AutomationPeer, TabViewItemAutomationPeer};
use crate::automation::AutomationProperties;
use crate::primitives::{TemplateAppliedEventArgs, TemplatedControl, TemplatedControlImpl, TemplatedControlImplExt};
use crate::templates::IDataTemplate;
use crate::{
    Button, ContentControlImpl, Control, ControlImpl, IconElement, IconSource, ListViewItem, Panel, ToolTip,
};
use ferroui_base::input::{InputElementImpl, InputElementImplExt, Key, KeyEventArgs, PointerEventArgs,
    PointerPressedEventArgs, PointerReleasedEventArgs, PointerUpdateKind};
use ferroui_base::interactivity::{InteractiveImpl, RoutedEventHandlerToken};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::Geometry;
use ferroui_base::metadata::{into_markup_value, MarkupDelegate};
use ferroui_base::reactive::{Disposable, Subscription};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_markup_type, ferro_properties, ferro_property,
    instantiate, BoxedValue, CornerRadius, DirectProperty, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, Ref, StyledElementImpl, StyledProperty, VisualImpl, WeakRef,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The values the template of a tab binds to (WinUI's `TabViewItemTemplateSettings`): the icon
/// element of [`TabViewItem::icon_source`] and the outline of the selected tab.
#[derive(Clone, Default)]
pub struct TabViewItemTemplateSettings {
    icon_element: Rc<RefCell<Option<Ref<IconElement>>>>,
    tab_geometry: Rc<RefCell<Option<Ref<Geometry>>>>,
}

impl TabViewItemTemplateSettings {
    /// The icon of the tab.
    pub fn icon_element(&self) -> Option<Ref<IconElement>> {
        self.icon_element.borrow().clone()
    }

    /// The outline of the selected tab: its rounded top corners and the curves at its foot.
    pub fn tab_geometry(&self) -> Option<Ref<Geometry>> {
        self.tab_geometry.borrow().clone()
    }
}

impl PartialEq for TabViewItemTemplateSettings {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.icon_element, &other.icon_element)
    }
}

ferro_markup_type!(class TabViewItemTemplateSettings {
    namespace: "FerroUI.Controls",
    handles: [TabViewItemTemplateSettings],
    properties: [
        IconElement: Option<Ref<IconElement>> { get: |settings: &TabViewItemTemplateSettings| settings.icon_element() },
        TabGeometry: Option<Ref<Geometry>> { get: |settings: &TabViewItemTemplateSettings| settings.tab_geometry() },
    ],
});

const PC_CLOSE_BUTTON_COLLAPSED: &str = ":close-button-collapsed";
const PC_COMPACT: &str = ":compact";
const PC_ICON: &str = ":icon";
const PC_FOREGROUND_SET: &str = ":foreground-set";
const PC_NO_BOTTOM_BORDER_LINE: &str = ":no-bottom-border-line";
const PC_LEFT_OF_SELECTED_TAB: &str = ":left-of-selected-tab";
const PC_RIGHT_OF_SELECTED_TAB: &str = ":right-of-selected-tab";

/// The corner radius of the selected tab when the theme has no `OverlayCornerRadius`.
const OVERLAY_CORNER_RADIUS_KEY: &str = "OverlayCornerRadius";

/// One tab of a [`TabView`] (WinUI's `TabViewItem`): a [`ListViewItem`] with a header, an icon and
/// a close button. Its content is shown by the tab view while the tab is selected.
///
/// The states of WinUI are pseudo-classes for the control theme of the WinUI theme:
/// `:close-button-collapsed` (CloseButtonCollapsed; its absence is CloseButtonVisible),
/// `:compact` (Compact; StandardWidth without it), `:icon` (Icon; NoIcon without it),
/// `:foreground-set` (a local `Foreground`), and the bottom border line of the tab strip:
/// `:no-bottom-border-line`, `:left-of-selected-tab` and `:right-of-selected-tab` (none of them is
/// NormalBottomBorderLine). The parts it looks up are `CloseButton`, `ContentPresenter` (the
/// header), `TabSeparator` and `SelectedBackgroundPath`.
#[repr(C)]
pub struct TabViewItem {
    base: ListViewItem,
    settings: TabViewItemTemplateSettings,
    parent_tab_view: RefCell<Option<WeakRef<TabView>>>,
    width_mode: Cell<TabViewWidthMode>,
    close_button_overlay_mode: Cell<TabViewCloseButtonOverlayMode>,
    is_pointer_over: Cell<bool>,
    is_middle_pointer_button_pressed: Cell<bool>,
    foreground_is_local: Cell<bool>,
    close_button: RefCell<Option<(WeakRef<Button>, RoutedEventHandlerToken)>>,
    header_presenter: RefCell<Option<WeakRef<Control>>>,
    tab_separator: RefCell<Option<WeakRef<Control>>>,
    tool_tip_set_by_tab: Cell<bool>,
    close_requested: HandlerList<dyn Fn(&Ref<TabViewItem>, &TabViewTabCloseRequestedEventArgs)>,
}

ferro_class!(TabViewItem: ListViewItem);
ferro_class_info!(TabViewItem {
    new: TabViewItem::new,
    markup: {
        namespace: "FerroUI.Controls",
        content: Content,
        events: [
            CloseRequested(Option<BoxedValue>, TabViewTabCloseRequestedEventArgs) =>
                |this: &Ref<TabViewItem>, handler: MarkupDelegate| {
                    this.close_requested(move |sender, args| {
                        handler.invoke(&[into_markup_value(Some(sender.clone())), into_markup_value(args.clone())]);
                    })
                },
        ],
        attributes: [
            TemplatePart("CloseButton", type(Ref<Button>)),
            PseudoClasses(
                ":close-button-collapsed",
                ":compact",
                ":icon",
                ":foreground-set",
                ":no-bottom-border-line",
                ":left-of-selected-tab",
                ":right-of-selected-tab"
            ),
        ],
    },
});
ferro_impl_classes!(TabViewItem: StyledElementImpl, VisualImpl, InteractiveImpl, ContentControlImpl);

ferro_properties! { impl TabViewItem {
    ferro_property!(
        /// Defines the `Header` property.
        pub fn header_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<TabViewItem, _>("Header", None)
        }
    );

    ferro_property!(
        /// Defines the `HeaderTemplate` property.
        pub fn header_template_property() -> StyledProperty<Option<Rc<dyn IDataTemplate>>> {
            FerroProperty::register::<TabViewItem, _>("HeaderTemplate", None)
        }
    );

    ferro_property!(
        /// Defines the `IconSource` property.
        pub fn icon_source_property() -> StyledProperty<Option<Ref<IconSource>>> {
            FerroProperty::register::<TabViewItem, _>("IconSource", None)
        }
    );

    ferro_property!(
        /// Defines the `IsClosable` property.
        pub fn is_closable_property() -> StyledProperty<bool> {
            FerroProperty::register::<TabViewItem, _>("IsClosable", true)
        }
    );

    ferro_property!(
        /// Defines the `TabViewTemplateSettings` property, read only.
        pub fn tab_view_template_settings_property() -> DirectProperty<TabViewItem, TabViewItemTemplateSettings> {
            FerroProperty::register_direct::<TabViewItem, _>(
                "TabViewTemplateSettings",
                |o| o.tab_view_template_settings(),
                None,
                TabViewItemTemplateSettings::default(),
            )
        }
    );
} }

impl FerroObjectImpl for TabViewItem {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.update_close_button();
        this.update_width_mode_visual_state();
        this.on_icon_source_changed();
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        let property = change.property();
        if property == TemplatedControl::foreground_property().as_property() {
            this.foreground_is_local.set(change.priority() == ferroui_base::data::BindingPriority::LocalValue);
            this.update_foreground();
            return;
        }
        if !change.is_effective_value_change() {
            return;
        }
        if property == ListViewItem::is_selected_property().as_property() {
            this.on_is_selected_changed();
        } else if property == Self::is_closable_property().as_property() {
            this.update_close_button();
        } else if property == Self::header_property().as_property() {
            this.on_header_changed();
        } else if property == Self::icon_source_property().as_property() {
            this.on_icon_source_changed();
        } else if property == ferroui_base::Visual::bounds_property().as_property() {
            this.update_tab_geometry();
        }
    }
}

impl ControlImpl for TabViewItem {
    fn on_create_automation_peer(this: &Self) -> Ref<AutomationPeer> {
        TabViewItemAutomationPeer::new(this).upcast()
    }
}

impl LayoutableImpl for TabViewItem {}

impl TemplatedControlImpl for TabViewItem {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, e);
        if let Some((button, token)) = this.close_button.borrow_mut().take() {
            if let Some(button) = button.upgrade() {
                button.remove_handler_token(token);
            }
        }
        let scope = e.name_scope();
        *this.header_presenter.borrow_mut() = scope.find_as::<Control>("ContentPresenter").map(|c| c.downgrade());
        *this.tab_separator.borrow_mut() = scope.find_as::<Control>("TabSeparator").map(|c| c.downgrade());
        if let Some(close_button) = scope.find_as::<Button>("CloseButton") {
            if AutomationProperties::get_name(&close_button).is_none_or(|name| name.is_empty()) {
                AutomationProperties::set_name(&close_button, Some(CLOSE_BUTTON_NAME));
            }
            ToolTip::set_tip(&close_button, Some(Rc::new(CLOSE_BUTTON_TOOLTIP_WITH_ACCELERATOR.to_string()) as BoxedValue));
            let weak = this.to_ref().downgrade();
            let token = close_button.click(move |_, _| {
                if let Some(tab) = weak.upgrade() {
                    tab.request_close();
                }
            });
            *this.close_button.borrow_mut() = Some((close_button.downgrade(), token));
        }
        this.on_header_changed();
        this.on_icon_source_changed();
        this.update_close_button();
        this.update_foreground();
        this.update_width_mode_visual_state();
        this.update_tab_geometry();
    }
}

impl InputElementImpl for TabViewItem {
    fn on_pointer_pressed(this: &Self, e: &PointerPressedEventArgs) {
        Self::parent_on_pointer_pressed(this, e);
        // A middle click closes a closable tab when it is released over it.
        if e.get_current_point(Some(this)).properties().pointer_update_kind() == PointerUpdateKind::MiddleButtonPressed {
            e.pointer().capture(Some(this));
            this.is_middle_pointer_button_pressed.set(true);
        }
    }

    fn on_pointer_released(this: &Self, e: &PointerReleasedEventArgs) {
        Self::parent_on_pointer_released(this, e);
        if this.is_middle_pointer_button_pressed.replace(false)
            && e.get_current_point(Some(this)).properties().pointer_update_kind() == PointerUpdateKind::MiddleButtonReleased
        {
            e.pointer().capture(None);
            if this.is_closable() && this.is_pointer_over.get() {
                this.request_close();
            }
        }
    }

    fn on_pointer_entered(this: &Self, e: &PointerEventArgs) {
        Self::parent_on_pointer_entered(this, e);
        this.is_pointer_over.set(true);
        this.update_close_button();
        this.hide_left_adjacent_tab_separator();
    }

    fn on_pointer_exited(this: &Self, e: &PointerEventArgs) {
        Self::parent_on_pointer_exited(this, e);
        this.is_pointer_over.set(false);
        this.update_close_button();
        this.update_foreground();
        this.restore_left_adjacent_tab_separator_visibility();
    }

    fn on_key_down(this: &Self, e: &KeyEventArgs) {
        if !e.handled() && matches!(e.key(), Key::Left | Key::Right) {
            let left_to_right = TabView::is_left_to_right(this);
            let move_forward = (left_to_right && e.key() == Key::Right) || (!left_to_right && e.key() == Key::Left);
            if let Some(tab_view) = this.parent_tab_view() {
                if tab_view.move_focus(move_forward) {
                    e.set_handled(true);
                    return;
                }
            }
        }
        Self::parent_on_key_down(this, e);
    }
}

impl TabViewItem {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: ListViewItem::construct(),
            settings: TabViewItemTemplateSettings::default(),
            parent_tab_view: RefCell::new(None),
            width_mode: Cell::new(TabViewWidthMode::Equal),
            close_button_overlay_mode: Cell::new(TabViewCloseButtonOverlayMode::Auto),
            is_pointer_over: Cell::new(false),
            is_middle_pointer_button_pressed: Cell::new(false),
            foreground_is_local: Cell::new(false),
            close_button: RefCell::new(None),
            header_presenter: RefCell::new(None),
            tab_separator: RefCell::new(None),
            tool_tip_set_by_tab: Cell::new(false),
            close_requested: HandlerList::new(),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    pub fn header(&self) -> Option<BoxedValue> {
        self.get_value(Self::header_property())
    }

    pub fn set_header(&self, value: Option<BoxedValue>) {
        self.set_value(Self::header_property(), value)
    }

    pub fn header_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.get_value(Self::header_template_property())
    }

    pub fn set_header_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        self.set_value(Self::header_template_property(), value)
    }

    pub fn icon_source(&self) -> Option<Ref<IconSource>> {
        self.get_value(Self::icon_source_property())
    }

    pub fn set_icon_source(&self, value: Option<Ref<IconSource>>) {
        self.set_value(Self::icon_source_property(), value)
    }

    pub fn is_closable(&self) -> bool {
        self.get_value(Self::is_closable_property())
    }

    pub fn set_is_closable(&self, value: bool) {
        self.set_value(Self::is_closable_property(), value)
    }

    /// WinUI's `TabViewTemplateSettings`.
    pub fn tab_view_template_settings(&self) -> TabViewItemTemplateSettings {
        self.settings.clone()
    }

    /// Adds a handler of `CloseRequested`, raised after the tab view's `TabCloseRequested`.
    /// Dropping the returned guard removes the handler; [`Subscription::detach`] keeps it.
    pub fn close_requested(
        &self,
        handler: impl Fn(&Ref<TabViewItem>, &TabViewTabCloseRequestedEventArgs) + 'static,
    ) -> Subscription {
        let token = self.close_requested.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Subscription::new(Disposable::create(move || {
            if let Some(owner) = weak.upgrade() {
                owner.close_requested.remove(token);
            }
        }))
    }

    pub(super) fn raise_close_requested(&self, args: &TabViewTabCloseRequestedEventArgs) {
        let this = self.to_ref();
        for (_, handler) in self.close_requested.snapshot().iter() {
            handler(&this, args);
        }
    }

    pub(super) fn close_button(&self) -> Option<Ref<Button>> {
        self.close_button.borrow().as_ref().and_then(|(button, _)| button.upgrade())
    }

    pub(super) fn parent_tab_view(&self) -> Option<Ref<TabView>> {
        if let Some(tab_view) = self.parent_tab_view.borrow().as_ref().and_then(WeakRef::upgrade) {
            return Some(tab_view);
        }
        self.find_ancestor_of_type::<TabView>()
    }

    pub(super) fn set_parent_tab_view(&self, tab_view: &TabView) {
        *self.parent_tab_view.borrow_mut() = Some(tab_view.to_ref().downgrade());
    }

    pub(super) fn on_tab_view_width_mode_changed(&self, mode: TabViewWidthMode) {
        self.width_mode.set(mode);
        self.update_width_mode_visual_state();
    }

    pub(super) fn on_close_button_overlay_mode_changed(&self, mode: TabViewCloseButtonOverlayMode) {
        self.close_button_overlay_mode.set(mode);
        self.update_close_button();
    }

    pub(super) fn set_separator_opacity(&self, opacity: f64) {
        if let Some(separator) = self.tab_separator.borrow().as_ref().and_then(WeakRef::upgrade) {
            separator.set_opacity(opacity);
        }
    }

    pub(super) fn set_bottom_border_line(&self, state: BottomBorderLine) {
        let classes = self.pseudo_classes();
        classes.set(PC_NO_BOTTOM_BORDER_LINE, state == BottomBorderLine::None);
        classes.set(PC_LEFT_OF_SELECTED_TAB, state == BottomBorderLine::LeftOfSelectedTab);
        classes.set(PC_RIGHT_OF_SELECTED_TAB, state == BottomBorderLine::RightOfSelectedTab);
    }

    fn request_close(&self) {
        if let Some(tab_view) = self.parent_tab_view() {
            tab_view.request_close_tab(&self.to_ref(), false);
        }
    }

    fn on_is_selected_changed(&self) {
        // The selected tab is drawn over its neighbours, and is brought into view.
        if self.is_selected() {
            self.set_z_index(20);
            self.start_bring_tab_into_view();
        } else {
            self.set_z_index(0);
        }
        self.update_width_mode_visual_state();
        self.update_close_button();
        self.update_foreground();
    }

    fn update_foreground(&self) {
        if !self.is_selected() && !self.is_pointer_over.get() {
            self.pseudo_classes().set(PC_FOREGROUND_SET, self.foreground_is_local.get());
        }
    }

    fn update_close_button(&self) {
        let collapsed = if !self.is_closable() {
            true
        } else {
            match self.close_button_overlay_mode.get() {
                TabViewCloseButtonOverlayMode::OnPointerOver => !(self.is_selected() || self.is_pointer_over.get()),
                _ => false,
            }
        };
        self.pseudo_classes().set(PC_CLOSE_BUTTON_COLLAPSED, collapsed);
    }

    fn update_width_mode_visual_state(&self) {
        let compact = !self.is_selected() && self.width_mode.get() == TabViewWidthMode::Compact;
        self.pseudo_classes().set(PC_COMPACT, compact);
    }

    fn on_header_changed(&self) {
        if let Some(presenter) = self.header_presenter.borrow().as_ref().and_then(WeakRef::upgrade) {
            if let Some(presenter) = presenter.cast::<crate::ContentPresenter>() {
                presenter.set_content(self.header());
            }
        }
        // The header text is the tool tip of the tab, unless the application set one.
        if ToolTip::get_tip(self).is_none() || self.tool_tip_set_by_tab.get() {
            let text = self.header().and_then(|header| header.downcast_ref::<String>().cloned()).filter(|t| !t.is_empty());
            self.tool_tip_set_by_tab.set(text.is_some());
            ToolTip::set_tip(self, text.map(|text| Rc::new(text) as BoxedValue));
        }
    }

    fn on_icon_source_changed(&self) {
        let element = self.icon_source().and_then(|source| source.create_icon_element());
        self.pseudo_classes().set(PC_ICON, element.is_some());
        *self.settings.icon_element.borrow_mut() = element;
        self.raise_template_settings_changed();
    }

    /// The outline of the selected tab, from its size and the theme's `OverlayCornerRadius`.
    fn update_tab_geometry(&self) {
        let radius = self
            .try_find_resource(OVERLAY_CORNER_RADIUS_KEY)
            .and_then(|value| value.downcast_ref::<CornerRadius>().copied())
            .unwrap_or_default();
        let (height, width) = (self.bounds().height, self.bounds().width);
        let (left, right) = (radius.top_left, radius.top_right);
        let data = format!(
            "F1 M0,{height} a 4,4 0 0 0 4,-4 L 4,{left} a {left},{left} 0 0 1 {left},-{left} l {},0 \
             a {right},{right} 0 0 1 {right},{right} l 0,{} a 4,4 0 0 0 4,4 Z",
            width - (left + right),
            height - (4.0 + right),
        );
        *self.settings.tab_geometry.borrow_mut() = Geometry::parse(&data).ok();
        self.raise_template_settings_changed();
    }

    fn raise_template_settings_changed(&self) {
        let settings = self.settings.clone();
        self.raise_property_changed(Self::tab_view_template_settings_property(), settings.clone(), settings);
    }

    fn hide_left_adjacent_tab_separator(&self) {
        if let Some(tab_view) = self.parent_tab_view() {
            let index = tab_view.index_from_container(&self.to_ref().upcast());
            tab_view.set_tab_separator_opacity(index - 1, 0.0);
        }
    }

    fn restore_left_adjacent_tab_separator_visibility(&self) {
        if let Some(tab_view) = self.parent_tab_view() {
            let index = tab_view.index_from_container(&self.to_ref().upcast());
            tab_view.set_tab_separator_opacity_for(index - 1);
        }
    }

    /// Brings the tab into view, two pixels wider (StartBringTabIntoView).
    fn start_bring_tab_into_view(&self) {
        let size = self.desired_size();
        self.bring_into_view_rect(ferroui_base::Rect::new(0.0, 0.0, size.width + 2.0, size.height));
    }
}

#[allow(dead_code)]
fn panel_of(control: &Control) -> Option<Ref<Panel>> {
    control.cast::<Panel>()
}

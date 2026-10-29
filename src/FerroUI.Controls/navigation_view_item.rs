//! WinUI's `NavigationViewItemBase`, `NavigationViewItem`, `NavigationViewItemHeader` and
//! `NavigationViewItemSeparator` (`controls/dev/NavigationView/NavigationViewItemBase.cpp`,
//! `NavigationViewItem.cpp`, `NavigationViewItemPresenter.cpp`, `NavigationViewItemHeader.cpp`,
//! `NavigationViewItemSeparator.cpp` and `NavigationView.idl` of microsoft-ui-xaml, commit
//! `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c) Microsoft Corporation, see
//! NOTICE.md).

use crate::automation::peers::{AutomationPeer, NavigationViewItemAutomationPeer};
use crate::items_source::items_equal;
use crate::primitives::{Popup, TemplateAppliedEventArgs, TemplatedControlImpl, TemplatedControlImplExt};
use crate::templates::{IControlTemplate, IDataTemplate};
use crate::{
    ContentControl, ContentControlImpl, Control, ControlImpl, IconElement, InfoBadge, ItemCollection, ItemsSource,
    ItemsSourceView, Panel, StackPanel, ToolTip,
};
use ferroui_base::input::{
    InputElement, InputElementImpl, InputElementImplExt, Key, KeyEventArgs, KeyModifiers, MouseButton,
    NavigationMethod, PointerCaptureLostEventArgs, PointerPressedEventArgs, PointerReleasedEventArgs,
};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{LayoutableImpl, LayoutableImplExt};
use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
use ferroui_base::media::FlowDirection;
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, BoxedValue, DirectProperty,
    FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref, StyledElementImpl,
    StyledProperty, Thickness, Visual, VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The indentation of a nested item per level (`NavigationViewItemBase::c_itemIndentation`).
pub(crate) const ITEM_INDENTATION: f64 = 31.0;

/// Sets the left margin of the part that WinUI indents (`UpdateItemIndentation`): the depth times
/// [`ITEM_INDENTATION`], the other sides kept.
fn indent(part: &Option<Ref<Control>>, depth: usize) {
    if let Some(part) = part {
        let old = part.margin();
        part.set_margin(Thickness::new(depth as f64 * ITEM_INDENTATION, old.top, old.right, old.bottom));
    }
}

/// The base of the items of a [`NavigationView`](crate::NavigationView): their content and
/// selection (WinUI's `NavigationViewItemBase`). As in WinUI, it cannot be created in markup.
#[repr(C)]
pub struct NavigationViewItemBase {
    base: ContentControl,
}

ferro_class!(NavigationViewItemBase: ContentControl);
ferro_class_info!(NavigationViewItemBase {
    markup: { namespace: "FerroUI.Controls", content: Content, attributes: [PseudoClasses(":selected")] },
});
ferro_impl_classes!(NavigationViewItemBase:
    StyledElementImpl, VisualImpl, LayoutableImpl, InteractiveImpl, InputElementImpl,
    ControlImpl, TemplatedControlImpl, ContentControlImpl
);

impl FerroObjectImpl for NavigationViewItemBase {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if change.is_effective_value_change() && change.property() == Self::is_selected_property().as_property() {
            this.pseudo_classes().set(":selected", this.is_selected());
        }
    }
}

ferro_properties! { impl NavigationViewItemBase {
    /// Defines the `IsSelected` property: whether the item is the selected item of its navigation
    /// view. `false` by default.
    pub fn is_selected_property() -> StyledProperty<bool> {
        FerroProperty::register::<NavigationViewItemBase, _>("IsSelected", false)
    }
} }

impl NavigationViewItemBase {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: ContentControl::construct() }
    }

    /// Gets whether the item is selected.
    pub fn is_selected(&self) -> bool {
        self.get_value(Self::is_selected_property())
    }

    /// Sets whether the item is selected.
    pub fn set_is_selected(&self, value: bool) {
        self.set_value(Self::is_selected_property(), value);
    }

    /// The state of the item in its navigation view: the compact pane length, whether the pane is
    /// closed in a compact mode (`IsClosedCompact`), and the depth of the item (`Depth`).
    pub(crate) fn set_navigation_context(&self, compact_length: f64, closed_compact: bool, depth: usize) {
        if let Some(item) = self.to_ref().cast::<NavigationViewItem>() {
            item.set_navigation_context(compact_length, closed_compact, depth);
        } else if let Some(header) = self.to_ref().cast::<NavigationViewItemHeader>() {
            header.closed_compact.set(closed_compact);
            header.depth.set(depth);
            header.update_visual_state();
        } else if let Some(separator) = self.to_ref().cast::<NavigationViewItemSeparator>() {
            separator.closed_compact.set(closed_compact);
            separator.depth.set(depth);
            separator.update_visual_state();
        }
    }

    /// Whether the item is in the top pane (WinUI's `Position` `TopPrimary` or `TopFooter`).
    pub(crate) fn set_top_navigation(&self, value: bool) {
        if let Some(item) = self.to_ref().cast::<NavigationViewItem>() {
            item.set_top_navigation(value);
        } else if let Some(header) = self.to_ref().cast::<NavigationViewItemHeader>() {
            header.top_navigation.set(value);
            header.update_visual_state();
        } else if let Some(separator) = self.to_ref().cast::<NavigationViewItemSeparator>() {
            separator.top_navigation.set(value);
            separator.update_visual_state();
        }
    }

    pub(crate) fn set_pane_open(&self, value: bool) {
        if let Some(item) = self.to_ref().cast::<NavigationViewItem>() {
            item.set_pane_open(value);
        }
    }
}

#[derive(Clone, Default)]
struct Parts {
    root_grid: Option<Ref<Panel>>,
    presenter: Option<Ref<Control>>,
    content_grid: Option<Ref<Control>>,
    chevron: Option<Ref<Control>>,
    selection_indicator: Option<Ref<Control>>,
    host: Option<Ref<StackPanel>>,
    flyout: Option<Ref<Popup>>,
    flyout_content: Option<Ref<Panel>>,
    subscriptions: Vec<Rc<dyn IDisposable>>,
}

impl Parts {
    fn detach(&mut self) {
        for subscription in self.subscriptions.drain(..) {
            subscription.dispose();
        }
        if let Some(flyout) = &self.flyout {
            flyout.set_is_open(false);
        }
        if let Some(host) = &self.host {
            host.children().clear();
        }
    }
}

/// A selectable item of a [`NavigationView`](crate::NavigationView), with an icon, an info badge
/// and child items (WinUI's `NavigationViewItem`).
///
/// The template of the WinUI theme draws the item as WinUI's item presenter does
/// (`NavigationViewItemPresenter` and its styles for the left and the top pane, folded into the
/// template of the item). The item sets the states of WinUI as pseudo-classes: `:selected`,
/// `:pressed`, `:on-top-navigation-primary`, `:icon-collapsed`, `:icon-only`, `:content-only`,
/// `:info-badge-collapsed`, `:chevron-visible-open`, `:chevron-visible-closed` and
/// `:closed-compact-and-top-level-item`. The child items are hosted in the template part
/// `NavigationViewItemMenuItemsHost`, below the item, or in the popup `ChildrenFlyout` when the item
/// is in the top pane or at the top level of a closed compact pane, as WinUI moves its repeater into
/// its flyout. The Fluent and Simple themes have no template.
///
/// The children are realized in a stack panel, not an `ItemsRepeater` (follow-up F1); the
/// containers of data items are reused across edits of the collections.
#[repr(C)]
pub struct NavigationViewItem {
    base: NavigationViewItemBase,
    menu_items: ItemCollection,
    menu_subscription: Cell<Option<u64>>,
    source_subscription: RefCell<Option<(Rc<ItemsSourceView>, u64)>>,
    children: RefCell<Vec<(Option<BoxedValue>, Ref<NavigationViewItemBase>)>>,
    child_template: RefCell<Option<Rc<dyn IDataTemplate>>>,
    applied_template: RefCell<Option<Rc<dyn IControlTemplate>>>,
    parts: RefCell<Parts>,
    compact_length: Cell<f64>,
    closed_compact: Cell<bool>,
    pane_open: Cell<bool>,
    restore_expanded: Cell<bool>,
    depth: Cell<usize>,
    top_navigation: Cell<bool>,
    repeater_in_flyout: Cell<bool>,
    suggested_tool_tip: RefCell<Option<BoxedValue>>,
    is_pressed: Cell<bool>,
    pressed_on_chevron: Cell<bool>,
    rebuilding: Cell<bool>,
    rebuild_pending: Cell<bool>,
    rebuild_force: Cell<bool>,
    updating: Cell<bool>,
    update_pending: Cell<bool>,
    moving_popup: Cell<bool>,
    invoked: HandlerList<dyn Fn()>,
    items_changed: HandlerList<dyn Fn()>,
}

ferro_class!(NavigationViewItem: NavigationViewItemBase);
ferro_class_info!(NavigationViewItem {
    new: NavigationViewItem::new,
    markup: {
        namespace: "FerroUI.Controls",
        content: Content,
        attributes: [
            TemplatePart("NVIRootGrid", type(Ref<Panel>)),
            TemplatePart("NavigationViewItemPresenter", type(Ref<Control>)),
            TemplatePart("ContentGrid", type(Ref<Control>)),
            TemplatePart("ExpandCollapseChevron", type(Ref<Control>)),
            TemplatePart("SelectionIndicator", type(Ref<Control>)),
            TemplatePart("NavigationViewItemMenuItemsHost", type(Ref<StackPanel>)),
            TemplatePart("ChildrenFlyout", type(Ref<Popup>)),
            TemplatePart("FlyoutContentGrid", type(Ref<Panel>)),
            PseudoClasses(":pressed", ":on-top-navigation-primary", ":icon-collapsed", ":icon-only", ":content-only",
                ":info-badge-collapsed", ":chevron-visible-open", ":chevron-visible-closed",
                ":closed-compact-and-top-level-item"),
        ],
    },
});
ferro_impl_classes!(NavigationViewItem: StyledElementImpl, InteractiveImpl, ContentControlImpl);

impl FerroObjectImpl for NavigationViewItem {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        let weak = this.to_ref().downgrade();
        let token = this.menu_items.add_collection_changed(Rc::new(move |_| {
            if let Some(owner) = weak.upgrade() {
                owner.rebuild_children();
            }
        }));
        this.menu_subscription.set(Some(token));
        this.update_visual_state();
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if !change.is_effective_value_change() {
            return;
        }
        let property = change.property();
        if property == Self::is_expanded_property().as_property() {
            // An explicit choice of the expansion supersedes the restoration after the pane closes;
            // ForceCollapse records the state after this callback returns.
            this.restore_expanded.set(false);
        }
        if property == Self::menu_items_source_property().as_property() {
            if let Some((source, token)) = this.source_subscription.take() {
                source.remove_collection_changed(token);
            }
            if let Some(source) = this.menu_items_source() {
                let view = ItemsSourceView::get_or_create(Some(&source));
                let weak = this.to_ref().downgrade();
                let token = view.add_collection_changed(Rc::new(move |_| {
                    if let Some(owner) = weak.upgrade() {
                        owner.rebuild_children();
                    }
                }));
                *this.source_subscription.borrow_mut() = Some((view, token));
            }
            this.rebuild_children();
        } else if property == ContentControl::content_property().as_property() {
            // OnContentChanged: the suggested tool tip is the content when it is a text.
            let content = this.content().filter(|content| {
                content.downcast_ref::<String>().is_some_and(|text| !text.is_empty())
                    || content.downcast_ref::<&str>().is_some_and(|text| !text.is_empty())
            });
            *this.suggested_tool_tip.borrow_mut() = content;
            this.update_parts();
        } else if property == Self::is_expanded_property().as_property()
            || property == Self::has_unrealized_children_property().as_property()
            || property == Self::is_child_selected_property().as_property()
            || property == NavigationViewItemBase::is_selected_property().as_property()
            || property == Self::icon_property().as_property()
            || property == Self::info_badge_property().as_property()
            || property == InputElement::is_enabled_property().as_property()
        {
            this.update_parts();
        }
    }
}

impl VisualImpl for NavigationViewItem {
    fn on_attached_to_visual_tree(this: &Self, args: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, args);
        this.update_parts();
    }

    fn on_detached_from_visual_tree(this: &Self, args: &VisualTreeAttachmentEventArgs) {
        let parts = this.parts.borrow().clone();
        if let Some(flyout) = parts.flyout {
            flyout.set_is_open(false);
        }
        Self::parent_on_detached_from_visual_tree(this, args);
    }
}

impl ControlImpl for NavigationViewItem {
    fn on_create_automation_peer(this: &Self) -> Ref<AutomationPeer> {
        NavigationViewItemAutomationPeer::new(this).upcast()
    }
}

impl LayoutableImpl for NavigationViewItem {
    fn apply_template(this: &Self) {
        let template = this.template();
        let applied = this.applied_template.borrow().clone();
        if applied != template && (template.is_some() || this.is_attached_to_logical_tree()) {
            // The templated control removes the old tree before OnApplyTemplate and does not call
            // it for no template: the handlers of the old parts are revoked first, so that they
            // cannot change the state of the item afterwards.
            let previous = this.moving_popup.replace(true);
            this.parts.take().detach();
            this.repeater_in_flyout.set(false);
            Self::parent_apply_template(this);
            *this.applied_template.borrow_mut() = template;
            this.moving_popup.set(previous);
        } else {
            Self::parent_apply_template(this);
        }
    }
}

impl InputElementImpl for NavigationViewItem {
    fn on_key_down(this: &Self, args: &KeyEventArgs) {
        Self::parent_on_key_down(this, args);
        if args.handled() || !this.is_effectively_enabled() || !this.is_own_event(args.source()) {
            return;
        }
        if args.key == Key::Enter || args.key == Key::Space && !args.key_modifiers.contains(KeyModifiers::ALT) {
            this.invoke();
            args.set_handled(true);
            return;
        }
        // NavigationView moves between the top level items of the top pane with the horizontal
        // arrows; the items in their flyouts keep the hierarchical arrows.
        if this.top_navigation.get() && (args.key == Key::Left || args.key == Key::Right) {
            return;
        }
        let rtl = this.flow_direction() == FlowDirection::RightToLeft;
        let expand = if rtl { Key::Left } else { Key::Right };
        let collapse = if rtl { Key::Right } else { Key::Left };
        if args.key == expand && this.has_children() {
            if !this.is_expanded() {
                this.set_current_value(Self::is_expanded_property(), true);
                args.set_handled(true);
            } else if let Some(child) = this.child_containers().into_iter().map(|(_, child)| child).find(|child| {
                child.cast::<NavigationViewItem>().is_some()
                    && child.focusable()
                    && child.is_visible()
                    && child.is_effectively_enabled()
            }) {
                args.set_handled(child.focus_with(NavigationMethod::Directional, args.key_modifiers));
            }
        } else if (args.key == collapse || args.key == Key::Escape) && this.is_expanded() {
            this.set_current_value(Self::is_expanded_property(), false);
            args.set_handled(true);
        }
    }

    /// `OnPresenterPointerPressed` and the presenter's `OnExpandCollapseChevronPointerPressed`: the
    /// primary button presses the item (or its chevron).
    fn on_pointer_pressed(this: &Self, e: &PointerPressedEventArgs) {
        Self::parent_on_pointer_pressed(this, e);
        if e.handled() || !this.is_effectively_enabled() || !this.is_own_event(e.source()) {
            return;
        }
        if !e.get_current_point(Some(this)).properties.is_left_button_pressed {
            return;
        }
        this.pressed_on_chevron.set(this.is_in_chevron(e.source()));
        this.set_pressed(true);
        e.set_handled(true);
    }

    /// `OnPresenterPointerReleased`: a release over the item invokes it
    /// (`NavigationView::OnNavigationViewItemClicked`), a release over the chevron that was pressed
    /// toggles the expansion (`OnExpandCollapseChevronPointerReleased`).
    fn on_pointer_released(this: &Self, e: &PointerReleasedEventArgs) {
        Self::parent_on_pointer_released(this, e);
        if !this.is_pressed.get() || e.initial_press_mouse_button() != MouseButton::Left {
            return;
        }
        this.set_pressed(false);
        let this_visual: &Visual = this;
        let is_over = this
            .get_visuals_at(e.get_position(Some(this)))
            .iter()
            .any(|visual| std::ptr::eq::<Visual>(&**visual, this_visual) || this.is_visual_ancestor_of(visual));
        if is_over {
            if this.pressed_on_chevron.get() && this.is_in_chevron(e.source()) {
                if this.has_children() {
                    this.set_current_value(Self::is_expanded_property(), !this.is_expanded());
                }
            } else {
                this.invoke();
                this.focus_with(NavigationMethod::Pointer, KeyModifiers::NONE);
            }
            e.set_handled(true);
        }
        this.pressed_on_chevron.set(false);
    }

    fn on_pointer_capture_lost(this: &Self, e: &PointerCaptureLostEventArgs) {
        Self::parent_on_pointer_capture_lost(this, e);
        this.set_pressed(false);
        this.pressed_on_chevron.set(false);
    }
}

impl TemplatedControlImpl for NavigationViewItem {
    fn on_apply_template(this: &Self, args: &TemplateAppliedEventArgs) {
        let previous = this.moving_popup.replace(true);
        this.parts.take().detach();
        this.repeater_in_flyout.set(false);
        this.moving_popup.set(previous);
        Self::parent_on_apply_template(this, args);
        let scope = args.name_scope();
        let mut parts = Parts {
            root_grid: scope.find_as("NVIRootGrid"),
            presenter: scope.find_as("NavigationViewItemPresenter"),
            content_grid: scope.find_as("ContentGrid"),
            chevron: scope.find_as("ExpandCollapseChevron"),
            selection_indicator: scope.find_as("SelectionIndicator"),
            host: scope.find_as("NavigationViewItemMenuItemsHost"),
            flyout: scope.find_as("ChildrenFlyout"),
            flyout_content: scope.find_as("FlyoutContentGrid"),
            ..Parts::default()
        };
        if let Some(flyout) = &parts.flyout {
            // OnFlyoutClosing: a flyout that closes (a light dismissal) collapses the item.
            let weak = this.to_ref().downgrade();
            parts.subscriptions.push(flyout.closed(move || {
                if let Some(owner) = weak.upgrade() {
                    if !owner.moving_popup.get() && owner.is_expanded() {
                        owner.set_current_value(Self::is_expanded_property(), false);
                    }
                }
            }));
        }
        *this.parts.borrow_mut() = parts;
        this.realize_children();
        this.update_parts();
    }
}

ferro_properties! { impl NavigationViewItem {
    /// Defines the `Icon` property: the icon of the item.
    pub fn icon_property() -> StyledProperty<Option<Ref<IconElement>>> {
        FerroProperty::register::<NavigationViewItem, _>("Icon", None)
    }
    /// Defines the `InfoBadge` property: the badge shown beside the content of the item.
    pub fn info_badge_property() -> StyledProperty<Option<Ref<InfoBadge>>> {
        FerroProperty::register::<NavigationViewItem, _>("InfoBadge", None)
    }
    /// Defines the `CompactPaneLength` property: the compact pane length of the navigation view
    /// (read-only).
    pub fn compact_pane_length_property() -> DirectProperty<NavigationViewItem, f64> {
        FerroProperty::register_direct::<NavigationViewItem, _>("CompactPaneLength", |owner| owner.compact_pane_length(), None, 48.0)
    }
    /// Defines the `SelectsOnInvoked` property: whether invoking the item selects it. `true` by
    /// default.
    pub fn selects_on_invoked_property() -> StyledProperty<bool> {
        FerroProperty::register::<NavigationViewItem, _>("SelectsOnInvoked", true)
    }
    /// Defines the `IsExpanded` property: whether the child items are shown.
    pub fn is_expanded_property() -> StyledProperty<bool> {
        FerroProperty::register::<NavigationViewItem, _>("IsExpanded", false)
    }
    /// Defines the `HasUnrealizedChildren` property: whether the item has children that the
    /// application adds when it expands.
    pub fn has_unrealized_children_property() -> StyledProperty<bool> {
        FerroProperty::register::<NavigationViewItem, _>("HasUnrealizedChildren", false)
    }
    /// Defines the `IsChildSelected` property: whether a descendant item is selected.
    pub fn is_child_selected_property() -> StyledProperty<bool> {
        FerroProperty::register::<NavigationViewItem, _>("IsChildSelected", false)
    }
    /// Defines the `MenuItems` property: the child items (read-only; the collection is writable).
    pub fn menu_items_property() -> DirectProperty<NavigationViewItem, ItemCollection> {
        FerroProperty::register_direct::<NavigationViewItem, _>("MenuItems", |owner| owner.menu_items(), None, ItemCollection::new())
    }
    /// Defines the `MenuItemsSource` property: the source of the child items, which takes
    /// precedence over `MenuItems` while it is set.
    pub fn menu_items_source_property() -> StyledProperty<Option<ItemsSource>> {
        FerroProperty::register::<NavigationViewItem, _>("MenuItemsSource", None)
    }
} }

/// Logs a child of a navigation item that the item does not take.
fn log_rejected_child(message: &str) {
    if let Some(logger) = Logger::try_get(LogEventLevel::Warning, LogArea::CONTROL) {
        logger.log(None, message);
    }
}

impl NavigationViewItem {
    fn static_constructor() {
        InputElement::focusable_property().override_default_value::<NavigationViewItem>(true);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: NavigationViewItemBase::construct(),
            menu_items: ItemCollection::new(),
            menu_subscription: Cell::new(None),
            source_subscription: RefCell::new(None),
            children: RefCell::new(Vec::new()),
            child_template: RefCell::new(None),
            applied_template: RefCell::new(None),
            parts: RefCell::new(Parts::default()),
            compact_length: Cell::new(48.0),
            closed_compact: Cell::new(false),
            pane_open: Cell::new(true),
            restore_expanded: Cell::new(false),
            depth: Cell::new(0),
            top_navigation: Cell::new(false),
            repeater_in_flyout: Cell::new(false),
            suggested_tool_tip: RefCell::new(None),
            is_pressed: Cell::new(false),
            pressed_on_chevron: Cell::new(false),
            rebuilding: Cell::new(false),
            rebuild_pending: Cell::new(false),
            rebuild_force: Cell::new(false),
            updating: Cell::new(false),
            update_pending: Cell::new(false),
            moving_popup: Cell::new(false),
            invoked: HandlerList::new(),
            items_changed: HandlerList::new(),
        }
    }

    /// Creates an item.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets the icon.
    pub fn icon(&self) -> Option<Ref<IconElement>> {
        self.get_value(Self::icon_property())
    }

    /// Sets the icon.
    pub fn set_icon(&self, value: Option<Ref<IconElement>>) {
        self.set_value(Self::icon_property(), value);
    }

    /// Gets the info badge.
    pub fn info_badge(&self) -> Option<Ref<InfoBadge>> {
        self.get_value(Self::info_badge_property())
    }

    /// Sets the info badge.
    pub fn set_info_badge(&self, value: Option<Ref<InfoBadge>>) {
        self.set_value(Self::info_badge_property(), value);
    }

    /// Gets the compact pane length of the navigation view of the item.
    pub fn compact_pane_length(&self) -> f64 {
        self.compact_length.get()
    }

    /// Gets whether invoking the item selects it.
    pub fn selects_on_invoked(&self) -> bool {
        self.get_value(Self::selects_on_invoked_property())
    }

    /// Sets whether invoking the item selects it.
    pub fn set_selects_on_invoked(&self, value: bool) {
        self.set_value(Self::selects_on_invoked_property(), value);
    }

    /// Gets whether the child items are shown.
    pub fn is_expanded(&self) -> bool {
        self.get_value(Self::is_expanded_property())
    }

    /// Sets whether the child items are shown.
    pub fn set_is_expanded(&self, value: bool) {
        self.set_value(Self::is_expanded_property(), value);
    }

    /// Gets whether the item has children the application adds when it expands.
    pub fn has_unrealized_children(&self) -> bool {
        self.get_value(Self::has_unrealized_children_property())
    }

    /// Sets whether the item has children the application adds when it expands.
    pub fn set_has_unrealized_children(&self, value: bool) {
        self.set_value(Self::has_unrealized_children_property(), value);
    }

    /// Gets whether a descendant item is selected.
    pub fn is_child_selected(&self) -> bool {
        self.get_value(Self::is_child_selected_property())
    }

    /// Sets whether a descendant item is selected.
    pub fn set_is_child_selected(&self, value: bool) {
        self.set_value(Self::is_child_selected_property(), value);
    }

    /// Gets the child items (writable, also while `MenuItemsSource` is set, as in WinUI).
    pub fn menu_items(&self) -> ItemCollection {
        self.menu_items.clone()
    }

    /// Gets the source of the child items.
    pub fn menu_items_source(&self) -> Option<ItemsSource> {
        self.get_value(Self::menu_items_source_property())
    }

    /// Sets the source of the child items.
    pub fn set_menu_items_source(&self, value: Option<ItemsSource>) {
        self.set_value(Self::menu_items_source_property(), value);
    }

    pub(crate) fn invoked(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        let token = self.invoked.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(owner) = weak.upgrade() {
                owner.invoked.remove(token);
            }
        })
    }

    pub(crate) fn items_changed(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        let token = self.items_changed.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(owner) = weak.upgrade() {
                owner.items_changed.remove(token);
            }
        })
    }

    /// Invokes the item: its navigation view selects it or raises `ItemInvoked`.
    pub(crate) fn invoke(&self) {
        if self.is_effectively_enabled() {
            for (_, handler) in self.invoked.snapshot().iter() {
                handler();
            }
        }
    }

    pub(crate) fn child_containers(&self) -> Vec<(Option<BoxedValue>, Ref<NavigationViewItemBase>)> {
        self.children.borrow().clone()
    }

    /// `HasChildren`: child items, realized or not; the chevron and automation use it.
    pub(crate) fn has_children(&self) -> bool {
        self.has_unrealized_children() || self.menu_items.count() > 0 || !self.children.borrow().is_empty()
    }

    /// `NavigationView::DoesNavigationViewItemHaveChildren`: an active source takes precedence.
    pub(crate) fn has_children_for_navigation(&self) -> bool {
        if self.menu_items_source().is_some() {
            self.source_subscription.borrow().as_ref().is_some_and(|(source, _)| source.count() > 0)
        } else {
            self.menu_items.count() > 0 || self.has_unrealized_children()
        }
    }

    pub(crate) fn set_navigation_context(&self, compact_length: f64, closed_compact: bool, depth: usize) {
        let old_compact = self.closed_compact.replace(closed_compact);
        let changed = self.set_and_raise_cell(Self::compact_pane_length_property(), &self.compact_length, compact_length)
            | (old_compact != closed_compact)
            | (self.depth.replace(depth) != depth);
        if changed {
            self.update_parts();
        }
    }

    /// `ForceCollapse` and `RestoreExpandedState`: an item collapses while the pane closes and
    /// expands again when it opens.
    pub(crate) fn set_pane_open(&self, value: bool) {
        if self.pane_open.replace(value) == value {
            return;
        }
        if value {
            if self.restore_expanded.replace(false) {
                self.set_current_value(Self::is_expanded_property(), true);
            }
        } else if self.is_expanded() {
            self.set_current_value(Self::is_expanded_property(), false);
            self.restore_expanded.set(true);
        }
    }

    pub(crate) fn set_top_navigation(&self, value: bool) {
        if value {
            self.restore_expanded.set(false);
        }
        if self.top_navigation.replace(value) != value {
            self.update_parts();
        }
    }

    pub(crate) fn is_top_navigation(&self) -> bool {
        self.top_navigation.get()
    }

    pub(crate) fn set_menu_item_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        let equal = match (&*self.child_template.borrow(), &value) {
            (None, None) => true,
            (Some(old), Some(new)) => Rc::ptr_eq(old, new),
            _ => false,
        };
        if !equal {
            *self.child_template.borrow_mut() = value;
            self.rebuild_force.set(true);
            self.rebuild_children();
        }
    }

    /// `ShouldRepeaterShowInFlyout`: the children are in a flyout for an item of the top pane and
    /// for a top level item of a closed compact pane.
    pub(crate) fn in_flyout(&self) -> bool {
        self.top_navigation.get() || self.closed_compact.get() && self.depth.get() == 0
    }

    /// Whether a routed event comes from this item and not from a child item it hosts.
    fn is_own_event(&self, source: Option<Ref<ferroui_base::FerroObject>>) -> bool {
        let Some(source) = source.and_then(|source| source.cast::<Visual>()) else { return true };
        match source.find_ancestor_of_type::<NavigationViewItem>(true) {
            Some(item) => item == self.to_ref(),
            None => true,
        }
    }

    /// Whether the source of an event is the chevron part or inside it.
    fn is_in_chevron(&self, source: Option<Ref<ferroui_base::FerroObject>>) -> bool {
        let chevron = self.parts.borrow().chevron.clone();
        let (Some(chevron), Some(source)) = (chevron, source.and_then(|source| source.cast::<Visual>())) else {
            return false;
        };
        let chevron: Ref<Visual> = chevron.upcast();
        chevron == source || chevron.is_visual_ancestor_of(&source)
    }

    fn set_pressed(&self, value: bool) {
        if self.is_pressed.replace(value) != value {
            self.pseudo_classes().set(":pressed", value);
        }
    }

    fn rebuild_children(&self) {
        if self.rebuilding.replace(true) {
            self.rebuild_pending.set(true);
            return;
        }
        struct Reset<'a>(&'a Cell<bool>);
        impl Drop for Reset<'_> {
            fn drop(&mut self) {
                self.0.set(false);
            }
        }
        let _reset = Reset(&self.rebuilding);
        loop {
            self.rebuild_pending.set(false);
            let force = self.rebuild_force.replace(false);
            let view = self
                .source_subscription
                .borrow()
                .as_ref()
                .map(|(source, _)| source.clone())
                .unwrap_or_else(|| self.menu_items.view().clone());
            let items: Vec<_> = (0..view.count()).map(|index| view.get_at(index)).collect();
            let mut previous = self.children.take();
            let mut children = Vec::with_capacity(items.len());
            let template = self.child_template.borrow().clone();
            for data in items {
                let explicit = data
                    .as_ref()
                    .and_then(Control::from_boxed)
                    .and_then(|control| control.cast::<NavigationViewItemBase>());
                let reusable =
                    if force { None } else { previous.iter().position(|(item, _)| items_equal(item, &data)) };
                let container = if let Some(container) = explicit {
                    container
                } else if let Some(index) = reusable {
                    previous.remove(index).1
                } else {
                    let built = template.as_ref().and_then(|template| template.build(&data));
                    let container =
                        built.as_ref().and_then(|control| control.cast::<NavigationViewItemBase>()).unwrap_or_else(
                            || {
                                let child = NavigationViewItem::new();
                                child.set_content(built.map(Control::boxed).or_else(|| data.clone()));
                                child.upcast()
                            },
                        );
                    // As the containers of an items control and of the root items of NavigationView,
                    // a control item keeps the data context it inherits.
                    if data.as_ref().and_then(Control::from_boxed).is_none() {
                        container.set_data_context(data.clone());
                    }
                    container
                };
                if !self.validate_child_hierarchy(&container) {
                    continue;
                }
                if let Some(child) = container.cast::<NavigationViewItem>() {
                    child.set_menu_item_template(template.clone());
                }
                children.push((data, container));
            }
            *self.children.borrow_mut() = children;
            self.realize_children();
            self.update_parts();
            for (_, handler) in self.items_changed.snapshot().iter() {
                handler();
            }
            if !self.rebuild_pending.get() {
                break;
            }
        }
    }

    /// Whether `child` may be a child of the item: an item cannot contain itself or one of its
    /// ancestors. Checked before the depth is propagated or a data template is applied, since a
    /// cycle can be built before anything is in the visual tree; a rejected child is left out and
    /// logged (#286 panicked).
    fn validate_child_hierarchy(&self, child: &Ref<NavigationViewItemBase>) -> bool {
        let owner = self.to_ref().upcast::<NavigationViewItemBase>();
        let mut pending = vec![child.clone()];
        let mut visited: Vec<Ref<NavigationViewItemBase>> = Vec::new();
        while let Some(candidate) = pending.pop() {
            if candidate == owner {
                log_rejected_child("A NavigationViewItem cannot contain itself or an ancestor; the child is left out.");
                return false;
            }
            if visited.iter().any(|seen| seen.ptr_eq(&candidate)) {
                continue;
            }
            if let Some(item) = candidate.cast::<NavigationViewItem>() {
                pending.extend(item.child_containers().into_iter().map(|(_, child)| child));
            }
            visited.push(candidate);
        }
        true
    }

    fn realize_children(&self) {
        let host = self.parts.borrow().host.clone();
        if let Some(host) = host {
            let children = self.child_containers();
            let current = host.children().snapshot();
            if current.len() != children.len()
                || current.iter().zip(&children).any(|(current, (_, child))| *current != child.clone().upcast::<Control>())
            {
                host.children().clear();
                for (_, child) in children {
                    host.children().add(child.upcast::<Control>());
                }
            }
        }
    }

    fn update_parts(&self) {
        if self.updating.replace(true) {
            self.update_pending.set(true);
            return;
        }
        struct Reset<'a>(&'a Cell<bool>);
        impl Drop for Reset<'_> {
            fn drop(&mut self) {
                self.0.set(false);
            }
        }
        let _reset = Reset(&self.updating);
        loop {
            self.update_pending.set(false);
            self.update_visual_state();
            self.update_children_host();
            if !self.update_pending.get() {
                break;
            }
        }
    }

    /// `UpdateVisualState`: the states of WinUI as pseudo-classes, the indentation of the content
    /// grid, the selection indicator and the tool tip of a closed compact item.
    fn update_visual_state(&self) {
        let classes = self.pseudo_classes();
        let top = self.top_navigation.get();
        let closed_compact_top_level = self.closed_compact.get() && self.depth.get() == 0;
        let show_icon = self.icon().is_some();
        let show_content = self.content().is_some();
        let has_children = self.has_children();
        let expanded = self.is_expanded();
        // UpdateVisualStateForNavigationViewPositionChange.
        classes.set(":on-top-navigation-primary", top);
        // IconStates (the left pane) and NavigationViewIconPositionStates (the top pane).
        classes.set(":icon-collapsed", !show_icon);
        classes.set(":icon-only", show_icon && !show_content);
        classes.set(":content-only", !show_icon);
        // InfoBadgeStates.
        classes.set(":info-badge-collapsed", self.info_badge().is_none());
        // ChevronStates: no chevron for a top level item of a closed compact pane.
        let chevron = has_children && !(self.closed_compact.get() && self.in_flyout());
        classes.set(":chevron-visible-open", chevron && expanded);
        classes.set(":chevron-visible-closed", chevron && !expanded);
        // PaneAndTopLevelItemStates.
        classes.set(":closed-compact-and-top-level-item", !top && closed_compact_top_level);

        let parts = self.parts.borrow().clone();
        // UpdateItemIndentation: an item in a flyout is at depth 0.
        indent(&parts.content_grid, if top { 0 } else { self.depth.get() });
        // ShowSelectionIndicator: the indicator of the selected item, or of the collapsed
        // ancestor of a selected child.
        if let Some(indicator) = &parts.selection_indicator {
            let visible = self.is_selected() || self.is_child_selected() && !expanded;
            indicator.set_opacity(if visible { 1.0 } else { 0.0 });
        }
        self.update_tool_tip();
    }

    /// `UpdateNavigationViewItemToolTip`: a closed compact item of the left pane shows its text
    /// content as its tool tip, unless the application gave it one.
    fn update_tool_tip(&self) {
        let suggested = self.suggested_tool_tip.borrow().clone();
        let current = ToolTip::get_tip(self);
        let own = match (&current, &suggested) {
            (None, _) => true,
            (Some(current), Some(suggested)) => Rc::ptr_eq(current, suggested),
            _ => false,
        };
        if own {
            let enable = !self.top_navigation.get() && self.closed_compact.get();
            let tip = if enable { suggested } else { None };
            let unchanged = match (&current, &tip) {
                (None, None) => true,
                (Some(a), Some(b)) => Rc::ptr_eq(a, b),
                _ => false,
            };
            if !unchanged {
                ToolTip::set_tip(self, tip);
            }
        }
    }

    /// `ShowHideChildren` and `ReparentRepeater`: the host of the children below the item or in the
    /// flyout, and the depth of the children.
    fn update_children_host(&self) {
        let parts = self.parts.borrow().clone();
        let flyout_mode = self.in_flyout();
        let expanded = self.is_expanded();
        if let (Some(root), Some(host), Some(flyout), Some(content)) =
            (&parts.root_grid, &parts.host, &parts.flyout, &parts.flyout_content)
        {
            let previous = self.moving_popup.replace(true);
            let host_control: Ref<Control> = host.clone().upcast();
            if flyout_mode && !self.repeater_in_flyout.get() {
                root.children().remove(&host_control);
                content.children().add(host_control.clone());
                self.repeater_in_flyout.set(true);
            } else if !flyout_mode && self.repeater_in_flyout.get() {
                flyout.set_is_open(false);
                content.children().remove(&host_control);
                root.children().add(host_control.clone());
                self.repeater_in_flyout.set(false);
            }
            host.set_is_visible(expanded);
            flyout.set_is_open(flyout_mode && expanded && self.has_children() && self.is_attached_to_visual_tree());
            self.moving_popup.set(previous);
        } else if let Some(host) = &parts.host {
            host.set_is_visible(expanded);
        }
        let depth = if flyout_mode { 0 } else { self.depth.get() + 1 };
        for (_, container) in self.child_containers() {
            container.set_navigation_context(self.compact_pane_length(), false, depth);
        }
    }
}

impl Drop for NavigationViewItem {
    fn drop(&mut self) {
        if let Some(token) = self.menu_subscription.take() {
            self.menu_items.remove_collection_changed(token);
        }
        if let Some((view, token)) = self.source_subscription.get_mut().take() {
            view.remove_collection_changed(token);
        }
        self.parts.get_mut().detach();
    }
}

/// A heading of a group of items of a [`NavigationView`](crate::NavigationView) (WinUI's
/// `NavigationViewItemHeader`). It is not focusable or enabled, as WinUI's style sets.
///
/// The control sets WinUI's states as pseudo-classes: `:header-text-collapsed` (a top level header
/// of a closed compact pane) and `:top-mode` (the top pane). The template of the WinUI theme
/// presents the content in the text block `HeaderText`; the part
/// `NavigationViewItemHeaderRootGrid` is indented by the depth of the header.
#[repr(C)]
pub struct NavigationViewItemHeader {
    base: NavigationViewItemBase,
    root_grid: RefCell<Option<Ref<Control>>>,
    closed_compact: Cell<bool>,
    top_navigation: Cell<bool>,
    depth: Cell<usize>,
}

ferro_class!(NavigationViewItemHeader: NavigationViewItemBase);
ferro_class_info!(NavigationViewItemHeader {
    new: NavigationViewItemHeader::new,
    markup: {
        namespace: "FerroUI.Controls",
        content: Content,
        attributes: [
            TemplatePart("NavigationViewItemHeaderRootGrid", type(Ref<Control>)),
            PseudoClasses(":header-text-collapsed", ":top-mode"),
        ],
    },
});
ferro_impl_classes!(NavigationViewItemHeader: FerroObjectImpl, StyledElementImpl, VisualImpl,
    LayoutableImpl, InteractiveImpl, InputElementImpl, ControlImpl, ContentControlImpl);

impl TemplatedControlImpl for NavigationViewItemHeader {
    fn on_apply_template(this: &Self, args: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, args);
        *this.root_grid.borrow_mut() = args.name_scope().find_as("NavigationViewItemHeaderRootGrid");
        this.update_visual_state();
    }
}

impl NavigationViewItemHeader {
    fn static_constructor() {
        InputElement::focusable_property().override_default_value::<NavigationViewItemHeader>(false);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: NavigationViewItemBase::construct(),
            root_grid: RefCell::new(None),
            closed_compact: Cell::new(false),
            top_navigation: Cell::new(false),
            depth: Cell::new(0),
        }
    }

    /// Creates a header.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// `UpdateVisualState` and `UpdateItemIndentation`.
    fn update_visual_state(&self) {
        let classes = self.pseudo_classes();
        classes.set(":header-text-collapsed", self.closed_compact.get() && self.depth.get() == 0);
        classes.set(":top-mode", self.top_navigation.get());
        indent(&self.root_grid.borrow(), self.depth.get());
    }
}

/// A line between items of a [`NavigationView`](crate::NavigationView) (WinUI's
/// `NavigationViewItemSeparator`). It is not focusable or enabled, as WinUI's style sets.
///
/// The control sets WinUI's states as pseudo-classes: `:horizontal-line-compact` (a closed compact
/// pane) and `:vertical-line` (the top pane); the part `NavigationViewItemSeparatorRootGrid` is
/// indented by the depth of the separator.
#[repr(C)]
pub struct NavigationViewItemSeparator {
    base: NavigationViewItemBase,
    root_grid: RefCell<Option<Ref<Control>>>,
    closed_compact: Cell<bool>,
    top_navigation: Cell<bool>,
    depth: Cell<usize>,
}

ferro_class!(NavigationViewItemSeparator: NavigationViewItemBase);
ferro_class_info!(NavigationViewItemSeparator {
    new: NavigationViewItemSeparator::new,
    markup: {
        namespace: "FerroUI.Controls",
        attributes: [
            TemplatePart("NavigationViewItemSeparatorRootGrid", type(Ref<Control>)),
            PseudoClasses(":horizontal-line-compact", ":vertical-line"),
        ],
    },
});
ferro_impl_classes!(NavigationViewItemSeparator: FerroObjectImpl, StyledElementImpl, VisualImpl,
    LayoutableImpl, InteractiveImpl, InputElementImpl, ControlImpl, ContentControlImpl);

impl TemplatedControlImpl for NavigationViewItemSeparator {
    fn on_apply_template(this: &Self, args: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, args);
        *this.root_grid.borrow_mut() = args.name_scope().find_as("NavigationViewItemSeparatorRootGrid");
        this.update_visual_state();
    }
}

impl NavigationViewItemSeparator {
    fn static_constructor() {
        InputElement::focusable_property().override_default_value::<NavigationViewItemSeparator>(false);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: NavigationViewItemBase::construct(),
            root_grid: RefCell::new(None),
            closed_compact: Cell::new(false),
            top_navigation: Cell::new(false),
            depth: Cell::new(0),
        }
    }

    /// Creates a separator.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// `UpdateVisualState` (`NavigationSeparatorLineStates`) and `UpdateItemIndentation`.
    fn update_visual_state(&self) {
        let classes = self.pseudo_classes();
        let top = self.top_navigation.get();
        classes.set(":vertical-line", top);
        classes.set(":horizontal-line-compact", !top && self.closed_compact.get());
        indent(&self.root_grid.borrow(), self.depth.get());
    }
}

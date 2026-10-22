// Copyright (c) Microsoft Corporation. Licensed under the MIT License. See NOTICE.md.
// Adapted from NavigationView.idl and NavigationViewItem*.cpp at WinUI a71c42bd.

use crate::automation::peers::NavigationViewItemAutomationPeer;
use crate::InfoBadge;
use crate::automation::peers::AutomationPeer;
use crate::automation::{AccessibilityView, AutomationProperties};
use crate::items_source::items_equal;
use crate::presenters::ContentPresenter;
use crate::primitives::{
    Popup, TemplateAppliedEventArgs, TemplatedControl, TemplatedControlImpl,
    TemplatedControlImplExt,
};
use crate::templates::{
    FuncControlTemplate, FuncTemplateNameScopeExtensions, IControlTemplate, IDataTemplate,
};
use crate::{
    Border, Button, ContentControl, ContentControlImpl, Control, ControlImpl, Grid, IconElement,
    ItemCollection, ItemsSource, ItemsSourceView, PlacementMode, StackPanel, TextBlock, ToolTip,
};
use ferroui_base::controls::ResourceKey;
use ferroui_base::data::TemplateBinding;
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::input::{
    InputElement, InputElementImpl, InputElementImplExt, Key, KeyEventArgs, KeyModifiers,
    NavigationMethod,
};
use ferroui_base::interactivity::{InteractiveImpl, RoutedEventHandlerToken};
use ferroui_base::layout::{
    HorizontalAlignment, LayoutableImpl, LayoutableImplExt, VerticalAlignment,
};
use ferroui_base::media::{Brushes, FlowDirection, FontWeight, IBrush, SolidColorBrush};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::styling::ThemeVariant;
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    BoxedValue, CornerRadius, DirectProperty, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, Ref, StyledElementImpl, StyledProperty, Thickness, Visual,
    VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs, WeakRef, ferro_class,
    ferro_class_info, ferro_impl_classes, ferro_properties, instantiate,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The shared content and selection contract of NavigationView items.
/// Like the source base class, it has no public default constructor in markup.
#[repr(C)]
pub struct NavigationViewItemBase {
    base: ContentControl,
}
ferro_class!(NavigationViewItemBase: ContentControl);
ferro_class_info!(NavigationViewItemBase {
    markup: { namespace: "FerroUI.Controls", content: Content },
});
ferro_impl_classes!(NavigationViewItemBase:
    StyledElementImpl, VisualImpl, LayoutableImpl, InteractiveImpl, InputElementImpl,
    ControlImpl, TemplatedControlImpl, ContentControlImpl
);
impl FerroObjectImpl for NavigationViewItemBase {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if change.is_effective_value_change()
            && change.property() == Self::is_selected_property().as_property()
        {
            this.pseudo_classes().set(":selected", this.is_selected());
        }
    }
}
ferro_properties! { impl NavigationViewItemBase {
    pub fn is_selected_property() -> StyledProperty<bool> {
        FerroProperty::register::<NavigationViewItemBase, _>("IsSelected", false)
    }
} }
impl NavigationViewItemBase {
    pub fn construct() -> Self {
        Self {
            base: ContentControl::construct(),
        }
    }
    pub fn is_selected(&self) -> bool {
        self.get_value(Self::is_selected_property())
    }
    pub fn set_is_selected(&self, value: bool) {
        self.set_value(Self::is_selected_property(), value);
    }
    pub(crate) fn set_navigation_context(
        &self,
        compact_length: f64,
        closed_compact: bool,
        depth: usize,
    ) {
        if let Some(item) = self.to_ref().cast::<NavigationViewItem>() {
            item.set_navigation_context(compact_length, closed_compact, depth);
        } else if let Some(header) = self.to_ref().cast::<NavigationViewItemHeader>() {
            header.closed_compact.set(closed_compact && depth == 0);
            header.depth.set(depth);
            header.update_presentation();
        } else if let Some(separator) = self.to_ref().cast::<NavigationViewItemSeparator>() {
            separator.closed_compact.set(closed_compact && depth == 0);
            separator.depth.set(depth);
            separator.update_presentation();
        }
    }
    pub(crate) fn set_top_navigation(&self, value: bool) {
        if let Some(item) = self.to_ref().cast::<NavigationViewItem>() {
            item.set_top_navigation(value);
        } else if let Some(separator) = self.to_ref().cast::<NavigationViewItemSeparator>() {
            separator.top_navigation.set(value);
            separator.update_presentation();
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
    row: Option<Ref<Button>>,
    content: Option<Ref<ContentPresenter>>,
    icon: Option<Ref<ContentPresenter>>,
    badge: Option<Ref<ContentPresenter>>,
    chevron: Option<Ref<Button>>,
    chevron_glyph: Option<Ref<TextBlock>>,
    selection: Option<Ref<Border>>,
    inline_host: Option<Ref<Border>>,
    popup: Option<Ref<Popup>>,
    children: Option<Ref<StackPanel>>,
    clicks: Vec<(WeakRef<Button>, RoutedEventHandlerToken)>,
    subscriptions: Vec<Rc<dyn IDisposable>>,
}
impl Parts {
    fn detach(&mut self) {
        for (button, token) in self.clicks.drain(..) {
            if let Some(button) = button.upgrade() {
                button.remove_handler(Button::click_event(), token);
            }
        }
        for subscription in self.subscriptions.drain(..) {
            subscription.dispose();
        }
        if let Some(popup) = &self.popup {
            popup.set_is_open(false);
            popup.set_child(None::<Ref<Control>>);
        }
        if let Some(host) = &self.inline_host {
            host.set_child(None::<Ref<Control>>);
        }
        if let Some(children) = &self.children {
            children.children().clear();
        }
        for presenter in [&self.content, &self.icon, &self.badge]
            .into_iter()
            .flatten()
        {
            // A data template is allowed to build a visual for null content.
            // Remove that factory before clearing the old presenter's content.
            presenter.set_content_template(None);
            presenter.set_content(None);
        }
    }
}

/// A native selectable navigation row, optional live InfoBadge, and expandable hierarchy.
///
/// Items and sources use the existing native collection contract. Explicit items remain
/// writable while MenuItemsSource is active, as in WinUI. Generated children are reused
/// across collection edits, and expansion uses an inline native panel or a native popup
/// for top-level compact/top navigation. Windows composition animations and exact
/// NavigationViewItemPresenter resource/template parity remain separate requirements.
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
    subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
    compact_length: Cell<f64>,
    closed_compact: Cell<bool>,
    pane_open: Cell<bool>,
    restore_expanded: Cell<bool>,
    depth: Cell<usize>,
    top_navigation: Cell<bool>,
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
    markup: { namespace: "FerroUI.Controls", content: Content },
});
ferro_impl_classes!(NavigationViewItem:
    StyledElementImpl, InteractiveImpl, ContentControlImpl
);
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
        let weak = this.to_ref().downgrade();
        let resource = this.resources_changed(move |_| {
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
        this.subscriptions.borrow_mut().extend([resource, theme]);
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if !change.is_effective_value_change() {
            return;
        }
        let property = change.property();
        if property == Self::is_expanded_property().as_property() {
            // An explicit expansion choice supersedes pane-close restoration.
            // ForceCollapse records its saved state after this callback returns.
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
        } else if property == Self::is_expanded_property().as_property()
            || property == Self::has_unrealized_children_property().as_property()
            || property == Self::is_child_selected_property().as_property()
            || property == NavigationViewItemBase::is_selected_property().as_property()
            || property == Self::icon_property().as_property()
            || property == Self::info_badge_property().as_property()
            || property == ContentControl::content_property().as_property()
            || property == ContentControl::content_template_property().as_property()
            || property == TemplatedControl::foreground_property().as_property()
            || property == TemplatedControl::background_property().as_property()
            || property == TemplatedControl::padding_property().as_property()
            || property == ToolTip::tip_property().as_property()
            || property == Visual::flow_direction_property().as_property()
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
        if let Some(popup) = parts.popup {
            popup.set_is_open(false);
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
            // Native TemplatedControl removes the old tree before OnApplyTemplate,
            // and never calls that hook when the new template is null. Revoke old
            // handlers first so retained parts cannot affect the new control state.
            struct Restore<'a>(&'a Cell<bool>, bool);
            impl Drop for Restore<'_> {
                fn drop(&mut self) {
                    self.0.set(self.1);
                }
            }
            let restore = Restore(&this.moving_popup, this.moving_popup.replace(true));
            let mut old = this.parts.take();
            old.detach();
            Self::parent_apply_template(this);
            *this.applied_template.borrow_mut() = template;
            drop(restore);
        } else {
            Self::parent_apply_template(this);
        }
    }
}
impl InputElementImpl for NavigationViewItem {
    fn on_key_down(this: &Self, args: &KeyEventArgs) {
        Self::parent_on_key_down(this, args);
        if args.handled() || !this.is_effectively_enabled() {
            return;
        }
        // A child item's bubbling key must never invoke or expand its ancestor.
        let mut source = args.source().and_then(|source| source.cast::<Visual>());
        while let Some(element) = source {
            if let Some(item) = element.cast::<NavigationViewItem>() {
                if item != this.to_ref() {
                    return;
                }
                break;
            }
            source = element.visual_parent();
        }
        if args.key == Key::Enter
            || args.key == Key::Space && !args.key_modifiers.contains(KeyModifiers::ALT)
        {
            this.invoke();
            args.set_handled(true);
            return;
        }
        // The owning NavigationView uses horizontal arrows between top roots.
        // Descendants inside their flyouts keep the hierarchical arrow behavior.
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
            } else if let Some(child) = this
                .child_containers()
                .into_iter()
                .map(|(_, child)| child)
                .find(|child| {
                    child.cast::<NavigationViewItem>().is_some()
                        && child.focusable()
                        && child.is_visible()
                        && child.is_effectively_enabled()
                })
            {
                args.set_handled(
                    child.focus_with(NavigationMethod::Directional, args.key_modifiers),
                );
            }
        } else if (args.key == collapse || args.key == Key::Escape) && this.is_expanded() {
            this.set_current_value(Self::is_expanded_property(), false);
            args.set_handled(true);
        }
    }
}
impl TemplatedControlImpl for NavigationViewItem {
    fn on_apply_template(this: &Self, args: &TemplateAppliedEventArgs) {
        let previous_moving = this.moving_popup.replace(true);
        let mut old = this.parts.take();
        old.detach();
        this.moving_popup.set(previous_moving);
        Self::parent_on_apply_template(this, args);
        let scope = args.name_scope();
        let mut parts = Parts {
            row: scope.find_as("NavigationViewItemPresenter"),
            content: scope.find_as("PART_ContentPresenter"),
            icon: scope.find_as("IconPresenter"),
            badge: scope.find_as("InfoBadgePresenter"),
            chevron: scope.find_as("ExpandCollapseChevron"),
            chevron_glyph: scope.find_as("ChevronGlyph"),
            selection: scope.find_as("SelectionIndicator"),
            inline_host: scope.find_as("PART_InlineHost"),
            popup: scope.find_as("PART_ChildrenPopup"),
            children: scope.find_as("NavigationViewItemMenuItemsHost"),
            ..Parts::default()
        };
        if let Some(row) = &parts.row {
            let weak = this.to_ref().downgrade();
            let button = row.downgrade();
            let token = row.add_handler(Button::click_event(), move |_, args| {
                if let (Some(owner), Some(button)) = (weak.upgrade(), button.upgrade()) {
                    if args.source().is_some_and(|source| source.ptr_eq(&button)) {
                        owner.invoke();
                        owner.focus_with(NavigationMethod::Pointer, KeyModifiers::NONE);
                        args.set_handled(true);
                    }
                }
            });
            parts.clicks.push((row.downgrade(), token));
        }
        if let Some(chevron) = &parts.chevron {
            let weak = this.to_ref().downgrade();
            let token = chevron.add_handler(Button::click_event(), move |_, args| {
                if let Some(owner) = weak.upgrade() {
                    if owner.is_effectively_enabled() && owner.has_children() {
                        owner.set_current_value(Self::is_expanded_property(), !owner.is_expanded());
                    }
                    args.set_handled(true);
                }
            });
            parts.clicks.push((chevron.downgrade(), token));
        }
        if let Some(popup) = &parts.popup {
            // PopupOpenState retains its placement target while open. Anchor a
            // template child instead of the owner, otherwise owner -> popup ->
            // open state -> owner would prevent disposal until visual detachment.
            let anchor = parts
                .row
                .as_ref()
                .map(|row| row.clone().upcast::<Control>())
                .or_else(|| {
                    this.visual_children()
                        .snapshot()
                        .iter()
                        .filter(|visual| visual.cast::<Popup>().is_none())
                        .find_map(|visual| visual.cast::<Control>())
                });
            if let Some(anchor) = anchor {
                popup.set_placement_target(Some(anchor));
            }
            let weak = this.to_ref().downgrade();
            parts.subscriptions.push(popup.closed(move || {
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
    pub fn icon_property() -> StyledProperty<Option<Ref<IconElement>>> {
        FerroProperty::register::<NavigationViewItem, _>("Icon", None)
    }
    pub fn info_badge_property() -> StyledProperty<Option<Ref<InfoBadge>>> {
        FerroProperty::register::<NavigationViewItem, _>("InfoBadge", None)
    }
    pub fn compact_pane_length_property() -> DirectProperty<NavigationViewItem, f64> {
        FerroProperty::register_direct::<NavigationViewItem, _>("CompactPaneLength", |owner| owner.compact_pane_length(), None, 48.0)
    }
    pub fn selects_on_invoked_property() -> StyledProperty<bool> {
        FerroProperty::register::<NavigationViewItem, _>("SelectsOnInvoked", true)
    }
    pub fn is_expanded_property() -> StyledProperty<bool> {
        FerroProperty::register::<NavigationViewItem, _>("IsExpanded", false)
    }
    pub fn has_unrealized_children_property() -> StyledProperty<bool> {
        FerroProperty::register::<NavigationViewItem, _>("HasUnrealizedChildren", false)
    }
    pub fn is_child_selected_property() -> StyledProperty<bool> {
        FerroProperty::register::<NavigationViewItem, _>("IsChildSelected", false)
    }
    pub fn menu_items_property() -> DirectProperty<NavigationViewItem, ItemCollection> {
        FerroProperty::register_direct::<NavigationViewItem, _>("MenuItems", |owner| owner.menu_items(), None, ItemCollection::new())
    }
    pub fn menu_items_source_property() -> StyledProperty<Option<ItemsSource>> {
        FerroProperty::register::<NavigationViewItem, _>("MenuItemsSource", None)
    }
} }
impl NavigationViewItem {
    fn static_constructor() {
        InputElement::focusable_property().override_default_value::<NavigationViewItem>(true);
        TemplatedControl::padding_property()
            .override_default_value::<NavigationViewItem>(Thickness::new(12.0, 8.0, 12.0, 8.0));
        let template = FuncControlTemplate::for_type::<NavigationViewItem>(|_, scope| {
            let root = StackPanel::new();
            root.set_name(Some("NVIRootGrid".into()));
            let row = Button::new();
            row.set_name(Some("NavigationViewItemPresenter".into()));
            row.set_focusable(false);
            row.set_min_height(40.0);
            row.set_margin(Thickness::new(4.0, 2.0, 4.0, 2.0));
            row.set_border_thickness(Thickness::uniform(0.0));
            row.set_corner_radius(CornerRadius::uniform(4.0));
            row.set_horizontal_alignment(HorizontalAlignment::Stretch);
            row.set_horizontal_content_alignment(HorizontalAlignment::Stretch);
            AutomationProperties::set_accessibility_view(&row, AccessibilityView::Raw);
            let overlay = Grid::new();
            let grid = Grid::new();
            grid.set_column_definitions(
                "Auto,*,Auto,Auto"
                    .parse()
                    .expect("NavigationViewItem columns"),
            );
            let icon = ContentPresenter::new();
            icon.set_name(Some("IconPresenter".into()));
            icon.set_width(20.0);
            icon.set_height(20.0);
            icon.set_vertical_alignment(VerticalAlignment::Center);
            icon.set_margin(Thickness::new(0.0, 0.0, 12.0, 0.0));
            grid.children()
                .add(icon.register_in_name_scope(&**scope).upcast::<Control>());
            let content = ContentPresenter::new();
            content.set_name(Some("PART_ContentPresenter".into()));
            content.set_vertical_alignment(VerticalAlignment::Center);
            Grid::set_column(&content, 1);
            grid.children()
                .add(content.register_in_name_scope(&**scope).upcast::<Control>());
            let badge = ContentPresenter::new();
            badge.set_name(Some("InfoBadgePresenter".into()));
            badge.set_vertical_alignment(VerticalAlignment::Center);
            badge.set_margin(Thickness::new(8.0, 0.0, 0.0, 0.0));
            Grid::set_column(&badge, 2);
            grid.children()
                .add(badge.register_in_name_scope(&**scope).upcast::<Control>());
            let chevron = Button::new();
            chevron.set_name(Some("ExpandCollapseChevron".into()));
            chevron.set_focusable(false);
            chevron.set_width(28.0);
            chevron.set_padding(Thickness::uniform(0.0));
            chevron.set_border_thickness(Thickness::uniform(0.0));
            chevron.set_background(Some(Brushes::transparent()));
            AutomationProperties::set_accessibility_view(&chevron, AccessibilityView::Raw);
            let glyph = TextBlock::new();
            glyph.set_name(Some("ChevronGlyph".into()));
            glyph.set_text(Some("›"));
            glyph.set_font_size(18.0);
            chevron.set_content(Some(Control::boxed(glyph.register_in_name_scope(&**scope))));
            Grid::set_column(&chevron, 3);
            grid.children()
                .add(chevron.register_in_name_scope(&**scope).upcast::<Control>());
            row.set_content(Some(Control::boxed(grid)));
            overlay
                .children()
                .add(row.register_in_name_scope(&**scope).upcast::<Control>());
            let selection = Border::new();
            selection.set_name(Some("SelectionIndicator".into()));
            selection.set_width(3.0);
            selection.set_height(16.0);
            selection.set_corner_radius(CornerRadius::uniform(2.0));
            selection.set_margin(Thickness::new(4.0, 0.0, 4.0, 0.0));
            selection.set_horizontal_alignment(HorizontalAlignment::Left);
            selection.set_vertical_alignment(VerticalAlignment::Center);
            selection.set_is_hit_test_visible(false);
            overlay.children().add(
                selection
                    .register_in_name_scope(&**scope)
                    .upcast::<Control>(),
            );
            root.children().add(overlay.upcast::<Control>());
            let inline = Border::new();
            inline.set_name(Some("PART_InlineHost".into()));
            let children = StackPanel::new();
            children.set_name(Some("NavigationViewItemMenuItemsHost".into()));
            inline.set_child(children.register_in_name_scope(&**scope));
            root.children()
                .add(inline.register_in_name_scope(&**scope).upcast::<Control>());
            let popup = Popup::new();
            popup.set_name(Some("PART_ChildrenPopup".into()));
            popup.set_is_light_dismiss_enabled(true);
            root.children()
                .add(popup.register_in_name_scope(&**scope).upcast::<Control>());
            root.register_in_name_scope(&**scope).upcast()
        });
        TemplatedControl::template_property()
            .override_default_value::<NavigationViewItem>(Some(template));
    }
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
            subscriptions: RefCell::new(Vec::new()),
            compact_length: Cell::new(48.0),
            closed_compact: Cell::new(false),
            pane_open: Cell::new(true),
            restore_expanded: Cell::new(false),
            depth: Cell::new(0),
            top_navigation: Cell::new(false),
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
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn icon(&self) -> Option<Ref<IconElement>> {
        self.get_value(Self::icon_property())
    }
    pub fn set_icon(&self, value: Option<Ref<IconElement>>) {
        self.set_value(Self::icon_property(), value);
    }
    pub fn info_badge(&self) -> Option<Ref<InfoBadge>> {
        self.get_value(Self::info_badge_property())
    }
    pub fn set_info_badge(&self, value: Option<Ref<InfoBadge>>) {
        self.set_value(Self::info_badge_property(), value);
    }
    pub fn compact_pane_length(&self) -> f64 {
        self.compact_length.get()
    }
    pub fn selects_on_invoked(&self) -> bool {
        self.get_value(Self::selects_on_invoked_property())
    }
    pub fn set_selects_on_invoked(&self, value: bool) {
        self.set_value(Self::selects_on_invoked_property(), value);
    }
    pub fn is_expanded(&self) -> bool {
        self.get_value(Self::is_expanded_property())
    }
    pub fn set_is_expanded(&self, value: bool) {
        self.set_value(Self::is_expanded_property(), value);
    }
    pub fn has_unrealized_children(&self) -> bool {
        self.get_value(Self::has_unrealized_children_property())
    }
    pub fn set_has_unrealized_children(&self, value: bool) {
        self.set_value(Self::has_unrealized_children_property(), value);
    }
    pub fn is_child_selected(&self) -> bool {
        self.get_value(Self::is_child_selected_property())
    }
    pub fn set_is_child_selected(&self, value: bool) {
        self.set_value(Self::is_child_selected_property(), value);
    }
    pub fn menu_items(&self) -> ItemCollection {
        self.menu_items.clone()
    }
    pub fn menu_items_source(&self) -> Option<ItemsSource> {
        self.get_value(Self::menu_items_source_property())
    }
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
    pub(crate) fn invoke(&self) {
        if self.is_effectively_enabled() {
            for (_, handler) in self.invoked.snapshot().iter() {
                handler();
            }
        }
    }
    pub(crate) fn child_containers(
        &self,
    ) -> Vec<(Option<BoxedValue>, Ref<NavigationViewItemBase>)> {
        self.children.borrow().clone()
    }
    pub(crate) fn has_children(&self) -> bool {
        self.has_unrealized_children()
            || self.menu_items.count() > 0
            || !self.children.borrow().is_empty()
    }
    pub(crate) fn has_children_for_navigation(&self) -> bool {
        // NavigationView::DoesNavigationViewItemHaveChildren gives an active
        // source precedence. The item's HasChildren predicate above deliberately
        // also considers explicit/unrealized children for its chevron and UIA.
        if self.menu_items_source().is_some() {
            self.source_subscription
                .borrow()
                .as_ref()
                .is_some_and(|(source, _)| source.count() > 0)
        } else {
            self.menu_items.count() > 0 || self.has_unrealized_children()
        }
    }
    pub(crate) fn set_navigation_context(
        &self,
        compact_length: f64,
        closed_compact: bool,
        depth: usize,
    ) {
        let old_compact = self.closed_compact.replace(closed_compact);
        let changed = self.set_and_raise_cell(
            Self::compact_pane_length_property(),
            &self.compact_length,
            compact_length,
        ) | (old_compact != closed_compact)
            | (self.depth.replace(depth) != depth);
        if changed {
            self.update_parts();
        }
    }
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
    pub(crate) fn in_flyout(&self) -> bool {
        self.top_navigation.get() || self.closed_compact.get() && self.depth.get() == 0
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
                let reusable = if force {
                    None
                } else {
                    previous
                        .iter()
                        .position(|(item, _)| items_equal(item, &data))
                };
                let container = if let Some(container) = explicit {
                    container
                } else if let Some(index) = reusable {
                    previous.remove(index).1
                } else {
                    let built = template.as_ref().and_then(|template| template.build(&data));
                    let container = built
                        .as_ref()
                        .and_then(|control| control.cast::<NavigationViewItemBase>())
                        .unwrap_or_else(|| {
                            let child = NavigationViewItem::new();
                            child.set_content(built.map(Control::boxed).or_else(|| data.clone()));
                            child.upcast()
                        });
                    // As with native ItemsControl and the NavigationView root
                    // factory, control content keeps the inherited application
                    // model instead of becoming its own data context.
                    if data.as_ref().and_then(Control::from_boxed).is_none() {
                        container.set_data_context(data.clone());
                    }
                    container
                };
                self.validate_child_hierarchy(&container);
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
    fn validate_child_hierarchy(&self, child: &Ref<NavigationViewItemBase>) {
        // Validate before propagating depth or applying a recursive data template.
        // A cycle can be assembled before any visual is attached, so relying on
        // visual-parent validation would permit unbounded context recursion.
        let owner = self.to_ref().upcast::<NavigationViewItemBase>();
        let mut pending = vec![child.clone()];
        let mut visited: Vec<Ref<NavigationViewItemBase>> = Vec::new();
        while let Some(candidate) = pending.pop() {
            assert!(
                candidate != owner,
                "A NavigationViewItem cannot contain itself or an ancestor."
            );
            if visited.iter().any(|seen| seen.ptr_eq(&candidate)) {
                continue;
            }
            if let Some(item) = candidate.cast::<NavigationViewItem>() {
                pending.extend(item.child_containers().into_iter().map(|(_, child)| child));
            }
            visited.push(candidate);
        }
    }
    fn realize_children(&self) {
        let parts = self.parts.borrow().clone();
        if let Some(panel) = parts.children {
            let children = self.child_containers();
            let current = panel.children().snapshot();
            if current.len() != children.len()
                || current
                    .iter()
                    .zip(&children)
                    .any(|(current, (_, child))| *current != child.clone().upcast::<Control>())
            {
                panel.children().clear();
                for (_, child) in children {
                    panel.children().add(child.upcast::<Control>());
                }
            }
        }
    }
    fn brush(&self, key: &str, fallback: u32) -> Option<Rc<dyn IBrush>> {
        self.try_find_resource(
            &ResourceKey::from(key),
            self.actual_theme_variant().as_ref(),
        )
        .flatten()
        .and_then(|value| ValueTypes::try_cast(&value, ValueType::of::<Rc<dyn IBrush>>()))
        .and_then(|value| value.downcast_ref::<Rc<dyn IBrush>>().cloned())
        .or_else(|| Some(SolidColorBrush::from_uint32(fallback).into()))
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
            self.update_parts_core();
            if !self.update_pending.get() {
                break;
            }
        }
    }
    fn update_parts_core(&self) {
        let parts = self.parts.borrow().clone();
        let compact = self.closed_compact.get() && self.depth.get() == 0;
        let flyout = self.in_flyout();
        let expanded = self.is_expanded();
        let selected = self.is_selected() || self.is_child_selected() && !expanded;
        let dark = self.actual_theme_variant().as_ref() == Some(&ThemeVariant::dark());
        let rtl = self.flow_direction() == FlowDirection::RightToLeft;
        self.pseudo_classes().set(":expanded", expanded);
        self.pseudo_classes()
            .set(":child-selected", self.is_child_selected());
        self.pseudo_classes().set(":closedcompact", compact);
        self.pseudo_classes().set(":top", self.top_navigation.get());
        if let Some(row) = &parts.row {
            row.set_padding(self.padding());
            let indent = if flyout {
                0.0
            } else {
                self.depth.get() as f64 * 16.0
            };
            row.set_margin(if rtl {
                Thickness::new(4.0, 2.0, 4.0 + indent, 2.0)
            } else {
                Thickness::new(4.0 + indent, 2.0, 4.0, 2.0)
            });
            row.set_background(if self.is_set(TemplatedControl::background_property()) {
                self.background()
            } else if selected {
                self.brush(
                    if self.top_navigation.get() {
                        "TopNavigationViewItemBackgroundSelected"
                    } else {
                        "NavigationViewItemBackgroundSelected"
                    },
                    if self.top_navigation.get() {
                        0
                    } else if dark {
                        0x12FFFFFF
                    } else {
                        0x09000000
                    },
                )
            } else {
                self.brush("NavigationViewItemBackground", 0)
            });
            row.set_foreground(self.foreground());
        }
        if let Some(content) = &parts.content {
            content.set_content(self.content());
            content.set_content_template(self.content_template());
            content.set_is_visible(!compact);
        }
        if let Some(icon) = &parts.icon {
            icon.set_content(self.icon().map(Control::boxed));
            icon.set_is_visible(self.icon().is_some());
            icon.set_margin(if compact {
                Thickness::uniform(0.0)
            } else {
                Thickness::new(0.0, 0.0, 12.0, 0.0)
            });
        }
        if let Some(badge) = &parts.badge {
            badge.set_content(self.info_badge().map(Control::boxed));
            badge.set_is_visible(self.info_badge().is_some());
            Grid::set_column(badge, if compact { 0 } else { 2 });
            badge.set_horizontal_alignment(if compact {
                HorizontalAlignment::Right
            } else {
                HorizontalAlignment::Center
            });
            badge.set_vertical_alignment(if compact {
                VerticalAlignment::Top
            } else {
                VerticalAlignment::Center
            });
            badge.set_margin(if compact {
                Thickness::new(12.0, -4.0, -4.0, 0.0)
            } else {
                Thickness::new(8.0, 0.0, 0.0, 0.0)
            });
        }
        if let Some(chevron) = &parts.chevron {
            chevron.set_is_visible(self.has_children() && !compact);
            chevron.set_foreground(self.foreground());
        }
        if let Some(glyph) = &parts.chevron_glyph {
            glyph.set_text(Some(if expanded {
                "⌄"
            } else if rtl {
                "‹"
            } else {
                "›"
            }));
        }
        if let Some(selection) = &parts.selection {
            selection.set_is_visible(selected);
            selection.set_background(self.brush(
                "NavigationViewSelectionIndicatorForeground",
                if dark { 0xFF60CDFF } else { 0xFF0078D4 },
            ));
            if self.top_navigation.get() {
                selection.set_width(16.0);
                selection.set_height(3.0);
                selection.set_horizontal_alignment(HorizontalAlignment::Center);
                selection.set_vertical_alignment(VerticalAlignment::Bottom);
                selection.set_margin(Thickness::new(0.0, 0.0, 0.0, 4.0));
            } else {
                selection.set_width(3.0);
                selection.set_height(16.0);
                selection.set_horizontal_alignment(if rtl {
                    HorizontalAlignment::Right
                } else {
                    HorizontalAlignment::Left
                });
                selection.set_vertical_alignment(VerticalAlignment::Center);
                selection.set_margin(Thickness::new(4.0, 0.0, 4.0, 0.0));
            }
        }
        if let Some(row) = &parts.row {
            // The compact label belongs to the template. Never replace an application's
            // tooltip on the NavigationViewItem itself.
            ToolTip::set_tip(
                row,
                if compact && !self.is_set(ToolTip::tip_property()) {
                    self.content().filter(|content| {
                        content
                            .downcast_ref::<String>()
                            .is_some_and(|text| !text.is_empty())
                            || content
                                .downcast_ref::<&str>()
                                .is_some_and(|text| !text.is_empty())
                    })
                } else {
                    None
                },
            );
        }
        if let (Some(host), Some(popup), Some(children)) =
            (&parts.inline_host, &parts.popup, &parts.children)
        {
            let previous_moving = self.moving_popup.replace(true);
            if flyout {
                host.set_child(None::<Ref<Control>>);
                if popup.child().as_ref() != Some(&children.clone().upcast::<Control>()) {
                    popup.set_child(Some(children.clone().upcast::<Control>()));
                }
                children.set_min_width(200.0);
                children.set_background(self.brush(
                    "SolidBackgroundFillColorBaseBrush",
                    if dark { 0xFF202020 } else { 0xFFF3F3F3 },
                ));
                popup.set_placement(if self.top_navigation.get() {
                    PlacementMode::Bottom
                } else if rtl {
                    PlacementMode::Left
                } else {
                    PlacementMode::Right
                });
                popup.set_is_open(
                    expanded
                        && self.is_attached_to_visual_tree()
                        && popup.placement_target().is_some(),
                );
            } else {
                popup.set_is_open(false);
                popup.set_child(None::<Ref<Control>>);
                if host.child().as_ref() != Some(&children.clone().upcast::<Control>()) {
                    host.set_child(Some(children.clone().upcast::<Control>()));
                }
                children.set_min_width(0.0);
                children.set_background(None);
            }
            self.moving_popup.set(previous_moving);
            children.set_is_visible(expanded);
            host.set_is_visible(expanded && !flyout);
        }
        for (_, container) in self.child_containers() {
            container.set_navigation_context(
                self.compact_pane_length(),
                false,
                if flyout { 0 } else { self.depth.get() + 1 },
            );
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
        for subscription in self.subscriptions.get_mut().drain(..) {
            subscription.dispose();
        }
        self.parts.get_mut().detach();
    }
}

/// A non-interactive heading separating groups of navigation items.
#[repr(C)]
pub struct NavigationViewItemHeader {
    base: NavigationViewItemBase,
    presenter: RefCell<Option<Ref<ContentPresenter>>>,
    closed_compact: Cell<bool>,
    depth: Cell<usize>,
}
ferro_class!(NavigationViewItemHeader: NavigationViewItemBase);
ferro_class_info!(NavigationViewItemHeader { new: NavigationViewItemHeader::new, markup: { namespace: "FerroUI.Controls", content: Content } });
ferro_impl_classes!(NavigationViewItemHeader: FerroObjectImpl, StyledElementImpl, VisualImpl,
    LayoutableImpl, InteractiveImpl, InputElementImpl, ControlImpl, ContentControlImpl);
impl TemplatedControlImpl for NavigationViewItemHeader {
    fn on_apply_template(this: &Self, args: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, args);
        *this.presenter.borrow_mut() = args.name_scope().find_as("PART_ContentPresenter");
        this.update_presentation();
    }
}
impl NavigationViewItemHeader {
    fn static_constructor() {
        InputElement::focusable_property()
            .override_default_value::<NavigationViewItemHeader>(false);
        TemplatedControl::font_weight_property()
            .override_default_value::<NavigationViewItemHeader>(FontWeight::SemiBold);
        TemplatedControl::padding_property().override_default_value::<NavigationViewItemHeader>(
            Thickness::new(16.0, 12.0, 16.0, 4.0),
        );
        let template = FuncControlTemplate::for_type::<NavigationViewItemHeader>(|_, scope| {
            let presenter = ContentPresenter::new();
            presenter.set_name(Some("PART_ContentPresenter".into()));
            for property in [
                ContentControl::content_property().as_property(),
                ContentControl::content_template_property().as_property(),
                TemplatedControl::padding_property().as_property(),
            ] {
                presenter.bind_binding(property, &TemplateBinding::new(property));
            }
            presenter.register_in_name_scope(&**scope).upcast()
        });
        TemplatedControl::template_property()
            .override_default_value::<NavigationViewItemHeader>(Some(template));
    }
    pub fn construct() -> Self {
        Self {
            base: NavigationViewItemBase::construct(),
            presenter: RefCell::new(None),
            closed_compact: Cell::new(false),
            depth: Cell::new(0),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    fn update_presentation(&self) {
        let presenter = self.presenter.borrow().clone();
        if let Some(presenter) = presenter {
            presenter.set_is_visible(!self.closed_compact.get());
            let indent = self.depth.get() as f64 * 16.0;
            presenter.set_margin(if self.flow_direction() == FlowDirection::RightToLeft {
                Thickness::new(0.0, 0.0, indent, 0.0)
            } else {
                Thickness::new(indent, 0.0, 0.0, 0.0)
            });
        }
    }
}

/// A non-interactive separator using the native Border renderer.
#[repr(C)]
pub struct NavigationViewItemSeparator {
    base: NavigationViewItemBase,
    line: RefCell<Option<Ref<Border>>>,
    closed_compact: Cell<bool>,
    top_navigation: Cell<bool>,
    depth: Cell<usize>,
    subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
}
ferro_class!(NavigationViewItemSeparator: NavigationViewItemBase);
ferro_class_info!(NavigationViewItemSeparator { new: NavigationViewItemSeparator::new, markup: { namespace: "FerroUI.Controls" } });
ferro_impl_classes!(NavigationViewItemSeparator: StyledElementImpl, VisualImpl,
    LayoutableImpl, InteractiveImpl, InputElementImpl, ControlImpl, ContentControlImpl);
impl FerroObjectImpl for NavigationViewItemSeparator {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        let weak = this.to_ref().downgrade();
        let resources = this.resources_changed(move |_| {
            if let Some(owner) = weak.upgrade() {
                owner.update_presentation();
            }
        });
        let weak = this.to_ref().downgrade();
        let theme = this.actual_theme_variant_changed(move || {
            if let Some(owner) = weak.upgrade() {
                owner.update_presentation();
            }
        });
        this.subscriptions.borrow_mut().extend([resources, theme]);
    }
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if change.is_effective_value_change()
            && (change.property() == TemplatedControl::background_property().as_property()
                || change.property() == TemplatedControl::foreground_property().as_property()
                || change.property() == Visual::flow_direction_property().as_property())
        {
            this.update_presentation();
        }
    }
}
impl TemplatedControlImpl for NavigationViewItemSeparator {
    fn on_apply_template(this: &Self, args: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, args);
        *this.line.borrow_mut() = args
            .name_scope()
            .find_as("NavigationViewItemSeparatorRootGrid");
        this.update_presentation();
    }
}
impl NavigationViewItemSeparator {
    fn static_constructor() {
        InputElement::focusable_property()
            .override_default_value::<NavigationViewItemSeparator>(false);
        let template = FuncControlTemplate::for_type::<NavigationViewItemSeparator>(|_, scope| {
            let border = Border::new();
            border.set_name(Some("NavigationViewItemSeparatorRootGrid".into()));
            border.register_in_name_scope(&**scope).upcast()
        });
        TemplatedControl::template_property()
            .override_default_value::<NavigationViewItemSeparator>(Some(template));
    }
    pub fn construct() -> Self {
        Self {
            base: NavigationViewItemBase::construct(),
            line: RefCell::new(None),
            closed_compact: Cell::new(false),
            top_navigation: Cell::new(false),
            depth: Cell::new(0),
            subscriptions: RefCell::new(Vec::new()),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    fn update_presentation(&self) {
        let line = self.line.borrow().clone();
        if let Some(border) = line {
            let brush = self
                .try_find_resource(
                    &ResourceKey::from(if self.top_navigation.get() {
                        "TopNavigationViewItemSeparatorForeground"
                    } else {
                        "NavigationViewItemSeparatorForeground"
                    }),
                    self.actual_theme_variant().as_ref(),
                )
                .flatten()
                .and_then(|value| ValueTypes::try_cast(&value, ValueType::of::<Rc<dyn IBrush>>()))
                .and_then(|value| value.downcast_ref::<Rc<dyn IBrush>>().cloned())
                .unwrap_or_else(|| SolidColorBrush::from_uint32(0x40808080).into());
            border.set_background(if self.is_set(TemplatedControl::foreground_property()) {
                self.foreground()
            } else if self.is_set(TemplatedControl::background_property()) {
                self.background()
            } else {
                Some(brush)
            });
            if self.top_navigation.get() {
                border.set_width(1.0);
                border.set_height(20.0);
                border.set_vertical_alignment(VerticalAlignment::Center);
                border.set_margin(Thickness::new(8.0, 0.0, 8.0, 0.0));
            } else {
                border.set_width(f64::NAN);
                border.set_height(1.0);
                border.set_vertical_alignment(VerticalAlignment::Stretch);
                let inset = if self.closed_compact.get() { 8.0 } else { 16.0 };
                let indent = self.depth.get() as f64 * 16.0;
                border.set_margin(if self.flow_direction() == FlowDirection::RightToLeft {
                    Thickness::new(inset, 8.0, inset + indent, 8.0)
                } else {
                    Thickness::new(inset + indent, 8.0, inset, 8.0)
                });
            }
        }
    }
}
impl Drop for NavigationViewItemSeparator {
    fn drop(&mut self) {
        for subscription in self.subscriptions.get_mut().drain(..) {
            subscription.dispose();
        }
    }
}

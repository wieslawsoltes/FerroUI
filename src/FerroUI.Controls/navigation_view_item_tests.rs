//! Tests of `NavigationViewItem`, `NavigationViewItemHeader`, `NavigationViewItemSeparator` and
//! their automation peer (WinUI; docs/porting/winui-controls.md, section 10.4), adapted from #286.
//!
//! The items have no template of their own: the theme of the tests gives them the parts of the
//! template of the WinUI theme (`navigation_test_support`). What the theme draws for a state (the
//! visibility of the icon, the badge, the chevron) is checked through the pseudo-class the item
//! sets; the WinUI theme's tests check the templates themselves.

use crate::automation::peers::{AutomationControlType, ControlAutomationPeer};
use crate::automation::provider::{IExpandCollapseProvider, IInvokeProvider, ISelectionItemProvider};
use crate::automation::{AutomationProperties, ExpandCollapseState};
use crate::mouse_test_helper::MouseTestHelper;
use crate::navigation_test_support::{has_class, has_no_template_of_its_own, part, show, start, start_with_focus, Scene};
use crate::presenters::ContentPresenter;
use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions};
use crate::testing::TestLogSink;
use crate::{
    Border, Control, InfoBadge, NavigationView, NavigationViewItem, NavigationViewItemBase, NavigationViewItemHeader,
    NavigationViewItemSeparator, NavigationViewPaneDisplayMode, StackPanel, Symbol, SymbolIcon, TextBlock,
};
use ferroui_base::collections::FerroList;
use ferroui_base::input::{InputElement, Key, KeyEventArgs, KeyModifiers, NavigationMethod};
use ferroui_base::logging::LogEventLevel;
use ferroui_base::media::FlowDirection;
use ferroui_base::{BoxedValue, Ref, StyledElement};
use std::cell::Cell;
use std::rc::Rc;

fn text(value: &str) -> Option<BoxedValue> {
    Some(Rc::new(value.to_owned()) as BoxedValue)
}

fn attach(control: &Control) -> Scene {
    show(control, 320.0, 1000.0)
}

fn children(item: &NavigationViewItem) -> Ref<StackPanel> {
    part(item, "NavigationViewItemMenuItemsHost")
}

fn child(item: &NavigationViewItem, index: usize) -> Ref<NavigationViewItem> {
    children(item).children().get(index).cast().unwrap()
}

fn key(item: &NavigationViewItem, key: Key) -> KeyEventArgs {
    let mut event = KeyEventArgs::new();
    event.key = key;
    event.set_routed_event(Some(InputElement::key_down_event()));
    item.raise_event(&event);
    event
}

fn layout(item: &NavigationViewItem) {
    // A change of the visibility invalidates the host first; the layout manager of the window
    // takes the change of the desired size through the ancestors.
    item.update_layout();
}

fn content_text(item: &NavigationViewItem) -> String {
    item.content().unwrap().downcast_ref::<String>().unwrap().clone()
}

#[test]
fn defaults_and_read_only_properties_preserve_source_inheritance_and_per_instance_collections() {
    let _app = start();
    let first = NavigationViewItem::new();
    let second = NavigationViewItem::new();
    assert!(!first.is_selected());
    assert!(!first.is_child_selected());
    assert!(!first.is_expanded());
    assert!(!first.has_unrealized_children());
    assert!(first.selects_on_invoked());
    assert!(first.icon().is_none());
    assert!(first.info_badge().is_none());
    assert!(first.menu_items_source().is_none());
    assert_eq!(first.compact_pane_length(), 48.0);
    assert!(first.focusable());
    assert_ne!(first.menu_items(), second.menu_items());
    assert!(NavigationViewItem::menu_items_property().setter().is_none());
    assert!(NavigationViewItem::compact_pane_length_property().setter().is_none());
    assert!(NavigationViewItemBase::TYPE.is_assignable_from(NavigationViewItem::TYPE));
    assert!(NavigationViewItemBase::TYPE.default_constructor().is_none());
    // No template is built in Rust: the template comes from the theme (here the theme of the tests).
    assert!(has_no_template_of_its_own(NavigationViewItem::TYPE));
    let _scene = attach(&first);
    assert!(first.template().is_some());
    assert!(first.visual_children_count() > 0);
}

#[test]
fn icon_content_and_badge_reach_the_template_and_can_be_replaced() {
    let _app = start();
    let item = NavigationViewItem::new();
    let _scene = attach(&item);
    item.set_content(text("Inbox"));
    let icon = SymbolIcon::with_symbol(Symbol::Mail);
    let badge = InfoBadge::new();
    badge.set_numeric_value(12);
    item.set_icon(Some(icon.clone().upcast()));
    item.set_info_badge(Some(badge.clone()));
    layout(&item);
    let content = part::<ContentPresenter>(&item, "PART_ContentPresenter");
    let icon_part = part::<ContentPresenter>(&item, "Icon");
    let badge_part = part::<ContentPresenter>(&item, "InfoBadgePresenter");
    assert_eq!(content.content().unwrap().downcast_ref::<String>().unwrap(), "Inbox");
    assert_eq!(icon_part.child().unwrap(), icon.clone().upcast::<Control>());
    assert_eq!(badge_part.child().unwrap(), badge.clone().upcast::<Control>());
    assert!(!has_class(&item, ":icon-collapsed"));
    assert!(!has_class(&item, ":info-badge-collapsed"));
    assert!(!has_class(&item, ":icon-only"));
    // The value text of the badge is drawn by the template of InfoBadge (the WinUI theme's).
    let replacement = InfoBadge::new();
    item.set_info_badge(Some(replacement.clone()));
    item.set_content(text("Updated"));
    item.set_icon(None);
    layout(&item);
    assert_eq!(badge_part.child().unwrap(), replacement.upcast::<Control>());
    assert!(badge.visual_parent().is_none());
    assert!(has_class(&item, ":icon-collapsed"));
    assert!(has_class(&item, ":content-only"));
    assert_eq!(content.content().unwrap().downcast_ref::<String>().unwrap(), "Updated");
    // An icon without content is the top pane's IconOnly state.
    item.set_icon(Some(SymbolIcon::with_symbol(Symbol::Mail).upcast()));
    item.set_content(None);
    assert!(has_class(&item, ":icon-only"));
}

#[test]
fn clearing_a_badge_removes_its_visual_and_keeps_the_item_available() {
    let _app = start();
    let item = NavigationViewItem::new();
    let _scene = attach(&item);
    assert!(has_class(&item, ":info-badge-collapsed"));
    item.set_info_badge(Some(InfoBadge::new()));
    layout(&item);
    assert!(!has_class(&item, ":info-badge-collapsed"));
    let presenter = part::<ContentPresenter>(&item, "InfoBadgePresenter");
    item.set_info_badge(None);
    layout(&item);
    assert!(presenter.child().is_none());
    assert!(has_class(&item, ":info-badge-collapsed"));
    assert!(item.is_visible());
    assert!(part::<Border>(&item, "NavigationViewItemPresenter").is_visible());
}

#[test]
fn explicit_and_data_children_are_live_and_existing_data_containers_survive_edits() {
    let _app = start();
    let item = NavigationViewItem::new();
    let _scene = attach(&item);
    let supplied = NavigationViewItem::new();
    supplied.set_content(text("Supplied"));
    item.menu_items().add(Some(Control::boxed(supplied.clone())));
    item.menu_items().add(text("Data"));
    assert_eq!(child(&item, 0), supplied);
    let generated = child(&item, 1);
    assert_eq!(content_text(&generated), "Data");
    assert!(has_class(&item, ":chevron-visible-closed"));
    generated.set_is_expanded(true);
    item.menu_items().insert(0, text("First"));
    assert_eq!(child(&item, 2), generated);
    assert!(child(&item, 2).is_expanded());
    item.menu_items().remove_at(0);
    assert_eq!(child(&item, 1), generated);
    item.menu_items().clear();
    assert_eq!(children(&item).children().count(), 0);
    assert!(supplied.visual_parent().is_none());
    assert!(!has_class(&item, ":chevron-visible-closed"));
    assert!(!has_class(&item, ":chevron-visible-open"));
}

#[test]
fn nested_control_items_preserve_live_inherited_application_data_context() {
    let _app = start();
    let nav = NavigationView::new();
    nav.set_pane_display_mode(NavigationViewPaneDisplayMode::Left);
    let model: BoxedValue = Rc::new("Application model".to_owned());
    nav.set_data_context(Some(model.clone()));
    let parent = NavigationViewItem::new();
    let content = TextBlock::new();
    content.set_text(Some("Nested control content"));
    parent.menu_items().add(Some(Control::boxed(content.clone())));
    parent.set_is_expanded(true);
    nav.menu_items().add(Some(Control::boxed(parent.clone())));
    let _scene = show(&nav, 800.0, 600.0);
    let generated = child(&parent, 0);
    assert!(!generated.is_set(StyledElement::data_context_property().as_property()));
    assert!(Rc::ptr_eq(&generated.data_context().unwrap(), &model));
    assert!(Rc::ptr_eq(&content.data_context().unwrap(), &model));
    assert_eq!(
        part::<ContentPresenter>(&generated, "PART_ContentPresenter").child().unwrap(),
        content.clone().upcast::<Control>()
    );

    let replacement: BoxedValue = Rc::new("Updated application model".to_owned());
    nav.set_data_context(Some(replacement.clone()));
    assert!(Rc::ptr_eq(&generated.data_context().unwrap(), &replacement));
    assert!(Rc::ptr_eq(&content.data_context().unwrap(), &replacement));

    // An active source uses the same container factory as the writable MenuItems.
    let source_content = StackPanel::new();
    let source = Rc::new(FerroList::from_items([Some(Control::boxed(source_content.clone()))]));
    parent.set_menu_items_source(Some(source.into()));
    parent.update_layout();
    let generated = child(&parent, 0);
    assert!(!generated.is_set(StyledElement::data_context_property().as_property()));
    assert!(Rc::ptr_eq(&generated.data_context().unwrap(), &replacement));
    assert!(Rc::ptr_eq(&source_content.data_context().unwrap(), &replacement));
}

/// #286 rejected a cycle with a panic; the port leaves the child out and logs a warning, so that
/// markup cannot end the application.
#[test]
fn cyclic_item_hierarchies_are_left_out_and_logged_before_visuals_or_context_recursion() {
    let _app = start();
    let parent = NavigationViewItem::new();
    let _scene = attach(&parent);
    let nested = NavigationViewItem::new();
    parent.menu_items().add(Some(Control::boxed(nested.clone())));
    let warnings = Rc::new(Cell::new(0));
    let counter = warnings.clone();
    let sink = TestLogSink::start(move |level, _, _, message, _| {
        if level >= LogEventLevel::Warning && message.contains("cannot contain") {
            counter.set(counter.get() + 1);
        }
    });
    nested.menu_items().add(Some(Control::boxed(parent.clone())));
    assert_eq!(1, warnings.get());
    sink.dispose();
    assert_eq!(child(&parent, 0), nested);
    assert_eq!(children(&nested).children().count(), 0);
    assert!(nested.child_containers().is_empty());
    // The application can repair the collection.
    nested.menu_items().clear();
    assert_eq!(child(&parent, 0), nested);
}

#[test]
fn menu_items_remain_writable_while_source_is_active_and_return_when_source_is_cleared() {
    let _app = start();
    let item = NavigationViewItem::new();
    let _scene = attach(&item);
    item.menu_items().add(text("Local"));
    let source = Rc::new(FerroList::from_items(["Source".to_string()]));
    item.set_menu_items_source(Some(source.clone().into()));
    assert!(!item.menu_items().is_read_only());
    item.menu_items().add(text("Hidden local"));
    assert_eq!(item.menu_items().count(), 2);
    assert_eq!(children(&item).children().count(), 1);
    assert_eq!(content_text(&child(&item, 0)), "Source");
    source.add("Source added".into());
    assert_eq!(children(&item).children().count(), 2);
    item.set_menu_items_source(None);
    assert_eq!(content_text(&child(&item, 0)), "Local");
    assert_eq!(content_text(&child(&item, 1)), "Hidden local");
    source.add("Detached".into());
    assert_eq!(children(&item).children().count(), 2);
}

#[test]
fn replacing_a_source_detaches_its_listener_and_observes_the_replacement() {
    let _app = start();
    let item = NavigationViewItem::new();
    let _scene = attach(&item);
    let old = Rc::new(FerroList::from_items(["Old".to_string()]));
    let new = Rc::new(FerroList::from_items(["New".to_string()]));
    item.set_menu_items_source(Some(old.clone().into()));
    item.apply_template();
    item.set_menu_items_source(Some(new.clone().into()));
    old.add("Must stay detached".into());
    assert_eq!(children(&item).children().count(), 1);
    new.add("Second".into());
    assert_eq!(children(&item).children().count(), 2);
    assert_eq!(content_text(&child(&item, 0)), "New");
}

#[test]
fn collapsed_children_do_not_consume_layout_and_expanding_restores_them() {
    let _app = start();
    let item = NavigationViewItem::new();
    let _scene = attach(&item);
    item.set_content(text("Parent"));
    item.menu_items().add(text("Child one"));
    item.menu_items().add(text("Child two"));
    layout(&item);
    let collapsed = item.desired_size().height;
    assert!(!children(&item).is_visible());
    item.set_is_expanded(true);
    layout(&item);
    assert!(children(&item).is_visible());
    assert!(has_class(&item, ":chevron-visible-open"));
    assert!(item.desired_size().height > collapsed);
    item.set_is_expanded(false);
    layout(&item);
    assert!(has_class(&item, ":chevron-visible-closed"));
    assert_eq!(item.desired_size().height, collapsed);
}

/// The chevron toggles the expansion when it is pressed and released over (the presenter's
/// `OnExpandCollapseChevronPointerReleased`); #286 had a button there.
#[test]
fn unrealized_children_expose_a_chevron_and_support_lazy_population() {
    let _app = start();
    let item = NavigationViewItem::new();
    item.set_content(text("Parent"));
    let _scene = attach(&item);
    let chevron = part::<Border>(&item, "ExpandCollapseChevron");
    assert!(!has_class(&item, ":chevron-visible-closed"));
    item.set_has_unrealized_children(true);
    assert!(has_class(&item, ":chevron-visible-closed"));
    let weak = item.downgrade();
    let subscription = item.property_changed(move |change| {
        if change.property() == NavigationViewItem::is_expanded_property().as_property() {
            if let Some(item) = weak.upgrade() {
                if item.is_expanded() && item.menu_items().count() == 0 {
                    item.menu_items().add(text("Loaded after expansion"));
                    item.set_has_unrealized_children(false);
                }
            }
        }
    });
    let invoked = Rc::new(Cell::new(0));
    let seen = invoked.clone();
    let _invocation = item.invoked(move || seen.set(seen.get() + 1));
    MouseTestHelper::new().click(&chevron);
    assert!(item.is_expanded());
    assert!(!item.has_unrealized_children());
    assert_eq!(children(&item).children().count(), 1);
    assert!(has_class(&item, ":chevron-visible-open"));
    assert_eq!(invoked.get(), 0, "the chevron does not invoke the item");
    subscription.dispose();
}

/// A press and a release over the item invokes it (`OnPresenterPointerReleased`).
#[test]
fn a_click_on_the_item_invokes_it_and_marks_it_pressed_while_down() {
    let _app = start();
    let item = NavigationViewItem::new();
    item.set_content(text("Item"));
    let _scene = attach(&item);
    let invoked = Rc::new(Cell::new(0));
    let seen = invoked.clone();
    let _invocation = item.invoked(move || seen.set(seen.get() + 1));
    let mouse = MouseTestHelper::new();
    mouse.down(&item);
    assert!(has_class(&item, ":pressed"));
    assert_eq!(invoked.get(), 0);
    mouse.up(&item);
    assert!(!has_class(&item, ":pressed"));
    assert_eq!(invoked.get(), 1);
    item.set_is_enabled(false);
    mouse.click(&item);
    assert_eq!(invoked.get(), 1, "a disabled item is not invoked");
}

#[test]
fn keyboard_expansion_mirrors_rtl_and_does_not_expand_leaf_items() {
    let _app = start();
    let item = NavigationViewItem::new();
    assert!(!key(&item, Key::Right).handled());
    assert!(!item.is_expanded());
    item.set_has_unrealized_children(true);
    assert!(key(&item, Key::Right).handled());
    assert!(item.is_expanded());
    assert!(key(&item, Key::Left).handled());
    assert!(!item.is_expanded());
    item.set_flow_direction(FlowDirection::RightToLeft);
    assert!(key(&item, Key::Left).handled());
    assert!(item.is_expanded());
    assert!(key(&item, Key::Right).handled());
    assert!(!item.is_expanded());
    item.set_is_enabled(false);
    assert!(!key(&item, Key::Left).handled());
    assert!(!item.is_expanded());
}

#[test]
fn expansion_direction_skips_nonfocusable_children_in_ltr_and_rtl() {
    let _app = start_with_focus();
    let parent = NavigationViewItem::new();
    parent.set_content(text("Parent"));
    let heading = NavigationViewItemHeader::new();
    heading.set_content(text("Heading"));
    parent.menu_items().add(Some(Control::boxed(heading)));
    let skipped = NavigationViewItem::new();
    skipped.set_content(text("Nonfocusable"));
    skipped.set_focusable(false);
    let hidden = NavigationViewItem::new();
    hidden.set_is_visible(false);
    let disabled = NavigationViewItem::new();
    disabled.set_is_enabled(false);
    let target = NavigationViewItem::new();
    target.set_content(text("Focusable child"));
    for item in [&skipped, &hidden, &disabled, &target] {
        parent.menu_items().add(Some(Control::boxed(item.clone())));
    }
    let _scene = attach(&parent);
    assert!(parent.focus_with(NavigationMethod::Directional, KeyModifiers::NONE));
    assert!(key(&parent, Key::Right).handled());
    assert!(parent.is_expanded());
    parent.update_layout();
    assert!(key(&parent, Key::Right).handled());
    assert!(target.is_focused());
    assert!(!skipped.is_focused());

    parent.set_flow_direction(FlowDirection::RightToLeft);
    assert!(parent.focus_with(NavigationMethod::Directional, KeyModifiers::NONE));
    assert!(key(&parent, Key::Left).handled());
    assert!(target.is_focused());
    assert!(!skipped.is_focused());

    target.set_focusable(false);
    assert!(parent.focus_with(NavigationMethod::Directional, KeyModifiers::NONE));
    assert!(!key(&parent, Key::Left).handled());
    assert!(parent.is_focused());
}

#[test]
fn bubbling_child_keyboard_events_do_not_change_the_ancestors_expansion() {
    let _app = start();
    let parent = NavigationViewItem::new();
    let _scene = attach(&parent);
    let nested = NavigationViewItem::new();
    nested.set_has_unrealized_children(true);
    parent.menu_items().add(Some(Control::boxed(nested.clone())));
    parent.set_is_expanded(true);
    layout(&parent);
    assert!(key(&nested, Key::Right).handled());
    assert!(nested.is_expanded());
    assert!(parent.is_expanded());
    assert!(key(&nested, Key::Left).handled());
    assert!(!nested.is_expanded());
    assert!(parent.is_expanded());
}

/// `ShowSelectionIndicator`: the opacity of the part `SelectionIndicator`.
#[test]
fn selection_indicator_uses_own_selection_or_a_hidden_selected_child() {
    let _app = start();
    let item = NavigationViewItem::new();
    let _scene = attach(&item);
    let selection = part::<Border>(&item, "SelectionIndicator");
    assert_eq!(selection.opacity(), 0.0);
    item.set_is_selected(true);
    assert_eq!(selection.opacity(), 1.0);
    assert!(has_class(&item, ":selected"));
    item.set_is_selected(false);
    item.set_is_child_selected(true);
    assert_eq!(selection.opacity(), 1.0);
    assert!(!has_class(&item, ":selected"), "the parent of the selection is not drawn selected");
    item.set_is_expanded(true);
    assert_eq!(selection.opacity(), 0.0);
    item.set_is_selected(true);
    assert_eq!(selection.opacity(), 1.0);
    item.set_is_child_selected(false);
    assert_eq!(selection.opacity(), 1.0);
}

/// A new template takes the children; the old host is emptied, and no template leaves the item
/// without visuals.
#[test]
fn retemplating_moves_the_children_and_releases_the_old_parts() {
    let _app = start();
    let item = NavigationViewItem::new();
    let _scene = attach(&item);
    let nested = NavigationViewItem::new();
    item.menu_items().add(Some(Control::boxed(nested.clone())));
    item.set_is_expanded(true);
    layout(&item);
    let old_host = children(&item);
    assert_eq!(old_host.children().count(), 1);
    item.set_template(Some(FuncControlTemplate::for_type::<NavigationViewItem>(|_, scope| {
        let root = StackPanel::new();
        let host = StackPanel::new();
        host.set_name(Some("NavigationViewItemMenuItemsHost".into()));
        root.children().add(host.register_in_name_scope(&**scope).upcast::<Control>());
        root.upcast()
    })));
    item.apply_template();
    layout(&item);
    assert_eq!(old_host.children().count(), 0);
    let new_host = children(&item);
    assert!(!new_host.ptr_eq(&old_host));
    assert_eq!(new_host.children().get(0), nested.clone().upcast::<Control>());
    assert!(new_host.is_visible());
    item.set_template(None);
    item.apply_template();
    assert_eq!(item.visual_children_count(), 0);
    assert_eq!(new_host.children().count(), 0);
    assert!(nested.visual_parent().is_none());
}

#[test]
fn header_and_separator_are_nonfocusable_items_with_templates_from_the_theme() {
    let _app = start();
    let header = NavigationViewItemHeader::new();
    let separator = NavigationViewItemSeparator::new();
    header.set_content(text("Documents"));
    assert!(!header.focusable());
    assert!(!separator.focusable());
    assert!(has_no_template_of_its_own(NavigationViewItemHeader::TYPE));
    assert!(has_no_template_of_its_own(NavigationViewItemSeparator::TYPE));
    let panel = StackPanel::new();
    panel.children().add(&header);
    panel.children().add(&separator);
    let _scene = show(&panel, 320.0, 400.0);
    header.apply_template();
    separator.apply_template();
    assert!(header.visual_children_count() > 0);
    assert!(separator.visual_children_count() > 0);
    assert!(header.is::<NavigationViewItemBase>());
    assert!(separator.is::<NavigationViewItemBase>());
    assert!(!has_class(&header, ":header-text-collapsed"));
    assert!(!has_class(&separator, ":vertical-line"));
}

#[test]
fn automation_has_selection_and_conditional_expansion_without_an_invoke_pattern() {
    let _app = start();
    let item = NavigationViewItem::new();
    item.set_content(text("Automation item"));
    let peer = ControlAutomationPeer::create_peer_for_element(&item);
    assert_eq!(peer.get_class_name(), "NavigationViewItem");
    assert_eq!(peer.get_automation_control_type(), AutomationControlType::ListItem);
    assert_eq!(peer.get_name(), "Automation item");
    assert!(peer.get_provider::<dyn IInvokeProvider>().is_none());
    assert!(peer.get_provider::<dyn ISelectionItemProvider>().is_some());
    assert!(peer.get_provider::<dyn IExpandCollapseProvider>().is_none());
    item.set_has_unrealized_children(true);
    let expansion = peer.get_provider::<dyn IExpandCollapseProvider>().unwrap();
    assert_eq!(expansion.expand_collapse_state(), ExpandCollapseState::Collapsed);
    expansion.expand().unwrap();
    assert!(item.is_expanded());
    assert_eq!(expansion.expand_collapse_state(), ExpandCollapseState::Expanded);
    expansion.collapse().unwrap();
    assert!(!item.is_expanded());
    AutomationProperties::set_name(&item, Some("Explicit name".into()));
    assert_eq!(peer.get_name(), "Explicit name");
}

#[test]
fn owned_automation_expansion_obeys_active_empty_source_precedence() {
    let _app = start();
    let nav = NavigationView::new();
    nav.set_pane_display_mode(NavigationViewPaneDisplayMode::Left);
    let item = NavigationViewItem::new();
    item.menu_items().add(text("Inactive local child"));
    item.set_has_unrealized_children(true);
    let source = Rc::new(FerroList::<String>::new());
    item.set_menu_items_source(Some(source.clone().into()));
    nav.menu_items().add(Some(Control::boxed(item.clone())));
    let _scene = show(&nav, 800.0, 600.0);
    let peer = ControlAutomationPeer::create_peer_for_element(&item);
    // HasChildren of the item advertises the pattern, while the source predicate of NavigationView
    // decides whether an owned provider expands or collapses it.
    let provider = peer.get_provider::<dyn IExpandCollapseProvider>().unwrap();
    provider.expand().unwrap();
    assert!(!item.is_expanded());
    nav.expand(&item);
    assert!(!item.is_expanded());
    item.set_is_expanded(true);
    provider.collapse().unwrap();
    assert!(item.is_expanded());
    nav.collapse(&item);
    assert!(item.is_expanded());

    source.add("Active source child".to_owned());
    provider.collapse().unwrap();
    assert!(!item.is_expanded());
    provider.expand().unwrap();
    assert!(item.is_expanded());
}

#[test]
fn retained_owned_expansion_provider_rechecks_children_after_removal() {
    let _app = start();
    let nav = NavigationView::new();
    nav.set_pane_display_mode(NavigationViewPaneDisplayMode::Left);
    let item = NavigationViewItem::new();
    item.menu_items().add(text("Child"));
    nav.menu_items().add(Some(Control::boxed(item.clone())));
    let _scene = show(&nav, 800.0, 600.0);
    let peer = ControlAutomationPeer::create_peer_for_element(&item);
    let provider = peer.get_provider::<dyn IExpandCollapseProvider>().unwrap();
    provider.expand().unwrap();
    assert!(item.is_expanded());
    item.menu_items().clear();
    assert!(peer.get_provider::<dyn IExpandCollapseProvider>().is_none());
    provider.collapse().unwrap();
    assert!(item.is_expanded());
    item.set_is_expanded(false);
    provider.expand().unwrap();
    assert!(!item.is_expanded());

    item.menu_items().add(text("Replacement child"));
    provider.expand().unwrap();
    assert!(item.is_expanded());
    provider.collapse().unwrap();
    assert!(!item.is_expanded());
}

#[test]
fn automation_respects_nonselecting_items_and_disabled_input() {
    let _app = start();
    let item = NavigationViewItem::new();
    let peer = ControlAutomationPeer::create_peer_for_element(&item);
    let selection = peer.get_provider::<dyn ISelectionItemProvider>().unwrap();
    item.set_selects_on_invoked(false);
    selection.select().unwrap();
    assert!(!item.is_selected());
    item.set_selects_on_invoked(true);
    selection.select().unwrap();
    assert!(item.is_selected());
    selection.remove_from_selection().unwrap();
    assert!(!item.is_selected());
    item.set_is_enabled(false);
    assert!(selection.select().is_err());
    assert!(!item.is_selected());
}

#[test]
fn nested_automation_selection_invokes_and_finds_its_container_after_flyout_reparenting() {
    let _app = start();
    let nav = NavigationView::new();
    nav.set_pane_display_mode(NavigationViewPaneDisplayMode::Left);
    let parent = NavigationViewItem::new();
    let nested = NavigationViewItem::new();
    parent.menu_items().add(Some(Control::boxed(nested.clone())));
    nav.menu_items().add(Some(Control::boxed(parent.clone())));
    let _scene = show(&nav, 800.0, 600.0);
    nav.apply_template();
    parent.apply_template();
    let invocations = Rc::new(Cell::new(0));
    let observed = invocations.clone();
    let subscription = nav.item_invoked(move |_| observed.set(observed.get() + 1));
    let nav_peer = ControlAutomationPeer::create_peer_for_element(&nav);
    let nested_peer = ControlAutomationPeer::create_peer_for_element(&nested);
    let selection = nested_peer.get_provider::<dyn ISelectionItemProvider>().unwrap();
    assert_eq!(selection.selection_container().unwrap().peer(), nav_peer);
    selection.select().unwrap();
    assert_eq!(invocations.get(), 1);
    assert!(nested.is_selected());
    assert!(parent.is_child_selected());
    selection.add_to_selection().unwrap();
    assert_eq!(invocations.get(), 2);
    assert!(nested.is_selected());
    selection.remove_from_selection().unwrap();
    assert_eq!(invocations.get(), 2);
    assert!(nav.selected_item().is_none());
    // In the top pane the children move into the flyout of the item.
    nav.set_pane_display_mode(NavigationViewPaneDisplayMode::Top);
    parent.set_is_expanded(true);
    assert!(has_class(&parent, ":on-top-navigation-primary"));
    assert!(part::<crate::Panel>(&parent, "FlyoutContentGrid")
        .children()
        .snapshot()
        .iter()
        .any(|host| host.ptr_eq(&children(&parent))));
    assert_eq!(selection.selection_container().unwrap().peer(), nav_peer);
    assert_eq!(nested_peer.get_automation_control_type(), AutomationControlType::ListItem);
    assert_eq!(
        ControlAutomationPeer::create_peer_for_element(&parent).get_automation_control_type(),
        AutomationControlType::TabItem
    );
    subscription.dispose();
}

#[test]
fn retained_external_sources_and_old_template_parts_do_not_keep_the_item_alive() {
    let _app = start();
    let source = Rc::new(FerroList::from_items(["Data".to_string()]));
    let item = NavigationViewItem::new();
    let scene = attach(&item);
    item.set_menu_items_source(Some(source.clone().into()));
    item.apply_template();
    let old_part = part::<Border>(&item, "NavigationViewItemPresenter");
    let peer = ControlAutomationPeer::create_peer_for_element(&item);
    let weak = item.downgrade();
    scene.0.set_content(None);
    scene.0.update_layout();
    drop(item);
    assert!(weak.upgrade().is_none());
    source.add("Later".to_string());
    assert!(weak.upgrade().is_none());
    drop(old_part);
    drop(peer);
}

#[test]
fn source_change_notifications_allow_reentrant_collection_mutation() {
    let _app = start();
    let item = NavigationViewItem::new();
    let _scene = attach(&item);
    let source = Rc::new(FerroList::from_items(["First".to_string()]));
    item.set_menu_items_source(Some(source.clone().into()));
    item.apply_template();
    let changed = Rc::new(Cell::new(false));
    let once = changed.clone();
    let weak = item.downgrade();
    let token = source.add_collection_changed(Rc::new(move |_| {
        if !once.replace(true) {
            if let Some(item) = weak.upgrade() {
                item.menu_items().add(text("Local change while source active"));
            }
        }
    }));
    source.add("Second".to_string());
    assert!(changed.get());
    assert_eq!(children(&item).children().count(), 2);
    assert_eq!(item.menu_items().count(), 1);
    source.remove_collection_changed(token);
}

/// The depth of an item indents its content grid by 31 per level (`c_itemIndentation`), and a
/// closed compact pane marks the top level item (`ClosedCompactAndTopLevelItem`).
#[test]
fn the_depth_indents_the_content_grid_and_the_closed_compact_state_is_set() {
    let _app = start();
    let parent = NavigationViewItem::new();
    let nested = NavigationViewItem::new();
    parent.menu_items().add(Some(Control::boxed(nested.clone())));
    parent.set_is_expanded(true);
    let _scene = attach(&parent);
    layout(&parent);
    assert_eq!(part::<crate::Grid>(&parent, "ContentGrid").margin().left, 0.0);
    assert_eq!(part::<crate::Grid>(&nested, "ContentGrid").margin().left, 31.0);
    parent.set_navigation_context(48.0, true, 0);
    assert!(has_class(&parent, ":closed-compact-and-top-level-item"));
    // A top level item of a closed compact pane has its children in the flyout, at depth 0.
    assert_eq!(part::<crate::Grid>(&nested, "ContentGrid").margin().left, 0.0);
    parent.set_navigation_context(48.0, false, 0);
    assert!(!has_class(&parent, ":closed-compact-and-top-level-item"));
}

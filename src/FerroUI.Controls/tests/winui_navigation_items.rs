//! Source-derived NavigationViewItem contracts exercised through native templates,
//! collections, property notifications, keyboard input and automation providers.

use ferroui_base::animation::{ClockBase, IClock, IGlobalClock, PlayState, TimeSpan};
use ferroui_base::collections::FerroList;
use ferroui_base::input::{InputElement, Key, KeyEventArgs, KeyModifiers, NavigationMethod};
use ferroui_base::media::FlowDirection;
use ferroui_base::reactive::{IDisposable, IObservable, IObserver};
use ferroui_base::{BoxedValue, ObjectType, Rect, Ref, Size, StyledElement};
use ferroui_controls::automation::peers::{AutomationControlType, ControlAutomationPeer};
use ferroui_controls::automation::provider::{
    IExpandCollapseProvider, IInvokeProvider, ISelectionItemProvider,
};
use ferroui_controls::automation::{AutomationProperties, ExpandCollapseState};
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions};
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_controls::{
    InfoBadge, NavigationView, NavigationViewItem, NavigationViewItemBase,
    NavigationViewItemHeader, NavigationViewItemSeparator, NavigationViewPaneDisplayMode, Symbol,
    SymbolIcon,
};
use ferroui_controls::{Border, Button, Control, StackPanel, TextBlock, Window};
use std::cell::Cell;
use std::rc::Rc;

// Native SplitView transitions require a global clock even when a test only
// verifies their synchronous state. Keep its time fixed for deterministic tests.
#[derive(Default)]
struct TestGlobalClock(ClockBase);

impl IObservable<TimeSpan> for TestGlobalClock {
    fn subscribe(&self, observer: Rc<dyn IObserver<TimeSpan>>) -> Rc<dyn IDisposable> {
        self.0.subscribe(observer)
    }
}

impl IClock for TestGlobalClock {
    fn play_state(&self) -> PlayState {
        self.0.play_state()
    }

    fn set_play_state(&self, value: PlayState) {
        self.0.set_play_state(value);
    }
}

impl IGlobalClock for TestGlobalClock {}

fn start() -> UnitTestApplicationScope {
    ferroui_controls::register_types();
    UnitTestApplication::start(
        TestServices::styled_window().with_global_clock(Rc::new(TestGlobalClock::default())),
    )
}

fn start_with_focus() -> UnitTestApplicationScope {
    ferroui_controls::register_types();
    let focus = TestServices::real_focus();
    let mut services =
        TestServices::styled_window().with_global_clock(Rc::new(TestGlobalClock::default()));
    services.keyboard_device = focus.keyboard_device;
    services.keyboard_navigation = focus.keyboard_navigation;
    services.input_manager = focus.input_manager;
    UnitTestApplication::start(services)
}

fn text(value: &str) -> Option<BoxedValue> {
    Some(Rc::new(value.to_owned()) as BoxedValue)
}

struct Scene(Ref<Window>);
impl Drop for Scene {
    fn drop(&mut self) {
        self.0.close();
    }
}

fn attach(control: &Control) -> Scene {
    let window = Window::new();
    window.set_width(320.0);
    window.set_height(1000.0);
    window.set_content(Some(Control::boxed(control.to_ref())));
    window.show();
    window.update_layout();
    Scene(window)
}

fn part<T: ObjectType>(item: &NavigationViewItem, name: &str) -> Ref<T> {
    item.apply_template();
    item.update_layout();
    item.get_template_descendants()
        .into_iter()
        .find(|control| control.name().as_deref() == Some(name))
        .unwrap_or_else(|| panic!("missing NavigationViewItem part {name}"))
        .cast::<T>()
        .unwrap()
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
    if item.is_attached_to_visual_tree() {
        // Visibility invalidates the immediate host first. Its native manager
        // propagates desired-size changes through ancestors during the pass.
        // Re-measuring an otherwise valid owner alone would skip that queue.
        item.update_layout();
    } else {
        item.measure(Size::new(320.0, 1000.0));
        item.arrange(Rect::new(0.0, 0.0, 320.0, item.desired_size().height));
    }
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
    assert!(
        NavigationViewItem::compact_pane_length_property()
            .setter()
            .is_none()
    );
    assert!(NavigationViewItemBase::TYPE.is_assignable_from(NavigationViewItem::TYPE));
    assert!(NavigationViewItemBase::TYPE.default_constructor().is_none());
    assert!(!first.has_local_value(TemplatedControl::template_property()));
    first.apply_template();
    assert!(first.visual_children_count() > 0);
}

#[test]
fn icon_content_and_badge_are_real_live_native_children_and_can_be_replaced() {
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
    let icon_part = part::<ContentPresenter>(&item, "IconPresenter");
    let badge_part = part::<ContentPresenter>(&item, "InfoBadgePresenter");
    assert_eq!(
        content.content().unwrap().downcast_ref::<String>().unwrap(),
        "Inbox"
    );
    assert_eq!(icon_part.child().unwrap(), icon.clone().upcast::<Control>());
    assert_eq!(
        badge_part.child().unwrap(),
        badge.clone().upcast::<Control>()
    );
    badge.set_numeric_value(37);
    layout(&item);
    let rendered = badge
        .get_template_descendants()
        .into_iter()
        .find_map(|control| {
            (control.name().as_deref() == Some("ValueTextBlock"))
                .then(|| control.cast::<TextBlock>())
                .flatten()
        })
        .unwrap();
    assert_eq!(rendered.text().as_deref(), Some("37"));
    let replacement = InfoBadge::new();
    item.set_info_badge(Some(replacement.clone()));
    item.set_content(text("Updated"));
    item.set_icon(None);
    layout(&item);
    assert_eq!(badge_part.child().unwrap(), replacement.upcast::<Control>());
    assert!(badge.visual_parent().is_none());
    assert!(!icon_part.is_visible());
    assert_eq!(
        content.content().unwrap().downcast_ref::<String>().unwrap(),
        "Updated"
    );
}

#[test]
fn clearing_a_badge_removes_its_visual_and_keeps_the_item_available() {
    let _app = start();
    let item = NavigationViewItem::new();
    let _scene = attach(&item);
    item.set_info_badge(Some(InfoBadge::new()));
    layout(&item);
    let presenter = part::<ContentPresenter>(&item, "InfoBadgePresenter");
    item.set_info_badge(None);
    layout(&item);
    assert!(presenter.child().is_none());
    assert!(!presenter.is_visible());
    assert!(item.is_visible());
    assert!(part::<Button>(&item, "NavigationViewItemPresenter").is_visible());
}

#[test]
fn explicit_and_data_children_are_live_and_existing_data_containers_survive_edits() {
    let _app = start();
    let item = NavigationViewItem::new();
    let _scene = attach(&item);
    let supplied = NavigationViewItem::new();
    supplied.set_content(text("Supplied"));
    item.menu_items()
        .add(Some(Control::boxed(supplied.clone())));
    item.menu_items().add(text("Data"));
    assert_eq!(child(&item, 0), supplied);
    let generated = child(&item, 1);
    assert_eq!(
        generated
            .content()
            .unwrap()
            .downcast_ref::<String>()
            .unwrap(),
        "Data"
    );
    generated.set_is_expanded(true);
    item.menu_items().insert(0, text("First"));
    assert_eq!(child(&item, 2), generated);
    assert!(child(&item, 2).is_expanded());
    item.menu_items().remove_at(0);
    assert_eq!(child(&item, 1), generated);
    item.menu_items().clear();
    assert_eq!(children(&item).children().count(), 0);
    assert!(supplied.visual_parent().is_none());
    assert!(!part::<Button>(&item, "ExpandCollapseChevron").is_visible());
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
    parent
        .menu_items()
        .add(Some(Control::boxed(content.clone())));
    parent.set_is_expanded(true);
    nav.menu_items().add(Some(Control::boxed(parent.clone())));
    let _scene = attach(&nav);
    let generated = child(&parent, 0);
    assert!(!generated.has_local_value(StyledElement::data_context_property()));
    assert!(Rc::ptr_eq(&generated.data_context().unwrap(), &model));
    assert!(Rc::ptr_eq(&content.data_context().unwrap(), &model));
    assert_eq!(
        part::<ContentPresenter>(&generated, "PART_ContentPresenter")
            .child()
            .unwrap(),
        content.clone().upcast::<Control>()
    );

    let replacement: BoxedValue = Rc::new("Updated application model".to_owned());
    nav.set_data_context(Some(replacement.clone()));
    assert!(Rc::ptr_eq(&generated.data_context().unwrap(), &replacement));
    assert!(Rc::ptr_eq(&content.data_context().unwrap(), &replacement));

    // An active source uses the same container factory as writable MenuItems.
    let source_content = StackPanel::new();
    let source = Rc::new(FerroList::from_items([Some(Control::boxed(
        source_content.clone(),
    ))]));
    parent.set_menu_items_source(Some(source.into()));
    parent.update_layout();
    let generated = child(&parent, 0);
    assert!(!generated.has_local_value(StyledElement::data_context_property()));
    assert!(Rc::ptr_eq(&generated.data_context().unwrap(), &replacement));
    assert!(Rc::ptr_eq(
        &source_content.data_context().unwrap(),
        &replacement
    ));
}

#[test]
fn cyclic_item_hierarchies_are_rejected_before_visuals_or_context_recursion() {
    let _app = start();
    let parent = NavigationViewItem::new();
    let nested = NavigationViewItem::new();
    parent
        .menu_items()
        .add(Some(Control::boxed(nested.clone())));
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        nested
            .menu_items()
            .add(Some(Control::boxed(parent.clone())));
    }));
    // ItemCollection reports changes after mutation. Clear the invalid entry so
    // the application can repair the collection after handling the rejection.
    nested.menu_items().clear();
    assert!(result.is_err());
    assert_eq!(child(&parent, 0), nested);
    assert_eq!(children(&nested).children().count(), 0);
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
    assert_eq!(
        child(&item, 0)
            .content()
            .unwrap()
            .downcast_ref::<String>()
            .unwrap(),
        "Source"
    );
    source.add("Source added".into());
    assert_eq!(children(&item).children().count(), 2);
    item.set_menu_items_source(None);
    assert_eq!(
        child(&item, 0)
            .content()
            .unwrap()
            .downcast_ref::<String>()
            .unwrap(),
        "Local"
    );
    assert_eq!(
        child(&item, 1)
            .content()
            .unwrap()
            .downcast_ref::<String>()
            .unwrap(),
        "Hidden local"
    );
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
    assert_eq!(
        child(&item, 0)
            .content()
            .unwrap()
            .downcast_ref::<String>()
            .unwrap(),
        "New"
    );
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
    assert!(item.desired_size().height > collapsed);
    item.set_is_expanded(false);
    layout(&item);
    assert_eq!(item.desired_size().height, collapsed);
}

#[test]
fn unrealized_children_expose_a_chevron_and_support_lazy_population() {
    let _app = start();
    let item = NavigationViewItem::new();
    let _scene = attach(&item);
    item.apply_template();
    let chevron = part::<Button>(&item, "ExpandCollapseChevron");
    assert!(!chevron.is_visible());
    item.set_has_unrealized_children(true);
    assert!(chevron.is_visible());
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
    chevron.on_click();
    assert!(item.is_expanded());
    assert!(!item.has_unrealized_children());
    assert_eq!(children(&item).children().count(), 1);
    assert!(chevron.is_visible());
    subscription.dispose();
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
    parent
        .menu_items()
        .add(Some(Control::boxed(nested.clone())));
    parent.set_is_expanded(true);
    layout(&parent);
    assert!(key(&nested, Key::Right).handled());
    assert!(nested.is_expanded());
    assert!(parent.is_expanded());
    assert!(key(&nested, Key::Left).handled());
    assert!(!nested.is_expanded());
    assert!(parent.is_expanded());
}

#[test]
fn selection_indicator_uses_own_selection_or_a_hidden_selected_child() {
    let _app = start();
    let item = NavigationViewItem::new();
    item.apply_template();
    let selection = part::<Border>(&item, "SelectionIndicator");
    assert!(!selection.is_visible());
    item.set_is_selected(true);
    assert!(selection.is_visible());
    item.set_is_selected(false);
    item.set_is_child_selected(true);
    assert!(selection.is_visible());
    item.set_is_expanded(true);
    assert!(!selection.is_visible());
    item.set_is_selected(true);
    assert!(selection.is_visible());
    item.set_is_child_selected(false);
    assert!(selection.is_visible());
}

#[test]
fn retemplating_detaches_old_buttons_and_releases_old_visual_ownership() {
    let _app = start();
    let item = NavigationViewItem::new();
    let _scene = attach(&item);
    item.set_has_unrealized_children(true);
    let badge = InfoBadge::new();
    item.set_info_badge(Some(badge.clone()));
    layout(&item);
    let old = part::<Button>(&item, "ExpandCollapseChevron");
    let old_badge = part::<ContentPresenter>(&item, "InfoBadgePresenter");
    item.set_template(Some(FuncControlTemplate::for_type::<NavigationViewItem>(
        |_, scope| {
            let root = StackPanel::new();
            let badge = ContentPresenter::new();
            badge.set_name(Some("InfoBadgePresenter".into()));
            root.children()
                .add(badge.register_in_name_scope(&**scope).upcast::<Control>());
            root.upcast()
        },
    )));
    item.apply_template();
    old.on_click();
    assert!(!item.is_expanded());
    layout(&item);
    assert!(old_badge.child().is_none());
    assert_eq!(
        part::<ContentPresenter>(&item, "InfoBadgePresenter")
            .child()
            .unwrap(),
        badge.clone().upcast::<Control>()
    );
    let replacement_presenter = part::<ContentPresenter>(&item, "InfoBadgePresenter");
    item.set_template(None);
    item.apply_template();
    assert_eq!(item.visual_children_count(), 0);
    assert!(replacement_presenter.content().is_none());
    assert!(replacement_presenter.child().is_none());
    assert!(badge.visual_parent().is_none());
    old.on_click();
    assert!(!item.is_expanded());
}

#[test]
fn header_and_separator_are_native_nonfocusable_items_with_replaceable_templates() {
    let _app = start();
    let header = NavigationViewItemHeader::new();
    let separator = NavigationViewItemSeparator::new();
    header.set_content(text("Documents"));
    assert!(!header.focusable());
    assert!(!separator.focusable());
    assert!(!header.has_local_value(TemplatedControl::template_property()));
    assert!(!separator.has_local_value(TemplatedControl::template_property()));
    header.apply_template();
    separator.apply_template();
    assert!(header.visual_children_count() > 0);
    assert!(separator.visual_children_count() > 0);
    assert!(header.is::<NavigationViewItemBase>());
    assert!(separator.is::<NavigationViewItemBase>());
}

#[test]
fn automation_has_selection_and_conditional_expansion_without_an_invoke_pattern() {
    let _app = start();
    let item = NavigationViewItem::new();
    item.set_content(text("Automation item"));
    let peer = ControlAutomationPeer::create_peer_for_element(&item);
    assert_eq!(peer.get_class_name(), "NavigationViewItem");
    assert_eq!(
        peer.get_automation_control_type(),
        AutomationControlType::ListItem
    );
    assert_eq!(peer.get_name(), "Automation item");
    assert!(peer.get_provider::<dyn IInvokeProvider>().is_none());
    assert!(peer.get_provider::<dyn ISelectionItemProvider>().is_some());
    assert!(peer.get_provider::<dyn IExpandCollapseProvider>().is_none());
    item.set_has_unrealized_children(true);
    let expansion = peer.get_provider::<dyn IExpandCollapseProvider>().unwrap();
    assert_eq!(
        expansion.expand_collapse_state(),
        ExpandCollapseState::Collapsed
    );
    expansion.expand().unwrap();
    assert!(item.is_expanded());
    assert_eq!(
        expansion.expand_collapse_state(),
        ExpandCollapseState::Expanded
    );
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
    let _scene = attach(&nav);
    let peer = ControlAutomationPeer::create_peer_for_element(&item);
    // Item.HasChildren advertises the pattern, while NavigationView's source
    // predicate controls whether an owned provider may expand or collapse it.
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
    let _scene = attach(&nav);
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
fn nested_automation_selection_invokes_and_finds_its_container_after_popup_reparenting() {
    let _app = start();
    let nav = NavigationView::new();
    nav.set_pane_display_mode(NavigationViewPaneDisplayMode::Left);
    let parent = NavigationViewItem::new();
    let nested = NavigationViewItem::new();
    parent
        .menu_items()
        .add(Some(Control::boxed(nested.clone())));
    nav.menu_items().add(Some(Control::boxed(parent.clone())));
    let _scene = attach(&nav);
    nav.apply_template();
    parent.apply_template();
    let invocations = Rc::new(Cell::new(0));
    let observed = invocations.clone();
    let subscription = nav.item_invoked(move |_| observed.set(observed.get() + 1));
    let nav_peer = ControlAutomationPeer::create_peer_for_element(&nav);
    let nested_peer = ControlAutomationPeer::create_peer_for_element(&nested);
    let selection = nested_peer
        .get_provider::<dyn ISelectionItemProvider>()
        .unwrap();
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
    nav.set_pane_display_mode(NavigationViewPaneDisplayMode::Top);
    parent.set_is_expanded(true);
    assert_eq!(selection.selection_container().unwrap().peer(), nav_peer);
    assert_eq!(
        nested_peer.get_automation_control_type(),
        AutomationControlType::ListItem
    );
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
    let old_button = part::<Button>(&item, "NavigationViewItemPresenter");
    let peer = ControlAutomationPeer::create_peer_for_element(&item);
    let weak = item.downgrade();
    scene.0.set_content(None);
    scene.0.update_layout();
    drop(item);
    assert!(weak.upgrade().is_none());
    source.add("Later".to_string());
    old_button.on_click();
    assert!(weak.upgrade().is_none());
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
                item.menu_items()
                    .add(text("Local change while source active"));
            }
        }
    }));
    source.add("Second".to_string());
    assert!(changed.get());
    assert_eq!(children(&item).children().count(), 2);
    assert_eq!(item.menu_items().count(), 1);
    source.remove_collection_changed(token);
}

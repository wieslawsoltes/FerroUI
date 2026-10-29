//! Real grouped selection, equal geometry, accessibility, RTL keyboard and native check motion.

use super::support::start_themed_application;
use crate::{
    MaterialMotionRole, MaterialMotionScheme, MaterialSegmentedContent, MaterialSegmentedGroup,
    MaterialTheme, register_types,
};
use ferroui_base::animation::{IClock, IGlobalClock, PlayState, TimeSpan};
use ferroui_base::input::{
    InputElement, Key, KeyEventArgs, KeyboardDevice, KeyboardNavigation, KeyboardNavigationMode,
};
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::media::FlowDirection;
use ferroui_base::reactive::{IDisposable, IObservable, IObserver, LightweightSubject};
use ferroui_base::{CornerRadius, ObjectType, Ref, Visual};
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::automation::peers::{
    AutomationControlType, ControlAutomationPeer, RadioButtonAutomationPeer,
    ToggleButtonAutomationPeer,
};
use ferroui_controls::automation::provider::ISelectionProvider;
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::primitives::ToggleButton;
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_controls::{Border, Control, RadioButton, StackPanel, Viewbox, Window};
use std::cell::Cell;
use std::rc::Rc;

fn host(control: &Control) -> Ref<Window> {
    let border = Border::new();
    border.set_horizontal_alignment(HorizontalAlignment::Left);
    border.set_vertical_alignment(VerticalAlignment::Top);
    border.set_child(control.to_ref());
    let window = Window::new();
    window.set_width(640.0);
    window.set_height(320.0);
    window.set_content(Some(Control::boxed(&border)));
    window.show();
    window.update_layout();
    window
}
fn radio(label: &str) -> Ref<RadioButton> {
    let radio = RadioButton::new();
    radio.set_content(Some(Rc::new(label.to_owned())));
    radio
}
fn toggle(label: &str) -> Ref<ToggleButton> {
    let toggle = ToggleButton::new();
    toggle.set_content(Some(Rc::new(label.to_owned())));
    toggle
}
fn named<T: ObjectType>(visual: &Visual, name: &str) -> Ref<T> {
    visual
        .get_visual_descendants()
        .filter(|node| node.name().as_deref() == Some(name))
        .find_map(|node| node.cast::<T>())
        .unwrap_or_else(|| panic!("missing segmented part {name}"))
}
fn select(button: &RadioButton) {
    ControlAutomationPeer::create_peer_for_element(button)
        .cast::<RadioButtonAutomationPeer>()
        .unwrap()
        .select()
        .unwrap();
}
fn activate(button: &ToggleButton) {
    ControlAutomationPeer::create_peer_for_element(button)
        .cast::<ToggleButtonAutomationPeer>()
        .unwrap()
        .toggle()
        .unwrap();
}
fn key(target: &Control, key: Key) {
    let mut event = KeyEventArgs::new();
    event.set_routed_event(Some(InputElement::key_down_event()));
    event.key = key;
    target.raise_event(&event);
    assert!(
        event.handled(),
        "{key:?} must be handled by the segmented group"
    );
}

#[test]
fn single_choice_owns_logical_parent_scope_and_isolates_neighboring_groups() {
    let _app = start_themed_application();
    let first = MaterialSegmentedGroup::new();
    let second = MaterialSegmentedGroup::new();
    let a = radio("Day");
    let b = radio("Week");
    let other = radio("Other group");
    for button in [&a, &b, &other] {
        button.set_group_name(Some("caller-group".into()));
    }
    first.children().add(a.clone().upcast::<Control>());
    first.children().add(b.clone().upcast::<Control>());
    second.children().add(other.clone().upcast::<Control>());
    let parent = StackPanel::new();
    parent.children().add(first.clone().upcast::<Control>());
    parent.children().add(second.clone().upcast::<Control>());
    let _window = host(&parent);
    assert_eq!(None, a.group_name());
    assert_eq!(None, b.group_name());
    assert_eq!(None, other.group_name());
    select(&other);
    select(&a);
    select(&b);
    assert_eq!(Some(false), a.is_checked());
    assert_eq!(Some(true), b.is_checked());
    assert_eq!(Some(true), other.is_checked());
    assert_eq!(vec![1], first.selected_indices());
    assert_eq!(1, first.selected_index());
    activate(&b);
    assert_eq!(
        Some(true),
        b.is_checked(),
        "selected radio must not toggle off"
    );
    // A later application GroupName assignment cannot merge two owned groups.
    b.set_group_name(Some("caller-group".into()));
    assert_eq!(None, b.group_name());
    assert_eq!(Some(true), other.is_checked());
    select(&a);
    assert_eq!(Some(false), b.is_checked());
    assert_eq!(Some(true), other.is_checked());
}

#[test]
fn multiple_choice_keeps_native_independent_toggles_and_routed_selection_changes() {
    let _app = start_themed_application();
    let group = MaterialSegmentedGroup::new();
    group.set_is_multi_select(true);
    let a = toggle("Bold");
    let b = toggle("Italic");
    group.children().add(a.clone().upcast::<Control>());
    group.children().add(b.clone().upcast::<Control>());
    let changed = Rc::new(Cell::new(0));
    let count = changed.clone();
    group.selection_changed(move |_, _| count.set(count.get() + 1));
    let _window = host(&group);
    activate(&a);
    activate(&b);
    assert_eq!(vec![0, 1], group.selected_indices());
    assert_eq!(2, changed.get());
    activate(&a);
    assert_eq!(vec![1], group.selected_indices());
    assert_eq!(Some(false), a.is_checked());
    b.set_is_enabled(false);
    assert!(
        ControlAutomationPeer::create_peer_for_element(&b)
            .cast::<ToggleButtonAutomationPeer>()
            .unwrap()
            .toggle()
            .is_err()
    );
    assert_eq!(vec![1], group.selected_indices());
}

#[test]
fn equal_widths_overlap_one_dp_and_reserve_check_space_before_selection() {
    let _app = start_themed_application();
    let group = MaterialSegmentedGroup::new();
    let first = radio("Day");
    let middle = radio("Longer label");
    let last = radio("Month");
    for button in [&first, &middle, &last] {
        group.children().add(button.clone().upcast::<Control>());
    }
    let window = host(&group);
    let width = group.bounds().width;
    assert_eq!(first.bounds().width, middle.bounds().width);
    assert_eq!(middle.bounds().width, last.bounds().width);
    assert_eq!(first.bounds().right() - 1.0, middle.bounds().x);
    assert_eq!(middle.bounds().right() - 1.0, last.bounds().x);
    assert_eq!(48.0, group.bounds().height);
    let surface: Ref<Border> = named(&middle, "ChipContainer");
    let content: Ref<MaterialSegmentedContent> = named(&middle, "SegmentContent");
    let label: Ref<ContentPresenter> = named(&middle, "PART_ContentPresenter");
    assert_eq!(40.0, surface.bounds().height);
    assert_eq!(13.0, label.bounds().x);
    let content_width = content.bounds().width;
    select(&middle);
    window.update_layout();
    assert_eq!(width, group.bounds().width);
    assert_eq!(content_width, content.bounds().width);
    assert_eq!(26.0, label.bounds().x);
    assert!(middle.z_index() >= 5);
    assert_eq!(
        CornerRadius::new(20.0, 0.0, 0.0, 20.0),
        MaterialSegmentedGroup::get_item_corner_radius(&first)
    );
    assert_eq!(
        CornerRadius::default(),
        MaterialSegmentedGroup::get_item_corner_radius(&middle)
    );
    assert_eq!(
        CornerRadius::new(0.0, 20.0, 20.0, 0.0),
        MaterialSegmentedGroup::get_item_corner_radius(&last)
    );
}

#[test]
fn hidden_or_removed_children_rejoin_shapes_and_restore_owned_properties() {
    let _app = start_themed_application();
    let group = MaterialSegmentedGroup::new();
    let first = radio("One");
    let middle = radio("Two");
    let last = radio("Three");
    first.set_group_name(Some("application-group".into()));
    first.set_z_index(9);
    for button in [&first, &middle, &last] {
        group.children().add(button.clone().upcast::<Control>());
    }
    let window = host(&group);
    first.set_is_visible(false);
    window.update_layout();
    assert_eq!(
        CornerRadius::new(20.0, 0.0, 0.0, 20.0),
        MaterialSegmentedGroup::get_item_corner_radius(&middle)
    );
    last.set_is_visible(false);
    window.update_layout();
    assert_eq!(
        CornerRadius::uniform(20.0),
        MaterialSegmentedGroup::get_item_corner_radius(&middle)
    );
    group.children().remove(&first.clone().upcast::<Control>());
    assert_eq!(Some("application-group".to_owned()), first.group_name());
    assert_eq!(9, first.z_index());
    assert!(first.theme().is_none());
}

#[test]
fn keyboard_skips_disabled_items_and_reverses_physical_arrows_in_rtl() {
    let _app = start_themed_application();
    let group = MaterialSegmentedGroup::new();
    let first = radio("One");
    let disabled = radio("Two");
    let last = radio("Three");
    disabled.set_is_enabled(false);
    for button in [&first, &disabled, &last] {
        group.children().add(button.clone().upcast::<Control>());
    }
    let window = host(&group);
    assert_eq!(
        KeyboardNavigationMode::Once,
        KeyboardNavigation::get_tab_navigation(&group)
    );
    assert!(first.focus());
    key(&first, Key::Right);
    assert!(last.is_focused());
    assert_eq!(Some(true), last.is_checked());
    group.set_flow_direction(FlowDirection::RightToLeft);
    window.update_layout();
    key(&last, Key::Right);
    assert!(first.is_focused());
    assert_eq!(Some(true), first.is_checked());
    key(&first, Key::End);
    assert!(last.is_focused());
    key(&last, Key::Home);
    assert!(first.is_focused());
    // Coordinates stay logical and the normal visual policy mirrors the row once.
    assert!(first.transform_to_visual(&window).unwrap().m11 < 0.0);
}

#[test]
fn multi_choice_arrow_navigation_moves_focus_without_changing_checked_values() {
    let _app = start_themed_application();
    let group = MaterialSegmentedGroup::new();
    group.set_is_multi_select(true);
    let first = toggle("Bold");
    let last = toggle("Italic");
    first.set_is_checked(Some(true));
    group.children().add(first.clone().upcast::<Control>());
    group.children().add(last.clone().upcast::<Control>());
    let _window = host(&group);
    assert!(first.focus());
    key(&first, Key::Right);
    assert!(last.is_focused());
    assert_eq!(vec![0], group.selected_indices());
}

#[test]
fn selection_provider_and_children_report_their_native_accessibility_roles() {
    let _app = start_themed_application();
    for multiple in [false, true] {
        let group = MaterialSegmentedGroup::new();
        group.set_is_multi_select(multiple);
        AutomationProperties::set_name(&group, Some("View options"));
        let child: Ref<ToggleButton> = if multiple {
            toggle("Details")
        } else {
            radio("Details").upcast()
        };
        child.set_is_checked(Some(true));
        group.children().add(child.clone().upcast::<Control>());
        let window = host(&group);
        let peer = ControlAutomationPeer::create_peer_for_element(&group);
        assert_eq!(
            AutomationControlType::Group,
            peer.get_automation_control_type()
        );
        assert_eq!("View options", peer.get_name());
        let provider = peer
            .get_provider::<dyn ISelectionProvider>()
            .expect("group selection provider");
        assert_eq!(multiple, provider.can_select_multiple());
        let selection = provider.get_selection();
        assert_eq!(1, selection.len());
        assert_eq!(
            if multiple {
                AutomationControlType::CheckBox
            } else {
                AutomationControlType::RadioButton
            },
            selection[0].get_automation_control_type()
        );
        assert_eq!("Details", selection[0].get_name());
        window.close();
    }
}

struct TestClock {
    subject: LightweightSubject<TimeSpan>,
    state: Cell<PlayState>,
}
impl IObservable<TimeSpan> for TestClock {
    fn subscribe(&self, observer: Rc<dyn IObserver<TimeSpan>>) -> Rc<dyn IDisposable> {
        self.subject.subscribe(observer)
    }
}
impl IClock for TestClock {
    fn play_state(&self) -> PlayState {
        self.state.get()
    }
    fn set_play_state(&self, value: PlayState) {
        self.state.set(value);
    }
}
impl IGlobalClock for TestClock {}
fn moving_application() -> (UnitTestApplicationScope, Rc<TestClock>) {
    register_types();
    let clock = Rc::new(TestClock {
        subject: LightweightSubject::new(),
        state: Cell::new(PlayState::Run),
    });
    let app = UnitTestApplication::start(
        TestServices::styled_window()
            .with_keyboard_device(|| Some(KeyboardDevice::new()))
            .with_global_clock(clock.clone())
            .with_platform_settings(crate::motion::MotionTestSettings::new())
            .with_theme(|| MaterialTheme::new().as_style()),
    );
    (app, clock)
}

#[test]
fn check_entry_uses_separate_effects_and_spatial_springs_but_exit_is_immediate() {
    let (_app, clock) = moving_application();
    let group = MaterialSegmentedGroup::new();
    let child = radio("Choice");
    group.children().add(child.clone().upcast::<Control>());
    let window = host(&group);
    let content: Ref<MaterialSegmentedContent> = named(&child, "SegmentContent");
    let check: Ref<Viewbox> = named(&child, "CheckIcon");
    let width = group.bounds().width;
    select(&child);
    clock.subject.on_next(TimeSpan::from_seconds(0.0));
    clock.subject.on_next(TimeSpan::from_seconds(0.05));
    let scheme = MaterialMotionScheme::Expressive;
    let spatial = scheme
        .spring(MaterialMotionRole::FastSpatial)
        .sample(0.05, 0.0, 1.0, 0.0);
    let effects = scheme
        .spring(MaterialMotionRole::DefaultEffects)
        .sample(0.05, 0.0, 1.0, 0.0);
    assert!((spatial.value - content.label_progress()).abs() < 1e-9);
    assert!((spatial.value - content.icon_scale()).abs() < 1e-9);
    assert!((effects.value - check.opacity()).abs() < 1e-9);
    window.update_layout();
    assert_eq!(width, group.bounds().width);
    child.set_is_checked(Some(false));
    assert_eq!(0.0, content.icon_scale());
    assert_eq!(0.0, check.opacity());
    assert!((spatial.value - content.label_progress()).abs() < 1e-9);
    clock.subject.on_next(TimeSpan::from_seconds(0.1));
    let reversed = scheme.spring(MaterialMotionRole::FastSpatial).sample(
        0.05,
        spatial.value,
        0.0,
        spatial.velocity,
    );
    assert!((reversed.value - content.label_progress()).abs() < 1e-9);
}

#[test]
fn reduced_motion_settles_and_hidden_or_removed_content_releases_its_clock() {
    let (_app, clock) = moving_application();
    let group = MaterialSegmentedGroup::new();
    let child = radio("Choice");
    group.children().add(child.clone().upcast::<Control>());
    let _window = host(&group);
    let content: Ref<MaterialSegmentedContent> = named(&child, "SegmentContent");
    select(&child);
    assert!(clock.subject.has_observers());
    child.set_is_visible(false);
    assert!(!clock.subject.has_observers());
    child.set_is_visible(true);
    assert!(clock.subject.has_observers());
    content.set_reduced_motion(true);
    assert_eq!(1.0, content.label_progress());
    assert_eq!(1.0, content.icon_opacity());
    assert!(!clock.subject.has_observers());
    content.set_reduced_motion(false);
    child.set_is_checked(Some(false));
    assert!(clock.subject.has_observers());
    group.children().remove(&child.clone().upcast::<Control>());
    assert!(!clock.subject.has_observers());
}

//! Tests of the state triggers (docs/porting/winui-calculator-gaps.md, section 5.7), against
//! WinUI's `StateTriggerVariantMap` (`VariantMap.h`) and `CVisualStateManager2::OnSelectionChanged`,
//! with a port of the Windows Calculator's `ControlSizeTrigger` as the example of the extension
//! point.

use crate::test_support::{test_scope, TestRoot};
use crate::{
    AdaptiveTrigger, Border, Control, StateTrigger, StateTriggerBase, VisualState, VisualStateGroup, VisualStateManager,
};
use ferroui_base::reactive::IDisposable;
use ferroui_base::styling::{Setter, TargetPropertyPath};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, Ref, Size, StyledProperty, Visual,
};
use std::cell::RefCell;
use std::rc::Rc;

/// A state named `name` that sets the opacity of `target` to `value`.
fn opacity_state(name: &str, target: &Border, value: f64) -> Ref<VisualState> {
    let state = VisualState::with_name(name);
    let setter = Setter::new(Visual::opacity_property(), value);
    setter.set_target(Some(TargetPropertyPath::new(target, Visual::opacity_property().as_property())));
    state.setters().add(setter);
    state
}

fn adaptive(min_width: f64, min_height: f64) -> Ref<StateTriggerBase> {
    let trigger = AdaptiveTrigger::new();
    trigger.set_min_window_width(min_width);
    trigger.set_min_window_height(min_height);
    trigger.upcast()
}

/// A root of the given client size with a border whose groups are `group`; the border is the
/// target of the setters.
fn rooted(size: Size, group: impl FnOnce(&Ref<Border>) -> Ref<VisualStateGroup>) -> (Ref<TestRoot>, Ref<Border>) {
    let border = Border::new();
    let group = group(&border);
    VisualStateManager::get_visual_state_groups(&border).add(group);
    let root = TestRoot::new();
    root.set_client_size(size);
    root.set_child(&border);
    root.execute_initial_layout_pass();
    (root, border)
}

fn current(border: &Border) -> Option<String> {
    VisualStateManager::get_visual_state_groups(border).get(0).current_state().map(|state| state.name())
}

#[test]
fn a_state_trigger_enters_its_state_while_it_is_active() {
    let _scope = test_scope();
    let trigger = StateTrigger::new();
    let (_root, border) = rooted(Size::new(800.0, 600.0), |border| {
        let group = VisualStateGroup::with_name("Modes");
        let state = opacity_state("Active", border, 0.5);
        state.state_triggers().add(trigger.clone().upcast());
        group.states().add(state);
        group
    });
    assert_eq!(current(&border), None);

    trigger.set_is_active(true);
    assert_eq!(current(&border).as_deref(), Some("Active"));
    assert_eq!(border.opacity(), 0.5);

    // No trigger qualifies: the group has no state, and the values are removed.
    trigger.set_is_active(false);
    assert_eq!(current(&border), None);
    assert_eq!(border.opacity(), 1.0);
}

#[test]
fn adaptive_triggers_compare_with_the_window_and_the_larger_minimum_wins() {
    let _scope = test_scope();
    let (root, border) = rooted(Size::new(500.0, 400.0), |border| {
        let group = VisualStateGroup::with_name("WindowStates");
        let narrow = opacity_state("Narrow", border, 0.25);
        narrow.state_triggers().add(adaptive(0.0, -1.0));
        let medium = opacity_state("Medium", border, 0.5);
        medium.state_triggers().add(adaptive(640.0, -1.0));
        let wide = opacity_state("Wide", border, 0.75);
        wide.state_triggers().add(adaptive(1008.0, -1.0));
        group.states().add(narrow);
        group.states().add(medium);
        group.states().add(wide);
        group
    });
    // The element is small; the window decides.
    border.set_width(10.0);
    assert_eq!(current(&border).as_deref(), Some("Narrow"));

    root.set_client_size(Size::new(700.0, 400.0));
    root.execute_initial_layout_pass();
    assert_eq!(current(&border).as_deref(), Some("Medium"));
    assert_eq!(border.opacity(), 0.5);

    root.set_client_size(Size::new(1200.0, 400.0));
    root.execute_initial_layout_pass();
    assert_eq!(current(&border).as_deref(), Some("Wide"));
}

#[test]
fn the_larger_minimum_height_breaks_a_tie_and_no_minimum_never_applies() {
    let _scope = test_scope();
    let (_root, border) = rooted(Size::new(800.0, 800.0), |border| {
        let group = VisualStateGroup::with_name("WindowStates");
        let never = opacity_state("Never", border, 0.1);
        never.state_triggers().add(adaptive(-1.0, -1.0));
        let short = opacity_state("Short", border, 0.25);
        short.state_triggers().add(adaptive(500.0, 300.0));
        let tall = opacity_state("Tall", border, 0.5);
        tall.state_triggers().add(adaptive(500.0, 700.0));
        group.states().add(never);
        group.states().add(short);
        group.states().add(tall);
        group
    });
    assert_eq!(current(&border).as_deref(), Some("Tall"));
}

#[test]
fn a_state_trigger_wins_over_adaptive_triggers() {
    let _scope = test_scope();
    let trigger = StateTrigger::new();
    let (_root, border) = rooted(Size::new(1200.0, 800.0), |border| {
        let group = VisualStateGroup::with_name("Modes");
        let wide = opacity_state("Wide", border, 0.75);
        wide.state_triggers().add(adaptive(1000.0, -1.0));
        let forced = opacity_state("Forced", border, 0.25);
        forced.state_triggers().add(trigger.clone().upcast());
        group.states().add(wide);
        group.states().add(forced);
        group
    });
    assert_eq!(current(&border).as_deref(), Some("Wide"));
    trigger.set_is_active(true);
    assert_eq!(current(&border).as_deref(), Some("Forced"));
    trigger.set_is_active(false);
    assert_eq!(current(&border).as_deref(), Some("Wide"));
}

#[test]
fn a_manual_state_holds_until_the_triggers_select_another_one() {
    let _scope = test_scope();
    let trigger = StateTrigger::new();
    let (_root, border) = rooted(Size::new(800.0, 600.0), |border| {
        let group = VisualStateGroup::with_name("Modes");
        let a = opacity_state("A", border, 0.25);
        a.state_triggers().add(trigger.clone().upcast());
        group.states().add(a);
        group.states().add(opacity_state("B", border, 0.5));
        group
    });
    // The element is the root of its own states, so its group is changed directly, as
    // `GoToState` of its control would.
    let group = VisualStateManager::get_visual_state_groups(&border).get(0);
    trigger.set_is_active(true);
    assert_eq!(current(&border).as_deref(), Some("A"));
    VisualStateManager::change_state(&border.to_ref().upcast(), &group, group.find_state("B").as_ref(), false);
    assert_eq!(current(&border).as_deref(), Some("B"));
    // Re-evaluating with the same selection changes nothing.
    trigger.set_is_active(true);
    assert_eq!(current(&border).as_deref(), Some("B"));
    // A new selection applies.
    trigger.set_is_active(false);
    assert_eq!(current(&border), None);
}

#[test]
fn triggers_are_evaluated_when_the_element_enters_a_tree() {
    let _scope = test_scope();
    let trigger = StateTrigger::new();
    trigger.set_is_active(true);
    let border = Border::new();
    let group = VisualStateGroup::with_name("Modes");
    let state = opacity_state("Active", &border, 0.5);
    state.state_triggers().add(trigger.clone().upcast());
    group.states().add(state);
    VisualStateManager::get_visual_state_groups(&border).add(group);
    assert_eq!(current(&border), None);

    let root = TestRoot::new();
    root.set_child(&border);
    assert_eq!(current(&border).as_deref(), Some("Active"));
    assert_eq!(border.opacity(), 0.5);
}

// --- The extension point: the Calculator's ControlSizeTrigger ------------------------------

/// A port of `Calculator/Views/StateTriggers/ControlSizeTrigger.cs` of the Windows Calculator
/// (microsoft/calculator `79e1ac53`, MIT): active while its source is at least `MinWidth` wide and
/// `MinHeight` high. It derives from [`StateTriggerBase`] and calls `set_active`.
#[repr(C)]
struct ControlSizeTrigger {
    base: StateTriggerBase,
    size_changed: RefCell<Option<Rc<dyn IDisposable>>>,
}

ferro_class!(ControlSizeTrigger: StateTriggerBase);
ferro_class_info!(ControlSizeTrigger {});

impl FerroObjectImpl for ControlSizeTrigger {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if change.property() == Self::source_property().as_property() {
            this.on_source_changed(this.source());
        }
    }
}

ferro_properties! { impl ControlSizeTrigger {
    fn source_property() -> StyledProperty<Option<Ref<Control>>> {
        FerroProperty::register::<ControlSizeTrigger, _>("Source", None)
    }
    fn min_width_property() -> StyledProperty<f64> {
        FerroProperty::register::<ControlSizeTrigger, _>("MinWidth", -1.0)
    }
    fn min_height_property() -> StyledProperty<f64> {
        FerroProperty::register::<ControlSizeTrigger, _>("MinHeight", -1.0)
    }
} }

impl ControlSizeTrigger {
    fn new(min_width: f64, min_height: f64) -> Ref<Self> {
        let this = instantiate(Self { base: StateTriggerBase::construct(), size_changed: RefCell::new(None) });
        // `ControlSizeTrigger()`: SetActive(false).
        this.set_active(false);
        this.set_value(Self::min_width_property(), min_width);
        this.set_value(Self::min_height_property(), min_height);
        this
    }

    fn source(&self) -> Option<Ref<Control>> {
        self.get_value(Self::source_property())
    }

    fn on_source_changed(&self, source: Option<Ref<Control>>) {
        if let Some(old) = self.size_changed.borrow_mut().take() {
            old.dispose();
        }
        let Some(source) = source else { return };
        let weak = self.to_ref().downgrade();
        let subscription = source.property_changed(move |change| {
            if change.property() == Visual::bounds_property().as_property() {
                if let Some(this) = weak.upgrade() {
                    let size = change.get_new_value::<ferroui_base::Rect>().size();
                    this.update_is_active(size);
                }
            }
        });
        *self.size_changed.borrow_mut() = Some(subscription);
        self.update_is_active(source.bounds().size());
    }

    // `UpdateIsActive`.
    fn update_is_active(&self, size: Size) {
        let min_height = self.get_value(Self::min_height_property());
        let min_width = self.get_value(Self::min_width_property());
        if min_height >= 0.0 {
            self.set_active(size.height >= min_height && (min_width < 0.0 || size.width >= min_width));
        } else {
            self.set_active(min_width >= 0.0 && size.width >= min_width);
        }
    }
}

#[test]
fn a_custom_trigger_derives_from_state_trigger_base() {
    let _scope = test_scope();
    let source = Border::new();
    let large = ControlSizeTrigger::new(780.0, 814.0);
    let medium = ControlSizeTrigger::new(468.0, 500.0);
    let (root, border) = rooted(Size::new(400.0, 400.0), |border| {
        let group = VisualStateGroup::with_name("Sizing");
        let large_state = opacity_state("Large", border, 0.75);
        large_state.state_triggers().add(large.clone().upcast());
        let medium_state = opacity_state("Medium", border, 0.5);
        medium_state.state_triggers().add(medium.clone().upcast());
        group.states().add(large_state);
        group.states().add(medium_state);
        group
    });
    border.set_child(Some(source.clone().upcast()));
    large.set_value(ControlSizeTrigger::source_property(), Some(source.clone().upcast()));
    medium.set_value(ControlSizeTrigger::source_property(), Some(source.clone().upcast()));
    root.execute_initial_layout_pass();
    assert_eq!(current(&border), None);

    root.set_client_size(Size::new(600.0, 600.0));
    root.execute_initial_layout_pass();
    assert_eq!(current(&border).as_deref(), Some("Medium"));

    // Both qualify: two extensible triggers rank alike, so the first in the source wins.
    root.set_client_size(Size::new(900.0, 900.0));
    root.execute_initial_layout_pass();
    assert_eq!(current(&border).as_deref(), Some("Large"));
    assert_eq!(border.opacity(), 0.75);
}

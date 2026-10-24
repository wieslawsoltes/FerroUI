//! Tests of the visual state manager (docs/porting/winui-calculator-gaps.md, section 5), against
//! the behaviour of WinUI's `VisualStateManager` (`CVisualStateManager2.cpp`,
//! `VisualStateManagerActuator.cpp`).

use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions};
use crate::test_support::test_scope;
use crate::{
    Border, ContentControl, Control, UserControl, VisualState, VisualStateGroup, VisualStateManager,
    VisualStateManagerImpl,
};
use ferroui_base::styling::{Selectors, Setter, Style, TargetPropertyPath};
use ferroui_base::threading::Dispatcher;
use ferroui_base::{ferro_class, ferro_class_info, instantiate, FerroObject, Ref, Visual};
use std::cell::RefCell;
use std::rc::Rc;

/// A setter of `property` of `target` to `value`.
fn setter(target: &FerroObject, property: &'static ferroui_base::StyledProperty<f64>, value: f64) -> Rc<Setter> {
    let setter = Setter::new(property, value);
    setter.set_target(Some(TargetPropertyPath::new(target, property.as_property())));
    setter
}

/// A state with setters of the opacity of `target`.
fn opacity_state(name: &str, target: &FerroObject, value: f64) -> Ref<VisualState> {
    let state = VisualState::with_name(name);
    state.setters().add(setter(target, Visual::opacity_property(), value));
    state
}

/// A content control whose template is a border (the root of the states) with a border named
/// `Inner`, the target of the setters. The template root gets `groups` (built for the inner
/// border, which is passed to `groups`).
fn templated(groups: impl Fn(&Ref<Border>) -> Vec<Ref<VisualStateGroup>> + 'static) -> Ref<ContentControl> {
    let control = ContentControl::new();
    let groups = Rc::new(groups);
    control.set_template(Some(FuncControlTemplate::new(move |_, scope| {
        let inner = Border::new();
        inner.set_name(Some("Inner".to_string()));
        let root = Border::new();
        root.set_child(inner.clone().register_in_name_scope(&**scope));
        let collection = VisualStateManager::get_visual_state_groups(&root);
        for group in groups(&inner) {
            collection.add(group);
        }
        root.upcast()
    })));
    control.apply_template();
    control
}

fn inner_of(control: &Ref<ContentControl>) -> Ref<Border> {
    let root = control.visual_children().snapshot()[0].cast::<Border>().unwrap();
    root.child().unwrap().cast::<Border>().unwrap()
}

fn common_states(inner: &Ref<Border>) -> Vec<Ref<VisualStateGroup>> {
    let group = VisualStateGroup::with_name("CommonStates");
    group.states().add(VisualState::with_name("Normal"));
    group.states().add(opacity_state("PointerOver", inner, 0.75));
    group.states().add(opacity_state("Pressed", inner, 0.25));
    vec![group]
}

#[test]
fn go_to_state_applies_the_setters_of_the_state() {
    let _scope = test_scope();
    let control = templated(common_states);
    let inner = inner_of(&control);

    assert!(VisualStateManager::go_to_state(&control, "PointerOver", false));
    assert_eq!(inner.opacity(), 0.75);

    assert!(VisualStateManager::go_to_state(&control, "Pressed", true));
    assert_eq!(inner.opacity(), 0.25);

    // A state without setters removes the values of the state it leaves.
    assert!(VisualStateManager::go_to_state(&control, "Normal", true));
    assert_eq!(inner.opacity(), 1.0);
}

#[test]
fn go_to_state_without_the_state_returns_false_and_changes_nothing() {
    let _scope = test_scope();
    let control = templated(common_states);
    let inner = inner_of(&control);
    assert!(VisualStateManager::go_to_state(&control, "Pressed", false));

    // Names are case-sensitive, as WinUI compares them.
    assert!(!VisualStateManager::go_to_state(&control, "pressed", false));
    assert!(!VisualStateManager::go_to_state(&control, "Missing", false));
    assert_eq!(inner.opacity(), 0.25);
}

#[test]
fn go_to_state_without_groups_returns_false() {
    let _scope = test_scope();
    let control = templated(|_| Vec::new());
    assert!(!VisualStateManager::go_to_state(&control, "Normal", false));

    // No template applied: no root.
    let bare = ContentControl::new();
    assert!(!VisualStateManager::go_to_state(&bare, "Normal", false));
}

#[test]
fn the_values_of_a_state_are_above_local_values_and_styles_and_leave_them() {
    let _scope = test_scope();
    let control = templated(common_states);
    let inner = inner_of(&control);
    inner.set_opacity(0.5);

    assert!(VisualStateManager::go_to_state(&control, "Pressed", false));
    assert_eq!(inner.opacity(), 0.25);
    // A local value set while the state is current does not replace the state's value...
    inner.set_opacity(0.4);
    assert_eq!(inner.opacity(), 0.25);

    // ...and is the value again when the state is left.
    assert!(VisualStateManager::go_to_state(&control, "Normal", false));
    assert_eq!(inner.opacity(), 0.4);
}

#[test]
fn the_values_of_a_state_survive_the_invalidation_of_styles() {
    let _scope = test_scope();
    let control = templated(common_states);
    let inner = inner_of(&control);
    inner.styles().add(Style::with_setters(Selectors::of_type::<Border>(), [Setter::new(Visual::opacity_property(), 0.9)]));
    inner.apply_styling();

    assert!(VisualStateManager::go_to_state(&control, "PointerOver", false));
    assert_eq!(inner.opacity(), 0.75);

    // The styles of the element are applied again (a class change, a new style).
    inner.styles().add(Style::with_setters(Selectors::of_type::<Control>(), [Setter::new(Visual::opacity_property(), 0.8)]));
    inner.apply_styling();
    assert_eq!(inner.opacity(), 0.75);

    assert!(VisualStateManager::go_to_state(&control, "Normal", false));
    assert_eq!(inner.opacity(), 0.9);
}

#[test]
fn the_groups_of_an_element_are_independent() {
    let _scope = test_scope();
    let control = templated(|inner| {
        let mut groups = common_states(inner);
        let focus = VisualStateGroup::with_name("FocusStates");
        let focused = VisualState::with_name("Focused");
        let width = Setter::new(crate::Layoutable::width_property(), 20.0);
        width.set_target(Some(TargetPropertyPath::new(inner, crate::Layoutable::width_property().as_property())));
        focused.setters().add(width);
        focus.states().add(focused);
        focus.states().add(VisualState::with_name("Unfocused"));
        groups.push(focus);
        groups
    });
    let inner = inner_of(&control);

    assert!(VisualStateManager::go_to_state(&control, "Pressed", false));
    assert!(VisualStateManager::go_to_state(&control, "Focused", false));
    assert_eq!(inner.opacity(), 0.25);
    assert_eq!(inner.width(), 20.0);

    assert!(VisualStateManager::go_to_state(&control, "Unfocused", false));
    assert_eq!(inner.opacity(), 0.25);
    assert!(inner.width().is_nan());
}

#[test]
fn the_current_state_and_the_events_of_a_group() {
    let _scope = test_scope();
    let control = templated(common_states);
    let root = control.visual_children().snapshot()[0].cast::<Control>().unwrap();
    let group = VisualStateManager::get_visual_state_groups(&root).get(0);
    assert!(group.current_state().is_none());

    let log = Rc::new(RefCell::new(Vec::<String>::new()));
    let names = |args: &crate::VisualStateChangedEventArgs| {
        let name = |state: Option<Ref<VisualState>>| state.map(|state| state.name()).unwrap_or_else(|| "-".to_string());
        format!("{}>{}", name(args.old_state()), name(args.new_state()))
    };
    let changing = log.clone();
    let _a = group.current_state_changing(move |_, args| changing.borrow_mut().push(format!("changing {}", names(args))));
    let changed = log.clone();
    let _b = group.current_state_changed(move |_, args| changed.borrow_mut().push(format!("changed {}", names(args))));

    assert!(VisualStateManager::go_to_state(&control, "PointerOver", true));
    assert_eq!(group.current_state().map(|state| state.name()).as_deref(), Some("PointerOver"));
    // The events are queued, as WinUI's event manager raises them.
    assert!(log.borrow().is_empty());
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(*log.borrow(), ["changing ->PointerOver", "changed ->PointerOver"]);

    // Entering the current state again raises nothing.
    assert!(VisualStateManager::go_to_state(&control, "PointerOver", true));
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(log.borrow().len(), 2);
}

#[test]
fn the_states_of_a_user_control_are_on_its_content() {
    let _scope = test_scope();
    let control = UserControl::new();
    control.set_template(Some(FuncControlTemplate::new(|_, _| Border::new().upcast())));
    let content = Border::new();
    let inner = Border::new();
    content.set_child(inner.clone());
    let group = VisualStateGroup::with_name("LayoutStates");
    group.states().add(opacity_state("Small", &inner, 0.5));
    VisualStateManager::get_visual_state_groups(&content).add(group);
    control.set_content(Some(Control::boxed(content)));
    control.apply_template();

    assert!(VisualStateManager::go_to_state(&control, "Small", false));
    assert_eq!(inner.opacity(), 0.5);
}

#[test]
fn replacing_the_groups_removes_their_values() {
    let _scope = test_scope();
    let control = templated(common_states);
    let inner = inner_of(&control);
    let root = control.visual_children().snapshot()[0].cast::<Control>().unwrap();
    assert!(VisualStateManager::go_to_state(&control, "Pressed", false));
    assert_eq!(inner.opacity(), 0.25);

    VisualStateManager::set_visual_state_groups(&root, None);
    assert_eq!(inner.opacity(), 1.0);
    assert!(!VisualStateManager::go_to_state(&control, "Pressed", false));
}

#[test]
fn a_setter_whose_target_is_gone_does_nothing() {
    let _scope = test_scope();
    let control = templated(|_| {
        // The target is held weakly, and nothing else holds this border.
        let gone = Border::new();
        let group = VisualStateGroup::with_name("CommonStates");
        group.states().add(opacity_state("Pressed", &gone, 0.25));
        vec![group]
    });
    assert!(VisualStateManager::go_to_state(&control, "Pressed", false));
}

/// A manager that records the calls it gets.
#[repr(C)]
struct RecordingVisualStateManager {
    base: VisualStateManager,
    calls: RefCell<Vec<(String, bool)>>,
}

ferro_class!(RecordingVisualStateManager: VisualStateManager);
ferro_class_info!(RecordingVisualStateManager {});
ferroui_base::ferro_impl_classes!(RecordingVisualStateManager: ferroui_base::FerroObjectImpl);

impl VisualStateManagerImpl for RecordingVisualStateManager {
    fn go_to_state_core(
        this: &Self,
        control: &Ref<Control>,
        template_root: &Ref<Control>,
        state_name: &str,
        group: Option<&Ref<VisualStateGroup>>,
        state: Option<&Ref<VisualState>>,
        use_transitions: bool,
    ) -> bool {
        this.calls.borrow_mut().push((state_name.to_string(), state.is_some()));
        Self::parent_go_to_state_core(this, control, template_root, state_name, group, state, use_transitions)
    }
}

#[test]
fn a_custom_manager_is_called_for_every_state_name() {
    let _scope = test_scope();
    let control = templated(common_states);
    let inner = inner_of(&control);
    let root = control.visual_children().snapshot()[0].cast::<Control>().unwrap();
    let manager: Ref<RecordingVisualStateManager> =
        instantiate(RecordingVisualStateManager { base: VisualStateManager::construct(), calls: RefCell::new(Vec::new()) });
    VisualStateManager::set_custom_visual_state_manager(&root, Some(manager.clone().upcast()));

    assert!(VisualStateManager::go_to_state(&control, "Pressed", false));
    assert!(!VisualStateManager::go_to_state(&control, "Missing", false));
    assert_eq!(inner.opacity(), 0.25);
    assert_eq!(*manager.calls.borrow(), [("Pressed".to_string(), true), ("Missing".to_string(), false)]);
}

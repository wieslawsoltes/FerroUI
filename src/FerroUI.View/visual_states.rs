//! Typed, scoped state styles on a declarative view.

mod styles;

use crate::{IntoValue, TypedProperty};
use ferroui_base::animation::{easings::Easing, Animation};
use ferroui_base::{AttachedProperty, BoxedValue, FerroProperty};
use ferroui_controls::Control;
use std::collections::BTreeSet;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

/// The typed property overrides for one visual state.
pub struct ViewState {
    name: String,
    setters: Vec<StateSetter>,
}

/// Mutually exclusive states and optional default and directional routes.
pub struct ViewStateGroup {
    name: String,
    states: Vec<ViewState>,
    routes: Vec<Route>,
}

struct StateSetter {
    target: String,
    property: &'static FerroProperty,
    value: BoxedValue,
}

struct Route {
    from: Option<String>,
    to: String,
    duration: Duration,
    easing: Easing,
}

/// Declare a state by name. Its overrides apply while the state is selected.
pub fn visual_state(name: &str) -> ViewState {
    ViewState {
        name: name.into(),
        setters: Vec::new(),
    }
}

/// Declare a group scoped to a control's `visual_states` builder field.
pub fn visual_state_group(name: &str) -> ViewStateGroup {
    ViewStateGroup {
        name: name.into(),
        states: Vec::new(),
        routes: Vec::new(),
    }
}

impl ViewState {
    /// Set an animatable styled property on the owner of the group.
    pub fn set<P: TypedProperty + ?Sized>(
        self,
        property: &'static P,
        value: impl IntoValue<P::Value>,
    ) -> Self {
        self.set_on("", property, value)
    }

    /// Set an animatable styled property on a named descendant in the owner's view.
    pub fn set_on<P: TypedProperty + ?Sized>(
        mut self,
        target: &str,
        property: &'static P,
        value: impl IntoValue<P::Value>,
    ) -> Self {
        assert!(
            target.chars().all(|ch| ch.is_alphanumeric() || ch == '_'),
            "A visual state target uses a control name"
        );
        let property = property.untyped();
        assert!(
            Animation::can_animate_property(property),
            "Visual state property {property} requires a styled property with a native or registered animator"
        );
        assert!(
            !self
                .setters
                .iter()
                .any(|item| item.target == target && std::ptr::eq(item.property, property)),
            "Each visual state sets a target property at most once"
        );
        self.setters.push(StateSetter {
            target: target.into(),
            property,
            value: Rc::new(value.into_value()),
        });
        self
    }
}

impl ViewStateGroup {
    /// Add a state to this group.
    pub fn state(mut self, state: ViewState) -> Self {
        self.states.push(state);
        self
    }

    /// Add a route to `to`. An exact `from` route wins over the default (`None`).
    /// The easing uses the native easing name or comma-separated Bézier controls.
    pub fn transition(
        mut self,
        from: Option<&str>,
        to: &str,
        duration: Duration,
        easing: &str,
    ) -> Self {
        assert_ne!(
            from,
            Some(to),
            "A visual state route connects distinct states"
        );
        let easing = easing.trim();
        let easing = if easing.is_empty() {
            Easing::default()
        } else {
            Easing::parse(easing).expect("Invalid visual state transition easing")
        };
        self.routes.push(Route {
            from: from.map(str::to_owned),
            to: to.into(),
            duration,
            easing,
        });
        self
    }
}

pub(crate) fn validate(groups: &[ViewStateGroup]) {
    let mut names = BTreeSet::new();
    for group in groups {
        assert!(
            !group.name.trim().is_empty() && names.insert(group.name.as_str()),
            "Visual state group names must be unique on the owner"
        );
        let mut states = BTreeSet::new();
        for state in &group.states {
            assert!(
                !state.name.trim().is_empty() && states.insert(state.name.as_str()),
                "Visual state names must be unique in the group"
            );
        }
        let mut routes = BTreeSet::new();
        for route in &group.routes {
            assert!(
                states.contains(route.to.as_str())
                    && route
                        .from
                        .as_deref()
                        .is_none_or(|from| states.contains(from)),
                "A visual state route uses states from its group"
            );
            assert!(
                routes.insert((&route.from, &route.to)),
                "A visual state route may only be declared once"
            );
        }
    }
}

#[derive(Clone)]
struct MountedGroups(Rc<Vec<MountedGroup>>);

impl PartialEq for MountedGroups {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

#[derive(Clone)]
struct MountedGroup {
    name: String,
    rules: Vec<StateRule>,
}

#[derive(Clone)]
struct StateRule {
    class: String,
    state: String,
    kind: StateRuleKind,
}

#[derive(Clone)]
enum StateRuleKind {
    Endpoint,
    Transition { from: Option<String> },
}

fn groups_property() -> &'static AttachedProperty<Option<MountedGroups>> {
    ferroui_base::ferro_thread_local! {
        static GROUPS: &'static AttachedProperty<Option<MountedGroups>> =
            FerroProperty::register_attached::<Control, Control, _>("ViewStateGroups", None);
    }
    GROUPS.with(|property| *property)
}

pub(crate) fn install(owner: &Control, groups: &[ViewStateGroup]) {
    static NEXT_SCOPE: AtomicUsize = AtomicUsize::new(0);
    let mut mounted = owner
        .get_value(groups_property())
        .map_or_else(Vec::new, |groups| (*groups.0).clone());
    for group in groups {
        assert!(
            !mounted.iter().any(|existing| existing.name == group.name),
            "Visual state group names must be unique on the owner"
        );
        let anchor = format!(
            "ferroui_view_state_{}",
            NEXT_SCOPE.fetch_add(1, Ordering::Relaxed)
        );
        let (style, state) = styles::build_style(group, &anchor);
        owner.classes().add(&anchor);
        owner.styles().add(style);
        mounted.push(state);
    }
    owner.set_value(groups_property(), Some(MountedGroups(Rc::new(mounted))));
}

/// Enter a declared state, or restore the underlying view with `None`.
/// Returns false if the caller names a group or state absent from this owner.
pub fn set_visual_state(owner: &Control, group: &str, state: Option<&str>, animate: bool) -> bool {
    let Some(groups) = owner.get_value(groups_property()) else {
        return false;
    };
    let Some(group) = groups.0.iter().find(|item| item.name == group) else {
        return false;
    };
    let classes = owner.classes();
    let current = group
        .rules
        .iter()
        .find(|rule| classes.contains(&rule.class));
    let previous = current.map(|rule| rule.state.as_str());
    let destination = match state {
        Some(state) => {
            let candidate = group
                .rules
                .iter()
                .find(|rule| rule.state == state && matches!(rule.kind, StateRuleKind::Endpoint));
            let Some(endpoint) = candidate else {
                return false;
            };
            if animate && previous == Some(state) {
                return true;
            }
            let route = |previous| {
                group.rules.iter().find(|rule| {
                rule.state == state && matches!(&rule.kind, StateRuleKind::Transition { from } if from.as_deref() == previous)
            })
            };
            Some(if animate {
                route(previous).or_else(|| route(None)).unwrap_or(endpoint)
            } else {
                endpoint
            })
        }
        None => None,
    };
    if current.map(|rule| &rule.class) == destination.map(|rule| &rule.class) {
        return true;
    }
    if let Some(current) = current {
        classes.remove(&current.class);
    }
    if let Some(rule) = destination {
        classes.add(&rule.class);
    }
    true
}

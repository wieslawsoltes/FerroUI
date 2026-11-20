//! Observational diagnostic for one property in the live value store.
use super::{FerroObjectDiagnosticExtensions, FerroPropertyValue};
use crate::data::BindingPriority;
use crate::property_store::{IValueEntry, entry_ptr_eq};
use crate::styling::{ControlTheme, IStyleInstance, Setter, SetterValue, StyleInstance};
use crate::{BoxedValue, FerroObject, FerroProperty, Ref};
use std::rc::Rc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValueContributorKind {
    Direct,
    CurrentValue,
    Animation,
    Local,
    Style,
    Theme,
    Template,
    Inherited,
    Default,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValueContributorState {
    Effective,
    Base,
    Overridden,
    Inactive,
    Unset,
}

pub struct ValueContributor {
    pub kind: ValueContributorKind,
    pub state: ValueContributorState,
    pub priority: BindingPriority,
    /// An observed value or a literal setter value. Other entries are left unevaluated.
    pub value: Option<BoxedValue>,
    pub source: Option<Ref<FerroObject>>,
    pub binding: bool,
}

impl ValueContributor {
    fn new(
        kind: ValueContributorKind,
        state: ValueContributorState,
        priority: BindingPriority,
    ) -> Self {
        Self {
            kind,
            state,
            priority,
            value: None,
            source: None,
            binding: false,
        }
    }
}

pub struct PropertyValueTrace {
    pub effective: FerroPropertyValue,
    pub contributors: Vec<ValueContributor>,
}

impl PropertyValueTrace {
    /// Reads the accepted effective entry by identity. This never subscribes to
    /// a losing binding or evaluates a style activator to fill a diagnostic row.
    pub fn read(object: &FerroObject, property: &'static FerroProperty) -> Self {
        use BindingPriority as Priority;
        use ValueContributorKind as Kind;
        use ValueContributorState as State;

        let effective = object.get_diagnostic(property);
        let priority = effective.priority();
        let value = || Some(effective.value().clone());
        if property.is_direct() {
            let mut entry = ValueContributor::new(Kind::Direct, State::Effective, priority);
            entry.value = value();
            entry.source = Some(object.to_ref());
            return Self {
                effective,
                contributors: vec![entry],
            };
        }

        let store = object.values();
        let selected = store.get_effective_value(property);
        let overridden = effective.is_overridden_current_value();
        let animated = priority == Priority::Animation;
        let selected_entry = selected.as_ref().and_then(|value| value.value_entry());
        let base_entry = selected
            .as_ref()
            .filter(|_| animated)
            .and_then(|value| value.base_value_entry());
        let base_priority = selected
            .as_ref()
            .map_or(Priority::Unset, |value| value.base_priority());
        let base_value = || selected.as_ref().and_then(|value| value.boxed_base_value());
        let mut contributors = Vec::new();
        if overridden {
            let mut entry = ValueContributor::new(Kind::CurrentValue, State::Effective, priority);
            entry.value = value();
            contributors.push(entry);
        }

        let local_binding = object.has_local_value_binding(property);
        if priority == Priority::LocalValue
            || (animated && base_priority == Priority::LocalValue)
            || local_binding
        {
            let state = if priority == Priority::LocalValue && !overridden {
                State::Effective
            } else if animated && base_priority == Priority::LocalValue {
                State::Base
            } else if overridden && priority == Priority::LocalValue {
                State::Overridden
            } else {
                State::Unset
            };
            let mut entry = ValueContributor::new(Kind::Local, state, Priority::LocalValue);
            entry.value = match state {
                State::Effective => value(),
                State::Base => base_value(),
                _ => None,
            };
            entry.source = Some(object.to_ref());
            entry.binding = local_binding;
            contributors.push(entry);
        }

        // Reverse store order visits entries of equal priority in precedence order.
        let mut found_selected = false;
        let mut found_base = false;
        for frame in store.frames().into_iter().rev() {
            let Some(entry) = frame.base().find_entry(property) else {
                continue;
            };
            let frame_priority = frame.base().priority();
            let style = frame.as_any().downcast_ref::<StyleInstance>();
            let active = style
                .is_none_or(|style| !style.has_activator() || IStyleInstance::is_active(style));
            let style_source = style.and_then(IStyleInstance::source);
            let kind = if frame_priority == Priority::Animation {
                Kind::Animation
            } else if let Some(source) = &style_source {
                if source.is::<ControlTheme>() {
                    Kind::Theme
                } else {
                    Kind::Style
                }
            } else {
                kind_for_priority(frame_priority)
            };
            let is_selected = !found_selected
                && frame_priority == priority
                && same_entry(&selected_entry, &entry);
            let is_base =
                !found_base && frame_priority == base_priority && same_entry(&base_entry, &entry);
            found_selected |= is_selected;
            found_base |= is_base;
            let state = if is_selected && !overridden {
                State::Effective
            } else if is_base {
                State::Base
            } else if !active && !is_selected {
                State::Inactive
            } else {
                State::Overridden
            };
            let (literal, binding) = style_source.as_ref().map_or((None, false), |source| {
                let setter = source.setters().snapshot();
                let value = setter
                    .iter()
                    .filter_map(|candidate| candidate.as_any()?.downcast_ref::<Setter>())
                    .find(|setter| {
                        setter
                            .property()
                            .is_some_and(|candidate| candidate.id() == property.id())
                    })
                    .and_then(Setter::value);
                let binding = matches!(
                    value,
                    Some(SetterValue::Binding(_) | SetterValue::BindingBase(_))
                );
                let literal = match value {
                    Some(SetterValue::Value(value)) => Some(value),
                    _ => None,
                };
                (literal, binding)
            });
            let mut contributor = ValueContributor::new(kind, state, frame_priority);
            contributor.value = match state {
                State::Effective => value(),
                State::Base => base_value(),
                _ => literal,
            };
            contributor.binding = binding;
            contributor.source = style_source.map(Ref::upcast);
            contributors.push(contributor);
        }

        let inherited_base = animated && base_priority >= Priority::Inherited;
        let inherited = inherited_source(object, property);
        let has_inherited = inherited.is_some();
        if let Some((owner, inherited_value)) = inherited {
            let state = if priority == Priority::Inherited && !overridden {
                State::Effective
            } else if inherited_base {
                State::Base
            } else {
                State::Overridden
            };
            let mut entry = ValueContributor::new(Kind::Inherited, state, Priority::Inherited);
            entry.value = Some(inherited_value);
            entry.source = Some(owner);
            contributors.push(entry);
        }

        let state = if priority == Priority::Unset && !overridden {
            State::Effective
        } else if inherited_base && !has_inherited {
            State::Base
        } else {
            State::Overridden
        };
        let mut default = ValueContributor::new(Kind::Default, state, Priority::Unset);
        default.value = if state == State::Effective {
            value()
        } else {
            Some(property.routes().route_get_default_value(object.get_type()))
        };
        contributors.push(default);

        if !contributors
            .iter()
            .any(|entry| entry.state == State::Effective)
        {
            let mut entry =
                ValueContributor::new(kind_for_priority(priority), State::Effective, priority);
            entry.value = value();
            contributors.push(entry);
        }
        contributors.sort_by_key(|entry| (entry.state != State::Effective, entry.priority));
        Self {
            effective,
            contributors,
        }
    }
}

fn same_entry(selected: &Option<Rc<dyn IValueEntry>>, entry: &Rc<dyn IValueEntry>) -> bool {
    selected
        .as_ref()
        .is_some_and(|selected| entry_ptr_eq(selected, entry))
}

fn kind_for_priority(priority: BindingPriority) -> ValueContributorKind {
    use BindingPriority::*;
    match priority {
        Animation => ValueContributorKind::Animation,
        LocalValue => ValueContributorKind::Local,
        StyleTrigger | Style => ValueContributorKind::Style,
        Template => ValueContributorKind::Template,
        Inherited => ValueContributorKind::Inherited,
        Unset => ValueContributorKind::Default,
    }
}

fn inherited_source(
    object: &FerroObject,
    property: &FerroProperty,
) -> Option<(Ref<FerroObject>, BoxedValue)> {
    if !property.inherits() {
        return None;
    }
    let mut parent = object.inheritance_parent();
    while let Some(owner) = parent {
        if let Some(value) = owner.values().get_effective_value(property) {
            return Some((owner, value.boxed_value()));
        }
        parent = owner.inheritance_parent();
    }
    None
}

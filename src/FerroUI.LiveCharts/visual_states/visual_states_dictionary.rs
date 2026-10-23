use super::{DrawnPropertySetter, IStateBehavior};
use crate::motion::{Animatable, PropertyError};
use std::{collections::BTreeMap, fmt, rc::Rc};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StateOrigin {
    Theme,
    #[default]
    User,
}

/// Ordered, named setters and shared behaviors for one state. Replacing a
/// setter preserves its insertion position, as in the source dictionary.
#[derive(Clone, Default)]
pub struct DrawnPropertiesDictionary {
    setters: Vec<DrawnPropertySetter>,
    pub origin: StateOrigin,
    pub behaviors: Vec<Rc<dyn IStateBehavior>>,
}
impl DrawnPropertiesDictionary {
    pub fn new(setters: impl IntoIterator<Item = DrawnPropertySetter>) -> Self {
        let mut state = Self::default();
        for setter in setters {
            state.insert(setter);
        }
        state
    }
    pub fn with_origin(mut self, origin: StateOrigin) -> Self {
        self.origin = origin;
        self
    }
    pub fn with_behavior(mut self, behavior: Rc<dyn IStateBehavior>) -> Self {
        self.behaviors.push(behavior);
        self
    }
    pub fn insert(&mut self, setter: DrawnPropertySetter) -> Option<DrawnPropertySetter> {
        if let Some(previous) = self
            .setters
            .iter_mut()
            .find(|entry| entry.property_name == setter.property_name)
        {
            return Some(std::mem::replace(previous, setter));
        }
        self.setters.push(setter);
        None
    }
    pub fn get(&self, name: &str) -> Option<&DrawnPropertySetter> {
        self.setters
            .iter()
            .find(|setter| setter.property_name == name)
    }
    pub fn setters(&self) -> impl Iterator<Item = &DrawnPropertySetter> {
        self.setters.iter()
    }
    pub fn len(&self) -> usize {
        self.setters.len()
    }
    pub fn is_empty(&self) -> bool {
        self.setters.is_empty()
    }
}
impl fmt::Debug for DrawnPropertiesDictionary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DrawnPropertiesDictionary")
            .field("setters", &self.setters)
            .field("origin", &self.origin)
            .field("behaviors", &self.behaviors.len())
            .finish()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StateError {
    Missing(String),
    MissingOriginal { state: String, property: String },
    Property(PropertyError),
}
impl From<PropertyError> for StateError {
    fn from(value: PropertyError) -> Self {
        Self::Property(value)
    }
}
impl fmt::Display for StateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing(name) => write!(f, "Visual state {name} is not defined."),
            Self::MissingOriginal { state, property } => write!(
                f,
                "Visual state {state} has no original value for property {property}."
            ),
            Self::Property(error) => fmt::Display::fmt(error, f),
        }
    }
}
impl std::error::Error for StateError {}

/// Reusable visual-state definitions. A user's definition takes precedence
/// over a theme definition, regardless of registration order.
#[derive(Clone, Debug, Default)]
pub struct VisualStatesDictionary {
    states: BTreeMap<String, DrawnPropertiesDictionary>,
}
impl VisualStatesDictionary {
    pub fn new() -> Self {
        Self::default()
    }
    /// Adds or replaces a state using source `HasState` precedence. Returns
    /// false only when a theme tries to replace a user's definition.
    pub fn insert(&mut self, name: impl Into<String>, state: DrawnPropertiesDictionary) -> bool {
        let name = name.into();
        if self.states.get(&name).is_some_and(|previous| {
            previous.origin == StateOrigin::User && state.origin == StateOrigin::Theme
        }) {
            return false;
        }
        self.states.insert(name, state);
        true
    }
    pub fn get(&self, name: &str) -> Option<&DrawnPropertiesDictionary> {
        self.states.get(name)
    }
    pub fn iter(&self) -> impl Iterator<Item = (&str, &DrawnPropertiesDictionary)> {
        self.states
            .iter()
            .map(|(name, state)| (name.as_str(), state))
    }
    pub fn len(&self) -> usize {
        self.states.len()
    }
    pub fn is_empty(&self) -> bool {
        self.states.is_empty()
    }
    pub fn contains_key(&self, name: &str) -> bool {
        self.states.contains_key(name)
    }

    /// Applies setters every time and behaviors only on first activation.
    /// Unregistered properties are ignored. Known property types are checked
    /// before changes are made, so a bad setter cannot leave a half-applied state.
    pub fn set_state(
        &self,
        name: &str,
        target: &mut Animatable,
        now_ms: f64,
    ) -> Result<(), StateError> {
        let state = self
            .states
            .get(name)
            .ok_or_else(|| StateError::Missing(name.to_owned()))?;
        let values = state
            .setters()
            .filter(|setter| {
                target
                    .get_property_definition(&setter.property_name)
                    .is_some()
            })
            .map(|setter| {
                target
                    .coerce_value(&setter.property_name, setter.value.clone())
                    .map(|value| (setter.property_name.clone(), value))
            })
            .collect::<Result<Vec<_>, _>>()?;
        for (property, value) in values {
            if !target.states.original_values.contains_key(&property) {
                let original = target.target_value(&property)?;
                target
                    .states
                    .original_values
                    .insert(property.clone(), original);
            }
            target.set_value(&property, value, now_ms)?;
        }
        if target.is_state_active(name) {
            return Ok(());
        }
        target.states.active.push(name.to_owned());
        for behavior in &state.behaviors {
            behavior.on_state_applied(target, now_ms);
        }
        Ok(())
    }

    /// Removes a state, restoring each property from the most recently activated
    /// remaining state, or from its original target when no state still sets it.
    pub fn clear_state(
        &self,
        name: &str,
        target: &mut Animatable,
        now_ms: f64,
    ) -> Result<(), StateError> {
        if !target.is_state_active(name) {
            return Ok(());
        }
        let state = self
            .states
            .get(name)
            .ok_or_else(|| StateError::Missing(name.to_owned()))?;
        let mut values = Vec::new();
        for setter in state.setters() {
            let property = &setter.property_name;
            if target.get_property_definition(property).is_none() {
                continue;
            }
            let mut replacement = None;
            for active in target
                .states
                .active
                .iter()
                .rev()
                .filter(|active| active.as_str() != name)
            {
                let other = self
                    .states
                    .get(active)
                    .ok_or_else(|| StateError::Missing(active.clone()))?;
                if let Some(setter) = other.get(property) {
                    replacement = Some(target.coerce_value(property, setter.value.clone())?);
                    break;
                }
            }
            let restore_original = replacement.is_none();
            let value = replacement
                .or_else(|| target.states.original_values.get(property).cloned())
                .ok_or_else(|| StateError::MissingOriginal {
                    state: name.to_owned(),
                    property: property.clone(),
                })?;
            values.push((property.clone(), value, restore_original));
        }
        for (property, value, restore_original) in values {
            target.set_value(&property, value, now_ms)?;
            if restore_original {
                target.states.original_values.remove(&property);
            }
        }
        target.states.active.retain(|active| active != name);
        for behavior in &state.behaviors {
            behavior.on_state_removed(target, now_ms);
        }
        Ok(())
    }

    /// Restores original values once and reverses each active behavior, in
    /// activation order. No global timer or target-identity table is involved.
    pub fn clear_states(&self, target: &mut Animatable, now_ms: f64) -> Result<(), StateError> {
        // Validate all referenced definitions before restoring any values.
        let active = target.states.active.clone();
        for name in &active {
            if !self.states.contains_key(name) {
                return Err(StateError::Missing(name.clone()));
            }
        }
        for name in active {
            let state = &self.states[&name];
            for setter in state.setters() {
                let property = &setter.property_name;
                if target.get_property_definition(property).is_none() {
                    continue;
                }
                if let Some(original) = target.states.original_values.get(property).cloned() {
                    target.set_value(property, original, now_ms)?;
                    target.states.original_values.remove(property);
                }
            }
            for behavior in &state.behaviors {
                behavior.on_state_removed(target, now_ms);
            }
        }
        target.states.active.clear();
        target.states.original_values.clear();
        Ok(())
    }
}

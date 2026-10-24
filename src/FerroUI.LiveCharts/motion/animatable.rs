//! Typed motion and state-property registration without global clocks or reflection.

use super::{Animation, Interpolate, MotionProperty};
use std::{
    any::{Any, TypeId, type_name},
    collections::BTreeMap,
    fmt,
    marker::PhantomData,
    rc::Rc,
};

/// A cloneable erased value. Typed setters use the registered type's `Clone`
/// semantics; reference values intentionally preserve their object identity.
#[derive(Clone)]
pub struct PropertyValue {
    value: Rc<dyn Any>,
    type_name: &'static str,
}
impl PropertyValue {
    pub fn new<T: 'static>(value: T) -> Self {
        Self {
            value: Rc::new(value),
            type_name: type_name::<T>(),
        }
    }
    pub fn downcast_ref<T: 'static>(&self) -> Option<&T> {
        self.value.as_ref().downcast_ref()
    }
    pub fn value_type(&self) -> TypeId {
        self.value.as_ref().type_id()
    }
    pub fn type_name(&self) -> &'static str {
        self.type_name
    }
}
impl fmt::Debug for PropertyValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PropertyValue")
            .field("type", &self.type_name)
            .finish_non_exhaustive()
    }
}
impl From<&str> for PropertyValue {
    fn from(value: &str) -> Self {
        Self::new(value.to_owned())
    }
}

/// A statically typed handle into an animatable's named property collection.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PropertyKey<T> {
    name: String,
    marker: PhantomData<fn() -> T>,
}
impl<T> PropertyKey<T> {
    pub fn name(&self) -> &str {
        &self.name
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PropertyDefinition {
    pub name: String,
    pub value_type: TypeId,
    pub value_type_name: &'static str,
    pub has_motion: bool,
}
impl PropertyDefinition {
    pub fn get_value(
        &self,
        target: &mut Animatable,
        now_ms: f64,
    ) -> Result<PropertyValue, PropertyError> {
        target.require_type(&self.name, self.value_type, self.value_type_name)?;
        target.get_value(&self.name, now_ms)
    }
    pub fn set_value(
        &self,
        target: &mut Animatable,
        value: PropertyValue,
        now_ms: f64,
    ) -> Result<(), PropertyError> {
        target.require_type(&self.name, self.value_type, self.value_type_name)?;
        target.set_value(&self.name, value, now_ms)
    }
    pub fn get_motion(&self, target: &Animatable) -> Result<Option<MotionInfo>, PropertyError> {
        target.require_type(&self.name, self.value_type, self.value_type_name)?;
        target.motion_info(&self.name)
    }
}
impl fmt::Display for PropertyDefinition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)
    }
}

#[derive(Clone, Debug)]
pub struct MotionInfo {
    pub from_value: PropertyValue,
    pub to_value: PropertyValue,
    pub is_completed: bool,
    pub animation: Option<Animation>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PropertyError {
    Duplicate(String),
    Missing(String),
    NotMotion(String),
    TypeMismatch {
        property: String,
        expected: &'static str,
        actual: &'static str,
    },
    MissingParser {
        property: String,
        value_type: &'static str,
    },
    ParseFailed {
        property: String,
        value: String,
    },
}
impl fmt::Display for PropertyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Duplicate(name) => write!(f, "Property {name} is already registered."),
            Self::Missing(name) => write!(f, "Property {name} is not registered."),
            Self::NotMotion(name) => write!(f, "Property {name} has no motion."),
            Self::TypeMismatch {
                property,
                expected,
                actual,
            } => write!(
                f,
                "Property {property} requires {expected}, received {actual}."
            ),
            Self::MissingParser {
                property,
                value_type,
            } => write!(
                f,
                "Property {property} has no string parser for {value_type}."
            ),
            Self::ParseFailed { property, value } => {
                write!(f, "Cannot parse {value:?} for property {property}.")
            }
        }
    }
}
impl std::error::Error for PropertyError {}

struct Sample {
    value: PropertyValue,
    completed_now: bool,
    is_completed: bool,
}

trait RegisteredValue {
    fn as_any(&self) -> &dyn Any;
    fn sample(&mut self, now_ms: f64) -> Sample;
    fn target(&self) -> PropertyValue;
    fn set(&mut self, from: &PropertyValue, value: &PropertyValue, now_ms: f64);
    fn set_animation(&mut self, animation: Option<Animation>);
    fn finish(&mut self);
    fn motion_info(&self) -> Option<MotionInfo>;
    fn copy_motion(&mut self, source: &dyn RegisteredValue) -> bool;
    fn clone_box(&self) -> Box<dyn RegisteredValue>;
    fn rebase(&mut self, value: &PropertyValue);
}

#[derive(Clone)]
struct MotionValue<T: Interpolate + 'static>(MotionProperty<T>);
impl<T: Interpolate + 'static> RegisteredValue for MotionValue<T> {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn sample(&mut self, now_ms: f64) -> Sample {
        let was_completed = self.0.is_completed();
        let value = PropertyValue::new(self.0.get_movement(now_ms));
        let is_completed = self.0.is_completed();
        Sample {
            value,
            completed_now: !was_completed && is_completed,
            is_completed,
        }
    }
    fn target(&self) -> PropertyValue {
        PropertyValue::new(self.0.to_value.clone())
    }
    fn set(&mut self, from: &PropertyValue, value: &PropertyValue, now_ms: f64) {
        self.0.set_movement_from(
            from.downcast_ref::<T>()
                .expect("registered value type was validated")
                .clone(),
            value
                .downcast_ref::<T>()
                .expect("registered value type was validated")
                .clone(),
            now_ms,
        );
    }
    fn set_animation(&mut self, animation: Option<Animation>) {
        self.0.animation = animation;
    }
    fn finish(&mut self) {
        self.0.finish();
    }
    fn motion_info(&self) -> Option<MotionInfo> {
        Some(MotionInfo {
            from_value: PropertyValue::new(self.0.from_value.clone()),
            to_value: self.target(),
            is_completed: self.0.is_completed(),
            animation: self.0.animation.clone(),
        })
    }
    fn copy_motion(&mut self, source: &dyn RegisteredValue) -> bool {
        let Some(source) = source.as_any().downcast_ref::<Self>() else {
            return false;
        };
        self.0.copy_from(&source.0);
        true
    }
    fn clone_box(&self) -> Box<dyn RegisteredValue> {
        Box::new(self.clone())
    }
    fn rebase(&mut self, value: &PropertyValue) {
        self.0 = MotionProperty::new(
            value
                .downcast_ref::<T>()
                .expect("registered value type was validated")
                .clone(),
        );
    }
}

#[derive(Clone)]
struct StateValue<T: Clone + 'static>(T);
impl<T: Clone + 'static> RegisteredValue for StateValue<T> {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn sample(&mut self, _now_ms: f64) -> Sample {
        Sample {
            value: self.target(),
            completed_now: false,
            is_completed: true,
        }
    }
    fn target(&self) -> PropertyValue {
        PropertyValue::new(self.0.clone())
    }
    fn set(&mut self, _from: &PropertyValue, value: &PropertyValue, _now_ms: f64) {
        self.0 = value
            .downcast_ref::<T>()
            .expect("registered value type was validated")
            .clone();
    }
    fn set_animation(&mut self, _animation: Option<Animation>) {}
    fn finish(&mut self) {}
    fn motion_info(&self) -> Option<MotionInfo> {
        None
    }
    fn copy_motion(&mut self, _source: &dyn RegisteredValue) -> bool {
        false
    }
    fn clone_box(&self) -> Box<dyn RegisteredValue> {
        Box::new(self.clone())
    }
    fn rebase(&mut self, value: &PropertyValue) {
        self.0 = value
            .downcast_ref::<T>()
            .expect("registered value type was validated")
            .clone();
    }
}

type CompletedHandler = Rc<dyn Fn(&mut Animatable, f64)>;
type Parser = Rc<dyn Fn(&str) -> Option<PropertyValue>>;
struct PropertySlot {
    definition: PropertyDefinition,
    value: Box<dyn RegisteredValue>,
    completed: Vec<(u64, CompletedHandler)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompletionToken {
    property: String,
    id: u64,
}

#[derive(Default)]
pub(crate) struct StatesTracker {
    pub active: Vec<String>,
    pub original_values: BTreeMap<String, PropertyValue>,
}

/// A named collection of typed motion values and motionless state properties.
///
/// Call `begin_frame` and read every drawn property, or call `sample_frame`, to
/// recompute `is_valid`. A getter invalidates the frame while its motion remains
/// active, exactly as source drawing getters do. Each frame uses one host time.
pub struct Animatable {
    pub is_valid: bool,
    pub remove_on_completed: bool,
    properties: BTreeMap<String, PropertySlot>,
    parsers: BTreeMap<TypeId, Parser>,
    next_handler: u64,
    pub(crate) states: StatesTracker,
}
impl Default for Animatable {
    fn default() -> Self {
        let mut result = Self {
            is_valid: true,
            remove_on_completed: false,
            properties: BTreeMap::new(),
            parsers: BTreeMap::new(),
            next_handler: 0,
            states: StatesTracker::default(),
        };
        // Source numeric string parsers fall back to zero when parsing fails.
        result.register_parser(|s| Some(s.trim().parse::<f64>().unwrap_or_default()));
        result.register_parser(|s| Some(s.trim().parse::<f32>().unwrap_or_default()));
        result.register_parser(|s| Some(s.trim().parse::<i32>().unwrap_or_default()));
        result
    }
}
impl fmt::Debug for Animatable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Animatable")
            .field("is_valid", &self.is_valid)
            .field("remove_on_completed", &self.remove_on_completed)
            .field("properties", &self.properties.keys().collect::<Vec<_>>())
            .finish()
    }
}

impl Animatable {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_motion<T: Interpolate + 'static>(
        &mut self,
        name: impl Into<String>,
        default: T,
    ) -> Result<PropertyKey<T>, PropertyError> {
        self.register(
            name.into(),
            true,
            Box::new(MotionValue(MotionProperty::new(default))),
        )
    }
    pub fn register_state<T: Clone + 'static>(
        &mut self,
        name: impl Into<String>,
        default: T,
    ) -> Result<PropertyKey<T>, PropertyError> {
        self.register(name.into(), false, Box::new(StateValue(default)))
    }
    fn register<T: 'static>(
        &mut self,
        name: String,
        has_motion: bool,
        value: Box<dyn RegisteredValue>,
    ) -> Result<PropertyKey<T>, PropertyError> {
        if self.properties.contains_key(&name) {
            return Err(PropertyError::Duplicate(name));
        }
        self.properties.insert(
            name.clone(),
            PropertySlot {
                definition: PropertyDefinition {
                    name: name.clone(),
                    value_type: TypeId::of::<T>(),
                    value_type_name: type_name::<T>(),
                    has_motion,
                },
                value,
                completed: Vec::new(),
            },
        );
        Ok(PropertyKey {
            name,
            marker: PhantomData,
        })
    }
    pub fn get_property_definition(&self, name: &str) -> Option<&PropertyDefinition> {
        self.properties.get(name).map(|slot| &slot.definition)
    }
    pub fn property_definitions(&self) -> impl Iterator<Item = &PropertyDefinition> {
        self.properties.values().map(|slot| &slot.definition)
    }
    fn require_type(
        &self,
        name: &str,
        value_type: TypeId,
        value_type_name: &'static str,
    ) -> Result<(), PropertyError> {
        let property = self
            .get_property_definition(name)
            .ok_or_else(|| PropertyError::Missing(name.to_owned()))?;
        if property.value_type != value_type {
            return Err(PropertyError::TypeMismatch {
                property: name.to_owned(),
                expected: property.value_type_name,
                actual: value_type_name,
            });
        }
        Ok(())
    }
    pub fn get<T: Clone + 'static>(
        &mut self,
        property: &PropertyKey<T>,
        now_ms: f64,
    ) -> Result<T, PropertyError> {
        self.require_type(&property.name, TypeId::of::<T>(), type_name::<T>())?;
        Ok(self
            .get_value(&property.name, now_ms)?
            .downcast_ref::<T>()
            .expect("registered value type was validated")
            .clone())
    }
    pub fn set<T: Clone + 'static>(
        &mut self,
        property: &PropertyKey<T>,
        value: T,
        now_ms: f64,
    ) -> Result<(), PropertyError> {
        self.require_type(&property.name, TypeId::of::<T>(), type_name::<T>())?;
        self.set_value(&property.name, PropertyValue::new(value), now_ms)
    }
    pub fn get_value(&mut self, name: &str, now_ms: f64) -> Result<PropertyValue, PropertyError> {
        let slot = self
            .properties
            .get_mut(name)
            .ok_or_else(|| PropertyError::Missing(name.to_owned()))?;
        let sample = slot.value.sample(now_ms);
        let callbacks = if sample.completed_now {
            slot.completed
                .iter()
                .map(|(_, f)| f.clone())
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        if !sample.is_completed {
            self.is_valid = false;
        }
        for callback in callbacks {
            callback(self, now_ms);
        }
        Ok(sample.value)
    }
    /// Gets the destination for motion values and the current value otherwise.
    /// Visual states use this to restore the original intended value mid-flight.
    pub fn target_value(&self, name: &str) -> Result<PropertyValue, PropertyError> {
        self.properties
            .get(name)
            .map(|slot| slot.value.target())
            .ok_or_else(|| PropertyError::Missing(name.to_owned()))
    }
    pub fn register_parser<T: 'static>(&mut self, parser: impl Fn(&str) -> Option<T> + 'static) {
        self.parsers.insert(
            TypeId::of::<T>(),
            Rc::new(move |s| parser(s).map(PropertyValue::new)),
        );
    }
    pub fn coerce_value(
        &self,
        name: &str,
        value: PropertyValue,
    ) -> Result<PropertyValue, PropertyError> {
        let definition = self
            .get_property_definition(name)
            .ok_or_else(|| PropertyError::Missing(name.to_owned()))?;
        if value.value_type() == definition.value_type {
            return Ok(value);
        }
        if let Some(text) = value.downcast_ref::<String>() {
            let parser = self.parsers.get(&definition.value_type).ok_or_else(|| {
                PropertyError::MissingParser {
                    property: name.to_owned(),
                    value_type: definition.value_type_name,
                }
            })?;
            return parser(text).ok_or_else(|| PropertyError::ParseFailed {
                property: name.to_owned(),
                value: text.clone(),
            });
        }
        Err(PropertyError::TypeMismatch {
            property: name.to_owned(),
            expected: definition.value_type_name,
            actual: value.type_name(),
        })
    }
    pub fn set_value(
        &mut self,
        name: &str,
        value: PropertyValue,
        now_ms: f64,
    ) -> Result<(), PropertyError> {
        let value = self.coerce_value(name, value)?;
        let from = self.get_value(name, now_ms)?;
        // A completion handler may compose a different prototype. Keep the
        // erased setter safe even if that changes this property's definition.
        self.require_type(name, value.value_type(), value.type_name())?;
        self.properties
            .get_mut(name)
            .expect("registered property was validated")
            .value
            .set(&from, &value, now_ms);
        self.is_valid = false;
        Ok(())
    }
    fn selected(&self, names: &[&str]) -> Result<Vec<String>, PropertyError> {
        if names.is_empty() {
            return Ok(self.properties.keys().cloned().collect());
        }
        for name in names {
            if !self.properties.contains_key(*name) {
                return Err(PropertyError::Missing((*name).to_owned()));
            }
        }
        Ok(names.iter().map(|name| (*name).to_owned()).collect())
    }
    pub fn set_transition(
        &mut self,
        animation: Option<Animation>,
        properties: &[&str],
    ) -> Result<(), PropertyError> {
        for name in self.selected(properties)? {
            self.properties
                .get_mut(&name)
                .unwrap()
                .value
                .set_animation(animation.clone());
        }
        Ok(())
    }
    pub fn remove_transition(&mut self, properties: &[&str]) -> Result<(), PropertyError> {
        self.set_transition(None, properties)
    }
    pub fn complete_transition(&mut self, properties: &[&str]) -> Result<(), PropertyError> {
        for name in self.selected(properties)? {
            self.properties.get_mut(&name).unwrap().value.finish();
        }
        Ok(())
    }
    pub fn motion_info(&self, name: &str) -> Result<Option<MotionInfo>, PropertyError> {
        self.properties
            .get(name)
            .map(|slot| slot.value.motion_info())
            .ok_or_else(|| PropertyError::Missing(name.to_owned()))
    }
    pub fn motion<T: Interpolate + 'static>(
        &self,
        property: &PropertyKey<T>,
    ) -> Result<Option<&MotionProperty<T>>, PropertyError> {
        self.require_type(&property.name, TypeId::of::<T>(), type_name::<T>())?;
        Ok(self.properties[&property.name]
            .value
            .as_any()
            .downcast_ref::<MotionValue<T>>()
            .map(|value| &value.0))
    }
    pub fn copy_motion_from(
        &mut self,
        name: &str,
        source: &Animatable,
        source_name: &str,
    ) -> Result<(), PropertyError> {
        let source = source
            .properties
            .get(source_name)
            .ok_or_else(|| PropertyError::Missing(source_name.to_owned()))?;
        self.require_type(
            name,
            source.definition.value_type,
            source.definition.value_type_name,
        )?;
        let destination = self.properties.get_mut(name).unwrap();
        if !destination.value.copy_motion(source.value.as_ref()) {
            return Err(PropertyError::NotMotion(name.to_owned()));
        }
        self.is_valid = false;
        Ok(())
    }
    /// Imports prototype values and definitions; later imports replace duplicate
    /// names, matching source base/derived definition precedence. Callbacks are
    /// instance-local and are never copied into the destination.
    pub fn import_properties(&mut self, prototype: &Animatable) {
        for (name, slot) in &prototype.properties {
            self.properties.insert(
                name.clone(),
                PropertySlot {
                    definition: slot.definition.clone(),
                    value: slot.value.clone_box(),
                    completed: Vec::new(),
                },
            );
        }
        self.parsers.extend(
            prototype
                .parsers
                .iter()
                .map(|(key, parser)| (*key, parser.clone())),
        );
        self.is_valid = false;
    }
    /// Clones the visible values into independent defaults/from/to baselines.
    /// Sampling happens once per property; no animation, callback or state
    /// tracker is carried into the resulting instance.
    pub fn clone_snapshot(&mut self, now_ms: f64) -> Result<Self, PropertyError> {
        let mut result = Self::new();
        result.import_properties(self);
        self.begin_frame();
        for name in self.properties.keys().cloned().collect::<Vec<_>>() {
            let value = self.get_value(&name, now_ms)?;
            result.require_type(&name, value.value_type(), value.type_name())?;
            result
                .properties
                .get_mut(&name)
                .unwrap()
                .value
                .rebase(&value);
        }
        result.is_valid = true;
        Ok(result)
    }
    pub fn on_completed(
        &mut self,
        name: &str,
        callback: impl Fn(&mut Animatable, f64) + 'static,
    ) -> Result<CompletionToken, PropertyError> {
        let slot = self
            .properties
            .get_mut(name)
            .ok_or_else(|| PropertyError::Missing(name.to_owned()))?;
        if !slot.definition.has_motion {
            return Err(PropertyError::NotMotion(name.to_owned()));
        }
        self.next_handler += 1;
        slot.completed.push((self.next_handler, Rc::new(callback)));
        Ok(CompletionToken {
            property: name.to_owned(),
            id: self.next_handler,
        })
    }
    pub fn remove_completed_handler(&mut self, token: CompletionToken) -> bool {
        let Some(slot) = self.properties.get_mut(&token.property) else {
            return false;
        };
        let old = slot.completed.len();
        slot.completed.retain(|(id, _)| *id != token.id);
        old != slot.completed.len()
    }
    pub fn begin_frame(&mut self) {
        self.is_valid = true;
    }
    pub fn sample_frame(&mut self, now_ms: f64) -> bool {
        self.begin_frame();
        for name in self.properties.keys().cloned().collect::<Vec<_>>() {
            // Completion handlers may register other properties. Existing names
            // remain registered, and additions are sampled on the next frame.
            let _ = self.get_value(&name, now_ms);
        }
        self.is_valid
    }
    pub fn can_remove(&self) -> bool {
        self.remove_on_completed && self.is_valid
    }
    pub fn is_state_active(&self, name: &str) -> bool {
        self.states.active.iter().any(|state| state == name)
    }
    /// Active states in activation order; setting an active state keeps its place.
    pub fn active_states(&self) -> impl Iterator<Item = &str> {
        self.states.active.iter().map(String::as_str)
    }
}

#[cfg(test)]
mod tests {
    use super::super::{ByReferenceMotionProperty, Discrete, Repeat};
    use super::*;
    use crate::easing_functions::EasingFunctions;
    use ferroui_base::media::Color;
    use std::cell::Cell;

    #[test]
    fn source_infinite_animation_tracks_all_registered_properties_and_validity() {
        let mut target = Animatable::new();
        let properties = ["X", "Y", "Width", "Height"]
            .map(|name| target.register_motion(name, 0.0_f64).unwrap());
        target
            .set_transition(
                Some(Animation::new(1000.0, EasingFunctions::lineal).with_repeat(Repeat::Forever)),
                &[],
            )
            .unwrap();
        for property in &properties {
            target.set(property, 50.0, 50.0).unwrap();
        }
        target.complete_transition(&[]).unwrap();
        for property in &properties {
            assert_eq!(target.get(property, 50.0).unwrap(), 50.0);
            target.set(property, 100.0, 50.0).unwrap();
        }
        for (time, expected) in [
            (50.0, 50.0),
            (550.0, 75.0),
            (1050.0, 100.0),
            (1550.0, 75.0),
            (2050.0, 100.0),
        ] {
            target.begin_frame();
            for property in &properties {
                assert_eq!(target.get(property, time).unwrap(), expected);
            }
            assert!(!target.is_valid);
        }
    }

    #[test]
    fn source_animation_complete_registry_boundary_and_forced_completion() {
        let mut target = Animatable::new();
        let properties = ["X", "Y", "Width", "Height"]
            .map(|name| target.register_motion(name, 0.0_f64).unwrap());
        let y = &properties[1];
        target
            .set_transition(Some(Animation::new(1000.0, EasingFunctions::lineal)), &[])
            .unwrap();
        for property in &properties {
            target.set(property, 0.0, 0.0).unwrap();
        }
        target.complete_transition(&[]).unwrap();
        assert!(target.sample_frame(0.0));
        target.set(y, 100.0, 0.0).unwrap();
        for time in [500.0, 1000.0] {
            assert!(!target.sample_frame(time));
            assert!(
                !target
                    .get_property_definition("Y")
                    .unwrap()
                    .get_motion(&target)
                    .unwrap()
                    .unwrap()
                    .is_completed
            );
        }
        assert!(target.sample_frame(1500.0));
        assert!(target.motion(y).unwrap().unwrap().is_completed());
        target.remove_on_completed = true;
        assert!(target.can_remove());
    }

    #[test]
    fn source_not_finished_animation_retargets_registered_property() {
        let mut target = Animatable::new();
        let x = target.register_motion("X", 0.0_f64).unwrap();
        target
            .set_transition(
                Some(Animation::new(100.0, EasingFunctions::lineal).with_repeat(Repeat::Forever)),
                &["X"],
            )
            .unwrap();
        target.set(&x, 0.0, 0.0).unwrap();
        target.complete_transition(&["X"]).unwrap();
        target.set(&x, 100.0, 0.0).unwrap();
        for i in 0..=10 {
            let time = f64::from(i * 10);
            target.begin_frame();
            if i == 8 {
                target.set(&x, 200.0, time).unwrap();
            }
            let expected = if i < 8 {
                time
            } else {
                80.0 + (time - 80.0) * 1.2
            };
            assert!((target.get(&x, time).unwrap() - expected).abs() < 1e-9);
            assert!(!target.is_valid);
        }
    }

    #[test]
    fn source_paint_reference_is_motionless_and_never_mutates_previous_value() {
        let red = Color::from_rgb(255, 0, 0);
        let old = Rc::new(Cell::new(red));
        let new = Rc::new(Cell::new(Color::from_rgb(0, 0, 255)));
        let mut target = Animatable::new();
        let fill = target.register_state("Fill", Some(old.clone())).unwrap();
        assert!(Rc::ptr_eq(&target.get(&fill, 0.0).unwrap().unwrap(), &old));
        target
            .set_transition(Some(Animation::new(1000.0, EasingFunctions::lineal)), &[])
            .unwrap();
        target.set(&fill, Some(new.clone()), 0.0).unwrap();
        assert!(!target.get_property_definition("Fill").unwrap().has_motion);
        assert!(target.motion_info("Fill").unwrap().is_none());
        for time in (0..=1000).step_by(100) {
            target.begin_frame();
            let value = target.get(&fill, f64::from(time)).unwrap().unwrap();
            assert!(Rc::ptr_eq(&value, &new));
            assert!(target.is_valid);
        }
        assert_eq!(old.get(), red);
    }

    #[test]
    fn discrete_motion_completes_and_calls_completion_once() {
        let mut target = Animatable::new();
        let value = target
            .register_motion("Reference", Discrete(None::<Rc<()>>))
            .unwrap();
        target
            .set_transition(Some(Animation::new(1000.0, EasingFunctions::lineal)), &[])
            .unwrap();
        let count = Rc::new(Cell::new(0));
        let counter = count.clone();
        let token = target
            .on_completed("Reference", move |_, _| counter.set(counter.get() + 1))
            .unwrap();
        let expected = Rc::new(());
        let mut reference = ByReferenceMotionProperty::new(None::<Rc<()>>);
        reference.set_animation(Some(Animation::new(1000.0, EasingFunctions::lineal)));
        reference.set_movement(Some(expected.clone()), 0.0);
        assert!(Rc::ptr_eq(
            &reference.get_movement(500.0).unwrap(),
            &expected
        ));
        assert!(reference.is_completed());
        target
            .set(&value, Discrete(Some(expected.clone())), 0.0)
            .unwrap();
        target.begin_frame();
        assert!(Rc::ptr_eq(
            &target.get(&value, 500.0).unwrap().0.unwrap(),
            &expected
        ));
        assert!(target.is_valid);
        target.get(&value, 600.0).unwrap();
        assert_eq!(count.get(), 1);
        assert!(target.remove_completed_handler(token));
    }

    #[test]
    fn completion_can_retarget_without_borrowing_or_callback_repetition() {
        let mut target = Animatable::new();
        let x = target.register_motion("X", 0.0_f64).unwrap();
        target
            .set_transition(Some(Animation::new(100.0, EasingFunctions::lineal)), &[])
            .unwrap();
        let count = Rc::new(Cell::new(0));
        let counter = count.clone();
        let key = x.clone();
        target
            .on_completed("X", move |target, time| {
                counter.set(counter.get() + 1);
                if counter.get() == 1 {
                    target.set(&key, 200.0, time).unwrap();
                }
            })
            .unwrap();
        target.set(&x, 100.0, 0.0).unwrap();
        assert!(!target.sample_frame(101.0));
        assert_eq!(target.get(&x, 151.0).unwrap(), 150.0);
        assert!(target.sample_frame(202.0));
        assert_eq!(count.get(), 2);
    }

    #[test]
    fn metadata_parser_copy_and_selection_are_type_checked() {
        let mut target = Animatable::new();
        let x = target.register_motion("X", 0.0_f64).unwrap();
        let label = target.register_state("Label", String::new()).unwrap();
        target
            .set_value("X", PropertyValue::from("12.5"), 0.0)
            .unwrap();
        assert_eq!(target.get(&x, 0.0).unwrap(), 12.5);
        target
            .set_value("X", PropertyValue::from("invalid"), 0.0)
            .unwrap();
        assert_eq!(target.get(&x, 0.0).unwrap(), 0.0);
        assert!(matches!(
            target.set_value("X", PropertyValue::new(true), 0.0),
            Err(PropertyError::TypeMismatch { .. })
        ));
        target
            .set_transition(
                Some(Animation::new(100.0, EasingFunctions::lineal)),
                &["X", "Label"],
            )
            .unwrap();
        assert!(target.motion_info(label.name()).unwrap().is_none());
        target.set(&x, 100.0, 0.0).unwrap();
        let mut copy = Animatable::new();
        let copy_x = copy.register_motion("X", 0.0_f64).unwrap();
        copy.copy_motion_from("X", &target, "X").unwrap();
        assert_eq!(copy.get(&copy_x, 50.0).unwrap(), 50.0);
        target.remove_transition(&["X"]).unwrap();
        assert_eq!(target.get(&x, 50.0).unwrap(), 100.0);
        assert!(target.set_transition(None, &["missing"]).is_err());
        assert!(target.register_state("X", false).is_err());
    }

    #[test]
    fn retarget_dispatches_completion_and_samples_repeat_boundary_once() {
        let mut target = Animatable::new();
        let x = target.register_motion("X", 0.0_f64).unwrap();
        target
            .set_transition(Some(Animation::new(100.0, EasingFunctions::lineal)), &[])
            .unwrap();
        let count = Rc::new(Cell::new(0));
        let counter = count.clone();
        target
            .on_completed("X", move |_, _| counter.set(counter.get() + 1))
            .unwrap();
        target.set(&x, 100.0, 0.0).unwrap();
        // A setter samples the old transition; completion is observable even
        // when no frame queried the property after its endpoint.
        target.set(&x, 200.0, 101.0).unwrap();
        assert_eq!(count.get(), 1);
        assert_eq!(target.get(&x, 151.0).unwrap(), 150.0);

        target
            .set_transition(
                Some(Animation::new(100.0, EasingFunctions::lineal).with_repeat(Repeat::Forever)),
                &[],
            )
            .unwrap();
        target.set(&x, 300.0, 151.0).unwrap();
        target.set(&x, 400.0, 251.0).unwrap();
        assert_eq!(target.motion(&x).unwrap().unwrap().from_value, 300.0);
        assert_eq!(target.get(&x, 301.0).unwrap(), 350.0);
    }

    #[test]
    fn imported_properties_preserve_derived_precedence_and_local_callbacks() {
        let mut base = Animatable::new();
        base.register_motion("X", 1.0_f64).unwrap();
        base.register_motion("Y", 2.0_f64).unwrap();
        let mut derived = Animatable::new();
        derived.register_motion("X", 3.0_f64).unwrap();
        let count = Rc::new(Cell::new(0));
        let counter = count.clone();
        derived
            .on_completed("X", move |_, _| counter.set(counter.get() + 1))
            .unwrap();
        let mut instance = Animatable::new();
        instance.import_properties(&base);
        instance.import_properties(&derived);
        assert_eq!(instance.property_definitions().count(), 2);
        assert_eq!(
            *instance
                .target_value("X")
                .unwrap()
                .downcast_ref::<f64>()
                .unwrap(),
            3.0
        );
        assert_eq!(
            *instance
                .target_value("Y")
                .unwrap()
                .downcast_ref::<f64>()
                .unwrap(),
            2.0
        );
        instance
            .set_transition(Some(Animation::new(10.0, EasingFunctions::lineal)), &[])
            .unwrap();
        instance
            .set_value("X", PropertyValue::new(10.0_f64), 0.0)
            .unwrap();
        assert!(instance.sample_frame(11.0));
        assert_eq!(count.get(), 0);
    }
}

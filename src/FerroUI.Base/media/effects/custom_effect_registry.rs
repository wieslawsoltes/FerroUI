//! The registry of custom effects (#455, not from upstream): the names
//! [`Effect::parse`](super::Effect::parse) knows besides `blur` and
//! `drop-shadow`, and so the custom effects markup and styles can write as
//! text (`Effect="tint(color=#0F9D8E, strength=0.55)"`).
//!
//! The registry is only needed for parsing. A custom effect value carries
//! its own equality, padding and interpolation, so transitions and
//! animations of `Visual.Effect` work for every custom effect, registered or
//! not, and the render thread never looks a name up.
//!
//! The effects FerroUI ships ([`library`](super::library)) are registered the
//! first time the registry is read. Registering a type also loads its effect
//! graph, if it has one ([`CustomEffect::graph`]), or else reads its runtime
//! shader ([`CustomEffect::shader`]), and registers either for the prewarm of
//! the GPU programs.

use super::custom_effect::normalize_identifier;
use super::{CustomEffect, EffectParameterValue, ImmutableCustomEffect};
use crate::utilities::FormatError;
use invocation::{Argument, Invocation};
use std::any::TypeId;
use std::fmt;
use std::sync::RwLock;

#[path = "custom_effect_invocation.rs"]
mod invocation;

/// The names of the built-in effects, normalized: a custom effect cannot
/// take them.
const RESERVED_NAMES: [&str; 2] = ["blur", "dropshadow"];

/// Parses the arguments of an effect string into a value of one registered
/// type; the error is the reason, for the message of the parse error.
type ParseFn = fn(&[Argument<'_>]) -> Result<ImmutableCustomEffect, String>;

struct Registration {
    name: &'static str,
    key: String,
    type_id: TypeId,
    parse: ParseFn,
}

static REGISTRY: RwLock<Vec<Registration>> = RwLock::new(Vec::new());

/// Why a custom effect could not be registered.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CustomEffectRegistrationError {
    /// The name is empty, or has characters other than letters, digits,
    /// `-` and `_`.
    InvalidName(&'static str),
    /// The name is that of a built-in effect (`blur`, `drop-shadow`).
    ReservedName(&'static str),
    /// Another type is registered under the name (compared ignoring case,
    /// `-` and `_`).
    NameTaken(&'static str),
}

impl fmt::Display for CustomEffectRegistrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidName(name) => write!(f, "'{name}' is not a valid effect name"),
            Self::ReservedName(name) => write!(f, "'{name}' is the name of a built-in effect"),
            Self::NameTaken(name) => write!(f, "another custom effect is registered as '{name}'"),
        }
    }
}

impl std::error::Error for CustomEffectRegistrationError {}

/// The custom effects that effect strings can name.
pub struct CustomEffectRegistry;

impl CustomEffectRegistry {
    /// Registers `T` under [`CustomEffect::NAME`], and loads its graph
    /// ([`CustomEffect::graph`]) or its shader ([`CustomEffect::shader`]),
    /// which is registered for the prewarm. Registering a type again does
    /// nothing.
    pub fn register<T: CustomEffect>() -> Result<(), CustomEffectRegistrationError> {
        let name = T::NAME;
        if name.is_empty() || !name.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
            return Err(CustomEffectRegistrationError::InvalidName(name));
        }
        let key = normalize_identifier(name);
        if key.is_empty() {
            return Err(CustomEffectRegistrationError::InvalidName(name));
        }
        if RESERVED_NAMES.contains(&key.as_str()) {
            return Err(CustomEffectRegistrationError::ReservedName(name));
        }
        let mut registry = REGISTRY.write().unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(existing) = registry.iter().find(|registration| registration.key == key) {
            return if existing.type_id == TypeId::of::<T>() {
                Ok(())
            } else {
                Err(CustomEffectRegistrationError::NameTaken(name))
            };
        }
        registry.push(Registration { name, key, type_id: TypeId::of::<T>(), parse: parse_registered::<T> });
        drop(registry);
        // Not under the lock: the graph and the shader are code of the effect.
        let _ = super::custom_effect_graph::load_custom_effect_graph::<T>();
        let _ = super::custom_effect_shader::load_custom_effect_shader::<T>();
        Ok(())
    }

    /// Whether a custom effect is registered as `name` (ignoring case, `-`
    /// and `_`).
    pub fn is_registered(name: &str) -> bool {
        let key = normalize_identifier(name);
        Self::registry().iter().any(|registration| registration.key == key)
    }

    /// The names of the registered custom effects, in the order of their
    /// registration.
    pub fn names() -> Vec<&'static str> {
        Self::registry().iter().map(|registration| registration.name).collect()
    }

    /// The name in a custom-effect invocation, without resolving it in this process.
    /// A designer can forward an invocation to the application that owns its registry.
    pub fn invocation_name(s: &str) -> Option<&str> {
        let name = Invocation::parse(s)?.name;
        let key = normalize_identifier(name);
        (!key.is_empty() && !RESERVED_NAMES.contains(&key.as_str())).then_some(name)
    }

    /// Parses an effect string that names a registered custom effect:
    /// `name(parameter=value, ...)`, with the parameters of the effect in
    /// any order (a missing one keeps its default) and their values in the
    /// invariant format of [`EffectParameterValue`](super::EffectParameterValue).
    /// `None` when `s` is not of that form or names no registered effect.
    pub fn parse(s: &str) -> Option<Result<ImmutableCustomEffect, FormatError>> {
        Self::parse_invocation(s, &Invocation::parse(s)?)
    }

    /// Edit one registered parameter in an effect literal. Unchanged arguments,
    /// spelling, and whitespace stay in place. A changed default is written
    /// explicitly. The registry validates the resulting value and its siblings.
    pub fn edit_literal_parameter(
        source: &str,
        parameter: &str,
        edit: impl FnOnce(EffectParameterValue) -> Result<EffectParameterValue, FormatError>,
    ) -> Result<String, FormatError> {
        Self::edit_literal_parameters(source, |effect| {
            let key = normalize_identifier(parameter);
            let (declared, original) = effect.parameters().into_iter()
                .find(|(name, _)| normalize_identifier(name) == key)
                .ok_or_else(|| FormatError::from_string(format!("The effect has no parameter '{parameter}'")))?;
            Ok(vec![(declared.to_owned(), edit(original)?)])
        })
    }

    /// Edit a set of declared parameters against the same original effect.
    /// Validate the finished literal so related parameters can change together.
    pub fn edit_literal_parameters(
        source: &str,
        edit: impl FnOnce(&ImmutableCustomEffect) -> Result<Vec<(String, EffectParameterValue)>, FormatError>,
    ) -> Result<String, FormatError> {
        let invalid = || FormatError::new("Use a literal for a registered effect");
        let before = Self::parse(source).ok_or_else(invalid)??;
        let mut parameters = before.parameters();
        let mut seen = std::collections::HashSet::new();
        let mut updated = source.to_owned();
        for (name, value) in edit(&before)? {
            let (_, original) = parameters.iter_mut().find(|(candidate, _)| *candidate == name.as_str())
                .ok_or_else(|| FormatError::from_string(format!("The effect has no parameter '{name}'")))?;
            if !seen.insert(name.clone()) {
                return Err(FormatError::new("Edit each effect parameter only once"));
            }
            if std::mem::discriminant(original) != std::mem::discriminant(&value) {
                return Err(FormatError::new("Keep the registered parameter type"));
            }
            if *original != value {
                updated = Invocation::parse(&updated).ok_or_else(invalid)?
                    .replace_argument(&updated, &name, &value.to_string());
                *original = value;
            }
        }
        if updated == source {
            return Ok(updated);
        }
        let after = Self::parse(&updated).ok_or_else(invalid)??;
        let following = after.parameters();
        if parameters.len() != following.len() || parameters.iter().any(|(name, expected)| {
            !following.iter().any(|(candidate, value)| candidate == name && value == expected)
        }) {
            return Err(FormatError::new(
                "This effect cannot preserve the requested parameter value and its siblings",
            ));
        }
        Ok(updated)
    }

    fn parse_invocation(s: &str, invocation: &Invocation<'_>) -> Option<Result<ImmutableCustomEffect, FormatError>> {
        let key = normalize_identifier(invocation.name);
        let parse = Self::registry().iter().find(|registration| registration.key == key)?.parse;
        Some(parse(&invocation.arguments).map_err(|reason| FormatError::from_string(format!("Unable to parse effect: {s} ({reason})"))))
    }

    /// The registrations, the effects of the library among them.
    fn registry() -> std::sync::RwLockReadGuard<'static, Vec<Registration>> {
        super::library::register_library_effects();
        REGISTRY.read().unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

/// Makes a `T` from its default and the named arguments.
fn parse_registered<T: CustomEffect>(arguments: &[Argument<'_>]) -> Result<ImmutableCustomEffect, String> {
    let mut value = T::default();
    for Argument { name: argument, value: text, .. } in arguments {
        let key = normalize_identifier(argument);
        let mut declared = None;
        CustomEffect::for_each_parameter(&value, &mut |name, current| {
            if declared.is_none() && normalize_identifier(name) == key {
                declared = Some((name, current));
            }
        });
        let Some((name, current)) = declared else {
            return Err(format!("{} has no parameter '{argument}'", T::NAME));
        };
        let Some(parsed) = current.parse_like(text) else {
            return Err(format!("'{text}' is not a valid value of the parameter '{name}'"));
        };
        if !value.set_parameter(name, parsed) {
            return Err(format!("the parameter '{name}' does not take '{text}'"));
        }
    }
    Ok(ImmutableCustomEffect::new(value))
}

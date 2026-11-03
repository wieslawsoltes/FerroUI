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
use super::{CustomEffect, ImmutableCustomEffect};
use crate::utilities::FormatError;
use std::any::TypeId;
use std::fmt;
use std::sync::RwLock;

/// The names of the built-in effects, normalized: a custom effect cannot
/// take them.
const RESERVED_NAMES: [&str; 2] = ["blur", "dropshadow"];

/// Parses the arguments of an effect string into a value of one registered
/// type; the error is the reason, for the message of the parse error.
type ParseFn = fn(&[(String, String)]) -> Result<ImmutableCustomEffect, String>;

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

    /// Parses an effect string that names a registered custom effect:
    /// `name(parameter=value, ...)`, with the parameters of the effect in
    /// any order (a missing one keeps its default) and their values in the
    /// invariant format of [`EffectParameterValue`](super::EffectParameterValue).
    /// `None` when `s` is not of that form or names no registered effect.
    pub fn parse(s: &str) -> Option<Result<ImmutableCustomEffect, FormatError>> {
        let (name, arguments) = parse_invocation(s)?;
        let key = normalize_identifier(name);
        let parse = Self::registry().iter().find(|registration| registration.key == key)?.parse;
        Some(parse(&arguments).map_err(|reason| FormatError::from_string(format!("Unable to parse effect: {s} ({reason})"))))
    }

    /// The registrations, the effects of the library among them.
    fn registry() -> std::sync::RwLockReadGuard<'static, Vec<Registration>> {
        super::library::register_library_effects();
        REGISTRY.read().unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

/// Makes a `T` from its default and the named arguments.
fn parse_registered<T: CustomEffect>(arguments: &[(String, String)]) -> Result<ImmutableCustomEffect, String> {
    let mut value = T::default();
    for (argument, text) in arguments {
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

/// Splits `name(argument=value, ...)` into the name and the arguments.
///
/// Arguments are separated by `,` or `;` outside parentheses, brackets,
/// braces and quotes. A piece without `=` continues the value before it, so
/// that `offset=10,20` is one argument.
fn parse_invocation(s: &str) -> Option<(&str, Vec<(String, String)>)> {
    let trimmed = s.trim();
    let open = trimmed.find('(')?;
    let inner = trimmed.strip_suffix(')')?.get(open + 1..)?.trim();
    let name = trimmed[..open].trim();
    if name.is_empty() || !name.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
        return None;
    }
    let mut arguments: Vec<(String, String)> = Vec::new();
    if inner.is_empty() {
        return Some((name, arguments));
    }
    for piece in split_top_level(inner)? {
        let piece = piece.trim();
        if piece.is_empty() {
            return None;
        }
        match find_top_level(piece, '=') {
            Some(equals) => {
                let argument = piece[..equals].trim();
                let value = piece[equals + 1..].trim();
                if argument.is_empty() || value.is_empty() {
                    return None;
                }
                arguments.push((argument.to_string(), value.to_string()));
            }
            None => {
                let (_, value) = arguments.last_mut()?;
                value.push(',');
                value.push_str(piece);
            }
        }
    }
    Some((name, arguments))
}

/// The pieces of `s` between the separators (`,` and `;`) that are outside
/// parentheses, brackets, braces and quotes; `None` when those do not
/// balance.
fn split_top_level(s: &str) -> Option<Vec<&str>> {
    let mut pieces = Vec::new();
    let mut start = 0;
    let mut depth = 0usize;
    let mut quote = None;
    for (index, c) in s.char_indices() {
        if let Some(open) = quote {
            if c == open {
                quote = None;
            }
            continue;
        }
        match c {
            '"' | '\'' => quote = Some(c),
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth = depth.checked_sub(1)?,
            ',' | ';' if depth == 0 => {
                pieces.push(&s[start..index]);
                start = index + c.len_utf8();
            }
            _ => {}
        }
    }
    if depth != 0 || quote.is_some() {
        return None;
    }
    pieces.push(&s[start..]);
    Some(pieces)
}

/// The offset of the first `target` outside parentheses, brackets, braces
/// and quotes.
fn find_top_level(s: &str, target: char) -> Option<usize> {
    let mut depth = 0usize;
    let mut quote = None;
    for (index, c) in s.char_indices() {
        if let Some(open) = quote {
            if c == open {
                quote = None;
            }
            continue;
        }
        match c {
            '"' | '\'' => quote = Some(c),
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth = depth.saturating_sub(1),
            _ if c == target && depth == 0 => return Some(index),
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_invocation_splits_into_its_name_and_arguments() {
        let (name, arguments) = parse_invocation(" tint ( color = #0F9D8E , strength=0.55 ) ").unwrap();
        assert_eq!("tint", name);
        assert_eq!(
            vec![("color".to_string(), "#0F9D8E".to_string()), ("strength".to_string(), "0.55".to_string())],
            arguments
        );
    }

    #[test]
    fn values_with_commas_stay_one_argument() {
        let (_, arguments) = parse_invocation("glow(color=rgba(1, 2, 3, 0.5), offset=10,20; label='a, b')").unwrap();
        assert_eq!(
            vec![
                ("color".to_string(), "rgba(1, 2, 3, 0.5)".to_string()),
                ("offset".to_string(), "10,20".to_string()),
                ("label".to_string(), "'a, b'".to_string()),
            ],
            arguments
        );
    }

    #[test]
    fn an_invocation_without_arguments_has_none() {
        assert_eq!(Some(("invert", Vec::new())), parse_invocation("invert()"));
    }

    #[test]
    fn malformed_invocations_are_rejected() {
        for s in ["", "tint", "tint(", "(a=1)", "tint(a=1", "tint(a=)", "tint(=1)", "tint(1)", "tint(a=(1)", "ti nt(a=1)", "tint(a=1,)"] {
            assert!(parse_invocation(s).is_none(), "{s}");
        }
    }
}

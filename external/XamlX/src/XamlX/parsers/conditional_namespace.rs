//! Conditional XML namespaces (`xmlns:new="uri?IsTypePresent(A.B)"`). Not from upstream.
//!
//! The text after `?` names a condition. The elements and attributes in such a namespace are
//! kept or dropped by the answer of a [`XamlConditionResolver`], and the namespace itself reads
//! as the part before `?`. A condition that is only known when the document loads stays in the
//! tree as a [`XamlLoadCondition`] of the namespace scope of the nodes it guards.
//!
//! A namespace without a condition never reaches this code: a document without one parses as
//! before.

/// A condition written in a namespace: `IsTypePresent(A.B)` or `cond:FeatureFlag(Name)`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct XamlCondition {
    /// The namespace prefix of a custom condition (`cond`).
    pub prefix: Option<String>,
    /// The name of the condition.
    pub name: String,
    /// The text between the parentheses, trimmed.
    pub argument: String,
}

impl XamlCondition {
    /// The arguments, separated by commas and trimmed.
    pub fn arguments(&self) -> Vec<&str> {
        if self.argument.is_empty() {
            return Vec::new();
        }
        self.argument.split(',').map(str::trim).collect()
    }
}

/// What a condition says about the nodes in its namespace.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum XamlConditionOutcome {
    /// The nodes are read.
    Included,
    /// The nodes are left out, before anything resolves their types.
    Excluded,
    /// The answer is known when the document loads: the nodes stay, guarded by the condition.
    AtLoad,
}

/// Answers the conditions of a document.
pub trait XamlConditionResolver {
    /// The outcome of `condition`, or the message of an error in it.
    fn resolve(&self, condition: &XamlCondition) -> Result<XamlConditionOutcome, String>;
}

/// A condition that guards nodes until the document loads.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct XamlLoadCondition {
    /// The name of the condition.
    pub name: String,
    /// The text of its argument.
    pub argument: String,
}

/// The namespace before `?` and the condition after it, for a namespace that has one. A `?`
/// that is not followed by `name(..)` is an ordinary character of the namespace.
pub fn split_conditional_namespace(namespace: &str) -> Option<(&str, XamlCondition)> {
    let (base, text) = namespace.split_once('?')?;
    if base.is_empty() {
        return None;
    }
    let inner = text.trim().strip_suffix(')')?;
    let (head, argument) = inner.split_once('(')?;
    let (prefix, name) = match head.split_once(':') {
        Some((prefix, name)) => (Some(prefix.trim()), name.trim()),
        None => (None, head.trim()),
    };
    let is_name = |text: &str| {
        text.chars().next().is_some_and(|c| c.is_alphabetic() || c == '_')
            && text.chars().all(|c| c.is_alphanumeric() || c == '_')
    };
    if !is_name(name) || prefix.is_some_and(|prefix| !is_name(prefix)) {
        return None;
    }
    Some((
        base,
        XamlCondition {
            prefix: prefix.map(str::to_string),
            name: name.to_string(),
            argument: argument.trim().to_string(),
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_namespace_with_a_condition_splits() {
        let (base, condition) = split_conditional_namespace("http://a/b?IsTypePresent(A.B)").unwrap();
        assert_eq!(base, "http://a/b");
        assert_eq!(condition.prefix, None);
        assert_eq!((condition.name.as_str(), condition.argument.as_str()), ("IsTypePresent", "A.B"));

        let (_, condition) = split_conditional_namespace("using:X?cond:Flag( New )").unwrap();
        assert_eq!(condition.prefix.as_deref(), Some("cond"));
        assert_eq!((condition.name.as_str(), condition.argument.as_str()), ("Flag", "New"));

        let (_, condition) = split_conditional_namespace("u?IsApiContractPresent(C.D, 8)").unwrap();
        assert_eq!(condition.arguments(), ["C.D", "8"]);
    }

    #[test]
    fn a_question_mark_alone_is_part_of_the_namespace() {
        let plain = ["http://a/b", "http://a/b?x=1", "http://a/b?", "?IsTypePresent(A)", "u?not a name(A)", "u?Name(A"];
        for namespace in plain {
            assert_eq!(split_conditional_namespace(namespace), None, "{namespace}");
        }
    }
}

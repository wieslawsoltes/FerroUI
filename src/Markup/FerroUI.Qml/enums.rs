//! Immutable type namespaces are shared between objects with the same imports.
//! Enum values retain declaration order for the Qt name/value conversion APIs.
use super::*;
use ferroui_qml_compiler::{Import, MemberKind, Object};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Namespace {
    members: BTreeMap<String, Value>,
    keys: Option<Vec<(String, i32)>>,
    pub(crate) list_owner: Option<ObjectId>,
}

impl Namespace {
    pub(crate) fn names(&self) -> impl Iterator<Item = &str> {
        self.members.keys().map(String::as_str)
    }
    pub(crate) fn inherit(&mut self, derived: &Self) {
        self.members.extend(derived.members.clone());
    }

    pub(crate) fn member(&self, name: &str) -> Option<Value> {
        self.members.get(name).cloned()
    }

    pub(crate) fn attach_list_view<'a>(
        &mut self,
        owner: ObjectId,
        names: impl Iterator<Item = &'a String>,
    ) {
        if let Some(Value::Namespace(namespace)) = self.members.get_mut("ListView") {
            let namespace = Rc::make_mut(namespace);
            namespace.list_owner = Some(owner);
            namespace
                .members
                .extend(names.map(|k| (k.clone(), Value::Undefined)));
        }
    }

    fn value(self) -> Value {
        Value::Namespace(Rc::new(self))
    }

    pub(crate) fn declare(&mut self, name: &str, keys: &[(String, i32)]) -> Result<(), Error> {
        let mut entries = BTreeMap::new();
        for (key, ordinal) in keys {
            if !key.starts_with(char::is_uppercase)
                || entries
                    .insert(key.clone(), Value::Number(*ordinal as f64))
                    .is_some()
            {
                return Err(Error::new(format!("invalid or duplicate enum key '{key}'")));
            }
            self.members
                .insert(key.clone(), Value::Number(*ordinal as f64));
        }
        if !name.starts_with(char::is_uppercase) {
            return Err(Error::new(format!("invalid enum name '{name}'")));
        }
        self.members.insert(
            name.into(),
            Self {
                members: entries,
                keys: Some(keys.to_vec()),
                list_owner: None,
            }
            .value(),
        );
        Ok(())
    }

    pub(crate) fn declared(object: &Object) -> Result<Self, Error> {
        let mut result = Self::default();
        for m in &object.members {
            if let MemberKind::Enum { name, values } = &m.kind {
                result.declare(name, values)?;
            }
        }
        Ok(result)
    }
}

type ScopeKey = Vec<(String, Option<String>)>;

#[derive(Clone, Default)]
pub(crate) struct Scopes {
    by_imports: BTreeMap<ScopeKey, Rc<Namespace>>,
    local: Option<(String, Value)>,
}

impl Scopes {
    pub(crate) fn named(name: Option<&str>, object: &Object) -> Result<Self, Error> {
        let namespace = Namespace::declared(object)?;
        let local = name.map(|name| (name.into(), namespace.value()));
        Ok(Self {
            by_imports: BTreeMap::new(),
            local,
        })
    }

    pub(crate) fn imports(&mut self, imports: &[Import], registry: &TypeRegistry) -> Rc<Namespace> {
        let key = imports
            .iter()
            .map(|i| (i.uri.clone(), i.alias.clone()))
            .collect::<ScopeKey>();
        self.by_imports
            .entry(key)
            .or_insert_with(|| {
                let mut scope = Namespace::default();
                for import in imports {
                    let mut module = Namespace::default();
                    for uri in registry.module_order(&import.uri).into_iter().rev() {
                        if let Some(types) = registry.modules.get(uri) {
                            for (name, ty) in types {
                                if !ty.enums.members.is_empty() {
                                    module
                                        .members
                                        .insert(name.clone(), Value::Namespace(ty.enums.clone()));
                                }
                            }
                        }
                    }
                    if let Some(alias) = &import.alias {
                        scope.members.insert(alias.clone(), module.value());
                    } else {
                        scope.members.append(&mut module.members);
                    }
                }
                if let Some((name, namespace)) = &self.local {
                    scope.members.insert(name.clone(), namespace.clone());
                }
                Rc::new(scope)
            })
            .clone()
    }
}

pub(crate) fn qt(name: &str, arguments: &[Value]) -> Result<Value, Error> {
    match (name, arguments) {
        (
            "enumStringToValue" | "enumValueToString" | "enumValueToStrings",
            [Value::Namespace(namespace), arg],
        ) => {
            let keys = namespace
                .keys
                .as_ref()
                .ok_or_else(|| Error::new("expected an enum namespace"))?;
            if name == "enumStringToValue" {
                let Value::String(key) = arg else {
                    return Err(Error::new("expected an enum key string"));
                };
                return namespace
                    .member(key)
                    .ok_or_else(|| Error::new(format!("unknown enum key '{key}'")));
            }
            let Value::Number(n) = arg else {
                return Err(Error::new("expected a numeric enum value"));
            };
            let mut matches = keys
                .iter()
                .filter(|(_, v)| *v as f64 == *n)
                .map(|(key, _)| Value::String(key.clone()));
            let first = matches
                .next()
                .ok_or_else(|| Error::new(format!("no key for enum value {n}")))?;
            if name == "enumValueToString" {
                Ok(first)
            } else {
                Ok(Value::List(std::iter::once(first).chain(matches).collect()))
            }
        }
        ("escapeHtml", [Value::String(text)]) => {
            let mut escaped = String::with_capacity(text.len());
            for c in text.chars() {
                match c {
                    '<' => escaped.push_str("&lt;"),
                    '>' => escaped.push_str("&gt;"),
                    '&' => escaped.push_str("&amp;"),
                    '"' => escaped.push_str("&quot;"),
                    _ => escaped.push(c),
                }
            }
            Ok(Value::String(escaped))
        }
        ("isQtObject", [v]) => Ok(Value::Bool(matches!(v, Value::Object(_)))),
        _ => Err(Error::new(format!("unsupported Qt.{name} invocation"))),
    }
}

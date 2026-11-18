use super::{MappedProperty, RegisteredClass};
use crate::{
    catalog::{self, Children},
    literal::Literal,
};
use std::{collections::BTreeSet, sync::Arc};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Class {
    Builtin(String),
    Registered(Arc<RegisteredClass>),
}

impl Class {
    pub(crate) fn name(&self) -> &str {
        match self {
            Self::Builtin(name) => name,
            Self::Registered(class) => &class.name,
        }
    }

    pub(crate) fn namespace(&self) -> Option<&str> {
        match self {
            Self::Builtin(_) => None,
            Self::Registered(class) => Some(&class.namespace),
        }
    }

    pub(crate) fn rust_name(&self) -> &str {
        match self {
            Self::Builtin(name) => name,
            Self::Registered(class) => &class.rust_path,
        }
    }

    fn base(&self) -> Option<&str> {
        match self {
            Self::Builtin(name) => Some(name),
            Self::Registered(class) => class.base.as_deref(),
        }
    }

    pub(crate) fn children(&self) -> Children {
        self.base()
            .and_then(catalog::children)
            .unwrap_or(Children::None)
    }

    pub(crate) fn text_property(&self) -> Option<&'static str> {
        self.base()
            .and_then(|base| catalog::text_property(base, self.children()))
    }

    pub(crate) fn mapped_events(&self) -> bool {
        self.base().is_some()
    }

    fn mapped_property(&self, name: &str) -> Option<&MappedProperty> {
        match self {
            Self::Builtin(_) => None,
            Self::Registered(class) => class.properties.get(name),
        }
    }

    pub(crate) fn literal(&self, name: &str) -> Option<Literal> {
        self.mapped_property(name)
            .map(|property| property.codec.clone())
            .or_else(|| {
                let base = self.base()?;
                if let Self::Registered(class) = self {
                    let setter = catalog::setter(name);
                    if class
                        .properties
                        .values()
                        .any(|property| property.setter == setter)
                    {
                        return None;
                    }
                }
                catalog::property(base, name)
            })
    }

    pub(crate) fn setter(&self, name: &str) -> String {
        self.mapped_property(name)
            .map_or_else(|| catalog::setter(name), |property| property.setter.clone())
    }

    pub(crate) fn xaml_property(&self, owner: Option<&str>, setter: &str) -> Option<String> {
        if let Self::Registered(class) = self {
            let explicit = class.properties.iter().find(|(_, property)| {
                owner.is_none() && property.setter.trim_start_matches("r#") == setter
            });
            if let Some((name, _)) = explicit { return Some(name.clone()); }
        }
        let name = catalog::from_setter(self.base()?, owner, setter)?;
        (self.mapped_property(&name).is_none()).then_some(name)
    }

    pub(crate) fn property_names(&self) -> BTreeSet<String> {
        let mut names = BTreeSet::new();
        if let Some(base) = self.base() {
            names.extend(
                catalog::schema()
                    .properties(base, None)
                    .into_values()
                    .filter(|info| !info.attached && catalog::property(base, &info.name).is_some())
                    .map(|info| info.name),
            );
            names.extend(["Name".into(), "Classes".into()]);
        }
        if let Self::Registered(class) = self {
            names.extend(class.properties.keys().cloned());
            names.retain(|name| self.literal(name).is_some());
        }
        names
    }

    pub(crate) fn property_type(&self, name: &str) -> String {
        self.mapped_property(name)
            .map(|property| property.value_type.clone())
            .or_else(|| {
                catalog::schema()
                    .property(self.base()?, None, name)
                    .map(|info| info.type_name)
            })
            .unwrap_or_else(|| "String".into())
    }
}

impl std::fmt::Display for Class {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.name())
    }
}

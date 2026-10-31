use super::{Error, Parameter, Value};
use ferroui_qml_compiler::Import;
use std::collections::BTreeMap;

pub type NativeCallback = fn(&[Value]) -> Result<Value, Error>;

#[derive(Clone, Debug)]
pub(crate) struct NativeMethod {
    pub(crate) parameters: Vec<Parameter>,
    pub(crate) result: String,
    pub(crate) callback: NativeCallback,
}

#[derive(Clone, Debug, Default)]
pub struct TypeDefinition {
    base: Option<(String, String)>,
    pub(crate) lineage: Vec<(String, String)>,
    pub(crate) properties: BTreeMap<String, (String, Value)>,
    pub(crate) signals: BTreeMap<String, Vec<Parameter>>,
    pub(crate) native: BTreeMap<String, NativeMethod>,
    pub(crate) default_property: Option<String>,
}

impl TypeDefinition {
    /// Extend a previously registered native type, retaining its members and
    /// assignability. A derived definition may specialize inherited defaults.
    pub fn inherits(mut self, module: &str, name: &str) -> Self {
        self.base = Some((module.into(), name.into()));
        self
    }

    pub fn property(mut self, name: &str, ty: &str, default: Value) -> Self {
        self.properties.insert(name.into(), (ty.into(), default));
        self
    }

    pub fn signal(mut self, name: &str, parameters: Vec<Parameter>) -> Self {
        self.signals.insert(name.into(), parameters);
        self
    }

    /// A deterministic Rust function may be called from methods or bindings.
    /// Its result must depend only on its arguments for invalidation to be correct.
    pub fn method(
        mut self,
        name: &str,
        parameters: Vec<Parameter>,
        result: &str,
        callback: NativeCallback,
    ) -> Self {
        self.native.insert(
            name.into(),
            NativeMethod {
                parameters,
                result: result.into(),
                callback,
            },
        );
        self
    }

    pub fn default_property(mut self, name: &str) -> Self {
        self.default_property = Some(name.into());
        self
    }
}

/// A registry explicitly owns the modules visible to a QML component.
pub struct TypeRegistry {
    modules: BTreeMap<String, BTreeMap<String, TypeDefinition>>,
    versions: BTreeMap<(String, u16), u16>,
}

impl Default for TypeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            modules: BTreeMap::new(),
            versions: BTreeMap::new(),
        };
        let qt_object = TypeDefinition::default().property(
            "objectName",
            "string",
            Value::String(String::new()),
        );
        registry
            .modules
            .entry("QtQml".into())
            .or_default()
            .insert("QtObject".into(), qt_object);
        let connections = TypeDefinition::default()
            .inherits("QtQml", "QtObject")
            .property("target", "QtObject", Value::Null)
            .property("enabled", "bool", Value::Bool(true))
            .property("ignoreUnknownSignals", "bool", Value::Bool(false));
        registry
            .register("QtQml", "Connections", connections)
            .expect("built-in Connections contract");
        registry.set_module_version("QtQml", 2, 15);
        registry.set_module_version("QtQml", 6, 12);
        registry
    }

    /// Permit versioned imports of this major up to the given minor version.
    pub fn set_module_version(&mut self, module: &str, major: u16, minor: u16) {
        self.versions.insert((module.into(), major), minor);
    }

    pub fn register(
        &mut self,
        module: &str,
        name: &str,
        mut definition: TypeDefinition,
    ) -> Result<(), Error> {
        if let Some((base_module, base_name)) = definition.base.take() {
            let base = self
                .modules
                .get(&base_module)
                .and_then(|types| types.get(&base_name))
                .ok_or_else(|| {
                    Error::new(format!("unknown base type {base_module}.{base_name}"))
                })?;
            let mut merged = base.clone();
            merged.lineage.push((base_module, base_name));
            merged.properties.append(&mut definition.properties);
            merged.signals.append(&mut definition.signals);
            merged.native.append(&mut definition.native);
            if definition.default_property.is_some() {
                merged.default_property = definition.default_property;
            }
            definition = merged;
        }
        if let Some(default) = &definition.default_property {
            if !definition.properties.contains_key(default) {
                return Err(Error::new(format!(
                    "missing default property {name}.{default}"
                )));
            }
        }
        let types = self.modules.entry(module.to_owned()).or_default();
        if types.contains_key(name) {
            return Err(Error::new(format!("duplicate type {module}.{name}")));
        }
        types.insert(name.to_owned(), definition);
        Ok(())
    }

    pub(crate) fn validate_imports(&self, imports: &[Import]) -> Result<(), Error> {
        for import in imports {
            if import.is_path {
                return Err(Error::new(format!(
                    "file import '{}' requires a component resolver",
                    import.uri
                )));
            }
            if !self.modules.contains_key(&import.uri) {
                return Err(Error::new(format!("unknown QML module '{}'", import.uri)));
            }
            if let Some(version) = &import.version {
                let (major, minor) = version.split_once('.').unwrap_or((version, "0"));
                let major = major
                    .parse::<u16>()
                    .map_err(|_| Error::new("invalid module major version"))?;
                let minor = minor
                    .parse::<u16>()
                    .map_err(|_| Error::new("invalid module minor version"))?;
                if !self
                    .versions
                    .get(&(import.uri.clone(), major))
                    .is_some_and(|maximum| minor <= *maximum)
                {
                    return Err(Error::new(format!(
                        "unavailable module version {} {version}",
                        import.uri
                    )));
                }
            }
        }
        Ok(())
    }

    pub(crate) fn resolve<'a>(
        &'a self,
        imports: &[Import],
        name: &str,
    ) -> Result<(&'a str, &'a str, &'a TypeDefinition), Error> {
        let (namespace, name) = name
            .split_once('.')
            .map_or((None, name), |(ns, n)| (Some(ns), n));
        for import in imports
            .iter()
            .rev()
            .filter(|i| i.alias.as_deref() == namespace)
        {
            let Some((module, types)) = self.modules.get_key_value(&import.uri) else {
                continue;
            };
            if let Some((name, ty)) = types.get_key_value(name) {
                return Ok((module, name, ty));
            }
        }
        Err(Error::new(format!(
            "unresolved QML type '{}{name}'",
            namespace.map(|s| format!("{s}.")).unwrap_or_default()
        )))
    }
}

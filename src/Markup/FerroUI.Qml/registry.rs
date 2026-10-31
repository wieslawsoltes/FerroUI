use super::{Error, Parameter, Value};
use crate::Namespace;
use ferroui_qml_compiler::Import;
use std::{
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
};

pub type NativeCallback = fn(&[Value]) -> Result<Value, Error>;

#[derive(Clone, Debug)]
pub(crate) struct NativeMethod {
    pub(crate) parameters: Vec<Parameter>,
    pub(crate) result: String,
    pub(crate) callback: NativeCallback,
}

#[derive(Clone, Debug, Default)]
pub struct TypeDefinition {
    pub(crate) enums: Rc<Namespace>,
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

    /// Register a Rust enumeration in the type namespace. Keys are accessible
    /// via both Type.Enum.Key and Type.Key.
    pub fn enumeration(mut self, name: &str, keys: &[(&str, i32)]) -> Result<Self, Error> {
        let keys = keys
            .iter()
            .map(|(k, v)| (k.to_string(), *v))
            .collect::<Vec<_>>();
        Rc::make_mut(&mut self.enums).declare(name, &keys)?;
        Ok(self)
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
    imports: BTreeMap<String, Vec<String>>,
    pub(crate) modules: BTreeMap<String, BTreeMap<String, TypeDefinition>>,
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
            imports: BTreeMap::new(),
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
        let binding = TypeDefinition::default()
            .inherits("QtQml", "QtObject")
            .property("target", "QtObject", Value::Null)
            .property("property", "string", Value::String(String::new()))
            .property("value", "var", Value::Undefined)
            .property("when", "bool", Value::Bool(true))
            .property("delayed", "bool", Value::Bool(false))
            .property("restoreMode", "int", Value::Number(3.))
            .enumeration(
                "RestoreMode",
                &[
                    ("RestoreNone", 0),
                    ("RestoreBinding", 1),
                    ("RestoreValue", 2),
                    ("RestoreBindingOrValue", 3),
                ],
            )
            .expect("built-in restore modes");
        registry
            .register("QtQml", "Binding", binding)
            .expect("built-in Binding contract");
        let timer = TypeDefinition::default()
            .inherits("QtQml", "QtObject")
            .property("interval", "int", Value::Number(1000.))
            .property("repeat", "bool", Value::Bool(false))
            .property("running", "bool", Value::Bool(false))
            .property("triggeredOnStart", "bool", Value::Bool(false))
            .signal("triggered", Vec::new());
        registry
            .register("QtQml", "Timer", timer)
            .expect("built-in Timer contract");
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

    /// Export another native module in the same import namespace and version.
    /// This is the native counterpart of a qmldir `import Module auto`.
    pub fn reexport(&mut self, module: &str, dependency: &str) -> Result<(), Error> {
        if !self.modules.contains_key(dependency) {
            return Err(Error::new(format!(
                "unknown re-exported module '{dependency}'"
            )));
        }
        if self.module_order(dependency).contains(&module) {
            return Err(Error::new(format!(
                "cyclic module import {module} -> {dependency}"
            )));
        }
        self.modules.entry(module.to_owned()).or_default();
        let imports = self.imports.entry(module.to_owned()).or_default();
        if !imports.iter().any(|uri| uri == dependency) {
            imports.push(dependency.to_owned());
        }
        Ok(())
    }

    pub(crate) fn module_order<'a>(&'a self, uri: &'a str) -> Vec<&'a str> {
        let mut pending = vec![uri];
        let mut order = Vec::new();
        let mut seen = BTreeSet::new();
        while let Some(current) = pending.pop() {
            if seen.insert(current) {
                order.push(current);
                if let Some(more) = self.imports.get(current) {
                    pending.extend(more.iter().map(String::as_str));
                }
            }
        }
        order
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
            Rc::make_mut(&mut merged.enums).inherit(&definition.enums);
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
                for uri in self.module_order(&import.uri) {
                    if !self
                        .versions
                        .get(&(uri.to_owned(), major))
                        .is_some_and(|maximum| minor <= *maximum)
                    {
                        return Err(Error::new(format!(
                            "unavailable module version {uri} {version}"
                        )));
                    }
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
            for uri in self.module_order(&import.uri) {
                if let Some((module, types)) = self.modules.get_key_value(uri) {
                    if let Some((name, ty)) = types.get_key_value(name) {
                        return Ok((module, name, ty));
                    }
                }
            }
        }
        Err(Error::new(format!(
            "unresolved QML type '{}{name}'",
            namespace.map(|s| format!("{s}.")).unwrap_or_default()
        )))
    }
}

//! Emit the QtQuick.tooling schema used by qmllint and Qt Creator.
use super::*;
use std::fmt::Write;

/// Files for an editor import path. Install `qmltypes` as `plugins.qmltypes`
/// next to `qmldir` in the directory corresponding to the dotted module URI.
pub struct ToolingMetadata {
    pub qmldir: String,
    pub qmltypes: String,
}

fn quote(value: &str) -> String {
    let mut result = String::from("\"");
    for ch in value.chars() {
        match ch {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            c if c.is_control() => write!(result, "\\u{:04x}", c as u32).unwrap(),
            _ => result.push(ch),
        }
    }
    result.push('"');
    result
}

fn class(module: &str, name: &str) -> String {
    match (module, name) {
        ("QtQml", "QtObject") => "QObject".into(),
        ("QtQml", "Component") => "QQmlComponent".into(),
        ("QtQml", "Binding") => "QQmlBind".into(),
        ("QtQml", "Connections" | "Timer")
        | ("QtQml.Models", "ListModel" | "ListElement" | "DelegateModel") => format!("QQml{name}"),
        ("QtQml.WorkerScript", "WorkerScript") => "QQuickWorkerScript".into(),
        ("QtQuick" | "QtQuick.Controls", name) => format!("QQuick{name}"),
        _ => format!("FerroUI::{}::{name}", module.replace('.', "::")),
    }
}

struct Export<'a> {
    registry: &'a TypeRegistry,
    uri: &'a str,
    dependencies: BTreeSet<String>,
    text: String,
}

impl TypeRegistry {
    /// Export the registered native module's public contract to Qt 6 tooling.
    /// Call after registering base types, properties, methods, enums and versions.
    pub fn tooling_metadata(&self, module: &str) -> Result<ToolingMetadata, Error> {
        let types = self
            .modules
            .get(module)
            .ok_or_else(|| Error::new(format!("unknown QML module '{module}'")))?;
        if !module.split('.').all(identifier) {
            return Err(Error::new("invalid QML module URI"));
        }
        if self
            .versions
            .iter()
            .any(|((uri, major), minor)| uri == module && (*major >= 255 || *minor >= 255))
        {
            return Err(Error::new(
                "Qt tooling requires version segments in the range 0 through 254",
            ));
        }
        let mut output = Export {
            registry: self,
            uri: module,
            dependencies: BTreeSet::new(),
            text: "import QtQuick.tooling 1.2\n\nModule {\n".into(),
        };
        for (name, definition) in types {
            if !identifier(name) || !name.starts_with(char::is_uppercase) {
                return Err(Error::new(format!("invalid exported type name '{name}'")));
            }
            output.component(name, definition)?;
        }
        output.text.push_str("}\n");
        if output
            .dependencies
            .iter()
            .chain(self.imports.get(module).into_iter().flatten())
            .any(|name| !name.split('.').all(identifier))
        {
            return Err(Error::new("invalid dependency module URI"));
        }
        let mut qmldir = format!("module {module}\ntypeinfo plugins.qmltypes\n");
        if let Some(imports) = self.imports.get(module) {
            for other in imports {
                writeln!(qmldir, "import {other} auto").unwrap();
                output.dependencies.remove(other);
            }
        }
        output.dependencies.remove(module);
        for dependency in output.dependencies {
            let major = self
                .versions
                .keys()
                .filter(|(m, _)| m == &dependency)
                .map(|(_, major)| *major)
                .max()
                .unwrap_or(0);
            writeln!(qmldir, "depends {dependency} {major}.0").unwrap();
        }
        Ok(ToolingMetadata {
            qmldir,
            qmltypes: output.text,
        })
    }
}

fn identifier(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(|c| c.is_alphabetic() || c == '_')
        && chars.all(|c| c.is_alphanumeric() || c == '_')
}

impl Export<'_> {
    fn component(&mut self, name: &str, definition: &TypeDefinition) -> Result<(), Error> {
        let class_name = class(self.uri, name);
        let base = definition.lineage.last();
        let prototype = if let Some((module, name)) = base {
            self.dependencies.insert(module.clone());
            class(module, name)
        } else {
            "QObject".into()
        };
        let mut versions = self
            .registry
            .versions
            .keys()
            .filter(|(m, _)| m == self.uri)
            .map(|(_, v)| *v)
            .collect::<Vec<_>>();
        // Qt's tooling schema requires a numeric version even for versionless modules.
        if versions.is_empty() {
            versions.push(0);
        }
        let exports = versions
            .iter()
            .map(|v| quote(&format!("{}/{name} {v}.0", self.uri)))
            .collect::<Vec<_>>()
            .join(", ");
        let revisions = versions
            .iter()
            .map(|v| (u32::from(*v) << 8).to_string())
            .collect::<Vec<_>>()
            .join(", ");
        self.header(&class_name);
        if prototype != class_name {
            writeln!(self.text, "        prototype: {}", quote(&prototype)).unwrap();
        }
        writeln!(
            self.text,
            "        exports: [{exports}]\n        exportMetaObjectRevisions: [{revisions}]"
        )
        .unwrap();
        if let Some(default) = &definition.default_property {
            writeln!(self.text, "        defaultProperty: {}", quote(default)).unwrap();
        }
        let inherited = base.and_then(|(m, t)| self.registry.modules.get(m)?.get(t));
        for (name, keys) in definition.enums.enumerations() {
            if inherited
                .is_some_and(|ty| ty.enums.enumerations().any(|(n, k)| n == name && k == keys))
            {
                continue;
            }
            let values = keys
                .iter()
                .map(|(key, value)| format!("{}: {value}", quote(key)))
                .collect::<Vec<_>>()
                .join(", ");
            writeln!(
                self.text,
                "        Enum {{ name: {}; values: {{ {values} }} }}",
                quote(name)
            )
            .unwrap();
        }
        // A derived member can extend one leaf of a grouped property. Export
        // that group's complete shape, including inherited siblings.
        let changed: BTreeSet<_> = definition
            .properties
            .iter()
            .filter(|(field, (ty, _))| {
                !inherited.is_some_and(|base| {
                    base.properties
                        .get(*field)
                        .is_some_and(|(old, _)| old == ty)
                        && base.readonly.contains(*field) == definition.readonly.contains(*field)
                })
            })
            .map(|(field, _)| field.split('.').next().unwrap())
            .collect();
        let mut groups = BTreeMap::<String, BTreeMap<String, (String, bool)>>::new();
        let mut group_names = BTreeSet::new();
        for (field, (ty, _)) in &definition.properties {
            if field.starts_with("$qml:") || !changed.contains(field.split('.').next().unwrap()) {
                continue;
            }
            let parts = field.split('.').collect::<Vec<_>>();
            let mut prefix = String::new();
            for (i, part) in parts.iter().enumerate() {
                let path = if prefix.is_empty() {
                    part.to_string()
                } else {
                    format!("{prefix}.{part}")
                };
                let children = i + 1 < parts.len()
                    || matches!(ty.as_str(), "$qml:propertyGroup" | "$qml:anchorGroup");
                if children {
                    group_names.insert(path.clone());
                    groups.entry(prefix.clone()).or_default().insert(
                        part.to_string(),
                        (
                            format!("{class_name}::Group::{}", path.replace('.', "::")),
                            true,
                        ),
                    );
                } else {
                    groups
                        .entry(prefix.clone())
                        .or_default()
                        .entry(part.to_string())
                        .or_insert_with(|| (ty.clone(), definition.readonly.contains(field)));
                }
                prefix = path;
            }
        }
        self.fields(groups.remove("").unwrap_or_default(), definition, true)?;
        for (signal, parameters) in &definition.signals {
            if inherited.is_some_and(|base| base.signals.get(signal) == Some(parameters)) {
                continue;
            }
            self.routine("Signal", signal, None, parameters, definition)?;
        }
        for (method, native) in &definition.native {
            if inherited.is_some_and(|base| {
                base.native
                    .get(method)
                    .is_some_and(|m| m.parameters == native.parameters && m.result == native.result)
            }) {
                continue;
            }
            self.routine(
                "Method",
                method,
                Some(&native.result),
                &native.parameters,
                definition,
            )?;
        }
        self.text.push_str("    }\n");
        for group in group_names {
            self.header(&format!(
                "{class_name}::Group::{}",
                group.replace('.', "::")
            ));
            self.text
                .push_str("        prototype: \"QObject\"\n        isCreatable: false\n");
            self.fields(groups.remove(&group).unwrap_or_default(), definition, false)?;
            self.text.push_str("    }\n");
        }
        Ok(())
    }

    fn header(&mut self, class_name: &str) {
        writeln!(
            self.text,
            "    Component {{\n        name: {}\n        accessSemantics: \"reference\"",
            quote(class_name)
        )
        .unwrap();
    }

    fn fields(
        &mut self,
        fields: BTreeMap<String, (String, bool)>,
        definition: &TypeDefinition,
        root: bool,
    ) -> Result<(), Error> {
        for (name, (ty, readonly)) in fields {
            let attrs = self.attributes(&ty, definition)?;
            let notify = format!("{name}Changed");
            write!(
                self.text,
                "        Property {{ name: {}; {attrs}; notify: {}",
                quote(&name),
                quote(&notify)
            )
            .unwrap();
            if readonly {
                self.text.push_str("; isReadonly: true");
            }
            self.text.push_str(" }\n");
            if !root || !definition.signals.contains_key(&notify) {
                writeln!(self.text, "        Signal {{ name: {} }}", quote(&notify)).unwrap();
            }
        }
        Ok(())
    }

    fn routine(
        &mut self,
        kind: &str,
        name: &str,
        result: Option<&str>,
        args: &[Parameter],
        def: &TypeDefinition,
    ) -> Result<(), Error> {
        write!(self.text, "        {kind} {{ name: {}", quote(name)).unwrap();
        if let Some(result) = result.filter(|s| *s != "void") {
            let attrs = self.attributes(result, def)?;
            write!(self.text, "; {attrs}").unwrap();
        }
        for arg in args {
            let attrs = self.attributes(arg.ty.as_deref().unwrap_or("var"), def)?;
            write!(
                self.text,
                "\n            Parameter {{ name: {}; {attrs} }}",
                quote(&arg.name)
            )
            .unwrap();
        }
        self.text.push_str("\n        }\n");
        Ok(())
    }

    fn attributes(&mut self, ty: &str, definition: &TypeDefinition) -> Result<String, Error> {
        let element = ty.strip_prefix("list<").and_then(|n| n.strip_suffix('>'));
        let name = element.unwrap_or(ty);
        let mut pointer = false;
        let kind = match name {
            "bool" | "int" | "float" | "double" => name.into(),
            "real" => "double".into(),
            "string" => "QString".into(),
            "var" | "variant" | "alias" => "QVariant".into(),
            "url" => "QUrl".into(),
            "color" => "QColor".into(),
            "point" => "QPointF".into(),
            "size" => "QSizeF".into(),
            "rect" => "QRectF".into(),
            "vector2d" => "QVector2D".into(),
            "vector3d" => "QVector3D".into(),
            "vector4d" => "QVector4D".into(),
            "quaternion" => "QQuaternion".into(),
            "matrix4x4" => "QMatrix4x4".into(),
            "$qml:anchorLine" => "QQuickAnchorLine".into(),
            n if n.contains("::Group::") => {
                pointer = true;
                n.to_owned()
            }
            n if n.starts_with("$qml:") => "QVariant".into(),
            other => {
                let (module, name) = if let Some((module, name)) = other.rsplit_once('.') {
                    (module, name)
                } else {
                    let module = self
                        .registry
                        .module_order(self.uri)
                        .into_iter()
                        .chain(
                            definition
                                .lineage
                                .iter()
                                .rev()
                                .map(|(module, _)| module.as_str()),
                        )
                        .chain(["QtQml"])
                        .find(|m| {
                            self.registry
                                .modules
                                .get(*m)
                                .is_some_and(|types| types.contains_key(other))
                        })
                        .ok_or_else(|| {
                            Error::new(format!(
                                "cannot export unresolved native type '{other}' in {}",
                                self.uri
                            ))
                        })?;
                    (module, other)
                };
                if !self
                    .registry
                    .modules
                    .get(module)
                    .is_some_and(|types| types.contains_key(name))
                {
                    return Err(Error::new(format!(
                        "cannot export unknown native type '{module}.{name}'"
                    )));
                }
                self.dependencies.insert(module.into());
                pointer = true;
                class(module, name)
            }
        };
        let mut out = format!("type: {}", quote(&kind));
        if element.is_some() {
            out.push_str("; isList: true");
        } else if pointer {
            out.push_str("; isPointer: true");
        }
        Ok(out)
    }
}

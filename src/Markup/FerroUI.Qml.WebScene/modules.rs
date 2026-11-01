//! Produce a private, in-memory V8 module graph from the compiler's embedded resources.
//! The linker has already resolved every static specifier to a canonical module URL.
use ferroui_qml::{
    ir::{ScriptKind, ScriptResource},
    Error,
};
use serde_json::{json, Value as Json};
use std::collections::BTreeMap;

pub(crate) struct Modules {
    pub files: BTreeMap<String, String>,
    pub manifest: Json,
}

impl Modules {
    pub fn prepare(resources: &BTreeMap<String, ScriptResource>) -> Result<Self, Error> {
        let mut files = BTreeMap::new();
        let mut manifest = BTreeMap::new();
        for (url, resource) in resources {
            if !url
                .strip_prefix("https://qml.invalid/modules/")
                .is_some_and(|path| path.split('/').all(valid_segment))
            {
                return Err(Error::new(format!("invalid embedded module URL '{url}'")));
            }
            for (specifier, target) in &resource.dependencies {
                if !resources.contains_key(target) {
                    return Err(Error::new(format!(
                        "'{url}' imports unavailable resource '{specifier}'"
                    )));
                }
            }
            let (kind, source) = match resource.kind {
                ScriptKind::Module => ("module", resource.executable.clone()),
                ScriptKind::Library => ("library", classic_module(resource, url)?),
                ScriptKind::Script => ("script", classic_module(resource, url)?),
            };
            let origin = remote_origin(url)?;
            manifest.insert(
                url,
                json!({
                    "kind": kind,
                    "origin":origin,
                    "imports": resource.imports,
                    "preload": resource.preload,
                    "dependencies": resource.dependencies,
                    "hook": resource.import_hook,
                }),
            );
            files.insert(url.clone(), source);
        }
        Ok(Self {
            files,
            manifest: json!(manifest),
        })
    }
}

// Remote files are served to V8 from the same private in-memory reader as
// bundled files. Restore the declaration URL for import.meta and Qt helpers.
fn remote_origin(key: &str) -> Result<Option<String>, Error> {
    let Some(rest) = key.strip_prefix("https://qml.invalid/modules/remote/") else {
        return Ok(None);
    };
    let (kind, hex) = rest
        .split_once('/')
        .ok_or_else(|| Error::new("invalid remote module identity"))?;
    if !matches!(kind, "module" | "classic") || hex.len() % 2 != 0 || hex.len() > 4096 {
        return Err(Error::new("invalid remote module identity"));
    }
    let bytes = (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| Error::new("invalid remote module identity"))?;
    let origin = String::from_utf8(bytes).map_err(|_| Error::new("invalid remote module URL"))?;
    if !origin.starts_with("https://") && !origin.starts_with("http://") {
        return Err(Error::new("invalid remote module URL"));
    }
    Ok(Some(origin))
}

/// Accept the linker's unreserved characters and percent-encoded file names.
fn valid_segment(segment: &str) -> bool {
    if matches!(segment, "" | "." | "..") {
        return false;
    }
    let mut bytes = segment.bytes();
    while let Some(byte) = bytes.next() {
        if byte == b'%' {
            if !(0..2).all(|_| bytes.next().is_some_and(|b| b.is_ascii_hexdigit())) {
                return false;
            }
        } else if !byte.is_ascii_alphanumeric() && !b"._-~".contains(&byte) {
            return false;
        }
    }
    true
}

/// A classic QML script acquires a fresh lexical environment when its importing
/// component is instantiated. Its export accessors continue to see reassigned
/// variables. The surrounding factory is registered with the same module linker as ESM.
fn classic_module(resource: &ScriptResource, url: &str) -> Result<String, Error> {
    let mut body = String::from("with (arguments[0]) { return (function () {\n");
    body.push_str(&resource.executable);
    body.push_str("\nreturn {\n");
    for name in &resource.exports {
        // Names originate in the ECMAScript syntax tree. Also validate a
        // caller-assembled IR before interpolating names into generated source.
        if !valid_identifier(name) {
            return Err(Error::new(format!(
                "invalid export name '{name}' in '{url}'"
            )));
        }
        let key = serde_json::to_string(name).expect("serialize string");
        let argument = if name == "nextValue" {
            "newValue"
        } else {
            "nextValue"
        };
        let assignment = if resource.writable_exports.contains(name) {
            format!("{name} = {argument};")
        } else {
            format!("throw new TypeError('Cannot assign to a read-only script export: ' + {key});")
        };
        body.push_str(&format!(
            "get [{key}]() {{ return {name}; }}, set [{key}]({argument}) {{ {assignment} }},\n"
        ));
    }
    body.push_str("};\n}).call(undefined); }\n//# sourceURL=");
    body.push_str(url);
    let body = serde_json::to_string(&body).expect("serialize string");
    Ok(format!("System.register([],function(publish){{return {{execute(){{publish('default',Function({body}));}}}};}});\n"))
}

fn valid_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|c| c == '_' || c == '$' || unicode_ident::is_xid_start(c))
        && chars.all(|c| {
            c == '$' || c == '\u{200c}' || c == '\u{200d}' || unicode_ident::is_xid_continue(c)
        })
}

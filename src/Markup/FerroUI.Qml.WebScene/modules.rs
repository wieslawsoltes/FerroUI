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
    pub aliases: Json,
}

impl Modules {
    pub fn prepare(
        resources: &BTreeMap<String, ScriptResource>,
        imports: &BTreeMap<String, String>,
        component: u64,
        origin: &str,
    ) -> Result<Self, Error> {
        let mut files = BTreeMap::new();
        let mut manifest = BTreeMap::new();
        for (url, resource) in resources {
            if !url.starts_with("https://qml.invalid/modules/") {
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
                ScriptKind::Module => ("module", resource.source.clone()),
                ScriptKind::Library => ("library", classic_module(resource, url)?),
                ScriptKind::Script => ("script", classic_module(resource, url)?),
            };
            manifest.insert(url, json!({"kind": kind, "imports": resource.dependencies}));
            files.insert(url.clone(), source);
        }
        for (alias, target) in imports {
            if !resources.contains_key(target) {
                return Err(Error::new(format!(
                    "missing embedded script for import '{alias}'"
                )));
            }
        }
        Ok(Self {
            files,
            aliases: json!({"names": imports, "resources": manifest, "context": [origin, component.to_string()]}),
        })
    }
}

/// A classic QML script acquires a fresh lexical environment when its importing
/// component is instantiated. Its export accessors continue to see reassigned
/// variables. ES modules use V8's own namespace and live-binding machinery.
fn classic_module(resource: &ScriptResource, url: &str) -> Result<String, Error> {
    let mut body = String::from("with (arguments[0]) { return (function () {\n");
    body.push_str(&resource.source);
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
        body.push_str(&format!("get [{key}]() {{ return {name}; }},\n"));
    }
    body.push_str("};\n}).call(undefined); }\n//# sourceURL=");
    body.push_str(url);
    let body = serde_json::to_string(&body).expect("serialize string");
    Ok(format!("export default Function({body});\n"))
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

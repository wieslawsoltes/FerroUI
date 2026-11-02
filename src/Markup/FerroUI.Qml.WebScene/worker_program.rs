//! WorkerScript entry resolution and initialization shared by desktop and browser actors.
use super::*;

pub(super) fn initialization(
    resources: &BTreeMap<String, ir::ScriptResource>,
    url: &str,
) -> Result<Json, Error> {
    let source = &resources[url];
    let classic = source.kind != ir::ScriptKind::Module;
    if classic && (source.kind == ir::ScriptKind::Library || !source.imports.is_empty()) {
        return Err(Error::new(
            "WorkerScript plain scripts cannot use .import or .pragma library",
        ));
    }
    let mut catalog = modules::Modules::prepare(resources)?;
    for definition in catalog
        .manifest
        .as_object_mut()
        .expect("resource manifest")
        .values_mut()
    {
        definition["preload"] = false.into();
    }
    Ok(
        json!({"url": url, "resources": catalog.manifest, "classic": classic.then_some(&source.source), "files": catalog.files}),
    )
}

pub(super) fn entry_url(
    resources: &BTreeMap<String, ir::ScriptResource>,
    origin: &str,
    source: &str,
) -> Result<String, Error> {
    if resources.contains_key(source) {
        return Ok(source.into());
    }
    let missing = || {
        Error::new(format!(
            "WorkerScript source '{source}' is outside the embedded catalog"
        ))
    };
    let base = origin
        .strip_prefix("https://qml.invalid/components/project/")
        .ok_or_else(missing)?;
    if source.starts_with('/') || source.contains([':', '?', '#', '\\']) {
        return Err(missing());
    }
    let mut parts: Vec<_> = base.split('/').map(str::to_owned).collect();
    parts.pop();
    for piece in source.split('/') {
        match piece {
            "" | "." => (),
            ".." => {
                parts.pop().ok_or_else(missing)?;
            }
            raw => {
                let name = percent_encoding::percent_decode_str(raw)
                    .decode_utf8()
                    .map_err(|_| missing())?;
                if name == "." || name == ".." || name.contains(['/', '\\']) {
                    return Err(missing());
                }
                const ENCODE: &percent_encoding::AsciiSet = &percent_encoding::NON_ALPHANUMERIC
                    .remove(b'-')
                    .remove(b'.')
                    .remove(b'~')
                    .remove(b'_');
                parts.push(percent_encoding::utf8_percent_encode(&name, ENCODE).to_string());
            }
        }
    }
    for root in [
        "https://qml.invalid/modules/classic/project/",
        "https://qml.invalid/modules/project/",
    ] {
        let url = format!("{root}{}", parts.join("/"));
        if resources.contains_key(&url) {
            return Ok(url);
        }
    }
    Err(missing())
}

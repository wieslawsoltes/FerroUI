//! Shared import rewriting and System.register compilation for filesystem and
//! transport-backed resource catalogs. Resolution never executes JavaScript.
use crate::{
    javascript::{import_sites, parse_resource, Dependency},
    JavaScript, ScriptForm, ScriptKind, ScriptResource, SourceSpan,
};
use std::collections::BTreeMap;

pub fn attach(
    script: &mut JavaScript,
    origin: &str,
    imports: &std::sync::Arc<BTreeMap<String, String>>,
    mut resolve: impl FnMut(&str) -> Result<String, String>,
) -> Result<(), String> {
    script.origin = origin.to_owned();
    script.imports.clone_from(imports);
    let sites = import_sites(script.code(), script.form == ScriptForm::Expression)
        .map_err(|e| e.to_string())?;
    script.import_hook = sites.hook;
    let mut edits = sites.edits;
    for dep in sites.dependencies {
        let target = resolve(&dep.specifier)?;
        relocate(&mut edits, dep, &target)?;
    }
    if !edits.is_empty() {
        let mut code = script.code().to_owned();
        replace(&mut code, edits);
        script.executable = Some(code);
    }
    Ok(())
}

pub fn compile(
    key: &str,
    source: &str,
    module: bool,
    mut resolve: impl FnMut(&str, bool) -> Result<String, String>,
) -> Result<ScriptResource, String> {
    let parsed = parse_resource(source, module).map_err(|e| e.to_string())?;
    let mut resource = parsed.resource;
    let mut edits = vec![];
    for dep in parsed.dependencies {
        let dependency = resolve(&dep.specifier, dep.namespace.is_none())?;
        if let Some(alias) = &dep.namespace {
            resource.imports.insert(alias.clone(), dependency.clone());
        }
        resource.dependencies.insert(
            dep.namespace
                .clone()
                .unwrap_or_else(|| dep.specifier.clone()),
            dependency.clone(),
        );
        relocate(&mut edits, dep, &dependency)?;
    }
    if resource.kind == ScriptKind::Module {
        replace(&mut resource.executable, edits);
        resource.executable = crate::system_module::compile(key, &resource.executable)?;
    } else {
        let sites = import_sites(&resource.executable, false).map_err(|e| e.to_string())?;
        resource.import_hook = sites.hook;
        edits.extend(sites.edits);
        replace(&mut resource.executable, edits);
        if let Some(lowered) =
            crate::async_functions::lower(&resource.executable, false).map_err(|e| e.to_string())?
        {
            resource.executable = lowered;
        }
    }
    Ok(resource)
}
fn relocate(
    edits: &mut Vec<(SourceSpan, String)>,
    dep: Dependency,
    target: &str,
) -> Result<(), String> {
    if let Some(span) = dep.span {
        if !target.is_ascii()
            || target
                .chars()
                .any(|c| c.is_control() || matches!(c, '"' | '\\'))
        {
            return Err("invalid canonical JavaScript resource URL".into());
        }
        edits.push((span, format!("\"{target}\"")));
    }
    Ok(())
}
fn replace(source: &mut String, mut edits: Vec<(SourceSpan, String)>) {
    edits.sort_by_key(|(s, _)| std::cmp::Reverse(s.start));
    for (range, text) in edits {
        source.replace_range(range.start..range.end, &text);
    }
}

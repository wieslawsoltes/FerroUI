use super::schema as s;
use crate::{library_scopes::LibraryScope, DesignerSession};
use ferroui_designer_language::LineIndex;
use ferroui_designer_rsx::RsxProjection;
use ferroui_designer_tools::ComponentCatalog;
use serde_json::{json, Value};
use std::collections::BTreeMap;

pub const LIBRARY_SCOPES: &str = "designer_library_scopes";

pub(super) fn tool() -> Value {
    s::tool(
        LIBRARY_SCOPES,
        "Browse authored resources, templates, styles, includes and behaviors by owning scope in the active XAML or mapped RSX view. Returns owner and declaration UTF-16 ranges with companion Rust module URIs when applicable.",
        s::object(json!({}), &[]),
        false,
    )
}

pub(super) fn call(session: &DesignerSession) -> Result<Value, String> {
    if let Some(rust) = &session.rust {
        return rust_library_scopes_report(session, rust.source(), &rust.projection()?);
    }
    let declarations =
        ComponentCatalog::declarations(session.document()).map_err(|error| error.to_string())?;
    report(
        session,
        session.authored_source(),
        session.library_scopes(&declarations)?,
        |_| None,
    )
}

pub fn rust_library_scopes_report(
    session: &DesignerSession,
    source: &str,
    projection: &RsxProjection<'_>,
) -> Result<Value, String> {
    report(
        session,
        source,
        session.rust_library_scopes(projection)?,
        |uri| {
            projection
                .module_sources()?
                .source(uri)
                .map(|file| file.source.as_ref())
        },
    )
}

fn report<'a>(
    session: &DesignerSession,
    source: &str,
    scopes: Vec<LibraryScope>,
    module_source: impl Fn(&str) -> Option<&'a str>,
) -> Result<Value, String> {
    let index = LineIndex::new(source);
    let mut modules = BTreeMap::new();
    let scopes = scopes.into_iter().map(|scope| {
        let owner_range = scope.owner_range.map(|span| index.range(span)).transpose()
            .map_err(|error| error.to_string())?;
        let objects = scope.objects.into_iter().map(|object| {
            let authored = if let Some(uri) = &object.source_uri {
                modules.entry(uri.clone()).or_insert_with(|| module_source(uri).map(LineIndex::new))
                    .as_ref().ok_or("The declaration is outside the accepted Rust sources")?
            } else {
                &index
            };
            let range = authored.range(object.source).map_err(|error| error.to_string())?;
            Ok::<_, String>(json!({
                "id":object.id,"node":object.node,"title":object.title,"kind":object.kind,"detail":object.detail,
                "definition":object.definition,"range":range,"sourceUri":object.source_uri
            }))
        }).collect::<Result<Vec<_>, _>>()?;
        Ok::<_, String>(json!({"owner":scope.owner,"title":scope.title,"path":scope.path,"ownerRange":owner_range,"objects":objects}))
    }).collect::<Result<Vec<_>, _>>()?;
    Ok(json!({
        "uri":session.document_uri(),"revision":session.document().revision(),
        "scopeEpoch":session.definition_scope_epoch(),"scopes":scopes,
    }))
}

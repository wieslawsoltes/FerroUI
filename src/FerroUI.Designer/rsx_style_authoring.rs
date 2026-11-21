//! Shared report and source transaction for Rust style editors.
use crate::DesignerSession;
use ferroui_designer_language::LineIndex;
use ferroui_designer_rsx::{RsxProjection, RsxStyleEdit, RsxStyleField};
use serde_json::{json, Value};

impl DesignerSession {
    pub(crate) fn rust_style_projection(&self) -> Result<RsxProjection<'_>, String> {
        if !self.authored_is_valid() {
            return Err("Repair the Rust draft to edit its styles".into());
        }
        self.rust
            .as_ref()
            .ok_or("Open a mapped RSX design")?
            .projection()
    }

    pub(crate) fn edit_rust_style(&mut self, edit: &RsxStyleEdit) -> Result<(), String> {
        let patches = self.rust_style_projection()?.plan_style(edit)?;
        if !patches.is_empty() {
            self.commit_rust_patches("Edit Rust style", patches)
                .map_err(|error| error.to_string())?;
            self.changed(true);
        }
        Ok(())
    }
}

pub fn rust_style_report(
    source: &str,
    projection: &RsxProjection<'_>,
    node: Option<usize>,
) -> Result<Value, String> {
    let index = LineIndex::new(source);
    let styles = projection.style_definitions();
    if let Some(node) = node {
        let (_, style) = styles
            .iter()
            .find(|(_, style)| style.declaration.start == node)
            .ok_or("Choose a style from the current Rust invocation")?;
        let setters = style.setters.iter().map(|setter| {
            let value = setter.value();
            let fallback = source.get(setter.source.range()).unwrap_or_default();
            json!({"node":setter.declaration.start,
                "property":setter.field.as_ref().map_or("Computed property", |field| field.name.as_str()),
                "value":value.as_deref().unwrap_or(fallback),
                "range":index.range(setter.source).ok(),
                "field":setter.field.as_ref().map(field).unwrap_or_else(|| json!({"kind":"text","options":[],"resources":false})),
                "expression":value.is_none(),"editable":value.is_some(),"can_remove":value.is_some(),"object":false})
        }).collect::<Vec<_>>();
        let mut available = projection
            .style_fields()
            .into_iter()
            .filter(|field| {
                !style
                    .setters
                    .iter()
                    .filter_map(|setter| setter.field.as_ref())
                    .any(|original| {
                        original.name == field.name && original.owner_name() == field.owner_name()
                    })
            })
            .collect::<Vec<_>>();
        available.sort_by(|a, b| {
            (&a.name, a.owner_name(), &a.getter).cmp(&(&b.name, b.owner_name(), &b.getter))
        });
        available.dedup_by(|a, b| a.name == b.name && a.owner_name() == b.owner_name());
        let available = available.into_iter().map(|property|
            json!({"property":property.name,"getter":property.getter,"field":field(property)})).collect::<Vec<_>>();
        return Ok(json!({"node":node,"kind":"Style","name":style.selector,
            "range":index.range(style.selector_span).ok(),
            "identity":{"kind":"selector","value":style.selector,"editable":true},
            "nested_allowed":true,"can_remove":style.can_remove(),"setters":setters,"available":available}));
    }
    let definitions = styles.into_iter().map(|(owner, style)| {
        let span = projection.nodes[owner].name;
        let name = &source[span.range()];
        let line = index.position(span.start).map_or(0, |position| position.line + 1);
        let scope = if style.is_nested() { format!("{name} · Nested · line {line}") } else { format!("{name} · line {line}") };
        json!({"node":style.declaration.start,"kind":"Style","name":style.selector,"scope":scope})
    }).collect::<Vec<_>>();
    let style_hosts = projection.nodes.iter().enumerate().filter(|(owner, _)| projection.can_create_style(*owner))
        .map(|(owner, node)| {
            let name = &source[node.name.range()];
            let line = index.position(node.name.start).map_or(0, |position| position.line + 1);
            json!({"node":owner,"label":format!("{name} · line {line}"),"type_name":name,"nested":false})
        }).collect::<Vec<_>>();
    let issues = projection
        .style_issues
        .iter()
        .map(|issue| json!({"range":index.range(issue.source).ok(),"reason":issue.reason}))
        .collect::<Vec<_>>();
    Ok(
        json!({"source_kind":"rust","definitions":definitions,"style_hosts":style_hosts,"palette_scopes":[],"issues":issues}),
    )
}

fn field(field: &RsxStyleField) -> Value {
    let options = if field.kind() == "boolean" {
        vec!["true".into(), "false".into()]
    } else {
        field.options().to_vec()
    };
    json!({"kind":field.kind(),"type_name":field.type_name,"options":options,"resources":false,"owner":field.owner_name()})
}

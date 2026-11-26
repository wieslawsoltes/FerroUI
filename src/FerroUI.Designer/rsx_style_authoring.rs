//! Shared report and source transaction for Rust style editors.
use crate::DesignerSession;
use ferroui_designer_language::LineIndex;
use ferroui_designer_rsx::{RsxProjection, RsxStyleEdit, RsxStyleField, RsxStylePlan};
use serde_json::{json, Value};

#[path = "rsx_style_palette.rs"]
pub(crate) mod palette;

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
        let plan = self.rust_style_projection()?.plan_style_source(edit)?;
        self.apply_rust_style_plan(plan)
    }

    pub(crate) fn apply_rust_style_plan(&mut self, plan: RsxStylePlan) -> Result<(), String> {
        if !plan.patches.is_empty() {
            self.commit_rust_style_plan(plan)
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
    let modules = projection.module_sources();
    let styles = projection.style_definitions();
    if let Some(node) = node {
        let (_, style) = styles
            .iter()
            .find(|(_, style)| style.declaration.start == node)
            .ok_or("Choose a style from the current Rust invocation")?;
        let families = projection.style_resource_families(node);
        let choices: Vec<_> = families.iter().filter(|family| family.can_assign_color()).collect();
        let has_choices = !choices.is_empty();
        let setters = style.setters.iter().map(|setter| {
            let value = setter.value();
            let fallback = source.get(setter.source.range()).unwrap_or_default();
            let resource = setter.resource_key().map(|key| {
                let declaration = choices.iter().find(|entry| entry.key == key)
                    .and_then(|family| family.common_declaration())
                    .and_then(|entry| palette::source_range(&index, modules, entry));
                json!({"key":key,"declaration":declaration.as_ref().map(|(range, _)| range),
                    "sourceAddress":declaration.and_then(|(_, address)| address)})
            });
            json!({"node":setter.declaration.start,
                "property":setter.field.as_ref().map_or("Computed property", |field| field.name.as_str()),
                "value":value.as_deref().unwrap_or(fallback),
                "range":index.range(setter.source).ok(),
                "field":setter.field.as_ref().map(|field| field_report(field, has_choices && value.is_some())).unwrap_or_else(|| json!({"kind":"text","options":[],"resources":false})),
                "resource":resource, "expression":value.is_none(),
                "editable":value.is_some() && resource.is_none(),"can_remove":value.is_some(),"object":false})
        }).collect::<Vec<_>>();
        let mut available = projection
            .style_fields()
            .into_iter()
            .filter(|field| !style.has_setter(field))
            .collect::<Vec<_>>();
        available.sort_by(|a, b| {
            (&a.name, a.owner_name(), &a.getter).cmp(&(&b.name, b.owner_name(), &b.getter))
        });
        available.dedup_by(|a, b| a.name == b.name && a.owner_name() == b.owner_name());
        let available = available.into_iter().map(|property|
            json!({"property":property.name,"getter":property.getter,"field":field_report(property, has_choices)})).collect::<Vec<_>>();
        let resources = style.resources.iter().map(|resource| palette::resource_report(&index, modules, resource)).collect::<Vec<_>>();
        let choices = choices.into_iter().map(|family| palette::choice_report(&index, modules, family)).collect::<Vec<_>>();
        let comparison = palette::comparison(&index, modules, &families);
        return Ok(json!({"node":node,"kind":"Style","name":style.selector,
            "range":index.range(style.selector_span).ok(),
            "identity":{"kind":"selector","value":style.selector,"editable":true},
            "nested_allowed":true,"can_remove":style.can_remove(),"setters":setters,"available":available,
            "color_resources":resources,"resource_choices":choices,"resource_comparison":comparison.rows}));
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

fn field_report(field: &RsxStyleField, has_choices: bool) -> Value {
    let options = if field.kind() == "boolean" {
        vec!["true".into(), "false".into()]
    } else {
        field.options().to_vec()
    };
    json!({"kind":field.kind(),"type_name":field.type_name,"options":options,
        "resources":has_choices && field.accepts_color_resource(),"owner":field.owner_name()})
}

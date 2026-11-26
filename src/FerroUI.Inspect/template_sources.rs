//! Read the templates held by a live control without instantiating them.
use crate::source::{rust_location, xaml_location};
use ferroui_base::{diagnostics::RsxSourceInfo, FerroObject};
use ferroui_controls::{
    diagnostics::{column_templates, data_template_slots},
    primitives::TemplatedControl,
    templates::IDataTemplate,
};
use ferroui_markup_xaml::diagnostics::XamlSourceInfo;
use serde_json::{json, Value};
use std::rc::Rc;

const MAX_TEMPLATES: usize = 2_000;

pub(super) fn list(object: &FerroObject) -> Result<Value, String> {
    let owner = RsxSourceInfo::get(object);
    let mut templates = Vec::new();
    if let Some(template) = object
        .downcast_ref::<TemplatedControl>()
        .and_then(TemplatedControl::template)
    {
        let source =
            XamlSourceInfo::for_control_template(&template).and_then(checked_xaml_location);
        templates.push(json!({"member":"Template", "source":source}));
    }
    templates.extend(
        data_template_slots(object).filter_map(|(member, template)| {
            let template = template?;
            Some(json!({"member":member, "source":data_source(&template, owner)}))
        }),
    );
    let columns = column_templates(object, MAX_TEMPLATES - templates.len())
        .ok_or("The template catalog exceeds the inspection limit")?;
    templates.extend(columns.into_iter().filter_map(|column| {
        let template = column.template?;
        let header = column
            .header
            .as_ref()
            .and_then(|value| {
                value
                    .downcast_ref::<String>()
                    .map(String::as_str)
                    .or_else(|| value.downcast_ref::<&'static str>().copied())
            })
            .filter(|text| !text.trim().is_empty())
            .map(|text| text.chars().take(256).collect::<String>());
        Some(json!({
            "member":column.member.name(),
            "column":{"index":column.index,"header":header,"expander_inner":column.expander_inner},
            "source":data_source(&template, owner),
        }))
    }));
    Ok(Value::Array(templates))
}

fn data_source(template: &Rc<dyn IDataTemplate>, owner: Option<RsxSourceInfo>) -> Option<Value> {
    XamlSourceInfo::for_data_template(template).and_then(checked_xaml_location).or_else(|| {
        let source = template.rust_editor_source()?;
        source.rendered_body.and_then(|body| rust_location(body, true))
            .filter(|location| location.get("revision").is_some())
            .or_else(|| {
                let proof = source.verified_constructor(owner)?;
                let (line, column) = proof.compiled.element;
                Some(json!({
                    "uri":proof.source_uri()?.as_str(), "language":"rust",
                    "line":line, "column":column, "columnEncoding":"unicodeScalar",
                    "revision":format!("{:016x}", proof.compiled.revision?),
                }))
            })
    })
}

fn checked_xaml_location(info: XamlSourceInfo) -> Option<Value> {
    info.source_revision()?;
    xaml_location(info)
}

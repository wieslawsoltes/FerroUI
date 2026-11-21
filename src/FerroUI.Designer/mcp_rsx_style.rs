use super::{ensure_revision, schema as s, state};
use crate::{rust_style_report, DesignerSession};
use ferroui_designer_rsx::{RsxProjection, RsxStyleEdit, RsxStylePlan};
use serde_json::{json, Value};

pub const RSX_STYLES: &str = "designer_rsx_styles";
pub const RSX_STYLE_EDIT: &str = "designer_rsx_style_edit";

pub(super) fn handles(name: &str) -> bool {
    matches!(name, RSX_STYLES | RSX_STYLE_EDIT)
}

pub(super) fn tools() -> Vec<Value> {
    let offset = json!({"type":"integer","minimum":0});
    vec![
        s::tool(RSX_STYLES,
            "Read mapped Rust styles in the selected RSX invocation. Omit node for the style catalog and owner ordinals. Pass a style node for typed setters, authored color layers, Light, Dark and High contrast comparison, compatible resource choices, and source ranges.",
            s::object(json!({"node":offset}), &[]), false),
        s::actions(RSX_STYLE_EDIT,
            "Update one inline Rust style in the accepted invocation. Use the current style and setter offsets. Computed values have source navigation. Every edit has one Undo.",
            vec![
                s::action("create", json!({"owner":offset,"selector":s::string()}), &["owner","selector"], true),
                s::action("create_nested", json!({"style":offset,"selector":s::string()}), &["style","selector"], true),
                s::action("selector", json!({"style":offset,"selector":s::string()}), &["style","selector"], true),
                s::action("setter", json!({"style":offset,"setter":offset,"value":s::nullable(s::string())}), &["style","setter","value"], true),
                s::action("add_setter", json!({"style":offset,"getter":s::string(),"value":s::string()}), &["style","getter","value"], true),
                s::action("set_resource", json!({"style":offset,"key":s::string(),"setter":offset,"getter":s::string()}), &["style","key"], true),
                s::action("add_color_resource", json!({"style":offset,"key":s::string(),"value":s::string(),
                    "layer":{"type":"string","enum":["shared","default","light","dark","high_contrast"]}}), &["style","key","value"], true),
                s::action("color_resource", json!({"style":offset,"resource":offset,"value":s::string(),
                    "mode":{"type":"string","enum":["literal","declaration","local"]}}), &["style","resource","value"], true),
                s::action("remove", json!({"style":offset}), &["style"], true),
            ], true),
    ]
}

pub(super) fn call(
    session: &mut DesignerSession,
    tool: &str,
    args: &Value,
) -> Result<Value, String> {
    if tool == RSX_STYLES {
        return rust_style_report(
            session.authored_source(),
            &session.rust_style_projection()?,
            args["node"]
                .as_u64()
                .and_then(|node| usize::try_from(node).ok()),
        );
    }
    ensure_revision(session, args)?;
    session.edit_rust_style(&decode_edit(args)?)?;
    Ok(state(session))
}

pub fn rust_style_source_plan(
    session: &DesignerSession,
    projection: &RsxProjection<'_>,
    args: &Value,
) -> Result<RsxStylePlan, String> {
    ensure_revision(session, args)?;
    projection.plan_style_source(&decode_edit(args)?)
}

pub(super) fn decode_edit(args: &Value) -> Result<RsxStyleEdit, String> {
    let mut args = args
        .as_object()
        .ok_or("arguments must be an object")?
        .clone();
    args.remove("expected_revision");
    serde_json::from_value(Value::Object(args)).map_err(|error| error.to_string())
}

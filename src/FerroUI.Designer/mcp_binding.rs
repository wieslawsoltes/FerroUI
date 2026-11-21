//! The binding dialog and agent share the native source transaction.
use super::{decode, node_id, schema as s, state, text};
use crate::editors::{PropertyBindingOption, BINDING_MODES, BINDING_TRIGGERS};
use crate::DesignerSession;
use serde_json::{json, Value};

pub(super) fn handles(name: &str) -> bool {
    matches!(name, "designer_binding" | "designer_binding_edit")
}

fn value_schema() -> Value {
    let variants = ["text", "expression"].map(|kind| {
        s::object(
            json!({"kind":{"const":kind},"value":s::string()}),
            &["kind", "value"],
        )
    });
    json!({"oneOf":variants})
}

fn source_schema() -> Value {
    json!({"oneOf":[
        s::object(json!({"kind":{"const":"dataContext"}}), &["kind"]),
        s::object(json!({"kind":{"const":"element"},"name":s::string()}), &["kind", "name"]),
        s::object(json!({"kind":{"const":"selfValue"}}), &["kind"]),
        s::object(json!({"kind":{"const":"templatedParent"}}), &["kind"]),
        s::object(json!({"kind":{"const":"ancestor"},"typeName":s::string(),
            "level":{"type":"integer","minimum":1,"maximum":i32::MAX},"logicalTree":{"type":"boolean"}}),
            &["kind", "typeName", "level", "logicalTree"]),
        s::object(json!({"kind":{"const":"explicit"},"value":value_schema()}), &["kind", "value"])
    ]})
}

pub(super) fn tools() -> Vec<Value> {
    let field = |name, value| {
        s::object(
            json!({"field":{"const":name},"value":value}),
            &["field", "value"],
        )
    };
    let options = PropertyBindingOption::ALL.map(PropertyBindingOption::name);
    let mut changes = s::array(
        json!({"oneOf":[
            field("path", s::string()),
            field("mode", json!({"enum":BINDING_MODES})),
            field("trigger", json!({"enum":BINDING_TRIGGERS})),
            field("delay", s::nullable(json!({"type":"integer","minimum":0,"maximum":i32::MAX}))),
            field("source", source_schema()),
            s::object(json!({"field":{"const":"option"},"name":{"enum":options},"value":s::nullable(value_schema())}),
                &["field", "name", "value"])
        ]}),
        24,
    );
    changes["minItems"] = json!(1);
    vec![
        s::tool(
            "designer_binding",
            "Read a XAML attribute binding, its structured fields, and available element names in the current namescope.",
            s::object(json!({"node":s::id(),"property":s::string()}), &["node", "property"]),
            false,
        ),
        s::tool(
            "designer_binding_edit",
            "Apply changed binding fields in one undoable edit. Keep untouched arguments and nested expressions. Pass replace=true to replace a literal or unsupported expression with a new binding.",
            s::object(
                json!({"node":s::id(),"property":s::string(),"changes":changes,"replace":{"type":"boolean"},
                    "expected_revision":s::revision()}),
                &["node", "property", "changes", "expected_revision"],
            ),
            true,
        ),
    ]
}

pub(super) fn call(
    session: &mut DesignerSession,
    name: &str,
    args: &Value,
) -> Result<Value, String> {
    let node = node_id(args, "node")?;
    let property = text(args, "property")?;
    if name == "designer_binding" {
        return serde_json::to_value(session.binding_model(node, &property)?)
            .map_err(|error| error.to_string());
    }
    session.edit_binding(
        node,
        &property,
        decode(args, "changes")?,
        args["replace"] == true,
    )?;
    Ok(state(session))
}

#[cfg(test)]
#[path = "mcp_binding_tests.rs"]
mod tests;

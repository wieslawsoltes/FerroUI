//! Session-backed component, resource and native definition-scope automation.
use super::schema as s;
use super::{decode, integer, node_id, state, text};
use crate::{DefinitionPreviewOptions, DesignerSession};
use ferroui_designer_tools::{ComponentAction, ComponentEntry, component_fragment};
use serde_json::{Value, json};

pub(super) fn handles(name: &str) -> bool {
    matches!(
        name,
        "designer_components"
            | "designer_component_resources"
            | "designer_component_references"
            | "designer_component"
            | "designer_definition_scope"
            | "designer_definition"
    )
}

pub(super) fn tools() -> Vec<Value> {
    let mut targets = s::array(s::id(), 4096);
    targets["minItems"] = json!(1);
    let options = s::object(
        json!({
            "target_type":s::nullable(s::string()),
            "sample_content":{"type":"string","maxLength":65536},
            "width":{"type":"number","minimum":1,"maximum":8192},
            "height":{"type":"number","minimum":1,"maximum":8192}
        }),
        &[],
    );
    let mut component = s::actions(
        "designer_component",
        "Author reusable components and resources through one atomic, undoable session transaction. Rename updates resolved references; removal refuses resources still in use; applying resources validates visibility for every target. Null property/setter values reset authored values. XAML object sources retain namespace scope. copy_definition_for_instance creates and applies a private template/theme copy in the same transaction. Every action checks expected_revision.",
        vec![
            s::action(
                "create_resource",
                json!({"owner":s::id(),"key":s::string(),"source":s::string()}),
                &["owner", "key", "source"],
                true,
            ),
            s::action(
                "duplicate_resource",
                json!({"resource":s::id(),"key":s::string()}),
                &["resource", "key"],
                true,
            ),
            s::action(
                "rename_resource",
                json!({"resource":s::id(),"key":s::string()}),
                &["resource", "key"],
                true,
            ),
            s::action(
                "remove_resource",
                json!({"resource":s::id()}),
                &["resource"],
                true,
            ),
            s::action(
                "apply_resource",
                json!({"resource":s::id(),"targets":targets,"property":s::string(),"dynamic":{"type":"boolean"}}),
                &["resource", "targets", "property"],
                true,
            ),
            s::action(
                "set_property",
                json!({"node":s::id(),"property":s::string(),"value":s::nullable(s::string())}),
                &["node", "property", "value"],
                true,
            ),
            s::action(
                "set_object_property",
                json!({"node":s::id(),"property":s::string(),"source":s::nullable(s::string())}),
                &["node", "property", "source"],
                true,
            ),
            s::action(
                "set_setter",
                json!({"owner":s::id(),"property":s::string(),"value":s::nullable(s::string())}),
                &["owner", "property", "value"],
                true,
            ),
            s::action(
                "set_setter_object",
                json!({"owner":s::id(),"property":s::string(),"source":s::nullable(s::string())}),
                &["owner", "property", "source"],
                true,
            ),
            s::action(
                "set_resource_value",
                json!({"resource":s::id(),"value":s::string()}),
                &["resource", "value"],
                true,
            ),
            s::action(
                "replace_definition",
                json!({"node":s::id(),"source":s::string()}),
                &["node", "source"],
                true,
            ),
            s::action(
                "create_theme",
                json!({"owner":s::id(),"key":s::string(),"target_type":s::string(),"based_on":s::nullable(s::id())}),
                &["owner", "key", "target_type"],
                true,
            ),
            s::action(
                "create_template",
                json!({"owner":s::id(),"key":s::string(),"target_type":s::nullable(s::string()),"content":s::string(),"data_template":{"type":"boolean"}}),
                &["owner", "key", "content"],
                true,
            ),
            s::action(
                "copy_template",
                json!({"template":s::id(),"owner":s::id(),"key":s::string()}),
                &["template", "owner", "key"],
                true,
            ),
            s::action(
                "extract_property",
                json!({"node":s::id(),"property":s::string(),"owner":s::id(),"key":s::string()}),
                &["node", "property", "owner", "key"],
                true,
            ),
            s::action(
                "copy_definition_for_instance",
                json!({"definition":s::id(),"target":s::id(),"property":s::string(),"key":s::string()}),
                &["definition", "target", "property", "key"],
                true,
            ),
        ],
        true,
    );
    // The outer schema is the union of shared argument shapes. Action branches still
    // require strings for creation/replacement and allow null only for reset actions.
    component["inputSchema"]["properties"]["source"] = s::nullable(s::string());
    component["inputSchema"]["properties"]["value"] = s::nullable(s::string());
    vec![
        s::tool(
            "designer_components",
            "Read the authored component/resource catalog, keys, lexical owners, target types and reference counts. Set include_source to include self-contained XAML fragments with inherited namespace declarations. Nodes are document-local.",
            s::object(json!({"include_source":{"type":"boolean"}}), &[]),
            false,
        ),
        s::tool(
            "designer_component_resources",
            "Read resources visible to one authored target using its accepted native theme context. Unknown runtime context uses conservative lexical resolution. Set include_source to include namespace-complete resource fragments.",
            s::object(
                json!({"target":s::id(),"include_source":{"type":"boolean"}}),
                &["target"],
            ),
            false,
        ),
        s::tool(
            "designer_component_references",
            "Read authored references to a resource, including possible uses across theme variants. This is the same conservative dependency analysis used by rename and deletion.",
            s::object(json!({"resource":s::id()}), &["resource"]),
            false,
        ),
        component,
        s::tool(
            "designer_definition_scope",
            "Read the active native definition preview, scope epoch, preview options and document state. A null definition means the main document preview is active.",
            s::object(json!({}), &[]),
            false,
        ),
        s::actions(
            "designer_definition",
            "Enter a native component/template/theme/resource preview or return to the document. Navigation uses original source identities, restores the instance selection on exit and changes no authored source or history. Re-enter with options to change the sample host. Navigation requires allow_property_writes and expected_revision but is available in viewer sessions; optional expected_scope_epoch rejects stale preview navigation.",
            vec![
                s::action(
                    "enter",
                    json!({"node":s::id(),"options":options,"expected_scope_epoch":s::revision()}),
                    &["node"],
                    true,
                ),
                s::action(
                    "exit",
                    json!({"expected_scope_epoch":s::revision()}),
                    &[],
                    true,
                ),
            ],
            true,
        ),
    ]
}

pub(super) fn call(
    session: &mut DesignerSession,
    name: &str,
    args: &Value,
) -> Result<Value, String> {
    match name {
        "designer_components" => Ok(json!({
            "document":state(session),
            "scope":scope_state(session),
            "components":entries(session,session.component_catalog()?.entries,args["include_source"] == true)?
        })),
        "designer_component_resources" => {
            let target = node_id(args, "target")?;
            Ok(json!({
                "document":state(session),
                "target":target,
                "context":session.component_resource_context(target),
                "resources":entries(session,session.visible_component_resources(target)?,args["include_source"] == true)?
            }))
        }
        "designer_component_references" => {
            let resource = node_id(args, "resource")?;
            Ok(json!({
                "document":state(session),
                "resource":resource,
                "references":session.component_resource_references(resource)?
            }))
        }
        "designer_component" => {
            session
                .ensure_writable()
                .map_err(|error| error.to_string())?;
            let created = if args["action"] == "copy_definition_for_instance" {
                Some(session.copy_definition_for_instance(
                    node_id(args, "definition")?,
                    node_id(args, "target")?,
                    &text(args, "property")?,
                    &text(args, "key")?,
                )?)
            } else {
                session.edit_component(component_action(args)?)?;
                None
            };
            Ok(json!({
                "document":state(session),
                "scope":scope_state(session),
                "created":created,
                "components":session.component_catalog()?.entries
            }))
        }
        "designer_definition_scope" => {
            Ok(json!({"document":state(session),"scope":scope_state(session)}))
        }
        "designer_definition" => {
            if args.get("expected_scope_epoch").is_some() {
                let expected = integer(args, "expected_scope_epoch")?;
                let actual = session.definition_scope_epoch();
                if expected != actual {
                    return Err(format!(
                        "Definition scope changed: expected epoch {expected}, actual {actual}"
                    ));
                }
            }
            match text(args, "action")?.as_str() {
                "enter" => session.enter_definition_scope_with_options(
                    node_id(args, "node")?,
                    preview_options(args)?,
                )?,
                "exit" => session.exit_definition_scope()?,
                _ => return Err("Unknown definition-scope action".into()),
            }
            Ok(json!({"document":state(session),"scope":scope_state(session)}))
        }
        _ => Err("Unknown component tool".into()),
    }
}

fn entries(
    session: &DesignerSession,
    entries: Vec<ComponentEntry>,
    include_source: bool,
) -> Result<Vec<Value>, String> {
    entries
        .into_iter()
        .map(|entry| {
            let mut value = serde_json::to_value(&entry).map_err(|error| error.to_string())?;
            if include_source {
                value["source"] = json!(
                    component_fragment(session.document(), entry.node)
                        .map_err(|error| error.to_string())?
                );
            }
            Ok(value)
        })
        .collect()
}

fn scope_state(session: &DesignerSession) -> Value {
    let definition = session
        .definition_scope()
        .map(|scope| json!({"node":scope.node,"title":scope.title,"kind":scope.kind}));
    let options = session.definition_preview_options();
    json!({
        "epoch":session.definition_scope_epoch(),
        "definition":definition,
        "preview_options":{
            "target_type":options.target_type,
            "sample_content":options.sample_content,
            "width":options.width,
            "height":options.height
        }
    })
}

fn preview_options(args: &Value) -> Result<DefinitionPreviewOptions, String> {
    let mut options = DefinitionPreviewOptions::default();
    if let Some(value) = args.get("options") {
        if value.get("target_type").is_some() {
            options.target_type = decode(value, "target_type")?;
        }
        if value.get("sample_content").is_some() {
            options.sample_content = text(value, "sample_content")?;
        }
        if value.get("width").is_some() {
            options.width = super::number(value, "width")?;
        }
        if value.get("height").is_some() {
            options.height = super::number(value, "height")?;
        }
    }
    options.validate()?;
    Ok(options)
}

fn component_action(args: &Value) -> Result<ComponentAction, String> {
    Ok(match text(args, "action")?.as_str() {
        "create_resource" => ComponentAction::CreateResource {
            owner: node_id(args, "owner")?,
            key: text(args, "key")?,
            source: text(args, "source")?,
        },
        "duplicate_resource" => ComponentAction::DuplicateResource {
            resource: node_id(args, "resource")?,
            key: text(args, "key")?,
        },
        "rename_resource" => ComponentAction::RenameResource {
            resource: node_id(args, "resource")?,
            key: text(args, "key")?,
        },
        "remove_resource" => ComponentAction::RemoveResource {
            resource: node_id(args, "resource")?,
        },
        "apply_resource" => ComponentAction::ApplyResource {
            resource: node_id(args, "resource")?,
            targets: decode(args, "targets")?,
            property: text(args, "property")?,
            dynamic: args["dynamic"].as_bool().unwrap_or(false),
        },
        "set_property" => ComponentAction::SetProperty {
            node: node_id(args, "node")?,
            property: text(args, "property")?,
            value: decode(args, "value")?,
        },
        "set_object_property" => ComponentAction::SetObjectProperty {
            node: node_id(args, "node")?,
            property: text(args, "property")?,
            source: decode(args, "source")?,
        },
        "set_setter" => ComponentAction::SetSetter {
            owner: node_id(args, "owner")?,
            property: text(args, "property")?,
            value: decode(args, "value")?,
        },
        "set_setter_object" => ComponentAction::SetSetterObject {
            owner: node_id(args, "owner")?,
            property: text(args, "property")?,
            source: decode(args, "source")?,
        },
        "set_resource_value" => ComponentAction::SetResourceValue {
            resource: node_id(args, "resource")?,
            value: text(args, "value")?,
        },
        "replace_definition" => ComponentAction::ReplaceDefinition {
            node: node_id(args, "node")?,
            source: text(args, "source")?,
        },
        "create_theme" => ComponentAction::CreateTheme {
            owner: node_id(args, "owner")?,
            key: text(args, "key")?,
            target_type: text(args, "target_type")?,
            based_on: decode(args, "based_on")?,
        },
        "create_template" => ComponentAction::CreateTemplate {
            owner: node_id(args, "owner")?,
            key: text(args, "key")?,
            target_type: decode(args, "target_type")?,
            content: text(args, "content")?,
            data_template: args["data_template"].as_bool().unwrap_or(false),
        },
        "copy_template" => ComponentAction::CopyTemplate {
            template: node_id(args, "template")?,
            owner: node_id(args, "owner")?,
            key: text(args, "key")?,
        },
        "extract_property" => ComponentAction::ExtractProperty {
            node: node_id(args, "node")?,
            property: text(args, "property")?,
            owner: node_id(args, "owner")?,
            key: text(args, "key")?,
        },
        _ => return Err("Unknown component action".into()),
    })
}

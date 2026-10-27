//! Automation uses the same accepted models, semantic commands and local view state as UI.
use super::schema as s;
use super::{integer, node_id, state, text};
use crate::{DesignTimeEdit, DesignerSession, FocusOrderCatalog, FocusOrderEdit, LayerCommand};
use ferroui_base::input::KeyboardNavigationMode;
use serde_json::{Value, json};
use std::hash::{Hash, Hasher};

pub(super) fn handles(name: &str) -> bool {
    matches!(
        name,
        "designer_design_data"
            | "designer_design_data_edit"
            | "designer_focus_order"
            | "designer_focus_edit"
            | "designer_layers"
            | "designer_layer"
    )
}

pub(super) fn tools() -> Vec<Value> {
    let mut nodes = s::array(s::id(), 4096);
    nodes["minItems"] = json!(1);
    let scope_action = |name: &str, mut properties: Value, required: &[&str]| {
        properties["expected_model"] = s::string();
        let mut required = required.to_vec();
        required.push("expected_model");
        s::action(name, properties, &required, true)
    };
    let layer_action = |name: &str, mut properties: Value, required: &[&str]| {
        properties["expected_layer_epoch"] = s::revision();
        properties["expected_scope_epoch"] = s::revision();
        let mut required = required.to_vec();
        required.push("expected_layer_epoch");
        required.push("expected_scope_epoch");
        s::action(name, properties, &required, true)
    };
    vec![
        s::tool(
            "designer_design_data",
            "Read design-time overrides for an authored node, preview enablement and design size. Omitted node uses the first selected node or document root. Source and preview are unchanged.",
            s::object(json!({"node":s::id()}), &[]),
            false,
        ),
        s::actions(
            "designer_design_data_edit",
            "Author design-namespace values without replacing application values or bindings. Scalar/object reset uses null; sample_items writes deterministic d:SampleData. Source commands use one undo transaction. enabled changes preview only and is available to collaboration viewers with tool write permission.",
            vec![
                s::action(
                    "set_value",
                    json!({"node":s::id(),"property":s::string(),"value":s::nullable(s::string())}),
                    &["node", "property", "value"],
                    true,
                ),
                s::action(
                    "set_object",
                    json!({"node":s::id(),"property":s::string(),"source":s::nullable(s::string())}),
                    &["node", "property", "source"],
                    true,
                ),
                s::action(
                    "sample_items",
                    json!({"node":s::id(),"count":s::nullable(json!({"type":"integer","minimum":0,"maximum":200}))}),
                    &["node", "count"],
                    true,
                ),
                s::action(
                    "enabled",
                    json!({"enabled":{"type":"boolean"}}),
                    &["enabled"],
                    true,
                ),
            ],
            true,
        ),
        s::tool(
            "designer_focus_order",
            "Read accepted native keyboard-navigation groups, authored/generated entries, label targets and model token. Runtime group IDs and model are ephemeral; obtain a new model after any preview change.",
            s::object(json!({}), &[]),
            false,
        ),
        s::actions(
            "designer_focus_edit",
            "Author keyboard navigation through one atomic source transaction. expected_model must match designer_focus_order. Ordering is a complete permutation within one native navigation group; generated controls cannot become authored targets. Label targets stay in the real namescope.",
            vec![
                scope_action(
                    "order",
                    json!({"scope":s::id(),"nodes":nodes}),
                    &["scope", "nodes"],
                ),
                scope_action("reset_order", json!({"scope":s::id()}), &["scope"]),
                scope_action(
                    "tab_stop",
                    json!({"node":s::id(),"value":{"type":"boolean"}}),
                    &["node", "value"],
                ),
                scope_action(
                    "focusable",
                    json!({"node":s::id(),"value":{"type":"boolean"}}),
                    &["node", "value"],
                ),
                scope_action(
                    "navigation",
                    json!({"node":s::id(),"mode":{"enum":["Continue","Local","Cycle","Contained","Once","None"]}}),
                    &["node", "mode"],
                ),
                scope_action(
                    "label_target",
                    json!({"label":s::id(),"target":s::nullable(s::id())}),
                    &["label", "target"],
                ),
            ],
            true,
        ),
        s::tool(
            "designer_layers",
            "Read local editor lock/hide/isolation state and its epoch. These flags do not change application visibility, layout, source history or collaboration data.",
            s::object(json!({}), &[]),
            false,
        ),
        s::actions(
            "designer_layer",
            "Change local layer presentation or hierarchy selection. Requires layer_epoch and scope_epoch from designer_layers and current document revision; view-only commands remain available to collaboration viewers with tool write permission. Hidden layers retain layout and authored property values.",
            vec![
                layer_action(
                    "set_locked",
                    json!({"nodes":nodes,"locked":{"type":"boolean"}}),
                    &["nodes", "locked"],
                ),
                layer_action(
                    "set_hidden",
                    json!({"nodes":nodes,"hidden":{"type":"boolean"}}),
                    &["nodes", "hidden"],
                ),
                layer_action("isolate", json!({"nodes":nodes}), &["nodes"]),
                layer_action("exit_isolation", json!({}), &[]),
                layer_action("reset", json!({}), &[]),
                layer_action("select_parent", json!({}), &[]),
                layer_action("select_children", json!({}), &[]),
                layer_action("select_next", json!({}), &[]),
                layer_action("select_previous", json!({}), &[]),
            ],
            true,
        ),
    ]
}

fn ids(args: &Value) -> Vec<u64> {
    args["nodes"]
        .as_array()
        .expect("validated nodes")
        .iter()
        .map(|node| node.as_u64().expect("validated node"))
        .collect()
}

fn focus_catalog(catalog: &FocusOrderCatalog) -> Value {
    json!({
        "scopes":catalog.scopes.iter().map(|scope| json!({
            "id":scope.id,"node":scope.node,"parent":scope.parent,"label":scope.label,
            "mode":crate::navigation_mode_name(scope.mode),"blocked_by_ancestor":scope.blocked_by_ancestor,"can_reorder":scope.can_reorder(),
            "items":scope.items.iter().map(|item| json!({"runtime_id":item.runtime_id,"node":item.node,"label":item.label,"tab_index":item.tab_index,"tree_order":item.tree_order,"focusable":item.focusable,"is_tab_stop":item.is_tab_stop,"visible":item.visible,"enabled":item.enabled,"group":item.group,"active":item.is_active()})).collect::<Vec<_>>()
        })).collect::<Vec<_>>(),
        "labels":catalog.labels.iter().map(|label| json!({"node":label.node,"label":label.label,"target":label.current_target,"candidates":label.candidates.iter().map(|target|json!({"node":target.node,"label":target.label})).collect::<Vec<_>>()})).collect::<Vec<_>>()
    })
}

fn model_token(session: &DesignerSession, catalog: &Value) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    session.document_uri().hash(&mut hasher);
    session.document().revision().hash(&mut hasher);
    session.definition_scope_epoch().hash(&mut hasher);
    catalog.to_string().hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn layers(session: &DesignerSession) -> Value {
    json!({"revision":session.document().revision(),"layer_epoch":session.layer_view_epoch(),"scope_epoch":session.definition_scope_epoch(),"isolation":session.layer_isolation(),"selection":session.selection(),"layers":session.document().ast().elements().map(|node| {
        let flags=session.layer_state(node.id);
        json!({"node":node.id,"name":node.xaml_name(),"type":node.name,"locked":flags.locked,"hidden":flags.hidden,"inherited_locked":flags.inherited_locked,"inherited_hidden":flags.inherited_hidden,"outside_isolation":flags.outside_isolation,"canvas_selectable":session.can_canvas_select(node.id)})
    }).collect::<Vec<_>>()})
}

pub(super) fn call(
    session: &mut DesignerSession,
    name: &str,
    args: &Value,
) -> Result<Value, String> {
    match name {
        "designer_design_data" => {
            let node = args["node"]
                .as_u64()
                .or_else(|| session.selection().first().copied())
                .unwrap_or(session.document().ast().root());
            let properties = session.design_time_properties(node)?;
            let (width, height) = session.design_preview_size();
            Ok(
                json!({"revision":session.document().revision(),"scope_epoch":session.definition_scope_epoch(),"enabled":session.design_time_enabled(),"node":node,"design_width":width,"design_height":height,"properties":properties.iter().map(|property|json!({"name":property.name,"value":property.value,"object":property.object})).collect::<Vec<_>>()}),
            )
        }
        "designer_design_data_edit" => {
            let action = text(args, "action")?;
            let command = match action.as_str() {
                "set_value" => Some(DesignTimeEdit::SetProperty {
                    node: node_id(args, "node")?,
                    property: text(args, "property")?,
                    value: args["value"].as_str().map(str::to_owned),
                }),
                "set_object" => Some(DesignTimeEdit::SetObject {
                    node: node_id(args, "node")?,
                    property: text(args, "property")?,
                    source: args["source"].as_str().map(str::to_owned),
                }),
                "sample_items" => Some(DesignTimeEdit::SampleItems {
                    node: node_id(args, "node")?,
                    count: args["count"].as_u64().map(|value| value as usize),
                }),
                "enabled" => {
                    session.set_design_time_enabled(args["enabled"].as_bool().unwrap())?;
                    None
                }
                _ => return Err("Unknown design-data action".into()),
            };
            if let Some(command) = command {
                session.apply_design_time_edits(&[command])?;
            }
            Ok(state(session))
        }
        "designer_focus_order" => {
            let catalog = focus_catalog(&session.focus_order_catalog()?);
            Ok(
                json!({"revision":session.document().revision(),"scope_epoch":session.definition_scope_epoch(),"model":model_token(session,&catalog),"catalog":catalog}),
            )
        }
        "designer_focus_edit" => {
            let snapshot = session.focus_order_snapshot()?;
            if text(args, "expected_model")?
                != model_token(session, &focus_catalog(&snapshot.catalog))
            {
                return Err(
                    "The native focus model changed; read designer_focus_order again".into(),
                );
            }
            let action = text(args, "action")?;
            let command = match action.as_str() {
                "order" => FocusOrderEdit::SetOrder {
                    scope: node_id(args, "scope")?,
                    order: ids(args),
                },
                "reset_order" => FocusOrderEdit::ResetOrder {
                    scope: node_id(args, "scope")?,
                },
                "tab_stop" => FocusOrderEdit::SetTabStop {
                    node: node_id(args, "node")?,
                    value: args["value"].as_bool().unwrap(),
                },
                "focusable" => FocusOrderEdit::SetFocusable {
                    node: node_id(args, "node")?,
                    value: args["value"].as_bool().unwrap(),
                },
                "navigation" => FocusOrderEdit::SetNavigation {
                    node: node_id(args, "node")?,
                    mode: match args["mode"].as_str().unwrap() {
                        "Continue" => KeyboardNavigationMode::Continue,
                        "Local" => KeyboardNavigationMode::Local,
                        "Cycle" => KeyboardNavigationMode::Cycle,
                        "Contained" => KeyboardNavigationMode::Contained,
                        "Once" => KeyboardNavigationMode::Once,
                        "None" => KeyboardNavigationMode::None,
                        _ => return Err("Unknown navigation mode".into()),
                    },
                },
                "label_target" => FocusOrderEdit::SetLabelTarget {
                    label: node_id(args, "label")?,
                    target: args["target"].as_u64(),
                },
                _ => return Err("Unknown focus action".into()),
            };
            session.commit_focus_order(&snapshot, &[command])?;
            Ok(state(session))
        }
        "designer_layers" => Ok(layers(session)),
        "designer_layer" => {
            if integer(args, "expected_layer_epoch")? != session.layer_view_epoch()
                || integer(args, "expected_scope_epoch")? != session.definition_scope_epoch()
            {
                return Err(
                    "Layer presentation or preview scope changed; read designer_layers again"
                        .into(),
                );
            }
            match text(args, "action")?.as_str() {
                "set_locked" => {
                    session.set_layers_locked(&ids(args), args["locked"].as_bool().unwrap())?
                }
                "set_hidden" => {
                    session.set_layers_hidden(&ids(args), args["hidden"].as_bool().unwrap())?
                }
                "isolate" => session.isolate_layers(&ids(args))?,
                "exit_isolation" => session.clear_layer_isolation(),
                "reset" => session.reset_layer_view(),
                "select_parent" => session.apply_layer_command(LayerCommand::SelectParent)?,
                "select_children" => session.apply_layer_command(LayerCommand::SelectChildren)?,
                "select_next" => session.apply_layer_command(LayerCommand::SelectNextSibling)?,
                "select_previous" => {
                    session.apply_layer_command(LayerCommand::SelectPreviousSibling)?
                }
                _ => return Err("Unknown layer action".into()),
            }
            Ok(layers(session))
        }
        _ => Err("Unknown designer workflow tool".into()),
    }
}

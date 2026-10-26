//! Model-backed animation, state, behavior and typed-property automation.
use super::schema as s;
use super::{decode, node_id, number, state, text, Permissions};
use crate::DesignerSession;
use ferroui_designer_animation::{AnimationValue, BehaviorModel, Timeline, VisualStateGroup};
use ferroui_designer_tools::PropertyKind;
use serde_json::{json, Value};
use std::collections::BTreeSet;

pub(super) fn handles(name: &str) -> bool {
    matches!(
        name,
        "designer_programs"
            | "designer_program"
            | "designer_states"
            | "designer_state"
            | "designer_state_actions"
            | "designer_behaviors"
            | "designer_behavior"
            | "designer_properties"
            | "designer_property"
    )
}
pub(super) fn tools() -> Vec<Value> {
    let ids = || {
        let mut schema = s::array(s::id(), 4096);
        schema["minItems"] = json!(1);
        schema
    };
    vec![
        s::tool("designer_programs","Read independently editable animation programs, persistent source keys, codec diagnostics and typed timelines. Opaque programs remain available as source.",s::object(json!({"include_source":{"type":"boolean"}}), &[]),false),
        s::actions("designer_program","Create/select/rename/duplicate/remove an animation program, replace its source or complete typed model, set authored selectors or preview-only targets, scrub several programs together, or play all programs. IDs refer to the current document; every action checks expected_revision.",vec![
            s::action("create",json!({"name":s::string(),"owner":s::id(),"selector":s::string(),"duration":s::time()}), &["name","owner","selector","duration"],true),
            s::action("select",json!({"program":s::id()}), &["program"],true),
            s::action("rename",json!({"program":s::id(),"name":s::string()}), &["program","name"],true),
            s::action("duplicate",json!({"program":s::id(),"name":s::string()}), &["program","name"],true),
            s::action("remove",json!({"program":s::id()}), &["program"],true),
            s::action("selector",json!({"program":s::id(),"selector":s::string()}), &["program","selector"],true),
            s::action("source",json!({"program":s::id(),"source":s::string()}), &["program","source"],true),
            s::action("update",json!({"program":s::id(),"timeline":s::timeline()}), &["program","timeline"],true),
            s::action("preview_targets",json!({"program":s::id(),"targets":s::nullable(s::array(s::id(),4096))}), &["program","targets"],true),
            s::action("preview",json!({"programs":ids(),"time":s::time()}), &["programs","time"],true),
            s::action("play_all",json!({}), &[],true),
        ],true),
        s::tool("designer_states","Read source-backed visual-state groups, setters and directional transitions, optionally for one owner.",s::object(json!({"owner":s::id()}), &[]),false),
        s::actions("designer_state","Author a validated visual-state group or remove it. Preview/advance/clear change live classes and animation clocks without changing source. Setter values use native XAML text and may contain bindings/resources.",vec![
            s::action("upsert",json!({"model":s::state_group()}), &["model"],true),
            s::action("remove",json!({"owner":s::id(),"group":s::string()}), &["owner","group"],true),
            s::action("preview",json!({"owner":s::id(),"group":s::string(),"state":s::nullable(s::string()),"time":s::time()}), &["owner","group","state"],true),
            s::action("advance_preview",json!({"seconds":s::time()}), &["seconds"],true),
            s::action("clear_preview",json!({}), &[],true),
        ],true),
        s::tool("designer_state_actions","Generate native behavior actions that route to a state, optionally from a particular state. Returns data; it does not execute the actions.",s::object(json!({"owner":s::id(),"group":s::string(),"from":s::nullable(s::string()),"to":s::string()}), &["owner","group","to"]),false),
        s::tool("designer_behaviors","Read typed and opaque behaviors attached to one source element.",s::object(json!({"owner":s::id()}), &["owner"]),false),
        s::actions("designer_behavior","Create/update/remove a native behavior or replace its preserved XAML. Preview executes the actual attached trigger actions and additionally requires allow_input; custom actions may invoke application commands. clear_preview restores authored preview state.",vec![
            s::action("upsert",json!({"model":s::behavior()}), &["model"],true),
            s::action("remove",json!({"behavior":s::id()}), &["behavior"],true),
            s::action("source",json!({"behavior":s::id(),"source":s::string()}), &["behavior","source"],true),
            s::action("preview",json!({"behavior":s::id()}), &["behavior"],true),
            s::action("clear_preview",json!({}), &[],true),
        ],true),
        s::tool("designer_properties","Read host-registered property descriptors and per-property JSON value schemas, including custom enum/numeric constraints and authored values.",s::object(json!({"node":s::id()}), &["node"]),false),
        s::actions("designer_property","Set or reset a property through the editor's current property registry and recording path. Values accept JSON scalars, native XAML text/bindings, or tagged animation numeric, color, geometry, relative-unit and discrete values. Per-target metadata validates the actual type before editing.",vec![
            s::action("set",json!({"nodes":ids(),"property":s::string(),"value":{"anyOf":[s::string(),s::number(),{"type":"boolean"},s::animation_value()]}}), &["nodes","property","value"],true),
            s::action("reset",json!({"nodes":ids(),"property":s::string()}), &["nodes","property"],true),
        ],true),
    ]
}

pub(super) fn call(
    session: &mut DesignerSession,
    name: &str,
    args: &Value,
    permissions: Permissions,
) -> Result<Value, String> {
    match name {
        "designer_programs" => {
            let mut programs = serde_json::to_value(session.animation_programs())
                .map_err(|error| error.to_string())?;
            if args["include_source"] == false {
                for program in programs.as_array_mut().unwrap() {
                    program.as_object_mut().unwrap().remove("source");
                }
            }
            return Ok(
                json!({"document":state(session),"active":session.active_animation_program(),"programs":programs}),
            );
        }
        "designer_program" => {
            let mut created = None;
            match text(args, "action")?.as_str() {
                "create" => {
                    created = Some(session.create_animation_program(
                        &text(args, "name")?,
                        node_id(args, "owner")?,
                        &text(args, "selector")?,
                        number(args, "duration")?,
                    )?)
                }
                "select" => session.select_animation_program(node_id(args, "program")?)?,
                "rename" => session
                    .rename_animation_program(node_id(args, "program")?, &text(args, "name")?)?,
                "duplicate" => {
                    created = Some(session.duplicate_animation_program(
                        node_id(args, "program")?,
                        &text(args, "name")?,
                    )?)
                }
                "remove" => session.remove_animation_program(node_id(args, "program")?)?,
                "selector" => session.set_animation_program_selector(
                    node_id(args, "program")?,
                    &text(args, "selector")?,
                )?,
                "source" => session.set_animation_program_source(
                    node_id(args, "program")?,
                    &text(args, "source")?,
                )?,
                "update" => {
                    let timeline: Timeline = decode(args, "timeline")?;
                    timeline.validate().map_err(|error| error.to_string())?;
                    session.update_animation_program(node_id(args, "program")?, timeline)?;
                }
                "preview_targets" => session.set_animation_program_preview_targets(
                    node_id(args, "program")?,
                    decode(args, "targets")?,
                )?,
                "preview" => {
                    let programs: Vec<u64> = decode(args, "programs")?;
                    distinct(&programs)?;
                    session.preview_animation_programs(&programs, number(args, "time")?)?;
                }
                "play_all" => session.play_all_animation_programs(),
                _ => return Err("Unknown animation program action".into()),
            }
            return Ok(
                json!({"document":state(session),"active":session.active_animation_program(),"created":created,"programs":session.animation_programs()}),
            );
        }
        "designer_states" => {
            return Ok(
                json!({"revision":session.document().revision(),"groups":session.state_groups().into_iter().filter(|group|args["owner"].as_u64().is_none_or(|owner|group.owner==owner)).collect::<Vec<_>>()}),
            )
        }
        "designer_state" => match text(args, "action")?.as_str() {
            "upsert" => {
                let model: VisualStateGroup = decode(args, "model")?;
                model.validate().map_err(|error| error.to_string())?;
                session.upsert_state_group(model)?;
            }
            "remove" => {
                session.remove_state_group(node_id(args, "owner")?, &text(args, "group")?)?
            }
            "preview" => session.preview_state_at(
                node_id(args, "owner")?,
                &text(args, "group")?,
                args["state"].as_str(),
                args["time"].as_f64(),
            )?,
            "advance_preview" => session.advance_state_preview(number(args, "seconds")?)?,
            "clear_preview" => session.clear_interaction_preview(),
            _ => return Err("Unknown visual-state action".into()),
        },
        "designer_state_actions" => {
            return Ok(
                json!({"revision":session.document().revision(),"actions":session.state_group_transition_actions(node_id(args,"owner")?,&text(args,"group")?,args["from"].as_str(),&text(args,"to")?)?}),
            )
        }
        "designer_behaviors" => {
            let owner = node_id(args, "owner")?;
            if !session
                .document()
                .ast()
                .node(owner)
                .is_some_and(|node| node.is_element())
            {
                return Err("Unknown behavior owner".into());
            }
            return Ok(
                json!({"revision":session.document().revision(),"behaviors":session.behaviors(owner)}),
            );
        }
        "designer_behavior" => {
            let mut created = None;
            match text(args, "action")?.as_str() {
                "upsert" => {
                    let model: BehaviorModel = decode(args, "model")?;
                    created = Some(session.upsert_behavior(model)?);
                }
                "remove" => session.remove_behavior(node_id(args, "behavior")?)?,
                "source" => session
                    .set_behavior_source(node_id(args, "behavior")?, &text(args, "source")?)?,
                "preview" => {
                    if !permissions.allow_input {
                        return Err("Executing behavior actions requires allow_input".into());
                    }
                    session.preview_behavior(node_id(args, "behavior")?)?;
                }
                "clear_preview" => session.clear_interaction_preview(),
                _ => return Err("Unknown behavior action".into()),
            }
            return Ok(json!({"document":state(session),"behavior":created}));
        }
        "designer_properties" => {
            let node = node_id(args, "node")?;
            let mut properties = Vec::new();
            for descriptor in session
                .property_tools()
                .descriptors(session.document(), node)
                .map_err(|error| error.to_string())?
            {
                let authored = session
                    .property_tools()
                    .state(session.document(), &[node], &descriptor.name)
                    .map_err(|error| error.to_string())?;
                properties.push(json!({"descriptor":descriptor,"value_schema":property_schema(&descriptor.kind),"value":authored.value,"expression":authored.expression,"authored":authored.authored}));
            }
            return Ok(
                json!({"revision":session.document().revision(),"node":node,"properties":properties}),
            );
        }
        "designer_property" => {
            let nodes: Vec<u64> = decode(args, "nodes")?;
            distinct(&nodes)?;
            let property = text(args, "property")?;
            for node in &nodes {
                let descriptor = session
                    .property_tools()
                    .descriptors(session.document(), *node)
                    .map_err(|error| error.to_string())?
                    .into_iter()
                    .find(|descriptor| descriptor.name == property)
                    .ok_or_else(|| {
                        format!("Property {property} is not registered on node {node}")
                    })?;
                if descriptor.read_only {
                    return Err(format!("Property {property} is read-only"));
                }
                if args["action"] == "set" {
                    s::validate(&property_schema(&descriptor.kind), &args["value"])?;
                }
            }
            if args["action"] == "reset" {
                session.reset_property(&nodes, &property)?;
            } else {
                let value = if let Some(value) = args["value"].as_str() {
                    value.to_owned()
                } else if let Some(value) = args["value"].as_bool() {
                    if value {
                        "True".into()
                    } else {
                        "False".into()
                    }
                } else if args["value"].is_number() {
                    args["value"].to_string()
                } else {
                    let value: AnimationValue = decode(args, "value")?;
                    value.validate().map_err(|error| error.to_string())?;
                    value.to_xaml()
                };
                session.set_property(&nodes, &property, &value)?;
            }
        }
        _ => return Err("Unknown designer authoring tool".into()),
    }
    Ok(json!({"document":state(session)}))
}
fn distinct(nodes: &[u64]) -> Result<(), String> {
    if nodes.len() != nodes.iter().collect::<BTreeSet<_>>().len() {
        Err("Node IDs must be distinct".into())
    } else {
        Ok(())
    }
}
fn property_schema(kind: &PropertyKind) -> Value {
    let typed = |kind: &str| json!({"type":"object","properties":{"kind":{"const":kind},"value":{}},"required":["kind","value"],"additionalProperties":false,"allOf":[s::animation_value()]});
    let value = match kind {
        PropertyKind::Boolean => json!({"type":"boolean"}),
        PropertyKind::Number {
            minimum, maximum, ..
        } => {
            let mut value = s::number();
            if let Some(minimum) = minimum {
                value["minimum"] = json!(minimum);
            }
            if let Some(maximum) = maximum {
                value["maximum"] = json!(maximum);
            }
            let tagged = s::object(
                json!({"kind":{"const":"Number"},"value":value}),
                &["kind", "value"],
            );
            json!({"anyOf":[value,tagged]})
        }
        PropertyKind::Integer { minimum, maximum } => {
            json!({"type":"integer","minimum":minimum,"maximum":maximum})
        }
        PropertyKind::Color => typed("Color"),
        PropertyKind::Thickness => json!({"anyOf":[typed("Thickness"),typed("CornerRadius")]}),
        PropertyKind::Text => {
            json!({"anyOf":[typed("Discrete"),typed("Point"),typed("Vector"),typed("Size"),typed("Rect"),typed("CornerRadius"),typed("RelativePoint"),typed("RelativeScalar")]})
        }
        _ => s::string(),
    };
    // Text is the native conversion/expression path; the property registry validates its
    // enum membership, finite ranges, custom editors, and markup grammar before commit.
    json!({"anyOf":[value,s::string()]})
}

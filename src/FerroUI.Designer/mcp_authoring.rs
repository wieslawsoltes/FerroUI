//! Model-backed animation, state, behavior and typed-property automation.
use super::schema as s;
use super::{decode, node_id, number, state, text, Permissions};
use crate::DesignerSession;
use crate::animation_keyframe_authoring::KeyframeEdit;
use ferroui_designer_animation::{
    AnimationValue, BehaviorModel, Easing, Repeat, Timeline, TimelineTimingEdit, VisualStateGroup,
};
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
        s::tool("designer_programs","Read animation programs, source keys, codec diagnostics and typed timelines. include_keyframe_text adds formatted XAML values and easing to the typed keys.",s::object(json!({"include_source":{"type":"boolean"},"include_keyframe_text":{"type":"boolean"}}), &[]),false),
        s::actions("designer_program","Author animation programs and keyframes, edit timing with proportional retiming, set selectors or preview targets, sample several programs together, or play all programs. IDs refer to the current document; every action checks expected_revision.",vec![
            s::action("create",json!({"name":s::string(),"owner":s::id(),"selector":s::string(),"duration":s::time()}), &["name","owner","selector","duration"],true),
            s::action("select",json!({"program":s::id()}), &["program"],true),
            s::action("rename",json!({"program":s::id(),"name":s::string()}), &["program","name"],true),
            s::action("duplicate",json!({"program":s::id(),"name":s::string()}), &["program","name"],true),
            s::action("remove",json!({"program":s::id()}), &["program"],true),
            s::action("selector",json!({"program":s::id(),"selector":s::string()}), &["program","selector"],true),
            s::action("source",json!({"program":s::id(),"source":s::string()}), &["program","source"],true),
            s::action("update",json!({"program":s::id(),"timeline":s::timeline()}), &["program","timeline"],true),
            s::action("update_timing",json!({"program":s::id(),"timing":s::timing_edit()}), &["program","timing"],true),
            s::action("upsert_keyframe",json!({"program":s::id(),"target":s::id(),"property":s::string(),"previous":s::time(),"time":s::time(),"value":s::string(),"easing":s::string()}), &["program","target","property","time","value","easing"],true),
            s::action("record_keyframe",json!({"program":s::id(),"target":s::id(),"property":s::string(),"time":s::time()}), &["program","target","property","time"],true),
            s::action("remove_keyframe",json!({"program":s::id(),"target":s::id(),"property":s::string(),"time":s::time()}), &["program","target","property","time"],true),
            s::action("preview_targets",json!({"program":s::id(),"targets":s::nullable(s::array(s::id(),4096))}), &["program","targets"],true),
            s::action("preview",json!({"programs":ids(),"time":s::time()}), &["programs","time"],true),
            s::action("scrub",json!({"program":s::id(),"time":s::time()}), &["program","time"],true),
            s::action("play_program",json!({"program":s::id(),"time":s::time(),"from_playhead":{"type":"boolean"}}), &["program"],true),
            s::action("advance_program",json!({"program":s::id(),"seconds":s::time()}), &["program","seconds"],true),
            s::action("pause_program",json!({"program":s::id()}), &["program"],true),
            s::action("stop_program",json!({}), &[],true),
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
            let programs = animation_programs(session, args)?;
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
                "update_timing" => {
                    let mut fields = args["timing"].clone();
                    if let Some(repeat) = fields["repeat"].as_str() {
                        fields["repeat"] = json!(Repeat::parse(repeat).map_err(|error| error.to_string())?);
                    }
                    if let Some(easing) = fields["easing"].as_str() {
                        fields["easing"] = json!(Easing::parse(easing).map_err(|error| error.to_string())?);
                    }
                    let timing: TimelineTimingEdit = serde_json::from_value(fields)
                        .map_err(|error| format!("Invalid timing: {error}"))?;
                    session.edit_animation_timing(node_id(args, "program")?, timing)?;
                }
                "upsert_keyframe" => session.edit_program_keyframe(
                    node_id(args, "program")?, node_id(args, "target")?, &text(args, "property")?,
                    KeyframeEdit::Upsert {
                        previous: args["previous"].as_f64(), time: number(args, "time")?,
                        value: &text(args, "value")?, easing: &text(args, "easing")?,
                    },
                )?,
                "record_keyframe" => {
                    let program = node_id(args, "program")?;
                    if session.active_animation_program() != Some(program) {
                        session.select_animation_program(program)?;
                    }
                    let time = number(args, "time")?;
                    if !(0.0..=session.timeline().duration).contains(&time) {
                        return Err("Choose a time within the animation duration".into());
                    }
                    session.record_effective_keyframe(
                        node_id(args, "target")?,
                        &text(args, "property")?,
                        time,
                    )?;
                }
                "remove_keyframe" => session.edit_program_keyframe(
                    node_id(args, "program")?, node_id(args, "target")?, &text(args, "property")?,
                    KeyframeEdit::Remove(number(args, "time")?),
                )?,
                "preview_targets" => session.set_animation_program_preview_targets(
                    node_id(args, "program")?,
                    decode(args, "targets")?,
                )?,
                "preview" => {
                    let programs: Vec<u64> = decode(args, "programs")?;
                    distinct(&programs)?;
                    session.preview_animation_programs(&programs, number(args, "time")?)?;
                }
                "scrub" => {
                    let id = node_id(args, "program")?;
                    if session.active_animation_program() != Some(id) {
                        session.select_animation_program(id)?;
                    }
                    let time = number(args, "time")?;
                    if time > session.timeline().duration {
                        return Err("Choose a time within the animation duration".into());
                    }
                    session.pause();
                    session.seek(time)?;
                    return Ok(json!({"timelinePosition":{"time":time,"playing":false}}));
                }
                "play_program" => {
                    session.start_animation_playback(
                        node_id(args, "program")?,
                        args["time"].as_f64(),
                        args["from_playhead"] == true,
                    )?;
                    return playback_position(session);
                }
                action @ ("advance_program" | "pause_program") => {
                    if session.active_animation_program() != Some(node_id(args, "program")?) {
                        return Err("Play this animation program before advancing or pausing it".into());
                    }
                    if action == "advance_program" {
                        session.advance(number(args, "seconds")?)?;
                    } else {
                        session.pause();
                    }
                    return playback_position(session);
                }
                "stop_program" => {
                    session.stop();
                    return Ok(json!({"timelinePosition":{"time":0,"playing":false}}));
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
fn playback_position(session: &DesignerSession) -> Result<Value, String> {
    let elapsed = session.time();
    let time = session.animation.compiled.playback_position(elapsed).map_err(|error| error.to_string())?;
    Ok(json!({"timelinePosition":{"time":time,"elapsed":elapsed,"playing":session.is_playing()}}))
}
fn animation_programs(session: &DesignerSession, args: &Value) -> Result<Vec<Value>, String> {
    session.animation_programs().iter().map(|program| {
        let mut model = serde_json::to_value(program).map_err(|error| error.to_string())?;
        if args["include_source"] == false {
            model.as_object_mut().expect("program is an object").remove("source");
        }
        if args["include_keyframe_text"] == true {
            for (track, authored) in program.timeline.iter().flat_map(|timeline| &timeline.tracks).enumerate() {
                for (key, authored) in authored.keyframes.iter().enumerate() {
                    let model = &mut model["timeline"]["tracks"][track]["keyframes"][key];
                    model["value_text"] = json!(authored.value.to_xaml());
                    model["easing_text"] = json!(authored.easing.to_xaml());
                }
            }
        }
        if let Some(timeline) = &program.timeline {
            model["timeline"]["repeat_text"] = json!(timeline.repeat.to_string());
            model["timeline"]["easing_text"] = json!(timeline.easing.to_xaml());
            model["timeline"]["total_duration"] = json!(timeline.total_duration());
        }
        Ok(model)
    }).collect()
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

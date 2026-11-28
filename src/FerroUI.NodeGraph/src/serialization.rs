//! Zero-dependency, human-readable JSON serializer & deserializer for [`NodeGraph`].
//!
//! Saves and restores every node, position, slider/toggle/text/color widget, per-port Grasshopper
//! modifier (`F`/`G`/`R`/`S`), wire, colored group, and camera transform cleanly!

use crate::data_tree::Value;
use crate::graph::NodeGraph;
use crate::node::NodeRegistry;
use crate::types::{DataMatching, GraphCurveKind, NodeId, ParamModifier, ParamWidget, WireDisplay};
use rustc_hash::FxHashMap;
use std::fmt::Write as _;

/// Validates persistence inputs before serializing a document for storage or clipboard export.
/// Runtime `Value::Custom` payloads are intentionally opaque and require an application codec.
pub fn try_graph_to_json(graph: &NodeGraph) -> Result<String, String> {
    for node in graph.nodes.values() {
        if node
            .input_defaults
            .iter()
            .flatten()
            .any(|value| matches!(value, Value::Custom(_)))
        {
            return Err(format!(
                "Node {} has an opaque input default; an application codec is required",
                node.id
            ));
        }
        if node
            .size
            .iter()
            .any(|size| !size.is_finite() || *size <= 0.0)
        {
            return Err(format!("Node {} has invalid dimensions", node.id));
        }
    }
    let json = graph_to_json(graph);
    // Also rejects non-finite numbers anywhere in widgets, geometry, defaults, or camera state.
    parse_json(&json)?;
    Ok(json)
}

/// Serializes `graph` into formatted JSON. Prefer [`try_graph_to_json`] for user-facing export,
/// which rejects non-finite numbers and opaque custom input defaults.
pub fn graph_to_json(graph: &NodeGraph) -> String {
    let mut out = String::with_capacity(2048);
    let _ = writeln!(out, "{{");
    let _ = writeln!(out, "  \"format\": \"ferroui-nodegraph/v1\",");
    let _ = writeln!(out, "  \"name\": \"{}\",", esc(&graph.name));
    let _ = writeln!(
        out,
        "  \"camera\": {{\"pan\": [{}, {}], \"zoom\": {}}},",
        graph.camera_pan[0], graph.camera_pan[1], graph.camera_zoom
    );

    let _ = writeln!(out,
        "  \"options\": {{\"fancy_wires\": {}, \"show_profiler\": {}, \"snap_to_grid\": {}, \"solution_locked\": {}}},",
        graph.fancy_wires, graph.show_profiler, graph.snap_to_grid, graph.solution_locked);

    // Nodes
    let _ = writeln!(out, "  \"nodes\": [");
    for (idx, (_, n)) in graph.nodes.iter().enumerate() {
        let comma = if idx + 1 < graph.nodes.len() { "," } else { "" };
        let title_json = match &n.custom_title {
            Some(t) => format!("\"{}\"", esc(t)),
            None => "null".into(),
        };
        let widget_json = match &n.widget {
            ParamWidget::None => "null".into(),
            ParamWidget::Slider { value, min, max, step, integer } => format!(
                "{{\"kind\":\"slider\",\"value\":{value},\"min\":{min},\"max\":{max},\"step\":{step},\"integer\":{integer}}}"
            ),
            ParamWidget::Toggle { value } => format!("{{\"kind\":\"toggle\",\"value\":{value}}}"),
            ParamWidget::Button { pressed } => format!("{{\"kind\":\"button\",\"pressed\":{pressed}}}"),
            ParamWidget::Color { argb } => format!("{{\"kind\":\"color\",\"argb\":{argb}}}"),
            ParamWidget::Text { text } => format!("{{\"kind\":\"text\",\"text\":\"{}\"}}", esc(text)),
            ParamWidget::GraphMapper { curve, p1, p2 } => format!(
                "{{\"kind\":\"graph_mapper\",\"curve\":\"{}\",\"p1\":[{},{}],\"p2\":[{},{}]}}",
                curve.label(), p1[0], p1[1], p2[0], p2[1]
            ),
            ParamWidget::MdSlider { x, y } => format!("{{\"kind\":\"md_slider\",\"x\":{x},\"y\":{y}}}"),
            ParamWidget::ValueList { options, selected } => {
                let opts: Vec<String> = options
                    .iter()
                    .map(|(k, v)| format!("[\"{}\",{v}]", esc(k)))
                    .collect();
                format!("{{\"kind\":\"value_list\",\"selected\":{selected},\"options\":[{}]}}", opts.join(","))
            }
            ParamWidget::PathMapper { source_mask, target_mask } => format!(
                "{{\"kind\":\"path_mapper\",\"src\":\"{}\",\"tgt\":\"{}\"}}",
                esc(source_mask), esc(target_mask)
            ),
        };
        let mods: Vec<String> = n
            .input_modifiers
            .iter()
            .map(|m| format!("\"{}\"", m.badge()))
            .collect();
        let out_mods: Vec<String> = n
            .output_modifiers
            .iter()
            .map(|modifier| format!("\"{}\"", modifier.badge()))
            .collect();
        let defaults: Vec<String> = n
            .input_defaults
            .iter()
            .map(|value| {
                value
                    .as_ref()
                    .map(value_to_json)
                    .unwrap_or_else(|| "null".into())
            })
            .collect();
        let matching = match n.matching {
            DataMatching::LongestList => "longest",
            DataMatching::ShortestList => "shortest",
            DataMatching::CrossReference => "cross",
        };
        let _ = writeln!(
            out,
            "    {{\"id\":{},\"type\":\"{}\",\"title\":{},\"pos\":[{},{}],\"disabled\":{},\"widget\":{},\"in_mods\":[{}],\"out_mods\":[{}],\"defaults\":[{}],\"matching\":\"{}\",\"preview\":{},\"size\":[{},{}]}}{comma}",
            n.id.0,
            esc(&n.type_id),
            title_json,
            n.position[0],
            n.position[1],
            n.disabled,
            widget_json,
            mods.join(","),
            out_mods.join(","),
            defaults.join(","),
            matching,
            n.preview,
            n.size[0], n.size[1],
        );
    }
    let _ = writeln!(out, "  ],");

    // Wires
    let _ = writeln!(out, "  \"wires\": [");
    for (idx, (_, w)) in graph.wires.iter().enumerate() {
        let comma = if idx + 1 < graph.wires.len() { "," } else { "" };
        let _ = writeln!(
            out,
            "    {{\"from\":[{},{}],\"to\":[{},{}],\"display\":\"{}\"}}{comma}",
            w.from_node.0,
            w.from_port,
            w.to_node.0,
            w.to_port,
            match w.display {
                WireDisplay::Default => "default",
                WireDisplay::Faint => "faint",
                WireDisplay::Hidden => "hidden",
            }
        );
    }
    let _ = writeln!(out, "  ],");

    // Groups
    let _ = writeln!(out, "  \"groups\": [");
    for (idx, (_, g)) in graph.groups.iter().enumerate() {
        let comma = if idx + 1 < graph.groups.len() {
            ","
        } else {
            ""
        };
        let members: Vec<String> = g.members.iter().map(|m| m.0.to_string()).collect();
        let _ = writeln!(
            out,
            "    {{\"title\":\"{}\",\"color\":{},\"members\":[{}]}}{comma}",
            esc(&g.title),
            g.color_argb,
            members.join(",")
        );
    }
    let _ = writeln!(out, "  ]");
    let _ = writeln!(out, "}}");
    out
}

/// Deserializes a JSON string produced by [`graph_to_json`] back into a live [`NodeGraph`].
pub fn graph_from_json(json: &str, registry: &NodeRegistry) -> Result<NodeGraph, String> {
    let root = parse_json(json)?;
    let obj = root.as_obj().ok_or("Expected JSON object at root")?;
    if let Some(format) = obj.get("format") {
        if format.as_str() != Some("ferroui-nodegraph/v1") {
            return Err("Unsupported node graph format".into());
        }
    }
    let name = obj
        .get("name")
        .and_then(JVal::as_str)
        .unwrap_or("Imported Definition");
    let mut g = NodeGraph::new(name);

    if let Some(cam) = obj.get("camera").and_then(JVal::as_obj) {
        if let Some(pan) = cam.get("pan").and_then(JVal::as_arr) {
            if pan.len() >= 2 {
                g.camera_pan = [
                    pan[0].as_f64().unwrap_or(40.0),
                    pan[1].as_f64().unwrap_or(40.0),
                ];
            }
        }
        if let Some(z) = cam.get("zoom").and_then(JVal::as_f64) {
            g.camera_zoom = z.clamp(0.25, 3.0);
        }
    }

    if let Some(options) = obj.get("options").and_then(JVal::as_obj) {
        if let Some(value) = options.get("fancy_wires").and_then(JVal::as_bool) {
            g.fancy_wires = value;
        }
        if let Some(value) = options.get("show_profiler").and_then(JVal::as_bool) {
            g.show_profiler = value;
        }
        if let Some(value) = options.get("snap_to_grid").and_then(JVal::as_bool) {
            g.snap_to_grid = value;
        }
        if let Some(value) = options.get("solution_locked").and_then(JVal::as_bool) {
            g.solution_locked = value;
        }
    }
    // Loading must not re-snap previously saved coordinates.
    let snap_to_grid = g.snap_to_grid;
    g.snap_to_grid = false;
    let mut id_map: FxHashMap<u32, NodeId> = FxHashMap::default();
    {
        let nodes = obj
            .get("nodes")
            .and_then(JVal::as_arr)
            .ok_or("Expected node array")?;
        for nval in nodes {
            let nobj = nval.as_obj().ok_or("Expected node object")?;
            let old_id = json_u32(nobj.get("id").ok_or("Missing node id")?)?;
            if id_map.contains_key(&old_id) {
                return Err(format!("Duplicate node id {old_id}"));
            }
            let type_id = nobj.get("type").and_then(JVal::as_str).unwrap_or("");
            let pos = nobj.get("pos").and_then(JVal::as_arr);
            let x = pos
                .and_then(|p| p.first())
                .and_then(JVal::as_f64)
                .unwrap_or(0.0);
            let y = pos
                .and_then(|p| p.get(1))
                .and_then(JVal::as_f64)
                .unwrap_or(0.0);

            let new_id = g
                .add_node(registry, type_id, x, y)
                .ok_or_else(|| format!("Unknown component `{type_id}`"))?;
            id_map.insert(old_id, new_id);

            if let Some(inst) = g.nodes.get_mut(&new_id) {
                if let Some(title) = nobj.get("title").and_then(JVal::as_str) {
                    inst.custom_title = Some(title.to_string());
                }
                if let Some(dis) = nobj.get("disabled").and_then(JVal::as_bool) {
                    inst.disabled = dis;
                }
                if let Some(preview) = nobj.get("preview").and_then(JVal::as_bool) {
                    inst.preview = preview;
                }
                if let Some(size) = nobj.get("size") {
                    inst.size = json_floats::<2>(size)?;
                    if inst.size.iter().any(|v| *v <= 0.0) {
                        return Err("Node size must be positive".into());
                    }
                }
                if let Some(matching) = nobj.get("matching") {
                    inst.matching = match matching.as_str() {
                        Some("longest") => DataMatching::LongestList,
                        Some("shortest") => DataMatching::ShortestList,
                        Some("cross") => DataMatching::CrossReference,
                        _ => return Err("Invalid data matching mode".into()),
                    };
                }
                if let Some(defaults) = nobj.get("defaults").and_then(JVal::as_arr) {
                    if defaults.len() > inst.input_defaults.len() {
                        return Err("Too many input defaults".into());
                    }
                    for (port, value) in defaults.iter().enumerate() {
                        if !matches!(value, JVal::Null) {
                            inst.input_defaults[port] = Some(value_from_json(value)?);
                        }
                    }
                }
                if let Some(modifiers) = nobj.get("out_mods").and_then(JVal::as_arr) {
                    read_modifiers(modifiers, &mut inst.output_modifiers)?;
                }
                if matches!(nobj.get("widget"), Some(JVal::Null)) {
                    inst.widget = ParamWidget::None;
                }
                if let Some(wobj) = nobj.get("widget").and_then(JVal::as_obj) {
                    match wobj.get("kind").and_then(JVal::as_str) {
                        Some("slider") => {
                            inst.widget = ParamWidget::Slider {
                                value: wobj.get("value").and_then(JVal::as_f64).unwrap_or(0.0),
                                min: wobj.get("min").and_then(JVal::as_f64).unwrap_or(0.0),
                                max: wobj.get("max").and_then(JVal::as_f64).unwrap_or(10.0),
                                step: wobj.get("step").and_then(JVal::as_f64).unwrap_or(0.1),
                                integer: wobj
                                    .get("integer")
                                    .and_then(JVal::as_bool)
                                    .unwrap_or(false),
                            };
                            if let ParamWidget::Slider {
                                value,
                                min,
                                max,
                                step,
                                ..
                            } = &mut inst.widget
                            {
                                if *min > *max || *step < 0.0 {
                                    return Err("Invalid slider range or step".into());
                                }
                                *value = value.clamp(*min, *max);
                            }
                        }
                        Some("toggle") => {
                            inst.widget = ParamWidget::Toggle {
                                value: wobj.get("value").and_then(JVal::as_bool).unwrap_or(false),
                            };
                        }
                        Some("button") => {
                            inst.widget = ParamWidget::Button {
                                pressed: wobj
                                    .get("pressed")
                                    .and_then(JVal::as_bool)
                                    .unwrap_or(false),
                            };
                        }
                        Some("color") => {
                            inst.widget = ParamWidget::Color {
                                argb: wobj
                                    .get("argb")
                                    .and_then(JVal::as_f64)
                                    .unwrap_or(0xFF38BDF8_u32 as f64)
                                    as u32,
                            };
                        }
                        Some("text") => {
                            inst.widget = ParamWidget::Text {
                                text: wobj
                                    .get("text")
                                    .and_then(JVal::as_str)
                                    .unwrap_or("")
                                    .to_string(),
                            };
                        }
                        Some("graph_mapper") => {
                            let curve = match wobj.get("curve").and_then(JVal::as_str) {
                                Some("Sine") => GraphCurveKind::Sine,
                                Some("Parabola") => GraphCurveKind::Parabola,
                                Some("Gaussian") => GraphCurveKind::Gaussian,
                                Some("Linear") => GraphCurveKind::Linear,
                                Some("Bezier") | None => GraphCurveKind::Bezier,
                                _ => return Err("Unknown graph mapper curve".into()),
                            };
                            let p1a = wobj.get("p1").and_then(JVal::as_arr);
                            let p2a = wobj.get("p2").and_then(JVal::as_arr);
                            inst.widget = ParamWidget::GraphMapper {
                                curve,
                                p1: [
                                    p1a.and_then(|a| a.first())
                                        .and_then(JVal::as_f64)
                                        .unwrap_or(0.25),
                                    p1a.and_then(|a| a.get(1))
                                        .and_then(JVal::as_f64)
                                        .unwrap_or(0.75),
                                ],
                                p2: [
                                    p2a.and_then(|a| a.first())
                                        .and_then(JVal::as_f64)
                                        .unwrap_or(0.75),
                                    p2a.and_then(|a| a.get(1))
                                        .and_then(JVal::as_f64)
                                        .unwrap_or(0.25),
                                ],
                            };
                        }
                        Some("md_slider") => {
                            inst.widget = ParamWidget::MdSlider {
                                x: wobj.get("x").and_then(JVal::as_f64).unwrap_or(0.5),
                                y: wobj.get("y").and_then(JVal::as_f64).unwrap_or(0.5),
                            };
                        }
                        Some("value_list") => {
                            let selected =
                                wobj.get("selected").and_then(JVal::as_f64).unwrap_or(0.0) as usize;
                            let options: Vec<(String, f64)> = wobj
                                .get("options")
                                .and_then(JVal::as_arr)
                                .map(|arr| {
                                    arr.iter()
                                        .filter_map(|e| e.as_arr())
                                        .map(|pair| {
                                            (
                                                pair.first()
                                                    .and_then(JVal::as_str)
                                                    .unwrap_or("Opt")
                                                    .to_string(),
                                                pair.get(1).and_then(JVal::as_f64).unwrap_or(0.0),
                                            )
                                        })
                                        .collect()
                                })
                                .unwrap_or_default();
                            inst.widget = ParamWidget::ValueList { options, selected };
                        }
                        Some("path_mapper") => {
                            inst.widget = ParamWidget::PathMapper {
                                source_mask: wobj
                                    .get("src")
                                    .and_then(JVal::as_str)
                                    .unwrap_or("{A;B}")
                                    .to_string(),
                                target_mask: wobj
                                    .get("tgt")
                                    .and_then(JVal::as_str)
                                    .unwrap_or("{B;A}")
                                    .to_string(),
                            };
                        }
                        _ => return Err("Unknown widget kind".into()),
                    }
                }
                if let Some(mods) = nobj.get("in_mods").and_then(JVal::as_arr) {
                    read_modifiers(mods, &mut inst.input_modifiers)?;
                }
            }
        }
    }

    if let Some(wires) = obj.get("wires").and_then(JVal::as_arr) {
        for value in wires {
            let wire = value.as_obj().ok_or("Expected wire object")?;
            let from = wire
                .get("from")
                .and_then(JVal::as_arr)
                .ok_or("Missing wire source")?;
            let to = wire
                .get("to")
                .and_then(JVal::as_arr)
                .ok_or("Missing wire target")?;
            if from.len() != 2 || to.len() != 2 {
                return Err("Invalid wire endpoint".into());
            }
            let source = id_map
                .get(&json_u32(&from[0])?)
                .ok_or("Unknown wire source node")?;
            let target = id_map
                .get(&json_u32(&to[0])?)
                .ok_or("Unknown wire target node")?;
            let id = g
                .connect(
                    *source,
                    json_u32(&from[1])? as usize,
                    *target,
                    json_u32(&to[1])? as usize,
                    true,
                )
                .map_err(str::to_string)?;
            if let Some(display) = wire.get("display") {
                g.wires.get_mut(&id).unwrap().display = match display.as_str() {
                    Some("default") => WireDisplay::Default,
                    Some("faint") => WireDisplay::Faint,
                    Some("hidden") => WireDisplay::Hidden,
                    _ => return Err("Invalid wire display mode".into()),
                };
            }
        }
    }

    if let Some(groups) = obj.get("groups").and_then(JVal::as_arr) {
        for gval in groups {
            let Some(gobj) = gval.as_obj() else { continue };
            let title = gobj.get("title").and_then(JVal::as_str).unwrap_or("Group");
            let color = gobj
                .get("color")
                .and_then(JVal::as_f64)
                .unwrap_or(0x3338BDF8_u32 as f64) as u32;
            let members: Vec<NodeId> = gobj
                .get("members")
                .and_then(JVal::as_arr)
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_f64().map(|n| n as u32))
                        .filter_map(|old| id_map.get(&old).copied())
                        .collect()
                })
                .unwrap_or_default();
            if !members.is_empty() {
                g.create_group(title, color, members);
            }
        }
    }

    g.snap_to_grid = snap_to_grid;
    g.mark_all_dirty();
    Ok(g)
}

fn read_modifiers(values: &[JVal], modifiers: &mut [ParamModifier]) -> Result<(), String> {
    if values.len() > modifiers.len() {
        return Err("Too many port modifiers".into());
    }
    for (value, modifier) in values.iter().zip(modifiers) {
        let badge = value.as_str().ok_or("Invalid port modifier")?;
        if !badge.chars().all(|c| matches!(c, 'F' | 'G' | 'R' | 'S')) {
            return Err("Invalid port modifier badge".into());
        }
        *modifier = ParamModifier {
            flatten: badge.contains('F'),
            graft: badge.contains('G'),
            reverse: badge.contains('R'),
            simplify: badge.contains('S'),
        };
    }
    Ok(())
}

fn json_u32(value: &JVal) -> Result<u32, String> {
    let number = value.as_f64().ok_or("Expected unsigned integer")?;
    if number < 0.0 || number > u32::MAX as f64 || number.fract() != 0.0 {
        return Err("Integer out of range".into());
    }
    Ok(number as u32)
}

fn json_floats<const N: usize>(value: &JVal) -> Result<[f64; N], String> {
    let array = value.as_arr().ok_or("Expected numeric array")?;
    if array.len() != N {
        return Err(format!("Expected array of {N} numbers"));
    }
    let mut result = [0.0; N];
    for (slot, value) in result.iter_mut().zip(array) {
        *slot = value.as_f64().ok_or("Expected number")?;
    }
    Ok(result)
}

fn value_to_json(value: &Value) -> String {
    match value {
        Value::Null => "{\"kind\":\"null\"}".into(),
        Value::Bool(value) => format!("{{\"kind\":\"bool\",\"value\":{value}}}"),
        // Strings preserve all 64 bits despite JSON readers which use f64 numbers.
        Value::Int(value) => format!("{{\"kind\":\"int\",\"value\":\"{value}\"}}"),
        Value::Float(value) => format!("{{\"kind\":\"float\",\"value\":{value}}}"),
        Value::Vec2([x, y]) => format!("{{\"kind\":\"vec2\",\"value\":[{x},{y}]}}"),
        Value::Vec3([x, y, z]) => format!("{{\"kind\":\"vec3\",\"value\":[{x},{y},{z}]}}"),
        Value::Color(value) => format!("{{\"kind\":\"color\",\"value\":{value}}}"),
        Value::String(value) => format!("{{\"kind\":\"string\",\"value\":\"{}\"}}", esc(value)),
        Value::Domain(a, b) => format!("{{\"kind\":\"domain\",\"value\":[{a},{b}]}}"),
        Value::Curve(points) => format!(
            "{{\"kind\":\"curve\",\"value\":[{}]}}",
            points_to_json(points)
        ),
        Value::Mesh { vertices, indices } => format!(
            "{{\"kind\":\"mesh\",\"vertices\":[{}],\"indices\":[{}]}}",
            points_to_json(vertices),
            indices
                .iter()
                .map(|[a, b, c]| format!("[{a},{b},{c}]"))
                .collect::<Vec<_>>()
                .join(",")
        ),
        Value::Custom(payload) => format!(
            "{{\"kind\":\"custom\",\"type\":\"{}\",\"summary\":\"{}\"}}",
            esc(&payload.type_tag),
            esc(&payload.summary)
        ),
    }
}

fn points_to_json(points: &[[f64; 2]]) -> String {
    points
        .iter()
        .map(|[x, y]| format!("[{x},{y}]"))
        .collect::<Vec<_>>()
        .join(",")
}

fn value_from_json(value: &JVal) -> Result<Value, String> {
    let object = value.as_obj().ok_or("Expected typed default value")?;
    let field = || {
        object
            .get("value")
            .ok_or_else(|| "Missing default value".to_string())
    };
    Ok(match object.get("kind").and_then(JVal::as_str) {
        Some("null") => Value::Null,
        Some("bool") => Value::Bool(field()?.as_bool().ok_or("Invalid Boolean default")?),
        Some("int") => Value::Int(
            field()?
                .as_str()
                .ok_or("Invalid integer default")?
                .parse()
                .map_err(|_| "Integer default out of range")?,
        ),
        Some("float") => Value::Float(field()?.as_f64().ok_or("Invalid number default")?),
        Some("vec2") => Value::Vec2(json_floats(field()?)?),
        Some("vec3") => Value::Vec3(json_floats(field()?)?),
        Some("color") => Value::Color(json_u32(field()?)?),
        Some("string") => Value::String(field()?.as_str().ok_or("Invalid text default")?.into()),
        Some("domain") => {
            let [a, b] = json_floats(field()?)?;
            Value::Domain(a, b)
        }
        Some("curve") => Value::Curve(points_from_json(field()?)?),
        Some("mesh") => {
            let vertices =
                points_from_json(object.get("vertices").ok_or("Missing mesh vertices")?)?;
            let indices = object
                .get("indices")
                .and_then(JVal::as_arr)
                .ok_or("Missing mesh indices")?
                .iter()
                .map(|value| {
                    let triangle = value.as_arr().ok_or("Invalid mesh triangle")?;
                    if triangle.len() != 3 {
                        return Err("Invalid mesh triangle".to_string());
                    }
                    let result = [
                        json_u32(&triangle[0])?,
                        json_u32(&triangle[1])?,
                        json_u32(&triangle[2])?,
                    ];
                    if result.iter().any(|&index| index as usize >= vertices.len()) {
                        return Err("Mesh triangle index out of range".into());
                    }
                    Ok(result)
                })
                .collect::<Result<Vec<_>, String>>()?;
            Value::Mesh { vertices, indices }
        }
        Some("custom") => {
            return Err("Opaque custom input defaults require an application-specific codec".into())
        }
        _ => return Err("Unknown default value kind".into()),
    })
}

fn points_from_json(value: &JVal) -> Result<Vec<[f64; 2]>, String> {
    value
        .as_arr()
        .ok_or("Expected point array")?
        .iter()
        .map(json_floats)
        .collect()
}

fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c <= '\u{1f}' => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out
}

// =============================================================================
// Compact Recursive-Descent JSON Parser (Zero External Dependencies)
// =============================================================================

#[derive(Debug)]
enum JVal {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<JVal>),
    Obj(FxHashMap<String, JVal>),
}

impl JVal {
    fn as_obj(&self) -> Option<&FxHashMap<String, JVal>> {
        match self {
            Self::Obj(m) => Some(m),
            _ => None,
        }
    }
    fn as_arr(&self) -> Option<&[JVal]> {
        match self {
            Self::Arr(a) => Some(a),
            _ => None,
        }
    }
    fn as_str(&self) -> Option<&str> {
        match self {
            Self::Str(s) => Some(s),
            _ => None,
        }
    }
    fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Num(n) => Some(*n),
            _ => None,
        }
    }
    fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(b) => Some(*b),
            _ => None,
        }
    }
}

fn parse_json(src: &str) -> Result<JVal, String> {
    let mut parser = JsonParser {
        bytes: src.as_bytes(),
        pos: 0,
        depth: 0,
    };
    let value = parser.parse_val()?;
    parser.skip_ws();
    if parser.pos != parser.bytes.len() {
        return Err("Trailing content after JSON document".into());
    }
    Ok(value)
}

struct JsonParser<'a> {
    bytes: &'a [u8],
    pos: usize,
    depth: usize,
}

impl JsonParser<'_> {
    fn skip_ws(&mut self) {
        while self
            .bytes
            .get(self.pos)
            .is_some_and(|b| matches!(b, b' ' | b'\t' | b'\r' | b'\n'))
        {
            self.pos += 1;
        }
    }

    fn peek(&mut self) -> Option<u8> {
        self.skip_ws();
        self.bytes.get(self.pos).copied()
    }

    fn parse_val(&mut self) -> Result<JVal, String> {
        if self.depth >= 128 {
            return Err("JSON nesting exceeds 128 levels".into());
        }
        self.depth += 1;
        let result = match self.peek() {
            Some(b'{') => self.parse_obj(),
            Some(b'[') => self.parse_arr(),
            Some(b'"') => self.parse_str().map(JVal::Str),
            Some(b't') if self.bytes[self.pos..].starts_with(b"true") => {
                self.pos += 4;
                Ok(JVal::Bool(true))
            }
            Some(b'f') if self.bytes[self.pos..].starts_with(b"false") => {
                self.pos += 5;
                Ok(JVal::Bool(false))
            }
            Some(b'n') if self.bytes[self.pos..].starts_with(b"null") => {
                self.pos += 4;
                Ok(JVal::Null)
            }
            Some(c) if c == b'-' || c.is_ascii_digit() => self.parse_num(),
            _ => Err(format!("Invalid JSON token at byte {}", self.pos)),
        };
        self.depth -= 1;
        result
    }

    fn parse_obj(&mut self) -> Result<JVal, String> {
        self.pos += 1;
        let mut map = FxHashMap::default();
        if self.peek() == Some(b'}') {
            self.pos += 1;
            return Ok(JVal::Obj(map));
        }
        loop {
            let key = self.parse_str()?;
            if self.peek() != Some(b':') {
                return Err("Expected ':' in JSON object".into());
            }
            self.pos += 1;
            let value = self.parse_val()?;
            if map.insert(key, value).is_some() {
                return Err("Duplicate JSON object key".into());
            }
            match self.peek() {
                Some(b',') => self.pos += 1,
                Some(b'}') => {
                    self.pos += 1;
                    return Ok(JVal::Obj(map));
                }
                _ => return Err("Expected ',' or '}' in JSON object".into()),
            }
        }
    }

    fn parse_arr(&mut self) -> Result<JVal, String> {
        self.pos += 1;
        let mut array = Vec::new();
        if self.peek() == Some(b']') {
            self.pos += 1;
            return Ok(JVal::Arr(array));
        }
        loop {
            array.push(self.parse_val()?);
            match self.peek() {
                Some(b',') => self.pos += 1,
                Some(b']') => {
                    self.pos += 1;
                    return Ok(JVal::Arr(array));
                }
                _ => return Err("Expected ',' or ']' in JSON array".into()),
            }
        }
    }

    fn parse_str(&mut self) -> Result<String, String> {
        if self.peek() != Some(b'"') {
            return Err("Expected string".into());
        }
        self.pos += 1;
        let mut bytes = Vec::new();
        while let Some(&byte) = self.bytes.get(self.pos) {
            self.pos += 1;
            match byte {
                b'"' => return String::from_utf8(bytes).map_err(|_| "Invalid UTF-8 string".into()),
                b'\\' => {
                    let escape = *self.bytes.get(self.pos).ok_or("Unterminated escape")?;
                    self.pos += 1;
                    match escape {
                        b'"' | b'\\' | b'/' => bytes.push(escape),
                        b'b' => bytes.push(8),
                        b'f' => bytes.push(12),
                        b'n' => bytes.push(b'\n'),
                        b'r' => bytes.push(b'\r'),
                        b't' => bytes.push(b'\t'),
                        b'u' => {
                            let first = self.hex_quad()?;
                            let scalar = if (0xD800..=0xDBFF).contains(&first) {
                                if self.bytes.get(self.pos..self.pos + 2) != Some(b"\\u") {
                                    return Err("Missing low Unicode surrogate".into());
                                }
                                self.pos += 2;
                                let second = self.hex_quad()?;
                                if !(0xDC00..=0xDFFF).contains(&second) {
                                    return Err("Invalid low Unicode surrogate".into());
                                }
                                0x10000 + ((first - 0xD800) << 10) + (second - 0xDC00)
                            } else {
                                first
                            };
                            let ch = char::from_u32(scalar).ok_or("Invalid Unicode scalar")?;
                            let mut buffer = [0; 4];
                            bytes.extend_from_slice(ch.encode_utf8(&mut buffer).as_bytes());
                        }
                        _ => return Err("Invalid JSON string escape".into()),
                    }
                }
                0..=31 => return Err("Unescaped control character in JSON string".into()),
                value => bytes.push(value),
            }
        }
        Err("Unterminated JSON string".into())
    }

    fn hex_quad(&mut self) -> Result<u32, String> {
        let raw = self
            .bytes
            .get(self.pos..self.pos + 4)
            .ok_or("Incomplete Unicode escape")?;
        if !raw.iter().all(u8::is_ascii_hexdigit) {
            return Err("Invalid Unicode escape".into());
        }
        let text = std::str::from_utf8(raw).map_err(|_| "Invalid Unicode escape")?;
        let value = u32::from_str_radix(text, 16).map_err(|_| "Invalid Unicode escape")?;
        self.pos += 4;
        Ok(value)
    }

    fn parse_num(&mut self) -> Result<JVal, String> {
        let start = self.pos;
        if self.bytes.get(self.pos) == Some(&b'-') {
            self.pos += 1;
        }
        match self.bytes.get(self.pos) {
            Some(b'0') => self.pos += 1,
            Some(b'1'..=b'9') => {
                self.pos += 1;
                while self.bytes.get(self.pos).is_some_and(u8::is_ascii_digit) {
                    self.pos += 1;
                }
            }
            _ => return Err("Invalid JSON number".into()),
        }
        if self.bytes.get(self.pos) == Some(&b'.') {
            self.pos += 1;
            let digits = self.pos;
            while self.bytes.get(self.pos).is_some_and(u8::is_ascii_digit) {
                self.pos += 1;
            }
            if digits == self.pos {
                return Err("Missing decimal digits".into());
            }
        }
        if self
            .bytes
            .get(self.pos)
            .is_some_and(|b| matches!(b, b'e' | b'E'))
        {
            self.pos += 1;
            if self
                .bytes
                .get(self.pos)
                .is_some_and(|b| matches!(b, b'+' | b'-'))
            {
                self.pos += 1;
            }
            let digits = self.pos;
            while self.bytes.get(self.pos).is_some_and(u8::is_ascii_digit) {
                self.pos += 1;
            }
            if digits == self.pos {
                return Err("Missing exponent digits".into());
            }
        }
        let text =
            std::str::from_utf8(&self.bytes[start..self.pos]).map_err(|_| "Invalid number")?;
        let value: f64 = text.parse().map_err(|_| "Invalid number")?;
        if !value.is_finite() {
            return Err("Non-finite JSON number".into());
        }
        Ok(JVal::Num(value))
    }
}

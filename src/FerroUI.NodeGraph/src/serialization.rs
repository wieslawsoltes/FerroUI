//! Zero-dependency, human-readable JSON serializer & deserializer for [`NodeGraph`].
//!
//! Saves and restores every node, position, slider/toggle/text/color widget, per-port Grasshopper
//! modifier (`F`/`G`/`R`/`S`), wire, colored group, and camera transform cleanly!

use crate::graph::NodeGraph;
use crate::node::NodeRegistry;
use crate::types::{NodeId, ParamModifier, ParamWidget};
use rustc_hash::FxHashMap;
use std::fmt::Write as _;

/// Serializes `graph` into clean, formatted JSON.
pub fn graph_to_json(graph: &NodeGraph) -> String {
    let mut out = String::with_capacity(2048);
    let _ = writeln!(out, "{{");
    let _ = writeln!(out, "  \"format\": \"ferroui-nodegraph/v1\",");
    let _ = writeln!(out, "  \"name\": \"{}\",", esc(&graph.name));
    let _ = writeln!(
        out,
        "  \"camera\": {{\"pan\": [{:.2}, {:.2}], \"zoom\": {:.4}}},",
        graph.camera_pan[0], graph.camera_pan[1], graph.camera_zoom
    );

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
                "{{\"kind\":\"slider\",\"value\":{value:.6},\"min\":{min:.6},\"max\":{max:.6},\"step\":{step:.6},\"integer\":{integer}}}"
            ),
            ParamWidget::Toggle { value } => format!("{{\"kind\":\"toggle\",\"value\":{value}}}"),
            ParamWidget::Color { argb } => format!("{{\"kind\":\"color\",\"argb\":{argb}}}"),
            ParamWidget::Text { text } => format!("{{\"kind\":\"text\",\"text\":\"{}\"}}", esc(text)),
        };
        let mods: Vec<String> = n
            .input_modifiers
            .iter()
            .map(|m| format!("\"{}\"", m.badge()))
            .collect();
        let _ = writeln!(
            out,
            "    {{\"id\":{},\"type\":\"{}\",\"title\":{},\"pos\":[{:.2},{:.2}],\"disabled\":{},\"widget\":{},\"in_mods\":[{}]}}{comma}",
            n.id.0,
            esc(&n.type_id),
            title_json,
            n.position[0],
            n.position[1],
            n.disabled,
            widget_json,
            mods.join(",")
        );
    }
    let _ = writeln!(out, "  ],");

    // Wires
    let _ = writeln!(out, "  \"wires\": [");
    for (idx, (_, w)) in graph.wires.iter().enumerate() {
        let comma = if idx + 1 < graph.wires.len() { "," } else { "" };
        let _ = writeln!(
            out,
            "    {{\"from\":[{},{}],\"to\":[{},{}]}}{comma}",
            w.from_node.0, w.from_port, w.to_node.0, w.to_port
        );
    }
    let _ = writeln!(out, "  ],");

    // Groups
    let _ = writeln!(out, "  \"groups\": [");
    for (idx, (_, g)) in graph.groups.iter().enumerate() {
        let comma = if idx + 1 < graph.groups.len() { "," } else { "" };
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

    let mut id_map: FxHashMap<u32, NodeId> = FxHashMap::default();
    if let Some(nodes) = obj.get("nodes").and_then(JVal::as_arr) {
        for nval in nodes {
            let Some(nobj) = nval.as_obj() else { continue };
            let old_id = nobj.get("id").and_then(JVal::as_f64).unwrap_or(0.0) as u32;
            let type_id = nobj.get("type").and_then(JVal::as_str).unwrap_or("");
            let pos = nobj.get("pos").and_then(JVal::as_arr);
            let x = pos.and_then(|p| p.first()).and_then(JVal::as_f64).unwrap_or(0.0);
            let y = pos.and_then(|p| p.get(1)).and_then(JVal::as_f64).unwrap_or(0.0);

            let Some(new_id) = g.add_node(registry, type_id, x, y) else {
                continue;
            };
            id_map.insert(old_id, new_id);

            if let Some(inst) = g.nodes.get_mut(&new_id) {
                if let Some(title) = nobj.get("title").and_then(JVal::as_str) {
                    inst.custom_title = Some(title.to_string());
                }
                if let Some(dis) = nobj.get("disabled").and_then(JVal::as_bool) {
                    inst.disabled = dis;
                }
                if let Some(wobj) = nobj.get("widget").and_then(JVal::as_obj) {
                    match wobj.get("kind").and_then(JVal::as_str) {
                        Some("slider") => {
                            inst.widget = ParamWidget::Slider {
                                value: wobj.get("value").and_then(JVal::as_f64).unwrap_or(0.0),
                                min: wobj.get("min").and_then(JVal::as_f64).unwrap_or(0.0),
                                max: wobj.get("max").and_then(JVal::as_f64).unwrap_or(10.0),
                                step: wobj.get("step").and_then(JVal::as_f64).unwrap_or(0.1),
                                integer: wobj.get("integer").and_then(JVal::as_bool).unwrap_or(false),
                            };
                        }
                        Some("toggle") => {
                            inst.widget = ParamWidget::Toggle {
                                value: wobj.get("value").and_then(JVal::as_bool).unwrap_or(false),
                            };
                        }
                        Some("color") => {
                            inst.widget = ParamWidget::Color {
                                argb: wobj.get("argb").and_then(JVal::as_f64).unwrap_or(0xFF38BDF8_u32 as f64) as u32,
                            };
                        }
                        Some("text") => {
                            inst.widget = ParamWidget::Text {
                                text: wobj.get("text").and_then(JVal::as_str).unwrap_or("").to_string(),
                            };
                        }
                        _ => {}
                    }
                }
                if let Some(mods) = nobj.get("in_mods").and_then(JVal::as_arr) {
                    for (idx, mv) in mods.iter().enumerate() {
                        if idx < inst.input_modifiers.len() {
                            if let Some(badge) = mv.as_str() {
                                inst.input_modifiers[idx] = ParamModifier {
                                    flatten: badge.contains('F'),
                                    graft: badge.contains('G'),
                                    reverse: badge.contains('R'),
                                    simplify: badge.contains('S'),
                                };
                            }
                        }
                    }
                }
            }
        }
    }

    if let Some(wires) = obj.get("wires").and_then(JVal::as_arr) {
        for wval in wires {
            let Some(wobj) = wval.as_obj() else { continue };
            let Some(from) = wobj.get("from").and_then(JVal::as_arr) else { continue };
            let Some(to) = wobj.get("to").and_then(JVal::as_arr) else { continue };
            if from.len() >= 2 && to.len() >= 2 {
                let fn_old = from[0].as_f64().unwrap_or(0.0) as u32;
                let fp = from[1].as_f64().unwrap_or(0.0) as usize;
                let tn_old = to[0].as_f64().unwrap_or(0.0) as u32;
                let tp = to[1].as_f64().unwrap_or(0.0) as usize;
                if let (Some(&fn_id), Some(&tn_id)) = (id_map.get(&fn_old), id_map.get(&tn_old)) {
                    let _ = g.connect(fn_id, fp, tn_id, tp, true);
                }
            }
        }
    }

    if let Some(groups) = obj.get("groups").and_then(JVal::as_arr) {
        for gval in groups {
            let Some(gobj) = gval.as_obj() else { continue };
            let title = gobj.get("title").and_then(JVal::as_str).unwrap_or("Group");
            let color = gobj.get("color").and_then(JVal::as_f64).unwrap_or(0x3338BDF8_u32 as f64) as u32;
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

    g.mark_all_dirty();
    Ok(g)
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
    let mut p = JsonParser { bytes: src.as_bytes(), pos: 0 };
    p.parse_val()
}

struct JsonParser<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> JsonParser<'a> {
    fn skip_ws(&mut self) {
        while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
    }

    fn peek(&mut self) -> Option<u8> {
        self.skip_ws();
        self.bytes.get(self.pos).copied()
    }

    fn parse_val(&mut self) -> Result<JVal, String> {
        match self.peek() {
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
        }
    }

    fn parse_obj(&mut self) -> Result<JVal, String> {
        self.pos += 1; // '{'
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
            let val = self.parse_val()?;
            map.insert(key, val);
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
        self.pos += 1; // '['
        let mut arr = Vec::new();
        if self.peek() == Some(b']') {
            self.pos += 1;
            return Ok(JVal::Arr(arr));
        }
        loop {
            arr.push(self.parse_val()?);
            match self.peek() {
                Some(b',') => self.pos += 1,
                Some(b']') => {
                    self.pos += 1;
                    return Ok(JVal::Arr(arr));
                }
                _ => return Err("Expected ',' or ']' in JSON array".into()),
            }
        }
    }

    fn parse_str(&mut self) -> Result<String, String> {
        if self.peek() != Some(b'"') {
            return Err("Expected '\"' starting string".into());
        }
        self.pos += 1;
        let mut s = String::new();
        while self.pos < self.bytes.len() {
            let b = self.bytes[self.pos];
            self.pos += 1;
            match b {
                b'"' => return Ok(s),
                b'\\' => {
                    if self.pos >= self.bytes.len() {
                        return Err("Unterminated escape".into());
                    }
                    let esc = self.bytes[self.pos];
                    self.pos += 1;
                    match esc {
                        b'"' => s.push('"'),
                        b'\\' => s.push('\\'),
                        b'/' => s.push('/'),
                        b'n' => s.push('\n'),
                        b'r' => s.push('\r'),
                        b't' => s.push('\t'),
                        _ => s.push(esc as char),
                    }
                }
                c => s.push(c as char),
            }
        }
        Err("Unterminated JSON string".into())
    }

    fn parse_num(&mut self) -> Result<JVal, String> {
        let start = self.pos;
        while self.pos < self.bytes.len() {
            let b = self.bytes[self.pos];
            if b.is_ascii_digit() || matches!(b, b'-' | b'+' | b'.' | b'e' | b'E') {
                self.pos += 1;
            } else {
                break;
            }
        }
        let s = std::str::from_utf8(&self.bytes[start..self.pos]).map_err(|_| "UTF-8 error")?;
        s.parse::<f64>().map(JVal::Num).map_err(|_| format!("Bad JSON number `{s}`"))
    }
}

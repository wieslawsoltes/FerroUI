//! Translate compositor state into the shell's OS-independent model. Workspace IDs and
//! logical rectangles come from Sway; output names and titles are display data.
use super::super::{ClientWindow, Snapshot};
use super::ipc::{self, invalid};
use serde_json::Value;
use std::{collections::HashMap, io, os::unix::net::UnixStream};

#[derive(Clone, Copy, Default, Debug, Eq, PartialEq)]
pub(super) struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Rect {
    pub fn valid(self) -> bool {
        self.width > 0 && self.height > 0
    }
    pub fn half(mut self, right: bool) -> Self {
        let half = self.width / 2;
        if right {
            self.x = self.x.saturating_add(half);
            self.width -= half;
        } else {
            self.width = half;
        }
        self
    }
}

fn rect(value: &Value) -> Rect {
    let get = |k: &str| {
        value[k]
            .as_i64()
            .and_then(|n| n.try_into().ok())
            .unwrap_or_default()
    };
    Rect {
        x: get("x"),
        y: get("y"),
        width: get("width"),
        height: get("height"),
    }
}

#[derive(Clone, Debug)]
pub(super) struct Workspace {
    pub id: u64,
    pub name: String,
    pub number: i64,
    pub output: String,
    pub area: Rect,
}

impl Workspace {
    pub fn target(&self, spaces: &[Self]) -> Option<String> {
        // Sway's number selector avoids interpolating user text and works even with
        // names containing quotes, separators and variables.
        if self.number >= 0 && spaces.iter().filter(|w| w.number == self.number).count() == 1 {
            return Some(format!("number {}", self.number));
        }
        // A literal quote of the other kind is allowed inside each delimiter. Sway's
        // strip_quotes retains backslashes; its variable expander runs after stripping.
        // Refuse cases which cannot round-trip exactly rather than target a different desk.
        if self.name.is_empty()
            || self
                .name
                .chars()
                .any(|c| c.is_control() || matches!(c, '$' | '\\'))
            || [
                "next",
                "prev",
                "next_on_output",
                "prev_on_output",
                "current",
                "back_and_forth",
                "number",
                "--no-auto-back-and-forth",
            ]
            .iter()
            .any(|s| self.name.eq_ignore_ascii_case(s))
        {
            return None;
        }
        let quote = if !self.name.contains('"') {
            '"'
        } else if !self.name.contains('\'') {
            '\''
        } else {
            return None;
        };
        Some(format!("{quote}{}{quote}", self.name))
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) struct Frame {
    pub workspace: Option<usize>,
    pub rect: Rect,
    pub floating: bool,
}

#[derive(Default)]
pub(super) struct Tree {
    pub snapshot: Snapshot,
    pub spaces: Vec<Workspace>,
    pub frames: HashMap<u32, Frame>,
}

impl Tree {
    pub fn read(requests: &mut UnixStream) -> io::Result<Self> {
        let spaces = ipc::request(requests, ipc::WORKSPACES, "")?;
        let tree = ipc::request(requests, ipc::TREE, "")?;
        Self::parse(&spaces, &tree)
    }

    pub fn parse(workspaces: &Value, root: &Value) -> io::Result<Self> {
        let mut result = Tree::default();
        for item in workspaces
            .as_array()
            .ok_or_else(|| invalid("Invalid workspace list"))?
        {
            let (Some(id), Some(name), Some(output)) = (
                item["id"].as_u64(),
                item["name"].as_str(),
                item["output"].as_str(),
            ) else {
                continue;
            };
            if item["focused"].as_bool() == Some(true) {
                result.snapshot.current_workspace = result.spaces.len();
            }
            result.spaces.push(Workspace {
                id,
                name: name.into(),
                number: item["num"].as_i64().unwrap_or(-1),
                output: output.into(),
                area: rect(&item["rect"]),
            });
        }
        if root["type"] != "root" {
            return Err(invalid("Invalid compositor tree"));
        }
        if !result.spaces.is_empty() {
            result.snapshot.workspace_names =
                result.spaces.iter().map(|w| w.name.clone()).collect();
        }
        result.walk(root, None, false, false);
        Ok(result)
    }

    fn walk(
        &mut self,
        node: &Value,
        mut workspace: Option<usize>,
        floating: bool,
        mut hidden: bool,
    ) {
        let kind = node["type"].as_str();
        if kind == Some("workspace") {
            hidden = node["name"] == "__i3_scratch";
            workspace = self
                .spaces
                .iter()
                .position(|w| Some(w.id) == node["id"].as_u64());
        }
        let is_view =
            node["shell"].is_string() || node["app_id"].is_string() || node["window"].is_number();
        if matches!(kind, Some("con" | "floating_con"))
            && is_view
            && node["pid"].as_u64() != Some(u64::from(std::process::id()))
        {
            if let Some(id) = node["id"].as_u64().and_then(|id| u32::try_from(id).ok()) {
                let frame = Frame {
                    rect: rect(&node["rect"]),
                    workspace,
                    floating,
                };
                let fullscreen = node["fullscreen_mode"].as_u64().unwrap_or(0) != 0;
                let can_place = !fullscreen && !hidden && workspace.is_some();
                self.frames.insert(id, frame);
                self.snapshot.windows.push(ClientWindow {
                    id,
                    title: node["name"].as_str().unwrap_or("Window").to_owned(),
                    workspace: if node["sticky"] == true {
                        None
                    } else {
                        workspace
                    },
                    active: node["focused"] == true && !hidden,
                    minimized: hidden,
                    maximized: false,
                    partially_maximized: false,
                    fullscreen,
                    can_fullscreen: !hidden,
                    above: false,
                    can_raise: false,
                    last_activation: 0,
                    can_maximize: can_place,
                    can_snap: can_place,
                    can_move_workspace: !hidden,
                    can_minimize: !hidden,
                    can_close: true,
                });
            }
        }
        for (key, float_children) in [("nodes", floating), ("floating_nodes", true)] {
            if let Some(children) = node[key].as_array() {
                for child in children {
                    self.walk(child, workspace, float_children, hidden);
                }
            }
        }
    }
}

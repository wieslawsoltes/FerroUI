use ferroui_designer_language::{Position, Range};
use serde::{Deserialize, Serialize};

pub(crate) const PROTOCOL_VERSION: u32 = 4;
pub(crate) const MAX_SOURCE_BYTES: usize = 1024 * 1024;

#[derive(Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Language {
    Xaml,
    Rust,
}

#[derive(PartialEq, Deserialize)]
pub(crate) struct Source {
    pub(crate) language: Language,
    pub(crate) uri: String,
    pub(crate) source: String,
    pub(crate) caret: Position,
    pub(crate) project: Option<u64>,
}

#[derive(Deserialize)]
pub(crate) struct Render {
    #[serde(flatten)]
    pub(crate) document: Source,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) dark: bool,
    pub(crate) frame: u64,
    #[serde(default, rename = "workspaceRoot")]
    pub(crate) workspace_root: Option<std::path::PathBuf>,
}

#[derive(Deserialize)]
pub(crate) struct EditRequest {
    #[serde(flatten)]
    pub(crate) document: Source,
    pub(crate) operation: Operation,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub(crate) enum Operation {
    SetProperty {
        node: u64,
        name: String,
        value: String,
    },
    ResetProperty { node: u64, name: String },
    SetEvent { node: u64, name: String, value: Option<String> },
    ApplyResource { frame: u64, node: u64, resource: u64, property: String, dynamic: bool },
    RemoveNode {
        node: u64,
    },
    Place {
        frame: u64,
        placement: Placement,
    },
    Resize { frame: u64, node: u64, handle: u8, dx: f64, dy: f64 },
    Apply {
        label: String,
        edits: Vec<ferroui_designer_core::Edit>,
    },
    ApplyCounterpart {
        source: String,
    },
}

#[derive(Deserialize)]
pub(crate) struct PlacementRequest {
    pub(crate) frame: u64,
    pub(crate) placement: Placement,
}

#[derive(Deserialize)]
pub(crate) struct Placement {
    pub(crate) origin: DragOrigin,
    pub(crate) target: DropTarget,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub(crate) enum DragOrigin {
    Asset { control: String },
    Image { source: String },
    Node { node: u64, grab: Option<ferroui_designer_tools::Point> },
    Nodes { nodes: Vec<u64>, anchor: u64, grab: Option<ferroui_designer_tools::Point> },
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub(crate) enum DropTarget {
    Canvas { x: f64, y: f64 },
    Outline { node: u64, position: DropPosition },
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum DropPosition { Inside, Before, After }

#[derive(Serialize)]
pub(crate) struct DropHint {
    pub(crate) bounds: Option<ferroui_designer_tools::Rect>,
    pub(crate) description: String,
}

#[derive(Serialize)]
pub(crate) struct EditResult {
    pub(crate) edits: Vec<ferroui_designer_language::TextEdit>,
    pub(crate) selection: Option<Range>,
}

#[derive(Clone, Serialize)]
pub(crate) struct Asset {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) category: String,
    pub(crate) description: String,
    pub(crate) icon: String,
    pub(crate) rust: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Snapshot {
    pub(crate) nodes: Vec<OutlineNode>,
    pub(crate) blocks: Vec<Range>,
    pub(crate) active_block: Option<Range>,
    pub(crate) counterpart: Counterpart,
    pub(crate) image: Option<Image>,
    pub(crate) render_error: Option<String>,
    pub(crate) notices: Vec<String>,
    pub(crate) project_assets: Vec<Asset>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OutlineNode {
    pub(crate) id: u64,
    pub(crate) parent: Option<u64>,
    pub(crate) label: String,
    pub(crate) name: Option<String>,
    pub(crate) range: Range,
    pub(crate) name_range: Range,
    pub(crate) bounds: Option<Bounds>,
    pub(crate) properties: Vec<Property>,
    pub(crate) addable_properties: Vec<String>,
    pub(crate) can_insert: bool,
    pub(crate) can_remove: bool,
    pub(crate) can_move: bool,
}

#[derive(Clone, Copy, Serialize)]
pub(crate) struct Bounds {
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) width: f64,
    pub(crate) height: f64,
}

#[derive(Serialize)]
pub(crate) struct Property {
    pub(crate) name: String,
    pub(crate) value: String,
    pub(crate) editable: bool,
}

#[derive(Serialize)]
pub(crate) struct Image {
    pub(crate) png: String,
    pub(crate) width: u32,
    pub(crate) height: u32,
}

#[derive(Serialize)]
pub(crate) struct Counterpart {
    pub(crate) source: Option<String>,
    pub(crate) editable: bool,
    pub(crate) reason: Option<String>,
}

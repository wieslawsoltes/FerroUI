use ferroui_designer_language::{Position, Range};
use serde::{Deserialize, Serialize};

pub(crate) const PROTOCOL_VERSION: u32 = 1;
pub(crate) const MAX_SOURCE_BYTES: usize = 1024 * 1024;

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Language {
    Xaml,
    Rust,
}

#[derive(Deserialize)]
pub(crate) struct Source {
    pub(crate) language: Language,
    pub(crate) uri: String,
    pub(crate) source: String,
    pub(crate) caret: Position,
}

#[derive(Deserialize)]
pub(crate) struct Render {
    #[serde(flatten)]
    pub(crate) document: Source,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) dark: bool,
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
    InsertChild {
        node: u64,
        control: String,
    },
    RemoveNode {
        node: u64,
    },
    ApplyCounterpart {
        source: String,
    },
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

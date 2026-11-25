//! Immutable, renderer-independent provenance for the UI recorded in a layer.

use crate::SvgError;
use ferroui_base::{Matrix, Rect, Size, Thickness};
use std::collections::{BTreeMap, HashMap};

/// Namespace of the version-one UI metadata vocabulary. It is an identifier, not a resource
/// fetched while viewing an SVG. Semantic metadata never introduces scripts or external assets.
pub const SVG_SEMANTIC_NAMESPACE: &str = "https://wieslawsoltes.github.io/FerroUI/svg/semantics/1";

/// How captured UI metadata is projected into an SVG. Ordinary exports remain unchanged.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SvgSemanticExportMode {
    #[default]
    None,
    /// A versioned, namespaced manifest inside each rendered layer's `<metadata>` element.
    Metadata,
    /// Also annotate the existing painted layer with `data-ui-*` summary attributes.
    /// This does not create interactive SVG controls or claim ownership of individual paths.
    MetadataAndAttributes,
}

/// Independent export-time projection of a previously captured snapshot.
/// Text, accessibility content and source locations require explicit opt-in at export time.
#[derive(Clone, Debug)]
pub struct SvgSemanticExportOptions {
    pub mode: SvgSemanticExportMode,
    pub include_names: bool,
    pub include_classes: bool,
    pub include_layout: bool,
    pub include_input_state: bool,
    pub include_relations: bool,
    pub include_properties: bool,
    pub include_text: bool,
    pub include_accessibility: bool,
    pub include_source: bool,
}

impl Default for SvgSemanticExportOptions {
    fn default() -> Self {
        Self {
            mode: SvgSemanticExportMode::None,
            include_names: true,
            include_classes: true,
            include_layout: true,
            include_input_state: true,
            include_relations: true,
            include_properties: true,
            include_text: false,
            include_accessibility: false,
            include_source: false,
        }
    }
}

impl SvgSemanticExportOptions {
    /// Structural metadata, with display text, accessibility and source fields excluded.
    pub fn metadata() -> Self {
        Self {
            mode: SvgSemanticExportMode::Metadata,
            ..Self::default()
        }
    }

    /// Export every field explicitly supplied in the snapshot, plus layer summary attributes.
    pub fn all() -> Self {
        Self {
            mode: SvgSemanticExportMode::MetadataAndAttributes,
            include_text: true,
            include_accessibility: true,
            include_source: true,
            ..Self::default()
        }
    }
}

/// Application-supplied values such as design tokens, resource keys, binding descriptions,
/// domain identifiers or test selectors. The exporter never evaluates bindings or commands.
#[derive(Clone, Debug, PartialEq)]
pub enum SvgSemanticValue {
    String(String),
    Number(f64),
    Bool(bool),
    StringList(Vec<String>),
}

/// An authored source location. No source file is opened or embedded by the exporter.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SvgSemanticSource {
    pub uri: String,
    /// One-based line and column when known.
    pub line: Option<u32>,
    pub column: Option<u32>,
}

/// Source-UI accessibility descriptors. These are data, not interactive SVG ARIA roles.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SvgSemanticAccessibility {
    pub automation_id: Option<String>,
    pub name: Option<String>,
    pub help_text: Option<String>,
    pub role: Option<String>,
    pub landmark_type: Option<String>,
    pub live_setting: Option<String>,
    pub required: Option<bool>,
    pub heading_level: Option<i32>,
    pub position_in_set: Option<i32>,
    pub size_of_set: Option<i32>,
}

/// Arranged UI layout, independent of the SVG's painted path geometry.
/// `None` width/height means Auto; `None` maximum means unbounded.
#[derive(Clone, Debug)]
pub struct SvgSemanticLayout {
    pub desired_size: Size,
    pub margin: Thickness,
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub min_width: f64,
    pub min_height: f64,
    pub max_width: Option<f64>,
    pub max_height: Option<f64>,
    pub horizontal_alignment: String,
    pub vertical_alignment: String,
    pub measure_valid: bool,
    pub arrange_valid: bool,
    /// Effective placement when the visual's immediate parent is a Grid.
    pub grid: Option<SvgSemanticGridPlacement>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SvgSemanticGridPlacement {
    pub row: i32,
    pub column: i32,
    pub row_span: i32,
    pub column_span: i32,
}

/// Source visual/input state. Visibility and layout bounds do not imply exact visible ink,
/// occlusion, clipping geometry or a hit-test result.
#[derive(Clone, Debug)]
pub struct SvgSemanticState {
    pub visible: bool,
    pub effectively_visible: bool,
    pub render_suppressed: bool,
    pub opacity: f64,
    pub z_index: i32,
    pub clip_to_bounds: bool,
    pub has_clip: bool,
    pub has_opacity_mask: bool,
    pub has_effect: bool,
    pub enabled: Option<bool>,
    pub effectively_enabled: Option<bool>,
    pub focusable: Option<bool>,
    pub focused: Option<bool>,
    pub hit_test_visible: Option<bool>,
    pub is_tab_stop: Option<bool>,
    pub tab_index: Option<i32>,
}

impl Default for SvgSemanticState {
    fn default() -> Self {
        Self {
            visible: true,
            effectively_visible: true,
            render_suppressed: false,
            opacity: 1.0,
            z_index: 0,
            clip_to_bounds: false,
            has_clip: false,
            has_opacity_mask: false,
            has_effect: false,
            enabled: None,
            effectively_enabled: None,
            focusable: None,
            focused: None,
            hit_test_visible: None,
            is_tab_stop: None,
            tab_index: None,
        }
    }
}

/// One visual or application-defined semantic node.
#[derive(Clone, Debug)]
pub struct SvgSemanticNode {
    /// Stable application key, or the capture API's structural traversal key. Unique per tree.
    pub id: String,
    /// Visual-tree parent key. Parents must precede children in a constructed snapshot.
    pub parent: Option<String>,
    pub type_name: String,
    pub name: Option<String>,
    pub classes: Vec<String>,
    /// Layout bounds in the source visual parent's layout space, before render transforms.
    pub layout_bounds: Rect,
    /// Row-vector mapping from the node's local coordinates into its recorded layer.
    /// All nine coefficients are retained, including perspective when present.
    pub local_to_layer: Matrix,
    pub state: SvgSemanticState,
    pub layout: Option<SvgSemanticLayout>,
    pub accessibility: Option<SvgSemanticAccessibility>,
    /// Explicitly captured display text, independent of outlined/editable drawing output.
    pub text: Option<String>,
    pub source: Option<SvgSemanticSource>,
    /// Relationships are included only when their target is present in this snapshot.
    pub logical_parent: Option<String>,
    pub templated_parent: Option<String>,
    pub properties: BTreeMap<String, SvgSemanticValue>,
}

impl SvgSemanticNode {
    pub fn new(id: impl Into<String>, type_name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            parent: None,
            type_name: type_name.into(),
            name: None,
            classes: Vec::new(),
            layout_bounds: Rect::default(),
            local_to_layer: Matrix::IDENTITY,
            state: SvgSemanticState::default(),
            layout: None,
            accessibility: None,
            text: None,
            source: None,
            logical_parent: None,
            templated_parent: None,
            properties: BTreeMap::new(),
        }
    }
}

/// A validated immutable snapshot with no retained UI references or callbacks.
/// Its visual tree may be empty or have several roots. Logical/template links are separate
/// relationships and are never substituted for visual ancestry.
#[derive(Clone, Debug, Default)]
pub struct SvgSemanticTree {
    nodes: Vec<SvgSemanticNode>,
}

impl SvgSemanticTree {
    pub const MAX_NODES: usize = 100_000;
    /// Allows structural keys through the capture API's default 256-level depth limit.
    pub const MAX_KEY_BYTES: usize = 4096;
    pub const MAX_TEXT_BYTES: usize = 16 * 1024 * 1024;
    /// Aggregate string/value entries, including empty list entries. This bounds XML expansion
    /// independently of the UTF-8 byte budget.
    pub const MAX_VALUES: usize = 500_000;

    pub fn new(nodes: Vec<SvgSemanticNode>) -> Result<Self, SvgError> {
        let tree = Self { nodes };
        tree.validate()?;
        Ok(tree)
    }

    pub fn nodes(&self) -> &[SvgSemanticNode] {
        &self.nodes
    }
    pub fn len(&self) -> usize {
        self.nodes.len()
    }
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Map the whole snapshot to a different recording coordinate system. For example, a
    /// compositor frame recorded at 2x DPI needs a logical-to-device scale of 2 here.
    /// Local layout bounds retain their original layout units.
    pub fn transformed(mut self, transform: Matrix) -> Result<Self, SvgError> {
        if !matrix_is_finite(transform) {
            return Err(invalid("nonfinite semantic mapping"));
        }
        for node in &mut self.nodes {
            node.local_to_layer *= transform;
        }
        self.validate()?;
        Ok(self)
    }

    fn validate(&self) -> Result<(), SvgError> {
        if self.nodes.len() > Self::MAX_NODES {
            return Err(invalid("too many semantic nodes"));
        }
        let mut ids = HashMap::with_capacity(self.nodes.len());
        let mut budget = SemanticTextBudget::default();
        for (index, node) in self.nodes.iter().enumerate() {
            budget.observe(node)?;
            if node.id.is_empty()
                || node.id.len() > Self::MAX_KEY_BYTES
                || !node
                    .id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"_.-".contains(&b))
            {
                return Err(invalid(
                    "semantic keys require 1..=4096 ASCII letters, digits, '.', '_' or '-'",
                ));
            }
            if let Some(parent) = &node.parent {
                if !ids.contains_key(parent.as_str()) {
                    return Err(invalid("semantic parents must precede their children"));
                }
            }
            if ids.insert(node.id.as_str(), index).is_some() {
                return Err(invalid("duplicate semantic key"));
            }
            if node.type_name.is_empty() {
                return Err(invalid("semantic type name must not be empty"));
            }
            let r = node.layout_bounds;
            if ![r.x, r.y, r.width, r.height, r.right(), r.bottom()]
                .iter()
                .all(|n| n.is_finite())
                || r.width < 0.0
                || r.height < 0.0
                || !matrix_is_finite(node.local_to_layer)
            {
                return Err(invalid(
                    "semantic bounds and transforms must be finite; sizes must be nonnegative",
                ));
            }
            if !node.state.opacity.is_finite() || !(0.0..=1.0).contains(&node.state.opacity) {
                return Err(invalid("semantic opacity must be in 0..=1"));
            }
            if let Some(layout) = &node.layout {
                for value in [
                    layout.desired_size.width,
                    layout.desired_size.height,
                    layout.min_width,
                    layout.min_height,
                ]
                .into_iter()
                .chain(
                    [
                        layout.width,
                        layout.height,
                        layout.max_width,
                        layout.max_height,
                    ]
                    .into_iter()
                    .flatten(),
                ) {
                    if !value.is_finite() || value < 0.0 {
                        return Err(invalid(
                            "semantic layout sizes must be finite and nonnegative",
                        ));
                    }
                }
                if ![
                    layout.margin.left,
                    layout.margin.top,
                    layout.margin.right,
                    layout.margin.bottom,
                ]
                .iter()
                .all(|n| n.is_finite())
                {
                    return Err(invalid("semantic margins must be finite"));
                }
                if let Some(grid) = &layout.grid {
                    if grid.row < 0
                        || grid.column < 0
                        || grid.row_span <= 0
                        || grid.column_span <= 0
                    {
                        return Err(invalid(
                            "semantic grid indices must be nonnegative and spans positive",
                        ));
                    }
                }
            }
            if let Some(source) = &node.source {
                if source.line == Some(0) || source.column == Some(0) {
                    return Err(invalid("source lines and columns are one-based"));
                }
            }
            for (key, value) in &node.properties {
                if key.is_empty() {
                    return Err(invalid("semantic property keys must not be empty"));
                }
                match value {
                    SvgSemanticValue::Number(value) if !value.is_finite() => {
                        return Err(invalid("semantic property numbers must be finite"))
                    }
                    _ => {}
                }
            }
        }
        for node in &self.nodes {
            for target in node
                .logical_parent
                .iter()
                .chain(node.templated_parent.iter())
            {
                if !ids.contains_key(target.as_str()) {
                    return Err(invalid("unresolved semantic relationship"));
                }
            }
        }
        // Logical and template ownership are ancestry graphs as well. Validate each iteratively
        // so custom metadata cannot introduce self-links, cycles, or recursive stack growth.
        for template in [false, true] {
            let links: Vec<_> = self
                .nodes
                .iter()
                .map(|node| {
                    let target = if template {
                        &node.templated_parent
                    } else {
                        &node.logical_parent
                    };
                    target.as_ref().map(|key| ids[key.as_str()])
                })
                .collect();
            let mut marks = vec![0u8; self.nodes.len()];
            for start in 0..self.nodes.len() {
                let mut current = Some(start);
                while let Some(index) = current {
                    match marks[index] {
                        2 => break,
                        1 => return Err(invalid("cyclic semantic ancestry")),
                        _ => marks[index] = 1,
                    }
                    current = links[index];
                }
                current = Some(start);
                while let Some(index) = current {
                    if marks[index] != 1 {
                        break;
                    }
                    marks[index] = 2;
                    current = links[index];
                }
            }
        }
        Ok(())
    }
}

/// Shared by final snapshot validation and incremental automatic capture. A large source tree
/// stops collection as soon as its text budget is exceeded, rather than after copying all nodes.
#[derive(Default)]
pub(crate) struct SemanticTextBudget {
    bytes: usize,
    values: usize,
}

impl SemanticTextBudget {
    fn text(&mut self, value: &str) -> Result<(), SvgError> {
        self.values += 1;
        if self.values > SvgSemanticTree::MAX_VALUES {
            return Err(invalid("semantic value budget exceeded"));
        }
        if value.chars().any(|c| !matches!(c, '\t' | '\n' | '\r' | '\u{20}'..='\u{d7ff}' | '\u{e000}'..='\u{fffd}' | '\u{10000}'..='\u{10ffff}')) {
            return Err(invalid("semantic strings must contain XML 1.0 characters"));
        }
        self.bytes = self
            .bytes
            .checked_add(value.len())
            .ok_or_else(|| invalid("semantic text budget overflow"))?;
        if self.bytes > SvgSemanticTree::MAX_TEXT_BYTES {
            return Err(invalid("semantic text budget exceeded"));
        }
        Ok(())
    }

    pub(crate) fn observe(&mut self, node: &SvgSemanticNode) -> Result<(), SvgError> {
        for value in [&node.id, &node.type_name]
            .into_iter()
            .chain(node.parent.iter())
            .chain(node.name.iter())
            .chain(node.classes.iter())
            .chain(node.text.iter())
            .chain(node.logical_parent.iter())
            .chain(node.templated_parent.iter())
        {
            self.text(value)?;
        }
        if let Some(layout) = &node.layout {
            self.text(&layout.horizontal_alignment)?;
            self.text(&layout.vertical_alignment)?;
        }
        if let Some(accessibility) = &node.accessibility {
            for value in [
                &accessibility.automation_id,
                &accessibility.name,
                &accessibility.help_text,
                &accessibility.role,
                &accessibility.landmark_type,
                &accessibility.live_setting,
            ]
            .into_iter()
            .flatten()
            {
                self.text(value)?;
            }
        }
        if let Some(source) = &node.source {
            self.text(&source.uri)?;
        }
        for (key, value) in &node.properties {
            self.text(key)?;
            match value {
                SvgSemanticValue::String(value) => self.text(value)?,
                SvgSemanticValue::StringList(values) => {
                    for value in values {
                        self.text(value)?;
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }
}

fn invalid(message: &str) -> SvgError {
    SvgError::InvalidLayer(format!("UI semantics: {message}"))
}

fn matrix_is_finite(m: Matrix) -> bool {
    [
        m.m11, m.m12, m.m13, m.m21, m.m22, m.m23, m.m31, m.m32, m.m33,
    ]
    .iter()
    .all(|n| n.is_finite())
}

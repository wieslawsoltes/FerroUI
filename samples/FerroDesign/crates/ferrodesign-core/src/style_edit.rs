//! Local style definitions and bindings are one authoring transaction.
//!
//! Nodes retain their effective values for portable documents. Definition edits
//! update linked values, including locked layers, without adding instance
//! overrides. Explicit application/detachment respects effective locks and marks
//! instance overrides. Legacy bindings whose values were independently edited
//! are detached on propagation rather than losing those edits.
use crate::{
    Document, DocumentError, Editor, LayoutGrid, Node, NodeId, NodeKind, Paint, Style, StyleId, StyleValue,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LocalStyleKind {
    Fill,
    Text,
    Effect,
    Grid,
}

impl LocalStyleKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Fill => "Fill",
            Self::Text => "Text",
            Self::Effect => "Effects",
            Self::Grid => "Grid",
        }
    }

    pub fn accepts(self, node: &Node) -> bool {
        match self {
            Self::Text => node.kind == NodeKind::Text,
            Self::Grid => LayoutGrid::supports(node.kind),
            _ => true,
        }
    }

    fn binding(self, node: &Node) -> Option<StyleId> {
        match self {
            Self::Fill => node.fill_style,
            Self::Text => node.text_style,
            Self::Effect => node.effect_style,
            Self::Grid => node.grid_style,
        }
    }

    fn bind(self, node: &mut Node, id: Option<StyleId>) {
        match self {
            Self::Fill => node.fill_style = id,
            Self::Text => node.text_style = id,
            Self::Effect => node.effect_style = id,
            Self::Grid => node.grid_style = id,
        }
    }

    fn same_node_values(self, before: &Node, after: &Node) -> bool {
        match self {
            Self::Fill => before.fills == after.fills,
            Self::Text => {
                after.kind == NodeKind::Text
                    && before.font_family == after.font_family
                    && before.font_size == after.font_size
                    && before.font_weight == after.font_weight
                    && before.line_height == after.line_height
                    && before.letter_spacing == after.letter_spacing
            }
            Self::Effect => before.effects == after.effects,
            Self::Grid => grid_values(before) == grid_values(after),
        }
    }

    fn mark_override(self, node: &mut Node) {
        match self {
            Self::Fill => {
                node.overridden_properties.insert("fills".into());
            }
            Self::Effect => {
                node.overridden_properties.insert("effects".into());
            }
            Self::Text => node.overridden_properties.extend(
                [
                    "font_family",
                    "font_size",
                    "font_weight",
                    "line_height",
                    "letter_spacing",
                    "text_style",
                ]
                .map(str::to_string),
            ),
            Self::Grid => {
                node.overridden_properties.insert("layout_grid".into());
            }
        }
    }
}

fn invalid(message: impl Into<String>) -> DocumentError {
    DocumentError::Invalid(message.into())
}

fn finite_point(point: crate::Point) -> bool {
    point.x.is_finite() && point.y.is_finite()
}

fn validate_stops(stops: &[crate::GradientStop]) -> Result<(), DocumentError> {
    if stops.is_empty()
        || stops
            .iter()
            .any(|stop| !stop.offset.is_finite() || !(0.0..=1.0).contains(&stop.offset))
    {
        return Err(invalid(
            "Style gradients require stops with finite offsets from 0 to 1.",
        ));
    }
    if stops.windows(2).any(|pair| pair[0].offset > pair[1].offset) {
        return Err(invalid(
            "Style gradient stops must be in increasing offset order.",
        ));
    }
    Ok(())
}

impl StyleValue {
    pub fn kind(&self) -> LocalStyleKind {
        match self {
            Self::Fill { .. } => LocalStyleKind::Fill,
            Self::Text { .. } => LocalStyleKind::Text,
            Self::Effect { .. } => LocalStyleKind::Effect,
            Self::Grid { .. } | Self::GridStack { .. } => LocalStyleKind::Grid,
        }
    }

    /// Capture the complete supported style payload, preserving stacked paints,
    /// gradient stops, image crops and effects. Character overrides stay on text.
    pub fn from_node(kind: LocalStyleKind, node: &Node) -> Result<Self, DocumentError> {
        if !kind.accepts(node) {
            return Err(invalid(format!(
                "{} styles cannot be captured from this layer.",
                kind.label()
            )));
        }
        let value = match kind {
            LocalStyleKind::Fill => Self::Fill {
                fills: node.fills.clone(),
            },
            LocalStyleKind::Text => Self::Text {
                family: node.font_family.clone(),
                size: node.font_size,
                weight: node.font_weight,
                line_height: node.line_height,
                letter_spacing: node.letter_spacing,
            },
            LocalStyleKind::Effect => Self::Effect {
                effects: node.effects.clone(),
            },
            LocalStyleKind::Grid => {
                let grid = node.layout_grid.unwrap_or_default();
                if node.layout_grid_overlays.is_empty() && grid.pattern.is_square() {
                    Self::Grid { size: grid.size, color: grid.color }
                } else {
                    Self::GridStack { grids: grid_values(node) }
                }
            }
        };
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> Result<(), DocumentError> {
        match self {
            Self::Fill { fills } => {
                for fill in fills {
                    if !fill.opacity.is_finite() || !(0.0..=1.0).contains(&fill.opacity) {
                        return Err(invalid(
                            "Style paint opacity must be a finite value from 0 to 1.",
                        ));
                    }
                    match &fill.paint {
                        Paint::Solid(_) => {}
                        Paint::LinearGradient { start, end, stops } => {
                            if !finite_point(*start) || !finite_point(*end) {
                                return Err(invalid("Style gradient coordinates must be finite."));
                            }
                            validate_stops(stops)?;
                        }
                        Paint::RadialGradient {
                            center,
                            radius,
                            stops,
                        } => {
                            if !finite_point(*center) || !radius.is_finite() || *radius <= 0.0 {
                                return Err(invalid(
                                    "Style gradient radius must be positive and finite.",
                                ));
                            }
                            validate_stops(stops)?;
                        }
                        Paint::AngularGradient { center, angle, stops } => {
                            if !finite_point(*center) || !angle.is_finite() {
                                return Err(invalid("Style angular gradient center and angle must be finite."));
                            }
                            validate_stops(stops)?;
                        }
                        Paint::Image { href, crop, .. } => {
                            if href.trim().is_empty()
                                || crop.is_some_and(|crop| !crate::valid_image_crop(crop))
                            {
                                return Err(invalid("Style image paints require an image reference and a valid source window."));
                            }
                        }
                    }
                }
            }
            Self::Text {
                family,
                size,
                weight,
                line_height,
                letter_spacing,
            } => {
                if family.trim().is_empty()
                    || family.len() > 1024
                    || !size.is_finite()
                    || *size <= 0.0
                    || !(1..=1000).contains(weight)
                    || !line_height.is_finite()
                    || *line_height < 0.0
                    || !letter_spacing.is_finite()
                {
                    return Err(invalid("Text styles require a font family, positive finite size, weight 1–1000, nonnegative finite line height and finite letter spacing."));
                }
            }
            Self::Effect { effects } => crate::validate_effect_stack(effects)?,
            Self::Grid { size, .. } if !size.is_finite() || *size <= 0.0 => {
                return Err(invalid("Grid style size must be positive and finite."))
            }
            Self::Grid { .. } => {}
            Self::GridStack { grids } => crate::validate_layout_grids(grids)?,
        }
        Ok(())
    }

    fn matches_node(&self, node: &Node) -> bool {
        match self {
            Self::Fill { fills } => node.fills == *fills,
            Self::Text {
                family,
                size,
                weight,
                line_height,
                letter_spacing,
            } => {
                node.kind == NodeKind::Text
                    && node.font_family == *family
                    && node.font_size == *size
                    && node.font_weight == *weight
                    && node.line_height == *line_height
                    && node.letter_spacing == *letter_spacing
            }
            Self::Effect { effects } => node.effects == *effects,
            Self::Grid { size, color } => grid_values(node) == vec![LayoutGrid { size: *size, color: *color, ..LayoutGrid::default() }],
            Self::GridStack { grids } => grid_values(node) == normalized_grids(grids.iter().copied()),
        }
    }

    fn write_node(&self, node: &mut Node) {
        match self {
            Self::Fill { fills } => node.fills.clone_from(fills),
            Self::Text {
                family,
                size,
                weight,
                line_height,
                letter_spacing,
            } => {
                node.font_family.clone_from(family);
                node.font_size = *size;
                node.font_weight = *weight;
                node.line_height = *line_height;
                node.letter_spacing = *letter_spacing;
            }
            Self::Effect { effects } => node.effects.clone_from(effects),
            Self::Grid { size, color } => {
                let visible = node.layout_grid.map_or(true, |grid| grid.visible);
                node.layout_grid = Some(LayoutGrid { size: *size, color: *color, visible, ..LayoutGrid::default() });
                node.layout_grid_overlays.clear();
                crate::layout_guides::prune_layout_guide_bindings(node);
            }
            Self::GridStack { grids } => {
                let previous: Vec<_> = node.layout_guides().map(|grid| grid.visible).collect();
                let effective: Vec<_> = grids.iter().enumerate().map(|(index, grid)| LayoutGrid {
                    visible: previous.get(index).copied().unwrap_or(grid.visible), ..*grid
                }).collect();
                node.set_layout_guides(&effective);
                crate::layout_guides::prune_layout_guide_bindings(node);
            }
        }
    }
}

fn normalized_grids(grids: impl Iterator<Item = LayoutGrid>) -> Vec<LayoutGrid> {
    grids.map(|grid| LayoutGrid { visible: true, ..grid }).collect()
}

fn grid_values(node: &Node) -> Vec<LayoutGrid> {
    normalized_grids(node.layout_guides().copied())
}

fn style_definition(
    id: StyleId,
    name: &str,
    description: &str,
    value: StyleValue,
) -> Result<Style, DocumentError> {
    if id == 0 {
        return Err(invalid("Style IDs must be nonzero."));
    }
    let name = name.trim();
    if name.is_empty() || name.len() > 1024 || description.len() > 65_536 {
        return Err(invalid(
            "A style needs a name of at most 1024 bytes and a description of at most 65536 bytes.",
        ));
    }
    value.validate()?;
    Ok(Style {
        id,
        name: name.into(),
        description: description.into(),
        value,
    })
}

fn checked_style_id(document: &Document) -> Result<StyleId, DocumentError> {
    let mut id = document.next_id.max(1);
    while document.nodes.contains_key(&id)
        || document.styles.contains_key(&id)
        || document.variables.contains_key(&id)
        || document.pages.iter().any(|page| page.id == id)
        || document.comments.iter().any(|comment| comment.id == id)
    {
        id = id
            .checked_add(1)
            .ok_or_else(|| invalid("ID space exhausted"))?;
    }
    id.checked_add(1)
        .ok_or_else(|| invalid("ID space exhausted"))?;
    Ok(id)
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LocalStyleUpdate {
    pub definition_changed: bool,
    pub updated_nodes: usize,
    pub detached_overrides: usize,
}

impl Document {
    /// Count users across every page, including locked and hidden layers.
    pub fn local_style_usage(&self, style: StyleId) -> usize {
        self.nodes
            .values()
            .filter(|node| {
                [node.fill_style, node.text_style, node.effect_style, node.grid_style].contains(&Some(style))
            })
            .count()
    }
}

/// Per-frame settings stay distinct from the editor's background snap grid.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LayoutGridEdit {
    Add,
    Size(f64),
    Color(crate::Color),
    Visible(bool),
    Remove,
}

impl Editor {
    /// Edit compatible unlocked frames atomically, retaining style ownership on
    /// visibility changes and detaching it only when effective size/color change.
    /// Missing targets and invalid values reject the whole batch before history.
    pub fn edit_layout_grids(
        &mut self,
        ids: &[NodeId],
        edit: LayoutGridEdit,
    ) -> Result<usize, DocumentError> {
        if let LayoutGridEdit::Size(size) = edit {
            LayoutGrid { size, ..LayoutGrid::default() }.validate()?;
        }
        let variable_field = match edit {
            LayoutGridEdit::Size(_) => Some(crate::LayoutGuideVariableField::Size),
            LayoutGridEdit::Color(_) => Some(crate::LayoutGuideVariableField::Color),
            LayoutGridEdit::Visible(_) => Some(crate::LayoutGuideVariableField::Visible),
            _ => None,
        };
        let mut changed = Vec::new();
        for id in ids.iter().copied().collect::<BTreeSet<_>>() {
            let node = self.document.nodes.get(&id).ok_or(DocumentError::MissingNode(id))?;
            if self.document.effective_locked(id) || !LayoutGrid::supports(node.kind) {
                continue;
            }
            let differs = match edit {
                LayoutGridEdit::Add => node.layout_grid.is_none(),
                LayoutGridEdit::Size(value) => node.layout_grid.is_some_and(|grid| grid.size != value),
                LayoutGridEdit::Color(value) => node.layout_grid.is_some_and(|grid| grid.color != value),
                LayoutGridEdit::Visible(value) => node.layout_grid.is_some_and(|grid| grid.visible != value),
                LayoutGridEdit::Remove => node.layout_grid.is_some() || !node.layout_grid_overlays.is_empty() || node.grid_style.is_some(),
            };
            if differs || (node.layout_grid.is_some() && variable_field.is_some_and(|field| node.variable_bindings.contains_key(&field.key(0)))) {
                changed.push(id);
            }
        }
        if changed.is_empty() { return Ok(0); }
        self.update_nodes(&changed, "Edit layout grid", |node| {
            match edit {
                LayoutGridEdit::Add => node.layout_grid = Some(LayoutGrid::default()),
                LayoutGridEdit::Size(value) => node.layout_grid.as_mut().unwrap().size = value,
                LayoutGridEdit::Color(value) => node.layout_grid.as_mut().unwrap().color = value,
                LayoutGridEdit::Visible(value) => node.layout_grid.as_mut().unwrap().visible = value,
                LayoutGridEdit::Remove => {
                    node.layout_grid = None; node.layout_grid_overlays.clear(); node.grid_style = None;
                    crate::remove_layout_guide_bindings(node);
                }
            }
            if let Some(field) = variable_field { node.variable_bindings.remove(&field.key(0)); }
            node.overridden_properties.insert(
                if matches!(edit, LayoutGridEdit::Visible(_)) { "layout_grid_visible" } else { "layout_grid" }.into()
            );
        });
        Ok(changed.len())
    }

    pub fn create_local_style(
        &mut self,
        name: &str,
        description: &str,
        value: StyleValue,
    ) -> Result<StyleId, DocumentError> {
        let id = checked_style_id(&self.document)?;
        let style = style_definition(id, name, description, value)?;
        self.transact("Create local style", |document| {
            document.next_id = id + 1;
            document.styles.insert(id, style);
        });
        Ok(id)
    }

    /// Capture the first compatible unlocked target and apply its definition to
    /// every compatible unlocked target in one undo entry. Missing IDs reject
    /// the entire operation before allocating a style or touching history.
    pub fn create_local_style_for_nodes(
        &mut self,
        ids: &[NodeId],
        kind: LocalStyleKind,
        name: &str,
        description: &str,
    ) -> Result<StyleId, DocumentError> {
        let unique: BTreeSet<_> = ids.iter().copied().collect();
        for id in &unique {
            if !self.document.nodes.contains_key(id) {
                return Err(DocumentError::MissingNode(*id));
            }
        }
        let source = ids
            .iter()
            .filter_map(|id| self.document.nodes.get(id))
            .find(|node| kind.accepts(node) && !self.document.effective_locked(node.id))
            .ok_or_else(|| invalid("Select an unlocked layer compatible with this style type."))?;
        let id = checked_style_id(&self.document)?;
        let style = style_definition(id, name, description, StyleValue::from_node(kind, source)?)?;
        let mut nodes = BTreeMap::new();
        for target in unique {
            let node = &self.document.nodes[&target];
            if self.document.effective_locked(target) || !kind.accepts(node) {
                continue;
            }
            let mut node = node.clone();
            apply_definition(&mut node, &style, true);
            nodes.insert(target, node);
        }
        self.transact("Create and apply local style", |document| {
            document.next_id = id + 1;
            document.styles.insert(id, style);
            document.nodes.extend(nodes);
            if kind == LocalStyleKind::Text {
                document.layout_all();
            }
        });
        Ok(id)
    }

    /// Applying a text style to a mixed selection affects only text layers.
    /// Grid styles affect only frames, components and instances; visibility stays local.
    pub fn apply_local_style(
        &mut self,
        ids: &[NodeId],
        style: StyleId,
    ) -> Result<usize, DocumentError> {
        let style_id = style;
        let style = self
            .document
            .styles
            .get(&style)
            .ok_or_else(|| invalid("The local style no longer exists."))?;
        if style_id == 0 || style.id != style_id {
            return Err(invalid("The local style ID does not match its definition."));
        }
        style.value.validate()?;
        let kind = style.value.kind();
        let mut nodes = BTreeMap::new();
        for id in ids.iter().copied().collect::<BTreeSet<_>>() {
            let node = self
                .document
                .nodes
                .get(&id)
                .ok_or(DocumentError::MissingNode(id))?;
            if self.document.effective_locked(id) || !kind.accepts(node) {
                continue;
            }
            let mut edited = node.clone();
            apply_definition(&mut edited, style, true);
            if edited != *node {
                nodes.insert(id, edited);
            }
        }
        let count = nodes.len();
        if count == 0 {
            return Ok(0);
        }
        let ids: Vec<_> = nodes.keys().copied().collect();
        if kind == LocalStyleKind::Text {
            self.transact("Apply local style", |document| {
                document.nodes.extend(nodes);
                document.layout_all();
            });
        } else {
            self.edit_nodes("Apply local style", &ids, |editor| {
                editor.document.nodes.extend(nodes)
            });
        }
        Ok(count)
    }

    pub fn detach_local_style(
        &mut self,
        ids: &[NodeId],
        kind: LocalStyleKind,
    ) -> Result<usize, DocumentError> {
        let mut changed = Vec::new();
        for id in ids.iter().copied().collect::<BTreeSet<_>>() {
            let node = self
                .document
                .nodes
                .get(&id)
                .ok_or(DocumentError::MissingNode(id))?;
            if !self.document.effective_locked(id) && kind.binding(node).is_some() {
                changed.push(id);
            }
        }
        if changed.is_empty() {
            return Ok(0);
        }
        self.edit_nodes("Detach local style", &changed, |editor| {
            for id in &changed {
                let node = editor
                    .document
                    .nodes
                    .get_mut(id)
                    .expect("validated style target");
                kind.bind(node, None);
                kind.mark_override(node);
            }
        });
        Ok(changed.len())
    }

    pub fn update_local_style(
        &mut self,
        id: StyleId,
        name: &str,
        description: &str,
        value: StyleValue,
    ) -> Result<LocalStyleUpdate, DocumentError> {
        let previous = self
            .document
            .styles
            .get(&id)
            .ok_or_else(|| invalid("The local style no longer exists."))?;
        let style = style_definition(id, name, description, value)?;
        if previous.value.kind() != style.value.kind() {
            return Err(invalid("A local style cannot change its type."));
        }
        if previous == &style {
            return Ok(LocalStyleUpdate::default());
        }
        let kind = style.value.kind();
        let value_changed = previous.value != style.value;
        let mut result = LocalStyleUpdate {
            definition_changed: true,
            ..LocalStyleUpdate::default()
        };
        let mut nodes = BTreeMap::new();
        if value_changed {
            for node in self
                .document
                .nodes
                .values()
                .filter(|node| kind.binding(node) == Some(id))
            {
                let mut edited = node.clone();
                if kind.accepts(node)
                    && (previous.value.matches_node(node) || style.value.matches_node(node))
                {
                    // Shared definitions update locked users too, just as an
                    // inherited component update does. This is not a direct edit.
                    style.value.write_node(&mut edited);
                    result.updated_nodes += 1;
                } else {
                    kind.bind(&mut edited, None);
                    kind.mark_override(&mut edited);
                    result.detached_overrides += 1;
                }
                if edited != *node {
                    nodes.insert(node.id, edited);
                }
            }
        }
        self.transact("Edit local style", |document| {
            document.styles.insert(id, style);
            document.nodes.extend(nodes);
            if value_changed && kind == LocalStyleKind::Text {
                document.layout_all();
            }
        });
        Ok(result)
    }

    /// Delete the definition while retaining every user's effective appearance.
    /// All binding slots are cleaned, including malformed legacy references.
    pub fn remove_local_style(&mut self, id: StyleId) -> Result<usize, DocumentError> {
        if !self.document.styles.contains_key(&id) {
            return Err(invalid("The local style no longer exists."));
        }
        let users: Vec<_> = self
            .document
            .nodes
            .values()
            .filter(|node| {
                [node.fill_style, node.text_style, node.effect_style, node.grid_style].contains(&Some(id))
            })
            .map(|node| node.id)
            .collect();
        self.transact("Delete local style", |document| {
            document.styles.remove(&id);
            for target in &users {
                let node = document
                    .nodes
                    .get_mut(target)
                    .expect("validated style user");
                for kind in [
                    LocalStyleKind::Fill,
                    LocalStyleKind::Text,
                    LocalStyleKind::Effect,
                    LocalStyleKind::Grid,
                ] {
                    if kind.binding(node) == Some(id) {
                        kind.bind(node, None);
                    }
                }
            }
        });
        Ok(users.len())
    }
}

fn apply_definition(node: &mut Node, style: &Style, explicit: bool) {
    let kind = style.value.kind();
    let variable = match kind {
        LocalStyleKind::Fill => Some("fill"),
        LocalStyleKind::Text => Some("font_size"),
        _ => None,
    };
    let changed = kind.binding(node) != Some(style.id)
        || !style.value.matches_node(node)
        || variable.is_some_and(|key| node.variable_bindings.contains_key(key))
        || (explicit && kind == LocalStyleKind::Grid && node.variable_bindings.keys().any(|key| crate::parse_layout_guide_binding(key).is_some()));
    if !changed {
        return;
    }
    style.value.write_node(node);
    kind.bind(node, Some(style.id));
    if explicit {
        if let Some(variable) = variable {
            node.variable_bindings.remove(variable);
        }
        if kind == LocalStyleKind::Grid { crate::remove_layout_guide_bindings(node); }
        kind.mark_override(node);
    }
}

/// Compare against an existing history/cache snapshot without duplicating large
/// paint payloads. A caller that directly changed the document without refreshing
/// it falls back to capturing only active bindings, keeping comparisons exact.
#[derive(Default)]
pub(crate) struct StyleBindingSnapshot<'a> {
    previous: Option<&'a Node>,
    values: Vec<(LocalStyleKind, StyleId, StyleValue)>,
    layout_bindings: crate::layout_variables::LayoutBindingSnapshot,
}
impl<'a> StyleBindingSnapshot<'a> {
    pub(crate) fn capture(node: &Node, previous: Option<&'a Node>) -> Self {
        if let Some(previous) = previous.filter(|previous| {
            [
                LocalStyleKind::Fill,
                LocalStyleKind::Text,
                LocalStyleKind::Effect,
                LocalStyleKind::Grid,
            ]
            .into_iter()
            .all(|kind| {
                kind.binding(previous) == kind.binding(node)
                    && (kind.binding(node).is_none()
                        || kind.accepts(previous) == kind.accepts(node)
                            && (!kind.accepts(node) || kind.same_node_values(previous, node)))
            })
        }) {
            return Self {
                previous: Some(previous),
                values: Vec::new(),
                layout_bindings: crate::layout_variables::LayoutBindingSnapshot::capture(node),
            };
        }
        let mut values = Vec::new();
        for kind in [
            LocalStyleKind::Fill,
            LocalStyleKind::Text,
            LocalStyleKind::Effect,
            LocalStyleKind::Grid,
        ] {
            if let Some(id) = kind.binding(node) {
                // A malformed legacy text binding can still be detached by an
                // explicit style operation. Generic unrelated edits preserve it.
                if let Ok(value) = StyleValue::from_node(kind, node) {
                    values.push((kind, id, value));
                }
            }
        }
        Self {
            previous: None,
            values,
            layout_bindings: crate::layout_variables::LayoutBindingSnapshot::capture(node),
        }
    }
    pub(crate) fn detach_changed(self, node: &mut Node) {
        self.layout_bindings.detach_changed(node);
        if let Some(previous) = self.previous {
            detach_changed_style_values(previous, node);
            return;
        }
        for (kind, id, value) in self.values {
            if kind.binding(node) == Some(id) && !value.matches_node(node) {
                kind.bind(node, None);
                kind.mark_override(node);
            }
        }
    }
}

/// `before` may be the editor's render snapshot or the source of a candidate
/// edit. Only changes to a bound style's effective fields detach that binding.
pub(crate) fn detach_changed_style_bindings(before: &Node, node: &mut Node) {
    crate::layout_variables::LayoutBindingSnapshot::capture(before).detach_changed(node);
    detach_changed_style_values(before, node);
}

fn detach_changed_style_values(before: &Node, node: &mut Node) {
    for kind in [
        LocalStyleKind::Fill,
        LocalStyleKind::Text,
        LocalStyleKind::Effect,
        LocalStyleKind::Grid,
    ] {
        if let Some(id) = kind.binding(before) {
            if kind.accepts(before)
                && kind.binding(node) == Some(id)
                && !kind.same_node_values(before, node)
            {
                kind.bind(node, None);
                kind.mark_override(node);
            }
        }
    }
}

/// Per-layer variables and local styles cannot own the same value in the
/// current document model. Explicit variable application takes ownership even
/// when its resolved value happens to equal the old style's value.
pub(crate) fn detach_style_for_variable(node: &mut Node, property: &str) {
    if let Some((_, field)) = crate::parse_layout_guide_binding(property) {
        if field == crate::LayoutGuideVariableField::Visible {
            node.overridden_properties.insert("layout_grid_visible".into());
        } else {
            node.grid_style = None;
            LocalStyleKind::Grid.mark_override(node);
        }
        return;
    }
    match property {
        "fill" => {
            node.fill_style = None;
            LocalStyleKind::Fill.mark_override(node);
        }
        "font_size" => {
            if node.text_style.take().is_some() {
                LocalStyleKind::Text.mark_override(node);
            } else {
                node.overridden_properties.insert("font_size".into());
            }
        }
        _ => {}
    }
}

#[cfg(test)]
#[path = "style_edit_tests.rs"]
mod tests;

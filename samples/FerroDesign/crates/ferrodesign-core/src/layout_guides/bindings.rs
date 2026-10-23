//! Guide bindings use the existing typed node-variable map, so old square-grid
//! documents remain readable without a migration or a second ownership system.
use crate::{LayoutGrid, LayoutGridKind, LayoutGuideEdit, LayoutGuideProperty, Node,
    VariableId, VariableKind, VariableValue};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayoutGuideVariableField { Size, Count, Gutter, Offset, Color, Visible }

impl LayoutGuideVariableField {
    pub const ALL: [Self; 6] = [Self::Size, Self::Count, Self::Gutter, Self::Offset, Self::Color, Self::Visible];
    pub fn name(self) -> &'static str {
        match self { Self::Size => "size", Self::Count => "count", Self::Gutter => "gutter", Self::Offset => "offset", Self::Color => "color", Self::Visible => "visible" }
    }
    pub fn label(self) -> &'static str {
        match self { Self::Size => "Size / spacing", Self::Count => "Count", Self::Gutter => "Gutter", Self::Offset => "Margin / offset", Self::Color => "Color", Self::Visible => "Visibility" }
    }
    pub fn kind(self) -> VariableKind {
        match self { Self::Color => VariableKind::Color, Self::Visible => VariableKind::Boolean, _ => VariableKind::Number }
    }
    pub fn supports(self, grid: &LayoutGrid) -> bool {
        !matches!(self, Self::Count | Self::Gutter | Self::Offset) || grid.pattern.axis().is_some()
    }
    pub fn key(self, index: usize) -> String { format!("layout_grid.{index}.{}", self.name()) }

    fn property(self, value: &VariableValue) -> Result<LayoutGuideProperty, String> {
        let invalid = || format!("Guide {} requires a valid {} variable in every mode.", self.label().to_lowercase(), self.kind().label());
        let property = match (self, value) {
            (Self::Color, VariableValue::Color(value)) => LayoutGuideProperty::Color(*value),
            (Self::Visible, VariableValue::Boolean(value)) => LayoutGuideProperty::Visible(*value),
            (Self::Size, VariableValue::Number(value)) if value.is_finite() && *value > 0.0 => LayoutGuideProperty::Size(*value),
            (Self::Gutter, VariableValue::Number(value)) if value.is_finite() && *value >= 0.0 => LayoutGuideProperty::Gutter(*value),
            (Self::Offset, VariableValue::Number(value)) if value.is_finite() && *value >= 0.0 => LayoutGuideProperty::Offset(*value),
            (Self::Count, VariableValue::Number(value)) if value.is_finite() && value.fract() == 0.0
                && (1.0..=super::MAX_LAYOUT_GUIDE_SECTIONS as f64).contains(value) => LayoutGuideProperty::Count(Some(*value as u32)),
            _ => return Err(invalid()),
        };
        Ok(property)
    }
}

/// Parse the canonical, index-addressed key. An alternate spelling such as
/// `layout_grid.00.size` is rejected rather than owning the same value twice.
pub fn parse_layout_guide_binding(property: &str) -> Option<(usize, LayoutGuideVariableField)> {
    let (index, field) = property.strip_prefix("layout_grid.")?.split_once('.')?;
    let parsed = index.parse::<usize>().ok()?;
    if parsed >= super::MAX_LAYOUT_GUIDES || index != parsed.to_string() { return None; }
    let field = LayoutGuideVariableField::ALL.into_iter().find(|candidate| candidate.name() == field)?;
    Some((parsed, field))
}

pub fn validate_layout_guide_binding_target(node: &Node, property: &str) -> Result<(), String> {
    let (index, field) = parse_layout_guide_binding(property).ok_or("This layout-guide variable property is unsupported.")?;
    if !LayoutGrid::supports(node.kind) { return Err("Only frames, components and instances can bind layout-guide properties.".into()); }
    let grid = node.layout_guide(index).ok_or_else(|| format!("{} does not contain guide {}.", node.name, index + 1))?;
    if !field.supports(grid) { return Err("Count, gutter and margin/offset bindings require a row or column guide.".into()); }
    Ok(())
}

pub fn validate_layout_guide_variable(node: &Node, property: &str, value: &VariableValue) -> Result<(), String> {
    validate_layout_guide_binding_target(node, property)?;
    let (index, field) = parse_layout_guide_binding(property).unwrap();
    let mut probe = *node.layout_guide(index).unwrap();
    field.property(value)?.apply(&mut probe);
    probe.validate().map_err(|error| error.to_string())
}

/// Apply an already validated mode value. `true` identifies a guide key even if
/// a caller skipped validation; invalid data never becomes invalid geometry.
pub fn apply_layout_guide_variable(node: &mut Node, property: &str, value: &VariableValue) -> bool {
    let Some((index, field)) = parse_layout_guide_binding(property) else { return false; };
    if validate_layout_guide_variable(node, property, value).is_ok() {
        if let Some(grid) = node.layout_guide_mut(index) { field.property(value).unwrap().apply(grid); }
    }
    true
}

/// Explicit style assignment and flattening must relinquish guide ownership.
pub fn remove_layout_guide_bindings(node: &mut Node) {
    node.variable_bindings.retain(|property, _| parse_layout_guide_binding(property).is_none());
}

pub(crate) fn prune_layout_guide_bindings(node: &mut Node) {
    let obsolete: Vec<_> = node.variable_bindings.keys().filter(|property|
        parse_layout_guide_binding(property).is_some() && validate_layout_guide_binding_target(node, property).is_err()).cloned().collect();
    for property in obsolete { node.variable_bindings.remove(&property); }
}

/// Component overrides copy only guide-owned keys; unrelated property bindings
/// keep their existing component inheritance rules.
pub(crate) fn copy_layout_guide_bindings(old: &Node, node: &mut Node, visibility_only: bool) {
    node.variable_bindings.retain(|property, _| !parse_layout_guide_binding(property)
        .is_some_and(|(_, field)| !visibility_only || field == LayoutGuideVariableField::Visible));
    for (property, variable) in &old.variable_bindings {
        if let Some((index, field)) = parse_layout_guide_binding(property) {
            if (!visibility_only || field == LayoutGuideVariableField::Visible)
                && node.layout_guide(index).is_some_and(|grid| field.supports(grid)) {
                node.variable_bindings.insert(property.clone(), *variable);
            }
        }
    }
}

pub(super) fn edited_bindings(node: &Node, edit: LayoutGuideEdit) -> BTreeMap<String, VariableId> {
    let count = node.layout_guide_count();
    let mut result = BTreeMap::new();
    for (property, &variable) in &node.variable_bindings {
        let Some((index, field)) = parse_layout_guide_binding(property) else { result.insert(property.clone(), variable); continue; };
        let mapped = match edit {
            LayoutGuideEdit::Clear => None,
            LayoutGuideEdit::Remove(removed) if removed < count => {
                if index == removed { None } else { Some(if index > removed { index - 1 } else { index }) }
            }
            LayoutGuideEdit::Duplicate(duplicate) if duplicate < count => {
                if index == duplicate { result.insert(field.key(index), variable); }
                Some(if index >= duplicate { index + 1 } else { index })
            }
            LayoutGuideEdit::Move { index: from, to } if from < count && to < count => Some(
                if index == from { to }
                else if from < to && index > from && index <= to { index - 1 }
                else if from > to && index >= to && index < from { index + 1 }
                else { index }),
            LayoutGuideEdit::Set { index: changed, property } if index == changed && index < count => {
                let detach = match property {
                    LayoutGuideProperty::Size(_) => field == LayoutGuideVariableField::Size,
                    LayoutGuideProperty::Color(_) => field == LayoutGuideVariableField::Color,
                    LayoutGuideProperty::Visible(_) => field == LayoutGuideVariableField::Visible,
                    LayoutGuideProperty::Count(_) => field == LayoutGuideVariableField::Count,
                    LayoutGuideProperty::Gutter(_) => field == LayoutGuideVariableField::Gutter,
                    LayoutGuideProperty::Offset(_) => field == LayoutGuideVariableField::Offset,
                    LayoutGuideProperty::Kind(LayoutGridKind::Square) | LayoutGuideProperty::Pattern(_) =>
                        matches!(field, LayoutGuideVariableField::Count | LayoutGuideVariableField::Gutter | LayoutGuideVariableField::Offset),
                    _ => false,
                };
                (!detach).then_some(index)
            }
            _ => Some(index),
        };
        if let Some(index) = mapped { result.insert(field.key(index), variable); }
    }
    result
}

#[cfg(test)]
mod tests;

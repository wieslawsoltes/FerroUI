//! Auto-layout spacing variables share the document's typed binding map.
//! Each padding side owns a separate key; there is no order-dependent uniform
//! alias competing with the side keys. Numeric edits relinquish only the sides
//! whose effective values changed, while explicit Unbind freezes the value.
use crate::{AutoLayout, Node, VariableId, VariableValue};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayoutVariableField {
    PaddingTop,
    PaddingRight,
    PaddingBottom,
    PaddingLeft,
    Gap,
    CrossGap,
}

impl LayoutVariableField {
    pub const ALL: [Self; 6] = [
        Self::PaddingTop,
        Self::PaddingRight,
        Self::PaddingBottom,
        Self::PaddingLeft,
        Self::Gap,
        Self::CrossGap,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Self::PaddingTop => "layout.padding.top",
            Self::PaddingRight => "layout.padding.right",
            Self::PaddingBottom => "layout.padding.bottom",
            Self::PaddingLeft => "layout.padding.left",
            Self::Gap => "layout.gap",
            Self::CrossGap => "layout.cross_gap",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::PaddingTop => "Top padding",
            Self::PaddingRight => "Right padding",
            Self::PaddingBottom => "Bottom padding",
            Self::PaddingLeft => "Left padding",
            Self::Gap => "Layout gap",
            Self::CrossGap => "Layout cross gap",
        }
    }

    pub fn value(self, layout: &AutoLayout) -> f64 {
        match self {
            Self::PaddingTop => layout.padding.top,
            Self::PaddingRight => layout.padding.right,
            Self::PaddingBottom => layout.padding.bottom,
            Self::PaddingLeft => layout.padding.left,
            Self::Gap => layout.gap,
            Self::CrossGap => layout.cross_gap,
        }
    }

    fn set(self, layout: &mut AutoLayout, value: f64) {
        *match self {
            Self::PaddingTop => &mut layout.padding.top,
            Self::PaddingRight => &mut layout.padding.right,
            Self::PaddingBottom => &mut layout.padding.bottom,
            Self::PaddingLeft => &mut layout.padding.left,
            Self::Gap => &mut layout.gap,
            Self::CrossGap => &mut layout.cross_gap,
        } = value;
    }
}

pub fn parse_layout_variable_binding(property: &str) -> Option<LayoutVariableField> {
    LayoutVariableField::ALL
        .into_iter()
        .find(|field| field.key() == property)
}

pub fn validate_layout_variable_target(node: &Node, property: &str) -> Result<(), String> {
    parse_layout_variable_binding(property).ok_or("Unsupported auto-layout variable property.")?;
    if !node.kind.is_container() || node.layout.is_none() {
        return Err("Padding and gap bindings require a layer with auto layout.".into());
    }
    Ok(())
}

pub fn validate_layout_variable(
    node: &Node,
    property: &str,
    value: &VariableValue,
) -> Result<(), String> {
    validate_layout_variable_target(node, property)?;
    if !matches!(value, VariableValue::Number(value) if value.is_finite() && *value >= 0.0) {
        return Err(format!(
            "{} requires a finite, nonnegative Number variable in every mode.",
            parse_layout_variable_binding(property).unwrap().label()
        ));
    }
    Ok(())
}

/// Legacy documents may contain invalid bindings. A recognized key is consumed
/// without applying invalid geometry; authoring rejects it before this path.
pub(crate) fn apply_layout_variable(
    node: &mut Node,
    property: &str,
    value: &VariableValue,
) -> bool {
    let Some(field) = parse_layout_variable_binding(property) else {
        return false;
    };
    if validate_layout_variable(node, property, value).is_ok() {
        let VariableValue::Number(value) = value else {
            unreachable!()
        };
        field.set(node.layout.as_mut().unwrap(), *value);
    }
    true
}

/// Fixed-size state keeps generic scoped edits independent of document size and
/// avoids cloning layout tracks, text, image payloads, or the binding map.
#[derive(Default)]
pub(crate) struct LayoutBindingSnapshot([Option<(VariableId, f64)>; 6]);
impl LayoutBindingSnapshot {
    pub(crate) fn capture(node: &Node) -> Self {
        if node.variable_bindings.is_empty() {
            return Self::default();
        }
        let Some(layout) = &node.layout else {
            return Self::default();
        };
        Self(std::array::from_fn(|index| {
            let field = LayoutVariableField::ALL[index];
            node.variable_bindings
                .get(field.key())
                .map(|id| (*id, field.value(layout)))
        }))
    }

    pub(crate) fn detach_changed(self, node: &mut Node) {
        for (field, previous) in LayoutVariableField::ALL.into_iter().zip(self.0) {
            if let Some((id, value)) = previous {
                if node.variable_bindings.get(field.key()) == Some(&id)
                    && node.layout.as_ref().map(|layout| field.value(layout)) != Some(value)
                {
                    node.variable_bindings.remove(field.key());
                    if node.component_source.is_some() {
                        node.overridden_properties.insert("layout".into());
                    }
                }
            }
        }
    }
}

pub(crate) fn copy_layout_variable_bindings(old: &Node, node: &mut Node) {
    remove_layout_variable_bindings(node);
    if node.layout.is_some() {
        for field in LayoutVariableField::ALL {
            if let Some(&id) = old.variable_bindings.get(field.key()) {
                node.variable_bindings.insert(field.key().into(), id);
            }
        }
    }
}

pub(crate) fn remove_layout_variable_bindings(node: &mut Node) {
    node.variable_bindings
        .retain(|property, _| parse_layout_variable_binding(property).is_none());
}

#[cfg(test)]
#[path = "layout_variables_tests.rs"]
mod tests;

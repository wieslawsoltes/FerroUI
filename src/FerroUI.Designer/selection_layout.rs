//! Selection-wide layout commands retain native property identity and panel semantics.
//! Reset removes local values; matching measures logical layout before render transforms.
use crate::{DesignerSession, Document, Edit, NodeId};
use ferroui_base::data::BindingOperations;
use ferroui_base::layout::Layoutable;
use ferroui_base::{FerroProperty, Ref, TypeInfo};
use ferroui_controls::Canvas;
use ferroui_designer_core::{Attribute, Node, NodeKind};
use ferroui_designer_tools::{LayoutAction, Rect};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Remove authored values so the native style, inheritance and default layers take effect.
/// `All` includes margin, alignment, explicit dimensions and min/max constraints. It does
/// not change padding, content alignment, transforms, panel placement or child definitions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LayoutReset {
    Margin,
    Alignment,
    Width,
    Height,
    Size,
    All,
}

impl LayoutReset {
    pub fn properties(self) -> &'static [&'static str] {
        match self {
            Self::Margin => &["Margin"],
            Self::Alignment => &["HorizontalAlignment", "VerticalAlignment"],
            Self::Width => &["Width"],
            Self::Height => &["Height"],
            Self::Size => &["Width", "Height"],
            Self::All => &[
                "Margin",
                "HorizontalAlignment",
                "VerticalAlignment",
                "Width",
                "Height",
                "MinWidth",
                "MinHeight",
                "MaxWidth",
                "MaxHeight",
            ],
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Margin => "Reset margins",
            Self::Alignment => "Reset alignment",
            Self::Width => "Reset width",
            Self::Height => "Reset height",
            Self::Size => "Reset size",
            Self::All => "Reset size and alignment",
        }
    }
}

/// Match logical layout dimensions to the first selected control. Render transforms and
/// ancestor scaling remain intact, so matching dimensions need not match screen-space size.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LayoutMatch {
    Width,
    Height,
    Size,
}

impl LayoutMatch {
    fn width(self) -> bool {
        self != Self::Height
    }

    fn height(self) -> bool {
        self != Self::Width
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Width => "Match width to first selection",
            Self::Height => "Match height to first selection",
            Self::Size => "Match size to first selection",
        }
    }

    fn retains(self, name: &str) -> bool {
        match name {
            "Width" | "Canvas.Left" | "Canvas.Right" => self.width(),
            "Height" | "Canvas.Top" | "Canvas.Bottom" => self.height(),
            _ => true,
        }
    }
}

impl DesignerSession {
    /// Plan a selective reset against current authored control layers. Metadata resolves
    /// owner-qualified aliases to actual native property IDs; foreign homonyms are retained.
    /// Planning is observational and remains available to collaboration viewers.
    pub fn plan_layout_reset(
        &self,
        nodes: &[NodeId],
        reset: LayoutReset,
    ) -> Result<Vec<Edit>, String> {
        self.ensure_selection_layout_ready()?;
        if nodes.is_empty() {
            return Err("Select at least one authored control to reset its layout".into());
        }
        let mut edits = Vec::new();
        let mut seen = BTreeSet::new();
        for node in nodes.iter().copied().filter(|node| seen.insert(*node)) {
            let layout = self.selection_layout_target(node)?;
            let element = self
                .document()
                .ast()
                .node(node)
                .ok_or("Unknown layout target")?;
            let expected = reset
                .properties()
                .iter()
                .map(|name| native_property(name).expect("known layout property"))
                .collect::<Vec<_>>();
            edits.extend(reset_element(
                self.document(),
                element,
                layout.get_type(),
                &expected,
            ));
        }
        validate_edits(self.document(), edits)
    }

    /// Reset the current selection as one ordinary source transaction, including while
    /// animation recording is enabled. A no-op does not add an undo entry or reload preview.
    pub fn reset_selection_layout(&mut self, reset: LayoutReset) -> Result<(), String> {
        self.ensure_writable().map_err(|error| error.to_string())?;
        let edits = self.plan_layout_reset(self.selection(), reset)?;
        if !edits.is_empty() {
            self.apply(reset.label(), edits)
                .map_err(|error| error.to_string())?;
        }
        Ok(())
    }

    /// Plan a measured resize for every selected control after the first. Each destination's
    /// registered layout adapter owns its positioning rules. Native min/max constraints,
    /// bindings and object-valued layout properties must permit the complete operation.
    /// The plan retains the untouched dimension and Canvas axis for Width/Height commands.
    /// Planning is observational and remains available to collaboration viewers.
    pub fn plan_match_selection_size(&self, mode: LayoutMatch) -> Result<Vec<Edit>, String> {
        self.ensure_selection_layout_ready()?;
        let [reference, targets @ ..] = self.selection() else {
            return Err("Select a reference control followed by the controls to resize".into());
        };
        if targets.is_empty() {
            return Err("Select at least two authored controls to match their size".into());
        }
        // Resizing an ancestor invalidates the descendant's accepted allocation. Such a
        // selection has no simultaneous immutable layout plan.
        let selected = self.selection().iter().copied().collect::<BTreeSet<_>>();
        for node in self.selection() {
            let mut parent = self
                .document()
                .ast()
                .node(*node)
                .and_then(|node| node.parent);
            while let Some(id) = parent {
                if selected.contains(&id) {
                    return Err("Match size between independent controls, without selecting their ancestors".into());
                }
                parent = self.document().ast().node(id).and_then(|node| node.parent);
            }
        }
        let reference = self.measured_selection_layout_target(*reference)?;
        let dimensions = reference.bounds();
        let mut edits = Vec::new();
        for node in targets {
            let layout = self.measured_selection_layout_target(*node)?;
            let before = layout.bounds();
            let width = if mode.width() {
                dimensions.width
            } else {
                before.width
            };
            let height = if mode.height() {
                dimensions.height
            } else {
                before.height
            };
            if width == before.width && height == before.height {
                continue;
            }
            for (name, value, minimum, maximum, enabled) in [
                (
                    "Width",
                    width,
                    layout.min_width(),
                    layout.max_width(),
                    mode.width(),
                ),
                (
                    "Height",
                    height,
                    layout.min_height(),
                    layout.max_height(),
                    mode.height(),
                ),
            ] {
                if enabled && (value < minimum || value > maximum) {
                    return Err(format!("{name} cannot match the reference within this control's minimum and maximum constraints"));
                }
            }
            let owner = crate::workspace_tools::parent(self, *node)?;
            if self
                .preview
                .visual(owner)
                .and_then(|visual| visual.cast::<Canvas>())
                .is_some()
            {
                for (name, active) in [
                    ("Canvas.Left", width != before.width),
                    ("Canvas.Right", width != before.width),
                    ("Canvas.Top", height != before.height),
                    ("Canvas.Bottom", height != before.height),
                ] {
                    if active
                        && BindingOperations::get_binding_expression_base(
                            &layout,
                            native_property(name).expect("native Canvas anchor"),
                        )
                        .is_some()
                    {
                        return Err(format!(
                            "{name} is driven by a native binding; edit its source explicitly"
                        ));
                    }
                }
            }
            if mode != LayoutMatch::Size
                && !self
                    .layout_tools()
                    .is_builtin_adapter_for(self.document(), owner)
                && !crate::definition_manipulation::is_template_owner(self.document(), owner)
            {
                return Err("This custom layout adapter supports complete size matching; use Match size or its property editor for one dimension".into());
            }
            // The existing resize planner owns margin/alignment slack and opposite Canvas
            // anchors. Its input is layout space, before the control's RenderTransform.
            let measured = crate::workspace_tools::measure(self, owner, Some(*node))?;
            let planned = self
                .layout_tools()
                .plan(
                    &measured.context(self.document()),
                    LayoutAction::Resize {
                        node: *node,
                        before: Rect::new(before.x, before.y, before.width, before.height),
                        after: Rect::new(before.x, before.y, width, height),
                    },
                )
                .map_err(|error| error.to_string())?;
            edits.extend(normalize_resize(self, *node, &layout, mode, planned)?);
        }
        validate_edits(self.document(), edits)
    }

    /// Apply size matching in one undo step without recording animation keys.
    pub fn match_selection_size(&mut self, mode: LayoutMatch) -> Result<(), String> {
        self.ensure_writable().map_err(|error| error.to_string())?;
        let edits = self.plan_match_selection_size(mode)?;
        if !edits.is_empty() {
            self.apply(mode.label(), edits)
                .map_err(|error| error.to_string())?;
        }
        Ok(())
    }

    fn ensure_selection_layout_ready(&self) -> Result<(), String> {
        if !self.document().is_valid()
            || !matches!(self.preview_status(), crate::PreviewStatus::Ready { revision }
                if *revision == self.document().revision())
        {
            return Err("Layout commands require a successful preview of the current XAML".into());
        }
        Ok(())
    }

    fn selection_layout_target(&self, node: NodeId) -> Result<Ref<Layoutable>, String> {
        self.ensure_layer_manipulable(node)?;
        self.preview
            .visual(node)
            .and_then(|visual| visual.cast::<Layoutable>())
            .ok_or_else(|| {
                "Select an authored control with native layout in the current preview".into()
            })
    }

    fn measured_selection_layout_target(&self, node: NodeId) -> Result<Ref<Layoutable>, String> {
        let layout = self.selection_layout_target(node)?;
        let bounds = layout.bounds();
        if !layout.is_measure_valid()
            || !layout.is_arrange_valid()
            || !bounds.width.is_finite()
            || !bounds.height.is_finite()
            || bounds.width <= 0.0
            || bounds.height <= 0.0
        {
            return Err("Wait for the selected controls to finish a nonzero native layout before matching size".into());
        }
        Ok(layout)
    }
}

fn native_property(name: &str) -> Option<&'static FerroProperty> {
    Some(match name {
        "Margin" => Layoutable::margin_property().as_property(),
        "HorizontalAlignment" => Layoutable::horizontal_alignment_property().as_property(),
        "VerticalAlignment" => Layoutable::vertical_alignment_property().as_property(),
        "Width" => Layoutable::width_property().as_property(),
        "Height" => Layoutable::height_property().as_property(),
        "MinWidth" => Layoutable::min_width_property().as_property(),
        "MinHeight" => Layoutable::min_height_property().as_property(),
        "MaxWidth" => Layoutable::max_width_property().as_property(),
        "MaxHeight" => Layoutable::max_height_property().as_property(),
        "Canvas.Left" => Canvas::left_property().as_property(),
        "Canvas.Top" => Canvas::top_property().as_property(),
        "Canvas.Right" => Canvas::right_property().as_property(),
        "Canvas.Bottom" => Canvas::bottom_property().as_property(),
        _ => return None,
    })
}

/// Resolve the source token in its own lexical scope before comparing property IDs. In
/// particular an application's `Layoutable.Width` never means the native Width property.
fn source_property(
    document: &Document,
    context: NodeId,
    target: &'static TypeInfo,
    name: &str,
) -> Option<&'static FerroProperty> {
    if let Some((owner, name)) = name.rsplit_once('.') {
        let (prefix, owner) = owner.split_once(':').unwrap_or(("", owner));
        let namespaces = document.ast().namespaces(context);
        let namespace = match namespaces.get(prefix) {
            Some(namespace) => namespace.as_str(),
            None if prefix.is_empty() => "https://github.com/ferroui",
            None => return None,
        };
        let owner = crate::native_types::registered_xml_type(namespace, owner)?;
        let property = owner.find_property(name)?;
        (property.is_attached() || owner.is_assignable_from(target)).then_some(property)
    } else if !name.contains(':') {
        target.find_property(name)
    } else {
        None
    }
}

fn matches_property(
    document: &Document,
    context: NodeId,
    target: &'static TypeInfo,
    name: &str,
    property: &FerroProperty,
) -> bool {
    source_property(document, context, target, name)
        .is_some_and(|candidate| candidate.id() == property.id())
}

fn reset_element(
    document: &Document,
    element: &Node,
    type_: &'static TypeInfo,
    properties: &[&FerroProperty],
) -> Vec<Edit> {
    let mut edits = Vec::new();
    for attribute in &element.attributes {
        if !attribute.is_namespace()
            && properties.iter().any(|property| {
                matches_property(document, element.id, type_, &attribute.name, property)
            })
        {
            edits.push(Edit::RemoveAttribute {
                node: element.id,
                name: attribute.name.clone(),
            });
        }
    }
    for child in element
        .children
        .iter()
        .filter_map(|id| document.ast().node(*id))
    {
        if child.kind == NodeKind::PropertyElement
            && properties
                .iter()
                .any(|property| matches_property(document, child.id, type_, &child.name, property))
        {
            edits.push(Edit::RemoveNode { node: child.id });
        }
    }
    edits
}

fn authored_attribute<'a>(
    document: &Document,
    element: &'a Node,
    type_: &'static TypeInfo,
    property: &FerroProperty,
) -> Result<Option<&'a Attribute>, String> {
    let mut attributes = element.attributes.iter().filter(|attribute| {
        !attribute.is_namespace()
            && matches_property(document, element.id, type_, &attribute.name, property)
    });
    let result = attributes.next();
    if attributes.next().is_some() {
        return Err(format!(
            "{} has more than one authored declaration",
            property.name()
        ));
    }
    if result.is_some_and(|attribute| {
        let value = attribute.value.trim_start();
        value.starts_with('{') && !value.starts_with("{}")
    }) {
        return Err(format!(
            "{} is an expression; edit its source explicitly",
            property.name()
        ));
    }
    if element
        .children
        .iter()
        .filter_map(|id| document.ast().node(*id))
        .any(|child| {
            child.kind == NodeKind::PropertyElement
                && matches_property(document, child.id, type_, &child.name, property)
        })
    {
        return Err(format!(
            "{} is authored as a property element; edit its source explicitly",
            property.name()
        ));
    }
    Ok(result)
}

fn normalize_resize(
    session: &DesignerSession,
    target: NodeId,
    layout: &Layoutable,
    mode: LayoutMatch,
    planned: Vec<Edit>,
) -> Result<Vec<Edit>, String> {
    let document = session.document();
    let element = document.ast().node(target).ok_or("Unknown resize target")?;
    let mut result = Vec::new();
    for edit in planned {
        let (node, name, value) = match &edit {
            Edit::SetAttribute { node, name, value } => (*node, name.as_str(), Some(value)),
            Edit::RemoveAttribute { node, name } => (*node, name.as_str(), None),
            _ => {
                result.push(edit);
                continue;
            }
        };
        if node != target {
            result.push(edit);
            continue;
        }
        if !mode.retains(name) {
            continue;
        }
        let property = native_property(name).or_else(|| {
            source_property(document, target, layout.get_type(), name).filter(|property| {
                ["Canvas.Left", "Canvas.Top", "Canvas.Right", "Canvas.Bottom"]
                    .iter()
                    .any(|name| {
                        native_property(name).is_some_and(|native| native.id() == property.id())
                    })
            })
        });
        let Some(property) = property else {
            // An explicitly registered custom layout adapter owns its own source commands.
            result.push(edit);
            continue;
        };
        let attribute = authored_attribute(document, element, layout.get_type(), property)?;
        if BindingOperations::get_binding_expression_base(layout, property).is_some() {
            return Err(format!(
                "{} is driven by a native binding; edit its source explicitly",
                property.name()
            ));
        }
        match value {
            Some(value) => {
                if attribute.is_some_and(|attribute| attribute.value == *value) {
                    continue;
                }
                if let Some(attribute) = attribute {
                    result.push(Edit::SetAttribute {
                        node,
                        name: attribute.name.clone(),
                        value: value.clone(),
                    });
                } else {
                    // This also qualifies a new native Canvas owner if the destination has
                    // a foreign default namespace. A foreign homonym is never overwritten.
                    result.extend(
                        ferroui_designer_tools::plan_layout_attributes(
                            document,
                            node,
                            &[(name.into(), value.clone())],
                        )
                        .map_err(|error| error.to_string())?,
                    );
                }
            }
            None => {
                if let Some(attribute) = attribute {
                    result.push(Edit::RemoveAttribute {
                        node,
                        name: attribute.name.clone(),
                    });
                }
            }
        }
    }
    Ok(result)
}

fn validate_edits(document: &Document, edits: Vec<Edit>) -> Result<Vec<Edit>, String> {
    if !edits.is_empty() {
        let mut candidate = document.clone();
        candidate
            .apply("Validate selection layout", edits.clone())
            .map_err(|error| error.to_string())?;
    }
    Ok(edits)
}

#[cfg(test)]
#[path = "selection_layout_tests.rs"]
mod tests;

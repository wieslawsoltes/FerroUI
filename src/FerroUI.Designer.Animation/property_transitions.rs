//! Source locations for FerroUI's transition collections on controls, styles, and resources.
use crate::programs::NATIVE_XMLNS;
use crate::{AnimationError, Result};
use ferroui_designer_core::{Document, Node, NodeId, NodeKind, SourceSpan};
use serde::Serialize;

mod timing;
pub use timing::PropertyTransitionTiming;

#[cfg(test)]
mod tests;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PropertyTransitionList {
    pub owner: NodeId,
    pub collection: Option<NodeId>,
    pub source: Option<SourceSpan>,
    pub entries: Vec<AuthoredPropertyTransition>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthoredPropertyTransition {
    pub id: NodeId,
    pub type_name: String,
    pub source: SourceSpan,
    pub property: Option<String>,
    pub duration: Option<String>,
    pub delay: Option<String>,
    pub easing: Option<String>,
    pub duration_seconds: Option<f64>,
    pub delay_seconds: Option<f64>,
    /// Timing fields that can be changed without replacing an authored expression or object.
    pub editable_timing: Vec<&'static str>,
}

impl PropertyTransitionList {
    pub fn read(document: &Document, owner: NodeId) -> Result<Self> {
        let owner_node = document
            .ast()
            .node(owner)
            .filter(|node| node.kind == NodeKind::Element)
            .ok_or_else(|| {
                AnimationError(
                    "Choose an authored control, style, or Transitions collection".into(),
                )
            })?;
        let mut result = Self {
            owner,
            collection: None,
            source: None,
            entries: Vec::new(),
        };
        if is_collection(owner_node) {
            result.load(document, owner_node);
            return Ok(result);
        }
        if let Some(attribute) = owner_node.attribute_info("Transitions") {
            result.source = Some(attribute.value_span);
            return Ok(result);
        }
        let containers = if is_style(owner_node) {
            let setters = children(document, owner_node)
                .filter(|property| {
                    is_styling_element(property, "Style.Setters")
                        || is_styling_element(property, "ControlTheme.Setters")
                })
                .flat_map(|property| children(document, property));
            children(document, owner_node)
                .chain(setters)
                .filter(|child| {
                    is_styling_element(child, "Setter")
                        && child.attribute("Property") == Some("Transitions")
                })
                .collect::<Vec<_>>()
        } else {
            children(document, owner_node)
                .filter(|child| {
                    child.kind == NodeKind::PropertyElement
                        && (child.local_name().split_once('.')
                            == Some((owner_node.local_name(), "Transitions"))
                            && child.namespace_uri == owner_node.namespace_uri
                            || native_animation_namespace(child)
                                && child.local_name() == "Animatable.Transitions")
                })
                .collect()
        };
        if let [container] = containers.as_slice() {
            result.source = Some(container.span);
            if let Some(attribute) = container.attribute_info("Value") {
                result.source = Some(attribute.value_span);
                return Ok(result);
            }
            let values = children(document, container).collect::<Vec<_>>();
            match values.as_slice() {
                [collection] if is_collection(collection) => result.load(document, collection),
                [value]
                    if is_styling_element(value, "Setter.Value")
                        && is_styling_element(container, "Setter") =>
                {
                    let values = children(document, value).collect::<Vec<_>>();
                    if let [collection] = values.as_slice() {
                        if is_collection(collection) {
                            result.load(document, collection);
                        }
                    }
                }
                _ => {}
            }
        } else if let Some(first) = containers.first() {
            // An ambiguous authored collection can still be revealed for repair.
            result.source = Some(first.span);
        }
        Ok(result)
    }

    fn load(&mut self, document: &Document, collection: &Node) {
        self.collection = Some(collection.id);
        self.source = Some(collection.span);
        self.entries = children(document, collection)
            .map(|node| AuthoredPropertyTransition::read(document, node))
            .collect();
    }
}

impl AuthoredPropertyTransition {
    fn read(document: &Document, node: &Node) -> Self {
        let field = |name| node.attribute(name).map(str::to_owned);
        let editable_timing = if native_transition(node) {
            timing::FIELDS
                .into_iter()
                .filter(|name| timing::editable(document, node, name))
                .collect()
        } else {
            Vec::new()
        };
        let seconds = |name| {
            editable_timing.contains(&name).then(|| {
                node.attribute(name)
                    .and_then(|value| crate::xaml::parse_time(value).ok())
                    .unwrap_or_default()
            })
        };
        Self {
            id: node.id,
            type_name: node.local_name().into(),
            source: node.span,
            property: field("Property"),
            duration: field("Duration"),
            delay: field("Delay"),
            easing: field("Easing"),
            duration_seconds: seconds("Duration"),
            delay_seconds: seconds("Delay"),
            editable_timing,
        }
    }
}

fn children<'a>(document: &'a Document, node: &Node) -> impl Iterator<Item = &'a Node> {
    document
        .ast()
        .element_children(node.id)
        .into_iter()
        .filter_map(|id| document.ast().node(id))
}

fn is_style(node: &Node) -> bool {
    is_styling_element(node, "Style") || is_styling_element(node, "ControlTheme")
}

fn is_collection(node: &Node) -> bool {
    node.local_name() == "Transitions" && native_animation_namespace(node)
}

fn is_styling_element(node: &Node, name: &str) -> bool {
    node.local_name() == name
        && matches!(
            node.namespace_uri.as_deref(),
            Some(
                NATIVE_XMLNS
                    | "using:FerroUI.Styling"
                    | "clr-namespace:FerroUI.Styling;assembly=FerroUI.Base"
            )
        )
}

fn native_animation_namespace(node: &Node) -> bool {
    matches!(
        node.namespace_uri.as_deref(),
        Some(
            NATIVE_XMLNS
                | "using:FerroUI.Animation"
                | "clr-namespace:FerroUI.Animation;assembly=FerroUI.Base"
        )
    )
}

fn native_transition(node: &Node) -> bool {
    native_animation_namespace(node)
        && matches!(
            node.local_name(),
            "BoolTransition"
                | "BoxShadowsTransition"
                | "BrushTransition"
                | "ColorTransition"
                | "CornerRadiusTransition"
                | "DoubleTransition"
                | "EffectTransition"
                | "FloatTransition"
                | "IntegerTransition"
                | "PointTransition"
                | "RelativePointTransition"
                | "Rotate3DTransition"
                | "SizeTransition"
                | "SolidColorBrushTransition"
                | "ThicknessTransition"
                | "TransformOperationsTransition"
                | "VectorTransition"
        )
}

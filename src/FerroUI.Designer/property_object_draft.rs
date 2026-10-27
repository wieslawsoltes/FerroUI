//! Source-preserving local object draft. Changes are deliberately isolated from the host
//! document until its guarded Apply callback accepts the complete property value.
use ferroui_designer_core::{Document, Edit, NodeId};

pub struct PropertyObjectDraft {
    document: Document,
}
impl PropertyObjectDraft {
    pub fn new(source: &str) -> Result<Self, String> {
        let document = Document::new(format!("<DesignerValue xmlns=\"https://github.com/ferroui\" xmlns:x=\"http://schemas.microsoft.com/winfx/2006/xaml\">{source}</DesignerValue>")).map_err(|e| e.to_string())?;
        Ok(Self { document })
    }
    pub fn document(&self) -> &Document {
        &self.document
    }
    pub fn source(&self) -> String {
        let root = self
            .document
            .ast()
            .node(self.document.ast().root())
            .unwrap();
        self.document.source()[root.opening_span.end..root.closing_span.unwrap().start].to_owned()
    }
    pub fn nodes(&self) -> Vec<NodeId> {
        self.document
            .ast()
            .nodes_in_source_order()
            .filter(|node| node.is_element() && node.id != self.document.ast().root())
            .map(|node| node.id)
            .collect()
    }
    pub fn set_attribute(
        &mut self,
        node: NodeId,
        name: &str,
        value: Option<&str>,
    ) -> Result<(), String> {
        if node == self.document.ast().root() {
            return Err("The draft container is not an authored object".into());
        }
        let edits = ferroui_designer_tools::plan_component_action(
            &self.document,
            &ferroui_designer_tools::ComponentAction::SetProperty {
                node,
                property: name.into(),
                value: value.map(str::to_owned),
            },
        )
        .map_err(|error| error.to_string())?;
        self.document
            .apply("Edit object draft", edits)
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
    pub fn set_object(
        &mut self,
        node: NodeId,
        name: &str,
        source: Option<&str>,
    ) -> Result<(), String> {
        if node == self.document.ast().root() {
            return Err("The draft container is not an authored object".into());
        }
        let edits =
            ferroui_designer_tools::plan_property_object(&self.document, &[node], name, source)
                .map_err(|error| error.to_string())?;
        self.document
            .apply("Edit nested object draft", edits)
            .map(|_| ())
            .map_err(|error| error.to_string())
    }
    pub fn insert(&mut self, parent: Option<NodeId>, source: &str) -> Result<(), String> {
        let parent = parent.unwrap_or(self.document.ast().root());
        let index = self
            .document
            .ast()
            .node(parent)
            .ok_or("Object no longer exists")?
            .children
            .iter()
            .filter_map(|id| self.document.ast().node(*id))
            .filter(|node| node.is_element())
            .count();
        self.document
            .apply(
                "Add object",
                vec![Edit::InsertChild {
                    parent,
                    index,
                    source: source.into(),
                }],
            )
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
    pub fn remove(&mut self, node: NodeId) -> Result<(), String> {
        if node == self.document.ast().root() {
            return Err("The draft container cannot be removed".into());
        }
        self.document
            .apply("Remove object", vec![Edit::RemoveNode { node }])
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
    pub fn move_by(&mut self, node: NodeId, delta: isize) -> Result<(), String> {
        let parent = self
            .document
            .ast()
            .node(node)
            .and_then(|node| node.parent)
            .ok_or("Object no longer exists")?;
        let siblings = self
            .document
            .ast()
            .node(parent)
            .unwrap()
            .children
            .iter()
            .filter_map(|id| self.document.ast().node(*id))
            .filter(|node| node.is_element())
            .map(|node| node.id)
            .collect::<Vec<_>>();
        let index = siblings
            .iter()
            .position(|id| *id == node)
            .ok_or("Object no longer exists")?;
        let target = index
            .checked_add_signed(delta)
            .filter(|index| *index < siblings.len())
            .ok_or("Already at the end of this collection")?;
        self.document
            .apply(
                "Reorder objects",
                vec![Edit::MoveNode {
                    node,
                    parent,
                    index: target,
                }],
            )
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
}

pub(super) fn object_attributes(name: &str) -> &'static [(&'static str, &'static str)] {
    match name {
        "SolidColorBrush" => &[("Color", "Color"), ("Opacity", "f64")],
        "LinearGradientBrush" => &[
            ("StartPoint", "RelativePoint"),
            ("EndPoint", "RelativePoint"),
            ("Opacity", "f64"),
            ("SpreadMethod", "GradientSpreadMethod"),
        ],
        "RadialGradientBrush" => &[
            ("Center", "RelativePoint"),
            ("GradientOrigin", "RelativePoint"),
            ("RadiusX", "f64"),
            ("RadiusY", "f64"),
            ("Opacity", "f64"),
        ],
        "ConicGradientBrush" => &[
            ("Center", "RelativePoint"),
            ("Angle", "f64"),
            ("Opacity", "f64"),
        ],
        "GradientStop" => &[("Color", "Color"), ("Offset", "f64")],
        "TranslateTransform" => &[("X", "f64"), ("Y", "f64")],
        "RotateTransform" => &[("Angle", "f64"), ("CenterX", "f64"), ("CenterY", "f64")],
        "ScaleTransform" => &[
            ("ScaleX", "f64"),
            ("ScaleY", "f64"),
            ("CenterX", "f64"),
            ("CenterY", "f64"),
        ],
        "SkewTransform" => &[
            ("AngleX", "f64"),
            ("AngleY", "f64"),
            ("CenterX", "f64"),
            ("CenterY", "f64"),
        ],
        "MatrixTransform" => &[("Matrix", "Matrix")],
        _ => &[],
    }
}

pub(super) const OBJECT_PRESETS: &[(&str, &str)] = &[
    (
        "Solid color brush",
        "<SolidColorBrush Color=\"#FF6C9EFF\" />",
    ),
    (
        "Linear gradient",
        "<LinearGradientBrush StartPoint=\"0%, 0%\" EndPoint=\"100%, 100%\"><GradientStop Color=\"#FF6C9EFF\" Offset=\"0\" /><GradientStop Color=\"#FFB58AFF\" Offset=\"1\" /></LinearGradientBrush>",
    ),
    (
        "Radial gradient",
        "<RadialGradientBrush Center=\"50%, 50%\" GradientOrigin=\"50%, 50%\" RadiusX=\"0.5\" RadiusY=\"0.5\"><GradientStop Color=\"White\" Offset=\"0\" /><GradientStop Color=\"#FF6C9EFF\" Offset=\"1\" /></RadialGradientBrush>",
    ),
    (
        "Conic gradient",
        "<ConicGradientBrush Center=\"50%, 50%\" Angle=\"0\"><GradientStop Color=\"#FF6C9EFF\" Offset=\"0\" /><GradientStop Color=\"#FFB58AFF\" Offset=\"1\" /></ConicGradientBrush>",
    ),
    (
        "Transform group",
        "<TransformGroup><TranslateTransform X=\"0\" Y=\"0\" /><RotateTransform Angle=\"0\" /><ScaleTransform ScaleX=\"1\" ScaleY=\"1\" /></TransformGroup>",
    ),
    ("Translate", "<TranslateTransform X=\"0\" Y=\"0\" />"),
    ("Rotate", "<RotateTransform Angle=\"0\" />"),
    ("Scale", "<ScaleTransform ScaleX=\"1\" ScaleY=\"1\" />"),
    ("Skew", "<SkewTransform AngleX=\"0\" AngleY=\"0\" />"),
    ("Matrix", "<MatrixTransform Matrix=\"1,0,0,1,0,0\" />"),
    (
        "Gradient stop",
        "<GradientStop Color=\"White\" Offset=\"0.5\" />",
    ),
];

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn structured_object_changes_preserve_unknown_source_and_collection_order() {
        let source = "<LinearGradientBrush xmlns:u='urn:custom' u:Hint='keep'><!-- preserve --><GradientStop Color='Red' Offset='0'/><GradientStop Color='Blue' Offset='1'/></LinearGradientBrush>";
        let mut draft = PropertyObjectDraft::new(source).unwrap();
        let stops = draft
            .nodes()
            .into_iter()
            .filter(|id| draft.document().ast().node(*id).unwrap().local_name() == "GradientStop")
            .collect::<Vec<_>>();
        draft
            .set_attribute(stops[0], "Color", Some("Green"))
            .unwrap();
        draft.move_by(stops[0], 1).unwrap();
        let edited = draft.source();
        assert!(edited.contains("xmlns:u='urn:custom' u:Hint='keep'"));
        assert!(edited.contains("<!-- preserve -->"));
        assert!(edited.find("Blue").unwrap() < edited.find("Green").unwrap());
        assert_eq!(source.matches("Green").count(), 0);
    }

    #[test]
    fn nested_object_property_edits_replace_scalar_or_graph_without_duplicate_values() {
        let mut draft = PropertyObjectDraft::new("<SolidColorBrush xmlns:u='urn:app' Color='Red' Transform='rotate(10deg)' u:Hint='Keep'/>").unwrap();
        let root = draft.nodes()[0];
        draft
            .set_object(root, "Transform", Some("<RotateTransform Angle='20' />"))
            .unwrap();
        assert!(!draft.source().contains("Transform='rotate"));
        assert!(draft.source().contains("SolidColorBrush.Transform"));
        draft
            .set_attribute(root, "Transform", Some("rotate(30deg)"))
            .unwrap();
        assert!(!draft.source().contains("SolidColorBrush.Transform"));
        assert!(draft.source().contains("u:Hint='Keep'"));
        assert_eq!(
            draft
                .document()
                .ast()
                .node(root)
                .unwrap()
                .attribute("Transform"),
            Some("rotate(30deg)")
        );
    }
}

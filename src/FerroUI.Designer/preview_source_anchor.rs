use ferroui_designer_core::{Document, NodeId, SourceAnchor, SourceSegment};
use serde::{Deserialize, Serialize};

/// A namespace-qualified path to an authored stage element.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreviewSourceAnchor {
    pub path: Vec<PreviewSourceSegment>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreviewSourceSegment {
    pub namespace: String,
    pub type_name: String,
    pub name: Option<String>,
    pub key: Option<String>,
}

impl PreviewSourceAnchor {
    pub fn capture(document: &Document, node: NodeId) -> Result<Self, String> {
        SourceAnchor::capture(document, node).map(Self::from)
    }

    pub fn resolve(&self, document: &Document) -> Result<NodeId, String> {
        SourceAnchor::from(self).resolve(document)
    }

    pub fn text_size(&self) -> usize {
        SourceAnchor::from(self).text_size()
    }

    pub fn validate(&self) -> Result<(), String> {
        SourceAnchor::from(self).validate()
    }
}

impl From<SourceAnchor> for PreviewSourceAnchor {
    fn from(anchor: SourceAnchor) -> Self {
        Self {
            path: anchor
                .path
                .into_iter()
                .map(|segment| PreviewSourceSegment {
                    namespace: segment.namespace,
                    type_name: segment.type_name,
                    name: segment.name,
                    key: segment.key,
                })
                .collect(),
        }
    }
}

impl From<&PreviewSourceAnchor> for SourceAnchor {
    fn from(anchor: &PreviewSourceAnchor) -> Self {
        Self {
            path: anchor
                .path
                .iter()
                .map(|segment| SourceSegment {
                    namespace: segment.namespace.clone(),
                    type_name: segment.type_name.clone(),
                    name: segment.name.clone(),
                    key: segment.key.clone(),
                })
                .collect(),
        }
    }
}

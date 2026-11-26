use crate::{Document, Node, NodeId};
use serde::{Deserialize, Serialize};

/// A namespace-qualified source path. No runtime or session-local numeric node ID
/// is persisted. Named/keyed paths survive sibling insertion and reordering, while
/// ambiguous unnamed siblings are rejected instead of guessing which view was meant.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceAnchor {
    pub path: Vec<SourceSegment>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceSegment {
    pub namespace: String,
    pub type_name: String,
    pub name: Option<String>,
    pub key: Option<String>,
}

impl SourceSegment {
    fn from_node(node: &Node) -> Self {
        Self {
            namespace: node.namespace_uri.clone().unwrap_or_default(),
            type_name: node.local_name().into(),
            name: node.xaml_name().map(str::to_owned),
            key: node
                .attributes
                .iter()
                .find(|attribute| {
                    attribute.namespace_uri.as_deref()
                        == Some("http://schemas.microsoft.com/winfx/2006/xaml")
                        && attribute.name.rsplit(':').next() == Some("Key")
                })
                .map(|attribute| attribute.value.clone()),
        }
    }

    fn matches(&self, node: &Node) -> bool {
        node.is_element() && *self == Self::from_node(node)
    }
}

impl SourceAnchor {
    pub fn capture(document: &Document, node: NodeId) -> Result<Self, String> {
        if !document.is_valid() {
            return Err("Repair the XAML before capturing a preview target".into());
        }
        let mut path = Vec::new();
        let mut current = Some(node);
        while let Some(id) = current {
            let node = document
                .ast()
                .node(id)
                .filter(|node| node.is_element())
                .ok_or("Preview target must be an authored element")?;
            path.push(SourceSegment::from_node(node));
            current = node.parent;
        }
        path.reverse();
        let anchor = Self { path };
        anchor.validate()?;
        if anchor.resolve(document)? != node {
            return Err("Preview target does not have an unambiguous source path".into());
        }
        Ok(anchor)
    }

    pub fn resolve(&self, document: &Document) -> Result<NodeId, String> {
        self.validate()?;
        if !document.is_valid() {
            return Err("The preview target source is invalid".into());
        }
        let root = document
            .ast()
            .node(document.ast().root())
            .ok_or("Missing source root")?;
        if !self.path[0].matches(root) {
            return Err("The preview target root was replaced or renamed".into());
        }
        let mut current = root.id;
        for segment in self.path.iter().skip(1) {
            let owner = document
                .ast()
                .node(current)
                .ok_or("Missing preview target ancestor")?;
            let candidates = owner
                .children
                .iter()
                .filter_map(|id| document.ast().node(*id))
                .filter(|node| segment.matches(node))
                .map(|node| node.id)
                .collect::<Vec<_>>();
            current = match candidates.as_slice() {
                [only] => *only,
                [] => return Err("The preview target was removed, renamed or moved".into()),
                _ => return Err("The preview target is ambiguous; name the control or use a unique resource key".into()),
            };
        }
        Ok(current)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.path.is_empty() || self.path.len() > 128 {
            return Err("A preview target path must contain between 1 and 128 elements".into());
        }
        for segment in &self.path {
            bounded(&segment.namespace, 4096, "Target namespace", true)?;
            bounded(&segment.type_name, 512, "Target type", false)?;
            for value in [&segment.name, &segment.key].into_iter().flatten() {
                bounded(value, 4096, "Target name or key", false)?;
            }
        }
        if self.text_size() > 65_536 {
            return Err("Preview target path text exceeds 64 KiB".into());
        }
        Ok(())
    }

    /// Conservative text size used when bounding saved preview state.
    pub fn text_size(&self) -> usize {
        self.path
            .iter()
            .map(|segment| {
                segment.namespace.len()
                    + segment.type_name.len()
                    + segment.name.as_ref().map_or(0, String::len)
                    + segment.key.as_ref().map_or(0, String::len)
                    + 64
            })
            .sum()
    }
}

fn bounded(value: &str, limit: usize, label: &str, empty: bool) -> Result<(), String> {
    if (!empty && value.trim().is_empty()) || value.len() > limit || value.contains('\0') {
        return Err(format!(
            "{label} must be {}and at most {limit} bytes",
            if empty { "" } else { "nonempty " }
        ));
    }
    Ok(())
}

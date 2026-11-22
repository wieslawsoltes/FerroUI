//! Source-backed image placement shared by Studio and the editor preview host.
use crate::{AssetIcon, DesignerAsset, DesignerSession};
use ferroui_designer_core::{Document, Edit, MarkupValue, Node, NodeId, NodeKind};
use ferroui_designer_language::Schema;
use ferroui_designer_project::{is_packaged_asset_uri, is_raster_asset, relative_asset_path};
use std::path::Path;

const IMAGE_ICON: &str =
    "F1 M3 4H21V20H3Z M5 6V18H19V6Z M6 16L10 11L13 14L16 10L18 13V17H6Z M7 7H10V10H7Z";

/// Construct an Image snippet with the document engine's escaping rules.
pub fn raster_asset_fragment(source: &str) -> Result<String, String> {
    let mut document = Document::new(
        r#"<Image xmlns="https://github.com/ferroui" Stretch="Uniform" Width="120"/>"#,
    )
    .map_err(|error| error.to_string())?;
    let node = document.ast().root();
    document
        .apply(
            "Set image source",
            vec![Edit::SetAttribute {
                node,
                name: "Source".into(),
                value: source.into(),
            }],
        )
        .map_err(|error| error.to_string())?;
    Ok(document.source().into())
}

pub fn xaml_raster_properties(document: &Document, schema: &Schema, node: &Node) -> Vec<String> {
    if node.kind != NodeKind::Element {
        return Vec::new();
    }
    schema
        .image_property_names(node.local_name(), node.namespace_uri.as_deref())
        .into_iter()
        .filter(|name| {
            let matching_member = |candidate: &str| {
                candidate
                    .rsplit_once('.')
                    .is_some_and(|(_, property)| property == *name)
            };
            !node
                .children
                .iter()
                .filter_map(|id| document.ast().node(*id))
                .any(|child| {
                    child.kind == NodeKind::PropertyElement && matching_member(child.local_name())
                })
                && node
                    .attributes
                    .iter()
                    .filter(|attribute| attribute.name == *name || matching_member(&attribute.name))
                    .all(|attribute| {
                        attribute.name == *name
                            && matches!(attribute.markup(), Ok(MarkupValue::Literal(_)))
                    })
        })
        .map(str::to_owned)
        .collect()
}

impl DesignerSession {
    pub(crate) fn raster_properties(&self, node: NodeId, schema: &Schema) -> Vec<String> {
        if !self.can_manipulate() || self.ensure_layer_manipulable(node).is_err() {
            return Vec::new();
        }
        let Some(element) = self.document().ast().node(node) else {
            return Vec::new();
        };
        if let Some(rust) = &self.rust {
            let Some((index, _)) = self
                .rust_map()
                .and_then(|map| map.control(element.span.start))
            else {
                return Vec::new();
            };
            return rust
                .projection()
                .ok()
                .and_then(|projection| {
                    projection
                        .nodes
                        .get(index)
                        .map(|node| node.editable_image_properties())
                })
                .unwrap_or_default();
        }
        xaml_raster_properties(self.document(), schema, element)
    }

    pub(crate) fn set_published_raster(
        &mut self,
        node: NodeId,
        property: Option<&str>,
        source: &str,
        schema: &Schema,
    ) -> Result<(), String> {
        if !is_packaged_asset_uri(source) {
            return Err("Choose a published address from the Assets view".into());
        }
        self.validate_raster_asset(source)?;
        let properties = self.raster_properties(node, schema);
        let selected = property.or_else(|| {
            properties
                .iter()
                .map(String::as_str)
                .find(|name| *name == "Source")
                .or_else(|| (properties.len() == 1).then(|| properties[0].as_str()))
        });
        let property = selected
            .filter(|name| properties.iter().any(|property| property == name))
            .ok_or_else(|| {
                if properties.is_empty() {
                    "Select an editable image property".into()
                } else {
                    format!(
                        "Choose an available image property: {}",
                        properties.join(", ")
                    )
                }
            })?;
        self.set_property(&[node], property, source)
    }

    pub(crate) fn validate_toolbox_asset(&self, asset: &DesignerAsset) -> Result<(), String> {
        if is_packaged_asset_uri(&asset.id) {
            self.validate_raster_asset(&asset.id)?;
        }
        Ok(())
    }

    /// Validate a raster passed through a placement action for the current design.
    pub fn validate_raster_asset(&self, source: &str) -> Result<(), String> {
        if source.len() <= 4096 && relative_asset_path(source).is_some() {
            return Ok(());
        }
        if source.len() <= 4096 && is_packaged_asset_uri(source) {
            let assets = self
                .preview
                .packaged_assets()
                .ok_or("Build the owning Cargo package to use its published images")?;
            let input = assets.resolve(source).map_err(|error| error.to_string())?;
            if input.is_some_and(|asset| is_raster_asset(&asset.file)) {
                return Ok(());
            }
        }
        Err("Choose an available raster image from the current Assets view".into())
    }

    pub(crate) fn published_rasters(&self) -> Vec<DesignerAsset> {
        self.preview
            .packaged_assets()
            .into_iter()
            .flat_map(|assets| assets.published())
            .filter_map(|asset| {
                let path = Path::new(asset.key);
                if !is_raster_asset(path) || asset.address.len() > 4096 {
                    return None;
                }
                Some(DesignerAsset {
                    id: asset.address.clone(),
                    title: path.file_name()?.to_str()?.into(),
                    category: "Packaged images".into(),
                    description: format!("{} · {}", asset.assembly, asset.key),
                    source: raster_asset_fragment(&asset.address).ok()?,
                    icon: AssetIcon::CustomPath(IMAGE_ICON.into()),
                })
            })
            .collect()
    }
}

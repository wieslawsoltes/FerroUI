//! Build receipt for the byte streams in a package's compiled asset table.

use super::{field, json_string, JsonParser, JsonValue};
use std::path::PathBuf;

#[path = "packaged_asset_publication.rs"]
mod publication;
pub use publication::{AssetDeclaration, AssetPublication};

const VERSION: u32 = 1;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackagedAssetIndex {
    pub manifest: PathBuf,
    pub assembly: String,
    pub crate_name: String,
    pub assets: Vec<PackagedAsset>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackagedAsset {
    /// Exact, rooted key in `compiled_markup::ASSETS`.
    pub key: String,
    /// Authored input relative to the Cargo manifest directory.
    pub source: PathBuf,
}

impl PackagedAssetIndex {
    pub const FILE_NAME: &str = "packaged-assets.json";

    pub fn parse(source: &str) -> Result<Self, String> {
        Self::parse_with_publication(source).map(|(index, _)| index)
    }

    pub fn parse_with_publication(
        source: &str,
    ) -> Result<(Self, Option<AssetPublication>), String> {
        let json = JsonParser {
            text: source.as_bytes(),
            at: 0,
        }
        .document()?;
        let root = json.object("packaged asset index")?;
        if field(root, "version")?.integer("version")? != VERSION {
            return Err("Unsupported packaged asset index version".into());
        }
        let assets = field(root, "assets")?
            .array("assets")?
            .iter()
            .map(|asset| {
                let asset = asset.object("packaged asset")?;
                Ok(PackagedAsset {
                    key: field(asset, "key")?.string("key")?.into(),
                    source: field(asset, "source")?.string("source")?.into(),
                })
            })
            .collect::<Result<_, String>>()?;
        let index = Self {
            manifest: field(root, "manifest")?.string("manifest")?.into(),
            assembly: field(root, "assembly")?.string("assembly")?.into(),
            crate_name: field(root, "crate_name")?.string("crate_name")?.into(),
            assets,
        };
        let publication = root
            .iter()
            .find(|(key, _)| key == "publication")
            .map(|(_, value)| AssetPublication::parse(value))
            .transpose()?;
        Ok((index, publication))
    }

    pub fn to_json(&self) -> String {
        let assets = self
            .assets
            .iter()
            .map(|asset| {
                format!(
                    "{{\"key\":{},\"source\":{}}}",
                    json_string(&asset.key),
                    json_string(&asset.source.to_string_lossy()),
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{{\"version\":{VERSION},\"manifest\":{},\"assembly\":{},\"crate_name\":{},\"assets\":[{assets}]}}",
            json_string(&self.manifest.to_string_lossy()),
            json_string(&self.assembly),
            json_string(&self.crate_name),
        )
    }

    pub fn to_json_with_publication(&self, publication: &AssetPublication) -> String {
        let mut json = self.to_json();
        json.pop();
        json.push_str(",\"publication\":");
        json.push_str(&publication.to_json());
        json.push('}');
        json
    }
}

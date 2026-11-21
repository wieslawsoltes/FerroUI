use crate::protocol::Asset;
use ferroui_designer::{default_designer_assets, DesignerAsset};
use ferroui_designer_core::Document;
use ferroui_designer_rsx::RsxCatalog;
use std::sync::OnceLock;

pub(crate) fn assets() -> &'static [DesignerAsset] {
    static ASSETS: OnceLock<Vec<DesignerAsset>> = OnceLock::new();
    ASSETS.get_or_init(default_designer_assets)
}

pub(crate) fn catalog() -> Vec<Asset> {
    assets().iter().map(|asset| describe(asset, RsxCatalog::built_in())).collect()
}

pub(crate) fn describe(asset: &DesignerAsset, catalog: &RsxCatalog) -> Asset {
    Asset {
            id: asset.id.clone(),
            title: asset.title.clone(),
            category: asset.category.clone(),
            description: asset.description.clone(),
            icon: asset.icon.path().trim_start_matches("F1 ").to_owned(),
            rust: Document::new(&asset.source)
                .ok()
                .is_some_and(|document| catalog.xaml_to_rsx(&document).is_ok()),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn studio_catalog_round_trips_in_rsx() {
        for asset in super::catalog() {
            assert!(asset.rust, "{} cannot be inserted into RSX", asset.id);
        }
    }
}

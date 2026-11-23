//! `MapControl`: a map of raster tiles, drawn by FerroUI on the GPU of every backend.
//!
//! No tile provider is built in. The application gives a [`RasterTileSource`] with the address
//! template and the attribution its provider requires, and an [`ITileLoader`] that fetches.

mod camera;
mod grid_tiles;
mod map_control;
mod map_elements;
mod projection;
mod tile_cache;
mod tile_source;
mod tile_store;

#[cfg(test)]
mod tests;

pub use camera::{MapCamera, MAX_ZOOM, MIN_ZOOM};
pub use grid_tiles::{grid_tile_source, GridTileLoader};
pub use map_control::{MapControl, MapControlAutomationPeer, MapInteractionMode};
pub use map_elements::{
    MapElement, MapElementClickEventArgs, MapElementsLayer, MapIcon, MapPolygon, MapPolyline,
};
pub use projection::{
    normalize_heading, pan, project, shortest_turn, unproject, view_to_world, visible_tiles,
    world_size, world_to_view, wrap_longitude, zoom_about, BasicGeoposition, MapView, PlacedTile,
    TileId, MAX_LATITUDE, MAX_TILE_ZOOM, TILE_SIZE,
};
pub use tile_cache::{source_key, DiskTileCache, LruBudget};
pub use tile_source::{
    ITileLoader, RasterTileSource, SharedTileLoader, TileError, TileFuture, TileTemplateError,
};

// Shared by the registration at run time and the scanner of the build-time XAML metadata.
const NAMESPACES: &[(&str, &str)] = &[("ferroui_controls_maps", "FerroUI.Controls.Maps")];
pub static ASSEMBLY: ferroui_base::metadata::MarkupAssembly =
    ferroui_base::metadata::MarkupAssembly {
        name: "FerroUI.Controls.Maps",
        crate_name: "ferroui_controls_maps",
        xmlns_definitions: &[ferroui_base::metadata::XmlnsDefinition {
            xml_namespace: ferroui_base::metadata::FERRO_XML_NAMESPACE,
            namespace: "FerroUI.Controls.Maps",
        }],
        xmlns_prefixes: &[],
        metadata: &[],
    };
ferroui_base::ferro_rust_paths! {
    classes: [crate::MapControl, crate::MapControlAutomationPeer],
    types: [crate::MapInteractionMode], contracts: [], generics: [], generic_contracts: [],
}
const TYPES: &[&ferroui_base::TypeInfo] = &[MapControl::TYPE, MapControlAutomationPeer::TYPE];

/// Registers the namespace, the assembly and the table of types of this crate and of the crates
/// it is built on. Cheap and idempotent.
pub fn register_types() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        ferroui_controls::register_types();
        ferroui_base::TypeInfo::register_namespaces(NAMESPACES);
        ferroui_base::metadata::MarkupAssembly::register_table(&ASSEMBLY);
        register_rust_paths();
        ferroui_base::TypeInfo::register_table(TYPES);
    });
}

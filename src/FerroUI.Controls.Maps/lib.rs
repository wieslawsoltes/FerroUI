//! `MapControl`: a map of raster tiles, drawn by FerroUI on the GPU of every backend.
//!
//! No tile provider is built in. The application gives a [`RasterTileSource`] with the address
//! template and the attribution its provider requires, and an [`ITileLoader`] that fetches.

mod camera;
mod projection;
mod tile_cache;
mod tile_source;

pub use camera::{MapCamera, MAX_ZOOM, MIN_ZOOM};
pub use projection::{
    pan, project, unproject, visible_tiles, world_size, wrap_longitude, zoom_about,
    BasicGeoposition, PlacedTile, TileId, MAX_LATITUDE, MAX_TILE_ZOOM, TILE_SIZE,
};
pub use tile_cache::{source_key, DiskTileCache, LruBudget};
pub use tile_source::{
    ITileLoader, RasterTileSource, SharedTileLoader, TileError, TileFuture, TileTemplateError,
};

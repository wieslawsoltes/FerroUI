//! Web Mercator: geographic positions, world pixels and the tiles that cover a view.

use std::f64::consts::PI;

/// The latitude at which Web Mercator's square world ends.
pub const MAX_LATITUDE: f64 = 85.051_128_779_806_59;
/// The side of a tile in device-independent pixels at its own zoom level.
pub const TILE_SIZE: f64 = 256.0;
/// The deepest zoom level a tile address can have here.
pub const MAX_TILE_ZOOM: u8 = 30;

/// A position on the globe, in degrees.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BasicGeoposition {
    pub latitude: f64,
    pub longitude: f64,
}

impl BasicGeoposition {
    /// A position with the latitude limited to the map and the longitude in `-180..180`.
    pub fn normalized(self) -> Self {
        Self {
            latitude: if self.latitude.is_nan() {
                0.0
            } else {
                self.latitude.clamp(-MAX_LATITUDE, MAX_LATITUDE)
            },
            longitude: wrap_longitude(self.longitude),
        }
    }
}

/// `longitude` in `-180..180`; 180 itself becomes -180, the same meridian.
pub fn wrap_longitude(longitude: f64) -> f64 {
    if !longitude.is_finite() {
        return 0.0;
    }
    (longitude + 180.0).rem_euclid(360.0) - 180.0
}

/// The side of the world in pixels at a zoom level.
pub fn world_size(zoom: f64) -> f64 {
    TILE_SIZE * zoom.exp2()
}

/// The position in world pixels at a zoom level, from the top left of the world.
pub fn project(position: BasicGeoposition, zoom: f64) -> (f64, f64) {
    let position = position.normalized();
    let size = world_size(zoom);
    let x = (position.longitude + 180.0) / 360.0 * size;
    let sine = position.latitude.to_radians().sin();
    let y = (0.5 - ((1.0 + sine) / (1.0 - sine)).ln() / (4.0 * PI)) * size;
    (x, y)
}

/// The position at world pixels of a zoom level. `x` wraps around the world, `y` is limited to it.
pub fn unproject(x: f64, y: f64, zoom: f64) -> BasicGeoposition {
    let size = world_size(zoom);
    let longitude = wrap_longitude(x / size * 360.0 - 180.0);
    let north = PI * (1.0 - 2.0 * (y / size).clamp(0.0, 1.0));
    let latitude = north.sinh().atan().to_degrees();
    BasicGeoposition {
        latitude: latitude.clamp(-MAX_LATITUDE, MAX_LATITUDE),
        longitude,
    }
}

/// The center after moving the content by `dx`, `dy` pixels: dragging right moves the center west.
pub fn pan(center: BasicGeoposition, zoom: f64, dx: f64, dy: f64) -> BasicGeoposition {
    let (x, y) = project(center, zoom);
    unproject(x - dx, y - dy, zoom)
}

/// The center after a zoom that keeps the point at `anchor`, in pixels from the center of the
/// view, over the same place on the globe.
pub fn zoom_about(
    center: BasicGeoposition,
    zoom: f64,
    new_zoom: f64,
    anchor: (f64, f64),
) -> BasicGeoposition {
    let (x, y) = project(center, zoom);
    let scale = (new_zoom - zoom).exp2();
    let (anchor_x, anchor_y) = ((x + anchor.0) * scale, (y + anchor.1) * scale);
    unproject(anchor_x - anchor.0, anchor_y - anchor.1, new_zoom)
}

/// The address of a tile.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TileId {
    pub zoom: u8,
    pub x: u32,
    pub y: u32,
}

impl TileId {
    /// The tile that covers this one at a lower zoom level, or `None` above it.
    pub fn ancestor(self, zoom: u8) -> Option<TileId> {
        let levels = self.zoom.checked_sub(zoom)?;
        Some(TileId {
            zoom,
            x: self.x >> levels,
            y: self.y >> levels,
        })
    }

    /// The part of the ancestor at `zoom` this tile covers: left, top and side, from 0 to 1.
    pub fn part_of_ancestor(self, zoom: u8) -> Option<(f64, f64, f64)> {
        let ancestor = self.ancestor(zoom)?;
        let levels = self.zoom - zoom;
        let count = f64::from(1u32 << levels);
        let left = f64::from(self.x - (ancestor.x << levels)) / count;
        let top = f64::from(self.y - (ancestor.y << levels)) / count;
        Some((left, top, 1.0 / count))
    }

    /// The key of the tile in the quadtree addressing some tile services use.
    pub fn quadkey(self) -> String {
        (1..=self.zoom)
            .rev()
            .map(|level| {
                let mask = 1u32 << (level - 1);
                let digit = u8::from(self.x & mask != 0) + 2 * u8::from(self.y & mask != 0);
                char::from(b'0' + digit)
            })
            .collect()
    }
}

/// A tile placed in a view, in pixels from its top left corner.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlacedTile {
    pub id: TileId,
    pub left: f64,
    pub top: f64,
    pub size: f64,
}

/// The tiles that cover a view of `width` by `height` pixels, nearest the center first so that
/// the middle of the view loads first. The world repeats sideways; above and below it there are
/// no tiles. `max_tile_zoom` is the deepest level the source has.
pub fn visible_tiles(
    center: BasicGeoposition,
    zoom: f64,
    width: f64,
    height: f64,
    max_tile_zoom: u8,
) -> Vec<PlacedTile> {
    if !(width > 0.0 && height > 0.0) || !zoom.is_finite() {
        return Vec::new();
    }
    let tile_zoom = zoom
        .floor()
        .clamp(0.0, f64::from(max_tile_zoom.min(MAX_TILE_ZOOM))) as u8;
    let size = TILE_SIZE * (zoom - f64::from(tile_zoom)).exp2();
    let count = 1i64 << tile_zoom;
    let (center_x, center_y) = project(center, zoom);
    let (view_left, view_top) = (center_x - width / 2.0, center_y - height / 2.0);

    let first_column = (view_left / size).floor() as i64;
    let last_column = ((view_left + width) / size).ceil() as i64;
    let first_row = ((view_top / size).floor() as i64).max(0);
    let last_row = (((view_top + height) / size).ceil() as i64).min(count);

    let mut tiles = Vec::new();
    for row in first_row..last_row {
        for column in first_column..last_column {
            tiles.push(PlacedTile {
                id: TileId {
                    zoom: tile_zoom,
                    x: column.rem_euclid(count) as u32,
                    y: row as u32,
                },
                left: column as f64 * size - view_left,
                top: row as f64 * size - view_top,
                size,
            });
        }
    }
    let distance = |tile: &PlacedTile| {
        let dx = tile.left + tile.size / 2.0 - width / 2.0;
        let dy = tile.top + tile.size / 2.0 - height / 2.0;
        dx * dx + dy * dy
    };
    tiles.sort_by(|a, b| distance(a).total_cmp(&distance(b)));
    tiles
}

#[cfg(test)]
mod tests {
    use super::*;

    fn near(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-6, "{a} is not {b}");
    }

    #[test]
    fn the_corners_of_the_world_project_to_its_corners() {
        let (x, y) = project(
            BasicGeoposition {
                latitude: MAX_LATITUDE,
                longitude: -180.0,
            },
            0.0,
        );
        near(x, 0.0);
        near(y, 0.0);
        let (x, y) = project(
            BasicGeoposition {
                latitude: 0.0,
                longitude: 0.0,
            },
            1.0,
        );
        near(x, 256.0);
        near(y, 256.0);
    }

    #[test]
    fn positions_round_trip() {
        for (latitude, longitude) in [
            (0.0, 0.0),
            (52.2297, 21.0122),
            (-33.8688, 151.2093),
            (84.0, -179.5),
        ] {
            for zoom in [0.0, 3.5, 12.0, 19.25] {
                let (x, y) = project(
                    BasicGeoposition {
                        latitude,
                        longitude,
                    },
                    zoom,
                );
                let back = unproject(x, y, zoom);
                near(back.latitude, latitude);
                near(back.longitude, longitude);
            }
        }
    }

    #[test]
    fn longitudes_wrap_and_latitudes_are_limited() {
        near(wrap_longitude(190.0), -170.0);
        near(wrap_longitude(-190.0), 170.0);
        near(wrap_longitude(180.0), -180.0);
        near(wrap_longitude(f64::NAN), 0.0);
        near(
            BasicGeoposition {
                latitude: 90.0,
                longitude: 0.0,
            }
            .normalized()
            .latitude,
            MAX_LATITUDE,
        );
    }

    #[test]
    fn panning_across_the_date_line_wraps() {
        let center = BasicGeoposition {
            latitude: 0.0,
            longitude: 179.0,
        };
        let moved = pan(center, 2.0, -(world_size(2.0) * 2.0 / 360.0), 0.0);
        near(moved.longitude, -179.0);
    }

    #[test]
    fn a_zoom_keeps_the_place_under_the_anchor() {
        let center = BasicGeoposition {
            latitude: 40.0,
            longitude: -74.0,
        };
        let anchor = (120.0, -80.0);
        let (x, y) = project(center, 5.0);
        let place = unproject(x + anchor.0, y + anchor.1, 5.0);
        let new_center = zoom_about(center, 5.0, 7.5, anchor);
        let (x, y) = project(new_center, 7.5);
        let after = unproject(x + anchor.0, y + anchor.1, 7.5);
        near(after.latitude, place.latitude);
        near(after.longitude, place.longitude);
    }

    #[test]
    fn the_quadkey_follows_the_tile() {
        assert_eq!(
            TileId {
                zoom: 3,
                x: 3,
                y: 5
            }
            .quadkey(),
            "213"
        );
        assert_eq!(
            TileId {
                zoom: 0,
                x: 0,
                y: 0
            }
            .quadkey(),
            ""
        );
    }

    #[test]
    fn an_ancestor_and_the_part_it_covers() {
        let tile = TileId {
            zoom: 5,
            x: 13,
            y: 22,
        };
        assert_eq!(
            tile.ancestor(3),
            Some(TileId {
                zoom: 3,
                x: 3,
                y: 5
            })
        );
        assert_eq!(tile.ancestor(6), None);
        assert_eq!(tile.part_of_ancestor(3), Some((0.25, 0.5, 0.25)));
    }

    #[test]
    fn the_tiles_cover_the_view_without_gaps() {
        let center = BasicGeoposition {
            latitude: 20.0,
            longitude: 10.0,
        };
        let (width, height) = (900.0, 500.0);
        let tiles = visible_tiles(center, 4.5, width, height, 19);
        assert!(!tiles.is_empty());
        for step_x in 0..30 {
            for step_y in 0..17 {
                let (x, y) = (
                    f64::from(step_x) * 30.0 + 1.0,
                    f64::from(step_y) * 29.0 + 1.0,
                );
                assert!(
                    tiles.iter().any(|t| x >= t.left
                        && x < t.left + t.size
                        && y >= t.top
                        && y < t.top + t.size),
                    "({x}, {y}) is not covered"
                );
            }
        }
        let first = tiles[0];
        assert!(
            first.left <= width / 2.0 && first.left + first.size >= width / 2.0,
            "the middle comes first"
        );
    }

    #[test]
    fn the_world_repeats_sideways_but_not_upwards() {
        let center = BasicGeoposition {
            latitude: 0.0,
            longitude: 179.0,
        };
        let tiles = visible_tiles(center, 1.0, 1200.0, 2000.0, 19);
        assert!(tiles.iter().all(|t| t.id.x < 2 && t.id.y < 2));
        assert!(tiles.iter().all(|t| t.top >= (2000.0 - 512.0) / 2.0 - 1e-6));
        let columns: std::collections::BTreeSet<i64> =
            tiles.iter().map(|t| t.left as i64).collect();
        assert!(columns.len() > 2, "the world is drawn more than once");
    }

    #[test]
    fn past_the_deepest_level_tiles_are_scaled() {
        let tiles = visible_tiles(BasicGeoposition::default(), 21.0, 256.0, 256.0, 19);
        assert!(tiles.iter().all(|t| t.id.zoom == 19));
        near(tiles[0].size, 1024.0);
    }

    #[test]
    fn an_empty_view_has_no_tiles() {
        assert!(visible_tiles(BasicGeoposition::default(), 3.0, 0.0, 100.0, 19).is_empty());
        assert!(visible_tiles(BasicGeoposition::default(), f64::NAN, 100.0, 100.0, 19).is_empty());
    }
}

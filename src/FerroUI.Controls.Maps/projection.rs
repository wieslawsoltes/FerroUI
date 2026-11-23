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

/// `heading` in `0..360`: the compass direction at the top of the view.
pub fn normalize_heading(heading: f64) -> f64 {
    if !heading.is_finite() {
        return 0.0;
    }
    let value = heading.rem_euclid(360.0);
    // `rem_euclid` of a tiny negative number rounds up to the modulus.
    if value >= 360.0 {
        0.0
    } else {
        value
    }
}

/// The shortest turn from one heading to another, in `-180..=180` degrees.
pub fn shortest_turn(from: f64, to: f64) -> f64 {
    let turn = (to - from).rem_euclid(360.0);
    if turn > 180.0 {
        turn - 360.0
    } else {
        turn
    }
}

/// A distance on the view as a distance on the world, whose north is up. At a heading of 90
/// degrees the top of the view is east.
pub fn view_to_world(dx: f64, dy: f64, heading: f64) -> (f64, f64) {
    let (sine, cosine) = heading.to_radians().sin_cos();
    (dx * cosine - dy * sine, dx * sine + dy * cosine)
}

/// A distance on the world as a distance on the view.
pub fn world_to_view(dx: f64, dy: f64, heading: f64) -> (f64, f64) {
    let (sine, cosine) = heading.to_radians().sin_cos();
    (dx * cosine + dy * sine, -dx * sine + dy * cosine)
}

/// What a view of a map shows, and the math between its pixels and the globe.
///
/// The tiles are placed in the frame of the world, north up, around the middle of the view; the
/// content is then turned about that middle by the heading.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MapView {
    pub center: BasicGeoposition,
    pub zoom: f64,
    pub heading: f64,
    pub width: f64,
    pub height: f64,
}

impl MapView {
    pub fn new(center: BasicGeoposition, zoom: f64, heading: f64, width: f64, height: f64) -> Self {
        Self {
            center: center.normalized(),
            zoom,
            heading: normalize_heading(heading),
            width,
            height,
        }
    }

    /// A point of the north-up frame of the tiles as a point of the view.
    pub fn frame_to_view(&self, x: f64, y: f64) -> (f64, f64) {
        let (dx, dy) = world_to_view(x - self.width / 2.0, y - self.height / 2.0, self.heading);
        (dx + self.width / 2.0, dy + self.height / 2.0)
    }

    /// A point of the view as a point of the north-up frame of the tiles.
    pub fn view_to_frame(&self, x: f64, y: f64) -> (f64, f64) {
        let (dx, dy) = view_to_world(x - self.width / 2.0, y - self.height / 2.0, self.heading);
        (dx + self.width / 2.0, dy + self.height / 2.0)
    }

    /// The pixel of the view that shows a place, on the copy of the world nearest the middle.
    pub fn place_to_view(&self, place: BasicGeoposition) -> (f64, f64) {
        let (center_x, center_y) = project(self.center, self.zoom);
        let (x, y) = project(place, self.zoom);
        let world = world_size(self.zoom);
        let x = x - world * ((x - center_x) / world).round();
        self.frame_to_view(
            x - center_x + self.width / 2.0,
            y - center_y + self.height / 2.0,
        )
    }

    /// The place shown at a pixel of the view.
    pub fn view_to_place(&self, x: f64, y: f64) -> BasicGeoposition {
        let (frame_x, frame_y) = self.view_to_frame(x, y);
        let (center_x, center_y) = project(self.center, self.zoom);
        unproject(
            center_x + frame_x - self.width / 2.0,
            center_y + frame_y - self.height / 2.0,
            self.zoom,
        )
    }

    /// The tiles that cover the view, placed in the north-up frame, nearest the middle first.
    pub fn tiles(&self, max_tile_zoom: u8) -> Vec<PlacedTile> {
        let (sine, cosine) = self.heading.to_radians().sin_cos();
        let (sine, cosine) = (sine.abs(), cosine.abs());
        let around_width = self.width * cosine + self.height * sine;
        let around_height = self.width * sine + self.height * cosine;
        let mut tiles = visible_tiles(
            self.center,
            self.zoom,
            around_width,
            around_height,
            max_tile_zoom,
        );
        let (shift_x, shift_y) = (
            (self.width - around_width) / 2.0,
            (self.height - around_height) / 2.0,
        );
        for tile in &mut tiles {
            tile.left += shift_x;
            tile.top += shift_y;
        }
        if self.heading != 0.0 {
            tiles.retain(|tile| self.shows(tile));
        }
        tiles
    }

    /// Whether a tile of the north-up frame overlaps the turned view: two rectangles are apart
    /// only if the axes of one of them separate them.
    fn shows(&self, tile: &PlacedTile) -> bool {
        let (left, top, right, bottom) = (
            tile.left,
            tile.top,
            tile.left + tile.size,
            tile.top + tile.size,
        );
        let in_view = [(left, top), (right, top), (right, bottom), (left, bottom)]
            .map(|(x, y)| self.frame_to_view(x, y));
        let in_frame = [
            (0.0, 0.0),
            (self.width, 0.0),
            (self.width, self.height),
            (0.0, self.height),
        ]
        .map(|(x, y)| self.view_to_frame(x, y));
        let apart = |points: &[(f64, f64); 4], low_x: f64, low_y: f64, high_x: f64, high_y: f64| {
            points.iter().all(|point| point.0 < low_x)
                || points.iter().all(|point| point.0 > high_x)
                || points.iter().all(|point| point.1 < low_y)
                || points.iter().all(|point| point.1 > high_y)
        };
        !apart(&in_view, 0.0, 0.0, self.width, self.height)
            && !apart(&in_frame, left, top, right, bottom)
    }
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
    fn a_heading_is_kept_in_its_range_and_turns_the_short_way() {
        near(normalize_heading(-90.0), 270.0);
        near(normalize_heading(450.0), 90.0);
        near(normalize_heading(f64::NAN), 0.0);
        assert!(normalize_heading(-1e-20) < 360.0);
        near(shortest_turn(350.0, 10.0), 20.0);
        near(shortest_turn(10.0, 350.0), -20.0);
    }

    #[test]
    fn at_a_heading_of_90_the_top_of_the_view_is_east() {
        let center = BasicGeoposition {
            latitude: 0.0,
            longitude: 0.0,
        };
        let view = MapView::new(center, 4.0, 90.0, 400.0, 300.0);
        let top = view.view_to_place(200.0, 0.0);
        assert!(top.longitude > 0.0, "{top:?}");
        near(top.latitude, 0.0);
        let east = BasicGeoposition {
            latitude: 0.0,
            longitude: 10.0,
        };
        let (x, y) = view.place_to_view(east);
        near(x, 200.0);
        assert!(y < 150.0, "east is up: {y}");
    }

    #[test]
    fn places_and_pixels_round_trip_at_any_heading() {
        let center = BasicGeoposition {
            latitude: 35.0,
            longitude: 139.0,
        };
        for heading in [0.0, 17.5, 90.0, 133.0, 180.0, 271.0, 359.0] {
            let view = MapView::new(center, 6.5, heading, 640.0, 480.0);
            near(view.place_to_view(center).0, 320.0);
            near(view.place_to_view(center).1, 240.0);
            for (x, y) in [(0.0, 0.0), (640.0, 0.0), (123.0, 456.0), (640.0, 480.0)] {
                let (back_x, back_y) = view.place_to_view(view.view_to_place(x, y));
                near(back_x, x);
                near(back_y, y);
            }
        }
    }

    #[test]
    fn the_tiles_cover_a_turned_view_without_gaps() {
        let center = BasicGeoposition {
            latitude: 20.0,
            longitude: 10.0,
        };
        for heading in [0.0, 30.0, 45.0, 90.0, 200.0, 315.0] {
            let view = MapView::new(center, 4.5, heading, 900.0, 500.0);
            let tiles = view.tiles(19);
            for step_x in 0..=30 {
                for step_y in 0..=20 {
                    let (x, y) =
                        view.view_to_frame(f64::from(step_x) * 30.0, f64::from(step_y) * 25.0);
                    assert!(
                        tiles.iter().any(|t| x >= t.left - 1e-6
                            && x <= t.left + t.size + 1e-6
                            && y >= t.top - 1e-6
                            && y <= t.top + t.size + 1e-6),
                        "heading {heading}: ({step_x}, {step_y}) is not covered"
                    );
                }
            }
        }
    }

    #[test]
    fn a_turned_view_asks_for_no_tile_it_does_not_show() {
        let center = BasicGeoposition {
            latitude: 0.0,
            longitude: 0.0,
        };
        let view = MapView::new(center, 6.0, 45.0, 1200.0, 100.0);
        let around = visible_tiles(
            center,
            6.0,
            (1300.0_f64) / 2.0_f64.sqrt(),
            1300.0 / 2.0_f64.sqrt(),
            19,
        );
        assert!(
            view.tiles(19).len() < around.len(),
            "the corners of the bounding box are left out"
        );
    }

    #[test]
    fn an_empty_view_has_no_tiles() {
        assert!(visible_tiles(BasicGeoposition::default(), 3.0, 0.0, 100.0, 19).is_empty());
        assert!(visible_tiles(BasicGeoposition::default(), f64::NAN, 100.0, 100.0, 19).is_empty());
    }
}

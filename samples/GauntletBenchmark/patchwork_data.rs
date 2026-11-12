//! The patchwork's data, as the benchmark's shared C# file makes it bit for bit: nine patches of
//! the still wall's tiles, three across and three down. Two never change; in the other seven a
//! frame changes a small part and leaves the rest as it was. Only what the patchwork shows is
//! kept.
//!
//! Integer arithmetic follows the 32-bit signed arithmetic of the shared C# file, with 64-bit
//! products where it widens to `long`, truncating division and a remainder with the sign of the
//! dividend (which Rust's `/` and `%` already are). Values the shared file computes in `float`
//! are `f32` here, so the same digits come out on the same frame.

use crate::perf_data::PALETTE_ARGB;

/// The patches in the order the screen shows them, row by row.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Patch {
    Still,
    Charts,
    Recolor,
    Cards,
    Reorder,
    Deep,
    Scroll,
    Paragraph,
    Pile,
}

impl Patch {
    /// Every patch, in the order the screen shows them.
    pub const ALL: [Patch; 9] = [
        Patch::Still,
        Patch::Charts,
        Patch::Recolor,
        Patch::Cards,
        Patch::Reorder,
        Patch::Deep,
        Patch::Scroll,
        Patch::Paragraph,
        Patch::Pile,
    ];
}

/// The items of a patch that change in turn: `columns` by `rows` tiles, cells or nests. Each
/// frame changes the items of one row, the next row down the frame after.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct InTurn {
    pub columns: i32,
    pub rows: i32,
}

impl InTurn {
    pub fn new(columns: i32, rows: i32) -> Self {
        Self { columns, rows }
    }

    /// The changes of item `index` up to frame `frame`.
    pub fn step(self, index: i32, frame: i32) -> i32 {
        (index / self.columns.max(1) + frame) / self.rows.max(1)
    }

    /// The row whose items change in frame `frame`, for frames from 1 on.
    pub fn changed_row(self, frame: i32) -> i32 {
        (self.rows - frame % self.rows) % self.rows
    }
}

/// One patch's tiles: `columns` by `rows` of them, `width` by `height` points each.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct PatchTiles {
    pub columns: i32,
    pub rows: i32,
    pub width: f32,
    pub height: f32,
}

impl PatchTiles {
    pub fn count(self) -> i32 {
        self.columns * self.rows
    }

    pub fn in_turn(self) -> InTurn {
        InTurn::new(self.columns, self.rows)
    }
}

/// A card of the cards patch in a frame: its number and its top in points from the patch's top.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct CardPlace {
    pub id: i32,
    pub top: f32,
}

/// The cards patch: a card is `columns` tiles across and `rows` rows of tiles under a header,
/// `height` points tall, and `slots` cards rest in the patch.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Cards {
    pub columns: i32,
    pub rows: i32,
    pub height: f32,
    pub slots: i32,
}

impl Cards {
    /// The places a frame may show a card at.
    pub fn places(self) -> i32 {
        self.slots + 1
    }

    pub fn tiles(self) -> i32 {
        self.columns * self.rows
    }

    /// The cards of a patch `columns` by `rows` tiles of `tile_height` points, in a patch
    /// `patch_height` points tall: a card holds a quarter of the patch's rows.
    pub fn of(columns: i32, rows: i32, tile_height: f32, patch_height: f32) -> Self {
        let card_rows = (rows / 4).max(1);
        let height = tile_height + card_rows as f32 * (tile_height + GAP);
        let slots = (((patch_height - GAP) / (height + GAP)) as i32).max(1);
        Self { columns, rows: card_rows, height, slots }
    }

    /// The card at place `place` in frame `frame`, the topmost first; none at the last place while
    /// no card is on its way in. The moves alternate, [`CARD_FRAMES`] frames each: a new card
    /// slides in and the others move down by a card, then the first and the third card move to
    /// each other's places and back.
    pub fn place(self, place: i32, frame: i32) -> Option<CardPlace> {
        let moves = frame / CARD_FRAMES;
        let along = (frame % CARD_FRAMES) as f32 / CARD_FRAMES as f32;
        let pitch = self.height + GAP;
        let came = (moves + 1) / 2 + (moves + 1) % 2;
        if moves % 2 == 0 {
            let id = came + self.slots - 1 - place;
            if id < 0 {
                return None;
            }
            return Some(CardPlace { id, top: GAP + (place as f32 - 1.0 + along) * pitch });
        }
        if place >= self.slots {
            return None;
        }
        let other = 2.min(self.slots - 1);
        let reach = other as f32 * pitch * (1.0 - (2.0 * along - 1.0).abs());
        let moved = if place == 0 {
            reach
        } else if place == other {
            -reach
        } else {
            0.0
        };
        Some(CardPlace { id: came + self.slots - 1 - place, top: GAP + place as f32 * pitch + moved })
    }
}

/// The space between two tiles and around a patch's tiles, in points.
pub const GAP: f32 = 1.0;

/// The patches across and down the screen.
pub const PATCH_COLUMNS: i32 = 3;
pub const PATCH_ROWS: i32 = 3;

/// The step, in tiles or words, of everything that jumps.
const JUMP: i64 = 37;

/// The points of one chart of the charts patch, and the width of its line.
pub const CHART_POINTS: i32 = 512;
pub const CHART_STROKE: f32 = 1.0;

/// The frames one move of the cards patch takes.
pub const CARD_FRAMES: i32 = 32;

/// The boxes around the digits of one nest of the deep patch.
pub const DEPTH: i32 = 12;

/// The opacity of the pile patch's tiles' fills, of 255, and the fill of its opaque tile.
pub const PILE_ALPHA: u8 = 0x99;
pub const PILE_OPAQUE_ARGB: u32 = 0xFF000000;

/// The tiles of `tile_width` by `tile_height` points that fill a patch `patch_width` by
/// `patch_height` points.
pub fn tiles(tile_width: f32, tile_height: f32, patch_width: f32, patch_height: f32) -> PatchTiles {
    fn fit(space: f32, tile: f32) -> i32 {
        1.max(((space - GAP) / (tile + GAP)).floor() as i32)
    }
    PatchTiles {
        columns: fit(patch_width, tile_width),
        rows: fit(patch_height, tile_height),
        width: tile_width,
        height: tile_height,
    }
}

/// The height of chart `index` at `point`, as a fraction of the chart.
pub fn chart_value(index: i32, point: i32) -> f32 {
    let at = point as f32 * 0.125;
    0.5 + 0.38 * (at * 0.32 + index as f32 * 0.77).sin() + 0.08 * (at * 1.7 + index as f32).sin()
}

/// The palette index of chart `index`'s line.
pub fn chart_color(index: i32) -> usize {
    (index % PALETTE_ARGB.len() as i32) as usize
}

/// The palette index of tile `index` of the recolor patch in frame `frame`.
pub fn recolor_color(index: i32, frame: i32, turn: InTurn) -> usize {
    ((index + turn.step(index, frame)) % PALETTE_ARGB.len() as i32) as usize
}

/// The header of card `id`.
pub fn card_title(id: i32) -> String {
    format!("Group {:05}", id % 100_000)
}

/// The wall tile at place `tile` of card `id`, whose card holds `tiles` tiles.
pub fn card_tile(id: i32, tile: i32, tiles: i32) -> i32 {
    ((i64::from(id) * i64::from(tiles) + i64::from(tile)) % 100_000) as i32
}

/// The items of the reorder patch for a patch of `tiles` tiles.
pub fn reorder_count(tiles: i32) -> i32 {
    (tiles * 2 / 5).max(1)
}

/// The first of the two neighbouring places whose items change places in frame `frame`.
pub fn reorder_swap(frame: i32, count: i32) -> i32 {
    (i64::from(frame) * JUMP % i64::from((count - 1).max(1))) as i32
}

/// The width of item `item` of the reorder patch, in tiles: 1 to 3.
pub fn reorder_units(item: i32) -> i32 {
    (((item as u32).wrapping_mul(2_654_435_761) >> 22) % 3) as i32 + 1
}

/// The width in points of an item `units` tiles wide.
pub fn reorder_width(units: i32, tile_width: f32) -> f32 {
    units as f32 * tile_width + (units - 1) as f32 * GAP
}

/// The nests or the charts across and down a patch of `columns` by `rows` tiles.
pub fn pairs(columns: i32, rows: i32) -> (i32, i32) {
    ((columns / 2).max(1), (rows / 2).max(1))
}

/// The width or height of an item that covers 2 tiles of `tile` points and the gap between them.
pub fn double(tile: f32) -> f32 {
    2.0 * tile + GAP
}

/// The palette index of box `level` of nest `index`, the outermost box being level 0.
pub fn deep_color(index: i32, level: i32) -> usize {
    ((index + level) % PALETTE_ARGB.len() as i32) as usize
}

/// The number inside nest `index` in frame `frame`, under 1000.
pub fn deep_number(index: i32, frame: i32, turn: InTurn) -> i32 {
    (index * 7 + turn.step(index, frame)) % 1000
}

/// The rows of the scroll patch's list for rows of `columns` tiles.
pub fn scroll_rows(columns: i32) -> i32 {
    100_000 / columns.max(1)
}

/// The first row the scroll patch shows in frame `frame`.
pub fn scroll_first(frame: i32, total: i32, visible: i32) -> i32 {
    frame % ((total - visible).max(0) + 1)
}

/// The words of the paragraph patch for a patch of `tiles` tiles.
pub fn paragraph_words(tiles: i32) -> i32 {
    tiles * 3
}

/// The word of the paragraph that shows the frame in frame `frame`.
pub fn paragraph_changed(frame: i32, words: i32) -> i32 {
    (i64::from(frame) * JUMP % i64::from(words.max(1))) as i32
}

/// The paragraph in frame `frame`: `words` five-digit words, one of which shows the frame.
pub fn paragraph(frame: i32, words: i32) -> String {
    let changed = paragraph_changed(frame, words);
    let mut text = String::with_capacity(words.max(0) as usize * 6);
    for word in 0..words {
        if word > 0 {
            text.push(' ');
        }
        text.push_str(&label(if word == changed { frame % 100_000 } else { word }));
    }
    text
}

/// The tiles across and down the pile patch for a patch of `columns` by `rows` tiles.
pub fn pile_grid(columns: i32, rows: i32) -> (i32, i32) {
    ((columns - 1).max(1), (rows - 1).max(1))
}

/// The tile of the pile patch whose fill is opaque in frame `frame`.
pub fn pile_opaque(frame: i32, tiles: i32) -> i32 {
    (i64::from(frame) * JUMP % i64::from(tiles.max(1))) as i32
}

/// A tile's label: its index in five digits.
pub fn label(index: i32) -> String {
    format!("{index:05}")
}

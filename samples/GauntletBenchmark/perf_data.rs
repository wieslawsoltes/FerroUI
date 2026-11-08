//! The deterministic data of the benchmark: the generator every app of the comparison implements
//! bit for bit, so every app draws identical content. Only what the gauntlet shows is kept.
//!
//! Integer arithmetic follows the 32-bit signed and unsigned arithmetic of the shared C# file:
//! wrapping multiplication where it is `unchecked`, truncating division and a remainder with the
//! sign of the dividend (which Rust's `/` and `%` on `i32` already are). Values the shared file
//! computes in `float` are `f32` here, so the same digits come out on the same frame.

pub const POST_COUNT: usize = 5000;
pub const BAR_COUNT: usize = 24;
pub const CARD_ROWS_PER_CLUSTER: i32 = 5;
pub const AVATAR_COUNT: usize = 8;
pub const AVATAR_SIZE: usize = 64;
pub const SPARK_POINTS: usize = 48;

const WORDS: [&str; 46] = [
    "lorem", "ipsum", "dolor", "sit", "amet", "consectetur", "adipiscing", "elit", "sed", "do",
    "eiusmod", "tempor", "incididunt", "ut", "labore", "et", "dolore", "magna", "aliqua", "enim",
    "ad", "minim", "veniam", "quis", "nostrud", "exercitation", "ullamco", "laboris", "nisi",
    "aliquip", "ex", "ea", "commodo", "consequat", "duis", "aute", "irure", "in", "reprehenderit",
    "voluptate", "velit", "esse", "cillum", "fugiat", "nulla", "pariatur",
];
const FIRST: [&str; 16] = [
    "Ada", "Linus", "Grace", "Alan", "Barbara", "Dennis", "Ken", "Margaret", "Edsger", "Donald",
    "Frances", "John", "Radia", "Tim", "Guido", "Bjarne",
];
const LAST: [&str; 16] = [
    "Lovelace", "Torvalds", "Hopper", "Turing", "Liskov", "Ritchie", "Thompson", "Hamilton",
    "Dijkstra", "Knuth", "Allen", "McCarthy", "Perlman", "Berners", "Rossum", "Stroustrup",
];
const TAGS: [&str; 12] = [
    "#rust", "#kotlin", "#compose", "#android", "#gpu", "#layout", "#text", "#perf", "#wgpu",
    "#skia", "#ui", "#mobile",
];

pub const PALETTE_ARGB: [u32; 8] =
    [0xFFEF4444, 0xFFF97316, 0xFFEAB308, 0xFF22C55E, 0xFF14B8A6, 0xFF3B82F6, 0xFF8B5CF6, 0xFFEC4899];
pub const CHIP_BACKGROUND_ARGB: [u32; 8] =
    [0xFFFEE2E2, 0xFFFFEDD5, 0xFFFEF9C3, 0xFFDCFCE7, 0xFFCCFBF1, 0xFFDBEAFE, 0xFFEDE9FE, 0xFFFCE7F3];
pub const GRADIENT_END_ARGB: [u32; 8] =
    [0xFF7F1D1D, 0xFF7C2D12, 0xFF713F12, 0xFF14532D, 0xFF134E4A, 0xFF1E3A8A, 0xFF4C1D95, 0xFF831843];

/// 32-bit xorshift, as in the other apps.
pub struct Rng {
    state: u32,
}

impl Rng {
    pub fn new(seed: i32) -> Self {
        let mut state = (seed as u32).wrapping_mul(0x9E37_79B9) ^ 0xA5A5_A5A5;
        if state == 0 {
            state = 1;
        }
        Self { state }
    }

    pub fn next(&mut self) -> u32 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.state = x;
        x
    }

    pub fn below(&mut self, bound: i32) -> i32 {
        (self.next() % bound as u32) as i32
    }

    pub fn unit(&mut self) -> f32 {
        (self.next() >> 8) as f32 / 16_777_216.0
    }
}

/// One post of the list: what a card shows.
pub struct Post {
    pub id: i32,
    pub author: String,
    pub color: usize,
    pub title: String,
    pub body: String,
    /// The three tags, each with its palette index.
    pub tags: [(&'static str, usize); 3],
    pub stats: [String; 4],
}

fn sentence(rng: &mut Rng, min: i32, max: i32) -> String {
    let count = min + rng.below(max - min + 1);
    let words: Vec<&str> = (0..count).map(|_| WORDS[rng.below(46) as usize]).collect();
    let text = words.join(" ");
    // The words are lower-case ASCII: the first character is one byte.
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
        None => text,
    }
}

pub fn compact_count(value: i32) -> String {
    if value >= 1000 {
        format!("{}.{}k", value / 1000, value % 1000 / 100)
    } else {
        value.to_string()
    }
}

pub fn make_post(index: i32) -> Post {
    let mut rng = Rng::new(index + 1);
    let first = FIRST[rng.below(16) as usize];
    let last = LAST[rng.below(16) as usize];
    rng.below(59); // minutes, in the handle the gauntlet does not show
    rng.below(1000); // the handle's number
    let color = rng.below(8) as usize;
    let title = sentence(&mut rng, 4, 8);
    let body = sentence(&mut rng, 22, 38) + ".";
    for _ in 0..BAR_COUNT {
        rng.unit();
    }
    let mut tags = [("", 0); 3];
    for tag in &mut tags {
        let which = rng.below(TAGS.len() as i32) as usize;
        *tag = (TAGS[which], which % 8);
    }
    let stats = std::array::from_fn(|_| compact_count(rng.below(20_000)));
    Post { id: index, author: format!("{first} {last}"), color, title, body, tags, stats }
}

pub fn posts() -> Vec<Post> {
    (0..POST_COUNT as i32).map(make_post).collect()
}

/// What one load tier puts on screen: `layers` translucent panels stacked over the list, each
/// with `layer_rows` rows of [`LAYER_COLUMNS`] cells.
#[derive(Clone, Copy)]
pub struct Tier {
    pub columns: usize,
    pub scale: f32,
    pub tickers: usize,
    pub depth: i32,
    pub layers: i32,
    pub layer_rows: i32,
}

const fn tier(columns: usize, scale: f32, tickers: usize, depth: i32, layers: i32, layer_rows: i32) -> Tier {
    Tier { columns, scale, tickers, depth, layers, layer_rows }
}

const TIERS: [Tier; 16] = [
    tier(1, 1.0, 8, 6, 1, 2), tier(2, 0.85, 12, 8, 1, 2), tier(2, 0.7, 16, 10, 2, 3), tier(3, 0.6, 20, 12, 2, 3),
    tier(3, 0.5, 28, 14, 2, 4), tier(4, 0.45, 36, 16, 3, 4), tier(4, 0.4, 44, 20, 3, 5), tier(5, 0.35, 56, 24, 3, 5),
    tier(6, 0.3, 72, 28, 4, 6), tier(7, 0.27, 96, 32, 4, 6), tier(8, 0.25, 120, 40, 4, 7), tier(10, 0.2, 160, 48, 5, 7),
    tier(12, 0.18, 200, 56, 5, 8), tier(14, 0.16, 240, 64, 5, 8), tier(16, 0.14, 300, 72, 6, 9), tier(20, 0.12, 400, 80, 6, 10),
];

/// Tier 1 to 16; anything else is clamped into that range.
pub fn gauntlet_tier(tier: i32) -> Tier {
    TIERS[(tier.clamp(1, TIERS.len() as i32) - 1) as usize]
}

#[derive(Clone)]
pub struct Ticker {
    pub symbol: String,
    pub base_cents: i32,
    pub swing_cents: i32,
    pub step: i32,
    pub phase: i32,
}

pub fn tickers(count: usize) -> Vec<Ticker> {
    let mut rng = Rng::new(31_337);
    (0..count)
        .map(|_| {
            let length = 3 + rng.below(2);
            let symbol: String = (0..length).map(|_| char::from(b'A' + rng.below(26) as u8)).collect();
            let base_cents = 1_000 + rng.below(99_000);
            let swing_cents = 50 + rng.below(950);
            let step = 1 + rng.below(9);
            let phase = rng.below(2_000);
            Ticker { symbol, base_cents, swing_cents, step, phase }
        })
        .collect()
}

/// A triangle wave over the period: 0 at the ends, half the period in the middle.
fn triangle(value: i32, period: i32) -> i32 {
    let position = (value % period + period) % period;
    period / 2 - (position - period / 2).abs()
}

/// The quote's price on the frame, in cents.
pub fn ticker_cents(ticker: &Ticker, frame: i32) -> i32 {
    let wave = triangle(frame * ticker.step + ticker.phase, 2_000);
    ticker.base_cents + ticker.swing_cents * (wave - 500) / 500
}

fn two_decimals(hundredths: i32) -> String {
    let value = hundredths.abs();
    let fraction = value % 100;
    format!("{}.{}{}", value / 100, if fraction < 10 { "0" } else { "" }, fraction)
}

/// 1234 -> 12.34.
pub fn cents_text(cents: i32) -> String {
    (if cents < 0 { "-" } else { "" }).to_string() + &two_decimals(cents)
}

/// The change from the base price as a signed percent: +1.25%.
pub fn change_text(ticker: &Ticker, cents: i32) -> String {
    let basis_points = (cents - ticker.base_cents) * 10_000 / ticker.base_cents;
    (if basis_points < 0 { "-" } else { "+" }).to_string() + &two_decimals(basis_points) + "%"
}

/// A card's progress on the frame, in thousandths.
pub fn progress_permille(card: i32, frame: i32) -> i32 {
    (frame * 3 + card * 37) % 1_000
}

/// The tilt of a card's badge on the frame, in degrees: -5 to 5.
pub fn badge_degrees(card: i32, frame: i32) -> f32 {
    triangle(frame * 2 + card * 30, 40) as f32 * 0.5 - 5.0
}

/// Cells in each row of a stacked panel's grid.
pub const LAYER_COLUMNS: i32 = 8;

/// The left edge of the stacked panel on the frame before its tilt, in dp from the list's left edge.
pub fn layer_x(layer: i32, frame: i32) -> f32 {
    (48 + 28 * layer + triangle(frame * 2 + layer * 37, 160) - 40) as f32
}

/// The top edge of the stacked panel on the frame before its tilt, in dp from the list's top edge.
pub fn layer_y(layer: i32, frame: i32) -> f32 {
    (48 + 72 * layer + triangle(frame + layer * 53, 120) - 30) as f32
}

/// The tilt of the stacked panel about its center on the frame, in degrees: -6 to 6.
pub fn layer_degrees(layer: i32, frame: i32) -> f32 {
    triangle(frame + layer * 29, 48) as f32 * 0.5 - 6.0
}

/// The palette index of a cell in a stacked panel.
pub fn layer_cell_color(layer: i32, cell: i32) -> usize {
    ((layer * 3 + cell) % PALETTE_ARGB.len() as i32) as usize
}

/// The content's share of the screen width on the frame: 0.92 to 1.
pub fn width_fraction(frame: i32) -> f32 {
    0.92 + 0.08 * triangle(frame * 3, 200) as f32 / 100.0
}

/// The sparkline's height at the point, as a fraction of the chart, on the frame.
pub fn spark_value(card: i32, point: i32, frame: i32) -> f32 {
    let phase = (frame as f32 + card as f32 * 7.0) * 0.11;
    0.5 + 0.38 * (point as f32 * 0.32 + phase).sin() + 0.08 * (point as f32 * 1.7 + card as f32).sin()
}

const AVATAR_COLORS: [[u8; 6]; 8] = [
    [0xEF, 0x44, 0x44, 0x7F, 0x1D, 0x1D], [0xF9, 0x73, 0x16, 0x7C, 0x2D, 0x12],
    [0xEA, 0xB3, 0x08, 0x71, 0x3F, 0x12], [0x22, 0xC5, 0x5E, 0x14, 0x53, 0x2D],
    [0x14, 0xB8, 0xA6, 0x13, 0x4E, 0x4A], [0x3B, 0x82, 0xF6, 0x1E, 0x3A, 0x8A],
    [0x8B, 0x5C, 0xF6, 0x4C, 0x1D, 0x95], [0xEC, 0x48, 0x99, 0x83, 0x18, 0x43],
];

/// Avatar pixels as RGBA bytes: a radial gradient crossed by diagonal stripes.
pub fn avatar_rgba(index: usize) -> Vec<u8> {
    let colors = &AVATAR_COLORS[index % AVATAR_COUNT];
    let size = AVATAR_SIZE as i32;
    let mut pixels = Vec::with_capacity(AVATAR_SIZE * AVATAR_SIZE * 4);
    for y in 0..size {
        for x in 0..size {
            let dx = x - size / 2;
            let dy = y - size * 3 / 8;
            let t = 255.min((dx * dx + dy * dy) * 255 / (40 * 40));
            let stripe = (x + y + index as i32 * 3) / 6 % 2 == 0;
            for channel in 0..3 {
                let mut value = (i32::from(colors[channel]) * (255 - t) + i32::from(colors[channel + 3]) * t) / 255;
                if stripe {
                    value += (255 - value) / 6;
                }
                pixels.push(value as u8);
            }
            pixels.push(0xFF);
        }
    }
    pixels
}

//! The images the page transitions sample: two pages and the pattern of a dissolve.
//!
//! An effect image is a handle of a bitmap (`EffectImage`), made by the application and uploaded
//! once by the backend. The ControlCatalog gives the transitions photographs of its assets; the
//! gallery application captures pages of its own: each is laid out and drawn once into a
//! `RenderTargetBitmap` (section 3.6 of `docs/porting/custom-effects.md`: capturing a page is
//! `RenderTargetBitmap`, made by the application, not per frame), as Effector's compiz sample
//! captures the page it leaves and the page it comes to.

use crate::tiles::{brush, column, row, text};
use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::media::effects::EffectImage;
use ferroui_base::media::imaging::RenderTargetBitmap;
use ferroui_base::media::{Color, FontWeight};
use ferroui_base::{CornerRadius, PixelSize, Rect, Ref, Size, Thickness};
use ferroui_controls::primitives::UniformGrid;
use ferroui_controls::Border;

/// The width of a captured page, in pixels.
pub const PAGE_WIDTH: i32 = 480;

/// The height of a captured page, in pixels.
pub const PAGE_HEIGHT: i32 = 300;

/// The columns and rows of the cells of the pattern.
const PATTERN_CELLS: (i32, i32) = (16, 10);

/// The images of the page transitions: the page a transition leaves, the page it comes to, and
/// the pattern of the dissolve (its luminance), which is also the mask of the mask transition.
/// The empty images (the default) are drawn transparent.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TransitionImages {
    /// The first page.
    pub first_page: EffectImage,
    /// The second page.
    pub second_page: EffectImage,
    /// The pattern of the dissolve and the mask of the mask transition.
    pub pattern: EffectImage,
}

impl TransitionImages {
    /// Two pages and a pattern drawn by the render interface of the platform into bitmaps of
    /// [`PAGE_WIDTH`] by [`PAGE_HEIGHT`] pixels.
    pub fn render() -> Self {
        Self {
            first_page: capture(&page(
                "Home",
                "The page the transition leaves.",
                Color::from_rgb(0x1F, 0x4E, 0x79),
                [(0x5B, 0xC0, 0xEB), (0xFD, 0xE7, 0x4C), (0x9B, 0xC5, 0x3D)],
            )),
            second_page: capture(&page(
                "Work",
                "The page the transition comes to.",
                Color::from_rgb(0x7A, 0x28, 0x4E),
                [(0xF5, 0x9E, 0x0B), (0xEC, 0x48, 0x99), (0x8B, 0x5C, 0xF6)],
            )),
            pattern: capture(&pattern()),
        }
    }
}

/// Lays `control` out at the size of a page and draws it into a bitmap, whose handle it returns.
fn capture(control: &Ref<Border>) -> EffectImage {
    let size = Size::new(f64::from(PAGE_WIDTH), f64::from(PAGE_HEIGHT));
    control.measure(size);
    control.arrange(Rect::from_size(size));
    let bitmap = RenderTargetBitmap::new(PixelSize::new(PAGE_WIDTH, PAGE_HEIGHT));
    bitmap.render(&control.clone().upcast());
    // The handle takes a count of the platform bitmap of its own.
    let image = EffectImage::new(&bitmap);
    bitmap.dispose();
    image
}

/// A page: a title and a line on a coloured background, and three cards.
fn page(title: &str, line: &str, background: Color, cards: [(u8, u8, u8); 3]) -> Ref<Border> {
    let heading = text(title, 40.0, FontWeight::Bold);
    heading.set_foreground(Some(brush(Color::from_rgb(0xFF, 0xFF, 0xFF))));
    let about = text(line, 16.0, FontWeight::Normal);
    about.set_foreground(Some(brush(Color::from_rgb(0xF0, 0xF0, 0xF0))));
    let row_of_cards = row(16.0);
    for (r, g, b) in cards {
        let card = Border::new();
        card.set_width(120.0);
        card.set_height(96.0);
        card.set_corner_radius(CornerRadius::uniform(12.0));
        card.set_background(Some(brush(Color::from_rgb(r, g, b))));
        row_of_cards.children().add(card);
    }
    let content = column(16.0);
    content.children().add(heading);
    content.children().add(about);
    content.children().add(row_of_cards);
    let page = Border::new();
    page.set_width(f64::from(PAGE_WIDTH));
    page.set_height(f64::from(PAGE_HEIGHT));
    page.set_padding(Thickness::uniform(24.0));
    page.set_background(Some(brush(background)));
    page.set_horizontal_alignment(HorizontalAlignment::Left);
    page.set_child(content);
    page
}

/// The grey of the cell `index` of the pattern: a hash of the index, so that the cells are
/// crossed in no visible order.
fn cell_grey(index: i32) -> u8 {
    let mut x = (index as u32).wrapping_mul(0x9E37_79B9) ^ 0x85EB_CA6B;
    x ^= x >> 15;
    x = x.wrapping_mul(0x2C1B_3C6D);
    x ^= x >> 12;
    (x & 0xFF) as u8
}

/// The pattern: cells of [`cell_grey`].
fn pattern() -> Ref<Border> {
    let (columns, rows) = PATTERN_CELLS;
    let grid = UniformGrid::new();
    grid.set_columns(columns);
    grid.set_rows(rows);
    for index in 0..columns * rows {
        let grey = cell_grey(index);
        let cell = Border::new();
        cell.set_background(Some(brush(Color::from_rgb(grey, grey, grey))));
        grid.children().add(cell);
    }
    let pattern = Border::new();
    pattern.set_width(f64::from(PAGE_WIDTH));
    pattern.set_height(f64::from(PAGE_HEIGHT));
    pattern.set_child(grid);
    pattern
}

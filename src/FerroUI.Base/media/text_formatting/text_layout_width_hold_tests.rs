//! `TextLayout::width_hold` (an addition, desktop-performance.md section 9.12): at every width a
//! layout claims to hold, a new layout of the same text makes the same lines.

use crate::media::text_formatting::testing::TextTestScope;
use crate::media::text_formatting::{TextLayout, TextLayoutOptions};
use crate::media::{Brushes, TextTrimming, TextWrapping, Typeface};

const TEXTS: [&str; 8] = [
    "Lorem ipsum dolor sit amet consectetur adipiscing elit sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.",
    "Short words a b c d e f g h i j k l m n o p q r s t u v w x y z and some longer reprehenderit voluptate.",
    "Hyphenated-words break after-the-hyphen, as do em\u{2014}dashes and slashes/like/this in URLs and paths.",
    "Two  spaces   between   some    words and a trailing space ",
    "A line\nwith a required break\nand more text after it that wraps again at the narrow widths.",
    "Supercalifragilisticexpialidocious is a long word next to short ones a b c.",
    "Numbers 1,234.56 and +2.31% and $9,876.54 stay together; (parentheses) and \"quotes\" too.",
    "word",
];

fn layout(text: &str, width: f64) -> TextLayout {
    TextLayout::new(
        text,
        Typeface::default_typeface(),
        TextLayoutOptions {
            font_size: 12.0,
            foreground: Some(Brushes::black()),
            max_width: width,
            text_trimming: Some(<dyn TextTrimming>::none()),
            text_wrapping: TextWrapping::Wrap,
            ..TextLayoutOptions::default()
        },
    )
}

fn lines(layout: &TextLayout) -> Vec<(i32, i32)> {
    layout.text_lines().iter().map(|line| (line.first_text_source_index(), line.length())).collect()
}

#[test]
fn a_layout_makes_the_same_lines_at_every_width_it_holds() {
    let _scope = TextTestScope::new();
    let mut held_widths = 0;
    for text in TEXTS {
        for start in (20..=400).step_by(13) {
            let start = f64::from(start);
            let first = layout(text, start);
            let expected = lines(&first);
            let Some((low, high)) = first.width_hold() else { continue };
            // The hold keeps a margin inside its exact bounds: a line exactly as wide as the
            // width puts the width of the layout itself on the margin.
            assert!(low <= start + 0.02 && start < high + 0.02, "{text:?} at {start}: the hold {low}..{high} misses its own width");
            let mut width = (low - 2.0).max(1.0);
            while width < high.min(start + 400.0) + 2.0 {
                if low <= width && width < high {
                    held_widths += 1;
                    let fresh = layout(text, width);
                    assert_eq!(
                        expected,
                        lines(&fresh),
                        "{text:?}: created at {start}, held over {low}..{high}, other lines at {width}"
                    );
                }
                width += 0.25;
            }
        }
    }
    assert!(held_widths > 10_000, "only {held_widths} widths were held");
}

#[test]
fn layouts_the_hold_does_not_describe_have_none() {
    let _scope = TextTestScope::new();
    // A word wider than the width is broken inside: not held.
    assert_eq!(None, layout("Supercalifragilisticexpialidocious", 30.0).width_hold());
    // Text that was trimmed, or cut at the line limit, is not held.
    let limited = |max_lines: i32, trimming: std::rc::Rc<dyn TextTrimming>| {
        TextLayout::new(
            TEXTS[0],
            Typeface::default_typeface(),
            TextLayoutOptions {
                font_size: 12.0,
                max_width: 100.0,
                max_lines,
                text_trimming: Some(trimming),
                text_wrapping: TextWrapping::Wrap,
                ..TextLayoutOptions::default()
            },
        )
    };
    let trimmed = limited(2, <dyn TextTrimming>::character_ellipsis());
    assert!(trimmed.has_collapsed_lines() && trimmed.is_truncated_by_max_lines());
    assert_eq!(None, trimmed.width_hold());
    let cut = limited(2, <dyn TextTrimming>::none());
    assert!(!cut.has_collapsed_lines() && cut.is_truncated_by_max_lines());
    assert_eq!(None, cut.width_hold());
    // Trimming and a line limit that changed nothing leave the hold of the plain text
    // (section 9.27).
    let untouched = limited(100, <dyn TextTrimming>::character_ellipsis());
    assert!(!untouched.has_collapsed_lines() && !untouched.is_truncated_by_max_lines());
    assert_eq!(layout(TEXTS[0], 100.0).width_hold(), untouched.width_hold());
    assert!(untouched.width_hold().is_some());
    // Text on one line holds from its width upwards.
    let one_line = layout("word", 300.0);
    let (low, high) = one_line.width_hold().expect("a hold");
    assert!(low < 300.0 && high == f64::INFINITY);
}

/// Section 9.27: with trimming and a line limit, a layout that claims a hold left nothing out,
/// and at every width it holds a new layout has the same lines, none trimmed and none left out.
#[test]
fn a_trimmed_limited_layout_makes_the_same_lines_at_every_width_it_holds() {
    let _scope = TextTestScope::new();
    let limited = |text: &str, width: f64, max_lines: i32| {
        TextLayout::new(
            text,
            Typeface::default_typeface(),
            TextLayoutOptions {
                font_size: 12.0,
                foreground: Some(Brushes::black()),
                max_width: width,
                max_lines,
                text_trimming: Some(<dyn TextTrimming>::character_ellipsis()),
                text_wrapping: TextWrapping::Wrap,
                ..TextLayoutOptions::default()
            },
        )
    };
    let mut held_widths = 0;
    let mut refused = 0;
    for text in TEXTS {
        for max_lines in [1, 2, 3, 5, 40] {
            for start in (20..=400).step_by(19) {
                let start = f64::from(start);
                let first = limited(text, start, max_lines);
                let expected = lines(&first);
                let Some((low, high)) = first.width_hold() else {
                    refused += 1;
                    continue;
                };
                assert!(!first.has_collapsed_lines() && !first.is_truncated_by_max_lines());
                // The plain text has the same lines: the limit and the trimming changed nothing.
                assert_eq!(expected, lines(&layout(text, start)));
                let mut width = (low - 2.0).max(1.0);
                while width < high.min(start + 400.0) + 2.0 {
                    if low <= width && width < high {
                        held_widths += 1;
                        let fresh = limited(text, width, max_lines);
                        assert_eq!(
                            expected,
                            lines(&fresh),
                            "{text:?} limited to {max_lines}: created at {start}, held over {low}..{high}, other lines at {width}"
                        );
                        assert!(!fresh.has_collapsed_lines() && !fresh.is_truncated_by_max_lines());
                    }
                    width += 0.5;
                }
            }
        }
    }
    assert!(held_widths > 10_000, "only {held_widths} widths were held");
    assert!(refused > 100, "only {refused} layouts were refused");
}

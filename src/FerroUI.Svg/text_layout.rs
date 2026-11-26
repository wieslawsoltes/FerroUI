//! What SVG text needs before any font is involved: its white space, its font properties and the
//! place of each run. The measuring is passed in, so that these rules are tested without fonts.

use crate::numbers::length;

/// The size of text that does not say (`medium` in CSS).
pub(crate) const DEFAULT_FONT_SIZE: f64 = 16.0;

/// `text-anchor`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum TextAnchor {
    #[default]
    Start,
    Middle,
    End,
}

pub(crate) fn parse_anchor(text: &str) -> Option<TextAnchor> {
    match text.trim() {
        "start" => Some(TextAnchor::Start),
        "middle" => Some(TextAnchor::Middle),
        "end" => Some(TextAnchor::End),
        _ => None,
    }
}

/// A `font-size` against the size of the parent. `None` for a value that is not valid.
pub(crate) fn parse_font_size(text: &str, parent: f64) -> Option<f64> {
    let text = text.trim();
    let size = match text {
        "xx-small" => 9.0,
        "x-small" => 10.0,
        "small" => 13.0,
        "medium" => DEFAULT_FONT_SIZE,
        "large" => 18.0,
        "x-large" => 24.0,
        "xx-large" => 32.0,
        "smaller" => parent / 1.2,
        "larger" => parent * 1.2,
        _ => match text.strip_suffix("em").and_then(|number| number.trim().parse::<f64>().ok()) {
            Some(factor) => factor * parent,
            None => length(text, parent)?,
        },
    };
    (size.is_finite() && size > 0.0).then_some(size)
}

/// A `font-weight` against the weight of the parent, by the table of CSS for the relative ones.
pub(crate) fn parse_font_weight(text: &str, parent: i32) -> Option<i32> {
    match text.trim() {
        "normal" => Some(400),
        "bold" => Some(700),
        "bolder" => Some(match parent {
            ..=349 => 400,
            350..=549 => 700,
            _ => 900,
        }),
        "lighter" => Some(match parent {
            ..=549 => 100,
            550..=749 => 400,
            _ => 700,
        }),
        number => number.parse::<i32>().ok().filter(|weight| (1..=1000).contains(weight)),
    }
}

/// The families of a `font-family`, without quotes: a generic family is left to the default font
/// of the platform, and a name is looked up among the fonts of the system. `None` when only
/// generic families are named.
pub(crate) fn font_family_names(text: &str) -> Option<String> {
    let names: Vec<&str> = text
        .split(',')
        .map(|name| name.trim().trim_matches(|c| c == '"' || c == '\''))
        .filter(|name| {
            !name.is_empty()
                && !matches!(
                    name.to_ascii_lowercase().as_str(),
                    "serif" | "sans-serif" | "monospace" | "cursive" | "fantasy" | "system-ui"
                )
        })
        .collect();
    (!names.is_empty()).then(|| names.join(", "))
}

/// Text under `xml:space="default"`: line breaks and tabs are spaces, and runs of spaces are one.
/// `after_space` says that what comes before ended with a space, or that nothing does, and is
/// updated: a space is not repeated across runs and the text does not start with one.
pub(crate) fn collapse_white_space(text: &str, after_space: &mut bool) -> String {
    let mut result = String::with_capacity(text.len());
    for c in text.chars() {
        if matches!(c, ' ' | '\t' | '\n' | '\r') {
            if !*after_space {
                result.push(' ');
                *after_space = true;
            }
        } else {
            result.push(c);
            *after_space = false;
        }
    }
    result
}

/// Where a run asks to be.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct RunPosition {
    pub(crate) x: Option<f64>,
    pub(crate) y: Option<f64>,
    pub(crate) dx: f64,
    pub(crate) dy: f64,
    pub(crate) anchor: TextAnchor,
}

/// The start of each run on its baseline. A run with an `x` or a `y` starts a new chunk, and
/// `text-anchor` moves a whole chunk by its width, as the first run of the chunk asks.
/// `advance(index)` is the width of a run.
pub(crate) fn place_runs(runs: &[RunPosition], mut advance: impl FnMut(usize) -> f64) -> Vec<(f64, f64)> {
    let widths: Vec<f64> = (0..runs.len()).map(&mut advance).collect();
    let mut places = vec![(0.0, 0.0); runs.len()];
    let (mut pen_x, mut pen_y) = (0.0, 0.0);
    let mut start = 0;
    while start < runs.len() {
        // The chunk runs up to the next run placed absolutely.
        let end = (start + 1..runs.len()).find(|index| runs[*index].x.is_some() || runs[*index].y.is_some()).unwrap_or(runs.len());
        pen_x = runs[start].x.unwrap_or(pen_x);
        pen_y = runs[start].y.unwrap_or(pen_y);
        let width: f64 = (start..end).map(|index| widths[index] + runs[index].dx).sum();
        let shift = match runs[start].anchor {
            TextAnchor::Start => 0.0,
            TextAnchor::Middle => -width / 2.0,
            TextAnchor::End => -width,
        };
        pen_x += shift;
        for index in start..end {
            pen_x += runs[index].dx;
            pen_y += runs[index].dy;
            places[index] = (pen_x, pen_y);
            pen_x += widths[index];
        }
        start = end;
    }
    places
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn white_space_collapses_across_runs() {
        let mut after_space = true;
        assert_eq!(collapse_white_space("\n  one\t two  ", &mut after_space), "one two ");
        assert!(after_space);
        assert_eq!(collapse_white_space("  three", &mut after_space), "three");
        assert!(!after_space);
    }

    #[test]
    fn font_properties_follow_css() {
        assert_eq!(parse_font_size("2em", 10.0), Some(20.0));
        assert_eq!(parse_font_size("150%", 10.0), Some(15.0));
        assert_eq!(parse_font_size("12", 10.0), Some(12.0));
        assert_eq!(parse_font_size("medium", 10.0), Some(16.0));
        assert_eq!(parse_font_size("-1", 10.0), None);
        assert_eq!(parse_font_weight("bold", 400), Some(700));
        assert_eq!(parse_font_weight("bolder", 700), Some(900));
        assert_eq!(parse_font_weight("lighter", 700), Some(400));
        assert_eq!(parse_font_weight("650", 400), Some(650));
        assert_eq!(parse_font_weight("0", 400), None);
        assert_eq!(font_family_names(r#""Segoe UI", Arial, sans-serif"#).as_deref(), Some("Segoe UI, Arial"));
        assert_eq!(font_family_names("sans-serif"), None);
    }

    fn at(x: f64, y: f64, anchor: TextAnchor) -> RunPosition {
        RunPosition { x: Some(x), y: Some(y), anchor, ..RunPosition::default() }
    }

    #[test]
    fn runs_follow_one_another_with_their_offsets() {
        let runs = [at(10.0, 20.0, TextAnchor::Start), RunPosition { dx: 2.0, dy: 3.0, ..RunPosition::default() }];
        assert_eq!(place_runs(&runs, |_| 5.0), [(10.0, 20.0), (17.0, 23.0)]);
    }

    #[test]
    fn the_anchor_moves_the_whole_chunk() {
        let runs = [at(100.0, 0.0, TextAnchor::Middle), RunPosition::default()];
        assert_eq!(place_runs(&runs, |_| 10.0), [(90.0, 0.0), (100.0, 0.0)]);
        let runs = [at(100.0, 0.0, TextAnchor::End), RunPosition::default()];
        assert_eq!(place_runs(&runs, |_| 10.0), [(80.0, 0.0), (90.0, 0.0)]);
    }

    #[test]
    fn a_run_placed_absolutely_starts_a_new_chunk() {
        let runs = [at(0.0, 0.0, TextAnchor::End), at(50.0, 10.0, TextAnchor::Start)];
        assert_eq!(place_runs(&runs, |_| 10.0), [(-10.0, 0.0), (50.0, 10.0)]);
    }
}

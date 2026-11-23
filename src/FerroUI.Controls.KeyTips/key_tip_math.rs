//! What a typed key matches, and where a Key Tip goes.

use ferroui_base::{Point, Rect, Size};

/// Where a Key Tip is placed relative to its element.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
#[repr(i32)]
pub enum KeyTipPlacementMode {
    /// Below the element, or above it when there is no room.
    #[default]
    Auto = 0,
    Bottom = 1,
    Top = 2,
    Left = 3,
    Right = 4,
    Center = 5,
    /// The key works, but no tip is shown.
    Hidden = 6,
}

/// A key as it is compared: without surrounding space, and without case.
pub(crate) fn normalize_key(key: &str) -> String {
    key.trim().to_uppercase()
}

/// What the keys typed so far match.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum KeyMatch {
    /// No key starts with them.
    None,
    /// Some keys start with them, and none is complete.
    Partial,
    /// The key at this position is complete.
    Exact(usize),
}

/// Compares what was typed with normalized keys. The first of equal keys wins.
pub(crate) fn match_keys<S: AsRef<str>>(keys: &[S], typed: &str) -> KeyMatch {
    if typed.is_empty() {
        return KeyMatch::None;
    }
    if let Some(index) = keys.iter().position(|key| key.as_ref() == typed) {
        return KeyMatch::Exact(index);
    }
    if keys.iter().any(|key| key.as_ref().starts_with(typed)) {
        KeyMatch::Partial
    } else {
        KeyMatch::None
    }
}

/// The top left of a tip of size `tip` for an element at `target`, moved by the offsets and kept
/// inside `bounds`.
pub(crate) fn place_tip(
    target: Rect,
    tip: Size,
    mode: KeyTipPlacementMode,
    horizontal_offset: f64,
    vertical_offset: f64,
    bounds: Size,
) -> Point {
    let center_x = target.x + (target.width - tip.width) / 2.0;
    let center_y = target.y + (target.height - tip.height) / 2.0;
    let below = target.y + target.height;
    let above = target.y - tip.height;
    let (x, y) = match mode {
        KeyTipPlacementMode::Auto | KeyTipPlacementMode::Hidden => {
            let fits_below = below + tip.height <= bounds.height;
            (
                center_x,
                if fits_below || above < 0.0 {
                    below
                } else {
                    above
                },
            )
        }
        KeyTipPlacementMode::Bottom => (center_x, below),
        KeyTipPlacementMode::Top => (center_x, above),
        KeyTipPlacementMode::Left => (target.x - tip.width, center_y),
        KeyTipPlacementMode::Right => (target.x + target.width, center_y),
        KeyTipPlacementMode::Center => (center_x, center_y),
    };
    let limit = |value: f64, size: f64, room: f64| value.min(room - size).max(0.0);
    Point::new(
        limit(x + horizontal_offset, tip.width, bounds.width),
        limit(y + vertical_offset, tip.height, bounds.height),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_are_compared_without_case_or_space() {
        assert_eq!(normalize_key(" fi "), "FI");
        assert_eq!(normalize_key("ß"), "SS");
    }

    #[test]
    fn typed_keys_match_exactly_or_as_a_start() {
        let keys = ["F", "ED", "EX"];
        assert_eq!(match_keys(&keys, "F"), KeyMatch::Exact(0));
        assert_eq!(match_keys(&keys, "E"), KeyMatch::Partial);
        assert_eq!(match_keys(&keys, "EX"), KeyMatch::Exact(2));
        assert_eq!(match_keys(&keys, "Q"), KeyMatch::None);
        assert_eq!(match_keys(&keys, ""), KeyMatch::None);
    }

    #[test]
    fn the_first_of_equal_keys_wins() {
        assert_eq!(match_keys(&["A", "B", "B"], "B"), KeyMatch::Exact(1));
    }

    const BOUNDS: Size = Size::new(400.0, 300.0);
    const TIP: Size = Size::new(20.0, 16.0);

    fn at(target: Rect, mode: KeyTipPlacementMode) -> (f64, f64) {
        let point = place_tip(target, TIP, mode, 0.0, 0.0, BOUNDS);
        (point.x, point.y)
    }

    #[test]
    fn each_mode_places_the_tip_on_its_side() {
        let target = Rect::new(100.0, 100.0, 60.0, 30.0);
        assert_eq!(at(target, KeyTipPlacementMode::Bottom), (120.0, 130.0));
        assert_eq!(at(target, KeyTipPlacementMode::Top), (120.0, 84.0));
        assert_eq!(at(target, KeyTipPlacementMode::Left), (80.0, 107.0));
        assert_eq!(at(target, KeyTipPlacementMode::Right), (160.0, 107.0));
        assert_eq!(at(target, KeyTipPlacementMode::Center), (120.0, 107.0));
        assert_eq!(at(target, KeyTipPlacementMode::Auto), (120.0, 130.0));
    }

    #[test]
    fn auto_goes_above_when_there_is_no_room_below() {
        let target = Rect::new(100.0, 280.0, 60.0, 20.0);
        assert_eq!(at(target, KeyTipPlacementMode::Auto), (120.0, 264.0));
    }

    #[test]
    fn a_tip_stays_inside_the_window_with_its_offsets() {
        let corner = Rect::new(390.0, 0.0, 10.0, 10.0);
        assert_eq!(at(corner, KeyTipPlacementMode::Top), (380.0, 0.0));
        let target = Rect::new(100.0, 100.0, 60.0, 30.0);
        let moved = place_tip(target, TIP, KeyTipPlacementMode::Bottom, 5.0, -3.0, BOUNDS);
        assert_eq!((moved.x, moved.y), (125.0, 127.0));
    }
}

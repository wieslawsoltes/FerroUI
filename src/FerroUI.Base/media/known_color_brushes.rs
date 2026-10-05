//! The brush lookups of the known colors.

use crate::media::immutable::ImmutableSolidColorBrush;
use crate::media::{KnownColor, KnownColors};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

thread_local! {
    static KNOWN_BRUSHES: RefCell<HashMap<u32, Rc<ImmutableSolidColorBrush>>> = RefCell::new(HashMap::new());
}

impl KnownColor {
    /// Converts the known color to a solid color brush.
    ///
    /// Brushes created from known colors are cached and reused for
    /// efficiency.
    pub fn to_brush(self) -> Rc<ImmutableSolidColorBrush> {
        KNOWN_BRUSHES.with(|brushes| {
            brushes
                .borrow_mut()
                .entry(self.value())
                .or_insert_with(|| Rc::new(ImmutableSolidColorBrush::new(self.to_color())))
                .clone()
        })
    }
}

impl KnownColors {
    /// Gets the cached brush of the known color with the given name (ASCII
    /// case-insensitive), or `None` when there is no such color.
    pub fn get_known_brush(s: &str) -> Option<Rc<ImmutableSolidColorBrush>> {
        let color = KnownColors::get_known_color(s);
        if color != KnownColor::NONE {
            Some(color.to_brush())
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::Colors;

    #[test]
    fn known_brushes_are_cached() {
        let a = KnownColor::RED.to_brush();
        let b = KnownColors::get_known_brush("red").unwrap();
        assert!(Rc::ptr_eq(&a, &b));
        assert_eq!(Colors::RED, a.color());
        assert!(KnownColors::get_known_brush("NotAColor").is_none());
        // Aliases share one brush.
        assert!(Rc::ptr_eq(&KnownColor::AQUA.to_brush(), &KnownColor::CYAN.to_brush()));
    }
}

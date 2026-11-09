//! Native context value equality, with explicit shared currency identity.
use super::KeyComputeContext;
use std::rc::Rc;
impl PartialEq for KeyComputeContext {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
            || self.mode() == other.mode()
                && self.shift_state() == other.shift_state()
                && self.variation() == other.variation()
                && self.direction() == other.direction()
                && self.is_char_half_width() == other.is_char_half_width()
                && self.is_kana_kata() == other.is_kana_kata()
                && self.culture() == other.culture()
                && self.enter_action() == other.enter_action()
                && match (self.currency_set(), other.currency_set()) {
                    (None, None) => true,
                    (Some(a), Some(b)) => Rc::ptr_eq(&a, &b),
                    _ => false,
                }
                && self.has_multiple_subtypes() == other.has_multiple_subtypes()
                && self.utility_key_action() == other.utility_key_action()
                && self.space_bar_mode() == other.space_bar_mode()
                && self.space_bar_label() == other.space_bar_label()
                && self.hint_mode() == other.hint_mode()
                && self.show_symbol_hints() == other.show_symbol_hints()
                && self.show_number_hints() == other.show_number_hints()
    }
}

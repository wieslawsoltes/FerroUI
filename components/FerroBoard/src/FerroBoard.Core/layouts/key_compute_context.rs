//! Shared state of the source sealed context record from Layouts/ComputedKeyboard.cs.
//! Field access releases its borrow before dispatch; copies retain reference fields but own values.
use super::{CurrencySet, IKeyEvaluator};
use crate::{
    editor::EnterAction,
    keys::*,
    settings::*,
    text::{TextCulture, Utf16Text},
};
use std::{cell::RefCell, rc::Rc};
#[derive(Clone)]
pub struct KeyComputeContextValues {
    pub mode: KeyboardMode,
    pub shift_state: ShiftState,
    pub variation: KeyVariation,
    pub direction: LayoutDirection,
    pub is_char_half_width: bool,
    pub is_kana_kata: bool,
    pub culture: Option<TextCulture>,
    pub enter_action: EnterAction,
    pub currency_set: Option<Rc<CurrencySet>>,
    pub has_multiple_subtypes: bool,
    pub utility_key_action: UtilityKeyAction,
    pub space_bar_mode: SpaceBarMode,
    pub space_bar_label: Option<Utf16Text>,
    pub hint_mode: KeyHintMode,
    pub show_symbol_hints: bool,
    pub show_number_hints: bool,
}
impl Default for KeyComputeContextValues {
    fn default() -> Self {
        Self {
            mode: KeyboardMode::Characters,
            shift_state: ShiftState::Unshifted,
            variation: KeyVariation::Normal,
            direction: LayoutDirection::Ltr,
            is_char_half_width: false,
            is_kana_kata: false,
            culture: Some(TextCulture::invariant()),
            enter_action: EnterAction::Done,
            currency_set: None,
            has_multiple_subtypes: false,
            utility_key_action: UtilityKeyAction::Dynamic,
            space_bar_mode: SpaceBarMode::CurrentLanguage,
            space_bar_label: Some(Utf16Text::default()),
            hint_mode: KeyHintMode::AccentPriority,
            show_symbol_hints: true,
            show_number_hints: true,
        }
    }
}

pub struct KeyComputeContext {
    values: RefCell<KeyComputeContextValues>,
}
impl KeyComputeContext {
    pub fn from_values(values: KeyComputeContextValues) -> Self {
        Self {
            values: RefCell::new(values),
        }
    }
    pub fn values(&self) -> KeyComputeContextValues {
        self.values.borrow().clone()
    }
    pub fn mode(&self) -> KeyboardMode {
        self.values.borrow().mode
    }
    pub fn set_mode(&self, value: KeyboardMode) {
        self.values.borrow_mut().mode = value;
    }
    pub fn shift_state(&self) -> ShiftState {
        self.values.borrow().shift_state
    }
    pub fn set_shift_state(&self, value: ShiftState) {
        self.values.borrow_mut().shift_state = value;
    }
    pub fn variation(&self) -> KeyVariation {
        self.values.borrow().variation
    }
    pub fn set_variation(&self, value: KeyVariation) {
        self.values.borrow_mut().variation = value;
    }
    pub fn direction(&self) -> LayoutDirection {
        self.values.borrow().direction
    }
    pub fn set_direction(&self, value: LayoutDirection) {
        self.values.borrow_mut().direction = value;
    }
    pub fn is_char_half_width(&self) -> bool {
        self.values.borrow().is_char_half_width
    }
    pub fn set_is_char_half_width(&self, value: bool) {
        self.values.borrow_mut().is_char_half_width = value;
    }
    pub fn is_kana_kata(&self) -> bool {
        self.values.borrow().is_kana_kata
    }
    pub fn set_is_kana_kata(&self, value: bool) {
        self.values.borrow_mut().is_kana_kata = value;
    }
    pub fn culture(&self) -> Option<TextCulture> {
        self.values.borrow().culture
    }
    pub fn set_culture(&self, value: Option<TextCulture>) {
        self.values.borrow_mut().culture = value;
    }
    pub fn enter_action(&self) -> EnterAction {
        self.values.borrow().enter_action
    }
    pub fn set_enter_action(&self, value: EnterAction) {
        self.values.borrow_mut().enter_action = value;
    }
    pub fn currency_set(&self) -> Option<Rc<CurrencySet>> {
        self.values.borrow().currency_set.clone()
    }
    pub fn set_currency_set(&self, value: Option<Rc<CurrencySet>>) {
        let old = std::mem::replace(&mut self.values.borrow_mut().currency_set, value);
        drop(old);
    }
    pub fn has_multiple_subtypes(&self) -> bool {
        self.values.borrow().has_multiple_subtypes
    }
    pub fn set_has_multiple_subtypes(&self, value: bool) {
        self.values.borrow_mut().has_multiple_subtypes = value;
    }
    pub fn utility_key_action(&self) -> UtilityKeyAction {
        self.values.borrow().utility_key_action
    }
    pub fn set_utility_key_action(&self, value: UtilityKeyAction) {
        self.values.borrow_mut().utility_key_action = value;
    }
    pub fn space_bar_mode(&self) -> SpaceBarMode {
        self.values.borrow().space_bar_mode
    }
    pub fn set_space_bar_mode(&self, value: SpaceBarMode) {
        self.values.borrow_mut().space_bar_mode = value;
    }
    pub fn space_bar_label(&self) -> Option<Utf16Text> {
        self.values.borrow().space_bar_label.clone()
    }
    pub fn set_space_bar_label(&self, value: Option<Utf16Text>) {
        let old = std::mem::replace(&mut self.values.borrow_mut().space_bar_label, value);
        drop(old);
    }
    pub fn hint_mode(&self) -> KeyHintMode {
        self.values.borrow().hint_mode
    }
    pub fn set_hint_mode(&self, value: KeyHintMode) {
        self.values.borrow_mut().hint_mode = value;
    }
    pub fn show_symbol_hints(&self) -> bool {
        self.values.borrow().show_symbol_hints
    }
    pub fn set_show_symbol_hints(&self, value: bool) {
        self.values.borrow_mut().show_symbol_hints = value;
    }
    pub fn show_number_hints(&self) -> bool {
        self.values.borrow().show_number_hints
    }
    pub fn set_show_number_hints(&self, value: bool) {
        self.values.borrow_mut().show_number_hints = value;
    }
    #[doc(hidden)]
    pub fn unshifted(&self) -> Self {
        let mut values = self.values();
        values.shift_state = ShiftState::Unshifted;
        Self::from_values(values)
    }
}
impl Default for KeyComputeContext {
    fn default() -> Self {
        Self::from_values(KeyComputeContextValues::default())
    }
}
impl Clone for KeyComputeContext {
    fn clone(&self) -> Self {
        Self::from_values(self.values())
    }
}
impl IKeyEvaluator for KeyComputeContext {
    fn mode(&self) -> KeyboardMode {
        self.mode()
    }
    fn shift_state(&self) -> ShiftState {
        self.shift_state()
    }
    fn variation(&self) -> KeyVariation {
        self.variation()
    }
    fn direction(&self) -> LayoutDirection {
        self.direction()
    }
    fn is_char_half_width(&self) -> bool {
        self.is_char_half_width()
    }
    fn is_kana_kata(&self) -> bool {
        self.is_kana_kata()
    }
    fn culture(&self) -> Option<TextCulture> {
        self.culture()
    }
}

// Generated from pinned upstream declarations by scripts/generate_settings.py.
use super::settings_enums::*;
use crate::emoji::EmojiSkinTone;
use ferroui_system::{component_model::INotifyPropertyChanged, events::Event};
use serde::{
    Deserialize,
    de::{self, MapAccess, Visitor},
};
use serde_json::Value;
use std::{cell::RefCell, rc::Rc};
/// Shared mutable list; nullable elements match List<string> deserialization.
pub type SubtypeList = Rc<RefCell<Vec<Option<String>>>>;
pub struct KeyboardSettings {
    property_changed: Event<str>,
    subtypes: RefCell<SubtypeList>,
    number_row: RefCell<bool>,
    hinted_number_row: RefCell<bool>,
    hinted_symbols: RefCell<bool>,
    hint_mode: RefCell<KeyHintMode>,
    utility_key_action: RefCell<UtilityKeyAction>,
    space_bar_mode: RefCell<SpaceBarMode>,
    key_height: RefCell<f64>,
    height_scale: RefCell<f64>,
    key_spacing_horizontal: RefCell<f64>,
    key_spacing_vertical: RefCell<f64>,
    font_scale: RefCell<f64>,
    max_keyboard_width: RefCell<f64>,
    one_handed_mode: RefCell<OneHandedMode>,
    one_handed_width: RefCell<f64>,
    split_keyboard: RefCell<bool>,
    split_gap_ratio: RefCell<f64>,
    floating: RefCell<bool>,
    floating_width_ratio: RefCell<f64>,
    show_key_preview: RefCell<bool>,
    long_press_delay: RefCell<i32>,
    key_repeat_delay: RefCell<i32>,
    key_repeat_interval: RefCell<i32>,
    double_tap_shift_for_caps_lock: RefCell<bool>,
    double_tap_delay: RefCell<i32>,
    auto_capitalization: RefCell<bool>,
    double_space_period: RefCell<bool>,
    auto_space_punctuation: RefCell<bool>,
    show_suggestions: RefCell<bool>,
    auto_correct: RefCell<bool>,
    next_word_prediction: RefCell<bool>,
    learn_words: RefCell<bool>,
    emoji_suggestions: RefCell<bool>,
    suggestion_count: RefCell<i32>,
    undo_auto_correct_on_backspace: RefCell<bool>,
    incognito_mode: RefCell<bool>,
    glide_typing: RefCell<bool>,
    glide_show_trail: RefCell<bool>,
    glide_trail_duration: RefCell<i32>,
    glide_preview: RefCell<bool>,
    swipe_distance_threshold: RefCell<f64>,
    swipe_up: RefCell<SwipeAction>,
    swipe_down: RefCell<SwipeAction>,
    swipe_left: RefCell<SwipeAction>,
    swipe_right: RefCell<SwipeAction>,
    space_bar_swipe_up: RefCell<SwipeAction>,
    space_bar_swipe_left: RefCell<SwipeAction>,
    space_bar_swipe_right: RefCell<SwipeAction>,
    space_bar_long_press: RefCell<SwipeAction>,
    delete_key_swipe_left: RefCell<SwipeAction>,
    delete_key_long_press: RefCell<SwipeAction>,
    haptic_feedback: RefCell<bool>,
    haptic_duration: RefCell<i32>,
    sound_feedback: RefCell<bool>,
    sound_volume: RefCell<f64>,
    smartbar_enabled: RefCell<bool>,
    smartbar_layout: RefCell<SmartbarLayout>,
    theme_mode: RefCell<ThemeMode>,
    light_theme: RefCell<String>,
    dark_theme: RefCell<String>,
    emoji_skin_tone: RefCell<EmojiSkinTone>,
    emoji_history_enabled: RefCell<bool>,
    clipboard_history_enabled: RefCell<bool>,
    clipboard_suggestion: RefCell<bool>,
}
impl Default for KeyboardSettings {
    fn default() -> Self {
        Self {
            property_changed: Event::new(),
            subtypes: RefCell::new(Rc::new(RefCell::new(vec![Some("en-US/qwerty".into())]))),
            number_row: RefCell::new(false),
            hinted_number_row: RefCell::new(true),
            hinted_symbols: RefCell::new(true),
            hint_mode: RefCell::new(KeyHintMode::AccentPriority),
            utility_key_action: RefCell::new(UtilityKeyAction::Dynamic),
            space_bar_mode: RefCell::new(SpaceBarMode::CurrentLanguage),
            key_height: RefCell::new(54.0),
            height_scale: RefCell::new(1.0),
            key_spacing_horizontal: RefCell::new(5.0),
            key_spacing_vertical: RefCell::new(9.0),
            font_scale: RefCell::new(1.0),
            max_keyboard_width: RefCell::new(1100.0),
            one_handed_mode: RefCell::new(OneHandedMode(0)),
            one_handed_width: RefCell::new(0.82),
            split_keyboard: RefCell::new(false),
            split_gap_ratio: RefCell::new(0.22),
            floating: RefCell::new(false),
            floating_width_ratio: RefCell::new(0.6),
            show_key_preview: RefCell::new(true),
            long_press_delay: RefCell::new(300),
            key_repeat_delay: RefCell::new(400),
            key_repeat_interval: RefCell::new(50),
            double_tap_shift_for_caps_lock: RefCell::new(true),
            double_tap_delay: RefCell::new(350),
            auto_capitalization: RefCell::new(true),
            double_space_period: RefCell::new(true),
            auto_space_punctuation: RefCell::new(true),
            show_suggestions: RefCell::new(true),
            auto_correct: RefCell::new(true),
            next_word_prediction: RefCell::new(true),
            learn_words: RefCell::new(true),
            emoji_suggestions: RefCell::new(true),
            suggestion_count: RefCell::new(3),
            undo_auto_correct_on_backspace: RefCell::new(true),
            incognito_mode: RefCell::new(false),
            glide_typing: RefCell::new(true),
            glide_show_trail: RefCell::new(true),
            glide_trail_duration: RefCell::new(200),
            glide_preview: RefCell::new(true),
            swipe_distance_threshold: RefCell::new(32.0),
            swipe_up: RefCell::new(SwipeAction::Shift),
            swipe_down: RefCell::new(SwipeAction::HideKeyboard),
            swipe_left: RefCell::new(SwipeAction::SwitchToNextSubtype),
            swipe_right: RefCell::new(SwipeAction::SwitchToPrevSubtype),
            space_bar_swipe_up: RefCell::new(SwipeAction::SwitchToClipboardContext),
            space_bar_swipe_left: RefCell::new(SwipeAction::MoveCursorLeft),
            space_bar_swipe_right: RefCell::new(SwipeAction::MoveCursorRight),
            space_bar_long_press: RefCell::new(SwipeAction::ShowSubtypePicker),
            delete_key_swipe_left: RefCell::new(SwipeAction::DeleteWordsPrecisely),
            delete_key_long_press: RefCell::new(SwipeAction::DeleteCharacter),
            haptic_feedback: RefCell::new(true),
            haptic_duration: RefCell::new(12),
            sound_feedback: RefCell::new(false),
            sound_volume: RefCell::new(0.5),
            smartbar_enabled: RefCell::new(true),
            smartbar_layout: RefCell::new(SmartbarLayout::SuggestionsWithActions),
            theme_mode: RefCell::new(ThemeMode::FollowSystem),
            light_theme: RefCell::new("floris_day".into()),
            dark_theme: RefCell::new("floris_night".into()),
            emoji_skin_tone: RefCell::new(EmojiSkinTone::Default),
            emoji_history_enabled: RefCell::new(true),
            clipboard_history_enabled: RefCell::new(true),
            clipboard_suggestion: RefCell::new(true),
        }
    }
}
impl INotifyPropertyChanged for KeyboardSettings {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}
impl KeyboardSettings {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn subtypes(&self) -> SubtypeList {
        self.subtypes.borrow().clone()
    }
    pub fn set_subtypes(&self, value: Option<SubtypeList>) {
        let value = value.unwrap_or_else(|| Rc::new(RefCell::new(Vec::new())));
        {
            let mut field = self.subtypes.borrow_mut();
            if Rc::ptr_eq(&field, &value) {
                return;
            }
            *field = value;
        }
        self.property_changed.raise("Subtypes");
    }
    pub fn number_row(&self) -> bool {
        *self.number_row.borrow()
    }
    pub fn set_number_row(&self, value: bool) {
        {
            let mut field = self.number_row.borrow_mut();
            if *field == value {
                return;
            }
            *field = value;
        }
        self.property_changed.raise("NumberRow");
    }
    pub fn hinted_number_row(&self) -> bool {
        *self.hinted_number_row.borrow()
    }
    pub fn set_hinted_number_row(&self, value: bool) {
        {
            let mut field = self.hinted_number_row.borrow_mut();
            if *field == value {
                return;
            }
            *field = value;
        }
        self.property_changed.raise("HintedNumberRow");
    }
    pub fn hinted_symbols(&self) -> bool {
        *self.hinted_symbols.borrow()
    }
    pub fn set_hinted_symbols(&self, value: bool) {
        {
            let mut field = self.hinted_symbols.borrow_mut();
            if *field == value {
                return;
            }
            *field = value;
        }
        self.property_changed.raise("HintedSymbols");
    }
    pub fn hint_mode(&self) -> KeyHintMode {
        *self.hint_mode.borrow()
    }
    pub fn set_hint_mode(&self, value: KeyHintMode) {
        {
            let mut field = self.hint_mode.borrow_mut();
            if *field == value {
                return;
            }
            *field = value;
        }
        self.property_changed.raise("HintMode");
    }
    pub fn utility_key_action(&self) -> UtilityKeyAction {
        *self.utility_key_action.borrow()
    }
    pub fn set_utility_key_action(&self, value: UtilityKeyAction) {
        {
            let mut field = self.utility_key_action.borrow_mut();
            if *field == value {
                return;
            }
            *field = value;
        }
        self.property_changed.raise("UtilityKeyAction");
    }
    pub fn space_bar_mode(&self) -> SpaceBarMode {
        *self.space_bar_mode.borrow()
    }
    pub fn set_space_bar_mode(&self, value: SpaceBarMode) {
        {
            let mut field = self.space_bar_mode.borrow_mut();
            if *field == value {
                return;
            }
            *field = value;
        }
        self.property_changed.raise("SpaceBarMode");
    }
    pub fn key_height(&self) -> f64 {
        *self.key_height.borrow()
    }
    pub fn set_key_height(&self, value: f64) {
        let value = value.clamp(24.0, 160.0);
        {
            let mut field = self.key_height.borrow_mut();
            if *field == value || (field.is_nan() && value.is_nan()) {
                return;
            }
            *field = value;
        }
        self.property_changed.raise("KeyHeight");
    }
    pub fn height_scale(&self) -> f64 {
        *self.height_scale.borrow()
    }
    pub fn set_height_scale(&self, value: f64) {
        let value = value.clamp(0.5, 2.0);
        {
            let mut field = self.height_scale.borrow_mut();
            if *field == value || (field.is_nan() && value.is_nan()) {
                return;
            }
            *field = value;
        }
        self.property_changed.raise("HeightScale");
    }
    pub fn key_spacing_horizontal(&self) -> f64 {
        *self.key_spacing_horizontal.borrow()
    }
    pub fn set_key_spacing_horizontal(&self, value: f64) {
        let value = value.clamp(0.0, 24.0);
        {
            let mut field = self.key_spacing_horizontal.borrow_mut();
            if *field == value || (field.is_nan() && value.is_nan()) {
                return;
            }
            *field = value;
        }
        self.property_changed.raise("KeySpacingHorizontal");
    }
    pub fn key_spacing_vertical(&self) -> f64 {
        *self.key_spacing_vertical.borrow()
    }
    pub fn set_key_spacing_vertical(&self, value: f64) {
        let value = value.clamp(0.0, 32.0);
        {
            let mut field = self.key_spacing_vertical.borrow_mut();
            if *field == value || (field.is_nan() && value.is_nan()) {
                return;
            }
            *field = value;
        }
        self.property_changed.raise("KeySpacingVertical");
    }
    pub fn font_scale(&self) -> f64 {
        *self.font_scale.borrow()
    }
    pub fn set_font_scale(&self, value: f64) {
        let value = value.clamp(0.5, 2.0);
        {
            let mut field = self.font_scale.borrow_mut();
            if *field == value || (field.is_nan() && value.is_nan()) {
                return;
            }
            *field = value;
        }
        self.property_changed.raise("FontScale");
    }
    pub fn max_keyboard_width(&self) -> f64 {
        *self.max_keyboard_width.borrow()
    }
    pub fn set_max_keyboard_width(&self, value: f64) {
        let value = if value < 0.0 { 0.0 } else { value };
        {
            let mut field = self.max_keyboard_width.borrow_mut();
            if *field == value || (field.is_nan() && value.is_nan()) {
                return;
            }
            *field = value;
        }
        self.property_changed.raise("MaxKeyboardWidth");
    }
    pub fn one_handed_mode(&self) -> OneHandedMode {
        *self.one_handed_mode.borrow()
    }
    pub fn set_one_handed_mode(&self, value: OneHandedMode) {
        {
            let mut field = self.one_handed_mode.borrow_mut();
            if *field == value {
                return;
            }
            *field = value;
        }
        self.property_changed.raise("OneHandedMode");
    }
    pub fn one_handed_width(&self) -> f64 {
        *self.one_handed_width.borrow()
    }
    pub fn set_one_handed_width(&self, value: f64) {
        let value = value.clamp(0.5, 0.95);
        {
            let mut field = self.one_handed_width.borrow_mut();
            if *field == value || (field.is_nan() && value.is_nan()) {
                return;
            }
            *field = value;
        }
        self.property_changed.raise("OneHandedWidth");
    }
    pub fn split_keyboard(&self) -> bool {
        *self.split_keyboard.borrow()
    }
    pub fn set_split_keyboard(&self, value: bool) {
        {
            let mut field = self.split_keyboard.borrow_mut();
            if *field == value {
                return;
            }
            *field = value;
        }
        self.property_changed.raise("SplitKeyboard");
    }
    pub fn split_gap_ratio(&self) -> f64 {
        *self.split_gap_ratio.borrow()
    }
    pub fn set_split_gap_ratio(&self, value: f64) {
        let value = value.clamp(0.05, 0.5);
        {
            let mut field = self.split_gap_ratio.borrow_mut();
            if *field == value || (field.is_nan() && value.is_nan()) {
                return;
            }
            *field = value;
        }
        self.property_changed.raise("SplitGapRatio");
    }
    pub fn floating(&self) -> bool {
        *self.floating.borrow()
    }
    pub fn set_floating(&self, value: bool) {
        {
            let mut field = self.floating.borrow_mut();
            if *field == value {
                return;
            }
            *field = value;
        }
        self.property_changed.raise("Floating");
    }
    pub fn floating_width_ratio(&self) -> f64 {
        *self.floating_width_ratio.borrow()
    }
    pub fn set_floating_width_ratio(&self, value: f64) {
        let value = value.clamp(0.3, 1.0);
        {
            let mut field = self.floating_width_ratio.borrow_mut();
            if *field == value || (field.is_nan() && value.is_nan()) {
                return;
            }
            *field = value;
        }
        self.property_changed.raise("FloatingWidthRatio");
    }
    pub fn show_key_preview(&self) -> bool {
        *self.show_key_preview.borrow()
    }
    pub fn set_show_key_preview(&self, value: bool) {
        {
            let mut field = self.show_key_preview.borrow_mut();
            if *field == value {
                return;
            }
            *field = value;
        }
        self.property_changed.raise("ShowKeyPreview");
    }
    pub fn long_press_delay(&self) -> i32 {
        *self.long_press_delay.borrow()
    }
    pub fn set_long_press_delay(&self, value: i32) {
        let value = value.clamp(100, 1500);
        {
            let mut field = self.long_press_delay.borrow_mut();
            if *field == value {
                return;
            }
            *field = value;
        }
        self.property_changed.raise("LongPressDelay");
    }
    pub fn key_repeat_delay(&self) -> i32 {
        *self.key_repeat_delay.borrow()
    }
    pub fn set_key_repeat_delay(&self, value: i32) {
        let value = value.clamp(100, 2000);
        {
            let mut field = self.key_repeat_delay.borrow_mut();
            if *field == value {
                return;
            }
            *field = value;
        }
        self.property_changed.raise("KeyRepeatDelay");
    }
    pub fn key_repeat_interval(&self) -> i32 {
        *self.key_repeat_interval.borrow()
    }
    pub fn set_key_repeat_interval(&self, value: i32) {
        let value = value.clamp(15, 500);
        {
            let mut field = self.key_repeat_interval.borrow_mut();
            if *field == value {
                return;
            }
            *field = value;
        }
        self.property_changed.raise("KeyRepeatInterval");
    }
    pub fn double_tap_shift_for_caps_lock(&self) -> bool {
        *self.double_tap_shift_for_caps_lock.borrow()
    }
    pub fn set_double_tap_shift_for_caps_lock(&self, value: bool) {
        {
            let mut field = self.double_tap_shift_for_caps_lock.borrow_mut();
            if *field == value {
                return;
            }
            *field = value;
        }
        self.property_changed.raise("DoubleTapShiftForCapsLock");
    }
    pub fn double_tap_delay(&self) -> i32 {
        *self.double_tap_delay.borrow()
    }
    pub fn set_double_tap_delay(&self, value: i32) {
        let value = value.clamp(100, 1000);
        {
            let mut field = self.double_tap_delay.borrow_mut();
            if *field == value {
                return;
            }
            *field = value;
        }
        self.property_changed.raise("DoubleTapDelay");
    }
    pub fn auto_capitalization(&self) -> bool {
        *self.auto_capitalization.borrow()
    }
    pub fn set_auto_capitalization(&self, value: bool) {
        {
            let mut field = self.auto_capitalization.borrow_mut();
            if *field == value {
                return;
            }
            *field = value;
        }
        self.property_changed.raise("AutoCapitalization");
    }
    pub fn double_space_period(&self) -> bool {
        *self.double_space_period.borrow()
    }
    pub fn set_double_space_period(&self, value: bool) {
        {
            let mut field = self.double_space_period.borrow_mut();
            if *field == value {
                return;
            }
            *field = value;
        }
        self.property_changed.raise("DoubleSpacePeriod");
    }
    pub fn auto_space_punctuation(&self) -> bool {
        *self.auto_space_punctuation.borrow()
    }
    pub fn set_auto_space_punctuation(&self, value: bool) {
        {
            let mut field = self.auto_space_punctuation.borrow_mut();
            if *field == value {
                return;
            }
            *field = value;
        }
        self.property_changed.raise("AutoSpacePunctuation");
    }
    pub fn show_suggestions(&self) -> bool {
        *self.show_suggestions.borrow()
    }
    pub fn set_show_suggestions(&self, value: bool) {
        {
            let mut field = self.show_suggestions.borrow_mut();
            if *field == value {
                return;
            }
            *field = value;
        }
        self.property_changed.raise("ShowSuggestions");
    }
    pub fn auto_correct(&self) -> bool {
        *self.auto_correct.borrow()
    }
    pub fn set_auto_correct(&self, value: bool) {
        {
            let mut field = self.auto_correct.borrow_mut();
            if *field == value {
                return;
            }
            *field = value;
        }
        self.property_changed.raise("AutoCorrect");
    }
    pub fn next_word_prediction(&self) -> bool {
        *self.next_word_prediction.borrow()
    }
    pub fn set_next_word_prediction(&self, value: bool) {
        {
            let mut field = self.next_word_prediction.borrow_mut();
            if *field == value {
                return;
            }
            *field = value;
        }
        self.property_changed.raise("NextWordPrediction");
    }
    pub fn learn_words(&self) -> bool {
        *self.learn_words.borrow()
    }
    pub fn set_learn_words(&self, value: bool) {
        {
            let mut field = self.learn_words.borrow_mut();
            if *field == value {
                return;
            }
            *field = value;
        }
        self.property_changed.raise("LearnWords");
    }
    pub fn emoji_suggestions(&self) -> bool {
        *self.emoji_suggestions.borrow()
    }
    pub fn set_emoji_suggestions(&self, value: bool) {
        {
            let mut field = self.emoji_suggestions.borrow_mut();
            if *field == value {
                return;
            }
            *field = value;
        }
        self.property_changed.raise("EmojiSuggestions");
    }
    pub fn suggestion_count(&self) -> i32 {
        *self.suggestion_count.borrow()
    }
    pub fn set_suggestion_count(&self, value: i32) {
        let value = value.clamp(1, 7);
        {
            let mut field = self.suggestion_count.borrow_mut();
            if *field == value {
                return;
            }
            *field = value;
        }
        self.property_changed.raise("SuggestionCount");
    }
    pub fn undo_auto_correct_on_backspace(&self) -> bool {
        *self.undo_auto_correct_on_backspace.borrow()
    }
    pub fn set_undo_auto_correct_on_backspace(&self, value: bool) {
        {
            let mut field = self.undo_auto_correct_on_backspace.borrow_mut();
            if *field == value {
                return;
            }
            *field = value;
        }
        self.property_changed.raise("UndoAutoCorrectOnBackspace");
    }
    pub fn incognito_mode(&self) -> bool {
        *self.incognito_mode.borrow()
    }
    pub fn set_incognito_mode(&self, value: bool) {
        {
            let mut field = self.incognito_mode.borrow_mut();
            if *field == value {
                return;
            }
            *field = value;
        }
        self.property_changed.raise("IncognitoMode");
    }
    pub fn glide_typing(&self) -> bool {
        *self.glide_typing.borrow()
    }
    pub fn set_glide_typing(&self, value: bool) {
        {
            let mut field = self.glide_typing.borrow_mut();
            if *field == value {
                return;
            }
            *field = value;
        }
        self.property_changed.raise("GlideTyping");
    }
    pub fn glide_show_trail(&self) -> bool {
        *self.glide_show_trail.borrow()
    }
    pub fn set_glide_show_trail(&self, value: bool) {
        {
            let mut field = self.glide_show_trail.borrow_mut();
            if *field == value {
                return;
            }
            *field = value;
        }
        self.property_changed.raise("GlideShowTrail");
    }
    pub fn glide_trail_duration(&self) -> i32 {
        *self.glide_trail_duration.borrow()
    }
    pub fn set_glide_trail_duration(&self, value: i32) {
        let value = value.clamp(0, 2000);
        {
            let mut field = self.glide_trail_duration.borrow_mut();
            if *field == value {
                return;
            }
            *field = value;
        }
        self.property_changed.raise("GlideTrailDuration");
    }
    pub fn glide_preview(&self) -> bool {
        *self.glide_preview.borrow()
    }
    pub fn set_glide_preview(&self, value: bool) {
        {
            let mut field = self.glide_preview.borrow_mut();
            if *field == value {
                return;
            }
            *field = value;
        }
        self.property_changed.raise("GlidePreview");
    }
    pub fn swipe_distance_threshold(&self) -> f64 {
        *self.swipe_distance_threshold.borrow()
    }
    pub fn set_swipe_distance_threshold(&self, value: f64) {
        let value = value.clamp(8.0, 200.0);
        {
            let mut field = self.swipe_distance_threshold.borrow_mut();
            if *field == value || (field.is_nan() && value.is_nan()) {
                return;
            }
            *field = value;
        }
        self.property_changed.raise("SwipeDistanceThreshold");
    }
    pub fn swipe_up(&self) -> SwipeAction {
        *self.swipe_up.borrow()
    }
    pub fn set_swipe_up(&self, value: SwipeAction) {
        {
            let mut field = self.swipe_up.borrow_mut();
            if *field == value {
                return;
            }
            *field = value;
        }
        self.property_changed.raise("SwipeUp");
    }
    pub fn swipe_down(&self) -> SwipeAction {
        *self.swipe_down.borrow()
    }
    pub fn set_swipe_down(&self, value: SwipeAction) {
        {
            let mut field = self.swipe_down.borrow_mut();
            if *field == value {
                return;
            }
            *field = value;
        }
        self.property_changed.raise("SwipeDown");
    }
    pub fn swipe_left(&self) -> SwipeAction {
        *self.swipe_left.borrow()
    }
    pub fn set_swipe_left(&self, value: SwipeAction) {
        {
            let mut field = self.swipe_left.borrow_mut();
            if *field == value {
                return;
            }
            *field = value;
        }
        self.property_changed.raise("SwipeLeft");
    }
    pub fn swipe_right(&self) -> SwipeAction {
        *self.swipe_right.borrow()
    }
    pub fn set_swipe_right(&self, value: SwipeAction) {
        {
            let mut field = self.swipe_right.borrow_mut();
            if *field == value {
                return;
            }
            *field = value;
        }
        self.property_changed.raise("SwipeRight");
    }
    pub fn space_bar_swipe_up(&self) -> SwipeAction {
        *self.space_bar_swipe_up.borrow()
    }
    pub fn set_space_bar_swipe_up(&self, value: SwipeAction) {
        {
            let mut field = self.space_bar_swipe_up.borrow_mut();
            if *field == value {
                return;
            }
            *field = value;
        }
        self.property_changed.raise("SpaceBarSwipeUp");
    }
    pub fn space_bar_swipe_left(&self) -> SwipeAction {
        *self.space_bar_swipe_left.borrow()
    }
    pub fn set_space_bar_swipe_left(&self, value: SwipeAction) {
        {
            let mut field = self.space_bar_swipe_left.borrow_mut();
            if *field == value {
                return;
            }
            *field = value;
        }
        self.property_changed.raise("SpaceBarSwipeLeft");
    }
    pub fn space_bar_swipe_right(&self) -> SwipeAction {
        *self.space_bar_swipe_right.borrow()
    }
    pub fn set_space_bar_swipe_right(&self, value: SwipeAction) {
        {
            let mut field = self.space_bar_swipe_right.borrow_mut();
            if *field == value {
                return;
            }
            *field = value;
        }
        self.property_changed.raise("SpaceBarSwipeRight");
    }
    pub fn space_bar_long_press(&self) -> SwipeAction {
        *self.space_bar_long_press.borrow()
    }
    pub fn set_space_bar_long_press(&self, value: SwipeAction) {
        {
            let mut field = self.space_bar_long_press.borrow_mut();
            if *field == value {
                return;
            }
            *field = value;
        }
        self.property_changed.raise("SpaceBarLongPress");
    }
    pub fn delete_key_swipe_left(&self) -> SwipeAction {
        *self.delete_key_swipe_left.borrow()
    }
    pub fn set_delete_key_swipe_left(&self, value: SwipeAction) {
        {
            let mut field = self.delete_key_swipe_left.borrow_mut();
            if *field == value {
                return;
            }
            *field = value;
        }
        self.property_changed.raise("DeleteKeySwipeLeft");
    }
    pub fn delete_key_long_press(&self) -> SwipeAction {
        *self.delete_key_long_press.borrow()
    }
    pub fn set_delete_key_long_press(&self, value: SwipeAction) {
        {
            let mut field = self.delete_key_long_press.borrow_mut();
            if *field == value {
                return;
            }
            *field = value;
        }
        self.property_changed.raise("DeleteKeyLongPress");
    }
    pub fn haptic_feedback(&self) -> bool {
        *self.haptic_feedback.borrow()
    }
    pub fn set_haptic_feedback(&self, value: bool) {
        {
            let mut field = self.haptic_feedback.borrow_mut();
            if *field == value {
                return;
            }
            *field = value;
        }
        self.property_changed.raise("HapticFeedback");
    }
    pub fn haptic_duration(&self) -> i32 {
        *self.haptic_duration.borrow()
    }
    pub fn set_haptic_duration(&self, value: i32) {
        let value = value.clamp(1, 100);
        {
            let mut field = self.haptic_duration.borrow_mut();
            if *field == value {
                return;
            }
            *field = value;
        }
        self.property_changed.raise("HapticDuration");
    }
    pub fn sound_feedback(&self) -> bool {
        *self.sound_feedback.borrow()
    }
    pub fn set_sound_feedback(&self, value: bool) {
        {
            let mut field = self.sound_feedback.borrow_mut();
            if *field == value {
                return;
            }
            *field = value;
        }
        self.property_changed.raise("SoundFeedback");
    }
    pub fn sound_volume(&self) -> f64 {
        *self.sound_volume.borrow()
    }
    pub fn set_sound_volume(&self, value: f64) {
        let value = value.clamp(0.0, 1.0);
        {
            let mut field = self.sound_volume.borrow_mut();
            if *field == value || (field.is_nan() && value.is_nan()) {
                return;
            }
            *field = value;
        }
        self.property_changed.raise("SoundVolume");
    }
    pub fn smartbar_enabled(&self) -> bool {
        *self.smartbar_enabled.borrow()
    }
    pub fn set_smartbar_enabled(&self, value: bool) {
        {
            let mut field = self.smartbar_enabled.borrow_mut();
            if *field == value {
                return;
            }
            *field = value;
        }
        self.property_changed.raise("SmartbarEnabled");
    }
    pub fn smartbar_layout(&self) -> SmartbarLayout {
        *self.smartbar_layout.borrow()
    }
    pub fn set_smartbar_layout(&self, value: SmartbarLayout) {
        {
            let mut field = self.smartbar_layout.borrow_mut();
            if *field == value {
                return;
            }
            *field = value;
        }
        self.property_changed.raise("SmartbarLayout");
    }
    pub fn theme_mode(&self) -> ThemeMode {
        *self.theme_mode.borrow()
    }
    pub fn set_theme_mode(&self, value: ThemeMode) {
        {
            let mut field = self.theme_mode.borrow_mut();
            if *field == value {
                return;
            }
            *field = value;
        }
        self.property_changed.raise("ThemeMode");
    }
    pub fn light_theme(&self) -> String {
        self.light_theme.borrow().clone()
    }
    pub fn set_light_theme(&self, value: Option<String>) {
        let value = value.unwrap_or_else(|| "floris_day".into());
        {
            let mut field = self.light_theme.borrow_mut();
            if *field == value {
                return;
            }
            *field = value;
        }
        self.property_changed.raise("LightTheme");
    }
    pub fn dark_theme(&self) -> String {
        self.dark_theme.borrow().clone()
    }
    pub fn set_dark_theme(&self, value: Option<String>) {
        let value = value.unwrap_or_else(|| "floris_night".into());
        {
            let mut field = self.dark_theme.borrow_mut();
            if *field == value {
                return;
            }
            *field = value;
        }
        self.property_changed.raise("DarkTheme");
    }
    pub fn emoji_skin_tone(&self) -> EmojiSkinTone {
        *self.emoji_skin_tone.borrow()
    }
    pub fn set_emoji_skin_tone(&self, value: EmojiSkinTone) {
        {
            let mut field = self.emoji_skin_tone.borrow_mut();
            if *field == value {
                return;
            }
            *field = value;
        }
        self.property_changed.raise("EmojiSkinTone");
    }
    pub fn emoji_history_enabled(&self) -> bool {
        *self.emoji_history_enabled.borrow()
    }
    pub fn set_emoji_history_enabled(&self, value: bool) {
        {
            let mut field = self.emoji_history_enabled.borrow_mut();
            if *field == value {
                return;
            }
            *field = value;
        }
        self.property_changed.raise("EmojiHistoryEnabled");
    }
    pub fn clipboard_history_enabled(&self) -> bool {
        *self.clipboard_history_enabled.borrow()
    }
    pub fn set_clipboard_history_enabled(&self, value: bool) {
        {
            let mut field = self.clipboard_history_enabled.borrow_mut();
            if *field == value {
                return;
            }
            *field = value;
        }
        self.property_changed.raise("ClipboardHistoryEnabled");
    }
    pub fn clipboard_suggestion(&self) -> bool {
        *self.clipboard_suggestion.borrow()
    }
    pub fn set_clipboard_suggestion(&self, value: bool) {
        {
            let mut field = self.clipboard_suggestion.borrow_mut();
            if *field == value {
                return;
            }
            *field = value;
        }
        self.property_changed.raise("ClipboardSuggestion");
    }
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str::<Option<Self>>(json).map(Option::unwrap_or_default)
    }
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        use serde::ser::{Serialize, SerializeMap, Serializer};
        struct View<'a>(&'a KeyboardSettings);
        impl Serialize for View<'_> {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                let s = self.0;
                let mut map = serializer.serialize_map(None)?;
                map.serialize_entry("subtypes", &&*s.subtypes().borrow())?;
                map.serialize_entry("numberRow", &s.number_row())?;
                map.serialize_entry("hintedNumberRow", &s.hinted_number_row())?;
                map.serialize_entry("hintedSymbols", &s.hinted_symbols())?;
                map.serialize_entry(
                    "hintMode",
                    &enum_json(
                        s.hint_mode().0,
                        &["Disabled", "AccentPriority", "HintPriority"],
                    ),
                )?;
                map.serialize_entry(
                    "utilityKeyAction",
                    &enum_json(
                        s.utility_key_action().0,
                        &["Emoji", "LanguageSwitch", "Dynamic", "Hidden"],
                    ),
                )?;
                map.serialize_entry(
                    "spaceBarMode",
                    &enum_json(
                        s.space_bar_mode().0,
                        &["Nothing", "CurrentLanguage", "SpaceBarKey"],
                    ),
                )?;
                if !s.key_height().is_finite() {
                    return Err(serde::ser::Error::custom("Non-finite number"));
                }
                map.serialize_entry("keyHeight", &s.key_height())?;
                if !s.height_scale().is_finite() {
                    return Err(serde::ser::Error::custom("Non-finite number"));
                }
                map.serialize_entry("heightScale", &s.height_scale())?;
                if !s.key_spacing_horizontal().is_finite() {
                    return Err(serde::ser::Error::custom("Non-finite number"));
                }
                map.serialize_entry("keySpacingHorizontal", &s.key_spacing_horizontal())?;
                if !s.key_spacing_vertical().is_finite() {
                    return Err(serde::ser::Error::custom("Non-finite number"));
                }
                map.serialize_entry("keySpacingVertical", &s.key_spacing_vertical())?;
                if !s.font_scale().is_finite() {
                    return Err(serde::ser::Error::custom("Non-finite number"));
                }
                map.serialize_entry("fontScale", &s.font_scale())?;
                if !s.max_keyboard_width().is_finite() {
                    return Err(serde::ser::Error::custom("Non-finite number"));
                }
                map.serialize_entry("maxKeyboardWidth", &s.max_keyboard_width())?;
                map.serialize_entry(
                    "oneHandedMode",
                    &enum_json(s.one_handed_mode().0, &["Off", "Left", "Right"]),
                )?;
                if !s.one_handed_width().is_finite() {
                    return Err(serde::ser::Error::custom("Non-finite number"));
                }
                map.serialize_entry("oneHandedWidth", &s.one_handed_width())?;
                map.serialize_entry("splitKeyboard", &s.split_keyboard())?;
                if !s.split_gap_ratio().is_finite() {
                    return Err(serde::ser::Error::custom("Non-finite number"));
                }
                map.serialize_entry("splitGapRatio", &s.split_gap_ratio())?;
                map.serialize_entry("floating", &s.floating())?;
                if !s.floating_width_ratio().is_finite() {
                    return Err(serde::ser::Error::custom("Non-finite number"));
                }
                map.serialize_entry("floatingWidthRatio", &s.floating_width_ratio())?;
                map.serialize_entry("showKeyPreview", &s.show_key_preview())?;
                map.serialize_entry("longPressDelay", &s.long_press_delay())?;
                map.serialize_entry("keyRepeatDelay", &s.key_repeat_delay())?;
                map.serialize_entry("keyRepeatInterval", &s.key_repeat_interval())?;
                map.serialize_entry(
                    "doubleTapShiftForCapsLock",
                    &s.double_tap_shift_for_caps_lock(),
                )?;
                map.serialize_entry("doubleTapDelay", &s.double_tap_delay())?;
                map.serialize_entry("autoCapitalization", &s.auto_capitalization())?;
                map.serialize_entry("doubleSpacePeriod", &s.double_space_period())?;
                map.serialize_entry("autoSpacePunctuation", &s.auto_space_punctuation())?;
                map.serialize_entry("showSuggestions", &s.show_suggestions())?;
                map.serialize_entry("autoCorrect", &s.auto_correct())?;
                map.serialize_entry("nextWordPrediction", &s.next_word_prediction())?;
                map.serialize_entry("learnWords", &s.learn_words())?;
                map.serialize_entry("emojiSuggestions", &s.emoji_suggestions())?;
                map.serialize_entry("suggestionCount", &s.suggestion_count())?;
                map.serialize_entry(
                    "undoAutoCorrectOnBackspace",
                    &s.undo_auto_correct_on_backspace(),
                )?;
                map.serialize_entry("incognitoMode", &s.incognito_mode())?;
                map.serialize_entry("glideTyping", &s.glide_typing())?;
                map.serialize_entry("glideShowTrail", &s.glide_show_trail())?;
                map.serialize_entry("glideTrailDuration", &s.glide_trail_duration())?;
                map.serialize_entry("glidePreview", &s.glide_preview())?;
                if !s.swipe_distance_threshold().is_finite() {
                    return Err(serde::ser::Error::custom("Non-finite number"));
                }
                map.serialize_entry("swipeDistanceThreshold", &s.swipe_distance_threshold())?;
                map.serialize_entry(
                    "swipeUp",
                    &enum_json(
                        s.swipe_up().0,
                        &[
                            "NoAction",
                            "CycleToPreviousKeyboardMode",
                            "CycleToNextKeyboardMode",
                            "DeleteCharacter",
                            "DeleteCharactersPrecisely",
                            "DeleteWord",
                            "DeleteWordsPrecisely",
                            "HideKeyboard",
                            "InsertSpace",
                            "MoveCursorUp",
                            "MoveCursorDown",
                            "MoveCursorLeft",
                            "MoveCursorRight",
                            "MoveCursorStartOfLine",
                            "MoveCursorEndOfLine",
                            "MoveCursorStartOfPage",
                            "MoveCursorEndOfPage",
                            "Redo",
                            "SelectCharactersPrecisely",
                            "SelectWordsPrecisely",
                            "Shift",
                            "ShowSubtypePicker",
                            "SwitchToPrevSubtype",
                            "SwitchToNextSubtype",
                            "SwitchToClipboardContext",
                            "SwitchToMediaContext",
                            "SwitchToEditingContext",
                            "ToggleOneHandedMode",
                            "ToggleSmartbarVisibility",
                            "Undo",
                        ],
                    ),
                )?;
                map.serialize_entry(
                    "swipeDown",
                    &enum_json(
                        s.swipe_down().0,
                        &[
                            "NoAction",
                            "CycleToPreviousKeyboardMode",
                            "CycleToNextKeyboardMode",
                            "DeleteCharacter",
                            "DeleteCharactersPrecisely",
                            "DeleteWord",
                            "DeleteWordsPrecisely",
                            "HideKeyboard",
                            "InsertSpace",
                            "MoveCursorUp",
                            "MoveCursorDown",
                            "MoveCursorLeft",
                            "MoveCursorRight",
                            "MoveCursorStartOfLine",
                            "MoveCursorEndOfLine",
                            "MoveCursorStartOfPage",
                            "MoveCursorEndOfPage",
                            "Redo",
                            "SelectCharactersPrecisely",
                            "SelectWordsPrecisely",
                            "Shift",
                            "ShowSubtypePicker",
                            "SwitchToPrevSubtype",
                            "SwitchToNextSubtype",
                            "SwitchToClipboardContext",
                            "SwitchToMediaContext",
                            "SwitchToEditingContext",
                            "ToggleOneHandedMode",
                            "ToggleSmartbarVisibility",
                            "Undo",
                        ],
                    ),
                )?;
                map.serialize_entry(
                    "swipeLeft",
                    &enum_json(
                        s.swipe_left().0,
                        &[
                            "NoAction",
                            "CycleToPreviousKeyboardMode",
                            "CycleToNextKeyboardMode",
                            "DeleteCharacter",
                            "DeleteCharactersPrecisely",
                            "DeleteWord",
                            "DeleteWordsPrecisely",
                            "HideKeyboard",
                            "InsertSpace",
                            "MoveCursorUp",
                            "MoveCursorDown",
                            "MoveCursorLeft",
                            "MoveCursorRight",
                            "MoveCursorStartOfLine",
                            "MoveCursorEndOfLine",
                            "MoveCursorStartOfPage",
                            "MoveCursorEndOfPage",
                            "Redo",
                            "SelectCharactersPrecisely",
                            "SelectWordsPrecisely",
                            "Shift",
                            "ShowSubtypePicker",
                            "SwitchToPrevSubtype",
                            "SwitchToNextSubtype",
                            "SwitchToClipboardContext",
                            "SwitchToMediaContext",
                            "SwitchToEditingContext",
                            "ToggleOneHandedMode",
                            "ToggleSmartbarVisibility",
                            "Undo",
                        ],
                    ),
                )?;
                map.serialize_entry(
                    "swipeRight",
                    &enum_json(
                        s.swipe_right().0,
                        &[
                            "NoAction",
                            "CycleToPreviousKeyboardMode",
                            "CycleToNextKeyboardMode",
                            "DeleteCharacter",
                            "DeleteCharactersPrecisely",
                            "DeleteWord",
                            "DeleteWordsPrecisely",
                            "HideKeyboard",
                            "InsertSpace",
                            "MoveCursorUp",
                            "MoveCursorDown",
                            "MoveCursorLeft",
                            "MoveCursorRight",
                            "MoveCursorStartOfLine",
                            "MoveCursorEndOfLine",
                            "MoveCursorStartOfPage",
                            "MoveCursorEndOfPage",
                            "Redo",
                            "SelectCharactersPrecisely",
                            "SelectWordsPrecisely",
                            "Shift",
                            "ShowSubtypePicker",
                            "SwitchToPrevSubtype",
                            "SwitchToNextSubtype",
                            "SwitchToClipboardContext",
                            "SwitchToMediaContext",
                            "SwitchToEditingContext",
                            "ToggleOneHandedMode",
                            "ToggleSmartbarVisibility",
                            "Undo",
                        ],
                    ),
                )?;
                map.serialize_entry(
                    "spaceBarSwipeUp",
                    &enum_json(
                        s.space_bar_swipe_up().0,
                        &[
                            "NoAction",
                            "CycleToPreviousKeyboardMode",
                            "CycleToNextKeyboardMode",
                            "DeleteCharacter",
                            "DeleteCharactersPrecisely",
                            "DeleteWord",
                            "DeleteWordsPrecisely",
                            "HideKeyboard",
                            "InsertSpace",
                            "MoveCursorUp",
                            "MoveCursorDown",
                            "MoveCursorLeft",
                            "MoveCursorRight",
                            "MoveCursorStartOfLine",
                            "MoveCursorEndOfLine",
                            "MoveCursorStartOfPage",
                            "MoveCursorEndOfPage",
                            "Redo",
                            "SelectCharactersPrecisely",
                            "SelectWordsPrecisely",
                            "Shift",
                            "ShowSubtypePicker",
                            "SwitchToPrevSubtype",
                            "SwitchToNextSubtype",
                            "SwitchToClipboardContext",
                            "SwitchToMediaContext",
                            "SwitchToEditingContext",
                            "ToggleOneHandedMode",
                            "ToggleSmartbarVisibility",
                            "Undo",
                        ],
                    ),
                )?;
                map.serialize_entry(
                    "spaceBarSwipeLeft",
                    &enum_json(
                        s.space_bar_swipe_left().0,
                        &[
                            "NoAction",
                            "CycleToPreviousKeyboardMode",
                            "CycleToNextKeyboardMode",
                            "DeleteCharacter",
                            "DeleteCharactersPrecisely",
                            "DeleteWord",
                            "DeleteWordsPrecisely",
                            "HideKeyboard",
                            "InsertSpace",
                            "MoveCursorUp",
                            "MoveCursorDown",
                            "MoveCursorLeft",
                            "MoveCursorRight",
                            "MoveCursorStartOfLine",
                            "MoveCursorEndOfLine",
                            "MoveCursorStartOfPage",
                            "MoveCursorEndOfPage",
                            "Redo",
                            "SelectCharactersPrecisely",
                            "SelectWordsPrecisely",
                            "Shift",
                            "ShowSubtypePicker",
                            "SwitchToPrevSubtype",
                            "SwitchToNextSubtype",
                            "SwitchToClipboardContext",
                            "SwitchToMediaContext",
                            "SwitchToEditingContext",
                            "ToggleOneHandedMode",
                            "ToggleSmartbarVisibility",
                            "Undo",
                        ],
                    ),
                )?;
                map.serialize_entry(
                    "spaceBarSwipeRight",
                    &enum_json(
                        s.space_bar_swipe_right().0,
                        &[
                            "NoAction",
                            "CycleToPreviousKeyboardMode",
                            "CycleToNextKeyboardMode",
                            "DeleteCharacter",
                            "DeleteCharactersPrecisely",
                            "DeleteWord",
                            "DeleteWordsPrecisely",
                            "HideKeyboard",
                            "InsertSpace",
                            "MoveCursorUp",
                            "MoveCursorDown",
                            "MoveCursorLeft",
                            "MoveCursorRight",
                            "MoveCursorStartOfLine",
                            "MoveCursorEndOfLine",
                            "MoveCursorStartOfPage",
                            "MoveCursorEndOfPage",
                            "Redo",
                            "SelectCharactersPrecisely",
                            "SelectWordsPrecisely",
                            "Shift",
                            "ShowSubtypePicker",
                            "SwitchToPrevSubtype",
                            "SwitchToNextSubtype",
                            "SwitchToClipboardContext",
                            "SwitchToMediaContext",
                            "SwitchToEditingContext",
                            "ToggleOneHandedMode",
                            "ToggleSmartbarVisibility",
                            "Undo",
                        ],
                    ),
                )?;
                map.serialize_entry(
                    "spaceBarLongPress",
                    &enum_json(
                        s.space_bar_long_press().0,
                        &[
                            "NoAction",
                            "CycleToPreviousKeyboardMode",
                            "CycleToNextKeyboardMode",
                            "DeleteCharacter",
                            "DeleteCharactersPrecisely",
                            "DeleteWord",
                            "DeleteWordsPrecisely",
                            "HideKeyboard",
                            "InsertSpace",
                            "MoveCursorUp",
                            "MoveCursorDown",
                            "MoveCursorLeft",
                            "MoveCursorRight",
                            "MoveCursorStartOfLine",
                            "MoveCursorEndOfLine",
                            "MoveCursorStartOfPage",
                            "MoveCursorEndOfPage",
                            "Redo",
                            "SelectCharactersPrecisely",
                            "SelectWordsPrecisely",
                            "Shift",
                            "ShowSubtypePicker",
                            "SwitchToPrevSubtype",
                            "SwitchToNextSubtype",
                            "SwitchToClipboardContext",
                            "SwitchToMediaContext",
                            "SwitchToEditingContext",
                            "ToggleOneHandedMode",
                            "ToggleSmartbarVisibility",
                            "Undo",
                        ],
                    ),
                )?;
                map.serialize_entry(
                    "deleteKeySwipeLeft",
                    &enum_json(
                        s.delete_key_swipe_left().0,
                        &[
                            "NoAction",
                            "CycleToPreviousKeyboardMode",
                            "CycleToNextKeyboardMode",
                            "DeleteCharacter",
                            "DeleteCharactersPrecisely",
                            "DeleteWord",
                            "DeleteWordsPrecisely",
                            "HideKeyboard",
                            "InsertSpace",
                            "MoveCursorUp",
                            "MoveCursorDown",
                            "MoveCursorLeft",
                            "MoveCursorRight",
                            "MoveCursorStartOfLine",
                            "MoveCursorEndOfLine",
                            "MoveCursorStartOfPage",
                            "MoveCursorEndOfPage",
                            "Redo",
                            "SelectCharactersPrecisely",
                            "SelectWordsPrecisely",
                            "Shift",
                            "ShowSubtypePicker",
                            "SwitchToPrevSubtype",
                            "SwitchToNextSubtype",
                            "SwitchToClipboardContext",
                            "SwitchToMediaContext",
                            "SwitchToEditingContext",
                            "ToggleOneHandedMode",
                            "ToggleSmartbarVisibility",
                            "Undo",
                        ],
                    ),
                )?;
                map.serialize_entry(
                    "deleteKeyLongPress",
                    &enum_json(
                        s.delete_key_long_press().0,
                        &[
                            "NoAction",
                            "CycleToPreviousKeyboardMode",
                            "CycleToNextKeyboardMode",
                            "DeleteCharacter",
                            "DeleteCharactersPrecisely",
                            "DeleteWord",
                            "DeleteWordsPrecisely",
                            "HideKeyboard",
                            "InsertSpace",
                            "MoveCursorUp",
                            "MoveCursorDown",
                            "MoveCursorLeft",
                            "MoveCursorRight",
                            "MoveCursorStartOfLine",
                            "MoveCursorEndOfLine",
                            "MoveCursorStartOfPage",
                            "MoveCursorEndOfPage",
                            "Redo",
                            "SelectCharactersPrecisely",
                            "SelectWordsPrecisely",
                            "Shift",
                            "ShowSubtypePicker",
                            "SwitchToPrevSubtype",
                            "SwitchToNextSubtype",
                            "SwitchToClipboardContext",
                            "SwitchToMediaContext",
                            "SwitchToEditingContext",
                            "ToggleOneHandedMode",
                            "ToggleSmartbarVisibility",
                            "Undo",
                        ],
                    ),
                )?;
                map.serialize_entry("hapticFeedback", &s.haptic_feedback())?;
                map.serialize_entry("hapticDuration", &s.haptic_duration())?;
                map.serialize_entry("soundFeedback", &s.sound_feedback())?;
                if !s.sound_volume().is_finite() {
                    return Err(serde::ser::Error::custom("Non-finite number"));
                }
                map.serialize_entry("soundVolume", &s.sound_volume())?;
                map.serialize_entry("smartbarEnabled", &s.smartbar_enabled())?;
                map.serialize_entry(
                    "smartbarLayout",
                    &enum_json(
                        s.smartbar_layout().0,
                        &["SuggestionsWithActions", "SuggestionsOnly", "ActionsOnly"],
                    ),
                )?;
                map.serialize_entry(
                    "themeMode",
                    &enum_json(s.theme_mode().0, &["FollowSystem", "Light", "Dark"]),
                )?;
                map.serialize_entry("lightTheme", &s.light_theme())?;
                map.serialize_entry("darkTheme", &s.dark_theme())?;
                map.serialize_entry(
                    "emojiSkinTone",
                    &enum_json(
                        s.emoji_skin_tone().0,
                        &[
                            "Default",
                            "Light",
                            "MediumLight",
                            "Medium",
                            "MediumDark",
                            "Dark",
                        ],
                    ),
                )?;
                map.serialize_entry("emojiHistoryEnabled", &s.emoji_history_enabled())?;
                map.serialize_entry("clipboardHistoryEnabled", &s.clipboard_history_enabled())?;
                map.serialize_entry("clipboardSuggestion", &s.clipboard_suggestion())?;
                map.end()
            }
        }
        serde_json::to_string_pretty(&View(self))
    }
    pub fn deep_clone(&self) -> Result<Self, serde_json::Error> {
        Self::from_json(&self.to_json()?)
    }
    pub fn copy_from(&self, other: &Self) -> Result<(), serde_json::Error> {
        let clone = other.deep_clone()?;
        self.set_subtypes(Some(clone.subtypes()));
        self.set_number_row(clone.number_row());
        self.set_hinted_number_row(clone.hinted_number_row());
        self.set_hinted_symbols(clone.hinted_symbols());
        self.set_hint_mode(clone.hint_mode());
        self.set_utility_key_action(clone.utility_key_action());
        self.set_space_bar_mode(clone.space_bar_mode());
        self.set_key_height(clone.key_height());
        self.set_height_scale(clone.height_scale());
        self.set_key_spacing_horizontal(clone.key_spacing_horizontal());
        self.set_key_spacing_vertical(clone.key_spacing_vertical());
        self.set_font_scale(clone.font_scale());
        self.set_max_keyboard_width(clone.max_keyboard_width());
        self.set_one_handed_mode(clone.one_handed_mode());
        self.set_one_handed_width(clone.one_handed_width());
        self.set_split_keyboard(clone.split_keyboard());
        self.set_split_gap_ratio(clone.split_gap_ratio());
        self.set_floating(clone.floating());
        self.set_floating_width_ratio(clone.floating_width_ratio());
        self.set_show_key_preview(clone.show_key_preview());
        self.set_long_press_delay(clone.long_press_delay());
        self.set_key_repeat_delay(clone.key_repeat_delay());
        self.set_key_repeat_interval(clone.key_repeat_interval());
        self.set_double_tap_shift_for_caps_lock(clone.double_tap_shift_for_caps_lock());
        self.set_double_tap_delay(clone.double_tap_delay());
        self.set_auto_capitalization(clone.auto_capitalization());
        self.set_double_space_period(clone.double_space_period());
        self.set_auto_space_punctuation(clone.auto_space_punctuation());
        self.set_show_suggestions(clone.show_suggestions());
        self.set_auto_correct(clone.auto_correct());
        self.set_next_word_prediction(clone.next_word_prediction());
        self.set_learn_words(clone.learn_words());
        self.set_emoji_suggestions(clone.emoji_suggestions());
        self.set_suggestion_count(clone.suggestion_count());
        self.set_undo_auto_correct_on_backspace(clone.undo_auto_correct_on_backspace());
        self.set_incognito_mode(clone.incognito_mode());
        self.set_glide_typing(clone.glide_typing());
        self.set_glide_show_trail(clone.glide_show_trail());
        self.set_glide_trail_duration(clone.glide_trail_duration());
        self.set_glide_preview(clone.glide_preview());
        self.set_swipe_distance_threshold(clone.swipe_distance_threshold());
        self.set_swipe_up(clone.swipe_up());
        self.set_swipe_down(clone.swipe_down());
        self.set_swipe_left(clone.swipe_left());
        self.set_swipe_right(clone.swipe_right());
        self.set_space_bar_swipe_up(clone.space_bar_swipe_up());
        self.set_space_bar_swipe_left(clone.space_bar_swipe_left());
        self.set_space_bar_swipe_right(clone.space_bar_swipe_right());
        self.set_space_bar_long_press(clone.space_bar_long_press());
        self.set_delete_key_swipe_left(clone.delete_key_swipe_left());
        self.set_delete_key_long_press(clone.delete_key_long_press());
        self.set_haptic_feedback(clone.haptic_feedback());
        self.set_haptic_duration(clone.haptic_duration());
        self.set_sound_feedback(clone.sound_feedback());
        self.set_sound_volume(clone.sound_volume());
        self.set_smartbar_enabled(clone.smartbar_enabled());
        self.set_smartbar_layout(clone.smartbar_layout());
        self.set_theme_mode(clone.theme_mode());
        self.set_light_theme(Some(clone.light_theme()));
        self.set_dark_theme(Some(clone.dark_theme()));
        self.set_emoji_skin_tone(clone.emoji_skin_tone());
        self.set_emoji_history_enabled(clone.emoji_history_enabled());
        self.set_clipboard_history_enabled(clone.clipboard_history_enabled());
        self.set_clipboard_suggestion(clone.clipboard_suggestion());
        Ok(())
    }
}
impl<'de> Deserialize<'de> for KeyboardSettings {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct SettingsVisitor;
        impl<'de> Visitor<'de> for SettingsVisitor {
            type Value = KeyboardSettings;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("settings object")
            }
            fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<Self::Value, M::Error> {
                let settings = KeyboardSettings::default();
                while let Some(name) = map.next_key::<String>()? {
                    match name.as_str() {
                        "subtypes" => settings.set_subtypes(
                            map.next_value::<Option<Vec<Option<String>>>>()?
                                .map(|v| Rc::new(RefCell::new(v))),
                        ),
                        "numberRow" => settings.set_number_row(map.next_value()?),
                        "hintedNumberRow" => settings.set_hinted_number_row(map.next_value()?),
                        "hintedSymbols" => settings.set_hinted_symbols(map.next_value()?),
                        "hintMode" => settings.set_hint_mode(KeyHintMode(
                            parse_enum(
                                map.next_value()?,
                                &["Disabled", "AccentPriority", "HintPriority"],
                            )
                            .map_err(de::Error::custom)?,
                        )),
                        "utilityKeyAction" => settings.set_utility_key_action(UtilityKeyAction(
                            parse_enum(
                                map.next_value()?,
                                &["Emoji", "LanguageSwitch", "Dynamic", "Hidden"],
                            )
                            .map_err(de::Error::custom)?,
                        )),
                        "spaceBarMode" => settings.set_space_bar_mode(SpaceBarMode(
                            parse_enum(
                                map.next_value()?,
                                &["Nothing", "CurrentLanguage", "SpaceBarKey"],
                            )
                            .map_err(de::Error::custom)?,
                        )),
                        "keyHeight" => settings.set_key_height(map.next_value()?),
                        "heightScale" => settings.set_height_scale(map.next_value()?),
                        "keySpacingHorizontal" => {
                            settings.set_key_spacing_horizontal(map.next_value()?)
                        }
                        "keySpacingVertical" => {
                            settings.set_key_spacing_vertical(map.next_value()?)
                        }
                        "fontScale" => settings.set_font_scale(map.next_value()?),
                        "maxKeyboardWidth" => settings.set_max_keyboard_width(map.next_value()?),
                        "oneHandedMode" => settings.set_one_handed_mode(OneHandedMode(
                            parse_enum(map.next_value()?, &["Off", "Left", "Right"])
                                .map_err(de::Error::custom)?,
                        )),
                        "oneHandedWidth" => settings.set_one_handed_width(map.next_value()?),
                        "splitKeyboard" => settings.set_split_keyboard(map.next_value()?),
                        "splitGapRatio" => settings.set_split_gap_ratio(map.next_value()?),
                        "floating" => settings.set_floating(map.next_value()?),
                        "floatingWidthRatio" => {
                            settings.set_floating_width_ratio(map.next_value()?)
                        }
                        "showKeyPreview" => settings.set_show_key_preview(map.next_value()?),
                        "longPressDelay" => settings.set_long_press_delay(map.next_value()?),
                        "keyRepeatDelay" => settings.set_key_repeat_delay(map.next_value()?),
                        "keyRepeatInterval" => settings.set_key_repeat_interval(map.next_value()?),
                        "doubleTapShiftForCapsLock" => {
                            settings.set_double_tap_shift_for_caps_lock(map.next_value()?)
                        }
                        "doubleTapDelay" => settings.set_double_tap_delay(map.next_value()?),
                        "autoCapitalization" => settings.set_auto_capitalization(map.next_value()?),
                        "doubleSpacePeriod" => settings.set_double_space_period(map.next_value()?),
                        "autoSpacePunctuation" => {
                            settings.set_auto_space_punctuation(map.next_value()?)
                        }
                        "showSuggestions" => settings.set_show_suggestions(map.next_value()?),
                        "autoCorrect" => settings.set_auto_correct(map.next_value()?),
                        "nextWordPrediction" => {
                            settings.set_next_word_prediction(map.next_value()?)
                        }
                        "learnWords" => settings.set_learn_words(map.next_value()?),
                        "emojiSuggestions" => settings.set_emoji_suggestions(map.next_value()?),
                        "suggestionCount" => settings.set_suggestion_count(map.next_value()?),
                        "undoAutoCorrectOnBackspace" => {
                            settings.set_undo_auto_correct_on_backspace(map.next_value()?)
                        }
                        "incognitoMode" => settings.set_incognito_mode(map.next_value()?),
                        "glideTyping" => settings.set_glide_typing(map.next_value()?),
                        "glideShowTrail" => settings.set_glide_show_trail(map.next_value()?),
                        "glideTrailDuration" => {
                            settings.set_glide_trail_duration(map.next_value()?)
                        }
                        "glidePreview" => settings.set_glide_preview(map.next_value()?),
                        "swipeDistanceThreshold" => {
                            settings.set_swipe_distance_threshold(map.next_value()?)
                        }
                        "swipeUp" => settings.set_swipe_up(SwipeAction(
                            parse_enum(
                                map.next_value()?,
                                &[
                                    "NoAction",
                                    "CycleToPreviousKeyboardMode",
                                    "CycleToNextKeyboardMode",
                                    "DeleteCharacter",
                                    "DeleteCharactersPrecisely",
                                    "DeleteWord",
                                    "DeleteWordsPrecisely",
                                    "HideKeyboard",
                                    "InsertSpace",
                                    "MoveCursorUp",
                                    "MoveCursorDown",
                                    "MoveCursorLeft",
                                    "MoveCursorRight",
                                    "MoveCursorStartOfLine",
                                    "MoveCursorEndOfLine",
                                    "MoveCursorStartOfPage",
                                    "MoveCursorEndOfPage",
                                    "Redo",
                                    "SelectCharactersPrecisely",
                                    "SelectWordsPrecisely",
                                    "Shift",
                                    "ShowSubtypePicker",
                                    "SwitchToPrevSubtype",
                                    "SwitchToNextSubtype",
                                    "SwitchToClipboardContext",
                                    "SwitchToMediaContext",
                                    "SwitchToEditingContext",
                                    "ToggleOneHandedMode",
                                    "ToggleSmartbarVisibility",
                                    "Undo",
                                ],
                            )
                            .map_err(de::Error::custom)?,
                        )),
                        "swipeDown" => settings.set_swipe_down(SwipeAction(
                            parse_enum(
                                map.next_value()?,
                                &[
                                    "NoAction",
                                    "CycleToPreviousKeyboardMode",
                                    "CycleToNextKeyboardMode",
                                    "DeleteCharacter",
                                    "DeleteCharactersPrecisely",
                                    "DeleteWord",
                                    "DeleteWordsPrecisely",
                                    "HideKeyboard",
                                    "InsertSpace",
                                    "MoveCursorUp",
                                    "MoveCursorDown",
                                    "MoveCursorLeft",
                                    "MoveCursorRight",
                                    "MoveCursorStartOfLine",
                                    "MoveCursorEndOfLine",
                                    "MoveCursorStartOfPage",
                                    "MoveCursorEndOfPage",
                                    "Redo",
                                    "SelectCharactersPrecisely",
                                    "SelectWordsPrecisely",
                                    "Shift",
                                    "ShowSubtypePicker",
                                    "SwitchToPrevSubtype",
                                    "SwitchToNextSubtype",
                                    "SwitchToClipboardContext",
                                    "SwitchToMediaContext",
                                    "SwitchToEditingContext",
                                    "ToggleOneHandedMode",
                                    "ToggleSmartbarVisibility",
                                    "Undo",
                                ],
                            )
                            .map_err(de::Error::custom)?,
                        )),
                        "swipeLeft" => settings.set_swipe_left(SwipeAction(
                            parse_enum(
                                map.next_value()?,
                                &[
                                    "NoAction",
                                    "CycleToPreviousKeyboardMode",
                                    "CycleToNextKeyboardMode",
                                    "DeleteCharacter",
                                    "DeleteCharactersPrecisely",
                                    "DeleteWord",
                                    "DeleteWordsPrecisely",
                                    "HideKeyboard",
                                    "InsertSpace",
                                    "MoveCursorUp",
                                    "MoveCursorDown",
                                    "MoveCursorLeft",
                                    "MoveCursorRight",
                                    "MoveCursorStartOfLine",
                                    "MoveCursorEndOfLine",
                                    "MoveCursorStartOfPage",
                                    "MoveCursorEndOfPage",
                                    "Redo",
                                    "SelectCharactersPrecisely",
                                    "SelectWordsPrecisely",
                                    "Shift",
                                    "ShowSubtypePicker",
                                    "SwitchToPrevSubtype",
                                    "SwitchToNextSubtype",
                                    "SwitchToClipboardContext",
                                    "SwitchToMediaContext",
                                    "SwitchToEditingContext",
                                    "ToggleOneHandedMode",
                                    "ToggleSmartbarVisibility",
                                    "Undo",
                                ],
                            )
                            .map_err(de::Error::custom)?,
                        )),
                        "swipeRight" => settings.set_swipe_right(SwipeAction(
                            parse_enum(
                                map.next_value()?,
                                &[
                                    "NoAction",
                                    "CycleToPreviousKeyboardMode",
                                    "CycleToNextKeyboardMode",
                                    "DeleteCharacter",
                                    "DeleteCharactersPrecisely",
                                    "DeleteWord",
                                    "DeleteWordsPrecisely",
                                    "HideKeyboard",
                                    "InsertSpace",
                                    "MoveCursorUp",
                                    "MoveCursorDown",
                                    "MoveCursorLeft",
                                    "MoveCursorRight",
                                    "MoveCursorStartOfLine",
                                    "MoveCursorEndOfLine",
                                    "MoveCursorStartOfPage",
                                    "MoveCursorEndOfPage",
                                    "Redo",
                                    "SelectCharactersPrecisely",
                                    "SelectWordsPrecisely",
                                    "Shift",
                                    "ShowSubtypePicker",
                                    "SwitchToPrevSubtype",
                                    "SwitchToNextSubtype",
                                    "SwitchToClipboardContext",
                                    "SwitchToMediaContext",
                                    "SwitchToEditingContext",
                                    "ToggleOneHandedMode",
                                    "ToggleSmartbarVisibility",
                                    "Undo",
                                ],
                            )
                            .map_err(de::Error::custom)?,
                        )),
                        "spaceBarSwipeUp" => settings.set_space_bar_swipe_up(SwipeAction(
                            parse_enum(
                                map.next_value()?,
                                &[
                                    "NoAction",
                                    "CycleToPreviousKeyboardMode",
                                    "CycleToNextKeyboardMode",
                                    "DeleteCharacter",
                                    "DeleteCharactersPrecisely",
                                    "DeleteWord",
                                    "DeleteWordsPrecisely",
                                    "HideKeyboard",
                                    "InsertSpace",
                                    "MoveCursorUp",
                                    "MoveCursorDown",
                                    "MoveCursorLeft",
                                    "MoveCursorRight",
                                    "MoveCursorStartOfLine",
                                    "MoveCursorEndOfLine",
                                    "MoveCursorStartOfPage",
                                    "MoveCursorEndOfPage",
                                    "Redo",
                                    "SelectCharactersPrecisely",
                                    "SelectWordsPrecisely",
                                    "Shift",
                                    "ShowSubtypePicker",
                                    "SwitchToPrevSubtype",
                                    "SwitchToNextSubtype",
                                    "SwitchToClipboardContext",
                                    "SwitchToMediaContext",
                                    "SwitchToEditingContext",
                                    "ToggleOneHandedMode",
                                    "ToggleSmartbarVisibility",
                                    "Undo",
                                ],
                            )
                            .map_err(de::Error::custom)?,
                        )),
                        "spaceBarSwipeLeft" => settings.set_space_bar_swipe_left(SwipeAction(
                            parse_enum(
                                map.next_value()?,
                                &[
                                    "NoAction",
                                    "CycleToPreviousKeyboardMode",
                                    "CycleToNextKeyboardMode",
                                    "DeleteCharacter",
                                    "DeleteCharactersPrecisely",
                                    "DeleteWord",
                                    "DeleteWordsPrecisely",
                                    "HideKeyboard",
                                    "InsertSpace",
                                    "MoveCursorUp",
                                    "MoveCursorDown",
                                    "MoveCursorLeft",
                                    "MoveCursorRight",
                                    "MoveCursorStartOfLine",
                                    "MoveCursorEndOfLine",
                                    "MoveCursorStartOfPage",
                                    "MoveCursorEndOfPage",
                                    "Redo",
                                    "SelectCharactersPrecisely",
                                    "SelectWordsPrecisely",
                                    "Shift",
                                    "ShowSubtypePicker",
                                    "SwitchToPrevSubtype",
                                    "SwitchToNextSubtype",
                                    "SwitchToClipboardContext",
                                    "SwitchToMediaContext",
                                    "SwitchToEditingContext",
                                    "ToggleOneHandedMode",
                                    "ToggleSmartbarVisibility",
                                    "Undo",
                                ],
                            )
                            .map_err(de::Error::custom)?,
                        )),
                        "spaceBarSwipeRight" => settings.set_space_bar_swipe_right(SwipeAction(
                            parse_enum(
                                map.next_value()?,
                                &[
                                    "NoAction",
                                    "CycleToPreviousKeyboardMode",
                                    "CycleToNextKeyboardMode",
                                    "DeleteCharacter",
                                    "DeleteCharactersPrecisely",
                                    "DeleteWord",
                                    "DeleteWordsPrecisely",
                                    "HideKeyboard",
                                    "InsertSpace",
                                    "MoveCursorUp",
                                    "MoveCursorDown",
                                    "MoveCursorLeft",
                                    "MoveCursorRight",
                                    "MoveCursorStartOfLine",
                                    "MoveCursorEndOfLine",
                                    "MoveCursorStartOfPage",
                                    "MoveCursorEndOfPage",
                                    "Redo",
                                    "SelectCharactersPrecisely",
                                    "SelectWordsPrecisely",
                                    "Shift",
                                    "ShowSubtypePicker",
                                    "SwitchToPrevSubtype",
                                    "SwitchToNextSubtype",
                                    "SwitchToClipboardContext",
                                    "SwitchToMediaContext",
                                    "SwitchToEditingContext",
                                    "ToggleOneHandedMode",
                                    "ToggleSmartbarVisibility",
                                    "Undo",
                                ],
                            )
                            .map_err(de::Error::custom)?,
                        )),
                        "spaceBarLongPress" => settings.set_space_bar_long_press(SwipeAction(
                            parse_enum(
                                map.next_value()?,
                                &[
                                    "NoAction",
                                    "CycleToPreviousKeyboardMode",
                                    "CycleToNextKeyboardMode",
                                    "DeleteCharacter",
                                    "DeleteCharactersPrecisely",
                                    "DeleteWord",
                                    "DeleteWordsPrecisely",
                                    "HideKeyboard",
                                    "InsertSpace",
                                    "MoveCursorUp",
                                    "MoveCursorDown",
                                    "MoveCursorLeft",
                                    "MoveCursorRight",
                                    "MoveCursorStartOfLine",
                                    "MoveCursorEndOfLine",
                                    "MoveCursorStartOfPage",
                                    "MoveCursorEndOfPage",
                                    "Redo",
                                    "SelectCharactersPrecisely",
                                    "SelectWordsPrecisely",
                                    "Shift",
                                    "ShowSubtypePicker",
                                    "SwitchToPrevSubtype",
                                    "SwitchToNextSubtype",
                                    "SwitchToClipboardContext",
                                    "SwitchToMediaContext",
                                    "SwitchToEditingContext",
                                    "ToggleOneHandedMode",
                                    "ToggleSmartbarVisibility",
                                    "Undo",
                                ],
                            )
                            .map_err(de::Error::custom)?,
                        )),
                        "deleteKeySwipeLeft" => settings.set_delete_key_swipe_left(SwipeAction(
                            parse_enum(
                                map.next_value()?,
                                &[
                                    "NoAction",
                                    "CycleToPreviousKeyboardMode",
                                    "CycleToNextKeyboardMode",
                                    "DeleteCharacter",
                                    "DeleteCharactersPrecisely",
                                    "DeleteWord",
                                    "DeleteWordsPrecisely",
                                    "HideKeyboard",
                                    "InsertSpace",
                                    "MoveCursorUp",
                                    "MoveCursorDown",
                                    "MoveCursorLeft",
                                    "MoveCursorRight",
                                    "MoveCursorStartOfLine",
                                    "MoveCursorEndOfLine",
                                    "MoveCursorStartOfPage",
                                    "MoveCursorEndOfPage",
                                    "Redo",
                                    "SelectCharactersPrecisely",
                                    "SelectWordsPrecisely",
                                    "Shift",
                                    "ShowSubtypePicker",
                                    "SwitchToPrevSubtype",
                                    "SwitchToNextSubtype",
                                    "SwitchToClipboardContext",
                                    "SwitchToMediaContext",
                                    "SwitchToEditingContext",
                                    "ToggleOneHandedMode",
                                    "ToggleSmartbarVisibility",
                                    "Undo",
                                ],
                            )
                            .map_err(de::Error::custom)?,
                        )),
                        "deleteKeyLongPress" => settings.set_delete_key_long_press(SwipeAction(
                            parse_enum(
                                map.next_value()?,
                                &[
                                    "NoAction",
                                    "CycleToPreviousKeyboardMode",
                                    "CycleToNextKeyboardMode",
                                    "DeleteCharacter",
                                    "DeleteCharactersPrecisely",
                                    "DeleteWord",
                                    "DeleteWordsPrecisely",
                                    "HideKeyboard",
                                    "InsertSpace",
                                    "MoveCursorUp",
                                    "MoveCursorDown",
                                    "MoveCursorLeft",
                                    "MoveCursorRight",
                                    "MoveCursorStartOfLine",
                                    "MoveCursorEndOfLine",
                                    "MoveCursorStartOfPage",
                                    "MoveCursorEndOfPage",
                                    "Redo",
                                    "SelectCharactersPrecisely",
                                    "SelectWordsPrecisely",
                                    "Shift",
                                    "ShowSubtypePicker",
                                    "SwitchToPrevSubtype",
                                    "SwitchToNextSubtype",
                                    "SwitchToClipboardContext",
                                    "SwitchToMediaContext",
                                    "SwitchToEditingContext",
                                    "ToggleOneHandedMode",
                                    "ToggleSmartbarVisibility",
                                    "Undo",
                                ],
                            )
                            .map_err(de::Error::custom)?,
                        )),
                        "hapticFeedback" => settings.set_haptic_feedback(map.next_value()?),
                        "hapticDuration" => settings.set_haptic_duration(map.next_value()?),
                        "soundFeedback" => settings.set_sound_feedback(map.next_value()?),
                        "soundVolume" => settings.set_sound_volume(map.next_value()?),
                        "smartbarEnabled" => settings.set_smartbar_enabled(map.next_value()?),
                        "smartbarLayout" => settings.set_smartbar_layout(SmartbarLayout(
                            parse_enum(
                                map.next_value()?,
                                &["SuggestionsWithActions", "SuggestionsOnly", "ActionsOnly"],
                            )
                            .map_err(de::Error::custom)?,
                        )),
                        "themeMode" => settings.set_theme_mode(ThemeMode(
                            parse_enum(map.next_value()?, &["FollowSystem", "Light", "Dark"])
                                .map_err(de::Error::custom)?,
                        )),
                        "lightTheme" => settings.set_light_theme(map.next_value()?),
                        "darkTheme" => settings.set_dark_theme(map.next_value()?),
                        "emojiSkinTone" => settings.set_emoji_skin_tone(EmojiSkinTone(
                            parse_enum(
                                map.next_value()?,
                                &[
                                    "Default",
                                    "Light",
                                    "MediumLight",
                                    "Medium",
                                    "MediumDark",
                                    "Dark",
                                ],
                            )
                            .map_err(de::Error::custom)?,
                        )),
                        "emojiHistoryEnabled" => {
                            settings.set_emoji_history_enabled(map.next_value()?)
                        }
                        "clipboardHistoryEnabled" => {
                            settings.set_clipboard_history_enabled(map.next_value()?)
                        }
                        "clipboardSuggestion" => {
                            settings.set_clipboard_suggestion(map.next_value()?)
                        }
                        _ => {
                            let _: de::IgnoredAny = map.next_value()?;
                        }
                    }
                }
                Ok(settings)
            }
        }
        deserializer.deserialize_map(SettingsVisitor)
    }
}
fn enum_json(value: i32, names: &[&str]) -> Value {
    names
        .get(value as usize)
        .map_or(Value::from(value), |n| Value::from(*n))
}
fn parse_enum(value: Value, names: &[&str]) -> Result<i32, &'static str> {
    match value {
        Value::Number(n) => n
            .as_i64()
            .and_then(|v| i32::try_from(v).ok())
            .ok_or("Invalid enum number"),
        Value::String(s) => {
            let s = s.trim();
            if let Ok(value) = s.parse::<i32>() {
                return Ok(value);
            }
            let mut result = 0;
            for part in s.split(',') {
                let value = names
                    .iter()
                    .position(|n| n.eq_ignore_ascii_case(part.trim()))
                    .ok_or("Invalid enum name")?;
                result |= value as i32;
            }
            Ok(result)
        }
        _ => Err("Expected enum string or number"),
    }
}

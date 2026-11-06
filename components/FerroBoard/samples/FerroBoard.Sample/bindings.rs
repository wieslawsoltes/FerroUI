//! Native binding adapter around original framework-independent ReactiveUI presentation state.
use crate::{native_command::NativeCommand, view_model::MainViewModel};
use ferroboard_core::settings::KeyboardSettings;
use ferroui_base::{
    data::core::ValueTypes,
    input::ICommand,
    metadata::{MarkupType, MarkupTyped},
};
use ferroui_system::{component_model::INotifyPropertyChanged, events::Event};
use std::rc::Rc;
pub struct SampleBindings {
    model: Rc<MainViewModel>,
    languages: Rc<crate::language_bindings::LanguagePanelBindings>,
    reset: Rc<dyn ICommand>,
    clear: Rc<dyn ICommand>,
    light_theme_items: ferroui_controls::ItemsSource,
    dark_theme_items: ferroui_controls::ItemsSource,
}
impl SampleBindings {
    pub fn new(
        model: Rc<MainViewModel>,
        languages: Rc<crate::language_bindings::LanguagePanelBindings>,
    ) -> Result<Rc<Self>, std::string::FromUtf16Error> {
        let reset = NativeCommand::wrap(model.reset_settings());
        let clear = NativeCommand::wrap(model.clear_learned_words());
        let names = model
            .themes()
            .items()
            .iter()
            .map(|t| {
                t.name
                    .as_ref()
                    .map(ferroboard_core::text::Utf16Text::to_utf8)
                    .transpose()
            })
            .collect::<Result<Vec<_>, _>>()?;
        let light_theme_items = ferroui_controls::ItemsSource::from_items(
            names
                .iter()
                .cloned()
                .map(ferroui_base::metadata::into_markup_value),
        );
        let dark_theme_items = ferroui_controls::ItemsSource::from_items(
            names
                .into_iter()
                .map(ferroui_base::metadata::into_markup_value),
        );
        Ok(Rc::new(Self {
            light_theme_items,
            dark_theme_items,
            model,
            languages,
            reset,
            clear,
        }))
    }
    pub fn languages(&self) -> Rc<crate::language_bindings::LanguagePanelBindings> {
        self.languages.clone()
    }
    pub fn model(&self) -> Rc<MainViewModel> {
        self.model.clone()
    }
    pub fn light_theme_items(&self) -> ferroui_controls::ItemsSource {
        self.light_theme_items.clone()
    }
    pub fn dark_theme_items(&self) -> ferroui_controls::ItemsSource {
        self.dark_theme_items.clone()
    }
    pub fn light_theme_index(&self) -> i32 {
        self.model.light_theme_index()
    }
    pub fn set_light_theme_index(&self, index: i32) {
        self.model.set_light_theme_index(index);
    }
    pub fn dark_theme_index(&self) -> i32 {
        self.model.dark_theme_index()
    }
    pub fn set_dark_theme_index(&self, index: i32) {
        self.model.set_dark_theme_index(index);
    }
    pub fn settings(&self) -> Rc<KeyboardSettings> {
        self.model.settings()
    }
    pub fn message(&self) -> Option<String> {
        self.model.message()
    }
    pub fn set_message(&self, v: Option<String>) {
        self.model.set_message(v);
    }
    pub fn status(&self) -> Option<String> {
        self.model.status()
    }
    pub fn reset_settings(&self) -> Rc<dyn ICommand> {
        self.reset.clone()
    }
    pub fn clear_learned_words(&self) -> Rc<dyn ICommand> {
        self.clear.clone()
    }
    pub fn settings_pane_open(&self) -> bool {
        self.model.settings_pane_open()
    }
    pub fn set_settings_pane_open(&self, value: bool) {
        self.model.set_settings_pane_open(value);
    }
    pub fn settings_display_mode(&self) -> ferroui_controls::SplitViewDisplayMode {
        if self.model.narrow() {
            ferroui_controls::SplitViewDisplayMode::Overlay
        } else {
            ferroui_controls::SplitViewDisplayMode::Inline
        }
    }
    pub fn key_height_value(&self) -> f64 {
        self.model.key_height_value()
    }
    pub fn set_key_height_value(&self, value: f64) {
        self.model.set_key_height_value(value);
    }
    pub fn font_scale_value(&self) -> f64 {
        self.model.font_scale_value()
    }
    pub fn set_font_scale_value(&self, value: f64) {
        self.model.set_font_scale_value(value);
    }
    pub fn key_spacing_horizontal_value(&self) -> f64 {
        self.model.key_spacing_horizontal_value()
    }
    pub fn set_key_spacing_horizontal_value(&self, value: f64) {
        self.model.set_key_spacing_horizontal_value(value);
    }
    pub fn key_spacing_vertical_value(&self) -> f64 {
        self.model.key_spacing_vertical_value()
    }
    pub fn set_key_spacing_vertical_value(&self, value: f64) {
        self.model.set_key_spacing_vertical_value(value);
    }
    pub fn suggestion_count_value(&self) -> f64 {
        self.model.suggestion_count_value()
    }
    pub fn set_suggestion_count_value(&self, value: f64) {
        self.model.set_suggestion_count_value(value);
    }
    pub fn long_press_delay_value(&self) -> f64 {
        self.model.long_press_delay_value()
    }
    pub fn set_long_press_delay_value(&self, value: f64) {
        self.model.set_long_press_delay_value(value);
    }
    pub(crate) fn notify_theme_changed(&self) {
        self.property_changed().raise("RequestedThemeVariant");
    }
    pub fn requested_theme_variant(&self) -> Option<ferroui_base::styling::ThemeVariant> {
        match self.model.settings().theme_mode().0 {
            1 => Some(ferroui_base::styling::ThemeVariant::light()),
            2 => Some(ferroui_base::styling::ThemeVariant::dark()),
            _ => None,
        }
    }
    pub fn theme_mode_index(&self) -> i32 {
        self.model.theme_mode_index()
    }
    pub fn set_theme_mode_index(&self, index: i32) {
        self.model.set_theme_mode_index(index);
    }
    pub fn one_handed_mode_index(&self) -> i32 {
        self.model.one_handed_mode_index()
    }
    pub fn set_one_handed_mode_index(&self, index: i32) {
        self.model.set_one_handed_mode_index(index);
    }
    pub fn smartbar_layout_index(&self) -> i32 {
        self.model.smartbar_layout_index()
    }
    pub fn set_smartbar_layout_index(&self, index: i32) {
        self.model.set_smartbar_layout_index(index);
    }
    pub fn space_bar_mode_index(&self) -> i32 {
        self.model.space_bar_mode_index()
    }
    pub fn set_space_bar_mode_index(&self, index: i32) {
        self.model.set_space_bar_mode_index(index);
    }
    pub fn utility_key_action_index(&self) -> i32 {
        self.model.utility_key_action_index()
    }
    pub fn set_utility_key_action_index(&self, index: i32) {
        self.model.set_utility_key_action_index(index);
    }
    pub fn hint_mode_index(&self) -> i32 {
        self.model.hint_mode_index()
    }
    pub fn set_hint_mode_index(&self, index: i32) {
        self.model.set_hint_mode_index(index);
    }
    pub fn swipe_up_index(&self) -> i32 {
        self.model.swipe_up_index()
    }
    pub fn set_swipe_up_index(&self, index: i32) {
        self.model.set_swipe_up_index(index);
    }
    pub fn swipe_down_index(&self) -> i32 {
        self.model.swipe_down_index()
    }
    pub fn set_swipe_down_index(&self, index: i32) {
        self.model.set_swipe_down_index(index);
    }
    pub fn swipe_left_index(&self) -> i32 {
        self.model.swipe_left_index()
    }
    pub fn set_swipe_left_index(&self, index: i32) {
        self.model.set_swipe_left_index(index);
    }
    pub fn swipe_right_index(&self) -> i32 {
        self.model.swipe_right_index()
    }
    pub fn set_swipe_right_index(&self, index: i32) {
        self.model.set_swipe_right_index(index);
    }
    pub fn space_bar_swipe_up_index(&self) -> i32 {
        self.model.space_bar_swipe_up_index()
    }
    pub fn set_space_bar_swipe_up_index(&self, index: i32) {
        self.model.set_space_bar_swipe_up_index(index);
    }
    pub fn space_bar_long_press_index(&self) -> i32 {
        self.model.space_bar_long_press_index()
    }
    pub fn set_space_bar_long_press_index(&self, index: i32) {
        self.model.set_space_bar_long_press_index(index);
    }
    pub fn delete_key_swipe_left_index(&self) -> i32 {
        self.model.delete_key_swipe_left_index()
    }
    pub fn set_delete_key_swipe_left_index(&self, index: i32) {
        self.model.set_delete_key_swipe_left_index(index);
    }
    pub fn emoji_skin_tone_index(&self) -> i32 {
        self.model.emoji_skin_tone_index()
    }
    pub fn set_emoji_skin_tone_index(&self, index: i32) {
        self.model.set_emoji_skin_tone_index(index);
    }
}
impl PartialEq for SampleBindings {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl INotifyPropertyChanged for SampleBindings {
    fn property_changed(&self) -> &Event<str> {
        self.model.property_changed()
    }
}
ferroui_base::ferro_markup_type!(class SampleBindings{
 namespace:"FerroBoard.Sample",this:Rc<SampleBindings>,handles:[Rc<SampleBindings>,SampleBindings],notify_property_changed:SampleBindings,
 properties:[RequestedThemeVariant:Option<ferroui_base::styling::ThemeVariant>{get:SampleBindings::requested_theme_variant},Languages:Rc<crate::language_bindings::LanguagePanelBindings>{get:SampleBindings::languages},LightThemeItems:ferroui_controls::ItemsSource{get:SampleBindings::light_theme_items},DarkThemeItems:ferroui_controls::ItemsSource{get:SampleBindings::dark_theme_items},LightThemeIndex:i32{get:SampleBindings::light_theme_index,set:SampleBindings::set_light_theme_index},DarkThemeIndex:i32{get:SampleBindings::dark_theme_index,set:SampleBindings::set_dark_theme_index},Settings:Rc<KeyboardSettings>{get:SampleBindings::settings},Message:Option<String>{get:SampleBindings::message,set:SampleBindings::set_message},Status:Option<String>{get:SampleBindings::status},ResetSettings:Rc<dyn ICommand>{get:SampleBindings::reset_settings},ClearLearnedWords:Rc<dyn ICommand>{get:SampleBindings::clear_learned_words},SettingsPaneOpen:bool{get:SampleBindings::settings_pane_open,set:SampleBindings::set_settings_pane_open},SettingsDisplayMode:ferroui_controls::SplitViewDisplayMode{get:SampleBindings::settings_display_mode},KeyHeightValue:f64{get:SampleBindings::key_height_value,set:SampleBindings::set_key_height_value},FontScaleValue:f64{get:SampleBindings::font_scale_value,set:SampleBindings::set_font_scale_value},KeySpacingHorizontalValue:f64{get:SampleBindings::key_spacing_horizontal_value,set:SampleBindings::set_key_spacing_horizontal_value},KeySpacingVerticalValue:f64{get:SampleBindings::key_spacing_vertical_value,set:SampleBindings::set_key_spacing_vertical_value},SuggestionCountValue:f64{get:SampleBindings::suggestion_count_value,set:SampleBindings::set_suggestion_count_value},LongPressDelayValue:f64{get:SampleBindings::long_press_delay_value,set:SampleBindings::set_long_press_delay_value},ThemeModeIndex:i32{get:SampleBindings::theme_mode_index,set:SampleBindings::set_theme_mode_index},OneHandedModeIndex:i32{get:SampleBindings::one_handed_mode_index,set:SampleBindings::set_one_handed_mode_index},SmartbarLayoutIndex:i32{get:SampleBindings::smartbar_layout_index,set:SampleBindings::set_smartbar_layout_index},SpaceBarModeIndex:i32{get:SampleBindings::space_bar_mode_index,set:SampleBindings::set_space_bar_mode_index},UtilityKeyActionIndex:i32{get:SampleBindings::utility_key_action_index,set:SampleBindings::set_utility_key_action_index},HintModeIndex:i32{get:SampleBindings::hint_mode_index,set:SampleBindings::set_hint_mode_index},SwipeUpIndex:i32{get:SampleBindings::swipe_up_index,set:SampleBindings::set_swipe_up_index},SwipeDownIndex:i32{get:SampleBindings::swipe_down_index,set:SampleBindings::set_swipe_down_index},SwipeLeftIndex:i32{get:SampleBindings::swipe_left_index,set:SampleBindings::set_swipe_left_index},SwipeRightIndex:i32{get:SampleBindings::swipe_right_index,set:SampleBindings::set_swipe_right_index},SpaceBarSwipeUpIndex:i32{get:SampleBindings::space_bar_swipe_up_index,set:SampleBindings::set_space_bar_swipe_up_index},SpaceBarLongPressIndex:i32{get:SampleBindings::space_bar_long_press_index,set:SampleBindings::set_space_bar_long_press_index},DeleteKeySwipeLeftIndex:i32{get:SampleBindings::delete_key_swipe_left_index,set:SampleBindings::set_delete_key_swipe_left_index},EmojiSkinToneIndex:i32{get:SampleBindings::emoji_skin_tone_index,set:SampleBindings::set_emoji_skin_tone_index}]
});
pub fn register() {
    MarkupType::register_all(&[SampleBindings::MARKUP]);
    ValueTypes::register_reference::<SampleBindings>();
}

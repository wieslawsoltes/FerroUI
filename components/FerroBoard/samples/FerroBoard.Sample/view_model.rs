//! Application presentation state, independent of controls, native focus and rendering.
use ferroboard_core::engine::LanguageModels;
use ferroboard_core::settings::KeyboardSettings;
use ferroui_reactive::concurrency::ImmediateScheduler;
use ferroui_reactiveui::{
    IReactiveObject, IReactiveObjectExt, ReactiveCommand, ReactiveObject, reactive_object,
};
use std::{cell::RefCell, rc::Rc, sync::Arc};
pub struct MainViewModel {
    reactive: ReactiveObject,
    context: Rc<crate::panel_context::SettingsPanelContext>,
    themes: crate::theme_choices::ThemeChoices,
    message: RefCell<Option<String>>,
    status: RefCell<Option<String>>,
    settings_pane_open: RefCell<bool>,
    narrow: RefCell<bool>,
    reset_settings: Rc<ReactiveCommand<(), ()>>,
    clear_learned_words: Rc<ReactiveCommand<(), ()>>,
}
reactive_object!(MainViewModel);
impl PartialEq for MainViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl MainViewModel {
    pub fn new(
        settings: Rc<KeyboardSettings>,
        models: Arc<LanguageModels>,
        themes: crate::theme_choices::ThemeChoices,
    ) -> Rc<Self> {
        Self::with_context(
            Rc::new(crate::panel_context::SettingsPanelContext::new(
                settings, models,
            )),
            themes,
        )
    }
    pub(crate) fn with_context(
        context: Rc<crate::panel_context::SettingsPanelContext>,
        themes: crate::theme_choices::ThemeChoices,
    ) -> Rc<Self> {
        Rc::new_cyclic(|weak: &std::rc::Weak<Self>| {
            let weak = weak.clone();
            let reset_context = context.clone();
            let clear_context = context.clone();
            let command = ReactiveCommand::create_with(
                move || {
                    reset_context
                        .settings()
                        .copy_from(&KeyboardSettings::new())
                        .expect("default settings serialize");
                    if let Some(vm) = weak.upgrade() {
                        vm.set_status(Some("Settings reset".into()));
                    }
                },
                None,
                Some(ImmediateScheduler::instance()),
            );
            let clear = ReactiveCommand::create_with(
                move || {
                    for (_, dictionary) in clear_context.models().user_dictionaries().iter() {
                        dictionary.clear();
                    }
                },
                None,
                Some(ImmediateScheduler::instance()),
            );
            Self {
                reactive: ReactiveObject::new(),
                context,
                themes,
                message: RefCell::new(None),
                status: RefCell::new(Some("Choose a field to start typing".into())),
                settings_pane_open: RefCell::new(true),
                narrow: RefCell::new(false),
                reset_settings: Rc::new(command),
                clear_learned_words: Rc::new(clear),
            }
        })
    }
    #[cfg(test)]
    pub(crate) fn panel_context(&self) -> Rc<crate::panel_context::SettingsPanelContext> {
        self.context.clone()
    }
    pub fn settings(&self) -> Rc<KeyboardSettings> {
        self.context.settings()
    }
    pub fn message(&self) -> Option<String> {
        self.message.borrow().clone()
    }
    pub fn set_message(&self, v: Option<String>) {
        self.raise_and_set_if_changed(&self.message, v, "Message");
    }
    pub fn enter_action_status(
        action: ferroboard_core::editor::EnterAction,
        name: Option<&str>,
    ) -> String {
        format!(
            "Enter action '{action:?}' requested by {}.",
            name.unwrap_or_default()
        )
    }
    pub fn handles_enter_action(action: ferroboard_core::editor::EnterAction) -> bool {
        matches!(
            action,
            ferroboard_core::editor::EnterAction::Go
                | ferroboard_core::editor::EnterAction::Send
                | ferroboard_core::editor::EnterAction::Search
        )
    }
    pub fn target_status(description: Option<&str>) -> String {
        description
            .map(|name| format!("Typing into: {name}"))
            .unwrap_or_default()
    }
    pub fn status(&self) -> Option<String> {
        self.status.borrow().clone()
    }
    pub fn set_status(&self, v: Option<String>) {
        self.raise_and_set_if_changed(&self.status, v, "Status");
    }
    pub fn reset_settings(&self) -> Rc<ReactiveCommand<(), ()>> {
        self.reset_settings.clone()
    }
    pub fn clear_learned_words(&self) -> Rc<ReactiveCommand<(), ()>> {
        self.clear_learned_words.clone()
    }
    pub fn settings_pane_open(&self) -> bool {
        *self.settings_pane_open.borrow()
    }
    pub fn set_settings_pane_open(&self, value: bool) {
        self.raise_and_set_if_changed(&self.settings_pane_open, value, "SettingsPaneOpen");
    }
    pub fn narrow(&self) -> bool {
        *self.narrow.borrow()
    }
    pub fn update_width(&self, width: f64, previous_width: f64) {
        self.raise_and_set_if_changed(&self.narrow, width < 900.0, "SettingsDisplayMode");
        if width < 900.0 && previous_width == 0.0 {
            self.set_settings_pane_open(false);
        }
    }
    // Source panel refreshes clamp presentation without assigning clamped values to Core.
    pub fn key_height_value(&self) -> f64 {
        (self.settings().key_height()).clamp(36.0, 90.0)
    }
    pub fn set_key_height_value(&self, value: f64) {
        if !self.context.is_refreshing() {
            self.settings().set_key_height(value);
        }
    }
    pub fn font_scale_value(&self) -> f64 {
        (self.settings().font_scale()).clamp(0.6, 1.6)
    }
    pub fn set_font_scale_value(&self, value: f64) {
        if !self.context.is_refreshing() {
            self.settings().set_font_scale(value);
        }
    }
    pub fn key_spacing_horizontal_value(&self) -> f64 {
        (self.settings().key_spacing_horizontal()).clamp(0.0, 16.0)
    }
    pub fn set_key_spacing_horizontal_value(&self, value: f64) {
        if !self.context.is_refreshing() {
            self.settings().set_key_spacing_horizontal(value);
        }
    }
    pub fn key_spacing_vertical_value(&self) -> f64 {
        (self.settings().key_spacing_vertical()).clamp(0.0, 24.0)
    }
    pub fn set_key_spacing_vertical_value(&self, value: f64) {
        if !self.context.is_refreshing() {
            self.settings().set_key_spacing_vertical(value);
        }
    }
    pub fn suggestion_count_value(&self) -> f64 {
        (self.settings().suggestion_count() as f64).clamp(1.0, 5.0)
    }
    pub fn set_suggestion_count_value(&self, value: f64) {
        if !self.context.is_refreshing() {
            self.settings().set_suggestion_count(value as i32);
        }
    }
    pub fn long_press_delay_value(&self) -> f64 {
        (self.settings().long_press_delay() as f64).clamp(150.0, 1000.0)
    }
    pub fn set_long_press_delay_value(&self, value: f64) {
        if !self.context.is_refreshing() {
            self.settings().set_long_press_delay(value as i32);
        }
    }
    pub fn refresh_presentation(&self) {
        let _guard = self.context.begin_refresh();
        self.raise_property_changed("ThemeModeIndex");
        self.raise_property_changed("LightThemeIndex");
        self.raise_property_changed("DarkThemeIndex");
        self.raise_property_changed("KeyHeightValue");
        self.raise_property_changed("FontScaleValue");
        self.raise_property_changed("KeySpacingHorizontalValue");
        self.raise_property_changed("KeySpacingVerticalValue");
        self.raise_property_changed("OneHandedModeIndex");
        self.raise_property_changed("SmartbarLayoutIndex");
        self.raise_property_changed("SpaceBarModeIndex");
        self.raise_property_changed("UtilityKeyActionIndex");
        self.raise_property_changed("HintModeIndex");
        self.raise_property_changed("SuggestionCountValue");
        self.raise_property_changed("SwipeUpIndex");
        self.raise_property_changed("SwipeDownIndex");
        self.raise_property_changed("SwipeLeftIndex");
        self.raise_property_changed("SwipeRightIndex");
        self.raise_property_changed("SpaceBarSwipeUpIndex");
        self.raise_property_changed("SpaceBarLongPressIndex");
        self.raise_property_changed("DeleteKeySwipeLeftIndex");
        self.raise_property_changed("LongPressDelayValue");
        self.raise_property_changed("EmojiSkinToneIndex");
    }
    pub fn themes(&self) -> &crate::theme_choices::ThemeChoices {
        &self.themes
    }
    pub fn light_theme_index(&self) -> i32 {
        self.themes.selected_index(
            &self.settings(),
            crate::theme_choices::ThemeProperty::LightTheme,
        )
    }
    pub fn set_light_theme_index(&self, index: i32) {
        if self.context.is_refreshing() || index < 0 {
            return;
        }
        self.themes
            .select(
                &self.settings(),
                crate::theme_choices::ThemeProperty::LightTheme,
                index,
                self.context.is_refreshing(),
            )
            .expect("source light theme choice");
    }
    pub fn dark_theme_index(&self) -> i32 {
        self.themes.selected_index(
            &self.settings(),
            crate::theme_choices::ThemeProperty::DarkTheme,
        )
    }
    pub fn set_dark_theme_index(&self, index: i32) {
        if self.context.is_refreshing() || index < 0 {
            return;
        }
        self.themes
            .select(
                &self.settings(),
                crate::theme_choices::ThemeProperty::DarkTheme,
                index,
                self.context.is_refreshing(),
            )
            .expect("source dark theme choice");
    }
    pub fn theme_mode_index(&self) -> i32 {
        crate::enum_choices::EnumChoice::ThemeMode.selected_index(&self.settings())
    }
    pub fn set_theme_mode_index(&self, index: i32) {
        if self.context.is_refreshing() || index < 0 {
            return;
        }
        crate::enum_choices::EnumChoice::ThemeMode
            .select(&self.settings(), index, self.context.is_refreshing())
            .expect("source choice index");
    }
    pub fn one_handed_mode_index(&self) -> i32 {
        crate::enum_choices::EnumChoice::OneHandedMode.selected_index(&self.settings())
    }
    pub fn set_one_handed_mode_index(&self, index: i32) {
        if self.context.is_refreshing() || index < 0 {
            return;
        }
        crate::enum_choices::EnumChoice::OneHandedMode
            .select(&self.settings(), index, self.context.is_refreshing())
            .expect("source choice index");
    }
    pub fn smartbar_layout_index(&self) -> i32 {
        crate::enum_choices::EnumChoice::SmartbarLayout.selected_index(&self.settings())
    }
    pub fn set_smartbar_layout_index(&self, index: i32) {
        if self.context.is_refreshing() || index < 0 {
            return;
        }
        crate::enum_choices::EnumChoice::SmartbarLayout
            .select(&self.settings(), index, self.context.is_refreshing())
            .expect("source choice index");
    }
    pub fn space_bar_mode_index(&self) -> i32 {
        crate::enum_choices::EnumChoice::SpaceBarMode.selected_index(&self.settings())
    }
    pub fn set_space_bar_mode_index(&self, index: i32) {
        if self.context.is_refreshing() || index < 0 {
            return;
        }
        crate::enum_choices::EnumChoice::SpaceBarMode
            .select(&self.settings(), index, self.context.is_refreshing())
            .expect("source choice index");
    }
    pub fn utility_key_action_index(&self) -> i32 {
        crate::enum_choices::EnumChoice::UtilityKeyAction.selected_index(&self.settings())
    }
    pub fn set_utility_key_action_index(&self, index: i32) {
        if self.context.is_refreshing() || index < 0 {
            return;
        }
        crate::enum_choices::EnumChoice::UtilityKeyAction
            .select(&self.settings(), index, self.context.is_refreshing())
            .expect("source choice index");
    }
    pub fn hint_mode_index(&self) -> i32 {
        crate::enum_choices::EnumChoice::HintMode.selected_index(&self.settings())
    }
    pub fn set_hint_mode_index(&self, index: i32) {
        if self.context.is_refreshing() || index < 0 {
            return;
        }
        crate::enum_choices::EnumChoice::HintMode
            .select(&self.settings(), index, self.context.is_refreshing())
            .expect("source choice index");
    }
    pub fn swipe_up_index(&self) -> i32 {
        crate::enum_choices::EnumChoice::SwipeUp.selected_index(&self.settings())
    }
    pub fn set_swipe_up_index(&self, index: i32) {
        if self.context.is_refreshing() || index < 0 {
            return;
        }
        crate::enum_choices::EnumChoice::SwipeUp
            .select(&self.settings(), index, self.context.is_refreshing())
            .expect("source choice index");
    }
    pub fn swipe_down_index(&self) -> i32 {
        crate::enum_choices::EnumChoice::SwipeDown.selected_index(&self.settings())
    }
    pub fn set_swipe_down_index(&self, index: i32) {
        if self.context.is_refreshing() || index < 0 {
            return;
        }
        crate::enum_choices::EnumChoice::SwipeDown
            .select(&self.settings(), index, self.context.is_refreshing())
            .expect("source choice index");
    }
    pub fn swipe_left_index(&self) -> i32 {
        crate::enum_choices::EnumChoice::SwipeLeft.selected_index(&self.settings())
    }
    pub fn set_swipe_left_index(&self, index: i32) {
        if self.context.is_refreshing() || index < 0 {
            return;
        }
        crate::enum_choices::EnumChoice::SwipeLeft
            .select(&self.settings(), index, self.context.is_refreshing())
            .expect("source choice index");
    }
    pub fn swipe_right_index(&self) -> i32 {
        crate::enum_choices::EnumChoice::SwipeRight.selected_index(&self.settings())
    }
    pub fn set_swipe_right_index(&self, index: i32) {
        if self.context.is_refreshing() || index < 0 {
            return;
        }
        crate::enum_choices::EnumChoice::SwipeRight
            .select(&self.settings(), index, self.context.is_refreshing())
            .expect("source choice index");
    }
    pub fn space_bar_swipe_up_index(&self) -> i32 {
        crate::enum_choices::EnumChoice::SpaceBarSwipeUp.selected_index(&self.settings())
    }
    pub fn set_space_bar_swipe_up_index(&self, index: i32) {
        if self.context.is_refreshing() || index < 0 {
            return;
        }
        crate::enum_choices::EnumChoice::SpaceBarSwipeUp
            .select(&self.settings(), index, self.context.is_refreshing())
            .expect("source choice index");
    }
    pub fn space_bar_long_press_index(&self) -> i32 {
        crate::enum_choices::EnumChoice::SpaceBarLongPress.selected_index(&self.settings())
    }
    pub fn set_space_bar_long_press_index(&self, index: i32) {
        if self.context.is_refreshing() || index < 0 {
            return;
        }
        crate::enum_choices::EnumChoice::SpaceBarLongPress
            .select(&self.settings(), index, self.context.is_refreshing())
            .expect("source choice index");
    }
    pub fn delete_key_swipe_left_index(&self) -> i32 {
        crate::enum_choices::EnumChoice::DeleteKeySwipeLeft.selected_index(&self.settings())
    }
    pub fn set_delete_key_swipe_left_index(&self, index: i32) {
        if self.context.is_refreshing() || index < 0 {
            return;
        }
        crate::enum_choices::EnumChoice::DeleteKeySwipeLeft
            .select(&self.settings(), index, self.context.is_refreshing())
            .expect("source choice index");
    }
    pub fn emoji_skin_tone_index(&self) -> i32 {
        crate::enum_choices::EnumChoice::EmojiSkinTone.selected_index(&self.settings())
    }
    pub fn set_emoji_skin_tone_index(&self, index: i32) {
        if self.context.is_refreshing() || index < 0 {
            return;
        }
        crate::enum_choices::EnumChoice::EmojiSkinTone
            .select(&self.settings(), index, self.context.is_refreshing())
            .expect("source choice index");
    }
}

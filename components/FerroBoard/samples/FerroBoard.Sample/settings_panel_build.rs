// Source Build statement order from unchanged SettingsPanel.cs.
impl SettingsPanel {
    fn build(&self) -> Result<(), SettingsPanelError> {
        self.body().children().clear();
        self.refreshers.clear();
        self.fields.borrow_mut().clear();
        self.language.borrow_mut().take();
        self.header("Appearance")?;
        self.field(FieldKind::Enum(EnumChoice::ThemeMode))?;
        let themes = self.snapshot_themes()?;
        self.field(FieldKind::Theme {
            property: ThemeProperty::LightTheme,
            choices: themes.clone(),
        })?;
        self.field(FieldKind::Theme {
            property: ThemeProperty::DarkTheme,
            choices: themes.clone(),
        })?;
        self.field(FieldKind::Number(NumberField::KeyHeight))?;
        self.field(FieldKind::Number(NumberField::FontScale))?;
        self.field(FieldKind::Number(NumberField::KeySpacingHorizontal))?;
        self.field(FieldKind::Number(NumberField::KeySpacingVertical))?;
        self.field(FieldKind::Enum(EnumChoice::OneHandedMode))?;
        self.field(FieldKind::Boolean(BooleanField::SplitKeyboard))?;
        self.field(FieldKind::Boolean(BooleanField::NumberRow))?;
        self.field(FieldKind::Boolean(BooleanField::SmartbarEnabled))?;
        self.field(FieldKind::Enum(EnumChoice::SmartbarLayout))?;
        self.field(FieldKind::Boolean(BooleanField::ShowKeyPreview))?;
        self.field(FieldKind::Enum(EnumChoice::SpaceBarMode))?;
        self.field(FieldKind::Enum(EnumChoice::UtilityKeyAction))?;
        self.field(FieldKind::Enum(EnumChoice::HintMode))?;
        self.field(FieldKind::Boolean(BooleanField::HintedNumberRow))?;
        self.field(FieldKind::Boolean(BooleanField::HintedSymbols))?;
        self.header("Languages")?;
        self.build_languages()?;
        self.header("Typing")?;
        self.field(FieldKind::Boolean(BooleanField::AutoCapitalization))?;
        self.field(FieldKind::Boolean(BooleanField::DoubleSpacePeriod))?;
        self.field(FieldKind::Boolean(BooleanField::AutoSpacePunctuation))?;
        self.field(FieldKind::Boolean(BooleanField::ShowSuggestions))?;
        self.field(FieldKind::Boolean(BooleanField::AutoCorrect))?;
        self.field(FieldKind::Boolean(BooleanField::NextWordPrediction))?;
        self.field(FieldKind::Boolean(BooleanField::LearnWords))?;
        self.field(FieldKind::Boolean(BooleanField::EmojiSuggestions))?;
        self.field(FieldKind::Boolean(BooleanField::IncognitoMode))?;
        self.field(FieldKind::Number(NumberField::SuggestionCount))?;
        self.header("Gestures")?;
        self.field(FieldKind::Boolean(BooleanField::GlideTyping))?;
        self.field(FieldKind::Boolean(BooleanField::GlideShowTrail))?;
        self.field(FieldKind::Boolean(BooleanField::GlidePreview))?;
        self.field(FieldKind::Enum(EnumChoice::SwipeUp))?;
        self.field(FieldKind::Enum(EnumChoice::SwipeDown))?;
        self.field(FieldKind::Enum(EnumChoice::SwipeLeft))?;
        self.field(FieldKind::Enum(EnumChoice::SwipeRight))?;
        self.field(FieldKind::Enum(EnumChoice::SpaceBarSwipeUp))?;
        self.field(FieldKind::Enum(EnumChoice::SpaceBarLongPress))?;
        self.field(FieldKind::Enum(EnumChoice::DeleteKeySwipeLeft))?;
        self.field(FieldKind::Number(NumberField::LongPressDelay))?;
        self.header("Emoji, clipboard & feedback")?;
        self.field(FieldKind::Enum(EnumChoice::EmojiSkinTone))?;
        self.field(FieldKind::Boolean(BooleanField::ClipboardHistoryEnabled))?;
        self.field(FieldKind::Boolean(BooleanField::ClipboardSuggestion))?;
        self.field(FieldKind::Boolean(BooleanField::HapticFeedback))?;
        self.header("Data")?;
        self.build_data_buttons()?;
        let footer = crate::compiled_settings_views::build_views_settings_footer_xaml(Some(
            XamlIlRuntimeHelpers::create_root_service_provider_v3(None),
        ))
        .map_err(|e| SettingsPanelError::Markup(e.to_string()))?;
        self.body().children().add(footer);
        self.on_settings_changed()
    }
}

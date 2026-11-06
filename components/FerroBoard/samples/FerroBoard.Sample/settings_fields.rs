//! SettingsPanel helper contracts: cached presentation, current-target callbacks and source guards.
//! Native controls/layout are separate. See NOTICE and the unchanged SettingsPanel.cs reference.
use crate::{
    enum_choices::EnumChoice,
    panel_context::SettingsPanelContext,
    theme_choices::{ThemeChoices, ThemeProperty},
};
use ferroboard_core::{settings::KeyboardSettings, text::Utf16Text};
use ferroui_reactiveui::{IReactiveObject, ReactiveObject, reactive_object};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
#[derive(Clone, Debug, PartialEq)]
pub enum FieldValue {
    Bool(bool),
    Number(f64),
    Index(i32),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldError {
    MissingTarget,
    Index { index: i32, count: usize },
    Encoding,
    WrongValue,
    UnsupportedIntegerConversion,
    ModifiedRefreshers,
}
impl std::fmt::Display for FieldError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for FieldError {}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BooleanField {
    SplitKeyboard,
    NumberRow,
    SmartbarEnabled,
    ShowKeyPreview,
    HintedNumberRow,
    HintedSymbols,
    AutoCapitalization,
    DoubleSpacePeriod,
    AutoSpacePunctuation,
    ShowSuggestions,
    AutoCorrect,
    NextWordPrediction,
    LearnWords,
    EmojiSuggestions,
    IncognitoMode,
    GlideTyping,
    GlideShowTrail,
    GlidePreview,
    ClipboardHistoryEnabled,
    ClipboardSuggestion,
    HapticFeedback,
}
impl BooleanField {
    pub fn label(self) -> &'static str {
        match self {
            Self::SplitKeyboard => "Split keyboard",
            Self::NumberRow => "Number row",
            Self::SmartbarEnabled => "Smartbar",
            Self::ShowKeyPreview => "Key press preview",
            Self::HintedNumberRow => "Number hints",
            Self::HintedSymbols => "Symbol hints",
            Self::AutoCapitalization => "Auto-capitalization",
            Self::DoubleSpacePeriod => "Double space inserts period",
            Self::AutoSpacePunctuation => "Smart punctuation spacing",
            Self::ShowSuggestions => "Show suggestions",
            Self::AutoCorrect => "Auto-correct",
            Self::NextWordPrediction => "Next word prediction",
            Self::LearnWords => "Learn new words",
            Self::EmojiSuggestions => "Emoji suggestions",
            Self::IncognitoMode => "Incognito mode",
            Self::GlideTyping => "Glide typing",
            Self::GlideShowTrail => "Show glide trail",
            Self::GlidePreview => "Live glide preview",
            Self::ClipboardHistoryEnabled => "Clipboard history",
            Self::ClipboardSuggestion => "Suggest recently copied text",
            Self::HapticFeedback => "Haptic feedback",
        }
    }
    pub fn get(self, s: &KeyboardSettings) -> bool {
        match self {
            Self::SplitKeyboard => s.split_keyboard(),
            Self::NumberRow => s.number_row(),
            Self::SmartbarEnabled => s.smartbar_enabled(),
            Self::ShowKeyPreview => s.show_key_preview(),
            Self::HintedNumberRow => s.hinted_number_row(),
            Self::HintedSymbols => s.hinted_symbols(),
            Self::AutoCapitalization => s.auto_capitalization(),
            Self::DoubleSpacePeriod => s.double_space_period(),
            Self::AutoSpacePunctuation => s.auto_space_punctuation(),
            Self::ShowSuggestions => s.show_suggestions(),
            Self::AutoCorrect => s.auto_correct(),
            Self::NextWordPrediction => s.next_word_prediction(),
            Self::LearnWords => s.learn_words(),
            Self::EmojiSuggestions => s.emoji_suggestions(),
            Self::IncognitoMode => s.incognito_mode(),
            Self::GlideTyping => s.glide_typing(),
            Self::GlideShowTrail => s.glide_show_trail(),
            Self::GlidePreview => s.glide_preview(),
            Self::ClipboardHistoryEnabled => s.clipboard_history_enabled(),
            Self::ClipboardSuggestion => s.clipboard_suggestion(),
            Self::HapticFeedback => s.haptic_feedback(),
        }
    }
    pub fn set(self, s: &KeyboardSettings, v: bool) {
        match self {
            Self::SplitKeyboard => s.set_split_keyboard(v),
            Self::NumberRow => s.set_number_row(v),
            Self::SmartbarEnabled => s.set_smartbar_enabled(v),
            Self::ShowKeyPreview => s.set_show_key_preview(v),
            Self::HintedNumberRow => s.set_hinted_number_row(v),
            Self::HintedSymbols => s.set_hinted_symbols(v),
            Self::AutoCapitalization => s.set_auto_capitalization(v),
            Self::DoubleSpacePeriod => s.set_double_space_period(v),
            Self::AutoSpacePunctuation => s.set_auto_space_punctuation(v),
            Self::ShowSuggestions => s.set_show_suggestions(v),
            Self::AutoCorrect => s.set_auto_correct(v),
            Self::NextWordPrediction => s.set_next_word_prediction(v),
            Self::LearnWords => s.set_learn_words(v),
            Self::EmojiSuggestions => s.set_emoji_suggestions(v),
            Self::IncognitoMode => s.set_incognito_mode(v),
            Self::GlideTyping => s.set_glide_typing(v),
            Self::GlideShowTrail => s.set_glide_show_trail(v),
            Self::GlidePreview => s.set_glide_preview(v),
            Self::ClipboardHistoryEnabled => s.set_clipboard_history_enabled(v),
            Self::ClipboardSuggestion => s.set_clipboard_suggestion(v),
            Self::HapticFeedback => s.set_haptic_feedback(v),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NumberField {
    KeyHeight,
    FontScale,
    KeySpacingHorizontal,
    KeySpacingVertical,
    SuggestionCount,
    LongPressDelay,
}
impl NumberField {
    pub fn label(self) -> &'static str {
        match self {
            Self::KeyHeight => "Key height",
            Self::FontScale => "Font scale",
            Self::KeySpacingHorizontal => "Horizontal key spacing",
            Self::KeySpacingVertical => "Vertical key spacing",
            Self::SuggestionCount => "Suggestion count",
            Self::LongPressDelay => "Long press delay (ms)",
        }
    }
    pub fn range(self) -> (f64, f64, f64) {
        match self {
            Self::KeyHeight => (36.0, 90.0, 1.0),
            Self::FontScale => (0.6, 1.6, 0.05),
            Self::KeySpacingHorizontal => (0.0, 16.0, 1.0),
            Self::KeySpacingVertical => (0.0, 24.0, 1.0),
            Self::SuggestionCount => (1.0, 5.0, 1.0),
            Self::LongPressDelay => (150.0, 1000.0, 10.0),
        }
    }
    pub fn get(self, s: &KeyboardSettings) -> f64 {
        match self {
            Self::KeyHeight => s.key_height(),
            Self::FontScale => s.font_scale(),
            Self::KeySpacingHorizontal => s.key_spacing_horizontal(),
            Self::KeySpacingVertical => s.key_spacing_vertical(),
            Self::SuggestionCount => s.suggestion_count() as f64,
            Self::LongPressDelay => s.long_press_delay() as f64,
        }
    }
    pub fn set(self, s: &KeyboardSettings, v: f64) -> Result<(), FieldError> {
        match self {
            Self::KeyHeight => {
                s.set_key_height(v);
            }
            Self::FontScale => {
                s.set_font_scale(v);
            }
            Self::KeySpacingHorizontal => {
                s.set_key_spacing_horizontal(v);
            }
            Self::KeySpacingVertical => {
                s.set_key_spacing_vertical(v);
            }
            Self::SuggestionCount => {
                if !v.is_finite() || v < i32::MIN as f64 || v > i32::MAX as f64 {
                    return Err(FieldError::UnsupportedIntegerConversion);
                }
                s.set_suggestion_count(v as i32);
            }
            Self::LongPressDelay => {
                if !v.is_finite() || v < i32::MIN as f64 || v > i32::MAX as f64 {
                    return Err(FieldError::UnsupportedIntegerConversion);
                }
                s.set_long_press_delay(v as i32);
            }
        }
        Ok(())
    }
}
#[derive(Clone)]
pub enum FieldKind {
    Boolean(BooleanField),
    Number(NumberField),
    Enum(EnumChoice),
    Theme {
        property: ThemeProperty,
        choices: Rc<ThemeChoices>,
    },
}
pub struct SettingsFieldViewModel {
    reactive: ReactiveObject,
    context: Rc<SettingsPanelContext>,
    kind: FieldKind,
    presentation: RefCell<FieldValue>,
}
reactive_object!(SettingsFieldViewModel);
impl PartialEq for SettingsFieldViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl SettingsFieldViewModel {
    pub(crate) fn new(context: Rc<SettingsPanelContext>, kind: FieldKind) -> Rc<Self> {
        let presentation = match kind {
            FieldKind::Boolean(_) => FieldValue::Bool(false),
            FieldKind::Number(n) => FieldValue::Number(n.range().0),
            _ => FieldValue::Index(-1),
        };
        Rc::new(Self {
            reactive: ReactiveObject::new(),
            context,
            kind,
            presentation: RefCell::new(presentation),
        })
    }
    pub fn kind(&self) -> &FieldKind {
        &self.kind
    }
    pub fn label(&self) -> &'static str {
        match &self.kind {
            FieldKind::Boolean(b) => b.label(),
            FieldKind::Number(n) => n.label(),
            FieldKind::Enum(e) => e.label(),
            FieldKind::Theme {
                property: ThemeProperty::LightTheme,
                ..
            } => "Light theme",
            FieldKind::Theme { .. } => "Dark theme",
        }
    }
    pub fn labels(&self) -> Vec<Option<Utf16Text>> {
        match &self.kind {
            FieldKind::Enum(e) => e
                .items()
                .iter()
                .map(|(name, _)| Some((*name).into()))
                .collect(),
            FieldKind::Theme { choices, .. } => {
                choices.items().iter().map(|i| i.name.clone()).collect()
            }
            _ => Vec::new(),
        }
    }
    pub fn value(&self) -> FieldValue {
        self.presentation.borrow().clone()
    }
    pub fn refresh(&self) -> Result<(), FieldError> {
        let s = self
            .context
            .try_settings()
            .map_err(|_| FieldError::MissingTarget)?;
        let value = match &self.kind {
            FieldKind::Boolean(b) => FieldValue::Bool(b.get(&s)),
            FieldKind::Number(n) => FieldValue::Number(n.get(&s)),
            FieldKind::Enum(e) => FieldValue::Index(e.selected_index(&s)),
            FieldKind::Theme { choices, property } => {
                FieldValue::Index(choices.selected_index(&s, *property))
            }
        };
        *self.presentation.borrow_mut() = value;
        self.raise_property_changed("Value"); // Source assigns UI property on every registered refresh.
        Ok(())
    }
    pub fn dispatch(&self, value: FieldValue) -> Result<(), FieldError> {
        if self.context.is_refreshing() {
            return Ok(());
        }
        match (&self.kind, value) {
            (FieldKind::Boolean(b), FieldValue::Bool(v)) => {
                let settings = self
                    .context
                    .try_settings()
                    .map_err(|_| FieldError::MissingTarget)?;
                b.set(&settings, v);
            }
            (FieldKind::Number(n), FieldValue::Number(v)) => {
                let settings = self
                    .context
                    .try_settings()
                    .map_err(|_| FieldError::MissingTarget)?;
                n.set(&settings, v)?;
            }
            (FieldKind::Enum(e), FieldValue::Index(i)) => {
                if i < 0 {
                    return Ok(());
                }
                let (_, v) = e.items().get(i as usize).ok_or(FieldError::Index {
                    index: i,
                    count: e.items().len(),
                })?;
                let settings = self
                    .context
                    .try_settings()
                    .map_err(|_| FieldError::MissingTarget)?;
                e.set_value(&settings, *v);
            }
            (FieldKind::Theme { choices, property }, FieldValue::Index(i)) => {
                if i < 0 {
                    return Ok(());
                }
                let item = choices.items().get(i as usize).ok_or(FieldError::Index {
                    index: i,
                    count: choices.items().len(),
                })?;
                let s = self
                    .context
                    .try_settings()
                    .map_err(|_| FieldError::MissingTarget)?;
                let value = item
                    .id
                    .as_ref()
                    .map(Utf16Text::to_utf8)
                    .transpose()
                    .map_err(|_| FieldError::Encoding)?;
                property.set(&s, value);
            }
            _ => return Err(FieldError::WrongValue),
        }
        Ok(())
    }
}
/// List<Action> enumeration checks its version before each MoveNext; no borrow crosses callbacks.
pub(crate) type RefreshCallback = dyn Fn() -> Result<(), FieldError>;
pub(crate) type RefreshAction = Rc<RefreshCallback>;
pub(crate) struct SettingsRefreshers {
    actions: RefCell<Vec<RefreshAction>>,
    version: Cell<u32>,
}
impl SettingsRefreshers {
    pub fn new() -> Self {
        Self {
            actions: RefCell::new(Vec::new()),
            version: Cell::new(0),
        }
    }
    pub fn clear(&self) {
        let old = self.actions.replace(Vec::new());
        self.version.set(self.version.get().wrapping_add(1));
        drop(old);
    }
    pub fn add(&self, action: RefreshAction) {
        self.actions.borrow_mut().push(action);
        self.version.set(self.version.get().wrapping_add(1));
    }
    pub fn len(&self) -> usize {
        self.actions.borrow().len()
    }
    #[cfg(test)]
    pub(crate) fn observe_actions(&self, wrap: impl Fn(usize, RefreshAction) -> RefreshAction) {
        let actions = self.actions.borrow().clone();
        for (index, old) in actions.into_iter().enumerate() {
            let new = wrap(index, old);
            self.actions.borrow_mut()[index] = new;
            self.version.set(self.version.get().wrapping_add(1));
        }
    }
    pub fn refresh(&self, context: &SettingsPanelContext) -> Result<(), FieldError> {
        let _guard = context.begin_refresh();
        let version = self.version.get();
        let mut index = 0;
        loop {
            if version != self.version.get() {
                return Err(FieldError::ModifiedRefreshers);
            }
            if index == self.len() {
                return Ok(());
            }
            let action = self.actions.borrow()[index].clone();
            index += 1;
            action()?;
        }
    }
}

#[cfg(test)]
#[path = "settings_field_tests.rs"]
mod tests;

//! KeyboardEngine.cs root; target/edit and suggestion portions are in adjacent modules.
use super::{
    LanguageModelError, LanguageModels,
    shift_state::{EngineShiftState, ShiftEffects},
};
use crate::{
    clipboard::{
        ClipboardError, ClipboardHistory, ClipboardServiceError, ClipboardSpawner,
        IClipboardService, InMemoryClipboardService,
    },
    editor::{
        EditorClock, EditorController, EditorError, ITextInputTarget, InputAttributes, InputKind,
    },
    emoji::{EmojiCatalog, EmojiError, EmojiHistory},
    glide::{GlideError, GlideTypingClassifier},
    keys::*,
    layouts::*,
    settings::{FeedbackKind, KeyboardSettings},
    text::{
        ISuggestionProvider, NormalizationError, OrdinalKey, Suggestion, SuggestionError,
        SuggestionList, TextCulture, UnsupportedCulture, Utf16Text, ordinal_ignore_case_equal,
    },
};
use ferroui_system::{component_model::INotifyPropertyChanged, events::Event};
use std::{
    cell::{Cell, RefCell},
    collections::{HashMap, HashSet},
    rc::{Rc, Weak},
    sync::{Arc, LazyLock},
};
#[path = "keyboard_engine_functions.rs"]
mod functions;
#[path = "keyboard_engine_glide.rs"]
mod glide_session;
#[path = "keyboard_engine_input.rs"]
mod input;
#[path = "keyboard_engine_suggestions.rs"]
mod suggestions;
#[path = "keyboard_engine_target.rs"]
mod target;
#[derive(Debug)]
pub enum EngineError {
    NullReference,
    ArgumentNull(&'static str),
    Resource(ResourceError),
    Key(KeyDataError),
    Glide(GlideError),
    Language(Arc<LanguageModelError>),
    Culture(UnsupportedCulture),
    UnrepresentedText,
    NoEditor,
    Editor(EditorError),
    Emoji(EmojiError),
    Suggestion(SuggestionError),
    Normalization(NormalizationError),
    Clipboard(ClipboardError),
    ClipboardService(ClipboardServiceError),
    ClipboardRuntimeRequired,
}
pub type EngineResult<T> = Result<T, EngineError>;
pub type ClipboardOperation =
    std::pin::Pin<Box<dyn std::future::Future<Output = EngineResult<()>> + 'static>>;
impl From<ResourceError> for EngineError {
    fn from(e: ResourceError) -> Self {
        Self::Resource(e)
    }
}
impl From<KeyDataError> for EngineError {
    fn from(e: KeyDataError) -> Self {
        Self::Key(e)
    }
}
impl From<GlideError> for EngineError {
    fn from(e: GlideError) -> Self {
        Self::Glide(e)
    }
}
impl From<Arc<LanguageModelError>> for EngineError {
    fn from(e: Arc<LanguageModelError>) -> Self {
        Self::Language(e)
    }
}
/// Custom providers are dispatched synchronously each time the source calls GetProvider.
pub type SuggestionProviderFactory =
    Rc<dyn Fn(Option<&Arc<Subtype>>) -> EngineResult<Option<Rc<dyn ISuggestionProvider>>>>;
/// Core state/layout, input, suggestion and glide engine; native control integration is separate.
pub struct KeyboardEngine {
    self_ref: Weak<Self>,
    last_auto_correction: RefCell<Option<AutoCorrection>>,
    last_space_time: Cell<Option<std::time::SystemTime>>,
    double_space_window: Cell<std::time::Duration>,
    half_width: Cell<bool>,
    kana_kata: Cell<bool>,
    glide_active: Cell<bool>,
    last_glide_word: RefCell<Option<Utf16Text>>,
    glide_preview_points: Cell<i32>,
    precise_origin: Cell<Option<i32>>,
    clipboard_service: RefCell<Rc<dyn IClipboardService>>,
    clipboard_spawner: RefCell<Option<ClipboardSpawner>>,
    clipboard_failed: Event<EngineError>,
    feedback_requested: Event<FeedbackKind>,
    hide_requested: Event<Self>,
    subtype_picker_requested: Event<Self>,
    settings_requested: Event<Self>,
    resize_mode_requested: Event<Self>,
    voice_input_requested: Event<Self>,
    settings: Rc<KeyboardSettings>,
    resources: Rc<KeyboardResources>,
    language_models: Arc<LanguageModels>,
    computer: LayoutComputer,
    cache: RefCell<HashMap<(KeyboardMode, Utf16Text, bool), Rc<ComputedKeyboard>>>,
    subtypes: Rc<RefCell<Vec<Arc<Subtype>>>>,
    active: RefCell<Arc<Subtype>>,
    keyboard: RefCell<Option<Rc<ComputedKeyboard>>>,
    mode: Cell<KeyboardMode>,
    panel: Cell<KeyboardUiMode>,
    shift: EngineShiftState,
    culture: RefCell<TextCulture>,
    width: Cell<f64>,
    height: Cell<f64>,
    geometry: RefCell<Rc<KeyboardGeometryOptions>>,
    glide: Arc<GlideTypingClassifier>,
    keyboard_changed: Event<Self>,
    state_changed: Event<Self>,
    resource_token: Cell<Option<u64>>,
    settings_token: Cell<Option<u64>>,
    clipboard_token: Cell<Option<u64>>,
    target: RefCell<Option<Rc<dyn ITextInputTarget>>>,
    editor: RefCell<Option<Rc<EditorController>>>,
    target_token: Cell<Option<u64>>,
    clock: RefCell<Option<EditorClock>>,
    suppress_target_changed: Cell<bool>,
    last_insert_was_auto_space: Cell<bool>,
    suggestions: RefCell<SuggestionList>,
    providers: RefCell<HashMap<OrdinalKey, Rc<dyn ISuggestionProvider>>>,
    provider_factory: RefCell<Option<SuggestionProviderFactory>>,
    rejected_corrections: RefCell<HashSet<OrdinalKey>>,
    last_clipboard_suggestion: RefCell<Option<Utf16Text>>,
    clipboard_history: Rc<ClipboardHistory>,
    emoji_history: Rc<EmojiHistory>,
    suggestions_changed: Event<Self>,
    text_committed: Event<Utf16Text>,
}
impl PartialEq for KeyboardEngine {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl Eq for KeyboardEngine {}
impl KeyboardEngine {
    /// Creates an engine with optional native dependencies. Each omitted dependency is owned
    /// by this engine. Pass the same Rc<KeyboardResources> to share mutable layouts and caches
    /// across engines; bundled defaults deliberately do not use a process-global registry.
    pub fn new(
        settings: Option<Rc<KeyboardSettings>>,
        resources: Option<Rc<KeyboardResources>>,
        language_models: Option<Arc<LanguageModels>>,
    ) -> EngineResult<Rc<Self>> {
        let settings = settings.unwrap_or_else(|| Rc::new(KeyboardSettings::new()));
        let resources = match resources {
            Some(resources) => resources,
            None => Rc::new(KeyboardResources::create_default()?),
        };
        let language_models = language_models.unwrap_or_else(LanguageModels::new);
        Self::new_explicit(settings, resources, language_models)
    }
    /// Creates a standalone engine with bundled layouts and the default language models/settings.
    /// Defaults are local to this engine; resource loading failures remain visible to the caller.
    pub fn with_defaults() -> EngineResult<Rc<Self>> {
        Self::new(None, None, None)
    }
    /// Creates an engine using the caller's original dependency handles without cloning their data.
    pub fn new_explicit(
        settings: Rc<KeyboardSettings>,
        resources: Rc<KeyboardResources>,
        language_models: Arc<LanguageModels>,
    ) -> EngineResult<Rc<Self>> {
        let e = Rc::new_cyclic(|weak| Self {
            self_ref: weak.clone(),
            last_auto_correction: RefCell::new(None),
            last_space_time: Cell::new(None),
            double_space_window: Cell::new(std::time::Duration::from_millis(800)),
            half_width: Cell::new(false),
            kana_kata: Cell::new(false),
            glide_active: Cell::new(false),
            last_glide_word: RefCell::new(None),
            glide_preview_points: Cell::new(0),
            precise_origin: Cell::new(None),
            clipboard_service: RefCell::new(Rc::new(InMemoryClipboardService::default())),
            clipboard_spawner: RefCell::new(None),
            clipboard_failed: Event::new(),
            feedback_requested: Event::new(),
            hide_requested: Event::new(),
            subtype_picker_requested: Event::new(),
            settings_requested: Event::new(),
            resize_mode_requested: Event::new(),
            voice_input_requested: Event::new(),
            computer: LayoutComputer::new(resources.clone()),
            settings,
            resources,
            language_models,
            cache: RefCell::new(HashMap::new()),
            subtypes: Rc::new(RefCell::new(Vec::new())),
            active: RefCell::new(Subtype::shared_default()),
            keyboard: RefCell::new(None),
            mode: Cell::new(KeyboardMode::Characters),
            panel: Cell::new(KeyboardUiMode::Text),
            shift: EngineShiftState::default(),
            culture: RefCell::new(TextCulture::invariant()),
            width: Cell::new(0.0),
            height: Cell::new(0.0),
            geometry: RefCell::new(Rc::new(KeyboardGeometryOptions::default())),
            glide: Arc::new(GlideTypingClassifier::default()),
            keyboard_changed: Event::new(),
            state_changed: Event::new(),
            resource_token: Cell::new(None),
            settings_token: Cell::new(None),
            clipboard_token: Cell::new(None),
            target: RefCell::new(None),
            editor: RefCell::new(None),
            target_token: Cell::new(None),
            clock: RefCell::new(Some(crate::editor::default_clock())),
            suppress_target_changed: Cell::new(false),
            last_insert_was_auto_space: Cell::new(false),
            suggestions: RefCell::new(SuggestionList::default()),
            providers: RefCell::new(HashMap::new()),
            provider_factory: RefCell::new(None),
            rejected_corrections: RefCell::new(HashSet::new()),
            last_clipboard_suggestion: RefCell::new(None),
            clipboard_history: Rc::new(ClipboardHistory::default()),
            emoji_history: Rc::new(EmojiHistory::default()),
            suggestions_changed: Event::new(),
            text_committed: Event::new(),
        });
        // Borrowed event sources must not own the engine. Upgrade only during a synchronous
        // notification; live callbacks retain their original order and typed failure behavior.
        let receiver = Rc::downgrade(&e);
        let token = e.resources.changed().add(Rc::new(move |_| {
            if let Some(receiver) = receiver.upgrade() {
                receiver
                    .on_resources_changed()
                    .unwrap_or_else(|error| std::panic::panic_any(error));
            }
        }));
        e.resource_token.set(Some(token));
        let receiver = Rc::downgrade(&e);
        let token = e.settings.property_changed().add(Rc::new(move |name| {
            if let Some(receiver) = receiver.upgrade() {
                receiver
                    .on_settings_changed(name)
                    .unwrap_or_else(|error| std::panic::panic_any(error));
            }
        }));
        e.settings_token.set(Some(token));
        let receiver = Rc::downgrade(&e);
        let token = e.clipboard_history.changed().add(Rc::new(move |_| {
            if let Some(receiver) = receiver.upgrade() {
                receiver
                    .update_suggestions()
                    .unwrap_or_else(|error| std::panic::panic_any(error));
            }
        }));
        e.clipboard_token.set(Some(token));
        e.load_subtypes()?;
        Ok(e)
    }
    pub fn settings(&self) -> Rc<KeyboardSettings> {
        self.settings.clone()
    }
    pub fn resources(&self) -> Rc<KeyboardResources> {
        self.resources.clone()
    }
    pub fn language_models(&self) -> Arc<LanguageModels> {
        self.language_models.clone()
    }
    pub fn glide_classifier(&self) -> Arc<GlideTypingClassifier> {
        self.glide.clone()
    }
    pub fn mode(&self) -> KeyboardMode {
        self.mode.get()
    }
    pub fn shift_state(&self) -> ShiftState {
        self.shift.state()
    }
    pub fn ui_mode(&self) -> KeyboardUiMode {
        self.panel.get()
    }
    pub fn active_subtype(&self) -> Arc<Subtype> {
        self.active.borrow().clone()
    }
    pub fn subtypes(&self) -> Rc<RefCell<Vec<Arc<Subtype>>>> {
        self.subtypes.clone()
    }
    /// The native casing profile supplied to suggestion requests.
    pub fn culture(&self) -> TextCulture {
        *self.culture.borrow()
    }
    pub fn keyboard_changed(&self) -> &Event<Self> {
        &self.keyboard_changed
    }
    pub fn state_changed(&self) -> &Event<Self> {
        &self.state_changed
    }
    pub fn keyboard(&self) -> EngineResult<Rc<ComputedKeyboard>> {
        let value = self.keyboard.borrow().clone();
        if let Some(k) = value {
            Ok(k)
        } else {
            self.build_keyboard()
        }
    }
    pub fn find_subtype(&self, id: &Utf16Text) -> Option<Arc<Subtype>> {
        let presets = self.resources.subtype_presets();
        if let Some(s) = presets
            .iter()
            .find(|s| ordinal_ignore_case_equal(&s.id(), id))
        {
            return Some(s.clone());
        }
        let parts: Vec<_> = id
            .units()
            .split(|&c| c == 47)
            .map(|s| Utf16Text::from_units(s.to_vec()))
            .collect();
        let preset = self.resources.find_subtype_preset(&parts[0])?;
        if parts.len() > 1 {
            let mut clone = preset.values();
            let mut layouts = preset
                .layouts()
                .unwrap_or_else(|| std::panic::panic_any(ResourceError::NullReference))
                .values();
            layouts.characters =
                ComponentName::parse(&parts[1], &SubtypeLayoutMap::CORE_LAYOUTS.into());
            clone.layouts = Some(Arc::new(SubtypeLayoutMap::from_values(layouts)));
            Some(Arc::new(Subtype::from_values(clone)))
        } else {
            Some(preset)
        }
    }
    fn load_subtypes(&self) -> EngineResult<()> {
        let ids = self.settings.subtypes().borrow().clone();
        let mut subtypes = Vec::new();
        for id in ids {
            let id = id.ok_or(EngineError::UnrepresentedText)?;
            if let Some(s) = self.find_subtype(&id.as_str().into()) {
                subtypes.push(s);
            }
        }
        self.set_subtypes(subtypes)
    }
    pub fn set_subtypes(
        &self,
        subtypes: impl IntoIterator<Item = Arc<Subtype>>,
    ) -> EngineResult<()> {
        self.subtypes.borrow_mut().clear();
        for s in subtypes {
            self.subtypes.borrow_mut().push(s);
        }
        if self.subtypes.borrow().is_empty() {
            self.subtypes.borrow_mut().push(Subtype::shared_default());
        }
        let id = self.active_subtype().id();
        let active = self
            .subtypes
            .borrow()
            .iter()
            .find(|s| s.id() == id)
            .cloned()
            .unwrap_or_else(|| self.subtypes.borrow()[0].clone());
        self.activate_subtype(active, true)
    }
    pub fn set_active_subtype(&self, subtype: Arc<Subtype>) -> EngineResult<()> {
        if !self
            .subtypes
            .borrow()
            .iter()
            .any(|s| s.id() == subtype.id())
        {
            self.subtypes.borrow_mut().push(subtype.clone());
        }
        self.activate_subtype(subtype, false)
    }
    pub fn next_subtype(&self) -> EngineResult<()> {
        self.cycle_subtype(1)
    }
    pub fn previous_subtype(&self) -> EngineResult<()> {
        self.cycle_subtype(-1)
    }
    fn cycle_subtype(&self, delta: i32) -> EngineResult<()> {
        let list = self.subtypes.borrow().clone();
        if list.len() < 2 {
            return Ok(());
        }
        let i = list
            .iter()
            .position(|s| s.id() == self.active_subtype().id())
            .map_or(-1, |i| i as i32);
        let n = list.len() as i32;
        let index = ((i.wrapping_add(delta)) % n + n) % n;
        self.activate_subtype(list[index as usize].clone(), false)
    }
    fn activate_subtype(&self, subtype: Arc<Subtype>, force: bool) -> EngineResult<()> {
        if !force && subtype.id() == self.active_subtype().id() {
            return Ok(());
        }
        *self.active.borrow_mut() = subtype.clone();
        *self.culture.borrow_mut() = subtype_culture(subtype.language_tag().as_ref())?;
        self.cache.borrow_mut().clear();
        self.invalidate_keyboard()?;
        self.update_glide_words()?;
        self.update_auto_capitalization()?;
        self.update_suggestions()?;
        self.state_changed.raise(self);
        Ok(())
    }
    pub fn set_mode(&self, mode: KeyboardMode) -> EngineResult<()> {
        if self.mode() == mode && self.ui_mode() == KeyboardUiMode::Text {
            return Ok(());
        }
        self.mode.set(mode);
        self.panel.set(KeyboardUiMode::Text);
        if mode != KeyboardMode::Characters {
            self.shift.assign_state(ShiftState::Unshifted);
        }
        self.invalidate_keyboard()?;
        self.update_auto_capitalization()?;
        self.state_changed.raise(self);
        Ok(())
    }
    pub fn set_ui_mode(&self, mode: KeyboardUiMode) -> EngineResult<()> {
        if self.ui_mode() == mode {
            return Ok(());
        }
        self.panel.set(mode);
        if let Some(editor) = self.editor() {
            editor.end_selection_mode();
        }
        self.update_suggestions()?;
        self.state_changed.raise(self);
        Ok(())
    }
    pub fn set_shift_state(&self, state: ShiftState) -> EngineResult<()> {
        self.shift.set_state(state, self)
    }
    pub fn update_auto_capitalization(&self) -> EngineResult<()> {
        self.shift.update_auto(
            self.editor().is_some(),
            self.mode(),
            || {
                if !self.settings.auto_capitalization() {
                    return Ok(false);
                }
                let editor = self.editor().ok_or(EngineError::NoEditor)?;
                let capitalization = self.attributes().capitalization();
                let rule = self.punctuation_rule();
                let should = editor.should_auto_capitalize(capitalization, &rule)?;
                Ok(should
                    && !matches!(
                        self.attributes().kind(),
                        InputKind::Email | InputKind::Uri | InputKind::Password
                    ))
            },
            self,
        )
    }
    pub fn update_layout(
        &self,
        width: f64,
        height: f64,
        geometry: Option<Rc<KeyboardGeometryOptions>>,
    ) -> EngineResult<()> {
        self.width.set(width);
        self.height.set(height);
        if let Some(g) = geometry {
            *self.geometry.borrow_mut() = g;
        }
        let k = self.keyboard()?;
        k.layout(width, height, Some(&self.geometry.borrow()))?;
        if self.mode() == KeyboardMode::Characters {
            let keys = k.character_keys()?;
            self.glide
                .set_computed_layout(keys.iter().map(|k| k.as_ref()))?;
        }
        Ok(())
    }
    pub fn get_subtype_display_name(subtype: &Subtype) -> EngineResult<Utf16Text> {
        if let Some(name) = &subtype.display_name()
            && !name.is_empty()
        {
            return Ok(name.clone());
        }
        let c = match culture_nullable(subtype.language_tag().as_ref()) {
            Ok(culture) => culture,
            Err(EngineError::Culture(_)) => {
                return subtype.language_tag().ok_or(EngineError::UnrepresentedText);
            }
            Err(error) => return Err(error),
        };
        let profile = c;
        let names = native_names();
        let name = names
            .get(profile.name())
            .ok_or_else(|| EngineError::Culture(UnsupportedCulture(profile.name().into())))?;
        if name.is_empty()
            || Some(Utf16Text::from(name.as_str())) == subtype.language_tag()
            || profile.name().is_empty()
        {
            return subtype.language_tag().ok_or(EngineError::UnrepresentedText);
        }
        Ok(crate::text::TextUtils::capitalize(&name.as_str().into(), c))
    }
    pub fn create_compute_context(&self) -> EngineResult<KeyComputeContext> {
        let direction = self
            .keyboard
            .borrow()
            .as_ref()
            .map_or(LayoutDirection::Ltr, |k| k.direction());
        let subtype = self.active_subtype();
        Ok(KeyComputeContext::from_values(
            crate::layouts::KeyComputeContextValues {
                mode: self.mode(),
                shift_state: self.shift_state(),
                variation: self.attributes().variation(),
                is_char_half_width: self.half_width.get(),
                is_kana_kata: self.kana_kata.get(),
                direction,
                culture: Some(self.culture()),
                enter_action: self.attributes().effective_enter_action(),
                currency_set: self.resources.get_currency_set(&subtype.currency_set()),
                has_multiple_subtypes: self.subtypes.borrow().len() > 1,
                utility_key_action: self.settings.utility_key_action(),
                space_bar_mode: self.settings.space_bar_mode(),
                space_bar_label: Some(Self::get_subtype_display_name(&subtype)?),
                hint_mode: self.settings.hint_mode(),
                show_number_hints: self.settings.hinted_number_row() && !self.settings.number_row(),
                show_symbol_hints: self.settings.hinted_symbols(),
            },
        ))
    }
    fn build_keyboard(&self) -> EngineResult<Rc<ComputedKeyboard>> {
        let row = self.settings.number_row() && self.mode() == KeyboardMode::Characters;
        let key = (self.mode(), self.active_subtype().id(), row);
        let cached = self.cache.borrow().get(&key).cloned();
        let k = if let Some(k) = cached {
            k
        } else {
            let k = Rc::new(
                self.computer
                    .compute(self.mode(), self.active_subtype(), row)?,
            );
            self.cache.borrow_mut().insert(key, k.clone());
            k
        };
        *self.keyboard.borrow_mut() = Some(k.clone());
        let context = self.create_compute_context()?;
        context.set_direction(k.direction());
        k.compute(Rc::new(context))?;
        if self.width.get() > 0.0 && self.height.get() > 0.0 {
            k.layout(
                self.width.get(),
                self.height.get(),
                Some(&self.geometry.borrow()),
            )?;
            if self.mode() == KeyboardMode::Characters {
                let keys = k.character_keys()?;
                self.glide
                    .set_computed_layout(keys.iter().map(|k| k.as_ref()))?;
            }
        }
        Ok(k)
    }
    fn invalidate_keyboard(&self) -> EngineResult<()> {
        *self.keyboard.borrow_mut() = None;
        self.keyboard()?;
        self.keyboard_changed.raise(self);
        Ok(())
    }
    pub fn recompute_keyboard(&self) -> EngineResult<()> {
        let k = self.keyboard.borrow().clone();
        let Some(k) = k else {
            return self.invalidate_keyboard();
        };
        let context = self.create_compute_context()?;
        context.set_direction(k.direction());
        k.compute(Rc::new(context))?;
        if self.width.get() > 0.0 && self.height.get() > 0.0 {
            k.layout(
                self.width.get(),
                self.height.get(),
                Some(&self.geometry.borrow()),
            )?;
        }
        self.keyboard_changed.raise(self);
        Ok(())
    }
    fn update_glide_words(&self) -> EngineResult<()> {
        if !self.settings.glide_typing() {
            return Ok(());
        }
        let Some(dictionary) = self
            .language_models
            .get_dictionary_nullable(self.active_subtype().language_tag().as_ref())?
        else {
            self.glide.set_words(Vec::new())?;
            return Ok(());
        };
        let user = self
            .language_models
            .get_user_dictionary_nullable(self.active_subtype().language_tag().as_ref())?;
        let mut words = Vec::new();
        for word in dictionary.words() {
            let f = dictionary
                .get_frequency(&word)
                .map_err(|e| EngineError::Language(Arc::new(LanguageModelError::Dictionary(e))))?;
            words.push((word, f));
        }
        for (word, count) in user.words_in_learning_order().iter() {
            if *count >= user.learn_threshold() {
                words.push((word.clone(), 128));
            }
        }
        self.glide.set_words(words)?;
        Ok(())
    }
    fn on_resources_changed(&self) -> EngineResult<()> {
        self.cache.borrow_mut().clear();
        self.invalidate_keyboard()
    }
    fn on_settings_changed(&self, name: &str) -> EngineResult<()> {
        match name {
            "Subtypes" => self.load_subtypes(),
            "NumberRow" => self.invalidate_keyboard(),
            "HintedNumberRow" | "HintedSymbols" | "HintMode" | "UtilityKeyAction"
            | "SpaceBarMode" => self.recompute_keyboard(),
            "ShowSuggestions" | "IncognitoMode" | "SuggestionCount" | "NextWordPrediction"
            | "EmojiSuggestions" => {
                self.update_suggestions()?;
                self.state_changed.raise(self);
                Ok(())
            }
            "AutoCapitalization" => self.update_auto_capitalization(),
            "GlideTyping" => self.update_glide_words(),
            _ => Ok(()),
        }
    }
    pub fn dispose(&self) -> EngineResult<()> {
        self.detach()?;
        if let Some(token) = self.resource_token.take() {
            self.resources.changed().remove(token);
        }
        if let Some(token) = self.settings_token.take() {
            self.settings.property_changed().remove(token);
        }
        Ok(())
    }
}
impl Drop for KeyboardEngine {
    fn drop(&mut self) {
        // Drop removes registrations only. Fallible detach/gesture completion and observable
        // state transitions belong to the explicit dispose API while the engine is still live.
        if let Some(token) = self.resource_token.take() {
            self.resources.changed().remove(token);
        }
        if let Some(token) = self.settings_token.take() {
            self.settings.property_changed().remove(token);
        }
        if let Some(token) = self.clipboard_token.take() {
            self.clipboard_history.changed().remove(token);
        }
        if let Some(token) = self.target_token.take()
            && let Some(target) = self.target.get_mut().as_ref()
        {
            target.changed().remove(token);
        }
    }
}
impl ShiftEffects for KeyboardEngine {
    type Error = EngineError;
    fn recompute_keyboard(&self) -> EngineResult<()> {
        self.recompute_keyboard()
    }
    fn state_changed(&self) -> EngineResult<()> {
        self.state_changed.raise(self);
        Ok(())
    }
    fn update_auto_capitalization(&self) -> EngineResult<()> {
        self.update_auto_capitalization()
    }
}
// Source TryGetCulture applies this fallback only to subtype activation. Null
// and native encoding errors remain errors, as do failures in other services.
fn subtype_culture(tag: Option<&Utf16Text>) -> EngineResult<TextCulture> {
    match culture_nullable(tag) {
        Err(EngineError::Culture(_)) => Ok(TextCulture::invariant()),
        result => result,
    }
}
fn culture_nullable(tag: Option<&Utf16Text>) -> EngineResult<TextCulture> {
    culture(tag.ok_or(EngineError::ArgumentNull("name"))?)
}
fn culture(tag: &Utf16Text) -> EngineResult<TextCulture> {
    let tag = tag.to_utf8().map_err(|_| EngineError::UnrepresentedText)?;
    TextCulture::get(&tag).map_err(EngineError::Culture)
}
fn native_names() -> &'static HashMap<String, String> {
    static NAMES: LazyLock<HashMap<String, String>> = LazyLock::new(|| {
        serde_json::from_str(include_str!("native_names.json"))
            .expect("frozen culture data checked")
    });
    &NAMES
}

#[derive(Clone)]
struct AutoCorrection {
    original: Utf16Text,
    corrected: Option<Utf16Text>,
    separator: Utf16Text,
    end: i32,
}
impl std::fmt::Display for EngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ArgumentNull(name) => write!(f, "required engine argument is missing: {name}"),
            Self::NullReference | Self::Editor(EditorError::NullReference) => {
                f.write_str("required engine input is missing")
            }
            _ => write!(f, "{self:?}"),
        }
    }
}
impl std::error::Error for EngineError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Resource(error) => Some(error),
            Self::Key(error) => Some(error),
            Self::Glide(error) => Some(error),
            Self::Language(error) => Some(error.as_ref()),
            Self::Normalization(error) => Some(error),
            Self::Culture(error) => Some(error),
            Self::Editor(error) => Some(error),
            Self::Emoji(error) => Some(error),
            Self::Suggestion(error) => Some(error),
            Self::Clipboard(error) => Some(error),
            Self::ClipboardService(error) => Some(error),
            _ => None,
        }
    }
}
impl From<ClipboardError> for EngineError {
    fn from(e: ClipboardError) -> Self {
        Self::Clipboard(e)
    }
}

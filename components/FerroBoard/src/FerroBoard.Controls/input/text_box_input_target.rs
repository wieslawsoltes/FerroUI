//! Native TextBox adapter from Input/TextBoxInputTarget.cs (Apache-2.0).
//! Event hooks and queued suppression use weak receivers. Drop removes owned native hooks;
//! Public constructors leave native input-method enablement unchanged. Keyboard-owned mobile
//! adapters temporarily suppress the system IME; detach/drop restores its prior value source.
use super::{InputOverrideError, ScreenKeyboardInput};
use ferroboard_core::{
    editor::{
        CapitalizationMode, EnterAction, EnterActionHandler, ITextInputTarget, InputAttributes,
        InputKind, TargetReplaceError, TargetReplaceResult,
    },
    text::Utf16Text,
};
use ferroui_base::{
    Ref,
    input::text_input::{
        TextInputContentType, TextInputOptions, TextInputReturnKeyType, TextSelection,
    },
    input::{
        InputElement, KeyModifiers, KeyboardNavigationHandler, NavigationDirection,
        NavigationMethod,
    },
    interactivity::RoutedEventHandlerToken,
    threading::{Dispatcher, DispatcherOperationStatus, DispatcherPriority},
};
use ferroui_controls::{TextBox, TopLevel};
use ferroui_system::{Subscription, events::Event};
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
    sync::Arc,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeInputError {
    NullReference,
    ArgumentNull(&'static str),
    Override(InputOverrideError),
    UnpairedSurrogate,
    SubstringOutOfRange,
    PasswordMaskRequired,
}
impl From<InputOverrideError> for NativeInputError {
    fn from(error: InputOverrideError) -> Self {
        Self::Override(error)
    }
}
impl std::fmt::Display for NativeInputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for NativeInputError {}

struct NativeTextCache {
    native: String,
    text: Utf16Text,
}
impl NativeTextCache {
    fn new(text_box: &TextBox) -> Self {
        text_box.with_value(TextBox::text_property(), |value| {
            let native = value.as_deref().unwrap_or_default();
            Self {
                native: native.to_owned(),
                text: native.into(),
            }
        })
    }
}

pub struct TextBoxInputTarget {
    self_ref: Weak<Self>,
    system_input_method: RefCell<Option<crate::services::SystemInputMethodGuard>>,
    text_box: Ref<TextBox>,
    enter_handler: Option<EnterActionHandler>,
    attributes: Arc<InputAttributes>,
    expected_text: RefCell<Option<Utf16Text>>,
    pending_internal_edit: Cell<bool>,
    cache: RefCell<NativeTextCache>,
    changed: Event<dyn ITextInputTarget>,
    text_token: Cell<Option<RoutedEventHandlerToken>>,
    selection_token: RefCell<Option<Subscription>>,
}
impl TextBoxInputTarget {
    pub fn new(
        text_box: Ref<TextBox>,
        enter_handler: Option<EnterActionHandler>,
    ) -> Result<Rc<Self>, NativeInputError> {
        let attributes = Self::compute_attributes(&text_box)?;
        let cache = NativeTextCache::new(&text_box);
        let target = Rc::new_cyclic(|weak| Self {
            self_ref: weak.clone(),
            system_input_method: RefCell::new(None),
            text_box,
            enter_handler,
            attributes,
            expected_text: RefCell::new(None),
            pending_internal_edit: Cell::new(false),
            cache: RefCell::new(cache),
            changed: Event::new(),
            text_token: Cell::new(None),
            selection_token: RefCell::new(None),
        });
        let receiver = Rc::downgrade(&target);
        let token = target.text_box.text_changed(move |_, _| {
            if let Some(receiver) = receiver.upgrade() {
                receiver.raise_if_external();
            }
        });
        target.text_token.set(Some(token));
        let receiver = Rc::downgrade(&target);
        let token = target.text_box.selection_changed(move || {
            if let Some(receiver) = receiver.upgrade() {
                receiver.raise_if_external();
            }
        });
        *target.selection_token.borrow_mut() = Some(token.into());
        Ok(target)
    }
    pub(crate) fn for_keyboard(
        text_box: Ref<TextBox>,
        enter_handler: Option<EnterActionHandler>,
    ) -> Result<Rc<Self>, NativeInputError> {
        let target = Self::new(text_box, enter_handler)?;
        #[cfg(any(target_os = "android", target_os = "ios"))]
        {
            let guard = crate::services::SystemInputMethodGuard::new(&target.text_box);
            drop(target.system_input_method.replace(Some(guard)));
        }
        Ok(target)
    }
    pub fn text_box(&self) -> Ref<TextBox> {
        self.text_box.clone()
    }
    pub fn compute_attributes(
        text_box: &TextBox,
    ) -> Result<Arc<InputAttributes>, NativeInputError> {
        let content = TextInputOptions::get_content_type(text_box);
        let mut cap = CapitalizationMode::Sentences;
        let mut kind = match content {
            TextInputContentType::Digits => InputKind::Number,
            TextInputContentType::Number => InputKind::Decimal,
            TextInputContentType::Pin => InputKind::NumericPassword,
            TextInputContentType::Password => InputKind::Password,
            TextInputContentType::Email => InputKind::Email,
            TextInputContentType::Url => InputKind::Uri,
            TextInputContentType::Search => InputKind::Search,
            TextInputContentType::Social => InputKind::Chat,
            TextInputContentType::Name => {
                cap = CapitalizationMode::Words;
                InputKind::Text
            }
            TextInputContentType::Alpha => {
                cap = CapitalizationMode::None;
                InputKind::Text
            }
            _ => InputKind::Text,
        };
        if TextInputOptions::get_uppercase(text_box) {
            cap = CapitalizationMode::Characters;
        } else if TextInputOptions::get_lowercase(text_box) {
            cap = CapitalizationMode::None;
        } else if text_box.is_set(TextInputOptions::auto_capitalization_property().as_property()) {
            cap = if TextInputOptions::get_auto_capitalization(text_box) {
                CapitalizationMode::Sentences
            } else {
                CapitalizationMode::None
            };
        }
        let return_key = match TextInputOptions::get_return_key_type(text_box) {
            TextInputReturnKeyType::Return => EnterAction::NewLine,
            TextInputReturnKeyType::Done => EnterAction::Done,
            TextInputReturnKeyType::Go => EnterAction::Go,
            TextInputReturnKeyType::Send => EnterAction::Send,
            TextInputReturnKeyType::Search => EnterAction::Search,
            TextInputReturnKeyType::Next => EnterAction::Next,
            TextInputReturnKeyType::Previous => EnterAction::Previous,
            _ => EnterAction::Default,
        };
        let masked = text_box.password_char() != '\0';
        if masked {
            kind = if matches!(
                content,
                TextInputContentType::Pin
                    | TextInputContentType::Digits
                    | TextInputContentType::Number
            ) {
                InputKind::NumericPassword
            } else {
                InputKind::Password
            };
            cap = CapitalizationMode::None;
        }
        let suggestions =
            !masked && TextInputOptions::get_show_suggestions(text_box).unwrap_or(true);
        let attributes =
            InputAttributes::from_values(ferroboard_core::editor::InputAttributesValues {
                kind,
                capitalization: cap,
                is_multiline: text_box.accepts_return(),
                allow_suggestions: suggestions,
                allow_auto_correct: suggestions,
                is_private: masked || TextInputOptions::get_is_sensitive(text_box),
                enter_action: return_key,
            });
        Ok(ScreenKeyboardInput::apply(text_box, Arc::new(attributes))?)
    }
    /// Fallible encoding boundary. FerroUI's native String storage cannot retain unpaired UTF-16.
    /// This capability remains explicitly incomplete; no lossy replacement or shadow-success path.
    pub fn try_replace(
        &self,
        start: i32,
        length: i32,
        text: &Utf16Text,
    ) -> Result<(), NativeInputError> {
        self.try_replace_nullable(start, length, Some(text))
    }
    pub fn try_replace_nullable(
        &self,
        start: i32,
        length: i32,
        text: Option<&Utf16Text>,
    ) -> Result<(), NativeInputError> {
        if self.text_box.is_read_only() || !self.text_box.is_effectively_enabled() {
            return Ok(());
        }
        let current = self.text();
        let start = start.clamp(0, current.len() as i32);
        let length = length.clamp(0, current.len() as i32 - start);
        let mut text = text.cloned();
        if self.text_box.max_length() > 0 {
            let available = self.text_box.max_length() - (current.len() as i32 - length);
            let text = text.as_mut().ok_or(NativeInputError::NullReference)?;
            if available <= 0 && !text.is_empty() {
                return Ok(());
            }
            if text.len() as i32 > available {
                let end = usize::try_from(available)
                    .map_err(|_| NativeInputError::SubstringOutOfRange)?;
                *text = Utf16Text::from_units(text.units()[..end].to_vec());
            }
        }
        let text = text.ok_or(NativeInputError::ArgumentNull("value"))?;
        let mut updated = current.units()[..start as usize].to_vec();
        updated.extend_from_slice(text.units());
        updated.extend_from_slice(&current.units()[(start + length) as usize..]);
        let updated = Utf16Text::from_units(updated);
        let native = updated
            .to_utf8()
            .map_err(|_| NativeInputError::UnpairedSurrogate)?;
        self.mark_internal_edit(updated);
        self.text_box.set_text(Some(&native));
        self.set_native_selection(start + text.len() as i32, 0);
        Ok(())
    }
    fn set_native_selection(&self, start: i32, length: i32) {
        // Caret writes can collapse the native range. Complete both in one native
        // input-method change scope, including collapse when the caret did not move.
        let end = start + length;
        self.text_box
            .set_selection_and_caret(TextSelection::new(start, end), end);
    }
    fn mark_internal_edit(&self, expected: Utf16Text) {
        *self.expected_text.borrow_mut() = Some(expected);
        if self.pending_internal_edit.get() {
            return;
        }
        self.pending_internal_edit.set(true);
        let receiver = self.self_ref.clone();
        let operation = Dispatcher::current_dispatcher().invoke_async_local_with_priority(
            move || {
                let Some(receiver) = receiver.upgrade() else {
                    return;
                };
                receiver.pending_internal_edit.set(false);
                *receiver.expected_text.borrow_mut() = None;
            },
            DispatcherPriority::BACKGROUND,
        );
        if operation.status() == DispatcherOperationStatus::Aborted {
            self.pending_internal_edit.set(false);
        }
    }
    fn raise_if_external(&self) {
        if self.pending_internal_edit.get()
            && self.expected_text.borrow().as_ref() == Some(&self.text())
        {
            return;
        }
        self.changed.raise(self);
    }
    /// Stops observing native edits and restores this adapter's input-method override.
    /// Idempotent; editing operations and public Changed handlers remain available.
    /// Final Rc drop also performs this cleanup.
    pub fn detach(&self) {
        if let Some(token) = self.text_token.take() {
            self.text_box
                .remove_handler(TextBox::text_changed_event(), token);
        }
        let token = self.selection_token.borrow_mut().take();
        if let Some(token) = token {
            token.dispose();
        }
        let input_method = self.system_input_method.take();
        drop(input_method);
    }
}
impl TextBoxInputTarget {
    pub fn as_input_target(self: &Rc<Self>) -> Rc<dyn ITextInputTarget> {
        self.clone()
    }
    pub fn changed(&self) -> &Event<dyn ITextInputTarget> {
        &self.changed
    }
    pub fn perform_enter_action(&self, action: EnterAction) -> bool {
        perform_enter_action(&self.text_box, self.enter_handler.as_ref(), action)
    }
}
impl Drop for TextBoxInputTarget {
    fn drop(&mut self) {
        self.detach();
    }
}
impl ITextInputTarget for TextBoxInputTarget {
    fn attributes(&self) -> Arc<InputAttributes> {
        self.attributes()
    }
    fn text(&self) -> Utf16Text {
        self.text()
    }
    fn selection_start(&self) -> i32 {
        self.selection_start()
    }
    fn selection_length(&self) -> i32 {
        self.selection_length()
    }
    fn replace(&self, start: i32, length: i32, text: Option<&Utf16Text>) -> TargetReplaceResult {
        self.try_replace_nullable(start, length, text)
            .map_err(|e| match e {
                NativeInputError::NullReference => TargetReplaceError::NullReference,
                NativeInputError::ArgumentNull(p) => TargetReplaceError::ArgumentNull(p),
                e => TargetReplaceError::Native(Arc::new(e)),
            })
    }
    fn select(&self, start: i32, length: i32) {
        self.select(start, length);
    }
    fn perform_enter_action(&self, action: EnterAction) -> bool {
        self.perform_enter_action(action)
    }
    fn changed(&self) -> &Event<dyn ITextInputTarget> {
        &self.changed
    }
}
impl TextBoxInputTarget {
    pub fn attributes(&self) -> Arc<InputAttributes> {
        self.attributes.clone()
    }
    pub fn text(&self) -> Utf16Text {
        cached_text(&self.text_box, &self.cache)
    }
    pub fn selection_start(&self) -> i32 {
        self.text_box
            .selection_start()
            .min(self.text_box.selection_end())
            .clamp(0, self.text().len() as i32)
    }
    pub fn selection_length(&self) -> i32 {
        let end = self
            .text_box
            .selection_start()
            .max(self.text_box.selection_end())
            .clamp(0, self.text().len() as i32);
        end - self.selection_start()
    }
    pub fn select(&self, start: i32, length: i32) {
        let len = self.text().len() as i32;
        let start = start.clamp(0, len);
        let length = length.clamp(0, len - start);
        self.mark_internal_edit(self.text());
        self.set_native_selection(start, length);
    }
}
fn perform_enter_action(
    text_box: &TextBox,
    handler: Option<&EnterActionHandler>,
    action: EnterAction,
) -> bool {
    if handler.is_some_and(|handler| handler(action)) {
        return true;
    }
    if matches!(action, EnterAction::Next | EnterAction::Previous) {
        // The framework's manager lookup falls back for detached controls, and its
        // default traversal origin is thread-global even for a window-owned manager.
        // Require the target's live window/input root and supply an explicitly scoped start.
        let Some(top) = TopLevel::get_top_level(Some(text_box)) else {
            return false;
        };
        if !top.is_visible() {
            return false;
        }
        let manager = top.focus_manager();
        // Content may be a data model. Bound traversal to the actual window subtree,
        // not that model or the renderer's TopLevelHost, which sits above the window.
        let root = top.upcast::<InputElement>();
        let focused = manager
            .get_focused_element()
            .filter(|element| element == &root || root.is_visual_ancestor_of(element))
            .unwrap_or_else(|| text_box.to_ref().upcast::<InputElement>());
        if focused != root && !root.is_visual_ancestor_of(&focused) {
            return false;
        }
        // Use the same candidate selection as native Tab, including enabled/visible
        // filtering, tab groups and custom navigation. The similarly named TryMoveFocus
        // prototype does not apply all of those rules in the selected framework.
        let Some(next) = KeyboardNavigationHandler::get_next(
            &focused,
            if action == EnterAction::Next {
                NavigationDirection::Next
            } else {
                NavigationDirection::Previous
            },
        ) else {
            return false;
        };
        if next != root && !root.is_visual_ancestor_of(&next) {
            return false;
        }
        return manager.focus(Some(&next), NavigationMethod::Tab, KeyModifiers::NONE);
    }
    false
}

/// Source PasswordBox adapter over FerroUI's masked TextBox. The native control must already have
/// PasswordChar set; the source PasswordBox type guarantees masking at construction instead.
pub struct PasswordBoxInputTarget {
    system_input_method: RefCell<Option<crate::services::SystemInputMethodGuard>>,
    text_box: Ref<TextBox>,
    enter_handler: Option<EnterActionHandler>,
    attributes: Arc<InputAttributes>,
    expected: RefCell<Option<Utf16Text>>,
    caret: Cell<i32>,
    changed: Event<dyn ITextInputTarget>,
    cache: RefCell<NativeTextCache>,
    token: Cell<Option<RoutedEventHandlerToken>>,
}
impl PasswordBoxInputTarget {
    pub fn new(
        text_box: Ref<TextBox>,
        enter_handler: Option<EnterActionHandler>,
    ) -> Result<Rc<Self>, NativeInputError> {
        if text_box.password_char() == '\0' {
            return Err(NativeInputError::PasswordMaskRequired);
        }
        let numeric = matches!(
            TextInputOptions::get_content_type(&text_box),
            TextInputContentType::Pin | TextInputContentType::Digits | TextInputContentType::Number
        );
        let attributes = ScreenKeyboardInput::apply(
            &text_box,
            Arc::new(InputAttributes::from_values(
                ferroboard_core::editor::InputAttributesValues {
                    kind: if numeric {
                        InputKind::NumericPassword
                    } else {
                        InputKind::Password
                    },
                    capitalization: CapitalizationMode::None,
                    allow_suggestions: false,
                    allow_auto_correct: false,
                    is_private: true,
                    ..(InputAttributes::default()).values()
                },
            )),
        )?;
        let cache = NativeTextCache::new(&text_box);
        let target = Rc::new(Self {
            system_input_method: RefCell::new(None),
            text_box,
            enter_handler,
            attributes,
            expected: RefCell::new(None),
            caret: Cell::new(-1),
            changed: Event::new(),
            cache: RefCell::new(cache),
            token: Cell::new(None),
        });
        let receiver = Rc::downgrade(&target);
        let token = target.text_box.text_changing(move |_, _| {
            let Some(receiver) = receiver.upgrade() else {
                return;
            };
            if receiver.expected.borrow().as_ref() == Some(&receiver.text()) {
                return;
            }
            *receiver.expected.borrow_mut() = None;
            receiver.caret.set(-1);
            receiver.changed.raise(receiver.as_ref());
        });
        target.token.set(Some(token));
        Ok(target)
    }
    pub(crate) fn for_keyboard(
        text_box: Ref<TextBox>,
        enter_handler: Option<EnterActionHandler>,
    ) -> Result<Rc<Self>, NativeInputError> {
        let target = Self::new(text_box, enter_handler)?;
        #[cfg(any(target_os = "android", target_os = "ios"))]
        {
            let guard = crate::services::SystemInputMethodGuard::new(&target.text_box);
            drop(target.system_input_method.replace(Some(guard)));
        }
        Ok(target)
    }
    pub fn try_replace(
        &self,
        start: i32,
        length: i32,
        text: &Utf16Text,
    ) -> Result<(), NativeInputError> {
        self.try_replace_nullable(start, length, Some(text))
    }
    pub fn try_replace_nullable(
        &self,
        start: i32,
        length: i32,
        text: Option<&Utf16Text>,
    ) -> Result<(), NativeInputError> {
        // FerroUI uses a masked TextBox rather than WinUI PasswordBox. Honor its
        // live native editing policy, including ancestors and pending paste completion.
        if self.text_box.is_read_only() || !self.text_box.is_effectively_enabled() {
            return Ok(());
        }
        let current = self.text();
        let start = start.clamp(0, current.len() as i32);
        let length = length.clamp(0, current.len() as i32 - start);
        let mut text = text.cloned();
        if self.text_box.max_length() > 0 {
            let available = self.text_box.max_length() - (current.len() as i32 - length);
            let text = text.as_mut().ok_or(NativeInputError::NullReference)?;
            if text.len() as i32 > available {
                *text = Utf16Text::from_units(text.units()[..available.max(0) as usize].to_vec());
            }
        }
        let text = text.ok_or(NativeInputError::ArgumentNull("value"))?;
        let mut updated = current.units()[..start as usize].to_vec();
        updated.extend_from_slice(text.units());
        updated.extend_from_slice(&current.units()[(start + length) as usize..]);
        let updated = Utf16Text::from_units(updated);
        let native = updated
            .to_utf8()
            .map_err(|_| NativeInputError::UnpairedSurrogate)?;
        *self.expected.borrow_mut() = Some(updated);
        let caret = start + text.len() as i32;
        self.caret.set(caret);
        self.text_box.set_text(Some(&native));
        // Keep the actual native editor/IME caret aligned with the password
        // target's logical UTF16 position. A reentrant external edit wins.
        let expected = self.expected.borrow().clone();
        if self.caret.get() == caret && expected.is_some_and(|value| value == self.text()) {
            self.text_box
                .set_selection_and_caret(TextSelection::new(caret, caret), caret);
        }
        Ok(())
    }
    /// Stops observing password changes and restores this adapter's input-method override.
    /// Idempotent; editing operations and public Changed handlers remain available.
    /// Final Rc drop also performs this cleanup.
    pub fn detach(&self) {
        if let Some(token) = self.token.take() {
            self.text_box
                .remove_handler(TextBox::text_changing_event(), token);
        }
        let input_method = self.system_input_method.take();
        drop(input_method);
    }
}
impl Drop for PasswordBoxInputTarget {
    fn drop(&mut self) {
        self.detach();
    }
}
impl ITextInputTarget for PasswordBoxInputTarget {
    fn attributes(&self) -> Arc<InputAttributes> {
        self.attributes.clone()
    }
    fn text(&self) -> Utf16Text {
        cached_text(&self.text_box, &self.cache)
    }
    fn selection_start(&self) -> i32 {
        let len = self.text().len() as i32;
        let caret = self.caret.get();
        if caret < 0 || caret > len { len } else { caret }
    }
    fn selection_length(&self) -> i32 {
        0
    }
    fn replace(&self, start: i32, length: i32, text: Option<&Utf16Text>) -> TargetReplaceResult {
        self.try_replace_nullable(start, length, text)
            .map_err(|e| match e {
                NativeInputError::NullReference => TargetReplaceError::NullReference,
                NativeInputError::ArgumentNull(p) => TargetReplaceError::ArgumentNull(p),
                e => TargetReplaceError::Native(Arc::new(e)),
            })
    }
    fn select(&self, start: i32, _length: i32) {
        self.caret.set(start.clamp(0, self.text().len() as i32));
    }
    fn perform_enter_action(&self, action: EnterAction) -> bool {
        perform_enter_action(&self.text_box, self.enter_handler.as_ref(), action)
    }
    fn changed(&self) -> &Event<dyn ITextInputTarget> {
        &self.changed
    }
}
fn cached_text(text_box: &TextBox, cache: &RefCell<NativeTextCache>) -> Utf16Text {
    text_box.with_value(TextBox::text_property(), |value| {
        let native = value.as_deref().unwrap_or_default();
        let mut cache = cache.borrow_mut();
        // Read the actual effective value, including before virtual/text/selection
        // callbacks and after disposal. Stable reads compare UTF-8 bytes, allocate
        // nothing, and share the existing UTF-16 buffer. Transcode only on change.
        if cache.native != native {
            cache.native.clear();
            cache.native.push_str(native);
            cache.text = native.into();
        }
        cache.text.clone()
    })
}

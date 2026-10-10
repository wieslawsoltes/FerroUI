//! The responder of text input: the object UIKit talks to while the
//! keyboard is up. It answers the questions of `UITextInput` (the text of
//! a range, the selection, the marked text, positions and ranges, the
//! rectangle of the caret) from the text input method client of the
//! focused control, applies insertions, deletions and marked text to it,
//! and states the traits of the keyboard from the text input options.
//!
//! The indices of positions and ranges are UTF-16 code units, the unit
//! of UIKit and of the selection of a client. What is arithmetic on them
//! and what follows from the options is in plain functions at the top of
//! the file; the classes of the Objective-C runtime are in `uikit`.

use crate::combined_span3::CombinedSpan3;
use ferroui_base::input::text_input::{
    TextInputContentType, TextInputOptions, TextInputReturnKeyType, TextSelection,
};

/// The log area of the text input of the platform.
pub const IME_LOG: &str = "IOSIME";

/// The values of `UITextLayoutDirection`.
pub mod text_layout_direction {
    pub const RIGHT: isize = 2;
    pub const LEFT: isize = 3;
    pub const UP: isize = 4;
    pub const DOWN: isize = 5;
}

/// The values of `UIKeyboardType` the options map to.
pub mod keyboard_type {
    pub const DEFAULT: isize = 0;
    pub const ASCII_CAPABLE: isize = 1;
    pub const URL: isize = 3;
    pub const NUMBER_PAD: isize = 4;
    pub const PHONE_PAD: isize = 5;
    pub const NAME_PHONE_PAD: isize = 6;
    pub const EMAIL_ADDRESS: isize = 7;
    pub const DECIMAL_PAD: isize = 8;
    pub const TWITTER: isize = 9;
    pub const WEB_SEARCH: isize = 10;
}

/// The values of `UIReturnKeyType` the options map to.
pub mod return_key_type {
    pub const DEFAULT: isize = 0;
    pub const GO: isize = 1;
    pub const NEXT: isize = 4;
    pub const SEARCH: isize = 6;
    pub const SEND: isize = 7;
    pub const DONE: isize = 9;
}

/// The values of `UITextAutocorrectionType` and of
/// `UITextSpellCheckingType`, which are the same.
pub mod text_trait {
    pub const DEFAULT: isize = 0;
    pub const NO: isize = 1;
    pub const YES: isize = 2;
}

/// The indices of a text range: never negative, the start before the end.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TextRangeIndices {
    pub start_index: i32,
    pub end_index: i32,
}

impl TextRangeIndices {
    /// The range between two indices, in either order.
    pub fn new(start_index: i32, end_index: i32) -> Self {
        let a = start_index.max(0);
        let b = end_index.max(0);
        Self { start_index: a.min(b), end_index: a.max(b) }
    }

    /// Whether the range has no characters.
    pub fn is_empty(&self) -> bool {
        self.start_index == self.end_index
    }
}

/// The index of a text position: never negative.
pub fn position_index(index: i32) -> i32 {
    index.max(0)
}

/// The length of the document: the surrounding text and the marked text.
pub fn document_length(surrounding_text: &[u16], marked_text: Option<&[u16]>) -> i32 {
    (surrounding_text.len() + marked_text.map_or(0, <[u16]>::len)) as i32
}

/// The position at an offset from a position, when it is in the document.
pub fn get_position_core(index: i32, offset: isize, document_length: i32) -> Option<i32> {
    let end = index.wrapping_add(offset as i32);
    if end < 0 || end > document_length {
        return None;
    }
    Some(end)
}

/// The position at an offset from a position in a layout direction, when
/// it is in the document. Up and down do not move.
pub fn get_position_core_in_direction(index: i32, direction: isize, offset: isize, document_length: i32) -> Option<i32> {
    let mut new_position = index;

    match direction {
        text_layout_direction::LEFT => new_position = new_position.wrapping_sub(offset as i32),
        text_layout_direction::RIGHT => new_position = new_position.wrapping_add(offset as i32),
        _ => {}
    }

    if new_position < 0 || new_position > document_length {
        return None;
    }

    Some(new_position)
}

/// The range from a position to the end of the document in a direction:
/// to its beginning for left and up, to its end otherwise.
pub fn character_range_by_extending(index: i32, direction: isize, document_length: i32) -> TextRangeIndices {
    if direction == text_layout_direction::LEFT || direction == text_layout_direction::UP {
        TextRangeIndices::new(0, index)
    } else {
        TextRangeIndices::new(index, document_length)
    }
}

/// Whether a text is nothing or white space only.
pub fn is_null_or_white_space(text: Option<&[u16]>) -> bool {
    text.is_none_or(|text| char::decode_utf16(text.iter().copied()).all(|c| c.is_ok_and(char::is_whitespace)))
}

/// The text of a range of the document.
///
/// Without marked text it is the part of the surrounding text, or nothing
/// when the range ends after it. With marked text the document is read as
/// the reference reads it: the text before the selection, the marked
/// text, and the selected text; what the range has beyond those is zero.
pub fn text_in_range(
    surrounding_text: &[u16],
    marked_text: Option<&[u16]>,
    current_selection: TextSelection,
    range: TextRangeIndices,
) -> Vec<u16> {
    let (start, end) = (range.start_index as usize, range.end_index as usize);
    match marked_text {
        None | Some([]) => {
            if end <= surrounding_text.len() {
                surrounding_text[start..end].to_vec()
            } else {
                Vec::new()
            }
        }
        Some(marked_text) => {
            // The reference slices with the selection as it is and fails
            // on one that is outside of the text; a failure cannot leave
            // a method UIKit called, so the selection is brought into
            // the text here.
            let length = surrounding_text.len();
            let selection_start = (current_selection.start.max(0) as usize).min(length);
            let selection_end = (current_selection.end.max(0) as usize).clamp(selection_start, length);
            let span = CombinedSpan3::new(
                &surrounding_text[..selection_start],
                marked_text,
                &surrounding_text[selection_start..selection_end],
            );
            let mut buf = vec![0u16; end - start];
            span.copy_to(&mut buf, start);
            buf
        }
    }
}

/// The range of the marked text: from the start of the selection, as long
/// as the marked text; none for marked text that is white space only.
pub fn marked_text_range(marked_text: Option<&[u16]>, selection: TextSelection) -> Option<TextRangeIndices> {
    if is_null_or_white_space(marked_text) {
        return None;
    }
    let length = marked_text.map_or(0, <[u16]>::len) as i32;
    Some(TextRangeIndices::new(selection.start, selection.start + length))
}

/// The keyboard of the content type of the options.
pub fn keyboard_type_of(options: Option<&TextInputOptions>) -> isize {
    match options {
        None => keyboard_type::DEFAULT,
        Some(options) => match options.content_type {
            TextInputContentType::Alpha => keyboard_type::ASCII_CAPABLE,
            TextInputContentType::Digits => keyboard_type::PHONE_PAD,
            TextInputContentType::Pin => keyboard_type::NUMBER_PAD,
            TextInputContentType::Number => keyboard_type::DECIMAL_PAD,
            TextInputContentType::Email => keyboard_type::EMAIL_ADDRESS,
            TextInputContentType::Url => keyboard_type::URL,
            TextInputContentType::Name => keyboard_type::NAME_PHONE_PAD,
            TextInputContentType::Social => keyboard_type::TWITTER,
            TextInputContentType::Search => keyboard_type::WEB_SEARCH,
            _ => keyboard_type::DEFAULT,
        },
    }
}

/// The return key of the options: the one they name, else "done" for a
/// single line and the default for several.
pub fn return_key_type_of(options: Option<&TextInputOptions>) -> isize {
    match options {
        Some(options) => match options.return_key_type {
            TextInputReturnKeyType::Done => return_key_type::DONE,
            TextInputReturnKeyType::Go => return_key_type::GO,
            TextInputReturnKeyType::Search => return_key_type::SEARCH,
            TextInputReturnKeyType::Next => return_key_type::NEXT,
            TextInputReturnKeyType::Return => return_key_type::DEFAULT,
            TextInputReturnKeyType::Send => return_key_type::SEND,
            _ => {
                if options.multiline {
                    return_key_type::DEFAULT
                } else {
                    return_key_type::DONE
                }
            }
        },
        None => return_key_type::DEFAULT,
    }
}

/// Whether the entry is secure: a password, a PIN, or sensitive content.
pub fn is_secure_entry(options: Option<&TextInputOptions>) -> bool {
    options.is_some_and(|options| {
        matches!(options.content_type, TextInputContentType::Password | TextInputContentType::Pin)
            || options.is_sensitive
    })
}

/// Autocorrection and spell checking: on unless the options say that no
/// suggestions are shown.
pub fn suggestions_trait_of(options: Option<&TextInputOptions>) -> isize {
    if options.and_then(|options| options.show_suggestions) == Some(false) {
        text_trait::NO
    } else {
        text_trait::YES
    }
}

/// What the return key does after its key press.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReturnKeyAction {
    /// Nothing more.
    None,
    /// The focus moves to the next element.
    MoveFocusNext,
    /// The keyboard is dismissed.
    ResignFirstResponder,
}

/// The action of a return key of a `UIReturnKeyType`.
pub fn return_key_action(return_key_type: isize) -> ReturnKeyAction {
    match return_key_type {
        return_key_type::NEXT => ReturnKeyAction::MoveFocusNext,
        return_key_type::DONE | return_key_type::GO | return_key_type::SEND | return_key_type::SEARCH => {
            ReturnKeyAction::ResignFirstResponder
        }
        _ => ReturnKeyAction::None,
    }
}

/// Whether the primary language of an input mode is the language of a
/// locale hint: the same, or a variant of it ("en" and "en-US"), without
/// regard to case.
pub fn language_matches_locale(lang: &str, locale: &str) -> bool {
    let lang = lang.to_lowercase();
    let locale = locale.to_lowercase();
    lang == locale || lang.starts_with(&(locale + "-"))
}

#[cfg(target_os = "ios")]
pub(crate) use uikit::{current_ferro_responder, set_current_ferro_responder, TextInputResponder};

#[cfg(target_os = "ios")]
mod uikit {
    use super::*;
    use crate::extensions::to_cg_rect;
    use crate::ferro_view::FerroView;
    use ferroui_base::input::raw::{IRawInputEventArgs, RawKeyEventArgs, RawKeyEventType, RawTextInputEventArgs};
    use ferroui_base::input::text_input::TextInputMethodClient;
    use ferroui_base::input::{
        FocusManager, IInputDevice, Key, KeyDeviceType, KeyboardDevice, NavigationDirection, PhysicalKey,
        RawInputModifiers,
    };
    use ferroui_base::logging::{LogEventLevel, Logger};
    use ferroui_base::reactive::IDisposable;
    use ferroui_base::Point;
    use ferroui_controls::presenters::TextPresenter;
    use objc2::rc::{Retained, Weak};
    use objc2::runtime::{AnyObject, ProtocolObject};
    use objc2::{define_class, msg_send, DefinedClass, MainThreadMarker, MainThreadOnly, Message};
    use objc2_core_foundation::{CGPoint, CGRect};
    use objc2_foundation::{
        CopyingHelper, NSArray, NSAttributedStringKey, NSComparisonResult, NSCopying, NSDictionary, NSInteger,
        NSObjectProtocol, NSRange, NSString, NSZone, NSUUID,
    };
    use objc2_ui_kit::{
        NSWritingDirection, UIEditingInteractionConfiguration, UIKeyInput, UIKeyboardType, UIResponder,
        UIReturnKeyType, UITextAutocapitalizationType, UITextAutocorrectionType, UITextInput, UITextInputDelegate,
        UITextInputMode, UITextInputPasswordRules, UITextInputStringTokenizer, UITextInputTokenizer,
        UITextInputTraits, UITextLayoutDirection, UITextPosition, UITextRange, UITextSelectionRect,
        UITextSmartDashesType, UITextSmartInsertDeleteType, UITextSmartQuotesType, UITextSpellCheckingType,
        UITextStorageDirection, UITextView,
    };
    use std::cell::{Cell, OnceCell, RefCell};
    use std::rc::Rc;

    const _: () = {
        // The constants of this file are those of UIKit.
        assert!(text_layout_direction::RIGHT == UITextLayoutDirection::Right.0);
        assert!(text_layout_direction::LEFT == UITextLayoutDirection::Left.0);
        assert!(text_layout_direction::UP == UITextLayoutDirection::Up.0);
        assert!(text_layout_direction::DOWN == UITextLayoutDirection::Down.0);
        assert!(keyboard_type::DEFAULT == UIKeyboardType::Default.0);
        assert!(keyboard_type::ASCII_CAPABLE == UIKeyboardType::ASCIICapable.0);
        assert!(keyboard_type::URL == UIKeyboardType::URL.0);
        assert!(keyboard_type::NUMBER_PAD == UIKeyboardType::NumberPad.0);
        assert!(keyboard_type::PHONE_PAD == UIKeyboardType::PhonePad.0);
        assert!(keyboard_type::NAME_PHONE_PAD == UIKeyboardType::NamePhonePad.0);
        assert!(keyboard_type::EMAIL_ADDRESS == UIKeyboardType::EmailAddress.0);
        assert!(keyboard_type::DECIMAL_PAD == UIKeyboardType::DecimalPad.0);
        assert!(keyboard_type::TWITTER == UIKeyboardType::Twitter.0);
        assert!(keyboard_type::WEB_SEARCH == UIKeyboardType::WebSearch.0);
        assert!(return_key_type::DEFAULT == UIReturnKeyType::Default.0);
        assert!(return_key_type::GO == UIReturnKeyType::Go.0);
        assert!(return_key_type::NEXT == UIReturnKeyType::Next.0);
        assert!(return_key_type::SEARCH == UIReturnKeyType::Search.0);
        assert!(return_key_type::SEND == UIReturnKeyType::Send.0);
        assert!(return_key_type::DONE == UIReturnKeyType::Done.0);
        assert!(text_trait::NO == UITextAutocorrectionType::No.0);
        assert!(text_trait::YES == UITextAutocorrectionType::Yes.0);
        assert!(text_trait::NO == UITextSpellCheckingType::No.0);
        assert!(text_trait::YES == UITextSpellCheckingType::Yes.0);
    };

    thread_local! {
        /// The responder of the platform that is the first responder: a
        /// view, or a text input responder of a view. What the reference
        /// keeps in a static property of the view.
        static CURRENT_FERRO_RESPONDER: RefCell<Option<Retained<UIResponder>>> = const { RefCell::new(None) };
    }

    /// The responder of the platform that is the first responder.
    pub(crate) fn current_ferro_responder() -> Option<Retained<UIResponder>> {
        CURRENT_FERRO_RESPONDER.with(|current| current.borrow().clone())
    }

    /// Sets the responder of the platform that is the first responder.
    pub(crate) fn set_current_ferro_responder(responder: Option<Retained<UIResponder>>) {
        // The old responder is released after the borrow ended: releasing
        // it may run code of UIKit.
        let _old = CURRENT_FERRO_RESPONDER.with(|current| current.replace(responder));
    }

    fn log(message: &str) {
        if let Some(logger) = Logger::try_get(LogEventLevel::Debug, IME_LOG) {
            logger.log(None, message);
        }
    }

    fn utf16(text: &str) -> Vec<u16> {
        text.encode_utf16().collect()
    }

    define_class!(
        // SAFETY: `UITextPosition` is made to be subclassed; the class
        // overrides nothing and does not implement `Drop`.
        #[unsafe(super(UITextPosition))]
        #[thread_kind = MainThreadOnly]
        #[name = "FerroTextPosition"]
        #[ivars = i32]
        pub(crate) struct FerroTextPosition;

        unsafe impl NSObjectProtocol for FerroTextPosition {}

        unsafe impl NSCopying for FerroTextPosition {
            #[unsafe(method_id(copyWithZone:))]
            fn copy_with_zone(&self, _zone: *const NSZone) -> Retained<Self> {
                Self::new(self.mtm(), *self.ivars())
            }
        }
    );

    // SAFETY: a copy of a position is a position.
    unsafe impl CopyingHelper for FerroTextPosition {
        type Result = Self;
    }

    impl FerroTextPosition {
        pub(crate) fn new(mtm: MainThreadMarker, index: i32) -> Retained<Self> {
            let this = mtm.alloc::<Self>().set_ivars(position_index(index));
            // SAFETY: `init` of the superclass, on the object that was
            // just allocated and whose instance variables are set.
            unsafe { msg_send![super(this), init] }
        }

        pub(crate) fn index(&self) -> i32 {
            *self.ivars()
        }
    }

    define_class!(
        // SAFETY: as for the position above.
        #[unsafe(super(UITextPosition))]
        #[thread_kind = MainThreadOnly]
        #[name = "FerroEmptyTextPosition"]
        pub(crate) struct FerroEmptyTextPosition;

        unsafe impl NSObjectProtocol for FerroEmptyTextPosition {}

        unsafe impl NSCopying for FerroEmptyTextPosition {
            #[unsafe(method_id(copyWithZone:))]
            fn copy_with_zone(&self, _zone: *const NSZone) -> Retained<Self> {
                self.retain()
            }
        }
    );

    // SAFETY: the copy of the empty position is the position itself.
    unsafe impl CopyingHelper for FerroEmptyTextPosition {
        type Result = Self;
    }

    thread_local! {
        static EMPTY_POSITION: OnceCell<Retained<FerroEmptyTextPosition>> = const { OnceCell::new() };
    }

    fn empty_position(mtm: MainThreadMarker) -> Retained<FerroEmptyTextPosition> {
        EMPTY_POSITION.with(|position| {
            position
                .get_or_init(|| {
                    let this = mtm.alloc::<FerroEmptyTextPosition>().set_ivars(());
                    // SAFETY: `init` of the superclass, on the object
                    // that was just allocated.
                    unsafe { msg_send![super(this), init] }
                })
                .clone()
        })
    }

    /// The instance variables of a text range.
    pub(crate) struct FerroTextRangeIvars {
        indices: TextRangeIndices,
        start: OnceCell<Retained<FerroTextPosition>>,
        end: OnceCell<Retained<FerroTextPosition>>,
    }

    define_class!(
        // SAFETY: `UITextRange` is made to be subclassed, with the three
        // properties overridden as here; the class does not implement
        // `Drop`.
        #[unsafe(super(UITextRange))]
        #[thread_kind = MainThreadOnly]
        #[name = "FerroTextRange"]
        #[ivars = FerroTextRangeIvars]
        pub(crate) struct FerroTextRange;

        impl FerroTextRange {
            #[unsafe(method(isEmpty))]
            fn is_empty(&self) -> bool {
                self.ivars().indices.is_empty()
            }

            #[unsafe(method_id(start))]
            fn start(&self) -> Retained<UITextPosition> {
                let ivars = self.ivars();
                let start = ivars.start.get_or_init(|| FerroTextPosition::new(self.mtm(), ivars.indices.start_index));
                Retained::into_super(start.clone())
            }

            #[unsafe(method_id(end))]
            fn end(&self) -> Retained<UITextPosition> {
                let ivars = self.ivars();
                let end = ivars.end.get_or_init(|| FerroTextPosition::new(self.mtm(), ivars.indices.end_index));
                Retained::into_super(end.clone())
            }
        }

        unsafe impl NSObjectProtocol for FerroTextRange {}

        unsafe impl NSCopying for FerroTextRange {
            #[unsafe(method_id(copyWithZone:))]
            fn copy_with_zone(&self, _zone: *const NSZone) -> Retained<Self> {
                let indices = self.ivars().indices;
                Self::new(self.mtm(), indices.start_index, indices.end_index)
            }
        }
    );

    // SAFETY: a copy of a range is a range.
    unsafe impl CopyingHelper for FerroTextRange {
        type Result = Self;
    }

    impl FerroTextRange {
        pub(crate) fn new(mtm: MainThreadMarker, start_index: i32, end_index: i32) -> Retained<Self> {
            Self::from_indices(mtm, TextRangeIndices::new(start_index, end_index))
        }

        fn from_indices(mtm: MainThreadMarker, indices: TextRangeIndices) -> Retained<Self> {
            let this = mtm.alloc::<Self>().set_ivars(FerroTextRangeIvars {
                indices,
                start: OnceCell::new(),
                end: OnceCell::new(),
            });
            // SAFETY: `init` of the superclass, on the object that was
            // just allocated and whose instance variables are set.
            unsafe { msg_send![super(this), init] }
        }

        pub(crate) fn indices(&self) -> TextRangeIndices {
            self.ivars().indices
        }

        fn into_range(this: Retained<Self>) -> Retained<UITextRange> {
            Retained::into_super(this)
        }
    }

    fn as_position(position: &UITextPosition) -> Option<&FerroTextPosition> {
        position.downcast_ref::<FerroTextPosition>()
    }

    fn as_range(range: &UITextRange) -> Option<&FerroTextRange> {
        range.downcast_ref::<FerroTextRange>()
    }

    /// The instance variables of the responder.
    pub(crate) struct TextInputResponderIvars {
        view: Weak<FerroView>,
        client: Rc<dyn TextInputMethodClient>,
        in_surrounding_text_update_event: Cell<i32>,
        beginning_of_document: Retained<FerroTextPosition>,
        tokenizer: OnceCell<Retained<UITextInputStringTokenizer>>,
        text_input_context_identifier: Retained<NSString>,
        is_in_update: Cell<bool>,
        marked_text: RefCell<Option<Vec<u16>>>,
        input_delegate: RefCell<Option<Weak<ProtocolObject<dyn UITextInputDelegate>>>>,
        surrounding_text_changed: RefCell<Option<Rc<dyn IDisposable>>>,

        enables_return_key_automatically: Cell<bool>,
        text_content_type: RefCell<Retained<NSString>>,
        smart_quotes_type: Cell<UITextSmartQuotesType>,
        smart_dashes_type: Cell<UITextSmartDashesType>,
        smart_insert_delete_type: Cell<UITextSmartInsertDeleteType>,
        password_rules: RefCell<Option<Retained<UITextInputPasswordRules>>>,
    }

    define_class!(
        // SAFETY: `UIResponder` may be subclassed; the overrides call the
        // superclass where UIKit asks for it (`becomeFirstResponder`,
        // `resignFirstResponder`), and the class does not implement
        // `Drop`. The methods of the three protocols have the signatures
        // the protocols declare.
        #[unsafe(super(UIResponder))]
        #[thread_kind = MainThreadOnly]
        #[name = "FerroTextInputResponder"]
        #[ivars = TextInputResponderIvars]
        pub(crate) struct TextInputResponder;

        impl TextInputResponder {
            #[unsafe(method_id(nextResponder))]
            fn next_responder(&self) -> Option<Retained<UIResponder>> {
                self.ivars().view.load().map(|view| Retained::into_super(Retained::into_super(view)))
            }

            #[unsafe(method(canResignFirstResponder))]
            fn can_resign_first_responder(&self) -> bool {
                true
            }

            #[unsafe(method(canBecomeFirstResponder))]
            fn can_become_first_responder(&self) -> bool {
                true
            }

            #[unsafe(method(editingInteractionConfiguration))]
            fn editing_interaction_configuration(&self) -> UIEditingInteractionConfiguration {
                UIEditingInteractionConfiguration::Default
            }

            #[unsafe(method_id(textInputContextIdentifier))]
            fn text_input_context_identifier(&self) -> Option<Retained<NSString>> {
                Some(self.ivars().text_input_context_identifier.clone())
            }

            #[unsafe(method_id(textInputMode))]
            fn text_input_mode(&self) -> Option<Retained<UITextInputMode>> {
                // The body is a closure so that it can return early.
                let get = || -> Option<Retained<UITextInputMode>> {
                let mtm = self.mtm();
                let locale_hints = self.options().and_then(|options| options.locale_hints);

                if let Some(locale_hints) = locale_hints.filter(|locale_hints| !locale_hints.is_empty()) {
                    let active_input_modes = UITextInputMode::activeInputModes(mtm);
                    for locale in locale_hints.iter() {
                        for input_mode in active_input_modes.iter() {
                            let lang = input_mode.primaryLanguage();
                            if lang.is_some_and(|lang| language_matches_locale(&lang.to_string(), locale)) {
                                return Some(input_mode);
                            }
                        }
                    }
                }

                #[allow(deprecated)]
                let mut mode = UITextInputMode::currentInputMode(mtm);
                // Can be empty, see the documentation of the active input
                // modes.
                if mode.is_none() {
                    mode = UITextInputMode::activeInputModes(mtm).iter().next();
                }
                // A text view always has an input mode.
                if mode.is_none() {
                    let tv = UITextView::new(mtm);
                    mode = tv.textInputMode();
                }
                mode
                };
                get()
            }

            #[unsafe(method(becomeFirstResponder))]
            fn become_first_responder(&self) -> bool {
                // SAFETY: the method of the superclass this one overrides.
                let res: bool = unsafe { msg_send![super(self), becomeFirstResponder] };
                if res {
                    log("Became first responder");
                    let weak = Weak::from_retained(&self.retain());
                    let subscription = self.ivars().client.surrounding_text_changed(Rc::new(move || {
                        if let Some(this) = weak.load() {
                            this.surrounding_text_changed();
                        }
                    }));
                    let old = self.ivars().surrounding_text_changed.replace(Some(subscription));
                    if let Some(old) = old {
                        old.dispose();
                    }
                    set_current_ferro_responder(Some(Retained::into_super(self.retain())));
                }

                res
            }

            #[unsafe(method(resignFirstResponder))]
            fn resign_first_responder(&self) -> bool {
                // SAFETY: the method of the superclass this one overrides.
                let res: bool = unsafe { msg_send![super(self), resignFirstResponder] };
                if res && self.is_current() {
                    log("Resigned first responder");
                    let subscription = self.ivars().surrounding_text_changed.take();
                    if let Some(subscription) = subscription {
                        subscription.dispose();
                    }
                    set_current_ferro_responder(None);
                }

                res
            }

            // The traits of the keyboard (`UITextInputTraits`).

            #[unsafe(method(autocapitalizationType))]
            fn autocapitalization_type(&self) -> UITextAutocapitalizationType {
                // The property of the reference is never set.
                UITextAutocapitalizationType::None
            }

            #[unsafe(method(autocorrectionType))]
            fn autocorrection_type(&self) -> UITextAutocorrectionType {
                UITextAutocorrectionType(suggestions_trait_of(self.options().as_ref()))
            }

            #[unsafe(method(keyboardType))]
            fn keyboard_type(&self) -> UIKeyboardType {
                UIKeyboardType(keyboard_type_of(self.options().as_ref()))
            }

            #[unsafe(method(returnKeyType))]
            fn return_key_type(&self) -> UIReturnKeyType {
                UIReturnKeyType(return_key_type_of(self.options().as_ref()))
            }

            #[unsafe(method(enablesReturnKeyAutomatically))]
            fn enables_return_key_automatically(&self) -> bool {
                self.ivars().enables_return_key_automatically.get()
            }

            #[unsafe(method(setEnablesReturnKeyAutomatically:))]
            fn set_enables_return_key_automatically(&self, value: bool) {
                self.ivars().enables_return_key_automatically.set(value);
            }

            #[unsafe(method(isSecureTextEntry))]
            fn is_secure_text_entry(&self) -> bool {
                is_secure_entry(self.options().as_ref())
            }

            #[unsafe(method(spellCheckingType))]
            fn spell_checking_type(&self) -> UITextSpellCheckingType {
                UITextSpellCheckingType(suggestions_trait_of(self.options().as_ref()))
            }

            #[unsafe(method_id(textContentType))]
            fn text_content_type(&self) -> Option<Retained<NSString>> {
                Some(self.ivars().text_content_type.borrow().clone())
            }

            #[unsafe(method(setTextContentType:))]
            fn set_text_content_type(&self, value: Option<&NSString>) {
                if let Some(value) = value {
                    let _old = self.ivars().text_content_type.replace(value.retain());
                }
            }

            #[unsafe(method(smartQuotesType))]
            fn smart_quotes_type(&self) -> UITextSmartQuotesType {
                self.ivars().smart_quotes_type.get()
            }

            #[unsafe(method(setSmartQuotesType:))]
            fn set_smart_quotes_type(&self, value: UITextSmartQuotesType) {
                self.ivars().smart_quotes_type.set(value);
            }

            #[unsafe(method(smartDashesType))]
            fn smart_dashes_type(&self) -> UITextSmartDashesType {
                self.ivars().smart_dashes_type.get()
            }

            #[unsafe(method(setSmartDashesType:))]
            fn set_smart_dashes_type(&self, value: UITextSmartDashesType) {
                self.ivars().smart_dashes_type.set(value);
            }

            #[unsafe(method(smartInsertDeleteType))]
            fn smart_insert_delete_type(&self) -> UITextSmartInsertDeleteType {
                self.ivars().smart_insert_delete_type.get()
            }

            #[unsafe(method(setSmartInsertDeleteType:))]
            fn set_smart_insert_delete_type(&self, value: UITextSmartInsertDeleteType) {
                self.ivars().smart_insert_delete_type.set(value);
            }

            #[unsafe(method_id(passwordRules))]
            fn password_rules(&self) -> Option<Retained<UITextInputPasswordRules>> {
                self.ivars().password_rules.borrow().clone()
            }

            #[unsafe(method(setPasswordRules:))]
            fn set_password_rules(&self, value: Option<&UITextInputPasswordRules>) {
                let _old = self.ivars().password_rules.replace(value.map(Message::retain));
            }
        }

        unsafe impl NSObjectProtocol for TextInputResponder {}

        unsafe impl UITextInputTraits for TextInputResponder {}

        unsafe impl UIKeyInput for TextInputResponder {
            #[unsafe(method(hasText))]
            fn has_text(&self) -> bool {
                true
            }

            #[unsafe(method(insertText:))]
            fn insert_text(&self, text: &NSString) {
                let text = text.to_string();
                log("IUIKeyInput.InsertText");

                if text == "\n" {
                    self.key_press(Key::Enter, PhysicalKey::Enter, Some("\r"));

                    match return_key_action(return_key_type_of(self.options().as_ref())) {
                        ReturnKeyAction::MoveFocusNext => {
                            let focus_manager = self
                                .ivars()
                                .view
                                .load()
                                .and_then(|view| FocusManager::get_focus_manager(&view.top_level()));
                            if let Some(focus_manager) = focus_manager {
                                focus_manager.try_move_focus(NavigationDirection::Next, None);
                            }
                        }
                        ReturnKeyAction::ResignFirstResponder => {
                            self.resignFirstResponder();
                        }
                        ReturnKeyAction::None => {}
                    }
                    return;
                }

                self.text_input(&text);
            }

            #[unsafe(method(deleteBackward))]
            fn delete_backward(&self) {
                self.key_press(Key::Back, PhysicalKey::Backspace, Some("\u{8}"));
            }
        }

        unsafe impl UITextInput for TextInputResponder {
            #[unsafe(method_id(textInRange:))]
            fn text_in_range(&self, range: &UITextRange) -> Option<Retained<NSString>> {
                // The body is a closure so that it can return early.
                let get = || -> Option<Retained<NSString>> {
                let r = as_range(range)?;
                let client = &self.ivars().client;
                let surrounding_text = utf16(&client.surrounding_text());
                log("IUIKeyInput.TextInRange");

                let marked_text = self.ivars().marked_text.borrow();
                let result = text_in_range(&surrounding_text, marked_text.as_deref(), client.selection(), r.indices());

                Some(NSString::from_str(&String::from_utf16_lossy(&result)))
                };
                get()
            }

            #[unsafe(method(replaceRange:withText:))]
            fn replace_range_with_text(&self, range: &UITextRange, text: &NSString) {
                if let Some(r) = as_range(range) {
                    log("IUIKeyInput.ReplaceText");
                    let indices = r.indices();
                    self.ivars().client.set_selection(TextSelection::new(indices.start_index, indices.end_index));
                    self.text_input(&text.to_string());
                }
            }

            #[unsafe(method_id(selectedTextRange))]
            fn selected_text_range(&self) -> Option<Retained<UITextRange>> {
                let selection = self.ivars().client.selection();
                Some(FerroTextRange::into_range(FerroTextRange::new(self.mtm(), selection.start, selection.end)))
            }

            #[unsafe(method(setSelectedTextRange:))]
            fn set_selected_text_range(&self, value: Option<&UITextRange>) {
                if self.ivars().in_surrounding_text_update_event.get() > 0 {
                    return;
                }

                match value.and_then(as_range) {
                    Some(r) => {
                        let indices = r.indices();
                        self.ivars().client.set_selection(TextSelection::new(indices.start_index, indices.end_index));
                    }
                    None => self.ivars().client.set_selection(TextSelection::default()),
                }
            }

            #[unsafe(method_id(markedTextRange))]
            fn marked_text_range(&self) -> Option<Retained<UITextRange>> {
                // The body is a closure so that it can return early.
                let get = || -> Option<Retained<UITextRange>> {
                let marked_text = self.ivars().marked_text.borrow();
                let indices = marked_text_range(marked_text.as_deref(), self.ivars().client.selection())?;
                Some(FerroTextRange::into_range(FerroTextRange::from_indices(self.mtm(), indices)))
                };
                get()
            }

            #[unsafe(method_id(markedTextStyle))]
            fn marked_text_style(&self) -> Option<Retained<NSDictionary<NSAttributedStringKey, AnyObject>>> {
                None
            }

            #[unsafe(method(setMarkedTextStyle:))]
            fn set_marked_text_style(&self, _value: Option<&NSDictionary<NSAttributedStringKey, AnyObject>>) {}

            #[unsafe(method(setMarkedText:selectedRange:))]
            fn set_marked_text_selected_range(&self, marked_text: Option<&NSString>, _selected_range: NSRange) {
                log("IUIKeyInput.SetMarkedText");

                let marked_text = marked_text.map(|marked_text| marked_text.to_string());
                *self.ivars().marked_text.borrow_mut() = marked_text.as_deref().map(utf16);
                self.ivars().client.set_preedit_text(marked_text.as_deref());
            }

            #[unsafe(method(unmarkText))]
            fn unmark_text(&self) {
                log("IUIKeyInput.UnmarkText");
                let Some(commit_string) = self.ivars().marked_text.borrow_mut().take() else {
                    return;
                };
                self.ivars().client.set_preedit_text(None);
                if is_null_or_white_space(Some(&commit_string)) {
                    return;
                }
                self.text_input(&String::from_utf16_lossy(&commit_string));
            }

            #[unsafe(method_id(beginningOfDocument))]
            fn beginning_of_document(&self) -> Retained<UITextPosition> {
                Retained::into_super(self.ivars().beginning_of_document.clone())
            }

            #[unsafe(method_id(endOfDocument))]
            fn end_of_document(&self) -> Retained<UITextPosition> {
                Retained::into_super(FerroTextPosition::new(self.mtm(), self.document_length()))
            }

            #[unsafe(method_id(textRangeFromPosition:toPosition:))]
            fn text_range_from_position_to_position(
                &self,
                from_position: &UITextPosition,
                to_position: &UITextPosition,
            ) -> Option<Retained<UITextRange>> {
                // The body is a closure so that it can return early.
                let get = || -> Option<Retained<UITextRange>> {
                let (f, t) = (as_position(from_position)?, as_position(to_position)?);
                log("IUIKeyInput.GetTextRange");
                Some(FerroTextRange::into_range(FerroTextRange::new(self.mtm(), f.index(), t.index())))
                };
                get()
            }

            #[unsafe(method_id(positionFromPosition:offset:))]
            fn position_from_position_offset(
                &self,
                from_position: &UITextPosition,
                offset: NSInteger,
            ) -> Option<Retained<UITextPosition>> {
                // The body is a closure so that it can return early.
                let get = || -> Option<Retained<UITextPosition>> {
                let pos = as_position(from_position)?;
                log("IUIKeyInput.GetPosition");
                let res = get_position_core(pos.index(), offset, self.document_length())?;
                Some(Retained::into_super(FerroTextPosition::new(self.mtm(), res)))
                };
                get()
            }

            #[unsafe(method_id(positionFromPosition:inDirection:offset:))]
            fn position_from_position_in_direction_offset(
                &self,
                from_position: &UITextPosition,
                in_direction: UITextLayoutDirection,
                offset: NSInteger,
            ) -> Option<Retained<UITextPosition>> {
                // The body is a closure so that it can return early.
                let get = || -> Option<Retained<UITextPosition>> {
                let pos = as_position(from_position)?;
                log("IUIKeyInput.GetPosition");
                let res = get_position_core_in_direction(pos.index(), in_direction.0, offset, self.document_length())?;
                Some(Retained::into_super(FerroTextPosition::new(self.mtm(), res)))
                };
                get()
            }

            #[unsafe(method(comparePosition:toPosition:))]
            fn compare_position_to_position(&self, first: &UITextPosition, second: &UITextPosition) -> NSComparisonResult {
                match (as_position(first), as_position(second)) {
                    (Some(f), Some(s)) => match f.index().cmp(&s.index()) {
                        std::cmp::Ordering::Less => NSComparisonResult::Ascending,
                        std::cmp::Ordering::Greater => NSComparisonResult::Descending,
                        std::cmp::Ordering::Equal => NSComparisonResult::Same,
                    },
                    // The default of the type, as in the reference.
                    _ => NSComparisonResult::Same,
                }
            }

            #[unsafe(method(offsetFromPosition:toPosition:))]
            fn offset_from_position_to_position(
                &self,
                from_position: &UITextPosition,
                to_position: &UITextPosition,
            ) -> NSInteger {
                match (as_position(from_position), as_position(to_position)) {
                    (Some(f), Some(t)) => (t.index() - f.index()) as NSInteger,
                    _ => 0,
                }
            }

            #[unsafe(method_id(inputDelegate))]
            fn input_delegate(&self) -> Option<Retained<ProtocolObject<dyn UITextInputDelegate>>> {
                self.ivars().input_delegate.borrow().as_ref().and_then(Weak::load)
            }

            #[unsafe(method(setInputDelegate:))]
            fn set_input_delegate(&self, value: Option<&ProtocolObject<dyn UITextInputDelegate>>) {
                *self.ivars().input_delegate.borrow_mut() = value.map(|value| Weak::from_retained(&value.retain()));
            }

            #[unsafe(method_id(tokenizer))]
            fn tokenizer(&self) -> Retained<ProtocolObject<dyn UITextInputTokenizer>> {
                let tokenizer = self.ivars().tokenizer.get_or_init(|| {
                    // SAFETY: the responder implements `UITextInput`, as
                    // the initializer asks of its argument.
                    unsafe { UITextInputStringTokenizer::initWithTextInput(self.mtm().alloc(), self) }
                });
                ProtocolObject::from_retained(tokenizer.clone())
            }

            #[unsafe(method_id(positionWithinRange:farthestInDirection:))]
            fn position_within_range_farthest_in_direction(
                &self,
                range: &UITextRange,
                direction: UITextLayoutDirection,
            ) -> Option<Retained<UITextPosition>> {
                // The body is a closure so that it can return early.
                let get = || -> Option<Retained<UITextPosition>> {
                as_range(range)?;
                if direction == UITextLayoutDirection::Right || direction == UITextLayoutDirection::Down {
                    return Some(range.end());
                }
                Some(range.start())
                };
                get()
            }

            #[unsafe(method_id(characterRangeByExtendingPosition:inDirection:))]
            fn character_range_by_extending_position_in_direction(
                &self,
                by_extending_position: &UITextPosition,
                direction: UITextLayoutDirection,
            ) -> Option<Retained<UITextRange>> {
                // The body is a closure so that it can return early.
                let get = || -> Option<Retained<UITextRange>> {
                let p = as_position(by_extending_position)?;
                let indices = character_range_by_extending(p.index(), direction.0, self.document_length());
                Some(FerroTextRange::into_range(FerroTextRange::from_indices(self.mtm(), indices)))
                };
                get()
            }

            #[unsafe(method(baseWritingDirectionForPosition:inDirection:))]
            fn base_writing_direction_for_position_in_direction(
                &self,
                _for_position: &UITextPosition,
                _direction: UITextStorageDirection,
            ) -> NSWritingDirection {
                // todo query and return RTL.
                NSWritingDirection::LeftToRight
            }

            #[unsafe(method(setBaseWritingDirection:forRange:))]
            fn set_base_writing_direction_for_range(&self, _writing_direction: NSWritingDirection, _range: &UITextRange) {
                // todo ? ignore?
            }

            #[unsafe(method(firstRectForRange:))]
            fn first_rect_for_range(&self, _range: &UITextRange) -> CGRect {
                log("IUITextInput:GetFirstRectForRange");
                // TODO: Query from the input client
                let r = self.ivars().view.load().map(|view| view.cursor_rect()).unwrap_or_default();

                to_cg_rect(r)
            }

            #[unsafe(method(caretRectForPosition:))]
            fn caret_rect_for_position(&self, _position: Option<&UITextPosition>) -> CGRect {
                // TODO: Query from the input client
                log("IUITextInput:GetCaretRectForPosition");
                to_cg_rect(self.ivars().client.cursor_rectangle())
            }

            #[unsafe(method_id(selectionRectsForRange:))]
            fn selection_rects_for_range(&self, _range: &UITextRange) -> Retained<NSArray<UITextSelectionRect>> {
                // TODO: Query from the input client
                log("IUITextInput:GetSelectionRect");
                NSArray::new()
            }

            #[unsafe(method_id(closestPositionToPoint:))]
            fn closest_position_to_point(&self, point: CGPoint) -> Option<Retained<UITextPosition>> {
                // The body is a closure so that it can return early.
                let get = || -> Option<Retained<UITextPosition>> {
                log("IUITextInput:GetClosestPositionToPoint");

                let presenter = self.ivars().client.text_view_visual().cast::<TextPresenter>();

                if let Some(presenter) = presenter {
                    let hit_result = presenter.text_layout().hit_test_point(Point::new(point.x, point.y));

                    return Some(Retained::into_super(FerroTextPosition::new(self.mtm(), hit_result.text_position())));
                }

                Some(Retained::into_super(empty_position(self.mtm())))
                };
                get()
            }

            #[unsafe(method_id(closestPositionToPoint:withinRange:))]
            fn closest_position_to_point_within_range(
                &self,
                _point: CGPoint,
                _within_range: &UITextRange,
            ) -> Option<Retained<UITextPosition>> {
                // TODO: Query from the input client
                log("IUITextInput:GetClosestPositionToPoint");
                Some(Retained::into_super(FerroTextPosition::new(self.mtm(), 0)))
            }

            #[unsafe(method_id(characterRangeAtPoint:))]
            fn character_range_at_point(&self, _point: CGPoint) -> Option<Retained<UITextRange>> {
                // TODO: Query from the input client
                log("IUITextInput:GetCharacterRangeAtPoint");
                Some(FerroTextRange::into_range(FerroTextRange::new(self.mtm(), 0, 0)))
            }

            #[unsafe(method_id(textStylingAtPosition:inDirection:))]
            fn text_styling_at_position_in_direction(
                &self,
                _position: &UITextPosition,
                _direction: UITextStorageDirection,
            ) -> Option<Retained<NSDictionary<NSAttributedStringKey, AnyObject>>> {
                None
            }
        }
    );

    impl TextInputResponder {
        /// Creates the responder of a client of a view.
        pub(crate) fn new(view: &FerroView, client: Rc<dyn TextInputMethodClient>) -> Retained<Self> {
            let mtm = view.mtm();
            let this = mtm.alloc::<Self>().set_ivars(TextInputResponderIvars {
                view: Weak::from_retained(&view.retain()),
                client,
                in_surrounding_text_update_event: Cell::new(0),
                beginning_of_document: FerroTextPosition::new(mtm, 0),
                tokenizer: OnceCell::new(),
                text_input_context_identifier: NSUUID::new().UUIDString(),
                is_in_update: Cell::new(false),
                marked_text: RefCell::new(None),
                input_delegate: RefCell::new(None),
                surrounding_text_changed: RefCell::new(None),
                enables_return_key_automatically: Cell::new(false),
                text_content_type: RefCell::new(NSString::from_str("text/plain")),
                smart_quotes_type: Cell::new(UITextSmartQuotesType::Default),
                smart_dashes_type: Cell::new(UITextSmartDashesType::Default),
                smart_insert_delete_type: Cell::new(UITextSmartInsertDeleteType::Default),
                password_rules: RefCell::new(None),
            });
            // SAFETY: `init` of the superclass, on the object that was
            // just allocated and whose instance variables are set.
            unsafe { msg_send![super(this), init] }
        }


        /// Whether the next responder is `view`.
        pub(crate) fn is_of_view(&self, view: &FerroView) -> bool {
            self.ivars().view.load().is_some_and(|own| std::ptr::eq(&*own, view))
        }

        /// The marked text, for an application that tests itself.
        pub(crate) fn marked_text(&self) -> Option<String> {
            self.ivars().marked_text.borrow().as_deref().map(String::from_utf16_lossy)
        }

        fn is_current(&self) -> bool {
            current_ferro_responder().is_some_and(|current| {
                std::ptr::eq(Retained::as_ptr(&current).cast::<AnyObject>(), (self as *const Self).cast::<AnyObject>())
            })
        }

        fn options(&self) -> Option<TextInputOptions> {
            self.ivars().view.load().and_then(|view| view.text_input_options())
        }

        fn document_length(&self) -> i32 {
            let surrounding_text = utf16(&self.ivars().client.surrounding_text());
            document_length(&surrounding_text, self.ivars().marked_text.borrow().as_deref())
        }

        fn surrounding_text_changed(&self) {
            log("SurroundingTextChanged");
            let delegate = self.ivars().input_delegate.borrow().as_ref().and_then(Weak::load);
            let Some(delegate) = delegate else {
                return;
            };
            if self.ivars().is_in_update.get() {
                return;
            }
            let count = &self.ivars().in_surrounding_text_update_event;
            count.set(count.get() + 1);
            let this: &ProtocolObject<dyn UITextInput> = ProtocolObject::from_ref(self);
            delegate.textWillChange(Some(this));
            delegate.textDidChange(Some(this));
            delegate.selectionWillChange(Some(this));
            delegate.selectionDidChange(Some(this));
            count.set(count.get() - 1);
        }

        fn send(&self, make: impl Fn(Rc<dyn IInputDevice>, &FerroView) -> Vec<Rc<dyn IRawInputEventArgs>>) {
            let Some(view) = self.ivars().view.load() else {
                return;
            };
            let keyboard_device: Rc<dyn IInputDevice> = match KeyboardDevice::instance() {
                Some(keyboard_device) => keyboard_device,
                None => panic!("The keyboard device of the platform is not registered."),
            };
            for args in make(keyboard_device, &view) {
                view.invoke_input(args);
            }
        }

        fn key_press(&self, key: Key, physical_key: PhysicalKey, key_symbol: Option<&str>) {
            self.ivars().is_in_update.set(true);
            log("Triggering key press");

            self.send(|keyboard_device, view| {
                [RawKeyEventType::KeyDown, RawKeyEventType::KeyUp]
                    .into_iter()
                    .map(|type_| -> Rc<dyn IRawInputEventArgs> {
                        Rc::new(RawKeyEventArgs::new(
                            keyboard_device.clone(),
                            0,
                            view.input_root(),
                            type_,
                            key,
                            RawInputModifiers::NONE,
                            physical_key,
                            key_symbol.map(str::to_string),
                            KeyDeviceType::Keyboard,
                        ))
                    })
                    .collect()
            });
            self.ivars().is_in_update.set(false);
        }

        fn text_input(&self, text: &str) {
            self.ivars().is_in_update.set(true);
            log("Triggering text input");
            self.send(|keyboard_device, view| {
                vec![Rc::new(RawTextInputEventArgs::new(keyboard_device, 0, view.input_root(), text))]
            });
            self.ivars().is_in_update.set(false);
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests of this file.
    use super::*;

    fn u(text: &str) -> Vec<u16> {
        text.encode_utf16().collect()
    }

    fn s(text: &[u16]) -> String {
        String::from_utf16_lossy(text)
    }

    #[test]
    fn a_range_is_ordered_and_not_negative() {
        assert_eq!(TextRangeIndices { start_index: 2, end_index: 5 }, TextRangeIndices::new(5, 2));
        assert_eq!(TextRangeIndices { start_index: 0, end_index: 3 }, TextRangeIndices::new(-4, 3));
        assert_eq!(TextRangeIndices { start_index: 0, end_index: 0 }, TextRangeIndices::new(-4, -1));
        assert!(TextRangeIndices::new(3, 3).is_empty());
        assert!(!TextRangeIndices::new(3, 4).is_empty());
        assert_eq!(0, position_index(-7));
        assert_eq!(7, position_index(7));
    }

    #[test]
    fn positions_stay_in_the_document() {
        assert_eq!(Some(3), get_position_core(1, 2, 5));
        assert_eq!(Some(5), get_position_core(5, 0, 5));
        assert_eq!(None, get_position_core(5, 1, 5));
        assert_eq!(Some(0), get_position_core(2, -2, 5));
        assert_eq!(None, get_position_core(2, -3, 5));

        use text_layout_direction::{DOWN, LEFT, RIGHT, UP};
        assert_eq!(Some(1), get_position_core_in_direction(3, LEFT, 2, 5));
        assert_eq!(Some(5), get_position_core_in_direction(3, RIGHT, 2, 5));
        assert_eq!(None, get_position_core_in_direction(3, RIGHT, 3, 5));
        assert_eq!(None, get_position_core_in_direction(3, LEFT, 4, 5));
        // Up and down do not move.
        assert_eq!(Some(3), get_position_core_in_direction(3, UP, 2, 5));
        assert_eq!(Some(3), get_position_core_in_direction(3, DOWN, 2, 5));
    }

    #[test]
    fn extending_a_position_reaches_an_end_of_the_document() {
        use text_layout_direction::{DOWN, LEFT, RIGHT, UP};
        assert_eq!(TextRangeIndices::new(0, 3), character_range_by_extending(3, LEFT, 9));
        assert_eq!(TextRangeIndices::new(0, 3), character_range_by_extending(3, UP, 9));
        assert_eq!(TextRangeIndices::new(3, 9), character_range_by_extending(3, RIGHT, 9));
        assert_eq!(TextRangeIndices::new(3, 9), character_range_by_extending(3, DOWN, 9));
    }

    #[test]
    fn the_text_of_a_range_without_marked_text_is_the_surrounding_text() {
        let text = u("hello world");
        let selection = TextSelection::new(5, 5);
        assert_eq!("hello", s(&text_in_range(&text, None, selection, TextRangeIndices::new(0, 5))));
        assert_eq!("world", s(&text_in_range(&text, None, selection, TextRangeIndices::new(6, 11))));
        assert_eq!("", s(&text_in_range(&text, None, selection, TextRangeIndices::new(4, 4))));
        // A range that ends after the text has no text.
        assert_eq!("", s(&text_in_range(&text, None, selection, TextRangeIndices::new(6, 12))));
        // Empty marked text is no marked text.
        assert_eq!("hello", s(&text_in_range(&text, Some(&[]), selection, TextRangeIndices::new(0, 5))));
    }

    #[test]
    fn the_indices_are_utf16_code_units() {
        // An emoji is two code units.
        let text = u("a\u{1F600}b");
        assert_eq!(4, document_length(&text, None));
        assert_eq!("\u{1F600}", s(&text_in_range(&text, None, TextSelection::default(), TextRangeIndices::new(1, 3))));
        assert_eq!("b", s(&text_in_range(&text, None, TextSelection::default(), TextRangeIndices::new(3, 4))));
    }

    #[test]
    fn the_text_of_a_range_with_marked_text_has_the_marked_text_at_the_selection() {
        let text = u("hello world");
        let marked = u("XY");
        let selection = TextSelection::new(5, 5);
        assert_eq!(13, document_length(&text, Some(&marked)));
        assert_eq!("helloXY", s(&text_in_range(&text, Some(&marked), selection, TextRangeIndices::new(0, 7))));
        assert_eq!("XY", s(&text_in_range(&text, Some(&marked), selection, TextRangeIndices::new(5, 7))));
        assert_eq!("loX", s(&text_in_range(&text, Some(&marked), selection, TextRangeIndices::new(3, 6))));
        // As the reference reads the document: after the marked text
        // comes the selected text, here none, and the rest is zero.
        assert_eq!(vec![b'Y' as u16, 0, 0], text_in_range(&text, Some(&marked), selection, TextRangeIndices::new(6, 9)));
        // With a selection, the selected text follows the marked text.
        let selection = TextSelection::new(2, 4);
        assert_eq!("heXYll", s(&text_in_range(&text, Some(&marked), selection, TextRangeIndices::new(0, 6))));
        // A selection outside of the text is brought into it.
        let selection = TextSelection::new(40, 50);
        assert_eq!("worldXY", s(&text_in_range(&text, Some(&marked), selection, TextRangeIndices::new(6, 13))));
    }

    #[test]
    fn the_marked_range_starts_at_the_selection() {
        let selection = TextSelection::new(4, 6);
        assert_eq!(None, marked_text_range(None, selection));
        assert_eq!(None, marked_text_range(Some(&u("")), selection));
        assert_eq!(None, marked_text_range(Some(&u("  \t")), selection));
        assert_eq!(Some(TextRangeIndices::new(4, 7)), marked_text_range(Some(&u("abc")), selection));
        assert!(is_null_or_white_space(None));
        assert!(is_null_or_white_space(Some(&u(" \n"))));
        assert!(!is_null_or_white_space(Some(&u(" a "))));
    }

    fn options(content_type: TextInputContentType) -> TextInputOptions {
        TextInputOptions { content_type, ..TextInputOptions::default() }
    }

    #[test]
    fn the_content_type_chooses_the_keyboard() {
        assert_eq!(keyboard_type::DEFAULT, keyboard_type_of(None));
        let of = |content_type| keyboard_type_of(Some(&options(content_type)));
        assert_eq!(keyboard_type::DEFAULT, of(TextInputContentType::Normal));
        assert_eq!(keyboard_type::ASCII_CAPABLE, of(TextInputContentType::Alpha));
        assert_eq!(keyboard_type::PHONE_PAD, of(TextInputContentType::Digits));
        assert_eq!(keyboard_type::NUMBER_PAD, of(TextInputContentType::Pin));
        assert_eq!(keyboard_type::DECIMAL_PAD, of(TextInputContentType::Number));
        assert_eq!(keyboard_type::EMAIL_ADDRESS, of(TextInputContentType::Email));
        assert_eq!(keyboard_type::URL, of(TextInputContentType::Url));
        assert_eq!(keyboard_type::NAME_PHONE_PAD, of(TextInputContentType::Name));
        assert_eq!(keyboard_type::TWITTER, of(TextInputContentType::Social));
        assert_eq!(keyboard_type::WEB_SEARCH, of(TextInputContentType::Search));
        assert_eq!(keyboard_type::DEFAULT, of(TextInputContentType::Password));
    }

    #[test]
    fn the_return_key_is_the_one_of_the_options_or_follows_the_lines() {
        assert_eq!(return_key_type::DEFAULT, return_key_type_of(None));
        let of = |return_key_type, multiline| {
            return_key_type_of(Some(&TextInputOptions { return_key_type, multiline, ..TextInputOptions::default() }))
        };
        assert_eq!(return_key_type::DONE, of(TextInputReturnKeyType::Done, true));
        assert_eq!(return_key_type::GO, of(TextInputReturnKeyType::Go, false));
        assert_eq!(return_key_type::SEARCH, of(TextInputReturnKeyType::Search, false));
        assert_eq!(return_key_type::NEXT, of(TextInputReturnKeyType::Next, false));
        assert_eq!(return_key_type::DEFAULT, of(TextInputReturnKeyType::Return, false));
        assert_eq!(return_key_type::SEND, of(TextInputReturnKeyType::Send, false));
        assert_eq!(return_key_type::DONE, of(TextInputReturnKeyType::Default, false));
        assert_eq!(return_key_type::DEFAULT, of(TextInputReturnKeyType::Default, true));
        assert_eq!(return_key_type::DONE, of(TextInputReturnKeyType::Previous, false));
    }

    #[test]
    fn the_return_key_moves_the_focus_or_dismisses_the_keyboard() {
        assert_eq!(ReturnKeyAction::MoveFocusNext, return_key_action(return_key_type::NEXT));
        for type_ in [return_key_type::DONE, return_key_type::GO, return_key_type::SEND, return_key_type::SEARCH] {
            assert_eq!(ReturnKeyAction::ResignFirstResponder, return_key_action(type_));
        }
        assert_eq!(ReturnKeyAction::None, return_key_action(return_key_type::DEFAULT));
    }

    #[test]
    fn passwords_pins_and_sensitive_content_are_secure() {
        assert!(!is_secure_entry(None));
        assert!(!is_secure_entry(Some(&options(TextInputContentType::Normal))));
        assert!(is_secure_entry(Some(&options(TextInputContentType::Password))));
        assert!(is_secure_entry(Some(&options(TextInputContentType::Pin))));
        assert!(is_secure_entry(Some(&TextInputOptions { is_sensitive: true, ..TextInputOptions::default() })));
    }

    #[test]
    fn suggestions_are_on_unless_the_options_turn_them_off() {
        assert_eq!(text_trait::YES, suggestions_trait_of(None));
        let of = |show_suggestions| {
            suggestions_trait_of(Some(&TextInputOptions { show_suggestions, ..TextInputOptions::default() }))
        };
        assert_eq!(text_trait::YES, of(None));
        assert_eq!(text_trait::YES, of(Some(true)));
        assert_eq!(text_trait::NO, of(Some(false)));
    }

    #[test]
    fn a_locale_hint_matches_its_language_and_the_variants_of_it() {
        assert!(language_matches_locale("en-US", "en"));
        assert!(language_matches_locale("EN-us", "en-US"));
        assert!(language_matches_locale("pl", "PL"));
        assert!(!language_matches_locale("en", "en-US"));
        assert!(!language_matches_locale("enx-US", "en"));
        assert!(!language_matches_locale("de-DE", "en"));
    }
}

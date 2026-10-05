use crate::primitives::TemplatedControlImpl;
use crate::utils::{ClipboardHelper, MaskedTextProvider};
use ferroui_base::utilities::CultureInfo;
use crate::{ControlImpl, TextBox, TextBoxImpl, TextBoxImplExt, TopLevel};
use ferroui_base::input::platform::{ClipboardErrorKind, ClipboardExtensions};
use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
use std::any::Any;
use ferroui_base::input::{FocusChangedEventArgs, InputElementImpl, InputElementImplExt, Key, KeyEventArgs, KeyGesture, TextInputEventArgs};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, DirectProperty, FerroObject, FerroObjectImpl,
    FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref, StaticType, StyledElementImpl,
    StyledProperty, StyledPropertyMetadata, StyledPropertyOptions, TypeInfo, VisualImpl,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// A text box that restricts its input with a mask.
#[repr(C)]
pub struct MaskedTextBox {
    base: TextBox,
    mask_provider: RefCell<Option<Rc<RefCell<MaskedTextProvider>>>>,
    ignore_text_changes: Cell<bool>,
}

ferro_class!(MaskedTextBox: TextBox);
ferroui_base::ferro_class_info!(MaskedTextBox { new: MaskedTextBox::new });

ferro_impl_classes!(MaskedTextBox: VisualImpl, LayoutableImpl, InteractiveImpl, ControlImpl, TemplatedControlImpl);

impl FerroObjectImpl for MaskedTextBox {
    fn constructed(this: &Self) {
        // The static constructor runs before the instance constructors.
        Self::parent_constructed(this);
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        let update_mask_provider = || {
            let mask = this.mask();
            let mut provider = match MaskedTextProvider::new_full(
                mask.as_deref().unwrap_or(""),
                this.culture(),
                true,
                this.prompt_char(),
                this.password_char(),
                this.ascii_only(),
            ) {
                Ok(provider) => provider,
                Err(error) => panic!("{error}"),
            };
            provider.set_reset_on_space(this.reset_on_space());
            provider.set_reset_on_prompt(this.reset_on_prompt());

            let provider = Rc::new(RefCell::new(provider));
            *this.mask_provider.borrow_mut() = Some(provider.clone());

            if let Some(text) = this.text() {
                provider.borrow_mut().set(&text);
            }
            this.refresh_text(Some(&provider), 0, false);
        };

        let property = change.property();
        let provider = this.mask_provider();

        if property == Self::mask_property().as_property() {
            update_mask_provider();

            if let Some(mask) = this.mask().filter(|mask| !mask.is_empty()) {
                for c in mask.chars() {
                    if !MaskedTextProvider::is_valid_mask_char(c) {
                        panic!("Specified mask contains characters that are not valid.");
                    }
                }
            }
        } else if property == TextBox::password_char_property().as_property() {
            if provider.is_some_and(|provider| provider.borrow().password_char() != this.password_char()) {
                update_mask_provider();
            }
        } else if property == Self::prompt_char_property().as_property() {
            if provider.is_some_and(|provider| provider.borrow().prompt_char() != this.prompt_char()) {
                update_mask_provider();
            }
        } else if property == Self::reset_on_prompt_property().as_property() {
            if let Some(provider) = provider {
                provider.borrow_mut().set_reset_on_prompt(change.get_new_value::<bool>());
            }
        } else if property == Self::reset_on_space_property().as_property() {
            if let Some(provider) = provider {
                provider.borrow_mut().set_reset_on_space(change.get_new_value::<bool>());
            }
        } else if (property == Self::ascii_only_property().as_property()
            && provider.as_ref().is_some_and(|provider| provider.borrow().ascii_only() != this.ascii_only()))
            || (property == Self::culture_property().as_property()
                && provider.as_ref().is_some_and(|provider| Some(provider.borrow().culture()) != this.culture().as_ref()))
        {
            update_mask_provider();
        }

        Self::parent_on_property_changed(this, change);
    }
}

impl StyledElementImpl for MaskedTextBox {
    fn style_key_override(_this: &Self) -> &'static TypeInfo {
        <TextBox as StaticType>::TYPE
    }
}

impl InputElementImpl for MaskedTextBox {
    fn on_got_focus(this: &Self, e: &FocusChangedEventArgs) {
        if this.hide_prompt_on_leave() {
            if let Some(provider) = this.mask_provider() {
                let text = provider.borrow().to_display_string();
                this.set_text_from_internal_synchronization(Some(text));
            }
        }
        Self::parent_on_got_focus(this, e);
    }

    /// The paste gesture reads the clipboard asynchronously: everything
    /// after the read runs when the clipboard has delivered its text, from a
    /// dispatcher job unless the clipboard completed at once. The event can
    /// only be marked as handled in the latter case: afterwards its routing
    /// is over.
    ///
    /// A clipboard that timed out is silently ignored and one that denied
    /// access is logged; nothing is pasted then. Any other failure, and a
    /// panic of the clipboard, is raised on the dispatcher.
    fn on_key_down(this: &Self, e: &KeyEventArgs) {
        let Some(provider) = this.mask_provider() else {
            Self::parent_on_key_down(this, e);
            return;
        };

        let keymap = this.get_platform_settings().map(|settings| settings.hotkey_configuration());

        let matches = |gestures: &[KeyGesture]| gestures.iter().any(|g| g.matches(Some(e)));

        if keymap.is_some_and(|keymap| matches(&keymap.paste)) {
            let Some(clipboard) = TopLevel::get_top_level(Some(this)).and_then(|top_level| top_level.clipboard())
            else {
                return;
            };

            let target = this.to_ref();
            // Set when the clipboard delivers its text before the operation
            // is started has returned: only then is the event still being
            // routed.
            let completed: Rc<Cell<bool>> = Rc::new(Cell::new(false));
            let completed_in_operation = completed.clone();

            ClipboardHelper::start(async move {
                let mut text: Option<String> = None;
                match clipboard.try_get_text_async().await {
                    Ok(value) => text = value,
                    Err(ex) if ex.kind() == ClipboardErrorKind::Timeout => {
                        // Silently ignore.
                    }
                    Err(uex) if uex.kind() == ClipboardErrorKind::AccessDenied => {
                        if let Some(logger) = Logger::try_get(LogEventLevel::Warning, LogArea::CONTROL) {
                            let source: &dyn Any = &target;
                            logger.log_with_values(
                                Some(source),
                                "Failed to read text from clipboard: {Error}",
                                &[&uex],
                            );
                        }
                    }
                    Err(ex) => return Err(ex),
                }

                let Some(text) = text else {
                    return Ok(());
                };

                // The provider is read again: it may have been replaced
                // while the clipboard was read.
                let Some(provider) = target.mask_provider() else {
                    return Ok(());
                };

                for item in text.chars() {
                    let mut index = target.get_next_character_position(target.caret_index());
                    let inserted = provider.borrow_mut().insert_at_char(item, index);
                    if inserted {
                        index += 1;
                        target.set_current_value(TextBox::caret_index_property(), index);
                    }
                }

                let text = provider.borrow().to_display_string();
                target.set_text_from_edit(Some(text));
                completed_in_operation.set(true);
                Ok(())
            });

            if completed.get() {
                e.set_handled(true);
            }
            return;
        }

        if e.key != Key::Back {
            Self::parent_on_key_down(this, e);
        }

        match e.key {
            Key::Delete => {
                if this.text_length().is_some_and(|length| this.caret_index() < length) {
                    let removed = provider.borrow_mut().remove_at(this.caret_index());
                    if removed {
                        this.refresh_text(Some(&provider), this.caret_index(), true);
                    }

                    e.set_handled(true);
                }
            }
            Key::Space => {
                let reset_on_space = provider.borrow().reset_on_space();
                if !reset_on_space || this.selected_text().is_empty() {
                    let inserted = provider.borrow_mut().insert_at(" ", this.caret_index());
                    if inserted {
                        this.refresh_text(Some(&provider), this.caret_index(), true);
                    }
                }

                e.set_handled(true);
            }
            Key::Back => {
                if this.caret_index() > 0 {
                    provider.borrow_mut().remove_at(this.caret_index() - 1);
                }
                this.refresh_text(Some(&provider), this.caret_index() - 1, true);
                e.set_handled(true);
            }
            _ => {}
        }
    }

    fn on_lost_focus(this: &Self, e: &FocusChangedEventArgs) {
        if this.hide_prompt_on_leave() {
            if let Some(provider) = this.mask_provider() {
                let text = provider.borrow().to_string_include(!this.hide_prompt_on_leave(), true);
                this.set_text_from_internal_synchronization(Some(text));
            }
        }
        Self::parent_on_lost_focus(this, e);
    }

    fn on_text_input(this: &Self, e: &TextInputEventArgs) {
        this.ignore_text_changes.set(true);

        // Clears the flag when the scope ends, also by a panic of a handler.
        struct Restore<'a>(&'a Cell<bool>);

        impl Drop for Restore<'_> {
            fn drop(&mut self) {
                self.0.set(false);
            }
        }

        let _restore = Restore(&this.ignore_text_changes);

        if this.is_read_only() {
            e.set_handled(true);
            Self::parent_on_text_input(this, e);
            return;
        }
        let Some(provider) = this.mask_provider() else {
            Self::parent_on_text_input(this, e);
            return;
        };

        let (reset_on_space, reset_on_prompt, prompt_char) = {
            let provider = provider.borrow();
            (provider.reset_on_space(), provider.reset_on_prompt(), provider.prompt_char())
        };
        let text = e.text.as_deref();
        let mut prompt_buffer = [0u8; 4];
        let prompt: &str = prompt_char.encode_utf8(&mut prompt_buffer);

        if (reset_on_space && text == Some(" ") || reset_on_prompt && text == Some(prompt))
            && !this.selected_text().is_empty()
        {
            let (selection_start, selection_end) = (this.selection_start(), this.selection_end());
            let removed = if selection_start > selection_end {
                provider.borrow_mut().remove_at_range(selection_end, selection_start - 1)
            } else {
                provider.borrow_mut().remove_at_range(selection_start, selection_end - 1)
            };
            if removed {
                this.set_selected_text(Some(""));
            }
        }

        if this.text_length().is_some_and(|length| this.caret_index() < length) {
            this.set_current_value(
                TextBox::caret_index_property(),
                this.get_next_character_position(this.caret_index()),
            );

            let input = text.expect("the text of a text input cannot be null");
            let inserted = provider.borrow_mut().insert_at(input, this.caret_index());
            if inserted {
                this.set_caret_index(this.caret_index() + 1);
            }
            let next_pos = this.get_next_character_position(this.caret_index());
            if next_pos != 0 && this.text_length() != Some(this.caret_index()) {
                this.set_current_value(TextBox::caret_index_property(), next_pos);
            }
        }

        this.refresh_text(Some(&provider), this.caret_index(), true);

        e.set_handled(true);

        Self::parent_on_text_input(this, e);
    }
}

impl TextBoxImpl for MaskedTextBox {
    fn coerce_text(this: &Self, mut text: Option<String>) -> Option<String> {
        if !this.ignore_text_changes.get() {
            if let Some(provider) = this.mask_provider() {
                let mut provider = provider.borrow_mut();
                match text.as_deref() {
                    None | Some("") => provider.clear(),
                    Some(text) => {
                        provider.set(text);
                    }
                }

                text = Some(provider.to_display_string());
            }
        }

        Self::parent_coerce_text(this, text)
    }
}

ferroui_base::ferro_properties! { impl MaskedTextBox {
    ferro_property!(
        /// Defines the `AsciiOnly` property.
        pub fn ascii_only_property() -> StyledProperty<bool> {
            FerroProperty::register::<MaskedTextBox, _>("AsciiOnly", false)
        }
    );

    ferro_property!(
        /// Defines the `Culture` property.
        pub fn culture_property() -> StyledProperty<Option<CultureInfo>> {
            FerroProperty::register::<MaskedTextBox, _>("Culture", Some(CultureInfo::current_culture()))
        }
    );

    ferro_property!(
        /// Defines the `HidePromptOnLeave` property.
        pub fn hide_prompt_on_leave_property() -> StyledProperty<bool> {
            FerroProperty::register::<MaskedTextBox, _>("HidePromptOnLeave", false)
        }
    );

    ferro_property!(
        /// Defines the `MaskCompleted` property.
        pub fn mask_completed_property() -> DirectProperty<MaskedTextBox, Option<bool>> {
            FerroProperty::register_direct::<MaskedTextBox, _>("MaskCompleted", |o| o.mask_completed(), None, None)
        }
    );

    ferro_property!(
        /// Defines the `MaskFull` property.
        pub fn mask_full_property() -> DirectProperty<MaskedTextBox, Option<bool>> {
            FerroProperty::register_direct::<MaskedTextBox, _>("MaskFull", |o| o.mask_full(), None, None)
        }
    );

    ferro_property!(
        /// Defines the `Mask` property.
        pub fn mask_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<MaskedTextBox, _>("Mask", Some(String::new()))
        }
    );

    ferro_property!(
        /// Defines the `PromptChar` property.
        pub fn prompt_char_property() -> StyledProperty<char> {
            FerroProperty::register_with::<MaskedTextBox, _>(
                "PromptChar",
                StyledPropertyOptions::new('_').coerce(Self::coerce_prompt_char),
            )
        }
    );

    ferro_property!(
        /// Defines the `ResetOnPrompt` property.
        pub fn reset_on_prompt_property() -> StyledProperty<bool> {
            FerroProperty::register::<MaskedTextBox, _>("ResetOnPrompt", true)
        }
    );

    ferro_property!(
        /// Defines the `ResetOnSpace` property.
        pub fn reset_on_space_property() -> StyledProperty<bool> {
            FerroProperty::register::<MaskedTextBox, _>("ResetOnSpace", true)
        }
    );
} }

impl MaskedTextBox {
    fn static_constructor() {
        TextBox::password_char_property().override_metadata::<MaskedTextBox>(
            StyledPropertyMetadata::new(Some('\0')).with_coerce(Self::coerce_password_char),
        );
    }

    fn coerce_password_char(sender: &FerroObject, base_value: char) -> char {
        if !MaskedTextProvider::is_valid_password_char(base_value) {
            panic!("'{base_value}' is not a valid value for PasswordChar.");
        }
        let textbox = sender.downcast_ref::<MaskedTextBox>().expect("the sender is not a masked text box");
        if let Some(mask_provider) = textbox.mask_provider() {
            if base_value == mask_provider.borrow().prompt_char() {
                // Prompt and password chars must be different.
                panic!("PasswordChar and PromptChar values cannot be the same.");
            }
        }

        base_value
    }

    fn coerce_prompt_char(sender: &FerroObject, base_value: char) -> char {
        if !MaskedTextProvider::is_valid_input_char(base_value) {
            panic!("'{base_value}' is not a valid value for PromptChar.");
        }
        if base_value == sender.get_value(TextBox::password_char_property()) {
            panic!("PasswordChar and PromptChar values cannot be the same.");
        }

        base_value
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: TextBox::construct(), mask_provider: RefCell::new(None), ignore_text_changes: Cell::new(false) }
    }

    /// Initializes a new instance of the `MaskedTextBox` class.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Constructs the masked text box with the settings of the specified
    /// provider.
    pub fn with_provider(masked_text_provider: &MaskedTextProvider) -> Ref<Self> {
        let this = Self::new();
        // These values are explicitly provided by a constructor parameter.
        this.set_ascii_only(masked_text_provider.ascii_only());
        this.set_culture(Some(masked_text_provider.culture().clone()));
        this.set_mask(Some(masked_text_provider.mask()));
        this.set_password_char(masked_text_provider.password_char());
        this.set_prompt_char(masked_text_provider.prompt_char());
        this
    }

    /// Whether the masked text box is restricted to accept only ASCII
    /// characters. Default value is false.
    pub fn ascii_only(&self) -> bool {
        self.get_value(Self::ascii_only_property())
    }

    pub fn set_ascii_only(&self, value: bool) {
        self.set_value(Self::ascii_only_property(), value)
    }

    /// The culture information associated with the masked text box.
    pub fn culture(&self) -> Option<CultureInfo> {
        self.get_value(Self::culture_property())
    }

    pub fn set_culture(&self, value: Option<CultureInfo>) {
        self.set_value(Self::culture_property(), value)
    }

    /// Whether the prompt character is hidden when the masked text box loses
    /// focus.
    pub fn hide_prompt_on_leave(&self) -> bool {
        self.get_value(Self::hide_prompt_on_leave_property())
    }

    pub fn set_hide_prompt_on_leave(&self, value: bool) {
        self.set_value(Self::hide_prompt_on_leave_property(), value)
    }

    /// The mask to apply to the text box.
    pub fn mask(&self) -> Option<String> {
        self.get_value(Self::mask_property())
    }

    pub fn set_mask(&self, value: Option<&str>) {
        self.set_value(Self::mask_property(), value.map(str::to_owned))
    }

    /// Specifies whether the test string required input positions, as
    /// specified by the mask, have all been assigned.
    pub fn mask_completed(&self) -> Option<bool> {
        self.mask_provider().map(|provider| provider.borrow().mask_completed())
    }

    /// Specifies whether all inputs (required and optional) have been
    /// provided into the mask successfully.
    pub fn mask_full(&self) -> Option<bool> {
        self.mask_provider().map(|provider| provider.borrow().mask_full())
    }

    /// The mask provider for the specified mask.
    pub fn mask_provider(&self) -> Option<Rc<RefCell<MaskedTextProvider>>> {
        self.mask_provider.borrow().clone()
    }

    /// The character used to represent the absence of user input in the
    /// masked text box.
    pub fn prompt_char(&self) -> char {
        self.get_value(Self::prompt_char_property())
    }

    pub fn set_prompt_char(&self, value: char) {
        self.set_value(Self::prompt_char_property(), value)
    }

    /// Whether selected characters should be reset when the prompt character
    /// is pressed.
    pub fn reset_on_prompt(&self) -> bool {
        self.get_value(Self::reset_on_prompt_property())
    }

    pub fn set_reset_on_prompt(&self, value: bool) {
        self.set_value(Self::reset_on_prompt_property(), value)
    }

    /// Whether selected characters should be reset when the space character
    /// is pressed.
    pub fn reset_on_space(&self) -> bool {
        self.get_value(Self::reset_on_space_property())
    }

    pub fn set_reset_on_space(&self, value: bool) {
        self.set_value(Self::reset_on_space_property(), value)
    }

    /// The length of the text in UTF-16 code units; `None` without a text.
    fn text_length(&self) -> Option<i32> {
        self.text().map(|text| text.encode_utf16().count() as i32)
    }

    fn get_next_character_position(&self, start_position: i32) -> i32 {
        if let Some(provider) = self.mask_provider() {
            let position = provider.borrow().find_edit_position_from(start_position, true);
            if self.caret_index() != -1 {
                return position;
            }
        }
        start_position
    }

    fn refresh_text(&self, provider: Option<&Rc<RefCell<MaskedTextProvider>>>, position: i32, is_edit: bool) {
        if let Some(provider) = provider {
            let text = provider.borrow().to_display_string();
            if is_edit {
                self.set_text_from_edit(Some(text));
            } else {
                self.set_text_from_internal_synchronization(Some(text));
            }
            self.set_current_value(TextBox::caret_index_property(), position);
        }
    }
}

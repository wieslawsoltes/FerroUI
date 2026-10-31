use crate::frn_string::to_c_string;
use crate::helpers::{to_frn_rect, ComResultExt};
use crate::interop::*;
use crate::native_text_input_options::{NativeTextContentType, NativeTextInputOptions};
use ferroui_base::input::text_input::{ITextInputMethodImpl, TextInputMethodClient, TextInputOptions, TextSelection};
use ferroui_base::reactive::IDisposable;
use ferroui_base::Rect;
use ferroui_microcom::ComPtr;
use std::cell::RefCell;
use std::ffi::CStr;
use std::rc::{Rc, Weak};

/// The text input method (IME) of a top-level.
pub struct FerroNativeTextInputMethod {
    weak_self: Weak<FerroNativeTextInputMethod>,
    client: RefCell<Option<Rc<dyn TextInputMethodClient>>>,
    /// The subscriptions to the events of the current client.
    subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
    native_client: RefCell<Option<ComPtr<IFrnTextInputMethodClient>>>,
    input_method: RefCell<Option<ComPtr<IFrnTextInputMethod>>>,
    /// The optional content-hint interface of the native input method (R12 of
    /// `docs/porting/winui-rich-text.md`); `None` for a library without it.
    native_options: RefCell<Option<ComPtr<IFrnTextInputMethodOptions>>>,
    /// The last options, read again with the scope of a connection client.
    options: RefCell<TextInputOptions>,
    content_hint: NativeTextInputOptions,
}

impl FerroNativeTextInputMethod {
    pub(crate) fn new(top_level: &IFrnTopLevel) -> Rc<FerroNativeTextInputMethod> {
        let input_method = top_level.get_input_method().check();
        let native_options = input_method.as_ref().and_then(|method| method.cast().ok());
        Rc::new_cyclic(|weak_self| FerroNativeTextInputMethod {
            weak_self: weak_self.clone(),
            client: RefCell::new(None),
            subscriptions: RefCell::new(Vec::new()),
            native_client: RefCell::new(None),
            input_method: RefCell::new(input_method),
            native_options: RefCell::new(native_options),
            options: RefCell::new(TextInputOptions::default()),
            content_hint: NativeTextInputOptions::default(),
        })
    }

    #[track_caller]
    fn input_method(&self) -> ComPtr<IFrnTextInputMethod> {
        match self.input_method.borrow().clone() {
            Some(input_method) => input_method,
            None => panic!("Cannot access a disposed object: the native text input method"),
        }
    }

    fn client(&self) -> Option<Rc<dyn TextInputMethodClient>> {
        self.client.borrow().clone()
    }

    /// Releases the native input method and the native client.
    pub fn dispose(&self) {
        // A hint a connection client left on the view is cleared; nothing is sent otherwise.
        self.content_hint.set_active(false);
        self.send_content_hint();
        drop(self.native_options.borrow_mut().take());
        let input_method = self.input_method.borrow_mut().take();
        drop(input_method);
        let native_client = self.native_client.borrow_mut().take();
        drop(native_client);
    }

    fn unsubscribe(&self) {
        let subscriptions = std::mem::take(&mut *self.subscriptions.borrow_mut());
        for subscription in subscriptions {
            subscription.dispose();
        }
    }

    fn on_cursor_rectangle_changed(&self) {
        let Some(client) = self.client() else {
            return;
        };

        let Some(text_view_visual) = client.text_view_visual() else {
            return;
        };

        let Some(visual_root) = text_view_visual.visual_root() else {
            return;
        };

        let Some(transform) = text_view_visual.transform_to_visual(&visual_root) else {
            return;
        };

        let rect = client.cursor_rectangle().transform_to_aabb(transform);

        self.input_method().set_cursor_rect(to_frn_rect(rect));
    }

    fn on_surrounding_text_changed(&self) {
        let Some(client) = self.client() else {
            return;
        };

        let surrounding_text = client.surrounding_text();
        let selection = client.selection();

        self.input_method().set_surrounding_text(Some(&to_c_string(&surrounding_text)), selection.start, selection.end);
    }

    fn on_selection_changed(&self) {
        let Some(client) = self.client() else {
            return;
        };

        let selection = client.selection();
        self.input_method().set_selection_in_surrounding_text(selection.start, selection.end);
    }

    /// Sends the content hint of a client of the exact-edit connection to the view
    /// (`IFrnTextInputMethodOptions`). Any other client keeps upstream's no-op
    /// `SetOptions` (decision D5): the hint is cleared when such a client follows a
    /// connection client, and nothing is ever sent otherwise.
    fn apply_options(&self) {
        let connection = self
            .client()
            .and_then(|client| client.input_method_connection())
            .filter(|connection| connection.is_current());
        let requested = connection.as_ref().map_or(NativeTextContentType::None, |connection| {
            NativeTextContentType::from_options(&self.options.borrow(), connection.effective_input_scope())
        });
        self.content_hint.set_active(connection.is_some());
        self.content_hint.set_requested(requested);
        self.send_content_hint();
    }

    fn send_content_hint(&self) {
        let native_options = self.native_options.borrow().clone();
        let Some(native_options) = native_options else {
            return;
        };
        self.content_hint.apply(|content| {
            let content = match content {
                NativeTextContentType::None => FrnTextInputContentType::FrnTextInputContentNone,
                NativeTextContentType::Url => FrnTextInputContentType::FrnTextInputContentUrl,
                NativeTextContentType::EmailAddress => FrnTextInputContentType::FrnTextInputContentEmailAddress,
                NativeTextContentType::Name => FrnTextInputContentType::FrnTextInputContentName,
                NativeTextContentType::TelephoneNumber => FrnTextInputContentType::FrnTextInputContentTelephoneNumber,
            };
            native_options.set_content_type(content);
        });
    }

    fn handler(&self, action: fn(&FerroNativeTextInputMethod)) -> Rc<dyn Fn()> {
        let this = self.weak_self.clone();
        Rc::new(move || {
            if let Some(this) = this.upgrade() {
                action(&this);
            }
        })
    }
}

impl ITextInputMethodImpl for FerroNativeTextInputMethod {
    fn reset(&self) {
        self.input_method().reset();
    }

    fn set_client(&self, client: Option<Rc<dyn TextInputMethodClient>>) {
        let old_client = self.client();
        if old_client.is_some_and(|client| client.supports_surrounding_text()) {
            self.unsubscribe();
        }

        let old_native_client = self.native_client.borrow_mut().take();
        drop(old_native_client);
        let old_client = self.client.replace(client.clone());
        drop(old_client);

        if let Some(client) = client {
            *self.native_client.borrow_mut() =
                Some(IFrnTextInputMethodClient::from_impl(NativeTextInputMethodClient { client: client.clone() }));

            self.on_surrounding_text_changed();
            self.on_cursor_rectangle_changed();
            // Note: on_selection_changed isn't called, it's already up-to-date thanks to on_surrounding_text_changed

            // A client that does not support surrounding text is never
            // unsubscribed from (see above), so stale subscriptions of such
            // a client are released here rather than accumulated.
            self.unsubscribe();
            let subscriptions = vec![
                client.surrounding_text_changed(self.handler(Self::on_surrounding_text_changed)),
                client.cursor_rectangle_changed(self.handler(Self::on_cursor_rectangle_changed)),
                client.selection_changed(self.handler(Self::on_selection_changed)),
            ];
            *self.subscriptions.borrow_mut() = subscriptions;
        }

        let native_client = self.native_client.borrow().clone();
        self.input_method().set_client(native_client.as_deref()).check();
        self.apply_options();
    }

    fn set_cursor_rect(&self, rect: Rect) {
        self.input_method().set_cursor_rect(to_frn_rect(rect));
    }

    fn set_options(&self, options: &TextInputOptions) {
        *self.options.borrow_mut() = options.clone();
        self.apply_options();
    }
}

/// The client handed to the native input method.
struct NativeTextInputMethodClient {
    client: Rc<dyn TextInputMethodClient>,
}

impl IFrnTextInputMethodClientImpl for NativeTextInputMethodClient {
    fn set_preedit_text(&self, preedit_text: Option<&CStr>) {
        crate::callback_base::guard((), || {
            if self.client.supports_preedit() {
                let preedit_text = preedit_text.map(|text| text.to_string_lossy());
                self.client.set_preedit_text(preedit_text.as_deref());
            }
        })
    }

    fn select_in_surrounding_text(&self, start: i32, end: i32) {
        crate::callback_base::guard((), || {
            if self.client.supports_surrounding_text() {
                self.client.set_selection(TextSelection::new(start, end));
            }
        })
    }
}

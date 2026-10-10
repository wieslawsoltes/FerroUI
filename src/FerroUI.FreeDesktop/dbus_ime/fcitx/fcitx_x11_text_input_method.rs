//! The input method of Fcitx, version 4 by its own name and version 5
//! through its portal name (the port of `FcitxX11TextInputMethod.cs`).

use super::dbus::{InputContext1Proxy, InputContextProxy, InputMethod1Proxy, InputMethodProxy};
use super::fcitx_enums::{FcitxCapabilityFlags, FcitxKeyEventType, FcitxKeyState};
use super::fcitx_ic_wrapper::{FcitxICWrapper, FormattedPreedit};
use crate::dbus_call_queue::DBusResult;
use crate::dbus_ime::dbus_text_input_method_base::{DBusTextInputMethodBase, DBusTextInputMethodCore};
use crate::ix11_input_method::X11InputMethodForwardedKey;
use ferroui_base::input::raw::{IRawInputEventArgs, RawKeyEventArgs, RawKeyEventType};
use ferroui_base::input::text_input::{TextInputContentType, TextInputOptions};
use ferroui_base::input::{KeyModifiers, LocalBoxFuture, RawInputModifiers};
use ferroui_base::PixelRect;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use zbus::proxy::CacheProperties;
use zbus::Connection;

pub struct FcitxX11TextInputMethod {
    base: Weak<DBusTextInputMethodBase>,
    weak_self: Weak<FcitxX11TextInputMethod>,
    context: RefCell<Option<FcitxICWrapper>>,
    last_reported_flags: Cell<Option<FcitxCapabilityFlags>>,
    option_flags: Cell<FcitxCapabilityFlags>,
    capability_flags: Cell<FcitxCapabilityFlags>,
}

/// The text of a formatted pre-edit and its cursor in UTF-16 code units
/// (`OnPreedit`): the parts joined; the cursor arrives as an offset in
/// the bytes of the UTF-8 text.
pub(crate) fn preedit_from_parts(str: &FormattedPreedit, cursorpos: i32) -> (Option<String>, Option<i32>) {
    let mut cursor = None;
    let mut preedit_string = None;
    if !str.is_empty() {
        let text: String = str.iter().map(|(part, _)| part.as_str()).collect();

        if !text.is_empty() && cursorpos >= 0 {
            // cursorpos is a byte offset in UTF8 sequence that got sent through dbus
            let utf8_string = text.as_bytes();
            if utf8_string.len() >= cursorpos as usize {
                cursor =
                    Some(String::from_utf8_lossy(&utf8_string[..cursorpos as usize]).encode_utf16().count() as i32);
            }
        }
        preedit_string = Some(text);
    }

    (preedit_string, cursor)
}

/// The capability flags of the options of a text input
/// (`UpdateOptionsField`).
pub(crate) fn option_flags(options: &TextInputOptions) -> FcitxCapabilityFlags {
    let mut flags = FcitxCapabilityFlags::empty();
    if options.lowercase {
        flags |= FcitxCapabilityFlags::CAPACITY_LOWERCASE;
    }
    if options.uppercase {
        flags |= FcitxCapabilityFlags::CAPACITY_UPPERCASE;
    }
    if !options.auto_capitalization {
        flags |= FcitxCapabilityFlags::CAPACITY_NOAUTOUPPERCASE;
    }
    if options.content_type == TextInputContentType::Email {
        flags |= FcitxCapabilityFlags::CAPACITY_EMAIL;
    } else if options.content_type == TextInputContentType::Number {
        flags |= FcitxCapabilityFlags::CAPACITY_NUMBER;
    } else if options.content_type == TextInputContentType::Password {
        flags |= FcitxCapabilityFlags::CAPACITY_PASSWORD;
    } else if options.content_type == TextInputContentType::Digits {
        flags |= FcitxCapabilityFlags::CAPACITY_DIALABLE;
    } else if options.content_type == TextInputContentType::Url {
        flags |= FcitxCapabilityFlags::CAPACITY_URL;
    }
    flags
}

/// The key state Fcitx is given with a key (`HandleKeyCore`).
pub(crate) fn fcitx_state(modifiers: RawInputModifiers) -> FcitxKeyState {
    let mut state = FcitxKeyState::empty();
    if modifiers.contains(RawInputModifiers::CONTROL) {
        state |= FcitxKeyState::CTRL;
    }
    if modifiers.contains(RawInputModifiers::ALT) {
        state |= FcitxKeyState::ALT;
    }
    if modifiers.contains(RawInputModifiers::SHIFT) {
        state |= FcitxKeyState::SHIFT;
    }
    if modifiers.contains(RawInputModifiers::META) {
        state |= FcitxKeyState::SUPER;
    }
    state
}

/// The key Fcitx forwards, as the window takes it (`OnForward`).
pub(crate) fn forwarded_key(keyval: u32, state: u32, type_: i32) -> X11InputMethodForwardedKey {
    let state = FcitxKeyState::from_bits_retain(state);
    let mut mods = KeyModifiers::empty();
    if state.contains(FcitxKeyState::CTRL) {
        mods |= KeyModifiers::CONTROL;
    }
    if state.contains(FcitxKeyState::ALT) {
        mods |= KeyModifiers::ALT;
    }
    if state.contains(FcitxKeyState::SHIFT) {
        mods |= KeyModifiers::SHIFT;
    }
    if state.contains(FcitxKeyState::SUPER) {
        mods |= KeyModifiers::META;
    }
    let is_press_key = type_ == FcitxKeyEventType::FcitxPressKey as i32;
    X11InputMethodForwardedKey {
        modifiers: mods,
        key_val: keyval as i32,
        type_: if is_press_key { RawKeyEventType::KeyDown } else { RawKeyEventType::KeyUp },
        with_text: is_press_key,
    }
}

impl FcitxX11TextInputMethod {
    /// The input method of a window, watching the names of Fcitx 4 and of
    /// the portal of Fcitx 5 on `connection`.
    #[allow(clippy::new_ret_no_self)]
    pub fn new(connection: Connection) -> Rc<DBusTextInputMethodBase> {
        DBusTextInputMethodBase::new(connection, &["org.fcitx.Fcitx", "org.freedesktop.portal.Fcitx"], |base| {
            Rc::new_cyclic(|weak_self| FcitxX11TextInputMethod {
                base,
                weak_self: weak_self.clone(),
                context: RefCell::new(None),
                last_reported_flags: Cell::new(None),
                option_flags: Cell::new(FcitxCapabilityFlags::empty()),
                capability_flags: Cell::new(FcitxCapabilityFlags::empty()),
            })
        })
    }

    fn context(&self) -> Option<FcitxICWrapper> {
        self.context.borrow().clone()
    }

    fn on_preedit(&self, args: (FormattedPreedit, i32)) {
        let (preedit_string, cursor) = preedit_from_parts(&args.0, args.1);

        if let Some(client) = self.base.upgrade().and_then(|base| base.client()) {
            if client.supports_preedit() {
                client.set_preedit_text_with_cursor(preedit_string.as_deref(), cursor);
            }
        }
    }

    fn update_options_field(&self, options: &TextInputOptions) {
        self.option_flags.set(option_flags(options));
    }

    async fn push_flags_if_needed(&self) -> DBusResult {
        let Some(context) = self.context() else {
            return Ok(());
        };

        let flags = self.option_flags.get() | self.capability_flags.get();

        if Some(flags) != self.last_reported_flags.get() {
            self.last_reported_flags.set(Some(flags));
            context.set_capacity_async(flags.bits()).await?;
        }
        Ok(())
    }

    fn on_forward(&self, ev: (u32, u32, i32)) {
        if let Some(base) = self.base.upgrade() {
            base.fire_forward(forwarded_key(ev.0, ev.1, ev.2));
        }
    }

    fn on_commit_string(&self, s: String) {
        if let Some(base) = self.base.upgrade() {
            base.fire_commit(s);
        }
    }
}

impl DBusTextInputMethodCore for FcitxX11TextInputMethod {
    fn connect(&self, name: String) -> LocalBoxFuture<DBusResult<bool>> {
        let (this, base) = (self.weak_self.clone(), self.base.clone());
        Box::pin(async move {
            let (Some(this), Some(base)) = (this.upgrade(), base.upgrade()) else {
                return Ok(false);
            };
            let connection = base.connection().clone();

            let context = if name == "org.fcitx.Fcitx" {
                let method = InputMethodProxy::builder(&connection)
                    .destination(name.clone())?
                    .path("/inputmethod")?
                    .cache_properties(CacheProperties::No)
                    .build()
                    .await?;
                let resp = method.create_icv3(&base.get_app_name(), std::process::id() as i32).await?;

                let proxy = InputContextProxy::builder(&connection)
                    .destination(name)?
                    .path(format!("/inputcontext_{}", resp.0))?
                    .cache_properties(CacheProperties::No)
                    .build()
                    .await?;
                FcitxICWrapper::from_old(proxy)
            } else {
                let method = InputMethod1Proxy::builder(&connection)
                    .destination(name.clone())?
                    .path("/inputmethod")?
                    .cache_properties(CacheProperties::No)
                    .build()
                    .await?;
                let resp = method.create_input_context(&[("appName", base.get_app_name().as_str())]).await?;

                let proxy = InputContext1Proxy::builder(&connection)
                    .destination(name)?
                    .path(resp.0.into_inner())?
                    .cache_properties(CacheProperties::No)
                    .build()
                    .await?;
                FcitxICWrapper::from_modern(proxy)
            };
            *this.context.borrow_mut() = Some(context.clone());

            let weak = Rc::downgrade(&this);
            base.add_disposable(
                context
                    .watch_commit_string_async({
                        let weak = weak.clone();
                        move |s| {
                            if let Some(this) = weak.upgrade() {
                                this.on_commit_string(s);
                            }
                        }
                    })
                    .await?,
            );
            base.add_disposable(
                context
                    .watch_forward_key_async({
                        let weak = weak.clone();
                        move |ev| {
                            if let Some(this) = weak.upgrade() {
                                this.on_forward(ev);
                            }
                        }
                    })
                    .await?,
            );
            base.add_disposable(
                context
                    .watch_update_formatted_preedit_async({
                        let weak = weak.clone();
                        move |args| {
                            if let Some(this) = weak.upgrade() {
                                this.on_preedit(args);
                            }
                        }
                    })
                    .await?,
            );

            Ok(true)
        })
    }

    fn disconnect_async(&self) -> LocalBoxFuture<DBusResult> {
        let context = self.context();
        Box::pin(async move {
            if let Some(context) = context {
                context.destroy_ic_async().await?;
            }
            Ok(())
        })
    }

    fn on_disconnected(&self) {
        *self.context.borrow_mut() = None;
    }

    fn reset(&self) {
        self.last_reported_flags.set(None);
    }

    fn set_cursor_rect_core(&self, cursor_rect: PixelRect) -> LocalBoxFuture<DBusResult> {
        let context = self.context();
        Box::pin(async move {
            if let Some(context) = context {
                context
                    .set_cursor_rect_async(
                        cursor_rect.x,
                        cursor_rect.y,
                        cursor_rect.width.max(1),
                        cursor_rect.height.max(1),
                    )
                    .await?;
            }
            Ok(())
        })
    }

    fn set_active_core(&self, active: bool) -> LocalBoxFuture<DBusResult> {
        let context = self.context();
        Box::pin(async move {
            if let Some(context) = context {
                if active {
                    context.focus_in_async().await?;
                } else {
                    context.focus_out_async().await?;
                }
            }
            Ok(())
        })
    }

    fn reset_context_core(&self) -> LocalBoxFuture<DBusResult> {
        let context = self.context();
        Box::pin(async move {
            if let Some(context) = context {
                context.reset_async().await?;
            }
            Ok(())
        })
    }

    fn handle_key_core(
        &self,
        args: Rc<dyn IRawInputEventArgs>,
        key_val: i32,
        key_code: i32,
    ) -> LocalBoxFuture<DBusResult<bool>> {
        let context = self.context();
        Box::pin(async move {
            let Some(key) = args.downcast_ref::<RawKeyEventArgs>() else {
                return Ok(false);
            };
            let state = fcitx_state(key.modifiers());

            let type_ = if key.type_() == RawKeyEventType::KeyDown {
                FcitxKeyEventType::FcitxPressKey
            } else {
                FcitxKeyEventType::FcitxReleaseKey
            };
            if let Some(context) = context {
                return Ok(context
                    .process_key_event_async(
                        key_val as u32,
                        key_code as u32,
                        state.bits(),
                        type_ as i32,
                        args.timestamp() as u32,
                    )
                    .await?);
            }

            Ok(false)
        })
    }

    fn set_capabilities_core(
        &self,
        supports_preedit: bool,
        _supports_surrounding_text: bool,
    ) -> LocalBoxFuture<DBusResult> {
        let this = self.weak_self.clone();
        Box::pin(async move {
            let Some(this) = this.upgrade() else {
                return Ok(());
            };
            this.capability_flags.set(FcitxCapabilityFlags::empty());
            if supports_preedit {
                this.capability_flags.set(FcitxCapabilityFlags::CAPACITY_PREEDIT);
            }

            this.push_flags_if_needed().await
        })
    }

    fn set_options(&self, options: &TextInputOptions) {
        let Some(base) = self.base.upgrade() else {
            return;
        };
        let (this, options) = (self.weak_self.clone(), options.clone());
        base.enqueue(move || async move {
            let Some(this) = this.upgrade() else {
                return Ok(());
            };
            this.update_options_field(&options);
            this.push_flags_if_needed().await
        });
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests of this class. The
    // input method against a service is tested in `dbus_ime/tests.rs`.
    use super::*;

    #[test]
    fn a_formatted_preedit_is_joined_and_its_cursor_converted_from_bytes() {
        assert_eq!(preedit_from_parts(&vec![], 0), (None, None));

        let parts = vec![("に".to_string(), 8), ("ほん".to_string(), 0)];
        // The cursor after the first character: three bytes, one code unit.
        assert_eq!(preedit_from_parts(&parts, 3), (Some("にほん".to_string()), Some(1)));
        assert_eq!(preedit_from_parts(&parts, 9), (Some("にほん".to_string()), Some(3)));
        // No cursor: a negative offset, or one beyond the text.
        assert_eq!(preedit_from_parts(&parts, -1), (Some("にほん".to_string()), None));
        assert_eq!(preedit_from_parts(&parts, 10), (Some("にほん".to_string()), None));
        // A character outside the basic plane is two code units.
        let parts = vec![("\u{1F600}x".to_string(), 0)];
        assert_eq!(preedit_from_parts(&parts, 4), (Some("\u{1F600}x".to_string()), Some(2)));
        // Parts without text: the empty text and no cursor.
        assert_eq!(preedit_from_parts(&vec![(String::new(), 0)], 0), (Some(String::new()), None));
    }

    #[test]
    fn the_options_of_a_text_input_become_capability_flags() {
        let mut options = TextInputOptions::default_options();
        options.auto_capitalization = true;
        assert_eq!(option_flags(&options), FcitxCapabilityFlags::empty());

        options.auto_capitalization = false;
        options.lowercase = true;
        options.content_type = TextInputContentType::Password;
        assert_eq!(
            option_flags(&options),
            FcitxCapabilityFlags::CAPACITY_NOAUTOUPPERCASE
                | FcitxCapabilityFlags::CAPACITY_LOWERCASE
                | FcitxCapabilityFlags::CAPACITY_PASSWORD
        );

        options.lowercase = false;
        options.uppercase = true;
        options.auto_capitalization = true;
        for (content_type, flag) in [
            (TextInputContentType::Email, FcitxCapabilityFlags::CAPACITY_EMAIL),
            (TextInputContentType::Number, FcitxCapabilityFlags::CAPACITY_NUMBER),
            (TextInputContentType::Digits, FcitxCapabilityFlags::CAPACITY_DIALABLE),
            (TextInputContentType::Url, FcitxCapabilityFlags::CAPACITY_URL),
        ] {
            options.content_type = content_type;
            assert_eq!(option_flags(&options), FcitxCapabilityFlags::CAPACITY_UPPERCASE | flag);
        }
    }

    #[test]
    fn modifiers_go_to_fcitx_as_its_key_state_and_come_back_with_a_forwarded_key() {
        assert_eq!(
            fcitx_state(RawInputModifiers::CONTROL | RawInputModifiers::META),
            FcitxKeyState::CTRL | FcitxKeyState::SUPER
        );
        assert_eq!(fcitx_state(RawInputModifiers::ALT | RawInputModifiers::SHIFT), FcitxKeyState::ALT_SHIFT);

        let key = forwarded_key(0x61, FcitxKeyState::CTRL_ALT.bits(), 0);
        assert_eq!(key.key_val, 0x61);
        assert_eq!(key.type_, RawKeyEventType::KeyDown);
        assert_eq!(key.modifiers, KeyModifiers::CONTROL | KeyModifiers::ALT);
        // A press carries text, a release does not.
        assert!(key.with_text);
        let key = forwarded_key(0x61, FcitxKeyState::SUPER.bits(), 1);
        assert_eq!(key.type_, RawKeyEventType::KeyUp);
        assert_eq!(key.modifiers, KeyModifiers::META);
        assert!(!key.with_text);
    }
}

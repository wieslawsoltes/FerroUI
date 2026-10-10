//! The input method of IBus, through its portal (the port of
//! `IBusX11TextInputMethod.cs`).

use super::dbus::{InputContextProxy, PortalProxy, ServiceProxy};
use super::ibus_enums::{IBusCapability, IBusModifierMask};
use crate::dbus_call_queue::DBusResult;
use crate::dbus_ime::dbus_text_input_method_base::{DBusTextInputMethodBase, DBusTextInputMethodCore};
use crate::ix11_input_method::X11InputMethodForwardedKey;
use crate::signal_watch::watch_stream;
use ferroui_base::input::raw::{IRawInputEventArgs, RawKeyEventArgs, RawKeyEventType};
use ferroui_base::input::text_input::TextInputOptions;
use ferroui_base::input::{KeyModifiers, LocalBoxFuture, RawInputModifiers};
use ferroui_base::PixelRect;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use zbus::proxy::CacheProperties;
use zbus::zvariant::Value;
use zbus::Connection;

pub struct IBusX11TextInputMethod {
    base: Weak<DBusTextInputMethodBase>,
    weak_self: Weak<IBusX11TextInputMethod>,
    service: RefCell<Option<ServiceProxy<'static>>>,
    context: RefCell<Option<InputContextProxy<'static>>>,
    preedit_text: RefCell<String>,
    preedit_cursor: Cell<i32>,
    inside_reset: Cell<i32>,
}

/// The text of an `IBusText` as it travels in a variant: a structure
/// whose third member is the string.
pub(crate) fn ibus_text(value: &Value<'_>) -> Option<String> {
    fn unwrap<'a, 'v>(mut value: &'a Value<'v>) -> &'a Value<'v> {
        while let Value::Value(inner) = value {
            value = inner;
        }
        value
    }

    let Value::Structure(structure) = unwrap(value) else {
        return None;
    };
    let fields = structure.fields();
    if fields.len() < 3 {
        return None;
    }
    match unwrap(&fields[2]) {
        Value::Str(text) => Some(text.as_str().to_string()),
        _ => None,
    }
}

/// The offset in UTF-16 code units of an offset in characters, at most
/// the length of the text (`Utf16Utils.CharacterOffsetToStringOffset`
/// without the range check).
pub(crate) fn character_offset_to_string_offset(text: &str, off: usize) -> i32 {
    text.chars().take(off).map(char::len_utf16).sum::<usize>() as i32
}

/// The modifier bits IBus is given with a key (`HandleKeyCore`).
pub(crate) fn ibus_state(modifiers: RawInputModifiers, type_: RawKeyEventType) -> IBusModifierMask {
    let mut state = IBusModifierMask::empty();
    if modifiers.contains(RawInputModifiers::CONTROL) {
        state |= IBusModifierMask::CONTROL_MASK;
    }
    if modifiers.contains(RawInputModifiers::ALT) {
        state |= IBusModifierMask::MOD1_MASK;
    }
    if modifiers.contains(RawInputModifiers::SHIFT) {
        state |= IBusModifierMask::SHIFT_MASK;
    }
    if modifiers.contains(RawInputModifiers::META) {
        state |= IBusModifierMask::MOD4_MASK;
    }

    if type_ == RawKeyEventType::KeyUp {
        state |= IBusModifierMask::RELEASE_MASK;
    }

    state
}

/// The key IBus forwards, as the window takes it (`OnForwardKey`).
pub(crate) fn forwarded_key(keyval: u32, state: u32) -> X11InputMethodForwardedKey {
    let state = IBusModifierMask::from_bits_retain(state);
    let mut mods = KeyModifiers::empty();
    if state.contains(IBusModifierMask::CONTROL_MASK) {
        mods |= KeyModifiers::CONTROL;
    }
    if state.contains(IBusModifierMask::MOD1_MASK) {
        mods |= KeyModifiers::ALT;
    }
    if state.contains(IBusModifierMask::SHIFT_MASK) {
        mods |= KeyModifiers::SHIFT;
    }
    if state.contains(IBusModifierMask::MOD4_MASK) {
        mods |= KeyModifiers::META;
    }

    X11InputMethodForwardedKey {
        key_val: keyval as i32,
        type_: if state.contains(IBusModifierMask::RELEASE_MASK) {
            RawKeyEventType::KeyUp
        } else {
            RawKeyEventType::KeyDown
        },
        modifiers: mods,
        with_text: false,
    }
}

impl IBusX11TextInputMethod {
    /// The input method of a window, watching the name of the IBus portal
    /// on `connection`.
    #[allow(clippy::new_ret_no_self)]
    pub fn new(connection: Connection) -> Rc<DBusTextInputMethodBase> {
        DBusTextInputMethodBase::new(connection, &["org.freedesktop.portal.IBus"], |base| {
            Rc::new_cyclic(|weak_self| IBusX11TextInputMethod {
                base,
                weak_self: weak_self.clone(),
                service: RefCell::new(None),
                context: RefCell::new(None),
                preedit_text: RefCell::new(String::new()),
                preedit_cursor: Cell::new(0),
                inside_reset: Cell::new(0),
            })
        })
    }

    fn context(&self) -> Option<InputContextProxy<'static>> {
        self.context.borrow().clone()
    }

    fn supports_preedit(&self) -> bool {
        self.base.upgrade().and_then(|base| base.client()).is_some_and(|client| client.supports_preedit())
    }

    fn on_hide_preedit(&self) {
        if !self.supports_preedit() || self.preedit_text.borrow().is_empty() {
            return;
        }

        self.preedit_text.borrow_mut().clear();
        if let Some(client) = self.base.upgrade().and_then(|base| base.client()) {
            client.set_preedit_text_with_cursor(Some(""), Some(0));
        }
    }

    fn on_show_preedit(&self) {}

    fn on_update_preedit(&self, text: &Value<'_>, cursor_pos: u32, _visible: bool) {
        let preedit_text = ibus_text(text).unwrap_or_default();

        if !self.supports_preedit() || preedit_text == *self.preedit_text.borrow() {
            return;
        }

        self.preedit_cursor.set(if !preedit_text.is_empty() {
            character_offset_to_string_offset(&preedit_text, cursor_pos.min(i32::MAX as u32) as usize)
        } else {
            0
        });
        *self.preedit_text.borrow_mut() = preedit_text.clone();

        if let Some(client) = self.base.upgrade().and_then(|base| base.client()) {
            client.set_preedit_text_with_cursor(Some(&preedit_text), Some(self.preedit_cursor.get()));
        }
    }

    fn on_forward_key(&self, keyval: u32, _keycode: u32, state: u32) {
        if let Some(base) = self.base.upgrade() {
            base.fire_forward(forwarded_key(keyval, state));
        }
    }

    fn on_commit_text(&self, variant_item: &Value<'_>) {
        if self.inside_reset.get() > 0 {
            // For some reason iBus can trigger a CommitText while being reset.
            // Thankfully the signal is sent _during_ Reset call processing,
            // so it arrives on-the-wire before Reset call result, so we can
            // check if we have any pending Reset calls and ignore the signal here
            return;
        }

        if let Some(text) = ibus_text(variant_item) {
            if let Some(base) = self.base.upgrade() {
                base.fire_commit(text);
            }
        }
    }
}

impl DBusTextInputMethodCore for IBusX11TextInputMethod {
    fn connect(&self, name: String) -> LocalBoxFuture<DBusResult<bool>> {
        let (this, base) = (self.weak_self.clone(), self.base.clone());
        Box::pin(async move {
            let (Some(this), Some(base)) = (this.upgrade(), base.upgrade()) else {
                return Ok(false);
            };
            let connection = base.connection().clone();

            let portal = PortalProxy::builder(&connection)
                .destination(name.clone())?
                .path("/org/freedesktop/IBus")?
                .cache_properties(CacheProperties::No)
                .build()
                .await?;
            let path = portal.create_input_context(&base.get_app_name()).await?;

            let service = ServiceProxy::builder(&connection)
                .destination(name.clone())?
                .path(path.clone().into_inner())?
                .cache_properties(CacheProperties::No)
                .build()
                .await?;
            let context = InputContextProxy::builder(&connection)
                .destination(name)?
                .path(path.into_inner())?
                .cache_properties(CacheProperties::No)
                .build()
                .await?;
            *this.service.borrow_mut() = Some(service);
            *this.context.borrow_mut() = Some(context.clone());

            let weak = Rc::downgrade(&this);
            base.add_disposable(watch_stream(context.receive_commit_text().await?, {
                let weak = weak.clone();
                move |signal| {
                    if let (Some(this), Ok(args)) = (weak.upgrade(), signal.args()) {
                        this.on_commit_text(args.text());
                    }
                }
            }));
            base.add_disposable(watch_stream(context.receive_forward_key_event().await?, {
                let weak = weak.clone();
                move |signal| {
                    if let (Some(this), Ok(args)) = (weak.upgrade(), signal.args()) {
                        this.on_forward_key(*args.keyval(), *args.keycode(), *args.state());
                    }
                }
            }));
            base.add_disposable(watch_stream(context.receive_update_preedit_text().await?, {
                let weak = weak.clone();
                move |signal| {
                    if let (Some(this), Ok(args)) = (weak.upgrade(), signal.args()) {
                        this.on_update_preedit(args.text(), *args.cursor_pos(), *args.visible());
                    }
                }
            }));
            base.add_disposable(watch_stream(context.receive_show_preedit_text().await?, {
                let weak = weak.clone();
                move |_| {
                    if let Some(this) = weak.upgrade() {
                        this.on_show_preedit();
                    }
                }
            }));
            base.add_disposable(watch_stream(context.receive_hide_preedit_text().await?, {
                let weak = weak.clone();
                move |_| {
                    if let Some(this) = weak.upgrade() {
                        this.on_hide_preedit();
                    }
                }
            }));
            base.enqueue(move || async move {
                context.set_capabilities(IBusCapability::CAP_FOCUS.bits()).await?;
                Ok(())
            });
            Ok(true)
        })
    }

    fn disconnect_async(&self) -> LocalBoxFuture<DBusResult> {
        let service = self.service.borrow().clone();
        Box::pin(async move {
            if let Some(service) = service {
                service.destroy().await?;
            }
            Ok(())
        })
    }

    fn on_disconnected(&self) {
        *self.service.borrow_mut() = None;
        *self.context.borrow_mut() = None;
    }

    fn set_cursor_rect_core(&self, rect: PixelRect) -> LocalBoxFuture<DBusResult> {
        let context = self.context();
        Box::pin(async move {
            if let Some(context) = context {
                context.set_cursor_location(rect.x, rect.y, rect.width, rect.height).await?;
            }
            Ok(())
        })
    }

    fn set_active_core(&self, active: bool) -> LocalBoxFuture<DBusResult> {
        let context = self.context();
        Box::pin(async move {
            if let Some(context) = context {
                if active {
                    context.focus_in().await?;
                } else {
                    context.focus_out().await?;
                }
            }
            Ok(())
        })
    }

    fn reset_context_core(&self) -> LocalBoxFuture<DBusResult> {
        let (this, context) = (self.weak_self.clone(), self.context());
        Box::pin(async move {
            let (Some(this), Some(context)) = (this.upgrade(), context) else {
                return Ok(());
            };

            this.inside_reset.set(this.inside_reset.get() + 1);
            let result = context.reset().await;
            this.inside_reset.set(this.inside_reset.get() - 1);
            result?;
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
            let state = ibus_state(key.modifiers(), key.type_());

            match context {
                Some(context) => Ok(context.process_key_event(key_val as u32, key_code as u32, state.bits()).await?),
                None => Ok(false),
            }
        })
    }

    fn set_options(&self, _options: &TextInputOptions) {
        // No-op, because ibus
    }

    fn set_capabilities_core(
        &self,
        supports_preedit: bool,
        _supports_surrounding_text: bool,
    ) -> LocalBoxFuture<DBusResult> {
        let context = self.context();
        Box::pin(async move {
            let mut caps = IBusCapability::CAP_FOCUS;
            if supports_preedit {
                caps |= IBusCapability::CAP_PREEDIT_TEXT;
            }

            if let Some(context) = context {
                context.set_capabilities(caps.bits()).await?;
            }
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests of this class. The
    // input method against a service is tested in `dbus_ime/tests.rs`.
    use super::*;
    use zbus::zvariant::{Array, Structure};

    fn ibus_text_value(text: &str) -> Value<'static> {
        // IBusText: (sa{sv}sv): the type name, attachments, the text, the
        // attribute list.
        let attachments: std::collections::HashMap<String, Value<'static>> = std::collections::HashMap::new();
        Value::Structure(Structure::from((
            "IBusText".to_string(),
            attachments,
            text.to_string(),
            Value::from(Array::from(Vec::<u32>::new())),
        )))
    }

    #[test]
    fn the_text_of_an_ibus_text_is_its_third_member() {
        assert_eq!(ibus_text(&ibus_text_value("にほん")).as_deref(), Some("にほん"));
        // Wrapped once more, as a variant in a variant.
        assert_eq!(ibus_text(&Value::Value(Box::new(ibus_text_value("a")))).as_deref(), Some("a"));
        // Not a structure, too short, or a third member of another type.
        assert_eq!(ibus_text(&Value::from("text")), None);
        assert_eq!(ibus_text(&Value::Structure(Structure::from(("a".to_string(), "b".to_string())))), None);
        assert_eq!(ibus_text(&Value::Structure(Structure::from(("a".to_string(), 1u32, 2u32)))), None);
    }

    #[test]
    fn the_cursor_of_a_preedit_counts_characters_and_is_reported_in_code_units() {
        // "a", a character outside the basic plane (two code units), "b".
        let text = "a\u{1F600}b";
        assert_eq!(character_offset_to_string_offset(text, 0), 0);
        assert_eq!(character_offset_to_string_offset(text, 1), 1);
        assert_eq!(character_offset_to_string_offset(text, 2), 3);
        assert_eq!(character_offset_to_string_offset(text, 3), 4);
        // Beyond the end: the length.
        assert_eq!(character_offset_to_string_offset(text, 99), 4);
    }

    #[test]
    fn modifiers_and_the_release_go_to_ibus_as_its_mask() {
        assert_eq!(ibus_state(RawInputModifiers::empty(), RawKeyEventType::KeyDown), IBusModifierMask::empty());
        assert_eq!(
            ibus_state(RawInputModifiers::CONTROL | RawInputModifiers::SHIFT, RawKeyEventType::KeyDown),
            IBusModifierMask::CONTROL_MASK | IBusModifierMask::SHIFT_MASK
        );
        assert_eq!(
            ibus_state(RawInputModifiers::ALT | RawInputModifiers::META, RawKeyEventType::KeyUp),
            IBusModifierMask::MOD1_MASK | IBusModifierMask::MOD4_MASK | IBusModifierMask::RELEASE_MASK
        );
        assert_eq!(IBusModifierMask::RELEASE_MASK.bits(), 0x4000_0000);
    }

    #[test]
    fn a_forwarded_key_carries_its_modifiers_and_whether_it_is_a_release() {
        let key = forwarded_key(0xff0d, (IBusModifierMask::CONTROL_MASK | IBusModifierMask::MOD1_MASK).bits());
        assert_eq!(key.key_val, 0xff0d);
        assert_eq!(key.type_, RawKeyEventType::KeyDown);
        assert_eq!(key.modifiers, KeyModifiers::CONTROL | KeyModifiers::ALT);
        assert!(!key.with_text);

        let key = forwarded_key(0x61, (IBusModifierMask::RELEASE_MASK | IBusModifierMask::SHIFT_MASK).bits());
        assert_eq!(key.type_, RawKeyEventType::KeyUp);
        assert_eq!(key.modifiers, KeyModifiers::SHIFT);
    }
}

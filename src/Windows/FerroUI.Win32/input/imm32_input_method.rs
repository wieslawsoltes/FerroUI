//! A Windows input method editor based on Windows Input Method Manager
//! (IMM32) (the port of `Input/Imm32InputMethod.cs`).
//!
//! The input method is written against the calls it makes of the system
//! ([`ImmSystem`]) and against what it needs of the window it serves
//! ([`Imm32Parent`]), so that what it does with a composition is tested on
//! every host: on Windows the first is the system and the second the
//! window implementation.

use super::imm32_caret_manager::Imm32CaretManager;
use crate::interop::unmanaged_methods::{
    lgid, primary_lang_id, CANDIDATEFORM, CFS_EXCLUDE, CFS_POINT, COMPOSITIONFORM, CPS_COMPLETE, GCS, LANG_KO,
    NI_COMPOSITIONSTR, POINT, RECT,
};
use ferroui_base::input::text_input::{ITextInputMethodImpl, TextInputMethodClient, TextInputOptions};
use ferroui_base::input::{Key, PhysicalKey};
use ferroui_base::Rect;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// The calls the input method makes of the system.
pub(crate) trait ImmSystem {
    fn imm_get_context(&self, hwnd: isize) -> isize;
    fn imm_create_context(&self) -> isize;
    fn imm_associate_context(&self, hwnd: isize, himc: isize);
    fn imm_release_context(&self, hwnd: isize, himc: isize);
    fn imm_notify_ime(&self, himc: isize, action: i32, index: i32, value: i32);
    fn imm_set_candidate_window(&self, himc: isize, candidate: &CANDIDATEFORM);
    fn imm_set_composition_window(&self, himc: isize, form: &COMPOSITIONFORM);
    fn imm_set_composition_font(&self, himc: isize, height: i32, quality: u8);
    /// A string of the composition; `None` when there is none.
    fn imm_get_composition_string(&self, himc: isize, flag: u32) -> Option<String>;
    /// A number of the composition (the cursor position); negative for an
    /// error.
    fn imm_get_composition_value(&self, himc: isize, flag: u32) -> i32;
    fn create_caret(&self, hwnd: isize, width: i32, height: i32) -> bool;
    fn set_caret_pos(&self, x: i32, y: i32);
    fn destroy_caret(&self);
    fn get_active_window(&self) -> isize;
    /// Runs an action later on the UI thread (`Dispatcher.UIThread.Post`).
    fn post(&self, action: Box<dyn FnOnce()>);
}

/// What the input method needs of the window it serves (the reference
/// holds the window implementation).
pub(crate) trait Imm32Parent {
    fn desktop_scaling(&self) -> f64;
    /// The next `WM_CHAR` message of the window is not text.
    fn set_ignore_wm_char(&self, value: bool);
    /// Raw text input for the window. False when the window has no input
    /// callback.
    fn raw_text_input(&self, timestamp: u64, text: &str) -> bool;
    /// A key pressed and released, without modifiers, for the window.
    fn raw_key_press(&self, key: Key, physical_key: PhysicalKey);
}

const CARET_MARGIN: i32 = 1;

/// The rectangle of the text cursor in device pixels of the client area:
/// its two corners scaled and truncated.
pub(crate) fn cursor_rect_in_pixels(rect: Rect, scaling: f64) -> (i32, i32, i32, i32) {
    let p1 = rect.top_left();
    let p2 = rect.bottom_right();
    ((p1.x * scaling) as i32, (p1.y * scaling) as i32, (p2.x * scaling) as i32, (p2.y * scaling) as i32)
}

/// The rectangle the candidate window of an input method keeps clear of,
/// for a text cursor in pixels and the primary language of the keyboard
/// layout.
// Chinese, Japanese, and Korean(CJK) IMEs also use the rectangle given to
// ::ImmSetCandidateWindow() with its 'dwStyle' parameter CFS_EXCLUDE
// to move their candidate windows when a user disables TSF and CUAS.
// Therefore, we also set this parameter here.
pub(crate) fn candidate_exclude_rectangle(x1: i32, y1: i32, x2: i32, mut y2: i32, lang_id: u16) -> CANDIDATEFORM {
    if i32::from(lang_id) == LANG_KO {
        // Chinese IMEs and Japanese IMEs require the upper-left corner of
        // the caret to move the position of their candidate windows.
        // On the other hand, Korean IMEs require the lower-left corner of the
        // caret to move their candidate windows.
        y2 += CARET_MARGIN;
    }

    CANDIDATEFORM {
        dwIndex: 0,
        dwStyle: CFS_EXCLUDE,
        ptCurrentPos: POINT { x: x1, y: y1 },
        rcArea: RECT { left: x1, top: y1, right: x2, bottom: y2 + CARET_MARGIN },
    }
}

/// The low 32 bits of a message parameter (`ToInt32`).
fn to_int32(ptr: isize) -> i32 {
    (ptr as i64 & 0xffff_ffff) as u32 as i32
}

/// A Windows input method editor based on Windows Input Method Manager
/// (IMM32).
pub(crate) struct Imm32InputMethod {
    this: Weak<Imm32InputMethod>,
    system: Rc<dyn ImmSystem>,
    hwnd: Cell<isize>,
    current_himc: Cell<isize>,
    parent: RefCell<Option<Weak<dyn Imm32Parent>>>,
    caret_manager: RefCell<Imm32CaretManager>,
    lang_id: Cell<u16>,
    ignore_composition: Cell<bool>,
    composition_cursor_position: Cell<Option<i32>>,
    client: RefCell<Option<Rc<dyn TextInputMethodClient>>>,
    is_composing: Cell<bool>,
    composition: RefCell<Option<String>>,
}

impl Imm32InputMethod {
    pub fn new(system: Rc<dyn ImmSystem>) -> Rc<Imm32InputMethod> {
        Rc::new_cyclic(|this| Imm32InputMethod {
            this: this.clone(),
            system,
            hwnd: Cell::new(0),
            current_himc: Cell::new(0),
            parent: RefCell::new(None),
            caret_manager: RefCell::new(Imm32CaretManager::default()),
            lang_id: Cell::new(0),
            ignore_composition: Cell::new(false),
            composition_cursor_position: Cell::new(None),
            client: RefCell::new(None),
            is_composing: Cell::new(false),
            composition: RefCell::new(None),
        })
    }

    /// The window the input method serves; 0 for none (`Hwnd`).
    pub fn hwnd(&self) -> isize {
        self.hwnd.get()
    }

    pub fn client(&self) -> Option<Rc<dyn TextInputMethodClient>> {
        self.client.borrow().clone()
    }

    pub fn is_active(&self) -> bool {
        self.client.borrow().is_some()
    }

    pub fn is_composing(&self) -> bool {
        self.is_composing.get()
    }

    #[allow(dead_code)] // A member of the reference without a caller there.
    pub fn set_is_composing(&self, value: bool) {
        self.is_composing.set(value);
    }

    pub fn show_composition_window(&self) -> bool {
        false
    }

    /// The composition string as it was last reported (`Composition`).
    pub fn composition(&self) -> Option<String> {
        self.composition.borrow().clone()
    }

    fn parent(&self) -> Option<Rc<dyn Imm32Parent>> {
        self.parent.borrow().as_ref().and_then(Weak::upgrade)
    }

    #[allow(dead_code)] // A member of the reference without a caller there.
    pub fn create_caret(&self) {
        self.caret_manager.borrow_mut().try_create(&*self.system, self.hwnd.get());
    }

    pub fn enable_imm(&self) {
        let hwnd = self.hwnd.get();
        let mut himc = self.system.imm_get_context(hwnd);

        if himc == 0 {
            himc = self.system.imm_create_context();
        }

        if himc != self.current_himc.get() {
            if self.current_himc.get() != 0 {
                self.disable_imm();
            }

            self.system.imm_associate_context(hwnd, himc);

            self.system.imm_release_context(hwnd, himc);

            self.current_himc.set(himc);

            self.caret_manager.borrow_mut().try_create(&*self.system, hwnd);
        }
    }

    pub fn disable_imm(&self) {
        self.caret_manager.borrow_mut().try_destroy(&*self.system);

        ITextInputMethodImpl::reset(self);

        self.system.imm_associate_context(self.hwnd.get(), 0);

        self.caret_manager.borrow_mut().try_destroy(&*self.system);

        self.current_himc.set(0);
    }

    pub fn set_language_and_window(&self, parent: Weak<dyn Imm32Parent>, hwnd: isize, hkl: isize) {
        self.hwnd.set(hwnd);
        *self.parent.borrow_mut() = Some(parent);

        let lang_id = primary_lang_id(lgid(hkl));

        if self.is_active() && lang_id != self.lang_id.get() {
            self.disable_imm();
            self.enable_imm();
        }

        self.lang_id.set(lang_id);
    }

    pub fn clear_language_and_window(&self) {
        self.disable_imm();

        self.hwnd.set(0);
        *self.parent.borrow_mut() = None;
        *self.client.borrow_mut() = None;
        self.lang_id.set(0);

        self.is_composing.set(false);
        self.composition_cursor_position.set(None);
    }

    // see: https://chromium.googlesource.com/experimental/chromium/src/+/bf09a5036ccfb77d2277247c66dc55daf41df3fe/chrome/browser/ime_input.cc
    // see: https://engine.chinmaygarde.com/window__win32_8cc_source.html
    fn move_ime_window(&self, rect: Rect, himc: isize) {
        let s = self.parent().map_or(1.0, |parent| parent.desktop_scaling());
        let (x1, y1, x2, y2) = cursor_rect_in_pixels(rect, s);

        self.caret_manager.borrow_mut().try_move(&*self.system, x2, y2);

        if self.show_composition_window() {
            self.configure_composition_window(x1, y1, himc, y2 - y1);
            // Don't need to set the position of candidate window.
            return;
        }

        let exclude_rectangle = candidate_exclude_rectangle(x1, y1, x2, y2, self.lang_id.get());

        self.system.imm_set_candidate_window(himc, &exclude_rectangle);
    }

    fn configure_composition_window(&self, x1: i32, y1: i32, himc: isize, height: i32) {
        let comp_form = COMPOSITIONFORM { dwStyle: CFS_POINT, ptCurrentPos: POINT { x: x1, y: y1 }, ..Default::default() };

        self.system.imm_set_composition_window(himc, &comp_form);

        // lfQuality: 5, CLEARTYPE_QUALITY
        self.system.imm_set_composition_font(himc, height, 5);
    }

    pub fn composition_changed(&self, composition: Option<String>, cursor_position: Option<i32>) {
        *self.composition.borrow_mut() = composition.clone();
        self.composition_cursor_position.set(cursor_position);

        let Some(client) = self.client() else {
            return;
        };
        if !client.supports_preedit() {
            return;
        }

        client.set_preedit_text_with_cursor(composition.as_deref(), cursor_position);
    }

    pub fn get_composition_string(&self, flag: u32) -> Option<String> {
        if !self.is_composing.get() {
            return None;
        }

        // As in the reference, the context is not given back here.
        let himc = self.system.imm_get_context(self.hwnd.get());

        self.system.imm_get_composition_string(himc, flag)
    }

    fn get_composition_cursor_position(&self) -> Option<i32> {
        if !self.is_composing.get() {
            return None;
        }

        let hwnd = self.hwnd.get();
        let himc = self.system.imm_get_context(hwnd);

        if himc == 0 {
            return None;
        }

        let cursor_position = self.system.imm_get_composition_value(himc, GCS::GCS_CURSORPOS);
        self.system.imm_release_context(hwnd, himc);

        (cursor_position >= 0).then_some(cursor_position)
    }

    pub fn handle_composition_start(&self) {
        *self.composition.borrow_mut() = None;
        self.composition_cursor_position.set(None);

        if let Some(client) = self.client() {
            client.set_preedit_text_with_cursor(None, None);

            let selection = client.selection();
            if client.supports_surrounding_text() && selection.start != selection.end {
                self.key_press(Key::Delete, PhysicalKey::Delete);
            }
        }

        self.is_composing.set(true);
    }

    pub fn handle_composition_end(&self, timestamp: u64) {
        //Cleanup composition state.
        self.is_composing.set(false);

        let composition = self.composition();
        if let (Some(parent), Some(composition)) = (self.parent(), composition.filter(|text| !text.is_empty())) {
            if parent.raw_text_input(timestamp, &composition) {
                parent.set_ignore_wm_char(true);
            }
        }

        *self.composition.borrow_mut() = None;
        self.composition_cursor_position.set(None);

        if let Some(client) = self.client() {
            client.set_preedit_text_with_cursor(None, None);
        }
    }

    pub fn handle_composition(&self, _w_param: usize, l_param: isize, timestamp: u64) {
        if self.ignore_composition.replace(false) {
            return;
        }

        let flags = to_int32(l_param) as u32;
        let result_changed = (flags & GCS::GCS_RESULTSTR) != 0;

        if flags == 0 {
            self.composition_changed(Some(String::new()), None);
        }

        if result_changed {
            let result_string = self.get_composition_string(GCS::GCS_RESULTSTR);

            *self.composition.borrow_mut() = None;
            self.composition_cursor_position.set(None);

            if let Some(client) = self.client() {
                client.set_preedit_text_with_cursor(None, None);
            }

            if let (Some(parent), Some(result_string)) = (self.parent(), result_string.filter(|text| !text.is_empty())) {
                if parent.raw_text_input(timestamp, &result_string) {
                    parent.set_ignore_wm_char(true);
                }
            }
        }

        let composition_changed = (flags & GCS::GCS_COMPSTR) != 0;
        let cursor_position_changed = (flags & GCS::GCS_CURSORPOS) != 0;

        if composition_changed || (cursor_position_changed && !result_changed) {
            let composition_string =
                if composition_changed { self.get_composition_string(GCS::GCS_COMPSTR) } else { self.composition() };

            let cursor_position = if cursor_position_changed {
                self.get_composition_cursor_position()
            } else {
                self.composition_cursor_position.get()
            };

            self.composition_changed(composition_string, cursor_position);
        }
    }

    fn key_press(&self, key: Key, physical_key: PhysicalKey) {
        if let Some(parent) = self.parent() {
            parent.raw_key_press(key, physical_key);
        }
    }
}

impl ITextInputMethodImpl for Imm32InputMethod {
    fn set_client(&self, client: Option<Rc<dyn TextInputMethodClient>>) {
        if let Some(old) = self.client() {
            *self.composition.borrow_mut() = None;
            self.composition_cursor_position.set(None);

            old.set_preedit_text_with_cursor(None, None);
        }

        *self.client.borrow_mut() = client;

        let this = self.this.clone();
        self.system.post(Box::new(move || {
            let Some(this) = this.upgrade() else {
                return;
            };
            if this.is_active() {
                this.enable_imm();
            } else {
                // A renderer process have moved its input focus to a password input
                // when there is an ongoing composition, e.g. a user has clicked a
                // mouse button and selected a password input while composing a text.
                // For this case, we have to complete the ongoing composition and
                // clean up the resources attached to this object BEFORE DISABLING THE IME.

                this.disable_imm();
            }
        }));
    }

    fn set_cursor_rect(&self, rect: Rect) {
        let focused = self.system.get_active_window() == self.hwnd.get();

        if !focused {
            return;
        }

        let this = self.this.clone();
        self.system.post(Box::new(move || {
            let Some(this) = this.upgrade() else {
                return;
            };
            let hwnd = this.hwnd.get();
            let himc = this.system.imm_get_context(hwnd);

            if himc == 0 {
                return;
            }

            this.move_ime_window(rect, himc);

            this.system.imm_release_context(hwnd, himc);
        }));
    }

    fn set_options(&self, _options: &TextInputOptions) {
        // we're skipping this. not usable on windows
    }

    fn reset(&self) {
        self.composition_cursor_position.set(None);

        let this = self.this.clone();
        self.system.post(Box::new(move || {
            let Some(this) = this.upgrade() else {
                return;
            };
            let hwnd = this.hwnd.get();
            let himc = this.system.imm_get_context(hwnd);

            if himc != 0 {
                if this.is_composing.get() {
                    this.ignore_composition.set(true);
                }

                if let Some(parent) = this.parent() {
                    parent.set_ignore_wm_char(true);
                }

                this.system.imm_notify_ime(himc, NI_COMPOSITIONSTR, CPS_COMPLETE, 0);

                this.system.imm_release_context(hwnd, himc);

                this.is_composing.set(false);

                *this.composition.borrow_mut() = None;
                this.composition_cursor_position.set(None);
            }
        }));
    }
}

impl Drop for Imm32InputMethod {
    fn drop(&mut self) {
        self.caret_manager.borrow_mut().try_destroy(&*self.system);
    }
}

#[cfg(windows)]
#[allow(unused_imports)]
pub(crate) use imp::Win32ImmSystem;

#[cfg(windows)]
mod imp {
    use super::{Imm32InputMethod, ImmSystem};
    use crate::interop::unmanaged_methods as native;
    use crate::interop::unmanaged_methods::{CANDIDATEFORM, COMPOSITIONFORM};
    use ferroui_base::threading::{Dispatcher, DispatcherPriority};
    use std::rc::Rc;

    /// The system.
    pub(crate) struct Win32ImmSystem;

    impl ImmSystem for Win32ImmSystem {
        fn imm_get_context(&self, hwnd: isize) -> isize {
            native::imm_get_context(hwnd)
        }

        fn imm_create_context(&self) -> isize {
            native::imm_create_context()
        }

        fn imm_associate_context(&self, hwnd: isize, himc: isize) {
            native::imm_associate_context(hwnd, himc);
        }

        fn imm_release_context(&self, hwnd: isize, himc: isize) {
            native::imm_release_context(hwnd, himc);
        }

        fn imm_notify_ime(&self, himc: isize, action: i32, index: i32, value: i32) {
            native::imm_notify_ime(himc, action, index, value);
        }

        fn imm_set_candidate_window(&self, himc: isize, candidate: &CANDIDATEFORM) {
            native::imm_set_candidate_window(himc, candidate);
        }

        fn imm_set_composition_window(&self, himc: isize, form: &COMPOSITIONFORM) {
            native::imm_set_composition_window(himc, form);
        }

        fn imm_set_composition_font(&self, himc: isize, height: i32, quality: u8) {
            native::imm_set_composition_font(himc, height, quality);
        }

        fn imm_get_composition_string(&self, himc: isize, flag: u32) -> Option<String> {
            native::imm_get_composition_string(himc, flag)
        }

        fn imm_get_composition_value(&self, himc: isize, flag: u32) -> i32 {
            native::imm_get_composition_value(himc, flag)
        }

        fn create_caret(&self, hwnd: isize, width: i32, height: i32) -> bool {
            native::create_caret(hwnd, width, height)
        }

        fn set_caret_pos(&self, x: i32, y: i32) {
            native::set_caret_pos(x, y);
        }

        fn destroy_caret(&self) {
            native::destroy_caret();
        }

        fn get_active_window(&self) -> isize {
            native::get_active_window()
        }

        fn post(&self, action: Box<dyn FnOnce()>) {
            Dispatcher::ui_thread().post_local(action, DispatcherPriority::default());
        }
    }

    impl Imm32InputMethod {
        //Dependant on CurrentThread. When the toolkit will support Multiple Dispatchers -
        //every Dispatcher should have their own InputMethod.
        /// The input method of the calling thread (`Current`).
        pub fn current() -> Rc<Imm32InputMethod> {
            thread_local! {
                static CURRENT: Rc<Imm32InputMethod> = Imm32InputMethod::new(Rc::new(Win32ImmSystem));
            }
            CURRENT.with(Rc::clone)
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    // Not from upstream: the reference has no tests of the input method.
    use super::*;
    use ferroui_base::input::text_input::{TextInputMethodClientEvents, TextSelection};
    use ferroui_base::{Ref, Visual};

    #[derive(Clone, Debug, PartialEq)]
    pub(crate) enum Call {
        GetContext(isize),
        CreateContext,
        AssociateContext(isize, isize),
        ReleaseContext(isize, isize),
        NotifyIme(isize, i32, i32, i32),
        SetCandidateWindow(isize, CANDIDATEFORM),
        SetCompositionWindow(isize, COMPOSITIONFORM),
        SetCompositionFont(isize, i32, u8),
        CreateCaret(isize, i32, i32),
        SetCaretPos(i32, i32),
        DestroyCaret,
    }

    /// The system as a record of the calls made of it, with the answers a
    /// test gives it.
    pub(crate) struct FakeSystem {
        pub calls: RefCell<Vec<Call>>,
        pub posted: RefCell<Vec<Box<dyn FnOnce()>>>,
        /// The input context of every window; 0 for none.
        pub context: Cell<isize>,
        pub active_window: Cell<isize>,
        pub refuse_caret: Cell<bool>,
        pub composition_string: RefCell<Option<String>>,
        pub result_string: RefCell<Option<String>>,
        pub cursor: Cell<i32>,
    }

    impl FakeSystem {
        pub fn new() -> Rc<FakeSystem> {
            Rc::new(FakeSystem {
                calls: RefCell::new(Vec::new()),
                posted: RefCell::new(Vec::new()),
                context: Cell::new(0x1C),
                active_window: Cell::new(0),
                refuse_caret: Cell::new(false),
                composition_string: RefCell::new(None),
                result_string: RefCell::new(None),
                cursor: Cell::new(-1),
            })
        }

        pub fn take_calls(&self) -> Vec<Call> {
            std::mem::take(&mut *self.calls.borrow_mut())
        }

        /// Runs what was posted, and what that posts.
        pub fn run_posted(&self) {
            loop {
                let posted = std::mem::take(&mut *self.posted.borrow_mut());
                if posted.is_empty() {
                    return;
                }
                for action in posted {
                    action();
                }
            }
        }

        fn record(&self, call: Call) {
            self.calls.borrow_mut().push(call);
        }
    }

    impl ImmSystem for FakeSystem {
        fn imm_get_context(&self, hwnd: isize) -> isize {
            self.record(Call::GetContext(hwnd));
            self.context.get()
        }

        fn imm_create_context(&self) -> isize {
            self.record(Call::CreateContext);
            0x2C
        }

        fn imm_associate_context(&self, hwnd: isize, himc: isize) {
            self.record(Call::AssociateContext(hwnd, himc));
        }

        fn imm_release_context(&self, hwnd: isize, himc: isize) {
            self.record(Call::ReleaseContext(hwnd, himc));
        }

        fn imm_notify_ime(&self, himc: isize, action: i32, index: i32, value: i32) {
            self.record(Call::NotifyIme(himc, action, index, value));
        }

        fn imm_set_candidate_window(&self, himc: isize, candidate: &CANDIDATEFORM) {
            self.record(Call::SetCandidateWindow(himc, *candidate));
        }

        fn imm_set_composition_window(&self, himc: isize, form: &COMPOSITIONFORM) {
            self.record(Call::SetCompositionWindow(himc, *form));
        }

        fn imm_set_composition_font(&self, himc: isize, height: i32, quality: u8) {
            self.record(Call::SetCompositionFont(himc, height, quality));
        }

        fn imm_get_composition_string(&self, _himc: isize, flag: u32) -> Option<String> {
            match flag {
                GCS::GCS_COMPSTR => self.composition_string.borrow().clone(),
                GCS::GCS_RESULTSTR => self.result_string.borrow().clone(),
                _ => None,
            }
        }

        fn imm_get_composition_value(&self, _himc: isize, _flag: u32) -> i32 {
            self.cursor.get()
        }

        fn create_caret(&self, hwnd: isize, width: i32, height: i32) -> bool {
            self.record(Call::CreateCaret(hwnd, width, height));
            !self.refuse_caret.get()
        }

        fn set_caret_pos(&self, x: i32, y: i32) {
            self.record(Call::SetCaretPos(x, y));
        }

        fn destroy_caret(&self) {
            self.record(Call::DestroyCaret);
        }

        fn get_active_window(&self) -> isize {
            self.active_window.get()
        }

        fn post(&self, action: Box<dyn FnOnce()>) {
            self.posted.borrow_mut().push(action);
        }
    }

    #[derive(Default)]
    struct FakeParent {
        scaling: Cell<f64>,
        no_input_callback: Cell<bool>,
        ignore_wm_char: Cell<Option<bool>>,
        text: RefCell<Vec<(u64, String)>>,
        keys: RefCell<Vec<(Key, PhysicalKey)>>,
    }

    impl Imm32Parent for FakeParent {
        fn desktop_scaling(&self) -> f64 {
            self.scaling.get()
        }

        fn set_ignore_wm_char(&self, value: bool) {
            self.ignore_wm_char.set(Some(value));
        }

        fn raw_text_input(&self, timestamp: u64, text: &str) -> bool {
            if self.no_input_callback.get() {
                return false;
            }
            self.text.borrow_mut().push((timestamp, text.to_owned()));
            true
        }

        fn raw_key_press(&self, key: Key, physical_key: PhysicalKey) {
            self.keys.borrow_mut().push((key, physical_key));
        }
    }

    /// A text input client that records the preedit text it is given.
    struct Client {
        events: TextInputMethodClientEvents,
        supports_preedit: bool,
        supports_surrounding_text: bool,
        selection: Cell<TextSelection>,
        preedit: RefCell<Vec<(Option<String>, Option<i32>)>>,
    }

    impl Client {
        fn new() -> Rc<Client> {
            Rc::new(Client {
                events: TextInputMethodClientEvents::new(),
                supports_preedit: true,
                supports_surrounding_text: true,
                selection: Cell::new(TextSelection::new(0, 0)),
                preedit: RefCell::new(Vec::new()),
            })
        }

        fn take_preedit(&self) -> Vec<(Option<String>, Option<i32>)> {
            std::mem::take(&mut *self.preedit.borrow_mut())
        }
    }

    impl TextInputMethodClient for Client {
        fn events(&self) -> &TextInputMethodClientEvents {
            &self.events
        }

        fn text_view_visual(&self) -> Ref<Visual> {
            unreachable!("the input method of the system does not ask for the visual")
        }

        fn supports_preedit(&self) -> bool {
            self.supports_preedit
        }

        fn supports_surrounding_text(&self) -> bool {
            self.supports_surrounding_text
        }

        fn surrounding_text(&self) -> String {
            String::new()
        }

        fn cursor_rectangle(&self) -> Rect {
            Rect::default()
        }

        fn selection(&self) -> TextSelection {
            self.selection.get()
        }

        fn set_selection(&self, value: TextSelection) {
            self.selection.set(value);
        }

        fn set_preedit_text_with_cursor(&self, preedit_text: Option<&str>, cursor_pos: Option<i32>) {
            self.preedit.borrow_mut().push((preedit_text.map(str::to_owned), cursor_pos));
        }
    }

    const HWND: isize = 0x77;
    /// A keyboard layout of Japanese (0x0411) and one of Korean (0x0412).
    const HKL_JA: isize = 0x0411_0411;
    const HKL_KO: isize = 0x0412_0412;

    struct Setup {
        system: Rc<FakeSystem>,
        parent: Rc<FakeParent>,
        input_method: Rc<Imm32InputMethod>,
        client: Rc<Client>,
    }

    /// An input method of a window with a client, enabled.
    fn setup(hkl: isize) -> Setup {
        let system = FakeSystem::new();
        let parent = Rc::new(FakeParent::default());
        parent.scaling.set(1.0);
        let input_method = Imm32InputMethod::new(system.clone());
        let weak: Weak<dyn Imm32Parent> = Rc::downgrade(&(parent.clone() as Rc<dyn Imm32Parent>));
        input_method.set_language_and_window(weak, HWND, hkl);
        let client = Client::new();
        input_method.set_client(Some(client.clone()));
        system.run_posted();
        system.take_calls();
        Setup { system, parent, input_method, client }
    }

    #[test]
    fn the_language_of_a_keyboard_layout_is_the_low_word_and_its_primary_language_the_low_ten_bits() {
        assert_eq!(0x0411, lgid(HKL_JA));
        assert_eq!(0x11, primary_lang_id(lgid(HKL_JA)));
        assert_eq!(LANG_KO as u16, primary_lang_id(lgid(HKL_KO)));
        // A layout handle of a 64-bit system with the sign extended.
        assert_eq!(0x0804, lgid(0xFFFF_FFFF_E008_0804_u64 as isize));
        assert_eq!(0x04, primary_lang_id(0x0804));
    }

    #[test]
    fn the_forms_have_the_layout_of_the_system() {
        assert_eq!(32, std::mem::size_of::<CANDIDATEFORM>());
        assert_eq!(28, std::mem::size_of::<COMPOSITIONFORM>());
        assert_eq!(16, std::mem::offset_of!(CANDIDATEFORM, rcArea));
        assert_eq!(12, std::mem::offset_of!(COMPOSITIONFORM, rcArea));
    }

    #[test]
    fn the_cursor_rectangle_is_scaled_to_pixels_and_truncated() {
        assert_eq!((10, 20, 12, 36), cursor_rect_in_pixels(Rect::new(10.0, 20.0, 2.0, 16.0), 1.0));
        assert_eq!((15, 30, 18, 54), cursor_rect_in_pixels(Rect::new(10.0, 20.0, 2.0, 16.0), 1.5));
        assert_eq!((21, 41, 25, 73), cursor_rect_in_pixels(Rect::new(10.6, 20.6, 2.0, 16.0), 2.0));
    }

    #[test]
    fn the_candidate_window_keeps_clear_of_the_caret_and_of_a_line_more_for_korean() {
        let japanese = candidate_exclude_rectangle(10, 20, 12, 36, 0x11);
        assert_eq!(CFS_EXCLUDE, japanese.dwStyle);
        assert_eq!(0, japanese.dwIndex);
        assert_eq!(POINT { x: 10, y: 20 }, japanese.ptCurrentPos);
        assert_eq!(RECT { left: 10, top: 20, right: 12, bottom: 37 }, japanese.rcArea);

        let korean = candidate_exclude_rectangle(10, 20, 12, 36, LANG_KO as u16);
        assert_eq!(RECT { left: 10, top: 20, right: 12, bottom: 38 }, korean.rcArea);
    }

    #[test]
    fn a_client_enables_the_input_context_of_the_window_and_creates_the_caret() {
        let system = FakeSystem::new();
        let input_method = Imm32InputMethod::new(system.clone());
        let parent = Rc::new(FakeParent::default());
        let weak: Weak<dyn Imm32Parent> = Rc::downgrade(&(parent.clone() as Rc<dyn Imm32Parent>));
        input_method.set_language_and_window(weak, HWND, HKL_JA);
        assert!(system.take_calls().is_empty());

        input_method.set_client(Some(Client::new()));
        // Nothing before the dispatcher runs the posted action.
        assert!(system.take_calls().is_empty());
        system.run_posted();

        assert_eq!(
            vec![
                Call::GetContext(HWND),
                Call::AssociateContext(HWND, 0x1C),
                Call::ReleaseContext(HWND, 0x1C),
                Call::CreateCaret(HWND, 2, 2)
            ],
            system.take_calls()
        );

        // The same context again: nothing to do.
        input_method.enable_imm();
        assert_eq!(vec![Call::GetContext(HWND)], system.take_calls());
    }

    #[test]
    fn a_window_without_an_input_context_gets_a_new_one() {
        let system = FakeSystem::new();
        system.context.set(0);
        let input_method = Imm32InputMethod::new(system.clone());
        input_method.hwnd.set(HWND);

        input_method.enable_imm();

        assert_eq!(
            vec![
                Call::GetContext(HWND),
                Call::CreateContext,
                Call::AssociateContext(HWND, 0x2C),
                Call::ReleaseContext(HWND, 0x2C),
                Call::CreateCaret(HWND, 2, 2)
            ],
            system.take_calls()
        );
    }

    #[test]
    fn no_client_completes_the_composition_and_takes_the_input_context_from_the_window() {
        let Setup { system, parent, input_method, client } = setup(HKL_JA);
        input_method.handle_composition_start();
        client.take_preedit();

        input_method.set_client(None);
        // The old client loses its preedit text at once.
        assert_eq!(vec![(None, None)], client.take_preedit());
        system.run_posted();

        assert_eq!(
            vec![
                Call::DestroyCaret,
                Call::AssociateContext(HWND, 0),
                // The reset that disabling posted: the composition is completed.
                Call::GetContext(HWND),
                Call::NotifyIme(0x1C, NI_COMPOSITIONSTR, CPS_COMPLETE, 0),
                Call::ReleaseContext(HWND, 0x1C),
            ],
            system.take_calls()
        );
        assert!(!input_method.is_composing());
        assert_eq!(Some(true), parent.ignore_wm_char.get());
        // The WM_IME_COMPOSITION message the completion sends is not a
        // composition of the next client.
        *system.result_string.borrow_mut() = Some("x".to_owned());
        input_method.handle_composition(0, GCS::GCS_RESULTSTR as isize, 1);
        assert!(parent.text.borrow().is_empty());
    }

    #[test]
    fn the_composition_string_reaches_the_client_with_the_cursor_position() {
        let Setup { system, parent, input_method, client } = setup(HKL_JA);

        input_method.handle_composition_start();
        assert!(input_method.is_composing());
        assert_eq!(vec![(None, None)], client.take_preedit());
        // No selection: nothing is deleted.
        assert!(parent.keys.borrow().is_empty());

        *system.composition_string.borrow_mut() = Some("にほ".to_owned());
        system.cursor.set(2);
        input_method.handle_composition(0, (GCS::GCS_COMPSTR | GCS::GCS_CURSORPOS) as isize, 10);
        assert_eq!(vec![(Some("にほ".to_owned()), Some(2))], client.take_preedit());
        assert_eq!(Some("にほ".to_owned()), input_method.composition());

        // The cursor alone moves: the string is the one kept.
        system.cursor.set(1);
        input_method.handle_composition(0, GCS::GCS_CURSORPOS as isize, 11);
        assert_eq!(vec![(Some("にほ".to_owned()), Some(1))], client.take_preedit());

        // The string alone changes: the cursor is the one kept.
        *system.composition_string.borrow_mut() = Some("にほん".to_owned());
        input_method.handle_composition(0, GCS::GCS_COMPSTR as isize, 12);
        assert_eq!(vec![(Some("にほん".to_owned()), Some(1))], client.take_preedit());

        // A cursor position the system cannot give is no position.
        system.cursor.set(-1);
        input_method.handle_composition(0, (GCS::GCS_COMPSTR | GCS::GCS_CURSORPOS) as isize, 13);
        assert_eq!(vec![(Some("にほん".to_owned()), None)], client.take_preedit());
        assert!(parent.text.borrow().is_empty());
    }

    #[test]
    fn the_result_string_is_raw_text_input_and_the_next_character_message_is_ignored() {
        let Setup { system, parent, input_method, client } = setup(HKL_JA);
        input_method.handle_composition_start();
        *system.composition_string.borrow_mut() = Some("にほん".to_owned());
        input_method.handle_composition(0, GCS::GCS_COMPSTR as isize, 1);
        client.take_preedit();

        *system.result_string.borrow_mut() = Some("日本".to_owned());
        *system.composition_string.borrow_mut() = None;
        // The flags of a result as a Japanese input method sends them:
        // the result string, and the cursor of a composition that is over.
        input_method.handle_composition(0, (GCS::GCS_RESULTSTR | GCS::GCS_CURSORPOS) as isize, 42);

        assert_eq!(vec![(42, "日本".to_owned())], *parent.text.borrow());
        assert_eq!(Some(true), parent.ignore_wm_char.get());
        // The preedit text is taken away, and not set again.
        assert_eq!(vec![(None, None)], client.take_preedit());
        assert_eq!(None, input_method.composition());

        // The end of the composition has nothing left to commit.
        input_method.handle_composition_end(43);
        assert!(!input_method.is_composing());
        assert_eq!(1, parent.text.borrow().len());
        assert_eq!(vec![(None, None)], client.take_preedit());
    }

    #[test]
    fn a_result_together_with_a_new_composition_commits_and_goes_on_composing() {
        // A Korean input method commits a syllable and starts the next in
        // one message.
        let Setup { system, parent, input_method, client } = setup(HKL_KO);
        input_method.handle_composition_start();
        client.take_preedit();

        *system.result_string.borrow_mut() = Some("한".to_owned());
        *system.composition_string.borrow_mut() = Some("ㄱ".to_owned());
        system.cursor.set(1);
        input_method.handle_composition(0, (GCS::GCS_RESULTSTR | GCS::GCS_COMPSTR | GCS::GCS_CURSORPOS) as isize, 5);

        assert_eq!(vec![(5, "한".to_owned())], *parent.text.borrow());
        assert_eq!(vec![(None, None), (Some("ㄱ".to_owned()), Some(1))], client.take_preedit());
    }

    #[test]
    fn a_composition_that_ends_with_text_left_commits_it() {
        let Setup { system, parent, input_method, client } = setup(HKL_KO);
        input_method.handle_composition_start();
        *system.composition_string.borrow_mut() = Some("가".to_owned());
        input_method.handle_composition(0, GCS::GCS_COMPSTR as isize, 1);
        client.take_preedit();

        input_method.handle_composition_end(9);

        assert_eq!(vec![(9, "가".to_owned())], *parent.text.borrow());
        assert_eq!(Some(true), parent.ignore_wm_char.get());
        assert_eq!(vec![(None, None)], client.take_preedit());
        assert_eq!(None, input_method.composition());
        assert!(!input_method.is_composing());
    }

    #[test]
    fn a_window_without_an_input_callback_commits_nothing_and_ignores_no_character() {
        let Setup { system, parent, input_method, .. } = setup(HKL_KO);
        parent.no_input_callback.set(true);
        input_method.handle_composition_start();
        *system.composition_string.borrow_mut() = Some("가".to_owned());
        input_method.handle_composition(0, GCS::GCS_COMPSTR as isize, 1);

        input_method.handle_composition_end(9);

        assert_eq!(None, parent.ignore_wm_char.get());
    }

    #[test]
    fn a_composition_message_without_flags_empties_the_preedit_text() {
        let Setup { input_method, client, .. } = setup(HKL_JA);
        input_method.handle_composition_start();
        client.take_preedit();

        input_method.handle_composition(0, 0, 1);

        assert_eq!(vec![(Some(String::new()), None)], client.take_preedit());
    }

    #[test]
    fn the_start_of_a_composition_deletes_the_selection_of_a_client_with_surrounding_text() {
        let Setup { parent, input_method, client, .. } = setup(HKL_JA);
        client.selection.set(TextSelection::new(2, 5));

        input_method.handle_composition_start();

        assert_eq!(vec![(Key::Delete, PhysicalKey::Delete)], *parent.keys.borrow());
    }

    #[test]
    fn a_client_without_preedit_is_told_no_composition_string() {
        let system = FakeSystem::new();
        let input_method = Imm32InputMethod::new(system.clone());
        let client = Rc::new(Client { supports_preedit: false, ..Rc::try_unwrap(Client::new()).ok().expect("one owner") });
        input_method.set_client(Some(client.clone()));
        input_method.set_is_composing(true);
        *system.composition_string.borrow_mut() = Some("a".to_owned());

        input_method.handle_composition(0, GCS::GCS_COMPSTR as isize, 1);

        assert!(client.take_preedit().is_empty());
        assert_eq!(Some("a".to_owned()), input_method.composition());
    }

    #[test]
    fn the_cursor_rectangle_moves_the_caret_and_the_candidate_window_of_the_active_window() {
        let Setup { system, parent, input_method, .. } = setup(HKL_JA);
        parent.scaling.set(2.0);

        // Another window is active: nothing is asked of the system.
        system.active_window.set(0x99);
        input_method.set_cursor_rect(Rect::new(10.0, 20.0, 1.0, 16.0));
        system.run_posted();
        assert!(system.take_calls().is_empty());

        system.active_window.set(HWND);
        input_method.set_cursor_rect(Rect::new(10.0, 20.0, 1.0, 16.0));
        assert!(system.take_calls().is_empty());
        system.run_posted();

        assert_eq!(
            vec![
                Call::GetContext(HWND),
                // The caret goes to the lower right corner of the cursor.
                Call::SetCaretPos(22, 72),
                Call::SetCandidateWindow(
                    0x1C,
                    CANDIDATEFORM {
                        dwIndex: 0,
                        dwStyle: CFS_EXCLUDE,
                        ptCurrentPos: POINT { x: 20, y: 40 },
                        rcArea: RECT { left: 20, top: 40, right: 22, bottom: 73 },
                    }
                ),
                Call::ReleaseContext(HWND, 0x1C),
            ],
            system.take_calls()
        );
    }

    #[test]
    fn a_change_of_the_language_makes_the_input_context_again() {
        let Setup { system, parent, input_method, .. } = setup(HKL_JA);
        let weak: Weak<dyn Imm32Parent> = Rc::downgrade(&(parent.clone() as Rc<dyn Imm32Parent>));

        // The same primary language: nothing.
        input_method.set_language_and_window(weak.clone(), HWND, HKL_JA);
        assert!(system.take_calls().is_empty());

        input_method.set_language_and_window(weak, HWND, HKL_KO);
        let calls = system.take_calls();
        assert_eq!(
            vec![
                Call::DestroyCaret,
                Call::AssociateContext(HWND, 0),
                Call::GetContext(HWND),
                Call::AssociateContext(HWND, 0x1C),
                Call::ReleaseContext(HWND, 0x1C),
                Call::CreateCaret(HWND, 2, 2),
            ],
            calls
        );
    }

    #[test]
    fn a_window_that_is_destroyed_gives_up_the_input_method() {
        let Setup { system, input_method, .. } = setup(HKL_JA);
        input_method.handle_composition_start();

        input_method.clear_language_and_window();

        assert_eq!(0, input_method.hwnd());
        assert!(!input_method.is_active());
        assert!(!input_method.is_composing());
        assert_eq!(vec![Call::DestroyCaret, Call::AssociateContext(HWND, 0)], system.take_calls());
        // The posted reset finds no window.
        system.context.set(0);
        system.run_posted();
        assert_eq!(vec![Call::GetContext(0)], system.take_calls());
    }
}

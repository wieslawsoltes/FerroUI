//! Microsoft Edge WebView2 on Windows.
//!
//! The runtime is reached through `WebView2Loader.dll`, loaded when the first web view is created
//! and shipped next to the application. The interfaces are declared by the slots of their
//! tables, which COM never reorders; see `SLOTS` below.

use super::bridge;
use crate::engine::{
    CoreWebView2WebErrorStatus, IWebViewEngine, IWebViewEngineEvents, IWebViewEngineFactory, ScriptCompletion,
    WebViewError, WebViewNavigation,
};
use crate::json;
use ferroui_controls::platform::{INativeControlHostDestroyableControlHandle, IPlatformHandle};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::ffi::{c_void, OsStr, OsString};
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::rc::Rc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Once;
use windows_sys::core::GUID;
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows_sys::Win32::System::Com::CoTaskMemFree;
use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress, LoadLibraryW};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetClientRect, GetWindowLongPtrW, IsWindow, RegisterClassExW,
    SetWindowLongPtrW, GWLP_USERDATA, WM_SETFOCUS, WM_SIZE, WNDCLASSEXW, WS_CHILD, WS_CLIPCHILDREN, WS_VISIBLE,
};

type Hresult = i32;
type Raw = *mut c_void;

const S_OK: Hresult = 0;
const E_NOINTERFACE: Hresult = 0x8000_4002_u32 as i32;
const E_POINTER: Hresult = 0x8000_4003_u32 as i32;

const IID_IUNKNOWN: GUID = GUID::from_u128(0x00000000_0000_0000_c000_000000000046);
const IID_ENVIRONMENT_COMPLETED: GUID = GUID::from_u128(0x4e8a3389_c9d8_4bd2_b6b5_124fee6cc14d);
const IID_CONTROLLER_COMPLETED: GUID = GUID::from_u128(0x6c4819f3_c9b7_4260_8127_c9f5bde7f68c);
const IID_NAVIGATION_STARTING: GUID = GUID::from_u128(0x9adbe429_f36d_432b_9ddc_f8881fbd76e3);
const IID_NAVIGATION_COMPLETED: GUID = GUID::from_u128(0xd33a35bf_1c49_4f98_93ab_006e0533fe1c);
const IID_SOURCE_CHANGED: GUID = GUID::from_u128(0x3c067f9f_5388_4772_8b48_79f7ef1ab37c);
const IID_HISTORY_CHANGED: GUID = GUID::from_u128(0xc79a420c_efd9_4058_9295_3e8b4bcab645);
const IID_TITLE_CHANGED: GUID = GUID::from_u128(0xf5f2b923_953e_4042_9f95_f3a118e1afd4);
const IID_WEB_MESSAGE: GUID = GUID::from_u128(0x57213f19_00e6_49fa_8e07_898ea01ecbd2);
const IID_SCRIPT_COMPLETED: GUID = GUID::from_u128(0x49511172_cc67_4bca_9923_137112f4c4cc);

/// Slots of the interface tables, from the WebView2 SDK header.
mod slots {
    pub const RELEASE: usize = 2;
    pub const ADD_REF: usize = 1;

    pub const ENVIRONMENT_CREATE_CONTROLLER: usize = 3;

    pub const CONTROLLER_PUT_IS_VISIBLE: usize = 4;
    pub const CONTROLLER_PUT_BOUNDS: usize = 6;
    pub const CONTROLLER_MOVE_FOCUS: usize = 12;
    pub const CONTROLLER_CLOSE: usize = 24;
    pub const CONTROLLER_GET_CORE: usize = 25;

    pub const CORE_GET_SOURCE: usize = 4;
    pub const CORE_NAVIGATE: usize = 5;
    pub const CORE_NAVIGATE_TO_STRING: usize = 6;
    pub const CORE_ADD_NAVIGATION_STARTING: usize = 7;
    pub const CORE_ADD_SOURCE_CHANGED: usize = 11;
    pub const CORE_ADD_HISTORY_CHANGED: usize = 13;
    pub const CORE_ADD_NAVIGATION_COMPLETED: usize = 15;
    pub const CORE_EXECUTE_SCRIPT: usize = 29;
    pub const CORE_RELOAD: usize = 31;
    pub const CORE_POST_JSON: usize = 32;
    pub const CORE_ADD_WEB_MESSAGE: usize = 34;
    pub const CORE_GET_CAN_GO_BACK: usize = 38;
    pub const CORE_GET_CAN_GO_FORWARD: usize = 39;
    pub const CORE_GO_BACK: usize = 40;
    pub const CORE_GO_FORWARD: usize = 41;
    pub const CORE_STOP: usize = 43;
    pub const CORE_ADD_TITLE_CHANGED: usize = 46;
    pub const CORE_GET_TITLE: usize = 48;

    pub const STARTING_GET_URI: usize = 3;
    pub const STARTING_GET_IS_USER: usize = 4;
    pub const STARTING_GET_IS_REDIRECTED: usize = 5;
    pub const STARTING_PUT_CANCEL: usize = 8;
    pub const STARTING_GET_ID: usize = 9;

    pub const COMPLETED_GET_IS_SUCCESS: usize = 3;
    pub const COMPLETED_GET_STATUS: usize = 4;
    pub const COMPLETED_GET_ID: usize = 5;

    pub const MESSAGE_GET_SOURCE: usize = 3;
    pub const MESSAGE_GET_JSON: usize = 4;
}

/// The function in slot `index` of the table of `this`.
///
/// # Safety
/// `this` is a live COM object whose table has a function of type `F` in that slot.
unsafe fn slot<F: Copy>(this: Raw, index: usize) -> F {
    debug_assert_eq!(size_of::<F>(), size_of::<*const c_void>());
    let table = *(this as *const *const *const c_void);
    std::mem::transmute_copy(&*table.add(index))
}

/// An owned reference to a COM object.
struct Com(Raw);

impl Com {
    /// # Safety
    /// `raw` is a live COM object; the new owner takes one more reference.
    unsafe fn add_ref(raw: Raw) -> Self {
        slot::<unsafe extern "system" fn(Raw) -> u32>(raw, slots::ADD_REF)(raw);
        Self(raw)
    }
}

impl Drop for Com {
    fn drop(&mut self) {
        // SAFETY: this owner holds one reference.
        unsafe { slot::<unsafe extern "system" fn(Raw) -> u32>(self.0, slots::RELEASE)(self.0) };
    }
}

fn wide(text: &str) -> Vec<u16> {
    OsStr::new(text).encode_wide().chain(Some(0)).collect()
}

/// Takes a string the runtime allocated.
///
/// # Safety
/// `text` is null or a terminated string from the COM allocator.
unsafe fn take_string(text: *mut u16) -> String {
    if text.is_null() {
        return String::new();
    }
    let value = borrowed_string(text);
    CoTaskMemFree(text as *const c_void);
    value
}

/// # Safety
/// `text` is null or a terminated string.
unsafe fn borrowed_string(text: *const u16) -> String {
    if text.is_null() {
        return String::new();
    }
    let mut length = 0;
    while *text.add(length) != 0 {
        length += 1;
    }
    OsString::from_wide(std::slice::from_raw_parts(text, length)).to_string_lossy().into_owned()
}

/// Reads a string property.
///
/// # Safety
/// Slot `index` of `this` is `HRESULT (LPWSTR*)`.
unsafe fn get_string(this: Raw, index: usize) -> String {
    let mut value = std::ptr::null_mut::<u16>();
    let result = slot::<unsafe extern "system" fn(Raw, *mut *mut u16) -> Hresult>(this, index)(this, &mut value);
    if result < 0 {
        return String::new();
    }
    take_string(value)
}

/// Reads a `BOOL` property.
///
/// # Safety
/// Slot `index` of `this` is `HRESULT (BOOL*)`.
unsafe fn get_bool(this: Raw, index: usize) -> bool {
    let mut value = 0i32;
    let result = slot::<unsafe extern "system" fn(Raw, *mut i32) -> Hresult>(this, index)(this, &mut value);
    result >= 0 && value != 0
}

/// Reads a `UINT64` property.
///
/// # Safety
/// Slot `index` of `this` is `HRESULT (UINT64*)`.
unsafe fn get_u64(this: Raw, index: usize) -> u64 {
    let mut value = 0u64;
    let _ = slot::<unsafe extern "system" fn(Raw, *mut u64) -> Hresult>(this, index)(this, &mut value);
    value
}

/// Calls a method without arguments.
///
/// # Safety
/// Slot `index` of `this` is `HRESULT ()`.
unsafe fn call(this: Raw, index: usize) -> Hresult {
    slot::<unsafe extern "system" fn(Raw) -> Hresult>(this, index)(this)
}

/// Calls a method taking a string.
///
/// # Safety
/// Slot `index` of `this` is `HRESULT (LPCWSTR)`.
unsafe fn call_with_text(this: Raw, index: usize, text: &str) -> Hresult {
    let text = wide(text);
    slot::<unsafe extern "system" fn(Raw, *const u16) -> Hresult>(this, index)(this, text.as_ptr())
}

/// Every handler interface of WebView2 is `IUnknown` and one `Invoke` of two arguments.
#[repr(C)]
struct CallbackTable<A, B> {
    query_interface: unsafe extern "system" fn(Raw, *const GUID, *mut Raw) -> Hresult,
    add_ref: unsafe extern "system" fn(Raw) -> u32,
    release: unsafe extern "system" fn(Raw) -> u32,
    invoke: unsafe extern "system" fn(Raw, A, B) -> Hresult,
}

#[repr(C)]
struct Callback<A: 'static, B: 'static> {
    table: *const CallbackTable<A, B>,
    references: AtomicU32,
    iid: GUID,
    action: Box<dyn Fn(A, B)>,
}

fn same_guid(a: &GUID, b: &GUID) -> bool {
    a.data1 == b.data1 && a.data2 == b.data2 && a.data3 == b.data3 && a.data4 == b.data4
}

impl<A: 'static, B: 'static> Callback<A, B> {
    const TABLE: CallbackTable<A, B> = CallbackTable {
        query_interface: Self::query_interface,
        add_ref: Self::add_ref,
        release: Self::release,
        invoke: Self::invoke,
    };

    fn create(iid: GUID, action: impl Fn(A, B) + 'static) -> Com {
        let this = Box::new(Self {
            table: &Self::TABLE,
            references: AtomicU32::new(1),
            iid,
            action: Box::new(action),
        });
        Com(Box::into_raw(this).cast())
    }

    unsafe extern "system" fn query_interface(this: Raw, iid: *const GUID, out: *mut Raw) -> Hresult {
        if out.is_null() || iid.is_null() {
            return E_POINTER;
        }
        let callback = &*(this as *const Self);
        if same_guid(&*iid, &IID_IUNKNOWN) || same_guid(&*iid, &callback.iid) {
            callback.references.fetch_add(1, Ordering::Relaxed);
            *out = this;
            S_OK
        } else {
            *out = std::ptr::null_mut();
            E_NOINTERFACE
        }
    }

    unsafe extern "system" fn add_ref(this: Raw) -> u32 {
        (*(this as *const Self)).references.fetch_add(1, Ordering::Relaxed) + 1
    }

    unsafe extern "system" fn release(this: Raw) -> u32 {
        let left = (*(this as *const Self)).references.fetch_sub(1, Ordering::AcqRel) - 1;
        if left == 0 {
            drop(Box::from_raw(this as *mut Self));
        }
        left
    }

    unsafe extern "system" fn invoke(this: Raw, a: A, b: B) -> Hresult {
        // The object stays alive for the call, also if the action closes the engine.
        let _hold = Com::add_ref(this);
        ((*(this as *const Self)).action)(a, b);
        S_OK
    }
}

fn status_of(value: i32) -> CoreWebView2WebErrorStatus {
    use CoreWebView2WebErrorStatus as Status;
    match value {
        1..=5 => Status::CertificateInvalid,
        6 | 12 => Status::ServerUnreachable,
        7 => Status::Timeout,
        9 | 10 => Status::ConnectionAborted,
        11 => Status::Disconnected,
        13 => Status::HostNameNotResolved,
        14 => Status::OperationCanceled,
        15 => Status::RedirectFailed,
        _ => Status::Unknown,
    }
}

fn failure(what: &str, result: Hresult) -> WebViewError {
    WebViewError::Engine(format!("{what} failed with 0x{:08x}", result as u32))
}

struct EngineState {
    events: Rc<dyn IWebViewEngineEvents>,
    host: HWND,
    controller: RefCell<Option<Com>>,
    core: RefCell<Option<Com>>,
    is_closed: Cell<bool>,
}

impl EngineState {
    fn core(&self) -> Option<Raw> {
        self.core.borrow().as_ref().map(|core| core.0)
    }

    fn fit(&self) {
        let controller = self.controller.borrow().as_ref().map(|controller| controller.0);
        let Some(controller) = controller else { return };
        let mut bounds = RECT { left: 0, top: 0, right: 0, bottom: 0 };
        // SAFETY: the host window is alive while the engine is open; slot from the SDK header.
        unsafe {
            if GetClientRect(self.host, &mut bounds) != 0 {
                slot::<unsafe extern "system" fn(Raw, RECT) -> Hresult>(controller, slots::CONTROLLER_PUT_BOUNDS)(
                    controller, bounds,
                );
            }
        }
    }

    fn focus(&self) {
        let controller = self.controller.borrow().as_ref().map(|controller| controller.0);
        if let Some(controller) = controller {
            // SAFETY: slot from the SDK header; reason 0 is "programmatic".
            unsafe {
                slot::<unsafe extern "system" fn(Raw, i32) -> Hresult>(controller, slots::CONTROLLER_MOVE_FOCUS)(
                    controller, 0,
                );
            }
        }
    }

    fn controller_created(self: &Rc<Self>, result: Hresult, controller: Raw) {
        if self.is_closed.get() {
            if result >= 0 && !controller.is_null() {
                // SAFETY: the runtime passes a live controller; it is not kept.
                unsafe { call(controller, slots::CONTROLLER_CLOSE) };
            }
            return;
        }
        if result < 0 || controller.is_null() {
            self.events.initialized(Err(failure("creating the controller", result)));
            return;
        }
        // SAFETY: a live controller from the runtime; each slot is from the SDK header.
        let core = unsafe {
            let controller = Com::add_ref(controller);
            let mut core = std::ptr::null_mut();
            let got = slot::<unsafe extern "system" fn(Raw, *mut Raw) -> Hresult>(
                controller.0,
                slots::CONTROLLER_GET_CORE,
            )(controller.0, &mut core);
            if got < 0 || core.is_null() {
                self.events.initialized(Err(failure("reading the web view", got)));
                call(controller.0, slots::CONTROLLER_CLOSE);
                return;
            }
            slot::<unsafe extern "system" fn(Raw, i32) -> Hresult>(controller.0, slots::CONTROLLER_PUT_IS_VISIBLE)(
                controller.0,
                1,
            );
            *self.controller.borrow_mut() = Some(controller);
            Com(core)
        };
        let raw = core.0;
        *self.core.borrow_mut() = Some(core);
        self.fit();
        // SAFETY: `raw` is the live web view kept above.
        if let Err(error) = unsafe { self.subscribe(raw) } {
            self.events.initialized(Err(error));
            return;
        }
        self.events.initialized(Ok(()));
    }

    /// # Safety
    /// `core` is a live `ICoreWebView2`.
    unsafe fn subscribe(self: &Rc<Self>, core: Raw) -> Result<(), WebViewError> {
        unsafe fn add(core: Raw, index: usize, handler: Com) -> Result<(), WebViewError> {
            let mut token = 0i64;
            let result = slot::<unsafe extern "system" fn(Raw, Raw, *mut i64) -> Hresult>(core, index)(
                core,
                handler.0,
                &mut token,
            );
            if result < 0 {
                return Err(failure("subscribing to the web view", result));
            }
            Ok(())
        }

        // The handlers hold the events only: the engine owns the state, and closing the
        // controller releases the handlers.
        let events = self.events.clone();
        add(
            core,
            slots::CORE_ADD_NAVIGATION_STARTING,
            Callback::create(IID_NAVIGATION_STARTING, move |_sender: Raw, args: Raw| {
                // SAFETY: the runtime passes live event arguments for the call.
                unsafe {
                    let navigation = WebViewNavigation {
                        id: get_u64(args, slots::STARTING_GET_ID),
                        uri: get_string(args, slots::STARTING_GET_URI),
                        is_user_initiated: get_bool(args, slots::STARTING_GET_IS_USER),
                        is_redirected: get_bool(args, slots::STARTING_GET_IS_REDIRECTED),
                    };
                    if !events.navigation_starting(&navigation) {
                        slot::<unsafe extern "system" fn(Raw, i32) -> Hresult>(args, slots::STARTING_PUT_CANCEL)(
                            args, 1,
                        );
                    }
                }
            }),
        )?;
        let events = self.events.clone();
        add(
            core,
            slots::CORE_ADD_NAVIGATION_COMPLETED,
            Callback::create(IID_NAVIGATION_COMPLETED, move |_sender: Raw, args: Raw| {
                // SAFETY: the runtime passes live event arguments for the call.
                unsafe {
                    let mut status = 0i32;
                    slot::<unsafe extern "system" fn(Raw, *mut i32) -> Hresult>(args, slots::COMPLETED_GET_STATUS)(
                        args,
                        &mut status,
                    );
                    events.navigation_completed(
                        get_u64(args, slots::COMPLETED_GET_ID),
                        get_bool(args, slots::COMPLETED_GET_IS_SUCCESS),
                        status_of(status),
                    );
                }
            }),
        )?;
        let events = self.events.clone();
        add(
            core,
            slots::CORE_ADD_SOURCE_CHANGED,
            Callback::create(IID_SOURCE_CHANGED, move |sender: Raw, _args: Raw| {
                // SAFETY: the sender is the live web view.
                let uri = unsafe { get_string(sender, slots::CORE_GET_SOURCE) };
                events.source_changed(&uri);
            }),
        )?;
        let events = self.events.clone();
        add(
            core,
            slots::CORE_ADD_HISTORY_CHANGED,
            Callback::create(IID_HISTORY_CHANGED, move |sender: Raw, _args: Raw| {
                // SAFETY: the sender is the live web view.
                let (back, forward) = unsafe {
                    (get_bool(sender, slots::CORE_GET_CAN_GO_BACK), get_bool(sender, slots::CORE_GET_CAN_GO_FORWARD))
                };
                events.history_changed(back, forward);
            }),
        )?;
        let events = self.events.clone();
        add(
            core,
            slots::CORE_ADD_TITLE_CHANGED,
            Callback::create(IID_TITLE_CHANGED, move |sender: Raw, _args: Raw| {
                // SAFETY: the sender is the live web view.
                let title = unsafe { get_string(sender, slots::CORE_GET_TITLE) };
                events.document_title_changed(&title);
            }),
        )?;
        let events = self.events.clone();
        add(
            core,
            slots::CORE_ADD_WEB_MESSAGE,
            Callback::create(IID_WEB_MESSAGE, move |_sender: Raw, args: Raw| {
                // SAFETY: the runtime passes live event arguments for the call.
                let (source, message) =
                    unsafe { (get_string(args, slots::MESSAGE_GET_SOURCE), get_string(args, slots::MESSAGE_GET_JSON)) };
                events.web_message_received(&source, &message);
            }),
        )
    }
}

const HOST_CLASS: &str = "FerroUIWebViewHost";

unsafe extern "system" fn host_proc(window: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if message == WM_SIZE || message == WM_SETFOCUS {
        let state = GetWindowLongPtrW(window, GWLP_USERDATA) as *const EngineState;
        if !state.is_null() {
            // SAFETY: the engine clears this pointer before its state can go away.
            if message == WM_SIZE {
                (*state).fit();
            } else {
                (*state).focus();
            }
            return 0;
        }
    }
    DefWindowProcW(window, message, wparam, lparam)
}

fn create_host(parent: HWND) -> Result<HWND, WebViewError> {
    static REGISTER: Once = Once::new();
    let class = wide(HOST_CLASS);
    // SAFETY: the class name outlives the calls and the parent is a window of the host.
    unsafe {
        let instance = GetModuleHandleW(std::ptr::null());
        REGISTER.call_once(|| {
            let mut description: WNDCLASSEXW = std::mem::zeroed();
            description.cbSize = size_of::<WNDCLASSEXW>() as u32;
            description.lpfnWndProc = Some(host_proc);
            description.hInstance = instance;
            description.lpszClassName = class.as_ptr();
            RegisterClassExW(&description);
        });
        let window = CreateWindowExW(
            0,
            class.as_ptr(),
            std::ptr::null(),
            WS_CHILD | WS_VISIBLE | WS_CLIPCHILDREN,
            0,
            0,
            1,
            1,
            parent,
            std::ptr::null_mut(),
            instance,
            std::ptr::null(),
        );
        if window.is_null() {
            return Err(WebViewError::Engine("the host window could not be created".to_owned()));
        }
        Ok(window)
    }
}

/// The folder of the browser profile: next to the program it may not be writable.
fn user_data_folder() -> Option<Vec<u16>> {
    let base = std::env::var_os("LOCALAPPDATA")?;
    let program = std::env::current_exe().ok()?;
    let name = program.file_stem()?;
    let folder = std::path::Path::new(&base).join(name).join("WebView2");
    Some(folder.as_os_str().encode_wide().chain(Some(0)).collect())
}

type CreateEnvironment = unsafe extern "system" fn(*const u16, *const u16, Raw, Raw) -> Hresult;

fn load_create_environment() -> Result<CreateEnvironment, WebViewError> {
    let library = wide("WebView2Loader.dll");
    // SAFETY: the name is a terminated string; the library stays loaded for the process.
    unsafe {
        let module = LoadLibraryW(library.as_ptr());
        if module.is_null() {
            return Err(WebViewError::Unavailable("WebView2Loader.dll was not found".to_owned()));
        }
        let function = GetProcAddress(module, c"CreateCoreWebView2EnvironmentWithOptions".as_ptr().cast());
        match function {
            // SAFETY: this export has the signature of the SDK header.
            Some(function) => Ok(std::mem::transmute::<unsafe extern "system" fn() -> isize, CreateEnvironment>(
                function,
            )),
            None => Err(WebViewError::Unavailable("WebView2Loader.dll has no environment function".to_owned())),
        }
    }
}

struct HostHandle {
    window: Cell<HWND>,
}

impl IPlatformHandle for HostHandle {
    fn handle(&self) -> isize {
        self.window.get() as isize
    }

    fn handle_descriptor(&self) -> Option<&str> {
        Some("HWND")
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_native_control_host_destroyable_control_handle(
        &self,
    ) -> Option<&dyn INativeControlHostDestroyableControlHandle> {
        Some(self)
    }
}

impl INativeControlHostDestroyableControlHandle for HostHandle {
    fn destroy(&self) {
        let window = self.window.replace(std::ptr::null_mut());
        // SAFETY: `IsWindow` accepts any value; the window is destroyed on its own thread.
        unsafe {
            if !window.is_null() && IsWindow(window) != 0 {
                DestroyWindow(window);
            }
        }
    }
}

struct WindowsEngine {
    state: Rc<EngineState>,
    control: Rc<HostHandle>,
}

impl WindowsEngine {
    /// Runs a call on the web view, once it exists. Commands wait in the control until the
    /// engine reported that it is ready, so a missing web view means it is closed.
    fn with_core(&self, action: impl FnOnce(Raw)) {
        if let Some(core) = self.state.core() {
            action(core);
        }
    }
}

impl IWebViewEngine for WindowsEngine {
    fn native_control(&self) -> Rc<dyn IPlatformHandle> {
        self.control.clone()
    }

    fn close(&self) {
        if self.state.is_closed.replace(true) {
            return;
        }
        // SAFETY: the window is alive until `destroy` below; the pointer is cleared first so
        // that no message reaches the state after this.
        unsafe { SetWindowLongPtrW(self.state.host, GWLP_USERDATA, 0) };
        self.state.core.borrow_mut().take();
        let controller = self.state.controller.borrow_mut().take();
        if let Some(controller) = controller {
            // SAFETY: slot from the SDK header; the controller is closed once.
            unsafe { call(controller.0, slots::CONTROLLER_CLOSE) };
        }
        self.control.destroy();
    }

    fn navigate(&self, uri: &str) {
        self.with_core(|core| {
            // SAFETY: slot from the SDK header.
            if unsafe { call_with_text(core, slots::CORE_NAVIGATE, uri) } < 0 {
                self.state.events.navigation_completed(0, false, CoreWebView2WebErrorStatus::Unknown);
            }
        });
    }

    fn navigate_to_string(&self, html: &str) {
        self.with_core(|core| {
            // SAFETY: slot from the SDK header.
            if unsafe { call_with_text(core, slots::CORE_NAVIGATE_TO_STRING, html) } < 0 {
                self.state.events.navigation_completed(0, false, CoreWebView2WebErrorStatus::Unknown);
            }
        });
    }

    fn reload(&self) {
        // SAFETY: slot from the SDK header.
        self.with_core(|core| unsafe {
            call(core, slots::CORE_RELOAD);
        });
    }

    fn stop(&self) {
        // SAFETY: slot from the SDK header.
        self.with_core(|core| unsafe {
            call(core, slots::CORE_STOP);
        });
    }

    fn go_back(&self) {
        // SAFETY: slot from the SDK header.
        self.with_core(|core| unsafe {
            call(core, slots::CORE_GO_BACK);
        });
    }

    fn go_forward(&self) {
        // SAFETY: slot from the SDK header.
        self.with_core(|core| unsafe {
            call(core, slots::CORE_GO_FORWARD);
        });
    }

    fn execute_script(&self, script: &str, completion: ScriptCompletion) {
        let Some(core) = self.state.core() else {
            completion(Err(WebViewError::Closed));
            return;
        };
        let completion = Rc::new(RefCell::new(Some(completion)));
        let answer = completion.clone();
        let handler = Callback::create(IID_SCRIPT_COMPLETED, move |result: Hresult, text: *const u16| {
            let Some(completion) = answer.borrow_mut().take() else { return };
            if result < 0 {
                completion(Err(failure("running the script", result)));
                return;
            }
            // SAFETY: the runtime passes a terminated string for the call.
            let text = unsafe { borrowed_string(text) };
            // The value of the result script is a string, which WebView2 returns as JSON.
            completion(match json::parse_string(&text) {
                Some(value) => bridge::parse_result(&value),
                None => Err(WebViewError::Engine("the script gave no result".to_owned())),
            });
        });
        let source = wide(&bridge::result_script(script));
        // SAFETY: slot from the SDK header; the runtime holds the handler while it needs it.
        let result = unsafe {
            slot::<unsafe extern "system" fn(Raw, *const u16, Raw) -> Hresult>(core, slots::CORE_EXECUTE_SCRIPT)(
                core,
                source.as_ptr(),
                handler.0,
            )
        };
        if result < 0 {
            if let Some(completion) = completion.borrow_mut().take() {
                completion(Err(failure("running the script", result)));
            }
        }
    }

    fn post_web_message_as_json(&self, json: &str) {
        // SAFETY: slot from the SDK header.
        self.with_core(|core| unsafe {
            call_with_text(core, slots::CORE_POST_JSON, json);
        });
    }
}

impl Drop for WindowsEngine {
    fn drop(&mut self) {
        self.close();
    }
}

/// Creates WebView2 engines.
pub(super) struct WindowsEngineFactory;

impl IWebViewEngineFactory for WindowsEngineFactory {
    fn create(
        &self,
        parent: Rc<dyn IPlatformHandle>,
        events: Rc<dyn IWebViewEngineEvents>,
    ) -> Result<Rc<dyn IWebViewEngine>, WebViewError> {
        if parent.handle_descriptor() != Some("HWND") {
            return Err(WebViewError::Unavailable("WebView2 needs a window handle".to_owned()));
        }
        let create_environment = load_create_environment()?;
        let host = create_host(parent.handle() as HWND)?;
        let state = Rc::new(EngineState {
            events,
            host,
            controller: RefCell::new(None),
            core: RefCell::new(None),
            is_closed: Cell::new(false),
        });
        let control = Rc::new(HostHandle { window: Cell::new(host) });
        // SAFETY: the engine clears the pointer in `close`, before the state can go away.
        unsafe { SetWindowLongPtrW(host, GWLP_USERDATA, Rc::as_ptr(&state) as isize) };

        let starting = state.clone();
        let environment_created = Callback::create(IID_ENVIRONMENT_COMPLETED, move |result: Hresult, environment: Raw| {
            if starting.is_closed.get() {
                return;
            }
            if result < 0 || environment.is_null() {
                starting.events.initialized(Err(WebViewError::Unavailable(format!(
                    "the WebView2 runtime could not be started (0x{:08x})",
                    result as u32
                ))));
                return;
            }
            let created = starting.clone();
            let handler = Callback::create(IID_CONTROLLER_COMPLETED, move |result: Hresult, controller: Raw| {
                created.controller_created(result, controller);
            });
            // SAFETY: a live environment from the runtime; slot from the SDK header.
            let result = unsafe {
                slot::<unsafe extern "system" fn(Raw, HWND, Raw) -> Hresult>(
                    environment,
                    slots::ENVIRONMENT_CREATE_CONTROLLER,
                )(environment, starting.host, handler.0)
            };
            if result < 0 {
                starting.events.initialized(Err(failure("creating the controller", result)));
            }
        });
        let folder = user_data_folder();
        let folder_pointer = folder.as_ref().map_or(std::ptr::null(), |folder| folder.as_ptr());
        // SAFETY: the export of the loader; the strings live for the call, the handler is
        // referenced by the runtime until it ran.
        let result = unsafe {
            create_environment(std::ptr::null(), folder_pointer, std::ptr::null_mut(), environment_created.0)
        };
        if result < 0 {
            control.destroy();
            return Err(WebViewError::Unavailable(format!(
                "the WebView2 runtime is not available (0x{:08x})",
                result as u32
            )));
        }
        Ok(Rc::new(WindowsEngine { state, control }))
    }
}

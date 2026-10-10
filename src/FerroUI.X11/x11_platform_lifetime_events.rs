//! The shutdown request of a session manager (the port of
//! `X11PlatformLifetimeEvents.cs`): the platform is a client of the X
//! Session Management Protocol through `libSM` over `libICE`, and raises
//! the shutdown request of the application lifetime when the session
//! manager asks whether the session may end.
//!
//! The messages of the session manager are read on a thread of their own,
//! and the library calls the handlers there. What that thread shares with
//! the UI thread is [`Shared`]: the connection and the phase, as atomics.
//! The event and its handlers belong to the UI thread, which a job of its
//! dispatcher reaches.
//!
//! The libraries are opened at run time. Without them the platform has no
//! session management and logs that; the reference fails to start
//! (DEVIATIONS.md).

use crate::ice_lib::{ice_lib, ICELib, IceProcessMessagesStatus};
use crate::sm_lib::{sm_lib, SmDialogValue, SMLib, SmcCallbacks};
use ferroui_base::logging::{LogEventLevel, Logger};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::utilities::{HandlerList, ThreadBound};
use ferroui_controls::application_lifetimes::ShutdownRequestedEventArgs;
use ferroui_controls::platform::IPlatformLifetimeEventsImpl;
use std::collections::HashMap;
use std::ffi::{c_char, c_int, c_ulong, c_void, CStr};
use std::rc::{Rc, Weak};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

const LOG_AREA: &str = "X11Platform";

const SMC_SAVE_YOURSELF_PROC_MASK: c_ulong = 1;
const SMC_DIE_PROC_MASK: c_ulong = 2;
const SMC_SAVE_COMPLETE_PROC_MASK: c_ulong = 4;
const SMC_SHUTDOWN_CANCELLED_PROC_MASK: c_ulong = 8;

fn warn(message: &str) {
    if let Some(logger) = Logger::try_get(LogEventLevel::Warning, LOG_AREA) {
        logger.log(None, message);
    }
}

/// The calls to the session manager, behind a trait so that the handlers
/// can be driven without one. A connection is the address the library
/// gave, as a number.
pub(crate) trait ISmConnectionCalls: Send + Sync {
    fn save_yourself_done(&self, smc_conn: usize, success: bool);
    /// Asks for the turn to interact with the user; the interact handler
    /// is called when it is granted.
    fn interact_request(&self, smc_conn: usize, client_data: usize);
    fn interact_done(&self, smc_conn: usize, cancel_shutdown: bool);
    fn close_connection(&self, smc_conn: usize, reason: &str);
    /// Reads and handles the messages of a connection; `false` when the
    /// connection is lost.
    fn process_messages(&self, ice_conn: usize) -> bool;
}

struct LibraryCalls {
    sm: &'static SMLib,
    ice: &'static ICELib,
}

impl ISmConnectionCalls for LibraryCalls {
    fn save_yourself_done(&self, smc_conn: usize, success: bool) {
        // SAFETY: the connection is one `SmcOpenConnection` returned and that was not closed:
        // the callers take it from the callback of the library or from the field that is
        // cleared before the connection is closed.
        unsafe { (self.sm.smc_save_yourself_done)(smc_conn as *mut c_void, success as c_int) }
    }

    fn interact_request(&self, smc_conn: usize, client_data: usize) {
        // SAFETY: as above; the handler is a function of this module, which lives as long as
        // the process.
        unsafe {
            (self.sm.smc_interact_request)(
                smc_conn as *mut c_void,
                SmDialogValue::SmDialogError as c_int,
                static_interact_handler,
                client_data as *mut c_void,
            )
        };
    }

    fn interact_done(&self, smc_conn: usize, cancel_shutdown: bool) {
        // SAFETY: as `save_yourself_done`.
        unsafe { (self.sm.smc_interact_done)(smc_conn as *mut c_void, cancel_shutdown as c_int) }
    }

    fn close_connection(&self, smc_conn: usize, reason: &str) {
        let reason = std::ffi::CString::new(reason).unwrap_or_default();
        let mut reasons = [reason.as_ptr().cast_mut()];
        // SAFETY: the connection was taken out of its field by the caller, so it is closed
        // once; the reasons are one terminated string that lives for the call.
        unsafe { (self.sm.smc_close_connection)(smc_conn as *mut c_void, 1, reasons.as_mut_ptr()) };
    }

    fn process_messages(&self, ice_conn: usize) -> bool {
        let mut reply_ready: c_int = 0;
        // SAFETY: the connection is the one of the session management connection, read on
        // this one thread; no reply is waited for (a null wait), and the flag is a valid
        // place for an integer.
        let status =
            unsafe { (self.ice.ice_process_messages)(ice_conn as *mut c_void, std::ptr::null_mut(), &mut reply_ready) };
        status != IceProcessMessagesStatus::IceProcessMessagesIoError as c_int
    }
}

/// What the thread of the session manager and the UI thread share.
pub(crate) struct Shared {
    calls: Box<dyn ISmConnectionCalls>,
    current_smc_conn: AtomicUsize,
    current_ice_conn: AtomicUsize,
    save_yourself_phase: AtomicBool,
    cancellation_requested: AtomicBool,
    /// The object of the UI thread, for the job that raises the event.
    owner: ThreadBound<Weak<X11PlatformLifetimeEvents>>,
    /// The dispatcher of the UI thread, when there is one.
    dispatcher: Option<Arc<Dispatcher>>,
}

static NATIVE_TO_MANAGED_MAPPER: Mutex<Option<HashMap<usize, Arc<Shared>>>> = Mutex::new(None);

fn get_instance(smc_conn: usize) -> Option<Arc<Shared>> {
    NATIVE_TO_MANAGED_MAPPER
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .as_ref()
        .and_then(|map| map.get(&smc_conn).cloned())
}

/// A handler the library called: a panic ends here, it cannot unwind into
/// the library.
fn guarded(call: impl FnOnce()) {
    if std::panic::catch_unwind(std::panic::AssertUnwindSafe(call)).is_err() {
        warn("A handler of the session management connection panicked.");
    }
}

unsafe extern "C" fn smc_save_complete_handler(smc_conn: *mut c_void, _client_data: *mut c_void) {
    guarded(|| {
        if let Some(instance) = get_instance(smc_conn as usize) {
            instance.save_complete_handler();
        }
    });
}

unsafe extern "C" fn smc_shutdown_cancelled_handler(smc_conn: *mut c_void, _client_data: *mut c_void) {
    guarded(|| {
        if let Some(instance) = get_instance(smc_conn as usize) {
            instance.shutdown_cancelled_handler();
        }
    });
}

unsafe extern "C" fn smc_die_handler(smc_conn: *mut c_void, _client_data: *mut c_void) {
    guarded(|| {
        if let Some(instance) = get_instance(smc_conn as usize) {
            instance.die_handler();
        }
    });
}

unsafe extern "C" fn smc_save_yourself_handler(
    smc_conn: *mut c_void,
    client_data: *mut c_void,
    _save_type: c_int,
    shutdown: c_int,
    _interact_style: c_int,
    fast: c_int,
) {
    guarded(|| {
        if let Some(instance) = get_instance(smc_conn as usize) {
            instance.save_yourself_handler(smc_conn as usize, client_data as usize, shutdown != 0, fast != 0);
        }
    });
}

unsafe extern "C" fn static_interact_handler(smc_conn: *mut c_void, _client_data: *mut c_void) {
    guarded(|| {
        if let Some(instance) = get_instance(smc_conn as usize) {
            instance.interact_handler(smc_conn as usize);
        }
    });
}

unsafe extern "C" fn static_ice_io_error_handler(_ice_conn: *mut c_void) {
    guarded(|| warn("ICELib reported an unknown IO Error."));
}

unsafe extern "C" fn static_error_handler(
    smc_conn: *mut c_void,
    _swap: c_int,
    offending_minor_opcode: c_int,
    offending_sequence: c_ulong,
    error_class: c_int,
    severity: c_int,
    _values: *mut c_void,
) {
    guarded(|| {
        // The same handler serves both libraries; an error of the ICE library names a
        // connection that is not in the map and is passed over, as in the reference.
        if get_instance(smc_conn as usize).is_some() {
            warn(&error_message(offending_minor_opcode, offending_sequence as usize, error_class, severity));
        }
    });
}

unsafe extern "C" fn ice_watch_handler(
    _ice_conn: *mut c_void,
    _client_data: *mut c_void,
    opening: c_int,
    _watch_data: *mut *mut c_void,
) {
    if opening == 0 {
        return;
    }

    if let Ok(ice) = ice_lib() {
        // SAFETY: removes the watch this module added, with the arguments it was added with.
        unsafe { (ice.ice_remove_connection_watch)(ice_watch_handler, std::ptr::null_mut()) };
    }
}

/// The text of `ErrorHandler`.
pub(crate) fn error_message(offending_minor_opcode: i32, offending_sequence: usize, error_class: i32, severity: i32) -> String {
    format!(
        "SMLib reported an error: severity {severity:X} mOpcode {offending_minor_opcode:X} mSeq {offending_sequence:X} \
         errClass {error_class:X}."
    )
}

impl Shared {
    /// `Dispose`: reachable from the Die callback, from `handle_requests`
    /// and from platform teardown, but closing frees both the connection
    /// and its ICE connection.
    fn dispose(&self) {
        let smc_conn = self.current_smc_conn.swap(0, Ordering::SeqCst);
        if smc_conn == 0 {
            return;
        }

        // The pump would otherwise keep using the ICE connection we are about to free.
        self.cancellation_requested.store(true, Ordering::SeqCst);

        if let Some(map) = NATIVE_TO_MANAGED_MAPPER.lock().unwrap_or_else(PoisonError::into_inner).as_mut() {
            map.remove(&smc_conn);
        }

        self.calls.close_connection(smc_conn, "X11PlatformLifetimeEvents was disposed in managed code.");
    }

    fn handle_requests(&self) {
        if self.cancellation_requested.load(Ordering::SeqCst) {
            return;
        }

        if !self.calls.process_messages(self.current_ice_conn.load(Ordering::SeqCst)) {
            warn("SMLib lost its underlying ICE connection.");
            self.dispose();
        }
    }

    fn save_complete_handler(&self) {
        self.save_yourself_phase.store(false, Ordering::SeqCst);
    }

    fn shutdown_cancelled_handler(&self) {
        let smc_conn = self.current_smc_conn.load(Ordering::SeqCst);
        if self.save_yourself_phase.load(Ordering::SeqCst) && smc_conn != 0 {
            self.calls.save_yourself_done(smc_conn, true);
        }
        self.save_yourself_phase.store(false, Ordering::SeqCst);
    }

    fn die_handler(&self) {
        self.dispose();
    }

    fn save_yourself_handler(&self, smc_conn: usize, client_data: usize, shutdown: bool, fast: bool) {
        if self.save_yourself_phase.load(Ordering::SeqCst) {
            self.calls.save_yourself_done(smc_conn, true);
        }

        self.save_yourself_phase.store(true, Ordering::SeqCst);

        if shutdown && !fast {
            self.calls.interact_request(smc_conn, client_data);
        } else {
            self.calls.save_yourself_done(smc_conn, true);
            self.save_yourself_phase.store(false, Ordering::SeqCst);
        }
    }

    fn interact_handler(self: &Arc<Self>, smc_conn: usize) {
        let Some(dispatcher) = &self.dispatcher else {
            return;
        };
        let this = self.clone();
        dispatcher.post(move || this.actual_interact_handler(smc_conn), DispatcherPriority::NORMAL);
    }

    /// `ActualInteractHandler`, on the UI thread.
    fn actual_interact_handler(&self, smc_conn: usize) {
        let e = ShutdownRequestedEventArgs::with_is_os_shutdown(true);

        let owner = if self.owner.is_on_thread() { self.owner.get().upgrade() } else { None };
        if let Some(owner) = owner {
            if owner.enable_session_management {
                for (_, handler) in owner.shutdown_requested.snapshot().iter() {
                    handler(&e);
                }
            }
        }

        self.calls.interact_done(smc_conn, e.cancel());

        if e.cancel() {
            return;
        }

        self.save_yourself_phase.store(false, Ordering::SeqCst);

        self.calls.save_yourself_done(smc_conn, true);
    }
}

/// The lifetime events of the platform.
pub struct X11PlatformLifetimeEvents {
    /// The option of the platform, read when the platform is initialized.
    enable_session_management: bool,
    shutdown_requested: HandlerList<dyn Fn(&ShutdownRequestedEventArgs)>,
    shared: Arc<Shared>,
    this: Weak<X11PlatformLifetimeEvents>,
}

impl X11PlatformLifetimeEvents {
    /// Connects to the session manager of the environment
    /// (`SESSION_MANAGER`). Without one, or without the libraries, the
    /// object exists and never raises its event; the reason is logged.
    pub fn new(enable_session_management: bool) -> Rc<Self> {
        let libraries = match (sm_lib(), ice_lib()) {
            (Ok(sm), Ok(ice)) => Some((sm, ice)),
            (Err(error), _) | (_, Err(error)) => {
                warn(&format!("Session management is not available: {error}"));
                None
            }
        };
        let calls: Box<dyn ISmConnectionCalls> = match libraries {
            Some((sm, ice)) => Box::new(LibraryCalls { sm, ice }),
            None => Box::new(NoCalls),
        };
        let this = Self::with_calls(enable_session_management, calls, Some(Dispatcher::ui_thread()));
        if let Some((sm, ice)) = libraries {
            this.connect(sm, ice);
        }
        this
    }

    pub(crate) fn with_calls(
        enable_session_management: bool,
        calls: Box<dyn ISmConnectionCalls>,
        dispatcher: Option<Arc<Dispatcher>>,
    ) -> Rc<Self> {
        Rc::new_cyclic(|this: &Weak<Self>| Self {
            enable_session_management,
            shutdown_requested: HandlerList::new(),
            shared: Arc::new(Shared {
                calls,
                current_smc_conn: AtomicUsize::new(0),
                current_ice_conn: AtomicUsize::new(0),
                save_yourself_phase: AtomicBool::new(false),
                cancellation_requested: AtomicBool::new(false),
                owner: ThreadBound::new(this.clone()),
                dispatcher,
            }),
            this: this.clone(),
        })
    }

    /// The constructor of the reference after its first line.
    fn connect(&self, sm: &'static SMLib, ice: &'static ICELib) {
        // SAFETY: the watch is a function of this module, which lives as long as the process.
        if unsafe { (ice.ice_add_connection_watch)(ice_watch_handler, std::ptr::null_mut()) } == 0 {
            warn("SMLib was unable to add an ICE connection watcher.");
            return;
        }

        let mut error_buf = [0 as c_char; 255];
        let mut client_id_ret: *mut c_char = std::ptr::null_mut();
        let mut callbacks = SmcCallbacks {
            save_yourself: Some(smc_save_yourself_handler),
            save_yourself_client_data: std::ptr::null_mut(),
            die: Some(smc_die_handler),
            die_client_data: std::ptr::null_mut(),
            save_complete: Some(smc_save_complete_handler),
            save_complete_client_data: std::ptr::null_mut(),
            shutdown_cancelled: Some(smc_shutdown_cancelled_handler),
            shutdown_cancelled_client_data: std::ptr::null_mut(),
        };
        // SAFETY: a null network identifier is the session manager of the environment; the
        // callbacks are copied by the call and are functions of this module; the two out
        // places are valid, and the error buffer has the length that is passed. The client
        // identifier the library allocates is kept for the life of the process, as in the
        // reference.
        let smc_conn = unsafe {
            (sm.smc_open_connection)(
                std::ptr::null(),
                std::ptr::null_mut(),
                1,
                0,
                SMC_SAVE_YOURSELF_PROC_MASK
                    | SMC_SAVE_COMPLETE_PROC_MASK
                    | SMC_SHUTDOWN_CANCELLED_PROC_MASK
                    | SMC_DIE_PROC_MASK,
                &mut callbacks,
                std::ptr::null(),
                &mut client_id_ret,
                error_buf.len() as c_int,
                error_buf.as_mut_ptr(),
            )
        } as usize;

        if smc_conn == 0 {
            error_buf[254] = 0;
            // SAFETY: the buffer is terminated: the library writes a terminated text, and
            // its last element was just set to zero.
            let error = unsafe { CStr::from_ptr(error_buf.as_ptr()) }.to_string_lossy();
            warn(&format!("SMLib/ICELib reported a new error: {error}"));
            return;
        }

        {
            let mut map = NATIVE_TO_MANAGED_MAPPER.lock().unwrap_or_else(PoisonError::into_inner);
            let map = map.get_or_insert_with(HashMap::new);
            if map.contains_key(&smc_conn) {
                warn("SMLib was unable to add this instance to the native to managed map.");
                return;
            }
            map.insert(smc_conn, self.shared.clone());
        }

        // SAFETY: the handlers are functions of this module; the connection is open.
        let ice_conn = unsafe {
            let _ = (sm.smc_set_error_handler)(Some(static_error_handler));
            let _ = (ice.ice_set_error_handler)(Some(static_error_handler));
            let _ = (ice.ice_set_io_error_handler)(Some(static_ice_io_error_handler));
            (sm.smc_get_ice_connection)(smc_conn as *mut c_void) as usize
        };

        self.shared.current_smc_conn.store(smc_conn, Ordering::SeqCst);
        self.shared.current_ice_conn.store(ice_conn, Ordering::SeqCst);

        let shared = self.shared.clone();
        let spawned = std::thread::Builder::new().name("X11 session management".to_string()).spawn(move || {
            while !shared.cancellation_requested.load(Ordering::SeqCst) {
                shared.handle_requests();
            }
        });
        if spawned.is_err() {
            warn("The thread that reads the session management connection could not be started.");
        }
    }

    /// Whether a session manager is connected.
    pub fn is_connected(&self) -> bool {
        self.shared.current_smc_conn.load(Ordering::SeqCst) != 0
    }

    pub fn dispose(&self) {
        self.shared.dispose();
    }

    #[cfg(test)]
    pub(crate) fn shared(&self) -> &Arc<Shared> {
        &self.shared
    }
}

/// The calls of an object that has no libraries: it has no connection, so
/// none of them is ever made.
struct NoCalls;

impl ISmConnectionCalls for NoCalls {
    fn save_yourself_done(&self, _smc_conn: usize, _success: bool) {}
    fn interact_request(&self, _smc_conn: usize, _client_data: usize) {}
    fn interact_done(&self, _smc_conn: usize, _cancel_shutdown: bool) {}
    fn close_connection(&self, _smc_conn: usize, _reason: &str) {}
    fn process_messages(&self, _ice_conn: usize) -> bool {
        false
    }
}

impl IPlatformLifetimeEventsImpl for X11PlatformLifetimeEvents {
    fn shutdown_requested(&self, handler: Rc<dyn Fn(&ShutdownRequestedEventArgs)>) -> Rc<dyn IDisposable> {
        let token = self.shutdown_requested.add(handler);
        let this = self.this.clone();
        Disposable::create(move || {
            if let Some(this) = this.upgrade() {
                this.shutdown_requested.remove(token);
            }
        })
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests for this file.
    use super::*;
    use std::cell::Cell;

    #[derive(Clone, Debug, PartialEq, Eq)]
    enum Call {
        SaveYourselfDone(usize, bool),
        InteractRequest(usize, usize),
        InteractDone(usize, bool),
        Close(usize),
    }

    struct Recorder(Arc<Mutex<Vec<Call>>>);

    impl ISmConnectionCalls for Recorder {
        fn save_yourself_done(&self, smc_conn: usize, success: bool) {
            self.0.lock().unwrap().push(Call::SaveYourselfDone(smc_conn, success));
        }
        fn interact_request(&self, smc_conn: usize, client_data: usize) {
            self.0.lock().unwrap().push(Call::InteractRequest(smc_conn, client_data));
        }
        fn interact_done(&self, smc_conn: usize, cancel_shutdown: bool) {
            self.0.lock().unwrap().push(Call::InteractDone(smc_conn, cancel_shutdown));
        }
        fn close_connection(&self, smc_conn: usize, _reason: &str) {
            self.0.lock().unwrap().push(Call::Close(smc_conn));
        }
        fn process_messages(&self, _ice_conn: usize) -> bool {
            false
        }
    }

    const CONN: usize = 0x5151;

    fn events(enabled: bool) -> (Rc<X11PlatformLifetimeEvents>, Arc<Mutex<Vec<Call>>>) {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let events = X11PlatformLifetimeEvents::with_calls(enabled, Box::new(Recorder(calls.clone())), None);
        events.shared().current_smc_conn.store(CONN, Ordering::SeqCst);
        (events, calls)
    }

    fn taken(calls: &Arc<Mutex<Vec<Call>>>) -> Vec<Call> {
        std::mem::take(&mut *calls.lock().unwrap())
    }

    #[test]
    fn a_save_without_a_shutdown_or_a_fast_one_is_answered_at_once() {
        let (events, calls) = events(true);
        let shared = events.shared();
        shared.save_yourself_handler(CONN, 7, false, false);
        assert_eq!(taken(&calls), [Call::SaveYourselfDone(CONN, true)]);
        assert!(!shared.save_yourself_phase.load(Ordering::SeqCst));

        shared.save_yourself_handler(CONN, 7, true, true);
        assert_eq!(taken(&calls), [Call::SaveYourselfDone(CONN, true)]);
        assert!(!shared.save_yourself_phase.load(Ordering::SeqCst));
    }

    #[test]
    fn a_shutdown_asks_to_interact_and_the_application_may_refuse_it() {
        let (events, calls) = events(true);
        let shared = events.shared();
        shared.save_yourself_handler(CONN, 7, true, false);
        assert_eq!(taken(&calls), [Call::InteractRequest(CONN, 7)]);
        assert!(shared.save_yourself_phase.load(Ordering::SeqCst));

        // A second request during the phase answers the first.
        shared.save_yourself_handler(CONN, 7, true, false);
        assert_eq!(taken(&calls), [Call::SaveYourselfDone(CONN, true), Call::InteractRequest(CONN, 7)]);

        // The application refuses: the interaction ends with a canceled shutdown, and the
        // phase stays open until the session manager says the shutdown was canceled.
        let refuse = Rc::new(Cell::new(true));
        let seen_os_shutdown = Rc::new(Cell::new(false));
        let (r, s) = (refuse.clone(), seen_os_shutdown.clone());
        let subscription = events.shutdown_requested(Rc::new(move |e| {
            s.set(e.is_os_shutdown());
            e.set_cancel(r.get());
        }));
        shared.actual_interact_handler(CONN);
        assert!(seen_os_shutdown.get());
        assert_eq!(taken(&calls), [Call::InteractDone(CONN, true)]);
        assert!(shared.save_yourself_phase.load(Ordering::SeqCst));
        shared.shutdown_cancelled_handler();
        assert_eq!(taken(&calls), [Call::SaveYourselfDone(CONN, true)]);
        assert!(!shared.save_yourself_phase.load(Ordering::SeqCst));
        // Outside the phase the cancellation is not answered.
        shared.shutdown_cancelled_handler();
        assert!(taken(&calls).is_empty());

        // The application agrees.
        refuse.set(false);
        shared.save_yourself_handler(CONN, 7, true, false);
        taken(&calls);
        shared.actual_interact_handler(CONN);
        assert_eq!(taken(&calls), [Call::InteractDone(CONN, false), Call::SaveYourselfDone(CONN, true)]);
        assert!(!shared.save_yourself_phase.load(Ordering::SeqCst));

        // Without its handler the shutdown goes on.
        subscription.dispose();
        refuse.set(true);
        shared.actual_interact_handler(CONN);
        assert_eq!(taken(&calls), [Call::InteractDone(CONN, false), Call::SaveYourselfDone(CONN, true)]);
    }

    #[test]
    fn with_session_management_turned_off_the_event_is_not_raised() {
        let (events, calls) = events(false);
        let raised = Rc::new(Cell::new(false));
        let r = raised.clone();
        let _subscription = events.shutdown_requested(Rc::new(move |e| {
            r.set(true);
            e.set_cancel(true);
        }));
        events.shared().actual_interact_handler(CONN);
        assert!(!raised.get());
        assert_eq!(taken(&calls), [Call::InteractDone(CONN, false), Call::SaveYourselfDone(CONN, true)]);
    }

    #[test]
    fn the_connection_is_closed_once_and_the_pump_stops() {
        let (events, calls) = events(true);
        let shared = events.shared();
        shared.save_complete_handler();
        // A lost connection (the recorder reports one) disposes.
        shared.handle_requests();
        assert_eq!(taken(&calls), [Call::Close(CONN)]);
        assert!(shared.cancellation_requested.load(Ordering::SeqCst));
        assert!(!events.is_connected());
        shared.die_handler();
        events.dispose();
        shared.handle_requests();
        assert!(taken(&calls).is_empty());
    }

    #[test]
    fn the_error_text_is_that_of_the_reference() {
        assert_eq!(error_message(0x1f, 0x20, 3, 2), "SMLib reported an error: severity 2 mOpcode 1F mSeq 20 errClass 3.");
    }
}

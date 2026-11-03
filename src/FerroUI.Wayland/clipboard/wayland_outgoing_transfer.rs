//! The data this application offers: the selection it sets and the drag it
//! starts (the port of `WaylandOutgoingTransfer.cs`).
//!
//! The reference object is used by closures of both threads. Here the object
//! and the data transfer of the application stay on the UI thread; the
//! callbacks of the data source, which run on the Wayland thread, hold it as
//! a handle and post to it. The table of the drags of this process, which the
//! reference keeps for the Wayland thread, is a table of the UI thread, where
//! the transfers live: an entry is made when the drag is asked for and
//! removed by a posted call when the source ends.

use super::wayland_clipboard_impl::serialize_for_mime_async;
use crate::server::cross_thread_result::{cross_thread_result, CrossThreadCompletion, CrossThreadResult};
use crate::server::interop::pipe2_stream::{Pipe2Stream, PipeDirection};
use crate::server::persistent::wayland_input_event_cookie::WaylandInputEventCookie;
use crate::server::transient::clipboard::wayland_data_device::{actions_to_effects, effects_to_actions, set_selection};
use crate::server::transient::clipboard::wayland_data_source::WaylandDataSource;
use crate::server::transient::wayland_globals::WaylandGlobals;
use crate::server::wayland_marshallers::UiThreadRef;
use crate::server::wayland_worker::WaylandWorker;
use ferroui_base::input::{
    DataFormatKind, DataTransfer, DragDropEffects, IAsyncDataTransfer, IDataTransfer, LocalBoxFuture, StandardCursorType,
};
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use std::cell::{Cell, RefCell};
use std::collections::hash_map::RandomState;
use std::collections::HashMap;
use std::hash::{BuildHasher, Hasher};
use std::io::Write;
use std::os::fd::OwnedFd;
use std::rc::Rc;
use std::sync::{Arc, Mutex, OnceLock, PoisonError};
use wayland_client::protocol::wl_data_device_manager::DndAction;

/// The prefix of the sentinel MIME type of a drag of this process. The reference has the
/// name of its project here (DEVIATIONS.md).
const IN_PROCESS_MIME_PREFIX: &str = "application/x-ferroui-inproc-dnd-";

/// 32 hexadecimal digits that differ from call to call and from process to process (the
/// `Guid.NewGuid().ToString("N")` of the reference): two values of the hasher the standard
/// library seeds at random.
fn new_identifier() -> String {
    let mut identifier = String::with_capacity(32);
    for _ in 0..2 {
        let mut hasher = RandomState::new().build_hasher();
        hasher.write_u32(std::process::id());
        identifier.push_str(&format!("{:016x}", hasher.finish()));
    }
    identifier
}

/// Full MIME prefix for this app instance: `application/x-ferroui-inproc-dnd-{instance}-`.
/// The instance part is a unique identifier of this application instance, used to
/// distinguish our own drags from drags originating in other processes of the framework.
fn instance_prefix() -> &'static str {
    static INSTANCE_PREFIX: OnceLock<String> = OnceLock::new();
    INSTANCE_PREFIX.get_or_init(|| format!("{IN_PROCESS_MIME_PREFIX}{}-", new_identifier()))
}

ferroui_base::ferro_thread_local! {
    /// Maps in-process DnD operation keys to the original data transfer.
    /// Accessed exclusively from the UI thread.
    /// When the receive side sees a sentinel MIME type matching this instance's prefix,
    /// it uses the original transfer directly instead of reading through Wayland pipes,
    /// which would deadlock (the source's `send` handler dispatches to the UI thread).
    static IN_PROCESS_DRAGS: RefCell<HashMap<String, Rc<dyn IDataTransfer>>> = RefCell::new(HashMap::new());
}

/// The result of a drag, which the first of several callbacks gives.
#[derive(Clone)]
struct DragResult(Arc<Mutex<Option<CrossThreadCompletion<DragDropEffects>>>>);

impl DragResult {
    fn try_set_result(&self, effects: DragDropEffects) {
        if let Some(completion) = self.0.lock().unwrap_or_else(PoisonError::into_inner).take() {
            completion.complete(effects);
        }
    }
}

/// Manages the outgoing side of a data transfer (clipboard set or DnD initiate).
/// Created on the UI thread; coordinates between the UI-thread data transfer
/// and the Wayland-thread data source.
///
/// The `send` callback dispatches to the UI thread so that user-provided data transfer
/// methods run on the correct thread.
pub struct WaylandOutgoingTransfer {
    transfer: Rc<dyn IAsyncDataTransfer>,
    mime_types: Vec<String>,
    /// Called on the UI thread when the data source is cancelled (lost ownership or DnD ended).
    cancelled: RefCell<Option<Rc<dyn Fn()>>>,
}

impl WaylandOutgoingTransfer {
    pub fn new(transfer: Rc<dyn IAsyncDataTransfer>) -> Rc<Self> {
        let mut mime_types = Vec::new();
        for format in transfer.formats().iter() {
            // In-process formats are only meaningful within this process and must never be
            // advertised to (or serialized for) other Wayland clients.
            if format.kind() == DataFormatKind::InProcess {
                continue;
            }
            mime_types.extend(super::wayland_mime_mapper::to_mime_types(format));
        }
        Rc::new(Self { transfer, mime_types, cancelled: RefCell::new(None) })
    }

    /// Sets what is called on the UI thread when the data source is cancelled.
    pub fn set_cancelled(&self, cancelled: Option<Rc<dyn Fn()>>) {
        *self.cancelled.borrow_mut() = cancelled;
    }

    fn raise_cancelled(&self) {
        let cancelled = self.cancelled.borrow().clone();
        if let Some(cancelled) = cancelled {
            cancelled();
        }
    }

    /// Creates a data source on the Wayland thread, configures it,
    /// and sets it as the clipboard selection.
    /// Returns a result that completes when the selection has been set.
    pub fn set_as_selection(self: &Rc<Self>, worker: &Arc<WaylandWorker>) -> CrossThreadResult<()> {
        let this = UiThreadRef::new(self.clone(), Dispatcher::ui_thread());
        let mime_types = self.mime_types.clone();
        worker.invoke_oob_async(move |worker| {
            let Some(globals) = worker.state.globals.as_mut() else {
                return;
            };

            let Some(seat) = globals.input_dispatcher.get_data_device().map(|device| device.seat()) else {
                return;
            };

            let Some(mut source) = Self::create_source(globals, &mime_types, &this) else {
                return;
            };

            let id = source.id();
            source.on_cancelled = Some(Box::new(move |state, ()| {
                if let Some(globals) = state.globals.as_mut() {
                    if let Some(device) = globals.input_dispatcher.data_device_of_mut(seat) {
                        if device.active_source.as_ref() == Some(&id) {
                            device.active_source = None;
                        }
                    }
                    globals.dispose_data_source(&id);
                }
                this.post(|this| this.raise_cancelled());
            }));

            set_selection(globals, Some(source));
        })
    }

    /// Creates a data source on the Wayland thread, configures it,
    /// and starts a drag-and-drop operation using `wl_data_device.start_drag`.
    /// The `input_cookie` runs the command with the globals of the connection it was made
    /// on, ensuring reconnect-safe access to transient state.
    /// Returns a future that resolves to the resulting effects when
    /// the DnD operation completes (finished, cancelled, or failed to start).
    ///
    /// `data_transfer` is the transfer of the drag as the application gave it: what a
    /// surface of this process receives when the drag enters it.
    pub fn set_as_drag(
        self: &Rc<Self>,
        data_transfer: Rc<dyn IDataTransfer>,
        input_cookie: Arc<WaylandInputEventCookie>,
        allowed_effects: DragDropEffects,
    ) -> LocalBoxFuture<DragDropEffects> {
        let (completion, result) = cross_thread_result();
        let tcs = DragResult(Arc::new(Mutex::new(Some(completion))));
        let this = UiThreadRef::new(self.clone(), Dispatcher::ui_thread());
        let mime_types = self.mime_types.clone();

        // Register for in-process detection so the receive side can use
        // the original data transfer directly instead of reading through
        // Wayland pipes (which would deadlock).
        let operation_key = new_identifier();
        IN_PROCESS_DRAGS.with(|drags| drags.borrow_mut().insert(operation_key.clone(), data_transfer));
        let remove_in_process_drag = {
            let this = this.clone();
            let operation_key = operation_key.clone();
            move || {
                let operation_key = operation_key.clone();
                this.post(move |_| {
                    IN_PROCESS_DRAGS.with(|drags| drags.borrow_mut().remove(&operation_key));
                });
            }
        };

        let cookie = input_cookie.clone();
        let posted = {
            let tcs = tcs.clone();
            let remove_in_process_drag = remove_in_process_drag.clone();
            move |worker: &mut crate::server::wayland_worker::WaylandWorkerThread| {
                let fail = |tcs: &DragResult| {
                    remove_in_process_drag();
                    tcs.try_set_result(DragDropEffects::NONE);
                };
                let Some(globals) = worker.state.globals.as_mut() else {
                    fail(&tcs);
                    return;
                };

                if globals.input_dispatcher.get_data_device().is_none() {
                    fail(&tcs);
                    return;
                }

                let Some(mut source) = Self::create_source(globals, &mime_types, &this) else {
                    fail(&tcs);
                    return;
                };

                source.offer(&format!("{}{operation_key}", instance_prefix()));

                let id = source.id();
                let allowed_actions = effects_to_actions(allowed_effects);
                let negotiated_effects = Rc::new(Cell::new(DragDropEffects::NONE));

                source.on_action = Some(Box::new({
                    let negotiated_effects = negotiated_effects.clone();
                    move |state, action: DndAction| {
                        negotiated_effects.set(actions_to_effects(action));
                        let cursor_type = if action == DndAction::Copy {
                            StandardCursorType::DragCopy
                        } else if action == DndAction::Move {
                            StandardCursorType::DragMove
                        } else if action == DndAction::Ask {
                            StandardCursorType::DragLink
                        } else {
                            StandardCursorType::No
                        };
                        if let Some(globals) = &state.globals {
                            globals.input_dispatcher.set_dnd_cursor(cursor_type, &globals.cursor_manager);
                        }
                    }
                }));

                source.on_cancelled = Some(Box::new({
                    let tcs = tcs.clone();
                    let this = this.clone();
                    let remove_in_process_drag = remove_in_process_drag.clone();
                    let id = id.clone();
                    move |state, ()| {
                        remove_in_process_drag();
                        if let Some(globals) = state.globals.as_mut() {
                            globals.dispose_data_source(&id);
                        }
                        let tcs = tcs.clone();
                        this.post(move |this| {
                            this.raise_cancelled();
                            tcs.try_set_result(DragDropEffects::NONE);
                        });
                    }
                }));

                source.on_dnd_finished = Some(Box::new({
                    let tcs = tcs.clone();
                    let this = this.clone();
                    let remove_in_process_drag = remove_in_process_drag.clone();
                    let id = id.clone();
                    move |state, ()| {
                        remove_in_process_drag();
                        if let Some(globals) = state.globals.as_mut() {
                            globals.dispose_data_source(&id);
                        }
                        let result = negotiated_effects.get();
                        let tcs = tcs.clone();
                        this.post(move |_| tcs.try_set_result(result));
                    }
                }));

                source.on_dnd_drop_performed = Some(Box::new(|_, ()| {
                    // Drop performed by user but destination may still reject —
                    // wait for DndFinished or Cancelled to resolve the result.
                }));

                let connection_id = globals.connection_id;
                let origin_surface = globals.input_dispatcher.find_origin_surface();
                let started = globals.input_dispatcher.get_data_device().is_some_and(|device| {
                    device.start_drag(&source, &cookie, allowed_actions, connection_id, origin_surface.as_ref())
                });
                if started {
                    globals.data_sources.insert(id, source);
                } else {
                    source.dispose();
                    fail(&tcs);
                }
            }
        };
        input_cookie.post_oob(posted);

        Box::pin(async move {
            // A command that never ran (the compositor was replaced since the press) gives
            // no result: the drag did not happen.
            let effects = result.await;
            if effects.is_none() {
                remove_in_process_drag();
            }
            effects.unwrap_or(DragDropEffects::NONE)
        })
    }

    /// Creates and configures a data source with MIME type offers
    /// and a send handler that dispatches to the UI thread.
    fn create_source(globals: &WaylandGlobals, mime_types: &[String], this: &UiThreadRef<Self>) -> Option<WaylandDataSource> {
        let manager = globals.data_device_manager.as_ref()?;

        let wl_source = manager.create_data_source(globals.queue_handle(), ());
        let mut source = WaylandDataSource::new(wl_source);

        for mime in mime_types {
            source.offer(mime);
        }

        let this = this.clone();
        source.on_send = Some(Box::new(move |_, (mime, fd): (String, OwnedFd)| Self::handle_send(&this, mime, fd)));

        Some(source)
    }

    /// Called on the Wayland thread when the compositor requests data.
    /// Dispatches to the UI thread to read from the user's data transfer and write to the pipe fd.
    fn handle_send(this: &UiThreadRef<Self>, mime: String, fd: OwnedFd) {
        this.post(move |this| {
            let transfer = this.transfer.clone();
            // The data is taken from the transfer on the UI thread and written on a thread of
            // its own, so that a reader that is slow does not hold the UI thread up. A
            // transfer that fails, and one that has nothing for the type, write nothing:
            // the descriptor is closed either way.
            drop(Dispatcher::ui_thread().spawn_local_with_priority(
                async move {
                    let bytes = serialize_for_mime_async(&transfer, &mime).await;
                    let spawned = std::thread::Builder::new().name("FerroWaylandTransfer".to_string()).spawn(move || {
                        let mut stream = Pipe2Stream::new(fd, PipeDirection::Out);
                        if let Some(bytes) = bytes {
                            let _ = stream.write_all(&bytes);
                        }
                    });
                    drop(spawned);
                },
                DispatcherPriority::DEFAULT,
            ));
        });
    }

    /// Checks whether the offered MIME types contain an in-process sentinel from this app instance.
    /// If so, returns the original data transfer directly, avoiding
    /// pipe-based reads that would deadlock on self-drags.
    /// If the sentinel matches our instance but the operation key is not found
    /// (e.g., source already cleaned up), returns an empty data transfer
    /// as a safe dummy — we cannot interact with our own data offer via pipes.
    /// Called on the UI thread when a drag enters a surface.
    pub(crate) fn try_get_in_process_transfer(mime_types: &[String]) -> Option<Rc<dyn IDataTransfer>> {
        let instance_prefix = instance_prefix();
        for mime in mime_types {
            let Some(operation_key) = mime.strip_prefix(instance_prefix) else {
                continue;
            };

            // Found our instance prefix — either return the real transfer or a safe dummy
            return Some(IN_PROCESS_DRAGS.with(|drags| {
                drags.borrow().get(operation_key).cloned().unwrap_or_else(|| DataTransfer::new() as Rc<dyn IDataTransfer>)
            }));
        }

        None
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests for this file.
    use super::*;

    #[test]
    fn identifiers_are_32_hexadecimal_digits_and_differ() {
        let first = new_identifier();
        let second = new_identifier();
        assert_eq!(first.len(), 32);
        assert!(first.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(first, second);
    }

    #[test]
    fn a_drag_of_this_process_is_found_by_its_sentinel_type() {
        let transfer: Rc<dyn IDataTransfer> = DataTransfer::new();
        IN_PROCESS_DRAGS.with(|drags| drags.borrow_mut().insert("abc".to_string(), transfer.clone()));

        let offered = vec!["text/plain".to_string(), format!("{}abc", instance_prefix())];
        let found = WaylandOutgoingTransfer::try_get_in_process_transfer(&offered).unwrap();
        assert!(Rc::ptr_eq(&found, &transfer));

        // A drag of this process that ended: an empty transfer, not the pipes.
        let ended = vec![format!("{}gone", instance_prefix())];
        let dummy = WaylandOutgoingTransfer::try_get_in_process_transfer(&ended).unwrap();
        assert!(dummy.formats().is_empty());

        // The drag of another process of the framework, and of anything else.
        let other = vec![format!("{IN_PROCESS_MIME_PREFIX}{}-abc", "0".repeat(32)), "text/plain".to_string()];
        assert!(WaylandOutgoingTransfer::try_get_in_process_transfer(&other).is_none());

        IN_PROCESS_DRAGS.with(|drags| drags.borrow_mut().clear());
    }

    #[test]
    fn in_process_formats_are_not_offered() {
        use ferroui_base::input::{DataFormat, DataTransferItem};
        let transfer = DataTransfer::new();
        let item = DataTransferItem::new();
        item.set(&DataFormat::text(), Some("text".to_string()));
        item.set(&DataFormat::create_in_process_format::<String>("private"), Some("x".to_string()));
        item.set(&DataFormat::create_bytes_application_format("thing"), Some(Rc::from([1u8, 2].as_slice())));
        transfer.add(item);
        let outgoing = WaylandOutgoingTransfer::new(transfer);
        assert_eq!(outgoing.mime_types, ["text/plain;charset=utf-8", "text/plain", "application/x-ferroui-thing"]);
    }
}

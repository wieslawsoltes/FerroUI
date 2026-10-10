//! The clipboard over a selection of the X server (the port of
//! `X11ClipboardImpl.cs`).

use crate::selections::clipboard::clipboard_data_reader::ClipboardDataReader;
use crate::selections::clipboard::clipboard_data_transfer::ClipboardDataTransfer;
use crate::selections::clipboard::clipboard_read_session_factory;
use crate::selections::data_format_helper;
use crate::selections::selection_data_provider::SelectionDataProvider;
use crate::selections::selection_data_reader::create_items_async;
use crate::selections::selection_helper::{create_event_window, TaskCompletionSource};
use crate::x11_platform::FerroX11Platform;
use crate::x11_structs::XEventName;
use crate::xlib::{self, Atom, PropertyMode, XEvent, XID};
use ferroui_base::input::platform::{ClipboardError, IClipboardImpl, IOwnedClipboardImpl};
use ferroui_base::input::{AsyncDataTransferExtensions, DataFormat, IAsyncDataTransfer, LocalBoxFuture};
use ferroui_base::reactive::IDisposable;
use std::cell::{Cell, RefCell};
use std::future::ready;
use std::rc::{Rc, Weak};

/// The clipboard of a selection (`CLIPBOARD` or `PRIMARY`).
///
/// The reference class derives from the selection data provider; here it
/// holds one.
pub struct X11ClipboardImpl {
    weak_self: Weak<X11ClipboardImpl>,
    provider: SelectionDataProvider,
    selection: Atom,
    store_atom_tcs: RefCell<Option<Rc<TaskCompletionSource<()>>>>,
    handle: Cell<XID>,
}

impl X11ClipboardImpl {
    pub fn new(platform: &Rc<FerroX11Platform>, selection: Atom) -> Rc<X11ClipboardImpl> {
        let this = Rc::new_cyclic(|weak_self| X11ClipboardImpl {
            weak_self: weak_self.clone(),
            provider: SelectionDataProvider::new(platform, selection),
            selection,
            store_atom_tcs: RefCell::new(None),
            handle: Cell::new(0),
        });

        let weak = Rc::downgrade(&this);
        let handle = create_event_window(
            platform,
            Rc::new(move |evt: &mut XEvent| {
                if let Some(this) = weak.upgrade() {
                    this.on_event(evt);
                }
            }),
        );
        this.handle.set(handle);

        this
    }

    /// The part of the clipboard that answers the requests for the
    /// selection.
    pub fn provider(&self) -> &SelectionDataProvider {
        &self.provider
    }

    fn store_atom_tcs(&self) -> Option<Rc<TaskCompletionSource<()>>> {
        self.store_atom_tcs.borrow().clone()
    }

    fn on_event(&self, evt: &mut XEvent) {
        let event_type = xlib::event_type(evt);

        if event_type == XEventName::SelectionClear as i32 {
            // We might have already regained the clipboard ownership by the time a SelectionClear message arrives.
            if self.provider.get_owner() != self.handle.get() {
                self.provider.set_data_transfer(None);
            }

            if let Some(store_atom_tcs) = self.store_atom_tcs() {
                store_atom_tcs.try_set_result(());
            }
        } else if event_type == XEventName::SelectionNotify as i32 {
            let atoms = self.provider.info().atoms();
            let selection_event = xlib::selection_event(evt);

            if selection_event.selection == atoms.CLIPBOARD_MANAGER && selection_event.target == atoms.SAVE_TARGETS {
                if let Some(store_atom_tcs) = self.store_atom_tcs() {
                    store_atom_tcs.try_set_result(());
                }
            }
        } else if event_type == XEventName::SelectionRequest as i32 {
            self.provider.on_selection_request(evt);
        }
    }

    fn store_atoms_in_clipboard_manager(
        &self,
        data_transfer: Rc<dyn IAsyncDataTransfer>,
    ) -> LocalBoxFuture<Result<(), ClipboardError>> {
        let atoms = self.provider.info().atoms();

        // The clipboard manager (SAVE_TARGETS) protocol only applies to the CLIPBOARD selection.
        if self.selection != atoms.CLIPBOARD {
            return Box::pin(ready(Ok(())));
        }

        let clipboard_manager = xlib::x_get_selection_owner(self.provider.display(), atoms.CLIPBOARD_MANAGER);
        if clipboard_manager == 0 {
            return Box::pin(ready(Ok(())));
        }

        // Skip storing atoms if the data object contains any non-trivial formats
        let formats = match data_transfer.try_formats() {
            Ok(formats) => formats,
            Err(error) => return Box::pin(ready(Err(error))),
        };
        if formats.iter().any(|f| DataFormat::text() != *f) {
            return Box::pin(ready(Ok(())));
        }

        let Some(this) = self.weak_self.upgrade() else {
            return Box::pin(ready(Ok(())));
        };

        // `StoreTextCoreAsync`
        Box::pin(async move {
            // Skip storing atoms if the trivial formats are too big
            let Some(text) = data_transfer.try_get_text_async().await? else {
                return Ok(());
            };
            if text.encode_utf16().count() * 2 > 64 * 1024 {
                return Ok(());
            }

            let store_atom_tcs = {
                let mut store_atom_tcs = this.store_atom_tcs.borrow_mut();
                match store_atom_tcs.as_ref() {
                    Some(pending) if !pending.is_completed() => pending.clone(),
                    _ => {
                        let created = TaskCompletionSource::new();
                        *store_atom_tcs = Some(created.clone());
                        created
                    }
                }
            };

            let atoms = this.provider.info().atoms();
            let display = this.provider.display();
            let atom_values = this.provider.convert_data_transfer(Some(&*data_transfer));

            xlib::x_change_property_longs(
                display,
                this.handle.get(),
                atoms.FERRO_SAVE_TARGETS_PROPERTY_ATOM,
                atoms.ATOM,
                PropertyMode::Replace,
                &atom_values,
            );

            xlib::x_convert_selection(
                display,
                atoms.CLIPBOARD_MANAGER,
                atoms.SAVE_TARGETS,
                atoms.FERRO_SAVE_TARGETS_PROPERTY_ATOM,
                this.handle.get(),
                0,
            );

            store_atom_tcs.task().await;
            Ok(())
        })
    }

    async fn try_get_data_core_async(&self) -> Result<Option<Rc<dyn IAsyncDataTransfer>>, ClipboardError> {
        let owner = self.provider.get_owner();
        if owner == 0 {
            return Ok(None);
        }

        if owner == self.handle.get() {
            if let Some(stored_data_transfer) = self.provider.data_transfer() {
                return Ok(Some(stored_data_transfer));
            }
        }

        // Get the formats while we're in an async method, since the formats of a data transfer are synchronous.
        let (data_formats, text_format_atoms) = self.get_data_formats_core_async().await;
        if data_formats.is_empty() {
            return Ok(None);
        }

        let Some(platform) = self.provider.platform() else {
            return Ok(None);
        };

        // Get the items while we're in an async method. This does not get values, except for the file format.
        let reader =
            ClipboardDataReader::new(&platform, self.selection, text_format_atoms, data_formats.clone(), owner);
        let items = create_items_async(&reader).await?;
        Ok(Some(ClipboardDataTransfer::new(reader, data_formats, items)))
    }

    async fn get_data_formats_core_async(&self) -> (Vec<DataFormat>, Vec<Atom>) {
        let Some(platform) = self.provider.platform() else {
            return (Vec::new(), Vec::new());
        };

        let session = clipboard_read_session_factory::create_session(&platform, self.selection);

        let format_atoms = session.send_format_request(0).await.unwrap_or_default();
        session.dispose();
        data_format_helper::to_data_formats(&format_atoms, self.provider.info().atoms())
    }
}

impl IClipboardImpl for X11ClipboardImpl {
    fn try_get_data_async(&self) -> LocalBoxFuture<Result<Option<Rc<dyn IAsyncDataTransfer>>, ClipboardError>> {
        let Some(this) = self.weak_self.upgrade() else {
            return Box::pin(ready(Ok(None)));
        };

        Box::pin(async move { this.try_get_data_core_async().await })
    }

    fn set_data_async(&self, data_transfer: Rc<dyn IAsyncDataTransfer>) -> LocalBoxFuture<Result<(), ClipboardError>> {
        self.provider.set_data_transfer(Some(data_transfer.clone()));
        self.provider.set_owner(self.handle.get());
        self.store_atoms_in_clipboard_manager(data_transfer)
    }

    fn clear_async(&self) -> LocalBoxFuture<Result<(), ClipboardError>> {
        self.provider.set_data_transfer(None);
        self.provider.set_owner(0);
        Box::pin(ready(Ok(())))
    }

    fn as_owned_clipboard_impl(&self) -> Option<&dyn IOwnedClipboardImpl> {
        Some(self)
    }
}

impl IOwnedClipboardImpl for X11ClipboardImpl {
    fn is_current_owner_async(&self) -> LocalBoxFuture<Result<bool, ClipboardError>> {
        Box::pin(ready(Ok(self.provider.get_owner() == self.handle.get())))
    }
}

impl IDisposable for X11ClipboardImpl {
    fn dispose(&self) {
        let handle = self.handle.get();
        if handle == 0 {
            return;
        }

        if let Some(platform) = self.provider.platform() {
            platform.remove_window(handle);
        }
        xlib::x_destroy_window(self.provider.display(), handle);
        self.handle.set(0);
    }
}

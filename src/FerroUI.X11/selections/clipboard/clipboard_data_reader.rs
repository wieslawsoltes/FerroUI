//! Reading the values of the clipboard (the port of
//! `ClipboardDataReader.cs`).

use crate::selections::clipboard::clipboard_data_transfer_item::ClipboardDataTransferItem;
use crate::selections::clipboard::clipboard_read_session_factory;
use crate::selections::selection_data_reader::{
    try_get_async_core, ISelectionDataReader, RawValueFuture, SelectionDataReader,
};
use crate::selections::selection_read_session::SelectionReadSession;
use crate::x11_platform::FerroX11Platform;
use crate::xlib::{self, Atom, XDisplay, XID};
use ferroui_base::input::platform::chromium_clipboard::{ChromiumClipboard, FormatAlias, MAX_MAP_BYTES};
use ferroui_base::input::platform::PlatformDataTransferItem;
use ferroui_base::input::{DataFormat, IAsyncDataTransferItem};
use std::cell::{Cell, OnceCell};
use std::rc::{Rc, Weak};

/// An object used to read values, converted to the correct format, from the X11 clipboard.
pub struct ClipboardDataReader {
    base: SelectionDataReader,
    platform: Weak<FerroX11Platform>,
    display: XDisplay,
    selection: Atom,
    owner: Cell<XID>,
    web_formats: OnceCell<Vec<FormatAlias>>,
}

impl ClipboardDataReader {
    pub fn new(
        platform: &Rc<FerroX11Platform>,
        selection: Atom,
        text_format_atoms: Vec<Atom>,
        data_formats: Vec<DataFormat>,
        owner: XID,
    ) -> Rc<Self> {
        Rc::new(Self {
            base: SelectionDataReader::new(platform.info().clone(), text_format_atoms, data_formats),
            platform: Rc::downgrade(platform),
            display: platform.display(),
            selection,
            owner: Cell::new(owner),
            web_formats: OnceCell::new(),
        })
    }

    fn is_owner_still_valid(&self) -> bool {
        self.owner.get() != 0 && xlib::x_get_selection_owner(self.display, self.selection) == self.owner.get()
    }

    pub(crate) async fn read_web_formats(&self, formats: &[DataFormat]) -> Vec<DataFormat> {
        let clipboard = ChromiumClipboard::Linux;
        let Some(map) = clipboard.find_map(formats) else {
            return Vec::new();
        };
        if !self.is_owner_still_valid() {
            return Vec::new();
        }
        let Some(session) = self.create_read_session() else {
            return Vec::new();
        };
        let atom = self.base.atoms().get_atom(map.identifier());
        let data = session.send_bounded_data_request(atom, 0, MAX_MAP_BYTES).await;
        session.dispose();
        let Some(data) = data.filter(|data| data.type_atom() == atom && self.is_owner_still_valid()) else {
            return Vec::new();
        };
        self.web_formats
            .get_or_init(|| clipboard.parse_map(data.as_bytes(), formats))
            .iter()
            .map(|alias| alias.application.clone())
            .collect()
    }
}

impl ISelectionDataReader for ClipboardDataReader {
    type Item = dyn IAsyncDataTransferItem;

    fn reader(&self) -> &SelectionDataReader {
        &self.base
    }

    fn try_get_async(self: Rc<Self>, format: &DataFormat) -> RawValueFuture {
        let alias = self.web_formats.get().and_then(|aliases| {
            aliases.iter().find(|alias| alias.application == *format).cloned()
        });
        let format = alias.as_ref().map_or_else(|| format.clone(), |alias| (*alias.slot).clone());
        Box::pin(async move {
            if !self.is_owner_still_valid() {
                return Ok(None);
            }
            let value = try_get_async_core(self.clone(), &format).await?;
            if !self.is_owner_still_valid() {
                return Ok(None);
            }
            Ok(match alias {
                Some(alias) => value.and_then(|value| alias.decode(value)),
                None => value,
            })
        })
    }

    fn create_single_item(self: Rc<Self>, mut non_file_formats: Vec<DataFormat>) -> Rc<dyn IAsyncDataTransferItem> {
        if let Some(aliases) = self.web_formats.get() {
            non_file_formats.extend(aliases.iter().map(|alias| alias.application.clone()));
        }
        ClipboardDataTransferItem::new(self, non_file_formats)
    }

    fn create_file_item(item: Rc<PlatformDataTransferItem>) -> Rc<dyn IAsyncDataTransferItem> {
        item
    }

    fn create_read_session(&self) -> Option<SelectionReadSession> {
        let platform = self.platform.upgrade()?;
        Some(clipboard_read_session_factory::create_session(&platform, self.selection))
    }

    fn dispose(&self) {
        self.owner.set(0);
    }
}

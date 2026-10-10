//! Reading the values of a selection in the formats of the framework (the
//! port of `SelectionDataReader.cs`).

use crate::selections::data_format_helper;
use crate::selections::selection_read_session::{GetDataResult, SelectionReadSession};
use crate::selections::uri_list_helper;
use crate::x11_atoms::X11Atoms;
use crate::x11_info::X11Info;
use crate::xlib::Atom;
use ferroui_base::input::platform::{ClipboardError, PlatformDataTransferItem};
use ferroui_base::input::{DataFormat, LocalBoxFuture};
use ferroui_base::media::imaging::Bitmap;
use ferroui_base::platform::storage::IStorageItem;
use std::any::Any;
use std::rc::Rc;

/// A value of a selection, or the failure of reading it.
pub type RawValueFuture = LocalBoxFuture<Result<Option<Rc<dyn Any>>, ClipboardError>>;

/// An object used to read values, converted to the correct format, from an X11 selection (clipboard/drag-and-drop).
///
/// The reference class is the abstract, generic base of the readers; here
/// it is the part they hold, and [`ISelectionDataReader`] has the members
/// a reader overrides.
pub struct SelectionDataReader {
    info: Rc<X11Info>,
    text_format_atoms: Vec<Atom>,
    data_formats: Vec<DataFormat>,
}

impl SelectionDataReader {
    pub fn new(info: Rc<X11Info>, text_format_atoms: Vec<Atom>, data_formats: Vec<DataFormat>) -> Self {
        Self { info, text_format_atoms, data_formats }
    }

    pub fn atoms(&self) -> &X11Atoms {
        self.info.atoms()
    }
}

/// The members of a reader of a selection that depend on the selection.
pub trait ISelectionDataReader: 'static {
    /// The items the reader makes (`TItem`).
    type Item: ?Sized + 'static;

    /// The part every reader has.
    fn reader(&self) -> &SelectionDataReader;

    /// Reads the value of a format. The value has the type of the format:
    /// `String` for the text format, `Rc<Bitmap>` for the bitmap format,
    /// `Vec<Rc<dyn IStorageItem>>` (every file of the selection) for the
    /// file format and `Rc<[u8]>` for every other format.
    ///
    /// The reference returns a string for the formats an application
    /// declares with the string type and bytes for the others; a format of
    /// the port does not carry its type, and the formats of a selection
    /// are all made with the type of bytes, so bytes it is.
    fn try_get_async(self: Rc<Self>, format: &DataFormat) -> RawValueFuture {
        try_get_async_core(self, format)
    }

    fn create_single_item(self: Rc<Self>, non_file_formats: Vec<DataFormat>) -> Rc<Self::Item>;

    /// The item of a file as an item of the reader (the cast of the
    /// reference).
    fn create_file_item(item: Rc<PlatformDataTransferItem>) -> Rc<Self::Item>;

    /// A session to read the selection with; `None` when the platform is
    /// gone.
    fn create_read_session(&self) -> Option<SelectionReadSession>;

    fn dispose(&self);
}

/// The items of the selection: one for every file, and one for all the
/// other formats.
pub fn create_items_async<R: ISelectionDataReader>(
    reader: &Rc<R>,
) -> LocalBoxFuture<Result<Vec<Rc<R::Item>>, ClipboardError>> {
    let reader = reader.clone();
    Box::pin(async move {
        let mut non_file_formats: Option<Vec<DataFormat>> = None;
        let mut items: Vec<Rc<R::Item>> = Vec::new();
        let mut has_files = false;

        let data_formats = reader.reader().data_formats.clone();
        for format in &data_formats {
            if DataFormat::file() == *format {
                if has_files {
                    continue;
                }

                let value = reader.clone().try_get_async(format).await?;
                if let Some(storage_items) =
                    value.as_deref().and_then(|value| value.downcast_ref::<Vec<Rc<dyn IStorageItem>>>())
                {
                    has_files = true;

                    for storage_item in storage_items {
                        items.push(R::create_file_item(PlatformDataTransferItem::create(
                            &DataFormat::file(),
                            storage_item.clone(),
                        )));
                    }
                }
            } else {
                non_file_formats.get_or_insert_with(Vec::new).push(format.clone());
            }
        }

        // Single item containing all formats except for DataFormat.File.
        if let Some(non_file_formats) = non_file_formats {
            items.push(reader.clone().create_single_item(non_file_formats));
        }

        Ok(items)
    })
}

/// The reading every reader does (the base implementation of
/// `TryGetAsync`).
pub fn try_get_async_core<R: ISelectionDataReader + ?Sized>(reader: Rc<R>, format: &DataFormat) -> RawValueFuture {
    let format = format.clone();
    Box::pin(async move {
        let base = reader.reader();
        let format_atom =
            data_format_helper::to_atom(&format, &base.text_format_atoms, base.atoms(), &base.data_formats);
        if format_atom == 0 {
            return Ok(None);
        }

        let Some(session) = reader.create_read_session() else {
            return Ok(None);
        };
        let result = session.send_data_request(format_atom, 0).await;
        session.dispose();
        convert_data_result(result, &format, format_atom, base.atoms())
    })
}

/// The value of a format from the data of a selection target.
pub fn convert_data_result(
    result: Option<GetDataResult>,
    format: &DataFormat,
    format_atom: Atom,
    atoms: &X11Atoms,
) -> Result<Option<Rc<dyn Any>>, ClipboardError> {
    let Some(result) = result else {
        return Ok(None);
    };

    if DataFormat::text() == *format {
        return Ok(data_format_helper::try_get_string_encoding(result.type_atom(), atoms)
            .map(|text_encoding| Rc::new(text_encoding.get_string(result.as_bytes())) as Rc<dyn Any>));
    }

    if DataFormat::bitmap() == *format {
        // A bitmap that cannot be decoded is a failure of the read (the
        // reference throws the exception of the bitmap constructor).
        return match Bitmap::from_stream(&mut result.as_stream()) {
            Ok(bitmap) => Ok(Some(Rc::new(Rc::new(bitmap)) as Rc<dyn Any>)),
            Err(error) => Err(ClipboardError::other(format!("Unable to load the bitmap: {error}"))),
        };
    }

    if DataFormat::file() == *format {
        // text/uri-list might not be supported
        return Ok(if format_atom != 0 && result.type_atom() == format_atom {
            Some(Rc::new(uri_list_helper::utf8_bytes_to_file_uri_list(result.as_bytes())) as Rc<dyn Any>)
        } else {
            None
        });
    }

    let bytes: Rc<[u8]> = Rc::from(result.into_bytes());
    Ok(Some(Rc::new(bytes)))
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests for this file.
    use super::*;

    fn atoms() -> X11Atoms {
        let mut next = 100;
        X11Atoms::with_interned(|_| {
            next += 1;
            next
        })
    }

    #[test]
    fn there_is_no_value_without_data() {
        let atoms = atoms();
        let text: DataFormat = DataFormat::text().into();
        assert!(convert_data_result(None, &text, atoms.UTF8_STRING, &atoms).unwrap().is_none());
    }

    #[test]
    fn text_is_decoded_by_the_type_of_the_answer() {
        let atoms = atoms();
        let text: DataFormat = DataFormat::text().into();

        // UTF-8 was asked for; the owner answered with UTF-16.
        let bytes: Vec<u8> = "h\u{e9}".encode_utf16().flat_map(u16::to_le_bytes).collect();
        let result = GetDataResult::new(bytes, atoms.UTF16_STRING);
        let value = convert_data_result(Some(result), &text, atoms.UTF8_STRING, &atoms).unwrap().unwrap();
        assert_eq!(value.downcast_ref::<String>().unwrap(), "h\u{e9}");

        let result = GetDataResult::new("h\u{e9}".as_bytes().to_vec(), atoms.UTF8_STRING);
        let value = convert_data_result(Some(result), &text, atoms.UTF8_STRING, &atoms).unwrap().unwrap();
        assert_eq!(value.downcast_ref::<String>().unwrap(), "h\u{e9}");

        let result = GetDataResult::new(vec![b'h', 0xe9], atoms.STRING);
        let value = convert_data_result(Some(result), &text, atoms.UTF8_STRING, &atoms).unwrap().unwrap();
        assert_eq!(value.downcast_ref::<String>().unwrap(), "h?");

        // An answer of a type that is not text.
        let result = GetDataResult::new(b"text".to_vec(), atoms.TARGETS);
        assert!(convert_data_result(Some(result), &text, atoms.UTF8_STRING, &atoms).unwrap().is_none());
    }

    #[test]
    fn other_formats_are_bytes() {
        let atoms = atoms();
        let format: DataFormat = DataFormat::create_bytes_platform_format("application/x-thing").into();
        let result = GetDataResult::new(vec![1, 2, 3], 777);
        let value = convert_data_result(Some(result), &format, 777, &atoms).unwrap().unwrap();
        assert_eq!(&**value.downcast_ref::<Rc<[u8]>>().unwrap(), [1, 2, 3]);
    }

    #[test]
    fn files_need_an_answer_of_the_type_that_was_asked_for() {
        let atoms = atoms();
        let file: DataFormat = DataFormat::file().into();

        let result = GetDataResult::new(b"file:///x\r\n".to_vec(), 778);
        assert!(convert_data_result(Some(result), &file, 777, &atoms).unwrap().is_none());

        // An answer of the right type is a list of the files that exist.
        let result = GetDataResult::new(b"file:///this/path/does/not/exist/anywhere-4f1c\r\n".to_vec(), 777);
        let value = convert_data_result(Some(result), &file, 777, &atoms).unwrap().unwrap();
        assert!(value.downcast_ref::<Vec<Rc<dyn IStorageItem>>>().unwrap().is_empty());
    }
}

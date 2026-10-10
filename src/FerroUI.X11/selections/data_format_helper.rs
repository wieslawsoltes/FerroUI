//! The data formats of the framework as the atoms of a selection and back
//! (the port of `DataFormatHelper.cs`).

use crate::x11_atoms::X11Atoms;
use crate::xlib::Atom;
use ferroui_base::input::{DataFormat, DataFormatKind};
use std::rc::Rc;

/// The prefix of the names of the application formats. The reference has
/// its own abbreviation in this name; the port uses the one of its other
/// backends.
const APP_PREFIX: &str = "application/frn-fmt.";

pub const MIME_TYPE_TEXT_URI_LIST: &str = "text/uri-list";
pub const MIME_TYPE_PNG_FORMAT: &str = "image/png";
pub const MIME_TYPE_JPEG_FORMAT: &str = "image/jpeg";
pub const MIME_TYPE_TEXT_PLAIN: &str = "text/plain";
pub const MIME_TYPE_TEXT_PLAIN_UTF8: &str = "text/plain;charset=utf-8";

/// The encoding of the text of a selection target (the three encodings
/// `TryGetStringEncoding` of the reference chooses from).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StringEncoding {
    /// UTF-16, little endian, without a byte order mark.
    Unicode,
    /// UTF-8 without a byte order mark.
    Utf8,
    /// Seven bit ASCII: every other character is a question mark.
    Ascii,
}

impl StringEncoding {
    /// The bytes of a text in the encoding.
    pub fn get_bytes(self, text: &str) -> Vec<u8> {
        match self {
            StringEncoding::Unicode => text.encode_utf16().flat_map(u16::to_le_bytes).collect(),
            StringEncoding::Utf8 => text.as_bytes().to_vec(),
            // One question mark for every UTF-16 code unit that is not
            // ASCII, as the encoder of the reference's base library gives.
            StringEncoding::Ascii => {
                text.encode_utf16().map(|unit| if unit < 0x80 { unit as u8 } else { b'?' }).collect()
            }
        }
    }

    /// The text of bytes in the encoding. Invalid sequences become
    /// replacement characters (question marks for ASCII).
    pub fn get_string(self, bytes: &[u8]) -> String {
        match self {
            StringEncoding::Unicode => {
                let units = bytes.chunks_exact(2).map(|pair| u16::from_le_bytes([pair[0], pair[1]]));
                let mut text: String =
                    char::decode_utf16(units).map(|c| c.unwrap_or(char::REPLACEMENT_CHARACTER)).collect();
                if bytes.len() % 2 != 0 {
                    text.push(char::REPLACEMENT_CHARACTER);
                }
                text
            }
            StringEncoding::Utf8 => String::from_utf8_lossy(bytes).into_owned(),
            StringEncoding::Ascii => bytes.iter().map(|byte| if *byte < 0x80 { *byte as char } else { '?' }).collect(),
        }
    }
}

/// The data format of a target atom, if the atom names data.
pub fn to_data_format(format_atom: Atom, atoms: &X11Atoms) -> Option<DataFormat> {
    if format_atom == 0 {
        return None;
    }

    if atoms.text_formats().contains(&format_atom) {
        return Some(DataFormat::text().into());
    }

    if format_atom == atoms.MULTIPLE || format_atom == atoms.TARGETS || format_atom == atoms.SAVE_TARGETS {
        return None;
    }

    let atom_name = atoms.get_atom_name(format_atom)?;
    Some(if atom_name == MIME_TYPE_TEXT_URI_LIST {
        DataFormat::file().into()
    } else {
        DataFormat::from_system_name::<Rc<[u8]>>(&atom_name, APP_PREFIX).into()
    })
}

/// The data formats of the target atoms of a selection, and those of the
/// atoms that are text formats.
pub fn to_data_formats(format_atoms: &[Atom], atoms: &X11Atoms) -> (Vec<DataFormat>, Vec<Atom>) {
    if format_atoms.is_empty() {
        return (Vec::new(), Vec::new());
    }

    let mut formats: Vec<DataFormat> = Vec::with_capacity(format_atoms.len());
    let mut text_format_atoms: Option<Vec<Atom>> = None;

    let mut has_image = false;

    for &format_atom in format_atoms {
        let Some(format) = to_data_format(format_atom, atoms) else {
            continue;
        };

        if DataFormat::text() == format {
            let text_format_atoms = text_format_atoms.get_or_insert_with(|| {
                formats.push(format.clone());
                Vec::new()
            });
            text_format_atoms.push(format_atom);
        } else {
            if !has_image && matches!(format.identifier(), MIME_TYPE_JPEG_FORMAT | MIME_TYPE_PNG_FORMAT) {
                has_image = true;
            }

            formats.push(format);
        }
    }

    if has_image {
        formats.push(DataFormat::bitmap().into());
    }

    (formats, text_format_atoms.unwrap_or_default())
}

/// The target atom to ask a selection for to get a data format: zero when
/// there is none.
pub fn to_atom(format: &DataFormat, text_format_atoms: &[Atom], atoms: &X11Atoms, data_formats: &[DataFormat]) -> Atom {
    if DataFormat::text() == *format {
        return get_preferred_string_format_atom(text_format_atoms, atoms);
    }

    if DataFormat::file() == *format {
        return atoms.get_atom(MIME_TYPE_TEXT_URI_LIST);
    }

    if DataFormat::bitmap() == *format {
        let mut png_format: Option<&DataFormat> = None;
        let mut jpeg_format: Option<&DataFormat> = None;
        for image_format in data_formats {
            if image_format.identifier() == MIME_TYPE_PNG_FORMAT {
                png_format = Some(image_format);
            } else if image_format.identifier() == MIME_TYPE_JPEG_FORMAT {
                jpeg_format = Some(image_format);
            }

            if png_format.is_some() && jpeg_format.is_some() {
                break;
            }
        }

        return match png_format.or(jpeg_format) {
            Some(preferred_format) => atoms.get_atom(preferred_format.identifier()),
            None => 0,
        };
    }

    let system_name = format.to_system_name(APP_PREFIX);
    atoms.get_atom(&system_name)
}

/// The target atoms a data format is offered as.
pub fn to_atoms(format: &DataFormat, atoms: &X11Atoms) -> Vec<Atom> {
    if DataFormat::text() == *format {
        return atoms.text_formats().to_vec();
    }

    if DataFormat::file() == *format {
        return vec![atoms.get_atom(MIME_TYPE_TEXT_URI_LIST)];
    }

    if DataFormat::bitmap() == *format {
        return vec![atoms.get_atom(MIME_TYPE_PNG_FORMAT)];
    }

    let system_name = format.to_system_name(APP_PREFIX);
    vec![atoms.get_atom(&system_name)]
}

/// The target atoms a list of data formats is offered as. Formats that
/// only exist in the process have no atom.
pub fn formats_to_atoms(formats: &[DataFormat], atoms: &X11Atoms) -> Vec<Atom> {
    let mut atom_values = Vec::with_capacity(formats.len());

    for format in formats {
        if format.kind() == DataFormatKind::InProcess {
            continue;
        }

        atom_values.extend(to_atoms(format, atoms));
    }

    atom_values
}

fn get_preferred_string_format_atom(text_format_atoms: &[Atom], atoms: &X11Atoms) -> Atom {
    preferred_string_format_atom(text_format_atoms, atoms.text_formats(), atoms.UTF8_STRING)
}

/// The first atom of `preferred_formats` (the text formats by order of
/// preference) that is among `text_format_atoms` (the text formats a
/// selection offers), or `fallback`.
pub fn preferred_string_format_atom(text_format_atoms: &[Atom], preferred_formats: &[Atom], fallback: Atom) -> Atom {
    preferred_formats
        .iter()
        .copied()
        .find(|preferred_format| text_format_atoms.contains(preferred_format))
        .unwrap_or(fallback)
}

/// The encoding of the text of a target atom, if the atom is a text
/// format.
pub fn try_get_string_encoding(format_atom: Atom, atoms: &X11Atoms) -> Option<StringEncoding> {
    if format_atom == atoms.UTF16_STRING {
        return Some(StringEncoding::Unicode);
    }

    if format_atom == atoms.UTF8_STRING
        || format_atom == atoms.get_atom(MIME_TYPE_TEXT_PLAIN)
        || format_atom == atoms.get_atom(MIME_TYPE_TEXT_PLAIN_UTF8)
    {
        return Some(StringEncoding::Utf8);
    }

    if format_atom == atoms.STRING {
        return Some(StringEncoding::Ascii);
    }

    None
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests for this file.
    use super::*;

    /// Atoms without a connection: the interned names get the numbers 101
    /// and up, and no other name can be interned (its atom is zero).
    fn atoms() -> X11Atoms {
        let mut next = 100;
        X11Atoms::with_interned(|_| {
            next += 1;
            next
        })
    }

    #[test]
    fn the_names_of_the_formats_are_those_of_the_protocols() {
        assert_eq!(MIME_TYPE_TEXT_URI_LIST, "text/uri-list");
        assert_eq!(MIME_TYPE_PNG_FORMAT, "image/png");
        assert_eq!(MIME_TYPE_JPEG_FORMAT, "image/jpeg");
        assert_eq!(MIME_TYPE_TEXT_PLAIN, "text/plain");
        assert_eq!(MIME_TYPE_TEXT_PLAIN_UTF8, "text/plain;charset=utf-8");
        assert_eq!(APP_PREFIX, "application/frn-fmt.");
    }

    #[test]
    fn utf16_text_is_little_endian_without_a_mark() {
        let bytes = StringEncoding::Unicode.get_bytes("a\u{e9}\u{1f600}");
        assert_eq!(bytes, [0x61, 0x00, 0xe9, 0x00, 0x3d, 0xd8, 0x00, 0xde]);
        assert_eq!(StringEncoding::Unicode.get_string(&bytes), "a\u{e9}\u{1f600}");
        // An unpaired surrogate and a trailing odd byte are replaced.
        assert_eq!(StringEncoding::Unicode.get_string(&[0x3d, 0xd8, 0x61, 0x00, 0x62]), "\u{fffd}a\u{fffd}");
        assert_eq!(StringEncoding::Unicode.get_string(&[]), "");
    }

    #[test]
    fn utf8_text_is_the_bytes_of_the_string() {
        let bytes = StringEncoding::Utf8.get_bytes("a\u{e9}\u{1f600}");
        assert_eq!(bytes, "a\u{e9}\u{1f600}".as_bytes());
        assert_eq!(StringEncoding::Utf8.get_string(&bytes), "a\u{e9}\u{1f600}");
        assert_eq!(StringEncoding::Utf8.get_string(&[0x61, 0xff, 0x62]), "a\u{fffd}b");
    }

    #[test]
    fn ascii_text_replaces_what_is_not_ascii() {
        // One question mark per UTF-16 code unit: two for a character
        // outside the basic plane.
        assert_eq!(StringEncoding::Ascii.get_bytes("a\u{e9}\u{1f600}b"), b"a???b");
        assert_eq!(StringEncoding::Ascii.get_string(&[0x61, 0xe9, 0x7f]), "a?\u{7f}");
    }

    #[test]
    fn the_encoding_follows_the_text_atom() {
        let atoms = atoms();
        assert_eq!(try_get_string_encoding(atoms.UTF16_STRING, &atoms), Some(StringEncoding::Unicode));
        assert_eq!(try_get_string_encoding(atoms.UTF8_STRING, &atoms), Some(StringEncoding::Utf8));
        assert_eq!(try_get_string_encoding(atoms.STRING, &atoms), Some(StringEncoding::Ascii));
        assert_eq!(try_get_string_encoding(atoms.TARGETS, &atoms), None);
        assert_eq!(try_get_string_encoding(9999, &atoms), None);
    }

    #[test]
    fn the_preferred_text_atom_is_the_first_of_the_preference_that_is_offered() {
        let (utf16, utf8, plain, plain_utf8, string) = (10, 11, 12, 13, 14);
        let preference = [utf16, utf8, plain, plain_utf8, string];
        assert_eq!(preferred_string_format_atom(&[string, utf8], &preference, utf8), utf8);
        assert_eq!(preferred_string_format_atom(&[string, plain, utf16], &preference, utf8), utf16);
        assert_eq!(preferred_string_format_atom(&[string], &preference, utf8), string);
        assert_eq!(preferred_string_format_atom(&[plain_utf8, plain], &preference, utf8), plain);
        // Nothing known is offered: UTF-8 is asked for.
        assert_eq!(preferred_string_format_atom(&[], &preference, utf8), utf8);
        assert_eq!(preferred_string_format_atom(&[99], &preference, utf8), utf8);
    }

    #[test]
    fn the_text_atom_asked_for_is_chosen_among_the_offered_ones() {
        let atoms = atoms();
        let text: DataFormat = DataFormat::text().into();
        assert_eq!(to_atom(&text, &[atoms.STRING, atoms.UTF8_STRING], &atoms, &[]), atoms.UTF8_STRING);
        assert_eq!(to_atom(&text, &[atoms.STRING, atoms.UTF16_STRING], &atoms, &[]), atoms.UTF16_STRING);
        assert_eq!(to_atom(&text, &[atoms.STRING], &atoms, &[]), atoms.STRING);
        assert_eq!(to_atom(&text, &[], &atoms, &[]), atoms.UTF8_STRING);
    }

    #[test]
    fn atoms_that_are_not_data_have_no_format() {
        let atoms = atoms();
        assert!(to_data_format(0, &atoms).is_none());
        assert!(to_data_format(atoms.TARGETS, &atoms).is_none());
        assert!(to_data_format(atoms.MULTIPLE, &atoms).is_none());
        assert!(to_data_format(atoms.SAVE_TARGETS, &atoms).is_none());
        // An atom whose name is not known (there is no server to ask).
        assert!(to_data_format(9999, &atoms).is_none());
    }

    #[test]
    fn text_atoms_are_the_text_format_and_named_atoms_are_platform_formats() {
        let atoms = atoms();
        assert!(DataFormat::text() == to_data_format(atoms.UTF8_STRING, &atoms).unwrap());
        assert!(DataFormat::text() == to_data_format(atoms.UTF16_STRING, &atoms).unwrap());
        assert!(DataFormat::text() == to_data_format(atoms.STRING, &atoms).unwrap());

        let format = to_data_format(atoms.INCR, &atoms).unwrap();
        assert_eq!(format.kind(), DataFormatKind::Platform);
        assert_eq!(format.identifier(), "INCR");
    }

    #[test]
    fn the_text_format_is_listed_once_with_all_its_atoms() {
        let atoms = atoms();
        let (formats, text_format_atoms) =
            to_data_formats(&[atoms.TARGETS, atoms.STRING, atoms.INCR, atoms.UTF8_STRING, 0, 9999], &atoms);
        assert_eq!(formats.len(), 2);
        assert!(DataFormat::text() == formats[0]);
        assert_eq!(formats[1].identifier(), "INCR");
        assert_eq!(text_format_atoms, [atoms.STRING, atoms.UTF8_STRING]);

        let (formats, text_format_atoms) = to_data_formats(&[], &atoms);
        assert!(formats.is_empty());
        assert!(text_format_atoms.is_empty());
    }

    #[test]
    fn the_text_format_is_offered_as_every_text_atom() {
        let atoms = atoms();
        let text: DataFormat = DataFormat::text().into();
        assert_eq!(to_atoms(&text, &atoms), atoms.text_formats());
        assert_eq!(atoms.text_formats()[0], atoms.UTF16_STRING);
        assert_eq!(atoms.text_formats()[1], atoms.UTF8_STRING);
        assert_eq!(atoms.text_formats()[4], atoms.STRING);

        let in_process: DataFormat = DataFormat::create_in_process_format::<String>("only-here").into();
        assert_eq!(formats_to_atoms(&[in_process, text], &atoms), atoms.text_formats());
    }

    #[test]
    fn a_bitmap_is_asked_for_as_the_image_format_that_is_offered() {
        let atoms = atoms();
        let bitmap: DataFormat = DataFormat::bitmap().into();
        // No image format is offered: there is nothing to ask for.
        assert_eq!(to_atom(&bitmap, &[], &atoms, &[DataFormat::text().into()]), 0);
    }
}

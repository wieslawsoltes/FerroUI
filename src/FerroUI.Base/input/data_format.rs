use super::{DataFormatKind, DataFormatOf};
use crate::media::imaging::Bitmap;
use crate::platform::storage::IStorageItem;
use std::any::TypeId;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

/// Represents a format usable with the clipboard and drag-and-drop.
///
/// A format is a value: two formats are equal when their kind and their
/// identifier are equal. The typed counterpart, which also carries the type
/// of the data, is [`DataFormatOf`]; it dereferences to `DataFormat`.
#[derive(Clone)]
pub struct DataFormat {
    kind: DataFormatKind,
    identifier: Rc<str>,
    /// The type of the values of the format, when the format was created
    /// as a typed format (the type argument of the managed
    /// `DataFormat<T>`). Not part of the identity of the format.
    data_type: Option<TypeId>,
}

impl PartialEq for DataFormat {
    fn eq(&self, other: &DataFormat) -> bool {
        self.kind == other.kind && self.identifier == other.identifier
    }
}

impl Eq for DataFormat {}

impl Hash for DataFormat {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.kind.hash(state);
        self.identifier.hash(state);
    }
}

thread_local! {
    static TEXT: DataFormatOf<String> = DataFormat::create_universal_format("Text");
    static BITMAP: DataFormatOf<Rc<Bitmap>> = DataFormat::create_universal_format("Bitmap");
    static FILE: DataFormatOf<Rc<dyn IStorageItem>> = DataFormat::create_universal_format("File");
}

impl DataFormat {
    pub(super) fn new(kind: DataFormatKind, identifier: &str, data_type: Option<TypeId>) -> Self {
        Self { kind, identifier: Rc::from(identifier), data_type }
    }

    /// The type of the values of the format (`String` for a format of
    /// strings, `Rc<[u8]>` for a format of bytes), when the format was
    /// created with one: the counterpart of asking whether a format of the
    /// managed original is a `DataFormat<string>` or a `DataFormat<byte[]>`,
    /// which a platform backend does to decide how to read data the system
    /// does not describe. Two formats that differ in it alone are equal.
    #[inline]
    pub fn data_type(&self) -> Option<TypeId> {
        self.data_type
    }

    /// The kind of the data format.
    #[inline]
    pub fn kind(&self) -> DataFormatKind {
        self.kind
    }

    /// The identifier of the data format.
    #[inline]
    pub fn identifier(&self) -> &str {
        &self.identifier
    }

    /// A data format representing plain text. Its data type is [`String`].
    pub fn text() -> DataFormatOf<String> {
        TEXT.with(Clone::clone)
    }

    /// A data format representing a bitmap. Its data type is a shared
    /// [`Bitmap`].
    pub fn bitmap() -> DataFormatOf<Rc<Bitmap>> {
        BITMAP.with(Clone::clone)
    }

    /// A data format representing a single file. Its data type is a shared
    /// [`IStorageItem`].
    pub fn file() -> DataFormatOf<Rc<dyn IStorageItem>> {
        FILE.with(Clone::clone)
    }

    /// Creates a name for this format, usable by the underlying platform.
    ///
    /// `application_prefix` is the system prefix used to recognize the name
    /// as an application format.
    ///
    /// # Panics
    /// This method can only be called if [`kind`](Self::kind) is
    /// [`DataFormatKind::Application`] or [`DataFormatKind::Platform`].
    pub fn to_system_name(&self, application_prefix: &str) -> String {
        match self.kind {
            DataFormatKind::Application => {
                let mut name = String::with_capacity(application_prefix.len() + self.identifier.len());
                name.push_str(application_prefix);
                name.push_str(&self.identifier);
                name
            }
            DataFormatKind::Platform => self.identifier.to_string(),
            _ => panic!("Cannot get system name for {} format {}", self.kind, self.identifier),
        }
    }

    fn create_universal_format<T: 'static>(identifier: &str) -> DataFormatOf<T> {
        DataFormatOf::new(DataFormatKind::Universal, identifier)
    }

    /// Creates a new format specific to the application that returns an
    /// array of bytes.
    ///
    /// To avoid conflicts with system identifiers, `identifier` isn't
    /// passed to the underlying platform directly. However, two different
    /// applications using the same identifier with
    /// `create_bytes_application_format` or
    /// [`create_string_application_format`](Self::create_string_application_format)
    /// are able to share data using this format.
    ///
    /// # Panics
    /// Only ASCII letters (A-Z, a-z), digits (0-9), the dot (.) and the
    /// hyphen (-) are accepted in the identifier, which must not be empty.
    pub fn create_bytes_application_format(identifier: &str) -> DataFormatOf<Rc<[u8]>> {
        Self::create_application_format(identifier)
    }

    /// Creates a new format specific to the application that returns a
    /// [`String`].
    ///
    /// See [`create_bytes_application_format`](Self::create_bytes_application_format)
    /// for the identifier.
    pub fn create_string_application_format(identifier: &str) -> DataFormatOf<String> {
        Self::create_application_format(identifier)
    }

    /// Creates a new format that stays within the current process and is
    /// never serialized to a platform clipboard or drag-and-drop operation.
    ///
    /// `T` is the data type. `identifier` is only used for equality
    /// comparisons within the process and is never passed to the underlying
    /// platform.
    ///
    /// # Panics
    /// Panics if the identifier is empty.
    pub fn create_in_process_format<T: 'static>(identifier: &str) -> DataFormatOf<T> {
        if identifier.is_empty() {
            panic!("The value cannot be an empty string. (Parameter 'identifier')");
        }

        DataFormatOf::new(DataFormatKind::InProcess, identifier)
    }

    fn create_application_format<T: 'static>(identifier: &str) -> DataFormatOf<T> {
        if !Self::is_valid_application_format_identifier(identifier) {
            panic!("Invalid application identifier (Parameter 'identifier')");
        }

        DataFormatOf::new(DataFormatKind::Application, identifier)
    }

    /// Creates a new format for the current platform that returns an array
    /// of bytes.
    ///
    /// `identifier` is not validated and is passed as is to the underlying
    /// platform. Most systems use mime types, but macOS requires Uniform
    /// Type Identifiers (UTI).
    ///
    /// # Panics
    /// Panics if the identifier is empty.
    pub fn create_bytes_platform_format(identifier: &str) -> DataFormatOf<Rc<[u8]>> {
        Self::create_platform_format(identifier)
    }

    /// Creates a new format for the current platform that returns a
    /// [`String`].
    ///
    /// See [`create_bytes_platform_format`](Self::create_bytes_platform_format)
    /// for the identifier.
    pub fn create_string_platform_format(identifier: &str) -> DataFormatOf<String> {
        Self::create_platform_format(identifier)
    }

    fn create_platform_format<T: 'static>(identifier: &str) -> DataFormatOf<T> {
        if identifier.is_empty() {
            panic!("The value cannot be an empty string. (Parameter 'identifier')");
        }

        DataFormatOf::new(DataFormatKind::Platform, identifier)
    }

    /// Creates a format from a name coming from the underlying platform.
    ///
    /// `application_prefix` is the system prefix used to recognize the name
    /// as an application format. This is an implementation detail of the
    /// platform backends.
    pub fn from_system_name<T: 'static>(system_name: &str, application_prefix: &str) -> DataFormatOf<T> {
        let prefix_length = application_prefix.len();
        let has_prefix = system_name
            .as_bytes()
            .get(..prefix_length)
            .is_some_and(|start| start.eq_ignore_ascii_case(application_prefix.as_bytes()))
            && system_name.is_char_boundary(prefix_length);

        if has_prefix {
            let identifier = &system_name[prefix_length..];
            if Self::is_valid_application_format_identifier(identifier) {
                return DataFormatOf::new(DataFormatKind::Application, identifier);
            }
        }

        DataFormatOf::new(DataFormatKind::Platform, system_name)
    }

    fn is_valid_application_format_identifier(identifier: &str) -> bool {
        !identifier.is_empty() && identifier.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
    }
}

impl fmt::Display for DataFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.kind, self.identifier)
    }
}

impl fmt::Debug for DataFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

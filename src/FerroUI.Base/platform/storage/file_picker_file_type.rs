use super::file_io::path;
use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

/// Represents a name mapped to the associated file types (extensions).
///
/// A file type is a reference object: it is shared as `Rc<FilePickerFileType>`
/// and compared by identity.
pub struct FilePickerFileType {
    name: String,
    patterns: RefCell<Option<Rc<[String]>>>,
    mime_types: RefCell<Option<Rc<[String]>>>,
    apple_uniform_type_identifiers: RefCell<Option<Rc<[String]>>>,
}

fn to_list(values: &[&str]) -> Option<Rc<[String]>> {
    Some(values.iter().map(|value| (*value).to_owned()).collect())
}

impl FilePickerFileType {
    /// Creates a file type with the given name; `None` gives an empty name.
    pub fn new(name: Option<&str>) -> Rc<Self> {
        Rc::new(Self {
            name: name.unwrap_or_default().to_owned(),
            patterns: RefCell::new(None),
            mime_types: RefCell::new(None),
            apple_uniform_type_identifiers: RefCell::new(None),
        })
    }

    /// File type name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// List of extensions in GLOB format. I.e. "*.png" or "*.*".
    ///
    /// Used on Windows, Linux and Browser platforms.
    pub fn patterns(&self) -> Option<Rc<[String]>> {
        self.patterns.borrow().clone()
    }

    pub fn set_patterns(&self, value: Option<Vec<String>>) {
        *self.patterns.borrow_mut() = value.map(Rc::from);
    }

    /// Sets [`patterns`](Self::patterns) and returns the file type.
    pub fn with_patterns(self: Rc<Self>, value: &[&str]) -> Rc<Self> {
        *self.patterns.borrow_mut() = to_list(value);
        self
    }

    /// List of extensions in MIME format.
    ///
    /// Used on Android, Linux and Browser platforms.
    pub fn mime_types(&self) -> Option<Rc<[String]>> {
        self.mime_types.borrow().clone()
    }

    pub fn set_mime_types(&self, value: Option<Vec<String>>) {
        *self.mime_types.borrow_mut() = value.map(Rc::from);
    }

    /// Sets [`mime_types`](Self::mime_types) and returns the file type.
    pub fn with_mime_types(self: Rc<Self>, value: &[&str]) -> Rc<Self> {
        *self.mime_types.borrow_mut() = to_list(value);
        self
    }

    /// List of extensions in Apple uniform format.
    ///
    /// Used only on Apple devices.
    ///
    /// See <https://developer.apple.com/documentation/uniformtypeidentifiers/system_declared_uniform_type_identifiers>.
    pub fn apple_uniform_type_identifiers(&self) -> Option<Rc<[String]>> {
        self.apple_uniform_type_identifiers.borrow().clone()
    }

    pub fn set_apple_uniform_type_identifiers(&self, value: Option<Vec<String>>) {
        *self.apple_uniform_type_identifiers.borrow_mut() = value.map(Rc::from);
    }

    /// Sets [`apple_uniform_type_identifiers`](Self::apple_uniform_type_identifiers)
    /// and returns the file type.
    pub fn with_apple_uniform_type_identifiers(self: Rc<Self>, value: &[&str]) -> Rc<Self> {
        *self.apple_uniform_type_identifiers.borrow_mut() = to_list(value);
        self
    }

    /// The simple extension names (without the dot) of the patterns.
    ///
    /// This is an implementation detail of the platform backends.
    pub fn try_get_extensions(&self) -> Option<Vec<String>> {
        // Converts random glob pattern to a simple extension name.
        // The extension of the pattern should be sufficient here,
        // Only exception is "*.*proj" patterns that should be filtered as well.
        let patterns = self.patterns()?;
        Some(
            patterns
                .iter()
                .map(|pattern| path::get_extension(pattern))
                .filter(|e| !e.is_empty() && !e.contains('*') && e.starts_with('.'))
                .map(|e| e.trim_start_matches('.').to_owned())
                .collect(),
        )
    }
}

impl PartialEq for FilePickerFileType {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl fmt::Display for FilePickerFileType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)
    }
}

impl fmt::Debug for FilePickerFileType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FilePickerFileType").field("name", &self.name).finish_non_exhaustive()
    }
}

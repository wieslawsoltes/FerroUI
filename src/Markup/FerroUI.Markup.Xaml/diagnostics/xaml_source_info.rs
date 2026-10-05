//! Port of `Diagnostics/XamlSourceInfo.cs`.

use crate::object_casts::{as_object, resource_key_of};
use crate::{FromXamlObject, XamlLoadException};
use ferroui_base::controls::{IResourceDictionary, ResourceDictionary, ResourceKey};
use ferroui_base::utilities::{Uri, UriExtensions, UriKind};
use ferroui_base::{
    ferro_markup_type, ferro_properties, ferro_static_type, AnyValue, AttachedProperty, BoxedValue, FerroObject,
    FerroProperty, WeakRef,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::rc::{Rc, Weak};

/// Represents source location information for an element within a XAML
/// file.
///
/// This type is instantiated through the XAML compiler.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct XamlSourceInfo {
    source_uri: Option<Uri>,
    line_number: i32,
    line_position: i32,
}

ferro_static_type!(XamlSourceInfo);

ferro_properties! {
    impl XamlSourceInfo {
        fn xaml_source_info_property() -> AttachedProperty<Option<XamlSourceInfo>> {
            FerroProperty::register_attached::<XamlSourceInfo, FerroObject, _>("XamlSourceInfo", None)
        }
    }
}

enum DictionaryKey {
    Object(WeakRef<FerroObject>),
    Other(Weak<dyn IResourceDictionary>),
}

impl DictionaryKey {
    fn of(dictionary: &Rc<dyn IResourceDictionary>) -> (usize, DictionaryKey) {
        match dictionary.as_object() {
            Some(object) => (object as *const FerroObject as usize, DictionaryKey::Object(object.to_weak())),
            None => (Rc::as_ptr(dictionary) as *const () as usize, DictionaryKey::Other(Rc::downgrade(dictionary))),
        }
    }

    fn is_alive(&self) -> bool {
        match self {
            DictionaryKey::Object(object) => object.upgrade().is_some(),
            DictionaryKey::Other(dictionary) => dictionary.strong_count() > 0,
        }
    }
}

type KeyedSourceInfo = (DictionaryKey, HashMap<ResourceKey, XamlSourceInfo>);

thread_local! {
    /// The source info of objects that are not part of the class model,
    /// keyed by identity and held weakly.
    static SOURCE_INFO: RefCell<HashMap<usize, (Weak<dyn AnyValue>, Option<XamlSourceInfo>)>> =
        RefCell::new(HashMap::new());
    /// The source info of the entries of resource dictionaries.
    static KEYED_SOURCE_INFO: RefCell<HashMap<usize, KeyedSourceInfo>> = RefCell::new(HashMap::new());
}

fn identity(obj: &BoxedValue) -> usize {
    Rc::as_ptr(obj) as *const () as usize
}

impl XamlSourceInfo {
    /// Initializes a new instance with the specified line, column, and file
    /// path.
    pub fn new(line: i32, column: i32, file_path: Option<&str>) -> Self {
        Self { line_number: line, line_position: column, source_uri: file_path.and_then(file_uri) }
    }

    /// The full path of the source file containing the XAML element.
    pub fn source_uri(&self) -> Option<&Uri> {
        self.source_uri.as_ref()
    }

    /// The 1-based line number in the source file where the element is
    /// defined.
    pub fn line_number(&self) -> i32 {
        self.line_number
    }

    /// The 1-based column position in the source file where the element is
    /// defined.
    pub fn line_position(&self) -> i32 {
        self.line_position
    }

    /// Associates XAML source information with the specified object for
    /// debugging or diagnostic purposes.
    ///
    /// This method is typically used to attach source information to objects
    /// at run time, which can be useful for debugging tools or diagnostics.
    /// The information does not keep the object alive.
    pub fn set_xaml_source_info(obj: &BoxedValue, info: Option<XamlSourceInfo>) {
        if let Some(object) = as_object(obj) {
            object.set_value(Self::xaml_source_info_property(), info);
        } else {
            SOURCE_INFO.with(|table| {
                let mut table = table.borrow_mut();
                table.retain(|_, (object, _)| object.strong_count() > 0);
                table.insert(identity(obj), (Rc::downgrade(obj), info));
            });
        }
    }

    /// Associates XAML source information with a specified key in the given
    /// resource dictionary. Passing `None` removes the source information of
    /// the key.
    pub fn set_xaml_source_info_for_key(
        dictionary: &Rc<dyn IResourceDictionary>,
        key: impl Into<ResourceKey>,
        info: Option<XamlSourceInfo>,
    ) {
        let (id, weak) = DictionaryKey::of(dictionary);
        KEYED_SOURCE_INFO.with(|table| {
            let mut table = table.borrow_mut();
            table.retain(|_, (dictionary, _)| dictionary.is_alive());
            let (_, entries) = table.entry(id).or_insert_with(|| (weak, HashMap::new()));
            match info {
                None => {
                    entries.remove(&key.into());
                }
                Some(info) => {
                    entries.insert(key.into(), info);
                }
            }
        });
    }

    /// Retrieves the XAML source information associated with the specified
    /// object, if available.
    pub fn get_xaml_source_info(obj: &BoxedValue) -> Option<XamlSourceInfo> {
        if let Some(object) = as_object(obj) {
            object.get_value(Self::xaml_source_info_property())
        } else {
            SOURCE_INFO.with(|table| {
                let table = table.borrow();
                let (object, info) = table.get(&identity(obj))?;
                // The address of a dead object may have been reused.
                if object.strong_count() == 0 {
                    return None;
                }
                info.clone()
            })
        }
    }

    /// Retrieves the XAML source information associated with the specified
    /// key in the given resource dictionary, if available.
    pub fn get_xaml_source_info_for_key(
        dictionary: &Rc<dyn IResourceDictionary>,
        key: impl Into<ResourceKey>,
    ) -> Option<XamlSourceInfo> {
        let (id, _) = DictionaryKey::of(dictionary);
        KEYED_SOURCE_INFO.with(|table| {
            let table = table.borrow();
            let (dictionary, entries) = table.get(&id)?;
            if !dictionary.is_alive() {
                return None;
            }
            entries.get(&key.into()).cloned()
        })
    }
}

/// `new UriBuilder("file", "") { Path = filePath }.Uri`.
fn file_uri(file_path: &str) -> Option<Uri> {
    let mut path = file_path.replace('\\', "/");
    if !path.starts_with('/') {
        path.insert(0, '/');
    }
    let mut escaped = String::with_capacity(path.len() + 8);
    escaped.push_str("file://");
    for c in path.chars() {
        match c {
            ' ' => escaped.push_str("%20"),
            '%' => escaped.push_str("%25"),
            '#' => escaped.push_str("%23"),
            '?' => escaped.push_str("%3F"),
            c => escaped.push(c),
        }
    }
    Uri::try_create(&escaped, UriKind::Absolute)
}

/// Returns a string that represents the current object:
/// `path:line,position`.
impl fmt::Display for XamlSourceInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let file_path = match &self.source_uri {
            Some(uri) => UriExtensions::get_unescape_absolute_path(uri),
            None => "(unknown)".to_string(),
        };
        write!(f, "{}:{},{}", file_path, self.line_number, self.line_position)
    }
}

impl FromXamlObject for Rc<dyn IResourceDictionary> {
    fn from_xaml_object(value: &BoxedValue) -> Option<Self> {
        as_object(value)?.cast::<ResourceDictionary>().map(Into::into)
    }
}

fn object_argument(obj: Option<BoxedValue>, name: &str) -> Result<BoxedValue, XamlLoadException> {
    obj.ok_or_else(|| XamlLoadException::with_message(format!("Value cannot be null. (Parameter '{name}')")))
}

ferro_markup_type!(class XamlSourceInfo {
    handles: [XamlSourceInfo, Option<XamlSourceInfo>],
    constructors: [
        (i32, i32, Option<String>) => |line: i32, column: i32, file_path: Option<String>| {
            XamlSourceInfo::new(line, column, file_path.as_deref())
        },
    ],
    properties: [
        SourceUri: Option<Uri> { get: |info: &XamlSourceInfo| info.source_uri().cloned() },
        LineNumber: i32 { get: XamlSourceInfo::line_number },
        LinePosition: i32 { get: XamlSourceInfo::line_position },
    ],
    methods: [
        static try fn SetXamlSourceInfo(Option<BoxedValue>, Option<XamlSourceInfo>) =>
            |obj: Option<BoxedValue>, info: Option<XamlSourceInfo>| -> Result<(), XamlLoadException> {
                XamlSourceInfo::set_xaml_source_info(&object_argument(obj, "obj")?, info);
                Ok(())
            },
        static try fn SetXamlSourceInfo(Rc<dyn IResourceDictionary>, Option<BoxedValue>, Option<XamlSourceInfo>) =>
            |dictionary: Rc<dyn IResourceDictionary>, key: Option<BoxedValue>, info: Option<XamlSourceInfo>|
             -> Result<(), XamlLoadException> {
                let key = resource_key_of(&object_argument(key, "key")?);
                XamlSourceInfo::set_xaml_source_info_for_key(&dictionary, key, info);
                Ok(())
            },
        static try fn GetXamlSourceInfo(Option<BoxedValue>) -> Option<XamlSourceInfo> =>
            |obj: Option<BoxedValue>| -> Result<Option<XamlSourceInfo>, XamlLoadException> {
                Ok(XamlSourceInfo::get_xaml_source_info(&object_argument(obj, "obj")?))
            },
        static try fn GetXamlSourceInfo(Rc<dyn IResourceDictionary>, Option<BoxedValue>) -> Option<XamlSourceInfo> =>
            |dictionary: Rc<dyn IResourceDictionary>, key: Option<BoxedValue>|
             -> Result<Option<XamlSourceInfo>, XamlLoadException> {
                let key = resource_key_of(&object_argument(key, "key")?);
                Ok(XamlSourceInfo::get_xaml_source_info_for_key(&dictionary, key))
            },
    ],
});

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_base::metadata::{into_markup_value, MarkupTyped};
    use ferroui_controls::Border;

    #[test]
    fn constructor_builds_a_file_uri() {
        let info = XamlSourceInfo::new(3, 7, Some("/src/My View.xaml"));
        assert_eq!(info.line_number(), 3);
        assert_eq!(info.line_position(), 7);
        assert_eq!(info.source_uri().unwrap().absolute_uri(), "file:///src/My%20View.xaml");
        assert_eq!(info.to_string(), "/src/My View.xaml:3,7");

        let unknown = XamlSourceInfo::new(1, 2, None);
        assert!(unknown.source_uri().is_none());
        assert_eq!(unknown.to_string(), "(unknown):1,2");
    }

    #[test]
    fn records_compare_by_value() {
        assert_eq!(XamlSourceInfo::new(1, 2, Some("/a")), XamlSourceInfo::new(1, 2, Some("/a")));
        assert_ne!(XamlSourceInfo::new(1, 2, Some("/a")), XamlSourceInfo::new(1, 3, Some("/a")));
    }

    #[test]
    fn source_info_of_a_class_instance_is_an_attached_property() {
        let border: BoxedValue = Rc::new(Border::new());
        assert_eq!(XamlSourceInfo::get_xaml_source_info(&border), None);

        let info = XamlSourceInfo::new(10, 4, Some("/a.xaml"));
        XamlSourceInfo::set_xaml_source_info(&border, Some(info.clone()));
        assert_eq!(XamlSourceInfo::get_xaml_source_info(&border), Some(info));

        // Any handle of the same object reads it.
        let other: BoxedValue = Rc::new(as_object(&border).unwrap());
        assert!(XamlSourceInfo::get_xaml_source_info(&other).is_some());

        XamlSourceInfo::set_xaml_source_info(&border, None);
        assert_eq!(XamlSourceInfo::get_xaml_source_info(&border), None);
    }

    #[test]
    fn source_info_of_other_objects_is_kept_by_identity() {
        let a: BoxedValue = Rc::new("text".to_string());
        let b: BoxedValue = Rc::new("text".to_string());
        let info = XamlSourceInfo::new(1, 1, None);

        XamlSourceInfo::set_xaml_source_info(&a, Some(info.clone()));
        assert_eq!(XamlSourceInfo::get_xaml_source_info(&a), Some(info));
        assert_eq!(XamlSourceInfo::get_xaml_source_info(&b), None);
    }

    #[test]
    fn source_info_of_dictionary_entries_is_keyed() {
        let dictionary = ResourceDictionary::new();
        let handle: Rc<dyn IResourceDictionary> = dictionary.clone().into();
        let info = XamlSourceInfo::new(5, 9, Some("/res.xaml"));

        assert_eq!(XamlSourceInfo::get_xaml_source_info_for_key(&handle, "Brush"), None);
        XamlSourceInfo::set_xaml_source_info_for_key(&handle, "Brush", Some(info.clone()));

        // Another handle of the same dictionary sees the entry.
        let again: Rc<dyn IResourceDictionary> = dictionary.clone().into();
        assert_eq!(XamlSourceInfo::get_xaml_source_info_for_key(&again, "Brush"), Some(info));
        assert_eq!(XamlSourceInfo::get_xaml_source_info_for_key(&again, "Other"), None);

        XamlSourceInfo::set_xaml_source_info_for_key(&handle, "Brush", None);
        assert_eq!(XamlSourceInfo::get_xaml_source_info_for_key(&handle, "Brush"), None);
    }

    #[test]
    fn metadata_constructs_and_attaches() {
        crate::register_types();
        let markup = <XamlSourceInfo as MarkupTyped>::MARKUP;
        assert_eq!(markup.name, "XamlSourceInfo");
        let info = (markup.constructors[0].invoke)(&[
            into_markup_value(2i32),
            into_markup_value(3i32),
            into_markup_value("/x.xaml".to_string()),
        ])
        .unwrap();
        let line = markup.find_property("LineNumber").unwrap();
        assert_eq!((line.get.unwrap())(&[info.clone()]).unwrap().unwrap().downcast_ref::<i32>(), Some(&2));

        let border: BoxedValue = Rc::new(Border::new());
        let set = markup.find_methods("SetXamlSourceInfo").find(|m| m.parameters.len() == 2).unwrap();
        (set.invoke)(&[Some(border.clone()), info]).unwrap();
        assert_eq!(XamlSourceInfo::get_xaml_source_info(&border).unwrap().line_position(), 3);

        let dictionary: BoxedValue = Rc::new(ResourceDictionary::new());
        let set = markup.find_methods("SetXamlSourceInfo").find(|m| m.parameters.len() == 3).unwrap();
        (set.invoke)(&[
            Some(dictionary.clone()),
            into_markup_value("Key".to_string()),
            into_markup_value(XamlSourceInfo::new(8, 8, None)),
        ])
        .unwrap();
        let get = markup.find_methods("GetXamlSourceInfo").find(|m| m.parameters.len() == 2).unwrap();
        let found = (get.invoke)(&[Some(dictionary), into_markup_value("Key".to_string())]).unwrap().unwrap();
        assert_eq!(found.downcast_ref::<XamlSourceInfo>().unwrap().line_number(), 8);
    }
}

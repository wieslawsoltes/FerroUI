use super::{FerroDialogsInternalViewModelBase, ManagedFileChooserItemType, ManagedFileChooserNavigationItem};
use ferroui_base::data::model::{Event, INotifyPropertyChanged};
use ferroui_base::ferro_markup_type;
use ferroui_base::utilities::DateTime;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::SystemTime;

/// An entry of the managed file chooser: a file, a folder or a volume.
pub struct ManagedFileChooserItemViewModel {
    base: FerroDialogsInternalViewModelBase,
    display_name: RefCell<Option<String>>,
    path: RefCell<Option<String>>,
    modified: Cell<Option<SystemTime>>,
    type_: RefCell<Option<String>>,
    size: Cell<i64>,
    item_type: Cell<ManagedFileChooserItemType>,
}

impl PartialEq for ManagedFileChooserItemViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl INotifyPropertyChanged for ManagedFileChooserItemViewModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}

impl ManagedFileChooserItemViewModel {
    pub fn new() -> Rc<Self> {
        Rc::new(Self {
            base: FerroDialogsInternalViewModelBase::new(),
            display_name: RefCell::new(None),
            path: RefCell::new(None),
            modified: Cell::new(None),
            type_: RefCell::new(None),
            size: Cell::new(0),
            item_type: Cell::new(ManagedFileChooserItemType::File),
        })
    }

    pub fn from_navigation_item(item: &ManagedFileChooserNavigationItem) -> Rc<Self> {
        let this = Self::new();
        this.set_item_type(item.item_type);
        this.set_path(item.path.clone());
        this.set_display_name(item.display_name.clone());
        this
    }

    pub fn display_name(&self) -> Option<String> {
        self.display_name.borrow().clone()
    }

    pub fn set_display_name(&self, value: Option<String>) {
        self.base.raise_and_set_if_changed(&self.display_name, value, "DisplayName");
    }

    pub fn path(&self) -> Option<String> {
        self.path.borrow().clone()
    }

    pub fn set_path(&self, value: Option<String>) {
        self.base.raise_and_set_if_changed(&self.path, value, "Path");
    }

    /// The time of the last write of the entry, `None` when it is not known
    /// (the `Modified` of the reference, a local `DateTime`).
    pub fn modified(&self) -> Option<SystemTime> {
        self.modified.get()
    }

    pub fn set_modified(&self, value: Option<SystemTime>) {
        self.base.raise_and_set_if_changed_cell(&self.modified, value, "Modified");
    }

    /// The markup property `Modified`: the time of the last write in local
    /// time, `DateTime::default()` when it is not known, as in the reference.
    fn modified_date_time(&self) -> DateTime {
        let date = self.modified().and_then(|time| DateTime::try_from(time).ok());
        date.map(DateTime::to_local_time).unwrap_or_default()
    }

    fn set_modified_date_time(&self, value: DateTime) {
        self.set_modified(SystemTime::try_from(value).ok());
    }

    /// The `Type` property: the extension of a file, or the kind of a
    /// folder.
    pub fn type_(&self) -> Option<String> {
        self.type_.borrow().clone()
    }

    pub fn set_type(&self, value: Option<String>) {
        self.base.raise_and_set_if_changed(&self.type_, value, "Type");
    }

    pub fn size(&self) -> i64 {
        self.size.get()
    }

    pub fn set_size(&self, value: i64) {
        self.base.raise_and_set_if_changed_cell(&self.size, value, "Size");
    }

    pub fn item_type(&self) -> ManagedFileChooserItemType {
        self.item_type.get()
    }

    pub fn set_item_type(&self, value: ManagedFileChooserItemType) {
        self.base.raise_and_set_if_changed_cell(&self.item_type, value, "ItemType");
    }

    /// The key of the icon of the entry in the icon resources of the theme.
    pub fn icon_key(&self) -> String {
        match self.item_type() {
            ManagedFileChooserItemType::Folder => "Icon_Folder".to_string(),
            ManagedFileChooserItemType::Volume => "Icon_Volume".to_string(),
            _ => "Icon_File".to_string(),
        }
    }
}

ferro_markup_type!(class ManagedFileChooserItemViewModel {
    this: Rc<ManagedFileChooserItemViewModel>,
    handles: [
        ManagedFileChooserItemViewModel,
        Rc<ManagedFileChooserItemViewModel>,
        Option<Rc<ManagedFileChooserItemViewModel>>
    ],
    constructors: [() => ManagedFileChooserItemViewModel::new],
    properties: [
        DisplayName: Option<String> {
            get: ManagedFileChooserItemViewModel::display_name,
            set: ManagedFileChooserItemViewModel::set_display_name
        },
        Path: Option<String> { get: ManagedFileChooserItemViewModel::path, set: ManagedFileChooserItemViewModel::set_path },
        Modified: DateTime {
            get: ManagedFileChooserItemViewModel::modified_date_time,
            set: ManagedFileChooserItemViewModel::set_modified_date_time
        },
        Type: Option<String> { get: ManagedFileChooserItemViewModel::type_, set: ManagedFileChooserItemViewModel::set_type },
        Size: i64 { get: ManagedFileChooserItemViewModel::size, set: ManagedFileChooserItemViewModel::set_size },
        ItemType: ManagedFileChooserItemType {
            get: ManagedFileChooserItemViewModel::item_type,
            set: ManagedFileChooserItemViewModel::set_item_type
        },
        IconKey: String { get: ManagedFileChooserItemViewModel::icon_key },
    ],
    notify_property_changed: ManagedFileChooserItemViewModel,
});

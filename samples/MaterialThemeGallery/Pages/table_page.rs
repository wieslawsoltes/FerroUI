//! The class of the document `Pages/TablePage.xaml`: TableView with its column headers, rows and
//! cells. The rows are set here, the same rows as the native page (`TablePage.winui.xml`); the
//! second row of the first table is selected, and the disabled table has the first three.

use crate::markup::xaml_class;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_markup_type, instantiate, BoxedValue, FerroObjectImpl, Ref,
    StyledElementImpl, VisualImpl,
};
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{ContentControlImpl, Control, ControlImpl, ItemsSource, TableView, UserControl};
use std::rc::Rc;

/// The rows of the tables: name, kind, size.
const ROWS: [(&str, &str, &str); 5] = [
    ("Report.docx", "Document", "24 KB"),
    ("Budget.xlsx", "Spreadsheet", "48 KB"),
    ("Photo.jpg", "Picture", "2.1 MB"),
    ("Notes.txt", "Text", "2 KB"),
    ("Archive.zip", "Archive", "12 MB"),
];

/// A row of the tables, which the columns bind to.
pub struct TableRow {
    name: String,
    kind: String,
    size: String,
}

impl PartialEq for TableRow {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl TableRow {
    pub fn new(name: &str, kind: &str, size: &str) -> Rc<TableRow> {
        Rc::new(TableRow { name: name.to_string(), kind: kind.to_string(), size: size.to_string() })
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn kind(&self) -> String {
        self.kind.clone()
    }

    pub fn size(&self) -> String {
        self.size.clone()
    }
}

ferro_markup_type!(class TableRow {
    this: Rc<TableRow>,
    handles: [TableRow, Rc<TableRow>, Option<Rc<TableRow>>],
    constructors: [
        (String, String, String) => |name: String, kind: String, size: String| TableRow::new(&name, &kind, &size),
    ],
    properties: [
        Name: String { get: |this: &Rc<TableRow>| this.name() },
        Kind: String { get: |this: &Rc<TableRow>| this.kind() },
        Size: String { get: |this: &Rc<TableRow>| this.size() },
    ],
});

/// The first `count` rows as the items of a table.
fn rows(count: usize) -> ItemsSource {
    ItemsSource::from_items(
        ROWS.iter().take(count).map(|&(name, kind, size)| Some(Rc::new(TableRow::new(name, kind, size)) as BoxedValue)),
    )
}

#[repr(C)]
pub struct TablePage {
    base: UserControl,
}

ferro_class!(TablePage: UserControl);
ferro_impl_classes!(
    TablePage: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl
);
ferro_class_info!(TablePage { new: TablePage::new });
xaml_class!(TablePage, "/Pages/TablePage.xaml");

impl TablePage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();

        let table = this.get_control::<TableView>("Table");
        table.set_items_source(Some(rows(ROWS.len())));
        table.set_selected_index(1);
        this.get_control::<TableView>("DisabledTable").set_items_source(Some(rows(3)));
        this
    }
}

/// The page as a control (the entry of the page table).
pub fn create() -> Ref<Control> {
    TablePage::new().upcast()
}

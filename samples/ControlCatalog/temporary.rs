//! TEMPORARY: what lets the application start while parts of the real
//! shell do not load. Nothing here is a port; every item names what it
//! waits for and goes away with it.
//!
//! - [`load_main_window_subset`]: `MainWindow.xaml` without the elements
//!   that do not load yet ([`MAIN_WINDOW_REMOVALS`]).

use ferroui_base::BoxedValue;
use ferroui_markup_xaml::XamlLoadException;

/// An element of a document that is removed from the subset that loads:
/// what it waits for, and the text from `start` up to and including `end`
/// (each must occur exactly as written; whole lines are removed).
pub struct DocumentRemoval {
    /// The gap or missing type the element waits for.
    pub waits_for: &'static str,
    /// The first text of the element.
    pub start: &'static str,
    /// The last text of the element.
    pub end: &'static str,
}

/// `xaml` without the elements of `removals`.
///
/// # Panics
/// Panics when an element is not found: the table no longer matches the
/// document.
pub fn remove_elements(xaml: &str, removals: &[DocumentRemoval]) -> String {
    let mut text = xaml.to_string();
    for removal in removals {
        let start = text.find(removal.start).unwrap_or_else(|| panic!("the document has no {}", removal.start));
        let end = text[start..]
            .find(removal.end)
            .map(|offset| start + offset + removal.end.len())
            .unwrap_or_else(|| panic!("the element {} does not end with {}", removal.start, removal.end));
        // Whole lines: from the start of the first line to the end of the last one.
        let line_start = text[..start].rfind('\n').map_or(0, |i| i + 1);
        let line_end = text[end..].find('\n').map_or(text.len(), |i| end + i + 1);
        text.replace_range(line_start..line_end, "");
    }
    text
}

/// The elements of `MainWindow.xaml` that do not load yet.
pub const MAIN_WINDOW_REMOVALS: &[DocumentRemoval] = &[
    DocumentRemoval {
        waits_for: "missing: Win32Properties",
        start: "Win32Properties.WindowCornerPreference=\"{Binding Win32WindowCornerPreference}\"",
        end: "\"",
    },
];

/// The data templates of `MainWindow.xaml`, removed while the view they
/// name is not ported.
const MAIN_WINDOW_DATA_TEMPLATES: DocumentRemoval = DocumentRemoval {
    waits_for: "pending: Views/CustomNotificationView",
    start: "<Window.DataTemplates>",
    end: "</Window.DataTemplates>",
};

/// Populates the main window `root` from the subset of `MainWindow.xaml`
/// that loads.
pub fn load_main_window_subset(root: BoxedValue) -> Result<(), XamlLoadException> {
    let path = crate::MainWindow::DOCUMENT_PATH;
    let mut subset = remove_elements(crate::markup::embedded_text(path)?, MAIN_WINDOW_REMOVALS);
    if crate::markup::XamlClass::find("/Views/CustomNotificationView.xaml").is_none() {
        subset = remove_elements(&subset, &[MAIN_WINDOW_DATA_TEMPLATES]);
    }
    crate::markup::try_load_text(&subset, Some(path), Some(root)).map(|_| ())
}

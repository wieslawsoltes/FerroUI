//! Port of `StorageProvider/Utilities/FileFilterParser.cs`.

use ferroui_base::platform::storage::FilePickerFileType;
use std::rc::Rc;

/// Parses the filters of the pickers (an internal class of the reference).
pub(crate) struct FileFilterParser;

impl FileFilterParser {
    /// Converts a filter in the format `Description (*.ext)|*.ext;*.other|...` (pairs of a
    /// description and the patterns of the type, separated by `|`) to the file types of a picker.
    /// `None` when the filter is blank or not made of pairs.
    pub(crate) fn convert_to_file_picker_file_type(filter: &str) -> Option<Vec<Rc<FilePickerFileType>>> {
        if filter.trim().is_empty() {
            return None;
        }

        let parts: Vec<&str> = filter.split('|').collect();
        if parts.len() % 2 != 0 {
            return None;
        }

        let mut file_types = Vec::new();

        for pair in parts.chunks(2) {
            let mut description = pair[0];
            let pattern_part = pair[1];
            if let Some(index) = description.find(" (").filter(|index| *index > 0) {
                description = &description[..index];
            }

            let description = description.trim();

            let patterns: Vec<String> = pattern_part
                .split(';')
                .filter(|pattern| !pattern.is_empty())
                .map(|pattern| pattern.trim().to_string())
                .collect();

            let file_type = FilePickerFileType::new(Some(description));
            file_type.set_patterns(Some(patterns));
            file_types.push(file_type);
        }

        Some(file_types)
    }
}

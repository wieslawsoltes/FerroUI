use super::FerroDialogsInternalViewModelBase;
use ferroui_base::data::model::{Event, INotifyPropertyChanged};
use ferroui_base::ferro_markup_type;
use ferroui_base::platform::storage::FilePickerFileType;
use std::fmt;
use std::rc::Rc;

/// A file name pattern of a filter: `*` stands for any text (line breaks
/// included), every other character for itself, compared without regard to
/// case.
///
/// The counterpart of the regular expression the original builds from a
/// pattern (`^` + the escaped pattern with `\*` replaced by `.*` + `$`,
/// single line, ignoring case): the escaping leaves `*` as the only
/// wildcard, so a match of literal parts is the same test.
struct FileNamePattern {
    /// The literal parts between the wildcards, in lower case.
    parts: Vec<Vec<char>>,
}

fn lower(text: &str) -> Vec<char> {
    text.chars().flat_map(char::to_lowercase).collect()
}

impl FileNamePattern {
    fn new(pattern: &str) -> Self {
        Self { parts: pattern.split('*').map(lower).collect() }
    }

    fn is_match(&self, file_name: &str) -> bool {
        let text = lower(file_name);
        let (first, rest) = self.parts.split_first().expect("splitting yields at least one part");

        // Without a wildcard the pattern is the whole name.
        let Some((last, middle)) = rest.split_last() else {
            return text == *first;
        };

        if text.len() < first.len() + last.len() || !text.starts_with(first) || !text.ends_with(last) {
            return false;
        }

        // The middle parts in order, each as early as possible, between the prefix and the suffix.
        let end = text.len() - last.len();
        let mut position = first.len();
        for part in middle {
            if part.is_empty() {
                continue;
            }
            match text[position..end].windows(part.len()).position(|window| window == part.as_slice()) {
                Some(offset) => position += offset + part.len(),
                None => return false,
            }
        }

        true
    }
}

/// A file type of the managed file chooser: its name and the patterns of
/// the file names it shows.
pub struct ManagedFileChooserFilterViewModel {
    base: FerroDialogsInternalViewModelBase,
    patterns: Option<Vec<FileNamePattern>>,
    name: String,
    index: i32,
}

impl PartialEq for ManagedFileChooserFilterViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl INotifyPropertyChanged for ManagedFileChooserFilterViewModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}

impl ManagedFileChooserFilterViewModel {
    pub fn new(filter: &FilePickerFileType) -> Rc<Self> {
        Self::with_index(filter, 0)
    }

    pub fn with_index(filter: &FilePickerFileType, index: i32) -> Rc<Self> {
        let name = filter.name().to_string();
        let filter_patterns = filter.patterns();

        let patterns = if filter_patterns.as_deref().is_some_and(|patterns| patterns.iter().any(|p| p == "*.*")) {
            None
        } else {
            filter_patterns.map(|patterns| patterns.iter().map(|pattern| FileNamePattern::new(pattern)).collect())
        };

        Rc::new(Self { base: FerroDialogsInternalViewModelBase::new(), patterns, name, index })
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }

    /// The position of the file type in the list of the picker options.
    pub(crate) fn index(&self) -> i32 {
        self.index
    }

    pub fn is_match(&self, filename: &str) -> bool {
        match &self.patterns {
            None => true,
            Some(patterns) => patterns.iter().any(|pattern| pattern.is_match(filename)),
        }
    }
}

impl fmt::Display for ManagedFileChooserFilterViewModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)
    }
}

ferro_markup_type!(class ManagedFileChooserFilterViewModel {
    this: Rc<ManagedFileChooserFilterViewModel>,
    handles: [
        ManagedFileChooserFilterViewModel,
        Rc<ManagedFileChooserFilterViewModel>,
        Option<Rc<ManagedFileChooserFilterViewModel>>
    ],
    properties: [Name: String { get: ManagedFileChooserFilterViewModel::name }],
    methods: [
        fn Match(String) -> bool => |this: &Rc<ManagedFileChooserFilterViewModel>, filename: String| this.is_match(&filename),
        fn ToString() -> String => |this: &Rc<ManagedFileChooserFilterViewModel>| this.to_string(),
    ],
    notify_property_changed: ManagedFileChooserFilterViewModel,
});

#[cfg(test)]
mod tests {
    // Not from upstream: the upstream project has no tests.
    use super::*;

    fn filter(patterns: Option<&[&str]>) -> Rc<ManagedFileChooserFilterViewModel> {
        let file_type = FilePickerFileType::new(Some("Filter"));
        let file_type = match patterns {
            Some(patterns) => file_type.with_patterns(patterns),
            None => file_type,
        };
        ManagedFileChooserFilterViewModel::with_index(&file_type, 3)
    }

    #[test]
    fn keeps_the_name_and_the_index_of_the_file_type() {
        let filter = filter(Some(&["*.txt"]));

        assert_eq!("Filter", filter.name());
        assert_eq!("Filter", filter.to_string());
        assert_eq!(3, filter.index());
    }

    #[test]
    fn matches_every_name_without_patterns_or_with_the_all_files_pattern() {
        assert!(filter(None).is_match("anything.bin"));
        assert!(filter(Some(&["*.txt", "*.*"])).is_match("anything.bin"));
    }

    #[test]
    fn matches_the_whole_name_ignoring_case() {
        let filter = filter(Some(&["*.txt", "readme"]));

        assert!(filter.is_match("notes.txt"));
        assert!(filter.is_match("NOTES.TXT"));
        assert!(filter.is_match(".txt"));
        assert!(filter.is_match("README"));
        assert!(!filter.is_match("notes.txt.bak"));
        assert!(!filter.is_match("readme.md"));
        assert!(!filter.is_match("notes.tx"));
    }

    #[test]
    fn treats_only_the_star_as_a_wildcard() {
        let filter = filter(Some(&["a?c.[x]", "img*_*.png"]));

        assert!(filter.is_match("a?c.[x]"));
        assert!(!filter.is_match("abc.x"));
        assert!(filter.is_match("img_.png"));
        assert!(filter.is_match("img01_large.png"));
        assert!(!filter.is_match("img01.png"));
    }

    #[test]
    fn a_star_matches_line_breaks() {
        assert!(filter(Some(&["a*b"])).is_match("a\nb"));
    }

    #[test]
    fn overlapping_prefix_and_suffix_do_not_match() {
        // `^ab.*ba$` needs at least four characters.
        assert!(!filter(Some(&["ab*ba"])).is_match("aba"));
        assert!(filter(Some(&["ab*ba"])).is_match("abba"));
    }
}

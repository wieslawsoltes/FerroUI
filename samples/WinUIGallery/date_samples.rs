//! The code behind of the pinned DatePicker, TimePicker and CalendarView pages in Rust:
//! `OnNavigatedTo` of `DatePickerPage.xaml.cs`, the time of day the TimePicker page binds, and
//! `CalendarViewPage.xaml.cs` with its language list (`Helpers/Language/LanguageList.cs`). The
//! CalendarDatePicker page
//! has no code-behind upstream. The pickers and the calendar own their input; the sample owns
//! only its values and options.

use ferroui_base::animation::SignedDuration;
use ferroui_base::utilities::{DateTime, DateTimeOffset, Locale};
use ferroui_base::{BoxedValue, Ref};
use ferroui_controls::primitives::{SelectingItemsControl, ToggleButton};
use ferroui_controls::{
    Calendar, CalendarIdentifiers, CalendarSelectionMode, CheckBox, ComboBox, Control, DatePicker, ItemsSource,
    TimePicker,
};
use std::rc::Rc;

pub(crate) fn wire(page_id: &str, root: &Ref<Control>) {
    match page_id {
        "DatePicker" => wire_date_picker(root),
        "TimePicker" => wire_time_picker(root),
        "CalendarView" => wire_calendar_view(root),
        _ => {}
    }
}

/// `OnNavigatedTo` of `DatePickerPage.xaml.cs`: the second picker shows the date two months from
/// now, within the years from this one to five years from now.
fn wire_date_picker(root: &Ref<Control>) {
    let picker = root.get_control::<DatePicker>("Control2");
    let now = DateTimeOffset::now();
    picker.set_selected_date(Some(now.add_months(2)));
    picker.set_min_year(now);
    picker.set_max_year(now.add_years(5));
}

/// The source's `sys:DateTime.Now.TimeOfDay`: the time elapsed since midnight of today.
pub(crate) fn time_of_day(now: DateTime) -> SignedDuration {
    now.subtract_date_time(DateTime::new(now.year(), now.month(), now.day()))
}

/// The OneTime `{x:Bind sys:DateTime.Now.TimeOfDay}` of the two clock pickers of the TimePicker
/// page, set when the page is created.
fn wire_time_picker(root: &Ref<Control>) {
    let now = time_of_day(DateTime::now());
    for name in ["TwelveHourPicker", "TwentyFourHourPicker"] {
        root.get_control::<TimePicker>(name)
            .set_selected_time(Some(now));
    }
}

/// The selection mode of WinUI's `CalendarViewSelectionMode` name (None, Single, Multiple).
pub(crate) fn calendar_selection_mode(name: &str) -> Option<CalendarSelectionMode> {
    match name {
        "None" => Some(CalendarSelectionMode::None),
        "Single" => Some(CalendarSelectionMode::SingleDate),
        "Multiple" => Some(CalendarSelectionMode::MultipleRange),
        _ => None,
    }
}

/// The languages of the CalendarView page (`LanguageList.Languages` of the source): the name
/// shown and the language tag.
pub(crate) const CALENDAR_LANGUAGES: [(&str, &str); 86] = [
    ("English", "en"),
    ("Arabic", "ar"),
    ("Afrikaans", "af"),
    ("Albanian", "sq"),
    ("Amharic", "am"),
    ("Armenian", "hy"),
    ("Assamese", "as"),
    ("Azerbaijani", "az"),
    ("Basque ", "eu"),
    ("Belarusian", "be"),
    ("Bangla", "bn"),
    ("Bosnian", "bs"),
    ("Bulgarian", "bg"),
    ("Catalan", "ca"),
    ("Chinese (Simplified)", "zh"),
    ("Croatian", "hr"),
    ("Czech", "cs"),
    ("Danish", "da"),
    ("Dari", "prs"),
    ("Dutch", "nl"),
    ("Estonian", "et"),
    ("Filipino", "fil"),
    ("Finnish", "fi"),
    ("French ", "fr"),
    ("Galician", "gl"),
    ("Georgian", "ka"),
    ("German", "de"),
    ("Greek", "el"),
    ("Gujarati", "gu"),
    ("Hausa", "ha"),
    ("Hebrew", "he"),
    ("Hindi", "hi"),
    ("Hungarian", "hu"),
    ("Icelandic", "is"),
    ("Indonesian", "id"),
    ("Irish", "ga"),
    ("isiXhosa", "xh"),
    ("isiZulu", "zu"),
    ("Italian", "it"),
    ("Japanese ", "ja"),
    ("Kannada", "kn"),
    ("Kazakh", "kk"),
    ("Khmer", "km"),
    ("Kinyarwanda", "rw"),
    ("KiSwahili", "sw"),
    ("Konkani", "kok"),
    ("Korean", "ko"),
    ("Lao", "lo"),
    ("Latvian", "lv"),
    ("Lithuanian", "lt"),
    ("Luxembourgish", "lb"),
    ("Macedonian", "mk"),
    ("Malay", "ms"),
    ("Malayalam", "ml"),
    ("Maltese", "mt"),
    ("Maori ", "mi"),
    ("Marathi", "mr"),
    ("Nepali", "ne"),
    ("Norwegian", "nb"),
    ("Odia", "or"),
    ("Persian", "fa"),
    ("Polish", "pl"),
    ("Portuguese", "pt"),
    ("Punjabi", "pa"),
    ("Quechua", "quz"),
    ("Romanian", "ro"),
    ("Russian", "ru"),
    ("Serbian (Latin)", "sr"),
    ("Sesotho sa Leboa", "nso"),
    ("Setswana", "tn"),
    ("Sinhala", "si"),
    ("Slovak ", "sk"),
    ("Slovenian", "sl"),
    ("Spanish", "es"),
    ("Swedish", "sv"),
    ("Tamil", "ta"),
    ("Telugu", "te"),
    ("Thai", "th"),
    ("Tigrinya", "ti"),
    ("Turkish", "tr"),
    ("Ukrainian", "uk"),
    ("Urdu", "ur"),
    ("Uzbek (Latin)", "uz"),
    ("Vietnamese", "vi"),
    ("Welsh", "cy"),
    ("Wolof", "wo"),
];

/// The selected item of a combo box of the page as text.
fn selected_text(combo_box: &ComboBox) -> String {
    combo_box
        .selected_item()
        .as_ref()
        .and_then(|item| item.downcast_ref::<String>().cloned())
        .unwrap_or_default()
}

fn text_items<'a>(texts: impl IntoIterator<Item = &'a str>) -> ItemsSource {
    ItemsSource::from_items(texts.into_iter().map(|text| Some(Rc::new(text.to_owned()) as BoxedValue)))
}

/// `CalendarViewPage.xaml.cs`: `SelectionMode_SelectionChanged` (the selected name is the
/// selection mode of the calendar), the calendar identifiers of the constructor (bound to
/// `CalendarIdentifier`), `calendarLanguages_SelectionChanged` (the selected language is the
/// `Language` of the calendar) and the `IsGroupLabelVisible` and `IsOutOfScopeEnabled` check boxes.
fn wire_calendar_view(root: &Ref<Control>) {
    let calendar = root.get_control::<Calendar>("Control1");
    let identifiers = root.get_control::<ComboBox>("calendarIdentifier");
    identifiers.set_items_source(Some(text_items(CalendarIdentifiers::ALL)));
    identifiers.set_selected_index(0);
    let weak_identifiers = identifiers.downgrade();
    let weak = calendar.downgrade();
    identifiers.property_changed(move |change| {
        if change.property() != SelectingItemsControl::selected_index_property().as_property() {
            return;
        }
        if let (Some(identifiers), Some(calendar)) = (weak_identifiers.upgrade(), weak.upgrade()) {
            let identifier = selected_text(&identifiers);
            if !identifier.is_empty() {
                calendar.set_calendar_identifier(identifier);
            }
        }
    });

    let languages = root.get_control::<ComboBox>("calendarLanguages");
    languages.set_items_source(Some(text_items(CALENDAR_LANGUAGES.iter().map(|(name, _)| *name))));
    let weak_languages = languages.downgrade();
    let weak = calendar.downgrade();
    languages.property_changed(move |change| {
        if change.property() != SelectingItemsControl::selected_index_property().as_property() {
            return;
        }
        let (Some(languages), Some(calendar)) = (weak_languages.upgrade(), weak.upgrade()) else {
            return;
        };
        // `Language.IsWellFormed` of the source: a tag that does not parse leaves the language.
        let tag = usize::try_from(languages.selected_index()).ok().and_then(|index| CALENDAR_LANGUAGES.get(index));
        if let Some(locale) = tag.and_then(|(_, tag)| Locale::try_from_str(tag).ok()) {
            calendar.set_language(locale);
        }
    });
    languages.set_selected_index(0);

    let group_label = root.get_control::<CheckBox>("isGroupLabelVisible");
    let weak_check_box = group_label.downgrade();
    let weak = calendar.downgrade();
    group_label.property_changed(move |change| {
        if change.property() != ToggleButton::is_checked_property().as_property() {
            return;
        }
        if let (Some(check_box), Some(calendar)) = (weak_check_box.upgrade(), weak.upgrade()) {
            calendar.set_is_group_label_visible(check_box.is_checked() == Some(true));
        }
    });

    let out_of_scope = root.get_control::<CheckBox>("isOutOfScopeEnabled");
    let weak_check_box = out_of_scope.downgrade();
    let weak = calendar.downgrade();
    out_of_scope.property_changed(move |change| {
        if change.property() != ToggleButton::is_checked_property().as_property() {
            return;
        }
        if let (Some(check_box), Some(calendar)) = (weak_check_box.upgrade(), weak.upgrade()) {
            calendar.set_is_out_of_scope_enabled(check_box.is_checked() == Some(true));
        }
    });


    let combo_box = root.get_control::<ComboBox>("selectionMode");
    let weak_combo_box = combo_box.downgrade();
    let weak_calendar = root.get_control::<Calendar>("Control1").downgrade();
    combo_box.property_changed(move |change| {
        if change.property() != SelectingItemsControl::selected_index_property().as_property() {
            return;
        }
        let (Some(combo_box), Some(calendar)) = (weak_combo_box.upgrade(), weak_calendar.upgrade())
        else {
            return;
        };
        let item = combo_box.selected_item();
        let name = item
            .as_ref()
            .and_then(|item| item.downcast_ref::<String>())
            .map(String::as_str)
            .unwrap_or_default();
        if let Some(mode) = calendar_selection_mode(name) {
            calendar.set_selection_mode(mode);
        }
    });
}

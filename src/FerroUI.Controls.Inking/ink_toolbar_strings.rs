//! The source resource keys, translated through FerroUI culture and resource APIs.
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::{ContentControl, Control, TextBlock, ToolTip};
use ferroui_base::controls::{ResourceHostRef, ResourceKey};
use ferroui_base::data::BindingPriority;
use ferroui_base::reactive::{IObservable, ObservableExt};
use ferroui_base::utilities::CultureInfo;
use ferroui_base::{BoxedValue, StyledElement};
use std::rc::Rc;

#[path = "ink_toolbar_strings_data.rs"]
mod data;

fn culture_entries(name: &str) -> Option<&'static [(&'static str, &'static str)]> {
    data::CULTURES
        .binary_search_by_key(&name, |(culture, _)| *culture)
        .ok()
        .map(|index| data::CULTURES[index].1)
}

fn lookup(entries: &[(&'static str, &'static str)], key: &str) -> Option<&'static str> {
    entries
        .binary_search_by_key(&key, |(name, _)| *name)
        .ok()
        .map(|index| entries[index].1)
}

/// Match a supported language tag and then its language/script, with English fallback.
/// Culture is provided by the existing framework service, including on browser targets.
pub(crate) fn for_culture(key: &str, culture: &str) -> &'static str {
    let name = culture.replace('_', "-").to_ascii_lowercase();
    if let Some(value) = culture_entries(&name).and_then(|entries| lookup(entries, key)) {
        return value;
    }
    let preferred = if name.starts_with("zh-hant") || ["zh-hk", "zh-mo"].contains(&name.as_str()) {
        "zh-tw"
    } else if name.starts_with("zh") {
        "zh-cn"
    } else if name.starts_with("sr-latn") {
        "sr-latn-rs"
    } else if name.starts_with("sr") {
        "sr-cyrl-rs"
    } else {
        match name.split('-').next().unwrap_or_default() {
            "en" => "en-us",
            "es" => "es-es",
            "fr" => "fr-fr",
            "pt" => "pt-pt",
            language => data::CULTURES
                .iter()
                .find_map(|(tag, _)| (tag.split('-').next() == Some(language)).then_some(*tag))
                .unwrap_or("en-us"),
        }
    };
    culture_entries(preferred)
        .and_then(|entries| lookup(entries, key))
        .or_else(|| culture_entries("en-us").and_then(|entries| lookup(entries, key)))
        .unwrap_or_default()
}

pub(crate) fn text(control: &StyledElement, key: &str) -> String {
    control
        .try_find_resource(&ResourceKey::from(key), None)
        .flatten()
        .and_then(|value| value.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| for_culture(key, CultureInfo::current_ui_culture().name()).to_owned())
}

pub(crate) fn observable(
    control: &StyledElement,
    key: &'static str,
) -> Rc<dyn IObservable<String>> {
    ResourceHostRef::from(&control.to_ref())
        .resource_observable(key, None)
        .select(move |value| {
            value
                .and_then(|value| value.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| {
                    for_culture(key, CultureInfo::current_ui_culture().name()).to_owned()
                })
        })
}

/// Default names remain overridable by local values and by application resources.
pub(crate) fn bind_name(control: &Control, key: &'static str) {
    control.bind(
        AutomationProperties::name_property(),
        observable(control, key).select(Some),
        BindingPriority::Style,
    );
    control.bind(
        ToolTip::tip_property(),
        observable(control, key).select(|text| Some(Rc::new(text) as BoxedValue)),
        BindingPriority::Style,
    );
}

pub(crate) fn bind_content(control: &ContentControl, key: &'static str) {
    control.bind(
        ContentControl::content_property(),
        observable(control, key).select(|text| Some(Rc::new(text) as BoxedValue)),
        BindingPriority::Style,
    );
    bind_name(control, key);
}

pub(crate) fn bind_label(control: &TextBlock, key: &'static str) {
    control.bind(
        TextBlock::text_property(),
        observable(control, key).select(Some),
        BindingPriority::Style,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_source_locale_has_all_resource_keys_in_lookup_order() {
        assert_eq!(data::CULTURES.len(), 86);
        assert!(data::CULTURES.windows(2).all(|pair| pair[0].0 < pair[1].0));
        let keys: Vec<_> = culture_entries("en-us")
            .unwrap()
            .iter()
            .map(|entry| entry.0)
            .collect();
        assert_eq!(keys.len(), 59);
        for (culture, entries) in data::CULTURES {
            assert_eq!(
                entries.iter().map(|entry| entry.0).collect::<Vec<_>>(),
                keys,
                "{culture}"
            );
            for (key, translation) in *entries {
                assert_eq!(for_culture(key, culture), *translation, "{culture}: {key}");
            }
        }
    }

    #[test]
    fn culture_fallback_preserves_language_and_script() {
        let key = "InkToolbarBallpointPenButtonName";
        assert_eq!(for_culture(key, "PL_pl"), "Pióro kulkowe");
        assert_eq!(for_culture(key, "fr-CA"), "Stylet pointe bille");
        assert_eq!(for_culture(key, "zh-Hant-HK"), for_culture(key, "zh-TW"));
        assert_eq!(for_culture(key, "zh-Hans-SG"), for_culture(key, "zh-CN"));
        assert_eq!(
            for_culture(key, "sr-Latn-BA"),
            for_culture(key, "sr-Latn-RS")
        );
        assert_eq!(
            for_culture(key, "sr-Cyrl-ME"),
            for_culture(key, "sr-Cyrl-RS")
        );
        // Bosnia has an independent pinned translation and must take precedence over fallback.
        assert_eq!(for_culture(key, "sr-Cyrl-BA"), "Хемијска оловка");
        assert_eq!(for_culture(key, "unknown"), "Ballpoint pen");
        assert_eq!(for_culture("UnknownResource", "pl-PL"), "");
    }
}

//! Port of `tests/Themes.UnitTests/ResourceDictionaryTests.cs`.

use super::support::*;
use ferroui_base::controls::{ResourceDictionary, ResourceKey};
use ferroui_base::styling::ThemeVariant;

#[test]
fn should_define_identical_keys_in_theme_dictionaries() {
    // Any resource defined in one ThemeDictionary, must be defined in the rest
    let _app = start_application();
    let theme = create_attached_theme();

    // We expect that our built-in themes define ThemeDictionaries on the root style object.
    let flat: Vec<(ThemeVariant, Vec<ResourceKey>)> = theme
        .resources()
        .theme_dictionaries_snapshot()
        .into_iter()
        .map(|(variant, provider)| {
            let dictionary = provider
                .as_object()
                .and_then(|object| object.downcast_ref::<ResourceDictionary>())
                .expect("a resource dictionary")
                .to_ref();
            let mut entries = Vec::new();
            enumerate_dictionary_resources(&dictionary, None, &mut entries);
            (variant, entries.into_iter().map(|entry| entry.key).collect())
        })
        .collect();

    assert!(!flat.is_empty());
    for (variant, keys) in &flat {
        let other_keys: Vec<&ResourceKey> =
            flat.iter().filter(|(other, _)| other != variant).flat_map(|(_, keys)| keys.iter()).collect();
        for key in keys {
            assert!(other_keys.contains(&key), "{key:?} of {variant:?} is not defined in the other variants");
        }
    }
}

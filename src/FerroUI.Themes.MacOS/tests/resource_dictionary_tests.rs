//! The rule of `tests/Themes.UnitTests/ResourceDictionaryTests.cs` of the upstream project (as
//! the tests of the Fluent theme), for the three theme dictionaries of the WinUI theme.

use super::support::*;
use crate::MacOSTheme;
use ferroui_base::controls::{ResourceDictionary, ResourceKey};
use ferroui_base::styling::ThemeVariant;

#[test]
fn should_define_identical_keys_in_theme_dictionaries() {
    // Any resource defined in one ThemeDictionary, must be defined in the rest
    let _app = start_application();
    let theme = create_attached_theme();

    // The theme defines its theme dictionaries on the root style object: the resource documents
    // are merged into its dictionary.
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

    // Default (the dark theme of WinUI), Light and HighContrast.
    assert_eq!(3, flat.len());
    assert!(flat.iter().any(|(variant, _)| *variant == ThemeVariant::default()));
    assert!(flat.iter().any(|(variant, _)| *variant == ThemeVariant::light()));
    assert!(flat.iter().any(|(variant, _)| *variant == MacOSTheme::high_contrast()));
    for (variant, keys) in &flat {
        for (other_variant, other_keys) in &flat {
            if variant == other_variant {
                continue;
            }
            let missing = missing_keys(keys, other_keys);
            assert!(missing.is_empty(),
                "resources declared by {variant:?} but missing from {other_variant:?}: {missing:?}");
        }
    }
}

/// Compare each variant independently. Comparing with the union of the other
/// variants accepts a key present in Light and Default but absent in contrast.
fn missing_keys<'a>(expected: &'a [ResourceKey], actual: &[ResourceKey]) -> Vec<&'a ResourceKey> {
    expected.iter().filter(|key| !actual.contains(key)).collect()
}

#[test]
fn pairwise_comparison_detects_a_key_missing_from_only_one_variant() {
    let normal = vec![ResourceKey::from("Common"), ResourceKey::from("Surface")];
    let contrast = vec![ResourceKey::from("Common")];
    assert!(missing_keys(&normal, &normal).is_empty());
    assert!(missing_keys(&contrast, &normal).is_empty());
    assert_eq!(vec![&normal[1]], missing_keys(&normal, &contrast));
}

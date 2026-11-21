//! Native dictionary traversal for RibbonElementFactory.FindTemplate.
//! Source Uno TryGetValue/GetFromMerged bodies are retained by the development
//! reference harness. Copyright Uno Platform, Apache-2.0; see third-party notices.
//! This adapter changes no native dictionary storage or ordinary lookup policy.
use super::custom_template_theme::{TemplateTheme, select_theme};
use ferroui_base::controls::{IResourceDictionary, ResourceKey, ResourceValue};
#[cfg(test)]
use ferroui_base::styling::ThemeVariant;

/// Lookup with an explicitly selected native theme and a real system-resource
/// endpoint supplied by the caller. Children never invoke the system fallback.
/// Some(None) means a present null and must stop traversal.
#[cfg(test)]
pub(super) fn lookup(
    dictionary: &dyn IResourceDictionary,
    key: &ResourceKey,
    theme: &ThemeVariant,
    system: impl FnOnce() -> Option<ResourceValue>,
) -> Option<ResourceValue> {
    lookup_with_context(
        dictionary,
        key,
        &TemplateTheme::new(theme.clone(), false),
        system,
    )
}

#[cfg(test)]
pub(super) fn lookup_with_context(
    dictionary: &dyn IResourceDictionary,
    key: &ResourceKey,
    theme: &TemplateTheme,
    system: impl FnOnce() -> Option<ResourceValue>,
) -> Option<ResourceValue> {
    lookup_with_theme(dictionary, key, &|| theme.clone(), system)
}

/// Ask for the actual theme at each selection point, including after lazy child
/// construction, so native callbacks can change the ambient context reentrantly.
pub(super) fn lookup_with_theme(
    dictionary: &dyn IResourceDictionary,
    key: &ResourceKey,
    theme: &dyn Fn() -> TemplateTheme,
    system: impl FnOnce() -> Option<ResourceValue>,
) -> Option<ResourceValue> {
    lookup_dictionary(dictionary, key, theme).or_else(system)
}

fn lookup_dictionary(
    dictionary: &dyn IResourceDictionary,
    key: &ResourceKey,
    theme: &dyn Fn() -> TemplateTheme,
) -> Option<ResourceValue> {
    if let Some(value) = dictionary.try_get_value(key) {
        return Some(value);
    }
    // Source captures Count once, then indexes the live list for each child.
    // Refreshing the native snapshot for each index preserves changes made by
    // deferred entries or include constructors between child visits.
    let count = dictionary.merged_dictionaries_snapshot().len();
    for index in (0..count).rev() {
        let child = dictionary.merged_dictionaries_snapshot()[index].clone();
        let child = child.resource_dictionary().expect(
            "Ribbon custom template lookup requires merged providers to expose native dictionaries",
        );
        if let Some(value) = lookup_dictionary(child.as_ref(), key, theme) {
            return Some(value);
        }
    }
    if let Some(dictionary) = select_theme(dictionary, &theme()) {
        return lookup_dictionary(dictionary.as_ref(), key, theme);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_base::{BoxedValue, Ref, controls::ResourceDictionary};
    use serde_json::Value;
    use std::{cell::Cell, rc::Rc};

    fn put(dictionary: &ResourceDictionary, text: Option<&str>) {
        dictionary.set("key", text.map(|s| Rc::new(s.to_owned()) as BoxedValue));
    }
    fn selected(dictionary: &ResourceDictionary, text: &str) {
        let values = ResourceDictionary::new();
        put(&values, Some(text));
        dictionary.add_theme_dictionary(ThemeVariant::dark(), values);
    }
    fn result(dictionary: &Ref<ResourceDictionary>, system: Option<&str>) -> Option<ResourceValue> {
        let dictionary: Rc<dyn IResourceDictionary> = dictionary.clone().into();
        lookup(
            dictionary.as_ref(),
            &"key".into(),
            &ThemeVariant::dark(),
            || system.map(|s| Some(Rc::new(s.to_owned()) as BoxedValue)),
        )
    }

    #[test]
    fn original_resource_lookup_corpus_preserves_all_21_cases() {
        let fixture: Value = serde_json::from_str(include_str!(
            "../../../tests/fixtures/custom-resource-lookup.json"
        ))
        .unwrap();
        let cases = fixture["corpus"]["cases"].as_array().unwrap();
        assert_eq!(cases.len(), 21);
        for case in cases {
            let root = ResourceDictionary::new();
            let mut system = None;
            match case["name"].as_str().unwrap() {
                "precedence" => {
                    let mask = case["mask"].as_u64().unwrap();
                    let first = ResourceDictionary::new();
                    let last = ResourceDictionary::new();
                    root.add_merged_dictionary(first.clone());
                    root.add_merged_dictionary(last.clone());
                    if mask & 1 != 0 {
                        put(&root, Some("own"));
                    }
                    if mask & 2 != 0 {
                        put(&first, Some("merged-first"));
                    }
                    if mask & 4 != 0 {
                        put(&last, Some("merged-last"));
                    }
                    if mask & 8 != 0 {
                        selected(&root, "theme");
                    }
                    system = Some("system");
                }
                "own_null" => {
                    put(&root, None);
                    selected(&root, "theme");
                }
                "merged_null" => {
                    let child = ResourceDictionary::new();
                    put(&child, None);
                    root.add_merged_dictionary(child);
                    selected(&root, "theme");
                }
                "merged_theme" => {
                    let child = ResourceDictionary::new();
                    selected(&child, "merged-theme");
                    root.add_merged_dictionary(child);
                    selected(&root, "outer-theme");
                }
                "nested_merged" => {
                    let child = ResourceDictionary::new();
                    let nested = ResourceDictionary::new();
                    put(&nested, Some("nested"));
                    child.add_merged_dictionary(nested);
                    root.add_merged_dictionary(child);
                    selected(&root, "outer-theme");
                }
                "missing" => {}
                name => panic!("Unmapped source resource case: {name}"),
            }
            let actual = result(&root, system);
            assert_eq!(actual.is_some(), case["found"].as_bool().unwrap(), "{case}");
            let value = actual
                .flatten()
                .map(|v| v.downcast_ref::<String>().unwrap().clone());
            assert_eq!(value.as_deref(), case["value"].as_str(), "{case}");
        }
    }

    #[test]
    fn selected_dictionary_absent_key_does_not_fall_back_to_default_dictionary() {
        let root = ResourceDictionary::new();
        let defaults = ResourceDictionary::new();
        put(&defaults, Some("default"));
        root.add_theme_dictionary(ThemeVariant::default(), defaults);
        root.add_theme_dictionary(ThemeVariant::dark(), ResourceDictionary::new());
        assert!(result(&root, None).is_none());
        root.remove_theme_dictionary(&ThemeVariant::dark());
        assert_eq!(
            result(&root, None)
                .unwrap()
                .unwrap()
                .downcast_ref::<String>()
                .unwrap(),
            "default"
        );
    }

    #[test]
    fn null_stops_system_lookup_and_mutations_are_visible_without_stale_caches() {
        let root = ResourceDictionary::new();
        put(&root, None);
        let calls = Cell::new(0);
        let handle: Rc<dyn IResourceDictionary> = root.clone().into();
        assert!(matches!(
            lookup(
                handle.as_ref(),
                &"key".into(),
                &ThemeVariant::dark(),
                || {
                    calls.set(calls.get() + 1);
                    None
                }
            ),
            Some(None)
        ));
        assert_eq!(calls.get(), 0);
        root.remove(&"key".into());
        assert!(result(&root, None).is_none());
        selected(&root, "new theme");
        assert_eq!(
            result(&root, None)
                .unwrap()
                .unwrap()
                .downcast_ref::<String>()
                .unwrap(),
            "new theme"
        );
    }
}

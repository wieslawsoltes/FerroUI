//! Source custom-template theme-selection state, expressed through native
//! dictionaries and a scope-owned collection subscription. Copyright Uno
//! Platform, Apache-2.0; original methods/notices are retained in development tools.
use ferroui_base::{
    Ref,
    collections::FerroDictionary,
    controls::{IResourceDictionary, IThemeVariantProvider, ResourceDictionary},
    styling::ThemeVariant,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

#[derive(Clone, PartialEq)]
pub(super) struct TemplateTheme {
    base: ThemeVariant,
    high_contrast: bool,
}
impl TemplateTheme {
    pub(super) fn new(base: ThemeVariant, high_contrast: bool) -> Self {
        Self {
            base,
            high_contrast,
        }
    }
}
#[derive(Clone)]
struct Selection {
    context: TemplateTheme,
    dictionary: Rc<dyn IResourceDictionary>,
}
struct ThemeSelection {
    selected: RefCell<Option<Selection>>,
    themes: FerroDictionary<ThemeVariant, Rc<dyn IThemeVariantProvider>>,
    token: Cell<Option<u64>>,
}
impl Drop for ThemeSelection {
    fn drop(&mut self) {
        if let Some(token) = self.token.take() {
            self.themes.remove_collection_changed(token);
        }
    }
}
fn state(dictionary: &ResourceDictionary) -> Rc<ThemeSelection> {
    if let Some(state) = dictionary.instance_data::<ThemeSelection>() {
        return state;
    }
    let state = Rc::new(ThemeSelection {
        selected: RefCell::new(None),
        themes: dictionary.theme_dictionaries(),
        token: Cell::new(None),
    });
    let weak = Rc::downgrade(&state);
    let token = state.themes.add_collection_changed(Rc::new(move |_| {
        if let Some(state) = weak.upgrade() {
            // Release outside any RefCell borrow: resource disposal can reenter.
            let old = state.selected.replace(None);
            drop(old);
        }
    }));
    state.token.set(Some(token));
    dictionary.set_instance_data(Some(state.clone()));
    state
}

/// Source EnsureActiveThemeDictionary selection. A missing replacement preserves
/// the previously selected dictionary. Native collection changes clear it.
/// The cache owns a dictionary already owned by the native theme collection; the
/// callback holds a weak state reference and Drop unsubscribes, adding no owner cycle.
pub(super) fn select_theme(
    dictionary: &dyn IResourceDictionary,
    context: &TemplateTheme,
) -> Option<Rc<dyn IResourceDictionary>> {
    let native: Ref<ResourceDictionary> = dictionary
        .as_object()
        .and_then(|object| object.to_ref().cast::<ResourceDictionary>())
        .expect("Ribbon custom template themes require a native ResourceDictionary object");
    let state = state(&native);
    let previous = state.selected.borrow().clone();
    if let Some(previous) = &previous
        && previous.context == *context
    {
        return Some(previous.dictionary.clone());
    }
    // Obtain a fresh map for each attempt: lazy include construction can mutate it.
    let find = |variant: &ThemeVariant| {
        dictionary
            .theme_dictionaries_snapshot()
            .into_iter()
            .find(|(key, _)| key == variant)
            .and_then(|(_, provider)| provider.resource_dictionary())
    };
    let mut selected = None;
    if context.high_contrast {
        let key = if context.base == ThemeVariant::dark() {
            "HighContrastBlack"
        } else {
            "HighContrastWhite"
        };
        selected = find(&ThemeVariant::new(key, None))
            .or_else(|| find(&ThemeVariant::new("HighContrast", None)));
    }
    if selected.is_none() {
        if context.base != ThemeVariant::default() {
            selected = find(&context.base);
        }
        if selected.is_none() {
            selected = find(&ThemeVariant::default());
        }
    }
    if let Some(dictionary) = selected {
        let old = state.selected.replace(Some(Selection {
            context: context.clone(),
            dictionary: dictionary.clone(),
        }));
        drop(old);
        Some(dictionary)
    } else {
        previous.map(|previous| previous.dictionary)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    fn variant(key: &str) -> ThemeVariant {
        ThemeVariant::new(key, None)
    }
    fn put(root: &ResourceDictionary, key: &str) {
        let child = ResourceDictionary::new();
        child.add_value("key", key.to_string());
        root.add_theme_dictionary(variant(key), child);
    }
    fn check(root: &ResourceDictionary, expected: &Value) {
        let context = TemplateTheme::new(
            variant(expected["theme"].as_str().unwrap()),
            expected["contrast"].as_bool().unwrap(),
        );
        let dictionary: Rc<dyn IResourceDictionary> = root.to_ref().into();
        let selected = select_theme(dictionary.as_ref(), &context);
        let actual = selected
            .and_then(|d| d.try_get_value(&"key".into()).flatten())
            .map(|v| v.downcast_ref::<String>().unwrap().clone());
        assert_eq!(
            actual.as_deref(),
            expected["selected"].as_str(),
            "{expected}"
        );
        assert_eq!(actual.as_deref(), expected["value"].as_str(), "{expected}");
        // Source internal cache-invalidation counters and IsHighContrast flags
        // are recorded by the oracle, not exported as a parallel native API.
    }
    #[test]
    fn original_384_theme_selections_and_24_transition_steps_match() {
        let fixture: Value = serde_json::from_str(include_str!(
            "../../../tests/fixtures/custom-resource-themes.json"
        ))
        .unwrap();
        let keys = [
            "Default",
            "Light",
            "Dark",
            "HighContrast",
            "HighContrastWhite",
            "HighContrastBlack",
        ];
        let cases = fixture["corpus"]["cases"].as_array().unwrap();
        assert_eq!(cases.len(), 384);
        for expected in cases {
            let root = ResourceDictionary::new();
            let mask = expected["mask"].as_u64().unwrap();
            for (index, key) in keys.iter().enumerate() {
                if mask & (1 << index) != 0 {
                    put(&root, key);
                }
            }
            check(&root, expected);
        }
        let transitions = fixture["corpus"]["transitions"].as_array().unwrap();
        assert_eq!(transitions.len(), 4);
        for transition in transitions {
            let root = ResourceDictionary::new();
            for key in transition["available"].as_array().unwrap() {
                put(&root, key.as_str().unwrap());
            }
            assert_eq!(transition["steps"].as_array().unwrap().len(), 6);
            for step in transition["steps"].as_array().unwrap() {
                check(&root, step);
            }
        }
    }
    #[test]
    fn native_collection_change_clears_cached_choice_and_root_drop_releases_subscription() {
        let root = ResourceDictionary::new();
        put(&root, "Light");
        let dictionary: Rc<dyn IResourceDictionary> = root.clone().into();
        let light = TemplateTheme::new(ThemeVariant::light(), false);
        let dark = TemplateTheme::new(ThemeVariant::dark(), false);
        let first = select_theme(dictionary.as_ref(), &light).unwrap();
        assert!(*first == *select_theme(dictionary.as_ref(), &dark).unwrap());
        put(&root, "Dark");
        assert!(*first != *select_theme(dictionary.as_ref(), &dark).unwrap());
        root.remove_theme_dictionary(&ThemeVariant::dark());
        assert!(select_theme(dictionary.as_ref(), &dark).is_none());
        let cache = root.instance_data::<ThemeSelection>().unwrap();
        let weak = Rc::downgrade(&cache);
        let retained_map = root.theme_dictionaries();
        drop(cache);
        drop(dictionary);
        drop(root);
        assert!(
            weak.upgrade().is_none(),
            "native event handler retained theme-selection owner"
        );
        drop(retained_map);
    }
}

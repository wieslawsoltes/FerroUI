#[cfg(test)]
use ferroribbon_core::collections::RibbonFixedItems;
use ferroribbon_core::search::{RibbonSearchEngine, RibbonSearchLabelComparer};
use ferroui_base::{
    FerroLocator, LocatorExtensions,
    utilities::{CompareOptions, CultureInfo, ICultureDataProvider},
};
use std::rc::Rc;
pub(crate) fn current_compare_rules() -> Option<Rc<dyn ferroui_base::utilities::ICompareRules>> {
    let mut culture = CultureInfo::current_formatting();
    let mut rules = None;
    if let Some(provider) = FerroLocator::current().get_service::<dyn ICultureDataProvider>() {
        while rules.is_none() && !culture.is_invariant() {
            rules = provider.get_compare_rules_by_locale(culture.locale());
            culture = culture.parent();
        }
    }
    rules
}
pub(crate) fn install_culture_provider() {
    RibbonSearchEngine::set_label_comparer_provider(Some(Rc::new(|| {
        if let Some(rules) = current_compare_rules() {
            return Rc::new(
                move |a: &ferroribbon_core::RibbonString, b: &ferroribbon_core::RibbonString| {
                    let a = a
                        .to_utf8()
                        .expect("native culture comparison of malformed UTF16");
                    let b = b
                        .to_utf8()
                        .expect("native culture comparison of malformed UTF16");
                    rules.compare(&a, &b, CompareOptions::IGNORE_CASE)
                },
            ) as Rc<RibbonSearchLabelComparer>;
        }
        #[cfg(any(
            target_os = "macos",
            target_os = "windows",
            target_os = "emscripten",
            target_os = "linux"
        ))]
        {
            let collator = ferroribbon_platform::SearchLabelCollator::new(
                CultureInfo::current_formatting().name(),
            )
            .expect("native search locale");
            Rc::new(
                move |a: &ferroribbon_core::RibbonString, b: &ferroribbon_core::RibbonString| {
                    collator
                        .compare(a.as_utf16(), b.as_utf16())
                        .expect("native search label comparison")
                },
            ) as Rc<RibbonSearchLabelComparer>
        }
        #[cfg(not(any(
            target_os = "macos",
            target_os = "windows",
            target_os = "emscripten",
            target_os = "linux"
        )))]
        {
            Rc::new(
                |_: &ferroribbon_core::RibbonString,
                 _: &ferroribbon_core::RibbonString|
                 -> std::cmp::Ordering {
                    panic!(
                        "Unavailable: source current-culture search collation requires a provider on this platform"
                    )
                },
            ) as Rc<RibbonSearchLabelComparer>
        }
    })));
}
#[cfg(test)]
mod tests {
    use super::*;
    use ferroribbon_core::{
        collections::RibbonReadOnlyList,
        search::{RibbonSearchEntry, RibbonSearchEntryOptions},
    };
    use ferroui_base::utilities::ICompareRules;
    use std::cmp::Ordering;
    #[cfg(not(any(
        target_os = "macos",
        target_os = "windows",
        target_os = "emscripten",
        target_os = "linux"
    )))]
    use std::panic::{AssertUnwindSafe, catch_unwind};
    struct Rules(bool);
    impl ICompareRules for Rules {
        fn compare(&self, a: &str, b: &str, options: CompareOptions) -> Ordering {
            assert_eq!(options, CompareOptions::IGNORE_CASE);
            if self.0 { b.cmp(a) } else { a.cmp(b) }
        }
        fn is_prefix(&self, _: &str, _: &str, _: CompareOptions) -> bool {
            panic!("not used by search comparison")
        }
        fn index_of(&self, _: &str, _: &str, _: CompareOptions) -> i32 {
            panic!("not used by search comparison")
        }
    }
    struct Provider;
    impl ICultureDataProvider for Provider {
        fn get_compare_rules(&self, culture: &str) -> Option<Rc<dyn ICompareRules>> {
            match culture {
                "xx" => Some(Rc::new(Rules(true))),
                "yy" => Some(Rc::new(Rules(false))),
                _ => None,
            }
        }
    }
    #[test]
    fn search_captures_registered_parent_rules_and_uses_native_fallback() {
        let scope = FerroLocator::enter_scope();
        let old_culture = CultureInfo::current_formatting();
        let old_provider = RibbonSearchEngine::set_label_comparer_provider(None);
        FerroLocator::current_mutable()
            .bind::<dyn ICultureDataProvider>()
            .to_constant(Rc::new(Provider));
        install_culture_provider();
        let engine = RibbonSearchEngine::new();
        let options = RibbonSearchEntryOptions {
            keywords: Some(RibbonFixedItems::new(vec![Some("tie".into())]) as RibbonReadOnlyList<_>),
            ..Default::default()
        };
        engine.add(Some(RibbonSearchEntry::with_options(
            Some("a".into()),
            Some("A".into()),
            options.clone(),
        )));
        engine.add(Some(RibbonSearchEntry::with_options(
            Some("b".into()),
            Some("B".into()),
            options,
        )));
        CultureInfo::set_current_formatting(CultureInfo::get_culture_info("xx-YY"));
        assert_eq!(
            engine
                .search(Some(&"tie".into()), 12)
                .get(0)
                .unwrap()
                .entry()
                .unwrap()
                .id()
                .unwrap()
                .to_utf8()
                .unwrap(),
            "b"
        );
        CultureInfo::set_current_formatting(CultureInfo::get_culture_info("yy"));
        assert_eq!(
            engine
                .search(Some(&"tie".into()), 12)
                .get(0)
                .unwrap()
                .entry()
                .unwrap()
                .id()
                .unwrap()
                .to_utf8()
                .unwrap(),
            "a"
        );
        CultureInfo::set_current_formatting(CultureInfo::invariant_culture());
        #[cfg(not(any(
            target_os = "macos",
            target_os = "windows",
            target_os = "emscripten",
            target_os = "linux"
        )))]
        assert!(catch_unwind(AssertUnwindSafe(|| engine.search(Some(&"tie".into()), 12))).is_err());
        #[cfg(any(
            target_os = "macos",
            target_os = "windows",
            target_os = "emscripten",
            target_os = "linux"
        ))]
        assert_eq!(
            engine
                .search(Some(&"tie".into()), 12)
                .get(0)
                .unwrap()
                .entry()
                .unwrap()
                .id()
                .unwrap()
                .to_utf8()
                .unwrap(),
            "a"
        );
        CultureInfo::set_current_formatting(old_culture);
        RibbonSearchEngine::set_label_comparer_provider(old_provider);
        scope.dispose();
    }
    #[cfg(any(
        target_os = "macos",
        target_os = "windows",
        target_os = "emscripten",
        target_os = "linux"
    ))]
    #[test]
    fn native_tie_breaking_matches_original_engine_order_in_eight_cultures() {
        use ferroribbon_core::model::RibbonItemsSource;
        let corpus: serde_json::Value = serde_json::from_str(include_str!(
            "../../../tests/fixtures/search-collation.json"
        ))
        .unwrap();
        let old_culture = CultureInfo::current_formatting();
        let old_provider = RibbonSearchEngine::set_label_comparer_provider(None);
        install_culture_provider();
        for culture in corpus["corpus"]["cultures"].as_array().unwrap() {
            CultureInfo::set_current_formatting(CultureInfo::get_culture_info(
                culture["name"].as_str().unwrap(),
            ));
            let engine = RibbonSearchEngine::new();
            let entries = ["z", "ä", "A", "a", "Ö", "Z", "I", "ı", "İ", "i"]
                .into_iter()
                .enumerate()
                .map(|(index, label)| {
                    Some(RibbonSearchEntry::with_options(
                        Some(index.to_string().into()),
                        Some(label.into()),
                        RibbonSearchEntryOptions {
                            keywords: Some(RibbonFixedItems::new(vec![Some("query".into())])
                                as RibbonReadOnlyList<_>),
                            ..Default::default()
                        },
                    ))
                })
                .collect();
            engine.set_entries(Some(RibbonItemsSource::from_values(entries)));
            let actual: Vec<_> = engine
                .search(Some(&"query".into()), 100)
                .iter()
                .map(|result| {
                    result
                        .unwrap()
                        .entry()
                        .unwrap()
                        .id()
                        .unwrap()
                        .to_utf8()
                        .unwrap()
                })
                .collect();
            let expected: Vec<_> = culture["ids"]
                .as_array()
                .unwrap()
                .iter()
                .map(|id| id.as_str().unwrap().to_string())
                .collect();
            assert_eq!(actual, expected, "culture {}", culture["name"]);
        }
        CultureInfo::set_current_formatting(old_culture);
        RibbonSearchEngine::set_label_comparer_provider(old_provider);
    }
}

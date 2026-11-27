//! Joined original FindTemplate and Uno dictionary/selection corpus in native controls.
use super::*;
use crate::{compat::DataTemplateSelectorImpl, controls::Ribbon};
use ferroui_base::{
    BoxedValue, FerroLocator,
    controls::ResourceDictionary,
    data::core::ValueTypes,
    platform::{DefaultPlatformSettings, IPlatformSettings},
    reactive::IDisposable,
    styling::ThemeVariant,
};
use ferroui_controls::{Application, templates::IDataTemplate};
use ferroui_markup_xaml::templates::DataTemplate;
use serde_json::{Value, json};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
struct Scope(Rc<dyn IDisposable>);
impl Drop for Scope {
    fn drop(&mut self) {
        self.0.dispose();
    }
}
struct Selector {
    template: Rc<dyn IDataTemplate>,
    trace: RefCell<Vec<String>>,
}
impl DataTemplateSelectorImpl for Selector {
    fn select_template_core(&self, content: Option<BoxedValue>) -> Option<Rc<dyn IDataTemplate>> {
        self.trace.borrow_mut().push(
            content
                .as_ref()
                .map(|v| ValueTypes::to_display_string(Some(v)))
                .unwrap_or("<null>".into()),
        );
        Some(self.template.clone())
    }
}
fn named(values: &mut Vec<(String, Rc<dyn IDataTemplate>)>, name: &str) -> Option<BoxedValue> {
    let template: Rc<dyn IDataTemplate> = DataTemplate::new();
    values.push((name.to_owned(), template.clone()));
    Some(Rc::new(template))
}
fn put_theme(root: &ResourceDictionary, name: &str, value: Option<BoxedValue>) {
    let child = ResourceDictionary::new();
    child.add("key", value);
    root.add_theme_dictionary(ThemeVariant::new(name, None), child);
}
#[test]
fn original_joined_factory_lookup_precedence_nulls_high_contrast_and_reentrant_application() {
    crate::group_tests::run(|| {
        let fixture: Value = serde_json::from_str(include_str!(
            "../../../tests/fixtures/factory-resource.json"
        ))
        .unwrap();
        assert_eq!(fixture["corpus"]["cases"].as_array().unwrap().len(), 79);
        for expected in fixture["corpus"]["cases"].as_array().unwrap() {
            let _scope = Scope(FerroLocator::enter_scope());
            FerroLocator::current_mutable()
                .bind::<dyn IPlatformSettings>()
                .to_constant(Rc::new(DefaultPlatformSettings::new()));
            let app = Application::new();
            Application::bind_current(app.clone());
            app.set_value(
                Application::requested_theme_variant_property(),
                Some(ThemeVariant::dark()),
            );
            let ribbon = Ribbon::new();
            ribbon.set_value(
                ThemeVariant::requested_theme_variant_property(),
                Some(ThemeVariant::dark()),
            );
            let factory = RibbonElementFactory::new();
            factory.set_ribbon(Some(ribbon.clone()));
            let selector = Rc::new(Selector {
                template: DataTemplate::new(),
                trace: RefCell::new(vec![]),
            });
            factory.set_content_template_selector(Some(selector.clone()));
            let mut values = vec![("selector".to_string(), selector.template.clone())];
            let mut key = Some("key");
            let calls = Rc::new(Cell::new(0));
            let root = ribbon.resources();
            match expected["name"].as_str().unwrap() {
                "precedence" => {
                    let mask = expected["mask"].as_u64().unwrap();
                    let first = ResourceDictionary::new();
                    let last = ResourceDictionary::new();
                    root.add_merged_dictionary(first.clone());
                    root.add_merged_dictionary(last.clone());
                    if mask & 1 != 0 {
                        root.add("key", named(&mut values, "own"));
                    }
                    if mask & 2 != 0 {
                        first.add("key", named(&mut values, "merged-first"));
                    }
                    if mask & 4 != 0 {
                        last.add("key", named(&mut values, "merged-last"));
                    }
                    if mask & 8 != 0 {
                        put_theme(&root, "Dark", named(&mut values, "theme"));
                    }
                    if mask & 16 != 0 {
                        app.resources()
                            .add("key", named(&mut values, "application"));
                    }
                    if mask & 32 != 0 {
                        app.styles()
                            .resources()
                            .add("key", named(&mut values, "system"));
                    }
                }
                "wrong_local" | "null_local" => {
                    root.add(
                        "key",
                        if expected["name"] == "wrong_local" {
                            Some(Rc::new("wrong type".to_string()))
                        } else {
                            None
                        },
                    );
                    app.resources()
                        .add("key", named(&mut values, "application"));
                    app.styles()
                        .resources()
                        .add("key", named(&mut values, "system"));
                }
                "wrong_system" | "null_system" => {
                    app.styles().resources().add(
                        "key",
                        if expected["name"] == "wrong_system" {
                            Some(Rc::new("wrong type".to_string()))
                        } else {
                            None
                        },
                    );
                    app.resources()
                        .add("key", named(&mut values, "application"));
                }
                "wrong_application" => app
                    .resources()
                    .add("key", Some(Rc::new("wrong type".to_string()))),
                "missing" => {}
                "no_ribbon" => {
                    factory.set_ribbon(None);
                    app.resources()
                        .add("key", named(&mut values, "application"));
                    app.styles()
                        .resources()
                        .add("key", named(&mut values, "system"));
                }
                "no_key" => {
                    root.add("key", named(&mut values, "own"));
                    key = None;
                }
                "empty_key" | "unicode_key" => {
                    key = Some(if expected["name"] == "empty_key" {
                        ""
                    } else {
                        "𝔸é"
                    });
                    root.add(key.unwrap(), named(&mut values, "own"));
                }
                "hc_white" | "hc_black" | "hc_general" | "hc_base" => {
                    let base = if expected["name"] == "hc_white" {
                        ThemeVariant::light()
                    } else {
                        ThemeVariant::dark()
                    };
                    ribbon.set_value(
                        ThemeVariant::requested_theme_variant_property(),
                        Some(ThemeVariant::new("HighContrast", Some(base))),
                    );
                    put_theme(&root, "Default", named(&mut values, "default"));
                    put_theme(&root, "Dark", named(&mut values, "dark"));
                    if expected["name"] != "hc_base" {
                        put_theme(&root, "HighContrast", named(&mut values, "high-contrast"));
                    }
                    if expected["name"] == "hc_white" {
                        put_theme(&root, "HighContrastWhite", named(&mut values, "white"));
                    }
                    if expected["name"] == "hc_black" {
                        put_theme(&root, "HighContrastBlack", named(&mut values, "black"));
                    }
                }
                "rebind_application" => {
                    let next = Application::new();
                    next.resources()
                        .add("key", named(&mut values, "replacement-application"));
                    let calls = calls.clone();
                    root.add_deferred_fn("key", move |_| {
                        calls.set(calls.get() + 1);
                        Application::bind_current(next.clone());
                        Some(Rc::new("wrong type".to_string()))
                    });
                }
                mode => panic!("Unmapped original factory/resource case: {mode}"),
            }
            let chosen =
                factory.find_template(key.map(Into::into), Some(Rc::new("payload".to_string())));
            let name = chosen.as_ref().map(|template| {
                values
                    .iter()
                    .find(|(_, candidate)| Rc::ptr_eq(template, candidate))
                    .expect("factory reconstructed or returned an unknown template")
                    .0
                    .clone()
            });
            assert_eq!(json!(name), expected["chosen"], "{expected}");
            assert_eq!(
                json!(*selector.trace.borrow()),
                expected["selector"],
                "{expected}"
            );
            if expected["name"] == "rebind_application" {
                assert_eq!(json!(calls.get()), expected["calls"]);
            }
        }
    });
}

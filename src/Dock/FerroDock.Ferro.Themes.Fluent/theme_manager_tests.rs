//! Original preset-manager contract and native resource graph scenarios.
use crate::{DockFluentTheme, DockFluentThemeManager};
use ferrodock_ferro::themes::{
    DockPresetThemeManagerBase, DockPresetThemeManagerHooks, IDockThemeManager,
};
use ferrodock_model::collections::ReadOnlyListHandle;
use ferroui_base::{
    collections::FerroList,
    controls::{
        IResourceDictionary, IResourceNode, IResourceProvider, IThemeVariantProvider,
        ResourceHostRef, ResourceKey, ResourceValue,
    },
    reactive::IDisposable,
    styling::{Style, Styles, ThemeVariant},
    threading::CancellationToken,
    utilities::Uri,
};
use ferroui_controls::{AppBuilder, Application};
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, FerroTestApplicationAttribute,
    FerroTestAssembly, HeadlessUnitTestSession,
};
use ferroui_markup_xaml::styling::ResourceInclude;
use std::{any::Any, rc::Rc};
fn build() -> AppBuilder {
    AppBuilder::configure::<Application>().use_headless(FerroHeadlessPlatformOptions::default())
}
const ASSEMBLY: FerroTestAssembly = FerroTestAssembly {
    name: "original-preset-managers",
    test_application: Some(FerroTestApplicationAttribute::new(build)),
    test_isolation: None,
};
fn run(test: impl FnOnce() + Send + 'static) {
    HeadlessUnitTestSession::get_or_start_for_assembly(Some(&ASSEMBLY))
        .dispatch(
            move || {
                crate::register_types();
                let app = Application::current().unwrap();
                app.styles().clear();
                app.resources().merged_dictionaries().clear();
                app.set_requested_theme_variant(None);
                test();
            },
            CancellationToken::none(),
        )
        .wait()
        .unwrap();
}
fn uri(name: &str) -> Uri {
    Uri::absolute(&format!(
        "ferres://FerroDock.Ferro.Themes.Fluent/Presets/Ide/{name}.xaml"
    ))
    .unwrap()
}
fn include(name: &str) -> Rc<ResourceInclude> {
    let uri = uri(name);
    let include = ResourceInclude::new(Some(uri.clone()));
    include.set_source(Some(uri));
    include
}
fn sources(dictionary: &dyn IResourceDictionary) -> Vec<String> {
    dictionary
        .merged_dictionaries()
        .snapshot()
        .iter()
        .filter_map(|provider| {
            (&**provider as &dyn Any)
                .downcast_ref::<ResourceInclude>()
                .and_then(ResourceInclude::source)
                .map(|uri| uri.to_string())
        })
        .collect()
}
struct CustomDictionary {
    inner: Rc<dyn IResourceDictionary>,
}
impl CustomDictionary {
    fn new() -> Rc<Self> {
        Rc::new(Self {
            inner: ferroui_base::controls::ResourceDictionary::new().into(),
        })
    }
}
impl IResourceNode for CustomDictionary {
    fn has_resources(&self) -> bool {
        self.inner.has_resources()
    }
    fn try_get_resource(
        &self,
        key: &ResourceKey,
        theme: Option<&ThemeVariant>,
    ) -> Option<ResourceValue> {
        self.inner.try_get_resource(key, theme)
    }
}
impl IResourceProvider for CustomDictionary {
    fn owner(&self) -> Option<ResourceHostRef> {
        self.inner.owner()
    }
    fn owner_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        self.inner.owner_changed(handler)
    }
    fn add_owner(&self, owner: &ResourceHostRef) {
        self.inner.add_owner(owner)
    }
    fn remove_owner(&self, owner: &ResourceHostRef) {
        self.inner.remove_owner(owner)
    }
    fn as_resource_dictionary(&self) -> Option<&dyn IResourceDictionary> {
        Some(self)
    }
}
impl IResourceDictionary for CustomDictionary {
    fn merged_dictionaries(&self) -> FerroList<Rc<dyn IResourceProvider>> {
        self.inner.merged_dictionaries()
    }
    fn merged_dictionaries_snapshot(&self) -> Rc<Vec<Rc<dyn IResourceProvider>>> {
        self.inner.merged_dictionaries_snapshot()
    }
    fn theme_dictionaries_snapshot(&self) -> Vec<(ThemeVariant, Rc<dyn IThemeVariantProvider>)> {
        self.inner.theme_dictionaries_snapshot()
    }
    fn count(&self) -> usize {
        self.inner.count()
    }
    fn add(&self, key: ResourceKey, value: ResourceValue) {
        self.inner.add(key, value)
    }
    fn set(&self, key: ResourceKey, value: ResourceValue) {
        self.inner.set(key, value)
    }
    fn contains_key(&self, key: &ResourceKey) -> bool {
        self.inner.contains_key(key)
    }
    fn remove(&self, key: &ResourceKey) -> bool {
        self.inner.remove(key)
    }
    fn try_get_value(&self, key: &ResourceKey) -> Option<ResourceValue> {
        self.inner.try_get_value(key)
    }
    fn clear(&self) {
        self.inner.clear()
    }
}
#[test]
fn constructor_validation_order_source_lists_and_absent_application_behavior() {
    let names = || ReadOnlyListHandle::from_items(vec!["Default".into()]);
    let uris = || ReadOnlyListHandle::from_items(vec![uri("Default")]);
    let fail = |n, u, p| DockPresetThemeManagerBase::new(n, u, p).err().unwrap();
    assert_eq!(fail(None, None, None).parameter, Some("presetNames"));
    assert_eq!(
        fail(Some(names()), None, None).parameter,
        Some("presetUris")
    );
    for prefix in [None, Some("".into()), Some("\u{0085}\u{3000}".into())] {
        let error = fail(Some(names()), Some(uris()), prefix);
        assert_eq!(error.parameter, Some("presetUriPrefix"));
        assert_eq!(error.message, "Preset URI prefix cannot be empty.");
    }
    assert_eq!(
        fail(
            Some(ReadOnlyListHandle::from_items(vec![])),
            Some(uris()),
            Some("prefix".into())
        )
        .parameter,
        Some("presetNames")
    );
    assert_eq!(
        fail(
            Some(names()),
            Some(ReadOnlyListHandle::from_items(vec![])),
            Some("prefix".into())
        )
        .parameter,
        Some("presetUris")
    );
    assert_eq!(
        fail(
            Some(names()),
            Some(ReadOnlyListHandle::from_items(vec![
                uri("Default"),
                uri("RiderDark")
            ])),
            Some("prefix".into())
        )
        .message,
        "Preset names and preset URIs must have the same count."
    );
    let supplied = names();
    let base = DockPresetThemeManagerBase::new(
        Some(supplied.clone()),
        Some(uris()),
        Some("prefix".into()),
    )
    .unwrap();
    assert_eq!(base.preset_names().identity(), supplied.identity());
    assert!(Application::current().is_none());
    base.switch(1);
    assert_eq!(base.current_preset_index(), 0);
    let first = DockFluentThemeManager::new();
    let second = DockFluentThemeManager::new();
    assert_eq!(
        first.preset_names().identity(),
        second.preset_names().identity()
    );
    assert_eq!(
        first.preset_names().iter().collect::<Vec<_>>(),
        [
            "Default",
            "VS Code Light",
            "VS Code Dark",
            "Rider Light",
            "Rider Dark"
        ]
    );
    first.switch_preset(4);
    assert_eq!(first.current_preset_index(), 0);
}
#[test]
fn source_switch_removes_every_matching_include_through_styles_and_custom_dictionaries() {
    run(|| {
        let app = Application::current().unwrap();
        let root: Rc<dyn IResourceDictionary> = app.resources().into();
        let theme = DockFluentTheme::new();
        let theme_dictionary: Rc<dyn IResourceDictionary> = theme.resources().into();
        app.styles().add(theme.as_style());
        let parent = Styles::new();
        let nested = Styles::new();
        let nested_dictionary: Rc<dyn IResourceDictionary> = nested.resources().into();
        parent.add(nested.clone());
        app.styles().add(parent);
        let custom = CustomDictionary::new();
        root.merged_dictionaries()
            .add(custom.clone() as Rc<dyn IResourceProvider>);
        for dictionary in [
            &*root,
            &*theme_dictionary,
            &*nested_dictionary,
            &*custom as &dyn IResourceDictionary,
        ] {
            dictionary
                .merged_dictionaries()
                .add(include("VsCodeLight").as_resource_provider());
            dictionary
                .merged_dictionaries()
                .add(include("RiderLight").as_resource_provider());
        }
        let unrelated = include("Default");
        unrelated.loaded();
        unrelated.set_source(Some(
            Uri::absolute("ferres://Other/Presets/Ide/Default.xaml").unwrap(),
        ));
        root.merged_dictionaries()
            .add(unrelated.as_resource_provider());
        let unset = include("Default");
        unset.loaded();
        unset.set_source(None);
        root.merged_dictionaries().add(unset.as_resource_provider());
        let plain = Style::new();
        let plain_dictionary: Rc<dyn IResourceDictionary> = plain.resources().into();
        plain_dictionary
            .merged_dictionaries()
            .add(include("VsCodeLight").as_resource_provider());
        app.styles().add(plain);
        let manager = DockFluentThemeManager::new();
        assert_eq!(manager.current_preset_index(), 1);
        manager.switch_preset(4);
        assert_eq!(manager.current_preset_index(), 4);
        assert_eq!(
            sources(&*root),
            [
                Uri::absolute("ferres://Other/Presets/Ide/Default.xaml")
                    .unwrap()
                    .to_string(),
                uri("RiderDark").to_string()
            ]
        );
        assert_eq!(
            sources(&*theme_dictionary)
                .iter()
                .filter(|s| s.contains("/Presets/Ide/"))
                .cloned()
                .collect::<Vec<_>>(),
            [uri("RiderDark").to_string()]
        );
        assert!(sources(&*nested_dictionary).is_empty());
        assert!(sources(&*custom).is_empty());
        assert_eq!(
            sources(&*plain_dictionary),
            [uri("VsCodeLight").to_string()],
            "the original traverses Styles only, not a plain Style's dictionary"
        );
        assert!(root
            .merged_dictionaries()
            .snapshot()
            .iter()
            .any(|provider| (&**provider as &dyn Any)
                .downcast_ref::<ResourceInclude>()
                .is_some_and(|include| std::ptr::eq(include, &*unrelated))));
        let stable = root.merged_dictionaries_snapshot();
        let theme_stable = theme_dictionary.merged_dictionaries_snapshot();
        manager.switch_preset(4);
        manager.switch_preset(-1);
        manager.switch_preset(5);
        assert!(Rc::ptr_eq(&stable, &root.merged_dictionaries_snapshot()));
        assert!(Rc::ptr_eq(
            &theme_stable,
            &theme_dictionary.merged_dictionaries_snapshot()
        ));
        manager.switch(0);
        assert_eq!(app.requested_theme_variant(), Some(ThemeVariant::light()));
        manager.switch(1);
        assert_eq!(app.requested_theme_variant(), Some(ThemeVariant::dark()));
        manager.switch(-1);
        assert_eq!(app.requested_theme_variant(), Some(ThemeVariant::dark()));
    });
}
struct CustomManager {
    base: DockPresetThemeManagerBase,
    owner: Rc<dyn IResourceDictionary>,
}
impl DockPresetThemeManagerHooks for CustomManager {
    fn base(&self) -> &DockPresetThemeManagerBase {
        &self.base
    }
    fn try_get_default_preset_owner(&self, _: &Application) -> Option<Rc<dyn IResourceDictionary>> {
        Some(self.owner.clone())
    }
}
#[test]
fn first_match_case_unknown_uri_and_default_owner_identity_follow_the_original_algorithm() {
    run(|| {
        let app = Application::current().unwrap();
        let root: Rc<dyn IResourceDictionary> = app.resources().into();
        let custom = CustomDictionary::new();
        root.merged_dictionaries()
            .add(custom.clone() as Rc<dyn IResourceProvider>);
        let source = include("RiderDark");
        source.loaded();
        source.set_source(Some(
            Uri::absolute("FERRES://FERRODOCK.FERRO.THEMES.FLUENT/PRESETS/IDE/RIDERDARK.XAML")
                .unwrap(),
        ));
        custom
            .merged_dictionaries()
            .add(source.as_resource_provider());
        let manager = DockFluentThemeManager::new();
        assert_eq!(manager.current_preset_index(), 4);
        let stable = root.merged_dictionaries_snapshot();
        manager.switch_preset(4);
        assert!(Rc::ptr_eq(&stable, &root.merged_dictionaries_snapshot()));
        source.set_source(Some(
            Uri::absolute("ferres://FerroDock.Ferro.Themes.Fluent/Presets/Ide/Unknown.xaml")
                .unwrap(),
        ));
        assert_eq!(manager.current_preset_index(), 0);
        let owned = CustomManager {
            base: DockPresetThemeManagerBase::new(
                Some(ReadOnlyListHandle::from_items(vec!["Default".into()])),
                Some(ReadOnlyListHandle::from_items(vec![uri("Default")])),
                Some("ferres://FerroDock.Ferro.Themes.Fluent/Presets/Ide/".into()),
            )
            .unwrap(),
            owner: custom.clone(),
        };
        owned.switch_preset(0);
        assert_eq!(sources(&*custom), [uri("Default").to_string()]);
        assert_eq!(sources(&*root), [uri("Default").to_string()]);
        root.merged_dictionaries().clear();
        let owned = CustomManager {
            base: DockPresetThemeManagerBase::new(
                Some(ReadOnlyListHandle::from_items(vec!["Default".into()])),
                Some(ReadOnlyListHandle::from_items(vec![uri("Default")])),
                Some("ferres://FerroDock.Ferro.Themes.Fluent/Presets/Ide/".into()),
            )
            .unwrap(),
            owner: app.resources().into(),
        };
        owned.switch_preset(0);
        assert_eq!(
            sources(&*root),
            [uri("Default").to_string()],
            "the default owner is the application dictionary, so the source adds only once"
        );
    });
}

#[repr(C)]
struct DerivedTheme {
    base: DockFluentTheme,
}
ferroui_base::ferro_class!(DerivedTheme:DockFluentTheme);
ferroui_base::ferro_class_info!(DerivedTheme {});
ferroui_base::ferro_impl_classes!(DerivedTheme:ferroui_base::FerroObjectImpl);
#[test]
fn source_theme_constructor_loads_resources_for_native_derived_instances() {
    run(|| {
        let theme = ferroui_base::instantiate(DerivedTheme {
            base: DockFluentTheme::construct(),
        });
        assert_eq!(theme.resources().merged_dictionaries().count(), 50);
        theme.set_density_style(ferrodock_ferro::themes::DockDensityStyle::Compact);
        assert!(theme
            .try_get_resource(
                &ferroui_base::controls::ResourceKey::from("DockTabItemMinHeight"),
                None
            )
            .is_some());
        theme.set_cache_document_tab_content(true);
        assert_eq!(theme.count(), 2);
    });
}

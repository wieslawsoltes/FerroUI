//! Shared helpers of the tests: the application scope, loading a document
//! of the theme on its own and realising everything a loaded document
//! holds. As the helpers of the tests of the Fluent theme
//! (`src/FerroUI.Themes.Fluent/tests/support.rs`), for the macOS theme.

use crate::{assets, register_types, MacOSTheme, ASSEMBLY};
use ferroui_base::controls::{IResourceProvider, ResourceDictionary, ResourceKey};
use ferroui_base::metadata::{from_markup_value, MarkupAttribute, MarkupAttributeValue};
use ferroui_base::styling::{ControlTheme, IStyle, Style, StyleBase, Styles, ThemeVariant};
use ferroui_base::utilities::Uri;
use ferroui_base::{BoxedValue, FerroObject, Ref, TypeInfo};
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_markup_xaml::{RuntimeXamlLoaderConfiguration, RuntimeXamlLoaderDocument, XamlLoadException};
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;
use std::rc::Rc;

/// Starts a unit test application with the services of a styled window but
/// without a theme, and with the run-time loader registered (includes of a
/// document that is loaded on its own are resolved through it).
pub fn start_application() -> UnitTestApplicationScope {
    register_types();
    let mut services = TestServices::styled_window();
    services.theme = None;
    let scope = UnitTestApplication::start(services);
    FerroRuntimeXamlLoader::register();
    scope
}

/// Starts a unit test application with the services of a styled window and
/// the macOS theme as the theme of the application.
pub fn start_themed_application() -> UnitTestApplicationScope {
    register_types();
    let services = TestServices::styled_window()
        .with_global_clock(Rc::new(MockGlobalClock::new()))
        .with_theme(|| MacOSTheme::new().as_style());
    let scope = UnitTestApplication::start(services);
    FerroRuntimeXamlLoader::register();
    scope
}

/// The global clock of the themed application: the transitions of the control themes need one.
/// It never pulses, so a transition keeps its start value.
pub struct MockGlobalClock {
    subject: ferroui_base::reactive::LightweightSubject<ferroui_base::animation::TimeSpan>,
    play_state: std::cell::Cell<ferroui_base::animation::PlayState>,
}

impl MockGlobalClock {
    pub fn new() -> Self {
        Self {
            subject: ferroui_base::reactive::LightweightSubject::new(),
            play_state: std::cell::Cell::new(ferroui_base::animation::PlayState::Run),
        }
    }

    /// A deterministic frame for behavioral motion tests, without wall-clock waits.
    pub fn pulse(&self, seconds: f64) {
        ferroui_base::reactive::IObserver::on_next(&self.subject, ferroui_base::animation::TimeSpan::from_seconds(seconds));
    }

    pub fn has_observers(&self) -> bool {
        self.subject.has_observers()
    }
}

impl ferroui_base::reactive::IObservable<ferroui_base::animation::TimeSpan> for MockGlobalClock {
    fn subscribe(
        &self,
        observer: Rc<dyn ferroui_base::reactive::IObserver<ferroui_base::animation::TimeSpan>>,
    ) -> Rc<dyn ferroui_base::reactive::IDisposable> {
        self.subject.subscribe(observer)
    }
}

impl ferroui_base::animation::IClock for MockGlobalClock {
    fn play_state(&self) -> ferroui_base::animation::PlayState {
        self.play_state.get()
    }

    fn set_play_state(&self, value: ferroui_base::animation::PlayState) {
        self.play_state.set(value)
    }
}

impl ferroui_base::animation::IGlobalClock for MockGlobalClock {}

/// A load error as text, with the error of the compiler it carries.
pub fn describe(error: &XamlLoadException) -> String {
    match error.inner_exception() {
        Some(inner) => format!("{}\n ---> {inner}", error.message()),
        None => error.message().to_string(),
    }
}

/// Loads the embedded document with the rooted asset path `path` on its
/// own.
pub fn try_load_document(path: &str) -> Result<BoxedValue, XamlLoadException> {
    register_types();
    let content = assets::document(path).unwrap_or_else(|| panic!("{path} is not a document of the theme"));
    let text = std::str::from_utf8(content).unwrap_or_else(|e| panic!("{path} is not UTF-8: {e}"));
    let uri = Uri::absolute(&format!("ferres://{}{path}", ASSEMBLY.name)).expect("a valid URI");
    let mut document = RuntimeXamlLoaderDocument::with_base_uri(Some(uri), text);
    document.document = Some(path.trim_start_matches('/').to_string());
    let mut configuration = RuntimeXamlLoaderConfiguration::new();
    configuration.local_assembly = Some(&ASSEMBLY);
    FerroRuntimeXamlLoader::load_document(document, Some(configuration))
}

/// Loads markup text with the run-time loader.
pub fn try_load_text(xaml: &str) -> Result<BoxedValue, XamlLoadException> {
    register_types();
    let mut configuration = RuntimeXamlLoaderConfiguration::new();
    configuration.local_assembly = Some(&ASSEMBLY);
    FerroRuntimeXamlLoader::load_document(RuntimeXamlLoaderDocument::new(xaml), Some(configuration))
}

/// Loads markup text; a failure panics with the described error.
#[track_caller]
pub fn load_text(xaml: &str) -> BoxedValue {
    match try_load_text(xaml) {
        Ok(value) => value,
        Err(error) => panic!("the document failed to load: {}", describe(&error)),
    }
}

/// What realising a loaded document found.
#[derive(Default)]
pub struct Realised {
    /// The number of resources (of all dictionaries, nested ones included).
    pub resources: usize,
}

/// Realises every (deferred) resource a loaded object holds, recursively:
/// the resources of dictionaries, of their merged and theme dictionaries
/// and of styles and control themes.
pub fn realise(value: &BoxedValue) -> Realised {
    let mut realised = Realised::default();
    realise_value(value, &mut realised);
    realised
}

fn realise_value(value: &BoxedValue, realised: &mut Realised) {
    let value = Some(value.clone());
    if let Some(dictionary) = from_markup_value::<Ref<ResourceDictionary>>(&value) {
        realise_dictionary(&dictionary, realised);
    } else if let Some(theme) = from_markup_value::<Ref<ControlTheme>>(&value) {
        realise_object(&theme, realised);
    } else if let Some(style) = from_markup_value::<Ref<Style>>(&value) {
        realise_object(&style, realised);
    } else if let Some(styles) = from_markup_value::<Ref<Styles>>(&value) {
        realise_object(&styles, realised);
    }
}

fn realise_object(object: &FerroObject, realised: &mut Realised) {
    if let Some(dictionary) = object.downcast_ref::<ResourceDictionary>() {
        realise_dictionary(&dictionary.to_ref(), realised);
    } else if let Some(theme) = object.downcast_ref::<ControlTheme>() {
        realise_dictionary(&theme.resources(), realised);
        for child in theme.children().snapshot().iter() {
            realise_style(child, realised);
        }
    } else if let Some(style) = object.downcast_ref::<Style>() {
        realise_dictionary(&style.resources(), realised);
        for child in style.children().snapshot().iter() {
            realise_style(child, realised);
        }
    } else if let Some(styles) = object.downcast_ref::<Styles>() {
        realise_dictionary(&styles.resources(), realised);
        for child in styles.snapshot().iter() {
            realise_style(child, realised);
        }
    }
}

fn realise_style(style: &Rc<dyn IStyle>, realised: &mut Realised) {
    if let Some(object) = style.as_object() {
        realise_object(object, realised);
    }
}

fn realise_provider(provider: &Rc<dyn IResourceProvider>, realised: &mut Realised) {
    if let Some(object) = provider.as_object() {
        realise_object(object, realised);
    }
}

fn realise_dictionary(dictionary: &Ref<ResourceDictionary>, realised: &mut Realised) {
    for key in dictionary.keys() {
        realised.resources += 1;
        if let Some(Some(value)) = dictionary.try_get_value(&key) {
            realise_value(&value, realised);
        }
    }
    for merged in dictionary.merged_dictionaries().snapshot().iter() {
        realise_provider(merged, realised);
    }
    for (_, provider) in dictionary.theme_dictionaries_snapshot() {
        if let Some(object) = provider.as_object() {
            realise_object(object, realised);
        }
    }
}

/// The theme, created with a root service provider and added to the styles
/// of the application (`ThemeTestBase.CreateAttachedTheme()` of the theme
/// tests of the upstream project).
pub fn create_attached_theme() -> Ref<MacOSTheme> {
    let application = ferroui_controls::Application::current().expect("Application.Current should not be null.");
    let sp = ferroui_markup_xaml::xaml_il::runtime::XamlIlRuntimeHelpers::create_root_service_provider_v2();
    let theme = MacOSTheme::with_service_provider(Some(sp));
    application.styles().add(theme.as_style());
    theme
}

/// The three theme variants of the theme.
pub fn variants() -> [ThemeVariant; 3] {
    [ThemeVariant::light(), ThemeVariant::dark(), MacOSTheme::high_contrast()]
}

/// A resource found by [`enumerate_resources`].
pub struct ResourceEntry {
    pub key: ResourceKey,
    pub value: Option<BoxedValue>,
    pub theme_variant: ThemeVariant,
}

impl ResourceEntry {
    /// The value as a control theme.
    pub fn control_theme(&self) -> Option<Ref<ControlTheme>> {
        from_markup_value::<Ref<ControlTheme>>(&self.value)
    }

    /// The key as a type.
    pub fn type_key(&self) -> Option<&'static TypeInfo> {
        match &self.key {
            ResourceKey::Type(type_) => Some(type_),
            _ => None,
        }
    }
}

/// The resources of a style and of its child styles, with those of the
/// theme and merged dictionaries (`ResourceExtensions.EnumerateResources`).
pub fn enumerate_resources(style: &Rc<dyn IStyle>) -> Vec<ResourceEntry> {
    let mut entries = Vec::new();
    enumerate_style_resources(style, &mut entries);
    entries
}

fn enumerate_style_resources(style: &Rc<dyn IStyle>, entries: &mut Vec<ResourceEntry>) {
    if let Some(object) = style.as_object() {
        if let Some(styles) = object.downcast_ref::<Styles>() {
            enumerate_dictionary_resources(&styles.resources(), None, entries);
        } else if let Some(style_base) = object.downcast_ref::<StyleBase>() {
            enumerate_dictionary_resources(&style_base.resources(), None, entries);
        }
    }
    for child in style.children().iter() {
        enumerate_style_resources(child, entries);
    }
}

/// The resources of a dictionary, with those of its theme and merged
/// dictionaries.
pub fn enumerate_dictionary_resources(
    dictionary: &Ref<ResourceDictionary>,
    theme_variant: Option<ThemeVariant>,
    entries: &mut Vec<ResourceEntry>,
) {
    for key in dictionary.keys() {
        let value = dictionary.try_get_resource(&key, theme_variant.as_ref());
        assert!(value.is_some(), "Key \"{key:?}\" defined, but value was not found");
        entries.push(ResourceEntry {
            key,
            value: value.flatten(),
            theme_variant: theme_variant.clone().unwrap_or_else(ThemeVariant::default),
        });
    }
    for (variant, provider) in dictionary.theme_dictionaries_snapshot() {
        if let Some(dictionary) = provider.as_object().and_then(|object| object.downcast_ref::<ResourceDictionary>()) {
            enumerate_dictionary_resources(&dictionary.to_ref(), Some(variant), entries);
        }
    }
    for merged in dictionary.merged_dictionaries().snapshot().iter() {
        if let Some(dictionary) = merged.as_object().and_then(|object| object.downcast_ref::<ResourceDictionary>()) {
            enumerate_dictionary_resources(&dictionary.to_ref(), theme_variant.clone(), entries);
        } else if let Some(keys) = try_get_known_resource_keys(merged) {
            for key in keys {
                let key = ResourceKey::from(key);
                let value = merged.try_get_resource(&key, theme_variant.as_ref());
                assert!(value.is_some(), "Key \"{key:?}\" defined, but value was not found");
                entries.push(ResourceEntry {
                    key,
                    value: value.flatten(),
                    theme_variant: theme_variant.clone().unwrap_or_else(ThemeVariant::default),
                });
            }
        }
    }
}

/// The keys of the resource providers of the theme that compute their
/// resources and have no keys collection to read: the accent colours.
fn try_get_known_resource_keys(provider: &Rc<dyn IResourceProvider>) -> Option<Vec<&'static str>> {
    const ACCENT_KEYS: [&str; 7] = [
        "SystemAccentColor",
        "SystemAccentColorDark1",
        "SystemAccentColorDark2",
        "SystemAccentColorDark3",
        "SystemAccentColorLight1",
        "SystemAccentColorLight2",
        "SystemAccentColorLight3",
    ];
    let object = provider.as_object()?;
    if object.downcast_ref::<crate::accents::SystemAccentColors>().is_some() {
        return Some(ACCENT_KEYS.to_vec());
    }
    None
}

/// A style (when it is a style and not a collection of styles) and its
/// nested styles (`StyleExtensions.EnumerateStyles`).
pub fn enumerate_styles(style: &Rc<dyn IStyle>) -> Vec<Ref<StyleBase>> {
    let mut styles = Vec::new();
    enumerate_styles_into(style, &mut styles);
    styles
}

fn enumerate_styles_into(style: &Rc<dyn IStyle>, styles: &mut Vec<Ref<StyleBase>>) {
    if let Some(style_base) = style.as_object().and_then(|o| o.downcast_ref::<StyleBase>()) {
        styles.push(style_base.to_ref());
    }
    for child in style.children().iter() {
        enumerate_styles_into(child, styles);
    }
}

/// The template parts a class declares itself (`inherit: false`) and the
/// required ones of the classes it derives from.
pub fn requested_parts(target_type: &'static TypeInfo) -> Vec<&'static MarkupAttribute> {
    fn parts(type_: &'static TypeInfo) -> impl Iterator<Item = &'static MarkupAttribute> {
        type_.markup().into_iter().flat_map(|markup| markup.attributes.iter()).filter(|a| a.name == "TemplatePart")
    }
    fn part_name(attribute: &MarkupAttribute) -> &'static str {
        match attribute.arguments.first() {
            Some(MarkupAttributeValue::Str(name)) => name,
            _ => panic!("a template part without a name"),
        }
    }

    let mut requested: Vec<&'static MarkupAttribute> = parts(target_type).collect();
    let mut base = target_type.base_type();
    while let Some(type_) = base {
        for part in parts(type_) {
            let required = matches!(part.property("IsRequired"), Some(MarkupAttributeValue::Bool(true)));
            if required && !requested.iter().any(|known| part_name(known) == part_name(part)) {
                requested.push(part);
            }
        }
        base = type_.base_type();
    }
    requested
}

/// Whether the class of `object`, or a class it derives from, is the class
/// whose handle type is named `handle_type` (`..::Ref<module::Class>`). A
/// part declared with a contract instead of a class is accepted as it is.
pub fn is_instance_of(object: &Ref<FerroObject>, handle_type: &str) -> bool {
    if !handle_type.contains("Ref<") {
        return true;
    }
    let mut current = Some(object.get_type());
    while let Some(type_) = current {
        if handle_type.ends_with(&format!("Ref<{}::{}>", type_.module_path(), type_.name())) {
            return true;
        }
        current = type_.base_type();
    }
    false
}

/// The control themes of the WinUI theme that are not ported yet, by the
/// name of the class they style: the controls the Fluent theme styles and
/// this theme does not, emptied batch by batch (docs/porting/winui-theme.md,
/// section 7). A test that needs every control themed reports these and
/// does not fail on them; it fails when a control of the list has a theme,
/// so that the list is kept current.
pub const PENDING_CONTROLS: &[&str] = &[
];

/// Whether the control named `name` waits for a batch.
pub fn is_pending(name: &str) -> bool {
    PENDING_CONTROLS.contains(&name)
}

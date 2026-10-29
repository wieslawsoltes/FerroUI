//! The type table of this crate: its namespaces, its classes, what it states about itself for
//! markup, its embedded assets and the loader table of its compiled markup.
//!
//! The shared modules (the views, the view models, the controls, ...) list their types here; a
//! page lists its own types, and the types only it uses, in its entry of the page table
//! ([`crate::pages::PageInfo`]), so that porting a page touches no list of this file.

use crate::markup::XamlClass;
use crate::pages::{PAGES, PAGE_TABLE};
use crate::{
    animations, behaviors, controls, converters, dialogs, models, renderers, triggers, view_models,
    views, App,
};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::{MarkupAssembly, MarkupType, XmlnsDefinition, FERRO_XML_NAMESPACE};
use ferroui_base::TypeInfo;

/// The dotted namespaces of the modules of this crate (the namespaces of the reference sample,
/// with the name of the gallery). A type belongs to the namespace of the longest module path
/// that is a prefix of the path of its declaring module.
const NAMESPACES: &[(&str, &str)] = &[
    ("behaviors_gallery", "BehaviorsGallery"),
    (
        "behaviors_gallery::animations",
        "BehaviorsGallery.Animations",
    ),
    ("behaviors_gallery::behaviors", "BehaviorsGallery.Behaviors"),
    ("behaviors_gallery::controls", "BehaviorsGallery.Controls"),
    (
        "behaviors_gallery::converters",
        "BehaviorsGallery.Converters",
    ),
    ("behaviors_gallery::dialogs", "BehaviorsGallery.Dialogs"),
    ("behaviors_gallery::models", "BehaviorsGallery.Models"),
    ("behaviors_gallery::pages", "BehaviorsGallery.Views.Pages"),
    ("behaviors_gallery::renderers", "BehaviorsGallery.Renderers"),
    ("behaviors_gallery::triggers", "BehaviorsGallery.Triggers"),
    (
        "behaviors_gallery::view_models",
        "BehaviorsGallery.ViewModels",
    ),
    (
        "behaviors_gallery::view_models::reactive_ui",
        "BehaviorsGallery.ViewModels.ReactiveUI",
    ),
    ("behaviors_gallery::views", "BehaviorsGallery.Views"),
    (
        "behaviors_gallery::views::windows",
        "BehaviorsGallery.Views.Windows",
    ),
];

/// What this crate states about itself for markup: the namespaces of the gallery, of its controls,
/// of its converters and of its behaviors are in the default XML namespace of the framework, as the
/// reference sample states for its own (`Properties/AssemblyInfo.cs`), so that documents name them
/// without a prefix.
pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "BehaviorsGallery",
    crate_name: "behaviors_gallery",
    xmlns_definitions: &[
        XmlnsDefinition {
            xml_namespace: FERRO_XML_NAMESPACE,
            namespace: "BehaviorsGallery",
        },
        XmlnsDefinition {
            xml_namespace: FERRO_XML_NAMESPACE,
            namespace: "BehaviorsGallery.Controls",
        },
        XmlnsDefinition {
            xml_namespace: FERRO_XML_NAMESPACE,
            namespace: "BehaviorsGallery.Converters",
        },
        XmlnsDefinition {
            xml_namespace: FERRO_XML_NAMESPACE,
            namespace: "BehaviorsGallery.Behaviors",
        },
    ],
    xmlns_prefixes: &[],
    metadata: &[],
};

/// The classes of the root namespace `BehaviorsGallery` (`X::TYPE`).
const TYPES: &[&TypeInfo] = &[App::TYPE];

/// The classes of the root namespace that have a document (`&X::XAML_CLASS`).
const ROOT_CLASSES: &[&XamlClass] = &[&App::XAML_CLASS];

/// The classes with a document of the shared namespaces.
const SHARED_CLASSES: [&[&XamlClass]; 4] = [
    ROOT_CLASSES,
    views::CLASSES,
    controls::CLASSES,
    dialogs::CLASSES,
];

/// The number of the lists of the classes with a document: the shared ones and one per page.
const CLASS_LISTS: usize = SHARED_CLASSES.len() + PAGE_TABLE.len();

/// The classes with a document of the gallery: the shared namespaces, then the pages
/// (`SAMPLE.classes`).
pub(crate) static CLASSES: [&[&XamlClass]; CLASS_LISTS] = {
    let mut lists: [&[&XamlClass]; CLASS_LISTS] = [&[]; CLASS_LISTS];
    let mut index = 0;
    while index < SHARED_CLASSES.len() {
        lists[index] = SHARED_CLASSES[index];
        index += 1;
    }
    let mut page = 0;
    while page < PAGE_TABLE.len() {
        lists[index + page] = PAGE_TABLE[page].classes;
        page += 1;
    }
    lists
};

/// Registers the namespaces, the types, the assembly, the embedded assets and the compiled
/// markup of this crate (and of the crates it is built on). Cheap and idempotent.
pub fn register_types() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        // The runtime library of compiled markup (the markup extensions the compiled documents
        // create), which registers the controls; the theme; the behaviors.
        ferroui_markup_xaml::register_types();
        ferroui_themes_fluent::register_types();
        ferroui_reactiveui_controls::register_types();
        ferroui_xaml_interactivity::register_types();
        ferroui_xaml_interactions::register_types();
        ferroui_xaml_interactions_custom::register_types();
        ferroui_xaml_interactions_draganddrop::register_types();
        ferroui_xaml_interactions_draggable::register_types();
        ferroui_xaml_interactions_events::register_types();
        ferroui_xaml_interactions_responsive::register_types();
        ferroui_xaml_interactions_reactiveui::register_types();
        ferroui_xaml_behaviors_animations::register_types();
        TypeInfo::register_namespaces(NAMESPACES);
        for types in [
            TYPES,
            animations::TYPES,
            behaviors::TYPES,
            controls::TYPES,
            converters::TYPES,
            dialogs::TYPES,
            models::TYPES,
            renderers::TYPES,
            triggers::TYPES,
            view_models::TYPES,
            views::TYPES,
        ] {
            TypeInfo::register_all(types);
        }
        for markup_types in [
            animations::MARKUP_TYPES,
            behaviors::MARKUP_TYPES,
            controls::MARKUP_TYPES,
            converters::MARKUP_TYPES,
            dialogs::MARKUP_TYPES,
            models::MARKUP_TYPES,
            renderers::MARKUP_TYPES,
            triggers::MARKUP_TYPES,
            view_models::MARKUP_TYPES,
            views::MARKUP_TYPES,
        ] {
            MarkupType::register_all(markup_types);
        }
        for page in PAGES {
            TypeInfo::register_all(page.types);
            MarkupType::register_all(page.markup_types);
        }
        ValueTypes::register_global(register_value_types);
        MarkupAssembly::register(&ASSEMBLY);
        crate::SAMPLE.register_assets();
        // The loader table of the compiled markup, which the build of the crate generates: a load
        // of a document by its URI creates the class of the document with its constructor.
        crate::compiled_markup::register();
    });
}

/// What the untyped value conversions must know about the types of this crate
/// (`ValueTypes::register_reference::<X>()`, nullable forms, casts to contracts).
fn register_value_types() {
    animations::register_value_types();
    behaviors::register_value_types();
    controls::register_value_types();
    converters::register_value_types();
    dialogs::register_value_types();
    models::register_value_types();
    renderers::register_value_types();
    triggers::register_value_types();
    view_models::register_value_types();
    views::register_value_types();
    for page in PAGES {
        (page.register_value_types)();
    }
}

//! The type table of this crate: its namespaces, its classes, what it
//! states about itself for markup, its embedded assets and the loader of
//! the documents with a class.

use crate::internal::{
    ChildFitter, FileSizeStringConverter, ManagedFileChooserFilterViewModel, ManagedFileChooserItemType,
    ManagedFileChooserItemViewModel, ManagedFileChooserViewModel, ResourceSelectorConverter,
};
use crate::{AboutFerroDialog, ManagedFileChooser, ManagedFileChooserOverwritePrompt};
use ferroui_base::data::converters::IValueConverter;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::{IServiceProvider, MarkupAssembly, MarkupType, MarkupTyped};
use ferroui_base::{BoxedValue, TypeInfo};
use ferroui_controls::primitives::SelectedItemsList;
use ferroui_markup_xaml::{FerroXamlLoader, XamlLoadException};
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;
use std::rc::Rc;

/// The dotted namespaces of the modules of this crate.
const NAMESPACES: &[(&str, &str)] = &[
    ("ferroui_dialogs", "FerroUI.Dialogs"),
    ("ferroui_dialogs::internal", "FerroUI.Dialogs.Internal"),
];

/// What this crate states about itself for markup: its assembly name. The
/// upstream project maps no XML namespace; its documents name it with
/// `using:`.
pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "FerroUI.Dialogs",
    crate_name: "ferroui_dialogs",
    xmlns_definitions: &[],
    xmlns_prefixes: &[],
    metadata: &[],
};

const TYPES: &[&TypeInfo] = &[
    AboutFerroDialog::TYPE,
    ManagedFileChooser::TYPE,
    ManagedFileChooserOverwritePrompt::TYPE,
    // FerroUI.Dialogs.Internal
    ChildFitter::TYPE,
    ResourceSelectorConverter::TYPE,
];

const MARKUP_TYPES: &[&MarkupType] = &[
    <FileSizeStringConverter as MarkupTyped>::MARKUP,
    <ManagedFileChooserFilterViewModel as MarkupTyped>::MARKUP,
    <ManagedFileChooserItemType as MarkupTyped>::MARKUP,
    <ManagedFileChooserItemViewModel as MarkupTyped>::MARKUP,
    <ManagedFileChooserViewModel as MarkupTyped>::MARKUP,
];

// The lists of the view models that are delivered to items source properties.
ferroui_controls::ferro_markup_list!(ItemViewModelList: Rc<ManagedFileChooserItemViewModel>);
ferroui_controls::ferro_markup_list!(FilterViewModelList: Rc<ManagedFileChooserFilterViewModel>);

/// Registers the namespaces, the types, the assembly, the embedded assets
/// and the document loader of this crate (and of the crates it is built
/// on). Cheap and idempotent.
pub fn register_types() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        ferroui_markup_xaml::register_types();
        TypeInfo::register_namespaces(NAMESPACES);
        TypeInfo::register_all(TYPES);
        MarkupType::register_all(MARKUP_TYPES);
        ItemViewModelList::register();
        FilterViewModelList::register();
        ValueTypes::register_global(register_value_types);
        MarkupAssembly::register(&ASSEMBLY);
        crate::assets::register();
        FerroRuntimeXamlLoader::register_class_document(AboutFerroDialog::TYPE, AboutFerroDialog::DOCUMENT_URI);
        FerroXamlLoader::register_compiled_xaml(ASSEMBLY.name, try_load);
    });
}

/// What the untyped value conversions must know about the types of this
/// crate: the view models and converters are reference types, the
/// converter is assignable to its contract.
fn register_value_types() {
    ValueTypes::register_reference::<FileSizeStringConverter>();
    ValueTypes::register_reference::<ManagedFileChooserFilterViewModel>();
    ValueTypes::register_reference::<ManagedFileChooserItemViewModel>();
    ValueTypes::register_reference::<ManagedFileChooserViewModel>();
    ValueTypes::register_cast::<Rc<FileSizeStringConverter>, Rc<dyn IValueConverter>>(|c| c.clone());
    ValueTypes::register_display::<ManagedFileChooserFilterViewModel>();
    ValueTypes::register_nullable::<ManagedFileChooserItemType>();
    ValueTypes::register_nullable::<SelectedItemsList>();
}

/// The loader of the documents of this assembly that have a class: loading
/// the document of a class by URI creates an instance of the class (whose
/// constructor populates it). The other documents are left to the run-time
/// loader.
///
/// This is the table the XAML compiler generates per crate; it is written
/// by hand until the compiler exists.
fn try_load(_service_provider: Option<&Rc<dyn IServiceProvider>>, uri: &str) -> Result<Option<BoxedValue>, XamlLoadException> {
    if uri.eq_ignore_ascii_case(AboutFerroDialog::DOCUMENT_URI) {
        return Ok(Some(Rc::new(AboutFerroDialog::new()) as BoxedValue));
    }
    Ok(None)
}

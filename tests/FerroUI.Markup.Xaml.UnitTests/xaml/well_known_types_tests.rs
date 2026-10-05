//! The diagnostic of the framework language over the real registries: what
//! the compiler looks up by name must be resolvable before any document can
//! be loaded at run time.

use std::rc::Rc;

use ferroui_markup_xaml_loader::compiler_extensions::transformers::FerroXamlIlWellKnownTypes;
use ferroui_markup_xaml_loader::compiler_extensions::FerroXamlIlLanguage;
use ferroui_markup_xaml_loader::FerroXamlIlRuntimeCompiler;
use xamlx::type_system::IXamlTypeSystem;

/// The types the language and the well-known types of the compiler look up
/// by name.
const WELL_KNOWN_TYPE_NAMES: &[&str] = &[
    "FerroUI.Animation.Transitions",
    "FerroUI.AttachedProperty`1",
    "FerroUI.Collections.FerroListConverter`1",
    "FerroUI.Collections.FerroList`1",
    "FerroUI.Controls.Classes",
    "FerroUI.Controls.ColumnDefinition",
    "FerroUI.Controls.ColumnDefinitions",
    "FerroUI.Controls.ContentControl",
    "FerroUI.Controls.Control",
    "FerroUI.Controls.GridLength",
    "FerroUI.Controls.GridUnitType",
    "FerroUI.Controls.IDeferredContent",
    "FerroUI.Controls.INameScope",
    "FerroUI.Controls.IResourceDictionary",
    "FerroUI.Controls.ITemplate`1",
    "FerroUI.Controls.IThemeVariantProvider",
    "FerroUI.Controls.ItemsControl",
    "FerroUI.Controls.NameScope",
    "FerroUI.Controls.ResourceDictionary",
    "FerroUI.Controls.RowDefinition",
    "FerroUI.Controls.RowDefinitions",
    "FerroUI.Controls.Templates.IDataTemplate",
    "FerroUI.Controls.WindowIcon",
    "FerroUI.Controls.WindowTransparencyLevel",
    "FerroUI.CornerRadius",
    "FerroUI.Data.AssignBindingAttribute",
    "FerroUI.Data.BindingBase",
    "FerroUI.Data.BindingExpressionBase",
    "FerroUI.Data.BindingPriority",
    "FerroUI.Data.CompiledBinding",
    "FerroUI.Data.CompiledBindingPath",
    "FerroUI.Data.CompiledBindingPathBuilder",
    "FerroUI.Data.Core.ClrPropertyInfo",
    "FerroUI.Data.Core.ClrPropertyInfo`2",
    "FerroUI.Data.Core.IPropertyInfo",
    "FerroUI.Data.Core.IPropertyInfo`2",
    "FerroUI.Data.Core.Plugins.IPropertyAccessor",
    "FerroUI.Data.MultiBinding",
    "FerroUI.Data.RelativeSource",
    "FerroUI.FerroObject",
    "FerroUI.FerroObjectExtensions",
    "FerroUI.FerroProperty",
    "FerroUI.FerroProperty`1",
    "FerroUI.Input.Cursor",
    "FerroUI.Input.StandardCursorType",
    "FerroUI.Interactivity.Interactive",
    "FerroUI.Interactivity.RoutedEvent",
    "FerroUI.Interactivity.RoutedEventArgs",
    "FerroUI.Markup.Xaml.Converters.BitmapTypeConverter",
    "FerroUI.Markup.Xaml.Converters.FerroUriTypeConverter",
    "FerroUI.Markup.Xaml.Converters.FontFamilyTypeConverter",
    "FerroUI.Markup.Xaml.Converters.IconTypeConverter",
    "FerroUI.Markup.Xaml.Converters.PointsListTypeConverter",
    "FerroUI.Markup.Xaml.Converters.TimeSpanTypeConverter",
    "FerroUI.Markup.Xaml.Diagnostics.XamlSourceInfo",
    "FerroUI.Markup.Xaml.IProvideValueTarget",
    "FerroUI.Markup.Xaml.IRootObjectProvider",
    "FerroUI.Markup.Xaml.IUriContext",
    "FerroUI.Markup.Xaml.MarkupExtensions.CompiledBindingExtension",
    "FerroUI.Markup.Xaml.MarkupExtensions.CompiledBindings.PropertyInfoAccessorFactory",
    "FerroUI.Markup.Xaml.MarkupExtensions.On",
    "FerroUI.Markup.Xaml.MarkupExtensions.ReflectionBindingExtension",
    "FerroUI.Markup.Xaml.MarkupExtensions.ResolveByNameExtension",
    "FerroUI.Markup.Xaml.Styling.MergeResourceInclude",
    "FerroUI.Markup.Xaml.Styling.ResourceInclude",
    "FerroUI.Markup.Xaml.Styling.StyleInclude",
    "FerroUI.Markup.Xaml.Templates.ControlTemplate",
    "FerroUI.Markup.Xaml.Templates.DataTemplate",
    "FerroUI.Markup.Xaml.XamlIl.Runtime.IFerroXamlIlEagerParentStackProvider",
    "FerroUI.Markup.Xaml.XamlIl.Runtime.IFerroXamlIlParentStackProvider",
    "FerroUI.Markup.Xaml.XamlIl.Runtime.IFerroXamlIlXmlNamespaceInfoProvider",
    "FerroUI.Markup.Xaml.XamlIl.Runtime.XamlIlRuntimeHelpers",
    "FerroUI.Matrix",
    "FerroUI.Media.Color",
    "FerroUI.Media.FontFamily",
    "FerroUI.Media.IBrush",
    "FerroUI.Media.IImage",
    "FerroUI.Media.IImageBrushSource",
    "FerroUI.Media.Imaging.Bitmap",
    "FerroUI.Media.Immutable.ImmutableSolidColorBrush",
    "FerroUI.Media.TextDecorationCollection",
    "FerroUI.Media.TextDecorations",
    "FerroUI.Media.TextTrimming",
    "FerroUI.Metadata.ContentAttribute",
    "FerroUI.Metadata.ControlTemplateScopeAttribute",
    "FerroUI.Metadata.DataTypeAttribute",
    "FerroUI.Metadata.DependsOnAttribute",
    "FerroUI.Metadata.FerroListAttribute",
    "FerroUI.Metadata.IAddChild",
    "FerroUI.Metadata.IAddChild`1",
    "FerroUI.Metadata.InheritDataTypeFromAttribute",
    "FerroUI.Metadata.InheritDataTypeFromItemsAttribute",
    "FerroUI.Metadata.MarkupExtensionDefaultOptionAttribute",
    "FerroUI.Metadata.MarkupExtensionOptionAttribute",
    "FerroUI.Metadata.TemplateContentAttribute",
    "FerroUI.Metadata.TrimSurroundingWhitespaceAttribute",
    "FerroUI.Metadata.UsableDuringInitializationAttribute",
    "FerroUI.Metadata.WhitespaceSignificantCollectionAttribute",
    "FerroUI.Metadata.XmlnsDefinitionAttribute",
    "FerroUI.Point",
    "FerroUI.RelativePoint",
    "FerroUI.RelativeUnit",
    "FerroUI.Size",
    "FerroUI.StyledElement",
    "FerroUI.StyledElementExtensions",
    "FerroUI.StyledProperty`1",
    "FerroUI.Styling.ContainerQuery",
    "FerroUI.Styling.ControlTheme",
    "FerroUI.Styling.IStyle",
    "FerroUI.Styling.Selectors",
    "FerroUI.Styling.Setter",
    "FerroUI.Styling.SetterBase",
    "FerroUI.Styling.Style",
    "FerroUI.Styling.StyleQueries",
    "FerroUI.Styling.Styles",
    "FerroUI.Styling.ThemeVariant",
    "FerroUI.Thickness",
    "FerroUI.UnsetValueType",
    "FerroUI.Utilities.TypeUtilities",
    "FerroUI.Vector",
    "System.Collections.Generic.IDictionary`2",
    "System.Collections.Generic.IReadOnlyList`1",
    "System.ComponentModel.CultureInfoConverter",
    "System.ComponentModel.ISupportInitialize",
    "System.EventHandler`1",
    "System.Globalization.CultureInfo",
    "System.IDisposable",
    "System.IObservable`1",
    "System.Int32",
    "System.Int64",
    "System.Threading.Tasks.Task`1",
    "System.TimeSpan",
    "System.UInt32",
    "System.Uri",
    "System.UriKind",
    "System.WeakReference`1",
    "System.Windows.Input.ICommand",
];

/// Reports what the language still cannot resolve over the real registries:
/// every missing type name at once, then the first member the well-known
/// types fail on. Fails while anything is missing.
#[test]
fn the_framework_language_resolves_over_the_registered_metadata() {
    crate::register_types();
    let system = FerroXamlIlRuntimeCompiler::type_system();
    let type_system: Rc<dyn IXamlTypeSystem> = system.as_type_system();
    let missing: Vec<&str> =
        WELL_KNOWN_TYPE_NAMES.iter().copied().filter(|name| type_system.find_type(name).is_none()).collect();
    let language = FerroXamlIlLanguage::configure(&type_system).err().map(|e| e.message());
    let well_known = FerroXamlIlWellKnownTypes::new(&type_system).err().map(|e| e.message());
    assert!(
        missing.is_empty() && language.is_none() && well_known.is_none(),
        "unresolved types: {missing:#?}
language: {language:?}
well-known types: {well_known:?}"
    );
}

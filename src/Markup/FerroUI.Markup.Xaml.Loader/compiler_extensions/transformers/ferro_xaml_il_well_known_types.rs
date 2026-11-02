//! Port of `CompilerExtensions/Transformers/FerroXamlIlWellKnownTypes.cs`.
//!
//! Every framework type, method, constructor and property the compiler extensions refer to is
//! resolved once, by name, when the compiler configuration is created. Field names are the
//! mechanical `snake_case` of the upstream property names (`XamlIlTypes` is `xaml_il_types`,
//! `UInt` is `u_int`, `IBrush` is `i_brush`, `FerroPropertyT` is `ferro_property_t`).

use std::rc::Rc;

use xamlx::exceptions::{XamlError, XamlResult};
use xamlx::transform::{AstTransformationContext, TransformerConfiguration};
use xamlx::type_system::{
    FindMethodMethodSignature, IXamlConstructor, IXamlMethod, IXamlProperty, IXamlType,
    IXamlTypeSystem, XamlTypeWellKnownTypes,
};

use crate::compiler_extensions::group_transformers::AstGroupTransformationContext;

/// The XML namespace of the framework, mapped by the `XmlnsDefinition` attributes of the
/// framework assemblies.
pub use ferroui_base::metadata::FERRO_XML_NAMESPACE;

pub struct FerroXamlIlWellKnownTypes {
    pub runtime_helpers: Rc<dyn IXamlType>,
    pub ferro_object: Rc<dyn IXamlType>,
    pub binding_priority: Rc<dyn IXamlType>,
    pub ferro_object_extensions: Rc<dyn IXamlType>,
    pub ferro_property: Rc<dyn IXamlType>,
    pub ferro_property_t: Rc<dyn IXamlType>,
    pub styled_property_t: Rc<dyn IXamlType>,
    pub ferro_object_set_styled_property_value: Rc<dyn IXamlMethod>,
    pub ferro_attached_property_t: Rc<dyn IXamlType>,
    pub binding_base: Rc<dyn IXamlType>,
    pub binding_expression_base: Rc<dyn IXamlType>,
    pub multi_binding: Rc<dyn IXamlType>,
    pub ferro_object_bind_method: Rc<dyn IXamlMethod>,
    pub ferro_object_set_value_method: Rc<dyn IXamlMethod>,
    pub i_disposable: Rc<dyn IXamlType>,
    pub i_command: Rc<dyn IXamlType>,
    pub xaml_il_types: Rc<XamlTypeWellKnownTypes>,
    pub transitions: Rc<dyn IXamlType>,
    pub assign_binding_attribute: Rc<dyn IXamlType>,
    pub depends_on_attribute: Rc<dyn IXamlType>,
    pub data_type_attribute: Rc<dyn IXamlType>,
    pub inherit_data_type_from_items_attribute: Rc<dyn IXamlType>,
    pub inherit_data_type_from_attribute: Rc<dyn IXamlType>,
    pub markup_extension_option_attribute: Rc<dyn IXamlType>,
    pub markup_extension_default_option_attribute: Rc<dyn IXamlType>,
    pub control_template_scope_attribute: Rc<dyn IXamlType>,
    pub ferro_list_attribute: Rc<dyn IXamlType>,
    pub ferro_list: Rc<dyn IXamlType>,
    pub on_extension_type: Rc<dyn IXamlType>,
    pub unset_value_type: Rc<dyn IXamlType>,
    pub styled_element: Rc<dyn IXamlType>,
    pub name_scope: Rc<dyn IXamlType>,
    pub name_scope_set_name_scope: Rc<dyn IXamlMethod>,
    pub i_name_scope: Rc<dyn IXamlType>,
    pub i_name_scope_register: Rc<dyn IXamlMethod>,
    pub i_name_scope_complete: Rc<dyn IXamlMethod>,
    pub i_property_info: Rc<dyn IXamlType>,
    pub clr_property_info: Rc<dyn IXamlType>,
    pub i_property_info_t: Rc<dyn IXamlType>,
    pub clr_property_info_t: Rc<dyn IXamlType>,
    pub i_property_accessor: Rc<dyn IXamlType>,
    pub property_info_accessor_factory: Rc<dyn IXamlType>,
    pub compiled_binding: Rc<dyn IXamlType>,
    pub compiled_binding_path_builder: Rc<dyn IXamlType>,
    pub compiled_binding_path: Rc<dyn IXamlType>,
    pub compiled_binding_extension: Rc<dyn IXamlType>,

    pub resolve_by_name_extension: Rc<dyn IXamlType>,

    pub data_template: Rc<dyn IXamlType>,
    pub i_data_template: Rc<dyn IXamlType>,
    pub i_template_of_control: Rc<dyn IXamlType>,
    pub control: Rc<dyn IXamlType>,
    pub content_control: Rc<dyn IXamlType>,
    pub items_control: Rc<dyn IXamlType>,
    pub reflection_binding_extension: Rc<dyn IXamlType>,

    pub relative_source: Rc<dyn IXamlType>,
    pub u_int: Rc<dyn IXamlType>,
    pub int: Rc<dyn IXamlType>,
    pub long: Rc<dyn IXamlType>,
    pub uri: Rc<dyn IXamlType>,
    pub task_of_t: Rc<dyn IXamlType>,
    pub i_dictionary_t: Rc<dyn IXamlType>,
    pub weak_reference_of_t: Rc<dyn IXamlType>,
    pub i_observable_of_t: Rc<dyn IXamlType>,
    pub font_family: Rc<dyn IXamlType>,
    pub font_family_constructor_uri_name: Rc<dyn IXamlConstructor>,
    pub thickness: Rc<dyn IXamlType>,
    pub thickness_full_constructor: Rc<dyn IXamlConstructor>,
    pub theme_variant: Rc<dyn IXamlType>,
    pub point: Rc<dyn IXamlType>,
    pub point_full_constructor: Rc<dyn IXamlConstructor>,
    pub vector: Rc<dyn IXamlType>,
    pub vector_full_constructor: Rc<dyn IXamlConstructor>,
    pub size: Rc<dyn IXamlType>,
    pub size_full_constructor: Rc<dyn IXamlConstructor>,
    pub matrix: Rc<dyn IXamlType>,
    pub matrix_full_constructor: Rc<dyn IXamlConstructor>,
    pub corner_radius: Rc<dyn IXamlType>,
    pub corner_radius_full_constructor: Rc<dyn IXamlConstructor>,
    pub relative_unit: Rc<dyn IXamlType>,
    pub relative_point: Rc<dyn IXamlType>,
    pub relative_point_full_constructor: Rc<dyn IXamlConstructor>,
    pub grid_length: Rc<dyn IXamlType>,
    pub grid_length_constructor_value_type: Rc<dyn IXamlConstructor>,
    pub color: Rc<dyn IXamlType>,
    pub standard_cursor_type: Rc<dyn IXamlType>,
    pub cursor: Rc<dyn IXamlType>,
    pub cursor_type_constructor: Rc<dyn IXamlConstructor>,
    pub row_definition: Rc<dyn IXamlType>,
    pub row_definitions: Rc<dyn IXamlType>,
    pub column_definition: Rc<dyn IXamlType>,
    pub column_definitions: Rc<dyn IXamlType>,
    pub classes: Rc<dyn IXamlType>,
    pub classes_bind_method: Rc<dyn IXamlMethod>,
    pub styled_element_classes_property: Rc<dyn IXamlProperty>,
    pub i_brush: Rc<dyn IXamlType>,
    pub immutable_solid_color_brush: Rc<dyn IXamlType>,
    pub immutable_solid_color_brush_constructor_color: Rc<dyn IXamlConstructor>,
    pub type_utilities: Rc<dyn IXamlType>,
    pub text_decoration_collection: Rc<dyn IXamlType>,
    pub text_decorations: Rc<dyn IXamlType>,
    pub text_trimming: Rc<dyn IXamlType>,
    pub setter_base: Rc<dyn IXamlType>,
    pub setter: Rc<dyn IXamlType>,
    pub i_style: Rc<dyn IXamlType>,
    pub style_include: Rc<dyn IXamlType>,
    pub resource_include: Rc<dyn IXamlType>,
    pub merge_resource_include: Rc<dyn IXamlType>,
    pub i_resource_dictionary: Rc<dyn IXamlType>,
    pub resource_dictionary: Rc<dyn IXamlType>,
    pub resource_dictionary_deferred_add: Rc<dyn IXamlMethod>,
    pub resource_dictionary_not_shared_deferred_add: Rc<dyn IXamlMethod>,
    pub resource_dictionary_ensure_capacity: Rc<dyn IXamlMethod>,
    pub resource_dictionary_get_count: Rc<dyn IXamlMethod>,
    pub i_theme_variant_provider: Rc<dyn IXamlType>,
    pub uri_kind: Rc<dyn IXamlType>,
    pub uri_constructor: Rc<dyn IXamlConstructor>,
    pub style: Rc<dyn IXamlType>,
    pub container: Rc<dyn IXamlType>,
    pub styles: Rc<dyn IXamlType>,
    pub style_queries: Rc<dyn IXamlType>,
    pub selectors: Rc<dyn IXamlType>,
    pub control_theme: Rc<dyn IXamlType>,
    pub window_transparency_level: Rc<dyn IXamlType>,
    pub i_read_only_list_of_t: Rc<dyn IXamlType>,
    pub control_template: Rc<dyn IXamlType>,
    pub event_handler_t: Rc<dyn IXamlType>,
    pub get_class_property: Rc<dyn IXamlMethod>,
    pub xaml_source_info_constructor: Rc<dyn IXamlConstructor>,
    pub xaml_source_info_setter: Rc<dyn IXamlMethod>,
    pub xaml_source_info_dictionary_setter: Rc<dyn IXamlMethod>,

    pub interactivity: InteractivityWellKnownTypes,
}

pub struct InteractivityWellKnownTypes {
    pub interactive: Rc<dyn IXamlType>,
    pub routed_event: Rc<dyn IXamlType>,
    pub routed_event_args: Rc<dyn IXamlType>,
    pub routed_event_handler: Rc<dyn IXamlType>,
    pub add_handler: Rc<dyn IXamlMethod>,
    pub add_handler_t: Rc<dyn IXamlMethod>,
}

impl InteractivityWellKnownTypes {
    pub(crate) fn new(
        type_system: &Rc<dyn IXamlTypeSystem>,
        well_known_types: &XamlTypeWellKnownTypes,
    ) -> XamlResult<Self> {
        let interactive = type_system.get_type("FerroUI.Interactivity.Interactive")?;
        let routed_event = type_system.get_type("FerroUI.Interactivity.RoutedEvent")?;
        let routed_event_args = type_system.get_type("FerroUI.Interactivity.RoutedEventArgs")?;
        let event_handler_t = type_system.get_type("System.EventHandler`1")?;
        let routed_event_handler =
            event_handler_t.make_generic_type(std::slice::from_ref(&routed_event_args))?;
        let add_handler = interactive.get_method(|m| {
            let parameters = m.parameters();
            m.is_public()
                && !m.is_static()
                && m.name() == "AddHandler"
                && parameters.len() == 4
                && parameters[0].equals(&*routed_event)
                && parameters[1].equals(&*well_known_types.delegate)
                && parameters[2].is_enum()
                && parameters[3].equals(&*well_known_types.boolean)
        })?;
        let add_handler_t = interactive.get_method(|m| {
            let parameters = m.parameters();
            m.is_public()
                && !m.is_static()
                && m.name() == "AddHandler"
                && parameters.len() == 4
                && routed_event.is_assignable_from(&*parameters[0])
                // This is specific this case workaround to check is generic method
                && parameters[0].generic_arguments().len() == 1
                && well_known_types.delegate.is_assignable_from(&*parameters[1])
                && parameters[2].is_enum()
                && parameters[3].equals(&*well_known_types.boolean)
        })?;

        Ok(Self {
            interactive,
            routed_event,
            routed_event_args,
            routed_event_handler,
            add_handler,
            add_handler_t,
        })
    }
}

impl FerroXamlIlWellKnownTypes {
    pub fn new(type_system: &Rc<dyn IXamlTypeSystem>) -> XamlResult<Self> {
        let get = |name: &str| type_system.get_type(name);

        let runtime_helpers = get("FerroUI.Markup.Xaml.XamlIl.Runtime.XamlIlRuntimeHelpers")?;

        let xaml_il_types = type_system.well_known_types();
        let ferro_object = get("FerroUI.FerroObject")?;
        let ferro_object_extensions = get("FerroUI.FerroObjectExtensions")?;
        let ferro_property = get("FerroUI.FerroProperty")?;
        let ferro_property_t = get("FerroUI.FerroProperty`1")?;
        let styled_property_t = get("FerroUI.StyledProperty`1")?;
        let ferro_attached_property_t = get("FerroUI.AttachedProperty`1")?;
        let binding_priority = get("FerroUI.Data.BindingPriority")?;
        let ferro_object_set_styled_property_value = ferro_object.get_method(|m| {
            let parameters = m.parameters();
            m.is_public()
                && !m.is_static()
                && m.name() == "SetValue"
                && parameters.len() == 3
                && parameters[0].name() == "StyledProperty`1"
                && parameters[2].equals(&*binding_priority)
        })?;
        let binding_base = get("FerroUI.Data.BindingBase")?;
        let binding_expression_base = get("FerroUI.Data.BindingExpressionBase")?;
        let multi_binding = get("FerroUI.Data.MultiBinding")?;
        let i_disposable = get("System.IDisposable")?;
        let i_command = get("System.Windows.Input.ICommand")?;
        let transitions = get("FerroUI.Animation.Transitions")?;
        let assign_binding_attribute = get("FerroUI.Data.AssignBindingAttribute")?;
        let depends_on_attribute = get("FerroUI.Metadata.DependsOnAttribute")?;
        let data_type_attribute = get("FerroUI.Metadata.DataTypeAttribute")?;
        let inherit_data_type_from_items_attribute =
            get("FerroUI.Metadata.InheritDataTypeFromItemsAttribute")?;
        let inherit_data_type_from_attribute =
            get("FerroUI.Metadata.InheritDataTypeFromAttribute")?;
        let markup_extension_option_attribute =
            get("FerroUI.Metadata.MarkupExtensionOptionAttribute")?;
        let markup_extension_default_option_attribute =
            get("FerroUI.Metadata.MarkupExtensionDefaultOptionAttribute")?;
        let control_template_scope_attribute =
            get("FerroUI.Metadata.ControlTemplateScopeAttribute")?;
        let ferro_list_attribute = get("FerroUI.Metadata.FerroListAttribute")?;
        let ferro_list = get("FerroUI.Collections.FerroList`1")?;
        let on_extension_type = get("FerroUI.Markup.Xaml.MarkupExtensions.On")?;
        let ferro_object_bind_method = ferro_object.get_method_by_name(
            "Bind",
            &*binding_expression_base,
            false,
            &[ferro_property.clone(), binding_base.clone()],
        )?;
        let unset_value_type = get("FerroUI.UnsetValueType")?;
        let styled_element = get("FerroUI.StyledElement")?;
        let i_name_scope = get("FerroUI.Controls.INameScope")?;
        let i_name_scope_register = i_name_scope.get_method_by_signature(&{
            let mut signature = FindMethodMethodSignature::new(
                "Register",
                xaml_il_types.void.clone(),
                vec![xaml_il_types.string.clone(), xaml_il_types.object.clone()],
            );
            signature.is_static = false;
            signature.declaring_only = true;
            signature.is_exact_match = true;
            signature
        })?;
        let i_name_scope_complete = i_name_scope.get_method_by_signature(&{
            let mut signature =
                FindMethodMethodSignature::new("Complete", xaml_il_types.void.clone(), vec![]);
            signature.is_static = false;
            signature.declaring_only = true;
            signature.is_exact_match = true;
            signature
        })?;
        let name_scope = get("FerroUI.Controls.NameScope")?;
        let name_scope_set_name_scope = name_scope.get_method_by_signature(&{
            let mut signature = FindMethodMethodSignature::new(
                "SetNameScope",
                xaml_il_types.void.clone(),
                vec![styled_element.clone(), i_name_scope.clone()],
            );
            signature.is_static = true;
            signature
        })?;
        let ferro_object_set_value_method = ferro_object.get_method_by_name(
            "SetValue",
            &*i_disposable,
            false,
            &[
                ferro_property.clone(),
                xaml_il_types.object.clone(),
                binding_priority.clone(),
            ],
        )?;
        let i_property_info = get("FerroUI.Data.Core.IPropertyInfo")?;
        let clr_property_info = get("FerroUI.Data.Core.ClrPropertyInfo")?;
        let i_property_info_t = get("FerroUI.Data.Core.IPropertyInfo`2")?;
        let clr_property_info_t = get("FerroUI.Data.Core.ClrPropertyInfo`2")?;
        let i_property_accessor = get("FerroUI.Data.Core.Plugins.IPropertyAccessor")?;
        let property_info_accessor_factory = get(
            "FerroUI.Markup.Xaml.MarkupExtensions.CompiledBindings.PropertyInfoAccessorFactory",
        )?;
        let compiled_binding = get("FerroUI.Data.CompiledBinding")?;
        let compiled_binding_path_builder = get("FerroUI.Data.CompiledBindingPathBuilder")?;
        let compiled_binding_path = get("FerroUI.Data.CompiledBindingPath")?;
        let compiled_binding_extension =
            get("FerroUI.Markup.Xaml.MarkupExtensions.CompiledBindingExtension")?;
        let resolve_by_name_extension =
            get("FerroUI.Markup.Xaml.MarkupExtensions.ResolveByNameExtension")?;
        let data_template = get("FerroUI.Markup.Xaml.Templates.DataTemplate")?;
        let i_data_template = get("FerroUI.Controls.Templates.IDataTemplate")?;
        let control = get("FerroUI.Controls.Control")?;
        let content_control = get("FerroUI.Controls.ContentControl")?;
        let i_template_of_control = get("FerroUI.Controls.ITemplate`1")?
            .make_generic_type(std::slice::from_ref(&control))?;
        let items_control = get("FerroUI.Controls.ItemsControl")?;
        let reflection_binding_extension =
            get("FerroUI.Markup.Xaml.MarkupExtensions.ReflectionBindingExtension")?;
        let relative_source = get("FerroUI.Data.RelativeSource")?;
        let u_int = get("System.UInt32")?;
        let int = get("System.Int32")?;
        let long = get("System.Int64")?;
        let uri = get("System.Uri")?;
        let task_of_t = get("System.Threading.Tasks.Task`1")?;
        let i_dictionary_t = get("System.Collections.Generic.IDictionary`2")?;
        let weak_reference_of_t = get("System.WeakReference`1")?;
        let i_observable_of_t = get("System.IObservable`1")?;
        let font_family = get("FerroUI.Media.FontFamily")?;
        let font_family_constructor_uri_name =
            font_family.get_constructor(Some(&[uri.clone(), xaml_il_types.string.clone()]))?;
        let theme_variant = get("FerroUI.Styling.ThemeVariant")?;
        let window_transparency_level = get("FerroUI.Controls.WindowTransparencyLevel")?;

        let get_numeric_type_info = |name: &str,
                                     component_type: &Rc<dyn IXamlType>,
                                     component_count: usize|
         -> XamlResult<(Rc<dyn IXamlType>, Rc<dyn IXamlConstructor>)> {
            let type_ = type_system.get_type(name)?;
            let arguments: Vec<Rc<dyn IXamlType>> =
                (0..component_count).map(|_| component_type.clone()).collect();
            let ctor = type_.get_constructor(Some(&arguments))?;

            Ok((type_, ctor))
        };

        let double = &xaml_il_types.double;
        let (thickness, thickness_full_constructor) =
            get_numeric_type_info("FerroUI.Thickness", double, 4)?;
        let (point, point_full_constructor) = get_numeric_type_info("FerroUI.Point", double, 2)?;
        let (vector, vector_full_constructor) =
            get_numeric_type_info("FerroUI.Vector", double, 2)?;
        let (size, size_full_constructor) = get_numeric_type_info("FerroUI.Size", double, 2)?;
        let (matrix, matrix_full_constructor) =
            get_numeric_type_info("FerroUI.Matrix", double, 6)?;
        let (corner_radius, corner_radius_full_constructor) =
            get_numeric_type_info("FerroUI.CornerRadius", double, 4)?;

        let relative_unit = get("FerroUI.RelativeUnit")?;
        let relative_point = get("FerroUI.RelativePoint")?;
        let relative_point_full_constructor = relative_point.get_constructor(Some(&[
            xaml_il_types.double.clone(),
            xaml_il_types.double.clone(),
            relative_unit.clone(),
        ]))?;

        let grid_length = get("FerroUI.Controls.GridLength")?;
        let grid_length_constructor_value_type = grid_length.get_constructor(Some(&[
            xaml_il_types.double.clone(),
            get("FerroUI.Controls.GridUnitType")?,
        ]))?;
        let color = get("FerroUI.Media.Color")?;
        let standard_cursor_type = get("FerroUI.Input.StandardCursorType")?;
        let cursor = get("FerroUI.Input.Cursor")?;
        let cursor_type_constructor =
            cursor.get_constructor(Some(std::slice::from_ref(&standard_cursor_type)))?;
        let column_definition = get("FerroUI.Controls.ColumnDefinition")?;
        let column_definitions = get("FerroUI.Controls.ColumnDefinitions")?;
        let row_definition = get("FerroUI.Controls.RowDefinition")?;
        let row_definitions = get("FerroUI.Controls.RowDefinitions")?;
        let classes = get("FerroUI.Controls.Classes")?;
        let styled_element_classes_property = styled_element
            .properties()
            .into_iter()
            .find(|x| x.name() == "Classes" && x.property_type().equals(&*classes))
            .ok_or_else(|| XamlError::invalid_operation("Sequence contains no matching element"))?;
        let classes_bind_method = get("FerroUI.StyledElementExtensions")?.get_method_by_name(
            "BindClass",
            &*i_disposable,
            false,
            &[
                styled_element.clone(),
                xaml_il_types.string.clone(),
                binding_base.clone(),
                xaml_il_types.object.clone(),
            ],
        )?;

        let i_brush = get("FerroUI.Media.IBrush")?;
        let immutable_solid_color_brush = get("FerroUI.Media.Immutable.ImmutableSolidColorBrush")?;
        let immutable_solid_color_brush_constructor_color =
            immutable_solid_color_brush.get_constructor(Some(std::slice::from_ref(&u_int)))?;
        let type_utilities = get("FerroUI.Utilities.TypeUtilities")?;
        let text_decoration_collection = get("FerroUI.Media.TextDecorationCollection")?;
        let text_decorations = get("FerroUI.Media.TextDecorations")?;
        let text_trimming = get("FerroUI.Media.TextTrimming")?;
        let setter_base = get("FerroUI.Styling.SetterBase")?;
        let setter = get("FerroUI.Styling.Setter")?;
        let i_style = get("FerroUI.Styling.IStyle")?;
        let style_include = get("FerroUI.Markup.Xaml.Styling.StyleInclude")?;
        let resource_include = get("FerroUI.Markup.Xaml.Styling.ResourceInclude")?;
        let merge_resource_include = get("FerroUI.Markup.Xaml.Styling.MergeResourceInclude")?;
        let i_resource_dictionary = get("FerroUI.Controls.IResourceDictionary")?;
        let resource_dictionary = get("FerroUI.Controls.ResourceDictionary")?;
        let resource_dictionary_deferred_add = resource_dictionary.get_method_by_name(
            "AddDeferred",
            &*xaml_il_types.void,
            true,
            &[
                xaml_il_types.object.clone(),
                get("FerroUI.Controls.IDeferredContent")?,
            ],
        )?;
        let resource_dictionary_not_shared_deferred_add = resource_dictionary.get_method_by_name(
            "AddNotSharedDeferred",
            &*xaml_il_types.void,
            true,
            &[
                xaml_il_types.object.clone(),
                get("FerroUI.Controls.IDeferredContent")?,
            ],
        )?;

        let resource_dictionary_ensure_capacity = resource_dictionary.get_method_by_name(
            "EnsureCapacity",
            &*xaml_il_types.void,
            true,
            std::slice::from_ref(&xaml_il_types.int32),
        )?;
        let resource_dictionary_get_count =
            resource_dictionary.get_method_by_name("get_Count", &*xaml_il_types.int32, true, &[])?;
        let i_theme_variant_provider = get("FerroUI.Controls.IThemeVariantProvider")?;
        let uri_kind = get("System.UriKind")?;
        let uri_constructor =
            uri.get_constructor(Some(&[xaml_il_types.string.clone(), uri_kind.clone()]))?;
        let style = get("FerroUI.Styling.Style")?;
        let container = get("FerroUI.Styling.ContainerQuery")?;
        let styles = get("FerroUI.Styling.Styles")?;
        let style_queries = get("FerroUI.Styling.StyleQueries")?;
        let selectors = get("FerroUI.Styling.Selectors")?;
        let control_theme = get("FerroUI.Styling.ControlTheme")?;
        let control_template = get("FerroUI.Markup.Xaml.Templates.ControlTemplate")?;
        let i_read_only_list_of_t = get("System.Collections.Generic.IReadOnlyList`1")?;
        let event_handler_t = get("System.EventHandler`1")?;
        let interactivity = InteractivityWellKnownTypes::new(type_system, &xaml_il_types)?;

        let get_class_property = get("FerroUI.StyledElementExtensions")?.get_method_by_name(
            "GetClassProperty",
            &*ferro_property,
            false,
            std::slice::from_ref(&xaml_il_types.string),
        )?;

        let xaml_source_info = get("FerroUI.Markup.Xaml.Diagnostics.XamlSourceInfo")?;
        let xaml_source_info_constructor = xaml_source_info.get_constructor(Some(&[
            xaml_il_types.int32.clone(),
            xaml_il_types.int32.clone(),
            xaml_il_types.string.clone(),
        ]))?;
        let xaml_source_info_setter = xaml_source_info.get_method_by_name(
            "SetXamlSourceInfo",
            &*xaml_il_types.void,
            false,
            &[xaml_il_types.object.clone(), xaml_source_info.clone()],
        )?;
        let xaml_source_info_dictionary_setter = xaml_source_info.get_method_by_name(
            "SetXamlSourceInfo",
            &*xaml_il_types.void,
            false,
            &[
                i_resource_dictionary.clone(),
                xaml_il_types.object.clone(),
                xaml_source_info.clone(),
            ],
        )?;

        Ok(Self {
            runtime_helpers,
            ferro_object,
            binding_priority,
            ferro_object_extensions,
            ferro_property,
            ferro_property_t,
            styled_property_t,
            ferro_object_set_styled_property_value,
            ferro_attached_property_t,
            binding_base,
            binding_expression_base,
            multi_binding,
            ferro_object_bind_method,
            ferro_object_set_value_method,
            i_disposable,
            i_command,
            xaml_il_types,
            transitions,
            assign_binding_attribute,
            depends_on_attribute,
            data_type_attribute,
            inherit_data_type_from_items_attribute,
            inherit_data_type_from_attribute,
            markup_extension_option_attribute,
            markup_extension_default_option_attribute,
            control_template_scope_attribute,
            ferro_list_attribute,
            ferro_list,
            on_extension_type,
            unset_value_type,
            styled_element,
            name_scope,
            name_scope_set_name_scope,
            i_name_scope,
            i_name_scope_register,
            i_name_scope_complete,
            i_property_info,
            clr_property_info,
            i_property_info_t,
            clr_property_info_t,
            i_property_accessor,
            property_info_accessor_factory,
            compiled_binding,
            compiled_binding_path_builder,
            compiled_binding_path,
            compiled_binding_extension,
            resolve_by_name_extension,
            data_template,
            i_data_template,
            i_template_of_control,
            control,
            content_control,
            items_control,
            reflection_binding_extension,
            relative_source,
            u_int,
            int,
            long,
            uri,
            task_of_t,
            i_dictionary_t,
            weak_reference_of_t,
            i_observable_of_t,
            font_family,
            font_family_constructor_uri_name,
            thickness,
            thickness_full_constructor,
            theme_variant,
            point,
            point_full_constructor,
            vector,
            vector_full_constructor,
            size,
            size_full_constructor,
            matrix,
            matrix_full_constructor,
            corner_radius,
            corner_radius_full_constructor,
            relative_unit,
            relative_point,
            relative_point_full_constructor,
            grid_length,
            grid_length_constructor_value_type,
            color,
            standard_cursor_type,
            cursor,
            cursor_type_constructor,
            row_definition,
            row_definitions,
            column_definition,
            column_definitions,
            classes,
            classes_bind_method,
            styled_element_classes_property,
            i_brush,
            immutable_solid_color_brush,
            immutable_solid_color_brush_constructor_color,
            type_utilities,
            text_decoration_collection,
            text_decorations,
            text_trimming,
            setter_base,
            setter,
            i_style,
            style_include,
            resource_include,
            merge_resource_include,
            i_resource_dictionary,
            resource_dictionary,
            resource_dictionary_deferred_add,
            resource_dictionary_not_shared_deferred_add,
            resource_dictionary_ensure_capacity,
            resource_dictionary_get_count,
            i_theme_variant_provider,
            uri_kind,
            uri_constructor,
            style,
            container,
            styles,
            style_queries,
            selectors,
            control_theme,
            window_transparency_level,
            i_read_only_list_of_t,
            control_template,
            event_handler_t,
            get_class_property,
            xaml_source_info_constructor,
            xaml_source_info_setter,
            xaml_source_info_dictionary_setter,
            interactivity,
        })
    }
}

/// `FerroXamlIlWellKnownTypesExtensions`: `cfg.GetFerroTypes()` / `ctx.GetFerroTypes()`.
///
/// Upstream reads the instance the compiler configuration registered as a configuration extra
/// (`cfg.GetExtra<FerroXamlIlWellKnownTypes>()`). The port does the same and, for a plain
/// `TransformerConfiguration` that was not created through
/// [`crate::compiler_extensions::FerroXamlIlCompilerConfiguration`], resolves the types on first
/// use and caches them as that extra, so there is exactly one instance per configuration.
pub trait FerroXamlIlWellKnownTypesExtensions {
    /// The well-known types of the configuration.
    ///
    /// # Panics
    /// Panics when the extra is missing and the type system of the configuration lacks a
    /// well-known framework type (upstream: the configuration constructor throws). Use
    /// [`FerroXamlIlWellKnownTypesExtensions::try_get_ferro_types`] to get the error instead.
    fn get_ferro_types(&self) -> Rc<FerroXamlIlWellKnownTypes> {
        match self.try_get_ferro_types() {
            Ok(types) => types,
            Err(e) => panic!("The framework's well-known types are not available: {e}"),
        }
    }

    fn try_get_ferro_types(&self) -> XamlResult<Rc<FerroXamlIlWellKnownTypes>>;
}

impl FerroXamlIlWellKnownTypesExtensions for TransformerConfiguration {
    fn try_get_ferro_types(&self) -> XamlResult<Rc<FerroXamlIlWellKnownTypes>> {
        if let Ok(types) = self.get_extra::<FerroXamlIlWellKnownTypes>() {
            return Ok(types);
        }
        let types = Rc::new(FerroXamlIlWellKnownTypes::new(&self.type_system)?);
        self.add_extra(types.clone());
        Ok(types)
    }
}

impl FerroXamlIlWellKnownTypesExtensions for AstTransformationContext {
    fn try_get_ferro_types(&self) -> XamlResult<Rc<FerroXamlIlWellKnownTypes>> {
        self.configuration().try_get_ferro_types()
    }
}

impl FerroXamlIlWellKnownTypesExtensions for AstGroupTransformationContext {
    fn try_get_ferro_types(&self) -> XamlResult<Rc<FerroXamlIlWellKnownTypes>> {
        self.configuration().try_get_ferro_types()
    }
}

/// `context.GetFerroTypes()` as a free function.
pub fn get_ferro_types(context: &AstTransformationContext) -> Rc<FerroXamlIlWellKnownTypes> {
    context.get_ferro_types()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::create_test_framework;
    use xamlx::testing::FakeTypeSystem;

    fn parameter_names(method: &Rc<dyn IXamlMethod>) -> Vec<String> {
        method.parameters().iter().map(|p| p.full_name()).collect()
    }

    fn constructor_parameter_names(constructor: &Rc<dyn IXamlConstructor>) -> Vec<String> {
        constructor.parameters().iter().map(|p| p.full_name()).collect()
    }

    #[test]
    fn resolves_completely_against_the_fixture() {
        let fw = create_test_framework();
        let types = FerroXamlIlWellKnownTypes::new(&fw.as_type_system());
        assert!(types.is_ok(), "{:?}", types.err().map(|e| e.to_string()));
    }

    #[test]
    fn resolves_types_by_their_framework_names() {
        let fw = create_test_framework();
        let t = &fw.types;
        for (type_, name) in [
            (&t.runtime_helpers, "FerroUI.Markup.Xaml.XamlIl.Runtime.XamlIlRuntimeHelpers"),
            (&t.ferro_object, "FerroUI.FerroObject"),
            (&t.binding_priority, "FerroUI.Data.BindingPriority"),
            (&t.ferro_object_extensions, "FerroUI.FerroObjectExtensions"),
            (&t.ferro_property, "FerroUI.FerroProperty"),
            (&t.ferro_property_t, "FerroUI.FerroProperty`1"),
            (&t.styled_property_t, "FerroUI.StyledProperty`1"),
            (&t.ferro_attached_property_t, "FerroUI.AttachedProperty`1"),
            (&t.binding_base, "FerroUI.Data.BindingBase"),
            (&t.binding_expression_base, "FerroUI.Data.BindingExpressionBase"),
            (&t.multi_binding, "FerroUI.Data.MultiBinding"),
            (&t.i_disposable, "System.IDisposable"),
            (&t.i_command, "System.Windows.Input.ICommand"),
            (&t.transitions, "FerroUI.Animation.Transitions"),
            (&t.assign_binding_attribute, "FerroUI.Data.AssignBindingAttribute"),
            (&t.depends_on_attribute, "FerroUI.Metadata.DependsOnAttribute"),
            (&t.data_type_attribute, "FerroUI.Metadata.DataTypeAttribute"),
            (
                &t.inherit_data_type_from_items_attribute,
                "FerroUI.Metadata.InheritDataTypeFromItemsAttribute",
            ),
            (
                &t.inherit_data_type_from_attribute,
                "FerroUI.Metadata.InheritDataTypeFromAttribute",
            ),
            (
                &t.markup_extension_option_attribute,
                "FerroUI.Metadata.MarkupExtensionOptionAttribute",
            ),
            (
                &t.markup_extension_default_option_attribute,
                "FerroUI.Metadata.MarkupExtensionDefaultOptionAttribute",
            ),
            (
                &t.control_template_scope_attribute,
                "FerroUI.Metadata.ControlTemplateScopeAttribute",
            ),
            (&t.ferro_list_attribute, "FerroUI.Metadata.FerroListAttribute"),
            (&t.ferro_list, "FerroUI.Collections.FerroList`1"),
            (&t.on_extension_type, "FerroUI.Markup.Xaml.MarkupExtensions.On"),
            (&t.unset_value_type, "FerroUI.UnsetValueType"),
            (&t.styled_element, "FerroUI.StyledElement"),
            (&t.name_scope, "FerroUI.Controls.NameScope"),
            (&t.i_name_scope, "FerroUI.Controls.INameScope"),
            (&t.i_property_info, "FerroUI.Data.Core.IPropertyInfo"),
            (&t.clr_property_info, "FerroUI.Data.Core.ClrPropertyInfo"),
            (&t.i_property_info_t, "FerroUI.Data.Core.IPropertyInfo`2"),
            (&t.clr_property_info_t, "FerroUI.Data.Core.ClrPropertyInfo`2"),
            (&t.i_property_accessor, "FerroUI.Data.Core.Plugins.IPropertyAccessor"),
            (
                &t.property_info_accessor_factory,
                "FerroUI.Markup.Xaml.MarkupExtensions.CompiledBindings.PropertyInfoAccessorFactory",
            ),
            (&t.compiled_binding, "FerroUI.Data.CompiledBinding"),
            (&t.compiled_binding_path_builder, "FerroUI.Data.CompiledBindingPathBuilder"),
            (&t.compiled_binding_path, "FerroUI.Data.CompiledBindingPath"),
            (
                &t.compiled_binding_extension,
                "FerroUI.Markup.Xaml.MarkupExtensions.CompiledBindingExtension",
            ),
            (
                &t.resolve_by_name_extension,
                "FerroUI.Markup.Xaml.MarkupExtensions.ResolveByNameExtension",
            ),
            (&t.data_template, "FerroUI.Markup.Xaml.Templates.DataTemplate"),
            (&t.i_data_template, "FerroUI.Controls.Templates.IDataTemplate"),
            (
                &t.i_template_of_control,
                "FerroUI.Controls.ITemplate`1[FerroUI.Controls.Control]",
            ),
            (&t.control, "FerroUI.Controls.Control"),
            (&t.content_control, "FerroUI.Controls.ContentControl"),
            (&t.items_control, "FerroUI.Controls.ItemsControl"),
            (
                &t.reflection_binding_extension,
                "FerroUI.Markup.Xaml.MarkupExtensions.ReflectionBindingExtension",
            ),
            (&t.relative_source, "FerroUI.Data.RelativeSource"),
            (&t.u_int, "System.UInt32"),
            (&t.int, "System.Int32"),
            (&t.long, "System.Int64"),
            (&t.uri, "System.Uri"),
            (&t.task_of_t, "System.Threading.Tasks.Task`1"),
            (&t.i_dictionary_t, "System.Collections.Generic.IDictionary`2"),
            (&t.weak_reference_of_t, "System.WeakReference`1"),
            (&t.i_observable_of_t, "System.IObservable`1"),
            (&t.font_family, "FerroUI.Media.FontFamily"),
            (&t.thickness, "FerroUI.Thickness"),
            (&t.theme_variant, "FerroUI.Styling.ThemeVariant"),
            (&t.point, "FerroUI.Point"),
            (&t.vector, "FerroUI.Vector"),
            (&t.size, "FerroUI.Size"),
            (&t.matrix, "FerroUI.Matrix"),
            (&t.corner_radius, "FerroUI.CornerRadius"),
            (&t.relative_unit, "FerroUI.RelativeUnit"),
            (&t.relative_point, "FerroUI.RelativePoint"),
            (&t.grid_length, "FerroUI.Controls.GridLength"),
            (&t.color, "FerroUI.Media.Color"),
            (&t.standard_cursor_type, "FerroUI.Input.StandardCursorType"),
            (&t.cursor, "FerroUI.Input.Cursor"),
            (&t.row_definition, "FerroUI.Controls.RowDefinition"),
            (&t.row_definitions, "FerroUI.Controls.RowDefinitions"),
            (&t.column_definition, "FerroUI.Controls.ColumnDefinition"),
            (&t.column_definitions, "FerroUI.Controls.ColumnDefinitions"),
            (&t.classes, "FerroUI.Controls.Classes"),
            (&t.i_brush, "FerroUI.Media.IBrush"),
            (
                &t.immutable_solid_color_brush,
                "FerroUI.Media.Immutable.ImmutableSolidColorBrush",
            ),
            (&t.type_utilities, "FerroUI.Utilities.TypeUtilities"),
            (&t.text_decoration_collection, "FerroUI.Media.TextDecorationCollection"),
            (&t.text_decorations, "FerroUI.Media.TextDecorations"),
            (&t.text_trimming, "FerroUI.Media.TextTrimming"),
            (&t.setter_base, "FerroUI.Styling.SetterBase"),
            (&t.setter, "FerroUI.Styling.Setter"),
            (&t.i_style, "FerroUI.Styling.IStyle"),
            (&t.style_include, "FerroUI.Markup.Xaml.Styling.StyleInclude"),
            (&t.resource_include, "FerroUI.Markup.Xaml.Styling.ResourceInclude"),
            (
                &t.merge_resource_include,
                "FerroUI.Markup.Xaml.Styling.MergeResourceInclude",
            ),
            (&t.i_resource_dictionary, "FerroUI.Controls.IResourceDictionary"),
            (&t.resource_dictionary, "FerroUI.Controls.ResourceDictionary"),
            (&t.i_theme_variant_provider, "FerroUI.Controls.IThemeVariantProvider"),
            (&t.uri_kind, "System.UriKind"),
            (&t.style, "FerroUI.Styling.Style"),
            (&t.container, "FerroUI.Styling.ContainerQuery"),
            (&t.styles, "FerroUI.Styling.Styles"),
            (&t.style_queries, "FerroUI.Styling.StyleQueries"),
            (&t.selectors, "FerroUI.Styling.Selectors"),
            (&t.control_theme, "FerroUI.Styling.ControlTheme"),
            (&t.window_transparency_level, "FerroUI.Controls.WindowTransparencyLevel"),
            (&t.i_read_only_list_of_t, "System.Collections.Generic.IReadOnlyList`1"),
            (&t.control_template, "FerroUI.Markup.Xaml.Templates.ControlTemplate"),
            (&t.event_handler_t, "System.EventHandler`1"),
            (&t.interactivity.interactive, "FerroUI.Interactivity.Interactive"),
            (&t.interactivity.routed_event, "FerroUI.Interactivity.RoutedEvent"),
            (&t.interactivity.routed_event_args, "FerroUI.Interactivity.RoutedEventArgs"),
            (
                &t.interactivity.routed_event_handler,
                "System.EventHandler`1[FerroUI.Interactivity.RoutedEventArgs]",
            ),
        ] {
            assert_eq!(type_.full_name(), name);
        }
        assert!(Rc::ptr_eq(&t.xaml_il_types, &fw.as_type_system().well_known_types()));
    }

    #[test]
    fn resolves_methods_with_the_expected_signatures() {
        let fw = create_test_framework();
        let t = &fw.types;

        // The generic `SetValue<T>(StyledProperty<T>, T, BindingPriority)`.
        let set_styled = &t.ferro_object_set_styled_property_value;
        assert_eq!(set_styled.name(), "SetValue");
        assert!(set_styled.is_generic_method_definition());
        assert_eq!(set_styled.parameters()[0].name(), "StyledProperty`1");
        let constructed = set_styled
            .make_generic_method(&[fw.t("System.Double")])
            .expect("SetValue<double>");
        assert_eq!(
            parameter_names(&constructed),
            [
                "FerroUI.StyledProperty`1[System.Double]",
                "System.Double",
                "FerroUI.Data.BindingPriority"
            ]
        );

        // The untyped `SetValue(FerroProperty, object, BindingPriority)`.
        assert_eq!(t.ferro_object_set_value_method.name(), "SetValue");
        assert!(!t.ferro_object_set_value_method.is_generic_method());
        assert_eq!(
            parameter_names(&t.ferro_object_set_value_method),
            ["FerroUI.FerroProperty", "System.Object", "FerroUI.Data.BindingPriority"]
        );
        assert_eq!(
            t.ferro_object_set_value_method.return_type().full_name(),
            "System.IDisposable"
        );

        assert_eq!(t.ferro_object_bind_method.name(), "Bind");
        assert_eq!(
            parameter_names(&t.ferro_object_bind_method),
            ["FerroUI.FerroProperty", "FerroUI.Data.BindingBase"]
        );
        assert_eq!(
            t.ferro_object_bind_method.return_type().full_name(),
            "FerroUI.Data.BindingExpressionBase"
        );

        assert_eq!(t.name_scope_set_name_scope.name(), "SetNameScope");
        assert!(t.name_scope_set_name_scope.is_static());
        assert_eq!(
            parameter_names(&t.name_scope_set_name_scope),
            ["FerroUI.StyledElement", "FerroUI.Controls.INameScope"]
        );
        assert_eq!(t.i_name_scope_register.name(), "Register");
        assert_eq!(
            parameter_names(&t.i_name_scope_register),
            ["System.String", "System.Object"]
        );
        assert_eq!(t.i_name_scope_complete.name(), "Complete");
        assert!(t.i_name_scope_complete.parameters().is_empty());

        assert_eq!(t.classes_bind_method.name(), "BindClass");
        assert!(t.classes_bind_method.is_static());
        assert_eq!(
            parameter_names(&t.classes_bind_method),
            [
                "FerroUI.StyledElement",
                "System.String",
                "FerroUI.Data.BindingBase",
                "System.Object"
            ]
        );
        assert_eq!(t.get_class_property.name(), "GetClassProperty");
        assert_eq!(parameter_names(&t.get_class_property), ["System.String"]);
        assert_eq!(t.get_class_property.return_type().full_name(), "FerroUI.FerroProperty");

        assert_eq!(t.styled_element_classes_property.name(), "Classes");
        assert!(t
            .styled_element_classes_property
            .property_type()
            .equals(&*t.classes));

        assert_eq!(t.resource_dictionary_deferred_add.name(), "AddDeferred");
        assert_eq!(
            parameter_names(&t.resource_dictionary_deferred_add),
            ["System.Object", "FerroUI.Controls.IDeferredContent"]
        );
        assert_eq!(
            t.resource_dictionary_not_shared_deferred_add.name(),
            "AddNotSharedDeferred"
        );
        assert_eq!(t.resource_dictionary_ensure_capacity.name(), "EnsureCapacity");
        assert_eq!(
            parameter_names(&t.resource_dictionary_ensure_capacity),
            ["System.Int32"]
        );
        assert_eq!(t.resource_dictionary_get_count.name(), "get_Count");
        assert_eq!(
            t.resource_dictionary_get_count.return_type().full_name(),
            "System.Int32"
        );

        assert_eq!(t.xaml_source_info_setter.name(), "SetXamlSourceInfo");
        assert_eq!(
            parameter_names(&t.xaml_source_info_setter),
            ["System.Object", "FerroUI.Markup.Xaml.Diagnostics.XamlSourceInfo"]
        );
        assert_eq!(
            parameter_names(&t.xaml_source_info_dictionary_setter),
            [
                "FerroUI.Controls.IResourceDictionary",
                "System.Object",
                "FerroUI.Markup.Xaml.Diagnostics.XamlSourceInfo"
            ]
        );
    }

    #[test]
    fn resolves_constructors_with_the_expected_signatures() {
        let fw = create_test_framework();
        let t = &fw.types;
        for (constructor, declaring_type, parameters) in [
            (
                &t.font_family_constructor_uri_name,
                "FerroUI.Media.FontFamily",
                vec!["System.Uri", "System.String"],
            ),
            (&t.thickness_full_constructor, "FerroUI.Thickness", vec!["System.Double"; 4]),
            (&t.point_full_constructor, "FerroUI.Point", vec!["System.Double"; 2]),
            (&t.vector_full_constructor, "FerroUI.Vector", vec!["System.Double"; 2]),
            (&t.size_full_constructor, "FerroUI.Size", vec!["System.Double"; 2]),
            (&t.matrix_full_constructor, "FerroUI.Matrix", vec!["System.Double"; 6]),
            (
                &t.corner_radius_full_constructor,
                "FerroUI.CornerRadius",
                vec!["System.Double"; 4],
            ),
            (
                &t.relative_point_full_constructor,
                "FerroUI.RelativePoint",
                vec!["System.Double", "System.Double", "FerroUI.RelativeUnit"],
            ),
            (
                &t.grid_length_constructor_value_type,
                "FerroUI.Controls.GridLength",
                vec!["System.Double", "FerroUI.Controls.GridUnitType"],
            ),
            (
                &t.cursor_type_constructor,
                "FerroUI.Input.Cursor",
                vec!["FerroUI.Input.StandardCursorType"],
            ),
            (
                &t.immutable_solid_color_brush_constructor_color,
                "FerroUI.Media.Immutable.ImmutableSolidColorBrush",
                vec!["System.UInt32"],
            ),
            (&t.uri_constructor, "System.Uri", vec!["System.String", "System.UriKind"]),
            (
                &t.xaml_source_info_constructor,
                "FerroUI.Markup.Xaml.Diagnostics.XamlSourceInfo",
                vec!["System.Int32", "System.Int32", "System.String"],
            ),
        ] {
            assert_eq!(constructor.declaring_type().full_name(), declaring_type);
            assert_eq!(constructor_parameter_names(constructor), parameters);
        }
    }

    #[test]
    fn resolves_the_two_add_handler_overloads() {
        let fw = create_test_framework();
        let interactivity = &fw.types.interactivity;

        assert!(!interactivity.add_handler.is_generic_method());
        assert_eq!(
            parameter_names(&interactivity.add_handler),
            [
                "FerroUI.Interactivity.RoutedEvent",
                "System.Delegate",
                "FerroUI.Interactivity.RoutingStrategies",
                "System.Boolean"
            ]
        );

        assert!(interactivity.add_handler_t.is_generic_method_definition());
        let constructed = interactivity
            .add_handler_t
            .make_generic_method(std::slice::from_ref(&interactivity.routed_event_args))
            .expect("AddHandler<RoutedEventArgs>");
        assert_eq!(
            parameter_names(&constructed),
            [
                "FerroUI.Interactivity.RoutedEvent`1[FerroUI.Interactivity.RoutedEventArgs]",
                "System.EventHandler`1[FerroUI.Interactivity.RoutedEventArgs]",
                "FerroUI.Interactivity.RoutingStrategies",
                "System.Boolean"
            ]
        );
        assert!(constructed.parameters()[1].equals(&*interactivity.routed_event_handler));
    }

    #[test]
    fn is_cached_per_configuration() {
        let fw = create_test_framework();
        let context = fw.create_context();
        let from_configuration = fw.configuration.get_ferro_types();
        assert!(Rc::ptr_eq(&from_configuration, &fw.types));
        assert!(Rc::ptr_eq(&context.get_ferro_types(), &fw.types));
        assert!(Rc::ptr_eq(&get_ferro_types(&context), &fw.types));
        assert!(Rc::ptr_eq(
            &fw.configuration
                .get_extra::<FerroXamlIlWellKnownTypes>()
                .expect("registered as a configuration extra"),
            &fw.types
        ));
    }

    #[test]
    fn is_created_on_demand_for_a_plain_configuration() {
        let fw = create_test_framework();
        let type_system = fw.as_type_system();
        let (language, _) =
            crate::compiler_extensions::FerroXamlIlLanguage::configure(&type_system).expect("language");
        let configuration =
            TransformerConfiguration::new(type_system, None, language, None, None, None, None)
                .expect("configuration");
        assert!(configuration.get_extra::<FerroXamlIlWellKnownTypes>().is_err());
        let first = configuration.get_ferro_types();
        assert!(Rc::ptr_eq(&first, &configuration.get_ferro_types()));
        assert!(!Rc::ptr_eq(&first, &fw.types));
    }

    #[test]
    fn reports_a_missing_framework_type() {
        let type_system = FakeTypeSystem::new().as_type_system();
        let error = FerroXamlIlWellKnownTypes::new(&type_system)
            .err()
            .expect("the framework is missing");
        assert!(error.is_type_system_exception());
        assert_eq!(
            error.message(),
            "Unable to resolve type FerroUI.Markup.Xaml.XamlIl.Runtime.XamlIlRuntimeHelpers"
        );
    }
}

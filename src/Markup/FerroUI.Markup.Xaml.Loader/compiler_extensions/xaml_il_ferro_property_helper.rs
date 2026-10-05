//! Port of `CompilerExtensions/XamlIlFerroPropertyHelper.cs`.
//!
//! Registered-property support: locating the static `FooProperty` field behind a CLR
//! property, the AST nodes that stand for a registered property, the `XamlAstClrProperty`
//! "subclass" that knows its registered property, and the setters that assign such a property
//! (plain value with priority, binding, binding with priority, unset value).
//!
//! The members that emit IL upstream carry only their data here; each type documents what its
//! IL does so that the back ends can implement it.

use std::any::Any;
use std::cell::Cell;
use std::rc::Rc;

use xamlx::ast::{
    IXamlAstClrPropertyExtension, IXamlAstNode, IXamlAstTypeReference, IXamlAstValueNode,
    IXamlLineInfo, IXamlPropertySetter, PropertySetterBinderParameters, XamlAstCast,
    XamlAstClrProperty, XamlAstClrTypeReference, XamlAstNamePropertyReference, XamlAstNode,
    XamlAstNodeExtensions,
};
use xamlx::exceptions::{XamlError, XamlResult};
use xamlx::extensions::{query_clr_property_extension_interface, query_node_interface};
use xamlx::transform::transformers::{PropertyReferenceResolver, TypeReferenceResolver};
use xamlx::transform::{AstTransformationContext, IXamlAstTransformer};
use xamlx::type_system::{
    IXamlCustomAttribute, IXamlField, IXamlMethod, IXamlProperty, IXamlType, XamlPseudoType,
};
use xamlx::{xaml_ast_node_members, xaml_line_info_impl, xaml_query_interface};

use crate::compiler_extensions::transformers::{
    FerroXamlIlWellKnownTypes, FerroXamlIlWellKnownTypesExtensions,
};
use crate::parsers::PropertyParser;

/// What `EmitProvideValueTarget` loads as the `IProvideValueTarget.TargetProperty` of a
/// property assignment.
#[derive(Clone)]
pub enum XamlIlProvideValueTargetProperty {
    /// The registered property stored in this static field (`ldsfld field`).
    FerroProperty(Rc<dyn IXamlField>),
    /// A CLR property info object for this property, produced by the back end's
    /// `XamlIlClrPropertyInfoEmitter` equivalent.
    ClrProperty(Rc<dyn IXamlProperty>),
}

pub struct XamlIlFerroPropertyHelper;

impl XamlIlFerroPropertyHelper {
    /// `EmitProvideValueTarget(context, emitter, property)`: the language's
    /// `ProvideValueTargetPropertyEmitter`.
    ///
    /// Upstream emits the value and returns `true`, or returns `false` when there is nothing to
    /// emit. Here the decision is returned as data (`None` is `false`):
    ///
    /// 1. when [`XamlIlFerroPropertyHelper::try_get_ferro_property_field`] finds a registered
    ///    property field, the back end loads that static field;
    /// 2. otherwise, when the declaring type declares (not inherits) a CLR property with the
    ///    same name, the back end emits a CLR property info for it;
    /// 3. otherwise nothing is emitted.
    pub fn try_get_provide_value_target(
        property: &XamlAstClrProperty,
    ) -> Option<XamlIlProvideValueTargetProperty> {
        if let Some(field) = Self::try_get_ferro_property_field(property) {
            return Some(XamlIlProvideValueTargetProperty::FerroProperty(field));
        }
        let name = property.name();
        let found_clr = property
            .declaring_type()
            .properties()
            .into_iter()
            .find(|p| p.name() == name)?;
        Some(XamlIlProvideValueTargetProperty::ClrProperty(found_clr))
    }

    /// `Emit(context, emitter, XamlAstClrProperty property)`: the static field holding the
    /// registered property behind `property`, which upstream loads with `ldsfld`.
    ///
    /// The field is the one the property node carries when it is a registered property node
    /// ([`IXamlIlFerroProperty`]); otherwise the static field named `<Name>Property` declared by
    /// (not inherited into) the property's declaring type, of any visibility.
    pub fn try_get_ferro_property_field(property: &XamlAstClrProperty) -> Option<Rc<dyn IXamlField>> {
        if let Some(ap) = as_xaml_il_ferro_property(property) {
            return Some(ap.ferro_property());
        }
        let type_ = property.declaring_type();
        let name = format!("{}Property", property.name());
        type_
            .fields()
            .into_iter()
            .find(|f| f.is_static() && f.name() == name)
    }

    /// `Emit(context, emitter, IXamlProperty property)`: as above for a type system property.
    pub fn try_get_ferro_property_field_for_property(
        property: &dyn IXamlProperty,
    ) -> Option<Rc<dyn IXamlField>> {
        let name = format!("{}Property", property.name());
        property
            .declaring_type()
            .fields()
            .into_iter()
            .find(|f| f.is_static() && f.name() == name)
    }

    pub fn create_node(
        context: &AstTransformationContext,
        property_name: &str,
        selector_type_reference: Rc<dyn IXamlAstTypeReference>,
        line_info: &dyn IXamlLineInfo,
    ) -> XamlResult<Rc<dyn IXamlIlFerroPropertyNode>> {
        let (ns, owner, name) = PropertyParser::parse(property_name)
            .map_err(|e| XamlError::internal("ExpressionParseException", e.message()))?;

        let forged_reference = match owner {
            None => XamlAstNamePropertyReference::new(
                line_info,
                selector_type_reference.clone(),
                property_name,
                selector_type_reference,
            ),
            Some(owner)
                if is_null_or_white_space(ns.as_deref())
                    && ordinal_ignore_case_equals(&owner, "Classes")
                    && !is_null_or_white_space(Some(name.as_str())) =>
            {
                return Ok(XamlIlFerroClassProperty::new(
                    &context.try_get_ferro_types()?,
                    &name,
                    line_info,
                )?);
            }
            Some(owner) => {
                let xml_owner = match ns {
                    Some(ns) => format!("{ns}:{owner}"),
                    None => owner,
                };

                let tref = TypeReferenceResolver::resolve_type_by_xml_name(
                    context, &xml_owner, false, line_info, true,
                )?;

                let property_field_name = format!("{name}Property");
                let found = tref
                    .type_
                    .get_all_fields()
                    .into_iter()
                    .find(|f| f.is_static() && f.is_public() && f.name() == property_field_name);
                let Some(found) = found else {
                    return Err(XamlError::transform_exception(
                        format!(
                            "Unable to find {property_field_name} field on type {}",
                            tref.type_.get_full_name()
                        ),
                        Some(line_info),
                    ));
                };
                return Ok(XamlIlFerroPropertyFieldNode::new(
                    &*context.try_get_ferro_types()?,
                    line_info,
                    found,
                )?);
            }
        };

        let transformed = PropertyReferenceResolver.transform(context, forged_reference)?;
        let transformed_type_name = transformed.type_name();
        let clr_property = transformed.cast::<XamlAstClrProperty>().ok_or_else(|| {
            XamlError::invalid_cast(format!(
                "Unable to cast object of type '{transformed_type_name}' to type 'XamlAstClrProperty'."
            ))
        })?;
        let ferro_property_base_type = context.try_get_ferro_types()?.ferro_property.clone();

        // PropertyReferenceResolver.Transform failed resolving property, return empty stub from here:
        if XamlPseudoType::is_unknown(&*clr_property.declaring_type()) {
            return Ok(XamlIlFerroPropertyNode::with_property_type(
                line_info,
                ferro_property_base_type,
                clr_property,
                XamlPseudoType::unknown(),
            ));
        }

        Ok(XamlIlFerroPropertyNode::new(
            line_info,
            ferro_property_base_type,
            clr_property,
        )?)
    }

    /// The value type `T` of the registered property stored in `field`: the type argument of
    /// the `FerroProperty<T>` the field's type is or derives from.
    pub fn get_ferro_property_type(
        field: &dyn IXamlField,
        types: &FerroXamlIlWellKnownTypes,
        line_info: &dyn IXamlLineInfo,
    ) -> XamlResult<Rc<dyn IXamlType>> {
        let mut ferro_property_type = Some(field.field_type());
        while let Some(current) = ferro_property_type {
            if current
                .generic_type_definition()
                .is_some_and(|d| d.equals(&*types.ferro_property_t))
            {
                if let Some(argument) = current.generic_arguments().into_iter().next() {
                    return Ok(argument);
                }
            }

            ferro_property_type = current.base_type();
        }

        Err(XamlError::transform_exception(
            format!(
                "{}'s type {} doesn't inherit from  FerroProperty<T>, make sure to use typed properties",
                field.name(),
                field.field_type().to_type_string()
            ),
            Some(line_info),
        ))
    }
}

/// `string.IsNullOrWhiteSpace`.
fn is_null_or_white_space(s: Option<&str>) -> bool {
    s.is_none_or(|s| s.chars().all(char::is_whitespace))
}

/// `string.Equals(a, b, StringComparison.OrdinalIgnoreCase)`.
pub(crate) fn ordinal_ignore_case_equals(a: &str, b: &str) -> bool {
    let mut x = a.chars().flat_map(char::to_uppercase);
    let mut y = b.chars().flat_map(char::to_uppercase);
    loop {
        match (x.next(), y.next()) {
            (None, None) => return true,
            (Some(p), Some(q)) if p == q => {}
            _ => return false,
        }
    }
}

/// A value node that evaluates to a registered property (a `FerroProperty` instance).
///
/// `node is IXamlIlFerroPropertyNode` is `node.cast::<dyn IXamlIlFerroPropertyNode>()`.
pub trait IXamlIlFerroPropertyNode: IXamlAstValueNode {
    /// The value type of the registered property.
    fn ferro_property_type(&self) -> Rc<dyn IXamlType>;

    /// `this is IXamlIlFerroClassPropertyNode`: the marker interface used to identify whether
    /// the registered property represents a style class (`Classes.foo`).
    fn is_xaml_il_ferro_class_property_node(&self) -> bool {
        false
    }
}

impl XamlAstCast for dyn IXamlIlFerroPropertyNode {
    fn kind_name() -> &'static str {
        "IXamlIlFerroPropertyNode"
    }
    fn cast_from(node: Rc<dyn IXamlAstNode>) -> Option<Rc<Self>> {
        query_node_interface::<dyn IXamlIlFerroPropertyNode>(&node)
    }
    fn upcast(this: Rc<Self>) -> Rc<dyn IXamlAstNode> {
        this
    }
}

/// The registered property behind a CLR property (or a failed lookup, see
/// [`XamlIlFerroPropertyHelper::create_node`]).
///
/// # What a back end has to do (upstream IL)
///
/// Stack effect: pushes one value, the registered property, typed [`IXamlAstValueNode::type_`]
/// (the non-generic `FerroProperty` base type).
///
/// Load the static field returned by [`XamlIlFerroPropertyNode::resolve_ferro_property_field`]
/// (`ldsfld`). When there is no such field the node cannot be emitted: upstream throws
/// `XamlLoadException("<Name> is not a FerroProperty")` positioned at this node, which is the
/// error that method returns.
pub struct XamlIlFerroPropertyNode {
    base: XamlAstNode,
    type_: Rc<dyn IXamlAstTypeReference>,
    pub property: Rc<XamlAstClrProperty>,
    ferro_property_type: Rc<dyn IXamlType>,
}

impl XamlIlFerroPropertyNode {
    /// `XamlIlFerroPropertyNode(lineInfo, type, property, propertyType)`.
    pub fn with_property_type(
        line_info: &dyn IXamlLineInfo,
        type_: Rc<dyn IXamlType>,
        property: Rc<XamlAstClrProperty>,
        property_type: Rc<dyn IXamlType>,
    ) -> Rc<Self> {
        Rc::new(Self {
            base: XamlAstNode::new(line_info),
            type_: XamlAstClrTypeReference::new(line_info, type_, false),
            property,
            ferro_property_type: property_type,
        })
    }

    /// `XamlIlFerroPropertyNode(lineInfo, type, property)`: the property type is the getter's
    /// return type or, without a getter, the first parameter of the first setter.
    pub fn new(
        line_info: &dyn IXamlLineInfo,
        type_: Rc<dyn IXamlType>,
        property: Rc<XamlAstClrProperty>,
    ) -> XamlResult<Rc<Self>> {
        let property_type = Self::get_property_type(&property)?;
        Ok(Self::with_property_type(
            line_info,
            type_,
            property,
            property_type,
        ))
    }

    /// The static field to load, or the `XamlLoadException` upstream throws while emitting.
    pub fn resolve_ferro_property_field(&self) -> XamlResult<Rc<dyn IXamlField>> {
        XamlIlFerroPropertyHelper::try_get_ferro_property_field(&self.property).ok_or_else(|| {
            XamlError::load_exception(
                format!("{} is not a FerroProperty", self.property.name()),
                Some(self),
            )
        })
    }

    fn get_property_type(property: &XamlAstClrProperty) -> XamlResult<Rc<dyn IXamlType>> {
        if let Some(getter) = property.getter() {
            return Ok(getter.return_type());
        }
        match property.setters().first() {
            Some(setter) => setter.parameters().into_iter().next().ok_or_else(|| {
                XamlError::internal(
                    "ArgumentOutOfRangeException",
                    "Index was out of range. Must be non-negative and less than the size of the collection.",
                )
            }),
            None => Err(XamlError::invalid_operation(format!(
                "Unable to resolve \"{}.{}\" property type. There is no setter or getter.",
                property.declaring_type().name(),
                property.name()
            ))),
        }
    }
}

xaml_line_info_impl!(XamlIlFerroPropertyNode, base);

impl IXamlAstNode for XamlIlFerroPropertyNode {
    xaml_ast_node_members!("XamlIlFerroPropertyNode", value);

    fn query_interface(self: Rc<Self>, slot: &mut dyn Any) -> bool {
        xaml_query_interface!(self, slot, dyn IXamlIlFerroPropertyNode)
    }
}

impl IXamlAstValueNode for XamlIlFerroPropertyNode {
    fn type_(&self) -> Rc<dyn IXamlAstTypeReference> {
        self.type_.clone()
    }
}

impl IXamlIlFerroPropertyNode for XamlIlFerroPropertyNode {
    fn ferro_property_type(&self) -> Rc<dyn IXamlType> {
        self.ferro_property_type.clone()
    }
}

/// A registered property referenced through its static field (`Owner.FooProperty`).
///
/// # What a back end has to do (upstream IL)
///
/// Stack effect: pushes one value typed as the field's type. Load the static field
/// [`XamlIlFerroPropertyFieldNode::field`] (`ldsfld`).
pub struct XamlIlFerroPropertyFieldNode {
    base: XamlAstNode,
    field: Rc<dyn IXamlField>,
    ferro_property_type: Rc<dyn IXamlType>,
}

impl XamlIlFerroPropertyFieldNode {
    pub fn new(
        types: &FerroXamlIlWellKnownTypes,
        line_info: &dyn IXamlLineInfo,
        field: Rc<dyn IXamlField>,
    ) -> XamlResult<Rc<Self>> {
        let ferro_property_type =
            XamlIlFerroPropertyHelper::get_ferro_property_type(&*field, types, line_info)?;
        Ok(Rc::new(Self {
            base: XamlAstNode::new(line_info),
            field,
            ferro_property_type,
        }))
    }

    /// `_field` (private upstream; exposed for the back ends).
    pub fn field(&self) -> &Rc<dyn IXamlField> {
        &self.field
    }
}

xaml_line_info_impl!(XamlIlFerroPropertyFieldNode, base);

impl IXamlAstNode for XamlIlFerroPropertyFieldNode {
    xaml_ast_node_members!("XamlIlFerroPropertyFieldNode", value);

    fn query_interface(self: Rc<Self>, slot: &mut dyn Any) -> bool {
        xaml_query_interface!(self, slot, dyn IXamlIlFerroPropertyNode)
    }
}

impl IXamlAstValueNode for XamlIlFerroPropertyFieldNode {
    fn type_(&self) -> Rc<dyn IXamlAstTypeReference> {
        XamlAstClrTypeReference::new(self, self.field.field_type(), false)
    }
}

impl IXamlIlFerroPropertyNode for XamlIlFerroPropertyFieldNode {
    fn ferro_property_type(&self) -> Rc<dyn IXamlType> {
        self.ferro_property_type.clone()
    }
}

/// Implemented by the `XamlAstClrProperty` "subclasses" that know the static field of the
/// registered property behind them.
pub trait IXamlIlFerroProperty: 'static {
    fn ferro_property(&self) -> Rc<dyn IXamlField>;
}

/// `property as IXamlIlFerroProperty`.
pub fn as_xaml_il_ferro_property(
    property: &XamlAstClrProperty,
) -> Option<Rc<dyn IXamlIlFerroProperty>> {
    property
        .extension()
        .and_then(|e| query_clr_property_extension_interface::<dyn IXamlIlFerroProperty>(&e))
}

/// `XamlIlFerroProperty : XamlAstClrProperty, IXamlIlFerroProperty`: a CLR property backed by a
/// registered property.
///
/// The derived class is represented by a `XamlAstClrProperty` whose
/// [`XamlAstClrProperty::extension`] is an instance of this type:
/// [`XamlIlFerroProperty::new`] returns that property node, `prop is XamlIlFerroProperty` is
/// [`XamlIlFerroProperty::from_clr_property`].
pub struct XamlIlFerroProperty {
    ferro_property: Rc<dyn IXamlField>,
}

impl XamlIlFerroProperty {
    /// `new XamlIlFerroProperty(original, field, types)`.
    ///
    /// The new property node copies `original` (name, declaring type, getter, setters, custom
    /// attributes, type converters) and gets these setters in front of the original ones, in
    /// this order:
    ///
    /// 1. [`UnsetValueSetter`] (always);
    /// 2. [`SetValueWithPrioritySetter`] (styled and attached properties only);
    /// 3. [`BindingWithPrioritySetter`] (styled and attached properties, unless the property
    ///    has the `AssignBinding` attribute);
    /// 4. [`BindingSetter`] (unless the property has the `AssignBinding` attribute).
    pub fn new(
        original: &Rc<XamlAstClrProperty>,
        field: Rc<dyn IXamlField>,
        types: &Rc<FerroXamlIlWellKnownTypes>,
    ) -> XamlResult<Rc<XamlAstClrProperty>> {
        let rv = XamlAstClrProperty::with_setters(
            &**original,
            &original.name(),
            original.declaring_type(),
            original.getter(),
            Some(original.setters()),
            Some(original.custom_attributes()),
        );
        let assign_binding = original
            .custom_attributes()
            .iter()
            .any(|ca| ca.type_().equals(&*types.assign_binding_attribute));

        let declaring_type = original.declaring_type();
        if !assign_binding {
            rv.setters.borrow_mut().insert(
                0,
                BindingSetter::new(types, declaring_type.clone(), field.clone()),
            );
        }

        // Styled and attached properties can be set with a BindingPriority when they're
        // assigned in a ControlTemplate.
        let field_type = field.field_type();
        let field_type_definition = field_type.generic_type_definition();
        if field_type_definition.as_ref().is_some_and(|d| {
            d.equals(&*types.styled_property_t) || d.equals(&*types.ferro_attached_property_t)
        }) {
            let property_type = field_type.generic_arguments().into_iter().next().ok_or_else(|| {
                XamlError::internal(
                    "ArgumentOutOfRangeException",
                    "Index was out of range. Must be non-negative and less than the size of the collection.",
                )
            })?;
            rv.setters.borrow_mut().insert(
                0,
                SetValueWithPrioritySetter::new(
                    types,
                    declaring_type.clone(),
                    field.clone(),
                    property_type,
                ),
            );
            if !assign_binding {
                rv.setters.borrow_mut().insert(
                    1,
                    BindingWithPrioritySetter::new(types, declaring_type.clone(), field.clone()),
                );
            }
        }

        rv.setters
            .borrow_mut()
            .insert(0, UnsetValueSetter::new(types, declaring_type, field.clone()));
        *rv.type_converters.borrow_mut() = original.type_converters.borrow().clone();
        rv.set_extension(Rc::new(XamlIlFerroProperty {
            ferro_property: field,
        }));
        Ok(rv)
    }

    /// `property as XamlIlFerroProperty`.
    pub fn from_clr_property(property: &XamlAstClrProperty) -> Option<Rc<XamlIlFerroProperty>> {
        property.extension_as::<XamlIlFerroProperty>()
    }

    /// `FerroProperty`: the static field holding the registered property.
    pub fn ferro_property(&self) -> Rc<dyn IXamlField> {
        self.ferro_property.clone()
    }
}

impl IXamlIlFerroProperty for XamlIlFerroProperty {
    fn ferro_property(&self) -> Rc<dyn IXamlField> {
        self.ferro_property.clone()
    }
}

impl IXamlAstClrPropertyExtension for XamlIlFerroProperty {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn into_any_rc(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }
    fn type_name(&self) -> &'static str {
        "XamlIlFerroProperty"
    }
    fn query_interface(self: Rc<Self>, slot: &mut dyn Any) -> bool {
        xaml_query_interface!(self, slot, dyn IXamlIlFerroProperty)
    }
}

/// The state of the upstream abstract `FerroPropertyCustomSetter`: a property setter that
/// assigns a registered property through the property system instead of the CLR setter.
///
/// Two setters are equal when they are of the same class and refer to an equal registered
/// property field.
pub struct FerroPropertyCustomSetter {
    pub types: Rc<FerroXamlIlWellKnownTypes>,
    /// The static field holding the registered property.
    pub ferro_property: Rc<dyn IXamlField>,
    pub target_type: Rc<dyn IXamlType>,
    pub binder_parameters: PropertySetterBinderParameters,
    pub parameters: Vec<Rc<dyn IXamlType>>,
}

impl FerroPropertyCustomSetter {
    fn new(
        types: &Rc<FerroXamlIlWellKnownTypes>,
        declaring_type: Rc<dyn IXamlType>,
        ferro_property: Rc<dyn IXamlField>,
        allow_null: bool,
        parameters: Vec<Rc<dyn IXamlType>>,
    ) -> Self {
        Self {
            types: types.clone(),
            ferro_property,
            target_type: declaring_type,
            parameters,
            binder_parameters: PropertySetterBinderParameters {
                allow_x_null: Cell::new(allow_null),
                allow_runtime_null: Cell::new(allow_null),
                ..PropertySetterBinderParameters::default()
            },
        }
    }
}

macro_rules! ferro_property_custom_setter {
    ($ty:ident) => {
        impl $ty {
            /// The base class state: the registered property field, the well-known types, the
            /// target type and the parameters.
            pub fn base(&self) -> &FerroPropertyCustomSetter {
                &self.base
            }

            /// The static field holding the registered property.
            pub fn ferro_property(&self) -> &Rc<dyn IXamlField> {
                &self.base.ferro_property
            }
        }

        impl IXamlPropertySetter for $ty {
            fn target_type(&self) -> Rc<dyn IXamlType> {
                self.base.target_type.clone()
            }
            fn binder_parameters(&self) -> &PropertySetterBinderParameters {
                &self.base.binder_parameters
            }
            fn parameters(&self) -> Vec<Rc<dyn IXamlType>> {
                self.base.parameters.clone()
            }
            fn custom_attributes(&self) -> Vec<Rc<dyn IXamlCustomAttribute>> {
                Vec::new()
            }
            fn as_any(&self) -> &dyn Any {
                self
            }
            fn type_name(&self) -> &'static str {
                stringify!($ty)
            }
            fn equals(&self, other: &dyn IXamlPropertySetter) -> bool {
                match other.as_any().downcast_ref::<$ty>() {
                    Some(other) => {
                        std::ptr::eq(self, other)
                            || self
                                .base
                                .ferro_property
                                .equals(&*other.base.ferro_property)
                    }
                    None => false,
                }
            }
        }
    };
}

/// Binds the registered property: parameters `[BindingBase]`, null not allowed.
///
/// # What a back end has to do (upstream IL)
///
/// The setter consumes the target object and its arguments and leaves nothing on the stack.
///
/// * `Emit` (stack: target object, binding): store the binding in a temporary, load the static
///   field `ferro_property`, load the binding back, then call
///   `types.ferro_object_bind_method` (`FerroObject.Bind(FerroProperty, BindingBase)`) on the
///   target object and discard its `BindingExpressionBase` result.
/// * `EmitWithArguments` (stack: target object; `arguments[0]` is the binding node): load the
///   static field `ferro_property`, emit `arguments[0]` converted to `parameters[0]`
///   (`BindingBase`), call the same method and discard the result.
pub struct BindingSetter {
    base: FerroPropertyCustomSetter,
}

impl BindingSetter {
    pub fn new(
        types: &Rc<FerroXamlIlWellKnownTypes>,
        declaring_type: Rc<dyn IXamlType>,
        ferro_property: Rc<dyn IXamlField>,
    ) -> Rc<Self> {
        Rc::new(Self {
            base: FerroPropertyCustomSetter::new(
                types,
                declaring_type,
                ferro_property,
                false,
                vec![types.binding_base.clone()],
            ),
        })
    }
}

ferro_property_custom_setter!(BindingSetter);

/// Binds the registered property when the assignment carries a priority: parameters
/// `[BindingPriority, BindingBase]`, null not allowed. The priority is ignored (a binding
/// decides its own priority).
///
/// # What a back end has to do (upstream IL)
///
/// The setter consumes the target object and its arguments and leaves nothing on the stack.
///
/// * `Emit` (stack: target object, priority, binding): store the binding in a temporary, pop
///   and discard the priority, load the static field `ferro_property`, load the binding back,
///   then call `types.ferro_object_bind_method` (`FerroObject.Bind(FerroProperty,
///   BindingBase)`) on the target object and discard the result.
/// * `EmitWithArguments` (stack: target object; `arguments` are `[priority, binding]`): load
///   the static field `ferro_property`, emit `arguments[1]` converted to `parameters[1]`
///   (`BindingBase`), call the same method and discard the result. `arguments[0]` is not
///   emitted at all.
pub struct BindingWithPrioritySetter {
    base: FerroPropertyCustomSetter,
}

impl BindingWithPrioritySetter {
    pub fn new(
        types: &Rc<FerroXamlIlWellKnownTypes>,
        declaring_type: Rc<dyn IXamlType>,
        ferro_property: Rc<dyn IXamlField>,
    ) -> Rc<Self> {
        Rc::new(Self {
            base: FerroPropertyCustomSetter::new(
                types,
                declaring_type,
                ferro_property,
                false,
                vec![types.binding_priority.clone(), types.binding_base.clone()],
            ),
        })
    }
}

ferro_property_custom_setter!(BindingWithPrioritySetter);

/// Sets a styled or attached property with an explicit priority: parameters
/// `[BindingPriority, T]` where `T` is the property's value type; null is allowed when `T`
/// accepts null.
///
/// # What a back end has to do (upstream IL)
///
/// The setter consumes the target object and its arguments and leaves nothing on the stack.
/// The method called is [`SetValueWithPrioritySetter::set_styled_property_value_method`]:
/// `FerroObject.SetValue<T>(StyledProperty<T> property, T value, BindingPriority priority)`
/// instantiated with `T = parameters[1]`; its `IDisposable` result is discarded.
///
/// * `Emit` (stack: target object, priority, value): store the value in a temporary of type
///   `parameters[1]` and the priority in a temporary (`int`), then load the static field
///   `ferro_property`, the value and the priority (in that order) and call the method on the
///   target object.
/// * `EmitWithArguments` (stack: target object; `arguments` are `[priority, value]`): load the
///   static field `ferro_property`, emit `arguments[1]` converted to `parameters[1]`, emit
///   `arguments[0]` converted to `parameters[0]` (the value is evaluated before the priority),
///   then call the method on the target object.
pub struct SetValueWithPrioritySetter {
    base: FerroPropertyCustomSetter,
}

impl SetValueWithPrioritySetter {
    pub fn new(
        types: &Rc<FerroXamlIlWellKnownTypes>,
        declaring_type: Rc<dyn IXamlType>,
        ferro_property: Rc<dyn IXamlField>,
        property_type: Rc<dyn IXamlType>,
    ) -> Rc<Self> {
        let allow_null = property_type.accepts_null();
        Rc::new(Self {
            base: FerroPropertyCustomSetter::new(
                types,
                declaring_type,
                ferro_property,
                allow_null,
                vec![types.binding_priority.clone(), property_type],
            ),
        })
    }

    /// `Types.FerroObjectSetStyledPropertyValue.MakeGenericMethod(Parameters[1])`.
    pub fn set_styled_property_value_method(&self) -> XamlResult<Rc<dyn IXamlMethod>> {
        self.base
            .types
            .ferro_object_set_styled_property_value
            .make_generic_method(&self.base.parameters[1..2])
    }
}

ferro_property_custom_setter!(SetValueWithPrioritySetter);

/// Assigns `FerroProperty.UnsetValue`: parameters `[UnsetValueType]`, null not allowed.
///
/// # What a back end has to do (upstream IL)
///
/// The setter consumes the target object and its arguments and leaves nothing on the stack.
///
/// * `Emit` (stack: target object, unset value instance): pop and discard the instance, then
///   set the value as described below.
/// * `EmitWithArguments` (stack: target object): the argument is not emitted at all; set the
///   value as described below.
///
/// Setting the value: load the static field `ferro_property`, load the static field
/// [`UnsetValueSetter::unset_value_field`] (`FerroProperty.UnsetValue`; the instance passed to
/// the setter is ignored in favour of this field to avoid a temporary), push the integer 0
/// (`BindingPriority.LocalValue`) and call `types.ferro_object_set_value_method`
/// (`FerroObject.SetValue(FerroProperty, object, BindingPriority)`) on the target object,
/// discarding its `IDisposable` result.
pub struct UnsetValueSetter {
    base: FerroPropertyCustomSetter,
}

impl UnsetValueSetter {
    pub fn new(
        types: &Rc<FerroXamlIlWellKnownTypes>,
        declaring_type: Rc<dyn IXamlType>,
        ferro_property: Rc<dyn IXamlField>,
    ) -> Rc<Self> {
        Rc::new(Self {
            base: FerroPropertyCustomSetter::new(
                types,
                declaring_type,
                ferro_property,
                false,
                vec![types.unset_value_type.clone()],
            ),
        })
    }

    /// `Types.FerroProperty.Fields.First(f => f.Name == "UnsetValue")`.
    pub fn unset_value_field(&self) -> XamlResult<Rc<dyn IXamlField>> {
        self.base
            .types
            .ferro_property
            .fields()
            .into_iter()
            .find(|f| f.name() == "UnsetValue")
            .ok_or_else(|| XamlError::invalid_operation("Sequence contains no matching element"))
    }
}

ferro_property_custom_setter!(UnsetValueSetter);

/// `XamlIlFerroClassProperty`: the registered boolean property that stands for a style class
/// (`Classes.foo` in a setter or a selector).
///
/// Upstream the class derives from `XamlAstClrProperty` (name: the class name, declaring type:
/// `Classes`, no getter, no setters) and is at the same time a value node; the embedded
/// [`XamlIlFerroClassProperty::base`] is that property node.
///
/// # What a back end has to do (upstream IL)
///
/// Stack effect: pushes one value typed [`IXamlAstValueNode::type_`] (`FerroProperty<bool>`).
/// Push the string `class_name` and call the static method
/// [`XamlIlFerroClassProperty::method`] (`StyledElementExtensions.GetClassProperty(string)`),
/// whose result is the property.
pub struct XamlIlFerroClassProperty {
    /// The `XamlAstClrProperty` base class instance.
    pub base: Rc<XamlAstClrProperty>,
    method: Rc<dyn IXamlMethod>,
    types: Rc<FerroXamlIlWellKnownTypes>,
    class_name: String,
    type_: Rc<dyn IXamlAstTypeReference>,
    return_type: Rc<dyn IXamlType>,
    ferro_property_type: Rc<dyn IXamlType>,
    parameters: Vec<Rc<dyn IXamlType>>,
    binder_parameters: PropertySetterBinderParameters,
}

impl XamlIlFerroClassProperty {
    pub fn new(
        types: &Rc<FerroXamlIlWellKnownTypes>,
        class_name: &str,
        line_info: &dyn IXamlLineInfo,
    ) -> XamlResult<Rc<Self>> {
        let base = XamlAstClrProperty::new(line_info, class_name, types.classes.clone(), None);
        let return_type = types
            .ferro_property_t
            .make_generic_type(std::slice::from_ref(&types.xaml_il_types.boolean))?;
        Ok(Rc::new(Self {
            parameters: vec![types.xaml_il_types.string.clone()],
            method: types.get_class_property.clone(),
            ferro_property_type: types.xaml_il_types.boolean.clone(),
            types: types.clone(),
            type_: XamlAstClrTypeReference::new(line_info, return_type.clone(), false),
            return_type,
            class_name: class_name.to_string(),
            binder_parameters: PropertySetterBinderParameters::default(),
            base,
        }))
    }

    pub fn parameters(&self) -> Vec<Rc<dyn IXamlType>> {
        self.parameters.clone()
    }

    pub fn binder_parameters(&self) -> &PropertySetterBinderParameters {
        &self.binder_parameters
    }

    /// `_method`: `StyledElementExtensions.GetClassProperty(string)`.
    pub fn method(&self) -> &Rc<dyn IXamlMethod> {
        &self.method
    }

    /// `_types` (private upstream; exposed for the back ends).
    pub fn types(&self) -> &Rc<FerroXamlIlWellKnownTypes> {
        &self.types
    }

    /// `_className`: the style class name.
    pub fn class_name(&self) -> &str {
        &self.class_name
    }

    /// `_returnType`: `FerroProperty<bool>`.
    pub fn return_type(&self) -> &Rc<dyn IXamlType> {
        &self.return_type
    }
}

impl IXamlLineInfo for XamlIlFerroClassProperty {
    fn line(&self) -> i32 {
        self.base.line()
    }
    fn position(&self) -> i32 {
        self.base.position()
    }
    fn set_line(&self, value: i32) {
        self.base.set_line(value)
    }
    fn set_position(&self, value: i32) {
        self.base.set_position(value)
    }
}

impl IXamlAstNode for XamlIlFerroClassProperty {
    xaml_ast_node_members!("XamlIlFerroClassProperty", value, property_reference);

    fn to_node_string(&self) -> String {
        self.base.to_node_string()
    }

    fn query_interface(self: Rc<Self>, slot: &mut dyn Any) -> bool {
        xaml_query_interface!(self, slot, dyn IXamlIlFerroPropertyNode)
    }
}

impl xamlx::ast::IXamlAstPropertyReference for XamlIlFerroClassProperty {}

impl IXamlAstValueNode for XamlIlFerroClassProperty {
    fn type_(&self) -> Rc<dyn IXamlAstTypeReference> {
        self.type_.clone()
    }
}

impl IXamlIlFerroPropertyNode for XamlIlFerroClassProperty {
    fn ferro_property_type(&self) -> Rc<dyn IXamlType> {
        self.ferro_property_type.clone()
    }

    fn is_xaml_il_ferro_class_property_node(&self) -> bool {
        true
    }
}

#[cfg(test)]
#[path = "xaml_il_ferro_property_helper_tests.rs"]
mod tests;

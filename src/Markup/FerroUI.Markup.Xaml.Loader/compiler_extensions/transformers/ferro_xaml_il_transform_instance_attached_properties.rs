//! Port of `CompilerExtensions/Transformers/FerroXamlIlTransformInstanceAttachedProperties.cs`.

use std::any::Any;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::rc::{Rc, Weak};

use xamlx::ast::{
    IXamlAstClrPropertyExtension, IXamlAstNode, IXamlPropertySetter,
    PropertySetterBinderParameters, XamlAstClrProperty, XamlAstClrTypeReference,
    XamlAstNamePropertyReference, XamlAstNodeExtensions,
};
use xamlx::exceptions::{XamlError, XamlResult};
use xamlx::transform::{AstTransformationContext, IXamlAstTransformer, TransformerConfiguration};
use xamlx::type_system::{
    AnonymousParameterInfo, IXamlCustomAttribute, IXamlField, IXamlMember, IXamlMethod,
    IXamlParameterInfo, IXamlType,
};
use xamlx::xaml_query_interface;

use super::FerroXamlIlWellKnownTypesExtensions;
use crate::compiler_extensions::IXamlIlFerroProperty;

/// Lets a registered (non-attached) property of one `FerroObject` class be set on an instance
/// of an unrelated `FerroObject` class with the attached property syntax
/// (`<Border TextBlock.Foreground="Red"/>`): the reference becomes a
/// [`FerroAttachedInstanceProperty`], which reads and writes the value through
/// `FerroObject.GetValue`/`SetValue`.
pub struct FerroXamlIlTransformInstanceAttachedProperties;

impl IXamlAstTransformer for FerroXamlIlTransformInstanceAttachedProperties {
    fn transform(
        &self,
        context: &AstTransformationContext,
        node: Rc<dyn IXamlAstNode>,
    ) -> XamlResult<Rc<dyn IXamlAstNode>> {
        let Some(prop) = node.cast::<XamlAstNamePropertyReference>() else {
            return Ok(node);
        };
        let target_ref = prop.target_type.borrow().clone().cast::<XamlAstClrTypeReference>();
        let declaring_ref = prop
            .declaring_type
            .borrow()
            .clone()
            .cast::<XamlAstClrTypeReference>();
        let (Some(target_ref), Some(declaring_ref)) = (target_ref, declaring_ref) else {
            return Ok(node);
        };

        // Target and declared type aren't assignable but both inherit from FerroObject
        let ferro_object = context.try_get_ferro_types()?.ferro_object.clone();
        if !(ferro_object.is_assignable_from(&*target_ref.type_)
            && ferro_object.is_assignable_from(&*declaring_ref.type_)
            && !declaring_ref.type_.is_assignable_from(&*target_ref.type_))
        {
            return Ok(node);
        }

        // Instance property
        let prop_name = prop.name();
        let clr_prop = declaring_ref
            .type_
            .get_all_properties()
            .into_iter()
            .find(|p| p.name() == prop_name);
        let Some(clr_prop) = clr_prop else {
            return Ok(node);
        };
        if !(clr_prop.getter().is_some_and(|g| !g.is_static())
            || clr_prop.setter().is_some_and(|s| !s.is_static()))
        {
            return Ok(node);
        }

        let declaring_type = clr_prop.declaring_type();
        let ferro_property_field_name = format!("{prop_name}Property");
        let ferro_property_field = declaring_type
            .fields()
            .into_iter()
            .find(|f| f.is_static() && f.name() == ferro_property_field_name);
        let Some(ferro_property_field) = ferro_property_field else {
            return Ok(node);
        };

        let mut ferro_property_type = Some(ferro_property_field.field_type());
        while let Some(current) = ferro_property_type.clone() {
            let name = current.name();
            if current.namespace().as_deref() == Some("FerroUI")
                && (name == "FerroProperty" || name == "FerroProperty`1")
            {
                break;
            }

            // Attached properties are handled by vanilla XamlIl
            if name.starts_with("AttachedProperty") {
                return Ok(node);
            }

            ferro_property_type = current.base_type();
        }

        let Some(ferro_property_type) = ferro_property_type else {
            return Ok(node);
        };

        let generic_arguments = ferro_property_type.generic_arguments();
        if generic_arguments.len() > 1 {
            return Ok(node);
        }

        let property_type = match generic_arguments.into_iter().next() {
            Some(argument) => argument,
            None => context.configuration().well_known_types().object.clone(),
        };

        Ok(FerroAttachedInstanceProperty::new(
            &prop,
            context.configuration(),
            declaring_type,
            property_type,
            ferro_property_type,
            ferro_object,
            ferro_property_field,
        ))
    }
}

/// `FerroAttachedInstanceProperty : XamlAstClrProperty, IXamlIlFerroProperty`: a registered
/// property of another class used on an unrelated `FerroObject`.
///
/// The derived class is represented by a `XamlAstClrProperty` whose
/// [`XamlAstClrProperty::extension`] is an instance of this type:
/// [`FerroAttachedInstanceProperty::new`] returns that property node. Its only setter is a
/// [`FerroAttachedInstancePropertySetterMethod`] and its getter a
/// [`FerroAttachedInstancePropertyGetterMethod`].
pub struct FerroAttachedInstanceProperty {
    /// The type that declares the CLR property (and the registered property field).
    pub declaring_type: Rc<dyn IXamlType>,
    /// The non-generic registered property type the `GetValue`/`SetValue` overloads are looked
    /// up with: the field's `FerroProperty` base. `None` when the generic property type has no
    /// base type (upstream stores `null` and fails to find the methods when emitting).
    pub ferro_property_type: Option<Rc<dyn IXamlType>>,
    pub ferro_object: Rc<dyn IXamlType>,
    /// The static field holding the registered property.
    pub field: Rc<dyn IXamlField>,
    /// `PropertyType`: the value type of the property.
    pub property_type: Rc<dyn IXamlType>,
    /// `System.Object`, taken from the configuration upstream keeps a reference to.
    pub object_type: Rc<dyn IXamlType>,
}

impl FerroAttachedInstanceProperty {
    /// `new FerroAttachedInstanceProperty(prop, config, declaringType, type, ferroPropertyType,
    /// ferroObject, field)`: the property node, named like `prop` and declared by
    /// `declaring_type`.
    pub fn new(
        prop: &Rc<XamlAstNamePropertyReference>,
        config: &TransformerConfiguration,
        declaring_type: Rc<dyn IXamlType>,
        type_: Rc<dyn IXamlType>,
        ferro_property_type: Rc<dyn IXamlType>,
        ferro_object: Rc<dyn IXamlType>,
        field: Rc<dyn IXamlField>,
    ) -> Rc<XamlAstClrProperty> {
        // XamlIl doesn't support generic methods yet
        let ferro_property_type = if !ferro_property_type.generic_arguments().is_empty() {
            ferro_property_type.base_type()
        } else {
            Some(ferro_property_type)
        };

        let extension = Rc::new(FerroAttachedInstanceProperty {
            declaring_type: declaring_type.clone(),
            ferro_property_type,
            ferro_object,
            field,
            property_type: type_,
            object_type: config.well_known_types().object.clone(),
        });

        let rv = XamlAstClrProperty::new(&**prop, &prop.name(), declaring_type, None);
        rv.set_extension(extension.clone());
        rv.setters
            .borrow_mut()
            .push(Rc::new(FerroAttachedInstancePropertySetterMethod {
                parent: extension.clone(),
                parent_node: Rc::downgrade(&rv),
                binder_parameters: PropertySetterBinderParameters::default(),
                parameters: vec![
                    extension.ferro_object.clone(),
                    extension.property_type.clone(),
                ],
            }));
        *rv.getter.borrow_mut() = Some(Rc::new(FerroAttachedInstancePropertyGetterMethod {
            name: format!("FerroObject:GetValue_{}", prop.name()),
            parameters: vec![extension.ferro_object.clone()],
            parent: extension,
        }));
        rv
    }

    /// `property as FerroAttachedInstanceProperty`.
    pub fn from_clr_property(
        property: &XamlAstClrProperty,
    ) -> Option<Rc<FerroAttachedInstanceProperty>> {
        property.extension_as::<FerroAttachedInstanceProperty>()
    }

    /// The method the upstream setter IL looks up: the public instance method
    /// `SetValue(FerroProperty, object, BindingPriority)` of `FerroObject` (three parameters:
    /// the registered property type, `object`, an enum).
    pub fn set_value_method(&self) -> XamlResult<Rc<dyn IXamlMethod>> {
        let so = &self.object_type;
        self.ferro_object
            .find_method(|m| {
                let parameters = m.parameters();
                m.is_public()
                    && !m.is_static()
                    && m.name() == "SetValue"
                    && parameters.len() == 3
                    && self
                        .ferro_property_type
                        .as_ref()
                        .is_some_and(|t| parameters[0].equals(&**t))
                    && parameters[1].equals(&**so)
                    && parameters[2].is_enum()
            })
            .ok_or_else(|| {
                XamlError::type_system_exception(
                    "Unable to find SetValue(FerroProperty, object, BindingPriority) on FerroObject",
                )
            })
    }

    /// The method the upstream getter IL looks up: the public instance method
    /// `GetValue(FerroProperty)` of `FerroObject` (one parameter of the registered property
    /// type).
    pub fn get_value_method(&self) -> XamlResult<Rc<dyn IXamlMethod>> {
        self.ferro_object
            .find_method(|m| {
                let parameters = m.parameters();
                m.is_public()
                    && !m.is_static()
                    && m.name() == "GetValue"
                    && parameters.len() == 1
                    && self
                        .ferro_property_type
                        .as_ref()
                        .is_some_and(|t| parameters[0].equals(&**t))
            })
            .ok_or_else(|| {
                XamlError::type_system_exception(
                    "Unable to find T GetValue<T>(FerroProperty<T>) on FerroObject",
                )
            })
    }
}

impl IXamlIlFerroProperty for FerroAttachedInstanceProperty {
    fn ferro_property(&self) -> Rc<dyn IXamlField> {
        self.field.clone()
    }
}

impl IXamlAstClrPropertyExtension for FerroAttachedInstanceProperty {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn into_any_rc(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }
    fn type_name(&self) -> &'static str {
        "FerroAttachedInstanceProperty"
    }
    fn query_interface(self: Rc<Self>, slot: &mut dyn Any) -> bool {
        xaml_query_interface!(self, slot, dyn IXamlIlFerroProperty)
    }
}

/// `FerroAttachedInstanceProperty.SetterMethod`: target type is the declaring type of the
/// property, parameters `[FerroObject, PropertyType]`, default binder parameters. The custom
/// attributes are those of the property node. Setters compare by identity.
///
/// # What a back end has to do (upstream IL)
///
/// `Emit` (stack: target `FerroObject`, value of `PropertyType`; leaves nothing):
///
/// 1. store the value in a temporary of type `parent.property_type`;
/// 2. load the static field `parent.field` (the registered property);
/// 3. load the value back and, when `parent.property_type` is a value type, box it;
/// 4. load the `int` constant `0` (`BindingPriority.LocalValue`);
/// 5. call [`FerroAttachedInstanceProperty::set_value_method`] on the target and discard a
///    non-void result.
///
/// That is `target.SetValue(Field, (object)value, BindingPriority.LocalValue)`. A missing
/// method is a type system error raised while emitting.
pub struct FerroAttachedInstancePropertySetterMethod {
    pub parent: Rc<FerroAttachedInstanceProperty>,
    parent_node: Weak<XamlAstClrProperty>,
    pub binder_parameters: PropertySetterBinderParameters,
    pub parameters: Vec<Rc<dyn IXamlType>>,
}

impl IXamlPropertySetter for FerroAttachedInstancePropertySetterMethod {
    fn target_type(&self) -> Rc<dyn IXamlType> {
        match self.parent_node.upgrade() {
            Some(node) => node.declaring_type(),
            None => self.parent.declaring_type.clone(),
        }
    }
    fn binder_parameters(&self) -> &PropertySetterBinderParameters {
        &self.binder_parameters
    }
    fn parameters(&self) -> Vec<Rc<dyn IXamlType>> {
        self.parameters.clone()
    }
    fn custom_attributes(&self) -> Vec<Rc<dyn IXamlCustomAttribute>> {
        match self.parent_node.upgrade() {
            Some(node) => node.custom_attributes(),
            None => Vec::new(),
        }
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn type_name(&self) -> &'static str {
        "SetterMethod"
    }
}

/// `FerroAttachedInstanceProperty.GetterMethod`: a public static pseudo method
/// `PropertyType FerroObject:GetValue_<Name>(FerroObject)` declared by the property's
/// declaring type (whose custom attributes it reports). Two getters are equal when their names
/// and declaring types are.
///
/// # What a back end has to do (upstream IL)
///
/// `EmitCall` (stack: target `FerroObject`; leaves the value):
///
/// 1. load the static field `parent.field` (the registered property);
/// 2. call [`FerroAttachedInstanceProperty::get_value_method`] on the target (it returns
///    `object`);
/// 3. when `parent.property_type` is a value type, unbox the result to it (`unbox.any`).
///
/// That is `(PropertyType) target.GetValue(Field)`. A missing method is a type system error
/// raised while emitting.
pub struct FerroAttachedInstancePropertyGetterMethod {
    pub parent: Rc<FerroAttachedInstanceProperty>,
    name: String,
    parameters: Vec<Rc<dyn IXamlType>>,
}

impl IXamlMember for FerroAttachedInstancePropertyGetterMethod {
    fn name(&self) -> String {
        self.name.clone()
    }
    fn declaring_type(&self) -> Rc<dyn IXamlType> {
        self.parent.declaring_type.clone()
    }
}

impl IXamlMethod for FerroAttachedInstancePropertyGetterMethod {
    fn is_public(&self) -> bool {
        true
    }
    fn is_private(&self) -> bool {
        false
    }
    fn is_family(&self) -> bool {
        false
    }
    fn is_static(&self) -> bool {
        true
    }
    fn contains_generic_parameters(&self) -> bool {
        false
    }
    fn is_generic_method(&self) -> bool {
        false
    }
    fn is_generic_method_definition(&self) -> bool {
        false
    }
    fn return_type(&self) -> Rc<dyn IXamlType> {
        self.parent.property_type.clone()
    }
    fn parameters(&self) -> Vec<Rc<dyn IXamlType>> {
        self.parameters.clone()
    }
    fn make_generic_method(
        &self,
        _type_arguments: &[Rc<dyn IXamlType>],
    ) -> XamlResult<Rc<dyn IXamlMethod>> {
        Err(XamlError::not_supported(
            "Specified method is not supported.",
        ))
    }
    fn custom_attributes(&self) -> Vec<Rc<dyn IXamlCustomAttribute>> {
        self.parent.declaring_type.custom_attributes()
    }
    fn get_parameter_info(&self, index: usize) -> XamlResult<Rc<dyn IXamlParameterInfo>> {
        let parameter = self.parameters.get(index).ok_or_else(|| {
            XamlError::internal(
                "ArgumentOutOfRangeException",
                "Index was out of range. Must be non-negative and less than the size of the collection.",
            )
        })?;
        Ok(Rc::new(AnonymousParameterInfo::with_index(
            parameter.clone(),
            index,
        )))
    }
    fn generic_parameters(&self) -> Vec<Rc<dyn IXamlType>> {
        Vec::new()
    }
    fn generic_arguments(&self) -> Vec<Rc<dyn IXamlType>> {
        Vec::new()
    }
    fn equals(&self, other: &dyn IXamlMethod) -> bool {
        match other
            .as_any()
            .downcast_ref::<FerroAttachedInstancePropertyGetterMethod>()
        {
            Some(m) => {
                m.name == self.name && m.declaring_type().equals(&*self.declaring_type())
            }
            None => false,
        }
    }
    fn get_hash_code(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.name.hash(&mut hasher);
        hasher.finish()
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

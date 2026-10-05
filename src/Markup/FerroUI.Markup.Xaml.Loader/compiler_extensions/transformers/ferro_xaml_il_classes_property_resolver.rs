//! Port of `CompilerExtensions/Transformers/FerroXamlIlClassesPropertyResolver.cs`.

use std::any::Any;
use std::cell::Cell;
use std::rc::Rc;

use xamlx::ast::{
    IXamlAstNode, IXamlPropertySetter, PropertySetterBinderParameters, XamlAstClrProperty,
    XamlAstClrTypeReference, XamlAstNamePropertyReference, XamlAstNodeExtensions,
};
use xamlx::exceptions::XamlResult;
use xamlx::transform::{AstTransformationContext, IXamlAstTransformer};
use xamlx::type_system::{
    FindMethodMethodSignature, IXamlCustomAttribute, IXamlMethod, IXamlType,
};

use super::{FerroXamlIlWellKnownTypes, FerroXamlIlWellKnownTypesExtensions};

/// Resolves `Classes.<name>` property references on styled elements
/// (`<Button Classes.accent="True"/>`, `<Button Classes.accent="{Binding IsAccent}"/>`) to a
/// property named `class:<name>` declared by `Classes` with two setters: [`ClassValueSetter`]
/// and [`ClassBindingSetter`].
pub struct FerroXamlIlResolveClassesPropertiesTransformer;

impl IXamlAstTransformer for FerroXamlIlResolveClassesPropertiesTransformer {
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
        if let (Some(target_ref), Some(declaring_ref)) = (target_ref, declaring_ref) {
            let types = context.try_get_ferro_types()?;
            if types.styled_element.is_assignable_from(&*target_ref.type_)
                && types.classes.equals(&*declaring_ref.type_)
            {
                let name = prop.name();
                let setters: Vec<Rc<dyn IXamlPropertySetter>> = vec![
                    ClassValueSetter::new(&types, &name),
                    ClassBindingSetter::new(&types, &name),
                ];
                return Ok(XamlAstClrProperty::with_setters(
                    &*node,
                    &format!("class:{name}"),
                    types.classes.clone(),
                    None,
                    Some(setters),
                    None,
                ));
            }
        }
        Ok(node)
    }
}

fn no_x_null_binder_parameters() -> PropertySetterBinderParameters {
    PropertySetterBinderParameters {
        allow_x_null: Cell::new(false),
        ..PropertySetterBinderParameters::default()
    }
}

/// Adds or removes a style class: target type `StyledElement`, parameters `[bool]`, `x:Null`
/// not allowed. Setters compare by identity.
///
/// # What a back end has to do (upstream IL)
///
/// `Emit` (stack: target styled element, boolean value; leaves nothing):
///
/// 1. store the boolean in a temporary;
/// 2. call the getter of `types.styled_element_classes_property` (`StyledElement.Classes`) on
///    the target, leaving the `Classes` collection;
/// 3. load the string constant [`ClassValueSetter::class_name`];
/// 4. load the boolean back;
/// 5. call [`ClassValueSetter::classes_set_method`]: `void Classes.Set(string name, bool value)`
///    on the collection.
///
/// That is `target.Classes.Set(class_name, value)`.
pub struct ClassValueSetter {
    pub types: Rc<FerroXamlIlWellKnownTypes>,
    pub class_name: String,
    pub binder_parameters: PropertySetterBinderParameters,
    pub parameters: Vec<Rc<dyn IXamlType>>,
}

impl ClassValueSetter {
    pub fn new(types: &Rc<FerroXamlIlWellKnownTypes>, class_name: &str) -> Rc<Self> {
        Rc::new(Self {
            types: types.clone(),
            class_name: class_name.to_string(),
            binder_parameters: no_x_null_binder_parameters(),
            parameters: vec![types.xaml_il_types.boolean.clone()],
        })
    }

    /// The method the upstream IL looks up while emitting: the instance method
    /// `void Set(string, bool)` of `Classes`.
    pub fn classes_set_method(&self) -> XamlResult<Rc<dyn IXamlMethod>> {
        let well_known = &self.types.xaml_il_types;
        self.types
            .classes
            .get_method_by_signature(&FindMethodMethodSignature::new(
                "Set",
                well_known.void.clone(),
                vec![well_known.string.clone(), well_known.boolean.clone()],
            ))
    }
}

impl IXamlPropertySetter for ClassValueSetter {
    fn target_type(&self) -> Rc<dyn IXamlType> {
        self.types.styled_element.clone()
    }
    fn binder_parameters(&self) -> &PropertySetterBinderParameters {
        &self.binder_parameters
    }
    fn parameters(&self) -> Vec<Rc<dyn IXamlType>> {
        self.parameters.clone()
    }
    fn custom_attributes(&self) -> Vec<Rc<dyn IXamlCustomAttribute>> {
        Vec::new()
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn type_name(&self) -> &'static str {
        "ClassValueSetter"
    }
}

/// Binds the presence of a style class to a binding: target type `StyledElement`, parameters
/// `[BindingBase]`, `x:Null` not allowed. Setters compare by identity.
///
/// # What a back end has to do (upstream IL)
///
/// `Emit` (stack: target styled element, binding; leaves nothing):
///
/// 1. store the binding in a temporary;
/// 2. load the string constant [`ClassBindingSetter::class_name`];
/// 3. load the binding back;
/// 4. load `null` (the anchor argument);
/// 5. call the static method `types.classes_bind_method`:
///    `IDisposable StyledElementExtensions.BindClass(StyledElement target, string className,
///    BindingBase source, object anchor)` and discard its result.
///
/// That is `StyledElementExtensions.BindClass(target, class_name, binding, null)`.
pub struct ClassBindingSetter {
    pub types: Rc<FerroXamlIlWellKnownTypes>,
    pub class_name: String,
    pub binder_parameters: PropertySetterBinderParameters,
    pub parameters: Vec<Rc<dyn IXamlType>>,
}

impl ClassBindingSetter {
    pub fn new(types: &Rc<FerroXamlIlWellKnownTypes>, class_name: &str) -> Rc<Self> {
        Rc::new(Self {
            types: types.clone(),
            class_name: class_name.to_string(),
            binder_parameters: no_x_null_binder_parameters(),
            parameters: vec![types.binding_base.clone()],
        })
    }
}

impl IXamlPropertySetter for ClassBindingSetter {
    fn target_type(&self) -> Rc<dyn IXamlType> {
        self.types.styled_element.clone()
    }
    fn binder_parameters(&self) -> &PropertySetterBinderParameters {
        &self.binder_parameters
    }
    fn parameters(&self) -> Vec<Rc<dyn IXamlType>> {
        self.parameters.clone()
    }
    fn custom_attributes(&self) -> Vec<Rc<dyn IXamlCustomAttribute>> {
        Vec::new()
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn type_name(&self) -> &'static str {
        "ClassBindingSetter"
    }
}

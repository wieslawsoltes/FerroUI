//! Port of `Transform/Transformers/ResolvePropertyValueAddersTransformer.cs`.

use std::any::Any;
use std::cell::Cell;
use std::rc::Rc;

use crate::ast::{
    IXamlAstNode, IXamlPropertySetter, PropertySetterBinderParameters, XamlAstClrProperty,
    XamlAstNodeExtensions,
};
use crate::exceptions::{XamlError, XamlResult};
use crate::transform::{AstTransformationContext, IXamlAstTransformer, XamlTransformHelpers};
use crate::type_system::{IXamlCustomAttribute, IXamlMethod, IXamlType};

pub struct ResolvePropertyValueAddersTransformer;

impl IXamlAstTransformer for ResolvePropertyValueAddersTransformer {
    fn transform(
        &self,
        context: &AstTransformationContext,
        node: Rc<dyn IXamlAstNode>,
    ) -> XamlResult<Rc<dyn IXamlAstNode>> {
        if let Some(prop) = node.cast::<XamlAstClrProperty>() {
            if let Some(getter) = prop.getter() {
                for adder in
                    XamlTransformHelpers::find_possible_adders(context, &getter.return_type())?
                {
                    let setter = AdderSetter::new(getter.clone(), adder)?;
                    prop.setters.borrow_mut().push(setter);
                }
            }
        }

        Ok(node)
    }
}

/// A setter that adds the value to the collection returned by the property getter:
/// `target.get_Property().Add(args...)`. (Private nested class upstream; public here so that
/// emitter backends can recognize it.)
pub struct AdderSetter {
    getter: Rc<dyn IXamlMethod>,
    adder: Rc<dyn IXamlMethod>,
    target_type: Rc<dyn IXamlType>,
    binder_parameters: PropertySetterBinderParameters,
    parameters: Vec<Rc<dyn IXamlType>>,
}

impl AdderSetter {
    pub fn new(getter: Rc<dyn IXamlMethod>, adder: Rc<dyn IXamlMethod>) -> XamlResult<Rc<Self>> {
        let target_type = getter.declaring_type();
        let parameters: Vec<Rc<dyn IXamlType>> =
            adder.parameters_with_this().into_iter().skip(1).collect();

        let last = parameters.last().ok_or_else(|| {
            XamlError::internal(
                "ArgumentOutOfRangeException",
                "Adder method doesn't have a value parameter",
            )
        })?;
        let allow_null = last.accepts_null();
        Ok(Rc::new(Self {
            getter,
            adder,
            target_type,
            binder_parameters: PropertySetterBinderParameters {
                allow_multiple: Cell::new(true),
                allow_x_null: Cell::new(allow_null),
                allow_runtime_null: Cell::new(allow_null),
                allow_attribute_syntax: Cell::new(false),
            },
            parameters,
        }))
    }

    pub fn getter(&self) -> &Rc<dyn IXamlMethod> {
        &self.getter
    }

    pub fn adder(&self) -> &Rc<dyn IXamlMethod> {
        &self.adder
    }
}

impl IXamlPropertySetter for AdderSetter {
    fn target_type(&self) -> Rc<dyn IXamlType> {
        self.target_type.clone()
    }
    fn binder_parameters(&self) -> &PropertySetterBinderParameters {
        &self.binder_parameters
    }
    fn parameters(&self) -> Vec<Rc<dyn IXamlType>> {
        self.parameters.clone()
    }
    fn custom_attributes(&self) -> Vec<Rc<dyn IXamlCustomAttribute>> {
        self.adder.custom_attributes()
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn type_name(&self) -> &'static str {
        "AdderSetter"
    }
    fn equals(&self, other: &dyn IXamlPropertySetter) -> bool {
        match other.as_any().downcast_ref::<AdderSetter>() {
            Some(other) => self.getter.equals(&*other.getter) && self.adder.equals(&*other.adder),
            None => false,
        }
    }
}

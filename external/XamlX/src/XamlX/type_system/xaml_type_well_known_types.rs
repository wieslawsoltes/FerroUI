//! Port of `TypeSystem/XamlTypeWellKnownTypes.cs`.

use std::rc::Rc;

use crate::exceptions::XamlResult;

use super::{get_type, IXamlType, IXamlTypeSystem};

pub struct XamlTypeWellKnownTypes {
    action_of_t: Vec<Rc<dyn IXamlType>>,
    func_of_t: Vec<Rc<dyn IXamlType>>,

    pub action: Rc<dyn IXamlType>,
    pub array: Rc<dyn IXamlType>,
    pub boolean: Rc<dyn IXamlType>,
    pub culture_info: Rc<dyn IXamlType>,
    pub delegate: Rc<dyn IXamlType>,
    pub dictionary_of_t2: Rc<dyn IXamlType>,
    pub double: Rc<dyn IXamlType>,
    pub i_disposable: Rc<dyn IXamlType>,
    pub i_enumerable: Rc<dyn IXamlType>,
    pub i_enumerable_of_t: Rc<dyn IXamlType>,
    pub i_enumerator: Rc<dyn IXamlType>,
    pub i_enumerator_of_t: Rc<dyn IXamlType>,
    pub i_format_provider: Rc<dyn IXamlType>,
    pub i_list: Rc<dyn IXamlType>,
    pub i_list_of_t: Rc<dyn IXamlType>,
    pub i_read_only_list_of_t: Rc<dyn IXamlType>,
    pub int32: Rc<dyn IXamlType>,
    pub int_ptr: Rc<dyn IXamlType>,
    pub invalid_cast_exception: Rc<dyn IXamlType>,
    pub list_of_t: Rc<dyn IXamlType>,
    pub method_info: Rc<dyn IXamlType>,
    pub multicast_delegate: Rc<dyn IXamlType>,
    pub not_supported_exception: Rc<dyn IXamlType>,
    pub null_reference_exception: Rc<dyn IXamlType>,
    pub nullable_t: Rc<dyn IXamlType>,
    pub object: Rc<dyn IXamlType>,
    pub obsolete_attribute: Rc<dyn IXamlType>,
    pub string: Rc<dyn IXamlType>,
    pub type_: Rc<dyn IXamlType>,
    pub uri: Rc<dyn IXamlType>,
    pub void: Rc<dyn IXamlType>,
    pub experimental_attribute: Option<Rc<dyn IXamlType>>,
}

impl XamlTypeWellKnownTypes {
    /// `GetActionOfT(int typeParamCount)`; `type_param_count` must be in `1..=16`.
    pub fn get_action_of_t(&self, type_param_count: usize) -> Rc<dyn IXamlType> {
        self.action_of_t[type_param_count - 1].clone()
    }

    /// `GetFuncOfT(int typeParamCount)`; `type_param_count` must be in `1..=17`.
    pub fn get_func_of_t(&self, type_param_count: usize) -> Rc<dyn IXamlType> {
        self.func_of_t[type_param_count - 1].clone()
    }

    pub fn new<S: IXamlTypeSystem + ?Sized>(type_system: &S) -> XamlResult<Self> {
        let mut action_of_t = Vec::with_capacity(16);
        for c in 1..=16 {
            action_of_t.push(get_type(type_system, &format!("System.Action`{c}"))?);
        }
        let mut func_of_t = Vec::with_capacity(17);
        for c in 1..=17 {
            func_of_t.push(get_type(type_system, &format!("System.Func`{c}"))?);
        }
        Ok(Self {
            action_of_t,
            func_of_t,
            action: get_type(type_system, "System.Action")?,
            array: get_type(type_system, "System.Array")?,
            boolean: get_type(type_system, "System.Boolean")?,
            culture_info: get_type(type_system, "System.Globalization.CultureInfo")?,
            delegate: get_type(type_system, "System.Delegate")?,
            dictionary_of_t2: get_type(type_system, "System.Collections.Generic.Dictionary`2")?,
            double: get_type(type_system, "System.Double")?,
            i_disposable: get_type(type_system, "System.IDisposable")?,
            i_enumerable: get_type(type_system, "System.Collections.IEnumerable")?,
            i_enumerable_of_t: get_type(type_system, "System.Collections.Generic.IEnumerable`1")?,
            i_enumerator: get_type(type_system, "System.Collections.IEnumerator")?,
            i_enumerator_of_t: get_type(type_system, "System.Collections.Generic.IEnumerator`1")?,
            i_format_provider: get_type(type_system, "System.IFormatProvider")?,
            i_list: get_type(type_system, "System.Collections.IList")?,
            i_list_of_t: get_type(type_system, "System.Collections.Generic.IList`1")?,
            i_read_only_list_of_t: get_type(
                type_system,
                "System.Collections.Generic.IReadOnlyList`1",
            )?,
            int32: get_type(type_system, "System.Int32")?,
            int_ptr: get_type(type_system, "System.IntPtr")?,
            invalid_cast_exception: get_type(type_system, "System.InvalidCastException")?,
            list_of_t: get_type(type_system, "System.Collections.Generic.List`1")?,
            method_info: get_type(type_system, "System.Reflection.MethodInfo")?,
            multicast_delegate: get_type(type_system, "System.MulticastDelegate")?,
            not_supported_exception: get_type(type_system, "System.NotSupportedException")?,
            null_reference_exception: get_type(type_system, "System.NullReferenceException")?,
            nullable_t: get_type(type_system, "System.Nullable`1")?,
            object: get_type(type_system, "System.Object")?,
            obsolete_attribute: get_type(type_system, "System.ObsoleteAttribute")?,
            string: get_type(type_system, "System.String")?,
            type_: get_type(type_system, "System.Type")?,
            uri: get_type(type_system, "System.Uri")?,
            void: get_type(type_system, "System.Void")?,
            experimental_attribute: type_system
                .find_type("System.Diagnostics.CodeAnalysis.ExperimentalAttribute"),
        })
    }
}

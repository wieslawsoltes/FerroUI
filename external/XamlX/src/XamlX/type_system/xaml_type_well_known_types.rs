//! Port of `TypeSystem/XamlTypeWellKnownTypes.cs`.

use std::rc::Rc;

use crate::exceptions::XamlResult;

use super::{get_type, get_types, IXamlType, IXamlTypeSystem};

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
        macro_rules! resolve {
            ($($field:ident => $name:literal),+ $(,)?) => {
                let [$($field),+] = get_types([$($name),+], |name| get_type(type_system, name))?;
            }
        }
        resolve! {
            action => "System.Action",
            array => "System.Array",
            boolean => "System.Boolean",
            culture_info => "System.Globalization.CultureInfo",
            delegate => "System.Delegate",
            dictionary_of_t2 => "System.Collections.Generic.Dictionary`2",
            double => "System.Double",
            i_disposable => "System.IDisposable",
            i_enumerable => "System.Collections.IEnumerable",
            i_enumerable_of_t => "System.Collections.Generic.IEnumerable`1",
            i_enumerator => "System.Collections.IEnumerator",
            i_enumerator_of_t => "System.Collections.Generic.IEnumerator`1",
            i_format_provider => "System.IFormatProvider",
            i_list => "System.Collections.IList",
            i_list_of_t => "System.Collections.Generic.IList`1",
            i_read_only_list_of_t => "System.Collections.Generic.IReadOnlyList`1",
            int32 => "System.Int32",
            int_ptr => "System.IntPtr",
            invalid_cast_exception => "System.InvalidCastException",
            list_of_t => "System.Collections.Generic.List`1",
            method_info => "System.Reflection.MethodInfo",
            multicast_delegate => "System.MulticastDelegate",
            not_supported_exception => "System.NotSupportedException",
            null_reference_exception => "System.NullReferenceException",
            nullable_t => "System.Nullable`1",
            object => "System.Object",
            obsolete_attribute => "System.ObsoleteAttribute",
            string => "System.String",
            type_ => "System.Type",
            uri => "System.Uri",
            void => "System.Void",
        }
        Ok(Self {
            action_of_t,
            func_of_t,
            action,
            array,
            boolean,
            culture_info,
            delegate,
            dictionary_of_t2,
            double,
            i_disposable,
            i_enumerable,
            i_enumerable_of_t,
            i_enumerator,
            i_enumerator_of_t,
            i_format_provider,
            i_list,
            i_list_of_t,
            i_read_only_list_of_t,
            int32,
            int_ptr,
            invalid_cast_exception,
            list_of_t,
            method_info,
            multicast_delegate,
            not_supported_exception,
            null_reference_exception,
            nullable_t,
            object,
            obsolete_attribute,
            string,
            type_,
            uri,
            void,
            experimental_attribute: type_system
                .find_type("System.Diagnostics.CodeAnalysis.ExperimentalAttribute"),
        })
    }
}

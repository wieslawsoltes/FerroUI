//! Port of the XAML compiler's runtime contracts
//! (`XamlX.Runtime/Interfaces.cs`).
//!
//! They live in this crate because the compiler crate has no runtime part:
//! these are the generic contracts a language built on the compiler may use
//! when it does not define its own (this framework defines its own in
//! [`xaml_il::runtime`](crate::xaml_il::runtime)).

use ferroui_base::{ferro_markup_type, BoxedValue};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// Provides the objects being built above the current one.
pub trait IXamlParentStackProviderV1 {
    /// The parents of the object being built, the immediate parent first.
    fn parents(&self) -> Vec<BoxedValue>;
}

/// Where the types of an XML namespace live.
#[derive(Debug, Default, PartialEq)]
pub struct XamlXmlNamespaceInfoV1 {
    clr_namespace: RefCell<String>,
    clr_assembly_name: RefCell<String>,
}

impl XamlXmlNamespaceInfoV1 {
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    /// The dotted namespace.
    pub fn clr_namespace(&self) -> String {
        self.clr_namespace.borrow().clone()
    }

    pub fn set_clr_namespace(&self, value: String) {
        *self.clr_namespace.borrow_mut() = value;
    }

    /// The assembly name.
    pub fn clr_assembly_name(&self) -> String {
        self.clr_assembly_name.borrow().clone()
    }

    pub fn set_clr_assembly_name(&self, value: String) {
        *self.clr_assembly_name.borrow_mut() = value;
    }
}

/// Provides the XML namespaces in scope: prefix to the namespaces it maps
/// to.
pub trait IXamlXmlNamespaceInfoProviderV1 {
    fn xml_namespaces(&self) -> Rc<HashMap<String, Vec<Rc<XamlXmlNamespaceInfoV1>>>>;
}

impl PartialEq for dyn IXamlParentStackProviderV1 {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self, other)
    }
}

impl PartialEq for dyn IXamlXmlNamespaceInfoProviderV1 {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self, other)
    }
}

ferro_markup_type!(interface dyn IXamlParentStackProviderV1 as "IXamlParentStackProviderV1" {
    this: Rc<dyn IXamlParentStackProviderV1>,
    handles: [Rc<dyn IXamlParentStackProviderV1>, Option<Rc<dyn IXamlParentStackProviderV1>>],
    properties: [
        Parents: Vec<BoxedValue> { get: |t: &Rc<dyn IXamlParentStackProviderV1>| t.parents() },
    ],
});

ferro_markup_type!(class XamlXmlNamespaceInfoV1 {
    this: Rc<XamlXmlNamespaceInfoV1>,
    handles: [XamlXmlNamespaceInfoV1, Rc<XamlXmlNamespaceInfoV1>, Option<Rc<XamlXmlNamespaceInfoV1>>],
    constructors: [() => XamlXmlNamespaceInfoV1::new],
    properties: [
        ClrNamespace: String {
            get: |t: &Rc<XamlXmlNamespaceInfoV1>| t.clr_namespace(),
            set: |t: &Rc<XamlXmlNamespaceInfoV1>, value: String| t.set_clr_namespace(value)
        },
        ClrAssemblyName: String {
            get: |t: &Rc<XamlXmlNamespaceInfoV1>| t.clr_assembly_name(),
            set: |t: &Rc<XamlXmlNamespaceInfoV1>, value: String| t.set_clr_assembly_name(value)
        },
    ],
});

ferro_markup_type!(interface dyn IXamlXmlNamespaceInfoProviderV1 as "IXamlXmlNamespaceInfoProviderV1" {
    this: Rc<dyn IXamlXmlNamespaceInfoProviderV1>,
    handles: [Rc<dyn IXamlXmlNamespaceInfoProviderV1>, Option<Rc<dyn IXamlXmlNamespaceInfoProviderV1>>],
});

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_base::metadata::{into_markup_value, MarkupTyped, MarkupTypeKind};

    struct Parents(Vec<BoxedValue>);

    impl IXamlParentStackProviderV1 for Parents {
        fn parents(&self) -> Vec<BoxedValue> {
            self.0.clone()
        }
    }

    #[test]
    fn namespace_info_holds_its_members() {
        crate::register_types();
        let info = XamlXmlNamespaceInfoV1::new();
        assert_eq!(info.clr_namespace(), "");
        info.set_clr_namespace("FerroUI.Controls".to_string());
        info.set_clr_assembly_name("FerroUI.Controls".to_string());
        assert_eq!(info.clr_namespace(), "FerroUI.Controls");
        assert_eq!(info.clr_assembly_name(), "FerroUI.Controls");

        let markup = <XamlXmlNamespaceInfoV1 as MarkupTyped>::MARKUP;
        assert_eq!(markup.full_name(), "XamlX.Runtime.XamlXmlNamespaceInfoV1");
        let built = (markup.constructors[0].invoke)(&[]).unwrap();
        let namespace = markup.find_property("ClrNamespace").unwrap();
        (namespace.set.unwrap())(&[built.clone(), into_markup_value("Ns".to_string())]).unwrap();
        let read = (namespace.get.unwrap())(&[built]).unwrap().unwrap();
        assert_eq!(read.downcast_ref::<String>().unwrap(), "Ns");
    }

    #[test]
    fn contracts_are_published_as_interfaces() {
        let provider: Rc<dyn IXamlParentStackProviderV1> = Rc::new(Parents(vec![Rc::new(1i32)]));
        let markup = <dyn IXamlParentStackProviderV1 as MarkupTyped>::MARKUP;
        assert_eq!(markup.kind, MarkupTypeKind::Interface);
        let parents = markup.find_property("Parents").unwrap();
        let read = (parents.get.unwrap())(&[into_markup_value(provider)]).unwrap().unwrap();
        assert_eq!(read.downcast_ref::<Vec<BoxedValue>>().unwrap().len(), 1);
        assert_eq!(<dyn IXamlXmlNamespaceInfoProviderV1 as MarkupTyped>::MARKUP.kind, MarkupTypeKind::Interface);
    }
}

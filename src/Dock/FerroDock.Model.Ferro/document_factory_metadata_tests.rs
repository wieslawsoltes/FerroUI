// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Document factory contract and shared native Func semantics over original delegates.
use crate::*;
use ferrodock_model::{
    controls::{DocumentFactory, IDocumentDockFactory},
    core::*,
};
use ferrodock_model_reactiveui::{
    controls::{Document, DocumentDock, ToolDock},
    Factory,
};
use ferroui_base::{
    data::core::{ValueType, ValueTypes},
    metadata::{from_markup_value, MarkupAssembly, MarkupDelegate, MarkupType, MarkupTyped},
    BoxedValue,
};
use std::{any::TypeId, cell::Cell, rc::Rc};
fn invoke(value: BoxedValue) -> Rc<dyn IDockable> {
    from_markup_value(
        &(DockableFactoryReference::MARKUP
            .find_methods("Invoke")
            .next()
            .unwrap()
            .invoke)(&[Some(value)])
        .unwrap(),
    )
    .unwrap()
}
#[test]
fn original_document_factory_metadata_preserves_callback_identity_deferred_execution_null_and_checked_contract_membership(
) {
    register_types();
    let source = DocumentDock::new();
    let contract: Rc<dyn IDocumentDockFactory> = source.clone();
    let boxed: BoxedValue = Rc::new(contract);
    let metadata = <dyn IDocumentDockFactoryMetadata>::MARKUP;
    assert_eq!(
        MarkupAssembly::of_module(metadata.module_path)
            .unwrap()
            .name,
        "FerroDock.Model"
    );
    let dockable: Rc<dyn IDockable> = source.clone();
    let checked = ValueTypes::try_cast(
        &(Rc::new(dockable) as BoxedValue),
        ValueType::of::<Rc<dyn IDocumentDockFactory>>(),
    )
    .unwrap();
    assert!(ValueTypes::reference_equals(&boxed, &checked));
    let wrong: Rc<dyn IDockable> = ToolDock::new();
    assert!(ValueTypes::try_cast(
        &(Rc::new(wrong) as BoxedValue),
        ValueType::of::<Rc<dyn IDocumentDockFactory>>()
    )
    .is_none());
    let calls = Rc::new(Cell::new(0));
    let observed = calls.clone();
    let document: Rc<dyn IDockable> = Document::new();
    let returned = document.clone();
    let callback: DocumentFactory = Rc::new(move || {
        observed.set(observed.get() + 1);
        Some(returned.clone())
    });
    let wrapper = DocumentFactoryReference::new(callback.clone());
    let p = metadata.find_property("DocumentFactory").unwrap();
    (p.set.unwrap())(&[Some(boxed.clone()), Some(Rc::new(wrapper.clone()))]).unwrap();
    assert_eq!(calls.get(), 0);
    assert!(Rc::ptr_eq(&source.document_factory().unwrap(), &callback));
    let exposed = from_markup_value::<DocumentFactoryReference>(
        &(p.get.unwrap())(&[Some(boxed.clone())]).unwrap(),
    )
    .unwrap();
    assert!(Rc::ptr_eq(&exposed.model(), &callback));
    assert!(ValueTypes::reference_equals(
        &(Rc::new(wrapper) as BoxedValue),
        &(Rc::new(exposed.clone()) as BoxedValue)
    ));
    assert!(Rc::ptr_eq(&invoke(Rc::new(exposed)), &document));
    assert_eq!(calls.get(), 1);
    (p.set.unwrap())(&[Some(boxed.clone()), None]).unwrap();
    assert!(source.document_factory().is_none());
    assert!((p.get.unwrap())(&[Some(boxed)]).unwrap().is_none());
}
#[test]
fn both_source_factory_forms_have_one_logical_func_descriptor_and_keep_their_original_return_contracts(
) {
    register_types();
    assert_eq!(
        TypeId::of::<DocumentFactory>(),
        TypeId::of::<DockableFactory>()
    );
    let original = MarkupType::find_by_handle(TypeId::of::<DockableFactoryReference>()).unwrap();
    let document = MarkupType::find_by_handle(TypeId::of::<DocumentFactoryReference>()).unwrap();
    assert!(std::ptr::eq(original, document));
    assert_eq!(original.namespace(), "System");
    assert!(
        (original.find_methods("Invoke").next().unwrap().invoke)(&[Some(Rc::new(42_i32))]).is_err()
    );
    assert_eq!(original.generic.unwrap().definition, "Func`1");
    let source: Rc<dyn IDockable> = Document::new();
    let returned = source.clone();
    let callback: DocumentFactory = Rc::new(move || Some(returned.clone()));
    assert!(Rc::ptr_eq(
        &invoke(Rc::new(DocumentFactoryReference::new(callback.clone()))),
        &source
    ));
    let returned = source.clone();
    let nullable: DockableFactory = Rc::new(move || Some(returned.clone()));
    assert!(Rc::ptr_eq(
        &invoke(Rc::new(DockableFactoryReference::new(nullable.clone()))),
        &source
    ));
    let empty: DockableFactory = Rc::new(|| None);
    let result = (original.find_methods("Invoke").next().unwrap().invoke)(&[Some(Rc::new(
        DockableFactoryReference::new(empty),
    ))])
    .unwrap();
    assert!(result.is_none());
    assert!(Rc::ptr_eq(
        &DocumentFactoryReference::new(callback.clone()).model(),
        &callback
    ));
    assert!(Rc::ptr_eq(
        &DockableFactoryReference::new(nullable.clone()).model(),
        &nullable
    ));
}
#[test]
fn original_create_document_command_uses_the_native_assigned_source_callback_and_same_docking_engine(
) {
    register_types();
    let factory = Factory::new();
    let source = DocumentDock::new();
    source.set_factory(Some(factory.clone()));
    source.set_visible_dockables(Some(factory.create_list(Vec::new())));
    let created: Rc<dyn IDockable> = Document::new();
    let result = created.clone();
    let callback: DocumentFactory = Rc::new(move || Some(result.clone()));
    let receiver: Rc<dyn IDocumentDockFactory> = source.clone();
    let value: BoxedValue = Rc::new(receiver);
    (<dyn IDocumentDockFactoryMetadata>::MARKUP
        .find_property("DocumentFactory")
        .unwrap()
        .set
        .unwrap())(&[
        Some(value),
        Some(Rc::new(DocumentFactoryReference::new(callback))),
    ])
    .unwrap();
    source.create_document().unwrap().execute(None);
    assert!(Rc::ptr_eq(
        (source.visible_dockables().unwrap().get(0))
            .as_ref()
            .expect("nonnull identity fixture"),
        &created
    ));
    assert!(Rc::ptr_eq(&source.active_dockable().unwrap(), &created));
    let owner: Rc<dyn IDockable> = source;
    assert!(Rc::ptr_eq(&created.owner().unwrap(), &owner));
}
#[test]
fn assigning_and_releasing_document_factory_views_retains_then_releases_the_original_captured_state(
) {
    register_types();
    let captured = Rc::new(());
    let weak = Rc::downgrade(&captured);
    let callback: DocumentFactory = Rc::new(move || {
        let _ = &captured;
        Some(Document::new())
    });
    let source = DocumentDock::new();
    let view = DocumentFactoryReference::new(callback);
    let original = view.model();
    source.set_document_factory(Some(view.model()));
    drop(view);
    assert!(weak.upgrade().is_some());
    source.set_document_factory(None);
    assert!(weak.upgrade().is_some());
    drop(original);
    assert!(weak.upgrade().is_none());
    let calls = Rc::new(Cell::new(0));
    let observed = calls.clone();
    let untyped = MarkupDelegate::new(move |_| {
        observed.set(observed.get() + 1);
        let doc: Rc<dyn IDockable> = Document::new();
        Some(Rc::new(doc))
    });
    let native = DocumentFactoryReference::from_delegate(untyped).unwrap();
    assert_eq!(calls.get(), 0);
    let _ = native.invoke();
    assert_eq!(calls.get(), 1);
}
#[test]
fn original_requires_data_template_attributes_are_exported_on_exactly_the_fourteen_source_interfaces(
) {
    register_types();
    let model = ferroui_build_scan::model::AssemblyModel::parse(include_str!(concat!(
        env!("OUT_DIR"),
        "/ferrodock_model_ferro.xamlmeta"
    )))
    .unwrap();
    let expected = [
        "IDockDock",
        "IDocumentContent",
        "IDocumentDock",
        "IGridDock",
        "IGridDockSplitter",
        "IProportionalDock",
        "IProportionalDockSplitter",
        "IRootDock",
        "ISplitViewDock",
        "IStackDock",
        "IToolContent",
        "IToolDock",
        "IUniformGridDock",
        "IWrapDock",
    ];
    let mut count = 0;
    for t in model
        .types
        .iter()
        .filter(|t| t.namespace == "FerroDock.Model.Controls")
    {
        let m = MarkupType::find("FerroDock.Model.Controls", &t.name).unwrap();
        let actual = m
            .attributes
            .iter()
            .filter(|a| a.name == "RequiresDataTemplate")
            .count();
        assert_eq!(
            actual,
            usize::from(expected.contains(&t.name.as_str())),
            "{}",
            t.name
        );
        count += actual;
        for attr in m.attributes {
            assert!(attr.arguments.is_empty());
            assert!(attr.properties.is_empty());
        }
        assert_eq!(
            t.attributes
                .iter()
                .filter(|a| a.name == "RequiresDataTemplate")
                .count(),
            actual
        );
    }
    assert_eq!(count, 14);
}

struct CallbackSignatures;
ferroui_base::ferro_markup_type!(class CallbackSignatures{
    this:CallbackSignatures,handles:[CallbackSignatures],methods:[
        static fn Wrong()->bool => (||panic!("a rejected method must not run")),
        static fn WithArgument(i32)->Rc<dyn IDockable> => (|_|panic!("a rejected method must not run")),
        static fn Void() => (||panic!("a rejected method must not run")),
        static fn Valid()->Rc<dyn IDockable> => (||{let document:Rc<dyn IDockable>=Document::new();document}),
    ],
});
impl PartialEq for CallbackSignatures {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
#[test]
fn document_callback_method_adaptation_checks_declared_arity_and_return_type_before_execution() {
    register_types();
    let metadata = CallbackSignatures::MARKUP;
    for name in ["Wrong", "WithArgument", "Void"] {
        let method = metadata.find_methods(name).next().unwrap();
        let delegate = MarkupDelegate::for_method(None, metadata, method);
        assert!(
            DocumentFactoryReference::from_delegate(delegate).is_none(),
            "{name}"
        );
    }
    let method = metadata.find_methods("Valid").next().unwrap();
    let callback =
        DocumentFactoryReference::from_delegate(MarkupDelegate::for_method(None, metadata, method))
            .unwrap();
    assert!(callback.invoke().unwrap().as_mdi_document().is_some());
}

#[test]
fn native_callback_cross_assignment_through_original_properties_and_locator_maps_preserves_identity_and_typed_roundtrips(
) {
    register_types();
    let source = DocumentDock::new();
    let receiver: Rc<dyn IDocumentDockFactory> = source.clone();
    let boxed: BoxedValue = Rc::new(receiver);
    let property = <dyn IDocumentDockFactoryMetadata>::MARKUP
        .find_property("DocumentFactory")
        .unwrap();
    let document: Rc<dyn IDockable> = Document::new();
    let returned = document.clone();
    let original: DockableFactory = Rc::new(move || Some(returned.clone()));
    let nullable = DockableFactoryReference::new(original.clone());
    let native: BoxedValue = Rc::new(nullable);
    (property.set.unwrap())(&[Some(boxed.clone()), Some(native.clone())]).unwrap();
    let getter = (property.get.unwrap())(&[Some(boxed.clone())])
        .unwrap()
        .unwrap();
    assert!(ValueTypes::reference_equals(&getter, &native));
    assert!(Rc::ptr_eq(&invoke(getter.clone()), &document));
    let map = DockableLocatorMapReference::create();
    let p = DockableLocatorMapReference::MARKUP
        .indexers
        .first()
        .unwrap();
    (p.set.unwrap())(&[
        Some(Rc::new(map.clone())),
        Some(Rc::new("item".to_owned())),
        Some(getter),
    ])
    .unwrap();
    assert!(Rc::ptr_eq(
        map.model().borrow().get("item").unwrap().as_ref().unwrap(),
        &original
    ));
    let roundtrip: BoxedValue = Rc::new(map.get("item".into()).unwrap().unwrap());
    (property.set.unwrap())(&[Some(boxed.clone()), Some(roundtrip)]).unwrap();
    assert!(ValueTypes::reference_equals(
        &(property.get.unwrap())(&[Some(boxed.clone())])
            .unwrap()
            .unwrap(),
        &native
    ));
    let required: ferrodock_model::controls::DocumentFactory = Rc::new(|| Some(Document::new()));
    source.set_document_factory(Some(required.clone()));
    let getter = (property.get.unwrap())(&[Some(boxed.clone())])
        .unwrap()
        .unwrap();
    (p.set.unwrap())(&[
        Some(Rc::new(map.clone())),
        Some(Rc::new("required".to_owned())),
        Some(getter.clone()),
    ])
    .unwrap();
    let mapped: BoxedValue = Rc::new(map.get("required".into()).unwrap().unwrap());
    assert!(ValueTypes::reference_equals(&mapped, &getter));
    (property.set.unwrap())(&[Some(boxed), Some(mapped)]).unwrap();
    assert!(Rc::ptr_eq(&source.document_factory().unwrap(), &required));
}
#[test]
fn native_func_invocation_retains_original_null_result_after_cross_assignment_without_invoking_during_conversion(
) {
    register_types();
    let calls = Rc::new(Cell::new(0));
    let observed = calls.clone();
    let source: DockableFactory = Rc::new(move || {
        observed.set(observed.get() + 1);
        None
    });
    let original: BoxedValue = Rc::new(DockableFactoryReference::new(source.clone()));
    let required =
        ValueTypes::try_cast(&original, ValueType::of::<DocumentFactoryReference>()).unwrap();
    assert_eq!(calls.get(), 0);
    assert!(ValueTypes::reference_equals(&original, &required));
    let method = DockableFactoryReference::MARKUP
        .find_methods("Invoke")
        .next()
        .unwrap();
    assert!((method.invoke)(&[Some(required.clone())])
        .unwrap()
        .is_none());
    assert_eq!(calls.get(), 1);
    let back =
        ValueTypes::try_cast(&required, ValueType::of::<DockableFactoryReference>()).unwrap();
    assert!(Rc::ptr_eq(
        &back
            .downcast_ref::<DockableFactoryReference>()
            .unwrap()
            .model(),
        &source
    ));
    assert_eq!(calls.get(), 1);
}

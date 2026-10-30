// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Shared object namescope parity and independently compiled consumer tests.
use crate::{
    markup_extension_tests::{provider, run},
    named_document::NamedDocument,
};
use ferrodock_markup_extension::ReferenceExtension;
use ferrodock_model::core::IDockable;
use ferrodock_model_reactiveui::controls::Document;
use ferroui_base::{
    controls::{ChildNameScope, INameScope, NameScope, NameScopeError, NameScopeRef, NameScopeValueError},
    data::core::ValueTypes,
    metadata::{into_markup_value, MarkupType, MarkupTyped},
    threading::Dispatcher,
    utilities::SynchronousCompletionAsyncResult,
    BoxedValue, FerroObject, Ref,
};
use ferroui_controls::{Border, Button, ContentControl, StackPanel, TextBlock};
use std::{cell::Cell, rc::Rc};

#[test]
fn plain_values_preserve_reference_identity_and_share_native_name_collisions() {
    run(|| {
        let scope = NameScope::new();
        let value: BoxedValue = Rc::new("same text".to_owned());
        scope.try_register_value("model", value.clone()).unwrap();
        scope.try_register_value("model", value.clone()).unwrap();
        assert!(Rc::ptr_eq(&scope.find_value("model").unwrap(), &value));
        assert_eq!(
            scope.try_register_value("model", Rc::new("same text".to_owned())),
            Err(NameScopeValueError::DuplicateName("model".into()))
        );
        assert!(scope.find("model").is_none());
        assert_eq!(
            scope.try_register("model", Button::new().upcast()),
            Err(NameScopeError::DuplicateName("model".into()))
        );
        let button = Button::new();
        scope.register("control", button.clone().upcast());
        assert_eq!(
            ValueTypes::as_object(&*scope.find_value("control").unwrap()),
            Some(button.upcast())
        );
        assert!(scope.try_register_value("control", value).is_err());
        scope.complete();
        assert_eq!(
            scope.try_register_value("later", Rc::new(1i32)),
            Err(NameScopeValueError::Completed)
        );
    });
}
#[test]
fn multiple_boxed_handles_of_the_same_plain_reference_are_one_object() {
    run(|| {
        ValueTypes::register_reference::<Document>();
        let document = Document::new();
        let scope = Rc::new(NameScope::new());
        scope.register_value("model", Rc::new(document.clone()));
        scope
            .try_register_value("model", Rc::new(document.clone()))
            .unwrap();
        let resolved = ReferenceExtension::with_name("model".into())
            .provide_value(&provider(None, Some(scope)))
            .unwrap();
        assert!(Rc::ptr_eq(
            resolved.downcast_ref::<Rc<Document>>().unwrap(),
            &document
        ));
        document.set_title("same engine model".into());
        assert_eq!(
            resolved.downcast_ref::<Rc<Document>>().unwrap().title(),
            "same engine model"
        );
    });
}
#[test]
fn pending_plain_and_native_searches_complete_synchronously_without_scope_borrows() {
    let scope = Rc::new(NameScope::new());
    let result = scope.find_value_async("later");
    let typed = scope.find_async("later");
    let seen = Rc::new(Cell::new(false));
    let callback_scope = scope.clone();
    let callback_result = result.clone();
    let callback_seen = seen.clone();
    result.on_completed(move || {
        assert!(Rc::ptr_eq(
            &callback_result.get_result().unwrap(),
            &callback_scope.find_value("later").unwrap()
        ));
        callback_scope.register_value("reentrant", Rc::new(42i32));
        callback_seen.set(true);
    });
    assert!(!result.is_completed());
    scope.register_value("later", Rc::new("plain".to_owned()));
    assert!(seen.get());
    assert!(result.is_completed());
    assert!(typed.is_completed());
    assert!(typed.get_result().is_none());
    assert_eq!(
        scope.find_value("reentrant").unwrap().downcast_ref::<i32>(),
        Some(&42)
    );
}
#[test]
fn completion_resolves_missing_values_once_and_allows_lookup_reentry() {
    let scope = Rc::new(NameScope::new());
    let result = scope.find_value_async("missing");
    let calls = Rc::new(Cell::new(0));
    let callback_scope = scope.clone();
    let callback_calls = calls.clone();
    result.on_completed(move || {
        assert!(callback_scope
            .find_value_async("also missing")
            .is_completed());
        callback_scope.complete();
        callback_calls.set(callback_calls.get() + 1);
    });
    scope.complete();
    scope.complete();
    assert!(result.get_result().is_none());
    assert_eq!(calls.get(), 1);
}
#[test]
fn child_plain_values_shadow_parent_controls_for_both_lookup_forms() {
    run(|| {
        let parent = Rc::new(NameScope::new());
        parent.register("shared", Button::new().upcast());
        let child = ChildNameScope::new(NameScopeRef(parent));
        let pending = child.find_async("shared");
        let plain: BoxedValue = Rc::new("local".to_owned());
        child.register_value("shared", plain.clone());
        child.complete();
        assert!(pending.get_result().is_none());
        assert!(child.find("shared").is_none());
        assert!(Rc::ptr_eq(&child.find_value("shared").unwrap(), &plain));
    });
}
#[test]
fn child_search_waits_for_local_completion_then_parent_registration() {
    let parent = Rc::new(NameScope::new());
    let child = ChildNameScope::new(NameScopeRef(parent.clone()));
    let result = child.find_value_async("model");
    assert!(!result.is_completed());
    child.complete();
    assert!(!result.is_completed());
    let value: BoxedValue = Rc::new("parent".to_owned());
    parent.register_value("model", value.clone());
    assert!(Rc::ptr_eq(&result.get_result().unwrap(), &value));
    assert!(Rc::ptr_eq(&child.find_value("model").unwrap(), &value));
    assert!(!child.is_completed());
    parent.complete();
    assert!(child.is_completed());
    assert!(child.find_value_async("missing").get_result().is_none());
}
#[test]
fn native_scope_metadata_register_and_find_accept_actual_plain_engine_models() {
    run(|| {
        let scope = Rc::new(NameScope::new());
        let boxed_scope = into_markup_value(NameScopeRef(scope.clone()));
        let name = into_markup_value("model".to_owned());
        let document = Document::new();
        let value: BoxedValue = Rc::new(document.clone());
        let metadata = MarkupType::find("FerroUI.Controls", "INameScope").unwrap();
        let register = metadata.find_methods("Register").next().unwrap();
        (register.invoke)(&[boxed_scope.clone(), name.clone(), Some(value.clone())]).unwrap();
        let find = metadata.find_methods("Find").next().unwrap();
        let resolved = (find.invoke)(&[boxed_scope, name]).unwrap().unwrap();
        assert!(Rc::ptr_eq(&resolved, &value));
        let result = ReferenceExtension::with_name("model".into())
            .provide_value(&provider(None, Some(scope)))
            .unwrap();
        assert!(Rc::ptr_eq(
            result.downcast_ref::<Rc<Document>>().unwrap(),
            &document
        ));
    });
}
#[test]
fn compiled_plain_named_model_reference_and_binding_share_the_original_document() {
    run(|| {
        ValueTypes::register_reference::<NamedDocument>();
        MarkupType::register(NamedDocument::MARKUP);
        let value = crate::compiled_xaml::try_load(
            None,
            "ferres://ferrodock_markup_tests/NamedModelConsumer.xaml",
        )
        .unwrap()
        .unwrap();
        let panel = value.downcast_ref::<Ref<StackPanel>>().unwrap();
        let content = panel
            .children()
            .get(0)
            .cast::<ContentControl>()
            .unwrap()
            .content()
            .unwrap();
        let document = content.downcast_ref::<NamedDocument>().unwrap();
        let reference = panel
            .children()
            .get(1)
            .cast::<Border>()
            .unwrap()
            .tag()
            .unwrap();
        assert!(Rc::ptr_eq(&content, &reference));
        let scope = NameScope::get_name_scope(panel).unwrap();
        assert!(scope.is_completed());
        let named = scope.find_value("Document").unwrap();
        assert!(std::ptr::eq(
            &**named.downcast_ref::<Rc<NamedDocument>>().unwrap(),
            document
        ));
        let text = panel.children().get(2).cast::<TextBlock>().unwrap();
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(text.text().as_deref(), Some("Named engine document"));
        document.model().set_title("updated engine title".into());
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(text.text().as_deref(), Some("updated engine title"));
    });
}

struct LegacyNativeScope(NameScope);
impl INameScope for LegacyNativeScope {
    fn try_register(&self, name: &str, element: Ref<FerroObject>) -> Result<(), NameScopeError> {
        self.0.try_register(name, element)
    }
    fn find(&self, name: &str) -> Option<Ref<FerroObject>> {
        self.0.find(name)
    }
    fn find_async(&self, name: &str) -> SynchronousCompletionAsyncResult<Option<Ref<FerroObject>>> {
        self.0.find_async(name)
    }
    fn complete(&self) {
        self.0.complete();
    }
    fn is_completed(&self) -> bool {
        self.0.is_completed()
    }
}
#[test]
fn existing_custom_native_scopes_keep_their_contract_and_report_unsupported_plain_objects() {
    run(|| {
        let scope = LegacyNativeScope(NameScope::new());
        let result = scope.find_value_async("button");
        let button = Button::new();
        scope
            .try_register_value("button", Rc::new(button.clone()))
            .unwrap();
        assert_eq!(
            ValueTypes::as_object(&*result.get_result().unwrap()),
            Some(button.clone().upcast())
        );
        assert_eq!(
            ValueTypes::as_object(&*scope.find_value("button").unwrap()),
            Some(button.upcast())
        );
        assert_eq!(
            scope.try_register_value("plain", Rc::new(42i32)),
            Err(NameScopeValueError::UnsupportedValue)
        );
    });
}

#[test]
fn registered_nullable_null_is_rejected_after_the_completion_guard() {
    let scope = NameScope::new();
    let null: BoxedValue = Rc::new(None::<BoxedValue>);
    assert_eq!(
        scope.try_register_value("null", null.clone()),
        Err(NameScopeValueError::NullValue)
    );
    assert!(scope.find_value("null").is_none());
    scope.complete();
    assert_eq!(
        scope.try_register_value("null", null),
        Err(NameScopeValueError::Completed)
    );
}
#[test]
fn scope_keeps_plain_values_alive_and_preserves_native_attached_owner_lifetime() {
    run(|| {
        let scope = Rc::new(NameScope::new());
        let value: BoxedValue = Rc::new("owned model".to_owned());
        let weak_value = Rc::downgrade(&value);
        scope.register_value("model", value);
        let owner = Button::new();
        let weak_owner = owner.downgrade();
        scope.register_value("owner", Rc::new(owner.clone()));
        NameScope::set_name_scope(&owner, Some(NameScopeRef(scope.clone())));
        drop(owner);
        assert!(weak_owner.upgrade().is_none());
        assert!(scope.find_value("owner").is_none());
        assert_eq!(
            scope.try_register_value("owner", Rc::new(42i32)),
            Err(NameScopeValueError::DuplicateName("owner".into()))
        );
        assert!(weak_value.upgrade().is_some());
        drop(scope);
        assert!(weak_value.upgrade().is_none());
    });
}
#[test]
fn shared_resolve_by_name_extension_accepts_plain_models_and_preserves_missing_name_deferral() {
    run(|| {
        let scope = Rc::new(NameScope::new());
        let document = Document::new();
        scope.register_value("document", Rc::new(document.clone()));
        let extension =
            ferroui_markup_xaml::markup_extensions::ResolveByNameExtension::new("document");
        let provider = provider(None, Some(scope));
        let value = extension.provide_value(&provider).unwrap();
        assert!(Rc::ptr_eq(
            value.downcast_ref::<Rc<Document>>().unwrap(),
            &document
        ));
        let missing =
            ferroui_markup_xaml::markup_extensions::ResolveByNameExtension::new("missing");
        assert!(missing
            .provide_value(&provider)
            .unwrap()
            .downcast_ref::<ferroui_base::UnsetValueType>()
            .is_some());
    });
}

#[test]
fn compiled_plain_model_root_uses_declared_handle_and_completes_its_scope() {
    run(|| {
        ValueTypes::register_reference::<NamedDocument>();
        MarkupType::register(NamedDocument::MARKUP);
        let scope = Rc::new(NameScope::new());
        let document = crate::compiled_xaml::build_namedmodelroot_xaml(Some(provider(
            None,
            Some(scope.clone()),
        )))
        .unwrap();
        assert_eq!(document.model().title(), "Plain root");
        assert!(scope.is_completed());
        assert!(Rc::ptr_eq(
            scope
                .find_value("RootDocument")
                .unwrap()
                .downcast_ref::<Rc<NamedDocument>>()
                .unwrap(),
            &document
        ));
    });
}

#[test]
fn interface_and_inherited_class_views_of_a_document_share_one_scope_identity() {
    run(|| {
        use ferrodock_model::controls::IDocument;
        use ferrodock_model_reactiveui_ferro::reactive_metadata::*;
        ferrodock_ferro::register_types();
        ValueTypes::register_reference::<Document>();
        let document = Document::new();
        let dockable: Rc<dyn IDockable> = document.clone();
        let contract: Rc<dyn IDocument> = document.clone();
        let class = DocumentReference::new(document.clone());
        let values: Vec<BoxedValue> = vec![
            Rc::new(dockable),
            Rc::new(contract),
            Rc::new(class.clone()),
            Rc::new(class.base()),
            Rc::new(class.base().base()),
            Rc::new(class.base().base().base()),
            Rc::new(document.clone()),
            document.clone(),
        ];
        for first in &values {
            let scope = Rc::new(NameScope::new());
            scope.register_value("document", first.clone());
            for other in &values {
                assert!(ValueTypes::reference_equals(first, other));
                scope.try_register_value("document", other.clone()).unwrap();
            }
            let resolved = ReferenceExtension::with_name("document".into())
                .provide_value(&provider(None, Some(scope)))
                .unwrap();
            assert!(Rc::ptr_eq(&resolved, first));
        }
        let other = Document::new();
        let different: BoxedValue = Rc::new(DocumentReference::new(other));
        for value in &values {
            assert!(!ValueTypes::reference_equals(value, &different));
        }
    });
}
#[test]
fn dock_and_reactive_class_views_retain_the_root_engine_identity() {
    run(|| {
        use ferrodock_model::{controls::IRootDock, core::IDock};
        use ferrodock_model_reactiveui::controls::RootDock;
        use ferrodock_model_reactiveui_ferro::reactive_metadata::RootDockReference;
        ferrodock_ferro::register_types();
        ValueTypes::register_reference::<RootDock>();
        let root = RootDock::new();
        let class = RootDockReference::new(root.clone());
        let dockable: Rc<dyn IDockable> = root.clone();
        let dock: Rc<dyn IDock> = root.clone();
        let contract: Rc<dyn IRootDock> = root.clone();
        let values: Vec<BoxedValue> = vec![
            root,
            Rc::new(dockable),
            Rc::new(dock),
            Rc::new(contract),
            Rc::new(class.clone()),
            Rc::new(class.base()),
            Rc::new(class.base().base()),
            Rc::new(class.base().base().base()),
            Rc::new(class.base().base().base().base()),
        ];
        let scope = NameScope::new();
        scope.register_value("root", values[0].clone());
        for value in &values {
            scope.try_register_value("root", value.clone()).unwrap();
            assert!(ValueTypes::reference_equals(&values[0], value));
        }
    });
}
struct ReentrantIdentity(Rc<i32>, std::rc::Weak<NameScope>, Rc<Cell<bool>>);
impl PartialEq for ReentrantIdentity {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[test]
fn shared_identity_callbacks_allow_metadata_and_scope_reentry_without_value_equality() {
    ValueTypes::register_reference_identity::<ReentrantIdentity>(|value| {
        ValueTypes::register_nullable::<i32>();
        if value.2.replace(false) {
            value
                .1
                .upgrade()
                .unwrap()
                .register_value("callback", Rc::new(1_i32));
        }
        Rc::as_ptr(&value.0) as *const ()
    });
    let source = Rc::new(42);
    let scope = Rc::new(NameScope::new());
    let trigger = Rc::new(Cell::new(false));
    let first: BoxedValue = Rc::new(ReentrantIdentity(
        source.clone(),
        Rc::downgrade(&scope),
        trigger.clone(),
    ));
    let same: BoxedValue = Rc::new(ReentrantIdentity(
        source,
        Rc::downgrade(&scope),
        trigger.clone(),
    ));
    let different: BoxedValue = Rc::new(ReentrantIdentity(
        Rc::new(42),
        Rc::downgrade(&scope),
        trigger.clone(),
    ));
    assert!(ValueTypes::reference_equals(&first, &same));
    assert!(!ValueTypes::reference_equals(&first, &different));
    scope.register_value("object", first.clone());
    trigger.set(true);
    scope.try_register_value("object", same).unwrap();
    assert!(scope.find_value("callback").is_some());
    assert!(matches!(
        scope.try_register_value("object", different),
        Err(NameScopeValueError::DuplicateName(_))
    ));
    let a: BoxedValue = Rc::new(42_i32);
    let b: BoxedValue = Rc::new(42_i32);
    assert!(!ValueTypes::reference_equals(&a, &b));
    assert!(ValueTypes::reference_equals(&a, &a));
    assert_eq!(
        ValueTypes::reference_identity(&a),
        Rc::as_ptr(&a) as *const ()
    );
}

#[test]
fn runtime_compiler_emits_the_same_plain_model_root_value_and_single_construction() {
    run(|| {
        use ferroui_base::metadata::MarkupAssembly;
        const ASSEMBLY: MarkupAssembly = MarkupAssembly {
            name: "FerroDock.Markup.Tests",
            crate_name: "ferrodock_markup_tests",
            xmlns_definitions: &[],
            xmlns_prefixes: &[],
            metadata: &[],
        };
        MarkupAssembly::register(&ASSEMBLY);
        let mut configuration = ferroui_markup_xaml::RuntimeXamlLoaderConfiguration::default();
        configuration.local_assembly = Some(&ASSEMBLY);
        ValueTypes::register_reference::<NamedDocument>();
        MarkupType::register(NamedDocument::MARKUP);
        MarkupType::register_rust_paths(&[(
            NamedDocument::MARKUP,
            "ferrodock_markup_tests::named_document::NamedDocument",
            false,
        )]);
        let result = ferroui_markup_xaml_loader::rust_emitter::compile_documents(
            &[("NamedModelRoot.xaml", include_str!("NamedModelRoot.xaml"))],
            Some("ferres://ferrodock_markup_tests/"),
            &configuration,
            &[],
        );
        assert_eq!(result.len(), 1);
        let source = result[0].source.as_ref().unwrap();
        assert!(
            source.contains(
                "Result<::std::rc::Rc<ferrodock_markup_tests::named_document::NamedDocument>"
            ),
            "{source}"
        );
        assert_eq!(source.matches("::__markup_new_0()").count(), 1, "{source}");
        assert!(source.contains("complete_root_name_scope(::core::option::Option::None"));
    });
}
#[derive(Clone, PartialEq)]
struct NativeIdentityView {
    object: Ref<FerroObject>,
    reads: Rc<Cell<usize>>,
}
#[test]
fn shared_reference_comparison_projects_native_objects_once_and_retains_native_priority() {
    run(|| {
        ValueTypes::register_interface_object::<NativeIdentityView>(|value| {
            value.reads.set(value.reads.get() + 1);
            Some(value.object.clone())
        });
        ValueTypes::register_reference_identity::<NativeIdentityView>(|_| {
            panic!("native projection takes precedence")
        });
        let button = Button::new();
        let reads = Rc::new(Cell::new(0));
        let view: BoxedValue = Rc::new(NativeIdentityView {
            object: button.clone().upcast(),
            reads: reads.clone(),
        });
        let actual: BoxedValue = Rc::new(button.clone());
        assert!(ValueTypes::reference_equals(&view, &actual));
        assert_eq!(reads.get(), 1);
        assert_eq!(
            ValueTypes::reference_identity(&view),
            ValueTypes::reference_identity(&actual)
        );
        assert_eq!(reads.get(), 2);
        let different: BoxedValue = Rc::new(Button::new());
        assert!(!ValueTypes::reference_equals(&view, &different));
        assert_eq!(reads.get(), 3);
    });
}

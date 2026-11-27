//! Independent source-style scope construction/metadata and direct reusable model synchronization.
//! Historical manual-scope fixture; root_model_tests now separately exercises the root Model API.
use ferroribbon::{
    controls::{Ribbon, RibbonButton},
    metadata::model_bindings::RibbonModelBindingsMetadata,
    mvvm::RibbonModelBindings,
};
use ferroribbon_core::model::{RibbonButtonModel, RibbonNodeModelExt};
use ferroui_base::{
    data::core::{ValueType, ValueTypes},
    metadata::{MarkupDelegate, MarkupTyped},
};
use ferroui_controls::{Control, StackPanel, TextBlock, Window};
use ferroribbon_core::collections::ObservableCollection;
use ferroui_system::{
    IDisposable, component_model::INotifyPropertyChanged,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

#[test]
fn independent_scope_owners_release_live_child_bindings_without_disposal_traits() {
    super::tests::run(|| {
        for explicit_close in [false, true] {
            let model = RibbonButtonModel::new(Some("live".into()), None);
            let trace = Rc::new(RefCell::new(Vec::new()));
            let scope = RibbonModelBindings::new();
            let weak_scope = Rc::downgrade(&scope);
            let before = trace.clone();
            scope.add(Some(Rc::new(move || before.borrow_mut().push("before"))));
            let child = scope.child();
            let in_child = trace.clone();
            child.add(Some(Rc::new(move || in_child.borrow_mut().push("child"))));
            let after = trace.clone();
            scope.add(Some(Rc::new(move || after.borrow_mut().push("after"))));
            let lifetime = Rc::new(());
            let weak_lifetime = Rc::downgrade(&lifetime);
            let calls = Rc::new(Cell::new(0));
            let count = calls.clone();
            child.watch(
                Some(model.clone()),
                Some(Rc::new(move || {
                    let _keep_callback_payload_alive = &lifetime;
                    count.set(count.get() + 1);
                })),
                Some(vec![Some("Label".into())]),
            );
            let retained = scope.clone();
            drop(scope);
            assert!(weak_scope.upgrade().is_some());
            assert!(trace.borrow().is_empty());
            assert!(weak_lifetime.upgrade().is_some());
            assert_eq!(model.property_changed().handler_count(), 1);
            model.set_label(Some("Still bound".into()));
            assert_eq!(calls.get(), 1);
            if explicit_close {
                retained.close();
                retained.close();
                assert!(retained.is_disposed());
                let late = trace.clone();
                retained.add(Some(Rc::new(move || late.borrow_mut().push("late"))));
            }
            drop(retained);
            assert!(weak_scope.upgrade().is_none());
            assert!(child.is_disposed());
            assert_eq!(model.property_changed().handler_count(), 0);
            assert!(weak_lifetime.upgrade().is_none());
            model.set_label(Some("No longer bound".into()));
            assert_eq!(calls.get(), 1);
            assert_eq!(
                trace.borrow().as_slice(),
                if explicit_close {
                    &["before", "child", "after", "late"][..]
                } else {
                    &["before", "child", "after"][..]
                }
            );
        }
    });
}

#[test]
fn independent_compiled_runtime_scope_mirrors_actual_models_with_two_way_native_properties() {
    super::tests::run(|| {
        let _theme = super::tests::NativeThemeScope::new();
        for runtime in [false, true] {
            let value = if runtime {
                ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                    include_str!("ModelBindings.xaml"),
                    Some(&crate::CONSUMER_ASSEMBLY),
                )
                .unwrap()
            } else {
                crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferroribbon_markup_consumer/ModelBindings.xaml",
                )
                .unwrap()
                .unwrap()
            };
            let panel = ValueTypes::as_object(&*value)
                .unwrap()
                .cast::<StackPanel>()
                .unwrap();
            let ribbon = panel.children().get(0).cast::<Ribbon>().unwrap();
            assert_eq!(
                panel
                    .children()
                    .get(1)
                    .cast::<TextBlock>()
                    .unwrap()
                    .text()
                    .as_deref(),
                Some("False")
            );
            let v = ValueTypes::try_convert(
                ribbon.tag().as_ref(),
                ValueType::of::<Rc<RibbonModelBindings>>(),
            )
            .flatten()
            .unwrap();
            let scope = v.downcast_ref::<Rc<RibbonModelBindings>>().unwrap().clone();
            let group = ribbon.tabs().get(0).unwrap().groups().get(0).unwrap();
            let declared = group.items().get(0).unwrap();
            let a = RibbonButtonModel::new(Some("a".into()), Some("Alpha".into()));
            let b = RibbonButtonModel::new(Some("b".into()), Some("Beta".into()));
            let models =
                ObservableCollection::from_items_managed(vec![Some(a.clone()), Some(b.clone())]);
            let mirror = scope.items(
                models.clone(),
                Rc::new(group.items()),
                Rc::new(|m: Option<Rc<RibbonButtonModel>>, s| {
                    let m = m.unwrap();
                    let button = crate::button_consumer::ConsumerRibbonButton::new();
                    button.set_id(m.id().map(|s| s.to_utf8().unwrap()));
                    let weak = button.downgrade();
                    let model = m.clone();
                    let to_element = Rc::new(move || {
                        weak.upgrade()
                            .unwrap()
                            .set_label(model.label().map(|s| s.to_utf8().unwrap()))
                    });
                    let weak = button.downgrade();
                    let model = m.clone();
                    let to_model = Rc::new(move || {
                        model.set_label(weak.upgrade().unwrap().label().map(Into::into))
                    });
                    s.two_way(
                        Some(m),
                        Some("Label".into()),
                        Some(to_element),
                        Some(button.clone().upcast()),
                        Some(RibbonButton::label_property().as_property()),
                        Some(to_model),
                    );
                    Some(button.upcast::<Control>())
                }),
                true,
                None,
                None,
            );
            let window = Window::new();
            window.set_content(Some(Control::boxed(panel)));
            window.show();
            let first = mirror
                .find_element(&Some(a.clone()))
                .unwrap()
                .cast::<RibbonButton>()
                .unwrap();
            assert_eq!(first.label().as_deref(), Some("Alpha"));
            first.set_label(Some("From element".into()));
            assert_eq!(a.label(), Some("From element".into()));
            a.set_label(Some("From model".into()));
            assert_eq!(first.label().as_deref(), Some("From model"));
            models.move_item(1, 0);
            assert!(
                mirror
                    .find_element(&Some(a.clone()))
                    .unwrap()
                    .ptr_eq(&first)
            );
            assert!(group.items().get(0).unwrap().ptr_eq(&declared));
            scope.suspend();
            a.set_label(Some("Paused".into()));
            models.add(Some(RibbonButtonModel::new(
                Some("c".into()),
                Some("Gamma".into()),
            )));
            assert_eq!(group.items().count(), 3);
            assert_eq!(first.label().as_deref(), Some("From model"));
            scope.resume();
            assert_eq!(group.items().count(), 4);
            assert_eq!(first.label().as_deref(), Some("Paused"));
            models.remove(&Some(a.clone()));
            assert_eq!(a.property_changed().handler_count(), 0);
            a.set_label(Some("Detached".into()));
            assert_eq!(first.label().as_deref(), Some("Paused"));
            let calls = Rc::new(Cell::new(0));
            let count = calls.clone();
            let cleanup = MarkupDelegate::new(move |_| {
                count.set(count.get() + 1);
                None
            });
            let add = RibbonModelBindingsMetadata::MARKUP
                .find_methods("Add")
                .next()
                .unwrap();
            (add.invoke)(&[Some(Rc::new(scope.clone())), Some(Rc::new(Some(cleanup)))]).unwrap();
            assert!(RibbonModelBindingsMetadata::MARKUP.interfaces.is_empty());
            let boxed = Rc::new(scope.clone()) as ferroui_base::BoxedValue;
            assert!(
                ValueTypes::try_cast(&boxed, ValueType::of::<Rc<dyn IDisposable>>()).is_none(),
                "the scope must not export the removed general disposal interface"
            );
            if runtime {
                let dispose = RibbonModelBindingsMetadata::MARKUP
                    .find_methods("Dispose")
                    .next()
                    .unwrap();
                (dispose.invoke)(&[Some(boxed)]).unwrap();
            } else {
                scope.close();
            }
            scope.close();
            assert_eq!(calls.get(), 1);
            assert!(scope.is_disposed());
            assert_eq!(b.property_changed().handler_count(), 0);
            assert_eq!(group.items().count(), 1);
            assert!(group.items().get(0).unwrap().ptr_eq(&declared));
            window.close();
        }
    });
}

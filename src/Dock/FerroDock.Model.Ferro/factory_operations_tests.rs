//! Factory search and mutation metadata against the original production engine.
use crate::*;
use ferrodock_model::{
    controls::IRootDock,
    core::{IDock, IDockable, IFactory},
};
use ferrodock_model_reactiveui::Factory;
use ferroui_base::{
    data::core::ValueType,
    metadata::{from_markup_value, MarkupType, MarkupTyped},
    BoxedValue,
};
use std::{cell::Cell, rc::Rc};

fn invoke(
    target: BoxedValue,
    metadata: &'static MarkupType,
    name: &str,
    types: &[ValueType],
    values: &[Option<BoxedValue>],
) -> Option<BoxedValue> {
    let method = metadata
        .find_methods(name)
        .find(|method| {
            method.parameters.len() == types.len()
                && method.parameters.iter().zip(types).all(|(a, b)| a() == *b)
        })
        .unwrap_or_else(|| panic!("missing {name}"));
    let mut arguments = vec![Some(target)];
    arguments.extend_from_slice(values);
    (method.invoke)(&arguments).unwrap()
}
fn targets(factory: &Rc<Factory>) -> [(BoxedValue, &'static MarkupType); 2] {
    [
        (
            Rc::new(factory.clone() as Rc<dyn IFactory>),
            <dyn IFactoryMetadata>::MARKUP,
        ),
        (
            Rc::new(FactoryBaseReference::new(factory.clone())),
            FactoryBaseReference::MARKUP,
        ),
    ]
}
fn ids(dock: &Rc<dyn IDock>) -> Vec<String> {
    dock.visible_dockables()
        .unwrap()
        .iter()
        .map(|item| item.expect("nonnull operation fixture").id())
        .collect()
}
#[test]
fn collection_methods_mutate_the_original_graph_through_both_factory_views() {
    crate::register_types();
    for use_base in [false, true] {
        let factory = Factory::new();
        let root = factory.create_root_dock();
        let left = factory.create_document_dock();
        let right = factory.create_document_dock();
        factory.add_dockable(Some(root.clone()), Some(left.clone()));
        factory.add_dockable(Some(root.clone()), Some(right.clone()));
        let a = factory.create_document();
        a.set_id("a".into());
        let b = factory.create_document();
        b.set_id("b".into());
        let c = factory.create_document();
        c.set_id("c".into());
        factory.add_dockable(Some(left.clone()), Some(a.clone()));
        factory.add_dockable(Some(left.clone()), Some(b.clone()));
        factory.init_layout(root.clone());
        let (target, metadata) = targets(&factory)[usize::from(use_base)].clone();
        let dock_type = ValueType::of::<Option<Rc<dyn IDock>>>();
        let item_type = ValueType::of::<Option<Rc<dyn IDockable>>>();
        invoke(
            target.clone(),
            metadata,
            "InsertDockable",
            &[
                ValueType::of::<Option<Rc<dyn IDock>>>(),
                ValueType::of::<Option<Rc<dyn IDockable>>>(),
                ValueType::of::<i32>(),
            ],
            &[
                Some(Rc::new(left.clone() as Rc<dyn IDock>)),
                Some(Rc::new(c.clone() as Rc<dyn IDockable>)),
                Some(Rc::new(1i32)),
            ],
        );
        assert_eq!(ids(&(left.clone() as Rc<dyn IDock>)), ["a", "c", "b"]);
        assert!(Rc::ptr_eq(
            &c.owner().unwrap(),
            &(left.clone() as Rc<dyn IDockable>)
        ));
        invoke(
            target.clone(),
            metadata,
            "MoveDockable",
            &[
                dock_type,
                dock_type,
                item_type,
                ValueType::of::<Option<Rc<dyn IDockable>>>(),
            ],
            &[
                Some(Rc::new(left.clone() as Rc<dyn IDock>)),
                Some(Rc::new(right.clone() as Rc<dyn IDock>)),
                Some(Rc::new(c.clone() as Rc<dyn IDockable>)),
                None,
            ],
        );
        assert_eq!(ids(&(left.clone() as Rc<dyn IDock>)), ["a", "b"]);
        assert_eq!(ids(&(right.clone() as Rc<dyn IDock>)), ["c"]);
        invoke(
            target.clone(),
            metadata,
            "SwapDockable",
            &[dock_type, dock_type, item_type, item_type],
            &[
                Some(Rc::new(left.clone() as Rc<dyn IDock>)),
                Some(Rc::new(right.clone() as Rc<dyn IDock>)),
                Some(Rc::new(a.clone() as Rc<dyn IDockable>)),
                Some(Rc::new(c.clone() as Rc<dyn IDockable>)),
            ],
        );
        assert_eq!(ids(&(left.clone() as Rc<dyn IDock>)), ["c", "b"]);
        assert_eq!(ids(&(right.clone() as Rc<dyn IDock>)), ["a"]);
        invoke(
            target,
            metadata,
            "RemoveDockable",
            &[
                ValueType::of::<Option<Rc<dyn IDockable>>>(),
                ValueType::of::<bool>(),
            ],
            &[Some(Rc::new(b as Rc<dyn IDockable>)), Some(Rc::new(false))],
        );
        assert_eq!(ids(&(left as Rc<dyn IDock>)), ["c"]);
    }
}
#[test]
fn search_metadata_retains_lazy_repeatable_source_queries_and_original_models() {
    crate::register_types();
    assert_eq!(
        DockableQueryReference::MARKUP.kind,
        ferroui_base::metadata::MarkupTypeKind::Interface
    );
    assert_eq!(
        DockableQueryEnumeratorReference::MARKUP.kind,
        ferroui_base::metadata::MarkupTypeKind::Interface
    );
    for (target, metadata) in targets(&Factory::new()) {
        let factory = from_markup_value::<Rc<dyn IFactory>>(&Some(target.clone())).unwrap();
        let root = factory.create_root_dock();
        root.set_id("root".into());
        let nested = factory.create_document_dock();
        nested.set_id("nested".into());
        let item = factory.create_document();
        item.set_id("item".into());
        factory.add_dockable(Some(root.clone()), Some(nested.clone()));
        factory.add_dockable(Some(nested), Some(item.clone()));
        let calls = Rc::new(Cell::new(0));
        let count = calls.clone();
        let predicate = DockablePredicateReference::new(Rc::new(move |_| {
            count.set(count.get() + 1);
            true
        }));
        let result = invoke(
            target.clone(),
            metadata,
            "Find",
            &[
                ValueType::of::<Option<Rc<dyn IDock>>>(),
                ValueType::of::<Option<DockablePredicateReference>>(),
            ],
            &[
                Some(Rc::new(root.clone() as Rc<dyn IDock>)),
                Some(Rc::new(predicate)),
            ],
        );
        let query = from_markup_value::<DockableQueryReference>(&result).unwrap();
        assert_eq!(calls.get(), 0);
        let first = query.get_enumerator();
        let second = query.get_enumerator();
        assert_eq!(calls.get(), 0);
        assert!(first.current().is_none());
        assert!(first.move_next());
        assert_eq!(first.current().unwrap().id(), "root");
        assert!(second.move_next());
        assert_eq!(second.current().unwrap().id(), "root");
        assert!(first.move_next());
        assert_eq!(first.current().unwrap().id(), "nested");
        assert!(first.move_next());
        assert_eq!(first.current().unwrap().id(), "nested");
        assert!(first.move_next());
        assert!(Rc::ptr_eq(
            &first.current().unwrap(),
            &(item.clone() as Rc<dyn IDockable>)
        ));
        assert!(!first.move_next());
        assert_eq!(first.current().unwrap().id(), "item");
        assert!(first.reset().is_err());
        first.dispose();
        assert!(!first.move_next());
        let added = factory.create_tool();
        added.set_id("later".into());
        factory.add_dockable(Some(root.clone()), Some(added));
        let third = query.get_enumerator();
        let mut sequence = Vec::new();
        while third.move_next() {
            sequence.push(third.current().unwrap().id());
        }
        assert_eq!(sequence, ["root", "nested", "nested", "item", "later"]);
        let selected = DockablePredicateReference::new(Rc::new(|value| {
            value.is_some_and(|item| item.id() == "item")
        }));
        let found = invoke(
            target.clone(),
            metadata,
            "FindDockable",
            &[
                ValueType::of::<Option<Rc<dyn IDock>>>(),
                ValueType::of::<Option<DockablePredicateReference>>(),
            ],
            &[
                Some(Rc::new(root.clone() as Rc<dyn IDock>)),
                Some(Rc::new(selected)),
            ],
        );
        assert!(Rc::ptr_eq(
            &from_markup_value::<Rc<dyn IDockable>>(&found).unwrap(),
            &(item.clone() as Rc<dyn IDockable>)
        ));
        let found = invoke(
            target,
            metadata,
            "FindRoot",
            &[ValueType::of::<Option<Rc<dyn IDockable>>>()],
            &[Some(Rc::new(item as Rc<dyn IDockable>))],
        );
        assert!(Rc::ptr_eq(
            &from_markup_value::<Rc<dyn IRootDock>>(&found).unwrap(),
            &root
        ));
    }
}

#[derive(PartialEq)]
struct PredicateMethods(Cell<i32>);
ferroui_base::ferro_markup_type!(class PredicateMethods {
    this:Rc<PredicateMethods>,handles:[Rc<PredicateMethods>],
    methods:[
        fn Match(Rc<dyn IDockable>)->bool => (|this:&Rc<PredicateMethods>,value:Rc<dyn IDockable>|{this.0.set(this.0.get()+1);value.id()=="root"}),
        fn WrongReturn(Rc<dyn IDockable>)->i32 => (|_this:&Rc<PredicateMethods>,_value:Rc<dyn IDockable>|1),
        fn WrongInput(String)->bool => (|_this:&Rc<PredicateMethods>,_value:String|true),
        fn WrongArity()->bool => (|_this:&Rc<PredicateMethods>|true),
    ],
});
#[test]
fn predicate_method_adapters_check_input_variance_and_boolean_return_without_invoking() {
    use ferroui_base::metadata::MarkupDelegate;
    crate::register_types();
    let owner = Rc::new(PredicateMethods(Cell::new(0)));
    let delegate = |name| {
        MarkupDelegate::for_method(
            Some(Rc::new(owner.clone())),
            PredicateMethods::MARKUP,
            PredicateMethods::MARKUP.find_methods(name).next().unwrap(),
        )
    };
    for name in ["WrongReturn", "WrongInput", "WrongArity"] {
        assert!(DockablePredicateReference::from_delegate(delegate(name)).is_none());
        assert!(RootPredicateReference::from_delegate(delegate(name)).is_none());
    }
    let root_match = RootPredicateReference::from_delegate(delegate("Match")).unwrap();
    let item_match = DockablePredicateReference::from_delegate(delegate("Match")).unwrap();
    assert_eq!(owner.0.get(), 0);
    let factory = Factory::new();
    let root = factory.create_root_dock();
    root.set_id("root".into());
    assert!(root_match.invoke(root.clone()));
    assert!(item_match.invoke(Some(root)));
    assert_eq!(owner.0.get(), 2);
}

#[test]
fn lazy_cursor_fault_disposal_and_metadata_preserve_current_and_source_iterator_state() {
    use ferrodock_model::DockableQuery;
    use ferroui_base::data::core::ValueTypes;
    use std::panic::{catch_unwind, AssertUnwindSafe};
    crate::register_types();
    let factory = Factory::new();
    let root = factory.create_root_dock();
    let calls = Rc::new(Cell::new(0));
    let count = calls.clone();
    let query = factory.find_in_dock(
        Some(root.clone()),
        Some(Rc::new(move |_| {
            let n = count.get();
            count.set(n + 1);
            if n > 0 {
                panic!("source predicate failure");
            }
            true
        })),
    );
    let doc = factory.create_document();
    factory.add_dockable(Some(root), Some(doc));
    let query = DockableQueryReference::new(query);
    let cursor = query.get_enumerator();
    let metadata = DockableQueryEnumeratorReference::MARKUP;
    let value: BoxedValue = Rc::new(cursor.clone());
    assert!(ValueTypes::reference_equals(
        &value,
        &(Rc::new(cursor.clone()) as BoxedValue)
    ));
    assert!(cursor.move_next());
    let previous = cursor.current().unwrap();
    assert!(catch_unwind(AssertUnwindSafe(|| cursor.move_next())).is_err());
    assert!(Rc::ptr_eq(&cursor.current().unwrap(), &previous));
    assert!(!cursor.move_next());
    assert_eq!(calls.get(), 2);
    assert!(
        (metadata.find_methods("Reset").next().unwrap().invoke)(&[Some(value.clone())]).is_err()
    );
    let disposable =
        from_markup_value::<Rc<dyn ferroui_system::IDisposable>>(&Some(value.clone())).unwrap();
    disposable.dispose();
    assert!(Rc::ptr_eq(&cursor.current().unwrap(), &previous));
    let other = DockableQueryReference::new(DockableQuery::new(|| Box::new(std::iter::empty())))
        .get_enumerator();
    other.dispose();
    assert!(!other.move_next());
    assert!(other.current().is_none());
    let result = (metadata.find_property("Current").unwrap().get.unwrap())(&[Some(value)]).unwrap();
    assert!(Rc::ptr_eq(
        &from_markup_value::<Rc<dyn IDockable>>(&result).unwrap(),
        &previous
    ));
}

#[test]
fn pin_members_use_the_original_root_and_preserve_pinned_order() {
    crate::register_types();
    for use_base in [false, true] {
        let factory = Factory::new();
        let root = factory.create_root_dock();
        let tools = factory.create_tool_dock();
        let a = factory.create_tool();
        a.set_id("a".into());
        let b = factory.create_tool();
        b.set_id("b".into());
        factory.add_dockable(Some(root.clone()), Some(tools.clone()));
        factory.add_dockable(Some(tools.clone()), Some(a.clone()));
        factory.add_dockable(Some(tools), Some(b.clone()));
        factory.init_layout(root.clone());
        factory.pin_dockable(Some(a.clone()));
        factory.pin_dockable(Some(b.clone()));
        let (target, metadata) = targets(&factory)[usize::from(use_base)].clone();
        let item_type = ValueType::of::<Option<Rc<dyn IDockable>>>();
        let pinned = invoke(
            target.clone(),
            metadata,
            "IsDockablePinned",
            &[ValueType::of::<Option<Rc<dyn IDockable>>>()],
            &[Some(Rc::new(a.clone() as Rc<dyn IDockable>))],
        );
        assert!(from_markup_value::<bool>(&pinned).unwrap());
        let pinned = invoke(
            target.clone(),
            metadata,
            "IsDockablePinned",
            &[
                ValueType::of::<Option<Rc<dyn IDockable>>>(),
                ValueType::of::<Option<Rc<dyn IRootDock>>>(),
            ],
            &[
                Some(Rc::new(a.clone() as Rc<dyn IDockable>)),
                Some(Rc::new(Some(root.clone()))),
            ],
        );
        assert!(from_markup_value::<bool>(&pinned).unwrap());
        invoke(
            target,
            metadata,
            "MovePinnedDockable",
            &[item_type, item_type],
            &[
                Some(Rc::new(a.clone() as Rc<dyn IDockable>)),
                Some(Rc::new(b.clone() as Rc<dyn IDockable>)),
            ],
        );
        assert_eq!(
            root.left_pinned_dockables()
                .unwrap()
                .iter()
                .map(|value| value.expect("nonnull pin fixture").id())
                .collect::<Vec<_>>(),
            ["b", "a"]
        );
    }
}

#[test]
fn window_and_activation_metadata_update_original_ownership_lists_and_tracking() {
    use ferrodock_model::core::{IDockWindow, IHostWindow};
    crate::register_types();
    for use_base in [false, true] {
        let factory = Factory::new();
        let root = factory.create_root_dock();
        let owner_root = factory.create_root_dock();
        let item = factory.create_document();
        factory.add_dockable(Some(root.clone()), Some(item.clone()));
        factory.init_layout(root.clone());
        let window = factory.create_dock_window();
        window.set_layout(Some(root.clone()));
        root.set_window(Some(window.clone()));
        let extra = factory.create_dock_window();
        let (target, metadata) = targets(&factory)[usize::from(use_base)].clone();
        let root_type = ValueType::of::<Option<Rc<dyn IRootDock>>>();
        let window_type = ValueType::of::<Option<Rc<dyn IDockWindow>>>();
        invoke(
            target.clone(),
            metadata,
            "InitDockWindow",
            &[
                window_type,
                ValueType::of::<Option<Rc<dyn IDockable>>>(),
                ValueType::of::<Option<Rc<dyn IHostWindow>>>(),
            ],
            &[
                Some(Rc::new(window.clone())),
                Some(Rc::new(Some(owner_root.clone() as Rc<dyn IDockable>))),
                None,
            ],
        );
        assert!(Rc::ptr_eq(
            &window.factory().unwrap(),
            &(factory.clone() as Rc<dyn IFactory>)
        ));
        assert!(Rc::ptr_eq(
            &window.owner().unwrap(),
            &(owner_root.clone() as Rc<dyn IDockable>)
        ));
        invoke(
            target.clone(),
            metadata,
            "AddWindow",
            &[root_type, window_type],
            &[
                Some(Rc::new(owner_root.clone())),
                Some(Rc::new(window.clone())),
            ],
        );
        invoke(
            target.clone(),
            metadata,
            "InsertWindow",
            &[root_type, window_type, ValueType::of::<i32>()],
            &[
                Some(Rc::new(owner_root.clone())),
                Some(Rc::new(extra.clone())),
                Some(Rc::new(0i32)),
            ],
        );
        assert_eq!(owner_root.windows().unwrap().count(), 2);
        assert!(Rc::ptr_eq(
            &owner_root
                .windows()
                .unwrap()
                .get(0)
                .expect("nonnull window fixture"),
            &extra
        ));
        invoke(
            target.clone(),
            metadata,
            "RemoveWindow",
            &[window_type],
            &[Some(Rc::new(extra))],
        );
        assert_eq!(owner_root.windows().unwrap().count(), 1);
        assert!(Rc::ptr_eq(
            &owner_root
                .windows()
                .unwrap()
                .get(0)
                .expect("nonnull window fixture"),
            &window
        ));
        invoke(
            target.clone(),
            metadata,
            "InitActiveDockable",
            &[
                ValueType::of::<Option<Rc<dyn IDockable>>>(),
                ValueType::of::<Option<Rc<dyn IDock>>>(),
            ],
            &[
                Some(Rc::new(Some(item.clone() as Rc<dyn IDockable>))),
                Some(Rc::new(root.clone() as Rc<dyn IDock>)),
            ],
        );
        assert!(Rc::ptr_eq(
            &factory.current_dockable().unwrap(),
            &(item.clone() as Rc<dyn IDockable>)
        ));
        invoke(
            target,
            metadata,
            "ActivateWindow",
            &[ValueType::of::<Option<Rc<dyn IDockable>>>()],
            &[Some(Rc::new(item as Rc<dyn IDockable>))],
        );
        assert!(Rc::ptr_eq(&factory.current_dock_window().unwrap(), &window));
    }
}

#[test]
fn predicate_adapter_passes_actual_native_null_and_original_reference_to_untyped_delegates() {
    use ferroui_base::metadata::MarkupDelegate;
    use std::cell::RefCell;
    crate::register_types();
    let seen = Rc::new(RefCell::new(Vec::new()));
    let calls = seen.clone();
    let predicate =
        DockablePredicateReference::from_delegate(MarkupDelegate::new(move |arguments| {
            assert_eq!(arguments.len(), 1);
            let item = arguments[0].as_ref().map(|value| {
                value
                    .downcast_ref::<Rc<dyn IDockable>>()
                    .expect("original native dockable reference")
                    .clone()
            });
            calls.borrow_mut().push(item);
            Some(Rc::new(true))
        }))
        .unwrap();
    let factory = Factory::new();
    let item: Rc<dyn IDockable> = factory.create_document();
    assert!(predicate.invoke(None));
    assert!(predicate.invoke(Some(item.clone())));
    let calls = seen.borrow();
    assert_eq!(calls.len(), 2);
    assert!(calls[0].is_none());
    assert!(Rc::ptr_eq(calls[1].as_ref().unwrap(), &item));
}

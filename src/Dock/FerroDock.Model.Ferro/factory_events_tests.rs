//! Metadata integration checks using the unchanged native factory engine.
use crate::{event_metadata::*, register_types, FactoryBaseReference};
use ferrodock_model::core::{events::*, *};
use ferrodock_model_reactiveui::Factory;
use ferroui_base::{
    data::core::ValueTypes,
    metadata::{MarkupDelegate, MarkupType, MarkupTyped, MarkupValue},
    BoxedValue,
};
use ferroui_system::IDisposable;
use std::{cell::RefCell, rc::Rc};
fn subscribe(
    metadata: &'static MarkupType,
    value: BoxedValue,
    name: &str,
    handler: MarkupDelegate,
) -> Rc<dyn IDisposable> {
    let event = metadata.find_event(name).unwrap();
    assert_eq!(event.arguments.len(), 2);
    let model = value
        .downcast_ref::<FactoryBaseReference>()
        .unwrap()
        .model();
    match name {
        "ActiveDockableChanged" => crate::factory_events::active_dockable_changed(&model, handler),
        "FocusedDockableChanged" => {
            crate::factory_events::focused_dockable_changed(&model, handler)
        }
        "GlobalDockTrackingChanged" => {
            crate::factory_events::global_dock_tracking_changed(&model, handler)
        }
        "DockableInit" => crate::factory_events::dockable_init(&model, handler),
        "DockableAdded" => crate::factory_events::dockable_added(&model, handler),
        "DockableRemoved" => crate::factory_events::dockable_removed(&model, handler),
        "DockableClosing" => crate::factory_events::dockable_closing(&model, handler),
        "DockableClosed" => crate::factory_events::dockable_closed(&model, handler),
        "DockableMoved" => crate::factory_events::dockable_moved(&model, handler),
        "DockableDocked" => crate::factory_events::dockable_docked(&model, handler),
        "DockableUndocked" => crate::factory_events::dockable_undocked(&model, handler),
        "DockableSwapped" => crate::factory_events::dockable_swapped(&model, handler),
        "DockablePinned" => crate::factory_events::dockable_pinned(&model, handler),
        "DockableUnpinned" => crate::factory_events::dockable_unpinned(&model, handler),
        "DockableHidden" => crate::factory_events::dockable_hidden(&model, handler),
        "DockableRestored" => crate::factory_events::dockable_restored(&model, handler),
        "WindowOpened" => crate::factory_events::window_opened(&model, handler),
        "WindowClosing" => crate::factory_events::window_closing(&model, handler),
        "WindowClosed" => crate::factory_events::window_closed(&model, handler),
        "WindowAdded" => crate::factory_events::window_added(&model, handler),
        "WindowRemoved" => crate::factory_events::window_removed(&model, handler),
        "WindowMoveDragBegin" => crate::factory_events::window_move_drag_begin(&model, handler),
        "WindowMoveDrag" => crate::factory_events::window_move_drag(&model, handler),
        "WindowMoveDragEnd" => crate::factory_events::window_move_drag_end(&model, handler),
        "WindowActivated" => crate::factory_events::window_activated(&model, handler),
        "DockableActivated" => crate::factory_events::dockable_activated(&model, handler),
        "WindowDeactivated" => crate::factory_events::window_deactivated(&model, handler),
        "DockableDeactivated" => crate::factory_events::dockable_deactivated(&model, handler),
        _ => panic!("unexpected source event {name}"),
    }
}
fn invoke(value: BoxedValue, name: &str, args: &[MarkupValue]) -> MarkupValue {
    let method = FactoryBaseReference::MARKUP
        .find_methods(name)
        .find(|v| v.parameters.len() == args.len())
        .unwrap();
    let mut parameters = vec![Some(value)];
    parameters.extend_from_slice(args);
    (method.invoke)(&parameters).unwrap()
}
#[test]
fn all_source_factory_events_expose_the_original_sender_shared_arguments_and_disposal() {
    register_types();
    let metadata = FactoryBaseReference::MARKUP;
    assert_eq!(metadata.events.len(), 28);
    macro_rules! check {
 ($args:ty,$name:literal,$trigger:literal,$target:ident,$verify:expr $(,$operation:expr)?)=>{{
 let factory=Factory::new();let model:Rc<dyn IFactory>=factory.clone();let value:BoxedValue=Rc::new(FactoryBaseReference::new(factory.clone()));
 let target=match stringify!($target){"window"=>None,_=>Some(factory.create_document() as Rc<dyn IDockable>)};
 let root=factory.create_root_dock();if let Some(doc)=&target {doc.set_owner(Some(root.clone()));}
 let window=factory.create_dock_window();window.set_layout(Some(root.clone()));root.set_window(Some(window.clone()));
 let arguments:Vec<MarkupValue>=if stringify!($target)=="window"{vec![Some(Rc::new(Some(window.clone())))]}else{vec![Some(Rc::new(target.clone()))]};
 $(let arguments={let mut extended=arguments;extended.push(Some(Rc::new($operation)));extended};)?
 let seen=Rc::new(RefCell::new(Vec::new()));let record=seen.clone();let sender=Rc::downgrade(&model);let expected=target.clone();let expected_window=window.clone();
 let handler=MarkupDelegate::new(move |arguments|{
 assert_eq!(arguments.len(),2);let actual=arguments[0].as_ref().unwrap().downcast_ref::<Rc<dyn IFactory>>().unwrap();assert!(Rc::ptr_eq(actual,&sender.upgrade().unwrap()));
 let args=arguments[1].as_ref().unwrap().downcast_ref::<$args>().unwrap();
 ($verify)(args,expected.clone(),expected_window.clone());record.borrow_mut().push(args.clone());None
 });
 let subscription=subscribe(metadata,value.clone(),$name,handler.clone());
 invoke(value.clone(),$trigger,&arguments);assert_eq!(seen.borrow().len(),1,$name);
 subscription.dispose();subscription.dispose();invoke(value.clone(),$trigger,&arguments);assert_eq!(seen.borrow().len(),1,concat!("disposed ",$name));
 // Reset through the original protected hook before checking a second tracking transition.
 if $name=="GlobalDockTrackingChanged" { ferrodock_model::FactoryBaseImpl::factory_base(factory.as_ref()).update_global_dock_tracking(None,None,None,DockTrackingChangeReason::Unknown); }
 // Native event-add metadata follows the CLR void add accessor contract.
 assert!((metadata.find_event($name).unwrap().add)(&[Some(value.clone()),Some(Rc::new(handler))]).unwrap().is_none());
 invoke(value,$trigger,&arguments);assert_eq!(seen.borrow().len(),2,concat!("metadata ",$name));
 }};
 }
    check!(
        ActiveDockableChangedEventArgs,
        "ActiveDockableChanged",
        "OnActiveDockableChanged",
        dockable,
        |v: &ActiveDockableChangedEventArgs, doc: Option<Rc<dyn IDockable>>, _| assert!(
            Rc::ptr_eq(&v.dockable().unwrap(), &doc.unwrap())
        )
    );
    check!(
        FocusedDockableChangedEventArgs,
        "FocusedDockableChanged",
        "OnFocusedDockableChanged",
        dockable,
        |v: &FocusedDockableChangedEventArgs, doc: Option<Rc<dyn IDockable>>, _| assert!(
            Rc::ptr_eq(&v.dockable().unwrap(), &doc.unwrap())
        )
    );
    check!(
        GlobalDockTrackingChangedEventArgs,
        "GlobalDockTrackingChanged",
        "OnActiveDockableChanged",
        dockable,
        |v: &GlobalDockTrackingChangedEventArgs, _, _| {
            assert!(v.current().dockable().is_some());
            assert_eq!(v.reason(), DockTrackingChangeReason::ActiveDockableChanged);
        }
    );
    check!(
        DockableInitEventArgs,
        "DockableInit",
        "OnDockableInit",
        dockable,
        |v: &DockableInitEventArgs, doc: Option<Rc<dyn IDockable>>, _| assert!(Rc::ptr_eq(
            &v.dockable().unwrap(),
            &doc.unwrap()
        ))
    );
    check!(
        DockableAddedEventArgs,
        "DockableAdded",
        "OnDockableAdded",
        dockable,
        |v: &DockableAddedEventArgs, doc: Option<Rc<dyn IDockable>>, _| assert!(Rc::ptr_eq(
            &v.dockable().unwrap(),
            &doc.unwrap()
        ))
    );
    check!(
        DockableRemovedEventArgs,
        "DockableRemoved",
        "OnDockableRemoved",
        dockable,
        |v: &DockableRemovedEventArgs, doc: Option<Rc<dyn IDockable>>, _| assert!(Rc::ptr_eq(
            &v.dockable().unwrap(),
            &doc.unwrap()
        ))
    );
    check!(
        DockableClosingEventArgs,
        "DockableClosing",
        "OnDockableClosing",
        dockable,
        |v: &DockableClosingEventArgs, doc: Option<Rc<dyn IDockable>>, _| assert!(Rc::ptr_eq(
            &v.dockable().unwrap(),
            &doc.unwrap()
        ))
    );
    check!(
        DockableClosedEventArgs,
        "DockableClosed",
        "OnDockableClosed",
        dockable,
        |v: &DockableClosedEventArgs, doc: Option<Rc<dyn IDockable>>, _| assert!(Rc::ptr_eq(
            &v.dockable().unwrap(),
            &doc.unwrap()
        ))
    );
    check!(
        DockableMovedEventArgs,
        "DockableMoved",
        "OnDockableMoved",
        dockable,
        |v: &DockableMovedEventArgs, doc: Option<Rc<dyn IDockable>>, _| assert!(Rc::ptr_eq(
            &v.dockable().unwrap(),
            &doc.unwrap()
        ))
    );
    check!(
        DockableDockedEventArgs,
        "DockableDocked",
        "OnDockableDocked",
        dockable,
        |v: &DockableDockedEventArgs, doc: Option<Rc<dyn IDockable>>, _| assert!(Rc::ptr_eq(
            &v.dockable().unwrap(),
            &doc.unwrap()
        )),
        DockOperation::Left
    );
    check!(
        DockableUndockedEventArgs,
        "DockableUndocked",
        "OnDockableUndocked",
        dockable,
        |v: &DockableUndockedEventArgs, doc: Option<Rc<dyn IDockable>>, _| assert!(Rc::ptr_eq(
            &v.dockable().unwrap(),
            &doc.unwrap()
        )),
        DockOperation::Left
    );
    check!(
        DockableSwappedEventArgs,
        "DockableSwapped",
        "OnDockableSwapped",
        dockable,
        |v: &DockableSwappedEventArgs, doc: Option<Rc<dyn IDockable>>, _| assert!(Rc::ptr_eq(
            &v.dockable().unwrap(),
            &doc.unwrap()
        ))
    );
    check!(
        DockablePinnedEventArgs,
        "DockablePinned",
        "OnDockablePinned",
        dockable,
        |v: &DockablePinnedEventArgs, doc: Option<Rc<dyn IDockable>>, _| assert!(Rc::ptr_eq(
            &v.dockable().unwrap(),
            &doc.unwrap()
        ))
    );
    check!(
        DockableUnpinnedEventArgs,
        "DockableUnpinned",
        "OnDockableUnpinned",
        dockable,
        |v: &DockableUnpinnedEventArgs, doc: Option<Rc<dyn IDockable>>, _| assert!(Rc::ptr_eq(
            &v.dockable().unwrap(),
            &doc.unwrap()
        ))
    );
    check!(
        DockableHiddenEventArgs,
        "DockableHidden",
        "OnDockableHidden",
        dockable,
        |v: &DockableHiddenEventArgs, doc: Option<Rc<dyn IDockable>>, _| assert!(Rc::ptr_eq(
            &v.dockable().unwrap(),
            &doc.unwrap()
        ))
    );
    check!(
        DockableRestoredEventArgs,
        "DockableRestored",
        "OnDockableRestored",
        dockable,
        |v: &DockableRestoredEventArgs, doc: Option<Rc<dyn IDockable>>, _| assert!(Rc::ptr_eq(
            &v.dockable().unwrap(),
            &doc.unwrap()
        ))
    );
    check!(
        WindowOpenedEventArgs,
        "WindowOpened",
        "OnWindowOpened",
        window,
        |v: &WindowOpenedEventArgs, _, window| assert!(Rc::ptr_eq(&v.window().unwrap(), &window))
    );
    check!(
        WindowClosingEventArgs,
        "WindowClosing",
        "OnWindowClosing",
        window,
        |v: &WindowClosingEventArgs, _, window| assert!(Rc::ptr_eq(&v.window().unwrap(), &window))
    );
    check!(
        WindowClosedEventArgs,
        "WindowClosed",
        "OnWindowClosed",
        window,
        |v: &WindowClosedEventArgs, _, window| assert!(Rc::ptr_eq(&v.window().unwrap(), &window))
    );
    check!(
        WindowAddedEventArgs,
        "WindowAdded",
        "OnWindowAdded",
        window,
        |v: &WindowAddedEventArgs, _, window| assert!(Rc::ptr_eq(&v.window().unwrap(), &window))
    );
    check!(
        WindowRemovedEventArgs,
        "WindowRemoved",
        "OnWindowRemoved",
        window,
        |v: &WindowRemovedEventArgs, _, window| assert!(Rc::ptr_eq(&v.window().unwrap(), &window))
    );
    check!(
        WindowMoveDragBeginEventArgs,
        "WindowMoveDragBegin",
        "OnWindowMoveDragBegin",
        window,
        |v: &WindowMoveDragBeginEventArgs, _, window| assert!(Rc::ptr_eq(
            &v.window().unwrap(),
            &window
        ))
    );
    check!(
        WindowMoveDragEventArgs,
        "WindowMoveDrag",
        "OnWindowMoveDrag",
        window,
        |v: &WindowMoveDragEventArgs, _, window| assert!(Rc::ptr_eq(&v.window().unwrap(), &window))
    );
    check!(
        WindowMoveDragEndEventArgs,
        "WindowMoveDragEnd",
        "OnWindowMoveDragEnd",
        window,
        |v: &WindowMoveDragEndEventArgs, _, window| assert!(Rc::ptr_eq(
            &v.window().unwrap(),
            &window
        ))
    );
    check!(
        WindowActivatedEventArgs,
        "WindowActivated",
        "OnWindowActivated",
        window,
        |v: &WindowActivatedEventArgs, _, window| assert!(Rc::ptr_eq(
            &v.window().unwrap(),
            &window
        ))
    );
    check!(
        DockableActivatedEventArgs,
        "DockableActivated",
        "OnDockableActivated",
        dockable,
        |v: &DockableActivatedEventArgs, doc: Option<Rc<dyn IDockable>>, _| assert!(Rc::ptr_eq(
            &v.dockable().unwrap(),
            &doc.unwrap()
        ))
    );
    check!(
        WindowDeactivatedEventArgs,
        "WindowDeactivated",
        "OnWindowDeactivated",
        window,
        |v: &WindowDeactivatedEventArgs, _, window| assert!(Rc::ptr_eq(
            &v.window().unwrap(),
            &window
        ))
    );
    check!(
        DockableDeactivatedEventArgs,
        "DockableDeactivated",
        "OnDockableDeactivated",
        dockable,
        |v: &DockableDeactivatedEventArgs, doc: Option<Rc<dyn IDockable>>, _| assert!(Rc::ptr_eq(
            &v.dockable().unwrap(),
            &doc.unwrap()
        ))
    );
}
#[test]
fn metadata_handlers_mutate_the_same_cancel_and_context_arguments_used_by_the_engine() {
    register_types();
    let factory = Factory::new();
    let value: BoxedValue = Rc::new(FactoryBaseReference::new(factory.clone()));
    for name in ["DockableClosing", "WindowClosing", "WindowMoveDragBegin"] {
        let captured = Rc::new(RefCell::new(None));
        let record = captured.clone();
        let subscription = subscribe(
            FactoryBaseReference::MARKUP,
            value.clone(),
            name,
            MarkupDelegate::new(move |args| {
                let value = args[1].clone().unwrap();
                let metadata =
                    MarkupType::find_by_handle(value.as_ref().as_any().type_id()).unwrap();
                (metadata.find_property("Cancel").unwrap().set.unwrap())(&[
                    Some(value.clone()),
                    Some(Rc::new(true)),
                ])
                .unwrap();
                *record.borrow_mut() = Some(value);
                None
            }),
        );
        let result = invoke(value.clone(), &format!("On{name}"), &[None]).unwrap();
        assert_eq!(result.downcast_ref::<bool>(), Some(&false));
        let saved = captured.borrow().clone().unwrap();
        let metadata = MarkupType::find_by_handle(saved.as_ref().as_any().type_id()).unwrap();
        assert_eq!(
            (metadata.find_property("Cancel").unwrap().get.unwrap())(&[Some(saved)])
                .unwrap()
                .unwrap()
                .downcast_ref::<bool>(),
            Some(&true)
        );
        subscription.dispose();
        assert_eq!(
            invoke(value.clone(), &format!("On{name}"), &[None])
                .unwrap()
                .downcast_ref::<bool>(),
            Some(&true)
        );
    }
    let doc = factory.create_document();
    let context: BoxedValue = Rc::new("set by source event".to_owned());
    let replacement = context.clone();
    let subscription = subscribe(
        FactoryBaseReference::MARKUP,
        value.clone(),
        "DockableInit",
        MarkupDelegate::new(move |args| {
            let metadata = <dyn DockableInitEventArgsMetadata as MarkupTyped>::MARKUP;
            (metadata.find_property("Context").unwrap().set.unwrap())(&[
                args[1].clone(),
                Some(replacement.clone()),
            ])
            .unwrap();
            None
        }),
    );
    invoke(
        value,
        "OnDockableInit",
        &[Some(Rc::new(Some(doc.clone() as Rc<dyn IDockable>)))],
    );
    let stored = ferrodock_runtime::from_source_value(&doc.context().unwrap());
    assert!(Rc::ptr_eq(&stored, &context));
    subscription.dispose();
}
fn check_argument<T: Clone + PartialEq + 'static>(
    metadata: &'static MarkupType,
    args: &[MarkupValue],
    check: impl Fn(&T),
) {
    let constructor = metadata
        .constructors
        .iter()
        .find(|v| v.parameters.len() == args.len())
        .unwrap();
    let value = (constructor.invoke)(args).unwrap().unwrap();
    let original = value.downcast_ref::<T>().unwrap();
    check(original);
    let clone: BoxedValue = Rc::new(original.clone());
    assert!(ValueTypes::reference_equals(&value, &clone));
    let other = (constructor.invoke)(args).unwrap().unwrap();
    assert!(!ValueTypes::reference_equals(&value, &other));
    for property in metadata.properties {
        assert!(property.get.is_some());
        (property.get.unwrap())(&[Some(value.clone())]).unwrap();
    }
}
#[test]
fn every_event_argument_constructor_property_and_tracking_snapshot_uses_its_actual_source_type() {
    register_types();
    let factory = Factory::new();
    let doc: Rc<dyn IDockable> = factory.create_document();
    let window = factory.create_dock_window();
    let root = factory.create_root_dock();
    check_argument::<ActiveDockableChangedEventArgs>(
        <dyn ActiveDockableChangedEventArgsMetadata as MarkupTyped>::MARKUP,
        &[Some(Rc::new(Some(doc.clone())))],
        |v| {
            assert!(Rc::ptr_eq(&v.dockable().unwrap(), &doc));
            assert!(v.root_dock().is_none());
        },
    );
    check_argument::<ActiveDockableChangedEventArgs>(
        <dyn ActiveDockableChangedEventArgsMetadata as MarkupTyped>::MARKUP,
        &[
            Some(Rc::new(Some(doc.clone()))),
            Some(Rc::new(Some(root.clone()))),
            Some(Rc::new(Some(window.clone()))),
        ],
        |v| {
            assert!(Rc::ptr_eq(&v.root_dock().unwrap(), &root));
            assert!(Rc::ptr_eq(&v.window().unwrap(), &window));
        },
    );
    check_argument::<FocusedDockableChangedEventArgs>(
        <dyn FocusedDockableChangedEventArgsMetadata as MarkupTyped>::MARKUP,
        &[Some(Rc::new(Some(doc.clone())))],
        |v| {
            assert!(Rc::ptr_eq(&v.dockable().unwrap(), &doc));
            assert!(v.root_dock().is_none());
        },
    );
    check_argument::<FocusedDockableChangedEventArgs>(
        <dyn FocusedDockableChangedEventArgsMetadata as MarkupTyped>::MARKUP,
        &[
            Some(Rc::new(Some(doc.clone()))),
            Some(Rc::new(Some(root.clone()))),
            Some(Rc::new(Some(window.clone()))),
        ],
        |v| {
            assert!(Rc::ptr_eq(&v.root_dock().unwrap(), &root));
            assert!(Rc::ptr_eq(&v.window().unwrap(), &window));
        },
    );
    check_argument::<GlobalDockTrackingChangedEventArgs>(
        <dyn GlobalDockTrackingChangedEventArgsMetadata as MarkupTyped>::MARKUP,
        &[
            Some(Rc::new(GlobalDockTrackingState::empty())),
            Some(Rc::new(GlobalDockTrackingState::new(
                Some(doc.clone()),
                Some(root.clone()),
                Some(window.clone()),
            ))),
            Some(Rc::new(DockTrackingChangeReason::WindowActivated)),
        ],
        |v| {
            assert!(v.previous().dockable().is_none());
            assert!(Rc::ptr_eq(&v.current().dockable().unwrap(), &doc));
            assert_eq!(v.reason(), DockTrackingChangeReason::WindowActivated);
        },
    );
    check_argument::<DockableInitEventArgs>(
        <dyn DockableInitEventArgsMetadata as MarkupTyped>::MARKUP,
        &[Some(Rc::new(Some(doc.clone())))],
        |v| {
            assert!(Rc::ptr_eq(&v.dockable().unwrap(), &doc));
        },
    );
    check_argument::<DockableAddedEventArgs>(
        <dyn DockableAddedEventArgsMetadata as MarkupTyped>::MARKUP,
        &[Some(Rc::new(Some(doc.clone())))],
        |v| {
            assert!(Rc::ptr_eq(&v.dockable().unwrap(), &doc));
        },
    );
    check_argument::<DockableRemovedEventArgs>(
        <dyn DockableRemovedEventArgsMetadata as MarkupTyped>::MARKUP,
        &[Some(Rc::new(Some(doc.clone())))],
        |v| {
            assert!(Rc::ptr_eq(&v.dockable().unwrap(), &doc));
        },
    );
    check_argument::<DockableClosingEventArgs>(
        <dyn DockableClosingEventArgsMetadata as MarkupTyped>::MARKUP,
        &[Some(Rc::new(Some(doc.clone())))],
        |v| {
            assert!(Rc::ptr_eq(&v.dockable().unwrap(), &doc));
            assert!(!v.cancel());
        },
    );
    check_argument::<DockableClosedEventArgs>(
        <dyn DockableClosedEventArgsMetadata as MarkupTyped>::MARKUP,
        &[Some(Rc::new(Some(doc.clone())))],
        |v| {
            assert!(Rc::ptr_eq(&v.dockable().unwrap(), &doc));
        },
    );
    check_argument::<DockableMovedEventArgs>(
        <dyn DockableMovedEventArgsMetadata as MarkupTyped>::MARKUP,
        &[Some(Rc::new(Some(doc.clone())))],
        |v| {
            assert!(Rc::ptr_eq(&v.dockable().unwrap(), &doc));
        },
    );
    check_argument::<DockableDockedEventArgs>(
        <dyn DockableDockedEventArgsMetadata as MarkupTyped>::MARKUP,
        &[
            Some(Rc::new(Some(doc.clone()))),
            Some(Rc::new(DockOperation::Right)),
        ],
        |v| {
            assert!(Rc::ptr_eq(&v.dockable().unwrap(), &doc));
            assert_eq!(v.operation(), DockOperation::Right);
        },
    );
    check_argument::<DockableUndockedEventArgs>(
        <dyn DockableUndockedEventArgsMetadata as MarkupTyped>::MARKUP,
        &[
            Some(Rc::new(Some(doc.clone()))),
            Some(Rc::new(DockOperation::Right)),
        ],
        |v| {
            assert!(Rc::ptr_eq(&v.dockable().unwrap(), &doc));
            assert_eq!(v.operation(), DockOperation::Right);
        },
    );
    check_argument::<DockableSwappedEventArgs>(
        <dyn DockableSwappedEventArgsMetadata as MarkupTyped>::MARKUP,
        &[Some(Rc::new(Some(doc.clone())))],
        |v| {
            assert!(Rc::ptr_eq(&v.dockable().unwrap(), &doc));
        },
    );
    check_argument::<DockablePinnedEventArgs>(
        <dyn DockablePinnedEventArgsMetadata as MarkupTyped>::MARKUP,
        &[Some(Rc::new(Some(doc.clone())))],
        |v| {
            assert!(Rc::ptr_eq(&v.dockable().unwrap(), &doc));
        },
    );
    check_argument::<DockableUnpinnedEventArgs>(
        <dyn DockableUnpinnedEventArgsMetadata as MarkupTyped>::MARKUP,
        &[Some(Rc::new(Some(doc.clone())))],
        |v| {
            assert!(Rc::ptr_eq(&v.dockable().unwrap(), &doc));
        },
    );
    check_argument::<DockableHiddenEventArgs>(
        <dyn DockableHiddenEventArgsMetadata as MarkupTyped>::MARKUP,
        &[Some(Rc::new(Some(doc.clone())))],
        |v| {
            assert!(Rc::ptr_eq(&v.dockable().unwrap(), &doc));
        },
    );
    check_argument::<DockableRestoredEventArgs>(
        <dyn DockableRestoredEventArgsMetadata as MarkupTyped>::MARKUP,
        &[Some(Rc::new(Some(doc.clone())))],
        |v| {
            assert!(Rc::ptr_eq(&v.dockable().unwrap(), &doc));
        },
    );
    check_argument::<WindowOpenedEventArgs>(
        <dyn WindowOpenedEventArgsMetadata as MarkupTyped>::MARKUP,
        &[Some(Rc::new(Some(window.clone())))],
        |v| {
            assert!(Rc::ptr_eq(&v.window().unwrap(), &window));
        },
    );
    check_argument::<WindowClosingEventArgs>(
        <dyn WindowClosingEventArgsMetadata as MarkupTyped>::MARKUP,
        &[Some(Rc::new(Some(window.clone())))],
        |v| {
            assert!(Rc::ptr_eq(&v.window().unwrap(), &window));
            assert!(!v.cancel());
        },
    );
    check_argument::<WindowClosedEventArgs>(
        <dyn WindowClosedEventArgsMetadata as MarkupTyped>::MARKUP,
        &[Some(Rc::new(Some(window.clone())))],
        |v| {
            assert!(Rc::ptr_eq(&v.window().unwrap(), &window));
        },
    );
    check_argument::<WindowAddedEventArgs>(
        <dyn WindowAddedEventArgsMetadata as MarkupTyped>::MARKUP,
        &[Some(Rc::new(Some(window.clone())))],
        |v| {
            assert!(Rc::ptr_eq(&v.window().unwrap(), &window));
        },
    );
    check_argument::<WindowRemovedEventArgs>(
        <dyn WindowRemovedEventArgsMetadata as MarkupTyped>::MARKUP,
        &[Some(Rc::new(Some(window.clone())))],
        |v| {
            assert!(Rc::ptr_eq(&v.window().unwrap(), &window));
        },
    );
    check_argument::<WindowMoveDragBeginEventArgs>(
        <dyn WindowMoveDragBeginEventArgsMetadata as MarkupTyped>::MARKUP,
        &[Some(Rc::new(Some(window.clone())))],
        |v| {
            assert!(Rc::ptr_eq(&v.window().unwrap(), &window));
            assert!(!v.cancel());
        },
    );
    check_argument::<WindowMoveDragEventArgs>(
        <dyn WindowMoveDragEventArgsMetadata as MarkupTyped>::MARKUP,
        &[Some(Rc::new(Some(window.clone())))],
        |v| {
            assert!(Rc::ptr_eq(&v.window().unwrap(), &window));
        },
    );
    check_argument::<WindowMoveDragEndEventArgs>(
        <dyn WindowMoveDragEndEventArgsMetadata as MarkupTyped>::MARKUP,
        &[Some(Rc::new(Some(window.clone())))],
        |v| {
            assert!(Rc::ptr_eq(&v.window().unwrap(), &window));
        },
    );
    check_argument::<WindowActivatedEventArgs>(
        <dyn WindowActivatedEventArgsMetadata as MarkupTyped>::MARKUP,
        &[Some(Rc::new(Some(window.clone())))],
        |v| {
            assert!(Rc::ptr_eq(&v.window().unwrap(), &window));
        },
    );
    check_argument::<DockableActivatedEventArgs>(
        <dyn DockableActivatedEventArgsMetadata as MarkupTyped>::MARKUP,
        &[Some(Rc::new(Some(doc.clone())))],
        |v| {
            assert!(Rc::ptr_eq(&v.dockable().unwrap(), &doc));
        },
    );
    check_argument::<WindowDeactivatedEventArgs>(
        <dyn WindowDeactivatedEventArgsMetadata as MarkupTyped>::MARKUP,
        &[Some(Rc::new(Some(window.clone())))],
        |v| {
            assert!(Rc::ptr_eq(&v.window().unwrap(), &window));
        },
    );
    check_argument::<DockableDeactivatedEventArgs>(
        <dyn DockableDeactivatedEventArgsMetadata as MarkupTyped>::MARKUP,
        &[Some(Rc::new(Some(doc.clone())))],
        |v| {
            assert!(Rc::ptr_eq(&v.dockable().unwrap(), &doc));
        },
    );
    check_argument::<DockableOwnerChangingEventArgs>(
        <dyn DockableOwnerChangingEventArgsMetadata as MarkupTyped>::MARKUP,
        &[Some(Rc::new(Some(doc.clone()))), None],
        |v| {
            assert!(Rc::ptr_eq(&v.old_owner().unwrap(), &doc));
            assert!(v.new_owner().is_none());
        },
    );
    let metadata = <dyn GlobalDockTrackingStateMetadata as MarkupTyped>::MARKUP;
    check_argument::<GlobalDockTrackingState>(
        metadata,
        &[
            Some(Rc::new(Some(doc.clone()))),
            Some(Rc::new(Some(root.clone()))),
            Some(Rc::new(Some(window.clone()))),
        ],
        |v| {
            assert!(Rc::ptr_eq(&v.dockable().unwrap(), &doc));
            assert!(Rc::ptr_eq(&v.root_dock().unwrap(), &root));
            assert!(Rc::ptr_eq(&v.window().unwrap(), &window));
        },
    );
    let empty = metadata.find_static_property("Empty").unwrap().get.unwrap();
    let first = empty(&[]).unwrap().unwrap();
    let second = empty(&[]).unwrap().unwrap();
    assert!(ValueTypes::reference_equals(&first, &second));
    // The explicit-root overload must use that root even when Window.Layout is null.
    doc.set_owner(Some(root.clone()));
    root.set_focused_dockable(Some(doc.clone()));
    invoke(
        Rc::new(FactoryBaseReference::new(factory.clone())),
        "OnWindowActivated",
        &[
            Some(Rc::new(Some(window.clone()))),
            Some(Rc::new(Some(root.clone()))),
        ],
    );
    assert!(Rc::ptr_eq(&factory.current_root_dock().unwrap(), &root));
    // Source focus restoration rereads Root.Window, which is null in this graph.
    assert!(factory.current_dock_window().is_none());
    assert!(Rc::ptr_eq(&factory.current_dockable().unwrap(), &doc));
}

#[test]
fn source_subscriptions_survive_ignored_handles_but_do_not_keep_the_publisher_alive() {
    register_types();
    let factory = Factory::new();
    let model: Rc<dyn IFactory> = factory.clone();
    let weak = Rc::downgrade(&factory);
    let calls = Rc::new(std::cell::Cell::new(0));
    let count = calls.clone();
    let subscription = crate::factory_events::dockable_added(
        &model,
        MarkupDelegate::new(move |_| {
            count.set(count.get() + 1);
            None
        }),
    );
    drop(subscription);
    factory.on_dockable_added(None);
    assert_eq!(calls.get(), 1);
    let marker = Rc::new(());
    let marker_weak = Rc::downgrade(&marker);
    let subscription = crate::factory_events::window_added(
        &model,
        MarkupDelegate::new(move |_| {
            let _keep = &marker;
            None
        }),
    );
    drop(model);
    drop(factory);
    assert!(weak.upgrade().is_none());
    assert!(marker_weak.upgrade().is_none());
    subscription.dispose();
    subscription.dispose();
}

//! Virtual factory overloads preserve enclosing dispatch, argument order and original identity.
use crate::{FactoryBaseReference, IFactoryMetadata};
use ferrodock_model::{controls::IRootDock, core::*, FactoryBase, FactoryBaseImpl};
use ferrodock_model_reactiveui::{impl_factory, Factory, FactoryImpl};
use ferroui_base::{
    data::core::ValueType,
    metadata::{from_markup_value, MarkupTyped},
    BoxedValue,
};
use std::{cell::RefCell, rc::Rc};
type RecordedCall = (&'static str, Vec<ferrodock_model::BoxedValue>);
struct FactoryProbe {
    component: Factory,
    calls: RefCell<Vec<RecordedCall>>,
    item: Rc<dyn IDockable>,
    dock: Rc<dyn IDock>,
    window: Rc<dyn IDockWindow>,
}
impl FactoryProbe {
    fn new() -> Rc<Self> {
        let creator = Factory::new();
        Rc::new_cyclic(|runtime| Self {
            component: Factory::construct(runtime.clone()),
            calls: Default::default(),
            item: creator.create_document(),
            dock: creator.create_document_dock(),
            window: creator.create_dock_window(),
        })
    }
}
impl FactoryBaseImpl for FactoryProbe {
    fn factory_base(&self) -> &FactoryBase {
        self.component.base()
    }
}
impl FactoryImpl for FactoryProbe {
    fn factory(&self) -> &Factory {
        &self.component
    }
    #[allow(clippy::too_many_arguments)]
    fn get_container_from_item(
        this: &Self,
        item: Option<ferrodock_model::BoxedValue>,
    ) -> Option<Rc<dyn IDockable>> {
        this.calls
            .borrow_mut()
            .push(("get_container_from_item", vec![Rc::new(item)]));
        Some(this.item.clone())
    }
    #[allow(clippy::too_many_arguments)]
    fn init_dock_window_with_host(
        this: &Self,
        window: Option<Rc<dyn IDockWindow>>,
        owner: Option<Rc<dyn IDockable>>,
        host: Option<Rc<dyn IHostWindow>>,
    ) {
        this.calls.borrow_mut().push((
            "init_dock_window_with_host",
            vec![Rc::new(window), Rc::new(owner), Rc::new(host)],
        ));
    }
    #[allow(clippy::too_many_arguments)]
    fn init_active_dockable(
        this: &Self,
        dockable: Option<Rc<dyn IDockable>>,
        owner: Option<Rc<dyn IDock>>,
    ) {
        this.calls.borrow_mut().push((
            "init_active_dockable",
            vec![Rc::new(dockable), Rc::new(owner)],
        ));
    }
    #[allow(clippy::too_many_arguments)]
    fn insert_dockable(
        this: &Self,
        dock: Option<Rc<dyn IDock>>,
        dockable: Option<Rc<dyn IDockable>>,
        index: i32,
    ) {
        this.calls.borrow_mut().push((
            "insert_dockable",
            vec![Rc::new(dock), Rc::new(dockable), Rc::new(index)],
        ));
    }
    #[allow(clippy::too_many_arguments)]
    fn remove_dockable(this: &Self, dockable: Option<Rc<dyn IDockable>>, collapse: bool) {
        this.calls.borrow_mut().push((
            "remove_dockable",
            vec![Rc::new(dockable), Rc::new(collapse)],
        ));
    }
    #[allow(clippy::too_many_arguments)]
    fn move_dockable(
        this: &Self,
        dock: Option<Rc<dyn IDock>>,
        source_dockable: Option<Rc<dyn IDockable>>,
        target_dockable: Option<Rc<dyn IDockable>>,
    ) {
        this.calls.borrow_mut().push((
            "move_dockable",
            vec![
                Rc::new(dock),
                Rc::new(source_dockable),
                Rc::new(target_dockable),
            ],
        ));
    }
    #[allow(clippy::too_many_arguments)]
    fn move_dockable_between_docks(
        this: &Self,
        source_dock: Option<Rc<dyn IDock>>,
        target_dock: Option<Rc<dyn IDock>>,
        source_dockable: Option<Rc<dyn IDockable>>,
        target_dockable: Option<Rc<dyn IDockable>>,
    ) {
        this.calls.borrow_mut().push((
            "move_dockable_between_docks",
            vec![
                Rc::new(source_dock),
                Rc::new(target_dock),
                Rc::new(source_dockable),
                Rc::new(target_dockable),
            ],
        ));
    }
    #[allow(clippy::too_many_arguments)]
    fn swap_dockable(
        this: &Self,
        dock: Option<Rc<dyn IDock>>,
        source_dockable: Option<Rc<dyn IDockable>>,
        target_dockable: Option<Rc<dyn IDockable>>,
    ) {
        this.calls.borrow_mut().push((
            "swap_dockable",
            vec![
                Rc::new(dock),
                Rc::new(source_dockable),
                Rc::new(target_dockable),
            ],
        ));
    }
    #[allow(clippy::too_many_arguments)]
    fn swap_dockable_between_docks(
        this: &Self,
        source_dock: Option<Rc<dyn IDock>>,
        target_dock: Option<Rc<dyn IDock>>,
        source_dockable: Option<Rc<dyn IDockable>>,
        target_dockable: Option<Rc<dyn IDockable>>,
    ) {
        this.calls.borrow_mut().push((
            "swap_dockable_between_docks",
            vec![
                Rc::new(source_dock),
                Rc::new(target_dock),
                Rc::new(source_dockable),
                Rc::new(target_dockable),
            ],
        ));
    }
    #[allow(clippy::too_many_arguments)]
    fn move_pinned_dockable(
        this: &Self,
        source_dockable: Option<Rc<dyn IDockable>>,
        target_dockable: Option<Rc<dyn IDockable>>,
    ) {
        this.calls.borrow_mut().push((
            "move_pinned_dockable",
            vec![Rc::new(source_dockable), Rc::new(target_dockable)],
        ));
    }
    #[allow(clippy::too_many_arguments)]
    fn add_window(
        this: &Self,
        root_dock: Option<Rc<dyn IRootDock>>,
        window: Option<Rc<dyn IDockWindow>>,
    ) {
        this.calls
            .borrow_mut()
            .push(("add_window", vec![Rc::new(root_dock), Rc::new(window)]));
    }
    #[allow(clippy::too_many_arguments)]
    fn insert_window(
        this: &Self,
        root_dock: Option<Rc<dyn IRootDock>>,
        window: Option<Rc<dyn IDockWindow>>,
        index: i32,
    ) {
        this.calls.borrow_mut().push((
            "insert_window",
            vec![Rc::new(root_dock), Rc::new(window), Rc::new(index)],
        ));
    }
    #[allow(clippy::too_many_arguments)]
    fn remove_window(this: &Self, window: Option<Rc<dyn IDockWindow>>) {
        this.calls
            .borrow_mut()
            .push(("remove_window", vec![Rc::new(window)]));
    }
    #[allow(clippy::too_many_arguments)]
    fn collapse_dock(this: &Self, dock: Rc<dyn IDock>) {
        this.calls
            .borrow_mut()
            .push(("collapse_dock", vec![Rc::new(dock)]));
    }
    #[allow(clippy::too_many_arguments)]
    fn create_split_layout(
        this: &Self,
        dock: Rc<dyn IDock>,
        dockable: Rc<dyn IDockable>,
        operation: DockOperation,
    ) -> Rc<dyn IDock> {
        this.calls.borrow_mut().push((
            "create_split_layout",
            vec![Rc::new(dock), Rc::new(dockable), Rc::new(operation)],
        ));
        this.dock.clone()
    }
    #[allow(clippy::too_many_arguments)]
    fn split_to_dock(
        this: &Self,
        dock: Rc<dyn IDock>,
        dockable: Rc<dyn IDockable>,
        operation: DockOperation,
    ) {
        this.calls.borrow_mut().push((
            "split_to_dock",
            vec![Rc::new(dock), Rc::new(dockable), Rc::new(operation)],
        ));
    }
    #[allow(clippy::too_many_arguments)]
    fn create_window_from(
        this: &Self,
        dockable: Option<Rc<dyn IDockable>>,
    ) -> Option<Rc<dyn IDockWindow>> {
        this.calls
            .borrow_mut()
            .push(("create_window_from", vec![Rc::new(dockable)]));
        Some(this.window.clone())
    }
    #[allow(clippy::too_many_arguments)]
    fn create_window_from_with_options(
        this: &Self,
        dockable: Option<Rc<dyn IDockable>>,
        options: Option<Rc<DockWindowOptions>>,
    ) -> Option<Rc<dyn IDockWindow>> {
        this.calls.borrow_mut().push((
            "create_window_from_with_options",
            vec![Rc::new(dockable), Rc::new(options)],
        ));
        Some(this.window.clone())
    }
    #[allow(clippy::too_many_arguments)]
    fn split_to_window(
        this: &Self,
        dock: Option<Rc<dyn IDock>>,
        dockable: Option<Rc<dyn IDockable>>,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    ) {
        this.calls.borrow_mut().push((
            "split_to_window",
            vec![
                Rc::new(dock),
                Rc::new(dockable),
                Rc::new(x),
                Rc::new(y),
                Rc::new(width),
                Rc::new(height),
            ],
        ));
    }
    #[allow(clippy::too_many_arguments)]
    fn split_to_window_with_options(
        this: &Self,
        dock: Option<Rc<dyn IDock>>,
        dockable: Option<Rc<dyn IDockable>>,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        options: Option<Rc<DockWindowOptions>>,
    ) {
        this.calls.borrow_mut().push((
            "split_to_window_with_options",
            vec![
                Rc::new(dock),
                Rc::new(dockable),
                Rc::new(x),
                Rc::new(y),
                Rc::new(width),
                Rc::new(height),
                Rc::new(options),
            ],
        ));
    }
}
impl_factory!(FactoryProbe);
fn assert_argument(actual: &ferrodock_model::BoxedValue, expected: &ferrodock_model::BoxedValue) {
    macro_rules! compare {
        ($ty:ty) => {
            if let Some(value) = expected.downcast_ref::<$ty>() {
                assert!(actual
                    .downcast_ref::<$ty>()
                    .is_some_and(|actual| actual == value));
                return;
            }
        };
    }
    compare!(Rc<dyn IDockable>);
    compare!(Rc<dyn IDock>);
    compare!(Rc<dyn IRootDock>);
    compare!(Rc<dyn IDockWindow>);
    compare!(Option<Rc<dyn IDockable>>);
    compare!(Option<Rc<dyn IDock>>);
    compare!(Option<Rc<dyn IRootDock>>);
    compare!(Option<Rc<dyn IDockWindow>>);
    compare!(Option<Rc<dyn IHostWindow>>);
    compare!(i32);
    compare!(bool);
    compare!(f64);
    compare!(DockOperation);
    if let Some(expected) = expected.downcast_ref::<Option<Rc<DockWindowOptions>>>() {
        let actual = actual
            .downcast_ref::<Option<Rc<DockWindowOptions>>>()
            .unwrap();
        assert!(Rc::ptr_eq(
            actual.as_ref().unwrap(),
            expected.as_ref().unwrap()
        ));
        return;
    }
    if let Some(expected) = expected.downcast_ref::<Option<BoxedValue>>() {
        let actual = actual
            .downcast_ref::<Option<ferrodock_model::BoxedValue>>()
            .unwrap();
        let expected: ferrodock_model::BoxedValue = expected.as_ref().unwrap().clone();
        assert!(Rc::ptr_eq(actual.as_ref().unwrap(), &expected));
        return;
    }
    panic!("unhandled source argument");
}
#[test]
fn virtual_factory_overloads_dispatch_the_same_enclosing_override_with_original_arguments_and_results(
) {
    crate::register_types();
    let probe = FactoryProbe::new();
    let creator = Factory::new();
    let root = creator.create_root_dock();
    let other: Rc<dyn IDock> = creator.create_tool_dock();
    let target: Rc<dyn IDockable> = creator.create_tool();
    let options = DockWindowOptions::new();
    let raw: BoxedValue = Rc::new("source item".to_owned());
    for (instance, metadata) in [
        (
            Rc::new(probe.clone() as Rc<dyn IFactory>) as BoxedValue,
            <dyn IFactoryMetadata>::MARKUP,
        ),
        (
            Rc::new(FactoryBaseReference::new(probe.clone())),
            FactoryBaseReference::MARKUP,
        ),
    ] {
        {
            let values: Vec<BoxedValue> = vec![Rc::new({
                let value: Option<ferroui_base::BoxedValue> = Some(raw.clone());
                value
            })];
            let types = [ValueType::of::<Option<ferroui_base::BoxedValue>>()];
            let method = metadata
                .find_methods("GetContainerFromItem")
                .find(|method| {
                    method.parameters.len() == types.len()
                        && method.parameters.iter().zip(types).all(|(a, b)| a() == b)
                })
                .unwrap();
            let mut args = vec![Some(instance.clone())];
            args.extend(values.iter().cloned().map(Some));
            let result = (method.invoke)(&args).unwrap();
            let calls = probe.calls.borrow();
            let (name, arguments) = calls.last().unwrap();
            assert_eq!(*name, "get_container_from_item");
            assert_eq!(arguments.len(), values.len());
            for (actual, expected) in arguments.iter().zip(values) {
                let expected: ferrodock_model::BoxedValue = expected;
                assert_argument(actual, &expected);
            }
            assert!(Rc::ptr_eq(
                &from_markup_value::<Rc<dyn IDockable>>(&result).unwrap(),
                &probe.item
            ));
        }
        {
            let values: Vec<BoxedValue> = vec![
                Rc::new({
                    let value: Rc<dyn IDockWindow> = probe.window.clone();
                    value
                }),
                Rc::new({
                    let value: Option<Rc<dyn IDockable>> = Some(probe.item.clone());
                    value
                }),
                Rc::new({
                    let value: Option<Rc<dyn IHostWindow>> = None;
                    value
                }),
            ];
            let types = [
                ValueType::of::<Option<Rc<dyn IDockWindow>>>(),
                ValueType::of::<Option<Rc<dyn IDockable>>>(),
                ValueType::of::<Option<Rc<dyn IHostWindow>>>(),
            ];
            let method = metadata
                .find_methods("InitDockWindow")
                .find(|method| {
                    method.parameters.len() == types.len()
                        && method.parameters.iter().zip(types).all(|(a, b)| a() == b)
                })
                .unwrap();
            let mut args = vec![Some(instance.clone())];
            args.extend(values.iter().cloned().map(Some));
            let result = (method.invoke)(&args).unwrap();
            let calls = probe.calls.borrow();
            let (name, arguments) = calls.last().unwrap();
            assert_eq!(*name, "init_dock_window_with_host");
            assert_eq!(arguments.len(), values.len());
            for (actual, expected) in arguments.iter().zip(values) {
                let expected: ferrodock_model::BoxedValue =
                    if let Some(window) = expected.downcast_ref::<Rc<dyn IDockWindow>>() {
                        Rc::new(Some(window.clone()))
                    } else if let Some(root) = expected.downcast_ref::<Rc<dyn IRootDock>>() {
                        Rc::new(Some(root.clone()))
                    } else {
                        expected
                    };
                assert_argument(actual, &expected);
            }
            assert!(result.is_none());
        }
        {
            let values: Vec<BoxedValue> = vec![
                Rc::new({
                    let value: Option<Rc<dyn IDockable>> = Some(probe.item.clone());
                    value
                }),
                Rc::new({
                    let value: Rc<dyn IDock> = probe.dock.clone();
                    value
                }),
            ];
            let types = [
                ValueType::of::<Option<Rc<dyn IDockable>>>(),
                ValueType::of::<Option<Rc<dyn IDock>>>(),
            ];
            let method = metadata
                .find_methods("InitActiveDockable")
                .find(|method| {
                    method.parameters.len() == types.len()
                        && method.parameters.iter().zip(types).all(|(a, b)| a() == b)
                })
                .unwrap();
            let mut args = vec![Some(instance.clone())];
            args.extend(values.iter().cloned().map(Some));
            let result = (method.invoke)(&args).unwrap();
            let calls = probe.calls.borrow();
            let (name, arguments) = calls.last().unwrap();
            assert_eq!(*name, "init_active_dockable");
            assert_eq!(arguments.len(), values.len());
            for (actual, expected) in arguments.iter().zip(values) {
                let expected: ferrodock_model::BoxedValue =
                    if let Some(owner) = expected.downcast_ref::<Rc<dyn IDock>>() {
                        Rc::new(Some(owner.clone()))
                    } else {
                        expected
                    };
                assert_argument(actual, &expected);
            }
            assert!(result.is_none());
        }
        {
            let values: Vec<BoxedValue> = vec![
                Rc::new({
                    let value: Option<Rc<dyn IDock>> = Some(probe.dock.clone());
                    value
                }),
                Rc::new({
                    let value: Option<Rc<dyn IDockable>> = Some(probe.item.clone());
                    value
                }),
                Rc::new({
                    let value: i32 = 7;
                    value
                }),
            ];
            let types = [
                ValueType::of::<Option<Rc<dyn IDock>>>(),
                ValueType::of::<Option<Rc<dyn IDockable>>>(),
                ValueType::of::<i32>(),
            ];
            let method = metadata
                .find_methods("InsertDockable")
                .find(|method| {
                    method.parameters.len() == types.len()
                        && method.parameters.iter().zip(types).all(|(a, b)| a() == b)
                })
                .unwrap();
            let mut args = vec![Some(instance.clone())];
            args.extend(values.iter().cloned().map(Some));
            let result = (method.invoke)(&args).unwrap();
            let calls = probe.calls.borrow();
            let (name, arguments) = calls.last().unwrap();
            assert_eq!(*name, "insert_dockable");
            assert_eq!(arguments.len(), values.len());
            for (actual, expected) in arguments.iter().zip(values) {
                let expected: ferrodock_model::BoxedValue = expected;
                assert_argument(actual, &expected);
            }
            assert!(result.is_none());
        }
        {
            let values: Vec<BoxedValue> = vec![
                Rc::new({
                    let value: Option<Rc<dyn IDockable>> = Some(probe.item.clone());
                    value
                }),
                Rc::new({
                    let value: bool = true;
                    value
                }),
            ];
            let types = [
                ValueType::of::<Option<Rc<dyn IDockable>>>(),
                ValueType::of::<bool>(),
            ];
            let method = metadata
                .find_methods("RemoveDockable")
                .find(|method| {
                    method.parameters.len() == types.len()
                        && method.parameters.iter().zip(types).all(|(a, b)| a() == b)
                })
                .unwrap();
            let mut args = vec![Some(instance.clone())];
            args.extend(values.iter().cloned().map(Some));
            let result = (method.invoke)(&args).unwrap();
            let calls = probe.calls.borrow();
            let (name, arguments) = calls.last().unwrap();
            assert_eq!(*name, "remove_dockable");
            assert_eq!(arguments.len(), values.len());
            for (actual, expected) in arguments.iter().zip(values) {
                let expected: ferrodock_model::BoxedValue = expected;
                assert_argument(actual, &expected);
            }
            assert!(result.is_none());
        }
        {
            let values: Vec<BoxedValue> = vec![
                Rc::new({
                    let value: Option<Rc<dyn IDock>> = Some(probe.dock.clone());
                    value
                }),
                Rc::new({
                    let value: Option<Rc<dyn IDockable>> = Some(probe.item.clone());
                    value
                }),
                Rc::new({
                    let value: Option<Rc<dyn IDockable>> = Some(target.clone());
                    value
                }),
            ];
            let types = [
                ValueType::of::<Option<Rc<dyn IDock>>>(),
                ValueType::of::<Option<Rc<dyn IDockable>>>(),
                ValueType::of::<Option<Rc<dyn IDockable>>>(),
            ];
            let method = metadata
                .find_methods("MoveDockable")
                .find(|method| {
                    method.parameters.len() == types.len()
                        && method.parameters.iter().zip(types).all(|(a, b)| a() == b)
                })
                .unwrap();
            let mut args = vec![Some(instance.clone())];
            args.extend(values.iter().cloned().map(Some));
            let result = (method.invoke)(&args).unwrap();
            let calls = probe.calls.borrow();
            let (name, arguments) = calls.last().unwrap();
            assert_eq!(*name, "move_dockable");
            assert_eq!(arguments.len(), values.len());
            for (actual, expected) in arguments.iter().zip(values) {
                let expected: ferrodock_model::BoxedValue = expected;
                assert_argument(actual, &expected);
            }
            assert!(result.is_none());
        }
        {
            let values: Vec<BoxedValue> = vec![
                Rc::new({
                    let value: Option<Rc<dyn IDock>> = Some(probe.dock.clone());
                    value
                }),
                Rc::new({
                    let value: Option<Rc<dyn IDock>> = Some(other.clone());
                    value
                }),
                Rc::new({
                    let value: Option<Rc<dyn IDockable>> = Some(probe.item.clone());
                    value
                }),
                Rc::new({
                    let value: Option<Rc<dyn IDockable>> = Some(target.clone());
                    value
                }),
            ];
            let types = [
                ValueType::of::<Option<Rc<dyn IDock>>>(),
                ValueType::of::<Option<Rc<dyn IDock>>>(),
                ValueType::of::<Option<Rc<dyn IDockable>>>(),
                ValueType::of::<Option<Rc<dyn IDockable>>>(),
            ];
            let method = metadata
                .find_methods("MoveDockable")
                .find(|method| {
                    method.parameters.len() == types.len()
                        && method.parameters.iter().zip(types).all(|(a, b)| a() == b)
                })
                .unwrap();
            let mut args = vec![Some(instance.clone())];
            args.extend(values.iter().cloned().map(Some));
            let result = (method.invoke)(&args).unwrap();
            let calls = probe.calls.borrow();
            let (name, arguments) = calls.last().unwrap();
            assert_eq!(*name, "move_dockable_between_docks");
            assert_eq!(arguments.len(), values.len());
            for (actual, expected) in arguments.iter().zip(values) {
                let expected: ferrodock_model::BoxedValue = expected;
                assert_argument(actual, &expected);
            }
            assert!(result.is_none());
        }
        {
            let values: Vec<BoxedValue> = vec![
                Rc::new({
                    let value: Option<Rc<dyn IDock>> = Some(probe.dock.clone());
                    value
                }),
                Rc::new({
                    let value: Option<Rc<dyn IDockable>> = Some(probe.item.clone());
                    value
                }),
                Rc::new({
                    let value: Option<Rc<dyn IDockable>> = Some(target.clone());
                    value
                }),
            ];
            let types = [
                ValueType::of::<Option<Rc<dyn IDock>>>(),
                ValueType::of::<Option<Rc<dyn IDockable>>>(),
                ValueType::of::<Option<Rc<dyn IDockable>>>(),
            ];
            let method = metadata
                .find_methods("SwapDockable")
                .find(|method| {
                    method.parameters.len() == types.len()
                        && method.parameters.iter().zip(types).all(|(a, b)| a() == b)
                })
                .unwrap();
            let mut args = vec![Some(instance.clone())];
            args.extend(values.iter().cloned().map(Some));
            let result = (method.invoke)(&args).unwrap();
            let calls = probe.calls.borrow();
            let (name, arguments) = calls.last().unwrap();
            assert_eq!(*name, "swap_dockable");
            assert_eq!(arguments.len(), values.len());
            for (actual, expected) in arguments.iter().zip(values) {
                let expected: ferrodock_model::BoxedValue = expected;
                assert_argument(actual, &expected);
            }
            assert!(result.is_none());
        }
        {
            let values: Vec<BoxedValue> = vec![
                Rc::new({
                    let value: Option<Rc<dyn IDock>> = Some(probe.dock.clone());
                    value
                }),
                Rc::new({
                    let value: Option<Rc<dyn IDock>> = Some(other.clone());
                    value
                }),
                Rc::new({
                    let value: Option<Rc<dyn IDockable>> = Some(probe.item.clone());
                    value
                }),
                Rc::new({
                    let value: Option<Rc<dyn IDockable>> = Some(target.clone());
                    value
                }),
            ];
            let types = [
                ValueType::of::<Option<Rc<dyn IDock>>>(),
                ValueType::of::<Option<Rc<dyn IDock>>>(),
                ValueType::of::<Option<Rc<dyn IDockable>>>(),
                ValueType::of::<Option<Rc<dyn IDockable>>>(),
            ];
            let method = metadata
                .find_methods("SwapDockable")
                .find(|method| {
                    method.parameters.len() == types.len()
                        && method.parameters.iter().zip(types).all(|(a, b)| a() == b)
                })
                .unwrap();
            let mut args = vec![Some(instance.clone())];
            args.extend(values.iter().cloned().map(Some));
            let result = (method.invoke)(&args).unwrap();
            let calls = probe.calls.borrow();
            let (name, arguments) = calls.last().unwrap();
            assert_eq!(*name, "swap_dockable_between_docks");
            assert_eq!(arguments.len(), values.len());
            for (actual, expected) in arguments.iter().zip(values) {
                let expected: ferrodock_model::BoxedValue = expected;
                assert_argument(actual, &expected);
            }
            assert!(result.is_none());
        }
        {
            let values: Vec<BoxedValue> = vec![
                Rc::new({
                    let value: Option<Rc<dyn IDockable>> = Some(probe.item.clone());
                    value
                }),
                Rc::new({
                    let value: Option<Rc<dyn IDockable>> = Some(target.clone());
                    value
                }),
            ];
            let types = [
                ValueType::of::<Option<Rc<dyn IDockable>>>(),
                ValueType::of::<Option<Rc<dyn IDockable>>>(),
            ];
            let method = metadata
                .find_methods("MovePinnedDockable")
                .find(|method| {
                    method.parameters.len() == types.len()
                        && method.parameters.iter().zip(types).all(|(a, b)| a() == b)
                })
                .unwrap();
            let mut args = vec![Some(instance.clone())];
            args.extend(values.iter().cloned().map(Some));
            let result = (method.invoke)(&args).unwrap();
            let calls = probe.calls.borrow();
            let (name, arguments) = calls.last().unwrap();
            assert_eq!(*name, "move_pinned_dockable");
            assert_eq!(arguments.len(), values.len());
            for (actual, expected) in arguments.iter().zip(values) {
                let expected: ferrodock_model::BoxedValue = expected;
                assert_argument(actual, &expected);
            }
            assert!(result.is_none());
        }
        {
            let values: Vec<BoxedValue> = vec![
                Rc::new({
                    let value: Rc<dyn IRootDock> = root.clone();
                    value
                }),
                Rc::new({
                    let value: Rc<dyn IDockWindow> = probe.window.clone();
                    value
                }),
            ];
            let types = [
                ValueType::of::<Option<Rc<dyn IRootDock>>>(),
                ValueType::of::<Option<Rc<dyn IDockWindow>>>(),
            ];
            let method = metadata
                .find_methods("AddWindow")
                .find(|method| {
                    method.parameters.len() == types.len()
                        && method.parameters.iter().zip(types).all(|(a, b)| a() == b)
                })
                .unwrap();
            let mut args = vec![Some(instance.clone())];
            args.extend(values.iter().cloned().map(Some));
            let result = (method.invoke)(&args).unwrap();
            let calls = probe.calls.borrow();
            let (name, arguments) = calls.last().unwrap();
            assert_eq!(*name, "add_window");
            assert_eq!(arguments.len(), values.len());
            for (actual, expected) in arguments.iter().zip(values) {
                let expected: ferrodock_model::BoxedValue =
                    if let Some(window) = expected.downcast_ref::<Rc<dyn IDockWindow>>() {
                        Rc::new(Some(window.clone()))
                    } else if let Some(root) = expected.downcast_ref::<Rc<dyn IRootDock>>() {
                        Rc::new(Some(root.clone()))
                    } else {
                        expected
                    };
                assert_argument(actual, &expected);
            }
            assert!(result.is_none());
        }
        {
            let values: Vec<BoxedValue> = vec![
                Rc::new({
                    let value: Rc<dyn IRootDock> = root.clone();
                    value
                }),
                Rc::new({
                    let value: Rc<dyn IDockWindow> = probe.window.clone();
                    value
                }),
                Rc::new({
                    let value: i32 = 7;
                    value
                }),
            ];
            let types = [
                ValueType::of::<Option<Rc<dyn IRootDock>>>(),
                ValueType::of::<Option<Rc<dyn IDockWindow>>>(),
                ValueType::of::<i32>(),
            ];
            let method = metadata
                .find_methods("InsertWindow")
                .find(|method| {
                    method.parameters.len() == types.len()
                        && method.parameters.iter().zip(types).all(|(a, b)| a() == b)
                })
                .unwrap();
            let mut args = vec![Some(instance.clone())];
            args.extend(values.iter().cloned().map(Some));
            let result = (method.invoke)(&args).unwrap();
            let calls = probe.calls.borrow();
            let (name, arguments) = calls.last().unwrap();
            assert_eq!(*name, "insert_window");
            assert_eq!(arguments.len(), values.len());
            for (actual, expected) in arguments.iter().zip(values) {
                let expected: ferrodock_model::BoxedValue =
                    if let Some(window) = expected.downcast_ref::<Rc<dyn IDockWindow>>() {
                        Rc::new(Some(window.clone()))
                    } else if let Some(root) = expected.downcast_ref::<Rc<dyn IRootDock>>() {
                        Rc::new(Some(root.clone()))
                    } else {
                        expected
                    };
                assert_argument(actual, &expected);
            }
            assert!(result.is_none());
        }
        {
            let values: Vec<BoxedValue> = vec![Rc::new({
                let value: Rc<dyn IDockWindow> = probe.window.clone();
                value
            })];
            let types = [ValueType::of::<Option<Rc<dyn IDockWindow>>>()];
            let method = metadata
                .find_methods("RemoveWindow")
                .find(|method| {
                    method.parameters.len() == types.len()
                        && method.parameters.iter().zip(types).all(|(a, b)| a() == b)
                })
                .unwrap();
            let mut args = vec![Some(instance.clone())];
            args.extend(values.iter().cloned().map(Some));
            let result = (method.invoke)(&args).unwrap();
            let calls = probe.calls.borrow();
            let (name, arguments) = calls.last().unwrap();
            assert_eq!(*name, "remove_window");
            assert_eq!(arguments.len(), values.len());
            for (actual, expected) in arguments.iter().zip(values) {
                let expected: ferrodock_model::BoxedValue =
                    if let Some(window) = expected.downcast_ref::<Rc<dyn IDockWindow>>() {
                        Rc::new(Some(window.clone()))
                    } else if let Some(root) = expected.downcast_ref::<Rc<dyn IRootDock>>() {
                        Rc::new(Some(root.clone()))
                    } else {
                        expected
                    };
                assert_argument(actual, &expected);
            }
            assert!(result.is_none());
        }
        {
            let values: Vec<BoxedValue> = vec![Rc::new({
                let value: Rc<dyn IDock> = probe.dock.clone();
                value
            })];
            let types = [ValueType::of::<Rc<dyn IDock>>()];
            let method = metadata
                .find_methods("CollapseDock")
                .find(|method| {
                    method.parameters.len() == types.len()
                        && method.parameters.iter().zip(types).all(|(a, b)| a() == b)
                })
                .unwrap();
            let mut args = vec![Some(instance.clone())];
            args.extend(values.iter().cloned().map(Some));
            let result = (method.invoke)(&args).unwrap();
            let calls = probe.calls.borrow();
            let (name, arguments) = calls.last().unwrap();
            assert_eq!(*name, "collapse_dock");
            assert_eq!(arguments.len(), values.len());
            for (actual, expected) in arguments.iter().zip(values) {
                let expected: ferrodock_model::BoxedValue = expected;
                assert_argument(actual, &expected);
            }
            assert!(result.is_none());
        }
        {
            let values: Vec<BoxedValue> = vec![
                Rc::new({
                    let value: Rc<dyn IDock> = probe.dock.clone();
                    value
                }),
                Rc::new({
                    let value: Rc<dyn IDockable> = probe.item.clone();
                    value
                }),
                Rc::new({
                    let value: DockOperation = DockOperation::Right;
                    value
                }),
            ];
            let types = [
                ValueType::of::<Rc<dyn IDock>>(),
                ValueType::of::<Rc<dyn IDockable>>(),
                ValueType::of::<DockOperation>(),
            ];
            let method = metadata
                .find_methods("CreateSplitLayout")
                .find(|method| {
                    method.parameters.len() == types.len()
                        && method.parameters.iter().zip(types).all(|(a, b)| a() == b)
                })
                .unwrap();
            let mut args = vec![Some(instance.clone())];
            args.extend(values.iter().cloned().map(Some));
            let result = (method.invoke)(&args).unwrap();
            let calls = probe.calls.borrow();
            let (name, arguments) = calls.last().unwrap();
            assert_eq!(*name, "create_split_layout");
            assert_eq!(arguments.len(), values.len());
            for (actual, expected) in arguments.iter().zip(values) {
                let expected: ferrodock_model::BoxedValue = expected;
                assert_argument(actual, &expected);
            }
            assert!(Rc::ptr_eq(
                &from_markup_value::<Rc<dyn IDock>>(&result).unwrap(),
                &probe.dock
            ));
        }
        {
            let values: Vec<BoxedValue> = vec![
                Rc::new({
                    let value: Rc<dyn IDock> = probe.dock.clone();
                    value
                }),
                Rc::new({
                    let value: Rc<dyn IDockable> = probe.item.clone();
                    value
                }),
                Rc::new({
                    let value: DockOperation = DockOperation::Right;
                    value
                }),
            ];
            let types = [
                ValueType::of::<Rc<dyn IDock>>(),
                ValueType::of::<Rc<dyn IDockable>>(),
                ValueType::of::<DockOperation>(),
            ];
            let method = metadata
                .find_methods("SplitToDock")
                .find(|method| {
                    method.parameters.len() == types.len()
                        && method.parameters.iter().zip(types).all(|(a, b)| a() == b)
                })
                .unwrap();
            let mut args = vec![Some(instance.clone())];
            args.extend(values.iter().cloned().map(Some));
            let result = (method.invoke)(&args).unwrap();
            let calls = probe.calls.borrow();
            let (name, arguments) = calls.last().unwrap();
            assert_eq!(*name, "split_to_dock");
            assert_eq!(arguments.len(), values.len());
            for (actual, expected) in arguments.iter().zip(values) {
                let expected: ferrodock_model::BoxedValue = expected;
                assert_argument(actual, &expected);
            }
            assert!(result.is_none());
        }
        {
            let values: Vec<BoxedValue> = vec![Rc::new({
                let value: Option<Rc<dyn IDockable>> = Some(probe.item.clone());
                value
            })];
            let types = [ValueType::of::<Option<Rc<dyn IDockable>>>()];
            let method = metadata
                .find_methods("CreateWindowFrom")
                .find(|method| {
                    method.parameters.len() == types.len()
                        && method.parameters.iter().zip(types).all(|(a, b)| a() == b)
                })
                .unwrap();
            let mut args = vec![Some(instance.clone())];
            args.extend(values.iter().cloned().map(Some));
            let result = (method.invoke)(&args).unwrap();
            let calls = probe.calls.borrow();
            let (name, arguments) = calls.last().unwrap();
            assert_eq!(*name, "create_window_from");
            assert_eq!(arguments.len(), values.len());
            for (actual, expected) in arguments.iter().zip(values) {
                let expected: ferrodock_model::BoxedValue = expected;
                assert_argument(actual, &expected);
            }
            assert!(Rc::ptr_eq(
                &from_markup_value::<Rc<dyn IDockWindow>>(&result).unwrap(),
                &probe.window
            ));
        }
        {
            let values: Vec<BoxedValue> = vec![
                Rc::new({
                    let value: Option<Rc<dyn IDockable>> = Some(probe.item.clone());
                    value
                }),
                Rc::new({
                    let value: Option<Rc<DockWindowOptions>> = Some(options.clone());
                    value
                }),
            ];
            let types = [
                ValueType::of::<Option<Rc<dyn IDockable>>>(),
                ValueType::of::<Option<Rc<DockWindowOptions>>>(),
            ];
            let method = metadata
                .find_methods("CreateWindowFrom")
                .find(|method| {
                    method.parameters.len() == types.len()
                        && method.parameters.iter().zip(types).all(|(a, b)| a() == b)
                })
                .unwrap();
            let mut args = vec![Some(instance.clone())];
            args.extend(values.iter().cloned().map(Some));
            let result = (method.invoke)(&args).unwrap();
            let calls = probe.calls.borrow();
            let (name, arguments) = calls.last().unwrap();
            assert_eq!(*name, "create_window_from_with_options");
            assert_eq!(arguments.len(), values.len());
            for (actual, expected) in arguments.iter().zip(values) {
                let expected: ferrodock_model::BoxedValue = expected;
                assert_argument(actual, &expected);
            }
            assert!(Rc::ptr_eq(
                &from_markup_value::<Rc<dyn IDockWindow>>(&result).unwrap(),
                &probe.window
            ));
        }
        {
            let values: Vec<BoxedValue> = vec![
                Rc::new({
                    let value: Option<Rc<dyn IDock>> = Some(probe.dock.clone());
                    value
                }),
                Rc::new({
                    let value: Option<Rc<dyn IDockable>> = Some(probe.item.clone());
                    value
                }),
                Rc::new({
                    let value: f64 = 1.25;
                    value
                }),
                Rc::new({
                    let value: f64 = 2.5;
                    value
                }),
                Rc::new({
                    let value: f64 = 320.0;
                    value
                }),
                Rc::new({
                    let value: f64 = 240.0;
                    value
                }),
            ];
            let types = [
                ValueType::of::<Option<Rc<dyn IDock>>>(),
                ValueType::of::<Option<Rc<dyn IDockable>>>(),
                ValueType::of::<f64>(),
                ValueType::of::<f64>(),
                ValueType::of::<f64>(),
                ValueType::of::<f64>(),
            ];
            let method = metadata
                .find_methods("SplitToWindow")
                .find(|method| {
                    method.parameters.len() == types.len()
                        && method.parameters.iter().zip(types).all(|(a, b)| a() == b)
                })
                .unwrap();
            let mut args = vec![Some(instance.clone())];
            args.extend(values.iter().cloned().map(Some));
            let result = (method.invoke)(&args).unwrap();
            let calls = probe.calls.borrow();
            let (name, arguments) = calls.last().unwrap();
            assert_eq!(*name, "split_to_window");
            assert_eq!(arguments.len(), values.len());
            for (actual, expected) in arguments.iter().zip(values) {
                let expected: ferrodock_model::BoxedValue = expected;
                assert_argument(actual, &expected);
            }
            assert!(result.is_none());
        }
        {
            let values: Vec<BoxedValue> = vec![
                Rc::new({
                    let value: Option<Rc<dyn IDock>> = Some(probe.dock.clone());
                    value
                }),
                Rc::new({
                    let value: Option<Rc<dyn IDockable>> = Some(probe.item.clone());
                    value
                }),
                Rc::new({
                    let value: f64 = 1.25;
                    value
                }),
                Rc::new({
                    let value: f64 = 2.5;
                    value
                }),
                Rc::new({
                    let value: f64 = 320.0;
                    value
                }),
                Rc::new({
                    let value: f64 = 240.0;
                    value
                }),
                Rc::new({
                    let value: Option<Rc<DockWindowOptions>> = Some(options.clone());
                    value
                }),
            ];
            let types = [
                ValueType::of::<Option<Rc<dyn IDock>>>(),
                ValueType::of::<Option<Rc<dyn IDockable>>>(),
                ValueType::of::<f64>(),
                ValueType::of::<f64>(),
                ValueType::of::<f64>(),
                ValueType::of::<f64>(),
                ValueType::of::<Option<Rc<DockWindowOptions>>>(),
            ];
            let method = metadata
                .find_methods("SplitToWindow")
                .find(|method| {
                    method.parameters.len() == types.len()
                        && method.parameters.iter().zip(types).all(|(a, b)| a() == b)
                })
                .unwrap();
            let mut args = vec![Some(instance.clone())];
            args.extend(values.iter().cloned().map(Some));
            let result = (method.invoke)(&args).unwrap();
            let calls = probe.calls.borrow();
            let (name, arguments) = calls.last().unwrap();
            assert_eq!(*name, "split_to_window_with_options");
            assert_eq!(arguments.len(), values.len());
            for (actual, expected) in arguments.iter().zip(values) {
                let expected: ferrodock_model::BoxedValue = expected;
                assert_argument(actual, &expected);
            }
            assert!(result.is_none());
        }
    }
}

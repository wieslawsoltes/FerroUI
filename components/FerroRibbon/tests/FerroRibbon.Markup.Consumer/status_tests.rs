use crate::status_consumer::{ConsumerStatus, StatusView};
use ferroribbon::controls::{IRibbonItem, IRibbonItemOwner, RibbonToggleButton, RibbonZoomControl};
use ferroui_base::{
    data::core::{ValueType, ValueTypes},
    interactivity::RoutedEventArgs,
    metadata::{MarkupType, MarkupTyped},
    threading::Dispatcher,
};
use ferroui_controls::{Button, Control, Window};
use std::rc::Rc;
#[test]
fn independent_status_compiled_runtime_collections_owner_forwarding_and_subclass() {
    super::tests::run(|| {
        let _theme = super::tests::NativeThemeScope::new();
        ferroui_base::TypeInfo::register_namespaces(&[
            (
                "ferroribbon_markup_consumer::status_consumer",
                "FerroRibbon.Consumer",
            ),
            (
                "ferroribbon_markup_consumer::spinner_consumer",
                "FerroRibbon.Consumer",
            ),
        ]);
        ferroui_base::TypeInfo::register_all(&[ConsumerStatus::TYPE, StatusView::TYPE]);
        MarkupType::register_all(&[crate::spinner_consumer::NumberViewModel::MARKUP]);
        ValueTypes::register_reference::<crate::spinner_consumer::NumberViewModel>();
        for runtime in [false, true] {
            let value = if runtime {
                ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                    include_str!("Status.xaml"),
                    Some(&crate::CONSUMER_ASSEMBLY),
                )
                .unwrap()
            } else {
                crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferroribbon_markup_consumer/Status.xaml",
                )
                .unwrap()
                .unwrap()
            };
            let view = ValueTypes::as_object(&*value)
                .unwrap()
                .cast::<StatusView>()
                .unwrap();
            let bar = view.children().get(1).cast::<ConsumerStatus>().unwrap();
            let vm = ValueTypes::try_convert(
                view.data_context().as_ref(),
                ValueType::of::<Rc<crate::spinner_consumer::NumberViewModel>>(),
            )
            .flatten()
            .unwrap();
            let vm = vm
                .downcast_ref::<Rc<crate::spinner_consumer::NumberViewModel>>()
                .unwrap()
                .clone();
            let w = Window::new();
            w.set_content(Some(Control::boxed(view.clone())));
            w.show();
            w.update_layout();
            Dispatcher::ui_thread().run_jobs(None);
            assert!(bar.templates() > 0);
            assert_eq!(bar.items().count(), 1);
            assert_eq!(bar.end_items().count(), 1);
            let grid = bar
                .items()
                .get(0)
                .unwrap()
                .cast::<RibbonToggleButton>()
                .unwrap();
            assert!(grid.actual_show_label());
            bar.set_show_labels(false);
            assert!(!grid.actual_show_label());
            let boxed = Control::boxed(bar.clone());
            let owner =
                ValueTypes::try_convert(Some(&boxed), ValueType::of::<Rc<dyn IRibbonItemOwner>>())
                    .flatten()
                    .unwrap();
            let owner = owner.downcast_ref::<Rc<dyn IRibbonItemOwner>>().unwrap();
            assert_eq!(owner.metrics().unwrap().simplified_item_height(), 22.);
            assert!(owner.owner_ribbon().is_some());
            let grid_boxed = Control::boxed(grid.clone());
            let native_calls = Rc::new(std::cell::Cell::new(0));
            let count = native_calls.clone();
            bar.item_invoked(move |_, _| count.set(count.get() + 1));
            let item =
                ValueTypes::try_convert(Some(&grid_boxed), ValueType::of::<Rc<dyn IRibbonItem>>())
                    .flatten()
                    .unwrap();
            assert!(item.downcast_ref::<Rc<dyn IRibbonItem>>().unwrap().invoke());
            // The unchanged checked-control corpus includes the full Uno peer
            // bridge: programmatic Invoke clicks/notifies, then notifies again.
            let checked: serde_json::Value =
                serde_json::from_str(include_str!("../fixtures/checked-controls.json")).unwrap();
            let source_case = checked["cases"]
                .as_array()
                .unwrap()
                .iter()
                .find(|case| {
                    case["kind"] == "toggle"
                        && case["action"] == "invoke"
                        && case["copied"] == false
                        && case["state"] == false
                        && case["three"] == false
                        && case["grouped"] == false
                })
                .unwrap();
            let notifications = source_case["trace"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|v| v.as_str().unwrap().starts_with("notify:source:"))
                .count();
            assert_eq!(native_calls.get(), notifications);
            assert_eq!(view.calls(), vec![Some("grid".into()); notifications]);
            let zoom = bar
                .end_items()
                .get(0)
                .unwrap()
                .cast::<RibbonZoomControl>()
                .unwrap();
            let plus = zoom
                .get_visual_descendants()
                .find_map(|c| {
                    c.cast::<Button>()
                        .filter(|b| b.name().as_deref() == Some("PART_ZoomIn"))
                })
                .unwrap();
            plus.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
            assert_eq!(vm.calls(), [10.]);
            let mut expected = vec![Some("grid".into()); notifications];
            expected.push(Some("zoom".into()));
            assert_eq!(view.calls(), expected);
            w.close();
        }
    });
}

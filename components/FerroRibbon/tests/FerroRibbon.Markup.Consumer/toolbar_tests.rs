use crate::toolbar_consumer::ConsumerToolBarPanel;
use ferroribbon::{compat::Orientation, primitives::RibbonToolBarPanel};
use ferroui_base::{Rect, Size, TypeInfo, controls::NameScope, data::core::ValueTypes};
use ferroui_controls::{Button, Control, TextBlock, Window};
#[test]
fn toolbar_panel_compiled_runtime_subclass_orientation_collections_and_bound_count() {
    crate::tests::run_standalone(|| {
        TypeInfo::register_namespaces(&[(
            "ferroribbon_markup_consumer::toolbar_consumer",
            "FerroRibbon.Consumer",
        )]);
        TypeInfo::register_all(&[ConsumerToolBarPanel::TYPE]);
        for runtime in [false, true] {
            let value = if runtime {
                ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                    include_str!("ToolBarPanel.xaml"),
                    Some(&crate::CONSUMER_ASSEMBLY),
                )
                .unwrap()
            } else {
                crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferroribbon_markup_consumer/ToolBarPanel.xaml",
                )
                .unwrap()
                .unwrap()
            };
            let root = ValueTypes::as_object(&*value)
                .unwrap()
                .cast::<Control>()
                .unwrap();
            let scope = NameScope::get_name_scope(&root).unwrap();
            let window = Window::new();
            window.set_content(Some(Control::boxed(root.clone())));
            window.show();
            window.update_layout();
            let p = scope
                .find("Toolbar")
                .unwrap()
                .cast::<ConsumerToolBarPanel>()
                .unwrap();
            assert_eq!(p.orientation(), Orientation::Vertical);
            assert_eq!(p.orientation().raw(), 0);
            assert_eq!(p.columns(), 2);
            assert_eq!(p.spacing(), 1.25);
            let more = scope.find("More").unwrap().cast::<Button>().unwrap();
            assert!(p.overflow_button().unwrap().ptr_eq(&more));
            assert_eq!(p.children().count(), 5);
            let unknown = scope
                .find("UnknownOrientation")
                .unwrap()
                .cast::<RibbonToolBarPanel>()
                .unwrap();
            assert_eq!(unknown.orientation().raw(), 99);
            assert_eq!(unknown.columns(), -2);
            for (name, expected) in [
                ("MinimumOrientation", i32::MIN),
                ("MaximumOrientation", i32::MAX),
            ] {
                assert_eq!(
                    scope
                        .find(name)
                        .unwrap()
                        .cast::<RibbonToolBarPanel>()
                        .unwrap()
                        .orientation()
                        .raw(),
                    expected
                );
            }
            assert_eq!(
                scope
                    .find("NumericHorizontal")
                    .unwrap()
                    .cast::<RibbonToolBarPanel>()
                    .unwrap()
                    .orientation(),
                Orientation::Horizontal
            );
            let held = p.overflow_items();
            p.measure(Size::new(140., 30.));
            p.arrange(Rect::new(0., 0., 140., 30.));
            assert!(p.measure_calls() > 0);
            assert!(held.count() > 0);
            assert!(held == p.overflow_items());
            let count = scope
                .find("InitialCount")
                .unwrap()
                .cast::<TextBlock>()
                .unwrap();
            assert_eq!(count.text().as_deref(), Some("0"));
            let first = scope.find("First").unwrap();
            assert!(p.children().to_vec().iter().any(|v| v.ptr_eq(&first)));
            p.set_is_overflow_enabled(false);
            p.invalidate_measure();
            p.measure(Size::new(140., 500.));
            p.arrange(Rect::new(0., 0., 140., 500.));
            assert_eq!(held.count(), 0);
            let count = scope
                .find("InitialCount")
                .unwrap()
                .cast::<TextBlock>()
                .unwrap();
            // Source List<T> does not notify Count; the compiled binding initially reads
            // zero, and must not gain an invented notification from this adapter.
            assert_eq!(count.text().as_deref(), Some("0"));
        }
    });
}

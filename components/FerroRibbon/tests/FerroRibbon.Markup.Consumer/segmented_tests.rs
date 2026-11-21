use ferroribbon::{
    controls::*,
    metadata::models::{RibbonSegmentModelMetadata, RibbonSegmentedModelMetadata},
};
use ferroribbon_core::{RibbonString, commands::RibbonRelayCommand, model::*};
use ferroui_base::{
    BoxedValue, Rect, Size, TypeInfo,
    data::core::{ValueType, ValueTypes},
    metadata::MarkupTyped,
    threading::Dispatcher,
};
use ferroui_controls::{Control, StackPanel, TextBlock, Window};
use serde_json::{Value, json};
use std::{cell::RefCell, rc::Rc};
fn cast<T: Clone + 'static>(v: &BoxedValue) -> T {
    ValueTypes::try_cast(v, ValueType::of::<T>())
        .unwrap()
        .downcast_ref::<T>()
        .unwrap()
        .clone()
}
#[test]
fn original_segment_optional_constructor_arities_and_payloads_match_native_metadata() {
    crate::tests::run(|| {
        let source: Value =
            serde_json::from_str(include_str!("../fixtures/model-metadata.json")).unwrap();
        let args: Vec<Option<BoxedValue>> = vec![
            Some(Rc::new("id".to_owned())),
            Some(Rc::new("Label".to_owned())),
            None,
            Some(Rc::new(23i32)),
        ];
        let mut actual = vec![];
        for meta in [
            RibbonSegmentModelMetadata::MARKUP,
            RibbonSegmentedModelMetadata::MARKUP,
        ] {
            for ctor in meta.constructors {
                let arity = ctor.parameters.len();
                let value = (ctor.invoke)(&args[..arity]).unwrap().unwrap();
                let (node, payload, can_add): (Rc<dyn RibbonNodeModelImpl>, Value, Value) =
                    if meta.name == "RibbonSegmentModel" {
                        let m = cast::<Rc<dyn RibbonSegmentModelImpl>>(&value);
                        let payload = m.value().map_or(Value::Null, |v| {
                            if let Some(s) = v.downcast_ref::<RibbonString>() {
                                json!(s.to_utf8().unwrap())
                            } else {
                                json!(*v.downcast_ref::<i32>().unwrap())
                            }
                        });
                        (m, payload, Value::Null)
                    } else {
                        let m = cast::<Rc<dyn RibbonSegmentedModelImpl>>(&value);
                        (m.clone(), Value::Null, json!(m.can_add_to_quick_access()))
                    };
                let id = node.id().unwrap().to_utf8().unwrap();
                if arity == 0 {
                    assert!(id.len() == 32 && id.bytes().all(|b| b.is_ascii_hexdigit()));
                }
                actual.push(json!({"type":meta.name,"arity":arity,"id":if arity==0{"<generated-guid>"}else{&id},"label":node.label().map(|s|s.to_utf8().unwrap()),"value":payload,"canAdd":can_add}));
            }
        }
        assert_eq!(json!(actual), source["corpus"]["segmentConstructors"]);
    });
}
#[test]
fn compiled_runtime_segment_content_events_binding_subclass_template_and_model_graph() {
    crate::tests::run(|| {
        let _theme = crate::tests::NativeThemeScope::new();
        TypeInfo::register_namespaces(&[(
            "ferroribbon_markup_consumer::segmented_consumer",
            "FerroRibbon.Consumer",
        )]);
        TypeInfo::register_all(&[
            crate::segmented_consumer::ConsumerSegmented::TYPE,
            crate::segmented_consumer::SegmentedView::TYPE,
        ]);
        for runtime in [false, true] {
            let value = if runtime {
                ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                    include_str!("Segmented.xaml"),
                    Some(&crate::CONSUMER_ASSEMBLY),
                )
                .unwrap()
            } else {
                crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferroribbon_markup_consumer/Segmented.xaml",
                )
                .unwrap()
                .unwrap()
            };
            let host = ValueTypes::as_object(&*value)
                .unwrap()
                .cast::<crate::segmented_consumer::SegmentedView>()
                .unwrap();
            let panel = host.clone().upcast::<StackPanel>();
            let model = cast::<Rc<dyn RibbonSegmentedModelImpl>>(
                &host
                    .resources()
                    .try_get_resource(&"Model".into(), None)
                    .flatten()
                    .unwrap(),
            );
            let ribbon = panel.children().get(0).cast::<Ribbon>().unwrap();
            let direct = panel
                .children()
                .get(1)
                .cast::<crate::segmented_consumer::ConsumerSegmented>()
                .unwrap();
            let custom = panel
                .children()
                .get(3)
                .cast::<RibbonSegmentedControl>()
                .unwrap();
            let window = Window::new();
            window.set_content(Some(Control::boxed(host.clone())));
            window.show();
            window.measure(Size::new(950., 500.));
            window.arrange(Rect::new(0., 0., 950., 500.));
            Dispatcher::ui_thread().run_jobs(None);
            assert_eq!(direct.segments().count(), 2);
            assert_eq!(
                direct.segments().get(1).unwrap().label().as_deref(),
                Some("Review")
            );
            assert_eq!(direct.style_key(), RibbonSegmentedControl::TYPE);
            assert_eq!(
                direct
                    .get_visual_descendants()
                    .filter(|c| c.cast::<RibbonToggleButton>().is_some())
                    .count(),
                2
            );
            assert_eq!(
                custom
                    .get_visual_descendants()
                    .filter(|c| c.cast::<RibbonToggleButton>().is_some())
                    .count(),
                1
            );
            let owner = ValueTypes::try_cast(
                &Control::boxed(direct.clone()),
                ValueType::of::<Rc<dyn IRibbonItem>>(),
            )
            .unwrap();
            assert!(
                owner
                    .downcast_ref::<Rc<dyn IRibbonItem>>()
                    .unwrap()
                    .invoke()
            );
            assert_eq!(direct.invokes(), 1);
            assert_eq!(direct.selected_index(), 1);
            assert_eq!(host.calls(), vec![Some("Review".into())]);
            assert_eq!(
                panel
                    .children()
                    .get(2)
                    .cast::<TextBlock>()
                    .unwrap()
                    .text()
                    .as_deref(),
                Some("1")
            );
            direct.set_selected_index(-1);
            assert_eq!(host.calls(), vec![Some("Review".into()), None]);
            let control = ribbon
                .tabs()
                .get(0)
                .unwrap()
                .groups()
                .get(0)
                .unwrap()
                .items()
                .get(0)
                .unwrap()
                .cast::<RibbonSegmentedControl>()
                .unwrap();
            assert_eq!(control.selected_index(), 1);
            let a = model.segments().get(0).unwrap();
            let b = model.segments().get(1).unwrap();
            let old = control.segments().get(1).unwrap();
            b.set_label(Some("Inspect".into()));
            assert_eq!(
                control.segments().get(1).unwrap().label().as_deref(),
                Some("Inspect")
            );
            assert!(!old.ptr_eq(&control.segments().get(1).unwrap()));
            control.set_selected_index(0);
            assert!(Rc::ptr_eq(&model.selected_segment().unwrap(), &a));
            model.set_selected_segment(Some(b.clone()));
            assert_eq!(control.selected_index(), 1);
            let calls = Rc::new(RefCell::new(vec![]));
            let out = calls.clone();
            model.set_command(Some(RibbonRelayCommand::new_with_parameter(
                move |p| {
                    let v = p.unwrap();
                    out.borrow_mut()
                        .push(v.downcast_ref::<RibbonString>().unwrap().to_utf8().unwrap());
                },
                None,
            )));
            let items = control.create_overflow_menu_items_core();
            let menu = items
                .iter()
                .next()
                .unwrap()
                .unwrap()
                .cast::<ferroribbon::compat::RadioMenuFlyoutItem>()
                .unwrap();
            menu.invoke_menu_core();
            assert_eq!(*calls.borrow(), vec!["draw"]);
            assert_eq!(control.selected_index(), 0);
            assert!(Rc::ptr_eq(&model.selected_segment().unwrap(), &a));
            ribbon.set_model(None);
            a.set_label(Some("detached".into()));
            assert_eq!(
                control.segments().get(0).unwrap().label().as_deref(),
                Some("Draw")
            );
            control.set_selected_index(1);
            assert!(Rc::ptr_eq(&model.selected_segment().unwrap(), &a));
            window.close();
        }
    });
}

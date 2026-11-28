use ferroui_base::{
    Rect, Ref, Size, controls::NameScope, data::core::ValueTypes, media::ScaleTransform,
};
use ferroui_controls::{
    Border, ColumnDefinition, ContentControl, Control, Grid, GridLength, GridUnitType, StackPanel,
    Window,
};
use std::{cell::RefCell, rc::Rc};
type NamedTemplateParts = (Ref<ColumnDefinition>, Ref<ScaleTransform>);
fn load(name: &str, source: &str, runtime: bool) -> ferroui_base::BoxedValue {
    if runtime {
        ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
            source,
            Some(&crate::CONSUMER_ASSEMBLY),
        )
        .unwrap()
    } else {
        crate::compiled_xaml::try_load(
            None,
            &format!("ferres://ferroribbon_markup_consumer/{name}.xaml"),
        )
        .unwrap()
        .unwrap()
    }
}
#[test]
fn named_native_objects_have_actual_identity_and_compiled_binding_sources() {
    crate::tests::run_standalone(|| {
        for runtime in [false, true] {
            let value = load("NamedObjects", include_str!("NamedObjects.xaml"), runtime);
            let grid = ValueTypes::as_object(&*value)
                .unwrap()
                .cast::<Grid>()
                .unwrap();
            let scope = NameScope::get_name_scope(&grid).unwrap();
            assert_eq!(grid.name().as_deref(), Some("NativeRoot"));
            assert!(scope.find("NativeRoot").unwrap().ptr_eq(&grid));
            let column = scope
                .find("FirstSpacer")
                .unwrap()
                .cast::<ColumnDefinition>()
                .unwrap();
            assert!(column.ptr_eq(&grid.column_definitions().get(0)));
            let surface = scope.find("Surface").unwrap().cast::<Border>().unwrap();
            let scale = scope
                .find("ScaleTransform")
                .unwrap()
                .cast::<ScaleTransform>()
                .unwrap();
            assert!(
                surface
                    .render_transform()
                    .unwrap()
                    .as_object()
                    .unwrap()
                    .to_ref()
                    .ptr_eq(&scale)
            );
            assert_eq!(scale.scale_x(), 1.25);
            assert_eq!(scale.scale_y(), 2.5);
            let target = grid.children().get(1).cast::<ContentControl>().unwrap();
            assert_eq!(
                *target
                    .content()
                    .unwrap()
                    .downcast_ref::<GridLength>()
                    .unwrap(),
                column.width()
            );
            column.set_width(GridLength::new(17.75, GridUnitType::Pixel));
            assert_eq!(
                *target
                    .content()
                    .unwrap()
                    .downcast_ref::<GridLength>()
                    .unwrap(),
                column.width()
            );
        }
    });
}
#[test]
fn named_template_objects_are_scoped_per_instance_and_replaced_on_retemplate() {
    crate::tests::run_standalone(|| {
        let _theme = crate::tests::NativeThemeScope::new();
        for runtime in [false, true] {
            let value = load(
                "NamedObjectTemplates",
                include_str!("NamedObjectTemplates.xaml"),
                runtime,
            );
            let panel = ValueTypes::as_object(&*value)
                .unwrap()
                .cast::<StackPanel>()
                .unwrap();
            let found: Rc<RefCell<Vec<NamedTemplateParts>>> = Rc::default();
            for child in panel.children().snapshot().iter() {
                let host = child.cast::<ContentControl>().unwrap();
                let found = found.clone();
                host.template_applied(move |_, e| {
                    let scope = e.name_scope();
                    let column = scope
                        .find("FirstSpacer")
                        .unwrap()
                        .cast::<ColumnDefinition>()
                        .unwrap();
                    let transform = scope
                        .find("ScaleTransform")
                        .unwrap()
                        .cast::<ScaleTransform>()
                        .unwrap();
                    let surface = scope.find("Surface").unwrap().cast::<Border>().unwrap();
                    assert!(
                        surface
                            .render_transform()
                            .unwrap()
                            .as_object()
                            .unwrap()
                            .to_ref()
                            .ptr_eq(&transform)
                    );
                    found.borrow_mut().push((column, transform));
                });
            }
            let w = Window::new();
            w.set_content(Some(Control::boxed(panel.clone())));
            w.show();
            w.measure(Size::new(700., 700.));
            w.arrange(Rect::new(0., 0., 700., 700.));
            w.update_layout();
            assert_eq!(found.borrow().len(), 2);
            assert!(!found.borrow()[0].0.ptr_eq(&found.borrow()[1].0));
            assert!(!found.borrow()[0].1.ptr_eq(&found.borrow()[1].1));
            let first = panel.children().get(0).cast::<ContentControl>().unwrap();
            let template = first.template().unwrap();
            first.set_template(None);
            first.apply_template();
            first.set_template(Some(template));
            first.apply_template();
            w.update_layout();
            assert_eq!(found.borrow().len(), 3);
            assert!(!found.borrow()[0].0.ptr_eq(&found.borrow()[2].0));
            assert!(!found.borrow()[0].1.ptr_eq(&found.borrow()[2].1));
            w.close();
            w.set_content(None);
        }
    });
}

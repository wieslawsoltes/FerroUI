//! External dependency consumer: real compiled/runtime model XAML, native factory
//! and the same public palette/grid types as directly declared XAML controls.
use ferroribbon::{
    controls::*,
    mvvm::{RibbonElementFactory, RibbonModelBindings},
};
use ferroribbon_core::{commands::RibbonRelayCommand, model::*, theming::RibbonColor};
use ferroui_base::{
    BoxedValue,
    data::core::{ValueType, ValueTypes},
    media::Color,
    threading::Dispatcher,
};
use ferroui_controls::{Control, Flyout, StackPanel, TextBlock, Window};
use std::{cell::RefCell, rc::Rc};
fn cast<T: Clone + 'static>(value: &BoxedValue) -> T {
    ValueTypes::try_cast(value, ValueType::of::<T>())
        .unwrap()
        .downcast_ref::<T>()
        .unwrap()
        .clone()
}
#[test]
fn independent_compiled_runtime_model_flyouts_share_native_picker_controls_and_commands() {
    crate::tests::run(|| {
        let _theme = crate::tests::NativeThemeScope::new();
        for runtime in [false, true] {
            let value = if runtime {
                ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                    include_str!("FlyoutPickers.xaml"),
                    Some(&crate::CONSUMER_ASSEMBLY),
                )
                .unwrap()
            } else {
                crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferroribbon_markup_consumer/FlyoutPickers.xaml",
                )
                .unwrap()
                .unwrap()
            };
            let panel = ValueTypes::as_object(&*value)
                .unwrap()
                .cast::<StackPanel>()
                .unwrap();
            let resource = |key: &str| {
                panel
                    .resources()
                    .try_get_resource(&key.into(), None)
                    .flatten()
                    .unwrap()
            };
            let drop = cast::<Rc<dyn RibbonDropDownButtonModelImpl>>(&resource("Drop"));
            let colors = cast::<Rc<dyn RibbonColorPickerModelImpl>>(&resource("Colors"));
            let grid_model = cast::<Rc<dyn RibbonGridPickerModelImpl>>(&resource("Grid"));
            let trace = Rc::new(RefCell::new(Vec::new()));
            let t = trace.clone();
            colors.set_command(Some(RibbonRelayCommand::new_with_parameter(
                move |p| {
                    t.borrow_mut().push(format!(
                        "color:{}",
                        p.unwrap()
                            .downcast_ref::<RibbonColor>()
                            .expect("Core color parameter")
                    ));
                },
                None,
            )));
            let t = trace.clone();
            grid_model.set_command(Some(RibbonRelayCommand::new_with_parameter(
                move |p| {
                    t.borrow_mut().push(format!(
                        "grid:{}",
                        p.unwrap()
                            .downcast_ref::<RibbonGridSize>()
                            .expect("Core grid parameter")
                    ));
                },
                None,
            )));
            let f = RibbonElementFactory::new();
            let bindings = RibbonModelBindings::new();
            let generated = f
                .create_item(Some(drop.clone()), bindings.clone())
                .unwrap()
                .cast::<RibbonDropDownButton>()
                .unwrap();
            panel.children().add(generated.clone());
            let w = Window::new();
            w.set_width(700.);
            w.set_height(700.);
            w.set_content(Some(Control::boxed(panel.clone())));
            w.show();
            w.update_layout();
            Dispatcher::ui_thread().run_jobs(None);
            assert_eq!(
                panel
                    .children()
                    .get(0)
                    .cast::<TextBlock>()
                    .unwrap()
                    .text()
                    .as_deref(),
                Some("2")
            );
            let declared = panel
                .children()
                .get(1)
                .cast::<RibbonColorPalette>()
                .unwrap();
            let declared_grid = panel.children().get(2).cast::<RibbonGridPicker>().unwrap();
            let flyout = generated.flyout().unwrap().cast::<Flyout>().unwrap();
            let content = Control::from_boxed(&flyout.content().unwrap())
                .unwrap()
                .cast::<StackPanel>()
                .unwrap();
            let palette = content
                .children()
                .get(0)
                .cast::<RibbonColorPalette>()
                .expect("public palette implementation");
            let grid = content
                .children()
                .get(1)
                .cast::<RibbonGridPicker>()
                .expect("public grid implementation");
            assert_eq!(palette.selected_color(), declared.selected_color());
            assert_eq!(
                (grid.rows(), grid.columns()),
                (declared_grid.rows(), declared_grid.columns())
            );
            flyout.show_at(&generated);
            palette.select(Some(Color::from_argb(128, 4, 5, 6)));
            assert!(!flyout.is_open());
            assert_eq!(
                colors.selected_color(),
                Some(RibbonColor::new(128, 4, 5, 6))
            );
            flyout.show_at(&generated);
            grid.pick(RibbonGridSize::new(2, 3));
            assert!(!flyout.is_open());
            assert_eq!(*trace.borrow(), vec!["color:#80040506", "grid:3x2"]);
            bindings.dispose();
            palette.select(None);
            assert_eq!(
                colors.selected_color(),
                Some(RibbonColor::new(128, 4, 5, 6))
            );
            assert_eq!(trace.borrow().len(), 2);
            w.close();
            w.set_content(None);
        }
    });
}

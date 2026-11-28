//! Real public element factory integration for the entire original Excel model.
use crate::excel_view_model::ExcelViewModel;
use ferroribbon::controls::{IRibbonItem, Ribbon, RibbonComboBox, RibbonToggleButton};
use ferroribbon_core::model::*;
use ferroui_base::{
    Ref,
    data::core::{ValueType, ValueTypes},
    threading::{CancellationToken, Dispatcher},
};
use ferroui_controls::{AppBuilder, Application, Control, Window};
use ferroui_desktop::AppBuilderDesktopExtensions;
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, HeadlessUnitTestSession,
};
use std::rc::Rc;
pub(crate) fn run(f: impl FnOnce() + Send + 'static) {
    let s = HeadlessUnitTestSession::start_new(|| {
        AppBuilder::configure::<Application>()
            .use_platform_detect()
            .use_headless(FerroHeadlessPlatformOptions {
                use_headless_drawing: false,
                ..Default::default()
            })
    });
    let result = s
        .dispatch(
            move || {
                crate::register_types();
                Application::current()
                    .unwrap()
                    .styles()
                    .add(ferroui_themes_simple::SimpleTheme::new().as_style());
                crate::install_application_resources();
                f();
            },
            CancellationToken::none(),
        )
        .wait();
    s.dispose();
    result.unwrap();
}
fn vm() -> Rc<ExcelViewModel> {
    ExcelViewModel::new(Rc::new(|p| {
        if let Some(s) = p.downcast_ref::<String>() {
            Ok(s.clone())
        } else {
            Err("this fixture uses only string catalog parameters".into())
        }
    }))
}
fn mount(r: &Ref<Ribbon>) -> Ref<Window> {
    let w = Window::new();
    w.set_width(2000.);
    w.set_height(600.);
    w.set_content(Some(Control::boxed(r.clone())));
    w.show();
    Dispatcher::ui_thread().run_jobs(None);
    w
}
pub(crate) fn invoke(item: Ref<Control>) {
    let value = ValueTypes::try_cast(
        &Control::boxed(item),
        ValueType::of::<Rc<dyn IRibbonItem>>(),
    )
    .unwrap();
    value
        .downcast_ref::<Rc<dyn IRibbonItem>>()
        .unwrap()
        .invoke();
}
#[test]
fn excel_actual_model_factory_generates_controls_binds_models_and_executes_catalog() {
    run(|| {
        let vm = vm();
        let ribbon = Ribbon::new();
        ribbon.set_model(Some(vm.ribbon()));
        let w = mount(&ribbon);
        assert_eq!(ribbon.tabs().count(), 6);
        assert!(Rc::ptr_eq(
            &ribbon.command_catalog().unwrap(),
            &vm.commands()
        ));
        let bold = ribbon
            .find_item(Some("bold".into()))
            .unwrap()
            .cast::<RibbonToggleButton>()
            .unwrap();
        assert_eq!(bold.is_checked(), Some(false));
        invoke(bold.clone().upcast());
        assert!(vm.bold().is_checked());
        vm.bold().set_is_checked(false);
        assert_eq!(bold.is_checked(), Some(false));
        let family = ribbon
            .find_item(Some("fontFamily".into()))
            .unwrap()
            .cast::<RibbonComboBox>()
            .unwrap();
        assert_eq!(family.text().as_deref(), Some("Aptos Narrow"));
        family.commit_text(Some("Arial".into()));
        assert_eq!(vm.font_family().text(), Some("Arial".into()));
        vm.font_family().set_text(Some("Consolas".into()));
        assert_eq!(family.text().as_deref(), Some("Consolas"));
        invoke(ribbon.find_item(Some("paste".into())).unwrap());
        // Source controls invoke the resolved command directly. Catalog.Executed
        // runs only through Catalog.Execute; ExcelPage later sets the owner ID.
        assert_eq!(vm.status().as_deref(), Some("Paste"));
        vm.toggle_chart_command().execute(None);
        Dispatcher::ui_thread().run_jobs(None);
        assert!(vm.chart_tools().is_visible());
        assert_eq!(
            ribbon.selected_tab().unwrap().id().as_deref(),
            Some("chartDesign")
        );
        w.set_content(None);
        w.close();
    });
}
#[test]
fn excel_full_model_factory_and_presentation_graph_release_without_clearing_models() {
    run(|| {
        let vm = vm();
        let weak_vm = Rc::downgrade(&vm);
        let model = vm.ribbon();
        let weak_model = Rc::downgrade(&model);
        let catalog = vm.commands();
        let weak_catalog = Rc::downgrade(&catalog);
        let ribbon = Ribbon::new();
        let weak_ribbon = ribbon.downgrade();
        ribbon.set_model(Some(model.clone()));
        let w = mount(&ribbon);
        w.set_content(None);
        w.close();
        drop(w);
        drop(ribbon);
        drop(vm);
        drop(model);
        drop(catalog);
        Dispatcher::ui_thread().run_jobs(None);
        assert!(weak_vm.upgrade().is_none());
        assert!(weak_model.upgrade().is_none());
        assert!(weak_catalog.upgrade().is_none());
        assert!(weak_ribbon.upgrade().is_none());
    });
}

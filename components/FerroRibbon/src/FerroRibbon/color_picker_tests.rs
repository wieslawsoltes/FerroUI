use super::*;
use crate::controls::{IRibbonItem, Ribbon, RibbonItemHelper};
use ferroui_base::input::ICommand;
use ferroui_base::{
    media::Color,
    metadata::from_markup_value,
    reactive::{Disposable, IDisposable},
};
use serde_json::{Value, json};
fn item(c: &RibbonColorPicker) -> Rc<dyn IRibbonItem> {
    ferroui_base::data::core::ValueTypes::try_cast(
        &Control::boxed(c.to_ref()),
        ferroui_base::data::core::ValueType::of::<Rc<dyn IRibbonItem>>(),
    )
    .unwrap()
    .downcast_ref::<Rc<dyn IRibbonItem>>()
    .unwrap()
    .clone()
}
fn case(name: &str) -> Value {
    serde_json::from_str::<Value>(include_str!("../../tests/fixtures/color-picker.json")).unwrap()["corpus"].as_array().unwrap().iter().find(|v|v["name"]==name).unwrap().clone()
}
fn hex(c: Option<Color>) -> Option<String> {
    c.map(|c| crate::theming::to_ribbon_color(c).to_hex(true))
}
fn param(p: Option<&BoxedValue>) -> String {
    p.map_or_else(
        || "<null>".into(),
        |p| {
            if Rc::ptr_eq(p, &RibbonColorPicker::no_color_parameter()) {
                "<NoColor>".into()
            } else if let Some(c) = p.downcast_ref::<Color>() {
                hex(Some(*c)).unwrap()
            } else {
                panic!("unexpected payload")
            }
        },
    )
}
fn state(p: &RibbonColorPicker) -> Value {
    json!({"selected":hex(p.selected_color()),"palette":hex(p.palette.selected_color()),"parameter":param(p.command_parameter().as_ref()),"brush":hex(Some(p.color_bar().unwrap().as_solid_color_brush().unwrap().color())),"IsSplit":p.is_split(),"ShowAutomatic":p.show_automatic(),"ShowNoColor":p.show_no_color(),"ShowMoreColors":p.show_more_colors(),"automatic":hex(Some(p.automatic_color())),"command":p.command().is_some(),"themeCount":p.palette.theme_colors().count(),"recent":p.palette.recent_colors().iter().map(|c|hex(Some(c))).collect::<Vec<_>>(),"size":if p.size()==RibbonItemSize::Small{"Small".to_owned()}else{panic!("unexpected size")},"icon":p.icon().unwrap().downcast_ref::<String>().unwrap(),"flyout":"BottomEdgeAlignedLeft"})
}
struct Cmd(&'static str, Rc<RefCell<Vec<String>>>);
impl ICommand for Cmd {
    fn can_execute(&self, p: Option<&BoxedValue>) -> bool {
        self.1
            .borrow_mut()
            .push(format!("{}can:{}", self.0, param(p)));
        true
    }
    fn execute(&self, p: Option<&BoxedValue>) {
        self.1
            .borrow_mut()
            .push(format!("{}execute:{}", self.0, param(p)));
    }
    fn can_execute_changed(&self, _: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        Disposable::empty()
    }
}
fn watch(p: &RibbonColorPicker, id: &'static str) -> (Ref<Ribbon>, Rc<RefCell<Vec<String>>>) {
    let t = Rc::new(RefCell::new(vec![]));
    let r = Ribbon::new();
    RibbonItemHelper::set_owner(p, Some(r.item_owner())).unwrap();
    let trace = t.clone();
    r.add_item_invoked(Some(crate::root_command_tests::invocation(move |a| {
        trace
            .borrow_mut()
            .push(format!("{id}notify:{}", param(a.parameter.as_ref())))
    })));
    p.set_command(Some(Rc::new(Cmd(id, t.clone()))));
    let trace = t.clone();
    p.add_color_selected(Some(MarkupDelegate::new(move |a| {
        let c = from_markup_value::<Option<Color>>(&a[1]).unwrap();
        trace.borrow_mut().push(format!(
            "{id}selected:{}",
            hex(c).unwrap_or("<null>".into())
        ));
        None
    })));
    let trace = t.clone();
    p.add_click(Some(super::super::ribbon_menu::click_delegate(
        move |_, _| trace.borrow_mut().push(format!("{id}click")),
    )));
    (r, t)
}
#[test]
fn original_picker_defaults_null_selection_primary_and_no_color_identity() {
    crate::group_tests::run(|| {
        let p = RibbonColorPicker::new();
        let c = case("defaults_null");
        assert_eq!(state(&p), c["initial"]);
        p.set_selected_color(None);
        assert_eq!(state(&p), c["state"]);
        let observed = Rc::new(RefCell::new(Vec::new()));
        let sink = observed.clone();
        p.add_color_selected(Some(MarkupDelegate::new(move |a| {
            sink.borrow_mut().push(a[1].is_none());
            None
        })));
        p.palette.select(None);
        assert_eq!(
            *observed.borrow(),
            vec![true],
            "source boxed Nullable<Color> null remains actual null"
        );

        assert!(Rc::ptr_eq(
            &RibbonColorPicker::no_color_parameter(),
            &RibbonColorPicker::no_color_parameter()
        ));
        let p = RibbonColorPicker::new();
        let (_r, t) = watch(&p, "s:");
        p.palette.select(Some(Color::from_argb(128, 1, 2, 3)));
        p.palette.select(None);
        p.invoke_primary();
        let c = case("select_none_primary");
        assert_eq!(json!(*t.borrow()), c["trace"]);
        assert_eq!(state(&p), c["state"]);
    });
}
#[test]
fn original_picker_copy_links_snapshots_and_exact_command_counts() {
    crate::group_tests::run(|| {
        let p = RibbonColorPicker::new();
        let (_r, t) = watch(&p, "s:");
        p.set_show_automatic(false);
        p.set_show_no_color(true);
        p.set_show_more_colors(false);
        p.set_automatic_color(Color::from_rgb(9, 8, 7));
        p.palette.theme_colors().add(Color::from_rgb(2, 3, 4));
        p.set_selected_color(Some(Color::from_argb(128, 3, 4, 5)));
        let copy = item(&p)
            .create_linked_copy()
            .unwrap()
            .cast::<RibbonColorPicker>()
            .unwrap();
        let c = case("copy");
        assert_eq!(state(&copy), c["before"]);
        assert_ne!(p.palette, copy.palette);
        assert_ne!(p.private_flyout, copy.private_flyout);
        copy.palette.select(Some(Color::from_argb(128, 7, 8, 9)));
        copy.palette.select(None);
        copy.invoke_primary();
        assert_eq!(json!(*t.borrow()), c["trace"]);
        assert_eq!(state(&p), c["source"]);
        assert_eq!(state(&copy), c["copy"]);
        let p = RibbonColorPicker::new();
        let (_r, t) = watch(&p, "s:");
        let copy = item(&p)
            .create_linked_copy()
            .unwrap()
            .cast::<RibbonColorPicker>()
            .unwrap();
        copy.set_command(Some(Rc::new(Cmd("c:", t.clone()))));
        copy.palette.select(Some(Color::from_rgb(1, 2, 3)));
        let c = case("copy_explicit_command");
        assert_eq!(json!(*t.borrow()), c["trace"]);
    });
}
#[test]
fn more_colors_keeps_source_alive_until_completion_after_visual_removal() {
    crate::group_tests::run(|| {
        use ferroui_base::threading::Dispatcher;
        use ferroui_controls::{Button, Window};
        let p = RibbonColorPicker::new();
        p.set_selected_color(Some(Color::from_argb(128, 1, 2, 3)));
        let weak = p.downgrade();
        let w = Window::new();
        w.set_width(700.);
        w.set_height(700.);
        w.set_content(Some(Control::boxed(p.clone())));
        w.show();
        w.update_layout();
        let selected = Rc::new(RefCell::new(vec![]));
        let t = selected.clone();
        p.add_color_selected(Some(MarkupDelegate::new(move |a| {
            t.borrow_mut()
                .push(from_markup_value::<Option<Color>>(&a[1]).unwrap());
            None
        })));
        p.show_more_colors_dialog();
        w.set_content(None);
        drop(p);
        Dispatcher::ui_thread().run_jobs(None);
        assert!(
            weak.upgrade().is_some(),
            "async source method owns this across await"
        );
        let button = w
            .get_visual_descendants()
            .filter_map(|v| v.cast::<Button>())
            .find(|b| {
                b.content()
                    .as_ref()
                    .and_then(|v| v.downcast_ref::<String>())
                    .is_some_and(|s| s == "OK")
            })
            .unwrap();
        button.on_click();
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(*selected.borrow(), vec![Some(Color::from_rgb(1, 2, 3))]);
        assert!(weak.upgrade().is_none());
        w.close();
    });
}
#[test]
fn color_model_factory_commands_convert_to_core_color_and_null_preserving_alpha() {
    crate::group_tests::run(|| {
        use crate::mvvm::{RibbonElementFactory, RibbonModelBindings};
        use ferroribbon_core::{commands::ICommand as CoreCommand, model::*};
        struct CoreCmd(Rc<RefCell<Vec<String>>>);
        impl CoreCommand for CoreCmd {
            fn can_execute_changed(&self, _: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
                Disposable::empty()
            }
            fn can_execute(&self, _: Option<&dyn std::any::Any>) -> bool {
                true
            }
            fn execute(&self, p: Option<&dyn std::any::Any>) {
                self.0.borrow_mut().push(
                    if let Some(c) =
                        p.and_then(|p| p.downcast_ref::<ferroribbon_core::theming::RibbonColor>())
                    {
                        c.to_hex(true)
                    } else if p.is_none() {
                        "null".into()
                    } else {
                        panic!("wrong source command transport")
                    },
                );
            }
        }
        let trace = Rc::new(RefCell::new(vec![]));
        let m = RibbonColorPickerModel::with_options(
            Some("color".into()),
            Some("Color".into()),
            None,
            Some(Rc::new(CoreCmd(trace.clone()))),
        );
        let factory = RibbonElementFactory::new();
        let bindings = RibbonModelBindings::new();
        let p = factory
            .create_item(Some(m.clone()), bindings.clone())
            .unwrap()
            .cast::<RibbonColorPicker>()
            .unwrap();
        assert_eq!(
            p.command_parameter().unwrap().downcast_ref::<Color>(),
            Some(&Color::from_rgb(255, 0, 0)),
            "source BindCommon preserves the selected-color parameter"
        );
        p.invoke_primary();
        p.palette.select(Some(Color::from_argb(128, 4, 5, 6)));
        p.palette.select(None);
        p.invoke_primary();
        assert_eq!(
            *trace.borrow(),
            vec!["#FFFF0000", "#80040506", "null", "null"]
        );
        assert_eq!(m.selected_color(), None);
        assert_eq!(m.recent_colors().count(), 1);
        bindings.dispose();
        p.set_selected_color(Some(Color::from_rgb(2, 3, 4)));
        assert_eq!(m.selected_color(), None);
    });
}
#[test]
fn localized_dialog_fault_hides_private_flyout_without_selection_or_command() {
    if crate::test_process::isolated_case(concat!(
        module_path!(),
        "::localized_dialog_fault_hides_private_flyout_without_selection_or_command"
    )) {
        return;
    }
    use ferroui_headless::FerroHeadlessPlatformExtensions;
    ferroui_headless::HeadlessUnitTestSession::start_new(|| {
        ferroui_controls::AppBuilder::configure::<Application>()
            .use_headless(ferroui_headless::FerroHeadlessPlatformOptions::default())
    })
    .dispatch(
        || {
            crate::register_types();
            Application::current()
                .unwrap()
                .styles()
                .add(ferroui_themes_simple::SimpleTheme::new().as_style());
            use crate::controls::RibbonColorPalette;
            use ferroribbon_core::{
                localization::{RibbonCulture, RibbonStrings},
                model::RibbonItemsSource,
            };
            use ferroui_base::{
                controls::ResourceDictionary, data::core::ValueTypes, styling::ControlTheme,
                threading::Dispatcher,
            };
            use ferroui_controls::{Button, Window};
            let previous = RibbonStrings::current();
            RibbonStrings::register(
                Some("en-US".into()),
                Some(RibbonItemsSource::from_values(vec![(
                    Some("MoreColors".into()),
                    None,
                )])),
            )
            .unwrap();
            RibbonStrings::set_current(Some(
                RibbonStrings::for_culture(Some(RibbonCulture::new("en-US".into(), "en".into())))
                    .unwrap(),
            ))
            .unwrap();
            let p = RibbonColorPicker::new();
            p.set_is_split(false);
            let (_owner, trace) = watch(&p, "s:");
            let value = crate::compiled_shared::try_load(
                None,
                "ferres://FerroRibbon/Themes/ColorPalette.xaml",
            )
            .unwrap()
            .unwrap();
            let dictionary = ValueTypes::as_object(&*value)
                .unwrap()
                .cast::<ResourceDictionary>()
                .unwrap();
            let theme = dictionary
                .try_get_resource(
                    &<RibbonColorPalette as ferroui_base::StaticType>::TYPE.into(),
                    None,
                )
                .flatten()
                .unwrap();
            p.palette.set_resources(dictionary);
            p.palette.set_theme(
                ValueTypes::as_object(&*theme)
                    .unwrap()
                    .cast::<ControlTheme>()
                    .unwrap(),
            );
            p.palette.apply_styling();
            p.palette.apply_template();
            let w = Window::new();
            w.set_width(700.);
            w.set_height(700.);
            w.set_content(Some(Control::boxed(p.clone())));
            w.show();
            w.update_layout();
            p.open_drop_down();
            let reference = case("more_dialog_failure");
            assert_eq!(
                p.is_drop_down_open(),
                reference["opened"].as_bool().unwrap()
            );
            let more = p
                .palette
                .get_visual_descendants()
                .filter_map(|v| v.cast::<Button>())
                .last()
                .unwrap();
            Dispatcher::ui_thread().run_jobs(None);
            trace.borrow_mut().clear();
            more.on_click();
            Dispatcher::ui_thread().run_jobs(None);
            assert_eq!(p.is_drop_down_open(), reference["open"].as_bool().unwrap());
            assert_eq!(json!(*trace.borrow()), reference["trace"]);
            assert_eq!(state(&p), reference["state"]);
            let weak = p.downgrade();
            w.set_content(None);
            drop(p);
            drop(more);
            Dispatcher::ui_thread().run_jobs(None);
            assert!(
                weak.upgrade().is_none(),
                "completed failure must release the picker"
            );
            RibbonStrings::set_current(Some(previous)).unwrap();
            w.close();
        },
        ferroui_base::threading::CancellationToken::none(),
    )
    .wait()
    .unwrap();
}

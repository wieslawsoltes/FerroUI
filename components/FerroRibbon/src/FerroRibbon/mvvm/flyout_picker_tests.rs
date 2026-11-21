//! Source CreateFlyoutPalette corpus plus real native popup/dialog integration.
use super::*;
use crate::{controls::*, theming::to_ribbon_color};
use ferroribbon_core::collections::RibbonFixedItems;
use ferroribbon_core::{
    collections::RibbonReadOnlyList,
    commands::{ICommand, RibbonCommandCatalog, RibbonCommandDescriptor},
    model::*,
    theming::RibbonColor,
};
use ferroui_base::{
    Ref,
    media::Color,
    reactive::{Disposable, IDisposable},
    threading::Dispatcher,
};
use ferroui_controls::{Button, Control, Flyout, StackPanel, TextBlock, Window};
use ferroui_system::component_model::INotifyPropertyChanged;
use futures_util::FutureExt;
use serde_json::{Value, json};
use std::{any::Any, cell::RefCell, rc::Rc};
type Trace = Rc<RefCell<Vec<String>>>;
fn fixture(name: &str) -> Value {
    serde_json::from_str::<Value>(include_str!("../../../tests/fixtures/flyout-picker.json"))
        .unwrap()["corpus"]["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["name"] == name)
        .unwrap()
        .clone()
}
fn hex(color: Option<Color>) -> Option<String> {
    color.map(|c| to_ribbon_color(c).to_hex(false))
}
struct Cmd {
    id: &'static str,
    trace: Trace,
    allowed: bool,
    model: std::rc::Weak<RibbonColorPickerModel>,
    flyout: ferroui_base::WeakRef<Flyout>,
}
impl ICommand for Cmd {
    fn can_execute_changed(&self, _: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        Disposable::empty()
    }
    fn can_execute(&self, parameter: Option<&dyn Any>) -> bool {
        let color = parameter.map(|p| {
            p.downcast_ref::<RibbonColor>()
                .expect("Core RibbonColor")
                .to_hex(false)
        });
        let selected = self
            .model
            .upgrade()
            .unwrap()
            .selected_color()
            .map(|c| c.to_hex(false));
        self.trace.borrow_mut().push(format!(
            "{}:can:{}:model={}:open={}",
            self.id,
            color.as_deref().unwrap_or("<null>"),
            selected.as_deref().unwrap_or("<null>"),
            if self.flyout.upgrade().unwrap().is_open() {
                "True"
            } else {
                "False"
            }
        ));
        self.allowed
    }
    fn execute(&self, parameter: Option<&dyn Any>) {
        let color = parameter.map(|p| p.downcast_ref::<RibbonColor>().unwrap().to_hex(false));
        self.trace.borrow_mut().push(format!(
            "{}:execute:{}",
            self.id,
            color.as_deref().unwrap_or("<null>")
        ));
    }
}
fn command(
    id: &'static str,
    trace: &Trace,
    allowed: bool,
    model: &Rc<RibbonColorPickerModel>,
    flyout: &Flyout,
) -> Rc<dyn ICommand> {
    Rc::new(Cmd {
        id,
        trace: trace.clone(),
        allowed,
        model: Rc::downgrade(model),
        flyout: flyout.to_ref().downgrade(),
    })
}
fn mount(flyout: &Flyout) -> (Ref<Window>, Ref<Button>) {
    let target = Button::new();
    target.set_content(Some(Rc::new("Open palette".to_owned())));
    let w = Window::new();
    w.set_width(700.);
    w.set_height(700.);
    w.set_content(Some(Control::boxed(target.clone())));
    w.show();
    w.update_layout();
    flyout.show_at(&target);
    Dispatcher::ui_thread().run_jobs(None);
    assert!(flyout.is_open());
    (w, target)
}
fn palette(
    factory: &RibbonElementFactory,
    model: Rc<RibbonColorPickerModel>,
    scope: &Rc<RibbonModelBindings>,
) -> (Ref<Flyout>, Ref<RibbonColorPalette>) {
    let list = RibbonFixedItems::new(vec![Some(model as Rc<dyn RibbonNodeModelImpl>)])
        as RibbonReadOnlyList<_>;
    let flyout = factory
        .create_flyout(Some(list), None, Some(scope.clone()))
        .unwrap()
        .cast::<Flyout>()
        .unwrap();
    let panel = flyout
        .content()
        .and_then(|v| ferroui_base::data::core::ValueTypes::as_object(&*v))
        .unwrap()
        .cast::<StackPanel>()
        .unwrap();
    let p = panel
        .children()
        .get(0)
        .cast::<RibbonColorPalette>()
        .expect("native palette, not dropdown wrapper");
    // This test app loads the real reusable-library theme, as consumers do.
    let value =
        crate::compiled_shared::try_load(None, "ferres://FerroRibbon/Themes/ColorPalette.xaml")
            .unwrap()
            .unwrap();
    let dictionary = ferroui_base::data::core::ValueTypes::as_object(&*value)
        .unwrap()
        .cast::<ferroui_base::controls::ResourceDictionary>()
        .unwrap();
    let theme = dictionary
        .try_get_resource(
            &<RibbonColorPalette as ferroui_base::StaticType>::TYPE.into(),
            None,
        )
        .flatten()
        .unwrap();
    p.set_resources(dictionary);
    p.set_theme(
        ferroui_base::data::core::ValueTypes::as_object(&*theme)
            .unwrap()
            .cast::<ferroui_base::styling::ControlTheme>()
            .unwrap(),
    );

    (flyout, p)
}
#[test]
fn source_flyout_palette_selection_command_order_null_and_disposal_matrix() {
    crate::group_tests::run(|| {
        for mode in ["direct", "catalog", "denied", "null", "replace", "dispose"] {
            let factory = RibbonElementFactory::new();
            let scope = RibbonModelBindings::new();
            let model = RibbonColorPickerModel::new(Some("colors".into()), Some("Colors".into()));
            model.set_command_id(Some("catalog".into()));
            model.set_show_no_color(true);
            let (flyout, p) = palette(&factory, model.clone(), &scope);
            let trace: Trace = Rc::new(RefCell::default());
            let cmd = command("first", &trace, mode != "denied", &model, &flyout);
            let ribbon = Ribbon::new();
            let catalog = RibbonCommandCatalog::new();
            catalog.register(Some(RibbonCommandDescriptor::new(
                Some("catalog".into()),
                None,
                Some(cmd.clone()),
            )));
            ribbon.set_command_catalog(Some(catalog));
            factory.set_ribbon(Some(ribbon));
            if mode != "catalog" {
                model.set_command(Some(cmd));
            }
            let (w, _target) = mount(&flyout);
            let changes = Rc::new(RefCell::new(Vec::<String>::new()));
            let weak = Rc::downgrade(&model);
            let t = trace.clone();
            let c = changes.clone();
            let token = model.property_changed().add(Rc::new(move |name| {
                c.borrow_mut().push(name.to_owned());
                if name == "SelectedColor" {
                    t.borrow_mut().push(format!(
                        "model:{}",
                        weak.upgrade()
                            .unwrap()
                            .selected_color()
                            .map(|c| c.to_hex(false))
                            .as_deref()
                            .unwrap_or("<null>")
                    ));
                }
            }));
            let replacement = if mode == "replace" {
                let cmd = command("replacement", &trace, true, &model, &flyout);
                let weak = Rc::downgrade(&model);
                Some(model.property_changed().add(Rc::new(move |name| {
                    if name == "SelectedColor" {
                        weak.upgrade().unwrap().set_command(Some(cmd.clone()));
                    }
                })))
            } else {
                None
            };
            let t = trace.clone();
            let closed = flyout.closed(move || t.borrow_mut().push("hide".into()));
            if mode == "dispose" {
                scope.dispose();
            }
            p.select(if mode == "null" {
                None
            } else {
                Some(Color::from_argb(128, 1, 2, 3))
            });
            let source = fixture(mode);
            assert_eq!(json!(*trace.borrow()), source["trace"], "{mode}");
            assert_eq!(json!(*changes.borrow()), source["changes"], "{mode}");
            assert_eq!(
                json!(model.selected_color().map(|c| c.to_hex(false))),
                source["model"],
                "{mode}"
            );
            assert_eq!(json!(hex(p.selected_color())), source["palette"], "{mode}");
            assert_eq!(
                json!(
                    model
                        .recent_colors()
                        .iter()
                        .map(|c| c.to_hex(false))
                        .collect::<Vec<_>>()
                ),
                source["recent"],
                "{mode}"
            );
            assert_eq!(json!(flyout.is_open()), source["IsOpen"], "{mode}");
            model.property_changed().remove(token);
            if let Some(token) = replacement {
                model.property_changed().remove(token);
            }
            closed.dispose();
            scope.dispose();
            flyout.hide();
            w.close();
        }
    });
}
#[test]
fn source_flyout_palette_property_updates_and_binding_disposal() {
    crate::group_tests::run(|| {
        let f = RibbonElementFactory::new();
        let b = RibbonModelBindings::new();
        let m = RibbonColorPickerModel::new(None, None);
        let (_fly, p) = palette(&f, m.clone(), &b);
        m.set_show_automatic(false);
        m.set_show_no_color(true);
        m.set_show_more_colors(false);
        m.set_automatic_color(RibbonColor::from_rgb(1, 2, 3));
        m.set_selected_color(None);
        m.set_is_visible(false);
        m.set_is_enabled(false);
        let before = json!({"ShowAutomatic":p.show_automatic(),"ShowNoColor":p.show_no_color(),"ShowMoreColors":p.show_more_colors(),"automatic":hex(Some(p.automatic_color())),"selected":hex(p.selected_color()),"visible":p.is_visible(),"IsEnabled":p.is_enabled()});
        b.dispose();
        m.set_show_automatic(true);
        m.set_is_visible(true);
        m.set_selected_color(Some(RibbonColor::from_rgb(5, 6, 7)));
        let source = fixture("properties");
        assert_eq!(before, source["before"]);
        assert_eq!(
            json!({"ShowAutomatic":p.show_automatic(),"selected":hex(p.selected_color()),"visible":p.is_visible()}),
            source["disposed"]
        );
    });
}
fn more(palette: &RibbonColorPalette) {
    let button = palette
        .get_visual_descendants()
        .filter_map(|v| v.cast::<Button>())
        .find(|b| {
            b.get_visual_descendants()
                .filter_map(|v| v.cast::<TextBlock>())
                .any(|t| t.text().as_deref() == Some("More Colors..."))
        })
        .expect("actual More Colors button");
    button.on_click();
}
#[test]
fn native_flyout_more_colors_real_owner_result_cancel_busy_and_disposal() {
    crate::group_tests::run(|| {
        for mode in ["more_ok", "more_cancel", "more_busy", "more_dispose"] {
            let f = RibbonElementFactory::new();
            let b = RibbonModelBindings::new();
            let m = RibbonColorPickerModel::new(None, None);
            m.set_selected_color(None);
            let (fly, p) = palette(&f, m.clone(), &b);
            let trace: Trace = Rc::new(RefCell::default());
            m.set_command(Some(command("first", &trace, true, &m, &fly)));
            let (w, _target) = mount(&fly);
            let busy = if mode == "more_busy" {
                Some(crate::dialog_future_test_support::show_color(
                    Some(w.clone().upcast()),
                    Color::from_rgb(9, 8, 7),
                    None,
                ))
            } else {
                None
            };
            more(&p);
            assert!(!fly.is_open(), "flyout hides before dialog");
            Dispatcher::ui_thread().run_jobs(None);
            if mode == "more_dispose" {
                b.dispose();
            }
            let host = w
                .get_visual_descendants()
                .find_map(|v| v.cast::<RibbonColorDialogHost>())
                .expect("native dialog in owning window");
            if mode != "more_busy" {
                if mode != "more_cancel" {
                    let hex = host
                        .get_visual_descendants()
                        .find_map(|v| v.cast::<ferroui_controls::TextBox>())
                        .expect("hex editor");
                    hex.set_text(Some("#C8040506"));
                    Dispatcher::ui_thread().run_jobs(None);
                }
                host.process_button(if mode == "more_cancel" {
                    crate::compat::ContentDialogButton::Close
                } else {
                    crate::compat::ContentDialogButton::Primary
                });
                Dispatcher::ui_thread().run_jobs(None);
            } else {
                assert!(trace.borrow().is_empty());
                assert!(busy.as_ref().unwrap().clone().now_or_never().is_none());
                host.process_button(crate::compat::ContentDialogButton::Close);
                Dispatcher::ui_thread().run_jobs(None);
            }
            let source = fixture(mode);
            let expected = source["trace"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|v| v.as_str().unwrap().starts_with("first:"))
                .cloned()
                .collect::<Vec<_>>();
            assert_eq!(json!(*trace.borrow()), json!(expected), "{mode}");
            assert_eq!(
                json!(m.selected_color().map(|c| c.to_hex(false))),
                source["model"],
                "{mode}"
            );
            assert_eq!(json!(hex(p.selected_color())), source["palette"], "{mode}");
            b.dispose();
            w.close();
        }
    });
}

#[test]
fn native_model_flyout_grid_properties_current_command_hide_order_and_disposal() {
    crate::group_tests::run(|| {
        use ferroribbon_core::commands::RibbonRelayCommand;
        for dispose in [false, true] {
            let f = RibbonElementFactory::new();
            let b = RibbonModelBindings::new();
            let model = RibbonGridPickerModel::new(Some("grid".into()), Some("Table".into()));
            model.set_command_id(Some("table".into()));
            let catalog = RibbonCommandCatalog::new();
            let ribbon = Ribbon::new();
            ribbon.set_command_catalog(Some(catalog.clone()));
            f.set_ribbon(Some(ribbon));
            let list =
                RibbonFixedItems::new(vec![Some(model.clone() as Rc<dyn RibbonNodeModelImpl>)])
                    as RibbonReadOnlyList<_>;
            let fly = f
                .create_flyout(Some(list), None, Some(b.clone()))
                .unwrap()
                .cast::<Flyout>()
                .unwrap();
            let panel = Control::from_boxed(&fly.content().unwrap())
                .unwrap()
                .cast::<StackPanel>()
                .unwrap();
            let grid = panel
                .children()
                .get(0)
                .cast::<RibbonGridPicker>()
                .expect("same native grid, no extra dropdown");
            model.set_rows(4);
            model.set_columns(6);
            model.set_is_visible(false);
            model.set_is_enabled(false);
            assert_eq!(
                (
                    grid.rows(),
                    grid.columns(),
                    grid.is_visible(),
                    grid.is_enabled()
                ),
                (4, 6, false, false)
            );
            model.set_is_visible(true);
            model.set_is_enabled(true);
            let trace: Trace = Rc::new(RefCell::default());
            let t = trace.clone();
            let weak = fly.downgrade();
            let first = RibbonRelayCommand::new_with_parameter(
                move |p| {
                    let size = *p
                        .unwrap()
                        .downcast_ref::<RibbonGridSize>()
                        .expect("native grid parameter");
                    assert!(
                        !weak.upgrade().unwrap().is_open(),
                        "SizePicked hides before command"
                    );
                    t.borrow_mut()
                        .push(format!("first:{}x{}", size.columns, size.rows));
                },
                None,
            );
            catalog.register(Some(RibbonCommandDescriptor::new(
                Some("table".into()),
                None,
                Some(first),
            )));
            f.invalidate_commands();
            assert!(grid.command().is_some());
            let (w, target) = mount(&fly);
            if dispose {
                b.dispose();
            }
            grid.pick(RibbonGridSize {
                rows: 2,
                columns: 3,
            });
            assert_eq!(*trace.borrow(), vec!["first:3x2"]);
            assert!(!fly.is_open());
            model.set_rows(9);
            assert_eq!(grid.rows(), if dispose { 4 } else { 9 });
            if !dispose {
                let t = trace.clone();
                let second = RibbonRelayCommand::new_with_parameter(
                    move |_| t.borrow_mut().push("replacement".into()),
                    None,
                );
                let weak = Rc::downgrade(&model);
                let closed =
                    fly.closed(move || weak.upgrade().unwrap().set_command(Some(second.clone())));
                fly.show_at(&target);
                grid.pick(RibbonGridSize {
                    rows: 1,
                    columns: 1,
                });
                assert_eq!(*trace.borrow(), vec!["first:3x2", "replacement"]);
                closed.dispose();
            }
            b.dispose();
            fly.hide();
            w.close();
        }
    });
}

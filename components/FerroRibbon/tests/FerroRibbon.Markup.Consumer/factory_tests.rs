use ferroribbon::{
    controls::{RibbonButton, RibbonGroup, RibbonSearch, RibbonSlider},
    mvvm::{RibbonElementFactory, RibbonModelBindings},
};
use ferroribbon_core::model::*;
use ferroui_base::data::core::ValueTypes;
use ferroui_controls::StackPanel;
use ferroribbon_core::collections::ObservableCollection;
use std::rc::Rc;

#[test]
fn downstream_factory_execute_checks_once_and_passes_the_same_parameter() {
    use ferroribbon_core::commands::ICommand;
    use ferroui_base::reactive::{Disposable, IDisposable};
    use std::{
        any::Any,
        cell::{Cell, RefCell},
    };

    #[derive(Default)]
    struct Command {
        enabled: Cell<bool>,
        calls: RefCell<Vec<(&'static str, Option<*const String>)>>,
    }

    fn identity(value: Option<&dyn Any>) -> Option<*const String> {
        value.map(|value| std::ptr::from_ref(value.downcast_ref::<String>().unwrap()))
    }

    impl ICommand for Command {
        fn can_execute(&self, parameter: Option<&dyn Any>) -> bool {
            self.calls.borrow_mut().push(("check", identity(parameter)));
            self.enabled.get()
        }

        fn execute(&self, parameter: Option<&dyn Any>) {
            self.calls
                .borrow_mut()
                .push(("execute", identity(parameter)));
        }

        fn can_execute_changed(&self, _: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
            Disposable::empty()
        }
    }

    let command = Command::default();
    let parameter = "preview".to_owned();
    let address = Some(std::ptr::from_ref(&parameter));

    RibbonElementFactory::execute(None, Some(&parameter));
    RibbonElementFactory::execute(Some(&command), Some(&parameter));
    assert_eq!(*command.calls.borrow(), [("check", address)]);
    command.calls.borrow_mut().clear();

    command.enabled.set(true);
    RibbonElementFactory::execute(Some(&command), Some(&parameter));
    RibbonElementFactory::execute(Some(&command), None);
    assert_eq!(
        *command.calls.borrow(),
        [
            ("check", address),
            ("execute", address),
            ("check", None),
            ("execute", None),
        ]
    );
}

#[test]
fn independent_compiled_runtime_factory_subclass_and_attached_search_generate_same_public_controls()
{
    super::tests::run(|| {
        let _theme = super::tests::NativeThemeScope::new();
        for runtime in [false, true] {
            let value = if runtime {
                ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                    include_str!("Factory.xaml"),
                    Some(&crate::CONSUMER_ASSEMBLY),
                )
                .unwrap()
            } else {
                crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferroribbon_markup_consumer/Factory.xaml",
                )
                .unwrap()
                .unwrap()
            };
            let panel = ValueTypes::as_object(&*value)
                .unwrap()
                .cast::<StackPanel>()
                .unwrap();
            let declared = panel.children().get(0).cast::<RibbonButton>().unwrap();
            assert_eq!(
                RibbonSearch::get_keywords(&declared).as_deref(),
                Some("copy,paste")
            );
            assert!(!RibbonSearch::get_is_searchable(&declared));
            let native = ValueTypes::as_object(&*declared.tag().unwrap())
                .unwrap()
                .cast::<RibbonElementFactory>()
                .unwrap();
            assert!(native.ribbon().is_none());
            let group = panel.children().get(1).cast::<RibbonGroup>().unwrap();
            let factory = ValueTypes::as_object(&*group.tag().unwrap())
                .unwrap()
                .cast::<super::factory_consumer::ConsumerElementFactory>()
                .unwrap();
            let button = RibbonButtonModel::new(Some("cut".into()), Some("Cut".into()));
            let slider = RibbonSliderModel::new(Some("opacity".into()), Some("Opacity".into()));
            slider.set_value(64.);
            let source: ObservableCollection<Option<Rc<dyn RibbonItemModelImpl>>> =
                ObservableCollection::new_managed();
            source.add(Some(button.clone()));
            source.add(Some(slider.clone()));
            let bindings = RibbonModelBindings::new();
            let f = factory.clone().upcast::<RibbonElementFactory>();
            let mirror = bindings.items(
                source.clone(),
                Rc::new(group.items()),
                Rc::new(move |m, b| f.create_item(m, b)),
                true,
                None,
                None,
            );
            assert_eq!(factory.created_count(), 1);
            assert_eq!(group.items().count(), 3);
            let made = group.items().get(1).unwrap();
            assert!(made.is::<super::button_consumer::ConsumerRibbonButton>());
            let input = group
                .items()
                .get(2)
                .unwrap()
                .cast::<RibbonSlider>()
                .unwrap();
            assert_eq!(input.value(), 64.);
            input.set_numeric_value(39.);
            assert_eq!(slider.value(), 39.);
            source.move_item(1, 0);
            assert!(group.items().get(2).unwrap().ptr_eq(&made));
            assert_eq!(factory.created_count(), 1);
            button.set_label(Some("Cut updated".into()));
            assert_eq!(
                made.cast::<RibbonButton>().unwrap().label().as_deref(),
                Some("Cut updated")
            );
            source.remove_at(1);
            assert_eq!(mirror.elements().iter().count(), 1);
            bindings.dispose();
            assert_eq!(group.items().count(), 1);
            assert_eq!(
                group
                    .items()
                    .get(0)
                    .unwrap()
                    .cast::<RibbonButton>()
                    .unwrap()
                    .id()
                    .as_deref(),
                Some("declared")
            );
        }
    });
}

#[test]
fn independent_generated_tab_group_row_and_slider_share_native_ribbon_layout_and_lifecycle() {
    super::tests::run(|| {
        use ferroribbon::controls::{Ribbon, RibbonButtonGroup, RibbonStackPanel};
        use ferroui_base::{Rect, Size, threading::Dispatcher};
        use ferroui_controls::{Control, Window};
        let _theme = super::tests::NativeThemeScope::new();
        for runtime in [false, true] {
            let value = if runtime {
                ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                    include_str!("Factory.xaml"),
                    Some(&crate::CONSUMER_ASSEMBLY),
                )
                .unwrap()
            } else {
                crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferroribbon_markup_consumer/Factory.xaml",
                )
                .unwrap()
                .unwrap()
            };
            let panel = ValueTypes::as_object(&*value)
                .unwrap()
                .cast::<StackPanel>()
                .unwrap();
            let ribbon = panel.children().get(2).cast::<Ribbon>().unwrap();
            let factory = ValueTypes::as_object(
                &*panel
                    .children()
                    .get(1)
                    .cast::<RibbonGroup>()
                    .unwrap()
                    .tag()
                    .unwrap(),
            )
            .unwrap()
            .cast::<super::factory_consumer::ConsumerElementFactory>()
            .unwrap();
            factory.set_ribbon(Some(ribbon.clone()));
            let tab = RibbonTabModel::new(Some("generated".into()), Some("Generated".into()));
            let group = RibbonGroupModel::new(Some("tools".into()), Some("Tools".into()));
            let row = RibbonRowModel::new(Some("row".into()));
            let joined = RibbonButtonGroupModel::new(Some("joined".into()));
            let button = RibbonButtonModel::new(Some("button".into()), Some("Action".into()));
            let slider = RibbonSliderModel::new(Some("opacity".into()), Some("Opacity".into()));
            slider.set_value(75.);
            let commits = Rc::new(std::cell::RefCell::new(Vec::<f64>::new()));
            let out = commits.clone();
            slider.set_command(Some(Rc::new(
                ferroribbon_core::commands::RibbonRelayCommand::construct(
                    Rc::new(move |p| {
                        out.borrow_mut()
                            .push(*p.unwrap().downcast_ref::<f64>().unwrap())
                    }),
                    None,
                ),
            )));
            let clicks = Rc::new(std::cell::Cell::new(0));
            let out = clicks.clone();
            button.set_command(Some(ferroribbon_core::commands::RibbonRelayCommand::new(
                move || out.set(out.get() + 1),
            )));
            let invocations = Rc::new(std::cell::Cell::new(0));
            let out = invocations.clone();
            ribbon.add_item_invoked(Some(ferroui_base::metadata::MarkupDelegate::new(
                move |_| {
                    out.set(out.get() + 1);
                    None
                },
            )));

            joined.items().add(Some(button.clone()));
            row.items().add(Some(joined));
            row.items().add(Some(slider.clone()));
            group.items().add(Some(row));
            tab.groups().add(Some(group.clone()));
            let models: ObservableCollection<Option<Rc<dyn RibbonTabModelImpl>>> =
                ObservableCollection::new_managed();
            models.add(Some(tab.clone()));
            let bindings = RibbonModelBindings::new();
            let f = factory.clone();
            bindings.items(
                models.clone(),
                Rc::new(ribbon.tabs()),
                Rc::new(move |m, b| Some(f.create_tab(m.unwrap(), b))),
                true,
                None,
                None,
            );
            assert_eq!(ribbon.tabs().count(), 2);
            let created = ribbon.tabs().get(1).unwrap();
            assert_eq!(factory.created_group_count(), 1);
            let g = created.groups().get(0).unwrap();
            let r = g
                .items()
                .get(0)
                .unwrap()
                .cast::<RibbonStackPanel>()
                .unwrap();
            let joined = r
                .items()
                .get(0)
                .unwrap()
                .cast::<RibbonButtonGroup>()
                .unwrap();
            let command = joined
                .items()
                .get(0)
                .unwrap()
                .cast::<super::button_consumer::ConsumerRibbonButton>()
                .unwrap();
            let input = r.items().get(1).unwrap().cast::<RibbonSlider>().unwrap();
            assert!(Rc::ptr_eq(
                &Ribbon::get_model(&g).unwrap(),
                &(group.clone() as Rc<dyn RibbonNodeModelImpl>)
            ));
            let w = Window::new();
            w.set_content(Some(Control::boxed(panel.clone())));
            w.show();
            assert!(ribbon.select_tab_id("generated"));
            for width in [1400.25, 460.5, 920.75, 1400.25] {
                w.measure(Size::new(width, 800.));
                w.arrange(Rect::new(0., 0., width, 800.));
                Dispatcher::ui_thread().run_jobs(None);
            }
            assert!(created.is_loaded());
            assert!(input.is_loaded());
            assert!(input.bounds().width > 0.);
            assert!(command.is_loaded());
            assert!(command.bounds().width > 0.);
            let part = input
                .get_visual_descendants()
                .find_map(|v| v.cast::<ferroribbon::compat::RibbonInputSlider>())
                .unwrap();
            let mut key = ferroui_base::input::KeyEventArgs::new();
            key.key = ferroui_base::input::Key::Right;
            key.set_routed_event(Some(ferroui_base::input::InputElement::key_down_event()));
            part.raise_event(&key);
            assert!(key.handled());
            assert_eq!(slider.value(), 76.);
            assert_eq!(*commits.borrow(), vec![76.]);
            command.perform_click();
            assert_eq!(clicks.get(), 1);
            assert_eq!(invocations.get(), 2);
            input.set_numeric_value(43.);
            assert_eq!(slider.value(), 43.);
            slider.set_value(62.);
            assert_eq!(input.value(), 62.);
            tab.set_label(Some("Renamed".into()));
            assert_eq!(created.header().as_deref(), Some("Renamed"));
            group.set_label(Some("Changed tools".into()));
            assert_eq!(g.header().as_deref(), Some("Changed tools"));
            bindings.suspend();
            tab.set_label(Some("Paused".into()));
            assert_eq!(created.header().as_deref(), Some("Renamed"));
            bindings.resume();
            assert_eq!(created.header().as_deref(), Some("Paused"));
            bindings.dispose();
            assert_eq!(ribbon.tabs().count(), 1);
            assert_eq!(
                ribbon.tabs().get(0).unwrap().id().as_deref(),
                Some("declared-tab")
            );
            tab.set_label(Some("Disposed".into()));
            assert_eq!(created.header().as_deref(), Some("Paused"));
            factory.set_ribbon(None);
            w.close();
            w.set_content(None);
            Dispatcher::ui_thread().run_jobs(None);
        }
    });
}

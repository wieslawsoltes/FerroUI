//! Source MvvmTests.cs at 9755e281: actual public models and native controls.
//! Copyright Wiesław Šoltés, MIT. Both native markup paths supply the same host.
use ferroribbon::{
    compat::MenuFlyoutItem,
    controls::*,
    mvvm::{RibbonElementFactory, RibbonModelBindings},
};
use ferroribbon_core::{commands::*, model::*, theming::RibbonColor};
use ferroui_base::{
    Ref,
    data::core::{ValueType, ValueTypes},
    media::Color,
    threading::Dispatcher,
};
use ferroui_controls::{
    Control, Flyout, StackPanel, TextBlock, Window, automation::AutomationProperties,
};
use std::{
    any::Any,
    cell::{Cell, RefCell},
    rc::Rc,
};

fn settle() {
    Dispatcher::ui_thread().run_jobs(None);
}

fn mount(control: Ref<Control>, width: f64, runtime: bool) -> Ref<Window> {
    let value = if runtime {
        ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
            include_str!("RuntimeMvvmHost.xaml"),
            Some(&crate::CONSUMER_ASSEMBLY),
        )
        .unwrap()
    } else {
        crate::compiled_xaml::try_load(
            None,
            "ferres://ferroribbon_markup_consumer/RuntimeMvvmHost.xaml",
        )
        .unwrap()
        .unwrap()
    };
    let host = ValueTypes::as_object(&*value)
        .unwrap()
        .cast::<StackPanel>()
        .unwrap();
    host.children().add(control);
    let window = Window::new();
    window.set_width(width);
    window.set_height(550.);
    window.set_content(Some(Control::boxed(host)));
    window.show();
    window.update_layout();
    settle();
    window
}

fn finish(window: &Window) {
    window.close();
    window.set_content(None);
    settle();
}

fn create_model(
    catalog: Option<Rc<RibbonCommandCatalog>>,
) -> (Rc<RibbonModel>, Rc<RibbonGroupModel>) {
    let group = RibbonGroupModel::new(Some("g".into()), Some("Group".into()));
    let model = RibbonModel::new();
    model.set_command_catalog(catalog);
    let tab = RibbonTabModel::new(Some("home".into()), Some("Home".into()));
    tab.groups().add(Some(group.clone()));
    model.tabs().add(Some(tab));
    (model, group)
}

#[test]
fn command_items_follow_model_enabled_state_both_ways() {
    crate::tests::run(|| {
        let _theme = crate::tests::NativeThemeScope::new();
        for runtime in [false, true] {
            let catalog = RibbonCommandCatalog::new();
            catalog.register_action(
                Some("save".into()),
                Some("Save".into()),
                Rc::new(|_| {}),
                None,
                RibbonCommandMetadata::default(),
            );
            let (model, group) = create_model(Some(catalog));
            let item = RibbonButtonModel::new(Some("save".into()), Some("Save".into()));
            item.set_command_id(Some("save".into()));
            group.items().add(Some(item));
            let ribbon = Ribbon::new();
            ribbon.set_model(Some(model.clone()));
            let window = mount(ribbon.clone().upcast(), 1200., runtime);
            let button = ribbon
                .find_item(Some("save".into()))
                .unwrap()
                .cast::<RibbonButton>()
                .unwrap();
            model.set_command_enabled(Some("save".into()), false);
            settle();
            assert!(!button.is_enabled());
            model.set_command_enabled(Some("save".into()), true);
            settle();
            assert!(button.is_enabled());
            finish(&window);
        }
    });
}

#[test]
fn gallery_and_color_picker_execute_catalog_command_once_with_model_parameters() {
    crate::tests::run(|| {
        struct StyleValue(&'static str);

        let _theme = crate::tests::NativeThemeScope::new();
        for runtime in [false, true] {
            let catalog = RibbonCommandCatalog::new();
            let gallery_calls = Rc::new(RefCell::new(Vec::new()));
            let color_calls = Rc::new(RefCell::new(Vec::new()));
            let custom_style = Rc::new(StyleValue("custom-style"));
            let expected_style = custom_style.clone();
            let out = gallery_calls.clone();
            catalog.register_action(
                Some("style".into()),
                Some("Style".into()),
                Rc::new(move |p| {
                    let value = p.unwrap();
                    if let Some(style) = value.downcast_ref::<StyleValue>() {
                        assert!(std::ptr::eq(style, expected_style.as_ref()));
                        out.borrow_mut().push(style.0.to_owned());
                    } else {
                        out.borrow_mut()
                            .push(value.downcast_ref::<String>().unwrap().clone());
                    }
                }),
                None,
                RibbonCommandMetadata::default(),
            );
            let out = color_calls.clone();
            catalog.register_action(
                Some("fill".into()),
                Some("Fill".into()),
                Rc::new(move |p| {
                    out.borrow_mut()
                        .push(*p.unwrap().downcast_ref::<RibbonColor>().unwrap())
                }),
                None,
                RibbonCommandMetadata::default(),
            );
            let (model, group) = create_model(Some(catalog));
            let gallery = RibbonGalleryModel::new(Some("styles".into()), Some("Styles".into()));
            gallery.set_command_id(Some("style".into()));
            let item = RibbonGalleryItemModel::new(Some("normal".into()), Some("Normal".into()));
            item.set_value(Some(RibbonObject::value("normal-style".to_owned())));
            gallery.items().add(Some(item.clone()));
            let custom = RibbonGalleryItemModel::new(Some("custom".into()), Some("Custom".into()));
            custom.set_value(Some(RibbonObject::reference(custom_style)));
            gallery.items().add(Some(custom));
            group.items().add(Some(gallery));
            let picker = RibbonColorPickerModel::new(Some("fillColor".into()), Some("Fill".into()));
            picker.set_command_id(Some("fill".into()));
            group.items().add(Some(picker));
            let ribbon = Ribbon::new();
            ribbon.set_model(Some(model));
            let window = mount(ribbon.clone().upcast(), 1400., runtime);
            let gallery = ribbon
                .find_item(Some("styles".into()))
                .unwrap()
                .cast::<RibbonGallery>()
                .unwrap();
            let selected = gallery.effective_items().get(0).unwrap();
            let core = ValueTypes::try_cast(
                &selected,
                ValueType::of::<Rc<dyn RibbonGalleryItemModelImpl>>(),
            )
            .unwrap();
            assert!(Rc::ptr_eq(
                core.downcast_ref::<Rc<dyn RibbonGalleryItemModelImpl>>()
                    .unwrap(),
                &(item as Rc<dyn RibbonGalleryItemModelImpl>)
            ));
            gallery.pick(Some(selected));
            gallery.pick(gallery.effective_items().get(1));
            ribbon
                .find_item(Some("fillColor".into()))
                .unwrap()
                .cast::<RibbonColorPicker>()
                .unwrap()
                .palette()
                .select(Some(Color::from_argb(255, 0, 0, 255)));
            settle();
            assert_eq!(*gallery_calls.borrow(), ["normal-style", "custom-style"]);
            assert_eq!(
                *color_calls.borrow(),
                vec![RibbonColor::parse(Some(&"#0000FF".into())).unwrap()]
            );
            finish(&window);
        }
    });
}

#[test]
fn collection_changes_keep_existing_elements_and_relink_quick_access() {
    crate::tests::run(|| {
        let _theme = crate::tests::NativeThemeScope::new();
        for runtime in [false, true] {
            let (model, group) = create_model(None);
            let cut = RibbonButtonModel::new(Some("cut".into()), Some("Cut".into()));
            cut.set_icon(Some(RibbonIcon::glyph(Some(RibbonIcons::Cut.into()))));
            group.items().add(Some(cut.clone()));
            model.quick_access_items().add(Some(cut.clone()));
            let ribbon = Ribbon::new();
            ribbon.set_model(Some(model.clone()));
            let window = mount(ribbon.clone().upcast(), 1200., runtime);
            let tab = ribbon.find_tab("home").unwrap();
            let original = ribbon.find_item(Some("cut".into())).unwrap();
            let copy = RibbonButtonModel::new(Some("copy".into()), Some("Copy".into()));
            copy.set_icon(Some(RibbonIcon::glyph(Some(RibbonIcons::Copy.into()))));
            group.items().add(Some(copy));
            model.tabs().add(Some(RibbonTabModel::new(
                Some("view".into()),
                Some("View".into()),
            )));
            settle();
            assert!(tab.ptr_eq(&ribbon.find_tab("home").unwrap()));
            assert!(original.ptr_eq(&ribbon.find_item(Some("cut".into())).unwrap()));
            let other = RibbonGroupModel::new(Some("g2".into()), Some("Other".into()));
            model
                .tabs()
                .get(0)
                .unwrap()
                .groups()
                .add(Some(other.clone()));
            let source: Rc<dyn RibbonItemModelImpl> = cut.clone();
            assert!(group.items().remove(&Some(source)));
            other.items().add(Some(cut));
            settle();
            let replacement = ribbon.find_item(Some("cut".into())).unwrap();
            assert!(!original.ptr_eq(&replacement));
            let quick = ribbon.quick_access_tool_bar().unwrap();
            assert_eq!(quick.items().count(), 1);
            assert!(
                RibbonItemHelper::get_source_item(&quick.items().get(0).unwrap())
                    .unwrap()
                    .ptr_eq(&replacement)
            );
            finish(&window);
        }
    });
}

#[test]
fn menu_rebuilds_without_reparenting_errors() {
    crate::tests::run(|| {
        let _theme = crate::tests::NativeThemeScope::new();
        ValueTypes::register_erased_value::<Ref<TextBlock>>();
        for runtime in [false, true] {
            let (model, group) = create_model(None);
            let content = TextBlock::new();
            content.set_text(Some("Custom"));
            let drop = RibbonDropDownButtonModel::new(Some("more".into()), Some("More".into()));
            drop.set_drop_down_content(Some(RibbonObject::value(content.clone())));
            drop.menu_items().add(Some(RibbonMenuItemModel::new(
                Some("a".into()),
                Some("A".into()),
            )));
            group.items().add(Some(drop.clone()));
            let ribbon = Ribbon::new();
            ribbon.set_model(Some(model));
            let window = mount(ribbon.clone().upcast(), 1200., runtime);
            drop.menu_items().add(Some(RibbonMenuItemModel::new(
                Some("b".into()),
                Some("B".into()),
            )));
            drop.menu_items().add(Some(RibbonMenuItemModel::new(
                Some("c".into()),
                Some("C".into()),
            )));
            settle();
            let flyout = ribbon
                .find_item(Some("more".into()))
                .unwrap()
                .cast::<RibbonDropDownButton>()
                .unwrap()
                .flyout()
                .unwrap()
                .cast::<Flyout>()
                .unwrap();
            let panel = Control::from_boxed(&flyout.content().unwrap())
                .unwrap()
                .cast::<StackPanel>()
                .unwrap();
            assert!(
                panel
                    .children()
                    .snapshot()
                    .iter()
                    .any(|c| c.ptr_eq(&content))
            );
            finish(&window);
        }
    });
}

#[test]
fn backstage_items_added_later_replace_and_restore_xaml_backstage() {
    crate::tests::run(|| {
        let _theme = crate::tests::NativeThemeScope::new();
        for runtime in [false, true] {
            let (model, _) = create_model(None);
            let original = RibbonBackstage::new();
            let ribbon = Ribbon::new();
            ribbon.set_backstage(Some(original.clone()));
            ribbon.set_model(Some(model.clone()));
            let window = mount(ribbon.clone().upcast(), 1200., runtime);
            assert!(ribbon.backstage().unwrap().ptr_eq(&original));
            let item = RibbonBackstageItemModel::new(Some("info".into()), Some("Info".into()));
            item.set_content(Some(RibbonObject::value("Info page".to_owned())));
            model.backstage().items().add(Some(item));
            settle();
            assert!(!ribbon.backstage().unwrap().ptr_eq(&original));
            ribbon.set_model(None);
            settle();
            assert!(ribbon.backstage().unwrap().ptr_eq(&original));
            finish(&window);
        }
    });
}

#[test]
fn backstage_item_changes_refresh_navigation() {
    crate::tests::run(|| {
        let _theme = crate::tests::NativeThemeScope::new();
        for runtime in [false, true] {
            let backstage = RibbonBackstage::new();
            let item = RibbonBackstageItem::new();
            item.set_header(Some("Info".into()));
            item.set_content(Some(Rc::new("Page".to_owned())));
            backstage.items().add(Some(item.clone()));
            let window = mount(backstage.upcast(), 800., runtime);
            item.set_header(Some("Information".into()));
            item.set_is_enabled(false);
            let navigation = item.navigation_button().unwrap();
            assert_eq!(
                AutomationProperties::get_name(&navigation).as_deref(),
                Some("Information")
            );
            assert!(!navigation.is_enabled());
            finish(&window);
        }
    });
}

#[test]
fn menu_bar_rebuild_keeps_entries() {
    crate::tests::run(|| {
        let _theme = crate::tests::NativeThemeScope::new();
        for runtime in [false, true] {
            let file = RibbonMenuBarItem::new();
            file.set_header(Some("File".into()));
            let open = MenuFlyoutItem::new();
            open.set_text(Some("Open".into()));
            file.items().add(Some(open.upcast()));
            let bar = RibbonMenuBar::new();
            bar.items().add(Some(file.clone()));
            let window = mount(bar.clone().upcast(), 600., runtime);
            let edit = RibbonMenuBarItem::new();
            edit.set_header(Some("Edit".into()));
            bar.items().add(Some(edit));
            let save = MenuFlyoutItem::new();
            save.set_text(Some("Save".into()));
            file.items().add(Some(save.upcast()));
            settle();
            assert_eq!(file.items().count(), 2);
            assert!(bar.open_menu(Some("File")));
            settle();
            assert!(bar.is_menu_open());
            finish(&window);
        }
    });
}

#[derive(Clone, Debug, PartialEq)]
enum ParameterObservation {
    Color(RibbonColor),
    NativeColor(Color),
    Null,
    Opaque,
}
#[derive(PartialEq)]
struct OpaqueParameter(u8);
fn observe_parameter(value: Option<&dyn Any>, opaque: &OpaqueParameter) -> ParameterObservation {
    match value {
        None => ParameterObservation::Null,
        Some(value) => match value.downcast_ref::<RibbonColor>() {
            Some(color) => ParameterObservation::Color(*color),
            None if value.is::<Color>() => {
                ParameterObservation::NativeColor(*value.downcast_ref::<Color>().unwrap())
            }
            None => {
                assert!(std::ptr::eq(
                    value.downcast_ref::<OpaqueParameter>().unwrap(),
                    opaque
                ));
                ParameterObservation::Opaque
            }
        },
    }
}

#[test]
fn color_parameter_adapter_preserves_selection_rewraps_raw_commands_and_disposes() {
    crate::tests::run(|| {
        let _theme = crate::tests::NativeThemeScope::new();
        let opaque = Rc::new(OpaqueParameter(17));
        let calls = Rc::new(RefCell::new(Vec::new()));
        let checks = Rc::new(RefCell::new(Vec::new()));
        let out = calls.clone();
        let payload = opaque.clone();
        let check = checks.clone();
        let check_payload = opaque.clone();
        let core = RibbonRelayCommand::new_with_parameter(
            move |p| out.borrow_mut().push(observe_parameter(p, &payload)),
            Some(Rc::new(move |p| {
                check
                    .borrow_mut()
                    .push(observe_parameter(p, &check_payload));
                true
            })),
        );
        let model = RibbonColorPickerModel::new(Some("color".into()), Some("Color".into()));
        model.set_selected_color(Some(RibbonColor::new(73, 1, 2, 3)));
        model.set_command_parameter(Some(RibbonObject::value(
            "must not replace selected color".to_owned(),
        )));
        model.set_command(Some(core.clone()));
        let factory = RibbonElementFactory::new();
        let bindings = RibbonModelBindings::new();
        let picker = factory
            .create_item(Some(model.clone()), bindings.clone())
            .unwrap()
            .cast::<RibbonColorPicker>()
            .unwrap();
        assert_eq!(
            picker.command_parameter().unwrap().downcast_ref::<Color>(),
            Some(&Color::from_argb(73, 1, 2, 3))
        );
        let original = picker.command().unwrap();
        let raw = ferroribbon::commands::adapt_core_command(core.clone());
        // A raw command can be queried by the base property callback before
        // the per-instance factory observer replaces it. Its object predicate
        // accepts that intermediate native value; wrapper calls below must
        // all receive the source Core convention.
        picker.set_command(Some(raw.clone()));
        let wrapped = picker.command().unwrap();
        assert!(!Rc::ptr_eq(&raw, &wrapped));
        checks.borrow_mut().clear();
        let parameters = [
            Some(Rc::new(Color::from_argb(73, 4, 5, 6)) as ferroui_base::BoxedValue),
            Some(RibbonColorPicker::no_color_parameter()),
            None,
            Some(opaque.clone() as ferroui_base::BoxedValue),
        ];
        for parameter in &parameters {
            assert!(wrapped.can_execute(parameter.as_ref()));
            wrapped.execute(parameter.as_ref());
        }
        let expected = vec![
            ParameterObservation::Color(RibbonColor::new(73, 4, 5, 6)),
            ParameterObservation::Null,
            ParameterObservation::Null,
            ParameterObservation::Opaque,
        ];
        // RelayCommand.Execute performs its own source CanExecute check in
        // addition to the explicit wrapper query above. Executions remain once.
        let expected_checks = expected
            .iter()
            .flat_map(|value| [value.clone(), value.clone()])
            .collect::<Vec<_>>();
        assert_eq!(*checks.borrow(), expected_checks);
        assert_eq!(*calls.borrow(), expected);
        let notifications = Rc::new(Cell::new(0));
        let out = notifications.clone();
        let subscription = wrapped.can_execute_changed(Rc::new(move || out.set(out.get() + 1)));
        core.notify_can_execute_changed();
        assert_eq!(notifications.get(), 1);
        subscription.dispose();
        core.notify_can_execute_changed();
        assert_eq!(notifications.get(), 1);
        bindings.dispose();
        model.set_command(None);
        assert!(Rc::ptr_eq(&picker.command().unwrap(), &wrapped));
        picker.set_command(Some(raw.clone()));
        assert!(Rc::ptr_eq(&picker.command().unwrap(), &raw));
        let weak = Rc::downgrade(&wrapped);
        drop(wrapped);
        picker.set_command(None);
        assert!(weak.upgrade().is_none());
        drop(original);
    });
}

#[test]
fn model_grid_command_belongs_to_picker_without_dropdown_parameter_binding() {
    crate::tests::run(|| {
        let _theme = crate::tests::NativeThemeScope::new();
        for runtime in [false, true] {
            let model = RibbonGridPickerModel::new(Some("table".into()), Some("Table".into()));
            let calls = Rc::new(RefCell::new(Vec::new()));
            let out = calls.clone();
            model.set_command(Some(RibbonRelayCommand::new_with_parameter(
                move |p| {
                    out.borrow_mut()
                        .push(*p.unwrap().downcast_ref::<RibbonGridSize>().unwrap())
                },
                None,
            )));
            let factory = RibbonElementFactory::new();
            let bindings = RibbonModelBindings::new();
            let button = factory
                .create_item(Some(model), bindings.clone())
                .unwrap()
                .cast::<RibbonDropDownButton>()
                .unwrap();
            assert!(button.command().is_none());
            let window = mount(button.clone().upcast(), 600., runtime);
            button.invoke_core();
            assert!(calls.borrow().is_empty());
            let flyout = button.flyout().unwrap().cast::<Flyout>().unwrap();
            let picker = Control::from_boxed(&flyout.content().unwrap())
                .unwrap()
                .cast::<RibbonGridPicker>()
                .unwrap();
            picker.pick(RibbonGridSize::new(2, 3));
            assert_eq!(*calls.borrow(), vec![RibbonGridSize::new(2, 3)]);
            bindings.dispose();
            finish(&window);
        }
    });
}

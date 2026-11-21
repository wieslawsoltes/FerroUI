//! ItemTests.cs interaction sequences from RibbonSpace 9755e281.
//! Copyright Wiesław Šoltés, MIT. Native compiled and runtime Generic hosts.
use ferroribbon::{compat::FontIcon, controls::*, primitives::RibbonIconPresenter};
use ferroribbon_core::{commands::*, model::RibbonIcon};
use ferroui_base::{
    ObjectType, Ref, Visual,
    data::core::{ValueType, ValueTypes},
    media::Color,
    threading::Dispatcher,
};
use ferroui_controls::{Border, Control, StackPanel, TextBlock, ToolTip, Viewbox, Window};
use std::{cell::Cell, rc::Rc};

fn settle() {
    Dispatcher::ui_thread().run_jobs(None);
}
fn mount(control: Ref<Control>, runtime: bool) -> Ref<Window> {
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
    window.set_width(1400.);
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
fn item(control: Ref<Control>) -> Rc<dyn IRibbonItem> {
    ValueTypes::try_cast(
        &Control::boxed(control),
        ValueType::of::<Rc<dyn IRibbonItem>>(),
    )
    .unwrap()
    .downcast_ref::<Rc<dyn IRibbonItem>>()
    .unwrap()
    .clone()
}
fn check(test: impl Fn(bool) + Send + 'static) {
    crate::tests::run(move || {
        let _theme = crate::tests::NativeThemeScope::new();
        for runtime in [false, true] {
            test(runtime);
        }
    });
}

#[test]
fn non_split_color_picker_opens_palette_instead_of_applying() {
    check(|runtime| {
        let executed = Rc::new(Cell::new(0));
        let calls = executed.clone();
        let picker = RibbonColorPicker::new();
        picker.set_is_split(false);
        picker.set_label(Some("Fill".into()));
        picker.set_command(Some(ferroribbon::commands::adapt_core_command(
            RibbonRelayCommand::new(move || calls.set(calls.get() + 1)),
        )));
        let window = mount(picker.clone().upcast(), runtime);
        item(picker.clone().upcast()).invoke();
        settle();
        assert_eq!(
            executed.get(),
            0,
            "Opening a non-split picker must not apply its color"
        );
        assert!(picker.is_drop_down_open());
        picker.flyout().unwrap().hide();
        settle();
        finish(&window);
    });
}

#[test]
fn color_picker_palette_stays_open_with_a_selected_color() {
    check(|runtime| {
        let picker = RibbonColorPicker::new();
        picker.set_label(Some("Font Color".into()));
        picker.set_selected_color(Some(Color::from_argb(255, 0xc0, 0, 0)));
        let window = mount(picker.clone().upcast(), runtime);
        picker.open_drop_down();
        settle();
        window.update_layout();
        settle();
        assert!(
            picker.is_drop_down_open(),
            "Focusing the selected color must not close the palette"
        );
        picker.close_drop_down();
        settle();
        finish(&window);
    });
}

#[test]
fn gallery_element_items_survive_rebuilds() {
    check(|runtime| {
        let element = Border::new();
        element.set_width(20.);
        element.set_height(20.);
        let gallery = RibbonGallery::new();
        gallery.set_label(Some("Shapes".into()));
        gallery.set_max_columns(4);
        gallery
            .items()
            .add_native(Some(Control::boxed(element.clone())));
        let window = mount(gallery.clone().upcast(), runtime);
        gallery
            .items()
            .add_native(Some(Rc::new("Text item".to_owned())));
        gallery.set_item_template(None);
        settle();
        fn has_ancestor<T: ObjectType>(element: &Control) -> bool {
            let mut current = element
                .visual_parent()
                .or_else(|| element.parent().and_then(|parent| parent.cast::<Visual>()));
            let mut visited: Vec<Ref<Visual>> = Vec::new();
            while let Some(visual) = current {
                if visual.cast::<T>().is_some() {
                    return true;
                }
                assert!(
                    visited.iter().all(|old| !old.ptr_eq(&visual)),
                    "Repeated native ancestor {} in {:?}",
                    visual.get_type().name(),
                    visited
                        .iter()
                        .map(|old| old.get_type().name())
                        .collect::<Vec<_>>()
                );
                visited.push(visual.clone());
                current = visual
                    .visual_parent()
                    .or_else(|| visual.parent().and_then(|parent| parent.cast::<Visual>()));
            }
            false
        }
        assert!(
            has_ancestor::<Viewbox>(&element) && has_ancestor::<RibbonGallery>(&element),
            "Original element must still belong to its gallery Viewbox"
        );
        finish(&window);
    });
}

#[test]
fn plain_character_glyph_icons_render_as_text() {
    check(|runtime| {
        let presenter = RibbonIconPresenter::new();
        presenter.set_icon(Some(Rc::new(RibbonIcon::from("%"))));
        let window = mount(presenter.clone().upcast(), runtime);
        assert_eq!(
            presenter
                .children()
                .get(0)
                .cast::<TextBlock>()
                .unwrap()
                .text()
                .as_deref(),
            Some("%")
        );
        presenter.set_icon(Some(Rc::new(RibbonIcon::from("\u{e8c6}"))));
        settle();
        assert!(presenter.children().get(0).cast::<FontIcon>().is_some());
        finish(&window);
    });
}

#[test]
fn catalog_command_does_not_replace_user_command() {
    check(|runtime| {
        let ribbon = Ribbon::new();
        let tab = RibbonTab::new();
        let group = RibbonGroup::new();
        let button = RibbonButton::new();
        button.set_id(Some("copy".into()));
        button.set_label(Some("Copy".into()));
        button.set_command_id(Some("copy".into()));
        group.items().add(Some(button.clone().upcast()));
        tab.groups().add(Some(group));
        ribbon.tabs().add(Some(tab));
        let catalog = RibbonCommandCatalog::new();
        catalog.register_action(
            Some("copy".into()),
            Some("Copy".into()),
            Rc::new(|_| {}),
            None,
            RibbonCommandMetadata::default(),
        );
        ribbon.set_command_catalog(Some(catalog));
        let own = ferroribbon::commands::adapt_core_command(RibbonRelayCommand::new(|| {}));
        button.set_command(Some(own.clone()));
        let window = mount(ribbon.clone().upcast(), runtime);
        assert!(Rc::ptr_eq(&button.command().unwrap(), &own));
        button.set_command(None);
        button.set_command_id(None);
        button.set_command_id(Some("copy".into()));
        assert!(button.command().is_some());
        button.set_command_id(None);
        assert!(button.command().is_none());
        finish(&window);
    });
}

#[test]
fn screen_tips_are_not_shared_between_item_and_copy() {
    fn tip(button: &RibbonButton) -> Ref<RibbonScreenTip> {
        let value = button.get_value(ToolTip::tip_property()).unwrap();
        let tooltip = ValueTypes::as_object(&*value)
            .unwrap()
            .cast::<ToolTip>()
            .unwrap();
        ValueTypes::as_object(&*tooltip.content().unwrap())
            .unwrap()
            .cast::<RibbonScreenTip>()
            .unwrap()
    }
    check(|runtime| {
        let button = RibbonButton::new();
        button.set_label(Some("Cut".into()));
        let description = RibbonScreenTip::new();
        description.set_description(Some("Cut it".into()));
        description.set_disabled_reason(Some("Nothing selected".into()));
        button.set_screen_tip(Some(Control::boxed(description)));
        let copy = item(button.clone().upcast())
            .create_linked_copy()
            .unwrap()
            .cast::<RibbonButton>()
            .unwrap();
        let panel = StackPanel::new();
        panel.children().add(button.clone().upcast::<Control>());
        panel.children().add(copy.clone().upcast::<Control>());
        let window = mount(panel.upcast(), runtime);
        let source_tip = tip(&button);
        let copy_tip = tip(&copy);
        assert!(!source_tip.ptr_eq(&copy_tip));
        assert!(source_tip.disabled_reason().is_none_or(|s| s.is_empty()));
        button.set_is_enabled(false);
        settle();
        assert_eq!(
            tip(&button).disabled_reason().as_deref(),
            Some("Nothing selected")
        );
        finish(&window);
    });
}

#[test]
fn linked_toggle_copy_keeps_radio_semantics() {
    check(|runtime| {
        let left = RibbonToggleButton::new();
        left.set_label(Some("Left".into()));
        left.set_group_name(Some("align".into()));
        left.set_is_checked(Some(true));
        let copy = item(left.clone().upcast())
            .create_linked_copy()
            .unwrap()
            .cast::<RibbonToggleButton>()
            .unwrap();
        let panel = StackPanel::new();
        panel.children().add(left.clone().upcast::<Control>());
        panel.children().add(copy.clone().upcast::<Control>());
        let window = mount(panel.upcast(), runtime);
        assert_eq!(copy.group_name().as_deref(), Some("align"));
        item(copy.upcast()).invoke();
        settle();
        assert_eq!(left.is_checked(), Some(true));
        finish(&window);
    });
}

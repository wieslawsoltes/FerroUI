//! Pinned ItemTests.cs scenarios using native compiled themes and public controls.
//! Copyright Wiesław Šoltés, MIT. Configured image-root migration is explicit below.
use crate::{
    controls::*,
    primitives::RibbonIconPresenter,
    runtime_sample::{self as sample, Mounted, RuntimeHost},
};
use ferroribbon_core::{
    commands::{RibbonCommandCatalog, RibbonRelayCommand},
    model::RibbonIcon,
};
use ferroui_base::{
    ObjectType, Ref, Visual,
    data::core::{ValueType, ValueTypes},
    interactivity::RoutedEventArgs,
    media::Color,
    platform::{AssetLoader, register_assets},
};
use ferroui_controls::{
    Application, Border, Button, Control, StackPanel, TextBlock, ToolTip, Viewbox,
};
use std::{
    cell::Cell,
    rc::Rc,
    time::{Duration, Instant},
};

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
fn pair(first: Ref<Control>, second: Ref<Control>) -> Ref<StackPanel> {
    let panel = StackPanel::new();
    panel.children().add(first);
    panel.children().add(second);
    panel
}
fn has_ancestor<T: ObjectType>(element: &Control) -> bool {
    let mut current = element
        .visual_parent()
        .or_else(|| element.parent().and_then(|p| p.cast::<Visual>()));
    let mut visited: Vec<Ref<Visual>> = Vec::new();
    while let Some(visual) = current {
        if visual.is::<T>() {
            return true;
        }
        assert!(
            visited.iter().all(|old| !old.ptr_eq(&visual)),
            "Repeated native ancestor before required target"
        );
        visited.push(visual.clone());
        current = visual
            .visual_parent()
            .or_else(|| visual.parent().and_then(|p| p.cast::<Visual>()));
    }
    false
}
#[test]
fn spinner_copy_executes_command_once_and_only_for_user_changes() {
    crate::group_tests::run(|| {
        let calls = Rc::new(Cell::new(0));
        let count = calls.clone();
        let spinner = RibbonSpinner::new();
        spinner.set_label(Some("Size".into()));
        spinner.set_minimum(0.);
        spinner.set_maximum(10.);
        spinner.set_command(Some(crate::commands::adapt_core_command(
            RibbonRelayCommand::new_with_parameter(move |_| count.set(count.get() + 1), None),
        )));
        let copy = item(spinner.clone().upcast())
            .create_linked_copy()
            .unwrap()
            .cast::<RibbonSpinner>()
            .unwrap();
        let host = RuntimeHost::new(
            pair(spinner.clone().upcast(), copy.clone().upcast()).upcast(),
            None,
        );
        spinner.set_numeric_value(4.);
        host.settle();
        assert_eq!(calls.get(), 0);
        assert_eq!(copy.value(), 4.);
        // Source reflects into Step(1). Invoke the actual native increment button
        // rather than adding reflection or exposing a private method publicly.
        let mut pending = vec![copy.clone().upcast::<Visual>()];
        let mut button = None;
        while let Some(v) = pending.pop() {
            if let Some(c) = v.cast::<Control>()
                && c.name().as_deref() == Some("PART_UpButton")
            {
                button = Some(c.cast::<Button>().unwrap());
                break;
            }
            pending.extend(v.visual_children().snapshot().iter().cloned());
        }
        button
            .unwrap()
            .raise_event(&RoutedEventArgs::with_event(Button::click_event()));
        host.settle();
        assert_eq!(calls.get(), 1);
        assert_eq!(spinner.value(), 5.);
    });
}
#[test]
fn spinner_coerces_bound_values_and_range_changes() {
    crate::group_tests::run(|| {
        let spinner = RibbonSpinner::new();
        spinner.set_minimum(0.);
        spinner.set_maximum(10.);
        let _host = RuntimeHost::new(spinner.clone().upcast(), None);
        spinner.set_value(RibbonSpinner::value_property(), 25.);
        assert_eq!(spinner.value(), 10.);
        spinner.set_maximum(5.);
        assert_eq!(spinner.value(), 5.);
    });
}
#[test]
fn slider_does_not_execute_command_for_programmatic_changes() {
    crate::group_tests::run(|| {
        let calls = Rc::new(Cell::new(0));
        let count = calls.clone();
        let slider = RibbonSlider::new();
        slider.set_numeric_value(30.);
        slider.set_command(Some(crate::commands::adapt_core_command(
            RibbonRelayCommand::new(move || count.set(count.get() + 1)),
        )));
        let host = RuntimeHost::new(slider.clone().upcast(), None);
        slider.set_numeric_value(60.);
        slider.set_maximum(50.);
        host.settle();
        assert_eq!(calls.get(), 0);
        assert_eq!(slider.value(), 50.);
    });
}
#[test]
fn non_split_color_picker_opens_palette_instead_of_applying() {
    crate::group_tests::run(|| {
        let calls = Rc::new(Cell::new(0));
        let count = calls.clone();
        let picker = RibbonColorPicker::new();
        picker.set_is_split(false);
        picker.set_label(Some("Fill".into()));
        picker.set_command(Some(crate::commands::adapt_core_command(
            RibbonRelayCommand::new(move || count.set(count.get() + 1)),
        )));
        let host = RuntimeHost::new(picker.clone().upcast(), None);
        item(picker.clone().upcast()).invoke();
        host.settle();
        assert_eq!(calls.get(), 0);
        assert!(picker.is_drop_down_open());
        picker.flyout().unwrap().hide();
        host.settle();
    });
}
#[test]
fn color_picker_palette_stays_open_with_a_selected_color() {
    crate::group_tests::run(|| {
        let picker = RibbonColorPicker::new();
        picker.set_label(Some("Font Color".into()));
        picker.set_selected_color(Some(Color::from_argb(255, 0xc0, 0, 0)));
        let host = RuntimeHost::new(picker.clone().upcast(), None);
        picker.open_drop_down();
        let end = Instant::now() + Duration::from_millis(600);
        while Instant::now() < end {
            host.settle();
            std::thread::sleep(Duration::from_millis(5));
        }
        host.settle();
        assert!(picker.is_drop_down_open());
        picker.close_drop_down();
        host.settle();
    });
}
#[test]
fn gallery_element_items_survive_rebuilds() {
    crate::group_tests::run(|| {
        let element = Border::new();
        element.set_width(20.);
        element.set_height(20.);
        let gallery = RibbonGallery::new();
        gallery.set_label(Some("Shapes".into()));
        gallery.set_max_columns(4);
        gallery
            .items()
            .add_native(Some(Control::boxed(element.clone())));
        let host = RuntimeHost::new(gallery.clone().upcast(), None);
        gallery
            .items()
            .add_native(Some(Rc::new("Text item".to_owned())));
        gallery.set_item_template(None);
        host.settle();
        assert!(has_ancestor::<Viewbox>(&element) && has_ancestor::<RibbonGallery>(&element));
    });
}
#[test]
fn icon_presenter_accepts_relative_image_paths_and_element_icons() {
    crate::group_tests::run(|| {
        // Native application-root configuration replaces the source implicit package root.
        register_assets(
            "ferroribbon_runtime_items",
            &[(
                "Assets/cut.png",
                include_bytes!("../../tests/fixtures/images/bitmap-3x2.png"),
            )],
        );
        AssetLoader::invalidate_assembly_cache("ferroribbon_runtime_items");
        let app = Application::current().unwrap();
        let key = RibbonIconPresenter::IMAGE_BASE_URI_RESOURCE;
        struct RestoreRoot(
            Ref<ferroui_base::controls::ResourceDictionary>,
            Option<Option<ferroui_base::BoxedValue>>,
        );
        impl Drop for RestoreRoot {
            fn drop(&mut self) {
                let key = RibbonIconPresenter::IMAGE_BASE_URI_RESOURCE;
                if let Some(value) = self.1.take() {
                    self.0.set(key, value);
                } else {
                    self.0.remove(&key.into());
                }
            }
        }
        let _restore = RestoreRoot(
            app.resources(),
            app.resources().try_get_resource(&key.into(), None),
        );
        app.resources().set(
            key,
            Some(Rc::new("ferres://ferroribbon_runtime_items/".to_owned())),
        );
        let presenter = RibbonIconPresenter::new();
        presenter.set_icon(Some(Rc::new("Assets/cut.png".to_owned())));
        let host = RuntimeHost::new(presenter.clone().upcast(), None);
        assert!(presenter.has_icon());
        let shared = Border::new();
        shared.set_width(10.);
        shared.set_height(10.);
        presenter.set_icon(Some(Control::boxed(shared.clone())));
        presenter.set_icon_size(24.);
        host.settle();
        assert!(has_ancestor::<RibbonIconPresenter>(&shared));
        drop(host);
    });
}
#[test]
fn plain_character_glyph_icons_render_as_text() {
    crate::group_tests::run(|| {
        let presenter = RibbonIconPresenter::new();
        presenter.set_icon(Some(Rc::new(RibbonIcon::glyph(Some("%".into())))));
        let host = RuntimeHost::new(presenter.clone().upcast(), None);
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
        presenter.set_icon(Some(Rc::new(RibbonIcon::glyph(Some("".into())))));
        host.settle();
        assert!(presenter.children().get(0).is::<crate::compat::FontIcon>());
    });
}
#[test]
fn control_command_can_execute_combines_with_is_enabled() {
    crate::group_tests::run(|| {
        let allowed = Rc::new(Cell::new(false));
        let value = allowed.clone();
        let command =
            RibbonRelayCommand::new_with_can_execute(|| {}, Some(Rc::new(move || value.get())));
        let native = crate::commands::adapt_core_command(command.clone());
        let split = RibbonSplitButton::new();
        split.set_label(Some("Paste".into()));
        split.set_command(Some(native.clone()));
        let _host = RuntimeHost::new(split.clone().upcast(), None);
        assert!(!split.is_enabled());
        allowed.set(true);
        command.notify_can_execute_changed();
        assert!(split.is_enabled());
        allowed.set(false);
        command.notify_can_execute_changed();
        split.set_command(None);
        assert!(split.is_enabled());
        split.set_is_enabled(false);
        split.set_command(Some(native));
        allowed.set(true);
        command.notify_can_execute_changed();
        assert!(!split.is_enabled());
    });
}
#[test]
fn catalog_command_does_not_replace_user_command() {
    crate::group_tests::run(|| {
        let ribbon = sample::create();
        let catalog = RibbonCommandCatalog::new();
        catalog.register_action(
            Some("copy".into()),
            Some("Copy".into()),
            Rc::new(|_| {}),
            None,
            Default::default(),
        );
        ribbon.set_command_catalog(Some(catalog));
        let own = crate::commands::adapt_core_command(RibbonRelayCommand::new(|| {}));
        let button = sample::item::<RibbonButton>(&ribbon, "copy");
        button.set_command(Some(own.clone()));
        let _m = Mounted::with_ribbon(ribbon, 1400.);
        assert!(Rc::ptr_eq(&button.command().unwrap(), &own));
        button.set_command(None);
        button.set_command_id(None);
        button.set_command_id(Some("copy".into()));
        assert!(button.command().is_some());
        button.set_command_id(None);
        assert!(button.command().is_none());
    });
}
fn tip(button: &RibbonButton) -> Ref<RibbonScreenTip> {
    let tooltip = ValueTypes::as_object(&*button.get_value(ToolTip::tip_property()).unwrap())
        .unwrap()
        .cast::<ToolTip>()
        .unwrap();
    ValueTypes::as_object(&*tooltip.content().unwrap())
        .unwrap()
        .cast::<RibbonScreenTip>()
        .unwrap()
}
#[test]
fn screen_tips_are_not_shared_between_item_and_copy() {
    crate::group_tests::run(|| {
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
        let host = RuntimeHost::new(
            pair(button.clone().upcast(), copy.clone().upcast()).upcast(),
            None,
        );
        let source_tip = tip(&button);
        let copy_tip = tip(&copy);
        assert!(!source_tip.ptr_eq(&copy_tip));
        assert!(source_tip.disabled_reason().is_none_or(|s| s.is_empty()));
        button.set_is_enabled(false);
        host.settle();
        assert_eq!(
            tip(&button).disabled_reason().as_deref(),
            Some("Nothing selected")
        );
    });
}
#[test]
fn segmented_control_executes_only_for_user_selection() {
    crate::group_tests::run(|| {
        let calls = Rc::new(Cell::new(0));
        let count = calls.clone();
        let segmented = RibbonSegmentedControl::new();
        segmented.set_command(Some(crate::commands::adapt_core_command(
            RibbonRelayCommand::new(move || count.set(count.get() + 1)),
        )));
        for label in ["A", "B"] {
            let segment = RibbonSegment::new();
            segment.set_label(Some(label.into()));
            segmented.segments().add(Some(segment));
        }
        let _host = RuntimeHost::new(segmented.clone().upcast(), None);
        segmented.set_selected_index(1);
        segmented.set_selected_index(7);
        assert_eq!(calls.get(), 0);
        item(segmented.upcast()).invoke();
        assert_eq!(calls.get(), 1);
    });
}
#[test]
fn linked_toggle_copy_keeps_radio_semantics() {
    crate::group_tests::run(|| {
        let left = RibbonToggleButton::new();
        left.set_label(Some("Left".into()));
        left.set_group_name(Some("align".into()));
        left.set_is_checked(Some(true));
        let copy = item(left.clone().upcast())
            .create_linked_copy()
            .unwrap()
            .cast::<RibbonToggleButton>()
            .unwrap();
        let host = RuntimeHost::new(
            pair(left.clone().upcast(), copy.clone().upcast()).upcast(),
            None,
        );
        assert_eq!(copy.group_name().as_deref(), Some("align"));
        item(copy.upcast()).invoke();
        host.settle();
        assert_eq!(left.is_checked(), Some(true));
    });
}

//! The compiled native field views, without contracts for private framework templates.
use super::*;
use ferroboard_core::{engine::LanguageModels, layouts::KeyboardResources};
use ferroui_base::{
    Rect, Ref, Size, Thickness, controls::NameScope, media::TextWrapping, threading::Dispatcher,
};
use ferroui_controls::{
    ComboBox, Control, Grid, ItemsSource, Slider, StackPanel, TextBlock, TextBox, ToggleSwitch,
    Window, automation::peers::ControlAutomationPeer,
};
use serde_json::Value;

fn caption(root: &Ref<Control>) -> Ref<TextBlock> {
    NameScope::get_name_scope(root)
        .unwrap()
        .find("Header")
        .unwrap()
        .cast::<TextBlock>()
        .unwrap()
}

#[test]
fn compiled_captions_wrap_reflow_and_keep_names_and_callbacks() {
    crate::tests::run(|| {
        let engine = KeyboardEngine::new_explicit(
            Rc::new(KeyboardSettings::default()),
            Rc::new(KeyboardResources::create_empty()),
            LanguageModels::new(),
        )
        .unwrap();
        let keyboard = OnScreenKeyboard::new(Some(engine.clone())).unwrap();
        let panel = SettingsPanel::new();
        panel.attach(keyboard.clone()).unwrap();
        let window = Window::new();
        window.set_width(600.0);
        window.set_height(1000.0);
        window.set_content(ferroui_base::metadata::into_markup_value(panel.clone()));
        window.show();
        Dispatcher::ui_thread().run_jobs(None);
        let descriptors: Vec<Value> = serde_json::from_str(include_str!(
            "../../tests/fixtures/sample-header-resources.json"
        ))
        .unwrap();
        let fields = panel.fields.borrow().clone();
        let mut wrapped = 0;
        for field in &fields {
            let descriptor = &descriptors[match field.model.kind() {
                FieldKind::Number(_) => 0,
                FieldKind::Boolean(_) => 2,
                _ => 1,
            }];
            let caption = caption(&field.root);
            let grid = field.root.clone().cast::<Grid>().unwrap();
            assert_eq!(grid.children().count(), 2);
            assert_eq!(grid.children().get(1), field.input);
            assert_eq!(caption.text().as_deref(), Some(field.model.label()));
            assert_eq!(caption.text_wrapping(), TextWrapping::Wrap);
            assert!(!caption.is_hit_test_visible());
            assert_eq!(
                caption.margin(),
                Thickness::new(0.0, 0.0, 0.0, descriptor["margin"][3].as_f64().unwrap())
            );
            assert_eq!(Grid::get_row(&field.input), 1);
            assert_eq!(
                ControlAutomationPeer::create_peer_for_element(&field.input).get_name(),
                field.model.label()
            );
            grid.set_use_layout_rounding(false);
            caption.set_use_layout_rounding(false);
            field.input.set_use_layout_rounding(false);
            let mut heights = Vec::new();
            for width in [360.0, 100.0, 360.0] {
                grid.measure(Size::new(width, f64::INFINITY));
                grid.arrange(Rect::new(0.0, 0.0, width, grid.desired_size().height));
                assert_eq!(
                    field.input.bounds().y,
                    caption.bounds().bottom() + caption.margin().bottom,
                    "{}",
                    field.model.label()
                );
                assert!(field.input.bounds().height > 0.0);
                heights.push(caption.bounds().height);
            }
            assert_eq!(heights[0], heights[2], "{} reflow", field.model.label());
            wrapped += usize::from(heights[1] > heights[0]);
        }
        assert_eq!(fields.len(), 43);
        assert!(wrapped > 0, "long captions wrap at narrow width");
        let old = fields
            .iter()
            .find(|f| f.model.label() == "Key height")
            .unwrap()
            .clone();
        drop(fields);
        panel.attach(keyboard.clone()).unwrap();
        assert_eq!(caption(&old.root).text().as_deref(), Some("Key height"));
        assert_eq!(
            ControlAutomationPeer::create_peer_for_element(&old.input).get_name(),
            "Key height"
        );
        old.input
            .clone()
            .cast::<Slider>()
            .unwrap()
            .set_range_value(73.0);
        assert_eq!(keyboard.settings().key_height(), 73.0);
        panel.release_native_ownership();
        window.close();
        Dispatcher::ui_thread().run_jobs(None);
        keyboard.release_native_subscriptions().unwrap();
        engine.dispose().unwrap();
    });
}

#[test]
fn captions_do_not_activate_inputs_and_native_pointer_handlers_work() {
    crate::tests::run(|| {
        use ferroui_base::{
            Point,
            input::{MouseButton, RawInputModifiers},
            metadata::into_markup_value,
        };
        use ferroui_headless::HeadlessWindowExtensions;
        use ferroui_markup_xaml::xaml_il::runtime::XamlIlRuntimeHelpers;
        let services = || Some(XamlIlRuntimeHelpers::create_root_service_provider_v3(None));
        let toggle_root =
            crate::compiled_settings_views::build_views_settings_fieldtoggle_xaml(services())
                .unwrap();
        let combo_root =
            crate::compiled_settings_views::build_views_settings_fieldcombo_xaml(services())
                .unwrap();
        let slider_root =
            crate::compiled_settings_views::build_views_settings_fieldslider_xaml(services())
                .unwrap();
        let toggle = toggle_root
            .children()
            .get(1)
            .cast::<ToggleSwitch>()
            .unwrap();
        let combo = combo_root.children().get(1).cast::<ComboBox>().unwrap();
        let slider = slider_root.children().get(1).cast::<Slider>().unwrap();
        for (root, label) in [
            (&toggle_root, "Feature"),
            (&combo_root, "Language"),
            (&slider_root, "Volume"),
        ] {
            caption(&root.clone().upcast()).set_text(Some(label));
        }
        toggle.set_is_checked(Some(false));
        combo.set_items_source(Some(ItemsSource::from_items([
            into_markup_value("One".to_string()),
            into_markup_value("Two".to_string()),
        ])));
        combo.set_selected_index(0);
        slider.set_range_value(30.0);
        let text = TextBox::new();
        let panel = StackPanel::new();
        panel.children().add(text.clone());
        for root in [&toggle_root, &combo_root, &slider_root] {
            panel.children().add(root.clone());
        }
        let window = Window::new();
        window.set_width(360.0);
        window.set_height(400.0);
        window.set_content(Some(Control::boxed(panel)));
        window.show();
        Dispatcher::ui_thread().run_jobs(None);
        let click = |control: &Control, x: f64| {
            let point = control
                .translate_point(Point::new(x, control.bounds().height / 2.0), &window)
                .unwrap();
            window.mouse_down(point, MouseButton::Left, RawInputModifiers::NONE);
            window.mouse_up(point, MouseButton::Left, RawInputModifiers::NONE);
            Dispatcher::ui_thread().run_jobs(None);
        };
        assert!(text.focus());
        for root in [&toggle_root, &combo_root, &slider_root] {
            click(&caption(&root.clone().upcast()), 5.0);
            assert!(text.is_focused());
        }
        assert_eq!(toggle.is_checked(), Some(false));
        assert!(!combo.is_drop_down_open());
        assert_eq!(slider.value(), 30.0);
        click(&toggle, 20.0);
        assert_eq!(toggle.is_checked(), Some(true));
        click(&combo, 20.0);
        assert!(combo.is_drop_down_open());
        combo.set_is_drop_down_open(false);
        Dispatcher::ui_thread().run_jobs(None);
        click(&slider, slider.bounds().width * 0.8);
        assert_ne!(slider.value(), 30.0);
        window.close();
        Dispatcher::ui_thread().run_jobs(None);
    });
}

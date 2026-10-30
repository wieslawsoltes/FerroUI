//! The original ColorPicker page's options, selection handler and preview.
//! These tests exercise the explicit native composition, not upstream visual parity.

use crate::{examples, tests::start};
use ferroui_base::input::{FocusChangedEventArgs, InputElement};
use ferroui_base::media::{Color, Colors};
use ferroui_base::Ref;
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::primitives::ToggleButton;
use ferroui_controls::shapes::Rectangle;
use ferroui_controls::{CheckBox, Control, RadioButtons, StackPanel, TextBlock, TextBox};
use ferroui_controls_color_picker::primitives::ColorSlider;
use ferroui_controls_color_picker::{ColorModel, ColorSpectrumShape, ColorView};

fn page() -> Ref<Control> {
    examples::create(examples::find("ColorPicker").unwrap()).unwrap()
}

fn option(root: &Ref<Control>, name: &str, value: bool) {
    root.get_control::<CheckBox>(name)
        .set_is_checked(Some(value));
}

fn assert_color(root: &Ref<Control>, expected: Color) {
    assert_eq!(
        root.get_control::<ColorView>("colorPicker").color(),
        expected
    );
    assert_eq!(
        root.get_control::<ColorView>("ColorInputs").color(),
        expected
    );
    assert_eq!(
        root.get_control::<ColorSlider>("AlphaColorSlider").color(),
        expected
    );
    assert_eq!(
        root.get_control::<Rectangle>("previewRect")
            .fill()
            .unwrap()
            .as_solid_color_brush()
            .unwrap()
            .color(),
        expected
    );
}

#[test]
fn color_picker_page_preserves_source_option_defaults_with_native_inline_views() {
    let _application = start();
    let root = page();
    let picker = root.get_control::<ColorView>("colorPicker");
    let inputs = root.get_control::<ColorView>("ColorInputs");
    assert_eq!(picker.color_spectrum_shape(), ColorSpectrumShape::Box);
    assert_eq!(picker.selected_index(), 0);
    assert_eq!(inputs.selected_index(), 2);
    assert!(picker.is_color_spectrum_slider_visible());
    assert!(inputs.is_component_text_input_visible());
    assert!(!inputs.is_color_model_visible());
    assert_eq!(inputs.color_model(), ColorModel::Rgba);
    assert_eq!(
        root.get_control::<RadioButtons>("ColorModelRadioButtons")
            .selected_index(),
        0
    );
    assert!(!inputs.is_hex_input_visible());
    assert!(root.get_control::<StackPanel>("HexInputPanel").is_visible());
    assert!(picker.is_alpha_enabled() && inputs.is_alpha_enabled());
    assert_eq!(
        root.get_control::<CheckBox>("alpha").is_checked(),
        Some(false)
    );
    assert!(!root.get_control::<ToggleButton>("MoreButton").is_visible());
    assert!(root
        .get_control::<StackPanel>("ColorTextEntry")
        .is_visible());
    for name in ["alphaSlider", "alphaTextInput"] {
        let option = root.get_control::<CheckBox>(name);
        assert_eq!(option.is_checked(), Some(true));
        assert!(!option.is_enabled());
    }
    for name in ["AlphaSliderPanel", "AlphaTextPanel"] {
        assert!(!root.get_control::<StackPanel>(name).is_visible());
    }
    assert_color(&root, Colors::WHITE);
}

#[test]
fn color_picker_page_maps_visibility_options_and_box_ring_handler() {
    let _application = start();
    let root = page();
    let picker = root.get_control::<ColorView>("colorPicker");
    let inputs = root.get_control::<ColorView>("ColorInputs");
    for value in [false, true] {
        option(&root, "colorSlider", value);
        option(&root, "colorChannelInput", value);
        option(&root, "hexInput", value);
        assert_eq!(picker.is_color_spectrum_slider_visible(), value);
        assert_eq!(inputs.is_component_text_input_visible(), value);
        assert_eq!(
            root.get_control::<RadioButtons>("ColorModelRadioButtons")
                .is_visible(),
            value
        );
        assert_eq!(inputs.is_visible(), value);
        assert_eq!(
            root.get_control::<StackPanel>("HexInputPanel").is_visible(),
            value
        );
    }
    let shapes = root.get_control::<RadioButtons>("ColorSpectrumShapeRadioButtons");
    shapes.set_selected_index(1);
    assert_eq!(picker.color_spectrum_shape(), ColorSpectrumShape::Ring);
    shapes.set_selected_index(0);
    assert_eq!(picker.color_spectrum_shape(), ColorSpectrumShape::Box);
    let models = root.get_control::<RadioButtons>("ColorModelRadioButtons");
    models.set_selected_index(1);
    assert_eq!(inputs.color_model(), ColorModel::Hsva);
    models.set_selected_index(0);
    assert_eq!(inputs.color_model(), ColorModel::Rgba);
    assert_color(&root, Colors::WHITE);
}

#[test]
fn color_picker_page_keeps_alpha_visibility_independent_and_more_expansion_state() {
    let _application = start();
    let root = page();
    let more = root.get_control::<ToggleButton>("MoreButton");
    let entry = root.get_control::<StackPanel>("ColorTextEntry");
    let slider = root.get_control::<StackPanel>("AlphaSliderPanel");
    let input = root.get_control::<StackPanel>("AlphaTextPanel");
    option(&root, "alpha", true);
    assert!(slider.is_visible() && input.is_visible());
    for name in ["alphaSlider", "alphaTextInput"] {
        assert!(root.get_control::<CheckBox>(name).is_enabled());
    }
    option(&root, "alphaSlider", false);
    assert!(!slider.is_visible() && input.is_visible());
    option(&root, "alphaSlider", true);
    option(&root, "alphaTextInput", false);
    assert!(slider.is_visible() && !input.is_visible());
    option(&root, "alphaTextInput", true);

    option(&root, "moreBtn", true);
    assert!(more.is_visible());
    assert!(!entry.is_visible());
    assert!(slider.is_visible());
    more.set_is_checked(Some(true));
    assert!(entry.is_visible());
    assert_eq!(
        AutomationProperties::get_name(&more),
        Some("Hide color text inputs".to_string())
    );
    option(&root, "moreBtn", false);
    assert!(!more.is_visible() && entry.is_visible());
    assert_eq!(more.is_checked(), Some(true));
    option(&root, "moreBtn", true);
    assert!(entry.is_visible());
    more.set_is_checked(Some(false));
    assert!(!entry.is_visible());
    option(&root, "moreBtn", false);
    assert!(entry.is_visible());
}

#[test]
fn color_picker_page_synchronizes_views_alpha_input_and_rectangle_preview() {
    let _application = start();
    let root = page();
    let picker = root.get_control::<ColorView>("colorPicker");
    let inputs = root.get_control::<ColorView>("ColorInputs");
    picker.set_color(Color::from_rgb(15, 80, 190));
    assert_color(&root, Color::from_rgb(15, 80, 190));
    inputs.set_color(Color::from_rgb(90, 120, 30));
    assert_color(&root, Color::from_rgb(90, 120, 30));
    option(&root, "alpha", true);
    let alpha = root.get_control::<TextBox>("AlphaInput");
    alpha.set_text(Some("50%"));
    assert_color(&root, Color::new(128, 90, 120, 30));
    assert_eq!(alpha.text().as_deref(), Some("50%"));
    root.get_control::<ColorSlider>("AlphaColorSlider")
        .set_range_value(0.0);
    assert_color(&root, Color::new(0, 90, 120, 30));
    assert_eq!(alpha.text().as_deref(), Some("0%"));
    alpha.set_text(Some("100"));
    assert_color(&root, Color::from_rgb(90, 120, 30));
    alpha.set_text(Some("25"));
    assert_color(&root, Color::new(64, 90, 120, 30));
    option(&root, "alpha", false);
    assert_color(&root, Color::new(64, 90, 120, 30));
    assert!(!root
        .get_control::<StackPanel>("AlphaSliderPanel")
        .is_visible());
    assert!(!root
        .get_control::<StackPanel>("AlphaTextPanel")
        .is_visible());
}

#[test]
fn color_picker_page_rejects_invalid_alpha_without_changing_selected_color() {
    let _application = start();
    let root = page();
    option(&root, "alpha", true);
    root.get_control::<ColorView>("colorPicker")
        .set_color(Color::from_rgb(15, 80, 190));
    let alpha = root.get_control::<TextBox>("AlphaInput");
    let error = root.get_control::<TextBlock>("AlphaInputError");
    for text in [
        "", "-1", "101", "NaN", "inf", "invalid", "50%%", "50.5%", "5e1%", "50 %",
    ] {
        alpha.set_text(Some(text));
        assert!(error.is_visible(), "{text:?}");
        assert_color(&root, Color::from_rgb(15, 80, 190));
    }
    alpha.set_text(Some(" 50%"));
    assert!(!error.is_visible());
    assert_color(&root, Color::new(128, 15, 80, 190));
    assert_eq!(alpha.text().as_deref(), Some(" 50%"));
}

#[test]
fn color_picker_page_preserves_alpha_when_disabled_and_switches_hex_representation() {
    let _application = start();
    let root = page();
    let picker = root.get_control::<ColorView>("colorPicker");
    let hex = root.get_control::<TextBox>("HexInput");
    let color = Color::new(64, 15, 80, 190);
    picker.set_color(color);
    assert_color(&root, color);
    assert_eq!(hex.text().as_deref(), Some("#0F50BE"));
    assert_eq!(hex.max_length(), 7);
    option(&root, "alpha", true);
    assert_color(&root, color);
    assert_eq!(hex.text().as_deref(), Some("#400F50BE"));
    assert_eq!(hex.max_length(), 9);
    option(&root, "alpha", false);
    assert_color(&root, color);
    assert_eq!(hex.text().as_deref(), Some("#0F50BE"));
    option(&root, "alpha", true);
    assert_eq!(hex.text().as_deref(), Some("#400F50BE"));
}

#[test]
fn color_picker_page_parses_live_argb_and_rgb_hex_without_css_shorthand() {
    let _application = start();
    let root = page();
    let hex = root.get_control::<TextBox>("HexInput");
    let error = root.get_control::<TextBlock>("HexInputError");
    hex.set_text(Some("123abc"));
    assert_color(&root, Color::from_rgb(0x12, 0x3a, 0xbc));
    assert_eq!(hex.text().as_deref(), Some("#123abc"));
    option(&root, "alpha", true);
    hex.set_text(Some("80123456"));
    assert_color(&root, Color::new(0x80, 0x12, 0x34, 0x56));
    hex.set_text(Some("#F"));
    assert_color(&root, Color::new(0, 0, 0, 15));
    assert_eq!(hex.text().as_deref(), Some("#F"));
    for text in ["#", "#G0", "##FF", "#++F", "#0x+F"] {
        hex.set_text(Some(text));
        assert!(error.is_visible());
        assert_color(&root, Color::new(0, 0, 0, 15));
    }
    option(&root, "alpha", false);
    hex.set_text(Some("#112233"));
    assert_color(&root, Color::from_rgb(0x11, 0x22, 0x33));
    assert!(!error.is_visible());
}

#[test]
fn color_picker_page_preserves_live_text_and_rolls_invalid_edits_back_on_focus_loss() {
    let _application = start();
    let root = page();
    option(&root, "alpha", true);
    let alpha = root.get_control::<TextBox>("AlphaInput");
    let hex = root.get_control::<TextBox>("HexInput");
    alpha.raise_event(&mut FocusChangedEventArgs::new(
        InputElement::got_focus_event(),
    ));
    alpha.set_text(Some("050"));
    assert_eq!(alpha.text().as_deref(), Some("050%"));
    assert_color(&root, Color::new(128, 255, 255, 255));
    alpha.set_text(Some("oops"));
    assert_eq!(alpha.text().as_deref(), Some("oops%"));
    alpha.raise_event(&mut FocusChangedEventArgs::new(
        InputElement::lost_focus_event(),
    ));
    assert_eq!(alpha.text().as_deref(), Some("100%"));
    assert_color(&root, Colors::WHITE);
    assert!(!root
        .get_control::<TextBlock>("AlphaInputError")
        .is_visible());

    hex.raise_event(&mut FocusChangedEventArgs::new(
        InputElement::got_focus_event(),
    ));
    hex.set_text(Some("#80402010"));
    assert_color(&root, Color::new(128, 64, 32, 16));
    hex.set_text(Some("#invalid"));
    hex.raise_event(&mut FocusChangedEventArgs::new(
        InputElement::lost_focus_event(),
    ));
    assert_color(&root, Colors::WHITE);
    assert_eq!(hex.text().as_deref(), Some("#FFFFFFFF"));
    assert!(!root.get_control::<TextBlock>("HexInputError").is_visible());
}

#[test]
fn color_picker_callbacks_do_not_retain_the_gallery_page() {
    let _application = start();
    let root = page();
    let picker = root.get_control::<ColorView>("colorPicker");
    let option = root.get_control::<CheckBox>("alpha");
    let alpha = root.get_control::<TextBox>("AlphaInput");
    let hex = root.get_control::<TextBox>("HexInput");
    let weak_root = root.downgrade();
    drop(root);
    assert!(weak_root.upgrade().is_none());
    // A retained child is allowed to change after removal of its page.
    picker.set_color(Colors::RED);
    option.set_is_checked(Some(true));
    alpha.set_text(Some("50"));
    hex.set_text(Some("#FF0000"));
    alpha.raise_event(&mut FocusChangedEventArgs::new(
        InputElement::got_focus_event(),
    ));
    alpha.raise_event(&mut FocusChangedEventArgs::new(
        InputElement::lost_focus_event(),
    ));
    hex.raise_event(&mut FocusChangedEventArgs::new(
        InputElement::got_focus_event(),
    ));
    hex.raise_event(&mut FocusChangedEventArgs::new(
        InputElement::lost_focus_event(),
    ));
}

#[cfg(feature = "theme")]
mod themed {
    use super::*;
    use ferroui_base::animation::{IClock, IGlobalClock, PlayState, TimeSpan};
    use ferroui_base::reactive::{IDisposable, IObservable, IObserver, LightweightSubject};
    use ferroui_base::styling::ThemeVariant;
    use ferroui_controls::shapes::Path;
    use ferroui_controls::testing::{TestServices, UnitTestApplication};
    use ferroui_controls::{PathIcon, TabItem, Window};
    use std::cell::Cell;
    use std::rc::Rc;

    // The real theme needs a clock for its transitions. Like its own test host,
    // this clock does not pulse; the header foregrounds have no transitions.
    struct ThemeClock {
        ticks: LightweightSubject<TimeSpan>,
        state: Cell<PlayState>,
    }

    impl IObservable<TimeSpan> for ThemeClock {
        fn subscribe(&self, observer: Rc<dyn IObserver<TimeSpan>>) -> Rc<dyn IDisposable> {
            self.ticks.subscribe(observer)
        }
    }

    impl IClock for ThemeClock {
        fn play_state(&self) -> PlayState {
            self.state.get()
        }

        fn set_play_state(&self, value: PlayState) {
            self.state.set(value);
        }
    }

    impl IGlobalClock for ThemeClock {}

    fn visible_headers(view: &ColorView) -> Vec<(Ref<TabItem>, Ref<PathIcon>)> {
        view.get_visual_descendants()
            .filter_map(|visual| visual.cast::<TabItem>())
            .filter(|tab| tab.is_effectively_visible())
            .map(|tab| {
                let icons: Vec<_> = tab
                    .get_visual_descendants()
                    .filter_map(|visual| visual.cast::<PathIcon>())
                    .filter(|icon| icon.is_effectively_visible())
                    .collect();
                assert_eq!(icons.len(), 1, "one actual icon in each visible tab header");
                (tab, icons.into_iter().next().unwrap())
            })
            .collect()
    }

    #[test]
    fn color_picker_tab_icons_follow_winui_theme_changes() {
        ferroui_markup_xaml::register_types();
        ferroui_themes_winui::register_types();
        let clock = Rc::new(ThemeClock {
            ticks: LightweightSubject::new(),
            state: Cell::new(PlayState::Run),
        });
        let services = TestServices::styled_window()
            .with_global_clock(clock)
            .with_theme(|| ferroui_themes_winui::WinUITheme::new().as_style());
        let _application = UnitTestApplication::start(services);
        ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::register();
        let root = page();
        let picker = root.get_control::<ColorView>("colorPicker");
        let inputs = root.get_control::<ColorView>("ColorInputs");

        // Realize a second native tab to exercise selected and unselected
        // header brushes through normal selection, without forced pseudoclasses.
        picker.set_is_color_components_visible(true);
        let window = Window::new();
        window.set_width(1280.0);
        window.set_height(900.0);
        window.set_requested_theme_variant(Some(ThemeVariant::light()));
        window.set_content(Some(Control::boxed(&root)));
        window.show();
        window.update_layout();
        let picker_headers = visible_headers(&picker);
        let input_headers = visible_headers(&inputs);
        assert_eq!(picker_headers.len(), 2);
        assert_eq!(input_headers.len(), 1);

        // Keep the same realized icons across switches, so replacing a view or
        // resolving a static brush once cannot make this regression pass.
        for (variant, component) in [
            (ThemeVariant::light(), 0),
            (ThemeVariant::dark(), 255),
            (ThemeVariant::light(), 0),
        ] {
            window.set_requested_theme_variant(Some(variant));
            for selected_index in [0, 2] {
                picker.set_selected_index(selected_index);
                window.update_layout();
                assert_eq!(
                    picker_headers.iter().filter(|(tab, _)| tab.is_selected()).count(),
                    1
                );
                assert!(input_headers[0].0.is_selected());
                for (tab, icon) in picker_headers.iter().chain(input_headers.iter()) {
                    assert!(icon.is_effectively_visible());
                    let expected = Color::new(
                        if tab.is_selected() { 255 } else { 0x99 },
                        component,
                        component,
                        component,
                    );
                    let foreground = icon.foreground().expect("the realized icon has a foreground");
                    assert_eq!(foreground.as_solid_color_brush().unwrap().color(), expected);
                    let paths: Vec<_> = icon
                        .get_visual_descendants()
                        .filter_map(|visual| visual.cast::<Path>())
                        .collect();
                    assert_eq!(paths.len(), 1, "the native icon template draws one path");
                    assert!(paths[0].data().is_some());
                    assert_eq!(
                        paths[0].fill().unwrap().as_solid_color_brush().unwrap().color(),
                        expected,
                        "the drawn path follows its icon, including the dark theme"
                    );
                }
            }
        }
        window.close();
    }
}

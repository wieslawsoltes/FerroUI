//! Not from upstream: the theme documents of the library load with the
//! Fluent and the Simple theme, and every control of the library is
//! presented by its control theme with the template parts it names.

use crate::primitives::{ColorPreviewer, ColorSlider, ColorSpectrum};
use crate::{register_types, ColorPicker, ColorView, ASSEMBLY};
use ferroui_base::animation::{IClock, IGlobalClock, PlayState, SignedDuration};
use ferroui_base::controls::NameScopeRef;
use ferroui_base::metadata::{from_markup_value, MarkupAttributeValue};
use ferroui_base::reactive::{IDisposable, IObservable, IObserver, LightweightSubject};
use ferroui_base::styling::{IStyle, Styles};
use ferroui_base::{Ref, TypeInfo};
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_controls::{Application, Control, TextBox, Window};
use ferroui_markup_xaml::{RuntimeXamlLoaderConfiguration, RuntimeXamlLoaderDocument};
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The global clock of the themed application: the transitions of the control themes need
/// one. It never pulses.
struct MockGlobalClock {
    subject: LightweightSubject<SignedDuration>,
    play_state: Cell<PlayState>,
}

impl IObservable<SignedDuration> for MockGlobalClock {
    fn subscribe(&self, observer: Rc<dyn IObserver<SignedDuration>>) -> Rc<dyn IDisposable> {
        self.subject.subscribe(observer)
    }
}

impl IClock for MockGlobalClock {
    fn play_state(&self) -> PlayState {
        self.play_state.get()
    }

    fn set_play_state(&self, value: PlayState) {
        self.play_state.set(value)
    }
}

impl IGlobalClock for MockGlobalClock {}

#[derive(Clone, Copy)]
enum Theme {
    Fluent,
    Simple,
}

/// Starts a unit test application with the base theme and adds the styles
/// of the library for that theme, included as an application includes them.
fn start_application(theme: Theme) -> UnitTestApplicationScope {
    register_types();
    let clock = Rc::new(MockGlobalClock { subject: LightweightSubject::new(), play_state: Cell::new(PlayState::Run) });
    let services = TestServices::styled_window().with_global_clock(clock);
    let services = match theme {
        Theme::Fluent => services.with_theme(|| ferroui_themes_fluent::FluentTheme::new().as_style()),
        Theme::Simple => services.with_theme(|| ferroui_themes_simple::SimpleTheme::new().as_style()),
    };
    let scope = UnitTestApplication::start(services);
    FerroRuntimeXamlLoader::register();

    let name = match theme {
        Theme::Fluent => "Fluent",
        Theme::Simple => "Simple",
    };
    let xaml = format!(
        "<Styles xmlns=\"https://github.com/ferroui\">\
           <StyleInclude Source=\"ferres://FerroUI.Controls.ColorPicker/Themes/{name}/{name}.xaml\" />\
         </Styles>"
    );
    let mut configuration = RuntimeXamlLoaderConfiguration::new();
    configuration.local_assembly = Some(&ASSEMBLY);
    let loaded = FerroRuntimeXamlLoader::load_document(RuntimeXamlLoaderDocument::new(&xaml), Some(configuration))
        .unwrap_or_else(|error| match error.inner_exception() {
            Some(inner) => panic!("{name}.xaml failed to load: {}\n ---> {inner}", error.message()),
            None => panic!("{name}.xaml failed to load: {}", error.message()),
        });
    let styles = from_markup_value::<Ref<Styles>>(&Some(loaded)).expect("a Styles");
    let style: Rc<dyn IStyle> = styles.into();
    Application::current().expect("the application").styles().add(style);

    scope
}

/// The template parts the class states (`[TemplatePart]`).
fn template_parts(type_: &'static TypeInfo) -> Vec<(&'static str, &'static str)> {
    let markup = type_.markup().expect("the markup metadata of the class");
    markup
        .attributes
        .iter()
        .filter(|attribute| attribute.name == "TemplatePart")
        .filter_map(|attribute| match (attribute.arguments.first(), attribute.arguments.get(1)) {
            (Some(MarkupAttributeValue::Str(name)), Some(MarkupAttributeValue::Type(part_type))) => {
                Some((*name, part_type().name()))
            }
            _ => None,
        })
        .collect()
}

/// Shows `control` in a window and checks that its template was applied
/// with every part the class names. Returns the name scope of the template.
fn present(control: Ref<Control>, type_: &'static TypeInfo) -> (Ref<Window>, NameScopeRef) {
    let templated = control.cast::<TemplatedControl>().expect("a templated control");
    let name_scope: Rc<RefCell<Option<NameScopeRef>>> = Rc::new(RefCell::new(None));
    let sink = name_scope.clone();
    let _subscription = templated.template_applied(move |_, e| *sink.borrow_mut() = Some(e.name_scope().clone()));

    let window = Window::new();
    window.set_content(Some(Control::boxed(&control)));
    window.show();

    let name = type_.name();
    assert!(templated.template().is_some(), "{name}: no template");
    let name_scope = name_scope.borrow().clone().unwrap_or_else(|| panic!("{name}: the template was not applied"));
    for (part, part_type) in template_parts(type_) {
        let found = name_scope.find(part).unwrap_or_else(|| panic!("{name}: the part {part} is not in the template"));
        // The part type is named by its handle type (`Ref<module::Name>`).
        let mut found_type = Some(found.get_type());
        while let Some(t) = found_type {
            if part_type.ends_with(&format!("Ref<{}::{}>", t.module_path(), t.name())) {
                break;
            }
            found_type = t.base_type();
        }
        assert!(found_type.is_some(), "{name}: the part {part} is a {}, not a {part_type}", found.get_type().name());
    }
    (window, name_scope)
}

fn check_controls(theme: Theme) {
    let _app = start_application(theme);

    let view = ColorView::new();
    view.set_color(ferroui_base::media::Colors::RED);
    let (_window, name_scope) = present(view.clone().upcast(), ColorView::TYPE);
    let hex = name_scope.find("PART_HexTextBox").and_then(|found| found.cast::<TextBox>()).expect("the hex text box");
    // The control theme sets the trailing (CSS) alpha position.
    assert_eq!(crate::AlphaComponentPosition::Trailing, view.hex_input_alpha_position());
    assert_eq!(Some("FF0000FF".to_string()), hex.text());
    // The control theme sets the Fluent palette, which fills the palette colors.
    assert_eq!(6, view.palette_column_count());
    assert_eq!(48, view.palette_colors().expect("the palette colors").count());

    let _ = present(ColorPicker::new().upcast(), ColorPicker::TYPE);
    let spectrum = ColorSpectrum::new();
    spectrum.set_width(64.0);
    spectrum.set_height(64.0);
    let (_spectrum_window, spectrum_parts) = present(spectrum.clone().upcast(), ColorSpectrum::TYPE);
    // The bitmaps are created by background jobs after the layout pass sized the spectrum.
    ferroui_base::threading::Dispatcher::current_dispatcher().run_jobs(None);
    let rectangle = spectrum_parts
        .find("PART_SpectrumRectangle")
        .and_then(|found| found.cast::<ferroui_controls::shapes::Rectangle>())
        .expect("the spectrum rectangle");
    assert!(rectangle.fill().is_some(), "the spectrum has no bitmap");
    assert_eq!(64.0, rectangle.width());
    let _ = present(ColorSlider::new().upcast(), ColorSlider::TYPE);
    let _ = present(ColorPreviewer::new().upcast(), ColorPreviewer::TYPE);
}

/// The inline presentation (#1266): the parts of WinUI's layout, the More button that shows
/// and hides the text inputs, the automation names the control sets, and editing by the
/// channel inputs, the hex input and the keyboard on the spectrum.
fn check_inline_color_picker(theme: Theme) {
    use crate::ColorPickerDisplayMode;
    use ferroui_base::input::{InputElement, Key, KeyEventArgs, KeyModifiers};
    use ferroui_base::media::{Color, Colors};
    use ferroui_base::utilities::Decimal;
    use ferroui_controls::automation::AutomationProperties;
    use ferroui_controls::primitives::ToggleButton;
    use ferroui_controls::{NumericUpDown, TextBlock};

    let _app = start_application(theme);
    let picker = ColorPicker::new();
    picker.set_display_mode(ColorPickerDisplayMode::Inline);
    picker.set_color(Colors::RED);
    let (_window, parts) = present(picker.clone().upcast(), ColorPicker::TYPE);
    let find = |name: &str| parts.find(name).unwrap_or_else(|| panic!("the inline part {name}"));
    for name in ["ColorSpectrum", "AlphaSlider", "PART_ThirdDimensionSlider", "ColorPreviewRectangle"] {
        find(name);
    }
    assert!(parts.find("PART_TabControl").is_none(), "the inline template has no tabs");

    let hex = find("PART_HexTextBox").cast::<TextBox>().expect("the hex text box");
    assert_eq!(Some("FF0000FF".to_string()), hex.text());
    let grid = find("PART_TextEntryGrid").cast::<Control>().expect("the text entry grid");
    let more = find("PART_MoreButton").cast::<ToggleButton>().expect("the More button");
    let label = find("PART_MoreButtonLabel").cast::<TextBlock>().expect("the More label");
    // The More button is hidden by default and the text inputs are shown.
    assert!(!more.is_visible());
    assert!(grid.is_visible());
    picker.set_is_more_button_visible(true);
    assert!(more.is_visible());
    assert!(!grid.is_visible(), "the More button hides the text inputs until it is checked");
    assert_eq!(Some("More".to_string()), label.text());
    more.set_is_checked(Some(true));
    assert!(grid.is_visible());
    assert_eq!(Some("Less".to_string()), label.text());
    assert_eq!(Some("Less".to_string()), AutomationProperties::get_name(&more));
    more.set_is_checked(Some(false));
    assert!(!grid.is_visible());
    picker.set_is_more_button_visible(false);
    assert!(grid.is_visible());

    // The channel inputs are named by the color model and edit the color.
    let red = find("PART_Component1TextInput").cast::<NumericUpDown>().expect("the first channel input");
    assert_eq!(Some("Red".to_string()), AutomationProperties::get_name(&red));
    assert_eq!(Some(Decimal::from(255)), red.value());
    let green = find("PART_Component2TextInput").cast::<NumericUpDown>().expect("the second channel input");
    green.set_numeric_value(Some(Decimal::from(128)));
    assert_eq!(Color::from_rgb(255, 128, 0), picker.color());
    assert_eq!(Some("FF8000FF".to_string()), hex.text());
    let third = find("PART_ThirdDimensionSlider").cast::<Control>().expect("the third dimension slider");
    picker.set_color_model(crate::ColorModel::Hsva);
    assert_eq!(Some("Hue".to_string()), AutomationProperties::get_name(&red));
    assert_eq!(Some("Value (Brightness)".to_string()), AutomationProperties::get_name(&third));
    picker.set_color_spectrum_components(crate::ColorSpectrumComponents::SaturationValue);
    assert_eq!(Some("Hue".to_string()), AutomationProperties::get_name(&third));
    picker.set_color_spectrum_components(crate::ColorSpectrumComponents::HueSaturation);

    // Alpha: the slider and the text input follow IsAlphaEnabled and their own visibility.
    let alpha_slider = find("AlphaSlider").cast::<Control>().expect("the alpha slider");
    let alpha_input = find("AlphaTextInput").cast::<Control>().expect("the alpha input");
    assert!(alpha_slider.is_visible() && alpha_input.is_visible());
    picker.set_is_alpha_slider_visible(false);
    assert!(!alpha_slider.is_visible() && alpha_input.is_visible());
    picker.set_is_alpha_text_input_visible(false);
    assert!(!alpha_input.is_visible());
    picker.set_is_alpha_slider_visible(true);
    picker.set_is_alpha_text_input_visible(true);
    picker.set_is_alpha_enabled(false);
    assert!(!alpha_slider.is_visible() && !alpha_input.is_visible());
    picker.set_is_alpha_enabled(true);

    // The hex input edits the color on Enter.
    hex.set_text(Some("#0000FF"));
    let mut enter = KeyEventArgs::new();
    enter.set_routed_event(Some(InputElement::key_down_event()));
    enter.key = Key::Enter;
    hex.raise_event(&enter);
    assert_eq!(Colors::BLUE, picker.color());

    // The keyboard moves the selection on the spectrum (the hue on the horizontal axis).
    let spectrum = find("ColorSpectrum").cast::<ColorSpectrum>().expect("the spectrum");
    let before = picker.hsv_color();
    let mut right = KeyEventArgs::new();
    right.set_routed_event(Some(InputElement::key_down_event()));
    right.key = Key::Right;
    right.key_modifiers = KeyModifiers::NONE;
    spectrum.raise_event(&right);
    assert_ne!(before.h, picker.hsv_color().h, "the spectrum's key handling edits the picker");

    // The drop down presentation is restored with the display mode.
    picker.set_display_mode(ColorPickerDisplayMode::DropDown);
    ferroui_base::threading::Dispatcher::current_dispatcher().run_jobs(None);
}

#[test]
fn fluent_inline_color_picker() {
    check_inline_color_picker(Theme::Fluent);
}

#[test]
fn simple_inline_color_picker() {
    check_inline_color_picker(Theme::Simple);
}

#[test]
fn fluent_styles_present_every_control() {
    check_controls(Theme::Fluent);
}

#[test]
fn simple_styles_present_every_control() {
    check_controls(Theme::Simple);
}

#[test]
fn every_theme_document_is_embedded() {
    for theme in ["Fluent", "Simple"] {
        for document in ["ColorPicker", "ColorPreviewer", "ColorSlider", "ColorSpectrum", "ColorView", theme] {
            let path = format!("/Themes/{theme}/{document}.xaml");
            assert!(crate::assets::asset(&path).is_some(), "{path}");
        }
    }
}

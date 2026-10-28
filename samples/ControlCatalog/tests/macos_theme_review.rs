//! Render the actual compiled macOS templates with Skia/HarfBuzz.
//! This is a self-consistency gate, not a comparison with native AppKit pixels.
//! FERROUI_MACOS_REVIEW_DIR optionally preserves PNGs and a TSV capture manifest.
use super::support::{catalog_services_with_clock, TestGlobalClock};
use ferroui_base::media::imaging::{PngBitmapEncoderOptions, RenderTargetBitmap};
use ferroui_base::metadata::from_markup_value;
use ferroui_base::styling::ThemeVariant;
use ferroui_base::threading::Dispatcher;
use ferroui_base::{PixelRect, PixelSize, Ref, Thickness, Vector};
use ferroui_controls::testing::UnitTestApplication;
use ferroui_controls::{Application, Border, Canvas, Control, TextBlock, Window};
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;
use ferroui_themes_macos::{ControlSize, MacOSTheme};
use std::fmt::Write;
use std::path::PathBuf;
use std::rc::Rc;

const CASES: &[(&str, &str)] = &[
    ("button", "<Button Content='Continue'/>"),
    ("default-button", "<Button Content='Save' IsDefault='True'/>"),
    ("toggle", "<ToggleButton Content='Option' IsChecked='True'/>"),
    ("checkbox", "<CheckBox Content='Enable' IsChecked='True'/>"),
    ("mixed-checkbox", "<CheckBox Content='Mixed' IsThreeState='True' IsChecked='{x:Null}'/>"),
    ("radio", "<RadioButton Content='Choice' IsChecked='True'/>"),
    ("switch", "<ToggleSwitch IsChecked='True'/>"),
    ("text", "<TextBox Text='Value' PlaceholderText='Placeholder'/>"),
    ("combo", "<ComboBox SelectedIndex='0'><ComboBoxItem Content='Choice'/><ComboBoxItem Content='Other'/></ComboBox>"),
    ("numeric", "<NumericUpDown Value='42'/>"),
    ("slider", "<Slider Minimum='0' Maximum='100' Value='40'/>"),
    ("progress", "<ProgressBar Value='40'/>"),
    ("indeterminate", "<ProgressBar IsIndeterminate='True'/>"),
    ("list-selection", "<ListBoxItem Content='Selected' IsSelected='True'/>"),
    ("tree-selection", "<TreeViewItem Header='Folder' IsSelected='True'/>"),
    ("combo-selection", "<ComboBoxItem Content='Selected' IsSelected='True'/>"),
    ("dropdown", "<DropDownButton Content='Actions'/>"),
    ("split-button", "<SplitButton Content='Run'/>"),
    ("disclosure", "<Expander Header='Details'/>"),
    ("date", "<CalendarDatePicker/>"),
];
const STATES: &[&str] = &["normal", "hover", "pressed", "disabled", "disabled+input"];
const CELL_WIDTH: i32 = 152;
const ROW_HEIGHT: i32 = 56;
const LABEL_WIDTH: i32 = 144;
const HEADER_HEIGHT: i32 = 40;
/// The two compared cells sit at different x positions, and the rasterizer anti-aliases the
/// edges of the same geometry slightly differently there (up to about 11 levels of a channel
/// at the edges of the switch knob). A state that leaks through the disabled state changes
/// fills and strokes by far more.
const AA_TOLERANCE: u8 = 24;

fn text(canvas: &Canvas, value: &str, x: f64, y: f64) {
    let label = TextBlock::new();
    label.set_text(Some(value));
    label.set_font_size(11.0);
    Canvas::set_left(&label, x);
    Canvas::set_top(&label, y);
    canvas.children().add(label.upcast::<Control>());
}

fn control(xaml: &str) -> Ref<Control> {
    let text = format!(
        "<Border xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>{xaml}</Border>"
    );
    let loaded = FerroRuntimeXamlLoader::load(&text, None, None, None, false).expect("review fixture loads");
    let wrapper = from_markup_value::<Ref<Border>>(&Some(loaded)).expect("fixture wrapper");
    let control = wrapper.child().expect("fixture control");
    wrapper.set_child(None::<Ref<Control>>);
    control
}

#[test]
fn rendered_state_matrix_and_disabled_precedence() {
    ferroui_themes_macos::register_types();
    let clock = Rc::new(TestGlobalClock::default());
    let _app = UnitTestApplication::start(catalog_services_with_clock(clock)
        .with_theme(|| MacOSTheme::new().as_style()));
    FerroRuntimeXamlLoader::register();
    let app = Application::current().unwrap();
    let style = app.styles().get(0);
    let theme = style.as_object().unwrap().downcast_ref::<MacOSTheme>().unwrap().to_ref();
    // Static appearance states must not depend on wall-clock animation progress.
    theme.set_reduce_motion(true);
    theme.set_reduce_transparency(true);
    let output = std::env::var_os("FERROUI_MACOS_REVIEW_DIR").map(PathBuf::from);
    if let Some(output) = &output { std::fs::create_dir_all(output).unwrap(); }
    let mut manifest = String::from("file\tappearance\tactive_preview\traster_scale\tlayout_scale\twidth\theight\tcontrol_size\tbackend\tos\n");
    let mut failures = Vec::new();
    let mut captures = 0;
    let mut comparisons = 0;
    for (appearance, variant) in [
        ("light", ThemeVariant::light()),
        ("dark", ThemeVariant::dark()),
        ("contrast", MacOSTheme::high_contrast()),
    ] {
        for active in [true, false] {
            theme.set_window_active(active);
            theme.set_control_size(ControlSize::Regular);
            let width = LABEL_WIDTH + CELL_WIDTH * STATES.len() as i32;
            let height = HEADER_HEIGHT + ROW_HEIGHT * CASES.len() as i32;
            let canvas = Canvas::new();
            canvas.set_width(width as f64);
            canvas.set_height(height as f64);
            for (column, state) in STATES.iter().enumerate() {
                text(&canvas, state, (LABEL_WIDTH + column as i32 * CELL_WIDTH + 8) as f64, 12.0);
            }
            for (row, (name, markup)) in CASES.iter().enumerate() {
                text(&canvas, name, 8.0, (HEADER_HEIGHT + row as i32 * ROW_HEIGHT + 16) as f64);
                for column in 0..STATES.len() {
                    let item = control(markup);
                    item.set_is_enabled(column < 3);
                    item.pseudo_classes().set(":pointerover", matches!(column, 1 | 2 | 4));
                    item.pseudo_classes().set(":pressed", matches!(column, 2 | 4));
                    let cell = Border::new();
                    cell.set_width(CELL_WIDTH as f64);
                    cell.set_height(ROW_HEIGHT as f64);
                    cell.set_padding(Thickness::new(8.0, 8.0, 8.0, 8.0));
                    cell.set_child(&item);
                    Canvas::set_left(&cell, (LABEL_WIDTH + column as i32 * CELL_WIDTH) as f64);
                    Canvas::set_top(&cell, (HEADER_HEIGHT + row as i32 * ROW_HEIGHT) as f64);
                    canvas.children().add(cell.upcast::<Control>());
                }
            }
            let window = Window::new();
            window.set_width(width as f64);
            window.set_height(height as f64);
            window.set_requested_theme_variant(Some(variant.clone()));
            let background = Border::new();
            background.set_width(width as f64);
            background.set_height(height as f64);
            background.set_child(&canvas);
            window.set_content(Some(Control::boxed(&background)));
            window.show();
            Dispatcher::ui_thread().run_jobs(None);
            window.update_layout();
            background.set_background(window.background());
            for scale in [1, 2] {
                let pixel_width = width * scale;
                let pixel_height = height * scale;
                let bitmap = RenderTargetBitmap::with_dpi(PixelSize::new(pixel_width, pixel_height), Vector::new(96.0 * scale as f64, 96.0 * scale as f64));
                bitmap.render(&background.clone().upcast());
                let stride = pixel_width * 4;
                let mut pixels = vec![0u8; (stride * pixel_height) as usize];
                bitmap.copy_pixels(PixelRect::default(), &mut pixels, stride);
                assert!(pixels.chunks_exact(4).any(|p| p != &pixels[..4]), "a blank capture is not valid evidence");
                let filename = format!("{appearance}-{}-{scale}x.png", if active { "active-preview" } else { "inactive-preview" });
                if let Some(output) = &output {
                    bitmap.save_to_file(output.join(&filename).to_str().unwrap(), &PngBitmapEncoderOptions::default().into()).unwrap();
                }
                writeln!(manifest, "{filename}\t{appearance}\t{active}\t{scale}\t1\t{pixel_width}\t{pixel_height}\tregular\t{}\t{}", super::support::test_renderer().name(), std::env::consts::OS).unwrap();
                for (row, (name, _)) in CASES.iter().enumerate() {
                    let mut different = 0;
                    for y in (HEADER_HEIGHT + row as i32 * ROW_HEIGHT) * scale..(HEADER_HEIGHT + (row as i32 + 1) * ROW_HEIGHT) * scale {
                        let start = (y * stride + (LABEL_WIDTH + 3 * CELL_WIDTH) * scale * 4) as usize;
                        let other = start + (CELL_WIDTH * scale * 4) as usize;
                        let length = (CELL_WIDTH * scale * 4) as usize;
                        different += pixels[start..start + length].chunks_exact(4)
                            .zip(pixels[other..other + length].chunks_exact(4))
                            .filter(|(a, b)| a.iter().zip(b.iter()).any(|(x, y)| x.abs_diff(*y) > AA_TOLERANCE))
                            .count();
                    }
                    comparisons += 1;
                    if different != 0 {
                        failures.push(format!("{filename}/{name}: {different} pixels changed when disabled retained hover/press"));
                    }
                }
                captures += 1;
                bitmap.dispose();
            }
            window.close();
        }
    }
    if let Some(output) = &output {
        std::fs::write(output.join("captures.tsv"), manifest).unwrap();
        std::fs::write(output.join("failures.txt"), failures.join("\n")).unwrap();
    }
    println!("macOS render review: {captures} captures, {comparisons} disabled-precedence comparisons; reduced motion/transparency, logical layout scale 1; native fidelity NOT asserted");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

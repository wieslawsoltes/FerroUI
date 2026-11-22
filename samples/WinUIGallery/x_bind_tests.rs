//! `{x:Bind}` in the gallery on the compiled x:Bind core (#525, step X7, docs/porting/x-bind.md).
//!
//! The native pages whose source binds with x:Bind keep it (`Binding`, `Button`, `Canvas`,
//! `ComboBox`, `Grid`, `HyperlinkButton`, `InfoBar`, `Line`, `PipsPager`, `Popup`, `ProgressRing`,
//! `RatingControl`, `RepeatButton`, `RichTextBlock`, `ScrollViewer`, `Shape`, `Slider`, `SplitView`,
//! `StackPanel`, `TextBlock`, `TitleBar`, `ToggleButton`, `ToggleSwitch`, `Viewbox` and the window of
//! `TitleBarWindow.xaml`): the run-time loader compiles the paths of the named elements,
//! `examples::create` initializes the bindings once the page is wired, and the attach of a page
//! initializes them again after `StopTracking`. The WinUI preview of the catalog runs WinUI's own
//! x:Bind through the importer onto the same core, with no adapter.

use crate::{examples, tests::start, CATALOG};
use ferroui_base::Ref;
use ferroui_controls::primitives::ToggleButton;
use ferroui_controls::{CheckBox, Control, InfoBar, SplitView, TextBlock, TextBox, Window};
use ferroui_markup_xaml::markup_extensions::XBindings;

/// The native pages that keep the x:Bind of their source.
const X_BIND_PAGES: &[&str] = &[
    "Binding",
    "Button",
    "Canvas",
    "ComboBox",
    "Grid",
    "HyperlinkButton",
    "InfoBar",
    "Line",
    "PipsPager",
    "Popup",
    "ProgressRing",
    "RatingControl",
    "RepeatButton",
    "RichTextBlock",
    "ScrollViewer",
    "Shape",
    "Slider",
    "SplitView",
    "StackPanel",
    "TextBlock",
    "TitleBar",
    "ToggleButton",
    "ToggleSwitch",
    "Viewbox",
];

fn page(id: &str) -> Ref<Control> {
    examples::create(examples::find(id).unwrap()).unwrap_or_else(|error| panic!("{id}: {error}"))
}

#[test]
fn native_pages_compile_the_x_bind_of_their_source_and_initialize_it() {
    let _application = start();
    for id in X_BIND_PAGES {
        let adaptation = examples::find(id).unwrap();
        assert!(adaptation.xaml.contains("{x:Bind "), "{id} keeps the x:Bind of its source");
        let root = page(id);
        let bindings = XBindings::get_for_root(&root)
            .unwrap_or_else(|| panic!("{id}: the first x:Bind of the page creates its bindings"));
        assert!(bindings.is_initialized(), "{id}: the page is created with its bindings initialized");
        assert!(bindings.binding_count() > 0, "{id}");
        assert!(bindings.diagnostics().is_empty(), "{id}: {:?}", bindings.diagnostics());
    }
}

/// The Binding page: the source's OneWay and TwoWay x:Bind between its text boxes, and the x:Bind of
/// a text box's text through a converter.
#[test]
fn binding_page_x_bind_mirrors_the_text_boxes_and_converts_the_input() {
    let _application = start();
    let root = page("Binding");
    let text = |name: &str| root.get_control::<TextBox>(name).text().unwrap_or_default();
    root.get_control::<TextBox>("SourceTextBoxOneWay").set_text(Some("one way"));
    assert_eq!(text("TargetTextBoxOneWay"), "one way");
    root.get_control::<TextBox>("TargetTextBoxOneWay").set_text(Some("edited"));
    assert_eq!(text("SourceTextBoxOneWay"), "one way", "OneWay does not write back");
    root.get_control::<TextBox>("SourceTextBoxTwoWay").set_text(Some("two"));
    assert_eq!(text("TargetTextBoxTwoWay"), "two");
    root.get_control::<TextBox>("TargetTextBoxTwoWay").set_text(Some("way"));
    assert_eq!(text("SourceTextBoxTwoWay"), "way", "TwoWay writes back");
    let not_empty = root.get_control::<TextBlock>("NotEmptyText");
    assert!(!not_empty.is_visible());
    root.get_control::<TextBox>("InputTextBox").set_text(Some("x"));
    assert!(not_empty.is_visible());
}

/// SplitView: the TwoWay x:Bind of a nullable `IsChecked` with a `bool` target, read in both
/// directions.
#[test]
fn split_view_page_pane_follows_its_toggle_button() {
    let _application = start();
    let root = page("SplitView");
    let split = root.get_control::<SplitView>("splitView");
    assert!(split.is_pane_open());
    root.get_control::<ToggleButton>("togglePaneButton").set_is_checked(Some(false));
    assert!(!split.is_pane_open());
    root.get_control::<ToggleButton>("togglePaneButton").set_is_checked(Some(true));
    assert!(split.is_pane_open());
}

/// WinUI's `Loading`: bindings that were stopped read their sources again when the page is
/// attached to a window, and a TwoWay x:Bind writes back afterwards.
#[test]
fn a_page_whose_bindings_stopped_reads_its_sources_again_when_it_is_shown() {
    let _application = start();
    let root = page("InfoBar");
    let bindings = XBindings::get_for_root(&root).unwrap();
    let info = root.get_control::<InfoBar>("TestInfoBar1");
    let check = root.get_control::<CheckBox>("IsOpenCheckBox1");
    assert_eq!(check.is_checked(), Some(true));
    bindings.stop_tracking();
    info.set_is_open(false);
    assert_eq!(check.is_checked(), Some(true), "stopped bindings do not follow their sources");
    let window = Window::new();
    window.set_content(Some(Control::boxed(root.clone())));
    window.show();
    assert!(bindings.is_initialized(), "the attach of the page initializes its bindings");
    assert_eq!(check.is_checked(), Some(false));
    check.set_is_checked(Some(true));
    assert!(info.is_open());
    assert!(bindings.diagnostics().is_empty(), "{:?}", bindings.diagnostics());
    window.close();
}

/// After the Binding page's "Binding between controls" example, in WinUI markup: through the
/// importer onto the core, initialized when the preview is shown, OneWay and TwoWay with
/// `UpdateSourceTrigger=PropertyChanged`.
const BETWEEN_CONTROLS: &str = r#"<StackPanel xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml" Spacing="8">
    <TextBox x:Name="SourceTextBoxOneWay" PlaceholderText="Enter text here" />
    <TextBox x:Name="TargetTextBoxOneWay" PlaceholderText="Mirrors above text"
             Text="{x:Bind SourceTextBoxOneWay.Text, Mode=OneWay}" />
    <TextBox x:Name="SourceTextBoxTwoWay" PlaceholderText="Enter text here" />
    <TextBox x:Name="TargetTextBoxTwoWay" PlaceholderText="Mirrors and edits above text"
             Text="{x:Bind SourceTextBoxTwoWay.Text, Mode=TwoWay, UpdateSourceTrigger=PropertyChanged}" />
</StackPanel>"#;

#[test]
fn winui_preview_binds_between_controls_one_way_and_two_way_on_the_core() {
    let _application = start();
    let root = crate::gallery::load_winui_preview(BETWEEN_CONTROLS).unwrap_or_else(|error| panic!("{error}"));
    let bindings = XBindings::get_for_root(&root).expect("the bindings of the preview's root");
    assert!(!bindings.is_initialized());
    let window = Window::new();
    window.set_content(Some(Control::boxed(root.clone())));
    window.show();
    assert!(bindings.is_initialized(), "the preview's root is initialized when it is shown");
    let text_box = |name: &str| root.get_control::<TextBox>(name);
    let (one_way_source, one_way_target) = (text_box("SourceTextBoxOneWay"), text_box("TargetTextBoxOneWay"));
    let (two_way_source, two_way_target) = (text_box("SourceTextBoxTwoWay"), text_box("TargetTextBoxTwoWay"));
    one_way_source.set_text(Some("typed"));
    assert_eq!(one_way_target.text().as_deref(), Some("typed"));
    two_way_source.set_text(Some("from the source"));
    assert_eq!(two_way_target.text().as_deref(), Some("from the source"));
    // PropertyChanged: an edit of the target is written back at once.
    two_way_target.set_current_value(TextBox::text_property(), Some("from the target".to_string()));
    assert_eq!(two_way_source.text().as_deref(), Some("from the target"));
    assert!(bindings.diagnostics().is_empty(), "{:?}", bindings.diagnostics());
    window.close();
}

/// Every catalog sample whose markup uses x:Bind goes through the WinUI importer to the core
/// (the adapter and its lifecycle diagnostic `WXAML2004` were removed in X6): it loads, or fails
/// with a diagnostic of the importer or the compiler (most name code-behind members a wrapped
/// fragment does not have), and the bindings of one that loads can be initialized.
#[test]
fn winui_preview_of_every_x_bind_sample_reaches_the_core() {
    let _application = start();
    let mut samples = 0;
    for entry in CATALOG {
        for sample in entry.samples.iter().filter(|sample| sample.xaml.contains("{x:Bind")) {
            samples += 1;
            let wrapped = crate::gallery::wrap_winui_sample(sample);
            match crate::gallery::load_winui_preview(&wrapped) {
                Ok(root) => {
                    if let Some(bindings) = XBindings::get_for_root(&root) {
                        // A path that cannot be read is a diagnostic of the bindings.
                        let _ = bindings.initialize();
                        assert!(bindings.is_initialized(), "{} / {}", entry.id, sample.header);
                    }
                }
                Err(error) => {
                    let message = error.to_string();
                    assert!(!message.contains("WXAML2004"), "{} / {}: {message}", entry.id, sample.header);
                }
            }
        }
    }
    assert!(samples > 0, "the catalog has samples with x:Bind");
}

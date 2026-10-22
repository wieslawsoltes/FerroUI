//! Reviewed native adaptations. Original snippets remain in the catalog; these
//! markup documents state the replacement API and their behavior in Rust.

use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::Color;
use ferroui_base::{ObjectType, Ref, Thickness};
use ferroui_controls::primitives::ToggleButton;
use ferroui_controls::{
    Border, Button, ComboBox, Control, NumberBox, RadioButtons, Slider, SplitButton,
    TextBlock, TextBox, ToggleSplitButton,
};
use ferroui_markup_xaml::XamlLoadException;
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;
use std::cell::Cell;
use std::rc::Rc;

#[derive(Clone, Copy, Debug)]
pub struct PageAdaptation {
    pub page_id: &'static str,
    pub xaml: &'static str,
    pub notes: &'static str,
}

macro_rules! page {
    ($id:literal, $notes:literal) => {
        PageAdaptation {
            page_id: $id,
            xaml: include_str!(concat!("Pages/", $id, ".xaml")),
            notes: $notes,
        }
    };
}

pub static ADAPTATIONS: &[PageAdaptation] = &[
    page!("CompactSizing", "The source standard/compact page navigation copies all four text/password values and the selected date. The existing native compact dictionary is scoped to each compact page; global theme density remains unchanged. Header and PasswordBox are explicit compositions/adaptations; full WinUI control contracts and input/visual acceptance remain pending."),
    page!("InfoBadge", "All four source compositions use native InfoBadge: NavigationView item badges, severity styles, an embedded button badge and dynamic NumberBox values. Visibility and pane options retain the original handlers. Exact upstream rendering and host accessibility acceptance remain pending."),
    page!("NavigationView", "All eight source compositions use native NavigationView, items, hierarchy, footer, settings and Frame navigation. Model templates and C# handlers are explicitly adapted to Rust. Top overflow, selectors, transition effects and platform visual/accessibility acceptance remain pending."),
    page!("ProgressRing", "Both source examples use the native ProgressRing, active state, background options and original NumberBox empty-value recovery. Native arc animation is an explicit adaptation; exact Lottie motion, custom animated sources and platform accessibility acceptance remain pending."),
    page!("RatingControl", "Both source examples use native RatingControl input, caption changes, clearing/read-only options and half-step placeholder binding. Programmatic updates retain the original caption; user ValueChanged selects Your rating. Exact source animation and host accessibility acceptance remain pending."),
    page!("InfoBar", "All three original compositions use the native InfoBar with severity, short/long messages, optional button/hyperlink and visibility/closability options. x:Bind is explicitly adapted to ordinary TwoWay Binding; final layout, icons and live-region acceptance remain pending."),
    page!("Popup", "The source open/close handlers and offset/light-dismiss options use the native Popup. Placement is explicitly relative to its button; WinUI absolute-placement, acrylic and screen-edge parity remain pending."),
    page!("Clipboard", "Text copy/paste, available formats and clear use the host clipboard service asynchronously. Bitmap/file operations, history/roaming, content-change notifications and original accessibility announcements remain pending."),
    page!("PullToRefresh", "The source RefreshContainer request/deferral behavior inserts NewControl on the UI dispatcher. ListView is explicitly adapted to ListBox; the custom visualizer and full gesture/animation parity remain pending."),
    page!("PageTransition", "The original alternating sample-page layouts use the native Frame/FramePage types, back/forward journals and Rust handlers. Navigation transition effects remain pending."),
    page!("AppBarButton", "The path-icon sample uses the shared CommandBarButton with native click output. Other icon types, accelerators and flyout examples remain pending."),
    page!("AppBarToggleButton", "The three-state path-icon sample uses CommandBarToggleButton; checked, unchecked and indeterminate state remain observable."),
    page!("AppBarSeparator", "The source command groups use CommandBarSeparator; Symbol icons remain pending in this adaptation."),
    page!("CommandBar", "The source primary/secondary commands use the shared CommandBar family, observable open state and an explicit HotKey adaptation. Dynamic symbol conversion and the full option set remain pending."),
    page!("AccessibilityColorContrast", "The original luminance/contrast calculation and pass thresholds run in Rust. Opaque RGB text inputs replace the Gallery's custom inline color pickers; those picker controls remain pending."),
    page!("AutoSuggestBox", "Both examples use the native AutoSuggestBox: 97 source cat names, user-only token filtering, chosen-item output and control search/query with original title/subtitle and embedded bitmap details. Catalog titles are explicit item values; CurrentCulture matching and full input/accessibility parity remain pending."),
    page!("NumberBox", "The native NumberBox preserves spin-button, NaN and expression behavior using the original snippets. Locale-specific upstream number formatting remains pending."),
    page!("PipsPager", "The existing PipsPager provides page counts, selection and previous/next controls; wrapping remains explicit."),
    page!("TabView", "Native TabControl displays tab content; WinUI add/close/drag contracts remain pending."),
    page!("Pivot", "Native TabControl adapts the basic Pivot sample; Pivot-specific input and animation remain pending."),
    page!("RadialGradientBrush", "The shared RadialGradientBrush renders the source gradient with explicit relative radii."),
    page!("Geometry", "Shared Path geometry renders a triangle and a cubic curve; the upstream geometry editor remains pending."),
    page!("IconElement", "The original path, font and symbol icon examples use native PathIcon and the native FontIcon/SymbolIcon types. Bitmap/SVG variants and unavailable host icon fonts remain pending."),
    page!("Color", "Native brushes display the color swatches; the full upstream color tools are not claimed."),
    page!("Spacing", "Native element binding changes StackPanel spacing interactively."),
    page!("XamlStyles", "The explicit WinUI style example is adapted to an opt-in ControlTheme; implicit WinUI style semantics are not changed globally."),
    page!("Templates", "A shared ControlTemplate presents button content through TemplateBinding."),
    page!("CustomUserControls", "A native UserControl hosts the counter behavior; the original additional custom controls remain pending."),
    page!("AccessibilityScreenReader", "Native automation names and decorative accessibility views are assigned; screen-reader execution remains unverified."),
    page!("Button", "Text, graphical content, disabled state and click output use native Button; custom styles remain tracked separately."),
    page!("RepeatButton", "Native RepeatButton repeats the same Rust click handler; holding behavior belongs to FerroUI input."),
    page!("ToggleButton", "Native ToggleButton checked state and output; the original handler is ported to Rust."),
    page!("ToggleSwitch", "IsOn maps to IsChecked on the existing ToggleSwitch; Header is composed with a label and an automation name, and states remain observable."),
    page!("CheckBox", "All three source compositions use native CheckBox with original two/three-state output and select-all synchronization. Keyboard, accessibility and exact rendering acceptance remain pending."),
    page!("RadioButton", "Both original groups use native RadioButtons with the existing native RadioButton, source labels and Rust selection/color handlers. Focus, layout and screen-reader parity still require host verification."),
    page!("TextBox", "Text, placeholder, read-only and multiline examples use existing TextBox properties; Header is composed explicitly."),
    page!("PasswordBox", "The three source examples and original validation/reveal handlers use TextBox.PasswordChar and RevealPassword with composed Header. A distinct PasswordBox API and full input/accessibility parity remain pending."),
    page!("TextBlock", "Text content, wrapping, typography and inline text use native TextBlock."),
    page!("Slider", "WinUI StepFrequency becomes TickFrequency with IsSnapToTickEnabled. The sample output observes Value."),
    page!("ProgressBar", "Both determinate and indeterminate ProgressBar modes use the shared control."),
    page!("ComboBox", "Inline color choices and editable choices use native ComboBox; Header is composed explicitly."),
    page!("Border", "The source Border/TextBlock composition uses the shared Border control."),
    page!("Canvas", "Canvas coordinates use shared attached properties; ZIndex is the shared visual property."),
    page!("Grid", "The source grid uses shared row/column definitions and spacing."),
    page!("StackPanel", "Horizontal/vertical orientation and spacing use the shared StackPanel."),
    page!("RelativePanel", "The source relative constraints use the shared RelativePanel and named controls."),
    page!("Expander", "Header, content, expanded state and alignment use the shared Expander."),
    page!("Viewbox", "The Viewbox scales native vector content; the upstream bitmap asset is a separate pending sample."),
    page!("WrapPanel", "The source wrapping layout uses shared WrapPanel; ItemsStretch and Padding require explicit owner adaptation."),
    page!("ScrollViewer", "Native ScrollViewer hosts oversized content and scroll bars."),
    page!("Flyout", "Native Button.Flyout hosts the confirmation content; the action uses a Rust click handler."),
    page!("DropDownButton", "MenuFlyout entries map to native MenuItem headers; flyout ownership remains native."),
    page!("SplitButton", "Native SplitButton and Flyout keep separate primary and secondary actions."),
    page!("ToggleSplitButton", "Native ToggleSplitButton owns its checked state and secondary flyout."),
    page!("ToolTip", "ToolTipService.ToolTip is explicitly mapped to the shared ToolTip.Tip attached property."),
    page!("HyperlinkButton", "Native HyperlinkButton is used; external navigation remains explicit through NavigateUri."),
    page!("CalendarDatePicker", "Native CalendarDatePicker; header is composed and placeholder uses the shared property."),
    page!("CalendarView", "CalendarView is adapted to the shared Calendar; WinUI-specific day item customization is pending."),
    page!("DatePicker", "Native DatePicker composes a label for WinUI Header."),
    page!("TimePicker", "Native TimePicker demonstrates 12/24-hour clocks and minute increments."),
    page!("Shape", "Shared Ellipse, Rectangle and Polygon render through the framework geometry backend."),
    page!("Line", "Shared Line and Polyline use native geometry and stroke properties."),
    page!("Typography", "The WinUI type ramp is expressed with shared TextBlock font sizes and weights."),
    page!("ListView", "ListView is explicitly adapted to ListBox for selection; WinUI container/event parity remains pending."),
    page!("TreeView", "Native TreeViewItem hierarchy demonstrates expansion and selection; WinUI TreeViewNode parity is pending."),
    page!("MenuBar", "Native Menu with MenuItems represents WinUI MenuBar. Shortcuts remain shared key gestures."),
    page!("MenuFlyout", "Native MenuFlyout/MenuItem and separators preserve popup ownership and selection behavior."),
    page!("SplitView", "Native SplitView uses explicit IsPaneOpen binding to a ToggleButton; no x:Bind rewrite is assumed."),
    page!("Binding", "Element-name binding is explicitly ported to a normal shared Binding with an explicit mode."),
    page!("XamlResources", "Static and dynamic resource examples use the shared resource dictionary; broader WinUI lookup rules remain tracked."),
    page!("AccessibilityKeyboard", "Tab order examples use native enabled/focusable controls and explicit TabIndex."),
];

pub fn find(page_id: &str) -> Option<&'static PageAdaptation> {
    ADAPTATIONS.iter().find(|sample| sample.page_id == page_id)
}

/// Creates real framework controls from the reviewed markup, then wires each
/// behavior in Rust. No alternate drawing or input system is used.
pub fn create(sample: &PageAdaptation) -> Result<Ref<Control>, XamlLoadException> {
    crate::assets::register();
    let root = FerroRuntimeXamlLoader::parse_as::<Ref<Control>>(sample.xaml, None)?;
    crate::frame_sample::wire(&root)?;
    crate::navigation_view_samples::wire(&root)?;
    crate::sizing_sample::wire(&root)?;
    crate::platform_samples::wire(&root);
    crate::input_samples::wire(&root);
    crate::control_samples::wire(&root);
    crate::feedback_samples::wire(&root);
    wire(&root);
    Ok(root)
}

fn wire(root: &Ref<Control>) {
    wire_contrast(root);
    wire_radio_groups(root);
    let Some(output) = find_control::<TextBlock>(root, "Output") else {
        return;
    };
    if let Some(number) = find_control::<NumberBox>(root, "Number") {
        let weak_output = output.downgrade();
        let weak_number = number.downgrade();
        number.value_changed(move |_| {
            if let (Some(number), Some(output)) = (weak_number.upgrade(), weak_output.upgrade()) {
                let value = if number.value().is_nan() {
                    "empty".to_string()
                } else {
                    number.value().to_string()
                };
                output.set_text(Some(&format!("Value: {value}")));
            }
        });
    } else if let Some(primary) = find_control::<ToggleSplitButton>(root, "Primary") {
        let weak_output = output.downgrade();
        let weak_primary = primary.downgrade();
        primary.property_changed(move |_| {
            if let (Some(primary), Some(output)) = (weak_primary.upgrade(), weak_output.upgrade()) {
                output.set_text(Some(if primary.is_checked() {
                    "Checked"
                } else {
                    "Unchecked"
                }));
            }
        });
    } else if let Some(primary) = find_control::<ToggleButton>(root, "Primary") {
        let weak_output = output.downgrade();
        let weak_primary = primary.downgrade();
        primary.property_changed(move |_| {
            if let (Some(primary), Some(output)) = (weak_primary.upgrade(), weak_output.upgrade()) {
                output.set_text(Some(match primary.is_checked() {
                    Some(true) => "Checked",
                    Some(false) => "Unchecked",
                    None => "Indeterminate",
                }));
            }
        });
    } else if let Some(primary) = find_control::<TextBox>(root, "Primary") {
        let weak_output = output.downgrade();
        let weak_primary = primary.downgrade();
        primary.property_changed(move |_| {
            if let (Some(primary), Some(output)) = (weak_primary.upgrade(), weak_output.upgrade()) {
                let count = primary.text().unwrap_or_default().chars().count();
                output.set_text(Some(&format!("{count} characters")));
            }
        });
    } else if let Some(primary) = find_control::<Slider>(root, "Primary") {
        let weak_output = output.downgrade();
        let weak_primary = primary.downgrade();
        primary.property_changed(move |_| {
            if let (Some(primary), Some(output)) = (weak_primary.upgrade(), weak_output.upgrade()) {
                output.set_text(Some(&format!("Value: {}", primary.value())));
            }
        });
    } else if let Some(primary) = find_control::<ComboBox>(root, "Primary") {
        let weak_output = output.downgrade();
        let weak_primary = primary.downgrade();
        primary.property_changed(move |_| {
            if let (Some(primary), Some(output)) = (weak_primary.upgrade(), weak_output.upgrade()) {
                output.set_text(Some(&format!(
                    "Selected index: {}",
                    primary.selected_index()
                )));
            }
        });
    } else if let Some(primary) = find_control::<SplitButton>(root, "Primary") {
        let count = Rc::new(Cell::new(0u64));
        let weak_output = output.downgrade();
        primary.click(move |_, _| {
            count.set(count.get().saturating_add(1));
            if let Some(output) = weak_output.upgrade() {
                output.set_text(Some(&format!("Clicked {} times", count.get())));
            }
        });
    } else if let Some(primary) = find_control::<Button>(root, "Primary") {
        let count = Rc::new(Cell::new(0u64));
        let weak_output = output.downgrade();
        primary.click(move |_, _| {
            count.set(count.get().saturating_add(1));
            if let Some(output) = weak_output.upgrade() {
                output.set_text(Some(&format!("Clicked {} times", count.get())));
            }
        });
    }
    if let (Some(disable), Some(primary)) = (
        find_control::<ToggleButton>(root, "Disable"),
        find_control::<Control>(root, "Primary"),
    ) {
        let weak_primary = primary.downgrade();
        let weak_disable = disable.downgrade();
        disable.property_changed(move |_| {
            if let (Some(primary), Some(disable)) = (weak_primary.upgrade(), weak_disable.upgrade())
            {
                primary.set_is_enabled(disable.is_checked() != Some(true));
            }
        });
    }
    output.set_margin(Thickness::new(0.0, 4.0, 0.0, 0.0));
}

fn wire_radio_groups(root: &Ref<Control>) {
    let Some(output) = find_control::<Border>(root, "ControlOutput") else {
        return;
    };
    for name in ["BackgroundRadioButtons", "BorderRadioButtons"] {
        let radio = root.get_control::<RadioButtons>(name);
        let weak_output = output.downgrade();
        let weak_radio = radio.downgrade();
        radio.property_changed(move |change| {
            if change.property()
                != ferroui_controls::primitives::SelectingItemsControl::selected_index_property()
                    .as_property()
            {
                return;
            }
            if let (Some(output), Some(radio)) = (weak_output.upgrade(), weak_radio.upgrade()) {
                let color = match (name, radio.selected_index()) {
                    ("BackgroundRadioButtons", 0) => Color::from_rgb(0, 128, 0),
                    ("BackgroundRadioButtons", 1) => Color::from_rgb(255, 255, 0),
                    ("BorderRadioButtons", 0) => Color::from_rgb(0, 100, 0),
                    ("BorderRadioButtons", 1) => Color::from_rgb(255, 215, 0),
                    (_, 2) => Color::from_rgb(255, 255, 255),
                    _ => return,
                };
                let brush = Some(Rc::new(ImmutableSolidColorBrush::new(color))
                    as Rc<dyn ferroui_base::media::IBrush>);
                if name == "BackgroundRadioButtons" {
                    output.set_background(brush);
                } else {
                    output.set_border_brush(brush);
                }
            }
        });
    }
    let text = root.get_control::<TextBlock>("Control1Output");
    for (name, label) in [
        ("Option1", "Option 1"),
        ("Option2", "Option 2"),
        ("Option3", "Option 3"),
    ] {
        let radio = root.get_control::<ToggleButton>(name);
        let weak_radio = radio.downgrade();
        let weak_text = text.downgrade();
        radio.property_changed(move |change| {
            if change.property() == ToggleButton::is_checked_property().as_property() {
                if let (Some(radio), Some(text)) = (weak_radio.upgrade(), weak_text.upgrade()) {
                    if radio.is_checked() == Some(true) {
                        text.set_text(Some(&format!("You selected {label}")));
                    }
                }
            }
        });
    }
}

fn wire_contrast(root: &Ref<Control>) {
    let Some(text_color) = find_control::<TextBox>(root, "TextColor") else {
        return;
    };
    let background_color = root.get_control::<TextBox>("BackgroundColor");
    let weak_root = root.downgrade();
    let update = Rc::new(move || {
        let Some(root) = weak_root.upgrade() else {
            return;
        };
        let first = crate::contrast::parse_rgb(
            &root
                .get_control::<TextBox>("TextColor")
                .text()
                .unwrap_or_default(),
        );
        let second = crate::contrast::parse_rgb(
            &root
                .get_control::<TextBox>("BackgroundColor")
                .text()
                .unwrap_or_default(),
        );
        let ratio = root.get_control::<TextBlock>("ContrastRatio");
        let checks = root.get_control::<TextBlock>("ContrastChecks");
        let (Some(first), Some(second)) = (first, second) else {
            ratio.set_text(Some("Enter two six-digit RGB colors."));
            checks.set_text(Some("Contrast is unavailable while an input is invalid."));
            return;
        };
        let result = crate::contrast::ContrastResult::calculate(first, second);
        ratio.set_text(Some(&format!("{:.2}:1", result.ratio)));
        let state = |passed| if passed { "Pass" } else { "Fail" };
        checks.set_text(Some(&format!(
            "Regular text (4.5:1): {} · Large text (3:1): {} · Components (3:1): {}",
            state(result.normal_text),
            state(result.large_text),
            state(result.components),
        )));
        root.get_control::<Border>("ContrastPreview")
            .set_background(Some(Rc::new(ImmutableSolidColorBrush::new(second))));
        root.get_control::<TextBlock>("ContrastText")
            .set_foreground(Some(Rc::new(ImmutableSolidColorBrush::new(first))));
    });
    for input in [text_color, background_color] {
        let update = update.clone();
        input.property_changed(move |change| {
            if change.property() == TextBox::text_property().as_property() {
                update();
            }
        });
    }
    update();
}

fn find_control<T: ObjectType>(root: &Ref<Control>, name: &str) -> Option<Ref<T>> {
    root.find_control::<Control>(name)
        .and_then(|control| control.cast::<T>())
}

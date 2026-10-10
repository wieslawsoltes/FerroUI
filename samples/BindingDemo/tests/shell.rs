//! The real application of the sample, headless: the application ([`App`]: `App.xaml`, the
//! Fluent theme) with its main window, which renders through the compositor of the test with
//! Skia and takes the input of a mouse (`sample_testing::Shell`).
//!
//! Not ports: the upstream sample has no tests. Each test selects a tab of the window and
//! looks at what the tab shows and at what its bindings do; the last one visits every tab and
//! compares the errors the bindings reported with the accepted ones.

use crate::view_models::{
    DataAnnotationsErrorViewModel, ExceptionErrorViewModel, IndeiErrorViewModel, MainWindowViewModel,
};
use crate::{App, MainWindow, SAMPLE};
use ferroui_base::metadata::from_markup_value;
use ferroui_base::{ObjectType, Ref, Visual};
use ferroui_controls::primitives::ToggleButton;
use ferroui_controls::{
    Button, DataValidationErrors, ListBox, ListBoxItem, ProgressBar, TabControl, TabItem, TextBlock, TextBox,
};
use sample_testing::Shell;
use std::rc::Rc;
use std::time::Duration;

/// The size of the window of the test: the size the document of the main window states.
const SIZE: (f64, f64) = (800.0, 600.0);

/// The tabs of the main window, in their order.
const TABS: [&str; 5] = ["Basic", "ListBox", "Property Validation", "Commands", "Advanced"];

/// The reports of the bindings of the window that are accepted: the upstream sample makes
/// them too (`(place, report, count)`).
const ACCEPTED: &[(&str, &str, usize)] = &[
    // `Command="{Binding NestedModel.Command}"` of the button "Nested View Model Button":
    // `MainWindowViewModel.NestedModel` is null until the command of one of the buttons
    // above it ran, and a null in the middle of a path is an error of the binding upstream
    // too ("Value is null.").
    ("MainWindow", "Button.Command <- NestedModel.Command at NestedModel: Value is null.", 1),
];

/// The reports of the open gaps of the framework (`GAPS.md`), which the upstream sample does
/// not make: each goes with its gap.
const GAPS: &[(&str, &str, usize)] = &[
    // B002: `Content="{ReflectionBinding Selection.SelectedItems[0]}"`: the selection model
    // contract declares no members for markup, so the path stops at `SelectedItems`.
    (
        "MainWindow",
        "ContentControl.Content <- Selection.SelectedItems[0] at SelectedItems: Could not find a matching property accessor for 'SelectedItems' on 'FerroUI.Controls.Selection.ISelectionModel'.",
        1,
    ),
];

/// The application of the sample with its main window shown.
struct Demo {
    window: Ref<MainWindow>,
    view_model: Rc<MainWindowViewModel>,
    shell: Shell,
}

impl Demo {
    fn start() -> Demo {
        let shell = Shell::start(&SAMPLE, SIZE.0, SIZE.1, || App::new().upcast());
        let window = MainWindow::new();
        window.show();
        shell.settle();
        let view_model =
            from_markup_value::<Rc<MainWindowViewModel>>(&window.data_context()).expect("the view model of the window");
        Demo { window, view_model, shell }
    }

    fn tab_control(&self) -> Ref<TabControl> {
        shown::<TabControl>(&self.window).into_iter().next().expect("the tab control of the window")
    }

    /// Selects the tab `index` and runs the frames of the change.
    fn select_tab(&self, index: usize) {
        let tabs = self.tab_control();
        tabs.set_selected_index(index as i32);
        self.shell.settle();
        let item = from_markup_value::<Ref<TabItem>>(&tabs.selected_item()).expect("the selected tab");
        assert_eq!(Some(TABS[index].to_string()), from_markup_value::<String>(&item.header()), "the header of the tab {index}");
        self.assert_drawn(TABS[index]);
    }

    /// The last frame of the window has more than a background where the tab control is.
    fn assert_drawn(&self, context: &str) {
        assert!(self.shell.frames(0) > 0, "{context}: the compositor drew the window");
        let frame = self.shell.last_frame(0);
        let rect = Shell::frame_rect_of(&self.window, &self.tab_control());
        let colors = frame.colors(rect);
        assert!(colors > 2, "{context}: the tab control drew {colors} colours in {rect:?}");
    }

    /// The text box of the selected tab with the placeholder `placeholder`.
    fn text_box(&self, placeholder: &str) -> Ref<TextBox> {
        shown::<TextBox>(&self.window)
            .into_iter()
            .find(|text_box| text_box.placeholder_text().as_deref() == Some(placeholder))
            .unwrap_or_else(|| panic!("the text box '{placeholder}' of the selected tab"))
    }

    /// The button (a toggle button, a check box and a radio button are buttons) of the
    /// selected tab with the text content `content`.
    fn button(&self, content: &str) -> Ref<Button> {
        shown::<Button>(&self.window)
            .into_iter()
            .find(|button| from_markup_value::<String>(&button.content()).as_deref() == Some(content))
            .unwrap_or_else(|| panic!("the button '{content}' of the selected tab"))
    }

    fn toggle(&self, content: &str) -> Ref<ToggleButton> {
        self.button(content).cast::<ToggleButton>().unwrap_or_else(|| panic!("'{content}' is a toggle button"))
    }

    /// Presses and releases the left button of the mouse in the middle of `visual`.
    fn click(&self, visual: &Visual) {
        self.shell.click(0, Shell::bounds_of(&self.window, visual).center());
    }

    /// Runs frames, with the time of the machine passing between them, until `condition`
    /// holds: for what a timer or the background thread of the view model delivers.
    fn wait_for(&self, what: &str, condition: impl Fn() -> bool) {
        for _ in 0..150 {
            if condition() {
                return;
            }
            std::thread::sleep(Duration::from_millis(20));
            self.shell.frame();
        }
        panic!("{what}");
    }

    /// The reports of the bindings since the last call, as the guard takes them.
    fn binding_reports(&self) -> Vec<(String, String, usize)> {
        self.shell.take_binding_reports().into_iter().map(|(report, count)| ("MainWindow".to_string(), report.line(), count)).collect()
    }
}

impl Drop for Demo {
    fn drop(&mut self) {
        self.window.close();
    }
}

/// The elements of the class `T` below `root` that are shown.
fn shown<T: ObjectType>(root: &Visual) -> Vec<Ref<T>> {
    root.get_visual_descendants().filter(|visual| visual.is_effectively_visible()).filter_map(|visual| visual.cast::<T>()).collect()
}

/// The texts of the text blocks below `root` that are shown.
fn texts(root: &Visual) -> Vec<String> {
    shown::<TextBlock>(root).into_iter().filter_map(|text_block| text_block.text()).collect()
}

/// Changes the text of a text box the way typing does: the current value of the property,
/// which a binding of the property writes to its source.
fn type_text(text_box: &Ref<TextBox>, text: &str) {
    text_box.set_current_value(TextBox::text_property(), Some(text.to_string()));
}

fn text_of(text_box: &Ref<TextBox>) -> String {
    text_box.text().unwrap_or_default()
}

/// The tab "Basic": the modes of a binding, a binding into a collection, negation, numbers,
/// an element name and a resource as the source, the property the background thread sets and
/// the stream operator.
#[test]
fn the_basic_tab_binds_its_text_boxes() {
    let demo = Demo::start();
    demo.select_tab(0);
    let shown_texts = texts(&demo.window);
    for title in ["Simple Bindings", "Collection Bindings", "Negated Bindings", "Numeric Bindings", "Binding Sources", "Scheduler", "Stream Operator"] {
        assert!(shown_texts.iter().any(|text| text == title), "the tab shows the title {title}: {shown_texts:?}");
    }

    // Simple bindings: every mode shows the value, and a change of the two-way box reaches
    // the view model, the one-way box and the box that binds to the first by its name, and
    // not the one-time box.
    let two_way = demo.text_box("Two Way");
    let lost_focus = demo.text_box("Two Way (LostFocus)");
    let one_way = demo.text_box("One Way");
    let one_time = demo.text_box("One Time");
    let of_first = demo.text_box("Value of first TextBox");
    for text_box in [&two_way, &lost_focus, &one_way, &one_time, &of_first] {
        assert_eq!("Simple Binding", text_of(text_box), "{:?}", text_box.placeholder_text());
    }
    type_text(&two_way, "changed");
    demo.shell.settle();
    assert_eq!(Some(String::from("changed")), demo.view_model.string_value());
    assert_eq!("changed", text_of(&one_way));
    assert_eq!("changed", text_of(&lost_focus));
    assert_eq!("changed", text_of(&of_first));
    assert_eq!("Simple Binding", text_of(&one_time));

    // The box that updates its source when it loses the focus does not while it has not.
    type_text(&lost_focus, "typed");
    demo.shell.settle();
    assert_eq!(Some(String::from("changed")), demo.view_model.string_value());

    // The one-way box does not write.
    type_text(&one_way, "one way");
    demo.shell.settle();
    assert_eq!(Some(String::from("changed")), demo.view_model.string_value());

    // The box that binds to the first by its name writes to it, and so to the view model.
    type_text(&of_first, "by name");
    demo.shell.settle();
    assert_eq!("by name", text_of(&two_way));
    assert_eq!(Some(String::from("by name")), demo.view_model.string_value());

    // The collection binding shows the second item, also after the button moved another
    // item there (pressed with the mouse).
    let second = demo.text_box("Items[1].Value");
    assert_eq!("Item 1", text_of(&second));
    demo.click(&demo.button("Shuffle"));
    let moved = demo.view_model.items().items().get(1).value().expect("the value of the second item");
    assert_eq!(moved, text_of(&second));
    type_text(&second, "edited");
    demo.shell.settle();
    assert_eq!(Some(String::from("edited")), demo.view_model.items().items().get(1).value());

    // Negation: the text "True" is true.
    assert_eq!("True", text_of(&demo.text_box("Boolean String")));
    assert_eq!(Some(false), demo.toggle("!BooleanString").is_checked());
    assert_eq!(Some(true), demo.toggle("!!BooleanString").is_checked());
    type_text(&demo.text_box("Boolean String"), "False");
    demo.shell.settle();
    assert_eq!(Some(true), demo.toggle("!BooleanString").is_checked());
    assert_eq!(Some(false), demo.toggle("!!BooleanString").is_checked());

    // Numbers: the text box writes a number, which the text block and the progress bar show.
    let double = demo.text_box("Double");
    assert_eq!("5", text_of(&double));
    let progress_bar = shown::<ProgressBar>(&demo.window).into_iter().next().expect("the progress bar");
    assert_eq!(5.0, progress_bar.value());
    type_text(&double, "7.5");
    demo.shell.settle();
    assert_eq!(7.5, demo.view_model.double_value());
    assert_eq!(7.5, progress_bar.value());
    assert!(texts(&demo.window).iter().any(|text| text == "7.5"), "the text block shows the number");

    // A resource of the window as the source, of the instantiation of the nested generic
    // class the binding names as its data type: both boxes edit the one shared item.
    let shared = demo.text_box("Value of SharedItem.StringValue");
    let duplicate = demo.text_box("Value of SharedItem.StringValue (duplicate)");
    assert_eq!("shared", text_of(&shared));
    assert_eq!("shared", text_of(&duplicate));
    type_text(&shared, "both");
    demo.shell.settle();
    assert_eq!("both", text_of(&duplicate));

    // The background thread of the view model sets the time, and the timer the stream
    // operator subscribes to produces one.
    let background = demo.text_box("Background Thread");
    demo.wait_for("the background thread set the current time", || !text_of(&background).is_empty());
    assert_eq!(demo.view_model.current_time(), background.text());
    let stream = demo.text_box("StreamOperator");
    demo.wait_for("the stream produced a time", || !text_of(&stream).is_empty());

    demo.assert_drawn("Basic, after the changes");
}

/// The tab "ListBox": two list boxes over the items of the view model that share its
/// selection model.
#[test]
fn the_list_boxes_share_the_selection_of_the_view_model() {
    let demo = Demo::start();
    demo.select_tab(1);
    let lists = shown::<ListBox>(&demo.window);
    assert_eq!(2, lists.len(), "the two list boxes of the tab");
    for list in &lists {
        assert_eq!(20, list.item_count());
        let containers = list.get_realized_containers();
        assert!(!containers.is_empty(), "the list box realized containers");
        // The data template of the panel for the instantiation of the nested generic class
        // shows the value of an item.
        let shown_texts = texts(list);
        assert!(shown_texts.iter().any(|text| text == "Item 0"), "the list box shows its items: {shown_texts:?}");
    }

    // Pressing an item of the first list box selects it in the model, and so in the second.
    let third = lists[0].container_from_index(2).expect("the container of the third item");
    demo.click(&third);
    assert_eq!(2, demo.view_model.selection().selected_index());
    let other = lists[1].container_from_index(2).and_then(|container| container.cast::<ListBoxItem>()).expect("the same item of the second list box");
    assert!(other.is_selected(), "the second list box shows the selection of the first");

    // The selection is multiple: selecting through the model adds to it.
    demo.view_model.selection().select(4);
    demo.shell.settle();
    assert_eq!(2, demo.view_model.selection().count());
    let fifth = lists[0].container_from_index(4).and_then(|container| container.cast::<ListBoxItem>()).expect("the fifth item");
    assert!(fifth.is_selected());
    demo.assert_drawn("ListBox, with a selection");
}

/// The tab "Property Validation": a setter that fails, error notifications of the view
/// model, and data annotations.
#[test]
fn the_validation_tab_reports_the_errors_of_its_view_models() {
    let demo = Demo::start();
    demo.select_tab(2);
    let boxes = shown::<TextBox>(&demo.window);
    let of = |placeholder: &str, is_context: &dyn Fn(&Ref<TextBox>) -> bool| -> Ref<TextBox> {
        boxes
            .iter()
            .find(|text_box| text_box.placeholder_text().as_deref() == Some(placeholder) && is_context(text_box))
            .cloned()
            .unwrap_or_else(|| panic!("the text box '{placeholder}'"))
    };
    let has_context = |text_box: &Ref<TextBox>| text_box.data_context();

    // Exception validation: the setter rejects ten and more, and the text box shows the error.
    let exception = of("Less Than 10", &|text_box| from_markup_value::<Rc<ExceptionErrorViewModel>>(&has_context(text_box)).is_some());
    let view_model = demo.view_model.exception_data_validation();
    assert_eq!("0", text_of(&exception));
    type_text(&exception, "9");
    demo.shell.settle();
    assert_eq!(9, view_model.less_than_10());
    assert!(!DataValidationErrors::get_has_errors(&exception));
    type_text(&exception, "11");
    demo.shell.settle();
    assert_eq!(9, view_model.less_than_10(), "the setter rejected the value");
    assert!(DataValidationErrors::get_has_errors(&exception), "the text box shows the error of the setter");
    type_text(&exception, "3");
    demo.shell.settle();
    assert!(!DataValidationErrors::get_has_errors(&exception), "a valid value clears the error");

    // Error notifications: a value above the maximum is an error of the value, until the
    // maximum is raised.
    let maximum = of("Maximum", &|text_box| from_markup_value::<Rc<IndeiErrorViewModel>>(&has_context(text_box)).is_some());
    let value = of("Value", &|text_box| from_markup_value::<Rc<IndeiErrorViewModel>>(&has_context(text_box)).is_some());
    let view_model = demo.view_model.indei_data_validation();
    assert_eq!("10", text_of(&maximum));
    assert_eq!("0", text_of(&value));
    type_text(&value, "11");
    demo.shell.settle();
    assert_eq!(11, view_model.value(), "the value is set: the error is a notification");
    assert!(DataValidationErrors::get_has_errors(&value), "the text box shows the error of the value");
    assert!(!DataValidationErrors::get_has_errors(&maximum));
    type_text(&maximum, "20");
    demo.shell.settle();
    assert_eq!(20, view_model.maximum());
    assert!(!DataValidationErrors::get_has_errors(&value), "raising the maximum clears the error");

    // Data annotations: the properties are bound in both directions. The framework has no
    // validation by annotations (GAPS.md, B001): a value out of the range of the annotation
    // of the managed original is not an error here.
    let phone = of("Phone #", &|text_box| from_markup_value::<Rc<DataAnnotationsErrorViewModel>>(&has_context(text_box)).is_some());
    let less_than_10 = of("Less Than 10", &|text_box| from_markup_value::<Rc<DataAnnotationsErrorViewModel>>(&has_context(text_box)).is_some());
    let view_model = demo.view_model.data_annotations_validation();
    type_text(&phone, "555 0100");
    type_text(&less_than_10, "50");
    demo.shell.settle();
    assert_eq!(Some(String::from("555 0100")), view_model.phone_number());
    assert_eq!(50, view_model.less_than_10());
    assert!(!DataValidationErrors::get_has_errors(&less_than_10), "B001 is open: no validation by annotations");

    demo.assert_drawn("Property Validation, with errors");
}

/// The tab "Commands": a command with a parameter, bindings of its state, a command of a
/// nested view model and a method as a command.
#[test]
fn the_commands_tab_runs_the_commands_of_the_view_model() {
    let demo = Demo::start();
    demo.select_tab(3);
    let text_box = shown::<TextBox>(&demo.window).into_iter().next().expect("the text box of the tab");
    assert_eq!("Simple Binding", text_of(&text_box));
    assert_eq!(Some(false), demo.toggle("ToggleButton").is_checked());
    assert_eq!(Some(true), demo.toggle("CheckBox").is_checked());
    assert_eq!(Some(false), demo.toggle("Radio Button").is_checked());

    // `Command="{Binding Do}"`: the method is a command, which can execute while `CanDo`
    // says so, and `CanDo` depends on the flag.
    let method = demo.button("Command Method Do");
    assert!(method.command().is_some(), "the method is bound as a command");
    assert!(!method.is_effectively_enabled(), "CanDo is false while the flag is");
    // The command of the nested view model, which does not exist yet.
    let nested = demo.button("Nested View Model Button");
    assert!(nested.command().is_none());

    // The button, pressed with the mouse, runs the command with its parameter.
    demo.click(&demo.button("Button"));
    assert!(demo.view_model.boolean_flag());
    assert_eq!(Some(String::from("Button")), demo.view_model.string_value());
    assert_eq!("Button", text_of(&text_box));
    assert_eq!(Some(true), demo.toggle("ToggleButton").is_checked());
    assert_eq!(Some(false), demo.toggle("CheckBox").is_checked());
    assert_eq!(Some(true), demo.toggle("Radio Button").is_checked());
    assert!(method.is_effectively_enabled(), "CanDo follows the flag");
    assert!(demo.view_model.nested_model().is_some());
    assert!(nested.command().is_some(), "the command of the nested view model arrived");

    // The text box writes the value the command set.
    type_text(&text_box, "typed");
    demo.shell.settle();
    assert_eq!(Some(String::from("typed")), demo.view_model.string_value());
    demo.assert_drawn("Commands, after a command");
}

/// The tab "Advanced": a generic markup extension and a generic value converter, each with a
/// type argument stated in markup.
#[test]
fn the_advanced_tab_shows_what_the_generic_types_provide() {
    let demo = Demo::start();
    demo.select_tab(4);
    let blocks: Vec<Ref<TextBlock>> = shown::<TextBlock>(&demo.window);
    let block = |text: &str| blocks.iter().find(|block| block.text().as_deref() == Some(text)).cloned();
    // `{local:GenericMarkupExtension Value=Red, x:TypeArguments=Color}`.
    assert!(block("Color: Red").is_some(), "the markup extension provided its text: {:?}", texts(&demo.window));
    // `{Binding $self.Background, Converter={StaticResource BrushConverter}}`, with the
    // converter of the type argument `SolidColorBrush`.
    let converted = block("SolidColorBrush: Yellow").unwrap_or_else(|| panic!("the converter converted the brush: {:?}", texts(&demo.window)));

    // The text block with the yellow background is drawn.
    let frame = demo.shell.last_frame(0);
    let rect = Shell::frame_rect_of(&demo.window, &converted);
    let yellow = frame.count(rect, [255, 255, 0, 255], 2);
    assert!(yellow > 0, "the background of the text block is drawn in {rect:?}");
}

/// The bindings of the window report the accepted errors and no other, on every tab.
#[test]
fn the_bindings_of_the_window_report_the_accepted_errors() {
    let demo = Demo::start();
    for index in 0..TABS.len() {
        demo.select_tab(index);
    }
    let found = demo.binding_reports();
    let accepted: Vec<(&str, &str, usize)> = ACCEPTED.iter().chain(GAPS).copied().collect();
    sample_testing::assert_accepted(&found, &accepted);
}

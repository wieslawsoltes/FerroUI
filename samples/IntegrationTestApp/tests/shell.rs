//! The real application of the sample, headless: the application ([`App`]: `App.xaml`, the
//! Fluent theme) and its main window, which renders through the compositor of the test into
//! memory and takes the input of a mouse (`sample_testing::Shell`).
//!
//! Not ports: the upstream sample has no tests of its own. The user interface automation tests
//! of the upstream project drive it through the accessibility interface of the platform: they
//! find a control by its automation id (the `AutomationProperties.AutomationId` of the control,
//! or its name) or by its accessible name, read its text or value, and click it. The tests here
//! look for the same controls in the tree of automation peers of the main window, page by
//! page, and do with the mouse what a few of those tests do.

use crate::models::Page;
use crate::pages::ButtonPage;
use crate::view_models::MainWindowViewModel;
use crate::{App, MainWindow, SAMPLE};
use ferroui_base::metadata::from_markup_value;
use ferroui_base::{ObjectType, Ref};
use ferroui_controls::automation::peers::{AutomationPeer, ControlAutomationPeer};
use ferroui_controls::{Border, Button, CheckBox, ComboBox, Control, Decorator, ListBox, Slider, TextBlock, TextBox};
use sample_testing::{assert_accepted, Shell};
use std::rc::Rc;

/// The size of the main window: every page of the list of pages is in view.
const WIDTH: f64 = 1200.0;
const HEIGHT: f64 = 1000.0;

/// The controls the automation tests find on every page: the list of pages and the state of
/// the main window.
const SHELL_IDS: &[&str] = &["Pager", "MainWindowState", "AppOverlayPopups"];

/// The pages of the main window in the order of its list, each with the automation ids the
/// automation tests look for on it that are in its tree as it is shown (the content of a
/// popup, of a menu and of a window the page opens is in the tree once it is open).
const PAGES: &[(&str, &[&str])] = &[
    ("Automation", &["TextBlockWithName", "TextBlockWithNameAndAutomationId", "TextBlockAsLabel", "LabeledByTextBox"]),
    (
        "Button",
        &["DisabledButton", "EffectivelyDisabledButton", "BasicButton", "ButtonWithTextBlock", "ButtonWithAcceleratorKey"],
    ),
    ("CheckBox", &["UncheckedCheckBox", "CheckedCheckBox", "ThreeStateCheckBox"]),
    ("ComboBox", &["BasicComboBox", "ComboBoxWrapSelection", "ComboBoxSelectionClear", "ComboBoxSelectFirst"]),
    ("ContextMenu", &["ShowContextMenu"]),
    (
        "DesktopPage",
        &["TrayIconClicked", "TrayIconMenuClicked", "ToggleTrayIconVisible", "DockMenuItemCount", "AddDockMenuItem"],
    ),
    ("DragDrop", &["ResetDragDrop", "DragSource", "DropTarget", "DropPosition", "DragDropStatus", "DropTargetText"]),
    ("Embedding", &["Reset", "NativeTextBox", "EmbeddingPopupOpenCheckBox"]),
    ("Gestures", &["ResetGestures", "GestureBorder", "LastGesture"]),
    (
        "Keyboard",
        &[
            "ResetKeyboard",
            "GestureTextBox",
            "LastKeyBinding",
            "GestureTextBoxContent",
            "KeyDownTextBox",
            "LastKeyDown",
            "KeyDownCount",
            "LastTextInput",
        ],
    ),
    ("ListBox", &["ListBoxSelectionClear", "BasicListBox"]),
    ("Menu", &["MenuClickedMenuItemReset", "ClickedMenuItem", "RootMenuItem", "MenuFocusTest"]),
    ("Pointer", &["PointerPageShowDialog", "PointerCaptureStatus"]),
    ("Popups", &["ShowLightDismissPopup", "DismissButton", "ShowStaysOpenPopup", "ShowTopMostPopup", "OpenNewWindowButton"]),
    ("RadioButton", &["BasicRadioButton", "ThreeStatesRadioButton1", "ThreeStatesRadioButton2"]),
    (
        "Screens",
        &[
            "ScreenRefresh",
            "ScreenName",
            "ScreenHandle",
            "ScreenBounds",
            "ScreenWorkArea",
            "ScreenScaling",
            "ScreenOrientation",
            "ScreenSameReference",
        ],
    ),
    ("ScrollBar", &["MyScrollBar"]),
    ("Slider", &["ResetSliders", "HorizontalSlider", "thumb", "HorizontalSliderValue"]),
    (
        "Window Decorations",
        &[
            "WindowExtendClientAreaToDecorationsHint",
            "WindowShowTitleAreaControl",
            "WindowTitleBarHeightHint",
            "ApplyWindowDecorations",
            "ShowNewWindowDecorations",
            "WindowDecorationProperties",
        ],
    ),
    (
        "Window",
        &[
            "ShowWindowSize",
            "ShowWindowMode",
            "ShowWindowLocation",
            "ShowWindowState",
            "ShowWindowSystemDecorations",
            "ShowWindowExtendClientAreaToDecorationsHint",
            "ShowWindowCanResize",
            "ShowWindowCanMinimize",
            "ShowWindowCanMaximize",
            "ShowWindow",
            "SendToBack",
            "EnterFullscreen",
            "IntegrationTestApp_ExitFullscreen",
            "RestoreAll",
            "ShowTopmostWindow",
            "ShowTransparentWindow",
            "ShowTransparentPopup",
        ],
    ),
];

/// The reports the bindings of the sample are known to make, by page: the name of the page
/// (`(start)` for the main window before a page is selected), the report as `Report::line`
/// writes it, and how often it is made when the page is shown. Each is a report the upstream
/// sample makes too.
const ACCEPTED: &[(&str, &str, usize)] = &[
    // Pages/ButtonPage.xaml: `Command="{ReflectionBinding DoesntExist}"` of the button
    // `EffectivelyDisabledButton`. The page has no data context of its own and inherits the view
    // model of the main window, which has no such member: the binding fails, on purpose (the
    // button is disabled by a command that never arrives, which is what the automation test of
    // the button reads).
    (
        "Button",
        "Button.Command <- DoesntExist at DoesntExist: Could not find a matching property accessor for 'DoesntExist' on \
         'IntegrationTestApp.ViewModels.MainWindowViewModel'.",
        1,
    ),
];

/// The application of the sample with its main window shown.
struct Run {
    // Declared before the shell and closed when the run is dropped: the windows of the test
    // are closed before the shell is dropped.
    window: Ref<MainWindow>,
    shell: Shell,
}

impl Run {
    fn start() -> Run {
        let shell = Shell::start(&SAMPLE, WIDTH, HEIGHT, || App::new().upcast());
        let window = MainWindow::new();
        window.show();
        shell.settle();
        Run { window, shell }
    }

    fn view_model(&self) -> Rc<MainWindowViewModel> {
        from_markup_value::<Rc<MainWindowViewModel>>(&self.window.data_context()).expect("the view model of the main window")
    }

    fn pages(&self) -> Vec<Rc<Page>> {
        self.view_model().pages().items().to_vec()
    }

    fn pager(&self) -> Ref<ListBox> {
        self.window.get_control::<ListBox>("Pager")
    }

    fn pager_content(&self) -> Ref<Decorator> {
        self.window.get_control::<Decorator>("PagerContent")
    }

    /// Selects the page `name` through the view model, as the item of the view menu of the
    /// page does, and runs the frames of what the selection started.
    fn select(&self, name: &str) {
        let page = self.pages().into_iter().find(|page| page.name() == name).unwrap_or_else(|| panic!("no page {name}"));
        self.view_model().set_selected_page(Some(page));
        self.shell.settle();
        assert_eq!(self.view_model().selected_page().map(|page| page.name()).as_deref(), Some(name));
        assert!(self.pager_content().child().is_some(), "{name}: the main window shows the page");
    }

    /// The automation peers of the main window and of everything in it, as the accessibility
    /// interface of a platform walks them.
    fn elements(&self) -> Vec<Ref<AutomationPeer>> {
        fn collect(peer: &Ref<AutomationPeer>, depth: usize, out: &mut Vec<Ref<AutomationPeer>>) {
            out.push(peer.clone());
            if depth < 64 {
                for child in peer.get_children().iter() {
                    collect(child, depth + 1, out);
                }
            }
        }
        let mut elements = Vec::new();
        collect(&ControlAutomationPeer::create_peer_for_element(&self.window), 0, &mut elements);
        elements
    }

    /// The control with the automation id `id`.
    fn control<T: ObjectType>(&self, id: &str) -> Ref<T> {
        let elements = self.elements();
        let peer = find_by_id(&elements, id).unwrap_or_else(|| panic!("no element with the automation id {id}"));
        let owner = peer.cast::<ControlAutomationPeer>().unwrap_or_else(|| panic!("{id} is not the peer of a control")).owner();
        owner.cast::<T>().unwrap_or_else(|| panic!("{id} is a {}", owner.get_type().name()))
    }

    /// Clicks the middle of `control` with the mouse.
    fn click(&self, control: &Control) {
        let bounds = Shell::bounds_of(&self.window, control);
        assert!(bounds.width > 0.0 && bounds.height > 0.0, "the control is laid out: {bounds:?}");
        self.shell.click(0, bounds.center());
    }

    /// Clicks the control with the automation id `id` with the mouse.
    fn click_id(&self, id: &str) {
        self.click(&self.control::<Control>(id));
    }

    /// The text of the text block with the automation id `id`.
    fn text(&self, id: &str) -> String {
        self.control::<TextBlock>(id).text().unwrap_or_default()
    }
}

impl Drop for Run {
    fn drop(&mut self) {
        self.window.close();
    }
}

fn find_by_id<'a>(elements: &'a [Ref<AutomationPeer>], id: &str) -> Option<&'a Ref<AutomationPeer>> {
    elements.iter().find(|peer| peer.get_automation_id().as_deref() == Some(id))
}

fn find_by_name<'a>(elements: &'a [Ref<AutomationPeer>], name: &str) -> Option<&'a Ref<AutomationPeer>> {
    elements.iter().find(|peer| peer.get_name() == name)
}

#[test]
fn the_pages_of_the_main_window_are_the_pages_of_the_upstream_sample() {
    let run = Run::start();
    let names: Vec<String> = run.pages().iter().map(|page| page.name()).collect();
    let expected: Vec<&str> = PAGES.iter().map(|(name, _)| *name).collect();
    assert_eq!(names, expected);

    // The list of pages shows the name of each page, which is how the automation tests find
    // the item of a page (by its accessible name, inside the element `Pager`).
    let elements = run.elements();
    for name in expected {
        assert!(find_by_name(&elements, name).is_some(), "the list of pages has an item named {name}");
    }
    // The main window is found by its automation id, which is its name.
    assert_eq!(elements[0].get_automation_id().as_deref(), Some("MainWindow"));
}

#[test]
fn every_page_shows_the_controls_the_automation_tests_look_for() {
    let run = Run::start();
    let mut reports: Vec<(String, String, usize)> = Vec::new();
    let mut record = |page: &str, run: &Run| {
        for (report, count) in run.shell.take_binding_reports() {
            reports.push((page.to_string(), report.line(), count));
        }
    };
    record("(start)", &run);

    for (name, ids) in PAGES {
        run.select(name);

        let elements = run.elements();
        for id in SHELL_IDS.iter().chain(ids.iter()) {
            assert!(find_by_id(&elements, id).is_some(), "{name}: no element with the automation id {id}");
        }

        // The compositor drew the window, and the page drew something: its part of the last
        // frame has more than the colour of the background of the window.
        assert!(run.shell.frames(0) > 0, "{name}: the compositor drew a frame of the main window");
        let frame = run.shell.last_frame(0);
        let content = Shell::frame_rect_of(&run.window, &run.pager_content());
        assert!(content.2 > content.0 && content.3 > content.1, "{name}: the content of the page has the bounds {content:?}");
        assert!(frame.colors(content) > 1, "{name}: the page drew nothing");

        record(name, &run);
    }

    assert_accepted(&reports, ACCEPTED);
}

#[test]
fn the_list_of_pages_navigates_with_the_mouse() {
    let run = Run::start();
    assert!(run.pager_content().child().is_none(), "no page is shown before one is selected");

    // The second item of the list: the page of the button.
    let item = run.pager().container_from_index(1).expect("the container of the second page");
    run.click(&item);

    assert_eq!(run.view_model().selected_page().map(|page| page.name()).as_deref(), Some("Button"));
    assert_eq!(run.pager().selected_index(), 1);
    let content = run.pager_content().child().expect("the main window shows a page");
    assert!(content.is::<ButtonPage>(), "the main window shows a {}", content.get_type().name());
}

#[test]
fn a_click_on_a_button_changes_what_the_page_shows() {
    let run = Run::start();

    // AutomationTests: the button of the live region adds a sentence to it.
    run.select("Automation");
    assert_eq!(run.text("textLiveRegion"), "This is an assertive live region.");
    let button = run
        .pager_content()
        .get_visual_descendants()
        .filter_map(|visual| visual.cast::<Button>())
        .next()
        .expect("the button of the live region");
    run.click(&button);
    assert_eq!(run.text("textLiveRegion"), "This is an assertive live region. Lorem ipsum.");

    // GestureTests: a click on the border is a tap, and the reset button clears it.
    run.select("Gestures");
    assert_eq!(run.text("LastGesture"), "");
    run.click(&run.control::<Border>("GestureBorder"));
    assert_eq!(run.text("LastGesture"), "Tapped");
    run.click_id("ResetGestures");
    assert_eq!(run.text("LastGesture"), "");
}

#[test]
fn a_check_box_toggles_with_the_mouse() {
    let run = Run::start();
    run.select("CheckBox");

    // CheckBoxTests: the unchecked check box is checked by a click, the checked one unchecked.
    let unchecked = run.control::<CheckBox>("UncheckedCheckBox");
    assert_eq!(unchecked.is_checked(), Some(false));
    run.click(&unchecked);
    assert_eq!(unchecked.is_checked(), Some(true));

    let checked = run.control::<CheckBox>("CheckedCheckBox");
    assert_eq!(checked.is_checked(), Some(true));
    run.click(&checked);
    assert_eq!(checked.is_checked(), Some(false));

    // The check box of three states starts in its third state.
    assert_eq!(run.control::<CheckBox>("ThreeStateCheckBox").is_checked(), None);
}

#[test]
fn a_combo_box_selects_and_opens() {
    let run = Run::start();
    run.select("ComboBox");
    let combo_box = run.control::<ComboBox>("BasicComboBox");
    assert_eq!(combo_box.selected_index(), -1);

    // ComboBoxTests: the buttons of the page select the first item and clear the selection.
    run.click_id("ComboBoxSelectFirst");
    assert_eq!(combo_box.selected_index(), 0);
    run.click_id("ComboBoxSelectionClear");
    assert_eq!(combo_box.selected_index(), -1);

    // The check box of the page is bound to the wrap of the selection of the combo box.
    assert!(!combo_box.wrap_selection());
    run.click_id("ComboBoxWrapSelection");
    assert!(combo_box.wrap_selection());

    // A click opens the drop down; its items are the two of the document.
    assert!(!combo_box.is_drop_down_open());
    run.click(&combo_box);
    assert!(combo_box.is_drop_down_open());
    assert_eq!(combo_box.item_count(), 2);
    combo_box.set_selected_index(1);
    run.shell.settle();
    combo_box.set_is_drop_down_open(false);
    run.shell.settle();
    assert_eq!(combo_box.selected_index(), 1);
}

#[test]
fn a_slider_moves_with_the_mouse() {
    let run = Run::start();
    run.select("Slider");
    let slider = run.control::<Slider>("HorizontalSlider");
    let value_text = run.control::<TextBox>("HorizontalSliderValue");
    assert_eq!(slider.value(), 50.0);
    assert_eq!(value_text.text().as_deref(), Some("50"));

    // SliderTests: a click on the track to the right of the thumb raises the value, and the
    // text box bound to the value follows it.
    let bounds = Shell::bounds_of(&run.window, &slider);
    let position = ferroui_base::Point::new(bounds.x + bounds.width * 0.9, bounds.y + bounds.height / 2.0);
    run.shell.click(0, position);
    let value = slider.value();
    assert!(value > 50.0, "the value of the slider is {value}");
    let shown: f64 = value_text.text().unwrap_or_default().parse().expect("the text box shows a number");
    assert!((shown - value).abs() <= 0.5, "the text box shows {shown} for the value {value}");

    // The reset button sets the value back.
    run.click_id("ResetSliders");
    assert_eq!(slider.value(), 50.0);
    assert_eq!(value_text.text().as_deref(), Some("50"));
}

#[test]
fn the_items_of_the_list_box_page_have_their_names() {
    let run = Run::start();
    run.select("ListBox");

    // ListBoxTests: the items are found by their names inside the list box.
    let elements = run.elements();
    for name in ["Item 2", "Item 3", "Item 4"] {
        assert!(find_by_name(&elements, name).is_some(), "the list box has an item named {name}");
    }

    // The button of the page clears the selection.
    let list_box = run.control::<ListBox>("BasicListBox");
    list_box.set_selected_index(3);
    run.shell.settle();
    run.click_id("ListBoxSelectionClear");
    assert_eq!(list_box.selected_index(), -1);
}

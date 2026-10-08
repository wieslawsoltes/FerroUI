//! Tests of the samples of the navigation page gallery: the curved header, interactive header,
//! MVVM and pass data pages.
//!
//! Not ports: the upstream sample has no tests.

use super::support::*;
use crate::pages::{
    NavigationPageCurvedHeaderPage, NavigationPageInteractiveHeaderPage, NavigationPageMvvmPage,
    NavigationPageMvvmShellViewModel, NavigationPagePassDataPage,
};
use ferroui_base::input::ICommand;
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::metadata::from_markup_value;
use ferroui_base::threading::Dispatcher;
use ferroui_base::{IntoRef, ObjectType, Ref, StyledElement, Visual};
use ferroui_controls::shapes::Path;
use ferroui_controls::{
    Button, ComboBox, ContentPage, Control, ListBox, NavigationPage, Page, Panel, ScrollViewer, TextBlock, TextBox, Window,
};
use std::rc::Rc;

fn show(control: impl IntoRef<Control>) -> Ref<Window> {
    let window = Window::new();
    window.set_width(1100.0);
    window.set_height(800.0);
    window.set_content(Some(Control::boxed(control)));
    window.show();
    Dispatcher::ui_thread().run_jobs(None);
    window
}

fn descendants<T: ObjectType>(root: &Visual) -> Vec<Ref<T>> {
    root.get_visual_descendants().filter_map(|visual| visual.cast::<T>()).collect()
}

fn header(page: &Page) -> Option<String> {
    page.header().and_then(|header| header.downcast_ref::<String>().cloned())
}

fn current_header(navigation: &NavigationPage) -> Option<String> {
    navigation.current_page().and_then(|page| header(&page))
}

/// The control a content page shows.
fn content_of(page: &Page) -> Ref<Control> {
    let page = page.to_ref().cast::<ContentPage>().expect("a content page");
    page.content().and_then(|content| Control::from_boxed(&content)).expect("the content of the page")
}

fn click(button: &Button) {
    button.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    Dispatcher::ui_thread().run_jobs(None);
}

#[test]
fn the_pass_data_page_pushes_the_detail_page_of_the_contact_and_logs_it() {
    let _app = start_catalog_application();
    let page = NavigationPagePassDataPage::new();
    let window = show(page.clone());
    let navigation = page.get_control::<NavigationPage>("DemoNav");
    let log = page.get_control::<TextBlock>("NavigationLog");
    // The clock of the tests does not tick: a push with a transition would not complete.
    navigation.set_page_transition(None);
    assert_eq!(1, navigation.stack_depth());
    assert_eq!(Some(String::from("Contacts")), current_header(&navigation));
    assert_eq!(Some(String::from("Pushed \u{2192} Contacts")), log.text());

    // The list shows a card per contact.
    let list = navigation.current_page().expect("the list page");
    let cards = descendants::<Button>(&content_of(&list));
    assert_eq!(5, cards.len());

    // Through the constructor: the detail page has no data context of its own.
    click(&cards[1]);
    assert_eq!(2, navigation.stack_depth());
    assert_eq!(Some(String::from("Bob Smith")), current_header(&navigation));
    let detail = navigation.current_page().expect("the detail page");
    assert!(!detail.is_set(StyledElement::data_context_property().as_property()));
    let texts: Vec<_> = descendants::<TextBlock>(&content_of(&detail)).iter().filter_map(|text| text.text()).collect();
    assert_eq!(vec!["BS", "Bob Smith", "Passed via Constructor", "Product Designer", "Canada"], texts);
    assert_eq!(
        Some(String::from(
            "Pushed \u{2192} Contacts\nPushed \u{2192} Bob Smith\nNavigated to Bob Smith via Constructor"
        )),
        log.text()
    );

    let pop = descendants::<Button>(&page)
        .into_iter()
        .find(|button| button.content().is_some_and(|content| content.downcast_ref::<String>().is_some_and(|text| text == "Pop")))
        .expect("the pop button");
    click(&pop);
    assert_eq!(1, navigation.stack_depth());
    assert!(log.text().is_some_and(|text| text.ends_with("\nPopped \u{2190} Bob Smith")));

    // Through the data context.
    let description = page.get_control::<TextBlock>("MethodDescription");
    page.get_control::<ComboBox>("MethodCombo").set_selected_index(1);
    assert!(description.text().is_some_and(|text| text.starts_with("Data is passed by setting the new page's DataContext.")));
    click(&cards[4]);
    let detail = navigation.current_page().expect("the detail page");
    assert_eq!(Some(String::from("Emma Brown")), header(&detail));
    assert!(detail.is_set(StyledElement::data_context_property().as_property()));
    assert!(log.text().is_some_and(|text| text.ends_with("\nNavigated to Emma Brown via DataContext")));
    window.close();
}

#[test]
fn the_interactive_header_page_filters_the_contacts_by_the_text_of_the_header() {
    let _app = start_catalog_application();
    let page = NavigationPageInteractiveHeaderPage::new();
    let window = show(page.clone());
    let navigation = page.get_control::<NavigationPage>("DemoNav");
    assert_eq!(1, navigation.stack_depth());
    let contacts = navigation.current_page().expect("the page of the contacts");

    // The header of the page is a grid with the title and the search box.
    let header = contacts.header().and_then(|header| Control::from_boxed(&header)).expect("a control as the header");
    let search = descendants::<TextBox>(&header).pop().expect("the search box");
    let content = content_of(&contacts);
    let list = descendants::<ListBox>(&content).pop().expect("the list of the contacts");
    let label = descendants::<TextBlock>(&content).into_iter().next().expect("the label of the result");
    assert_eq!(20, list.item_count());
    assert_eq!(Some(String::from("20 contacts")), label.text());

    // Name and role match, whatever the case.
    search.set_text(Some("DESIGN"));
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(2, list.item_count());
    assert_eq!(Some(String::from("2 of 20 contacts")), label.text());

    search.set_text(Some("m\u{dc}ller"));
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(1, list.item_count());
    assert_eq!(Some(String::from("1 of 20 contacts")), label.text());

    search.set_text(Some(""));
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(20, list.item_count());
    assert_eq!(Some(String::from("20 contacts")), label.text());
    window.close();
}

#[test]
fn the_mvvm_page_navigates_through_the_commands_of_its_view_models() {
    let _app = start_catalog_application();
    let page = NavigationPageMvvmPage::new();
    let window = show(page.clone());
    let navigation = page.get_control::<NavigationPage>("DemoNav");
    let shell = from_markup_value::<Rc<NavigationPageMvvmShellViewModel>>(&page.data_context())
        .expect("the view model of the page");
    // The clock of the tests does not tick: a push with a transition would not complete.
    navigation.set_page_transition(None);
    assert_eq!(1, navigation.stack_depth());
    assert_eq!("Workspace", shell.current_page_header());
    assert_eq!(1, shell.navigation_depth());
    assert_eq!("Pushed Workspace", shell.last_action());

    // The side panel is bound to the view model.
    let side_panel = page.get_visual_descendants().find_map(|visual| visual.cast::<ScrollViewer>()).expect("the side panel");
    let texts: Vec<_> = descendants::<TextBlock>(&side_panel).iter().filter_map(|text| text.text()).collect();
    for expected in ["Current page: Workspace", "Stack depth: 1", "Last action: Pushed Workspace", "Status: Ready for QA"] {
        assert!(texts.iter().any(|text| text == expected), "{expected} is not shown in {texts:?}");
    }
    let projects = descendants::<ListBox>(&side_panel).pop().expect("the list of the projects");
    assert_eq!(3, projects.item_count());
    assert_eq!(0, projects.selected_index());

    // The workspace page shows a card per project; its button opens the detail page.
    let workspace = navigation.current_page().expect("the workspace page");
    let open: Vec<_> = descendants::<Button>(&content_of(&workspace));
    assert_eq!(3, open.len());
    open[2].command().expect("the command of the card").execute(None);
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(2, navigation.stack_depth());
    assert_eq!(Some(String::from("Docs Refresh")), current_header(&navigation));
    assert_eq!("Pushed Docs Refresh", shell.last_action());
    assert_eq!(2, shell.navigation_depth());

    // The detail page opens the activity page.
    let detail = navigation.current_page().expect("the detail page");
    let open_activity = descendants::<Button>(&content_of(&detail)).pop().expect("the button of the activity");
    open_activity.command().expect("the command of the detail").execute(None);
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(3, navigation.stack_depth());
    assert_eq!("Activity", shell.current_page_header());

    shell.go_back_command().execute(None);
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(2, navigation.stack_depth());
    assert_eq!("Popped Activity", shell.last_action());

    shell.pop_to_root_command().execute(None);
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(1, navigation.stack_depth());
    assert_eq!("Popped to root", shell.last_action());
    assert_eq!("Workspace", shell.current_page_header());

    shell.go_back_command().execute(None);
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(1, navigation.stack_depth());
    assert_eq!("Already at the root page", shell.last_action());

    // The selection of the list is the selected project of the view model.
    projects.set_selected_index(1);
    assert!(shell.selected_project().is_some_and(|selected| Rc::ptr_eq(&selected, &shell.projects()[1])));
    shell.open_selected_project_command().execute(None);
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(Some(String::from("Support Console")), current_header(&navigation));
    window.close();
}

#[test]
fn the_curved_header_page_shapes_the_dome_and_opens_the_profile_page() {
    let _app = start_catalog_application();
    let page = NavigationPageCurvedHeaderPage::new();
    let window = show(page.clone());
    let navigation = page.get_control::<NavigationPage>("NavPage");
    assert_eq!(1, navigation.stack_depth());
    assert!(page.get_control::<ScrollViewer>("InfoPanel").is_visible());

    // The dome is as wide as the header panel.
    let home = navigation.current_page().expect("the home page");
    assert!(!NavigationPage::get_has_navigation_bar(&home));
    let root = content_of(&home).cast::<Panel>().expect("the root panel of the home page");
    let header_panel = root.children().get(1).cast::<Panel>().expect("the header panel");
    let dome = header_panel.children().get(0).cast::<Path>().expect("the path of the dome");
    let bounds = dome.data().expect("the geometry of the dome").bounds();
    assert!(bounds.width > 1.0);
    assert_eq!(162.0, bounds.height);

    // A product of the home view opens the profile page.
    let product = descendants::<Button>(&content_of(&home)).into_iter().next().expect("a button of the home view");
    click(&product);
    assert_eq!(2, navigation.stack_depth());

    // Below 650 the panel with the notes is hidden.
    window.set_width(600.0);
    Dispatcher::ui_thread().run_jobs(None);
    assert!(!page.get_control::<ScrollViewer>("InfoPanel").is_visible());
    window.close();
}

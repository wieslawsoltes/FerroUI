//! The AppNotification, BadgeNotificationManager and JumpList pages: the notifications the
//! source builds, shown by the window's notification manager; the badge of the app on the
//! previews of its icon; and the jump list as the dock menu of the application. Nothing here
//! reaches the notification center, the badge or the dock of the system.

use crate::notification_samples::{
    self, badge_count, badge_description, current_badge, dock_menu, invoked_text, load_current,
    saved_text, support_info, Badge, JumpListItem, BADGE_GLYPHS, DEFAULT_GLYPH, SOUND_EVENTS,
};
use crate::{examples, tests::start};
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::{BoxedValue, ObjectType, Ref};
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::shapes::Ellipse;
use ferroui_controls::{
    Application, Button, ComboBox, Control, FontIconSource, Image, InfoBadge, InfoBar, NativeDock,
    NativeMenu, NativeMenuItem, NativeMenuItemSeparator, NotificationCard, NumberBox, ProgressBar,
    TextBlock, TextBox, Window,
};

fn page(id: &str) -> Ref<Control> {
    examples::create(examples::find(id).unwrap()).unwrap_or_else(|error| panic!("{id}: {error}"))
}

fn click(root: &Control, name: &str) {
    root.get_control::<Button>(name)
        .raise_event(&RoutedEventArgs::with_event(Button::click_event()));
}

/// The element of `name` among the visual descendants of `root` (the content of a notification is
/// built in Rust, without a name scope).
fn named<T: ObjectType>(root: &Control, name: &str) -> Ref<T> {
    root.get_visual_descendants()
        .filter_map(|visual| visual.cast::<Control>())
        .find(|control| control.name().as_deref() == Some(name))
        .and_then(|control| control.cast::<T>())
        .unwrap_or_else(|| panic!("the element {name}"))
}

fn text(root: &Control, name: &str) -> String {
    named::<TextBlock>(root, name).text().unwrap_or_default()
}

fn texts(root: &Control, name: &str) -> Vec<String> {
    root.get_visual_descendants()
        .filter_map(|visual| visual.cast::<TextBlock>())
        .filter(|block| block.name().as_deref() == Some(name))
        .map(|block| block.text().unwrap_or_default())
        .collect()
}

fn string(value: Option<BoxedValue>) -> Option<String> {
    value
        .as_ref()
        .and_then(|value| value.downcast_ref::<String>())
        .cloned()
}

#[test]
fn app_notification_builder_lays_out_the_source_notifications() {
    let _application = start();

    let basic = notification_samples::basic_notification();
    assert!(!basic.keep_open);
    assert_eq!(
        text(&basic.content, "NotificationTitle"),
        "Welcome to WinUI Gallery"
    );
    assert_eq!(
        texts(&basic.content, "NotificationText"),
        ["Explore interactive samples and discover the power of modern Windows UI."]
    );

    let informational = notification_samples::informational_notification("Reminder");
    assert_eq!(
        text(&informational.content, "NotificationTitle"),
        "Control Highlight: PersonPicture"
    );
    let logo = named::<Ellipse>(&informational.content, "NotificationAppLogo");
    assert_eq!(
        (logo.width(), logo.height()),
        (48.0, 48.0),
        "a circle of a notification's logo size"
    );
    assert_eq!(
        text(&informational.content, "NotificationSoundEvent"),
        "Sound: Reminder"
    );
    assert!(!text(&informational.content, "NotificationTimeStamp").is_empty());

    let visual = notification_samples::visual_notification();
    assert_eq!(
        named::<Image>(&visual.content, "NotificationHeroImage").height(),
        150.0
    );
    assert_eq!(
        text(&visual.content, "NotificationAttribution"),
        "WinUI gallery assets"
    );
    assert_eq!(
        text(&visual.content, "NotificationTitle"),
        "Harbor Scene with Boats"
    );

    let controls = notification_samples::controls_notification();
    assert!(
        controls.keep_open,
        "a notification with inputs waits for them"
    );
    let satisfaction = named::<ComboBox>(&controls.content, "satisfaction");
    assert_eq!(satisfaction.item_count(), 5);
    assert_eq!(satisfaction.selected_index(), 2);
    assert_eq!(
        string(satisfaction.selected_item()).as_deref(),
        Some("Neutral")
    );
    let comment = named::<TextBox>(&controls.content, "comment");
    assert_eq!(
        comment.placeholder_text().as_deref(),
        Some("Leave a comment here...")
    );
    assert_eq!(comment.text().unwrap_or_default(), "");
    let submit = named::<Button>(&controls.content, "NotificationButton0");
    assert_eq!(string(submit.content()).as_deref(), Some("Submit"));
    assert_eq!(
        string(submit.command_parameter()).as_deref(),
        Some("action=submit_survey")
    );

    let progress = notification_samples::progress_notification();
    assert!(!progress.keep_open);
    let bar = named::<ProgressBar>(&progress.content, "NotificationProgressBar");
    assert_eq!((bar.value(), bar.maximum()), (0.6, 1.0));
    assert_eq!(
        text(&progress.content, "NotificationProgressTitle"),
        "Demo Progress"
    );
    assert_eq!(
        text(&progress.content, "NotificationProgressStatus"),
        "In progress..."
    );
    assert_eq!(text(&progress.content, "NotificationProgressValue"), "60%");
}

#[test]
fn app_notification_page_keeps_the_source_options_and_says_where_it_shows() {
    let _application = start();
    let root = page("AppNotification");
    let sound_events = root.get_control::<ComboBox>("AppNotificationSoundEventsCombo");
    assert_eq!(sound_events.item_count(), SOUND_EVENTS.len() as i32);
    assert_eq!(sound_events.selected_index(), 0);
    root.get_control::<InfoBar>("NotificationHostInfoBar");
    // A page that is not in a window has no notification manager: the buttons show nothing.
    click(&root, "ShowNotificationButton");
    assert!(notification_samples::notification_manager(&root).is_none());
}

/// The cards of the window's notification manager, in the order they were shown.
fn cards(window: &Ref<Window>) -> Vec<Ref<NotificationCard>> {
    window.update_layout();
    window
        .get_visual_descendants()
        .filter_map(|visual| visual.cast::<NotificationCard>())
        .collect()
}

fn card_content(card: &NotificationCard) -> Ref<Control> {
    card.content()
        .as_ref()
        .and_then(Control::from_boxed)
        .expect("the content of the card")
}

/// The clock of the cards' closing animation: it never pulses, so a closing card stays closing.
#[derive(Default)]
struct IdleClock {
    ticks: ferroui_base::reactive::LightweightSubject<ferroui_base::animation::SignedDuration>,
    state: std::cell::Cell<ferroui_base::animation::PlayState>,
}

impl ferroui_base::reactive::IObservable<ferroui_base::animation::SignedDuration> for IdleClock {
    fn subscribe(
        &self,
        observer: std::rc::Rc<
            dyn ferroui_base::reactive::IObserver<ferroui_base::animation::SignedDuration>,
        >,
    ) -> std::rc::Rc<dyn ferroui_base::reactive::IDisposable> {
        self.ticks.subscribe(observer)
    }
}

impl ferroui_base::animation::IClock for IdleClock {
    fn play_state(&self) -> ferroui_base::animation::PlayState {
        self.state.get()
    }

    fn set_play_state(&self, value: ferroui_base::animation::PlayState) {
        self.state.set(value);
    }
}

impl ferroui_base::animation::IGlobalClock for IdleClock {}

/// [`start`] with a global clock, which the closing card's animation needs.
fn start_with_clock() -> ferroui_controls::testing::UnitTestApplicationScope {
    ferroui_markup_xaml::register_types();
    let services = ferroui_controls::testing::TestServices::styled_window()
        .with_global_clock(std::rc::Rc::new(IdleClock::default()));
    ferroui_controls::testing::UnitTestApplication::start(services)
}

#[test]
fn app_notification_buttons_show_the_source_notifications_in_the_window() {
    let _application = start_with_clock();
    let root = page("AppNotification");
    let window = Window::new();
    window.set_content(Some(Control::boxed(root.clone())));
    window.show();
    window.update_layout();

    click(&root, "ShowNotificationButton");
    let manager = notification_samples::notification_manager(&root)
        .expect("the window's notification manager");
    let shown = cards(&window);
    assert_eq!(shown.len(), 1);
    assert_eq!(
        text(&card_content(&shown[0]), "NotificationTitle"),
        "Welcome to WinUI Gallery"
    );

    // The selected sound event (Reminder) is the one the card names.
    root.get_control::<ComboBox>("AppNotificationSoundEventsCombo")
        .set_selected_index(2);
    click(&root, "ShowInformationalNotificationWithLogoButton");
    let shown = cards(&window);
    assert_eq!(shown.len(), 2);
    assert_eq!(
        text(&card_content(&shown[1]), "NotificationSoundEvent"),
        "Sound: Reminder"
    );
    assert!(
        notification_samples::notification_manager(&root).is_some_and(|again| again == manager),
        "one manager per window"
    );

    click(&root, "ShowNotificationWithControlsButton");
    let shown = cards(&window);
    assert_eq!(shown.len(), 3);
    let survey = shown[2].clone();
    assert!(!survey.is_closing());
    named::<Button>(&card_content(&survey), "NotificationButton0")
        .raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    assert!(survey.is_closing(), "Submit dismisses the notification");
    window.close();
}

#[test]
fn badge_buttons_set_the_count_and_clear_it_on_the_preview() {
    let _application = start();
    notification_samples::reset();
    assert_eq!(
        (badge_count(f64::NAN), badge_count(-1.0), badge_count(3.7)),
        (0, 0, 3)
    );

    let root = page("BadgeNotificationManager");
    let badge = root.get_control::<InfoBadge>("BadgeCountInfoBadge");
    let count_box = root.get_control::<NumberBox>("BadgeCountBox");
    assert!(!badge.is_visible());
    // BadgeCountBox_ValueChanged changes nothing before a button set the badge.
    count_box.set_numeric_value(9.0);
    assert_eq!(current_badge(), None);
    count_box.set_numeric_value(5.0);

    click(&root, "SetBadgeCountButton");
    assert_eq!(current_badge(), Some(Badge::Count(5)));
    assert!(badge.is_visible());
    assert_eq!(badge.value(), 5);
    let glyph_preview = root.get_control::<InfoBadge>("BadgeGlyphInfoBadge");
    assert_eq!(
        glyph_preview.value(),
        5,
        "both previews show the app's badge"
    );
    assert_eq!(
        AutomationProperties::get_name(&root.get_control::<Control>("BadgeCountPreview"))
            .as_deref(),
        Some("App icon, badge 5")
    );

    count_box.set_numeric_value(7.0);
    assert_eq!(current_badge(), Some(Badge::Count(7)));
    assert_eq!(badge.value(), 7);

    // The badge of the app is kept when the page is opened again.
    let again = page("BadgeNotificationManager");
    assert_eq!(
        again
            .get_control::<InfoBadge>("BadgeCountInfoBadge")
            .value(),
        7
    );

    click(&root, "ClearBadgeCountButton");
    assert_eq!(current_badge(), None);
    assert!(!badge.is_visible());
    assert_eq!(badge_description(None), "App icon");
}

#[test]
fn badge_buttons_set_the_glyph_and_the_selection_follows_once_set() {
    let _application = start();
    notification_samples::reset();
    let root = page("BadgeNotificationManager");
    let glyphs = root.get_control::<ComboBox>("BadgeNotificationGlyphsCombo");
    assert_eq!(glyphs.item_count(), BADGE_GLYPHS.len() as i32);
    assert_eq!(glyphs.selected_index(), DEFAULT_GLYPH as i32);
    assert_eq!(BADGE_GLYPHS[DEFAULT_GLYPH].0, "Activity");
    // BadgeNotificationGlyphsCombo_SelectionChanged changes nothing before a button set the badge.
    glyphs.set_selected_index(8);
    assert_eq!(current_badge(), None);
    glyphs.set_selected_index(DEFAULT_GLYPH as i32);

    click(&root, "SetBadgeGlyphButton");
    assert_eq!(current_badge(), Some(Badge::Glyph(DEFAULT_GLYPH)));
    let badge = root.get_control::<InfoBadge>("BadgeGlyphInfoBadge");
    assert!(badge.is_visible());
    assert_eq!(badge.value(), -1, "a glyph badge shows its icon");
    let glyph = |badge: &InfoBadge| {
        badge
            .icon_source()
            .and_then(|icon| icon.cast::<FontIconSource>())
            .map(|icon| icon.glyph())
    };
    assert_eq!(glyph(&badge).as_deref(), BADGE_GLYPHS[DEFAULT_GLYPH].1);

    glyphs.set_selected_index(8);
    assert_eq!(current_badge(), Some(Badge::Glyph(8)));
    assert_eq!(BADGE_GLYPHS[8].0, "Error");
    assert_eq!(glyph(&badge).as_deref(), BADGE_GLYPHS[8].1);
    assert_eq!(badge_description(current_badge()), "App icon, badge Error");

    // BadgeNotificationGlyph.None shows no badge.
    glyphs.set_selected_index(0);
    assert_eq!(current_badge(), None);
    assert!(!badge.is_visible());

    glyphs.set_selected_index(9);
    assert_eq!(current_badge(), Some(Badge::Glyph(9)));
    click(&root, "ClearBadgeGlyphButton");
    assert_eq!(current_badge(), None);
}

fn menu_items(menu: &NativeMenu) -> Vec<String> {
    menu.iter()
        .map(|item| {
            if item.clone().cast::<NativeMenuItemSeparator>().is_some() {
                return "-".to_string();
            }
            let item = item.cast::<NativeMenuItem>().expect("a menu item");
            let header = item.header().unwrap_or_default();
            if item.is_enabled() {
                header
            } else {
                format!("[{header}]")
            }
        })
        .collect()
}

fn app_dock_menu() -> Ref<NativeMenu> {
    let application = Application::current().expect("the application");
    NativeDock::get_menu(&application).expect("the dock menu of the application")
}

#[test]
fn jump_list_tasks_become_the_dock_menu_and_clear_empties_it() {
    let _application = start();
    notification_samples::reset();
    let root = page("JumpList");
    click(&root, "AddTasksButton");
    assert_eq!(load_current().len(), 2);
    let menu = app_dock_menu();
    assert_eq!(menu_items(&menu), ["New Message", "Search"]);
    let compose = menu
        .iter()
        .next()
        .and_then(|item| item.cast::<NativeMenuItem>())
        .unwrap();
    assert_eq!(compose.tool_tip().as_deref(), Some("Compose a new message"));
    assert_eq!(
        root.get_control::<TextBlock>("JumpListStatus")
            .text()
            .as_deref(),
        Some(saved_text(2).as_str())
    );

    // The source's handler adds its tasks again on each click.
    click(&root, "AddTasksButton");
    assert_eq!(
        menu_items(&app_dock_menu()),
        ["New Message", "Search", "New Message", "Search"]
    );

    click(&root, "ClearTasksButton");
    assert!(load_current().is_empty());
    assert!(menu_items(&app_dock_menu()).is_empty());
    assert_eq!(
        root.get_control::<TextBlock>("JumpListStatus")
            .text()
            .as_deref(),
        Some(saved_text(0).as_str())
    );
}

#[test]
fn jump_list_groups_come_before_the_tasks_under_their_header() {
    let _application = start();
    notification_samples::reset();
    let root = page("JumpList");
    click(&root, "AddTasksButton");
    click(&root, "AddCustomGroupButton");
    assert_eq!(
        menu_items(&app_dock_menu()),
        [
            "[Projects]",
            "Project Alpha",
            "Project Beta",
            "-",
            "New Message",
            "Search"
        ]
    );
    let items = load_current();
    assert_eq!(items[2].arguments, "/project-alpha");
    assert_eq!(items[2].group_name, "Projects");
    assert_eq!(items[3].description, "Open Project Beta");

    // Groups only: no separator.
    let groups = vec![
        JumpListItem::create_with_arguments("/a", "A"),
        JumpListItem::create_with_arguments("/b", "B"),
    ]
    .into_iter()
    .map(|mut item| {
        item.group_name = "G".into();
        item
    })
    .collect::<Vec<_>>();
    assert_eq!(menu_items(&dock_menu(&groups)), ["[G]", "A", "B"]);
}

#[test]
fn jump_list_item_reports_its_arguments_when_invoked() {
    let _application = start();
    notification_samples::reset();
    let root = page("JumpList");
    click(&root, "AddTasksButton");
    let search = app_dock_menu()
        .iter()
        .nth(1)
        .and_then(|item| item.cast::<NativeMenuItem>())
        .unwrap();
    search.to_exporter_events_bridge().raise_clicked();
    assert_eq!(
        root.get_control::<TextBlock>("JumpListStatus")
            .text()
            .as_deref(),
        Some(invoked_text("/search").as_str())
    );
}

#[test]
fn jump_list_page_says_where_the_jump_list_is_shown() {
    let _application = start();
    let root = page("JumpList");
    let info_bar = root.get_control::<InfoBar>("JumpListSupportInfoBar");
    let (severity, title, message) = support_info();
    assert_eq!(
        (info_bar.severity(), info_bar.title(), info_bar.message()),
        (severity, title.into(), message.into())
    );
    assert_eq!(
        notification_samples::is_supported(),
        cfg!(target_os = "macos")
    );
}

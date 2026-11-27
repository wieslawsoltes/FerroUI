//! The code behind of the AppNotification, BadgeNotificationManager and JumpList pages (the
//! handlers of `AppNotificationPage.xaml.cs`, `BadgeNotificationManagerPage.xaml.cs` and
//! `JumpListPage.xaml.cs`) over the services FerroUI has for them.
//!
//! FerroUI has no service for the notifications of the operating system, for the badge of the
//! app's icon or for the jump lists of Windows. An `AppNotificationBuilder` of the source builds
//! the content of a card that the window's `WindowNotificationManager` shows; the badge of
//! `BadgeNotificationManager.Current` is shown by an `InfoBadge` on a preview of the app's icon;
//! and the jump list of the app is its dock menu (`NativeDock.Menu` of the application), which
//! the macOS platform shows. Everything the pages draw is ordinary controls, drawn by each
//! backend's GPU renderer; nothing is sent to the system's notification center.

use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::media::{FontFamily, FontWeight, IBrush, ImageBrush, Stretch, TextWrapping};
use ferroui_base::utilities::{CultureInfo, DateTime};
use ferroui_base::{BoxedValue, Ref, WeakRef};
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::primitives::SelectingItemsControl;
use ferroui_controls::shapes::Ellipse;
use ferroui_controls::{
    Application, Button, ColumnDefinitions, ComboBox, Control, FontIconSource, Grid, Image,
    InfoBadge, InfoBar, InfoBarSeverity, NativeDock, NativeMenu, NativeMenuItem,
    NativeMenuItemSeparator, NotificationCard, NotificationPosition, NotificationType, NumberBox,
    ProgressBar, StackPanel, TextBlock, TextBox, TopLevel, WindowNotificationManager,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

pub(crate) fn wire(page_id: &str, root: &Ref<Control>) {
    match page_id {
        "AppNotification" => wire_app_notification(root),
        "BadgeNotificationManager" => wire_badge_notification_manager(root),
        "JumpList" => wire_jump_list(root),
        _ => {}
    }
}

// --- AppNotification ---

/// The source's `appNotificationSoundEvents`: the members of `AppNotificationSoundEvent` the
/// ComboBox lists, `Default` selected.
pub(crate) const SOUND_EVENTS: [&str; 6] = ["Default", "IM", "Reminder", "SMS", "Alarm", "Call"];

/// The crop of an image of a notification (`AppNotificationImageCrop`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum AppNotificationImageCrop {
    #[default]
    Default,
    Circle,
}

/// An `AppNotificationComboBox`: an input of the notification with its items (id and content)
/// and the id of the selected item.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AppNotificationComboBox {
    pub id: &'static str,
    pub items: Vec<(&'static str, &'static str)>,
    pub selected_item: Option<&'static str>,
}

impl AppNotificationComboBox {
    pub(crate) fn new(id: &'static str) -> Self {
        Self {
            id,
            items: Vec::new(),
            selected_item: None,
        }
    }

    pub(crate) fn add_item(mut self, id: &'static str, content: &'static str) -> Self {
        self.items.push((id, content));
        self
    }

    pub(crate) fn set_selected_item(mut self, id: &'static str) -> Self {
        self.selected_item = Some(id);
        self
    }
}

/// An `AppNotificationButton`: its content and its arguments (`AddArgument`), which the button
/// of the card carries as its command parameter (`key=value;key=value`, the arguments string of
/// a Windows notification).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AppNotificationButton {
    pub content: &'static str,
    pub arguments: Vec<(&'static str, &'static str)>,
}

impl AppNotificationButton {
    pub(crate) fn new(content: &'static str) -> Self {
        Self {
            content,
            arguments: Vec::new(),
        }
    }

    pub(crate) fn add_argument(mut self, key: &'static str, value: &'static str) -> Self {
        self.arguments.push((key, value));
        self
    }

    pub(crate) fn argument_string(&self) -> String {
        self.arguments
            .iter()
            .map(|(key, value)| format!("{key}={value}"))
            .collect::<Vec<_>>()
            .join(";")
    }
}

/// An `AppNotificationProgressBar`: its title, its value from 0 to 1, the text shown for the value
/// and its status.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct AppNotificationProgressBar {
    pub title: &'static str,
    pub value: f64,
    pub value_string_override: &'static str,
    pub status: &'static str,
}

/// The source's `AppNotificationBuilder`: what a notification shows, built into the content of a
/// card of the window's notification manager.
#[derive(Clone, Debug, Default)]
pub(crate) struct AppNotificationBuilder {
    pub texts: Vec<&'static str>,
    pub app_logo: Option<(&'static str, AppNotificationImageCrop)>,
    pub audio_event: Option<&'static str>,
    pub time_stamp: Option<DateTime>,
    pub hero_image: Option<&'static str>,
    pub attribution_text: Option<&'static str>,
    pub combo_boxes: Vec<AppNotificationComboBox>,
    pub text_boxes: Vec<(&'static str, &'static str, &'static str)>,
    pub buttons: Vec<AppNotificationButton>,
    pub progress_bar: Option<AppNotificationProgressBar>,
}

impl AppNotificationBuilder {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn add_text(mut self, text: &'static str) -> Self {
        self.texts.push(text);
        self
    }

    pub(crate) fn set_app_logo_override(
        mut self,
        uri: &'static str,
        crop: AppNotificationImageCrop,
    ) -> Self {
        self.app_logo = Some((uri, crop));
        self
    }

    pub(crate) fn set_audio_event(mut self, event: &'static str) -> Self {
        self.audio_event = Some(event);
        self
    }

    pub(crate) fn set_time_stamp(mut self, time: DateTime) -> Self {
        self.time_stamp = Some(time);
        self
    }

    pub(crate) fn set_hero_image(mut self, uri: &'static str) -> Self {
        self.hero_image = Some(uri);
        self
    }

    pub(crate) fn set_attribution_text(mut self, text: &'static str) -> Self {
        self.attribution_text = Some(text);
        self
    }

    pub(crate) fn add_combo_box(mut self, combo_box: AppNotificationComboBox) -> Self {
        self.combo_boxes.push(combo_box);
        self
    }

    pub(crate) fn add_text_box(
        mut self,
        id: &'static str,
        placeholder: &'static str,
        title: &'static str,
    ) -> Self {
        self.text_boxes.push((id, placeholder, title));
        self
    }

    pub(crate) fn add_button(mut self, button: AppNotificationButton) -> Self {
        self.buttons.push(button);
        self
    }

    pub(crate) fn add_progress_bar(mut self, progress_bar: AppNotificationProgressBar) -> Self {
        self.progress_bar = Some(progress_bar);
        self
    }

    /// `BuildNotification`: the content of the card. A notification with inputs stays until it
    /// is closed or a button is clicked, as a Windows notification with inputs waits for them; the
    /// others close after the manager's time.
    pub(crate) fn build_notification(self) -> AppNotification {
        let content = StackPanel::new();
        content.set_spacing(8.0);
        content.set_name(Some("NotificationContent".into()));

        if let Some(uri) = self.hero_image {
            let hero = Image::new();
            hero.set_name(Some("NotificationHeroImage".into()));
            hero.set_height(150.0);
            hero.set_stretch(Stretch::UniformToFill);
            if let Ok(bitmap) = crate::assets::load_bitmap(uri) {
                hero.set_source(Some(Rc::new(bitmap)));
            }
            content.children().add(hero);
        }

        let header = Grid::new();
        header.set_column_definitions(
            ColumnDefinitions::parse("Auto,*").expect("the columns of the header"),
        );
        header.set_column_spacing(12.0);
        let texts = StackPanel::new();
        texts.set_spacing(2.0);
        if let Some((uri, crop)) = self.app_logo {
            let logo = app_logo(uri, crop);
            logo.set_vertical_alignment(VerticalAlignment::Top);
            header.children().add(logo);
            Grid::set_column(&texts, 1);
        } else {
            Grid::set_column_span(&texts, 2);
        }
        for (index, text) in self.texts.iter().enumerate() {
            let block = text_block(
                text,
                if index == 0 {
                    "NotificationTitle"
                } else {
                    "NotificationText"
                },
            );
            if index == 0 {
                block.set_font_weight(FontWeight::SemiBold);
            }
            texts.children().add(block);
        }
        if let Some(attribution) = self.attribution_text {
            let block = caption(attribution, "NotificationAttribution");
            texts.children().add(block);
        }
        if let Some(time) = self.time_stamp {
            let block = caption(
                &time.to_string_format("t", &CultureInfo::current_formatting()),
                "NotificationTimeStamp",
            );
            texts.children().add(block);
        }
        if let Some(event) = self.audio_event {
            texts.children().add(caption(
                &format!("Sound: {event}"),
                "NotificationSoundEvent",
            ));
        }
        header.children().add(texts);
        content.children().add(header);

        for input in &self.combo_boxes {
            let combo = ComboBox::new();
            combo.set_name(Some(input.id.into()));
            combo.set_horizontal_alignment(HorizontalAlignment::Stretch);
            for (_, item) in &input.items {
                combo
                    .items()
                    .add(Some(Rc::new(item.to_string()) as BoxedValue));
            }
            if let Some(selected) = input.selected_item {
                if let Some(index) = input.items.iter().position(|(id, _)| *id == selected) {
                    combo.set_selected_index(index as i32);
                }
            }
            content.children().add(combo);
        }
        for (id, placeholder, text) in &self.text_boxes {
            let text_box = TextBox::new();
            text_box.set_name(Some(id.to_string()));
            text_box.set_placeholder_text(Some(placeholder));
            text_box.set_text(Some(*text));
            content.children().add(text_box);
        }

        if let Some(progress) = &self.progress_bar {
            content.children().add(progress_bar(progress));
        }

        let keep_open = !self.combo_boxes.is_empty() || !self.text_boxes.is_empty();
        if !self.buttons.is_empty() {
            let row = Grid::new();
            let columns = vec!["*"; self.buttons.len()].join(",");
            row.set_column_definitions(
                ColumnDefinitions::parse(&columns).expect("the columns of the buttons"),
            );
            row.set_column_spacing(8.0);
            let weak_content = content.downgrade();
            for (index, source) in self.buttons.iter().enumerate() {
                let button = Button::new();
                button.set_name(Some(format!("NotificationButton{index}")));
                button.set_content(Some(Rc::new(source.content.to_string()) as BoxedValue));
                button.set_horizontal_alignment(HorizontalAlignment::Stretch);
                button.set_command_parameter(Some(Rc::new(source.argument_string()) as BoxedValue));
                Grid::set_column(&button, index as i32);
                let weak_content = weak_content.clone();
                button.click(move |_, _| {
                    // A click on a button of a Windows notification dismisses it and activates the
                    // app with the button's arguments, which the source's app does not handle.
                    if let Some(card) = weak_content.upgrade().and_then(|content| {
                        content.find_ancestor_of_type::<NotificationCard>(false)
                    }) {
                        card.close();
                    }
                });
                row.children().add(button);
            }
            content.children().add(row);
        }

        AppNotification { content, keep_open }
    }
}

/// A built notification: the content of its card, and whether it stays until it is closed.
pub(crate) struct AppNotification {
    pub content: Ref<StackPanel>,
    pub keep_open: bool,
}

fn text_block(text: &str, name: &str) -> Ref<TextBlock> {
    let block = TextBlock::new();
    block.set_name(Some(name.into()));
    block.set_text(Some(text));
    block.set_text_wrapping(TextWrapping::Wrap);
    block
}

fn caption(text: &str, name: &str) -> Ref<TextBlock> {
    let block = text_block(text, name);
    block.set_font_size(12.0);
    block.set_opacity(0.8);
    block
}

/// `SetAppLogoOverride`: the logo at the size of a Windows notification's logo (48), a circle
/// with `AppNotificationImageCrop.Circle`.
fn app_logo(uri: &str, crop: AppNotificationImageCrop) -> Ref<Control> {
    let bitmap = crate::assets::load_bitmap(uri).ok().map(Rc::new);
    let logo: Ref<Control> = match crop {
        AppNotificationImageCrop::Circle => {
            let ellipse = Ellipse::new();
            if let Some(bitmap) = bitmap {
                let brush = ImageBrush::new();
                brush.set_stretch(Stretch::UniformToFill);
                brush.set_source(Some(bitmap));
                let brush: Rc<dyn IBrush> = brush.into();
                ellipse.set_fill(Some(brush));
            }
            ellipse.upcast()
        }
        AppNotificationImageCrop::Default => {
            let image = Image::new();
            image.set_stretch(Stretch::Uniform);
            if let Some(bitmap) = bitmap {
                image.set_source(Some(bitmap));
            }
            image.upcast()
        }
    };
    logo.set_name(Some("NotificationAppLogo".into()));
    logo.set_width(48.0);
    logo.set_height(48.0);
    logo
}

/// `AddProgressBar`: the title above the bar, the status below it on the left and the value's
/// text on the right, as a Windows notification lays them out.
fn progress_bar(progress: &AppNotificationProgressBar) -> Ref<Control> {
    let panel = StackPanel::new();
    panel.set_spacing(4.0);
    panel
        .children()
        .add(text_block(progress.title, "NotificationProgressTitle"));
    let bar = ProgressBar::new();
    bar.set_name(Some("NotificationProgressBar".into()));
    bar.set_minimum(0.0);
    bar.set_maximum(1.0);
    bar.set_range_value(progress.value);
    panel.children().add(bar);
    let row = Grid::new();
    row.set_column_definitions(
        ColumnDefinitions::parse("*,Auto").expect("the columns of the status"),
    );
    row.children()
        .add(caption(progress.status, "NotificationProgressStatus"));
    let value = caption(progress.value_string_override, "NotificationProgressValue");
    Grid::set_column(&value, 1);
    row.children().add(value);
    panel.children().add(row);
    panel.upcast()
}

ferroui_base::ferro_thread_local! {
    /// The notification manager of each window the page showed a notification in: the source's
    /// `AppNotificationManager.Default` of the app is the manager of the page's window.
    static MANAGERS: RefCell<Vec<(WeakRef<TopLevel>, Ref<WindowNotificationManager>)>> =
        const { RefCell::new(Vec::new()) };
}

/// The notification manager of the window of `root`, created when the page shows its first
/// notification there; `None` for a page that is not in a window.
pub(crate) fn notification_manager(root: &Control) -> Option<Ref<WindowNotificationManager>> {
    let top_level = TopLevel::get_top_level(Some(root))?;
    MANAGERS.with(|managers| {
        let mut managers = managers.borrow_mut();
        managers.retain(|(window, _)| window.upgrade().is_some());
        if let Some((_, manager)) = managers
            .iter()
            .find(|(window, _)| window.upgrade().as_ref() == Some(&top_level))
        {
            return Some(manager.clone());
        }
        let manager = WindowNotificationManager::with_host(Some(&top_level));
        // The corner of the screen where Windows shows its notifications.
        manager.set_position(NotificationPosition::BottomRight);
        // The cards of the first notification go to the items panel of the manager's template.
        manager.apply_template();
        managers.push((top_level.downgrade(), manager.clone()));
        Some(manager)
    })
}

/// `AppNotificationManager.Default.Show(notification)`: the card in the window of `root`.
pub(crate) fn show(root: &Control, notification: AppNotification) {
    let Some(manager) = notification_manager(root) else {
        return;
    };
    let expiration = notification.keep_open.then_some(Duration::ZERO);
    manager.show_with(
        Control::boxed(notification.content),
        NotificationType::Information,
        expiration,
        None,
        None,
        None,
    );
}

/// `ShowNotificationButton_Click`.
pub(crate) fn basic_notification() -> AppNotification {
    AppNotificationBuilder::new()
        .add_text("Welcome to WinUI Gallery")
        .add_text("Explore interactive samples and discover the power of modern Windows UI.")
        .build_notification()
}

/// `ShowInformationalNotificationWithLogoButton_Click` with the selected sound event.
pub(crate) fn informational_notification(sound_event: &'static str) -> AppNotification {
    AppNotificationBuilder::new()
        .add_text("Control Highlight: PersonPicture")
        .add_text("Use the PersonPicture control to display user avatars with initials or images.")
        .set_app_logo_override(
            "ferres://WinUIGallery/Assets/ControlImages/PersonPicture.png",
            AppNotificationImageCrop::Circle,
        )
        .set_audio_event(sound_event)
        .set_time_stamp(DateTime::now())
        .build_notification()
}

/// `ShowVisualNotificationWithHeroImageButton_Click`.
pub(crate) fn visual_notification() -> AppNotification {
    AppNotificationBuilder::new()
        .add_text("Harbor Scene with Boats")
        .add_text("A quiet harbor with boats gently anchored in view.")
        .set_hero_image("ferres://WinUIGallery/Assets/SampleMedia/LandscapeImage5.jpg")
        .set_attribution_text("WinUI gallery assets")
        .build_notification()
}

/// `ShowNotificationWithControlsButton_Click`.
pub(crate) fn controls_notification() -> AppNotification {
    AppNotificationBuilder::new()
        .add_text("Survey")
        .add_text("Please select your satisfaction level and leave a comment.")
        .add_combo_box(
            AppNotificationComboBox::new("satisfaction")
                .add_item("1", "Very Bad")
                .add_item("2", "Bad")
                .add_item("3", "Neutral")
                .add_item("4", "Good")
                .add_item("5", "Excellent")
                .set_selected_item("3"),
        )
        .add_text_box("comment", "Leave a comment here...", "")
        .add_button(AppNotificationButton::new("Submit").add_argument("action", "submit_survey"))
        .build_notification()
}

/// `ShowNotificationWithProgressBarButton_Click`.
pub(crate) fn progress_notification() -> AppNotification {
    AppNotificationBuilder::new()
        .add_text("Progress Bar Example")
        .add_text("This is a sample notification showing how to use a progress bar.")
        .add_progress_bar(AppNotificationProgressBar {
            title: "Demo Progress",
            value: 0.6,
            value_string_override: "60%",
            status: "In progress...",
        })
        .build_notification()
}

fn wire_app_notification(root: &Ref<Control>) {
    let sound_events = root.get_control::<ComboBox>("AppNotificationSoundEventsCombo");
    let weak_sound_events = sound_events.downgrade();
    let selected_sound_event = move || {
        weak_sound_events
            .upgrade()
            .and_then(|combo| usize::try_from(combo.selected_index()).ok())
            .and_then(|index| SOUND_EVENTS.get(index).copied())
            .unwrap_or(SOUND_EVENTS[0])
    };
    let selected_sound_event = Rc::new(selected_sound_event);
    let buttons: [(&str, Rc<dyn Fn() -> AppNotification>); 5] = [
        ("ShowNotificationButton", Rc::new(basic_notification)),
        ("ShowInformationalNotificationWithLogoButton", {
            let selected_sound_event = selected_sound_event.clone();
            Rc::new(move || informational_notification(selected_sound_event()))
        }),
        (
            "ShowVisualNotificationWithHeroImageButton",
            Rc::new(visual_notification),
        ),
        (
            "ShowNotificationWithControlsButton",
            Rc::new(controls_notification),
        ),
        (
            "ShowNotificationWithProgressBarButton",
            Rc::new(progress_notification),
        ),
    ];
    for (name, build) in buttons {
        let weak_root = root.downgrade();
        root.get_control::<Button>(name).click(move |_, _| {
            if let Some(root) = weak_root.upgrade() {
                show(&root, build());
            }
        });
    }
}

// --- BadgeNotificationManager ---

/// The members of `BadgeNotificationGlyph` in the order of `Enum.GetValues`, with the Segoe Fluent
/// Icons glyph of the same meaning that the preview shows (`None` shows no badge).
pub(crate) const BADGE_GLYPHS: [(&str, Option<&str>); 13] = [
    ("None", None),
    ("Activity", Some("\u{E895}")),
    ("Alarm", Some("\u{EA8F}")),
    ("Alert", Some("\u{E7BA}")),
    ("Attention", Some("\u{E8C9}")),
    ("Available", Some("\u{E73E}")),
    ("Away", Some("\u{E823}")),
    ("Busy", Some("\u{E733}")),
    ("Error", Some("\u{E783}")),
    ("NewMessage", Some("\u{E715}")),
    ("Paused", Some("\u{E769}")),
    ("Playing", Some("\u{E768}")),
    ("Unavailable", Some("\u{E711}")),
];

/// The source's `selectedGlyph`: `BadgeNotificationGlyph.Activity`.
pub(crate) const DEFAULT_GLYPH: usize = 1;

/// The badge of the app: a count, or a glyph (an index of [`BADGE_GLYPHS`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Badge {
    Count(u32),
    Glyph(usize),
}

ferroui_base::ferro_thread_local! {
    /// `BadgeNotificationManager.Current`: the badge of the app, kept when the page is opened
    /// again.
    static CURRENT_BADGE: Cell<Option<Badge>> = const { Cell::new(None) };
}

/// The badge the app shows.
pub(crate) fn current_badge() -> Option<Badge> {
    CURRENT_BADGE.with(Cell::get)
}

/// `SetBadgeAsCount`: a count of 0 shows no badge, as on Windows.
pub(crate) fn set_badge_as_count(count: u32) {
    CURRENT_BADGE.with(|badge| badge.set((count > 0).then_some(Badge::Count(count))));
}

/// `SetBadgeAsGlyph`: `BadgeNotificationGlyph.None` shows no badge.
pub(crate) fn set_badge_as_glyph(glyph: usize) {
    let badge = BADGE_GLYPHS
        .get(glyph)
        .and_then(|(_, code)| code.map(|_| Badge::Glyph(glyph)));
    CURRENT_BADGE.with(|current| current.set(badge));
}

/// `ClearBadge`.
pub(crate) fn clear_badge() {
    CURRENT_BADGE.with(|badge| badge.set(None));
}

/// The source's `(uint)BadgeCountBox.Value`: a value that is not a number, or below 0, is 0.
pub(crate) fn badge_count(value: f64) -> u32 {
    if value.is_finite() && value >= 0.0 {
        value.trunc().min(f64::from(u32::MAX)) as u32
    } else {
        0
    }
}

/// The text of the preview's automation name for a badge.
pub(crate) fn badge_description(badge: Option<Badge>) -> String {
    match badge {
        None => "App icon".to_string(),
        Some(Badge::Count(count)) => format!("App icon, badge {count}"),
        Some(Badge::Glyph(glyph)) => format!("App icon, badge {}", BADGE_GLYPHS[glyph].0),
    }
}

/// Shows the app's badge on the previews of the page.
fn show_badge(root: &Control) {
    let badge = current_badge();
    for (preview, info_badge) in [
        ("BadgeCountPreview", "BadgeCountInfoBadge"),
        ("BadgeGlyphPreview", "BadgeGlyphInfoBadge"),
    ] {
        let info_badge = root.get_control::<InfoBadge>(info_badge);
        match badge {
            None => {
                info_badge.set_is_visible(false);
                info_badge.set_icon_source(None);
                info_badge.set_numeric_value(-1);
            }
            Some(Badge::Count(count)) => {
                info_badge.set_icon_source(None);
                info_badge.set_numeric_value(i32::try_from(count).unwrap_or(i32::MAX));
                info_badge.set_is_visible(true);
            }
            Some(Badge::Glyph(glyph)) => {
                let icon = FontIconSource::new();
                icon.set_font_family(FontFamily::new("Segoe Fluent Icons"));
                icon.set_glyph(BADGE_GLYPHS[glyph].1.unwrap_or_default());
                info_badge.set_numeric_value(-1);
                info_badge.set_icon_source(Some(icon.upcast()));
                info_badge.set_is_visible(true);
            }
        }
        AutomationProperties::set_name(
            &root.get_control::<Control>(preview),
            Some(&badge_description(badge)),
        );
    }
}

fn wire_badge_notification_manager(root: &Ref<Control>) {
    // The source's `isBadgeSetted`: the options change the badge once a button of the page set it.
    let is_badge_set = Rc::new(Cell::new(false));
    let count_box = root.get_control::<NumberBox>("BadgeCountBox");
    let glyphs = root.get_control::<ComboBox>("BadgeNotificationGlyphsCombo");
    let weak_count_box = count_box.downgrade();
    let count = move || {
        weak_count_box
            .upgrade()
            .map_or(0, |count_box| badge_count(count_box.value()))
    };
    let weak_glyphs = glyphs.downgrade();
    let glyph = move || {
        weak_glyphs
            .upgrade()
            .and_then(|glyphs| usize::try_from(glyphs.selected_index()).ok())
            .filter(|index| *index < BADGE_GLYPHS.len())
            .unwrap_or(DEFAULT_GLYPH)
    };
    let count: Rc<dyn Fn() -> u32> = Rc::new(count);
    let glyph: Rc<dyn Fn() -> usize> = Rc::new(glyph);

    let on_click = |name: &str, handler: Rc<dyn Fn()>| {
        let weak_root = root.downgrade();
        root.get_control::<Button>(name).click(move |_, _| {
            handler();
            if let Some(root) = weak_root.upgrade() {
                show_badge(&root);
            }
        });
    };
    // SetBadgeCountButton_Click.
    on_click("SetBadgeCountButton", {
        let (count, is_badge_set) = (count.clone(), is_badge_set.clone());
        Rc::new(move || {
            set_badge_as_count(count());
            is_badge_set.set(true);
        })
    });
    // SetBadgeGlyphButton_Click.
    on_click("SetBadgeGlyphButton", {
        let (glyph, is_badge_set) = (glyph.clone(), is_badge_set.clone());
        Rc::new(move || {
            set_badge_as_glyph(glyph());
            is_badge_set.set(true);
        })
    });
    // ClearBadgeButton_Click, the handler of both examples' Clear badge buttons.
    on_click("ClearBadgeCountButton", Rc::new(clear_badge));
    on_click("ClearBadgeGlyphButton", Rc::new(clear_badge));

    // BadgeCountBox_ValueChanged.
    let weak_root = root.downgrade();
    let set = is_badge_set.clone();
    count_box
        .value_changed(move |_| {
            if set.get() {
                set_badge_as_count(count());
                if let Some(root) = weak_root.upgrade() {
                    show_badge(&root);
                }
            }
        })
        .detach();
    // BadgeNotificationGlyphsCombo_SelectionChanged.
    let weak_root = root.downgrade();
    glyphs.property_changed(move |change| {
        if change.property() != SelectingItemsControl::selected_index_property().as_property() {
            return;
        }
        if is_badge_set.get() {
            set_badge_as_glyph(glyph());
            if let Some(root) = weak_root.upgrade() {
                show_badge(&root);
            }
        }
    });
    show_badge(root);
}

// --- JumpList ---

/// A `JumpListItem`: the arguments the app is given, the name shown, the description and the
/// group (an empty group name is a task).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct JumpListItem {
    pub arguments: String,
    pub display_name: String,
    pub description: String,
    pub group_name: String,
}

impl JumpListItem {
    /// `JumpListItem.CreateWithArguments`.
    pub(crate) fn create_with_arguments(arguments: &str, display_name: &str) -> Self {
        Self {
            arguments: arguments.into(),
            display_name: display_name.into(),
            description: String::new(),
            group_name: String::new(),
        }
    }

    fn described(mut self, description: &str) -> Self {
        self.description = description.into();
        self
    }

    fn in_group(mut self, group_name: &str) -> Self {
        self.group_name = group_name.into();
        self
    }
}

ferroui_base::ferro_thread_local! {
    /// The saved jump list of the app (`JumpList.LoadCurrentAsync` reads it, `SaveAsync` writes it).
    static JUMP_LIST: RefCell<Vec<JumpListItem>> = const { RefCell::new(Vec::new()) };
    /// The status line of the page that is open, where an invoked item's arguments are reported.
    static JUMP_LIST_STATUS: RefCell<Option<WeakRef<TextBlock>>> = const { RefCell::new(None) };
}

/// `JumpList.IsSupported`: the dock menu of the app is shown on macOS only.
pub(crate) fn is_supported() -> bool {
    cfg!(target_os = "macos")
}

/// `JumpList.LoadCurrentAsync`: the items of the saved list.
pub(crate) fn load_current() -> Vec<JumpListItem> {
    JUMP_LIST.with(|list| list.borrow().clone())
}

/// `JumpList.SaveAsync`: the list becomes the dock menu of the app.
pub(crate) fn save(items: Vec<JumpListItem>) {
    let menu = dock_menu(&items);
    JUMP_LIST.with(|list| *list.borrow_mut() = items);
    if let Some(application) = Application::current() {
        NativeDock::set_menu(&application, Some(menu));
    }
}

/// The dock menu of a jump list: the items of each group, in the order the groups first appear,
/// below a disabled header with the group's name; then the tasks, after a separator when there
/// are groups. The description is the item's tool tip.
pub(crate) fn dock_menu(items: &[JumpListItem]) -> Ref<NativeMenu> {
    let menu = NativeMenu::new();
    let mut groups: Vec<&str> = Vec::new();
    for item in items {
        if !item.group_name.is_empty() && !groups.contains(&item.group_name.as_str()) {
            groups.push(&item.group_name);
        }
    }
    for group in &groups {
        let header = NativeMenuItem::with_header(group);
        header.set_is_enabled(false);
        menu.add(header);
        for item in items.iter().filter(|item| item.group_name == *group) {
            menu.add(menu_item(item));
        }
    }
    let tasks: Vec<_> = items
        .iter()
        .filter(|item| item.group_name.is_empty())
        .collect();
    if !groups.is_empty() && !tasks.is_empty() {
        menu.add(NativeMenuItemSeparator::new());
    }
    for item in tasks {
        menu.add(menu_item(item));
    }
    menu
}

fn menu_item(item: &JumpListItem) -> Ref<NativeMenuItem> {
    let menu_item = NativeMenuItem::with_header(&item.display_name);
    if !item.description.is_empty() {
        menu_item.set_tool_tip(Some(item.description.clone()));
    }
    let arguments = item.arguments.clone();
    menu_item.click(move |_| invoked(&arguments)).detach();
    menu_item
}

/// The text of the status line when the item with `arguments` is chosen in the dock menu.
pub(crate) fn invoked_text(arguments: &str) -> String {
    format!("The app was activated from its jump list with the arguments {arguments}.")
}

/// A jump list item launches the app with its arguments; the app is running when its dock menu is
/// used, so the open page reports them.
fn invoked(arguments: &str) {
    let status =
        JUMP_LIST_STATUS.with(|status| status.borrow().as_ref().and_then(WeakRef::upgrade));
    if let Some(status) = status {
        status.set_text(Some(&invoked_text(arguments)));
    }
}

/// The text of the status line after the list is saved.
pub(crate) fn saved_text(count: usize) -> String {
    let items = if count == 1 {
        "1 item".to_string()
    } else {
        format!("{count} items")
    };
    if is_supported() {
        format!("The jump list has {items}: they are in the dock menu of WinUI Gallery.")
    } else {
        format!("The jump list has {items}; this platform does not show them.")
    }
}

/// The InfoBar of the page: where the jump list is shown, or why it is not.
pub(crate) fn support_info() -> (InfoBarSeverity, &'static str, &'static str) {
    if is_supported() {
        (
            InfoBarSeverity::Informational,
            "The jump list is the dock menu of the app.",
            "Right-click the icon of WinUI Gallery in the Dock, or click and hold it, to see the items \
             the examples add.",
        )
    } else {
        (
            InfoBarSeverity::Warning,
            "JumpList is not available on this platform.",
            "FerroUI shows the jump list of an app as its dock menu on macOS; this platform has no jump \
             list that FerroUI sets.",
        )
    }
}

fn wire_jump_list(root: &Ref<Control>) {
    let (severity, title, message) = support_info();
    let info_bar = root.get_control::<InfoBar>("JumpListSupportInfoBar");
    info_bar.set_severity(severity);
    info_bar.set_title(title);
    info_bar.set_message(message);
    let status = root.get_control::<TextBlock>("JumpListStatus");
    JUMP_LIST_STATUS.with(|current| *current.borrow_mut() = Some(status.downgrade()));

    let on_click = |name: &str, handler: fn(&mut Vec<JumpListItem>)| {
        let weak_status = status.downgrade();
        root.get_control::<Button>(name).click(move |_, _| {
            let mut items = load_current();
            handler(&mut items);
            let count = items.len();
            save(items);
            if let Some(status) = weak_status.upgrade() {
                status.set_text(Some(&saved_text(count)));
            }
        });
    };
    on_click("AddTasksButton", add_tasks);
    on_click("ClearTasksButton", Vec::clear);
    on_click("AddCustomGroupButton", add_custom_group);
}

/// `AddTasksButton_Click`. The source's logo (`AppList.targetsize-48.png`) is not in the gallery's
/// assets: the menu items have no image.
pub(crate) fn add_tasks(items: &mut Vec<JumpListItem>) {
    items.push(
        JumpListItem::create_with_arguments("/compose", "New Message")
            .described("Compose a new message"),
    );
    items.push(
        JumpListItem::create_with_arguments("/search", "Search").described("Search for items"),
    );
}

/// `AddCustomGroupButton_Click`.
pub(crate) fn add_custom_group(items: &mut Vec<JumpListItem>) {
    items.push(
        JumpListItem::create_with_arguments("/project-alpha", "Project Alpha")
            .in_group("Projects")
            .described("Open Project Alpha"),
    );
    items.push(
        JumpListItem::create_with_arguments("/project-beta", "Project Beta")
            .in_group("Projects")
            .described("Open Project Beta"),
    );
}

/// Clears the app's jump list and its dock menu (for the tests).
#[cfg(test)]
pub(crate) fn reset() {
    JUMP_LIST.with(|list| list.borrow_mut().clear());
    clear_badge();
    MANAGERS.with(|managers| managers.borrow_mut().clear());
}

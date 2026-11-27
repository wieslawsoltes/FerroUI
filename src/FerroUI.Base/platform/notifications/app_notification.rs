//! What a notification shows (`AppNotification`) and its builder (`AppNotificationBuilder`).

use crate::utilities::DateTime;
use std::fmt::Write;

/// The crop of an image of a notification (`AppNotificationImageCrop`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AppNotificationImageCrop {
    /// The image as it is.
    #[default]
    Default,
    /// The image cropped to a circle.
    Circle,
}

/// The sound a notification plays (`AppNotificationSoundEvent`). Each platform plays the sound of
/// its own that comes closest, or its default sound.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AppNotificationSoundEvent {
    /// The default sound of a notification.
    #[default]
    Default,
    /// An instant message.
    IM,
    /// An e-mail.
    Mail,
    /// A reminder.
    Reminder,
    /// A text message.
    SMS,
    /// An alarm.
    Alarm,
    /// An incoming call.
    Call,
}

impl AppNotificationSoundEvent {
    /// Every sound event, in the order of the enumeration.
    pub const ALL: [AppNotificationSoundEvent; 7] = [
        Self::Default,
        Self::IM,
        Self::Mail,
        Self::Reminder,
        Self::SMS,
        Self::Alarm,
        Self::Call,
    ];

    /// The name of the member (`ToString`).
    pub const fn name(self) -> &'static str {
        match self {
            Self::Default => "Default",
            Self::IM => "IM",
            Self::Mail => "Mail",
            Self::Reminder => "Reminder",
            Self::SMS => "SMS",
            Self::Alarm => "Alarm",
            Self::Call => "Call",
        }
    }

    /// The sound event of a Windows notification (`ms-winsoundevent:`).
    pub const fn uri(self) -> &'static str {
        match self {
            Self::Default => "ms-winsoundevent:Notification.Default",
            Self::IM => "ms-winsoundevent:Notification.IM",
            Self::Mail => "ms-winsoundevent:Notification.Mail",
            Self::Reminder => "ms-winsoundevent:Notification.Reminder",
            Self::SMS => "ms-winsoundevent:Notification.SMS",
            Self::Alarm => "ms-winsoundevent:Notification.Looping.Alarm",
            Self::Call => "ms-winsoundevent:Notification.Looping.Call",
        }
    }
}

/// How the platform treats a notification (`AppNotificationScenario`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AppNotificationScenario {
    /// An ordinary notification.
    #[default]
    Default,
    /// A reminder, which stays on the screen until the user acts on it.
    Reminder,
    /// An alarm, which stays on the screen and repeats its sound.
    Alarm,
    /// An incoming call.
    IncomingCall,
    /// An important notification, shown even when the user does not want to be disturbed where
    /// the platform allows it.
    Urgent,
}

impl AppNotificationScenario {
    /// The `scenario` of a Windows notification; `None` for the default.
    pub const fn toast_scenario(self) -> Option<&'static str> {
        match self {
            Self::Default => None,
            Self::Reminder => Some("reminder"),
            Self::Alarm => Some("alarm"),
            Self::IncomingCall => Some("incomingCall"),
            Self::Urgent => Some("urgent"),
        }
    }
}

/// A button of a notification (`AppNotificationButton`): its content and the arguments the
/// application is activated with when it is clicked.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AppNotificationButton {
    content: String,
    arguments: Vec<(String, String)>,
    input_id: Option<String>,
}

impl AppNotificationButton {
    /// A button that shows `content`.
    pub fn new(content: impl Into<String>) -> Self {
        Self { content: content.into(), ..Self::default() }
    }

    /// Adds an argument the application is given when the button is clicked (`AddArgument`).
    pub fn add_argument(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.arguments.push((key.into(), value.into()));
        self
    }

    /// Places the button next to the text box `id` (`SetInputId`).
    pub fn set_input_id(mut self, id: impl Into<String>) -> Self {
        self.input_id = Some(id.into());
        self
    }

    /// What the button shows.
    pub fn content(&self) -> &str {
        &self.content
    }

    /// The arguments of the button, in the order they were added.
    pub fn arguments(&self) -> &[(String, String)] {
        &self.arguments
    }

    /// The arguments as one string: `key=value` pairs separated by `;`, with `%`, `;` and `=`
    /// of the keys and values escaped (the arguments string of a Windows notification).
    pub fn argument_string(&self) -> String {
        argument_string(&self.arguments)
    }

    /// The text box the button is placed next to.
    pub fn input_id(&self) -> Option<&str> {
        self.input_id.as_deref()
    }
}

/// A list the user picks an item of (`AppNotificationComboBox`): the id of the input, its items
/// (id and content) and the id of the item selected at first.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AppNotificationComboBox {
    id: String,
    title: Option<String>,
    items: Vec<(String, String)>,
    selected_item: Option<String>,
}

impl AppNotificationComboBox {
    /// A combo box whose selection the application is given as the input `id`.
    pub fn new(id: impl Into<String>) -> Self {
        Self { id: id.into(), ..Self::default() }
    }

    /// Adds an item (`AddItem`).
    pub fn add_item(mut self, id: impl Into<String>, content: impl Into<String>) -> Self {
        self.items.push((id.into(), content.into()));
        self
    }

    /// Selects the item `id` (`SetSelectedItem`).
    pub fn set_selected_item(mut self, id: impl Into<String>) -> Self {
        self.selected_item = Some(id.into());
        self
    }

    /// Sets the title shown above the combo box (`SetTitle`).
    pub fn set_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// The id of the input.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// The title shown above the combo box.
    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    /// The items: their ids and what they show.
    pub fn items(&self) -> &[(String, String)] {
        &self.items
    }

    /// The id of the item selected at first.
    pub fn selected_item(&self) -> Option<&str> {
        self.selected_item.as_deref()
    }
}

/// A text box of a notification (`AddTextBox`): the id of the input, the text shown while it is
/// empty and the title shown above it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AppNotificationTextBox {
    id: String,
    placeholder_text: String,
    title: String,
}

impl AppNotificationTextBox {
    /// A text box whose text the application is given as the input `id`.
    pub fn new(
        id: impl Into<String>,
        placeholder_text: impl Into<String>,
        title: impl Into<String>,
    ) -> Self {
        Self { id: id.into(), placeholder_text: placeholder_text.into(), title: title.into() }
    }

    /// The id of the input.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// The text shown while the text box is empty.
    pub fn placeholder_text(&self) -> &str {
        &self.placeholder_text
    }

    /// The title shown above the text box.
    pub fn title(&self) -> &str {
        &self.title
    }
}

/// The progress bar of a notification (`AppNotificationProgressBar`): its title, its value from 0
/// to 1, the text shown for the value and its status.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AppNotificationProgressBar {
    title: Option<String>,
    value: f64,
    value_string_override: Option<String>,
    status: String,
}

impl AppNotificationProgressBar {
    /// A progress bar at 0.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the title shown above the bar.
    pub fn set_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Sets the value, from 0 to 1 (clamped).
    pub fn set_value(mut self, value: f64) -> Self {
        self.value = if value.is_nan() { 0.0 } else { value.clamp(0.0, 1.0) };
        self
    }

    /// Sets the text shown for the value, in place of the percentage.
    pub fn set_value_string_override(mut self, text: impl Into<String>) -> Self {
        self.value_string_override = Some(text.into());
        self
    }

    /// Sets the status shown below the bar.
    pub fn set_status(mut self, status: impl Into<String>) -> Self {
        self.status = status.into();
        self
    }

    /// The title shown above the bar.
    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    /// The value, from 0 to 1.
    pub fn value(&self) -> f64 {
        self.value
    }

    /// The text shown for the value, in place of the percentage.
    pub fn value_string_override(&self) -> Option<&str> {
        self.value_string_override.as_deref()
    }

    /// The status shown below the bar.
    pub fn status(&self) -> &str {
        &self.status
    }
}

/// What a notification shows, built by an [`AppNotificationBuilder`] (`AppNotification`).
///
/// The first text is the title, the others are the body. Each platform shows what its
/// notification center can: see the platform crates for what they leave out.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AppNotification {
    pub(super) id: u32,
    tag: String,
    group: String,
    texts: Vec<String>,
    attribution_text: Option<String>,
    app_logo: Option<(String, AppNotificationImageCrop)>,
    hero_image: Option<String>,
    time_stamp: Option<DateTime>,
    audio_event: Option<AppNotificationSoundEvent>,
    mute_audio: bool,
    scenario: AppNotificationScenario,
    arguments: Vec<(String, String)>,
    buttons: Vec<AppNotificationButton>,
    text_boxes: Vec<AppNotificationTextBox>,
    combo_boxes: Vec<AppNotificationComboBox>,
    progress: Option<AppNotificationProgressBar>,
}

impl AppNotification {
    /// The id the manager gave the notification when it showed it; 0 before.
    pub fn id(&self) -> u32 {
        self.id
    }

    /// The tag of the notification: a notification shown with the tag (and group) of one that
    /// is shown replaces it.
    pub fn tag(&self) -> &str {
        &self.tag
    }

    /// Sets the tag.
    pub fn set_tag(&mut self, tag: impl Into<String>) {
        self.tag = tag.into();
    }

    /// The group of the notification.
    pub fn group(&self) -> &str {
        &self.group
    }

    /// Sets the group.
    pub fn set_group(&mut self, group: impl Into<String>) {
        self.group = group.into();
    }

    /// The texts: the title first, then the lines of the body.
    pub fn texts(&self) -> &[String] {
        &self.texts
    }

    /// The title: the first text.
    pub fn title(&self) -> &str {
        self.texts.first().map_or("", String::as_str)
    }

    /// The body: the texts after the first, one per line.
    pub fn body(&self) -> String {
        self.texts.get(1..).unwrap_or_default().join("\n")
    }

    /// The text that says where the notification comes from, below the texts.
    pub fn attribution_text(&self) -> Option<&str> {
        self.attribution_text.as_deref()
    }

    /// The URI of the logo that replaces the app's, and its crop.
    pub fn app_logo_override(&self) -> Option<(&str, AppNotificationImageCrop)> {
        self.app_logo.as_ref().map(|(uri, crop)| (uri.as_str(), *crop))
    }

    /// The URI of the image shown above the texts.
    pub fn hero_image(&self) -> Option<&str> {
        self.hero_image.as_deref()
    }

    /// The time the notification shows in place of the time it was shown.
    pub fn time_stamp(&self) -> Option<DateTime> {
        self.time_stamp
    }

    /// The sound the notification plays; `None` for the default.
    pub fn audio_event(&self) -> Option<AppNotificationSoundEvent> {
        self.audio_event
    }

    /// Whether the notification plays no sound.
    pub fn is_audio_muted(&self) -> bool {
        self.mute_audio
    }

    /// How the platform treats the notification.
    pub fn scenario(&self) -> AppNotificationScenario {
        self.scenario
    }

    /// The arguments the application is given when the notification itself is clicked.
    pub fn arguments(&self) -> &[(String, String)] {
        &self.arguments
    }

    /// The arguments as one string (see [`AppNotificationButton::argument_string`]).
    pub fn argument_string(&self) -> String {
        argument_string(&self.arguments)
    }

    /// The buttons.
    pub fn buttons(&self) -> &[AppNotificationButton] {
        &self.buttons
    }

    /// The text boxes.
    pub fn text_boxes(&self) -> &[AppNotificationTextBox] {
        &self.text_boxes
    }

    /// The combo boxes.
    pub fn combo_boxes(&self) -> &[AppNotificationComboBox] {
        &self.combo_boxes
    }

    /// The progress bar.
    pub fn progress_bar(&self) -> Option<&AppNotificationProgressBar> {
        self.progress.as_ref()
    }

    /// Whether the notification has inputs, which keep it until the user acts on it.
    pub fn has_inputs(&self) -> bool {
        !self.text_boxes.is_empty() || !self.combo_boxes.is_empty()
    }

    /// The XML of the notification as Windows shows it (`Payload`): the generic template with
    /// the texts, the images, the progress bar, the inputs, the buttons and the sound.
    ///
    /// An image is in the XML only when Windows can load its URI (`file:`, `http:`, `https:`,
    /// `ms-appx:` and `ms-appdata:`): Windows shows no notification whose image it cannot load.
    pub fn payload(&self) -> String {
        let mut xml = String::from("<toast");
        let launch = self.argument_string();
        if !launch.is_empty() {
            attribute(&mut xml, "launch", &launch);
        }
        if let Some(scenario) = self.scenario.toast_scenario() {
            attribute(&mut xml, "scenario", scenario);
        }
        if let Some(time) = self.time_stamp {
            attribute(&mut xml, "displayTimestamp", &iso_8601_utc(time));
        }
        xml.push_str("><visual><binding template=\"ToastGeneric\">");
        for text in &self.texts {
            element_text(&mut xml, "text", "", text);
        }
        if let Some(attribution) = &self.attribution_text {
            element_text(&mut xml, "text", " placement=\"attribution\"", attribution);
        }
        if let Some((uri, crop)) = self.app_logo.as_ref().filter(|(uri, _)| is_toast_image_uri(uri)) {
            xml.push_str("<image placement=\"appLogoOverride\"");
            attribute(&mut xml, "src", uri);
            if *crop == AppNotificationImageCrop::Circle {
                xml.push_str(" hint-crop=\"circle\"");
            }
            xml.push_str("/>");
        }
        if let Some(uri) = self.hero_image.as_deref().filter(|uri| is_toast_image_uri(uri)) {
            xml.push_str("<image placement=\"hero\"");
            attribute(&mut xml, "src", uri);
            xml.push_str("/>");
        }
        if let Some(progress) = &self.progress {
            xml.push_str("<progress");
            if let Some(title) = &progress.title {
                attribute(&mut xml, "title", title);
            }
            attribute(&mut xml, "value", &progress.value.to_string());
            if let Some(text) = &progress.value_string_override {
                attribute(&mut xml, "valueStringOverride", text);
            }
            attribute(&mut xml, "status", &progress.status);
            xml.push_str("/>");
        }
        xml.push_str("</binding></visual>");
        if self.has_inputs() || !self.buttons.is_empty() {
            xml.push_str("<actions>");
            for text_box in &self.text_boxes {
                xml.push_str("<input type=\"text\"");
                attribute(&mut xml, "id", &text_box.id);
                if !text_box.placeholder_text.is_empty() {
                    attribute(&mut xml, "placeHolderContent", &text_box.placeholder_text);
                }
                if !text_box.title.is_empty() {
                    attribute(&mut xml, "title", &text_box.title);
                }
                xml.push_str("/>");
            }
            for combo_box in &self.combo_boxes {
                xml.push_str("<input type=\"selection\"");
                attribute(&mut xml, "id", &combo_box.id);
                if let Some(title) = &combo_box.title {
                    attribute(&mut xml, "title", title);
                }
                if let Some(selected) = &combo_box.selected_item {
                    attribute(&mut xml, "defaultInput", selected);
                }
                xml.push('>');
                for (id, content) in &combo_box.items {
                    xml.push_str("<selection");
                    attribute(&mut xml, "id", id);
                    attribute(&mut xml, "content", content);
                    xml.push_str("/>");
                }
                xml.push_str("</input>");
            }
            for button in &self.buttons {
                xml.push_str("<action");
                attribute(&mut xml, "content", &button.content);
                attribute(&mut xml, "arguments", &button.argument_string());
                if let Some(input_id) = &button.input_id {
                    attribute(&mut xml, "hint-inputId", input_id);
                }
                xml.push_str("/>");
            }
            xml.push_str("</actions>");
        }
        if self.mute_audio {
            xml.push_str("<audio silent=\"true\"/>");
        } else if let Some(event) = self.audio_event {
            xml.push_str("<audio");
            attribute(&mut xml, "src", event.uri());
            xml.push_str("/>");
        }
        xml.push_str("</toast>");
        xml
    }
}

/// Builds an [`AppNotification`] (`AppNotificationBuilder`).
#[derive(Clone, Debug, Default)]
pub struct AppNotificationBuilder {
    notification: AppNotification,
}

impl AppNotificationBuilder {
    /// A builder of a notification that shows nothing yet.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a text (`AddText`): the first is the title, the others the lines of the body.
    pub fn add_text(mut self, text: impl Into<String>) -> Self {
        self.notification.texts.push(text.into());
        self
    }

    /// Sets the text that says where the notification comes from (`SetAttributionText`).
    pub fn set_attribution_text(mut self, text: impl Into<String>) -> Self {
        self.notification.attribution_text = Some(text.into());
        self
    }

    /// Replaces the app's logo with the image at `uri` (`SetAppLogoOverride`).
    pub fn set_app_logo_override(mut self, uri: impl Into<String>, crop: AppNotificationImageCrop) -> Self {
        self.notification.app_logo = Some((uri.into(), crop));
        self
    }

    /// Shows the image at `uri` above the texts (`SetHeroImage`).
    pub fn set_hero_image(mut self, uri: impl Into<String>) -> Self {
        self.notification.hero_image = Some(uri.into());
        self
    }

    /// Shows `time` in place of the time the notification is shown (`SetTimeStamp`).
    pub fn set_time_stamp(mut self, time: DateTime) -> Self {
        self.notification.time_stamp = Some(time);
        self
    }

    /// Sets the sound the notification plays (`SetAudioEvent`).
    pub fn set_audio_event(mut self, event: AppNotificationSoundEvent) -> Self {
        self.notification.audio_event = Some(event);
        self.notification.mute_audio = false;
        self
    }

    /// Plays no sound (`MuteAudio`).
    pub fn mute_audio(mut self) -> Self {
        self.notification.mute_audio = true;
        self
    }

    /// Sets how the platform treats the notification (`SetScenario`).
    pub fn set_scenario(mut self, scenario: AppNotificationScenario) -> Self {
        self.notification.scenario = scenario;
        self
    }

    /// Adds an argument the application is given when the notification is clicked
    /// (`AddArgument`).
    pub fn add_argument(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.notification.arguments.push((key.into(), value.into()));
        self
    }

    /// Adds a button (`AddButton`).
    pub fn add_button(mut self, button: AppNotificationButton) -> Self {
        self.notification.buttons.push(button);
        self
    }

    /// Adds a text box (`AddTextBox`).
    pub fn add_text_box(
        mut self,
        id: impl Into<String>,
        placeholder_text: impl Into<String>,
        title: impl Into<String>,
    ) -> Self {
        self.notification.text_boxes.push(AppNotificationTextBox::new(id, placeholder_text, title));
        self
    }

    /// Adds a combo box (`AddComboBox`).
    pub fn add_combo_box(mut self, combo_box: AppNotificationComboBox) -> Self {
        self.notification.combo_boxes.push(combo_box);
        self
    }

    /// Adds the progress bar (`AddProgressBar`); a notification has one.
    pub fn add_progress_bar(mut self, progress_bar: AppNotificationProgressBar) -> Self {
        self.notification.progress = Some(progress_bar);
        self
    }

    /// Sets the tag of the notification (`SetTag`).
    pub fn set_tag(mut self, tag: impl Into<String>) -> Self {
        self.notification.tag = tag.into();
        self
    }

    /// Sets the group of the notification (`SetGroup`).
    pub fn set_group(mut self, group: impl Into<String>) -> Self {
        self.notification.group = group.into();
        self
    }

    /// The notification (`BuildNotification`).
    pub fn build_notification(self) -> AppNotification {
        self.notification
    }
}

/// `key=value` pairs separated by `;`, escaped.
fn argument_string(arguments: &[(String, String)]) -> String {
    let mut text = String::new();
    for (index, (key, value)) in arguments.iter().enumerate() {
        if index > 0 {
            text.push(';');
        }
        escape_argument(&mut text, key);
        if !value.is_empty() {
            text.push('=');
            escape_argument(&mut text, value);
        }
    }
    text
}

fn escape_argument(text: &mut String, value: &str) {
    for c in value.chars() {
        match c {
            '%' => text.push_str("%25"),
            ';' => text.push_str("%3B"),
            '=' => text.push_str("%3D"),
            c => text.push(c),
        }
    }
}

/// The arguments of an arguments string, unescaped (the inverse of the strings above).
pub(super) fn parse_arguments(text: &str) -> Vec<(String, String)> {
    text.split(';')
        .filter(|pair| !pair.is_empty())
        .map(|pair| match pair.split_once('=') {
            Some((key, value)) => (unescape_argument(key), unescape_argument(value)),
            None => (unescape_argument(pair), String::new()),
        })
        .collect()
}

fn unescape_argument(value: &str) -> String {
    value.replace("%3B", ";").replace("%3D", "=").replace("%25", "%")
}

fn is_toast_image_uri(uri: &str) -> bool {
    let scheme = uri.split_once(':').map_or("", |(scheme, _)| scheme);
    ["file", "http", "https", "ms-appx", "ms-appdata"]
        .iter()
        .any(|known| scheme.eq_ignore_ascii_case(known))
}

/// The time in UTC as ISO 8601 (`2026-11-27T10:30:00Z`).
fn iso_8601_utc(time: DateTime) -> String {
    let utc = time.to_universal_time();
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        utc.year(),
        utc.month(),
        utc.day(),
        utc.hour(),
        utc.minute(),
        utc.second()
    )
}

fn attribute(xml: &mut String, name: &str, value: &str) {
    let _ = write!(xml, " {name}=\"");
    escape_xml(xml, value);
    xml.push('"');
}

fn element_text(xml: &mut String, name: &str, attributes: &str, text: &str) {
    let _ = write!(xml, "<{name}{attributes}>");
    escape_xml(xml, text);
    let _ = write!(xml, "</{name}>");
}

fn escape_xml(xml: &mut String, text: &str) {
    for c in text.chars() {
        match c {
            '&' => xml.push_str("&amp;"),
            '<' => xml.push_str("&lt;"),
            '>' => xml.push_str("&gt;"),
            '"' => xml.push_str("&quot;"),
            '\'' => xml.push_str("&apos;"),
            c => xml.push(c),
        }
    }
}

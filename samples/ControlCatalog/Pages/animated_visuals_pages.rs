//! The pages of `AnimatedVisualPlayer` and `AnimatedIcon`, built in code.

use ferroui_animated_visuals::{
    AnimatedBackVisualSource, AnimatedChevronDownSmallVisualSource, AnimatedFindVisualSource, AnimatedIcon,
    AnimatedSettingsVisualSource, AnimatedVisualPlayer, AnimatedVisualSource,
};
use ferroui_base::layout::Orientation;
use ferroui_base::{BoxedValue, Ref, Thickness};
use ferroui_controls::{Button, CheckBox, ContentPage, Control, ProgressRing, StackPanel, TextBlock};
use std::rc::Rc;

fn label(text: &str) -> Ref<TextBlock> {
    let block = TextBlock::new();
    block.set_text(Some(text));
    block
}

fn page_with(header: &str, panel: Ref<StackPanel>) -> Ref<ContentPage> {
    panel.set_margin(Thickness::uniform(24.0));
    panel.set_spacing(16.0);
    let page = ContentPage::new();
    page.set_header(Some(Rc::new(header.to_owned()) as BoxedValue));
    page.set_content(Some(Control::boxed(&panel)));
    page
}

fn action(text: &str, player: &Ref<AnimatedVisualPlayer>, run: fn(&AnimatedVisualPlayer)) -> Ref<Button> {
    let button = Button::new();
    button.set_content(Some(Rc::new(text.to_owned()) as BoxedValue));
    let player = player.downgrade();
    button.click(move |_, _| {
        if let Some(player) = player.upgrade() {
            run(&player);
        }
    });
    button
}

/// A player with its transport.
pub fn player_page() -> Ref<ContentPage> {
    let panel = StackPanel::new();
    panel.children().add(&label("Plays a Lottie document as composition visuals."));
    let player = AnimatedVisualPlayer::new();
    player.set_name(Some("Player".to_string()));
    player.set_auto_play(false);
    player.set_width(192.0);
    player.set_height(192.0);
    player.set_source(Some(AnimatedSettingsVisualSource::new().upcast()));
    panel.children().add(&player);
    let transport = StackPanel::new();
    transport.set_orientation(Orientation::Horizontal);
    transport.set_spacing(8.0);
    transport.children().add(&action("Play", &player, |player| drop(player.play_async(0.0, 1.0, false))));
    transport.children().add(&action("Loop", &player, |player| drop(player.play_async(0.0, 1.0, true))));
    transport.children().add(&action("Pause", &player, AnimatedVisualPlayer::pause));
    transport.children().add(&action("Resume", &player, AnimatedVisualPlayer::resume));
    transport.children().add(&action("Stop", &player, AnimatedVisualPlayer::stop));
    panel.children().add(&transport);
    panel.children().add(&label("ProgressRing shows the visual WinUI ships for it."));
    panel.children().add(&ProgressRing::new());
    page_with("AnimatedVisualPlayer", panel)
}

fn icon(source: Ref<AnimatedVisualSource>) -> Ref<AnimatedIcon> {
    let icon = AnimatedIcon::new();
    icon.set_width(20.0);
    icon.set_height(20.0);
    icon.set_source(Some(source));
    icon
}

/// Icons in the controls that drive their states.
pub fn icon_page() -> Ref<ContentPage> {
    let panel = StackPanel::new();
    panel.children().add(&label("An icon follows the state of the control around it."));
    let row = StackPanel::new();
    row.set_orientation(Orientation::Horizontal);
    row.set_spacing(8.0);
    let sources: [Ref<AnimatedVisualSource>; 4] = [
        AnimatedBackVisualSource::new().upcast(),
        AnimatedFindVisualSource::new().upcast(),
        AnimatedSettingsVisualSource::new().upcast(),
        AnimatedChevronDownSmallVisualSource::new().upcast(),
    ];
    for source in sources {
        let button = Button::new();
        button.set_content(Some(Control::boxed(&icon(source))));
        row.children().add(&button);
    }
    panel.children().add(&row);
    let check = CheckBox::new();
    check.set_content(Some(Rc::new("A check box".to_owned()) as BoxedValue));
    panel.children().add(&check);
    page_with("AnimatedIcon", panel)
}

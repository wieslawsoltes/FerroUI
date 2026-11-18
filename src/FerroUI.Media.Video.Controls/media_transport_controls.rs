//! WinUI's `MediaTransportControls` over the portable `MediaPlayer`.
//!
//! The control binds the named parts of its template by WinUI's names (`PlayPauseButton`,
//! `ProgressSlider`, `VolumeMuteButton`, ...), and by the names of the 0.5.0 template
//! (`MediaPlayPause`, `MediaSeek`, ...) for templates written against it. It sets WinUI's
//! visual states as pseudo-classes (see [`MediaTransportControls::PC_PAUSE_STATE`] and the
//! other `PC_` constants), and chooses the commands that fit by WinUI's dropout order.
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_markup_enum, ferro_properties,
    input::{InputElementImpl, InputElementImplExt, PointerEventArgs},
    instantiate,
    interactivity::InteractiveImpl,
    layout::{LayoutableImpl, LayoutableImplExt},
    media::StreamGeometry,
    reactive::Subscription,
    threading::{Dispatcher, DispatcherPriority, DispatcherTimer},
    FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref, Size,
    StyledElementImpl, StyledProperty, VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs,
};
use ferroui_controls::{
    automation::AutomationProperties,
    primitives::{TemplatedControl, TemplatedControlImplExt, ToggleButton},
    Button, Control, ControlImpl, Flyout, MenuFlyout, MenuItem, PathIcon, Slider, TextBlock,
    ToolTip,
};
use ferroui_media_video::{
    MediaErrorKind, MediaFuture, MediaPlaybackState, MediaPlayer, MediaPlayerEvent,
    MediaSubscription, MediaTrackKind, PlaybackIntent, WeakMediaPlayer,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};

const PLAY: &str = "M6 3 L17 10 L6 17 Z";
const PAUSE: &str = "M5 3 H8 V17 H5 Z M12 3 H15 V17 H12 Z";
const STOP: &str = "M5 5 H15 V15 H5 Z";
const SPEAKER: &str = "M2 7 H6 L11 3 V17 L6 13 H2 Z M13 6 V8 C15 9 15 11 13 12 V14 C18 12 18 8 13 6 Z M15 2 V4 C21 7 21 13 15 16 V18 C24 14 24 6 15 2 Z";
const MUTED: &str = "M2 7 H6 L11 3 V17 L6 13 H2 Z M14 6 L16 8 L18 6 L19.5 7.5 L17.5 9.5 L19.5 11.5 L18 13 L16 11 L14 13 L12.5 11.5 L14.5 9.5 L12.5 7.5 Z";
const MORE: &str = "M3 8 H7 V12 H3 Z M8 8 H12 V12 H8 Z M13 8 H17 V12 H13 Z";
const FULL_WINDOW: &str =
    "M2 8 V2 H8 V4 H4 V8 Z M12 2 H18 V8 H16 V4 H12 Z M18 12 V18 H12 V16 H16 V12 Z M8 18 H2 V12 H4 V16 H8 Z";

/// WinUI's `SkipForwardInSecs` and `SkipBackwardInSecs`.
const SKIP_FORWARD: Duration = Duration::from_secs(30);
const SKIP_BACKWARD: Duration = Duration::from_secs(10);
/// WinUI's `AvailablePlaybackRateList`.
const PLAYBACK_RATES: [f64; 5] = [0.25, 0.5, 1.0, 1.5, 2.0];
/// The fast forward rates of WinUI's trick mode (`UIA_MEDIA_FASTFORWARD_2X` to `16X`).
const FAST_FORWARD_RATES: [f64; 4] = [2.0, 4.0, 8.0, 16.0];
/// WinUI's `ControlPanelDisplayTimeoutInSecs`.
const CONTROL_PANEL_DISPLAY_TIMEOUT: Duration = Duration::from_secs(3);
/// WinUI's `MediaTransportControlsAppBarButtonWidth`, the width of a command whose own width is
/// not set.
const COMMAND_WIDTH: f64 = 40.;
/// The width of the seek bar beside the commands in the compact layout: WinUI's `MinWidth` of
/// `ProgressSlider` and `MediaTransportControlsProgressSliderCompactMargin` (12,-1,8,1).
const COMPACT_SEEK_WIDTH: f64 = 80. + 12. + 8.;

/// What the fast forward and rewind buttons do when the media cannot change its rate
/// (WinUI's `FastPlayFallbackBehaviour`, with its values).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum FastPlayFallbackBehaviour {
    /// The buttons skip forward and backward (the default).
    #[default]
    Skip = 0,
    /// The buttons are hidden.
    Hide = 1,
    /// The buttons are disabled.
    Disable = 2,
}
ferro_markup_enum!(FastPlayFallbackBehaviour { Skip, Hide, Disable }, { namespace: "FerroUI.Media.Video.Controls" });

fn set_icon(button: &Button, path: &str) {
    let icon = PathIcon::new();
    // An AppBarButton shows its Icon, a button its content.
    if let Some(command) = button.to_ref().cast::<ferroui_controls::CommandBarButton>() {
        icon.set_data(Some(
            StreamGeometry::parse(path)
                .expect("media icon geometry")
                .upcast::<ferroui_base::media::Geometry>(),
        ));
        command.set_icon(Some(Control::boxed(icon)));
        return;
    }
    icon.set_width(18.);
    icon.set_height(18.);
    icon.set_data(Some(
        StreamGeometry::parse(path)
            .expect("media icon geometry")
            .upcast::<ferroui_base::media::Geometry>(),
    ));
    button.set_content(Some(Control::boxed(icon)));
}
fn clock_text(value: Duration) -> String {
    let seconds = value.as_secs();
    if seconds >= 3600 {
        format!(
            "{}:{:02}:{:02}",
            seconds / 3600,
            seconds / 60 % 60,
            seconds % 60
        )
    } else {
        format!("{}:{:02}", seconds / 60, seconds % 60)
    }
}
fn set_name(part: &Button, label: &str) {
    AutomationProperties::set_name(part, Some(label.into()));
    ToolTip::set_tip(part, Some(Rc::new(label.to_owned())));
}
/// A volume (0 to 1) as the value of a slider of any range: WinUI's `VolumeSlider` goes from 0
/// to 100, the slider of the 0.5.0 template from 0 to 1.
fn volume_to_slider(slider: &Slider, volume: f64) -> f64 {
    let (min, max) = (slider.minimum(), slider.maximum());
    min + volume.clamp(0., 1.) * (max - min)
}
fn slider_to_volume(slider: &Slider, value: f64) -> f64 {
    let (min, max) = (slider.minimum(), slider.maximum());
    if max > min {
        ((value - min) / (max - min)).clamp(0., 1.)
    } else {
        0.
    }
}

/// The parts of the template. Every button is found as a `Button` (the `RepeatButton` part
/// is a `ToggleButton`, which is one).
#[derive(Default)]
struct Parts {
    /// The template has WinUI's layout (`ControlPanelGrid` or `PlayPauseButton`); otherwise it is
    /// a template of the 0.5.0 layout, with its `MediaMore` menu.
    winui: bool,
    play: Option<Ref<Button>>,
    play_left: Option<Ref<Button>>,
    stop: Option<Ref<Button>>,
    volume_mute: Option<Ref<Button>>,
    audio_mute: Option<Ref<Button>>,
    full_window: Option<Ref<Button>>,
    zoom: Option<Ref<Button>>,
    skip_backward: Option<Ref<Button>>,
    skip_forward: Option<Ref<Button>>,
    rewind: Option<Ref<Button>>,
    fast_forward: Option<Ref<Button>>,
    previous_track: Option<Ref<Button>>,
    next_track: Option<Ref<Button>>,
    playback_rate: Option<Ref<Button>>,
    captions: Option<Ref<Button>>,
    audio_tracks: Option<Ref<Button>>,
    cast: Option<Ref<Button>>,
    repeat: Option<Ref<Button>>,
    more: Option<Ref<Button>>,
    seek: Option<Ref<Slider>>,
    volume: Option<Ref<Slider>>,
    volume_value: Option<Ref<TextBlock>>,
    status: Option<Ref<TextBlock>>,
    clock: Option<Ref<TextBlock>>,
    elapsed: Option<Ref<TextBlock>>,
    remaining: Option<Ref<TextBlock>>,
    error: Option<Ref<TextBlock>>,
    /// The volume flyout was made by the control (`VOLUME_FLYOUT`): its mute button gets its
    /// icons from the control.
    volume_flyout_made: bool,
}
impl Parts {
    /// The buttons that open a menu of the control.
    fn menu_buttons(&self) -> impl Iterator<Item = &Ref<Button>> {
        [
            &self.more,
            &self.playback_rate,
            &self.captions,
            &self.audio_tracks,
        ]
        .into_iter()
        .flatten()
    }
}

/// A button of the command row, in the order of WinUI's template, with its
/// `MediaTransportControlsHelper.DropoutOrder`: when the row is too narrow, the commands of the
/// lowest order are left out first.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Command {
    VolumeMute,
    Captions,
    AudioTracks,
    Stop,
    SkipBackward,
    PreviousTrack,
    Rewind,
    PlayPause,
    FastForward,
    NextTrack,
    SkipForward,
    PlaybackRate,
    Repeat,
    Zoom,
    Cast,
    FullWindow,
    More,
}
impl Command {
    const ROW: [Command; 17] = [
        Command::VolumeMute,
        Command::Captions,
        Command::AudioTracks,
        Command::Stop,
        Command::SkipBackward,
        Command::PreviousTrack,
        Command::Rewind,
        Command::PlayPause,
        Command::FastForward,
        Command::NextTrack,
        Command::SkipForward,
        Command::PlaybackRate,
        Command::Repeat,
        Command::Zoom,
        Command::Cast,
        Command::FullWindow,
        Command::More,
    ];
    fn dropout_order(self) -> i32 {
        match self {
            Command::VolumeMute => 19,
            Command::Captions => 15,
            Command::AudioTracks => 13,
            Command::Stop => 7,
            Command::SkipBackward | Command::SkipForward => 5,
            Command::PreviousTrack | Command::NextTrack => 3,
            Command::Rewind | Command::FastForward | Command::Repeat => 1,
            Command::PlayPause => 23,
            Command::PlaybackRate => 10,
            Command::Zoom => 9,
            Command::Cast => 11,
            // FullWindowButton is commented out in the source template (full screen is not
            // supported there); its order is the one of that comment.
            Command::FullWindow => 17,
            // The menu of the 0.5.0 template, which holds the commands it leaves out.
            Command::More => 21,
        }
    }
    fn part(self, parts: &Parts) -> Option<&Ref<Button>> {
        match self {
            Command::VolumeMute => parts.volume_mute.as_ref(),
            Command::Captions => parts.captions.as_ref(),
            Command::AudioTracks => parts.audio_tracks.as_ref(),
            Command::Stop => parts.stop.as_ref(),
            Command::SkipBackward => parts.skip_backward.as_ref(),
            Command::PreviousTrack => parts.previous_track.as_ref(),
            Command::Rewind => parts.rewind.as_ref(),
            Command::PlayPause => parts.play.as_ref(),
            Command::FastForward => parts.fast_forward.as_ref(),
            Command::NextTrack => parts.next_track.as_ref(),
            Command::SkipForward => parts.skip_forward.as_ref(),
            Command::PlaybackRate => parts.playback_rate.as_ref(),
            Command::Repeat => parts.repeat.as_ref(),
            Command::Zoom => parts.zoom.as_ref(),
            Command::Cast => parts.cast.as_ref(),
            Command::FullWindow => parts.full_window.as_ref(),
            Command::More => parts.more.as_ref(),
        }
    }
}

/// The commands of the row that fit in `width`: the wanted ones, less those of the lowest
/// dropout order while they are wider than the row.
fn fitting_commands(wanted: &[(Command, f64)], width: f64) -> Vec<Command> {
    let mut kept: Vec<(Command, f64)> = wanted.to_vec();
    let mut total: f64 = kept.iter().map(|(_, w)| w).sum();
    while total > width + 0.5 && !kept.is_empty() {
        let (index, _) = kept
            .iter()
            .enumerate()
            .min_by_key(|(_, (command, _))| command.dropout_order())
            .unwrap();
        total -= kept.remove(index).1;
    }
    kept.into_iter().map(|(command, _)| command).collect()
}

/// A visual state of WinUI's template: the pseudo-classes of its group and the one it sets
/// (none for the default state of the group).
struct State {
    group: &'static [&'static str],
    pseudo_class: Option<&'static str>,
}

#[repr(C)]
pub struct MediaTransportControls {
    base: TemplatedControl,
    template_generation: Cell<u64>,
    player: RefCell<Option<WeakMediaPlayer>>,
    subscription: RefCell<Option<MediaSubscription>>,
    full_window_handler: RefCell<Option<Rc<dyn Fn()>>>,
    full_window_in_menu: Cell<bool>,
    parts: RefCell<Parts>,
    menu_openings: RefCell<Vec<Subscription>>,
    zoom_handler: RefCell<Option<Rc<dyn Fn(bool)>>>,
    zoomed: Cell<bool>,
    playing_icon: Cell<bool>,
    muted_icon: Cell<bool>,
    command_generation: Cell<u64>,
    updating: Cell<bool>,
    has_error: Cell<bool>,
    panel_hidden: Cell<bool>,
    hide_timer: RefCell<Option<Rc<DispatcherTimer>>>,
    last_state: Cell<Option<MediaPlaybackState>>,
}

#[cfg(test)]
mod option_tests {
    use super::*;
    use ferroui_controls::testing::{TestServices, UnitTestApplication};
    use ferroui_media_video::{MediaPlayerProvider, MediaSource, NativeMediaSession};
    /// A template of the 0.5.0 layout.
    fn test_transport() -> Ref<MediaTransportControls> {
        use ferroui_controls::templates::FuncControlTemplate;
        let transport = MediaTransportControls::new();
        transport.set_template(Some(FuncControlTemplate::new(|_, scope| {
            let panel = ferroui_controls::StackPanel::new();
            for name in [
                "MediaPlayPause",
                "MediaStop",
                "MediaMute",
                "MediaFullWindow",
            ] {
                let part = Button::new();
                scope.register(name, part.clone().upcast());
                panel.children().add(part);
            }
            for name in ["MediaSeek", "MediaVolume"] {
                // The range of the sliders of the 0.5.0 template.
                let part = Slider::new();
                part.set_maximum(1.);
                scope.register(name, part.clone().upcast());
                panel.children().add(part);
            }
            for name in ["MediaStatus", "MediaTime", "MediaError"] {
                let part = TextBlock::new();
                scope.register(name, part.clone().upcast());
                panel.children().add(part);
            }
            panel.upcast()
        })));
        transport.apply_template();
        transport
    }
    /// A template with the parts of WinUI's template (the volume slider in the row, no flyout).
    fn winui_transport() -> Ref<MediaTransportControls> {
        use ferroui_controls::templates::FuncControlTemplate;
        let transport = MediaTransportControls::new();
        transport.set_template(Some(FuncControlTemplate::new(|_, scope| {
            let panel = ferroui_controls::StackPanel::new();
            for name in [
                "PlayPauseButton",
                "PlayPauseButtonOnLeft",
                "StopButton",
                "VolumeMuteButton",
                "FullWindowButton",
                "ZoomButton",
                "SkipBackwardButton",
                "SkipForwardButton",
                "RewindButton",
                "FastForwardButton",
                "PreviousTrackButton",
                "NextTrackButton",
                "PlaybackRateButton",
                "CCSelectionButton",
                "AudioTracksSelectionButton",
                "CastButton",
            ] {
                let part = Button::new();
                scope.register(name, part.clone().upcast());
                panel.children().add(part);
            }
            let repeat = ToggleButton::new();
            scope.register("RepeatButton", repeat.clone().upcast());
            panel.children().add(repeat);
            let seek = Slider::new();
            scope.register("ProgressSlider", seek.clone().upcast());
            panel.children().add(seek);
            let volume = Slider::new();
            volume.set_maximum(100.);
            scope.register("VolumeSlider", volume.clone().upcast());
            panel.children().add(volume);
            for name in [
                "VolumeValue",
                "TimeElapsedElement",
                "TimeRemainingElement",
                "ErrorTextBlock",
            ] {
                let part = TextBlock::new();
                scope.register(name, part.clone().upcast());
                panel.children().add(part);
            }
            panel.upcast()
        })));
        transport.apply_template();
        transport
    }
    struct Unopened;
    #[test]
    fn clock_formats_minutes_and_hours_without_decimal_jitter() {
        assert_eq!(clock_text(Duration::from_millis(4999)), "0:04");
        assert_eq!(clock_text(Duration::from_secs(65)), "1:05");
        assert_eq!(clock_text(Duration::from_secs(3723)), "1:02:03");
    }
    impl MediaPlayerProvider for Unopened {
        fn open(
            &self,
            _: MediaSource,
            _: u64,
            _: u64,
            _: ferroui_base::rendering::composition::frames::PresentationPolicy,
        ) -> MediaFuture<Rc<dyn NativeMediaSession>> {
            panic!("idle option synchronization must not open media")
        }
    }
    #[test]
    fn idle_transport_tracks_programmatic_options_without_native_polling() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let player = MediaPlayer::new(Rc::new(Unopened));
        let transport = test_transport();
        transport.set_media_player(Some(&player));
        let mut options = player.options();
        options.volume = 0.4;
        options.muted = true;
        player.set_options(options).unwrap();
        let parts = transport.parts.borrow();
        assert_eq!(parts.volume.as_ref().unwrap().value(), 0.4);
        assert_eq!(
            AutomationProperties::get_name(parts.volume_mute.as_ref().unwrap()),
            Some("Unmute".into())
        );
        drop(parts);
        let mut invalid = player.options();
        invalid.volume = f64::NAN;
        assert!(player.set_options(invalid).is_err());
        assert_eq!(
            transport.parts.borrow().volume.as_ref().unwrap().value(),
            0.4
        );
    }
    #[test]
    fn winui_parts_are_bound_by_their_winui_names() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let transport = winui_transport();
        let parts = transport.parts.borrow();
        assert!(parts.winui);
        for (name, part) in [
            ("PlayPauseButton", &parts.play),
            ("PlayPauseButtonOnLeft", &parts.play_left),
            ("StopButton", &parts.stop),
            ("VolumeMuteButton", &parts.volume_mute),
            ("SkipBackwardButton", &parts.skip_backward),
            ("RepeatButton", &parts.repeat),
            ("CCSelectionButton", &parts.captions),
        ] {
            assert!(part.is_some(), "{name}");
        }
        assert!(parts.seek.is_some() && parts.volume.is_some() && parts.elapsed.is_some());
        assert!(parts.more.is_none());
        // The menu buttons of WinUI's template have their menus.
        assert!(parts.playback_rate.as_ref().unwrap().flyout().is_some());
        assert!(parts.captions.as_ref().unwrap().flyout().is_some());
    }
    #[test]
    fn winui_volume_flyout_is_made_without_a_volume_slider_in_the_template() {
        use ferroui_controls::templates::FuncControlTemplate;
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let player = MediaPlayer::new(Rc::new(Unopened));
        let transport = MediaTransportControls::new();
        transport.set_template(Some(FuncControlTemplate::new(|_, scope| {
            let panel = ferroui_controls::StackPanel::new();
            for name in ["PlayPauseButton", "VolumeMuteButton"] {
                let part = Button::new();
                scope.register(name, part.clone().upcast());
                panel.children().add(part);
            }
            panel.upcast()
        })));
        transport.apply_template();
        transport.set_media_player(Some(&player));
        let mut options = player.options();
        options.volume = 0.5;
        player.set_options(options).unwrap();
        let parts = transport.parts.borrow();
        assert!(parts.volume_flyout_made);
        assert!(parts.volume_mute.as_ref().unwrap().flyout().is_some());
        assert_eq!(parts.volume.as_ref().unwrap().value(), 50.);
        assert_eq!(
            parts.volume_value.as_ref().unwrap().text().as_deref(),
            Some("50")
        );
        assert_eq!(
            parts.audio_mute.as_ref().unwrap().name().as_deref(),
            Some("AudioMuteButton")
        );
        drop(parts);
        // A new template takes the flyout away from the old button.
        let old = transport.parts.borrow().volume_mute.clone().unwrap();
        transport.set_template(None);
        assert!(old.flyout().is_none());
    }
    #[test]
    fn winui_volume_slider_of_hundred_maps_to_the_player_volume() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let player = MediaPlayer::new(Rc::new(Unopened));
        let transport = winui_transport();
        transport.set_media_player(Some(&player));
        let mut options = player.options();
        options.volume = 0.25;
        player.set_options(options).unwrap();
        let parts = transport.parts.borrow();
        assert_eq!(parts.volume.as_ref().unwrap().value(), 25.);
        assert_eq!(
            parts.volume_value.as_ref().unwrap().text().as_deref(),
            Some("25")
        );
    }
    #[test]
    fn visual_states_follow_the_player_and_the_properties() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let player = MediaPlayer::new(Rc::new(Unopened));
        let transport = winui_transport();
        transport.set_media_player(Some(&player));
        let has = |name| transport.pseudo_classes().contains(name);
        assert!(!has(MediaTransportControls::PC_COMPACT_MODE));
        transport.set_is_compact(true);
        assert!(has(MediaTransportControls::PC_COMPACT_MODE));
        let mut options = player.options();
        options.muted = true;
        player.set_options(options).unwrap();
        assert!(has(MediaTransportControls::PC_MUTE_STATE));
        let mut options = player.options();
        options.looping = true;
        player.set_options(options).unwrap();
        assert!(has(MediaTransportControls::PC_REPEAT_ONE_STATE));
        transport.set_is_full_window(true);
        assert!(has(MediaTransportControls::PC_FULL_WINDOW_STATE));
        // Not playing: the play button shows Play.
        assert!(!has(MediaTransportControls::PC_PAUSE_STATE));
        transport.hide();
        assert!(has(MediaTransportControls::PC_CONTROL_PANEL_FADE_OUT));
        transport.show();
        assert!(!has(MediaTransportControls::PC_CONTROL_PANEL_FADE_OUT));
    }
    #[test]
    fn compact_mode_moves_play_to_the_left() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let transport = winui_transport();
        transport.update_visibility(640.);
        {
            let parts = transport.parts.borrow();
            assert!(parts.play.as_ref().unwrap().is_visible());
            assert!(!parts.play_left.as_ref().unwrap().is_visible());
        }
        transport.set_is_compact(true);
        transport.update_visibility(640.);
        let parts = transport.parts.borrow();
        assert!(!parts.play.as_ref().unwrap().is_visible());
        assert!(parts.play_left.as_ref().unwrap().is_visible());
    }
    #[test]
    fn winui_visibility_properties_hide_their_buttons() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let transport = winui_transport();
        transport.update_visibility(720.);
        let visible = |part: fn(&Parts) -> &Option<Ref<Button>>| {
            part(&transport.parts.borrow())
                .as_ref()
                .unwrap()
                .is_visible()
        };
        // The defaults of the source: these are hidden.
        assert!(!visible(|p| &p.fast_forward));
        assert!(!visible(|p| &p.rewind));
        assert!(!visible(|p| &p.next_track));
        assert!(!visible(|p| &p.previous_track));
        assert!(!visible(|p| &p.repeat));
        assert!(visible(|p| &p.stop));
        transport.set_is_repeat_button_visible(true);
        transport.set_is_next_track_button_visible(true);
        transport.set_is_stop_button_visible(false);
        transport.update_visibility(720.);
        assert!(visible(|p| &p.repeat));
        assert!(visible(|p| &p.next_track));
        assert!(!visible(|p| &p.stop));
        // No playlist: the track buttons are shown, and disabled.
        assert!(!transport
            .parts
            .borrow()
            .next_track
            .as_ref()
            .unwrap()
            .is_enabled());
    }
    #[test]
    fn enabled_properties_disable_their_buttons() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let transport = winui_transport();
        transport.set_is_volume_enabled(false);
        transport.set_is_repeat_button_visible(true);
        let parts = transport.parts.borrow();
        assert!(!parts.volume_mute.as_ref().unwrap().is_enabled());
        assert!(!parts.volume.as_ref().unwrap().is_enabled());
        // IsRepeatEnabled is false by default (the source's default).
        assert!(!parts.repeat.as_ref().unwrap().is_enabled());
    }
    #[test]
    fn dropout_leaves_out_the_lowest_order_first() {
        let wanted: Vec<_> = [
            Command::VolumeMute,
            Command::Stop,
            Command::PlayPause,
            Command::Zoom,
        ]
        .into_iter()
        .map(|c| (c, COMMAND_WIDTH))
        .collect();
        assert_eq!(fitting_commands(&wanted, 160.).len(), 4);
        assert_eq!(
            fitting_commands(&wanted, 120.),
            vec![Command::VolumeMute, Command::PlayPause, Command::Zoom]
        );
        assert_eq!(
            fitting_commands(&wanted, 80.),
            vec![Command::VolumeMute, Command::PlayPause]
        );
        assert_eq!(fitting_commands(&wanted, 40.), vec![Command::PlayPause]);
    }
    #[test]
    fn fast_forward_steps_through_the_trick_rates() {
        assert_eq!(next_fast_forward_rate(1.0), 2.0);
        assert_eq!(next_fast_forward_rate(2.0), 4.0);
        assert_eq!(next_fast_forward_rate(8.0), 16.0);
        assert_eq!(next_fast_forward_rate(16.0), 1.0);
        assert_eq!(next_fast_forward_rate(1.5), 2.0);
    }
}

/// The rate after `rate` for WinUI's fast forward: 2, 4, 8 and 16 times, then normal speed.
fn next_fast_forward_rate(rate: f64) -> f64 {
    FAST_FORWARD_RATES
        .into_iter()
        .find(|next| *next > rate + f64::EPSILON)
        .unwrap_or(1.0)
}

ferro_class!(MediaTransportControls:TemplatedControl);
ferro_class_info!(MediaTransportControls {
    new: MediaTransportControls::new
});
ferro_impl_classes!(MediaTransportControls:StyledElementImpl,InteractiveImpl);
ferro_properties! { impl MediaTransportControls {
    pub fn is_compact_property()->StyledProperty<bool> { FerroProperty::register::<Self,_>("IsCompact",false) }
    pub fn is_play_pause_button_visible_property()->StyledProperty<bool> { FerroProperty::register::<Self,_>("IsPlayPauseButtonVisible",true) }
    pub fn is_stop_button_visible_property()->StyledProperty<bool> { FerroProperty::register::<Self,_>("IsStopButtonVisible",true) }
    pub fn is_seek_bar_visible_property()->StyledProperty<bool> { FerroProperty::register::<Self,_>("IsSeekBarVisible",true) }
    pub fn is_volume_button_visible_property()->StyledProperty<bool> { FerroProperty::register::<Self,_>("IsVolumeButtonVisible",true) }
    pub fn is_volume_slider_visible_property()->StyledProperty<bool> { FerroProperty::register::<Self,_>("IsVolumeSliderVisible",true) }
    pub fn is_full_window_button_visible_property()->StyledProperty<bool> { FerroProperty::register::<Self,_>("IsFullWindowButtonVisible",true) }
    pub fn is_full_window_property()->StyledProperty<bool> { FerroProperty::register::<Self,_>("IsFullWindow",false) }
    pub fn is_skip_backward_button_visible_property()->StyledProperty<bool> { FerroProperty::register::<Self,_>("IsSkipBackwardButtonVisible",true) }
    pub fn is_skip_forward_button_visible_property()->StyledProperty<bool> { FerroProperty::register::<Self,_>("IsSkipForwardButtonVisible",true) }
    pub fn is_zoom_button_visible_property()->StyledProperty<bool> { FerroProperty::register::<Self,_>("IsZoomButtonVisible",true) }
    pub fn is_playback_rate_button_visible_property()->StyledProperty<bool> { FerroProperty::register::<Self,_>("IsPlaybackRateButtonVisible",true) }
    pub fn is_audio_tracks_button_visible_property()->StyledProperty<bool> { FerroProperty::register::<Self,_>("IsAudioTracksButtonVisible",true) }
    pub fn is_closed_captions_button_visible_property()->StyledProperty<bool> { FerroProperty::register::<Self,_>("IsClosedCaptionsButtonVisible",true) }
    pub fn is_cast_button_visible_property()->StyledProperty<bool> { FerroProperty::register::<Self,_>("IsCastButtonVisible",true) }
    pub fn is_fast_forward_button_visible_property()->StyledProperty<bool> { FerroProperty::register::<Self,_>("IsFastForwardButtonVisible",false) }
    pub fn is_fast_rewind_button_visible_property()->StyledProperty<bool> { FerroProperty::register::<Self,_>("IsFastRewindButtonVisible",false) }
    pub fn is_next_track_button_visible_property()->StyledProperty<bool> { FerroProperty::register::<Self,_>("IsNextTrackButtonVisible",false) }
    pub fn is_previous_track_button_visible_property()->StyledProperty<bool> { FerroProperty::register::<Self,_>("IsPreviousTrackButtonVisible",false) }
    pub fn is_repeat_button_visible_property()->StyledProperty<bool> { FerroProperty::register::<Self,_>("IsRepeatButtonVisible",false) }
    pub fn is_full_window_enabled_property()->StyledProperty<bool> { FerroProperty::register::<Self,_>("IsFullWindowEnabled",true) }
    pub fn is_zoom_enabled_property()->StyledProperty<bool> { FerroProperty::register::<Self,_>("IsZoomEnabled",true) }
    pub fn is_fast_forward_enabled_property()->StyledProperty<bool> { FerroProperty::register::<Self,_>("IsFastForwardEnabled",false) }
    pub fn is_fast_rewind_enabled_property()->StyledProperty<bool> { FerroProperty::register::<Self,_>("IsFastRewindEnabled",false) }
    pub fn is_stop_enabled_property()->StyledProperty<bool> { FerroProperty::register::<Self,_>("IsStopEnabled",true) }
    pub fn is_volume_enabled_property()->StyledProperty<bool> { FerroProperty::register::<Self,_>("IsVolumeEnabled",true) }
    pub fn is_playback_rate_enabled_property()->StyledProperty<bool> { FerroProperty::register::<Self,_>("IsPlaybackRateEnabled",true) }
    pub fn is_seek_enabled_property()->StyledProperty<bool> { FerroProperty::register::<Self,_>("IsSeekEnabled",true) }
    pub fn is_skip_forward_enabled_property()->StyledProperty<bool> { FerroProperty::register::<Self,_>("IsSkipForwardEnabled",true) }
    pub fn is_skip_backward_enabled_property()->StyledProperty<bool> { FerroProperty::register::<Self,_>("IsSkipBackwardEnabled",true) }
    pub fn is_repeat_enabled_property()->StyledProperty<bool> { FerroProperty::register::<Self,_>("IsRepeatEnabled",false) }
    pub fn fast_play_fallback_behaviour_property()->StyledProperty<FastPlayFallbackBehaviour> { FerroProperty::register::<Self,_>("FastPlayFallbackBehaviour",FastPlayFallbackBehaviour::Skip) }
    pub fn show_and_hide_automatically_property()->StyledProperty<bool> { FerroProperty::register::<Self,_>("ShowAndHideAutomatically",true) }
}}
impl LayoutableImpl for MediaTransportControls {
    fn measure_override(this: &Self, available: Size) -> Size {
        // Choose compact content from the parent's constraint before measuring.
        // Waiting for Bounds would let an oversized auto row keep itself wide.
        this.apply_template();
        this.update_visibility(available.width);
        Self::parent_measure_override(this, available)
    }
}
impl VisualImpl for MediaTransportControls {
    fn on_detached_from_visual_tree(this: &Self, args: &VisualTreeAttachmentEventArgs) {
        this.hide_menu();
        this.stop_hide_timer();
        this.subscription.borrow_mut().take();
        this.command_generation
            .set(this.command_generation.get().wrapping_add(1));
        Self::parent_on_detached_from_visual_tree(this, args);
    }
    fn on_attached_to_visual_tree(this: &Self, args: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, args);
        this.subscribe();
        this.refresh();
    }
}
impl InputElementImpl for MediaTransportControls {
    fn on_pointer_entered(this: &Self, e: &PointerEventArgs) {
        Self::parent_on_pointer_entered(this, e);
        this.show();
    }
    fn on_pointer_moved(this: &Self, e: &PointerEventArgs) {
        Self::parent_on_pointer_moved(this, e);
        this.show();
    }
    fn on_pointer_exited(this: &Self, e: &PointerEventArgs) {
        Self::parent_on_pointer_exited(this, e);
        this.start_hide_timer();
    }
}
impl FerroObjectImpl for MediaTransportControls {
    fn on_property_changed(this: &Self, args: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, args);
        let property = args.property();
        if property == Self::is_full_window_property().as_property() {
            this.update_full_window_name();
        }
        if property == Self::show_and_hide_automatically_property().as_property() {
            if this.show_and_hide_automatically() {
                this.start_hide_timer();
            } else {
                this.show();
            }
        }
        if [
            Self::is_compact_property().as_property(),
            Self::is_play_pause_button_visible_property().as_property(),
            Self::is_stop_button_visible_property().as_property(),
            Self::is_seek_bar_visible_property().as_property(),
            Self::is_volume_button_visible_property().as_property(),
            Self::is_volume_slider_visible_property().as_property(),
            Self::is_full_window_button_visible_property().as_property(),
            Self::is_full_window_property().as_property(),
            Self::is_skip_backward_button_visible_property().as_property(),
            Self::is_skip_forward_button_visible_property().as_property(),
            Self::is_zoom_button_visible_property().as_property(),
            Self::is_playback_rate_button_visible_property().as_property(),
            Self::is_audio_tracks_button_visible_property().as_property(),
            Self::is_closed_captions_button_visible_property().as_property(),
            Self::is_cast_button_visible_property().as_property(),
            Self::is_fast_forward_button_visible_property().as_property(),
            Self::is_fast_rewind_button_visible_property().as_property(),
            Self::is_next_track_button_visible_property().as_property(),
            Self::is_previous_track_button_visible_property().as_property(),
            Self::is_repeat_button_visible_property().as_property(),
            Self::fast_play_fallback_behaviour_property().as_property(),
        ]
        .contains(&property)
        {
            this.update_visibility(this.bounds().width);
            this.invalidate_measure();
        }
        if [
            Self::is_full_window_enabled_property().as_property(),
            Self::is_zoom_enabled_property().as_property(),
            Self::is_fast_forward_enabled_property().as_property(),
            Self::is_fast_rewind_enabled_property().as_property(),
            Self::is_stop_enabled_property().as_property(),
            Self::is_volume_enabled_property().as_property(),
            Self::is_playback_rate_enabled_property().as_property(),
            Self::is_seek_enabled_property().as_property(),
            Self::is_skip_forward_enabled_property().as_property(),
            Self::is_skip_backward_enabled_property().as_property(),
            Self::is_repeat_enabled_property().as_property(),
            Self::fast_play_fallback_behaviour_property().as_property(),
        ]
        .contains(&property)
        {
            this.refresh();
        }
        if property == Self::is_compact_property().as_property()
            || property == Self::is_full_window_property().as_property()
        {
            this.update_states();
        }
    }
}
impl ferroui_controls::primitives::TemplatedControlImpl for MediaTransportControls {
    fn on_template_changed(this: &Self, args: &FerroPropertyChangedEventArgs<'_>) {
        this.template_generation
            .set(this.template_generation.get().wrapping_add(1));
        this.command_generation
            .set(this.command_generation.get().wrapping_add(1));
        this.menu_openings.borrow_mut().clear();
        let parts = std::mem::take(&mut *this.parts.borrow_mut());
        let made = parts
            .volume_mute
            .as_ref()
            .filter(|_| parts.volume_flyout_made);
        for button in parts.menu_buttons().chain(made) {
            if let Some(menu) = button.flyout() {
                menu.hide();
            }
            button.set_flyout(None);
        }
        Self::parent_on_template_changed(this, args);
    }
    fn on_apply_template(
        this: &Self,
        args: &ferroui_controls::primitives::TemplateAppliedEventArgs,
    ) {
        Self::parent_on_apply_template(this, args);
        let generation = this.template_generation.get().wrapping_add(1);
        this.template_generation.set(generation);
        this.command_generation
            .set(this.command_generation.get().wrapping_add(1));
        let scope = args.name_scope();
        // A part by WinUI's name, else by the name of the 0.5.0 template.
        let button = |names: &[&str]| names.iter().find_map(|name| scope.find_as::<Button>(name));
        let slider = |names: &[&str]| names.iter().find_map(|name| scope.find_as::<Slider>(name));
        let text = |names: &[&str]| {
            names
                .iter()
                .find_map(|name| scope.find_as::<TextBlock>(name))
        };
        let mut parts = Parts {
            winui: scope.find_as::<Control>("ControlPanelGrid").is_some()
                || scope.find_as::<Button>("PlayPauseButton").is_some(),
            play: button(&["PlayPauseButton", "MediaPlayPause"]),
            play_left: button(&["PlayPauseButtonOnLeft"]),
            stop: button(&["StopButton", "MediaStop"]),
            volume_mute: button(&["VolumeMuteButton", "MediaMute"]),
            audio_mute: button(&["AudioMuteButton"]),
            full_window: button(&["FullWindowButton", "MediaFullWindow"]),
            zoom: button(&["ZoomButton"]),
            skip_backward: button(&["SkipBackwardButton"]),
            skip_forward: button(&["SkipForwardButton"]),
            rewind: button(&["RewindButton"]),
            fast_forward: button(&["FastForwardButton"]),
            previous_track: button(&["PreviousTrackButton"]),
            next_track: button(&["NextTrackButton"]),
            playback_rate: button(&["PlaybackRateButton"]),
            captions: button(&["CCSelectionButton"]),
            audio_tracks: button(&["AudioTracksSelectionButton"]),
            cast: button(&["CastButton"]),
            repeat: button(&["RepeatButton"]),
            more: button(&["MediaMore"]),
            seek: slider(&["ProgressSlider", "MediaSeek"]),
            volume: slider(&["VolumeSlider", "MediaVolume"]),
            volume_value: text(&["VolumeValue"]),
            status: text(&["MediaStatus"]),
            clock: text(&["MediaTime"]),
            elapsed: text(&["TimeElapsedElement"]),
            remaining: text(&["TimeRemainingElement"]),
            error: text(&["ErrorTextBlock", "MediaError"]),
            volume_flyout_made: false,
        };
        // WinUI's volume flyout: the mute button and the volume slider are in the flyout of
        // VolumeMuteButton, outside the visual tree of the template until it opens.
        if let Some(content) = parts
            .volume_mute
            .as_ref()
            .and_then(|button| button.flyout())
            .and_then(|flyout| flyout.cast::<Flyout>())
            .and_then(|flyout| flyout.content())
            .and_then(|content| Control::from_boxed(&content))
        {
            for element in content.get_self_and_logical_descendants() {
                match element.name().as_deref() {
                    Some("AudioMuteButton") if parts.audio_mute.is_none() => {
                        parts.audio_mute = element.cast::<Button>()
                    }
                    Some("VolumeSlider") if parts.volume.is_none() => {
                        parts.volume = element.cast::<Slider>()
                    }
                    Some("VolumeValue") if parts.volume_value.is_none() => {
                        parts.volume_value = element.cast::<TextBlock>()
                    }
                    _ => {}
                }
            }
        }
        // WinUI's VolumeFlyout, when the template has a VolumeMuteButton and no volume slider.
        if parts.winui && parts.volume.is_none() {
            if let Some(button) = parts.volume_mute.clone().filter(|b| b.flyout().is_none()) {
                this.make_volume_flyout(&button, &mut parts);
            }
        }
        // The icons of the 0.5.0 template are set here; WinUI's template has its own.
        if !parts.winui {
            for (part, icon) in [
                (&parts.play, PLAY),
                (&parts.stop, STOP),
                (&parts.volume_mute, SPEAKER),
                (&parts.full_window, FULL_WINDOW),
                (&parts.more, MORE),
            ] {
                if let Some(part) = part {
                    set_icon(part, icon);
                }
            }
        }
        let names: [(&Option<Ref<Button>>, &str); 13] = [
            (&parts.stop, "Stop"),
            (&parts.skip_backward, "Skip Backward"),
            (&parts.skip_forward, "Skip Forward"),
            (&parts.rewind, "Rewind"),
            (&parts.fast_forward, "Fast forward"),
            (&parts.previous_track, "Previous Track"),
            (&parts.next_track, "Next Track"),
            (&parts.playback_rate, "Show playback rate list"),
            (&parts.captions, "Show closed caption menu"),
            (&parts.audio_tracks, "Show audio selection menu"),
            (&parts.cast, "Cast to Device"),
            (&parts.zoom, "Aspect Ratio"),
            (&parts.more, "More playback controls"),
        ];
        for (part, name) in names {
            if let Some(part) = part {
                if AutomationProperties::get_name(part).is_none() {
                    set_name(part, name);
                }
            }
        }
        if let Some(button) = &parts.more {
            this.bind_menu(button, generation, Self::populate_menu);
        }
        if let Some(button) = &parts.playback_rate {
            this.bind_menu(button, generation, Self::populate_rates);
        }
        if let Some(button) = &parts.captions {
            this.bind_menu(button, generation, |view, menu| {
                view.populate_tracks(menu, MediaTrackKind::Captions)
            });
        }
        if let Some(button) = &parts.audio_tracks {
            this.bind_menu(button, generation, |view, menu| {
                view.populate_tracks(menu, MediaTrackKind::Audio)
            });
        }
        this.playing_icon.set(false);
        this.muted_icon.set(false);
        for play in [&parts.play, &parts.play_left].into_iter().flatten() {
            set_name(play, "Play");
        }
        for mute in [&parts.volume_mute, &parts.audio_mute]
            .into_iter()
            .flatten()
        {
            set_name(mute, "Mute");
        }
        let on_click = |part: &Option<Ref<Button>>, action: fn(&Self)| {
            if let Some(part) = part {
                let weak = this.to_ref().downgrade();
                part.click(move |_, _| {
                    if let Some(view) = weak
                        .upgrade()
                        .filter(|view| view.template_generation.get() == generation)
                    {
                        action(&view);
                        view.show();
                    }
                });
            }
        };
        on_click(&parts.play, Self::toggle_play);
        on_click(&parts.play_left, Self::toggle_play);
        on_click(&parts.stop, Self::stop_playback);
        on_click(&parts.full_window, Self::toggle_full_window);
        on_click(&parts.zoom, |view| view.set_is_zoomed(!view.is_zoomed()));
        on_click(&parts.skip_backward, |view| view.skip(false));
        on_click(&parts.skip_forward, |view| view.skip(true));
        on_click(&parts.fast_forward, |view| view.fast_play(true));
        on_click(&parts.rewind, |view| view.fast_play(false));
        on_click(&parts.repeat, Self::toggle_repeat);
        on_click(&parts.cast, |view| {
            if let Some(player) = view.media_player() {
                view.command(player.cast());
            }
        });
        on_click(&parts.audio_mute, Self::toggle_mute);
        // WinUI's VolumeMuteButton opens the volume flyout; without one it mutes.
        if parts
            .volume_mute
            .as_ref()
            .is_some_and(|b| b.flyout().is_none())
        {
            on_click(&parts.volume_mute, Self::toggle_mute);
        }
        let weak = this.to_ref().downgrade();
        if let Some(seek) = &parts.seek {
            seek.value_changed(move |_, args| {
                if let Some(view) = weak
                    .upgrade()
                    .filter(|view| view.template_generation.get() == generation)
                {
                    if !view.updating.get() {
                        if let Some(player) = view.media_player() {
                            if let Ok(position) = Duration::try_from_secs_f64(args.new_value()) {
                                let seek = player.seek(position);
                                view.command(Box::pin(async move { seek.await.map(|_| ()) }));
                            }
                        }
                    }
                }
            });
        }
        let weak = this.to_ref().downgrade();
        if let Some(volume) = &parts.volume {
            let slider = volume.downgrade();
            volume.value_changed(move |_, args| {
                if let (Some(view), Some(slider)) = (
                    weak.upgrade()
                        .filter(|view| view.template_generation.get() == generation),
                    slider.upgrade(),
                ) {
                    if !view.updating.get() {
                        if let Some(player) = view.media_player() {
                            let mut options = player.options();
                            options.volume = slider_to_volume(&slider, args.new_value());
                            if let Err(e) = player.set_options(options) {
                                view.show_error(&e.to_string());
                            }
                        }
                    }
                }
            });
        }
        *this.parts.borrow_mut() = parts;
        this.update_visibility(this.bounds().width);
        this.update_full_window_name();
        this.refresh();
    }
}

impl MediaTransportControls {
    /// `ControlPanelVisibilityStates`: `ControlPanelFadeIn` (no pseudo-class) and
    /// `ControlPanelFadeOut`.
    pub const PC_CONTROL_PANEL_FADE_OUT: &'static str = ":controlpanelfadeout";
    /// `MediaStates`: `Normal` (no pseudo-class), `Buffering`, `Loading` and `Error`.
    pub const PC_BUFFERING: &'static str = ":buffering";
    pub const PC_LOADING: &'static str = ":loading";
    pub const PC_ERROR: &'static str = ":error";
    /// `AudioSelectionAvailablityStates`: `AudioSelectionAvailable` and
    /// `AudioSelectionUnavailable` (no pseudo-class).
    pub const PC_AUDIO_SELECTION_AVAILABLE: &'static str = ":audioselectionavailable";
    /// `CCSelectionAvailablityStates`: `CCSelectionAvailable` and `CCSelectionUnavailable` (no
    /// pseudo-class).
    pub const PC_CC_SELECTION_AVAILABLE: &'static str = ":ccselectionavailable";
    /// `MediaTransportControlMode`: `NormalMode` (no pseudo-class) and `CompactMode`.
    pub const PC_COMPACT_MODE: &'static str = ":compactmode";
    /// `PlayPauseStates`: `PlayState` (no pseudo-class) and `PauseState`, in which the button
    /// shows Pause.
    pub const PC_PAUSE_STATE: &'static str = ":pausestate";
    /// `VolumeMuteStates`: `VolumeState` (no pseudo-class) and `MuteState`.
    pub const PC_MUTE_STATE: &'static str = ":mutestate";
    /// `FullWindowStates`: `NonFullWindowState` (no pseudo-class) and `FullWindowState`.
    pub const PC_FULL_WINDOW_STATE: &'static str = ":fullwindowstate";
    /// `RepeatStates`: `RepeatNoneState` (no pseudo-class), `RepeatOneState` and
    /// `RepeatAllState` (a playlist; the portable player has none).
    pub const PC_REPEAT_ONE_STATE: &'static str = ":repeatonestate";
    pub const PC_REPEAT_ALL_STATE: &'static str = ":repeatallstate";

    const CONTROL_PANEL_VISIBILITY: &'static [&'static str] = &[Self::PC_CONTROL_PANEL_FADE_OUT];
    const MEDIA_STATES: &'static [&'static str] =
        &[Self::PC_BUFFERING, Self::PC_LOADING, Self::PC_ERROR];
    const AUDIO_SELECTION: &'static [&'static str] = &[Self::PC_AUDIO_SELECTION_AVAILABLE];
    const CC_SELECTION: &'static [&'static str] = &[Self::PC_CC_SELECTION_AVAILABLE];
    const CONTROL_MODE: &'static [&'static str] = &[Self::PC_COMPACT_MODE];
    const PLAY_PAUSE: &'static [&'static str] = &[Self::PC_PAUSE_STATE];
    const VOLUME_MUTE: &'static [&'static str] = &[Self::PC_MUTE_STATE];
    const FULL_WINDOW_STATES: &'static [&'static str] = &[Self::PC_FULL_WINDOW_STATE];
    const REPEAT: &'static [&'static str] = &[Self::PC_REPEAT_ONE_STATE, Self::PC_REPEAT_ALL_STATE];

    pub fn construct() -> Self {
        Self {
            base: ferroui_controls::primitives::TemplatedControl::construct(),
            template_generation: Cell::new(0),
            player: RefCell::new(None),
            subscription: RefCell::new(None),
            full_window_handler: RefCell::new(None),
            full_window_in_menu: Cell::new(false),
            parts: RefCell::new(Parts::default()),
            menu_openings: RefCell::new(Vec::new()),
            zoom_handler: RefCell::new(None),
            zoomed: Cell::new(false),
            playing_icon: Cell::new(false),
            muted_icon: Cell::new(false),
            command_generation: Cell::new(0),
            updating: Cell::new(false),
            has_error: Cell::new(false),
            panel_hidden: Cell::new(false),
            hide_timer: RefCell::new(None),
            last_state: Cell::new(None),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn is_compact(&self) -> bool {
        self.get_value(Self::is_compact_property())
    }
    pub fn set_is_compact(&self, value: bool) {
        self.set_value(Self::is_compact_property(), value);
    }
    pub fn is_play_pause_button_visible(&self) -> bool {
        self.get_value(Self::is_play_pause_button_visible_property())
    }
    pub fn set_is_play_pause_button_visible(&self, value: bool) {
        self.set_value(Self::is_play_pause_button_visible_property(), value);
    }
    pub fn is_stop_button_visible(&self) -> bool {
        self.get_value(Self::is_stop_button_visible_property())
    }
    pub fn set_is_stop_button_visible(&self, value: bool) {
        self.set_value(Self::is_stop_button_visible_property(), value);
    }
    pub fn is_seek_bar_visible(&self) -> bool {
        self.get_value(Self::is_seek_bar_visible_property())
    }
    pub fn set_is_seek_bar_visible(&self, value: bool) {
        self.set_value(Self::is_seek_bar_visible_property(), value);
    }
    pub fn is_volume_button_visible(&self) -> bool {
        self.get_value(Self::is_volume_button_visible_property())
    }
    pub fn set_is_volume_button_visible(&self, value: bool) {
        self.set_value(Self::is_volume_button_visible_property(), value);
    }
    pub fn is_volume_slider_visible(&self) -> bool {
        self.get_value(Self::is_volume_slider_visible_property())
    }
    pub fn set_is_volume_slider_visible(&self, value: bool) {
        self.set_value(Self::is_volume_slider_visible_property(), value);
    }
    pub fn is_full_window_button_visible(&self) -> bool {
        self.get_value(Self::is_full_window_button_visible_property())
    }
    pub fn set_is_full_window_button_visible(&self, value: bool) {
        self.set_value(Self::is_full_window_button_visible_property(), value);
    }
    pub fn is_full_window(&self) -> bool {
        self.get_value(Self::is_full_window_property())
    }
    pub fn set_is_full_window(&self, value: bool) {
        self.set_value(Self::is_full_window_property(), value);
    }
    /// Installed by MediaPlayerElement. A standalone transport never advertises
    /// a full-window action that it cannot perform.
    pub fn set_full_window_handler(&self, handler: Option<Rc<dyn Fn()>>) {
        *self.full_window_handler.borrow_mut() = handler;
        self.update_visibility(self.bounds().width);
        self.invalidate_measure();
    }
    pub fn is_skip_backward_button_visible(&self) -> bool {
        self.get_value(Self::is_skip_backward_button_visible_property())
    }
    pub fn set_is_skip_backward_button_visible(&self, value: bool) {
        self.set_value(Self::is_skip_backward_button_visible_property(), value);
    }
    pub fn is_skip_forward_button_visible(&self) -> bool {
        self.get_value(Self::is_skip_forward_button_visible_property())
    }
    pub fn set_is_skip_forward_button_visible(&self, value: bool) {
        self.set_value(Self::is_skip_forward_button_visible_property(), value);
    }
    pub fn is_zoom_button_visible(&self) -> bool {
        self.get_value(Self::is_zoom_button_visible_property())
    }
    pub fn set_is_zoom_button_visible(&self, value: bool) {
        self.set_value(Self::is_zoom_button_visible_property(), value);
    }
    pub fn is_playback_rate_button_visible(&self) -> bool {
        self.get_value(Self::is_playback_rate_button_visible_property())
    }
    pub fn set_is_playback_rate_button_visible(&self, value: bool) {
        self.set_value(Self::is_playback_rate_button_visible_property(), value);
    }
    pub fn is_audio_tracks_button_visible(&self) -> bool {
        self.get_value(Self::is_audio_tracks_button_visible_property())
    }
    pub fn set_is_audio_tracks_button_visible(&self, value: bool) {
        self.set_value(Self::is_audio_tracks_button_visible_property(), value);
    }
    pub fn is_closed_captions_button_visible(&self) -> bool {
        self.get_value(Self::is_closed_captions_button_visible_property())
    }
    pub fn set_is_closed_captions_button_visible(&self, value: bool) {
        self.set_value(Self::is_closed_captions_button_visible_property(), value);
    }
    pub fn is_cast_button_visible(&self) -> bool {
        self.get_value(Self::is_cast_button_visible_property())
    }
    pub fn set_is_cast_button_visible(&self, value: bool) {
        self.set_value(Self::is_cast_button_visible_property(), value);
    }
    pub fn is_fast_forward_button_visible(&self) -> bool {
        self.get_value(Self::is_fast_forward_button_visible_property())
    }
    pub fn set_is_fast_forward_button_visible(&self, value: bool) {
        self.set_value(Self::is_fast_forward_button_visible_property(), value);
    }
    pub fn is_fast_rewind_button_visible(&self) -> bool {
        self.get_value(Self::is_fast_rewind_button_visible_property())
    }
    pub fn set_is_fast_rewind_button_visible(&self, value: bool) {
        self.set_value(Self::is_fast_rewind_button_visible_property(), value);
    }
    pub fn is_next_track_button_visible(&self) -> bool {
        self.get_value(Self::is_next_track_button_visible_property())
    }
    pub fn set_is_next_track_button_visible(&self, value: bool) {
        self.set_value(Self::is_next_track_button_visible_property(), value);
    }
    pub fn is_previous_track_button_visible(&self) -> bool {
        self.get_value(Self::is_previous_track_button_visible_property())
    }
    pub fn set_is_previous_track_button_visible(&self, value: bool) {
        self.set_value(Self::is_previous_track_button_visible_property(), value);
    }
    pub fn is_repeat_button_visible(&self) -> bool {
        self.get_value(Self::is_repeat_button_visible_property())
    }
    pub fn set_is_repeat_button_visible(&self, value: bool) {
        self.set_value(Self::is_repeat_button_visible_property(), value);
    }
    pub fn is_full_window_enabled(&self) -> bool {
        self.get_value(Self::is_full_window_enabled_property())
    }
    pub fn set_is_full_window_enabled(&self, value: bool) {
        self.set_value(Self::is_full_window_enabled_property(), value);
    }
    pub fn is_zoom_enabled(&self) -> bool {
        self.get_value(Self::is_zoom_enabled_property())
    }
    pub fn set_is_zoom_enabled(&self, value: bool) {
        self.set_value(Self::is_zoom_enabled_property(), value);
    }
    pub fn is_fast_forward_enabled(&self) -> bool {
        self.get_value(Self::is_fast_forward_enabled_property())
    }
    pub fn set_is_fast_forward_enabled(&self, value: bool) {
        self.set_value(Self::is_fast_forward_enabled_property(), value);
    }
    pub fn is_fast_rewind_enabled(&self) -> bool {
        self.get_value(Self::is_fast_rewind_enabled_property())
    }
    pub fn set_is_fast_rewind_enabled(&self, value: bool) {
        self.set_value(Self::is_fast_rewind_enabled_property(), value);
    }
    pub fn is_stop_enabled(&self) -> bool {
        self.get_value(Self::is_stop_enabled_property())
    }
    pub fn set_is_stop_enabled(&self, value: bool) {
        self.set_value(Self::is_stop_enabled_property(), value);
    }
    pub fn is_volume_enabled(&self) -> bool {
        self.get_value(Self::is_volume_enabled_property())
    }
    pub fn set_is_volume_enabled(&self, value: bool) {
        self.set_value(Self::is_volume_enabled_property(), value);
    }
    pub fn is_playback_rate_enabled(&self) -> bool {
        self.get_value(Self::is_playback_rate_enabled_property())
    }
    pub fn set_is_playback_rate_enabled(&self, value: bool) {
        self.set_value(Self::is_playback_rate_enabled_property(), value);
    }
    pub fn is_seek_enabled(&self) -> bool {
        self.get_value(Self::is_seek_enabled_property())
    }
    pub fn set_is_seek_enabled(&self, value: bool) {
        self.set_value(Self::is_seek_enabled_property(), value);
    }
    pub fn is_skip_forward_enabled(&self) -> bool {
        self.get_value(Self::is_skip_forward_enabled_property())
    }
    pub fn set_is_skip_forward_enabled(&self, value: bool) {
        self.set_value(Self::is_skip_forward_enabled_property(), value);
    }
    pub fn is_skip_backward_enabled(&self) -> bool {
        self.get_value(Self::is_skip_backward_enabled_property())
    }
    pub fn set_is_skip_backward_enabled(&self, value: bool) {
        self.set_value(Self::is_skip_backward_enabled_property(), value);
    }
    pub fn is_repeat_enabled(&self) -> bool {
        self.get_value(Self::is_repeat_enabled_property())
    }
    pub fn set_is_repeat_enabled(&self, value: bool) {
        self.set_value(Self::is_repeat_enabled_property(), value);
    }
    pub fn fast_play_fallback_behaviour(&self) -> FastPlayFallbackBehaviour {
        self.get_value(Self::fast_play_fallback_behaviour_property())
    }
    pub fn set_fast_play_fallback_behaviour(&self, value: FastPlayFallbackBehaviour) {
        self.set_value(Self::fast_play_fallback_behaviour_property(), value);
    }
    pub fn show_and_hide_automatically(&self) -> bool {
        self.get_value(Self::show_and_hide_automatically_property())
    }
    pub fn set_show_and_hide_automatically(&self, value: bool) {
        self.set_value(Self::show_and_hide_automatically_property(), value);
    }
    /// Shows the control panel (WinUI's `Show`), and starts the time after which
    /// `ShowAndHideAutomatically` hides it again.
    pub fn show(&self) {
        if self.panel_hidden.replace(false) {
            self.update_states();
        }
        self.start_hide_timer();
    }
    /// Hides the control panel (WinUI's `Hide`): the `ControlPanelFadeOut` state.
    pub fn hide(&self) {
        self.stop_hide_timer();
        self.hide_menu();
        if !self.panel_hidden.replace(true) {
            self.update_states();
        }
    }
    pub fn is_zoomed(&self) -> bool {
        self.zoomed.get()
    }
    pub fn set_is_zoomed(&self, zoomed: bool) {
        let handler = self.zoom_handler.borrow().clone();
        if let Some(handler) = handler {
            if self.zoomed.replace(zoomed) != zoomed {
                handler(zoomed);
            }
        }
    }
    pub(crate) fn set_zoom_handler(&self, handler: Rc<dyn Fn(bool)>) {
        *self.zoom_handler.borrow_mut() = Some(handler);
        self.update_visibility(self.bounds().width);
    }
    fn can_zoom(&self) -> bool {
        self.is_zoom_button_visible() && self.zoom_handler.borrow().is_some()
    }
    fn media_player(&self) -> Option<MediaPlayer> {
        self.player
            .borrow()
            .as_ref()
            .and_then(WeakMediaPlayer::upgrade)
    }
    /// WinUI's hide timer: while the media plays, the panel fades out after
    /// `ControlPanelDisplayTimeoutInSecs` without pointer input over the panel, keyboard focus in
    /// it or an open menu.
    fn start_hide_timer(&self) {
        if !self.show_and_hide_automatically() || !self.is_attached_to_visual_tree() {
            self.stop_hide_timer();
            return;
        }
        let timer = self
            .hide_timer
            .borrow_mut()
            .get_or_insert_with(|| {
                let timer = DispatcherTimer::with_priority(DispatcherPriority::BACKGROUND);
                timer.set_interval(CONTROL_PANEL_DISPLAY_TIMEOUT);
                let weak = self.to_ref().downgrade();
                // The handler lives as long as the timer (its token is not removed).
                let _ = timer.tick(move |timer| {
                    timer.stop();
                    if let Some(view) = weak.upgrade() {
                        if view.should_hide() {
                            view.hide();
                        }
                    }
                });
                timer
            })
            .clone();
        // Each input restarts the time (one timer for the life of the control).
        timer.stop();
        timer.start();
    }
    fn stop_hide_timer(&self) {
        if let Some(timer) = self.hide_timer.borrow().as_ref() {
            timer.stop();
        }
    }
    fn should_hide(&self) -> bool {
        let playing = self.media_player().is_some_and(|player| {
            is_playing(&player) && player.playback_session().state == MediaPlaybackState::Playing
        });
        let menu_open = self
            .parts
            .borrow()
            .menu_buttons()
            .chain(self.parts.borrow().volume_mute.as_ref())
            .any(|button| button.flyout().is_some_and(|menu| menu.is_open()));
        self.show_and_hide_automatically()
            && playing
            && !self.is_pointer_over()
            && !self.is_keyboard_focus_within()
            && !menu_open
    }
    fn toggle_full_window(&self) {
        let action = self.full_window_handler.borrow().clone();
        if let Some(action) = action {
            action();
        }
    }
    fn toggle_play(&self) {
        if let Some(player) = self.media_player() {
            // The requested transport state, not the native one: a player that is
            // buffering (or opening) towards playback is paused by this button.
            if is_playing(&player) {
                if let Err(e) = player.pause() {
                    self.show_error(&e.to_string());
                }
            } else {
                self.command(player.play());
            }
        }
    }
    fn stop_playback(&self) {
        if let Some(player) = self.media_player() {
            let stop = player.stop();
            self.command(Box::pin(async move { stop.await.map(|_| ()) }));
        }
    }
    fn toggle_mute(&self) {
        if let Some(player) = self.media_player() {
            let mut options = player.options();
            options.muted = !options.muted;
            if let Err(e) = player.set_options(options) {
                self.show_error(&e.to_string());
            }
            self.refresh();
        }
    }
    /// WinUI's repeat button without a playlist: `RepeatNoneState` and `RepeatOneState`, the
    /// looping of the player.
    fn toggle_repeat(&self) {
        if let Some(player) = self.media_player() {
            let mut options = player.options();
            options.looping = !options.looping;
            if let Err(e) = player.set_options(options) {
                self.show_error(&e.to_string());
            }
        }
        self.refresh();
    }
    /// WinUI's fast forward and rewind: the trick rates where the player can change its rate
    /// forward, else `FastPlayFallbackBehaviour` (the portable players play forward only, so
    /// rewind always falls back).
    fn fast_play(&self, forward: bool) {
        let Some(player) = self.media_player() else {
            return;
        };
        if forward && player.supports_playback_rate() {
            let rate = next_fast_forward_rate(player.playback_rate());
            self.player_action(|player| player.set_playback_rate(rate));
        } else if self.fast_play_fallback_behaviour() == FastPlayFallbackBehaviour::Skip {
            self.skip(forward);
        }
    }
    fn fast_play_supported(&self, forward: bool, player: Option<&MediaPlayer>) -> bool {
        forward && player.is_some_and(MediaPlayer::supports_playback_rate)
    }
    /// WinUI's `VolumeFlyout` of `VolumeMuteButton`: `AudioMuteButton`, `VolumeSlider` (0 to
    /// 100) and `VolumeValue` in a row, with the values of the source template. The presenter
    /// takes the keyed theme `MediaTransportControlsFlyoutStyle` of the theme (the `FlyoutStyle`
    /// of the source template), where it has one.
    fn make_volume_flyout(&self, button: &Button, parts: &mut Parts) {
        use ferroui_base::{
            controls::ResourceKey,
            data::core::{ValueType, ValueTypes},
            layout::{HorizontalAlignment, Orientation, VerticalAlignment},
            media::TextAlignment,
            styling::ControlTheme,
            Thickness,
        };
        let theme = |key: &str| {
            self.try_find_resource(
                &ResourceKey::from(key),
                self.actual_theme_variant().as_ref(),
            )
            .flatten()
            .and_then(|value| {
                ValueTypes::try_cast(&value, ValueType::of::<Option<Ref<ControlTheme>>>())
            })
            .and_then(|value| {
                value
                    .downcast_ref::<Option<Ref<ControlTheme>>>()
                    .cloned()
                    .flatten()
            })
        };
        let row = ferroui_controls::StackPanel::new();
        row.set_orientation(Orientation::Horizontal);
        row.set_margin(Thickness::new(3., 11., 3., 11.));
        // An AppBarButton of the style of the source template (40 by 40, no label).
        let mute = ferroui_controls::CommandBarButton::new();
        mute.set_name(Some("AudioMuteButton".into()));
        mute.set_width(COMMAND_WIDTH);
        mute.set_height(COMMAND_WIDTH);
        mute.set_label_position(ferroui_controls::CommandBarDefaultLabelPosition::Collapsed);
        mute.set_vertical_alignment(VerticalAlignment::Center);
        let mute = mute.upcast::<Button>();
        set_icon(&mute, SPEAKER);
        let slider = Slider::new();
        slider.set_name(Some("VolumeSlider".into()));
        slider.set_maximum(100.);
        slider.set_range_value(50.);
        slider.set_width(190.);
        slider.set_height(32.);
        slider.set_vertical_alignment(VerticalAlignment::Center);
        slider.set_margin(Thickness::new(12., 0., 8., 0.));
        AutomationProperties::set_name(&slider, Some("Volume".into()));
        let value = TextBlock::new();
        value.set_name(Some("VolumeValue".into()));
        value.set_width(24.);
        value.set_margin(Thickness::new(8., 0., 16., 0.));
        value.set_vertical_alignment(VerticalAlignment::Center);
        value.set_horizontal_alignment(HorizontalAlignment::Center);
        value.set_text_alignment(TextAlignment::Right);
        row.children().add(mute.clone());
        row.children().add(slider.clone());
        row.children().add(value.clone());
        let flyout = Flyout::new();
        if let Some(theme) = theme("MediaTransportControlsFlyoutStyle") {
            flyout.set_flyout_presenter_theme(theme);
        }
        flyout.set_content(Some(Control::boxed(row)));
        button.set_flyout(&flyout);
        parts.audio_mute = Some(mute);
        parts.volume = Some(slider);
        parts.volume_value = Some(value);
        parts.volume_flyout_made = true;
    }
    fn bind_menu(&self, button: &Button, generation: u64, populate: fn(&Self, &MenuFlyout)) {
        let menu = MenuFlyout::new();
        let weak = self.to_ref().downgrade();
        let weak_menu = menu.downgrade();
        self.menu_openings.borrow_mut().push(menu.opening(move |_| {
            if let (Some(view), Some(menu)) = (
                weak.upgrade()
                    .filter(|view| view.template_generation.get() == generation),
                weak_menu.upgrade(),
            ) {
                menu.items().clear();
                populate(&view, &menu);
            }
        }));
        button.set_flyout(&menu);
    }
    /// The menu of the 0.5.0 template: the commands its row does not have.
    fn populate_menu(&self, menu: &MenuFlyout) {
        let player = self.media_player();
        let seekable = player
            .as_ref()
            .is_some_and(|p| p.playback_session().seekable);
        if self.is_skip_backward_button_visible() {
            let enabled = seekable && self.is_skip_backward_enabled();
            self.add_menu_item(menu, "Skip back 10 seconds", enabled, |view| {
                view.skip(false)
            });
        }
        if self.is_skip_forward_button_visible() {
            let enabled = seekable && self.is_skip_forward_enabled();
            self.add_menu_item(menu, "Skip forward 30 seconds", enabled, |view| {
                view.skip(true)
            });
        }
        if self.can_zoom() {
            let label = if self.is_zoomed() {
                "Fit video"
            } else {
                "Zoom to fill"
            };
            self.add_menu_item(menu, label, self.is_zoom_enabled(), |view| {
                view.set_is_zoomed(!view.is_zoomed())
            });
        }
        if self.full_window_in_menu.get() {
            self.add_menu_item(
                menu,
                self.full_window_name(),
                self.is_full_window_enabled(),
                Self::toggle_full_window,
            );
        }
        if let Some(player) = player {
            if self.is_playback_rate_button_visible() && player.supports_playback_rate() {
                let submenu = MenuItem::new();
                submenu.set_header(Some(Rc::new("Playback speed".to_owned())));
                submenu.set_is_enabled(self.is_playback_rate_enabled());
                for item in self.rate_items(&player) {
                    submenu.items().add(Some(Control::boxed(item)));
                }
                menu.items().add(Some(Control::boxed(submenu)));
            }
            for (visible, kind, label) in [
                (
                    self.is_closed_captions_button_visible(),
                    MediaTrackKind::Captions,
                    "Closed captions",
                ),
                (
                    self.is_audio_tracks_button_visible(),
                    MediaTrackKind::Audio,
                    "Audio tracks",
                ),
            ] {
                if visible && Self::has_track_choice(&player, kind) {
                    let submenu = MenuItem::new();
                    submenu.set_header(Some(Rc::new(label.to_owned())));
                    for item in self.track_items(&player, kind) {
                        submenu.items().add(Some(Control::boxed(item)));
                    }
                    menu.items().add(Some(Control::boxed(submenu)));
                }
            }
            if self.is_cast_button_visible() && player.supports_casting() {
                self.add_menu_item(menu, "Cast to Device", true, |view| {
                    if let Some(player) = view.media_player() {
                        view.command(player.cast());
                    }
                });
            }
        }
    }
    /// The menu of WinUI's `PlaybackRateButton`.
    fn populate_rates(&self, menu: &MenuFlyout) {
        if let Some(player) = self.media_player() {
            for item in self.rate_items(&player) {
                menu.items().add(Some(Control::boxed(item)));
            }
        }
    }
    /// The menus of WinUI's `CCSelectionButton` and `AudioTracksSelectionButton`.
    fn populate_tracks(&self, menu: &MenuFlyout, kind: MediaTrackKind) {
        if let Some(player) = self.media_player() {
            for item in self.track_items(&player, kind) {
                menu.items().add(Some(Control::boxed(item)));
            }
        }
    }
    fn rate_items(&self, player: &MediaPlayer) -> Vec<Ref<MenuItem>> {
        PLAYBACK_RATES
            .into_iter()
            .map(|rate| {
                let item = self.menu_item(&format!("{rate}×"), true, move |view| {
                    view.player_action(|player| player.set_playback_rate(rate));
                });
                Self::mark_choice(&item, rate == player.playback_rate());
                item
            })
            .collect()
    }
    /// Whether there is a choice of tracks: one caption track (with Off), or two audio tracks.
    fn has_track_choice(player: &MediaPlayer, kind: MediaTrackKind) -> bool {
        let captions = kind == MediaTrackKind::Captions;
        player.tracks(kind).len() >= if captions { 1 } else { 2 }
    }
    fn track_items(&self, player: &MediaPlayer, kind: MediaTrackKind) -> Vec<Ref<MenuItem>> {
        let tracks = player.tracks(kind);
        let mut items = Vec::new();
        if kind == MediaTrackKind::Captions {
            let off = self.menu_item("Off", true, move |view| {
                view.player_action(|player| player.select_track(kind, None));
            });
            Self::mark_choice(&off, !tracks.iter().any(|track| track.selected));
            items.push(off);
        }
        for track in tracks {
            let id = track.id;
            let item = self.menu_item(&track.label, true, move |view| {
                view.player_action(|player| player.select_track(kind, Some(id)));
            });
            Self::mark_choice(&item, track.selected);
            items.push(item);
        }
        items
    }
    fn add_menu_item(
        &self,
        menu: &MenuFlyout,
        label: &str,
        enabled: bool,
        action: impl Fn(&Self) + 'static,
    ) {
        menu.items()
            .add(Some(Control::boxed(self.menu_item(label, enabled, action))));
    }
    fn menu_item(
        &self,
        label: &str,
        enabled: bool,
        action: impl Fn(&Self) + 'static,
    ) -> Ref<MenuItem> {
        let item = MenuItem::new();
        item.set_header(Some(Rc::new(label.to_owned())));
        item.set_is_enabled(enabled);
        let weak = self.to_ref().downgrade();
        let generation = self.template_generation.get();
        item.click(move |_, _| {
            if let Some(view) = weak
                .upgrade()
                .filter(|v| v.template_generation.get() == generation)
            {
                action(&view);
            }
        });
        item
    }
    fn mark_choice(item: &MenuItem, selected: bool) {
        item.set_toggle_type(ferroui_controls::MenuItemToggleType::Radio);
        item.set_is_checked(selected);
    }
    fn player_action(
        &self,
        action: impl FnOnce(&MediaPlayer) -> Result<(), ferroui_media_video::MediaError>,
    ) {
        if let Some(player) = self.media_player() {
            match action(&player) {
                Ok(()) => self.clear_error(),
                Err(error) => self.show_error(&error.to_string()),
            }
        }
    }
    fn skip(&self, forward: bool) {
        if let Some(player) = self.media_player() {
            let state = player.playback_session();
            if !state.seekable {
                return;
            }
            let target = if forward {
                let next = state.position.saturating_add(SKIP_FORWARD);
                state.duration.map_or(next, |end| next.min(end))
            } else {
                state.position.saturating_sub(SKIP_BACKWARD)
            };
            let seek = player.seek(target);
            self.command(Box::pin(async move { seek.await.map(|_| ()) }));
        }
    }
    fn update_visibility(&self, width: f64) {
        if self.parts.borrow().winui {
            self.update_winui_visibility(width);
        } else {
            self.update_legacy_visibility(width);
        }
    }
    /// The commands of WinUI's template: those whose properties and media allow them, less
    /// those of the lowest dropout order while the row is too narrow.
    fn update_winui_visibility(&self, width: f64) {
        let parts = self.parts.borrow();
        let player = self.media_player();
        let compact = self.is_compact();
        let hide_fast = |forward| {
            self.fast_play_fallback_behaviour() == FastPlayFallbackBehaviour::Hide
                && !self.fast_play_supported(forward, player.as_ref())
        };
        let wanted = |command: Command| match command {
            Command::VolumeMute => self.is_volume_button_visible(),
            Command::Captions => {
                self.is_closed_captions_button_visible()
                    && player
                        .as_ref()
                        .is_some_and(|p| Self::has_track_choice(p, MediaTrackKind::Captions))
            }
            Command::AudioTracks => {
                self.is_audio_tracks_button_visible()
                    && player
                        .as_ref()
                        .is_some_and(|p| Self::has_track_choice(p, MediaTrackKind::Audio))
            }
            Command::Stop => self.is_stop_button_visible(),
            Command::SkipBackward => self.is_skip_backward_button_visible(),
            Command::PreviousTrack => self.is_previous_track_button_visible(),
            Command::Rewind => self.is_fast_rewind_button_visible() && !hide_fast(false),
            Command::PlayPause => self.is_play_pause_button_visible() && !compact,
            Command::FastForward => self.is_fast_forward_button_visible() && !hide_fast(true),
            Command::NextTrack => self.is_next_track_button_visible(),
            Command::SkipForward => self.is_skip_forward_button_visible(),
            Command::PlaybackRate => {
                self.is_playback_rate_button_visible()
                    && player
                        .as_ref()
                        .is_some_and(MediaPlayer::supports_playback_rate)
            }
            Command::Repeat => self.is_repeat_button_visible(),
            Command::Zoom => self.can_zoom(),
            Command::Cast => {
                self.is_cast_button_visible()
                    && player.as_ref().is_some_and(MediaPlayer::supports_casting)
            }
            Command::FullWindow => {
                self.is_full_window_button_visible() && self.full_window_handler.borrow().is_some()
            }
            Command::More => true,
        };
        let candidates: Vec<(Command, f64)> = Command::ROW
            .into_iter()
            .filter_map(|command| {
                let part = command.part(&parts)?;
                let part_width = part.width();
                let part_width = if part_width.is_finite() {
                    part_width
                } else {
                    COMMAND_WIDTH
                };
                wanted(command).then_some((command, part_width))
            })
            .collect();
        // The row: the control less its border (WinUI's MediaTransportControlsBorderThickness);
        // in the compact layout also less the play button on the left and the seek bar.
        let mut row = width - 2.;
        if compact {
            row -= 6. + COMMAND_WIDTH;
            if self.is_seek_bar_visible() {
                row -= COMPACT_SEEK_WIDTH;
            }
        }
        let kept = if width.is_finite() {
            fitting_commands(&candidates, row.max(0.))
        } else {
            candidates.iter().map(|(command, _)| *command).collect()
        };
        for command in Command::ROW {
            if let Some(part) = command.part(&parts) {
                part.set_is_visible(kept.contains(&command));
            }
        }
        if let Some(play) = &parts.play_left {
            play.set_is_visible(compact && self.is_play_pause_button_visible());
        }
        if let Some(seek) = &parts.seek {
            seek.set_is_visible(self.is_seek_bar_visible());
        }
        if let Some(volume) = &parts.volume {
            volume.set_is_visible(self.is_volume_slider_visible());
        }
        self.full_window_in_menu.set(false);
    }
    /// The 0.5.0 template: a row of play, stop, time, status, mute, volume, full window and the
    /// menu, whose parts are shown by the width of the control.
    fn update_legacy_visibility(&self, width: f64) {
        let parts = self.parts.borrow();
        let full_window =
            self.is_full_window_button_visible() && self.full_window_handler.borrow().is_some();
        let more = self.is_skip_backward_button_visible()
            || self.is_skip_forward_button_visible()
            || self.can_zoom()
            || self.is_playback_rate_button_visible()
            || self.is_audio_tracks_button_visible()
            || self.is_closed_captions_button_visible()
            || self.is_cast_button_visible();
        self.full_window_in_menu
            .set(full_window && more && width < 184. && parts.more.is_some());
        for (button, visible) in [
            (&parts.play, self.is_play_pause_button_visible()),
            (
                &parts.stop,
                self.is_stop_button_visible()
                    && !self.is_compact()
                    && width >= if more { 224. } else { 200. },
            ),
            (&parts.volume_mute, self.is_volume_button_visible()),
            (&parts.more, more),
            (
                &parts.full_window,
                full_window && !self.full_window_in_menu.get(),
            ),
        ] {
            if let Some(button) = button {
                button.set_is_visible(visible);
            }
        }
        if let Some(seek) = &parts.seek {
            seek.set_is_visible(self.is_seek_bar_visible());
        }
        if let Some(volume) = &parts.volume {
            volume.set_is_visible(
                self.is_volume_slider_visible() && !self.is_compact() && width >= 500.,
            );
        }
        if let Some(status) = &parts.status {
            status.set_is_visible(!self.is_compact() && width >= 640.);
        }
        if let Some(clock) = &parts.clock {
            clock.set_is_visible(!self.is_compact() && width >= 320.);
        }
    }
    fn update_full_window_name(&self) {
        if let Some(button) = self.parts.borrow().full_window.as_ref() {
            set_name(button, self.full_window_name());
        }
    }
    fn full_window_name(&self) -> &'static str {
        if self.is_full_window() {
            "Exit full window"
        } else {
            "Enter full window"
        }
    }
    pub fn set_media_player(&self, player: Option<&MediaPlayer>) {
        self.hide_menu();
        self.command_generation
            .set(self.command_generation.get().wrapping_add(1));
        self.clear_error();
        self.subscription.borrow_mut().take();
        self.last_state.set(None);
        *self.player.borrow_mut() = player.as_ref().map(|p| p.downgrade());
        self.subscribe();
        self.refresh();
        self.update_visibility(self.bounds().width);
    }
    fn subscribe(&self) {
        if self.subscription.borrow().is_some() {
            return;
        }
        if let Some(player) = self.media_player() {
            let weak = self.to_ref().downgrade();
            *self.subscription.borrow_mut() = Some(player.events().subscribe(move |event| {
                if let Some(view) = weak.upgrade() {
                    match event {
                        // The player reports its state on each poll: only a change of the state
                        // changes the panel.
                        MediaPlayerEvent::StateChanged(state)
                            if view.last_state.replace(Some(state.state)) != Some(state.state) =>
                        {
                            if matches!(
                                state.state,
                                MediaPlaybackState::Closed
                                    | MediaPlaybackState::Failed
                                    | MediaPlaybackState::Opening
                            ) {
                                view.hide_menu();
                            }
                            // The media and its tracks change what the row can hold.
                            view.update_visibility(view.bounds().width);
                            if state.state == MediaPlaybackState::Playing {
                                view.start_hide_timer();
                            } else {
                                view.show();
                            }
                        }
                        MediaPlayerEvent::Opened
                        | MediaPlayerEvent::TrackSelectionChanged(_)
                        | MediaPlayerEvent::PlaybackRateChanged(_) => {
                            view.update_visibility(view.bounds().width)
                        }
                        _ => {}
                    }
                    view.refresh();
                }
            }));
        }
    }
    fn hide_menu(&self) {
        let menus: Vec<_> = self
            .parts
            .borrow()
            .menu_buttons()
            .chain(self.parts.borrow().volume_mute.as_ref())
            .filter_map(|button| button.flyout())
            .collect();
        for menu in menus {
            menu.hide();
        }
    }
    fn command(&self, task: MediaFuture<()>) {
        let generation = self.command_generation.get().wrapping_add(1);
        self.command_generation.set(generation);
        let weak = self.to_ref().downgrade();
        Dispatcher::ui_thread().spawn_local(async move {
            let result = task.await;
            if let Some(view) = weak
                .upgrade()
                .filter(|v| v.command_generation.get() == generation)
            {
                match result {
                    Err(error) if error.kind == MediaErrorKind::Cancelled => {}
                    Err(error) => view.show_error(&error.to_string()),
                    Ok(()) => view.clear_error(),
                }
            }
        });
    }
    fn show_error(&self, error: &str) {
        if let Some(label) = self.parts.borrow().error.as_ref() {
            label.set_text(Some(error));
            label.set_is_visible(true);
            ToolTip::set_tip(label, Some(Rc::new(error.to_owned())));
        }
        if !self.has_error.replace(true) {
            self.update_states();
        }
    }
    fn clear_error(&self) {
        if let Some(label) = self.parts.borrow().error.as_ref() {
            label.set_is_visible(false);
            label.set_text(None);
        }
        if self.has_error.replace(false) {
            self.update_states();
        }
    }
    fn go_to_state(&self, state: State) {
        let pseudo_classes = self.pseudo_classes();
        for name in state.group {
            pseudo_classes.set(name, state.pseudo_class == Some(*name));
        }
    }
    /// WinUI's visual states of the template, as pseudo-classes.
    fn update_states(&self) {
        let player = self.media_player();
        let session = player
            .as_ref()
            .map(|p| p.playback_session())
            .unwrap_or_default();
        let options = player.as_ref().map(|p| p.options()).unwrap_or_default();
        let state = |group, active: bool, pseudo_class| State {
            group,
            pseudo_class: active.then_some(pseudo_class),
        };
        self.go_to_state(state(
            Self::CONTROL_PANEL_VISIBILITY,
            self.panel_hidden.get(),
            Self::PC_CONTROL_PANEL_FADE_OUT,
        ));
        self.go_to_state(State {
            group: Self::MEDIA_STATES,
            pseudo_class: if self.has_error.get() || session.state == MediaPlaybackState::Failed {
                Some(Self::PC_ERROR)
            } else {
                match session.state {
                    MediaPlaybackState::Opening => Some(Self::PC_LOADING),
                    MediaPlaybackState::Buffering => Some(Self::PC_BUFFERING),
                    _ => None,
                }
            },
        });
        let tracks = |kind| {
            player
                .as_ref()
                .is_some_and(|p| Self::has_track_choice(p, kind))
        };
        self.go_to_state(state(
            Self::AUDIO_SELECTION,
            tracks(MediaTrackKind::Audio),
            Self::PC_AUDIO_SELECTION_AVAILABLE,
        ));
        self.go_to_state(state(
            Self::CC_SELECTION,
            tracks(MediaTrackKind::Captions),
            Self::PC_CC_SELECTION_AVAILABLE,
        ));
        self.go_to_state(state(
            Self::CONTROL_MODE,
            self.is_compact(),
            Self::PC_COMPACT_MODE,
        ));
        self.go_to_state(state(
            Self::PLAY_PAUSE,
            player.as_ref().is_some_and(is_playing),
            Self::PC_PAUSE_STATE,
        ));
        self.go_to_state(state(Self::VOLUME_MUTE, options.muted, Self::PC_MUTE_STATE));
        self.go_to_state(state(
            Self::FULL_WINDOW_STATES,
            self.is_full_window(),
            Self::PC_FULL_WINDOW_STATE,
        ));
        self.go_to_state(state(
            Self::REPEAT,
            options.looping,
            Self::PC_REPEAT_ONE_STATE,
        ));
    }
    pub fn refresh(&self) {
        self.updating.set(true);
        let player = self.media_player();
        let state = player
            .as_ref()
            .map(|p| p.playback_session())
            .unwrap_or_default();
        let enabled = !matches!(
            state.state,
            MediaPlaybackState::Closed | MediaPlaybackState::Failed | MediaPlaybackState::Opening
        );
        let parts = self.parts.borrow();
        let playing = player.as_ref().is_some_and(is_playing);
        if self.playing_icon.replace(playing) != playing {
            let label = if playing { "Pause" } else { "Play" };
            for play in [&parts.play, &parts.play_left].into_iter().flatten() {
                set_name(play, label);
            }
            if let (Some(play), false) = (&parts.play, parts.winui) {
                set_icon(play, if playing { PAUSE } else { PLAY });
            }
        }
        for play in [&parts.play, &parts.play_left].into_iter().flatten() {
            play.set_is_enabled(enabled);
        }
        let set_enabled = |part: &Option<Ref<Button>>, value: bool| {
            if let Some(part) = part {
                part.set_is_enabled(value);
            }
        };
        let rate = player
            .as_ref()
            .is_some_and(MediaPlayer::supports_playback_rate);
        let fast_enabled = |forward: bool| {
            if self.fast_play_supported(forward, player.as_ref()) {
                true
            } else {
                match self.fast_play_fallback_behaviour() {
                    FastPlayFallbackBehaviour::Skip => state.seekable,
                    FastPlayFallbackBehaviour::Hide | FastPlayFallbackBehaviour::Disable => false,
                }
            }
        };
        set_enabled(&parts.stop, enabled && self.is_stop_enabled());
        set_enabled(
            &parts.skip_backward,
            enabled && state.seekable && self.is_skip_backward_enabled(),
        );
        set_enabled(
            &parts.skip_forward,
            enabled && state.seekable && self.is_skip_forward_enabled(),
        );
        set_enabled(
            &parts.rewind,
            enabled && self.is_fast_rewind_enabled() && fast_enabled(false),
        );
        set_enabled(
            &parts.fast_forward,
            enabled && self.is_fast_forward_enabled() && fast_enabled(true),
        );
        // The portable player has no playlist (WinUI's MediaPlaybackList).
        set_enabled(&parts.previous_track, false);
        set_enabled(&parts.next_track, false);
        set_enabled(
            &parts.playback_rate,
            enabled && rate && self.is_playback_rate_enabled(),
        );
        set_enabled(&parts.repeat, player.is_some() && self.is_repeat_enabled());
        set_enabled(&parts.zoom, self.is_zoom_enabled());
        set_enabled(&parts.full_window, self.is_full_window_enabled());
        set_enabled(&parts.captions, enabled);
        set_enabled(&parts.audio_tracks, enabled);
        set_enabled(&parts.cast, enabled);
        set_enabled(&parts.volume_mute, self.is_volume_enabled());
        set_enabled(&parts.audio_mute, self.is_volume_enabled());
        if let Some(volume) = &parts.volume {
            volume.set_is_enabled(self.is_volume_enabled());
        }
        if let Some(seek) = &parts.seek {
            seek.set_is_enabled(enabled && state.seekable && self.is_seek_enabled());
            seek.set_maximum(
                state
                    .duration
                    .unwrap_or(Duration::from_secs(1))
                    .as_secs_f64()
                    .max(1.),
            );
            // The position is not written back while the user drags the thumb: the drag
            // owns the value, and each change of it seeks.
            if !seek.is_dragging() {
                seek.set_range_value(state.position.as_secs_f64());
            }
        }
        if let Some(player) = &player {
            let options = player.options();
            if let Some(volume) = &parts.volume {
                volume.set_range_value(volume_to_slider(volume, options.volume));
                if let Some(value) = &parts.volume_value {
                    value.set_text(Some(&format!("{:.0}", volume.value())));
                }
            }
            if self.muted_icon.replace(options.muted) != options.muted {
                let label = if options.muted { "Unmute" } else { "Mute" };
                for mute in [&parts.volume_mute, &parts.audio_mute]
                    .into_iter()
                    .flatten()
                {
                    set_name(mute, label);
                }
                let icon = if options.muted { MUTED } else { SPEAKER };
                if let (Some(mute), false) = (&parts.volume_mute, parts.winui) {
                    set_icon(mute, icon);
                }
                if let (Some(mute), true) = (&parts.audio_mute, parts.volume_flyout_made) {
                    set_icon(mute, icon);
                }
            }
            if let Some(repeat) = parts.repeat.as_ref().and_then(|b| b.cast::<ToggleButton>()) {
                repeat.set_is_checked(Some(options.looping));
                let name = if options.looping {
                    "Repeat One"
                } else {
                    "Repeat None"
                };
                AutomationProperties::set_name(&*repeat, Some(name.into()));
            }
        }
        if let Some(status) = &parts.status {
            let text = match state.state {
                MediaPlaybackState::Closed => "No media",
                MediaPlaybackState::Opening => "Loading",
                MediaPlaybackState::Buffering => "Buffering",
                MediaPlaybackState::Playing => "Playing",
                MediaPlaybackState::Paused => "Paused",
                MediaPlaybackState::Ended => "Ended",
                MediaPlaybackState::Failed => "Unavailable",
            };
            status.set_text(Some(text));
        }
        let duration = state
            .duration
            .map(clock_text)
            .unwrap_or_else(|| "--:--".into());
        if let Some(clock) = &parts.clock {
            clock.set_text(Some(&format!(
                "{} / {duration}",
                clock_text(state.position)
            )));
        }
        if let Some(elapsed) = &parts.elapsed {
            elapsed.set_text(Some(&clock_text(state.position)));
        }
        if let Some(remaining) = &parts.remaining {
            let text = state
                .duration
                .map(|end| clock_text(end.saturating_sub(state.position)))
                .unwrap_or_else(|| "--:--".into());
            remaining.set_text(Some(&text));
        }
        drop(parts);
        if let Some(error) = state.error {
            self.show_error(&error.to_string());
        }
        self.update_states();
        self.updating.set(false);
    }
}

/// Whether the transport is playing or on its way to playing: Play is the requested state and
/// the media has neither ended nor failed.
fn is_playing(player: &MediaPlayer) -> bool {
    player.requested_state() == PlaybackIntent::Play
        && !matches!(
            player.playback_session().state,
            MediaPlaybackState::Ended | MediaPlaybackState::Failed | MediaPlaybackState::Closed
        )
}

impl ControlImpl for MediaTransportControls {
    fn on_create_automation_peer(
        this: &Self,
    ) -> Ref<ferroui_controls::automation::peers::AutomationPeer> {
        crate::automation::MediaTransportControlsAutomationPeer::new(this).upcast()
    }
}

use ferroui_base::{
    CornerRadius, FerroObjectImpl, FerroObjectImplExt, Ref, RelativePoint, RelativeUnit, Size,
    StyledElementImpl, Thickness, VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs,
    ferro_class, ferro_class_info, ferro_impl_classes,
    input::InputElementImpl,
    instantiate,
    interactivity::InteractiveImpl,
    layout::{
        HorizontalAlignment, LayoutableImpl, LayoutableImplExt, Orientation, VerticalAlignment,
    },
    media::immutable::ImmutableSolidColorBrush,
    media::{
        Color, FontFeatureCollection, GradientStop, IBrush, LinearGradientBrush, StreamGeometry,
        TextTrimming, TextWrapping,
    },
    styling::ThemeVariant,
    threading::Dispatcher,
};
use ferroui_controls::{
    Border, Button, ColumnDefinitions, Control, ControlImpl, Grid, PanelImpl, PathIcon, Slider,
    StackPanel, StackPanelImpl, TextBlock, ThemeVariantScope, ToolTip,
    automation::AutomationProperties,
};
use ferroui_media_video::{
    MediaErrorKind, MediaFuture, MediaPlaybackState, MediaPlayer, MediaSubscription, PlaybackIntent,
    WeakMediaPlayer,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};

const ACCENT: Color = Color::from_rgb(128, 197, 255);
const PRIMARY: Color = Color::from_rgb(32, 109, 168);
const PLAY: &str = "M6 3 L17 10 L6 17 Z";
const PAUSE: &str = "M5 3 H8 V17 H5 Z M12 3 H15 V17 H12 Z";
const STOP: &str = "M5 5 H15 V15 H5 Z";
const SPEAKER: &str = "M2 7 H6 L11 3 V17 L6 13 H2 Z M13 6 V8 C15 9 15 11 13 12 V14 C18 12 18 8 13 6 Z M15 2 V4 C21 7 21 13 15 16 V18 C24 14 24 6 15 2 Z";
const MUTED: &str = "M2 7 H6 L11 3 V17 L6 13 H2 Z M14 6 L16 8 L18 6 L19.5 7.5 L17.5 9.5 L19.5 11.5 L18 13 L16 11 L14 13 L12.5 11.5 L14.5 9.5 L12.5 7.5 Z";
fn solid(color: Color) -> Rc<dyn IBrush> {
    Rc::new(ImmutableSolidColorBrush::new(color))
}
fn set_icon(button: &Button, path: &str) {
    let icon = PathIcon::new();
    icon.set_width(18.);
    icon.set_height(18.);
    icon.set_data(Some(
        StreamGeometry::parse(path)
            .expect("media icon geometry")
            .upcast::<ferroui_base::media::Geometry>(),
    ));
    icon.set_foreground(Some(solid(Color::from_rgb(255, 255, 255))));
    button.set_content(Some(Control::boxed(icon)));
}
fn icon_button(name: &str, label: &str, path: &str, primary: bool) -> Ref<Button> {
    let button = Button::new();
    button.set_name(Some(name.into()));
    button.set_width(40.);
    button.set_height(40.);
    button.set_min_width(0.);
    button.set_min_height(0.);
    button.set_padding(Thickness::uniform(0.));
    button.set_corner_radius(CornerRadius::uniform(if primary { 20. } else { 10. }));
    button.set_background(Some(solid(if primary {
        PRIMARY
    } else {
        Color::from_argb(0, 0, 0, 0)
    })));
    button.set_border_thickness(Thickness::uniform(0.));
    button.set_vertical_alignment(VerticalAlignment::Center);
    button.set_horizontal_content_alignment(HorizontalAlignment::Center);
    button.set_vertical_content_alignment(VerticalAlignment::Center);
    AutomationProperties::set_name(&button, Some(label));
    ToolTip::set_tip(&button, Some(Rc::new(label.to_owned())));
    set_icon(&button, path);
    button
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
#[repr(C)]
pub struct MediaTransportControls {
    base: StackPanel,
    player: RefCell<Option<WeakMediaPlayer>>,
    subscription: RefCell<Option<MediaSubscription>>,
    play: RefCell<Option<Ref<Button>>>,
    stop: RefCell<Option<Ref<Button>>>,
    mute: RefCell<Option<Ref<Button>>>,
    seek: RefCell<Option<Ref<Slider>>>,
    volume: RefCell<Option<Ref<Slider>>>,
    status: RefCell<Option<Ref<TextBlock>>>,
    clock: RefCell<Option<Ref<TextBlock>>>,
    error: RefCell<Option<Ref<TextBlock>>>,
    playing_icon: Cell<bool>,
    muted_icon: Cell<bool>,
    command_generation: Cell<u64>,
    updating: Cell<bool>,
}

#[cfg(test)]
mod option_tests {
    use super::*;
    use ferroui_controls::testing::{TestServices, UnitTestApplication};
    use ferroui_media_video::{MediaPlayerProvider, MediaSource, NativeMediaSession};
    struct Unopened;
    #[test]
    fn overlay_stays_inside_player_and_aligns_visible_actions_at_multiple_widths() {
        use ferroui_base::{Point, Rect, Size};
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let transport = MediaTransportControls::new();
        for width in [160., 240., 320., 640., 960.] {
            // Bounds determine compact visibility. Run the subsequent layout
            // pass just as the layout manager does after a resize.
            for _ in 0..2 {
                transport.measure(Size::new(width, 200.));
                transport.arrange(Rect::new(0., 0., width, transport.desired_size().height));
            }
            let seek = transport.seek.borrow().as_ref().unwrap().clone();
            let origin = seek
                .translate_point(Point::new(0., 0.), &transport)
                .unwrap();
            assert!(
                origin.x >= 0. && origin.x + seek.bounds().width <= width,
                "seek width {width}"
            );
            let mut centers = Vec::new();
            for button in [&transport.play, &transport.stop, &transport.mute] {
                let button = button.borrow().as_ref().unwrap().clone();
                if !button.is_visible() {
                    continue;
                }
                let origin = button
                    .translate_point(Point::new(0., 0.), &transport)
                    .unwrap();
                assert!(
                    origin.x >= 0. && origin.x + button.bounds().width <= width,
                    "action width {width}"
                );
                centers.push(origin.y + button.bounds().height / 2.);
            }
            assert!(
                centers.iter().all(|y| (*y - centers[0]).abs() < 0.01),
                "baseline width {width}: {centers:?}"
            );
        }
    }
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
        let transport = MediaTransportControls::new();
        transport.set_media_player(Some(&player));
        let mut options = player.options();
        options.volume = 0.4;
        options.muted = true;
        player.set_options(options).unwrap();
        assert_eq!(transport.volume.borrow().as_ref().unwrap().value(), 0.4);
        assert_eq!(
            AutomationProperties::get_name(transport.mute.borrow().as_ref().unwrap()),
            Some("Unmute".into())
        );
        let mut invalid = player.options();
        invalid.volume = f64::NAN;
        assert!(player.set_options(invalid).is_err());
        assert_eq!(transport.volume.borrow().as_ref().unwrap().value(), 0.4);
    }
}
ferro_class!(MediaTransportControls:StackPanel);
ferro_class_info!(MediaTransportControls {
    new: MediaTransportControls::new
});
ferro_impl_classes!(MediaTransportControls:StyledElementImpl,InteractiveImpl,InputElementImpl,ControlImpl,PanelImpl,StackPanelImpl);
impl LayoutableImpl for MediaTransportControls {
    fn measure_override(this: &Self, available: Size) -> Size {
        // Choose compact content from the parent's constraint before measuring.
        // Waiting for Bounds would let an oversized auto row keep itself wide.
        let width = available.width;
        if let Some(status) = this.status.borrow().as_ref() {
            status.set_is_visible(width >= 640.);
        }
        if let Some(volume) = this.volume.borrow().as_ref() {
            volume.set_is_visible(width >= 500.);
        }
        if let Some(clock) = this.clock.borrow().as_ref() {
            clock.set_is_visible(width >= 280.);
        }
        Self::parent_measure_override(this, available)
    }
}
impl VisualImpl for MediaTransportControls {
    fn on_detached_from_visual_tree(this: &Self, args: &VisualTreeAttachmentEventArgs) {
        this.subscription.borrow_mut().take();
        Self::parent_on_detached_from_visual_tree(this, args);
    }
    fn on_attached_to_visual_tree(this: &Self, args: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, args);
        this.subscribe();
    }
}
impl FerroObjectImpl for MediaTransportControls {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.set_orientation(Orientation::Vertical);
        this.set_clip_to_bounds(true);
        let play = icon_button("MediaPlayPause", "Play", PLAY, true);
        let stop = icon_button("MediaStop", "Stop", STOP, false);
        let mute = icon_button("MediaMute", "Mute", SPEAKER, false);
        let seek = Slider::new();
        seek.set_name(Some("MediaSeek".into()));
        seek.set_height(24.);
        seek.set_horizontal_alignment(HorizontalAlignment::Stretch);
        seek.set_foreground(Some(solid(ACCENT)));
        seek.set_minimum(0.);
        seek.set_maximum(1.);
        AutomationProperties::set_name(&seek, Some("Playback position".into()));
        let volume = Slider::new();
        volume.set_name(Some("MediaVolume".into()));
        volume.set_height(40.);
        volume.set_vertical_alignment(VerticalAlignment::Center);
        volume.set_foreground(Some(solid(ACCENT)));
        volume.set_minimum(0.);
        volume.set_maximum(1.);
        volume.set_range_value(1.);
        AutomationProperties::set_name(&volume, Some("Volume".into()));
        let status = TextBlock::new();
        status.set_name(Some("MediaStatus".into()));
        status.set_font_size(12.);
        status.set_max_width(120.);
        status.set_text_trimming(<dyn TextTrimming>::character_ellipsis());
        status.set_foreground(Some(solid(Color::from_argb(210, 255, 255, 255))));
        status.set_vertical_alignment(VerticalAlignment::Center);
        status.set_margin(Thickness::new(8., 0., 12., 0.));
        status.set_text(Some("No media"));
        let clock = TextBlock::new();
        clock.set_name(Some("MediaTime".into()));
        clock.set_font_size(12.);
        clock.set_max_width(126.);
        clock.set_text_trimming(<dyn TextTrimming>::character_ellipsis());
        clock.set_font_features(Some(FontFeatureCollection::parse("tnum").unwrap()));
        clock.set_foreground(Some(solid(Color::from_rgb(255, 255, 255))));
        clock.set_vertical_alignment(VerticalAlignment::Center);
        clock.set_margin(Thickness::new(8., 0., 8., 0.));
        let row = Grid::new();
        row.set_column_definitions(
            ColumnDefinitions::parse("44,Auto,Auto,*,Auto,40,Auto").unwrap(),
        );
        for (column, child) in [
            (0, play.clone().upcast::<Control>()),
            (1, stop.clone().upcast::<Control>()),
            (2, clock.clone().upcast::<Control>()),
            (4, status.clone().upcast::<Control>()),
            (5, mute.clone().upcast::<Control>()),
            (6, volume.clone().upcast::<Control>()),
        ] {
            Grid::set_column(&child, column);
            row.children().add(child);
        }
        volume.set_width(88.);
        let chrome = StackPanel::new();
        let error = TextBlock::new();
        error.set_name(Some("MediaError".into()));
        error.set_font_size(12.);
        error.set_foreground(Some(solid(Color::from_rgb(255, 255, 255))));
        error.set_text_wrapping(TextWrapping::Wrap);
        error.set_max_height(80.);
        error.set_margin(Thickness::new(0., 0., 0., 8.));
        error.set_is_visible(false);
        chrome.children().add(&error);
        chrome.children().add(&seek);
        chrome.children().add(&row);
        let backdrop = Border::new();
        backdrop.set_padding(Thickness::new(16., 16., 16., 12.));
        let gradient = LinearGradientBrush::new();
        gradient.set_start_point(RelativePoint::new(0., 0., RelativeUnit::Relative));
        gradient.set_end_point(RelativePoint::new(0., 1., RelativeUnit::Relative));
        for (alpha, offset) in [(0, 0.), (175, 0.45), (225, 1.)] {
            gradient
                .gradient_stops()
                .add(GradientStop::with_color_and_offset(
                    Color::from_argb(alpha, 8, 12, 18),
                    offset,
                ));
        }
        backdrop.set_background(Some(gradient.into()));
        backdrop.set_child(Some(chrome.upcast()));
        let scope = ThemeVariantScope::new();
        scope.set_requested_theme_variant(Some(ThemeVariant::dark()));
        // Keep the real themed Slider/Thumb and its focus, keyboard and pointer
        // behavior, with smaller visual thumbs inside the full-height hit area.
        for (name, value) in [
            ("SliderHorizontalThumbWidth", 12.),
            ("SliderHorizontalThumbHeight", 12.),
            ("SliderInnerThumbWidth", 8.),
            ("SliderInnerThumbHeight", 8.),
            ("SliderTrackThemeHeight", 3.),
        ] {
            scope.resources().add_value(name, value);
        }
        for (name, color) in [
            ("SliderThumbBackground", Color::from_rgb(255, 255, 255)),
            ("SliderOuterThumbBackground", Color::from_argb(0, 0, 0, 0)),
            ("SliderThumbBorderBrush", Color::from_argb(0, 0, 0, 0)),
        ] {
            scope.resources().add(name, Some(Rc::new(solid(color))));
        }
        scope.set_child(Some(backdrop.upcast()));
        this.children().add(scope);
        let weak = this.to_ref().downgrade();
        play.click(move |_, _| {
            if let Some(view) = weak.upgrade() {
                let player = view.player.borrow().as_ref().and_then(|p| p.upgrade());
                if let Some(player) = player {
                    // The requested transport state, not the native one: a player that is
                    // buffering (or opening) towards playback is paused by this button.
                    if is_playing(&player) {
                        if let Err(e) = player.pause() {
                            view.show_error(&e.to_string());
                        }
                    } else {
                        view.command(player.play());
                    }
                }
            }
        });
        let weak = this.to_ref().downgrade();
        stop.click(move |_, _| {
            if let Some(view) = weak.upgrade() {
                let player = view.player.borrow().as_ref().and_then(|p| p.upgrade());
                if let Some(player) = player {
                    let stop = player.stop();
                    view.command(Box::pin(async move { stop.await.map(|_| ()) }));
                }
            }
        });
        let weak = this.to_ref().downgrade();
        mute.click(move |_, _| {
            if let Some(view) = weak.upgrade() {
                let player = view.player.borrow().as_ref().and_then(|p| p.upgrade());
                if let Some(player) = player {
                    let mut options = player.options();
                    options.muted = !options.muted;
                    if let Err(e) = player.set_options(options) {
                        view.show_error(&e.to_string());
                    }
                    view.refresh();
                }
            }
        });
        let weak = this.to_ref().downgrade();
        seek.value_changed(move |_, args| {
            if let Some(view) = weak.upgrade() {
                if !view.updating.get() {
                    let player = view.player.borrow().as_ref().and_then(|p| p.upgrade());
                    if let Some(player) = player {
                        if let Ok(position) = Duration::try_from_secs_f64(args.new_value()) {
                            let seek = player.seek(position);
                            view.command(Box::pin(async move { seek.await.map(|_| ()) }));
                        }
                    }
                }
            }
        });
        let weak = this.to_ref().downgrade();
        volume.value_changed(move |_, args| {
            if let Some(view) = weak.upgrade() {
                if !view.updating.get() {
                    let player = view.player.borrow().as_ref().and_then(|p| p.upgrade());
                    if let Some(player) = player {
                        let mut options = player.options();
                        options.volume = args.new_value();
                        if let Err(e) = player.set_options(options) {
                            view.show_error(&e.to_string());
                        }
                    }
                }
            }
        });
        *this.play.borrow_mut() = Some(play);
        *this.stop.borrow_mut() = Some(stop);
        *this.mute.borrow_mut() = Some(mute);
        *this.seek.borrow_mut() = Some(seek);
        *this.volume.borrow_mut() = Some(volume);
        *this.status.borrow_mut() = Some(status);
        *this.clock.borrow_mut() = Some(clock);
        *this.error.borrow_mut() = Some(error);
        this.refresh();
    }
}
impl MediaTransportControls {
    pub fn construct() -> Self {
        Self {
            base: StackPanel::construct(),
            player: RefCell::new(None),
            subscription: RefCell::new(None),
            play: RefCell::new(None),
            stop: RefCell::new(None),
            mute: RefCell::new(None),
            seek: RefCell::new(None),
            volume: RefCell::new(None),
            status: RefCell::new(None),
            clock: RefCell::new(None),
            error: RefCell::new(None),
            playing_icon: Cell::new(false),
            muted_icon: Cell::new(false),
            command_generation: Cell::new(0),
            updating: Cell::new(false),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn set_media_player(&self, player: Option<&MediaPlayer>) {
        self.command_generation
            .set(self.command_generation.get().wrapping_add(1));
        self.clear_error();
        self.subscription.borrow_mut().take();
        *self.player.borrow_mut() = player.as_ref().map(|p| p.downgrade());
        self.subscribe();
        self.refresh();
    }
    fn subscribe(&self) {
        if self.subscription.borrow().is_some() {
            return;
        }
        let player = self.player.borrow().as_ref().and_then(|p| p.upgrade());
        if let Some(player) = player {
            let weak = self.to_ref().downgrade();
            *self.subscription.borrow_mut() = Some(player.events().subscribe(move |_| {
                if let Some(view) = weak.upgrade() {
                    view.refresh();
                }
            }));
        }
    }
    fn command(&self, task: MediaFuture<()>) {
        let generation = self.command_generation.get().wrapping_add(1);
        self.command_generation.set(generation);
        let weak = self.to_ref().downgrade();
        Dispatcher::ui_thread().invoke_async_task_local(move || async move {
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
        if let Some(label) = self.error.borrow().as_ref() {
            label.set_text(Some(error));
            label.set_is_visible(true);
            ToolTip::set_tip(label, Some(Rc::new(error.to_owned())));
        }
    }
    fn clear_error(&self) {
        if let Some(label) = self.error.borrow().as_ref() {
            label.set_is_visible(false);
            label.set_text(None);
        }
    }
    pub fn refresh(&self) {
        self.updating.set(true);
        let player = self.player.borrow().as_ref().and_then(|p| p.upgrade());
        let state = player
            .as_ref()
            .map(|p| p.playback_session())
            .unwrap_or_default();
        let enabled = !matches!(
            state.state,
            MediaPlaybackState::Closed | MediaPlaybackState::Failed | MediaPlaybackState::Opening
        );
        if let Some(play) = self.play.borrow().as_ref() {
            let playing = player.as_ref().is_some_and(is_playing);
            let label = if playing { "Pause" } else { "Play" };
            if self.playing_icon.replace(playing) != playing {
                set_icon(play, if playing { PAUSE } else { PLAY });
                ToolTip::set_tip(play, Some(Rc::new(label.to_owned())));
            }
            AutomationProperties::set_name(play, Some(label.into()));
            play.set_is_enabled(enabled);
        }
        if let Some(stop) = self.stop.borrow().as_ref() {
            stop.set_is_enabled(enabled);
        }
        if let Some(seek) = self.seek.borrow().as_ref() {
            seek.set_is_enabled(enabled && state.seekable);
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
        if let Some(player) = player {
            let options = player.options();
            if let Some(volume) = self.volume.borrow().as_ref() {
                volume.set_range_value(options.volume);
            }
            if let Some(mute) = self.mute.borrow().as_ref() {
                let label = if options.muted { "Unmute" } else { "Mute" };
                if self.muted_icon.replace(options.muted) != options.muted {
                    set_icon(mute, if options.muted { MUTED } else { SPEAKER });
                    ToolTip::set_tip(mute, Some(Rc::new(label.to_owned())));
                }
                AutomationProperties::set_name(mute, Some(label.into()));
            }
        }
        if let Some(error) = state.error {
            self.show_error(&error.to_string());
        }
        if let Some(status) = self.status.borrow().as_ref() {
            let text = {
                match state.state {
                    MediaPlaybackState::Closed => "No media",
                    MediaPlaybackState::Opening => "Loading",
                    MediaPlaybackState::Buffering => "Buffering",
                    MediaPlaybackState::Playing => "Playing",
                    MediaPlaybackState::Paused => "Paused",
                    MediaPlaybackState::Ended => "Ended",
                    MediaPlaybackState::Failed => "Unavailable",
                }
                .to_owned()
            };
            status.set_text(Some(&text));
        }
        if let Some(clock) = self.clock.borrow().as_ref() {
            clock.set_text(Some(&format!(
                "{} / {}",
                clock_text(state.position),
                state
                    .duration
                    .map(clock_text)
                    .unwrap_or_else(|| "--:--".into())
            )));
        }
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

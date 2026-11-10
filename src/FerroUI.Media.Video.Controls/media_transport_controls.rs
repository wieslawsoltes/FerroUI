use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties,
    input::InputElementImpl,
    instantiate,
    interactivity::InteractiveImpl,
    layout::{LayoutableImpl, LayoutableImplExt},
    media::StreamGeometry,
    threading::Dispatcher,
    FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref, Size,
    StyledElementImpl, StyledProperty, VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs,
};
use ferroui_controls::{
    automation::AutomationProperties,
    primitives::{TemplatedControl, TemplatedControlImplExt},
    Button, Control, ControlImpl, PathIcon, Slider, TextBlock, ToolTip,
};
use ferroui_media_video::{
    MediaErrorKind, MediaFuture, MediaPlaybackState, MediaPlayer, MediaSubscription,
    PlaybackIntent, WeakMediaPlayer,
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
fn set_icon(button: &Button, path: &str) {
    let icon = PathIcon::new();
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
#[repr(C)]
pub struct MediaTransportControls {
    base: TemplatedControl,
    template_generation: Cell<u64>,
    player: RefCell<Option<WeakMediaPlayer>>,
    subscription: RefCell<Option<MediaSubscription>>,
    full_window_handler: RefCell<Option<Rc<dyn Fn()>>>,
    full_window: RefCell<Option<Ref<Button>>>,
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
                let part = Slider::new();
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
ferro_class!(MediaTransportControls:TemplatedControl);
ferro_class_info!(MediaTransportControls {
    new: MediaTransportControls::new
});
ferro_impl_classes!(MediaTransportControls:StyledElementImpl,InteractiveImpl,InputElementImpl);
ferro_properties! { impl MediaTransportControls {
    pub fn is_compact_property()->StyledProperty<bool> { FerroProperty::register::<Self,_>("IsCompact",false) }
    pub fn is_play_pause_button_visible_property()->StyledProperty<bool> { FerroProperty::register::<Self,_>("IsPlayPauseButtonVisible",true) }
    pub fn is_stop_button_visible_property()->StyledProperty<bool> { FerroProperty::register::<Self,_>("IsStopButtonVisible",true) }
    pub fn is_seek_bar_visible_property()->StyledProperty<bool> { FerroProperty::register::<Self,_>("IsSeekBarVisible",true) }
    pub fn is_volume_button_visible_property()->StyledProperty<bool> { FerroProperty::register::<Self,_>("IsVolumeButtonVisible",true) }
    pub fn is_volume_slider_visible_property()->StyledProperty<bool> { FerroProperty::register::<Self,_>("IsVolumeSliderVisible",true) }
    pub fn is_full_window_button_visible_property()->StyledProperty<bool> { FerroProperty::register::<Self,_>("IsFullWindowButtonVisible",true) }
    pub fn is_full_window_property()->StyledProperty<bool> { FerroProperty::register::<Self,_>("IsFullWindow",false) }
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
impl FerroObjectImpl for MediaTransportControls {
    fn on_property_changed(this: &Self, args: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, args);
        if args.property() == Self::is_full_window_property().as_property() {
            this.update_full_window_name();
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
        ]
        .contains(&args.property())
        {
            this.update_visibility(this.bounds().width);
            this.invalidate_measure();
        }
    }
}
impl ferroui_controls::primitives::TemplatedControlImpl for MediaTransportControls {
    fn on_template_changed(this: &Self, args: &FerroPropertyChangedEventArgs<'_>) {
        this.template_generation
            .set(this.template_generation.get().wrapping_add(1));
        this.command_generation
            .set(this.command_generation.get().wrapping_add(1));
        for part in [&this.play, &this.stop, &this.mute, &this.full_window] {
            part.borrow_mut().take();
        }
        for part in [&this.seek, &this.volume] {
            part.borrow_mut().take();
        }
        for part in [&this.status, &this.clock, &this.error] {
            part.borrow_mut().take();
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
        let full_window = scope.find_as::<Button>("MediaFullWindow");
        *this.full_window.borrow_mut() = full_window.clone();
        let play = scope.find_as::<Button>("MediaPlayPause");
        *this.play.borrow_mut() = play.clone();
        let stop = scope.find_as::<Button>("MediaStop");
        *this.stop.borrow_mut() = stop.clone();
        let mute = scope.find_as::<Button>("MediaMute");
        *this.mute.borrow_mut() = mute.clone();
        let seek = scope.find_as::<Slider>("MediaSeek");
        *this.seek.borrow_mut() = seek.clone();
        let volume = scope.find_as::<Slider>("MediaVolume");
        *this.volume.borrow_mut() = volume.clone();
        let status = scope.find_as::<TextBlock>("MediaStatus");
        *this.status.borrow_mut() = status.clone();
        let clock = scope.find_as::<TextBlock>("MediaTime");
        *this.clock.borrow_mut() = clock.clone();
        let error = scope.find_as::<TextBlock>("MediaError");
        *this.error.borrow_mut() = error.clone();
        if let Some(play) = &play {
            set_icon(play, PLAY);
        }
        if let Some(stop) = &stop {
            set_icon(stop, STOP);
        }
        if let Some(mute) = &mute {
            set_icon(mute, SPEAKER);
        }
        if let Some(full_window) = &full_window {
            set_icon(full_window, "M2 8 V2 H8 V4 H4 V8 Z M12 2 H18 V8 H16 V4 H12 Z M18 12 V18 H12 V16 H16 V12 Z M8 18 H2 V12 H4 V16 H8 Z");
        }
        this.playing_icon.set(false);
        this.muted_icon.set(false);
        let weak = this.to_ref().downgrade();
        if let Some(full_window) = full_window {
            full_window.click(move |_, _| {
                if let Some(view) = weak
                    .upgrade()
                    .filter(|view| view.template_generation.get() == generation)
                {
                    let action = view.full_window_handler.borrow().clone();
                    if let Some(action) = action {
                        action();
                    }
                }
            });
        }
        let weak = this.to_ref().downgrade();
        if let Some(play) = play {
            play.click(move |_, _| {
                if let Some(view) = weak
                    .upgrade()
                    .filter(|view| view.template_generation.get() == generation)
                {
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
        }
        let weak = this.to_ref().downgrade();
        if let Some(stop) = stop {
            stop.click(move |_, _| {
                if let Some(view) = weak
                    .upgrade()
                    .filter(|view| view.template_generation.get() == generation)
                {
                    let player = view.player.borrow().as_ref().and_then(|p| p.upgrade());
                    if let Some(player) = player {
                        let stop = player.stop();
                        view.command(Box::pin(async move { stop.await.map(|_| ()) }));
                    }
                }
            });
        }
        let weak = this.to_ref().downgrade();
        if let Some(mute) = mute {
            mute.click(move |_, _| {
                if let Some(view) = weak
                    .upgrade()
                    .filter(|view| view.template_generation.get() == generation)
                {
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
        }
        let weak = this.to_ref().downgrade();
        if let Some(seek) = seek {
            seek.value_changed(move |_, args| {
                if let Some(view) = weak
                    .upgrade()
                    .filter(|view| view.template_generation.get() == generation)
                {
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
        }
        let weak = this.to_ref().downgrade();
        if let Some(volume) = volume {
            volume.value_changed(move |_, args| {
                if let Some(view) = weak
                    .upgrade()
                    .filter(|view| view.template_generation.get() == generation)
                {
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
        }
        this.update_visibility(this.bounds().width);
        this.update_full_window_name();
        this.refresh();
    }
}

impl MediaTransportControls {
    pub fn construct() -> Self {
        Self {
            base: ferroui_controls::primitives::TemplatedControl::construct(),
            template_generation: Cell::new(0),
            player: RefCell::new(None),
            subscription: RefCell::new(None),
            full_window: RefCell::new(None),
            full_window_handler: RefCell::new(None),
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
    fn update_visibility(&self, width: f64) {
        for (button, visible) in [
            (&self.play, self.is_play_pause_button_visible()),
            (
                &self.stop,
                self.is_stop_button_visible() && !self.is_compact() && width >= 200.,
            ),
            (&self.mute, self.is_volume_button_visible()),
            (
                &self.full_window,
                self.is_full_window_button_visible() && self.full_window_handler.borrow().is_some(),
            ),
        ] {
            if let Some(button) = button.borrow().as_ref() {
                button.set_is_visible(visible);
            }
        }
        if let Some(seek) = self.seek.borrow().as_ref() {
            seek.set_is_visible(self.is_seek_bar_visible());
        }
        if let Some(volume) = self.volume.borrow().as_ref() {
            volume.set_is_visible(
                self.is_volume_slider_visible() && !self.is_compact() && width >= 500.,
            );
        }
        if let Some(status) = self.status.borrow().as_ref() {
            status.set_is_visible(!self.is_compact() && width >= 640.);
        }
        if let Some(clock) = self.clock.borrow().as_ref() {
            clock.set_is_visible(!self.is_compact() && width >= 320.);
        }
    }
    fn update_full_window_name(&self) {
        if let Some(button) = self.full_window.borrow().as_ref() {
            let label = if self.is_full_window() {
                "Exit full window"
            } else {
                "Enter full window"
            };
            AutomationProperties::set_name(button, Some(label));
            ToolTip::set_tip(button, Some(Rc::new(label.to_owned())));
        }
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

impl ControlImpl for MediaTransportControls {
    fn on_create_automation_peer(
        this: &Self,
    ) -> Ref<ferroui_controls::automation::peers::AutomationPeer> {
        crate::automation::MediaTransportControlsAutomationPeer::new(this).upcast()
    }
}

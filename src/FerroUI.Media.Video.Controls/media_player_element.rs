use crate::{MediaTransportControls, TextureFrameImporter, TexturePresenter};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties,
    input::InputElementImpl,
    instantiate,
    interactivity::InteractiveImpl,
    layout::{LayoutableImpl, VerticalAlignment},
    media::Stretch,
    reactive::IDisposable,
    threading::{Dispatcher, DispatcherPriority, DispatcherTimer},
    FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref,
    StyledElementImpl, StyledProperty, VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs,
};
use ferroui_controls::{Control, ControlImpl, Panel, PanelImpl, TextBlock};
use ferroui_media_video::{
    MediaError, MediaErrorKind, MediaEvent, MediaFuture, MediaPlaybackState, MediaPlayer,
    MediaPlayerEvent, MediaPlayerProvider, MediaSource, MediaSubscription, WeakMediaPlayer,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};
#[repr(C)]
pub struct MediaPlayerElement {
    base: Panel,
    presenter: RefCell<Option<Ref<TexturePresenter>>>,
    chrome: RefCell<Option<Ref<MediaTransportControls>>>,
    player: RefCell<Option<MediaPlayer>>,
    shared_player: RefCell<Option<WeakMediaPlayer>>,
    owns_player: Cell<bool>,
    subscription: RefCell<Option<MediaSubscription>>,
    timer: RefCell<Option<Rc<dyn IDisposable>>>,
    settling_ticks: Cell<u32>,
    errors: MediaEvent<MediaError>,
    presentation_subscription: RefCell<Option<MediaSubscription>>,
    placeholder: RefCell<Option<Ref<TextBlock>>>,
    placeholder_content: RefCell<Option<Ref<Control>>>,
    frame_subscription: RefCell<Option<MediaSubscription>>,
    has_presentation: Cell<bool>,
}
ferro_class!(MediaPlayerElement:Panel);
ferro_class_info!(MediaPlayerElement {
    new: MediaPlayerElement::new
});
ferro_impl_classes!(MediaPlayerElement:StyledElementImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,PanelImpl);
ferro_properties! {impl MediaPlayerElement {
    pub fn autoplay_property()->StyledProperty<bool> {FerroProperty::register::<Self,_>("AutoPlay",false)}
    pub fn are_transport_controls_enabled_property()->StyledProperty<bool> {FerroProperty::register::<Self,_>("AreTransportControlsEnabled",false)}
    pub fn stretch_property()->StyledProperty<Stretch> {FerroProperty::register::<Self,_>("Stretch",Stretch::Uniform)}
}}
impl FerroObjectImpl for MediaPlayerElement {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        let presenter = TexturePresenter::new();
        let chrome = MediaTransportControls::new();
        chrome.set_vertical_alignment(VerticalAlignment::Bottom);
        chrome.set_is_visible(false);
        let placeholder = TextBlock::new();
        placeholder.set_text(Some("No media"));
        let weak = this.to_ref().downgrade();
        *this.presentation_subscription.borrow_mut() =
            Some(presenter.errors().subscribe(move |error| {
                if let Some(view) = weak.upgrade() {
                    if view.owns_player.get() {
                        if let Some(player) = view.media_player() {
                            player.clear_source();
                        }
                    }
                    if let Some(placeholder) = view.placeholder.borrow().as_ref() {
                        placeholder.set_text(Some("Media preview is unavailable"));
                    }
                    view.show_placeholder(true);
                    view.errors.emit(error);
                }
            }));
        let weak = this.to_ref().downgrade();
        *this.frame_subscription.borrow_mut() =
            Some(presenter.frame_presented().subscribe(move |_| {
                if let Some(view) = weak.upgrade() {
                    view.show_placeholder(false);
                }
            }));
        this.children().add(&presenter);
        this.children().add(&chrome);
        this.children().add(&placeholder);
        *this.placeholder.borrow_mut() = Some(placeholder);
        *this.presenter.borrow_mut() = Some(presenter);
        *this.chrome.borrow_mut() = Some(chrome);
    }
    fn on_property_changed(this: &Self, args: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, args);
        if args.property() == Self::stretch_property().as_property() {
            if let Some(presenter) = this.presenter.borrow().as_ref() {
                presenter.set_stretch(this.stretch());
            }
        } else if args.property() == Self::are_transport_controls_enabled_property().as_property() {
            if let Some(chrome) = this.chrome.borrow().as_ref() {
                chrome.set_is_visible(this.are_transport_controls_enabled());
            }
        } else if args.property() == Self::autoplay_property().as_property() {
            if let Some(player) = this.media_player() {
                let mut options = player.options();
                options.autoplay = this.autoplay();
                if let Err(e) = player.set_options(options) {
                    this.errors.emit(&e);
                }
            }
        }
    }
}
impl VisualImpl for MediaPlayerElement {
    fn on_attached_to_visual_tree(this: &Self, args: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, args);
        this.subscribe();
        this.start_polling();
    }
    fn on_detached_from_visual_tree(this: &Self, args: &VisualTreeAttachmentEventArgs) {
        this.stop_polling();
        this.subscription.borrow_mut().take();
        this.show_placeholder(true);
        if this.owns_player.get() {
            if let Some(player) = this.media_player() {
                player.clear_source();
            }
        }
        Self::parent_on_detached_from_visual_tree(this, args);
    }
}
impl MediaPlayerElement {
    pub fn construct() -> Self {
        Self {
            base: Panel::construct(),
            presenter: RefCell::new(None),
            chrome: RefCell::new(None),
            player: RefCell::new(None),
            shared_player: RefCell::new(None),
            owns_player: Cell::new(false),
            subscription: RefCell::new(None),
            timer: RefCell::new(None),
            settling_ticks: Cell::new(0),
            errors: MediaEvent::default(),
            presentation_subscription: RefCell::new(None),
            placeholder: RefCell::new(None),
            placeholder_content: RefCell::new(None),
            frame_subscription: RefCell::new(None),
            has_presentation: Cell::new(false),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn autoplay(&self) -> bool {
        self.get_value(Self::autoplay_property())
    }
    pub fn set_autoplay(&self, value: bool) {
        self.set_value(Self::autoplay_property(), value);
    }
    pub fn are_transport_controls_enabled(&self) -> bool {
        self.get_value(Self::are_transport_controls_enabled_property())
    }
    pub fn set_are_transport_controls_enabled(&self, value: bool) {
        self.set_value(Self::are_transport_controls_enabled_property(), value);
    }
    pub fn stretch(&self) -> Stretch {
        self.get_value(Self::stretch_property())
    }
    pub fn set_stretch(&self, value: Stretch) {
        self.set_value(Self::stretch_property(), value);
    }
    pub fn media_player(&self) -> Option<MediaPlayer> {
        self.player.borrow().clone().or_else(|| {
            self.shared_player
                .borrow()
                .as_ref()
                .and_then(|p| p.upgrade())
        })
    }
    pub fn errors(&self) -> &MediaEvent<MediaError> {
        &self.errors
    }
    /// Supplies normal styled content such as a poster Image or a custom
    /// placeholder. It remains visible until the compositor accepts a frame.
    pub fn set_placeholder(&self, content: Option<Ref<Control>>) {
        let previous = self.placeholder_content.borrow_mut().take();
        if let Some(previous) = previous {
            self.children().remove(&previous);
        }
        if let Some(content) = content.as_ref() {
            self.children().add(content);
        }
        *self.placeholder_content.borrow_mut() = content;
        self.show_placeholder(!self.has_presentation.get());
    }
    fn show_placeholder(&self, visible: bool) {
        self.has_presentation.set(!visible);
        let content = self.placeholder_content.borrow().clone();
        if let Some(placeholder) = self.placeholder.borrow().as_ref() {
            placeholder.set_is_visible(visible && content.is_none());
        }
        if let Some(content) = content {
            content.set_is_visible(visible);
        }
    }
    /// Caller-supplied sessions remain alive when this element detaches.
    pub fn set_media_player(
        &self,
        player: Option<&MediaPlayer>,
        importer: Option<Rc<dyn TextureFrameImporter>>,
    ) {
        self.stop_polling();
        self.subscription.borrow_mut().take();
        self.show_placeholder(true);
        let old = self.media_player();
        let same = old
            .as_ref()
            .zip(player)
            .is_some_and(|(old, new)| old.same_player(new));
        let retain_ownership = self.owns_player.get() && same;
        if self.owns_player.get() && !same {
            if let Some(old) = old {
                old.clear_source();
            }
        }
        self.owns_player.set(retain_ownership);
        *self.shared_player.borrow_mut() = player.map(|p| p.downgrade());
        *self.player.borrow_mut() = if retain_ownership {
            player.cloned()
        } else {
            None
        };
        if let Some(presenter) = self.presenter.borrow().as_ref() {
            presenter.set_frame_source(player.as_ref().map(|p| p.frames()), importer);
        }
        if let Some(chrome) = self.chrome.borrow().as_ref() {
            chrome.set_media_player(player);
        }
        self.subscribe();
        if self.is_attached_to_visual_tree() {
            self.start_polling();
        }
    }
    /// Creates an owned player for a source. This element retires it on detach.
    pub fn set_source(
        &self,
        source: MediaSource,
        provider: Rc<dyn MediaPlayerProvider>,
        importer: Rc<dyn TextureFrameImporter>,
    ) -> MediaFuture<()> {
        let player = MediaPlayer::new(provider);
        let mut options = player.options();
        options.autoplay = self.autoplay();
        if let Err(error) = player.set_options(options) {
            return Box::pin(async move { Err(error) });
        }
        self.set_media_player(Some(&player), Some(importer));
        self.owns_player.set(true);
        *self.player.borrow_mut() = Some(player.clone());
        player.set_source(source)
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
                        MediaPlayerEvent::Opened
                        | MediaPlayerEvent::SeekCompleted(_)
                        | MediaPlayerEvent::IntentChanged(
                            ferroui_media_video::PlaybackIntent::Play,
                        ) => view.start_polling(),
                        MediaPlayerEvent::IntentChanged(
                            ferroui_media_video::PlaybackIntent::Stop,
                        ) => {
                            view.show_placeholder(true);
                            if let Some(presenter) = view.presenter.borrow().as_ref() {
                                presenter.present_latest();
                            }
                            view.start_polling();
                        }
                        MediaPlayerEvent::StateChanged(state) => {
                            if matches!(
                                state.state,
                                MediaPlaybackState::Closed
                                    | MediaPlaybackState::Opening
                                    | MediaPlaybackState::Failed
                            ) {
                                if let Some(placeholder) = view.placeholder.borrow().as_ref() {
                                    placeholder.set_text(Some(match state.state {
                                        MediaPlaybackState::Opening => "Opening media",
                                        MediaPlaybackState::Failed => {
                                            "Media preview is unavailable"
                                        }
                                        _ => "No media",
                                    }));
                                }
                                view.show_placeholder(true);
                                if let Some(presenter) = view.presenter.borrow().as_ref() {
                                    presenter.present_latest();
                                }
                            }
                            if let Some(error) = &state.error {
                                view.errors.emit(error);
                            }
                        }
                        _ => {}
                    }
                }
            }));
        }
    }
    fn stop_polling(&self) {
        if let Some(timer) = self.timer.borrow_mut().take() {
            timer.dispose();
        }
    }
    fn start_polling(&self) {
        // Native commands may be accepted on another looper before actual
        // playback changes. A paused frame must not stop polling before that
        // transition, or before a newly sought frame reaches the producer fence.
        self.settling_ticks.set(30);
        if !self.is_attached_to_visual_tree() || self.timer.borrow().is_some() {
            return;
        }
        let weak = self.to_ref().downgrade();
        let timer = DispatcherTimer::run(
            move || {
                let Some(view) = weak.upgrade() else {
                    return false;
                };
                let Some(player) = view.media_player() else {
                    view.timer.borrow_mut().take();
                    return false;
                };
                match player.poll() {
                    Ok(Some(task)) => {
                        let weak = view.to_ref().downgrade();
                        Dispatcher::ui_thread().invoke_async_task_local(move || async move {
                            if let Err(error) = task.await {
                                if let Some(view) = weak.upgrade() {
                                    view.errors.emit(&error);
                                }
                            }
                        });
                    }
                    Ok(None) => {}
                    Err(error) => view.errors.emit(&error),
                }
                if let Some(presenter) = view.presenter.borrow().as_ref() {
                    presenter.present_latest();
                }
                if player.frames().latest().is_none() {
                    view.show_placeholder(true);
                }
                let settling = view.settling_ticks.get();
                view.settling_ticks.set(settling.saturating_sub(1));
                let state = player.playback_session().state;
                // Settled paused images stop polling. Producer/import work keeps
                // its bounded storage alive until readiness or a reported failure.
                let keep = matches!(
                    state,
                    MediaPlaybackState::Playing
                        | MediaPlaybackState::Buffering
                        | MediaPlaybackState::Opening
                ) || (settling > 0 && state == MediaPlaybackState::Paused)
                    || (matches!(
                        state,
                        MediaPlaybackState::Paused | MediaPlaybackState::Ended
                    ) && player.has_pending_frame_work());
                if !keep {
                    view.timer.borrow_mut().take();
                }
                keep
            },
            Duration::from_millis(33),
            DispatcherPriority::RENDER,
        );
        *self.timer.borrow_mut() = Some(timer);
    }
    pub fn play(&self) -> MediaFuture<()> {
        let Some(player) = self.media_player() else {
            return Box::pin(async {
                Err(MediaError::new(
                    MediaErrorKind::InvalidSource,
                    "No media source",
                ))
            });
        };
        self.start_polling();
        player.play()
    }
}

#[cfg(test)]
mod readiness_tests {
    use super::*;
    use ferroui_controls::{
        testing::{TestServices, UnitTestApplication},
        Window,
    };
    use ferroui_media_video::{frames::FrameLease, MediaPlaybackSession, NativeMediaSession};
    struct Native {
        pending: Cell<bool>,
        reads: Cell<u32>,
    }
    impl NativeMediaSession for Native {
        fn has_pending_frame_work(&self) -> bool {
            self.pending.get()
        }
        fn state(&self) -> Result<MediaPlaybackSession, MediaError> {
            Ok(MediaPlaybackSession {
                state: MediaPlaybackState::Paused,
                ..Default::default()
            })
        }
        fn play(&self) -> MediaFuture<()> {
            Box::pin(async { Ok(()) })
        }
        fn pause(&self) -> Result<(), MediaError> {
            Ok(())
        }
        fn seek(&self, position: Duration) -> MediaFuture<Duration> {
            Box::pin(async move { Ok(position) })
        }
        fn set_audio(&self, _: f64, _: bool) -> Result<(), MediaError> {
            Ok(())
        }
        fn take_frame(&self) -> Result<Option<FrameLease>, MediaError> {
            self.reads.set(self.reads.get() + 1);
            Ok(None)
        }
    }
    struct Provider(Rc<Native>);
    impl MediaPlayerProvider for Provider {
        fn open(
            &self,
            _: MediaSource,
            _: u64,
            _: u64,
            _: ferroui_media_video::frames::PresentationPolicy,
        ) -> MediaFuture<Rc<dyn NativeMediaSession>> {
            let native: Rc<dyn NativeMediaSession> = self.0.clone();
            Box::pin(async move { Ok(native) })
        }
    }
    fn tick() {
        Dispatcher::ui_thread().run_jobs(None);
        for timer in Dispatcher::timers_for_unit_tests() {
            Dispatcher::force_fire_timer_for_unit_tests(&timer);
        }
        Dispatcher::ui_thread().run_jobs(None);
    }
    #[test]
    fn paused_producer_outlives_settling_interval_then_sleeps() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let native = Rc::new(Native {
            pending: Cell::new(true),
            reads: Cell::new(0),
        });
        let player = MediaPlayer::new(Rc::new(Provider(native.clone())));
        let mut open =
            player.set_source(MediaSource::Url("https://example.invalid/video.mp4".into()));
        let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
        assert!(open.as_mut().poll(&mut cx).is_ready());
        let view = MediaPlayerElement::new();
        view.set_media_player(Some(&player), None);
        let window = Window::new();
        window.set_content(Some(Control::boxed(view.clone())));
        window.show();
        for _ in 0..65 {
            tick();
        }
        assert!(
            view.timer.borrow().is_some(),
            "pending GPU work must survive the settling timeout"
        );
        let reads = native.reads.get();
        tick();
        assert!(native.reads.get() > reads);
        native.pending.set(false);
        tick();
        assert!(
            view.timer.borrow().is_none(),
            "a settled paused image must stop polling"
        );
        let reads = native.reads.get();
        for _ in 0..4 {
            tick();
        }
        assert_eq!(native.reads.get(), reads);
        window.close();
        tick();
    }
}

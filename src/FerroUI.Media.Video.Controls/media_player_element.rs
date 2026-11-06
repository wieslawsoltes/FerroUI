use crate::{MediaTransportControls, TextureFrameImporter, TexturePresenter};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties,
    input::InputElementImpl,
    instantiate,
    interactivity::InteractiveImpl,
    layout::{LayoutableImpl, VerticalAlignment},
    media::Stretch,
    reactive::{IDisposable, Subscription},
    rendering::composition::{ElementComposition, ICompositionGpuInterop, RenderContextState},
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
/// Recreates renderer-bound playback and import resources. Capture a shared
/// FrameBudget so retirement from old devices remains charged to the same limit.
pub type MediaPlayerProviderFactory =
    dyn Fn(
        &dyn ICompositionGpuInterop,
    ) -> Result<(Rc<dyn MediaPlayerProvider>, Rc<dyn TextureFrameImporter>), MediaError>;

#[repr(C)]
pub struct MediaPlayerElement {
    base: Panel,
    presenter: RefCell<Option<Ref<TexturePresenter>>>,
    chrome: RefCell<Option<Ref<MediaTransportControls>>>,
    player: RefCell<Option<MediaPlayer>>,
    shared_player: RefCell<Option<WeakMediaPlayer>>,
    owns_player: Cell<bool>,
    subscription: RefCell<Option<MediaSubscription>>,
    timer: RefCell<Option<Subscription>>,
    settling_ticks: Cell<u32>,
    errors: MediaEvent<MediaError>,
    presentation_subscription: RefCell<Option<MediaSubscription>>,
    placeholder: RefCell<Option<Ref<TextBlock>>>,
    placeholder_content: RefCell<Option<Ref<Control>>>,
    frame_subscription: RefCell<Option<MediaSubscription>>,
    has_presentation: Cell<bool>,
    provider_factory: RefCell<Option<Rc<MediaPlayerProviderFactory>>>,
    context_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    device_suspended: Cell<bool>,
    recovery_generation: Cell<u64>,
    source_request: Cell<u64>,
    source_uri_suppressed: Cell<bool>,
    source_waiting_for_context: Cell<bool>,
}
ferro_class!(MediaPlayerElement:Panel);
ferro_class_info!(MediaPlayerElement {
    new: MediaPlayerElement::new
});
ferro_impl_classes!(MediaPlayerElement:StyledElementImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,PanelImpl);
ferro_properties! {impl MediaPlayerElement {
    pub fn source_property()->StyledProperty<Option<String>> {FerroProperty::register::<Self,_>("Source",None)}
    pub fn autoplay_property()->StyledProperty<bool> {FerroProperty::register::<Self,_>("AutoPlay",false)}
    pub fn are_transport_controls_enabled_property()->StyledProperty<bool> {FerroProperty::register::<Self,_>("AreTransportControlsEnabled",false)}
    pub fn stretch_property()->StyledProperty<Stretch> {FerroProperty::register::<Self,_>("Stretch",Stretch::Uniform)}
}}
impl FerroObjectImpl for MediaPlayerElement {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.set_clip_to_bounds(true);
        let presenter = TexturePresenter::new();
        let chrome = MediaTransportControls::new();
        chrome.set_vertical_alignment(VerticalAlignment::Bottom);
        chrome.set_horizontal_alignment(ferroui_base::layout::HorizontalAlignment::Stretch);
        chrome.set_is_visible(false);
        let placeholder = TextBlock::new();
        placeholder.set_text(Some("No media"));
        let weak = this.to_ref().downgrade();
        *this.presentation_subscription.borrow_mut() =
            Some(presenter.errors().subscribe(move |error| {
                if let Some(view) = weak.upgrade() {
                    // A lost device suspends the player, whose failed state carries the error and
                    // is reported by the player's state change: it is not reported twice.
                    let mut reported = false;
                    if view.owns_player.get() {
                        if let Some(player) = view.media_player() {
                            if error.kind == MediaErrorKind::DeviceLost {
                                view.suspend_device();
                                reported = player
                                    .playback_session()
                                    .error
                                    .is_some_and(|e| e.kind == MediaErrorKind::DeviceLost);
                            } else {
                                player.clear_source();
                            }
                        }
                    }
                    if let Some(placeholder) = view.placeholder.borrow().as_ref() {
                        placeholder.set_text(Some("Media preview is unavailable"));
                    }
                    view.show_placeholder(true);
                    if !reported {
                        view.errors.emit(error);
                    }
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
        if args.property() == Self::source_property().as_property() {
            if !this.source_uri_suppressed.get() || this.source_uri().is_some() {
                this.apply_source_uri();
            }
        } else if args.property() == Self::stretch_property().as_property() {
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
        this.subscribe_context();
        if this.source_uri().is_some() {
            this.apply_source_uri();
        }
        this.start_polling();
    }
    fn on_detached_from_visual_tree(this: &Self, args: &VisualTreeAttachmentEventArgs) {
        this.stop_polling();
        this.next_source_request();
        this.source_waiting_for_context.set(false);
        this.cancel_recovery();
        if let Some(subscription) = this.context_subscription.borrow_mut().take() {
            subscription.dispose();
        }
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
            provider_factory: RefCell::new(None),
            context_subscription: RefCell::new(None),
            device_suspended: Cell::new(false),
            recovery_generation: Cell::new(0),
            source_request: Cell::new(0),
            source_uri_suppressed: Cell::new(false),
            source_waiting_for_context: Cell::new(false),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    /// The focused markup `Source` property accepts a file path or supported
    /// network/file URL. Encoded resources use `set_source`; shared sessions use
    /// `set_media_player`. Configure a provider factory before attaching a new
    /// declarative view. Selection alone never enables autoplay.
    pub fn source_uri(&self) -> Option<String> {
        self.get_value(Self::source_property())
    }
    pub fn set_source_uri(&self, source: Option<String>) {
        self.set_value(Self::source_property(), source);
    }
    fn next_source_request(&self) -> u64 {
        let next = self
            .source_request
            .get()
            .checked_add(1)
            .expect("source request exhausted");
        self.source_request.set(next);
        next
    }
    fn clear_source_uri_selection(&self) -> bool {
        let request = self.next_source_request();
        self.source_waiting_for_context.set(false);
        if self.source_uri().is_some() {
            self.source_uri_suppressed.set(true);
            self.set_value(Self::source_property(), None::<String>);
            self.source_uri_suppressed.set(false);
        }
        self.source_request.get() == request
    }
    fn source_uri_error(&self, error: &MediaError) {
        if let Some(placeholder) = self.placeholder.borrow().clone() {
            placeholder.set_text(Some("Media source is unavailable"));
        }
        self.show_placeholder(true);
        let already_reported = self
            .media_player()
            .is_some_and(|p| p.playback_session().error.as_ref() == Some(error));
        if !already_reported {
            self.errors.emit(error);
        }
    }
    fn apply_source_uri(&self) {
        let request = self.next_source_request();
        self.cancel_recovery();
        self.source_waiting_for_context.set(false);
        let player = self.media_player().filter(|_| self.owns_player.get());
        if player.is_none() && self.media_player().is_some() {
            // Selecting a URI detaches this view from a caller-owned player;
            // clear its presentation without changing the caller's source.
            self.set_media_player_core(None, None);
        }
        if let Some(player) = &player {
            player.clear_source();
        }
        if self.source_request.get() != request {
            return;
        }
        self.show_placeholder(true);
        let Some(value) = self.source_uri() else {
            self.set_media_player_core(None, None);
            return;
        };
        let value = value.trim();
        if value.is_empty() {
            self.source_uri_error(&MediaError::new(
                MediaErrorKind::InvalidSource,
                "Media source path or URL is empty",
            ));
            return;
        }
        let source = if value.contains("://")
            || value
                .get(..5)
                .is_some_and(|v| v.eq_ignore_ascii_case("file:"))
        {
            MediaSource::Url(value.to_owned())
        } else {
            MediaSource::File(value.into())
        };
        self.source_waiting_for_context.set(true);
        if !self.is_attached_to_visual_tree() {
            return;
        }
        if let Some(placeholder) = self.placeholder.borrow().clone() {
            placeholder.set_text(Some("Opening media"));
        }
        let weak = self.to_ref().downgrade();
        let factory = self.provider_factory.borrow().clone();
        if factory.is_none() {
            let Some(player) = player else {
                self.source_waiting_for_context.set(false);
                self.source_uri_error(&MediaError::new(
                    MediaErrorKind::Unsupported,
                    "Source needs a platform provider factory or an existing owned player",
                ));
                return;
            };
            self.source_waiting_for_context.set(false);
            let pending = player.set_source(source);
            Dispatcher::ui_thread().invoke_async_task_local(move || async move {
                if let Err(error) = pending.await {
                    if error.kind != MediaErrorKind::Cancelled {
                        if let Some(view) =
                            weak.upgrade().filter(|v| v.source_request.get() == request)
                        {
                            view.source_uri_error(&error);
                        }
                    }
                }
            });
            return;
        }
        self.subscribe_context();
        let Some(visual) = ElementComposition::get_element_visual(&self.to_ref()) else {
            // Attachment may precede composition-visual creation. Retry once
            // on the existing render dispatcher; never start an idle poller.
            Dispatcher::ui_thread().post_local(
                move || {
                    if let Some(view) = weak.upgrade().filter(|v| {
                        v.source_request.get() == request && v.is_attached_to_visual_tree()
                    }) {
                        view.subscribe_context();
                        if ElementComposition::get_element_visual(&view).is_some() {
                            view.apply_source_uri();
                        } else {
                            view.source_uri_error(&MediaError::new(
                                MediaErrorKind::Unsupported,
                                "Source has no composition renderer",
                            ));
                        }
                    }
                },
                DispatcherPriority::RENDER,
            );
            return;
        };
        let compositor = visual.compositor().clone();
        let factory = factory.unwrap();
        Dispatcher::ui_thread().invoke_async_task_local(move || async move {
            let interop = compositor.try_get_composition_gpu_interop().await;
            let Some(view) = weak
                .upgrade()
                .filter(|v| v.source_request.get() == request && v.is_attached_to_visual_tree())
            else {
                return;
            };
            let Some(interop) = interop.filter(|i| !i.is_lost()) else {
                // A later ContextCreated can retry an initially unavailable
                // Android surface. Unsupported profiles still receive an error.
                view.source_uri_error(&MediaError::new(
                    MediaErrorKind::Unsupported,
                    "Renderer has no admitted media texture interop",
                ));
                return;
            };
            let (provider, importer) = match factory(&*interop) {
                Ok(value) => value,
                Err(error) => {
                    view.source_waiting_for_context.set(false);
                    view.source_uri_error(&error);
                    return;
                }
            };
            if view.source_request.get() != request {
                return;
            }
            view.source_waiting_for_context.set(false);
            let pending = view.set_source_core(source, provider, importer);
            drop(view);
            if let Err(error) = pending.await {
                if error.kind != MediaErrorKind::Cancelled {
                    if let Some(view) = weak.upgrade().filter(|v| v.source_request.get() == request)
                    {
                        view.source_uri_error(&error);
                    }
                }
            }
        });
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
        if !self.clear_source_uri_selection() {
            return;
        }
        self.set_media_player_core(player, importer);
    }
    fn set_media_player_core(
        &self,
        player: Option<&MediaPlayer>,
        importer: Option<Rc<dyn TextureFrameImporter>>,
    ) {
        self.stop_polling();
        self.cancel_recovery();
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
        if !self.clear_source_uri_selection() {
            return Box::pin(async { Err(MediaError::cancelled()) });
        }
        self.set_source_core(source, provider, importer)
    }
    fn set_source_core(
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
        self.set_media_player_core(Some(&player), Some(importer));
        self.owns_player.set(true);
        *self.player.borrow_mut() = Some(player.clone());
        player.set_source(source)
    }
    /// Enables automatic recovery for sessions owned by this element. Supplied
    /// players remain caller-owned; their owner calls `rebind_provider` once for
    /// all consumers. Removing the factory cancels pending element recovery.
    pub fn set_provider_factory(&self, factory: Option<Rc<MediaPlayerProviderFactory>>) {
        self.cancel_recovery();
        if let Some(subscription) = self.context_subscription.borrow_mut().take() {
            subscription.dispose();
        }
        *self.provider_factory.borrow_mut() = factory;
        self.subscribe_context();
        if self.source_uri().is_some()
            && (self.source_waiting_for_context.get() || self.media_player().is_none())
        {
            self.apply_source_uri();
        }
    }
    fn cancel_recovery(&self) {
        self.recovery_generation.set(
            self.recovery_generation
                .get()
                .checked_add(1)
                .expect("recovery generation exhausted"),
        );
        self.device_suspended.set(false);
    }
    fn suspend_device(&self) {
        if !self.owns_player.get() || self.device_suspended.replace(true) {
            return;
        }
        self.recovery_generation.set(
            self.recovery_generation
                .get()
                .checked_add(1)
                .expect("recovery generation exhausted"),
        );
        self.stop_polling();
        self.show_placeholder(true);
        if let Some(presenter) = self.presenter.borrow().clone() {
            presenter.invalidate_device();
        }
        if let Some(player) = self.media_player() {
            player.suspend_for_device_loss();
        }
    }
    fn subscribe_context(&self) {
        if self.context_subscription.borrow().is_some()
            || self.provider_factory.borrow().is_none()
            || !self.is_attached_to_visual_tree()
        {
            return;
        }
        let Some(visual) = ElementComposition::get_element_visual(&self.to_ref()) else {
            return;
        };
        let weak = self.to_ref().downgrade();
        *self.context_subscription.borrow_mut() =
            Some(visual.compositor().render_context_changed(move |state| {
                if let Some(view) = weak.upgrade() {
                    match state {
                        RenderContextState::Lost => view.suspend_device(),
                        RenderContextState::Created if view.source_waiting_for_context.get() => {
                            view.apply_source_uri()
                        }
                        RenderContextState::Created if view.device_suspended.get() => {
                            view.recover_device()
                        }
                        _ => {}
                    }
                }
            }));
    }
    fn recover_device(&self) {
        let Some(factory) = self.provider_factory.borrow().clone() else {
            return;
        };
        let Some(player) = self.media_player().filter(|_| self.owns_player.get()) else {
            return;
        };
        let Some(visual) = ElementComposition::get_element_visual(&self.to_ref()) else {
            return;
        };
        let compositor = visual.compositor().clone();
        let generation = self.recovery_generation.get();
        let weak_player = player.downgrade();
        let weak = self.to_ref().downgrade();
        self.device_suspended.set(false);
        Dispatcher::ui_thread().invoke_async_task_local(move || async move {
            let interop = compositor.try_get_composition_gpu_interop().await;
            let Some(view) = weak.upgrade().filter(|v| {
                v.recovery_generation.get() == generation && v.is_attached_to_visual_tree()
            }) else {
                return;
            };
            let Some(player) = weak_player.upgrade() else {
                return;
            };
            let result = match interop {
                Some(interop) if !interop.is_lost() => factory(&*interop),
                _ => Err(MediaError::new(
                    MediaErrorKind::DeviceLost,
                    "Renderer has no restored GPU context",
                )),
            };
            let (provider, importer) = match result {
                Ok(value) => value,
                Err(error) => {
                    view.device_suspended.set(true);
                    view.errors.emit(&error);
                    return;
                }
            };
            if view.recovery_generation.get() != generation {
                return;
            }
            let pending = player.rebind_provider(provider);
            if let Some(presenter) = view.presenter.borrow().clone() {
                presenter.set_frame_source(Some(player.frames()), Some(importer));
            }
            drop(view);
            drop(player);
            if let Err(error) = pending.await {
                if error.kind != MediaErrorKind::Cancelled {
                    if let Some(view) = weak
                        .upgrade()
                        .filter(|v| v.recovery_generation.get() == generation)
                    {
                        view.errors.emit(&error);
                    }
                }
            }
        });
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
        // Ended has the same race: Play is accepted before its native restart
        // changes state, with no pending import yet to keep the timer alive.
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
                ) || (settling > 0
                    && matches!(state, MediaPlaybackState::Paused | MediaPlaybackState::Ended))
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
        state: Cell<MediaPlaybackState>,
    }
    impl NativeMediaSession for Native {
        fn has_pending_frame_work(&self) -> bool {
            self.pending.get()
        }
        fn state(&self) -> Result<MediaPlaybackSession, MediaError> {
            Ok(MediaPlaybackSession {
                state: self.state.get(),
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
    fn ended_restart_keeps_polling_until_native_play_is_observed() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let native = Rc::new(Native {
            pending: Cell::new(false),
            reads: Cell::new(0),
            state: Cell::new(MediaPlaybackState::Ended),
        });
        let player = MediaPlayer::new(Rc::new(Provider(native.clone())));
        ready(player.set_source(MediaSource::Url("https://example.invalid/video.mp4".into())))
            .unwrap();
        let view = MediaPlayerElement::new();
        view.set_media_player(Some(&player), None);
        let window = Window::new();
        window.set_content(Some(Control::boxed(view.clone())));
        window.show();
        for _ in 0..40 {
            tick();
        }
        assert!(
            view.timer.borrow().is_none(),
            "settled Ended must not poll forever"
        );
        ready(player.play()).unwrap();
        // Command acceptance precedes the native state change. No pending GPU
        // job can keep this timer alive for us, and no second Play event occurs.
        for _ in 0..3 {
            tick();
        }
        assert!(
            view.timer.borrow().is_some(),
            "Ended must receive the command settling allowance too"
        );
        let reads = native.reads.get();
        native.state.set(MediaPlaybackState::Playing);
        for _ in 0..40 {
            tick();
        }
        assert!(native.reads.get() > reads && view.timer.borrow().is_some());
        native.state.set(MediaPlaybackState::Ended);
        tick();
        assert!(
            view.timer.borrow().is_none(),
            "settled EOF must still retire polling"
        );
        window.close();
        tick();
    }
    struct Importer;
    impl TextureFrameImporter for Importer {
        fn import(
            &self,
            _: &dyn ICompositionGpuInterop,
            _: &FrameLease,
        ) -> Result<ferroui_media_controls::ImportedTextureFrame, MediaError> {
            unreachable!("source-selection fixture produces no GPU frames")
        }
    }
    fn ready<T>(mut work: MediaFuture<T>) -> Result<T, MediaError> {
        match work
            .as_mut()
            .poll(&mut std::task::Context::from_waker(std::task::Waker::noop()))
        {
            std::task::Poll::Ready(value) => value,
            _ => panic!("expected ready"),
        }
    }
    fn owned_view() -> (Ref<MediaPlayerElement>, MediaPlayer) {
        let view = MediaPlayerElement::new();
        // This fixture checks source selection. The mock renderer has no GPU;
        // native smoke tests independently exercise admission and presentation.
        view.presentation_subscription.borrow_mut().take();
        let native = Rc::new(Native {
            pending: Cell::new(false),
            reads: Cell::new(0),
            state: Cell::new(MediaPlaybackState::Paused),
        });
        ready(view.set_source(
            MediaSource::File("initial.mp4".into()),
            Rc::new(Provider(native)),
            Rc::new(Importer),
        ))
        .unwrap();
        let player = view.media_player().unwrap();
        (view, player)
    }
    #[test]
    fn bound_uri_replaces_owned_source_latest_wins_and_none_clears_immediately() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let (view, player) = owned_view();
        view.set_source_uri(Some("second.mp4".into()));
        assert!(
            player.source().is_none(),
            "selection invalidates the previous source before attachment"
        );
        let window = Window::new();
        window.set_content(Some(Control::boxed(view.clone())));
        window.show();
        tick();
        assert!(
            matches!(player.source(), Some(MediaSource::File(path)) if path == std::path::Path::new("second.mp4"))
        );
        assert!(!player.options().autoplay);
        view.set_source_uri(Some("https://example.invalid/third.mp4".into()));
        view.set_source_uri(Some("fourth.mp4".into()));
        tick();
        assert!(
            matches!(player.source(), Some(MediaSource::File(path)) if path == std::path::Path::new("fourth.mp4"))
        );
        view.set_source_uri(None);
        assert!(player.source().is_none());
        assert!(view.media_player().is_none());
        window.close();
        tick();
    }
    #[test]
    fn explicit_shared_player_cancels_pending_uri_without_stopping_the_caller() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let (view, old) = owned_view();
        let external = MediaPlayer::new(Rc::new(Provider(Rc::new(Native {
            pending: Cell::new(false),
            reads: Cell::new(0),
            state: Cell::new(MediaPlaybackState::Paused),
        }))));
        ready(external.set_source(MediaSource::File("external.mp4".into()))).unwrap();
        let window = Window::new();
        window.set_content(Some(Control::boxed(view.clone())));
        window.show();
        tick();
        view.set_source_uri(Some("superseded.mp4".into()));
        view.set_media_player(Some(&external), None);
        tick();
        assert!(view.source_uri().is_none());
        assert!(view.media_player().unwrap().same_player(&external));
        assert!(old.source().is_none());
        window.close();
        tick();
        assert!(
            matches!(external.source(), Some(MediaSource::File(path)) if path == std::path::Path::new("external.mp4"))
        );
    }
    #[test]
    fn new_uri_supersedes_suspended_device_recovery() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let (view, player) = owned_view();
        view.suspend_device();
        assert!(view.device_suspended.get());
        let previous = view.recovery_generation.get();
        view.set_source_uri(Some("replacement.mp4".into()));
        assert!(!view.device_suspended.get());
        assert!(view.recovery_generation.get() > previous);
        assert!(player.source().is_none());
        assert!(view.source_waiting_for_context.get());
        let window = Window::new();
        window.set_content(Some(Control::boxed(view.clone())));
        window.show();
        tick();
        assert!(
            matches!(player.source(), Some(MediaSource::File(path)) if path == std::path::Path::new("replacement.mp4"))
        );
        window.close();
        tick();
    }
    #[test]
    fn configuring_recovery_factory_does_not_reopen_a_prepared_uri() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let (view, player) = owned_view();
        let window = Window::new();
        window.set_content(Some(Control::boxed(view.clone())));
        window.show();
        tick();
        view.set_source_uri(Some("selected.mp4".into()));
        tick();
        let calls = Rc::new(Cell::new(0));
        let seen = calls.clone();
        view.set_provider_factory(Some(Rc::new(move |_| {
            seen.set(seen.get() + 1);
            Err(MediaError::new(
                MediaErrorKind::Unsupported,
                "No GPU fixture",
            ))
        })));
        tick();
        assert_eq!(calls.get(), 0);
        assert!(view.media_player().unwrap().same_player(&player));
        assert!(
            matches!(player.source(), Some(MediaSource::File(path)) if path == std::path::Path::new("selected.mp4"))
        );
        window.close();
        tick();
    }
    #[test]
    fn uri_selection_releases_a_shared_view_without_clearing_the_external_source() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let view = MediaPlayerElement::new();
        let external = MediaPlayer::new(Rc::new(Provider(Rc::new(Native {
            pending: Cell::new(false),
            reads: Cell::new(0),
            state: Cell::new(MediaPlaybackState::Paused),
        }))));
        ready(external.set_source(MediaSource::File("caller.mp4".into()))).unwrap();
        view.set_media_player(Some(&external), None);
        view.set_source_uri(Some("deferred.mp4".into()));
        assert!(view.media_player().is_none());
        assert!(
            matches!(external.source(), Some(MediaSource::File(path)) if path == std::path::Path::new("caller.mp4"))
        );
    }
    #[test]
    fn empty_uri_clears_previous_source_and_reports_invalid_source() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let (view, player) = owned_view();
        let errors = Rc::new(RefCell::new(Vec::new()));
        let seen = errors.clone();
        let _subscription = view
            .errors()
            .subscribe(move |e| seen.borrow_mut().push(e.kind));
        view.set_source_uri(Some("  ".into()));
        assert!(player.source().is_none());
        assert_eq!(&*errors.borrow(), &[MediaErrorKind::InvalidSource]);
    }
    #[test]
    fn unattached_uri_does_not_invoke_a_provider_factory() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let view = MediaPlayerElement::new();
        let calls = Rc::new(Cell::new(0));
        let seen = calls.clone();
        view.set_provider_factory(Some(Rc::new(move |_| {
            seen.set(seen.get() + 1);
            Err(MediaError::new(
                MediaErrorKind::Unsupported,
                "No GPU fixture",
            ))
        })));
        view.set_source_uri(Some("deferred.mp4".into()));
        tick();
        assert_eq!(calls.get(), 0);
        assert!(view.media_player().is_none());
    }
    #[test]
    fn paused_producer_outlives_settling_interval_then_sleeps() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let native = Rc::new(Native {
            pending: Cell::new(true),
            reads: Cell::new(0),
            state: Cell::new(MediaPlaybackState::Paused),
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

impl Drop for MediaPlayerElement {
    fn drop(&mut self) {
        if let Some(subscription) = self.context_subscription.get_mut().take() {
            subscription.dispose();
        }
        if let Some(timer) = self.timer.get_mut().take() {
            timer.dispose();
        }
    }
}

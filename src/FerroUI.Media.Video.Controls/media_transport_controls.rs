use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes,
    input::InputElementImpl,
    instantiate,
    interactivity::InteractiveImpl,
    layout::{LayoutableImpl, Orientation},
    threading::Dispatcher,
    FerroObjectImpl, FerroObjectImplExt, Ref, StyledElementImpl, VisualImpl, VisualImplExt,
    VisualTreeAttachmentEventArgs,
};
use ferroui_controls::{
    automation::AutomationProperties, Button, ControlImpl, PanelImpl, Slider, StackPanel,
    StackPanelImpl, TextBlock,
};
use ferroui_media_video::{
    MediaFuture, MediaPlaybackState, MediaPlayer, MediaSubscription, WeakMediaPlayer,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};
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
    updating: Cell<bool>,
}
ferro_class!(MediaTransportControls:StackPanel);
ferro_class_info!(MediaTransportControls {
    new: MediaTransportControls::new
});
ferro_impl_classes!(MediaTransportControls:StyledElementImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,PanelImpl,StackPanelImpl);
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
        this.set_orientation(Orientation::Horizontal);
        this.set_spacing(8.);
        let button = |label: &str| {
            let b = Button::new();
            b.set_content(Some(Rc::new(label.to_owned())));
            AutomationProperties::set_name(&b, Some(label));
            b
        };
        let play = button("Play");
        let stop = button("Stop");
        let mute = button("Mute");
        let seek = Slider::new();
        seek.set_width(200.);
        seek.set_minimum(0.);
        seek.set_maximum(1.);
        AutomationProperties::set_name(&seek, Some("Playback position".into()));
        let volume = Slider::new();
        volume.set_width(80.);
        volume.set_minimum(0.);
        volume.set_maximum(1.);
        volume.set_range_value(1.);
        AutomationProperties::set_name(&volume, Some("Volume".into()));
        let status = TextBlock::new();
        status.set_text(Some("No media"));
        for child in [
            play.clone().upcast::<ferroui_controls::Control>(),
            stop.clone().upcast::<ferroui_controls::Control>(),
            seek.clone().upcast::<ferroui_controls::Control>(),
            volume.clone().upcast::<ferroui_controls::Control>(),
            mute.clone().upcast::<ferroui_controls::Control>(),
            status.clone().upcast::<ferroui_controls::Control>(),
        ] {
            this.children().add(child);
        }
        let weak = this.to_ref().downgrade();
        play.click(move |_, _| {
            if let Some(view) = weak.upgrade() {
                let player = view.player.borrow().as_ref().and_then(|p| p.upgrade());
                if let Some(player) = player {
                    if player.playback_session().state == MediaPlaybackState::Playing {
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
            updating: Cell::new(false),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn set_media_player(&self, player: Option<&MediaPlayer>) {
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
        let weak = self.to_ref().downgrade();
        Dispatcher::ui_thread().invoke_async_task_local(move || async move {
            if let Err(error) = task.await {
                if let Some(view) = weak.upgrade() {
                    view.show_error(&error.to_string());
                }
            }
        });
    }
    fn show_error(&self, error: &str) {
        if let Some(status) = self.status.borrow().as_ref() {
            status.set_text(Some(error));
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
            let label = if state.state == MediaPlaybackState::Playing {
                "Pause"
            } else {
                "Play"
            };
            play.set_content(Some(Rc::new(label.to_owned())));
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
            seek.set_range_value(state.position.as_secs_f64());
        }
        if let Some(player) = player {
            let options = player.options();
            if let Some(volume) = self.volume.borrow().as_ref() {
                volume.set_range_value(options.volume);
            }
            if let Some(mute) = self.mute.borrow().as_ref() {
                let label = if options.muted { "Unmute" } else { "Mute" };
                mute.set_content(Some(Rc::new(label.to_owned())));
                AutomationProperties::set_name(mute, Some(label.into()));
            }
        }
        if let Some(status) = self.status.borrow().as_ref() {
            let text = if let Some(error) = state.error {
                error.to_string()
            } else {
                format!("{:?} · {:.1}s", state.state, state.position.as_secs_f64())
            };
            status.set_text(Some(&text));
        }
        self.updating.set(false);
    }
}

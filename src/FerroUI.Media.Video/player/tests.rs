use super::*;
use std::{
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll, Wake, Waker},
};
struct Noop;
impl Wake for Noop {
    fn wake(self: Arc<Self>) {}
}
fn poll<T>(future: &mut Pin<Box<dyn Future<Output = T>>>) -> Poll<T> {
    future
        .as_mut()
        .poll(&mut Context::from_waker(&Waker::from(Arc::new(Noop))))
}
fn ready<T>(mut future: Pin<Box<dyn Future<Output = T>>>) -> T {
    match poll(&mut future) {
        Poll::Ready(value) => value,
        Poll::Pending => panic!("expected ready"),
    }
}
#[derive(Default)]
struct Native {
    plays: Cell<usize>,
    pauses: Cell<usize>,
    seeks: RefCell<Vec<Rc<RefCell<Option<Duration>>>>>,
    targets: RefCell<Vec<Duration>>,
    non_seekable: Cell<bool>,
    play_error: RefCell<Option<MediaError>>,
    frame_reads: Cell<usize>,
    drops: Rc<Cell<usize>>,
}
impl Drop for Native {
    fn drop(&mut self) {
        self.drops.set(self.drops.get() + 1);
    }
}
impl NativeMediaSession for Native {
    fn state(&self) -> Result<MediaPlaybackSession, MediaError> {
        Ok(MediaPlaybackSession {
            state: MediaPlaybackState::Paused,
            duration: Some(Duration::from_secs(20)),
            seekable: !self.non_seekable.get(),
            ..Default::default()
        })
    }
    fn play(&self) -> MediaFuture<()> {
        self.plays.set(self.plays.get() + 1);
        let error = self.play_error.borrow_mut().take();
        Box::pin(async move { error.map_or(Ok(()), Err) })
    }
    fn pause(&self) -> Result<(), MediaError> {
        self.pauses.set(self.pauses.get() + 1);
        Ok(())
    }
    fn seek(&self, target: Duration) -> MediaFuture<Duration> {
        self.targets.borrow_mut().push(target);
        let pending = Rc::new(RefCell::new(None));
        self.seeks.borrow_mut().push(pending.clone());
        Box::pin(std::future::poll_fn(move |_| {
            match pending.borrow_mut().take() {
                Some(value) => Poll::Ready(Ok(value)),
                None => Poll::Pending,
            }
        }))
    }
    fn set_audio(&self, _: f64, _: bool) -> Result<(), MediaError> {
        Ok(())
    }
    fn take_frame(&self) -> Result<Option<FrameLease>, MediaError> {
        self.frame_reads.set(self.frame_reads.get() + 1);
        Ok(None)
    }
}
struct Provider {
    native: Rc<Native>,
    opens: Cell<usize>,
}
impl MediaPlayerProvider for Provider {
    fn open(
        &self,
        _: MediaSource,
        _: u64,
        _: u64,
        _: PresentationPolicy,
    ) -> MediaFuture<Rc<dyn NativeMediaSession>> {
        self.opens.set(self.opens.get() + 1);
        let native: Rc<dyn NativeMediaSession> = self.native.clone();
        Box::pin(async move { Ok(native) })
    }
}
fn player() -> (MediaPlayer, Rc<Native>, Rc<Provider>) {
    let native = Rc::new(Native::default());
    let provider = Rc::new(Provider {
        native: native.clone(),
        opens: Cell::new(0),
    });
    (MediaPlayer::new(provider.clone()), native, provider)
}
fn source() -> MediaSource {
    MediaSource::Url("https://example.invalid/fixture.mp4".into())
}
#[test]
fn defaults_and_shared_ownership_do_not_duplicate_decode() {
    let (player, native, provider) = player();
    assert!(!player.options().autoplay);
    assert!(!player.options().looping);
    ready(player.set_source(source())).unwrap();
    let view = player.clone();
    drop(view);
    assert_eq!(provider.opens.get(), 1);
    assert_eq!(native.plays.get(), 0);
    ready(player.play()).unwrap();
    assert_eq!(native.plays.get(), 1);
}
#[test]
fn dropping_last_player_notifies_weak_views_to_clear_paused_presentation() {
    let (player, _, _) = player();
    ready(player.set_source(source())).unwrap();
    let closed = Rc::new(Cell::new(0));
    let seen = closed.clone();
    let weak = player.downgrade();
    let _subscription=player.events().subscribe(move|event|{
        if matches!(event,MediaPlayerEvent::StateChanged(state) if state.state==MediaPlaybackState::Closed){
            assert!(weak.upgrade().is_none());seen.set(seen.get()+1);
        }
    });
    let other = player.clone();
    drop(other);
    assert_eq!(closed.get(), 0);
    drop(player);
    assert_eq!(closed.get(), 1);
}
#[test]
fn synchronous_opened_callback_can_pause_autoplay_before_play_is_applied() {
    let (player, native, _) = player();
    player
        .set_options(MediaPlayerOptions {
            autoplay: true,
            ..Default::default()
        })
        .unwrap();
    let weak = Rc::downgrade(&player.0);
    let _subscription = player.events().subscribe(move |event| {
        if matches!(event, MediaPlayerEvent::Opened) {
            MediaPlayer(weak.upgrade().unwrap()).pause().unwrap();
        }
    });
    ready(player.set_source(source())).unwrap();
    assert_eq!(native.plays.get(), 0);
    assert_eq!(player.requested_state(), PlaybackIntent::Pause);
}
#[test]
fn opened_callback_replacement_prevents_stale_play() {
    let (player, native, _) = player();
    let weak = Rc::downgrade(&player.0);
    let _subscription = player.events().subscribe(move |event| {
        if matches!(event, MediaPlayerEvent::Opened) {
            MediaPlayer(weak.upgrade().unwrap()).clear_source();
        }
    });
    assert_eq!(
        ready(player.set_source(source())).unwrap_err().kind,
        MediaErrorKind::Cancelled
    );
    assert_eq!(native.plays.get(), 0);
    assert_eq!(player.playback_session().state, MediaPlaybackState::Closed);
}
#[test]
fn newest_seek_wins_when_native_completions_arrive_in_reverse_order() {
    let (player, native, _) = player();
    ready(player.set_source(source())).unwrap();
    let mut first = player.seek(Duration::from_secs(3));
    assert!(poll(&mut first).is_pending());
    let mut second = player.seek(Duration::from_secs(7));
    assert!(poll(&mut second).is_pending());
    *native.seeks.borrow()[1].borrow_mut() = Some(Duration::from_secs(7));
    assert_eq!(ready(second).unwrap(), Duration::from_secs(7));
    *native.seeks.borrow()[0].borrow_mut() = Some(Duration::from_secs(3));
    assert_eq!(ready(first).unwrap_err().kind, MediaErrorKind::Cancelled);
    assert_eq!(player.playback_session().position, Duration::from_secs(7));
}
#[test]
fn pending_open_does_not_keep_player_alive() {
    let (player, _, _) = player();
    let weak = Rc::downgrade(&player.0);
    let pending = player.set_source(source());
    drop(player);
    assert!(weak.upgrade().is_none());
    assert_eq!(ready(pending).unwrap_err().kind, MediaErrorKind::Cancelled);
}
#[test]
fn invalid_audio_never_reaches_native() {
    let (player, _, _) = player();
    assert!(player
        .set_options(MediaPlayerOptions {
            volume: f64::NAN,
            ..Default::default()
        })
        .is_err());
    assert_eq!(player.options().volume, 1.);
}

struct Provisional(Rc<Cell<usize>>);
impl Drop for Provisional {
    fn drop(&mut self) {
        self.0.set(self.0.get() + 1);
    }
}
struct PendingProvider(Rc<Cell<usize>>);
impl MediaPlayerProvider for PendingProvider {
    fn open(
        &self,
        _: MediaSource,
        _: u64,
        _: u64,
        _: PresentationPolicy,
    ) -> MediaFuture<Rc<dyn NativeMediaSession>> {
        let owner = Provisional(self.0.clone());
        Box::pin(async move {
            let _owner = owner;
            std::future::pending().await
        })
    }
}
#[test]
fn replacing_an_unpolled_open_releases_provisional_native_ownership_immediately() {
    let drops = Rc::new(Cell::new(0));
    let player = MediaPlayer::new(Rc::new(PendingProvider(drops.clone())));
    let pending = player.set_source(source());
    assert_eq!(drops.get(), 0);
    player.clear_source();
    assert_eq!(drops.get(), 1);
    assert_eq!(ready(pending).unwrap_err().kind, MediaErrorKind::Cancelled);
}
#[test]
fn dropping_preparation_before_commit_clears_loading_and_native_ownership() {
    let drops = Rc::new(Cell::new(0));
    let player = MediaPlayer::new(Rc::new(PendingProvider(drops.clone())));
    let pending = player.set_source(source());
    drop(pending);
    assert_eq!(drops.get(), 1);
    assert_eq!(player.playback_session().state, MediaPlaybackState::Closed);
}
#[test]
fn pause_preserves_an_existing_seek_and_does_not_force_resume_on_completion() {
    let (player, native, _) = player();
    ready(player.set_source(source())).unwrap();
    let mut seek = player.seek(Duration::from_secs(3));
    assert!(poll(&mut seek).is_pending());
    player.pause().unwrap();
    *native.seeks.borrow()[0].borrow_mut() = Some(Duration::from_secs(3));
    assert_eq!(ready(seek).unwrap(), Duration::from_secs(3));
    assert_eq!(player.requested_state(), PlaybackIntent::Pause);
    assert_eq!(native.plays.get(), 0);
}

#[test]
fn preparation_applies_only_the_latest_seek_before_autoplay() {
    let (player, native, _) = player();
    player
        .set_options(MediaPlayerOptions {
            autoplay: true,
            ..Default::default()
        })
        .unwrap();
    let mut opening = player.set_source(source());
    let mut first = player.seek(Duration::from_secs(3));
    assert!(poll(&mut first).is_pending());
    let mut newest = player.seek(Duration::from_secs(7));
    assert!(poll(&mut newest).is_pending());
    assert_eq!(ready(first).unwrap_err().kind, MediaErrorKind::Cancelled);
    assert!(poll(&mut opening).is_pending());
    assert_eq!(&*native.targets.borrow(), &[Duration::from_secs(7)]);
    assert_eq!(native.plays.get(), 0);
    *native.seeks.borrow()[0].borrow_mut() = Some(Duration::from_secs(7));
    ready(opening).unwrap();
    assert_eq!(ready(newest).unwrap(), Duration::from_secs(7));
    assert_eq!(native.plays.get(), 1);
}

#[test]
fn stop_during_preparation_overrides_autoplay_and_seeks_to_zero() {
    let (player, native, _) = player();
    player
        .set_options(MediaPlayerOptions {
            autoplay: true,
            ..Default::default()
        })
        .unwrap();
    let mut opening = player.set_source(source());
    let mut stopped = player.stop();
    assert!(poll(&mut stopped).is_pending());
    assert!(poll(&mut opening).is_pending());
    assert_eq!(&*native.targets.borrow(), &[Duration::ZERO]);
    *native.seeks.borrow()[0].borrow_mut() = Some(Duration::ZERO);
    ready(opening).unwrap();
    assert_eq!(ready(stopped).unwrap(), Duration::ZERO);
    assert_eq!(player.requested_state(), PlaybackIntent::Stop);
    assert_eq!(native.plays.get(), 0);
}

#[test]
fn stopping_a_nonseekable_source_succeeds_without_a_seek() {
    let (player, native, _) = player();
    native.non_seekable.set(true);
    ready(player.set_source(source())).unwrap();
    ready(player.stop()).unwrap();
    assert_eq!(native.pauses.get(), 1);
    assert!(native.targets.borrow().is_empty());
    assert!(player.frames().latest().is_none());
}

#[test]
fn abandoning_preparation_during_a_seek_cancels_its_receipt_and_native_owner() {
    let (player, _, _) = player();
    let mut opening = player.set_source(source());
    let mut seeking = player.seek(Duration::from_secs(3));
    assert!(poll(&mut seeking).is_pending());
    assert!(poll(&mut opening).is_pending());
    drop(opening);
    assert_eq!(ready(seeking).unwrap_err().kind, MediaErrorKind::Cancelled);
    assert_eq!(player.playback_session().state, MediaPlaybackState::Closed);
    assert!(player.0.native.borrow().is_none());
}

#[test]
fn autoplay_denial_keeps_a_prepared_session_available_for_explicit_play() {
    let (player, native, _) = player();
    player
        .set_options(MediaPlayerOptions {
            autoplay: true,
            ..Default::default()
        })
        .unwrap();
    *native.play_error.borrow_mut() = Some(MediaError::new(
        MediaErrorKind::AutoplayDenied,
        "gesture needed",
    ));
    assert_eq!(
        ready(player.set_source(source())).unwrap_err().kind,
        MediaErrorKind::AutoplayDenied
    );
    assert_eq!(player.playback_session().state, MediaPlaybackState::Paused);
    assert!(player.0.native.borrow().is_some());
    ready(player.play()).unwrap();
    assert_eq!(native.plays.get(), 2);
}

#[test]
fn a_new_seek_during_native_preparation_restarts_only_the_seek_not_the_source() {
    let (player, native, provider) = player();
    let mut opening = player.set_source(source());
    let first = player.seek(Duration::from_secs(2));
    assert!(poll(&mut opening).is_pending());
    let newest = player.seek(Duration::from_secs(6));
    assert!(poll(&mut opening).is_pending());
    assert_eq!(
        &*native.targets.borrow(),
        &[Duration::from_secs(2), Duration::from_secs(6)]
    );
    *native.seeks.borrow()[1].borrow_mut() = Some(Duration::from_secs(6));
    ready(opening).unwrap();
    assert_eq!(ready(first).unwrap_err().kind, MediaErrorKind::Cancelled);
    assert_eq!(ready(newest).unwrap(), Duration::from_secs(6));
    assert_eq!(provider.opens.get(), 1);
}

#[test]
fn stop_notification_can_issue_a_newer_seek_without_being_overwritten() {
    let (player, native, _) = player();
    ready(player.set_source(source())).unwrap();
    let newer = Rc::new(RefCell::new(None));
    let saved = newer.clone();
    let weak = player.downgrade();
    let _subscription = player.events().subscribe(move |event| {
        if matches!(event, MediaPlayerEvent::IntentChanged(PlaybackIntent::Stop)) {
            *saved.borrow_mut() = Some(weak.upgrade().unwrap().seek(Duration::from_secs(5)));
        }
    });
    assert_eq!(
        ready(player.stop()).unwrap_err().kind,
        MediaErrorKind::Cancelled
    );
    let mut seeking = newer.borrow_mut().take().unwrap();
    assert!(poll(&mut seeking).is_pending());
    assert_eq!(&*native.targets.borrow(), &[Duration::from_secs(5)]);
    *native.seeks.borrow()[0].borrow_mut() = Some(Duration::from_secs(5));
    assert_eq!(ready(seeking).unwrap(), Duration::from_secs(5));
}

#[test]
fn a_failed_open_preserves_failure_details_after_preparation_cleanup() {
    struct Failed;
    impl MediaPlayerProvider for Failed {
        fn open(
            &self,
            _: MediaSource,
            _: u64,
            _: u64,
            _: PresentationPolicy,
        ) -> MediaFuture<Rc<dyn NativeMediaSession>> {
            Box::pin(async {
                Err(MediaError::new(
                    MediaErrorKind::InvalidSource,
                    "bad fixture source",
                ))
            })
        }
    }
    let player = MediaPlayer::new(Rc::new(Failed));
    assert_eq!(
        ready(player.set_source(source())).unwrap_err().kind,
        MediaErrorKind::InvalidSource
    );
    assert_eq!(player.playback_session().state, MediaPlaybackState::Failed);
    assert_eq!(
        player.playback_session().error.unwrap().message,
        "bad fixture source"
    );
}

#[test]
fn a_preparation_time_play_receipt_is_not_superseded_by_its_own_application() {
    let (player, native, _) = player();
    let opening = player.set_source(source());
    let play = player.play();
    ready(opening).unwrap();
    ready(play).unwrap();
    assert_eq!(native.plays.get(), 1);
}

#[test]
fn stop_does_not_republish_the_old_native_position_while_reset_is_pending() {
    let (player, native, _) = player();
    ready(player.set_source(source())).unwrap();
    let mut stopped = player.stop();
    assert!(poll(&mut stopped).is_pending());
    player.poll().unwrap();
    assert_eq!(native.frame_reads.get(), 0);
    assert!(player.frames().latest().is_none());
    *native.seeks.borrow()[0].borrow_mut() = Some(Duration::ZERO);
    ready(stopped).unwrap();
    player.poll().unwrap();
    assert_eq!(native.frame_reads.get(), 1);
}

use crate::frames::{Acceleration, FrameLease, FrameMailbox, PresentationPolicy};
use crate::operation::{OperationScope, OperationSet};
use crate::{MediaError, MediaErrorKind, MediaEvent, MediaFuture};
use std::{
    cell::{Cell, RefCell},
    path::PathBuf,
    rc::Rc,
    sync::Arc,
    time::Duration,
};

#[derive(Clone, Debug)]
pub enum MediaSource {
    File(PathBuf),
    Url(String),
    Resource {
        name: String,
        content_type: String,
        encoded: Arc<[u8]>,
    },
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PlaybackIntent {
    Play,
    #[default]
    Pause,
    Stop,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MediaPlaybackState {
    #[default]
    Closed,
    Opening,
    Paused,
    Playing,
    Buffering,
    Ended,
    Failed,
}
#[derive(Clone, Debug)]
pub struct MediaPlaybackSession {
    pub state: MediaPlaybackState,
    pub position: Duration,
    pub duration: Option<Duration>,
    pub natural_size: Option<(u32, u32)>,
    pub seekable: bool,
    pub decode: Acceleration,
    pub error: Option<MediaError>,
}
impl Default for MediaPlaybackSession {
    fn default() -> Self {
        Self {
            state: MediaPlaybackState::Closed,
            position: Duration::ZERO,
            duration: None,
            natural_size: None,
            seekable: false,
            decode: Acceleration::Unknown,
            error: None,
        }
    }
}
#[derive(Clone, Debug)]
pub enum MediaPlayerEvent {
    IntentChanged(PlaybackIntent),
    Opened,
    StateChanged(MediaPlaybackSession),
    SeekCompleted(Duration),
    Ended,
}
#[derive(Clone, Debug)]
pub struct MediaPlayerOptions {
    pub autoplay: bool,
    pub looping: bool,
    pub volume: f64,
    pub muted: bool,
    pub presentation: PresentationPolicy,
}
impl Default for MediaPlayerOptions {
    fn default() -> Self {
        Self {
            autoplay: false,
            looping: false,
            volume: 1.,
            muted: false,
            presentation: PresentationPolicy::GpuPreferred,
        }
    }
}
/// Narrow platform boundary. `poll` reports native facts, not requested intent.
/// Implementations must release native resources on Drop and keep returned frame
/// allocations immutable until the last lease/GPU completion releases them.
pub trait NativeMediaSession {
    /// A producer/import is still pending, including a ready native image
    /// waiting for bounded GPU storage. Presentation must keep polling even
    /// while native playback is paused or ended; submission is not readiness.
    fn has_pending_frame_work(&self) -> bool {
        false
    }
    fn state(&self) -> Result<MediaPlaybackSession, MediaError>;
    fn play(&self) -> MediaFuture<()>;
    fn pause(&self) -> Result<(), MediaError>;
    /// Retires pending preview output while preserving source ownership. The
    /// player separately resets seekable media and owns frame-zero publication.
    fn stop(&self) -> Result<(), MediaError> {
        self.pause()
    }
    fn seek(&self, position: Duration) -> MediaFuture<Duration>;
    fn set_audio(&self, volume: f64, muted: bool) -> Result<(), MediaError>;
    fn take_frame(&self) -> Result<Option<FrameLease>, MediaError>;
}
pub trait MediaPlayerProvider {
    fn open(
        &self,
        source: MediaSource,
        source_id: u64,
        generation: u64,
        policy: PresentationPolicy,
    ) -> MediaFuture<Rc<dyn NativeMediaSession>>;
}
struct PlayerInner {
    provider: Rc<dyn MediaPlayerProvider>,
    source_id: u64,
    generation: Cell<u64>,
    command: Cell<u64>,
    seek_command: Cell<u64>,
    preparing: Cell<bool>,
    pending_seek: RefCell<Option<Rc<SeekRequest>>>,
    operations: OperationSet,
    intent: Cell<PlaybackIntent>,
    options: RefCell<MediaPlayerOptions>,
    source: RefCell<Option<MediaSource>>,
    native: RefCell<Option<Rc<dyn NativeMediaSession>>>,
    state: RefCell<MediaPlaybackSession>,
    frames: Arc<FrameMailbox>,
    events: MediaEvent<MediaPlayerEvent>,
    ended: Cell<bool>,
}
struct SeekRequest {
    command: u64,
    position: Duration,
    completed: Cell<bool>,
    result: RefCell<Option<Result<Duration, MediaError>>>,
    waker: RefCell<Option<std::task::Waker>>,
}
impl SeekRequest {
    fn finish(&self, result: Result<Duration, MediaError>) {
        if self.completed.replace(true) {
            return;
        }
        *self.result.borrow_mut() = Some(result);
        let waker = self.waker.borrow_mut().take();
        if let Some(waker) = waker {
            waker.wake();
        }
    }
}
/// Caller-owned playback can feed multiple views/texture consumers. Cloning a
/// player never opens another decoder. Futures hold weak player ownership.
#[derive(Clone)]
pub struct MediaPlayer(Rc<PlayerInner>);
#[derive(Clone)]
pub struct WeakMediaPlayer(std::rc::Weak<PlayerInner>);
impl WeakMediaPlayer {
    pub fn upgrade(&self) -> Option<MediaPlayer> {
        self.0.upgrade().map(MediaPlayer)
    }
}
impl Drop for PlayerInner {
    fn drop(&mut self) {
        self.frames.clear();
        if let Some(request) = self.pending_seek.get_mut().take() {
            request.finish(Err(MediaError::cancelled()));
        }
        // Caller-owned views hold weak player references. Notify them while
        // subscriptions still exist so a paused view can clear its presentation
        // without a perpetual polling timer or keeping the player alive.
        if self.state.get_mut().state != MediaPlaybackState::Closed {
            self.events.emit(&MediaPlayerEvent::StateChanged(
                MediaPlaybackSession::default(),
            ));
        }
    }
}
struct SourcePreparationGuard {
    player: std::rc::Weak<PlayerInner>,
    generation: u64,
    committed: bool,
}
impl Drop for SourcePreparationGuard {
    fn drop(&mut self) {
        if self.committed {
            return;
        }
        let Some(inner) = self.player.upgrade().filter(|inner| {
            inner.generation.get() == self.generation
                && inner.state.borrow().state != MediaPlaybackState::Failed
        }) else {
            return;
        };
        MediaPlayer(inner).clear_source();
    }
}
impl MediaPlayer {
    pub fn new(provider: Rc<dyn MediaPlayerProvider>) -> Self {
        let id = ferroui_media::allocate_source_id();
        Self(Rc::new(PlayerInner {
            provider,
            source_id: id,
            generation: Cell::new(1),
            command: Cell::new(0),
            seek_command: Cell::new(0),
            preparing: Cell::new(false),
            pending_seek: RefCell::new(None),
            operations: OperationSet::default(),
            intent: Cell::new(PlaybackIntent::Pause),
            options: RefCell::new(MediaPlayerOptions::default()),
            source: RefCell::new(None),
            native: RefCell::new(None),
            state: RefCell::new(MediaPlaybackSession::default()),
            frames: Arc::new(FrameMailbox::new(id, 1)),
            events: MediaEvent::default(),
            ended: Cell::new(false),
        }))
    }
    pub fn same_player(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
    pub fn downgrade(&self) -> WeakMediaPlayer {
        WeakMediaPlayer(Rc::downgrade(&self.0))
    }
    pub fn events(&self) -> &MediaEvent<MediaPlayerEvent> {
        &self.0.events
    }
    pub fn playback_session(&self) -> MediaPlaybackSession {
        self.0.state.borrow().clone()
    }
    pub fn options(&self) -> MediaPlayerOptions {
        self.0.options.borrow().clone()
    }
    pub fn frames(&self) -> Arc<FrameMailbox> {
        self.0.frames.clone()
    }
    pub fn has_pending_frame_work(&self) -> bool {
        self.0
            .native
            .borrow()
            .as_ref()
            .is_some_and(|native| native.has_pending_frame_work())
    }
    pub fn requested_state(&self) -> PlaybackIntent {
        self.0.intent.get()
    }
    pub fn set_options(&self, options: MediaPlayerOptions) -> Result<(), MediaError> {
        if !options.volume.is_finite() || !(0.0..=1.0).contains(&options.volume) {
            return Err(MediaError::new(
                MediaErrorKind::InvalidSource,
                "Volume must be finite and between zero and one",
            ));
        }
        let native = self.0.native.borrow().clone();
        if let Some(native) = native {
            native.set_audio(options.volume, options.muted)?;
        }
        *self.0.options.borrow_mut() = options;
        Ok(())
    }
    fn next_command(&self) -> u64 {
        let next = self
            .0
            .command
            .get()
            .checked_add(1)
            .expect("command identity exhausted");
        self.0.command.set(next);
        self.0.operations.cancel(Some(OperationScope::Transport));
        next
    }
    fn guard<T: 'static>(
        &self,
        pending: MediaFuture<T>,
        scope: OperationScope,
        generation: u64,
        command: u64,
    ) -> MediaFuture<T> {
        let weak = Rc::downgrade(&self.0);
        self.0.operations.register(pending, scope, move || {
            weak.upgrade().is_some_and(|inner| {
                inner.generation.get() == generation
                    && match scope {
                        OperationScope::Source => true,
                        OperationScope::Transport => inner.command.get() == command,
                        OperationScope::Seek => inner.seek_command.get() == command,
                    }
            })
        })
    }
    fn next_seek(&self) -> u64 {
        let next = self
            .0
            .seek_command
            .get()
            .checked_add(1)
            .expect("seek identity exhausted");
        self.0.seek_command.set(next);
        self.0.operations.cancel(Some(OperationScope::Seek));
        let pending = self.0.pending_seek.borrow_mut().take();
        if let Some(request) = pending {
            request.finish(Err(MediaError::cancelled()));
        }
        next
    }
    fn replace(&self) -> u64 {
        let generation = self
            .0
            .generation
            .get()
            .checked_add(1)
            .expect("source generation exhausted");
        self.0.generation.set(generation);
        self.0.preparing.set(false);
        self.next_seek();
        self.0
            .frames
            .replace_generation(generation)
            .expect("increasing generation");
        let old = self.0.native.borrow_mut().take();
        drop(old);
        self.next_command();
        self.0.operations.cancel(None);
        self.0.ended.set(false);
        generation
    }
    /// Replacement invalidates callbacks/frames immediately, even if the future
    /// is never polled. Dropping pending creation retires its provisional owner.
    pub fn set_source(&self, source: MediaSource) -> MediaFuture<()> {
        let generation = self.replace();
        self.0.preparing.set(true);
        *self.0.source.borrow_mut() = Some(source.clone());
        self.0.intent.set(if self.options().autoplay {
            PlaybackIntent::Play
        } else {
            PlaybackIntent::Pause
        });
        let opening = MediaPlaybackSession {
            state: MediaPlaybackState::Opening,
            ..Default::default()
        };
        *self.0.state.borrow_mut() = opening.clone();
        self.0.events.emit(&MediaPlayerEvent::StateChanged(opening));
        if self.0.generation.get() != generation {
            return Box::pin(async { Err(MediaError::cancelled()) });
        }
        let pending = self.0.provider.open(
            source,
            self.0.source_id,
            generation,
            self.options().presentation,
        );
        let pending = self.guard(pending, OperationScope::Source, generation, 0);
        let weak = Rc::downgrade(&self.0);
        let guard = SourcePreparationGuard {
            player: weak.clone(),
            generation,
            committed: false,
        };
        Box::pin(async move {
            let mut guard = guard;
            let result = pending.await;
            let inner = weak
                .upgrade()
                .filter(|i| i.generation.get() == generation)
                .ok_or_else(MediaError::cancelled)?;
            let native = match result {
                Ok(native) => native,
                Err(error) => {
                    Self(inner).fail(error.clone());
                    return Err(error);
                }
            };
            let player = Self(inner);
            let options = player.options();
            let prepared = native
                .set_audio(options.volume, options.muted)
                .and_then(|()| native.state());
            let state = match prepared {
                Ok(state) => state,
                Err(error) => {
                    player.fail(error.clone());
                    return Err(error);
                }
            };
            *player.0.native.borrow_mut() = Some(native);
            *player.0.state.borrow_mut() = state;
            player.0.events.emit(&MediaPlayerEvent::Opened);
            if player.0.generation.get() != generation {
                return Err(MediaError::cancelled());
            }
            drop(player);
            // One preparation driver applies the newest queued seek before
            // starting playback. Superseded completions cannot win a race.
            loop {
                let inner = weak
                    .upgrade()
                    .filter(|i| i.generation.get() == generation)
                    .ok_or_else(MediaError::cancelled)?;
                if inner.state.borrow().state == MediaPlaybackState::Failed {
                    return Err(inner
                        .state
                        .borrow()
                        .error
                        .clone()
                        .unwrap_or_else(MediaError::cancelled));
                }
                let request = inner.pending_seek.borrow().clone();
                let Some(request) = request else {
                    break;
                };
                let pending = Self(inner).execute_seek(request.clone(), generation);
                let result = pending.await;
                request.finish(result);
            }
            let inner = weak
                .upgrade()
                .filter(|i| i.generation.get() == generation)
                .ok_or_else(MediaError::cancelled)?;
            inner.preparing.set(false);
            let intent = inner.intent.get();
            let command = inner.command.get();
            drop(inner);
            guard.committed = true;
            if intent == PlaybackIntent::Play {
                let inner = weak.upgrade().ok_or_else(MediaError::cancelled)?;
                let play = Self(inner).play_for_command(command, generation);
                play.await?;
            }
            Ok(())
        })
    }
    fn fail(&self, error: MediaError) {
        let mut state = self.playback_session();
        state.state = MediaPlaybackState::Failed;
        state.error = Some(error);
        *self.0.state.borrow_mut() = state.clone();
        self.0.frames.clear();
        self.0.preparing.set(false);
        self.next_command();
        self.next_seek();
        self.0.operations.cancel(None);
        let native = self.0.native.borrow_mut().take();
        drop(native);
        self.0.events.emit(&MediaPlayerEvent::StateChanged(state));
    }
    pub fn play(&self) -> MediaFuture<()> {
        self.0.intent.set(PlaybackIntent::Play);
        let command = self.next_command();
        let generation = self.0.generation.get();
        self.0
            .events
            .emit(&MediaPlayerEvent::IntentChanged(PlaybackIntent::Play));
        self.play_for_command(command, generation)
    }
    fn play_for_command(&self, command: u64, generation: u64) -> MediaFuture<()> {
        let native = if self.0.preparing.get() {
            None
        } else {
            self.0.native.borrow().as_ref().map(Rc::downgrade)
        };
        let weak = Rc::downgrade(&self.0);
        Box::pin(async move {
            if !weak
                .upgrade()
                .is_some_and(|i| i.generation.get() == generation && i.command.get() == command)
            {
                return Err(MediaError::cancelled());
            }
            if let Some(native) = native.and_then(|native| native.upgrade()) {
                let inner = weak.upgrade().ok_or_else(MediaError::cancelled)?;
                let pending = Self(inner).guard(
                    native.play(),
                    OperationScope::Transport,
                    generation,
                    command,
                );
                drop(native);
                let result = pending.await;
                let inner = weak
                    .upgrade()
                    .filter(|i| i.generation.get() == generation && i.command.get() == command)
                    .ok_or_else(MediaError::cancelled)?;
                if let Err(error) = result {
                    // Autoplay denial is recoverable by a later explicit Play.
                    if error.kind != MediaErrorKind::AutoplayDenied {
                        Self(inner).fail(error.clone());
                    }
                    return Err(error);
                }
            }
            Ok(())
        })
    }
    pub fn pause(&self) -> Result<(), MediaError> {
        self.0.intent.set(PlaybackIntent::Pause);
        self.next_command();
        let native = self.0.native.borrow().clone();
        if let Some(native) = native {
            native.pause()?;
        }
        self.0
            .events
            .emit(&MediaPlayerEvent::IntentChanged(PlaybackIntent::Pause));
        Ok(())
    }
    pub fn seek(&self, position: Duration) -> MediaFuture<Duration> {
        let command = self.next_seek();
        let generation = self.0.generation.get();
        let request = Rc::new(SeekRequest {
            command,
            position,
            completed: Cell::new(false),
            result: RefCell::new(None),
            waker: RefCell::new(None),
        });
        *self.0.pending_seek.borrow_mut() = Some(request.clone());
        let weak = Rc::downgrade(&self.0);
        Box::pin(async move {
            let awaited = request.clone();
            let waiting = weak.clone();
            let result = std::future::poll_fn(move |cx| {
                let Some(inner) = waiting.upgrade().filter(|i| {
                    i.generation.get() == generation && i.seek_command.get() == command
                }) else {
                    return std::task::Poll::Ready(Some(Err(MediaError::cancelled())));
                };
                if let Some(result) = awaited.result.borrow_mut().take() {
                    return std::task::Poll::Ready(Some(result));
                }
                if inner.preparing.get() {
                    *awaited.waker.borrow_mut() = Some(cx.waker().clone());
                    std::task::Poll::Pending
                } else {
                    std::task::Poll::Ready(None)
                }
            })
            .await;
            if let Some(result) = result {
                return result;
            }
            let inner = weak.upgrade().ok_or_else(MediaError::cancelled)?;
            let pending = Self(inner).execute_seek(request.clone(), generation);
            let result = pending.await;
            request.finish(result.clone());
            result
        })
    }
    fn execute_seek(&self, request: Rc<SeekRequest>, generation: u64) -> MediaFuture<Duration> {
        let command = request.command;
        let native = self.0.native.borrow().as_ref().map(Rc::downgrade);
        let weak = Rc::downgrade(&self.0);
        Box::pin(async move {
            let result = async {
                if !weak.upgrade().is_some_and(|i| {
                    i.generation.get() == generation && i.seek_command.get() == command
                }) {
                    return Err(MediaError::cancelled());
                }
                let native = native.and_then(|native| native.upgrade()).ok_or_else(|| {
                    MediaError::new(MediaErrorKind::Unsupported, "Media is not ready")
                })?;
                let state = native.state()?;
                if !state.seekable {
                    if request.position == Duration::ZERO
                        && weak
                            .upgrade()
                            .is_some_and(|i| i.intent.get() == PlaybackIntent::Stop)
                    {
                        return Ok(state.position);
                    }
                    return Err(MediaError::new(
                        MediaErrorKind::Unsupported,
                        "Media is not seekable",
                    ));
                }
                let inner = weak
                    .upgrade()
                    .filter(|i| i.generation.get() == generation && i.seek_command.get() == command)
                    .ok_or_else(MediaError::cancelled)?;
                let pending = Self(inner).guard(
                    native.seek(request.position),
                    OperationScope::Seek,
                    generation,
                    command,
                );
                drop(native);
                let achieved = pending.await?;
                let inner = weak
                    .upgrade()
                    .filter(|i| i.generation.get() == generation && i.seek_command.get() == command)
                    .ok_or_else(MediaError::cancelled)?;
                inner.state.borrow_mut().position = achieved;
                let pending = inner.pending_seek.borrow_mut().take();
                drop(pending);
                inner
                    .events
                    .emit(&MediaPlayerEvent::SeekCompleted(achieved));
                Ok(achieved)
            }
            .await;
            if let Some(inner) = weak
                .upgrade()
                .filter(|i| i.generation.get() == generation && i.seek_command.get() == command)
            {
                inner.pending_seek.borrow_mut().take();
            }
            result
        })
    }
    pub fn stop(&self) -> MediaFuture<Duration> {
        self.0.intent.set(PlaybackIntent::Stop);
        let command = self.next_command();
        let generation = self.0.generation.get();
        let native = self.0.native.borrow().clone();
        let paused = native.map_or(Ok(()), |native| native.stop());
        if self.0.generation.get() != generation || self.0.command.get() != command {
            return Box::pin(async { Err(MediaError::cancelled()) });
        }
        self.0.frames.clear();
        let seek = if self.0.preparing.get() || self.playback_session().seekable {
            self.seek(Duration::ZERO)
        } else {
            self.next_seek();
            let position = self.playback_session().position;
            Box::pin(async move { Ok(position) }) as MediaFuture<Duration>
        };
        self.0
            .events
            .emit(&MediaPlayerEvent::IntentChanged(PlaybackIntent::Stop));
        Box::pin(async move {
            paused?;
            seek.await
        })
    }
    pub fn clear_source(&self) {
        self.replace();
        self.0.source.borrow_mut().take();
        self.0.intent.set(PlaybackIntent::Pause);
        *self.0.state.borrow_mut() = MediaPlaybackSession::default();
        self.0
            .events
            .emit(&MediaPlayerEvent::StateChanged(self.playback_session()));
    }
    /// Called by the framework render/timer lifecycle, never a new global clock.
    /// Loop restart is returned as explicit asynchronous work for that lifecycle.
    pub fn poll(&self) -> Result<Option<MediaFuture<()>>, MediaError> {
        let Some(native) = self.0.native.borrow().clone() else {
            return Ok(None);
        };
        let generation = self.0.generation.get();
        let state = match native.state() {
            Ok(state) => state,
            Err(error) => {
                self.fail(error.clone());
                return Err(error);
            }
        };
        let resetting = self.0.intent.get() == PlaybackIntent::Stop
            && (!state.seekable || self.0.pending_seek.borrow().is_some());
        let frame = match if self.0.preparing.get() || resetting {
            Ok(None)
        } else {
            native.take_frame()
        } {
            Ok(frame) => frame,
            Err(error) => {
                self.fail(error.clone());
                return Err(error);
            }
        };
        if let Some(frame) = frame {
            self.0
                .frames
                .publish(frame)
                .map_err(|e| MediaError::new(MediaErrorKind::NativeFailure, e.to_string()))?;
        }
        let ended = state.state == MediaPlaybackState::Ended;
        *self.0.state.borrow_mut() = state.clone();
        self.0.events.emit(&MediaPlayerEvent::StateChanged(state));
        if self.0.generation.get() != generation {
            return Ok(None);
        }
        if ended && !self.0.ended.replace(true) {
            self.0.events.emit(&MediaPlayerEvent::Ended);
            if self.0.generation.get() != generation {
                return Ok(None);
            }
            if self.options().looping {
                let seek = self.seek(Duration::ZERO);
                let transport_command = self.0.command.get();
                let weak = Rc::downgrade(&self.0);
                let generation = self.0.generation.get();
                return Ok(Some(Box::pin(async move {
                    seek.await?;
                    let inner = weak
                        .upgrade()
                        .filter(|i| {
                            i.generation.get() == generation
                                && i.command.get() == transport_command
                                && i.intent.get() == PlaybackIntent::Play
                        })
                        .ok_or_else(MediaError::cancelled)?;
                    inner.ended.set(false);
                    Self(inner).play().await
                })));
            }
        } else if !ended {
            self.0.ended.set(false);
        }
        Ok(None)
    }
}
#[cfg(test)]
mod tests;

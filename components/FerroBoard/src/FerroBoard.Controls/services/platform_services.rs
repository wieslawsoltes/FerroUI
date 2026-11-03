//! Derived from ScreenKeyboard.Uno/Services/PlatformServices.cs (Apache-2.0).
use ferroboard_core::{
    clipboard::{ClipboardFuture, IClipboardService},
    input::{IKeyboardScheduler, ScheduledAction},
    text::Utf16Text,
};
use ferroui_base::threading::{Dispatcher, DispatcherPriority, DispatcherTimer};
use ferroui_base::{
    FerroLocator, LocatorExtensions, Ref, Visual,
    input::{
        LocalBoxFuture,
        platform::{ClipboardError, ClipboardErrorKind, ClipboardExtensions, IClipboard},
    },
};
use ferroui_controls::TopLevel;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
    time::Duration,
};

/// One-shot keyboard callbacks on the selected UI dispatcher. Default is the calling thread's
/// dispatcher, matching the source current-thread queue rather than process-global focus/window state.
/// Retain the returned handle until completion. Its final Drop cancels pending work and releases
/// callback payloads, as does explicit cancellation; clones share this ownership.
pub struct DispatcherKeyboardScheduler {
    dispatcher: Arc<Dispatcher>,
}
impl Default for DispatcherKeyboardScheduler {
    fn default() -> Self {
        Self::new(None)
    }
}
impl DispatcherKeyboardScheduler {
    pub fn new(dispatcher: Option<Arc<Dispatcher>>) -> Self {
        Self {
            dispatcher: dispatcher.unwrap_or_else(Dispatcher::current_dispatcher),
        }
    }
}
struct CallbackState {
    cancelled: Cell<bool>,
    action: RefCell<Option<Rc<dyn Fn()>>>,
}
impl IKeyboardScheduler for DispatcherKeyboardScheduler {
    fn schedule(&self, delay: Duration, action: Rc<dyn Fn()>) -> ScheduledAction {
        let timer = DispatcherTimer::with_priority_and_dispatcher(
            DispatcherPriority::NORMAL,
            &self.dispatcher,
        );
        let interval = if delay.is_zero() {
            Duration::from_millis(1)
        } else {
            delay
        };
        timer.set_interval(interval);
        let callback = Rc::new(CallbackState {
            cancelled: Cell::new(false),
            action: RefCell::new(Some(action)),
        });
        let state = callback.clone();
        // The timer owns callback state, not its cancellation handle. Consume the action
        // before invoking it so one-shot completion releases captures even if the handle lives.
        timer.tick(move |timer| {
            timer.stop();
            let action = state.action.borrow_mut().take();
            if !state.cancelled.get()
                && let Some(action) = action
            {
                action();
            }
        });
        timer.start();
        ScheduledAction::new(move || {
            callback.cancelled.set(true);
            timer.stop();
            let action = callback.action.borrow_mut().take();
            drop(action);
        })
    }
}

/// Source platform clipboard service. The default resolves the calling UI thread's platform
/// service for every operation; `for_visual` uses the owning TopLevel instead. No focus lookup
/// or process-global clipboard cache is used. Backend failures retain the source fallbacks.
pub struct SystemClipboardService {
    resolve: Rc<dyn Fn() -> Option<Rc<dyn IClipboard>>>,
}
impl Default for SystemClipboardService {
    fn default() -> Self {
        Self::new()
    }
}
impl SystemClipboardService {
    pub fn new() -> Self {
        Self::with_resolver(|| FerroLocator::current().get_service::<dyn IClipboard>())
    }
    /// Resolves the visual's current window each time, including after reparenting. An unattached
    /// visual has no window clipboard. The default locator is intentionally not a window fallback.
    pub fn for_visual(visual: Ref<Visual>) -> Self {
        let visual = visual.downgrade();
        Self::with_resolver(move || {
            visual
                .upgrade()
                .and_then(|visual| TopLevel::get_top_level(Some(&visual)))
                .and_then(|root| root.clipboard())
        })
    }
    pub fn with_resolver(resolve: impl Fn() -> Option<Rc<dyn IClipboard>> + 'static) -> Self {
        Self {
            resolve: Rc::new(resolve),
        }
    }
    /// Native Rust API. Platform failures remain available to the caller.
    pub fn read_text(&self) -> LocalBoxFuture<Result<Option<String>, ClipboardError>> {
        match (self.resolve)() {
            Some(clipboard) => clipboard.try_get_text_async(),
            None => Box::pin(std::future::ready(Err(ClipboardError::new(
                ClipboardErrorKind::Platform,
                "No window clipboard is available",
            )))),
        }
    }
    /// Native Rust API. Unlike the historical engine adapter, completion means the write finished.
    pub fn write_text(&self, text: &str) -> LocalBoxFuture<Result<(), ClipboardError>> {
        match (self.resolve)() {
            Some(clipboard) => clipboard.set_text_async(Some(text)),
            None => Box::pin(std::future::ready(Err(ClipboardError::new(
                ClipboardErrorKind::Platform,
                "No window clipboard is available",
            )))),
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformServiceError {
    /// The native String clipboard contract cannot represent this source UTF-16 string.
    UnpairedSurrogate,
    /// The mobile composition root has not installed its native vibration device.
    MobileHapticsUnavailable,
}
impl std::fmt::Display for PlatformServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for PlatformServiceError {}
impl IClipboardService for SystemClipboardService {
    fn get_text_async(&self) -> ClipboardFuture<Option<Utf16Text>> {
        // Resolve/invoke before returning, as the source does before its first await.
        let read = self.read_text();
        Box::pin(async move { Ok(read.await.ok().flatten().map(|s| s.as_str().into())) })
    }
    fn set_text_async(&self, text: Option<Utf16Text>) -> ClipboardFuture<()> {
        // Source null/access failures are intentional no-ops; invalid native encoding is not.
        let Some(text) = text else {
            return Box::pin(std::future::ready(Ok(())));
        };
        let Ok(text) = text.to_utf8() else {
            return Box::pin(std::future::ready(Err(
                Arc::new(PlatformServiceError::UnpairedSurrogate)
                    as ferroboard_core::clipboard::ClipboardServiceError,
            )));
        };
        if let Some(clipboard) = (self.resolve)() {
            let write = clipboard.set_text_async(Some(&text));
            // Preserve source completed-write ordering while the native backend can pend.
            // Dispatcher owns this future; no Task/TaskCompletionSource adapter is involved.
            drop(
                Dispatcher::current_dispatcher()
                    .to_task_scheduler()
                    .start_local(async move {
                        let _ = write.await; // Source catches platform clipboard exceptions.
                    }),
            );
        }
        Box::pin(std::future::ready(Ok(())))
    }
}

/// A real mobile vibration device. False explicitly means hardware/access/duration is
/// unavailable, the source's intentional platform fallback. Missing integration is an error.
pub trait HapticDevice {
    fn vibrate(&self, duration_ms: i32) -> bool;
}
pub struct KeyboardHapticService {
    device: Rc<dyn HapticDevice>,
}
impl KeyboardHapticService {
    /// Concrete mobile defaults for a native AppBuilder composition root.
    #[cfg(any(target_os = "android", target_os = "ios"))]
    pub fn native() -> Self {
        #[cfg(target_os = "android")]
        let device = Rc::new(super::AndroidHaptics::new());
        #[cfg(target_os = "ios")]
        let device = Rc::new(super::IosHaptics::new());
        Self::new(device)
    }
    pub fn new(device: Rc<dyn HapticDevice>) -> Self {
        Self { device }
    }
    pub fn perform(&self, kind: ferroboard_core::settings::FeedbackKind, duration_ms: i32) -> bool {
        use ferroboard_core::settings::FeedbackKind;
        // C# unchecked integer arithmetic precedes its caught native vibration call.
        let duration = if kind == FeedbackKind::LongPress {
            duration_ms.wrapping_mul(2)
        } else {
            duration_ms
        };
        self.device.vibrate(duration)
    }
}

/// The source deliberately does nothing outside Android/iOS. Mobile hosts inject a real
/// device through AppBuilder::with. A denied/unavailable device is the source-defined no-op;
/// an absent adapter remains an explicit integration failure.
pub struct KeyboardHaptics;
impl KeyboardHaptics {
    pub fn perform(
        _kind: ferroboard_core::settings::FeedbackKind,
        _duration_ms: i32,
    ) -> Result<(), PlatformServiceError> {
        #[cfg(any(target_os = "android", target_os = "ios"))]
        {
            let service = FerroLocator::current()
                .get_service::<KeyboardHapticService>()
                .ok_or(PlatformServiceError::MobileHapticsUnavailable)?;
            let _available = service.perform(_kind, _duration_ms);
            Ok(())
        }
        #[cfg(not(any(target_os = "android", target_os = "ios")))]
        {
            Ok(())
        }
    }
}

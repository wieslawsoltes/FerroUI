use super::{IPlatformHandle, ITopLevelImpl, IWindowBaseImpl, Screen, ScreenHelper};
use ferroui_base::threading::{Dispatcher, DispatcherOperation, DispatcherPriority};
use ferroui_base::{PixelPoint, PixelRect};
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::hash::Hash;
use std::ops::Deref;
use std::pin::Pin;
use std::rc::Rc;

/// The screens of the platform.
pub trait IScreenImpl {
    /// Gets the total number of screens available on the device.
    fn screen_count(&self) -> i32;

    /// Gets the list of all screens available on the device.
    fn all_screens(&self) -> Vec<Rc<Screen>>;

    /// Gets the method called when the screens change.
    fn changed(&self) -> Option<Rc<dyn Fn()>>;

    /// Sets a method called when the screens change.
    fn set_changed(&self, value: Option<Rc<dyn Fn()>>);

    /// The screen that `window` is on.
    fn screen_from_window(&self, window: &dyn IWindowBaseImpl) -> Option<Rc<Screen>>;

    /// The screen that `top_level` is on.
    fn screen_from_top_level(&self, top_level: &dyn ITopLevelImpl) -> Option<Rc<Screen>>;

    /// The screen that contains `point`.
    fn screen_from_point(&self, point: PixelPoint) -> Option<Rc<Screen>>;

    /// The screen that the largest part of `rect` is on.
    fn screen_from_rect(&self, rect: PixelRect) -> Option<Rc<Screen>>;

    /// Asks the platform for the permission to access the details of all
    /// screens; resolves to whether it was granted.
    ///
    /// The returned future is not tied to an executor; on the UI thread it
    /// is driven by the dispatcher (`invoke_async_task_local`).
    fn request_screen_details(&self) -> Pin<Box<dyn Future<Output = bool>>>;
}

/// A screen that is identified by a platform handle.
///
/// Backend screen types embed a `PlatformScreen` and expose it through
/// `AsRef<PlatformScreen>`; the screen data itself is shared as
/// [`Rc<Screen>`](Screen).
pub struct PlatformScreen {
    screen: Rc<Screen>,
}

impl PlatformScreen {
    /// Creates a screen identified by `platform_handle`.
    pub fn new(platform_handle: Rc<dyn IPlatformHandle>) -> Self {
        Self { screen: Rc::new(Screen::new(Some(platform_handle))) }
    }

    /// The shared screen.
    pub fn screen(&self) -> &Rc<Screen> {
        &self.screen
    }
}

impl Deref for PlatformScreen {
    type Target = Screen;

    fn deref(&self) -> &Screen {
        &self.screen
    }
}

impl AsRef<PlatformScreen> for PlatformScreen {
    fn as_ref(&self) -> &PlatformScreen {
        self
    }
}

/// The state of a screens implementation that is based on
/// [`ScreensBaseImpl`]: the screens by key and the cached values.
pub struct ScreensBase<TKey, TScreen> {
    all_screens_by_key: RefCell<HashMap<TKey, Rc<TScreen>>>,
    all_screens: RefCell<Option<Vec<Rc<TScreen>>>>,
    screen_count: Cell<Option<i32>>,
    screen_details_request_granted: Rc<Cell<Option<bool>>>,
    on_change_operation: RefCell<Option<DispatcherOperation>>,
    changed: RefCell<Option<Rc<dyn Fn()>>>,
}

impl<TKey, TScreen> ScreensBase<TKey, TScreen> {
    /// Creates the state of a screens implementation with no screens.
    pub fn new() -> Self {
        Self {
            all_screens_by_key: RefCell::new(HashMap::new()),
            all_screens: RefCell::new(None),
            screen_count: Cell::new(None),
            screen_details_request_granted: Rc::new(Cell::new(None)),
            on_change_operation: RefCell::new(None),
            changed: RefCell::new(None),
        }
    }
}

impl<TKey, TScreen> Default for ScreensBase<TKey, TScreen> {
    fn default() -> Self {
        Self::new()
    }
}

/// The overridable part of a screens implementation that keeps one screen
/// object per platform screen key, preserving the objects across changes.
///
/// A backend embeds a [`ScreensBase`], returns it from
/// [`screens_base`](Self::screens_base) and implements the two required
/// members; it then is an [`IScreenImpl`] and has the members of
/// [`ScreensBaseImplExt`]. The backend calls
/// [`on_changed`](ScreensBaseImplExt::on_changed) when the platform reports
/// a change of the screens.
pub trait ScreensBaseImpl: 'static {
    /// What identifies a screen for the platform.
    type Key: Eq + Hash + Clone + 'static;

    /// The screen type of the backend.
    type Screen: AsRef<PlatformScreen> + 'static;

    /// The state embedded in the implementation.
    fn screens_base(&self) -> &ScreensBase<Self::Key, Self::Screen>;

    /// The keys of all screens that currently exist.
    fn get_all_screen_keys(&self) -> Vec<Self::Key>;

    /// Creates the screen for a key that was not known before.
    fn create_screen_from_key(&self, key: &Self::Key) -> Rc<Self::Screen>;

    /// Called for a screen that was just created.
    fn screen_added(&self, screen: &Rc<Self::Screen>) {
        self.screen_changed(screen);
    }

    /// Called for a known screen each time the screens are refreshed.
    fn screen_changed(&self, _screen: &Rc<Self::Screen>) {}

    /// Called for a screen whose key disappeared.
    fn screen_removed(&self, screen: &Rc<Self::Screen>) {
        (**screen).as_ref().on_removed();
    }

    /// The number of screens.
    fn get_screen_count(&self) -> i32 {
        self.all_platform_screens().len() as i32
    }

    /// See [`IScreenImpl::request_screen_details`]; the outcome is cached.
    fn request_screen_details_core(&self) -> Pin<Box<dyn Future<Output = bool>>> {
        Box::pin(std::future::ready(true))
    }

    /// The screen that `top_level` is on.
    fn screen_from_top_level_core(&self, top_level: &dyn ITopLevelImpl) -> Option<Rc<Screen>> {
        match top_level.as_window_impl() {
            Some(window) => ScreenHelper::screen_from_window(window, &IScreenImpl::all_screens(self)),
            None => None,
        }
    }

    /// The screen that contains `point`.
    fn screen_from_point_core(&self, point: PixelPoint) -> Option<Rc<Screen>> {
        ScreenHelper::screen_from_point(point, &IScreenImpl::all_screens(self))
    }

    /// The screen that the largest part of `rect` is on.
    fn screen_from_rect_core(&self, rect: PixelRect) -> Option<Rc<Screen>> {
        ScreenHelper::screen_from_rect(rect, &IScreenImpl::all_screens(self))
    }
}

/// The non-overridable members of a [`ScreensBaseImpl`].
pub trait ScreensBaseImplExt: ScreensBaseImpl {
    /// Invalidates the cached screens and schedules the changed
    /// notification, so that multiple continuous platform events are
    /// accumulated into one.
    ///
    /// Must be called on the UI thread.
    fn on_changed(self: &Rc<Self>);

    /// The screen of a key, if the key is one of the current screen keys.
    fn try_get_screen(&self, key: &Self::Key) -> Option<Rc<Self::Screen>>;

    /// All current screens, as the screen type of the backend.
    fn all_platform_screens(&self) -> Vec<Rc<Self::Screen>>;
}

fn ensure_screens<T: ScreensBaseImpl + ?Sized>(this: &T) {
    let base = this.screens_base();
    if base.all_screens.borrow().is_some() {
        return;
    }

    // This is not synchronized, as it is expected to be called on the UI
    // thread only.
    Dispatcher::ui_thread().verify_access();

    let screens = this.get_all_screen_keys();

    let removed_keys: Vec<T::Key> = {
        let screens_set: HashSet<&T::Key> = screens.iter().collect();
        let all_screens_by_key = base.all_screens_by_key.borrow();
        all_screens_by_key.keys().filter(|key| !screens_set.contains(key)).cloned().collect()
    };
    for old_screen_key in removed_keys {
        let removed = base.all_screens_by_key.borrow_mut().remove(&old_screen_key);
        if let Some(screen) = removed {
            this.screen_removed(&screen);
        }
    }

    let mut temp_screens = Vec::with_capacity(screens.len());
    for new_screen_key in &screens {
        let old_screen = base.all_screens_by_key.borrow().get(new_screen_key).cloned();
        match old_screen {
            Some(old_screen) => {
                this.screen_changed(&old_screen);
                temp_screens.push(old_screen);
            }
            None => {
                let new_screen = this.create_screen_from_key(new_screen_key);
                this.screen_added(&new_screen);
                base.all_screens_by_key.borrow_mut().insert(new_screen_key.clone(), new_screen.clone());
                temp_screens.push(new_screen);
            }
        }
    }

    *base.all_screens.borrow_mut() = Some(temp_screens);
}

impl<T: ScreensBaseImpl + ?Sized> ScreensBaseImplExt for T {
    fn on_changed(self: &Rc<Self>) {
        let base = self.screens_base();

        // Mark cached fields invalid.
        base.screen_count.set(None);
        *base.all_screens.borrow_mut() = None;

        // Schedule a delayed job, so we can accumulate multiple continuous
        // events into one.
        let previous = base.on_change_operation.take();
        if let Some(previous) = previous {
            previous.abort();
        }

        let this = Rc::downgrade(self);
        let operation = Dispatcher::ui_thread().invoke_async_local_with_priority(
            move || {
                let Some(this) = this.upgrade() else {
                    return;
                };
                let base = this.screens_base();

                // Ensure screens if there is at least one subscriber
                // already, or at least one screen was previously
                // materialized, which we need to update now.
                let has_subscriber = base.changed.borrow().is_some();
                if has_subscriber || !base.all_screens_by_key.borrow().is_empty() {
                    ensure_screens(&*this);
                    let changed = base.changed.borrow().clone();
                    if let Some(changed) = changed {
                        changed();
                    }
                }
            },
            DispatcherPriority::INPUT,
        );
        *base.on_change_operation.borrow_mut() = Some(operation);
    }

    fn try_get_screen(&self, key: &Self::Key) -> Option<Rc<Self::Screen>> {
        ensure_screens(self);
        self.screens_base().all_screens_by_key.borrow().get(key).cloned()
    }

    fn all_platform_screens(&self) -> Vec<Rc<Self::Screen>> {
        ensure_screens(self);
        self.screens_base().all_screens.borrow().clone().unwrap_or_default()
    }
}

impl<T: ScreensBaseImpl + ?Sized> IScreenImpl for T {
    fn screen_count(&self) -> i32 {
        let base = self.screens_base();
        match base.screen_count.get() {
            Some(screen_count) => screen_count,
            None => {
                let screen_count = self.get_screen_count();
                base.screen_count.set(Some(screen_count));
                screen_count
            }
        }
    }

    fn all_screens(&self) -> Vec<Rc<Screen>> {
        ensure_screens(self);
        match self.screens_base().all_screens.borrow().as_ref() {
            Some(all_screens) => {
                all_screens.iter().map(|screen| (**screen).as_ref().screen().clone()).collect()
            }
            None => Vec::new(),
        }
    }

    fn changed(&self) -> Option<Rc<dyn Fn()>> {
        self.screens_base().changed.borrow().clone()
    }

    fn set_changed(&self, value: Option<Rc<dyn Fn()>>) {
        let previous = self.screens_base().changed.replace(value);
        drop(previous);
    }

    fn screen_from_window(&self, window: &dyn IWindowBaseImpl) -> Option<Rc<Screen>> {
        IScreenImpl::screen_from_top_level(self, window)
    }

    fn screen_from_top_level(&self, top_level: &dyn ITopLevelImpl) -> Option<Rc<Screen>> {
        self.screen_from_top_level_core(top_level)
    }

    fn screen_from_point(&self, point: PixelPoint) -> Option<Rc<Screen>> {
        self.screen_from_point_core(point)
    }

    fn screen_from_rect(&self, rect: PixelRect) -> Option<Rc<Screen>> {
        self.screen_from_rect_core(rect)
    }

    fn request_screen_details(&self) -> Pin<Box<dyn Future<Output = bool>>> {
        let granted = self.screens_base().screen_details_request_granted.clone();
        if let Some(granted) = granted.get() {
            return Box::pin(std::future::ready(granted));
        }

        let request = self.request_screen_details_core();
        Box::pin(async move {
            let result = request.await;
            granted.set(Some(result));
            result
        })
    }
}

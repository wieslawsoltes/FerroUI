//! Native window service mapping. No WinUI interfaces or CLR interop runtime.
use super::*;
use ferroui_base::{
    Rect, Visual, WeakRef, input::WindowDecorationsElementRole as Role, media::FlowDirection,
};
use ferroui_controls::{
    Window, WindowDecorations, WindowState,
    chrome::{TitleBarDecorations, WindowDecorationProperties, WindowDrawnDecorations},
    platform::PlatformAllowedWindowActions,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RibbonTitleBarWindowState {
    Detached,
    /// The source association remains, but another titlebar owns the native region.
    Superseded,
    /// A synchronous property callback requested a change during another change.
    Pending,
    AwaitingGeometry,
    Ready,
    /// A queued request failed; direct requests also return the same error.
    Failed(RibbonTitleBarWindowError),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RibbonTitleBarWindowError {
    WindowClosed,
    ExtensionUnsupported,
}
impl std::fmt::Display for RibbonTitleBarWindowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::WindowClosed => "The window is closed",
            Self::ExtensionUnsupported => "The platform declined client-area extension",
        })
    }
}
impl std::error::Error for RibbonTitleBarWindowError {}
enum WindowMutation {
    Attach(WeakRef<Window>),
    Detach,
    RestoreRoles,
    InstallRoles,
    Refresh,
    Closed(WeakRef<Window>),
}
// This queue serializes only this control's native window ownership changes.
// Endpoints are weak; no UI object is kept alive by a pending operation.
enum QueuedWindowMutation {
    Control {
        bar: WeakRef<RibbonTitleBar>,
        request: WindowMutation,
    },
    Release(WindowAttachment),
}
#[derive(Default)]
struct WindowMutations {
    running: bool,
    pending: std::collections::VecDeque<QueuedWindowMutation>,
}
ferroui_base::ferro_thread_local! {
    static WINDOW_MUTATIONS: RefCell<WindowMutations> = RefCell::new(WindowMutations::default());
}
struct MutationGuard;
impl MutationGuard {
    fn enter() -> Option<Self> {
        WINDOW_MUTATIONS.with_borrow_mut(|state| {
            if state.running {
                None
            } else {
                state.running = true;
                Some(Self)
            }
        })
    }
}
impl Drop for MutationGuard {
    fn drop(&mut self) {
        WINDOW_MUTATIONS.with_borrow_mut(|state| state.running = false);
    }
}
// Identity belongs to the native attachment scope, independent of whether its
// weak control endpoint can still be upgraded during Drop.
struct WindowOwner {
    bar: WeakRef<RibbonTitleBar>,
}
impl PartialEq for WindowOwner {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
fn drain_window_mutations() {
    loop {
        let next = WINDOW_MUTATIONS.with_borrow_mut(|state| state.pending.pop_front());
        match next {
            None => break,
            Some(QueuedWindowMutation::Control { bar, request }) => {
                if let Some(bar) = bar.upgrade() {
                    bar.window_pending.set(bar.window_pending.get() - 1);
                    if let Err(error) = bar.apply_window_mutation(request) {
                        bar.window_state
                            .set(RibbonTitleBarWindowState::Failed(error));
                    }
                }
            }
            Some(QueuedWindowMutation::Release(attachment)) => {
                release_attachment(attachment, || {})
            }
        }
    }
}
struct OwnedRole {
    visual: WeakRef<Visual>,
    previous: Role,
    written: Role,
}
pub(super) struct WindowBinding {
    window: WeakRef<Window>,
    previous_inset: Thickness,
    written_inset: Option<Thickness>,
}
pub(super) struct WindowAttachment {
    owner: Rc<WindowOwner>,
    window: WeakRef<Window>,
    previous_extension: bool,
    previous_height: f64,
    previous_title: bool,
    window_role_baseline: Role,
    written_height: Option<f64>,
    roles: Vec<OwnedRole>,
    layout: Option<ferroui_base::reactive::Subscription>,
    closed: Option<ferroui_base::reactive::Subscription>,
}
impl RibbonTitleBar {
    ferroui_base::ferro_property!(for RibbonTitleBar;
        fn window_owner_property() -> ferroui_base::AttachedProperty<Option<Rc<WindowOwner>>> {
            FerroProperty::register_attached::<RibbonTitleBar, Window, _>("FerroRibbonTitleBarOwner", None)
        }
    );
    /// Requests native caption integration. Deferred geometry is observable in
    /// the returned state and `window_attachment_state`; it is not a ready result.
    ///
    /// Requests made synchronously from native property callbacks return Pending
    /// and run in order after the current mutation. Query the state after the
    /// outer request returns to observe the final result, including Failed.
    pub fn attach_to_window(
        &self,
        window: &Window,
    ) -> Result<RibbonTitleBarWindowState, RibbonTitleBarWindowError> {
        if window.platform_impl().is_none() {
            return Err(RibbonTitleBarWindowError::WindowClosed);
        }
        self.mutate_window(WindowMutation::Attach(window.to_ref().downgrade()))
    }
    fn attach_to_window_now(
        &self,
        window: &Window,
    ) -> Result<RibbonTitleBarWindowState, RibbonTitleBarWindowError> {
        if window.platform_impl().is_none() {
            return Err(RibbonTitleBarWindowError::WindowClosed);
        }
        if self
            .window_attachment
            .borrow()
            .as_ref()
            .and_then(|a| a.window.upgrade())
            .is_some_and(|w| w.ptr_eq(&window.to_ref()))
        {
            self.install_window_roles_now();
            self.refresh_window_chrome_now();
            return Ok(self.window_attachment_state());
        }
        let same_binding = self
            .window_binding
            .borrow()
            .as_ref()
            .and_then(|a| a.window.upgrade())
            .is_some_and(|w| w.ptr_eq(&window.to_ref()));
        if !same_binding {
            self.detach_from_window_now();
        }
        let previous_owner = window.get_value(Self::window_owner_property());
        let transferred = previous_owner
            .and_then(|owner| owner.bar.upgrade())
            .and_then(|previous| previous.suspend_window_now());
        let Some(platform) = window.platform_impl() else {
            if let Some(previous) = transferred {
                release_attachment(previous, || {});
            }
            return Err(RibbonTitleBarWindowError::WindowClosed);
        };
        let current_extension = window.extend_client_area_to_decorations_hint();
        let current_height = window.extend_client_area_title_bar_height_hint();
        let current_title = WindowDrawnDecorations::get_title_bar_decorations(window)
            .contains(TitleBarDecorations::TITLE);
        let current_role = WindowDecorationProperties::get_element_role(window);
        // Carry only still-owned settings into the next lease. A caller's
        // intervening native preference becomes that next lease's baseline.
        let previous_extension = transferred
            .as_ref()
            .filter(|_| current_extension)
            .map_or(current_extension, |a| a.previous_extension);
        let previous_height = transferred
            .as_ref()
            .filter(|a| a.written_height == Some(current_height))
            .map_or(current_height, |a| a.previous_height);
        let previous_title = transferred
            .as_ref()
            .filter(|_| !current_title)
            .map_or(current_title, |a| a.previous_title);
        let window_role_baseline = transferred
            .as_ref()
            .filter(|_| current_role == Role::User)
            .map_or(current_role, |a| a.window_role_baseline);
        let written_height = transferred
            .as_ref()
            .and_then(|a| a.written_height)
            .filter(|h| *h == current_height);
        if !same_binding {
            self.window_binding.replace(Some(WindowBinding {
                window: window.to_ref().downgrade(),
                previous_inset: self.caption_buttons_inset(),
                written_inset: None,
            }));
        }
        let owner = Rc::new(WindowOwner {
            bar: self.to_ref().downgrade(),
        });
        self.window_attachment.replace(Some(WindowAttachment {
            owner: owner.clone(),
            window: window.to_ref().downgrade(),
            previous_extension,
            previous_height,
            previous_title,
            window_role_baseline,
            written_height,
            roles: vec![],
            layout: None,
            closed: None,
        }));
        // Publish the native lease before observable window writes.
        window.set_value(Self::window_owner_property(), Some(owner));
        window.set_extend_client_area_to_decorations_hint(true);
        if window.platform_impl().is_none() {
            self.detach_from_window_now();
            return Err(RibbonTitleBarWindowError::WindowClosed);
        }
        if !platform.is_client_area_extended_to_decorations() {
            self.reject_unsupported_attachment_now();
            return Err(RibbonTitleBarWindowError::ExtensionUnsupported);
        }
        let hints = WindowDrawnDecorations::get_title_bar_decorations(window);
        WindowDrawnDecorations::set_title_bar_decorations(
            window,
            hints & !TitleBarDecorations::TITLE,
        );
        let weak = self.to_ref().downgrade();
        let layout = window.layout_updated(move || {
            if let Some(bar) = weak.upgrade() {
                bar.refresh_window_chrome();
            }
        });
        let weak = self.to_ref().downgrade();
        let anchor = window.to_ref().downgrade();
        let closed = window.add_disposable_handler(Window::window_closed_event(), move |_, _| {
            if let Some(bar) = weak.upgrade() {
                let _ = bar.mutate_window(WindowMutation::Closed(anchor.clone()));
            }
        });
        if let Some(a) = self.window_attachment.borrow_mut().as_mut() {
            a.layout = Some(layout);
            a.closed = Some(closed);
        }
        self.install_window_roles_now();
        self.refresh_window_chrome_now();
        Ok(self.window_attachment_state())
    }
    pub fn window_attachment_state(&self) -> RibbonTitleBarWindowState {
        if self.window_pending.get() == 0 {
            self.window_state.get()
        } else {
            RibbonTitleBarWindowState::Pending
        }
    }
    /// Restores owned native state. From a property callback this is queued;
    /// restoration completes before the outer mutation returns.
    pub fn detach_from_window(&self) {
        let _ = self.mutate_window(WindowMutation::Detach);
    }
    fn mutate_window(
        &self,
        request: WindowMutation,
    ) -> Result<RibbonTitleBarWindowState, RibbonTitleBarWindowError> {
        let Some(_guard) = MutationGuard::enter() else {
            self.window_pending.set(self.window_pending.get() + 1);
            WINDOW_MUTATIONS.with_borrow_mut(|state| {
                state.pending.push_back(QueuedWindowMutation::Control {
                    bar: self.to_ref().downgrade(),
                    request,
                })
            });
            return Ok(RibbonTitleBarWindowState::Pending);
        };
        let result = self.apply_window_mutation(request);
        if let Err(error) = result {
            self.window_state
                .set(RibbonTitleBarWindowState::Failed(error));
        }
        drain_window_mutations();
        result.map(|_| self.window_attachment_state())
    }
    /// Drop cannot enqueue a weak reference to an object being destroyed. Move
    /// its native lease into cleanup instead, without retaining/resurrecting self.
    pub(super) fn release_window_on_drop(&mut self) {
        self.window_binding.get_mut().take();
        let Some(attachment) = self.window_attachment.get_mut().take() else {
            return;
        };
        let Some(_guard) = MutationGuard::enter() else {
            // Restore a dead owner's baseline before any pending claimant reads
            // it. No control callback can upgrade the dying weak endpoint.
            WINDOW_MUTATIONS.with_borrow_mut(|state| {
                state
                    .pending
                    .push_front(QueuedWindowMutation::Release(attachment))
            });
            return;
        };
        release_attachment(attachment, || {});
        drain_window_mutations();
    }
    fn apply_window_mutation(
        &self,
        request: WindowMutation,
    ) -> Result<RibbonTitleBarWindowState, RibbonTitleBarWindowError> {
        match request {
            WindowMutation::Attach(window) => {
                let window = window
                    .upgrade()
                    .ok_or(RibbonTitleBarWindowError::WindowClosed)?;
                return self.attach_to_window_now(&window);
            }
            WindowMutation::Detach => self.detach_from_window_now(),
            WindowMutation::RestoreRoles => self.restore_window_roles_now(),
            WindowMutation::InstallRoles => self.install_window_roles_now(),
            WindowMutation::Refresh => self.refresh_window_chrome_now(),
            WindowMutation::Closed(window) => {
                let matches = self.window_attachment.borrow().as_ref().is_some_and(|a| {
                    a.window
                        .upgrade()
                        .zip(window.upgrade())
                        .is_some_and(|(a, b)| a.ptr_eq(&b))
                });
                if matches {
                    self.detach_from_window_now();
                }
            }
        }
        Ok(self.window_state.get())
    }
    fn detach_from_window_now(&self) {
        let attachment = self.window_attachment.borrow_mut().take();
        let binding = self.window_binding.borrow_mut().take();
        self.window_state.set(RibbonTitleBarWindowState::Detached);
        let inset = binding
            .filter(|a| a.written_inset == Some(self.caption_buttons_inset()))
            .map(|a| a.previous_inset);
        if let Some(a) = attachment {
            release_attachment(a, || {
                if let Some(inset) = inset {
                    self.set_caption_buttons_inset(inset);
                }
            });
        } else if let Some(inset) = inset {
            self.set_caption_buttons_inset(inset);
        }
    }
    fn reject_unsupported_attachment_now(&self) {
        self.window_state.set(RibbonTitleBarWindowState::Failed(
            RibbonTitleBarWindowError::ExtensionUnsupported,
        ));
        let attachment = self.window_attachment.borrow_mut().take();
        if let Some(attachment) = attachment {
            release_attachment(attachment, || {});
        }
        // Match the source catch-path layout and keep its window association so
        // a later template application can retry. Explicit Rust detach can still
        // restore the preassociation padding if the caller has not changed it.
        if let Some(binding) = self.window_binding.borrow_mut().as_mut() {
            binding.written_inset = Some(Thickness::default());
        }
        self.set_caption_buttons_inset(Thickness::default());
    }
    fn suspend_window_now(&self) -> Option<WindowAttachment> {
        let mut attachment = self.window_attachment.borrow_mut().take()?;
        self.window_state.set(RibbonTitleBarWindowState::Superseded);
        // Source SetTitleBar changes the active drag region, not the old bar's
        // stored window association or CaptionButtonsInset. Its next template
        // application is allowed to become the native owner again.
        let window = attachment.window.upgrade();
        let (window_roles, old_roles) = std::mem::take(&mut attachment.roles)
            .into_iter()
            .partition(|role| {
                window
                    .as_ref()
                    .is_some_and(|w| role.visual.upgrade().is_some_and(|v| v.ptr_eq(w)))
            });
        attachment.roles = window_roles;
        restore_roles(old_roles);
        if let Some(token) = attachment.layout.take() {
            token.dispose();
        }
        if let Some(token) = attachment.closed.take() {
            token.dispose();
        }
        Some(attachment)
    }
    pub(super) fn restore_window_roles(&self) {
        let _ = self.mutate_window(WindowMutation::RestoreRoles);
    }
    fn restore_window_roles_now(&self) {
        let roles = self
            .window_attachment
            .borrow_mut()
            .as_mut()
            .map(|a| std::mem::take(&mut a.roles))
            .unwrap_or_default();
        restore_roles(roles);
    }
    pub(super) fn install_window_roles(&self) {
        let _ = self.mutate_window(WindowMutation::InstallRoles);
    }
    fn install_window_roles_now(&self) {
        let active_window = self
            .window_attachment
            .borrow()
            .as_ref()
            .and_then(|a| a.window.upgrade());
        let Some(window) = active_window else {
            let associated = self
                .window_binding
                .borrow()
                .as_ref()
                .and_then(|a| a.window.upgrade());
            if let Some(window) = associated
                && let Err(error) = self.attach_to_window_now(&window)
            {
                self.window_state
                    .set(RibbonTitleBarWindowState::Failed(error));
            }
            return;
        };
        self.restore_window_roles_now();
        if let Some(a) = self.window_binding.borrow_mut().as_mut() {
            a.written_inset = None;
        }
        let (drag, title, qat, search, start, end) = {
            let p = self.parts.borrow();
            (
                p.drag.clone(),
                p.title.clone(),
                p.quick_access.clone(),
                p.search.clone(),
                p.start.clone(),
                p.end.clone(),
            )
        };
        let mut requested = vec![
            (window.clone().upcast::<Visual>(), Role::User),
            (
                self.to_ref().upcast::<Visual>(),
                if drag.is_some() {
                    Role::User
                } else {
                    Role::TitleBar
                },
            ),
        ];
        if let Some(drag) = drag {
            requested.push((drag.upcast(), Role::TitleBar));
        }
        if let Some(title) = title {
            requested.push((title.clone().upcast(), Role::TitleBar));
            requested.extend(title.get_visual_descendants().map(|v| (v, Role::TitleBar)));
        }
        requested.extend(qat.map(|v| (v.upcast::<Visual>(), Role::User)));
        requested.extend(search.map(|v| (v.upcast::<Visual>(), Role::User)));
        requested.extend(start.map(|v| (v.upcast::<Visual>(), Role::User)));
        requested.extend(end.map(|v| (v.upcast::<Visual>(), Role::User)));
        for (v, written) in requested {
            let previous = WindowDecorationProperties::get_element_role(&v);
            {
                let mut attachment = self.window_attachment.borrow_mut();
                let Some(a) = attachment.as_mut() else {
                    return;
                };
                if a.roles
                    .iter()
                    .any(|r| r.visual.upgrade().is_some_and(|old| old.ptr_eq(&v)))
                {
                    continue;
                }
                // Publish ownership before an observable native write.
                let previous = if v.ptr_eq(&window) {
                    a.window_role_baseline
                } else {
                    previous
                };
                a.roles.push(OwnedRole {
                    visual: v.downgrade(),
                    previous,
                    written,
                });
            }
            WindowDecorationProperties::set_element_role(&v, written);
        }
    }
    pub(super) fn refresh_window_chrome(&self) {
        let _ = self.mutate_window(WindowMutation::Refresh);
    }
    fn refresh_window_chrome_now(&self) {
        let Some(window) = self
            .window_attachment
            .borrow()
            .as_ref()
            .and_then(|a| a.window.upgrade())
        else {
            return;
        };
        let Some(platform) = window.platform_impl() else {
            self.detach_from_window_now();
            return;
        };
        let height = self.bounds().height;
        if height.is_finite()
            && height > 0.
            && window.extend_client_area_title_bar_height_hint() != height
        {
            if let Some(a) = self.window_attachment.borrow_mut().as_mut() {
                a.written_height = Some(height);
            }
            window.set_extend_client_area_title_bar_height_hint(height);
        }
        let inset = if platform.needs_managed_decorations() {
            measured_caption_insets(&window)
        } else {
            Some(if cfg!(target_os = "macos") {
                Thickness::new(76., 0., 0., 0.)
            } else {
                Thickness::new(0., 0., 140., 0.)
            })
        };
        if let Some(inset) = inset {
            let value = if self.flow_direction() == FlowDirection::RightToLeft {
                Thickness::new(inset.right, inset.top, inset.left, inset.bottom)
            } else {
                inset
            };
            let last = self
                .window_binding
                .borrow()
                .as_ref()
                .and_then(|a| a.written_inset);
            // A caller's explicit padding change owns the value until the next
            // attach/template application, as in the source's automatic setter.
            if last.is_none() || last == Some(self.caption_buttons_inset()) {
                if let Some(a) = self.window_binding.borrow_mut().as_mut() {
                    a.written_inset = Some(value);
                }
                self.set_caption_buttons_inset(value);
            }
            self.window_state.set(RibbonTitleBarWindowState::Ready);
        } else {
            self.window_state
                .set(RibbonTitleBarWindowState::AwaitingGeometry);
        }
    }
}
fn release_attachment(a: WindowAttachment, restore_inset: impl FnOnce()) {
    let window = a.window.upgrade();
    let owns_window = window.as_ref().is_some_and(|w| {
        w.get_value(RibbonTitleBar::window_owner_property())
            .as_ref()
            .is_some_and(|owner| Rc::ptr_eq(owner, &a.owner))
    });
    // A lease check prevents stale cleanup from changing a newer native owner.
    let roles = a
        .roles
        .into_iter()
        .filter(|role| {
            owns_window
                || !window
                    .as_ref()
                    .is_some_and(|w| role.visual.upgrade().is_some_and(|v| v.ptr_eq(w)))
        })
        .collect();
    restore_roles(roles);
    if owns_window
        && let Some(window) = window.as_ref()
        && WindowDecorationProperties::get_element_role(window) == Role::User
    {
        // A transferred scope may fail before its first role installation.
        WindowDecorationProperties::set_element_role(window, a.window_role_baseline);
    }
    if let Some(token) = a.layout {
        token.dispose();
    }
    if let Some(token) = a.closed {
        token.dispose();
    }
    restore_inset();
    if let Some(window) = window.filter(|_| owns_window) {
        if window.platform_impl().is_some() {
            let current = WindowDrawnDecorations::get_title_bar_decorations(&window);
            if !current.contains(TitleBarDecorations::TITLE) && a.previous_title {
                WindowDrawnDecorations::set_title_bar_decorations(
                    &window,
                    current | TitleBarDecorations::TITLE,
                );
            }
            if a.written_height == Some(window.extend_client_area_title_bar_height_hint()) {
                window.set_extend_client_area_title_bar_height_hint(a.previous_height);
            }
            if window.extend_client_area_to_decorations_hint() {
                window.set_extend_client_area_to_decorations_hint(a.previous_extension);
            }
        }
        window.clear_value(RibbonTitleBar::window_owner_property());
    }
}
fn restore_roles(roles: Vec<OwnedRole>) {
    for role in roles {
        if let Some(v) = role.visual.upgrade()
            && WindowDecorationProperties::get_element_role(&v) == role.written
        {
            WindowDecorationProperties::set_element_role(&v, role.previous);
        }
    }
}
/// Read the actual platform-provided caption visual geometry. Values are already
/// in client DIPs; native transforms account for render scaling and custom themes.
pub(super) fn measured_caption_insets(window: &Window) -> Option<Thickness> {
    let hints = WindowDrawnDecorations::get_title_bar_decorations(window);
    let actions = window.allowed_window_actions();
    let wanted = hints.contains(TitleBarDecorations::CLOSE_BUTTON)
        || (hints.contains(TitleBarDecorations::MINIMIZE_BUTTON)
            && actions.contains(PlatformAllowedWindowActions::MINIMIZE))
        || (hints.contains(TitleBarDecorations::MAXIMIZE_BUTTON)
            && actions.contains(PlatformAllowedWindowActions::MAXIMIZE))
        || (hints.contains(TitleBarDecorations::FULL_SCREEN_BUTTON)
            && actions.contains(PlatformAllowedWindowActions::FULLSCREEN));
    if !wanted
        || window.window_state() == WindowState::FullScreen
        || window.window_decorations() != WindowDecorations::Full
    {
        return Some(Thickness::default());
    }
    let width = window.client_size().width;
    if !width.is_finite() || width <= 0. {
        return None;
    }
    let root = window.presentation_source().root_visual()?;
    let mut left: f64 = 0.;
    let mut right: f64 = 0.;
    let mut found = false;
    for visual in root.get_visual_descendants() {
        if !matches!(
            WindowDecorationProperties::get_element_role(&visual),
            Role::MinimizeButton
                | Role::MaximizeButton
                | Role::CloseButton
                | Role::FullScreenButton
        ) || !visual.is_effectively_visible()
        {
            continue;
        }
        let Some(control) = visual.cast::<Control>() else {
            continue;
        };
        let size = control.bounds().size();
        if size.width <= 0.
            || size.height <= 0.
            || !size.width.is_finite()
            || !size.height.is_finite()
        {
            continue;
        }
        let Some(transform) = control.transform_to_visual(window) else {
            continue;
        };
        let rect = Rect::new(0., 0., size.width, size.height).transform_to_aabb(transform);
        if !rect.x.is_finite() || !rect.width.is_finite() {
            continue;
        }
        found = true;
        if rect.center().x <= width / 2. {
            left = left.max(rect.right());
        } else {
            right = right.max(width - rect.x);
        }
    }
    found.then(|| Thickness::new(left, 0., right, 0.))
}

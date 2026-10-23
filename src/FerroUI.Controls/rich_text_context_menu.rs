//! The configured context-menu path of read-only rich-text views.
//!
//! WinUI's synchronous flyout helper invokes RichTextBlock's opening event, but
//! has no RichTextBlockOverflow branch. The separate legacy selection manager
//! forwards an asynchronous, command-gated event to the master; that automatic
//! Copy-menu path is not projected by this configured-menu bridge.

use super::*;
use crate::primitives::{FlyoutBase, FlyoutBaseImplExt, PopupFlyoutBase};
use crate::{ContextMenu, ContextMenuEventArgs, RichEditBox, RichEditBoxTextPresenter};
use ferroui_base::input::ContextRequestedEventArgs;
use ferroui_base::interactivity::{
    IRoutedEventArgs, Interactive, InteractiveImplExt, RoutedEventHandlerToken,
};

#[derive(Clone)]
pub(super) struct ContextMenuKeyRequest {
    request: RoutedEventArgs,
    generation: u64,
    flyout: WeakRef<PopupFlyoutBase>,
}

struct ContextMenuKeyRequestScope<'a> {
    owner: &'a RichTextBlock,
    request: RoutedEventArgs,
}

impl Drop for ContextMenuKeyRequestScope<'_> {
    fn drop(&mut self) {
        let is_current = self
            .owner
            .context_menu_key_request
            .borrow()
            .as_ref()
            .is_some_and(|current| current.request == self.request);
        if is_current {
            self.owner.context_menu_key_request.borrow_mut().take();
        }
        // A nested key request supersedes the previous one. Never restore an
        // older identity after its generation was retired by application code.
    }
}

impl RichTextBlock {
    ferroui_base::ferro_routed_event!(
        /// Raised synchronously before this view opens a configured context
        /// menu or flyout. `Handled` cancels the associated request.
        ///
        /// Native overflow views do not raise this event for the modern
        /// context-flyout path. Their configured flyout still opens at the
        /// requesting overflow, including one inherited from the master.
        pub fn context_menu_opening_event() -> RoutedEvent<ContextMenuEventArgs> {
            RoutedEvent::register::<RichTextBlock, _>(
                "ContextMenuOpening",
                RoutingStrategies::BUBBLE,
            )
        }
    );

    pub fn context_menu_opening(
        &self,
        handler: impl Fn(&Interactive, &ContextMenuEventArgs) + 'static,
    ) -> RoutedEventHandlerToken {
        self.add_handler(Self::context_menu_opening_event(), handler)
    }

    pub(super) fn invalidate_context_menu_request(&self) {
        self.context_menu_request_generation
            .set(self.context_menu_request_generation.get().wrapping_add(1));
    }

    /// Shared menu dispatch performs another coordinate conversion after the
    /// typed event. A custom render transform can run application code there,
    /// so that dispatch must retain the accepted rich request's identity too.
    pub(crate) fn context_menu_request_generation(&self) -> u64 {
        self.context_menu_request_generation.get()
    }

    pub(super) fn try_handle_context_menu_key_up(&self, key: &KeyEventArgs) -> bool {
        if key.handled()
            || !key.is_source(self)
            || self.to_ref().cast::<RichEditBoxTextPresenter>().is_some()
            || self.context_menu().is_some()
        {
            return false;
        }
        let Some(flyout) = PopupFlyoutBase::effective_context_flyout(self)
            .and_then(|flyout| flyout.cast::<PopupFlyoutBase>())
            .filter(|flyout| flyout.is_open() && flyout.target() == Some(self.to_ref().upcast()))
        else {
            return false;
        };
        let Some(settings) = self.get_platform_settings() else {
            return false;
        };
        if !settings
            .hotkey_configuration()
            .open_context_menu
            .iter()
            .any(|gesture| gesture.matches(Some(key)))
        {
            return false;
        }

        // Control normally synthesizes this same routed request. For an open
        // owned popup, retain only this request's identity so its accepted key
        // action can use the framework toggle without classifying programmatic
        // pointerless requests (including nested ones) as keyboard input.
        self.invalidate_context_menu_request();
        let request = ContextRequestedEventArgs::new();
        let identity = request.as_routed_event_args().clone();
        let previous = self
            .context_menu_key_request
            .replace(Some(ContextMenuKeyRequest {
                request: identity.clone(),
                generation: self.context_menu_request_generation.get(),
                flyout: flyout.downgrade(),
            }));
        if let Some(previous) = previous {
            previous.request.set_handled(true);
        }
        let _scope = ContextMenuKeyRequestScope {
            owner: self,
            request: identity,
        };
        self.raise_event(&request);
        key.set_handled(request.handled());
        true
    }

    /// WinUI's overflow GetContextFlyout falls back to m_pMaster, not the
    /// preceding overflow. A local flyout on an intermediate continuation must
    /// not become the next continuation's inherited flyout.
    pub(crate) fn inherited_context_flyout(&self) -> Option<Ref<FlyoutBase>> {
        self.context_flyout_master()
            .and_then(|master| master.context_flyout())
    }

    fn context_flyout_master(&self) -> Option<Ref<RichTextBlock>> {
        self.cached_overflow_master()
    }

    /// CUIElement gates a default ContextFlyout by the text owner's selection
    /// policy. An overflow's default value is null, so inheriting a custom
    /// master flyout does not itself make the overflow property non-default.
    /// An explicitly assigned value, including null, does make it non-default.
    pub(crate) fn context_flyout_is_enabled(&self) -> bool {
        self.is_set(Control::context_flyout_property().as_property())
            || self
                .context_flyout_master()
                .is_some_and(|master| master.is_text_selection_enabled())
    }

    pub(super) fn on_context_requested(&self, request: &ContextRequestedEventArgs) {
        // The editable presenter shares the formatter by deriving from this
        // view. Its RichEditBox owns the public event and keyboard caret anchor.
        if self.to_ref().cast::<RichEditBoxTextPresenter>().is_some() {
            return;
        }
        // A nested text control owns its request, even if it has no configured
        // menu. Ordinary parent menu routing can still handle an unclaimed one.
        let mut source = request
            .source()
            .and_then(|source| source.cast::<Interactive>());
        while let Some(current) = source {
            if current.cast::<RichEditBox>().is_some() {
                return;
            }
            if let Some(view) = current.cast::<RichTextBlock>() {
                if view != self.to_ref() {
                    return;
                }
                break;
            }
            source = current.interactive_parent();
        }

        let key_request = self
            .context_menu_key_request
            .borrow()
            .as_ref()
            .filter(|current| &current.request == request.as_routed_event_args())
            .cloned();
        if let Some(key_request) = &key_request {
            if key_request.generation != self.context_menu_request_generation.get() {
                request.set_handled(true);
                return;
            }
        } else {
            self.invalidate_context_menu_request();
        }
        let generation = self.context_menu_request_generation.get();
        // Even an unconfigured nested request retires an earlier invocation.
        // This bridge covers an actual configured invocation. Native automatic
        // Copy menus and selection flyouts remain separate contracts.
        if self.context_menu().is_none()
            && PopupFlyoutBase::context_flyout_for_request(self).is_none()
        {
            return;
        }

        let root = self.visual_root();
        let root_point = request.try_get_position(None).unwrap_or_else(|| {
            // CUIElement::OnContextRequestedCore leaves read-only keyboard
            // invocations at the requesting view's local (0, 0). Its editable
            // GetContextMenuShowPosition branch does not apply here.
            root.as_ref()
                .and_then(|root| self.transform_to_visual(root))
                .map_or(Point::default(), |transform| {
                    Point::default().transform(transform)
                })
        });

        // Reading an ITransform is virtual. It may detach the view or make a
        // newer request before the typed event has even been raised.
        if request.handled()
            || generation != self.context_menu_request_generation.get()
            || root != self.visual_root()
        {
            request.set_handled(true);
            return;
        }

        if self.to_ref().cast::<RichTextBlockOverflow>().is_none() {
            let args = ContextMenuEventArgs::new(root_point.x, root_point.y);
            args.set_routed_event(Some(Self::context_menu_opening_event()));
            self.raise_event(&args);
            if args.handled()
                || generation != self.context_menu_request_generation.get()
                || root != self.visual_root()
            {
                request.set_handled(true);
                return;
            }
        }

        if let Some(key_request) = key_request {
            if let Some(previous) = key_request.flyout.upgrade() {
                let previous_base: Ref<FlyoutBase> = previous.to_ref().upcast();
                if previous.is_open() && previous.target() == Some(self.to_ref().upcast()) {
                    // The public opening event still precedes cancellation.
                    // Native closes/reopens every invocation; FerroUI's actual
                    // context key toggles its owned popup through shared Hide.
                    previous.hide();
                    if generation != self.context_menu_request_generation.get()
                        || root != self.visual_root()
                        || previous.is_open()
                    {
                        request.set_handled(true);
                        return;
                    }
                }
                if self.context_menu().is_none()
                    && PopupFlyoutBase::effective_context_flyout(self)
                        .is_none_or(|current| current == previous_base)
                {
                    request.set_handled(true);
                    return;
                }
                // A typed handler replaced the accepted menu. The previous
                // popup is closed before the replacement uses the ordinary
                // dispatch below; a canceled Closing keeps the current popup.
            }
        }

        let placement_owner: &Visual = root.as_deref().unwrap_or(self);
        request.set_placement_rect(
            placement_owner,
            Rect::new(root_point.x, root_point.y, 0.0, 0.0),
        );
        // Re-read after application handlers so a replacement is the accepted
        // menu. ContextMenu is FerroUI's existing alternative to ContextFlyout;
        // its normal precedence, popup lifecycle, and feedback remain shared.
        if self.context_menu().is_some() {
            ContextMenu::control_context_requested(self, request);
        } else {
            PopupFlyoutBase::on_control_context_requested(self, request);
        }
        // A configured invocation was accepted before raising the event. A
        // removed menu or a canceled Opening consumes it too: do not retry a
        // snapshotted instance handler or open an ancestor's menu instead.
        request.set_handled(true);
    }

    pub(super) fn on_context_canceled(&self, request: &RoutedEventArgs) {
        // Local properties already install the ordinary cancellation handlers.
        // Only the inherited overflow path needs this additional entry point.
        if self.context_menu().is_some() || self.context_flyout().is_some() {
            return;
        }
        if self.inherited_context_flyout().is_some_and(|flyout| {
            flyout.target().as_ref() == Some(&self.to_ref().upcast::<Control>())
        }) {
            PopupFlyoutBase::on_control_context_canceled(self, request);
        }
    }
}

#[cfg(test)]
#[path = "rich_text_context_menu_tests.rs"]
mod tests;

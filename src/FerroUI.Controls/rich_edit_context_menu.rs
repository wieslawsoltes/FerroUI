use super::*;
use crate::documents::TextGravity;
use crate::{ContextMenu, ContextMenuEventArgs};
use ferroui_base::input::ContextRequestedEventArgs;
use ferroui_base::interactivity::InteractiveImplExt;
use ferroui_base::media::FlowDirection;

impl RichEditBox {
    ferro_routed_event!(
        /// Raised synchronously before a pointer, holding, or keyboard context
        /// request opens a menu. `Handled` cancels that request.
        pub fn context_menu_opening_event() -> RoutedEvent<ContextMenuEventArgs> {
            RoutedEvent::register::<RichEditBox, _>("ContextMenuOpening", RoutingStrategies::BUBBLE)
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

    pub(super) fn on_context_requested(&self, request: &ContextRequestedEventArgs) {
        // An embedded editor owns its request. Its unhandled request may bubble
        // through an outer editor, but must not project a second opening event.
        let mut source = request
            .source()
            .and_then(|source| source.cast::<Interactive>());
        while let Some(current) = source {
            if let Some(owner) = current.cast::<RichEditBox>() {
                if owner != self.to_ref() {
                    return;
                }
                break;
            }
            source = current.interactive_parent();
        }

        self.invalidate_context_menu_request();
        let generation = self.context_menu_request_generation.get();
        let root = self.visual_root();
        let document = self.document();
        let version = document.mutation_version();
        let selection = self.selection();
        let direction = self.flow_direction();
        let point = request.try_get_position(Some(self)).unwrap_or_else(|| {
            self.keyboard_context_menu_position(&document, selection, direction)
        });

        // Shaping embedded content can call application code. Do not publish a
        // position computed for a document or visual owner that was replaced.
        if generation != self.context_menu_request_generation.get()
            || root != self.visual_root()
            || document != self.document()
            || version != document.mutation_version()
            || selection != self.selection()
            || direction != self.flow_direction()
        {
            request.set_handled(true);
            return;
        }

        let root_point = request.try_get_position(None).unwrap_or_else(|| {
            root.as_ref()
                .and_then(|root| self.transform_to_visual(root))
                .map_or(point, |transform| point.transform(transform))
        });
        let args = ContextMenuEventArgs::new(root_point.x, root_point.y);
        args.set_routed_event(Some(Self::context_menu_opening_event()));
        self.raise_event(&args);

        // A nested request owns its own opening. Detachment, including a
        // detach/reattach to the same root, invalidates the outer request.
        if args.handled()
            || generation != self.context_menu_request_generation.get()
            || root != self.visual_root()
        {
            request.set_handled(true);
            return;
        }

        let placement_owner: &Visual = root.as_deref().unwrap_or(self);
        request.set_placement_rect(
            placement_owner,
            Rect::new(root_point.x, root_point.y, 0.0, 0.0),
        );
        // Read the accepted menu after application callbacks. This also lets a
        // handler install a menu during ContextMenuOpening: the original route
        // snapshot would not contain its newly registered instance handler.
        // The ordinary consumers still own popup creation, precedence, focus,
        // dismissal, and holding feedback; handled requests skip them later.
        let has_menu = self.context_menu().is_some() || self.context_flyout().is_some();
        ContextMenu::control_context_requested(self, request);
        if !request.handled() {
            PopupFlyoutBase::on_control_context_requested(self, request);
        }
        if has_menu {
            // Opening may have been canceled by the menu itself. That is still
            // a consumed request; do not retry its snapshotted instance handler
            // or fall through to a menu owned by an ancestor.
            request.set_handled(true);
        }
    }

    fn keyboard_context_menu_position(
        &self,
        document: &RichTextDocument,
        selection: EditorSelection,
        direction: FlowDirection,
    ) -> Point {
        let range = selection.range();
        let rtl = direction == FlowDirection::RightToLeft;
        let presenter = &self.text_presenter;
        let rect = if range.is_empty() {
            presenter.cursor_rectangle()
        } else {
            // Use the selected edge fragment so an end exactly at a soft wrap
            // remains on the selected line. A caret at the same UTF-16 offset
            // may belong to the following line. Grapheme boundaries also avoid
            // splitting surrogate pairs or a combining sequence.
            let edge = if rtl {
                range.start..document.next_grapheme(range.start).min(range.end)
            } else {
                document.previous_grapheme(range.end).max(range.start)..range.end
            };
            let rects = presenter.get_rects_for_range(edge);
            let rect = if rtl { rects.first() } else { rects.last() };
            rect.copied().unwrap_or_else(|| {
                presenter.get_rect_from_text_position(TextPosition::new(
                    if rtl { range.start } else { range.end },
                    if rtl {
                        TextGravity::Forward
                    } else {
                        TextGravity::Backward
                    },
                ))
            })
        };
        let point = if rtl {
            rect.bottom_left()
        } else {
            rect.bottom_right()
        };
        // Native GetContextMenuShowPosition chooses the view-local edge before
        // transforming that point. Choosing a corner of a transformed AABB
        // would swap the edge under RTL reflection (or rotation).
        presenter
            .transform_to_visual(self)
            .map_or(point, |transform| point.transform(transform))
    }
}

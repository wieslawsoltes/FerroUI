//! Port of `ManagedDragDrop/DragPreviewService.cs`.

use crate::DragPreviewWindow;
use ferroui_base::{BoxedValue, Point, Ref};
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::templates::IDataTemplate;
use ferroui_controls::{Control, TopLevel};
use std::cell::RefCell;
use std::rc::Rc;

ferroui_base::ferro_thread_local! {
    static WINDOW: RefCell<Option<Ref<DragPreviewWindow>>> = const { RefCell::new(None) };
}

/// Provides a lightweight preview window shown during a managed drag operation.
///
/// Deviation (DEVIATIONS.md, Behaviors): the preview window of the reference is shared by the
/// threads of the process; a window belongs to the thread that uses it, so there is one per
/// thread.
pub struct DragPreviewService;

impl DragPreviewService {
    /// Shows a drag preview near the current pointer position.
    ///
    /// `content` is rendered inside the preview window, with `template` when there is one;
    /// `origin` is the top-level where the drag originates, `client_position` the current pointer
    /// position in its client coordinates, `logical_offset` a logical offset added to the preview
    /// position in client coordinates and `opacity` the preview window opacity, clamped to 0..1.
    pub fn show(
        content: Option<BoxedValue>,
        template: Option<Rc<dyn IDataTemplate>>,
        origin: &TopLevel,
        client_position: Point,
        logical_offset: Point,
        opacity: f64,
    ) {
        Self::hide();

        let window = DragPreviewWindow::new();

        if template.is_some() {
            let presenter = ContentPresenter::new();
            presenter.set_content(content);
            presenter.set_content_template(template);
            window.set_content(Some(Control::boxed(presenter)));
        } else {
            window.set_content(content);
        }

        let screen_pos = origin.point_to_screen(client_position + logical_offset);

        // Manual clamp for cross-targeting frameworks
        let mut o = opacity;
        if o < 0.0 {
            o = 0.0;
        } else if o > 1.0 {
            o = 1.0;
        }
        window.set_opacity(o);

        window.set_position(screen_pos);
        window.show();
        WINDOW.with(|current| *current.borrow_mut() = Some(window));
    }

    /// The default opacity of [`show`](Self::show) (the default argument of the reference).
    pub const DEFAULT_OPACITY: f64 = 0.65;

    /// Moves the drag preview to a new client position of `origin`, plus `logical_offset`.
    pub fn move_(origin: &TopLevel, client_position: Point, logical_offset: Point) {
        let Some(window) = WINDOW.with(|current| current.borrow().clone()) else {
            return;
        };

        let screen_pos = origin.point_to_screen(client_position + logical_offset);
        window.set_position(screen_pos);
    }

    /// Hides the preview if shown.
    pub fn hide() {
        let window = WINDOW.with(|current| current.borrow_mut().take());
        if let Some(window) = window {
            // The reference ignores a failure to close the window.
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| window.close()));
        }
    }

    /// Whether a preview is shown (not in the reference: for the tests).
    #[allow(dead_code)]
    pub(crate) fn is_shown() -> bool {
        WINDOW.with(|current| current.borrow().is_some())
    }
}

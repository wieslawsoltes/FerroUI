use super::popup_positioning::PopupPositionRequest;
use super::{OverlayPopupHost, PopupRoot, TemplateAppliedEventArgs};
use crate::presenters::ContentPresenter;
use crate::Control;
use ferroui_base::input::IFocusScope;
use ferroui_base::media::Transform;
use ferroui_base::reactive::IDisposable;
use ferroui_base::{Ref, Visual};
use std::rc::Rc;

/// Represents the top-level control opened by a [`Popup`](super::Popup).
///
/// A popup host can be either be a popup window created by the operating
/// system ([`PopupRoot`]) or an [`OverlayPopupHost`] which is created on a
/// popup overlay layer.
///
/// Both hosts convert to `Rc<dyn IPopupHost>` with `to_popup_host()`
/// (internal upstream).
pub trait IPopupHost: IDisposable + IFocusScope {
    /// Gets the fixed width of the popup.
    fn width(&self) -> f64;

    /// Sets the fixed width of the popup.
    fn set_width(&self, value: f64);

    /// Gets the minimum width of the popup.
    fn min_width(&self) -> f64;

    /// Sets the minimum width of the popup.
    fn set_min_width(&self, value: f64);

    /// Gets the maximum width of the popup.
    fn max_width(&self) -> f64;

    /// Sets the maximum width of the popup.
    fn set_max_width(&self, value: f64);

    /// Gets the fixed height of the popup.
    fn height(&self) -> f64;

    /// Sets the fixed height of the popup.
    fn set_height(&self, value: f64);

    /// Gets the minimum height of the popup.
    fn min_height(&self) -> f64;

    /// Sets the minimum height of the popup.
    fn set_min_height(&self, value: f64);

    /// Gets the maximum height of the popup.
    fn max_height(&self) -> f64;

    /// Sets the maximum height of the popup.
    fn set_max_height(&self, value: f64);

    /// Gets the presenter from the control's template.
    fn presenter(&self) -> Option<Ref<ContentPresenter>>;

    /// Gets whether the popup appears on top of all other windows.
    fn topmost(&self) -> bool;

    /// Sets whether the popup appears on top of all other windows.
    fn set_topmost(&self, value: bool);

    /// Gets whether the popup takes part in pointer hit testing.
    fn is_hit_test_visible(&self) -> bool;

    /// Sets whether the popup takes part in pointer hit testing.
    fn set_is_hit_test_visible(&self, value: bool);

    /// Gets a transform that will be applied to the popup.
    fn transform(&self) -> Option<Ref<Transform>>;

    /// Sets a transform that will be applied to the popup.
    fn set_transform(&self, value: Option<Ref<Transform>>);

    /// Gets the root of the visual tree in the case where the popup is
    /// presented using a separate visual tree.
    fn hosted_visual_tree_root(&self) -> Option<Ref<Visual>>;

    /// Raised when the control's template is applied. Disposing the
    /// returned handle unsubscribes.
    fn template_applied(&self, handler: Rc<dyn Fn(&TemplateAppliedEventArgs)>) -> Rc<dyn IDisposable>;

    /// Configures the position of the popup according to a target control
    /// and a set of placement parameters.
    fn configure_position(&self, position_request: PopupPositionRequest);

    /// Sets the control to display in the popup.
    fn set_child(&self, control: Option<Ref<Control>>);

    /// Shows the popup.
    fn show(&self);

    /// Hides the popup.
    fn hide(&self);

    /// Takes focus from any currently focused native control.
    fn take_focus(&self);

    /// The control that is the host (the host cast to a control).
    fn as_control(&self) -> Ref<Control>;

    /// The host as a popup root, if it is one.
    fn as_popup_root(&self) -> Option<Ref<PopupRoot>> {
        None
    }

    /// The host as an overlay popup host, if it is one.
    fn as_overlay_popup_host(&self) -> Option<Ref<OverlayPopupHost>> {
        None
    }
}

/// Popup hosts compare by the identity of the control behind them.
impl PartialEq for dyn IPopupHost {
    fn eq(&self, other: &Self) -> bool {
        self.as_control() == other.as_control()
    }
}

/// Subscribes `handler` to the template-applied event of the host control
/// `control` and returns the handle that unsubscribes it.
pub(super) fn subscribe_template_applied(
    control: &Ref<Control>,
    handler: Rc<dyn Fn(&TemplateAppliedEventArgs)>,
) -> Rc<dyn IDisposable> {
    use super::TemplatedControl;
    use ferroui_base::reactive::Disposable;

    let token = control.add_handler(TemplatedControl::template_applied_event(), move |_, e| handler(e));
    let weak = control.downgrade();
    Disposable::create(move || {
        if let Some(control) = weak.upgrade() {
            control.remove_handler(TemplatedControl::template_applied_event(), token);
        }
    })
}

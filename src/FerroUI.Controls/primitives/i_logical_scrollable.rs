use crate::Control;
use ferroui_base::input::{IScrollable, NavigationDirection};
use ferroui_base::reactive::IDisposable;
use ferroui_base::{FerroObject, ObjectType, Rect, Ref, Size, TypeInfo};
use std::cell::RefCell;
use std::rc::Rc;

/// Interface implemented by controls that handle their own scrolling when
/// placed inside a `ScrollViewer`.
///
/// Controls that implement this interface, when placed inside a
/// `ScrollViewer`, can override the physical scrolling behavior of the
/// scroll viewer with logical scrolling. Physical scrolling means that the
/// scroll viewer is a simple viewport onto a larger canvas whereas logical
/// scrolling means that the scrolling is handled by the child control itself
/// and it can choose to do handle the scroll information as it sees fit.
///
/// A class implementing the trait is made known with
/// [`register_logical_scrollable`], normally from its class initialization;
/// [`as_logical_scrollable`] then views an object of the class (or of a
/// class derived from it) as the interface.
pub trait ILogicalScrollable: IScrollable {
    /// Sets a value indicating whether the content can be scrolled
    /// horizontally.
    fn set_can_horizontally_scroll(&self, value: bool);

    /// Sets a value indicating whether the content can be scrolled
    /// vertically.
    fn set_can_vertically_scroll(&self, value: bool);

    /// Gets a value indicating whether logical scrolling is enabled on the
    /// control.
    fn is_logical_scroll_enabled(&self) -> bool;

    /// Gets the size to scroll by, in logical units.
    fn scroll_size(&self) -> Size;

    /// Gets the size to page by, in logical units.
    fn page_scroll_size(&self) -> Size;

    /// Raised when the scroll is invalidated.
    ///
    /// This event notifies an attached `ScrollViewer` of a change in one of
    /// the scroll properties. Disposing the returned handle removes the
    /// handler.
    fn scroll_invalidated(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable>;

    /// Attempts to bring a portion of the target visual into view by
    /// scrolling the content.
    ///
    /// `target` is the target visual and `target_rect` the portion of the
    /// target visual to bring into view. Returns true if the scroll offset
    /// was changed; otherwise false.
    fn bring_into_view(&self, target: &Ref<Control>, target_rect: Rect) -> bool;

    /// Gets the next control in the specified direction.
    ///
    /// `direction` is the movement direction and `from` the control from
    /// which movement begins.
    fn get_control_in_direction(
        &self,
        direction: NavigationDirection,
        from: Option<&Ref<Control>>,
    ) -> Option<Ref<Control>>;

    /// Raises the [`scroll_invalidated`](Self::scroll_invalidated) event.
    fn raise_scroll_invalidated(&self);
}

type LogicalScrollableCast = for<'a> fn(&'a FerroObject) -> Option<&'a dyn ILogicalScrollable>;

thread_local! {
    static LOGICAL_SCROLLABLE_TYPES: RefCell<Vec<(&'static TypeInfo, LogicalScrollableCast)>> =
        const { RefCell::new(Vec::new()) };
}

fn cast_logical_scrollable<T: ObjectType + ILogicalScrollable>(
    object: &FerroObject,
) -> Option<&dyn ILogicalScrollable> {
    object
        .downcast_ref::<T>()
        .map(|scrollable| scrollable as &dyn ILogicalScrollable)
}

/// Declares that class `T` (and the classes derived from it) implements
/// [`ILogicalScrollable`].
pub fn register_logical_scrollable<T: ObjectType + ILogicalScrollable>() {
    LOGICAL_SCROLLABLE_TYPES.with(|types| {
        let mut types = types.borrow_mut();
        if !types.iter().any(|(type_, _)| *type_ == T::TYPE) {
            types.push((T::TYPE, cast_logical_scrollable::<T> as LogicalScrollableCast));
        }
    });
}

/// The object viewed as a logical scrollable, if its class implements
/// [`ILogicalScrollable`].
pub fn as_logical_scrollable(object: &FerroObject) -> Option<&dyn ILogicalScrollable> {
    let cast = LOGICAL_SCROLLABLE_TYPES.with(|types| {
        let types = types.borrow();
        let mut current = Some(object.get_type());
        while let Some(type_) = current {
            if let Some((_, cast)) = types.iter().find(|(candidate, _)| *candidate == type_) {
                return Some(*cast);
            }
            current = type_.base_type();
        }
        None
    });
    cast.and_then(|cast| cast(object))
}

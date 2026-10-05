use super::SnapPointsAlignment;
use ferroui_base::interactivity::{Interactive, RoutedEventArgs};
use ferroui_base::layout::Orientation;
use ferroui_base::reactive::IDisposable;
use ferroui_base::{FerroObject, ObjectType, TypeInfo};
use std::cell::RefCell;
use std::rc::Rc;

/// A handler of the snap points changed events of [`IScrollSnapPointsInfo`].
pub type SnapPointsChangedHandler = Rc<dyn Fn(&Interactive, &RoutedEventArgs)>;

/// Describes snap point behaviour for objects that contain and present items.
pub trait IScrollSnapPointsInfo {
    /// Whether the horizontal snap points for the container are equidistant
    /// from each other.
    fn are_horizontal_snap_points_regular(&self) -> bool;

    fn set_are_horizontal_snap_points_regular(&self, value: bool);

    /// Whether the vertical snap points for the container are equidistant
    /// from each other.
    fn are_vertical_snap_points_regular(&self) -> bool;

    fn set_are_vertical_snap_points_regular(&self, value: bool);

    /// Returns the set of distances between irregular snap points for a
    /// specified orientation and alignment.
    ///
    /// Returns the collection of snap point distances for the provided
    /// orientation and alignment; an empty collection when no snap points are
    /// present.
    fn get_irregular_snap_points(
        &self,
        orientation: Orientation,
        snap_points_alignment: SnapPointsAlignment,
    ) -> Vec<f64>;

    /// Gets the distance between regular snap points for a specified
    /// orientation and alignment.
    ///
    /// Returns the distance between the equidistant snap points (0 when no
    /// snap points are present) and the offset of the first snap point.
    fn get_regular_snap_points(
        &self,
        orientation: Orientation,
        snap_points_alignment: SnapPointsAlignment,
    ) -> (f64, f64);

    /// Occurs when the measurements for horizontal snap points change.
    ///
    /// Disposing the returned handle removes the handler.
    fn horizontal_snap_points_changed(&self, handler: SnapPointsChangedHandler) -> Rc<dyn IDisposable>;

    /// Occurs when the measurements for vertical snap points change.
    ///
    /// Disposing the returned handle removes the handler.
    fn vertical_snap_points_changed(&self, handler: SnapPointsChangedHandler) -> Rc<dyn IDisposable>;
}

type SnapPointsInfoCast = for<'a> fn(&'a FerroObject) -> Option<&'a dyn IScrollSnapPointsInfo>;

thread_local! {
    static SNAP_POINTS_INFO_TYPES: RefCell<Vec<(&'static TypeInfo, SnapPointsInfoCast)>> =
        const { RefCell::new(Vec::new()) };
}

fn cast_snap_points_info<T: ObjectType + IScrollSnapPointsInfo>(
    object: &FerroObject,
) -> Option<&dyn IScrollSnapPointsInfo> {
    object
        .downcast_ref::<T>()
        .map(|info| info as &dyn IScrollSnapPointsInfo)
}

/// Declares that class `T` (and the classes derived from it) implements
/// [`IScrollSnapPointsInfo`].
pub fn register_scroll_snap_points_info<T: ObjectType + IScrollSnapPointsInfo>() {
    SNAP_POINTS_INFO_TYPES.with(|types| {
        let mut types = types.borrow_mut();
        if !types.iter().any(|(type_, _)| *type_ == T::TYPE) {
            types.push((T::TYPE, cast_snap_points_info::<T> as SnapPointsInfoCast));
        }
    });
}

/// The object viewed as a provider of snap points, if its class implements
/// [`IScrollSnapPointsInfo`].
pub fn as_scroll_snap_points_info(object: &FerroObject) -> Option<&dyn IScrollSnapPointsInfo> {
    let cast = SNAP_POINTS_INFO_TYPES.with(|types| {
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

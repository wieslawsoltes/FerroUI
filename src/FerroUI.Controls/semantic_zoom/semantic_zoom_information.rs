//! WinUI's `ISemanticZoomInformation`, `SemanticZoomLocation` and
//! `SemanticZoomViewChangedEventArgs` (`dxaml/xcp/dxaml/lib/SemanticZoomLocation_Partial.h`,
//! `SemanticZoomViewChangedEventArgs` of `controls.idl` and the interface of
//! `ListViewBase_Partial_SemanticZoomInformation.cpp` of microsoft-ui-xaml, commit
//! `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c) Microsoft Corporation, see
//! NOTICE.md).

use super::SemanticZoom;
use ferroui_base::{ferro_markup_type, BoxedValue, FerroObject, ObjectType, Point, Rect, Ref, TypeInfo};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// A view of a [`SemanticZoom`] (WinUI's `ISemanticZoomInformation`): the zoomed-in view or the
/// zoomed-out view, which the semantic zoom tells about a change from one view to the other.
///
/// [`ListView`](crate::ListView) and [`GridView`](crate::GridView) implement it (through
/// `ListViewBase`). A class of an application implements the trait and makes itself known with
/// [`register_semantic_zoom_information`]; the semantic zoom then views its
/// [`zoomed_in_view`](SemanticZoom::zoomed_in_view) and
/// [`zoomed_out_view`](SemanticZoom::zoomed_out_view) as this interface with
/// [`as_semantic_zoom_information`].
///
/// The order of the calls of a change (WinUI's `ChangeViews`): `initialize_view_change` of both
/// views, `start_view_change_from` of the source and `start_view_change_to` of the destination,
/// the `ViewChangeStarted` event, `set_is_active_view`, `make_visible` of the destination,
/// `complete_view_change_from` and `complete_view_change_to`, and when the transition has ended
/// the `ViewChangeCompleted` event and `complete_view_change` of both views.
pub trait ISemanticZoomInformation {
    /// Prepares the view for a change (WinUI's `InitializeViewChange`).
    fn initialize_view_change(&self);

    /// Cleans the view up after a change (WinUI's `CompleteViewChange`).
    fn complete_view_change(&self);

    /// Scrolls the view so that the item of `item` comes to its bounds (WinUI's `MakeVisible`).
    fn make_visible(&self, item: &SemanticZoomLocation);

    /// The view is the active view and the other view becomes it: it gives the item the change
    /// starts from in `source` and, if it knows it, the item to go to in `destination` (WinUI's
    /// `StartViewChangeFrom`).
    fn start_view_change_from(&self, source: &SemanticZoomLocation, destination: &SemanticZoomLocation);

    /// The view becomes the active view (WinUI's `StartViewChangeTo`).
    fn start_view_change_to(&self, source: &SemanticZoomLocation, destination: &SemanticZoomLocation);

    /// The change from this view has completed (WinUI's `CompleteViewChangeFrom`).
    fn complete_view_change_from(&self, source: &SemanticZoomLocation, destination: &SemanticZoomLocation);

    /// The change to this view has completed (WinUI's `CompleteViewChangeTo`).
    fn complete_view_change_to(&self, source: &SemanticZoomLocation, destination: &SemanticZoomLocation);

    /// Whether the view is the one the semantic zoom shows (WinUI's `IsActiveView`).
    fn is_active_view(&self) -> bool;

    /// Sets [`is_active_view`](Self::is_active_view).
    fn set_is_active_view(&self, value: bool);

    /// Whether the view is the zoomed-in view of its semantic zoom (WinUI's `IsZoomedInView`).
    fn is_zoomed_in_view(&self) -> bool;

    /// Sets [`is_zoomed_in_view`](Self::is_zoomed_in_view).
    fn set_is_zoomed_in_view(&self, value: bool);

    /// The semantic zoom the view belongs to (WinUI's `SemanticZoomOwner`).
    fn semantic_zoom_owner(&self) -> Option<Ref<SemanticZoom>>;

    /// Sets [`semantic_zoom_owner`](Self::semantic_zoom_owner); the view keeps a weak reference.
    fn set_semantic_zoom_owner(&self, value: Option<&Ref<SemanticZoom>>);
}

type InformationCast = for<'a> fn(&'a FerroObject) -> Option<&'a dyn ISemanticZoomInformation>;

ferroui_base::ferro_thread_local! {
    static INFORMATION_TYPES: RefCell<Vec<(&'static TypeInfo, InformationCast)>> = const { RefCell::new(Vec::new()) };
}

fn cast_information<T: ObjectType + ISemanticZoomInformation>(
    object: &FerroObject,
) -> Option<&dyn ISemanticZoomInformation> {
    object.downcast_ref::<T>().map(|view| view as &dyn ISemanticZoomInformation)
}

/// Declares that class `T` (and the classes derived from it) implements
/// [`ISemanticZoomInformation`]. `ListViewBase` is registered by the class initialization of
/// [`SemanticZoom`].
pub fn register_semantic_zoom_information<T: ObjectType + ISemanticZoomInformation>() {
    INFORMATION_TYPES.with(|types| {
        let mut types = types.borrow_mut();
        if !types.iter().any(|(type_, _)| *type_ == T::TYPE) {
            types.push((T::TYPE, cast_information::<T> as InformationCast));
        }
    });
}

/// The object viewed as a view of a semantic zoom, if its class implements
/// [`ISemanticZoomInformation`].
pub fn as_semantic_zoom_information(object: &FerroObject) -> Option<&dyn ISemanticZoomInformation> {
    let cast = INFORMATION_TYPES.with(|types| {
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

#[derive(Default)]
struct LocationState {
    item: RefCell<Option<BoxedValue>>,
    bounds: Cell<Rect>,
    zoom_point: Cell<Point>,
    remainder: Cell<Rect>,
    is_bottom_alignment: Cell<bool>,
}

/// An item of a view of a semantic zoom and where it is (WinUI's `SemanticZoomLocation`). A
/// location is shared: its clones are the same location, as the views and the handlers of a
/// change see one object in WinUI.
#[derive(Clone, Default)]
pub struct SemanticZoomLocation(Rc<LocationState>);

impl SemanticZoomLocation {
    /// A location without an item, at the bounds `(0, 0, 0, 0)`.
    pub fn new() -> Self {
        Self::default()
    }

    /// The item (WinUI's `Item`).
    pub fn item(&self) -> Option<BoxedValue> {
        self.0.item.borrow().clone()
    }

    /// Sets [`item`](Self::item).
    pub fn set_item(&self, value: Option<BoxedValue>) {
        *self.0.item.borrow_mut() = value;
    }

    /// The bounds of the item in the coordinates of its view (WinUI's `Bounds`).
    pub fn bounds(&self) -> Rect {
        self.0.bounds.get()
    }

    /// Sets [`bounds`](Self::bounds).
    pub fn set_bounds(&self, value: Rect) {
        self.0.bounds.set(value);
    }

    /// The point a zoom gesture centred on, in the coordinates of the view; `(0, 0)` when the
    /// change has no point (WinUI's internal `ZoomPoint`).
    pub(crate) fn zoom_point(&self) -> Point {
        self.0.zoom_point.get()
    }

    pub(crate) fn set_zoom_point(&self, value: Point) {
        self.0.zoom_point.set(value);
    }

    /// The distance `make_visible` could not scroll (WinUI's internal `Remainder`; the correction
    /// transforms of a pinch that use it are not ported).
    #[cfg(test)]
    pub(crate) fn remainder(&self) -> Rect {
        self.0.remainder.get()
    }

    pub(crate) fn set_remainder(&self, value: Rect) {
        self.0.remainder.set(value);
    }

    /// Whether the destination container goes to the bottom of a zoomed-out list (WinUI's
    /// internal `IsBottomAlignment` of the jump list).
    pub(crate) fn is_bottom_alignment(&self) -> bool {
        self.0.is_bottom_alignment.get()
    }

    pub(crate) fn set_is_bottom_alignment(&self, value: bool) {
        self.0.is_bottom_alignment.set(value);
    }
}

impl PartialEq for SemanticZoomLocation {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl std::fmt::Debug for SemanticZoomLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SemanticZoomLocation").field("item", &self.item()).field("bounds", &self.bounds()).finish()
    }
}

ferro_markup_type!(class SemanticZoomLocation {
    namespace: "FerroUI.Controls",
    handles: [SemanticZoomLocation],
    properties: [
        Item: Option<BoxedValue> {
            get: |location: &SemanticZoomLocation| location.item(),
            set: |location: &SemanticZoomLocation, value: Option<BoxedValue>| location.set_item(value)
        },
        Bounds: Rect {
            get: |location: &SemanticZoomLocation| location.bounds(),
            set: |location: &SemanticZoomLocation, value: Rect| location.set_bounds(value)
        },
    ],
});

#[derive(Default)]
struct ArgsState {
    is_source_zoomed_in_view: Cell<bool>,
    source_item: RefCell<SemanticZoomLocation>,
    destination_item: RefCell<SemanticZoomLocation>,
}

/// The arguments of [`SemanticZoom::view_change_started`] and
/// [`SemanticZoom::view_change_completed`] (WinUI's `SemanticZoomViewChangedEventArgs`). The two
/// events of one change share the arguments: a handler of `ViewChangeStarted` that sets the
/// destination item changes the item the destination view goes to.
#[derive(Clone, Default)]
pub struct SemanticZoomViewChangedEventArgs(Rc<ArgsState>);

impl SemanticZoomViewChangedEventArgs {
    /// Arguments of a change from the zoomed-in view when `is_source_zoomed_in_view`.
    pub fn new(
        is_source_zoomed_in_view: bool,
        source_item: SemanticZoomLocation,
        destination_item: SemanticZoomLocation,
    ) -> Self {
        Self(Rc::new(ArgsState {
            is_source_zoomed_in_view: Cell::new(is_source_zoomed_in_view),
            source_item: RefCell::new(source_item),
            destination_item: RefCell::new(destination_item),
        }))
    }

    /// Whether the change starts from the zoomed-in view (WinUI's `IsSourceZoomedInView`).
    pub fn is_source_zoomed_in_view(&self) -> bool {
        self.0.is_source_zoomed_in_view.get()
    }

    /// Sets [`is_source_zoomed_in_view`](Self::is_source_zoomed_in_view).
    pub fn set_is_source_zoomed_in_view(&self, value: bool) {
        self.0.is_source_zoomed_in_view.set(value);
    }

    /// The item the change starts from (WinUI's `SourceItem`).
    pub fn source_item(&self) -> SemanticZoomLocation {
        self.0.source_item.borrow().clone()
    }

    /// Sets [`source_item`](Self::source_item).
    pub fn set_source_item(&self, value: SemanticZoomLocation) {
        *self.0.source_item.borrow_mut() = value;
    }

    /// The item the destination view goes to (WinUI's `DestinationItem`).
    pub fn destination_item(&self) -> SemanticZoomLocation {
        self.0.destination_item.borrow().clone()
    }

    /// Sets [`destination_item`](Self::destination_item).
    pub fn set_destination_item(&self, value: SemanticZoomLocation) {
        *self.0.destination_item.borrow_mut() = value;
    }
}

impl PartialEq for SemanticZoomViewChangedEventArgs {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl std::fmt::Debug for SemanticZoomViewChangedEventArgs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SemanticZoomViewChangedEventArgs")
            .field("is_source_zoomed_in_view", &self.is_source_zoomed_in_view())
            .field("source_item", &self.source_item())
            .field("destination_item", &self.destination_item())
            .finish()
    }
}

ferro_markup_type!(class SemanticZoomViewChangedEventArgs {
    namespace: "FerroUI.Controls",
    handles: [SemanticZoomViewChangedEventArgs],
    properties: [
        IsSourceZoomedInView: bool {
            get: |args: &SemanticZoomViewChangedEventArgs| args.is_source_zoomed_in_view(),
            set: |args: &SemanticZoomViewChangedEventArgs, value: bool| args.set_is_source_zoomed_in_view(value)
        },
        SourceItem: SemanticZoomLocation {
            get: |args: &SemanticZoomViewChangedEventArgs| args.source_item(),
            set: |args: &SemanticZoomViewChangedEventArgs, value: SemanticZoomLocation| args.set_source_item(value)
        },
        DestinationItem: SemanticZoomLocation {
            get: |args: &SemanticZoomViewChangedEventArgs| args.destination_item(),
            set: |args: &SemanticZoomViewChangedEventArgs, value: SemanticZoomLocation| {
                args.set_destination_item(value)
            }
        },
    ],
});

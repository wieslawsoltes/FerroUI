use super::raw::RawPointerPoint;
use super::{
    CaptureSource, IKeyModifiersEventArgs, IPointer, InputElement, KeyModifiers, Pointer, PointerPoint,
    PointerPointProperties,
};
use crate::interactivity::{RoutedEvent, RoutedEventArgs};
use crate::rendering::IPresentationSource;
use crate::{ferro_routed_event_args, FerroObject, Nullable, Point, Ref, Visual};
use std::any::Any;
use std::cell::LazyCell;
use std::rc::Rc;

/// The lazily computed intermediate points of a pointer move: the positions
/// the platform coalesced into one event.
pub type IntermediatePoints =
    Rc<LazyCell<Option<Vec<RawPointerPoint>>, Box<dyn FnOnce() -> Option<Vec<RawPointerPoint>>>>>;

/// Provides data for pointer events.
#[derive(Clone)]
pub struct PointerEventArgs {
    base: RoutedEventArgs,
    /// The original observer of the event.
    event_presentation_source: Option<Rc<dyn IPresentationSource>>,
    presentation_source_position: Point,
    properties: PointerPointProperties,
    previous_points: Option<IntermediatePoints>,
    pointer: Rc<dyn IPointer>,
    timestamp: u64,
    platform_input_event_cookie: Option<Rc<dyn Any>>,
    key_modifiers: KeyModifiers,
}

ferro_routed_event_args!(PointerEventArgs: RoutedEventArgs);

impl PointerEventArgs {
    /// Creates pointer event args.
    ///
    /// `root_visual` is the root of the tree that observed the event and
    /// `root_visual_position` the position of the pointer in its
    /// coordinates. `timestamp` is the platform time stamp of the input.
    #[allow(clippy::too_many_arguments)]
    pub fn new<T: ?Sized>(
        routed_event: Option<&RoutedEvent<T>>,
        source: impl Into<Nullable<FerroObject>>,
        pointer: Rc<dyn IPointer>,
        root_visual: Option<&Visual>,
        root_visual_position: Point,
        timestamp: u64,
        properties: PointerPointProperties,
        modifiers: KeyModifiers,
    ) -> Self {
        let base = RoutedEventArgs::new();
        base.set_routed_event(routed_event);
        base.set_source(source);

        Self {
            base,
            event_presentation_source: root_visual.and_then(Visual::presentation_source),
            presentation_source_position: root_visual_position,
            properties,
            previous_points: None,
            pointer,
            timestamp,
            platform_input_event_cookie: None,
            key_modifiers: modifiers,
        }
    }

    /// Adds the intermediate points of the event.
    pub fn with_previous_points(mut self, previous_points: Option<IntermediatePoints>) -> Self {
        self.previous_points = previous_points;
        self
    }

    /// Adds the platform's cookie for the input event.
    pub fn with_platform_input_event_cookie(mut self, cookie: Option<Rc<dyn Any>>) -> Self {
        self.platform_input_event_cookie = cookie;
        self
    }

    /// The pointer that caused the event.
    #[inline]
    pub fn pointer(&self) -> &Rc<dyn IPointer> {
        &self.pointer
    }

    /// The time when the pointer event occurred, as reported by the
    /// platform.
    #[inline]
    pub fn timestamp(&self) -> u64 {
        self.timestamp
    }

    /// The platform's opaque cookie for the input event that caused this
    /// event, if any.
    #[inline]
    pub fn platform_input_event_cookie(&self) -> Option<&Rc<dyn Any>> {
        self.platform_input_event_cookie.as_ref()
    }

    /// Whether gesture recognition is skipped for the pointer of the event.
    pub(crate) fn is_gesture_recognition_skipped(&self) -> bool {
        self.pointer.as_any().downcast_ref::<Pointer>().is_some_and(Pointer::is_gesture_recognition_skipped)
    }

    /// The key modifiers held down when the event occurred.
    #[inline]
    pub fn key_modifiers(&self) -> KeyModifiers {
        self.key_modifiers
    }

    fn get_position_of(&self, pt: Point, relative_to: Option<&Visual>) -> Point {
        let Some(relative_to) = relative_to else { return pt };

        let Some(event_source) = &self.event_presentation_source else { return Point::default() };
        let Some(target_source) = relative_to.presentation_source() else { return Point::default() };
        let Some(target_root) = target_source.root_visual() else { return Point::default() };

        let mut pt = pt;

        if !std::ptr::addr_eq(Rc::as_ptr(&target_source), Rc::as_ptr(event_source)) {
            match event_source.point_to_screen(pt).and_then(|screen_pt| target_source.point_to_client(screen_pt)) {
                Some(target_client_pt) => pt = target_client_pt,
                None => return Point::default(),
            }
        }

        target_root.translate_point(pt, relative_to).unwrap_or_default()
    }

    /// Gets the pointer position relative to a control, or relative to the
    /// root of the tree that observed the event when `relative_to` is
    /// `None`.
    pub fn get_position(&self, relative_to: Option<&Visual>) -> Point {
        self.get_position_of(self.presentation_source_position, relative_to)
    }

    /// Returns the pointer point associated with the current event,
    /// positioned relative to a control.
    pub fn get_current_point(&self, relative_to: Option<&Visual>) -> PointerPoint {
        PointerPoint::new(self.pointer.clone(), self.get_position(relative_to), self.properties)
    }

    /// Returns all points since the last relevant pointer event, positioned
    /// relative to a control. The last one is the current point.
    pub fn get_intermediate_points(&self, relative_to: Option<&Visual>) -> Vec<PointerPoint> {
        let previous_points = match &self.previous_points {
            Some(lazy) => match &***lazy {
                Some(points) if !points.is_empty() => points.as_slice(),
                _ => &[],
            },
            None => &[],
        };

        let mut points = Vec::with_capacity(previous_points.len() + 1);

        for pt in previous_points {
            let point_properties = PointerPointProperties::based_on(self.properties, *pt);
            points.push(PointerPoint::new(
                self.pointer.clone(),
                self.get_position_of(pt.position, relative_to),
                point_properties,
            ));
        }

        points.push(self.get_current_point(relative_to));
        points
    }

    /// Prevents this event from being handled by other gesture recognizers
    /// in the route.
    pub fn prevent_gesture_recognition(&self) {
        if let Some(pointer) = self.pointer.as_any().downcast_ref::<Pointer>() {
            pointer.set_is_gesture_recognition_skipped(true);
        }
    }

    /// The state of the pointer device at the time of the event.
    #[inline]
    pub fn properties(&self) -> PointerPointProperties {
        self.properties
    }
}

impl IKeyModifiersEventArgs for PointerEventArgs {
    fn key_modifiers(&self) -> KeyModifiers {
        self.key_modifiers
    }
}

/// Enumerates the mouse buttons.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum MouseButton {
    #[default]
    None,
    Left,
    Right,
    Middle,
    XButton1,
    XButton2,
}

/// Provides data for the pointer pressed event.
#[derive(Clone)]
pub struct PointerPressedEventArgs {
    base: PointerEventArgs,
    click_count: i32,
}

ferro_routed_event_args!(PointerPressedEventArgs: PointerEventArgs);

impl PointerPressedEventArgs {
    /// Creates pointer pressed event args.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        source: impl Into<Nullable<FerroObject>>,
        pointer: Rc<dyn IPointer>,
        root_visual: &Visual,
        root_visual_position: Point,
        timestamp: u64,
        properties: PointerPointProperties,
        modifiers: KeyModifiers,
        click_count: i32,
    ) -> Self {
        Self {
            base: PointerEventArgs::new(
                Some(InputElement::pointer_pressed_event()),
                source,
                pointer,
                Some(root_visual),
                root_visual_position,
                timestamp,
                properties,
                modifiers,
            ),
            click_count,
        }
    }

    /// Adds the platform's cookie for the input event.
    pub fn with_platform_input_event_cookie(mut self, cookie: Option<Rc<dyn Any>>) -> Self {
        self.base = self.base.with_platform_input_event_cookie(cookie);
        self
    }

    /// The number of clicks: 1 for a single click, 2 for a double click and
    /// so on.
    #[inline]
    pub fn click_count(&self) -> i32 {
        self.click_count
    }
}

/// Provides data for the pointer released event.
#[derive(Clone)]
pub struct PointerReleasedEventArgs {
    base: PointerEventArgs,
    initial_press_mouse_button: MouseButton,
}

ferro_routed_event_args!(PointerReleasedEventArgs: PointerEventArgs);

impl PointerReleasedEventArgs {
    /// Creates pointer released event args.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        source: impl Into<Nullable<FerroObject>>,
        pointer: Rc<dyn IPointer>,
        root_visual: &Visual,
        root_visual_position: Point,
        timestamp: u64,
        properties: PointerPointProperties,
        modifiers: KeyModifiers,
        initial_press_mouse_button: MouseButton,
    ) -> Self {
        Self {
            base: PointerEventArgs::new(
                Some(InputElement::pointer_released_event()),
                source,
                pointer,
                Some(root_visual),
                root_visual_position,
                timestamp,
                properties,
                modifiers,
            ),
            initial_press_mouse_button,
        }
    }

    /// The mouse button that triggered the corresponding pointer pressed
    /// event.
    #[inline]
    pub fn initial_press_mouse_button(&self) -> MouseButton {
        self.initial_press_mouse_button
    }
}

/// Provides data for the pointer capture lost event.
#[derive(Clone)]
pub struct PointerCaptureLostEventArgs {
    base: RoutedEventArgs,
    pointer: Rc<dyn IPointer>,
}

ferro_routed_event_args!(PointerCaptureLostEventArgs: RoutedEventArgs);

impl PointerCaptureLostEventArgs {
    /// Creates pointer capture lost event args.
    pub fn new(source: impl Into<Nullable<FerroObject>>, pointer: Rc<dyn IPointer>) -> Self {
        Self {
            base: RoutedEventArgs::with_event_and_source(InputElement::pointer_capture_lost_event(), source),
            pointer,
        }
    }

    /// The pointer whose capture was lost.
    #[inline]
    pub fn pointer(&self) -> &Rc<dyn IPointer> {
        &self.pointer
    }
}

/// Provides data for the pointer capture changing event: raised before the
/// capture of a pointer changes; marking it handled cancels the change.
#[derive(Clone)]
pub struct PointerCaptureChangingEventArgs {
    base: RoutedEventArgs,
    pointer: Rc<dyn IPointer>,
    capture_source: CaptureSource,
    new_value: Option<Ref<InputElement>>,
}

ferro_routed_event_args!(PointerCaptureChangingEventArgs: RoutedEventArgs);

impl PointerCaptureChangingEventArgs {
    pub(crate) fn new(
        source: impl Into<Nullable<FerroObject>>,
        pointer: Rc<dyn IPointer>,
        new_value: Option<Ref<InputElement>>,
        capture_source: CaptureSource,
    ) -> Self {
        Self {
            base: RoutedEventArgs::with_event_and_source(InputElement::pointer_capture_changing_event(), source),
            pointer,
            capture_source,
            new_value,
        }
    }

    /// The pointer whose capture is changing.
    #[inline]
    pub fn pointer(&self) -> &Rc<dyn IPointer> {
        &self.pointer
    }

    /// What requested the capture change.
    #[inline]
    pub fn capture_source(&self) -> CaptureSource {
        self.capture_source
    }

    /// The element that will capture the pointer.
    #[inline]
    pub fn new_value(&self) -> Option<&Ref<InputElement>> {
        self.new_value.as_ref()
    }
}

use super::{
    HoldingRoutedEventArgs, HoldingState, IPointer, InputElement, MouseButton, PointerEventArgs,
    PointerPressedEventArgs, PointerReleasedEventArgs, PointerType, TappedEventArgs,
};
use crate::interactivity::{IRoutedEventArgs, Interactive, RoutingStrategies};
use crate::threading::{DispatcherPriority, DispatcherTimer};
use crate::utilities::HandlerList;
use crate::{FerroObject, Point, Rect, Ref, Size, Thickness, Visual, WeakRef};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[derive(Clone, Copy, PartialEq, Eq)]
enum GestureStateType {
    Pending,
    Holding,
    DoubleTapped,
}

#[derive(Clone)]
struct GestureState {
    type_: GestureStateType,
    pointer: Rc<dyn IPointer>,
}

type HoldingHandler = dyn Fn(&Interactive, &HoldingRoutedEventArgs);
type TappedHandler = dyn Fn(&Interactive, &TappedEventArgs);

struct GesturesData {
    // These events are only used internally and not propagated through a
    // route. They have routed event args as their target type because their
    // args are generated from pointer event args.
    holding: HandlerList<HoldingHandler>,
    tapped: HandlerList<TappedHandler>,
    right_tapped: HandlerList<TappedHandler>,
    double_tapped: HandlerList<TappedHandler>,

    gesture_state: RefCell<Option<GestureState>>,
    last_press: RefCell<Option<WeakRef<FerroObject>>>,
    last_press_point: Cell<Point>,
    hold_cancellation_token: RefCell<Option<Rc<Cell<bool>>>>,
}

thread_local! {
    static DATA: &'static GesturesData = Box::leak(Box::new(GesturesData {
        holding: HandlerList::new(),
        tapped: HandlerList::new(),
        right_tapped: HandlerList::new(),
        double_tapped: HandlerList::new(),
        gesture_state: RefCell::new(None),
        last_press: RefCell::new(None),
        last_press_point: Cell::new(Point::new(0.0, 0.0)),
        hold_cancellation_token: RefCell::new(None),
    }));
}

fn data() -> &'static GesturesData {
    DATA.with(|data| *data)
}

/// Synthesizes the tapped, double-tapped, right-tapped and holding gestures
/// from pointer events.
pub struct Gestures;

crate::ferro_static_type!(Gestures);

impl Gestures {
    /// Subscribes the gesture recognition to the pointer events.
    fn static_constructor() {
        InputElement::pointer_pressed_event().route_finished().subscribe(Self::pointer_pressed);
        InputElement::pointer_released_event().route_finished().subscribe(Self::pointer_released);
        InputElement::pointer_moved_event().route_finished().subscribe(Self::pointer_moved);
    }

    pub(crate) fn add_holding(handler: impl Fn(&Interactive, &HoldingRoutedEventArgs) + 'static) {
        data().holding.add(Rc::new(handler));
    }

    pub(crate) fn add_tapped(handler: impl Fn(&Interactive, &TappedEventArgs) + 'static) {
        data().tapped.add(Rc::new(handler));
    }

    pub(crate) fn add_right_tapped(handler: impl Fn(&Interactive, &TappedEventArgs) + 'static) {
        data().right_tapped.add(Rc::new(handler));
    }

    pub(crate) fn add_double_tapped(handler: impl Fn(&Interactive, &TappedEventArgs) + 'static) {
        data().double_tapped.add(Rc::new(handler));
    }

    fn invoke_holding(sender: &Interactive, e: &HoldingRoutedEventArgs) {
        for (_, handler) in data().holding.snapshot().iter() {
            handler(sender, e);
        }
    }

    fn invoke_tapped(list: &HandlerList<TappedHandler>, sender: &Interactive, e: &TappedEventArgs) {
        for (_, handler) in list.snapshot().iter() {
            handler(sender, e);
        }
    }

    fn gesture_state() -> Option<GestureState> {
        data().gesture_state.borrow().clone()
    }

    fn set_gesture_state(value: Option<GestureState>) {
        let old = data().gesture_state.replace(value);
        drop(old);
    }

    fn last_press() -> Option<Ref<FerroObject>> {
        data().last_press.borrow().as_ref().and_then(WeakRef::upgrade)
    }

    fn cancel_hold() {
        let token = data().hold_cancellation_token.borrow_mut().take();
        if let Some(token) = token {
            token.set(true);
        }
    }

    fn get_captured(args: &dyn IRoutedEventArgs) -> Option<Ref<FerroObject>> {
        let pointer_event_args = args.downcast_ref::<PointerEventArgs>()?;

        match pointer_event_args.pointer().captured() {
            Some(captured) => Some(captured.upcast()),
            None => pointer_event_args.source(),
        }
    }

    fn rect_around(point: Point, size: Size) -> Rect {
        Rect::from_position_size(point, Size::default())
            .inflate_thickness(Thickness::symmetric(size.width, size.height))
    }

    fn pointer_pressed(ev: &dyn IRoutedEventArgs) {
        let Some(source) = Self::get_captured(ev) else { return };

        if ev.route() != RoutingStrategies::BUBBLE {
            return;
        }

        let Some(e) = ev.downcast_ref::<PointerPressedEventArgs>() else { return };
        let Ok(visual) = source.clone().downcast::<Visual>() else { return };
        let interactive = source.cast::<Interactive>();
        let data = data();

        if let Some(state) = Self::gesture_state() {
            if state.type_ == GestureStateType::Holding {
                if let Some(i) = &interactive {
                    Self::invoke_holding(
                        i,
                        &HoldingRoutedEventArgs::new(
                            HoldingState::Canceled,
                            data.last_press_point.get(),
                            state.pointer.type_(),
                            e,
                        ),
                    );
                }
            }

            Self::cancel_hold();
            Self::set_gesture_state(None);
        }

        if e.click_count() % 2 == 1 {
            Self::set_gesture_state(Some(GestureState { type_: GestureStateType::Pending, pointer: e.pointer().clone() }));
            drop(data.last_press.replace(Some(source.downgrade())));
            data.last_press_point.set(e.get_position(Some(&visual)));

            let token = Rc::new(Cell::new(false));
            drop(data.hold_cancellation_token.replace(Some(token.clone())));

            if let Some(settings) = visual.get_platform_settings() {
                let e = e.clone();
                let source = source.clone();

                DispatcherTimer::run_once(
                    move || {
                        let Some(state) = Self::gesture_state() else { return };
                        if token.get() {
                            return;
                        }
                        let Some(i) = source.cast::<InputElement>() else { return };

                        if InputElement::get_is_holding_enabled(&i)
                            && (e.pointer().type_() != PointerType::Mouse
                                || InputElement::get_is_hold_with_mouse_enabled(&i))
                        {
                            let pointer_type = state.pointer.type_();
                            Self::set_gesture_state(Some(GestureState {
                                type_: GestureStateType::Holding,
                                pointer: state.pointer,
                            }));
                            Self::invoke_holding(
                                &i,
                                &HoldingRoutedEventArgs::new(
                                    HoldingState::Started,
                                    self::data().last_press_point.get(),
                                    pointer_type,
                                    &e,
                                ),
                            );
                        }
                    },
                    settings.hold_wait_duration(),
                    DispatcherPriority::default(),
                );
            }
        } else if e.click_count() % 2 == 0 && e.get_current_point(Some(&visual)).properties.is_left_button_pressed {
            if let Some(i) = &interactive {
                if Self::last_press().is_some_and(|target| target == source) {
                    Self::set_gesture_state(Some(GestureState {
                        type_: GestureStateType::DoubleTapped,
                        pointer: e.pointer().clone(),
                    }));
                    Self::invoke_tapped(
                        &data.double_tapped,
                        i,
                        &TappedEventArgs::new(Some(InputElement::double_tapped_event()), e),
                    );
                }
            }
        }
    }

    fn pointer_released(ev: &dyn IRoutedEventArgs) {
        if ev.route() != RoutingStrategies::BUBBLE {
            return;
        }

        let Some(e) = ev.downcast_ref::<PointerReleasedEventArgs>() else { return };
        let source = Self::get_captured(ev);
        let data = data();

        if let (Some(target), Some(source)) = (Self::last_press(), &source) {
            let button = e.initial_press_mouse_button();

            if target == *source && matches!(button, MouseButton::Left | MouseButton::Right) {
                if let (Some(i), Some(target_visual)) = (source.cast::<Interactive>(), target.cast::<Visual>()) {
                    let point = e.get_current_point(Some(&target_visual));
                    let tap_size = i
                        .get_platform_settings()
                        .map(|settings| settings.get_tap_size(point.pointer.type_()))
                        .unwrap_or(Size::new(4.0, 4.0));
                    let tap_rect = Self::rect_around(data.last_press_point.get(), tap_size);

                    if tap_rect.contains_exclusive(point.position) {
                        let state = Self::gesture_state();

                        if let Some(holding) = state.as_ref().filter(|s| s.type_ == GestureStateType::Holding) {
                            Self::invoke_holding(
                                &i,
                                &HoldingRoutedEventArgs::new(
                                    HoldingState::Completed,
                                    data.last_press_point.get(),
                                    holding.pointer.type_(),
                                    e,
                                ),
                            );
                            Self::invoke_tapped(
                                &data.right_tapped,
                                &i,
                                &TappedEventArgs::new(Some(InputElement::right_tapped_event()), e),
                            );
                        } else if button == MouseButton::Right {
                            Self::invoke_tapped(
                                &data.right_tapped,
                                &i,
                                &TappedEventArgs::new(Some(InputElement::right_tapped_event()), e),
                            );
                        }
                        // The double-tapped state is needed here to prevent
                        // invoking the tapped event when double-tapped is
                        // called.
                        else if state.as_ref().map(|s| s.type_) != Some(GestureStateType::DoubleTapped) {
                            Self::invoke_tapped(
                                &data.tapped,
                                &i,
                                &TappedEventArgs::new(Some(InputElement::tapped_event()), e),
                            );
                        }
                    }

                    Self::set_gesture_state(None);
                }
            }
        }

        Self::cancel_hold();
    }

    fn pointer_moved(ev: &dyn IRoutedEventArgs) {
        if ev.route() != RoutingStrategies::BUBBLE {
            return;
        }

        let Some(e) = ev.downcast_ref::<PointerEventArgs>() else { return };
        let source = Self::get_captured(ev);

        let Some(target) = Self::last_press() else { return };
        let Some(state) = Self::gesture_state() else { return };

        if !std::ptr::addr_eq(Rc::as_ptr(e.pointer()), Rc::as_ptr(&state.pointer)) {
            return;
        }

        let Some(i) = source.and_then(|source| source.downcast::<Interactive>().ok()) else { return };
        let Some(target_visual) = target.cast::<Visual>() else { return };

        let point = e.get_current_point(Some(&target_visual));
        let last_press_point = data().last_press_point.get();
        let hold_rect = Self::rect_around(last_press_point, Size::new(4.0, 4.0));

        if hold_rect.contains_exclusive(point.position) {
            return;
        }

        if state.type_ == GestureStateType::Holding {
            Self::invoke_holding(
                &i,
                &HoldingRoutedEventArgs::new(HoldingState::Canceled, last_press_point, state.pointer.type_(), e),
            );
        }

        Self::cancel_hold();
        Self::set_gesture_state(None);
    }
}

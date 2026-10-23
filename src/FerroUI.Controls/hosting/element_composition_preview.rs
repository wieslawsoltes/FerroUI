//! Port of `ElementCompositionPreview` (`Microsoft.UI.Xaml.Hosting`), from
//! `dxaml/xcp/dxaml/lib/ElementCompositionPreview.cpp` of microsoft-ui-xaml.
//!
//! The members that concern any visual are those of
//! [`ElementComposition`](ferroui_base::rendering::composition::ElementComposition) in the base
//! crate; this facade adds the WinUI names and the members that need controls (the scroll
//! viewer).

use crate::{ScrollViewer, TopLevel};
use ferroui_base::input::{InputElement, PointerEventArgs};
use ferroui_base::interactivity::{Interactive, RoutedEventHandlerToken, RoutingStrategies};
use ferroui_base::numerics::{Matrix4x4, Vector3};
use ferroui_base::reactive::IDisposable;
use ferroui_base::rendering::composition::animations::ICompositionAnimationBase;
use ferroui_base::rendering::composition::{CompositionPropertySet, CompositionVisual, Compositor, ElementComposition};
use ferroui_base::{Vector, Visual, WeakRef};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

ferroui_base::ferro_thread_local! {
    /// The manipulation property set of each scroll viewer
    /// (`CScrollViewer::EnsureManipulationTransformPropertySet` keeps one per scroll viewer).
    static MANIPULATION_SETS: RefCell<Vec<(WeakRef<ScrollViewer>, Rc<CompositionPropertySet>, Rc<dyn IDisposable>)>> =
        const { RefCell::new(Vec::new()) };
}

/// Access to the composition visuals of elements and to the composition-driven behaviours of
/// elements (`Microsoft.UI.Xaml.Hosting.ElementCompositionPreview`).
pub struct ElementCompositionPreview;

impl ElementCompositionPreview {
    /// `GetElementVisual`: the composition visual of `element` (WinUI's handoff visual). The
    /// same visual is returned while the element stays in the tree; `None` while the element is
    /// not in a tree rendered by a compositor.
    ///
    /// The renderer writes the `Offset`, `Size`, `Opacity`, `IsVisible`, clip and
    /// `TransformMatrix` of this visual when the element is rendered again, as WinUI's layout
    /// writes the handoff visual; `Translation`, `Scale`, `CenterPoint`, `RotationAngle`,
    /// `Orientation` and `AnchorPoint` belong to the application.
    pub fn get_element_visual(element: &Visual) -> Option<Rc<CompositionVisual>> {
        ElementComposition::get_element_visual(element)
    }

    /// `GetElementChildVisual`: the visual set with
    /// [`set_element_child_visual`](Self::set_element_child_visual) (WinUI's hand-in visual).
    pub fn get_element_child_visual(element: &Visual) -> Option<Rc<CompositionVisual>> {
        ElementComposition::get_element_child_visual(element)
    }

    /// `SetElementChildVisual`: shows `visual` as the last child of the composition visual of
    /// `element`, above the element's content and children. `None` removes it.
    pub fn set_element_child_visual(element: &Visual, visual: Option<Rc<CompositionVisual>>) {
        ElementComposition::set_element_child_visual(element, visual)
    }

    /// `SetImplicitShowAnimation`: the animation (or animation group) started on the visual of
    /// `element` when the element is added to the tree or its `IsVisible` becomes `true`.
    pub fn set_implicit_show_animation(element: &Visual, animation: Option<Rc<dyn ICompositionAnimationBase>>) {
        ElementComposition::set_implicit_show_animation(element, animation)
    }

    /// `SetImplicitHideAnimation`: the animation (or animation group) started on the visual of
    /// `element` when the element is removed from the tree or its `IsVisible` becomes `false`.
    /// The element stays visible until the animation completes.
    pub fn set_implicit_hide_animation(element: &Visual, animation: Option<Rc<dyn ICompositionAnimationBase>>) {
        ElementComposition::set_implicit_hide_animation(element, animation)
    }

    /// `SetIsTranslationEnabled`.
    ///
    /// In WinUI the handoff visual gains a `Translation` entry in its property set, which the
    /// framework then folds into the transform of the element. Here every composition visual has
    /// a `Translation` property, applied after its other transforms, so translation is always
    /// enabled; the call records the request and changes nothing else.
    pub fn set_is_translation_enabled(element: &Visual, value: bool) {
        ElementComposition::set_is_translation_enabled(element, value)
    }

    /// `GetScrollViewerManipulationPropertySet`: a property set that follows the scrolling of
    /// `scroll_viewer`, with the entries `Translation` (the negated offset, as a `Vector3`),
    /// `Pan` (the same), `Scale` (`(1, 1, 1)`, FerroUI's scroll viewer does not zoom),
    /// `CenterPoint` (zero) and `Matrix` (the translation as a `Matrix4x4`). The same set is
    /// returned for the same scroll viewer.
    ///
    /// Typical use: an expression animation of a header's `Offset` or `Translation` that
    /// reads `scroller.Translation.Y` for a parallax effect.
    ///
    /// In WinUI the compositor updates the set during a manipulation without the UI thread;
    /// here the set is updated when the offset of the scroll viewer changes, on the UI thread,
    /// in the same frame as the content moves. `None` when no compositor is available.
    pub fn get_scroll_viewer_manipulation_property_set(scroll_viewer: &ScrollViewer) -> Option<Rc<CompositionPropertySet>> {
        let existing = MANIPULATION_SETS.with(|sets| {
            let mut sets = sets.borrow_mut();
            sets.retain(|(key, _, subscription)| {
                let alive = key.upgrade().is_some();
                if !alive {
                    subscription.dispose();
                }
                alive
            });
            sets.iter()
                .find(|(key, _, _)| key.upgrade().is_some_and(|key| std::ptr::eq(&*key, scroll_viewer)))
                .map(|(_, set, _)| set.clone())
        });
        if existing.is_some() {
            return existing;
        }

        let compositor = compositor_for(scroll_viewer)?;
        let set = CompositionPropertySet::new(&compositor);
        write_manipulation(&set, scroll_viewer.offset());
        let weak_set = Rc::downgrade(&set);
        let subscription = scroll_viewer.property_changed(move |e| {
            if e.property() != ScrollViewer::offset_property().as_property() {
                return;
            }
            let (Some(set), Some(scroll_viewer)) = (weak_set.upgrade(), e.sender().downcast_ref::<ScrollViewer>())
            else {
                return;
            };
            write_manipulation(&set, scroll_viewer.offset());
        });
        MANIPULATION_SETS.with(|sets| {
            sets.borrow_mut().push((scroll_viewer.to_ref().downgrade(), set.clone(), subscription));
        });
        Some(set)
    }

    /// `GetPointerPositionPropertySet`: a property set with a `Position` entry (a `Vector3`)
    /// that follows the pointer over the window of `element`, in the coordinates of `element`.
    /// A new set is returned on each call, as in WinUI.
    ///
    /// WinUI drives the set from the compositor's hover input; here it is updated by the moves
    /// of the pointer that the window of the element receives, handled or not, on the UI thread.
    /// The set follows the element into another window. `None` when no compositor is
    /// available.
    pub fn get_pointer_position_property_set(element: &Visual) -> Option<Rc<CompositionPropertySet>> {
        let compositor = compositor_for(element)?;
        let set = CompositionPropertySet::new(&compositor);
        set.insert_vector3("Position", Vector3::new(0.0, 0.0, 0.0));
        PointerTracker::start(element, &set);
        Some(set)
    }
}

/// The compositor of `visual`: that of its composition visual, else the default one.
fn compositor_for(visual: &Visual) -> Option<Rc<Compositor>> {
    visual
        .composition_visual()
        .map(|composition_visual| composition_visual.compositor().clone())
        .or_else(Compositor::try_get_default_compositor)
}

fn write_manipulation(set: &CompositionPropertySet, offset: Vector) {
    let translation = Vector3::new(-offset.x as f32, -offset.y as f32, 0.0);
    set.insert_vector3("Translation", translation);
    set.insert_vector3("Pan", translation);
    set.insert_vector3("Scale", Vector3::new(1.0, 1.0, 1.0));
    set.insert_vector3("CenterPoint", Vector3::new(0.0, 0.0, 0.0));
    set.insert_matrix4x4("Matrix", Matrix4x4::create_translation(translation));
}

/// Writes the position of the pointer into a pointer position property set while the set is
/// alive: it follows the window the element is attached to.
struct PointerTracker {
    element: WeakRef<Visual>,
    set: Weak<CompositionPropertySet>,
    /// The window and the handler on it.
    hooked: RefCell<Option<(WeakRef<TopLevel>, RoutedEventHandlerToken)>>,
    subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
}

impl PointerTracker {
    fn start(element: &Visual, set: &Rc<CompositionPropertySet>) {
        let tracker = Rc::new(PointerTracker {
            element: element.to_ref().downgrade(),
            set: Rc::downgrade(set),
            hooked: RefCell::new(None),
            subscriptions: RefCell::new(Vec::new()),
        });
        let attached = {
            let tracker = tracker.clone();
            element.attached_to_visual_tree(move |_| tracker.hook())
        };
        let detached = {
            let tracker = tracker.clone();
            element.detached_from_visual_tree(move |_| tracker.unhook())
        };
        tracker.subscriptions.borrow_mut().extend([attached, detached]);
        tracker.hook();
    }

    /// Releases the tracker once its set is gone: the handlers hold the tracker.
    fn release_if_unused(&self) -> bool {
        if self.set.strong_count() > 0 && self.element.upgrade().is_some() {
            return false;
        }
        self.unhook();
        for subscription in self.subscriptions.borrow_mut().drain(..) {
            subscription.dispose();
        }
        true
    }

    fn hook(self: &Rc<Self>) {
        self.unhook();
        if self.release_if_unused() {
            return;
        }
        let Some(element) = self.element.upgrade() else { return };
        let Some(top_level) = TopLevel::get_top_level(Some(&element)) else { return };
        let weak = Rc::downgrade(self);
        let token = top_level.add_handler_with(
            &InputElement::pointer_moved_event(),
            move |_: &Interactive, e: &PointerEventArgs| {
                if let Some(tracker) = weak.upgrade() {
                    tracker.on_pointer_moved(e);
                }
            },
            RoutingStrategies::TUNNEL,
            true,
        );
        *self.hooked.borrow_mut() = Some((top_level.downgrade(), token));
    }

    fn unhook(&self) {
        let hooked = self.hooked.borrow_mut().take();
        if let Some((top_level, token)) = hooked {
            if let Some(top_level) = top_level.upgrade() {
                top_level.remove_handler(&InputElement::pointer_moved_event(), token);
            }
        }
    }

    fn on_pointer_moved(&self, e: &PointerEventArgs) {
        if self.release_if_unused() {
            return;
        }
        let (Some(set), Some(element)) = (self.set.upgrade(), self.element.upgrade()) else { return };
        let position = e.get_position(Some(&element));
        set.insert_vector3("Position", Vector3::new(position.x as f32, position.y as f32, 0.0));
    }
}

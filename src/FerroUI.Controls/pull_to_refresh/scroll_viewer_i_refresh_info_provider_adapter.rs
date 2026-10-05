use super::{RefreshInfoProvider, RefreshVisualizer, ScrollablePullGestureRecognizer};
use crate::{Control, ScrollViewer};
use ferroui_base::input::{InputElement, PullDirection};
use ferroui_base::interactivity::RoutedEventHandlerToken;
use ferroui_base::rendering::composition::ElementComposition;
use ferroui_base::{Ref, Size, Visual, WeakRef};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

const MAX_SEARCH_DEPTH: i32 = 10;
const INITIAL_OFFSET_THRESHOLD: f64 = 1.0;

/// Adapts a scroll viewer to a [`RefreshInfoProvider`]: turns the pull
/// gestures on the scrolled content into the interaction state a refresh
/// visualizer follows.
pub struct ScrollViewerIRefreshInfoProviderAdapter {
    this: Weak<ScrollViewerIRefreshInfoProviderAdapter>,
    refresh_pull_direction: Cell<PullDirection>,
    is_mouse_enabled: Cell<bool>,
    scroll_viewer: RefCell<Option<Ref<ScrollViewer>>>,
    refresh_info_provider: RefCell<Option<Ref<RefreshInfoProvider>>>,
    pull_gesture_recognizer: RefCell<Option<Ref<ScrollablePullGestureRecognizer>>>,
    interaction_source: RefCell<Option<Ref<InputElement>>>,
    is_visualizer_interaction_source_attached: Cell<bool>,
    /// The subscriptions of the provider to the pull gesture events of the
    /// interaction source.
    pull_gesture_handlers: RefCell<Vec<(RoutedEventHandlerToken, RoutedEventHandlerToken)>>,
    /// The subscriptions to the pointer pressed, pointer released and scroll
    /// changed events of the scroll viewer.
    scroll_viewer_handlers: RefCell<Vec<[RoutedEventHandlerToken; 3]>>,
    /// The subscriptions to the loaded event, with the scroll viewer each
    /// was made on.
    loaded_handlers: RefCell<Vec<(WeakRef<ScrollViewer>, RoutedEventHandlerToken)>>,
}

/// An adapter is a reference object: it compares by identity.
impl PartialEq for ScrollViewerIRefreshInfoProviderAdapter {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl ScrollViewerIRefreshInfoProviderAdapter {
    pub fn new(pull_direction: PullDirection, is_mouse_enabled: bool) -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            refresh_pull_direction: Cell::new(pull_direction),
            is_mouse_enabled: Cell::new(is_mouse_enabled),
            scroll_viewer: RefCell::new(None),
            refresh_info_provider: RefCell::new(None),
            pull_gesture_recognizer: RefCell::new(None),
            interaction_source: RefCell::new(None),
            is_visualizer_interaction_source_attached: Cell::new(false),
            pull_gesture_handlers: RefCell::new(Vec::new()),
            scroll_viewer_handlers: RefCell::new(Vec::new()),
            loaded_handlers: RefCell::new(Vec::new()),
        })
    }

    // Read by the tests only, as upstream.
    #[allow(dead_code)]
    pub(crate) fn interaction_source(&self) -> Option<Ref<InputElement>> {
        self.interaction_source.borrow().clone()
    }

    pub fn adapt_from_tree(&self, root: &Visual, refresh_visualizer_size: Option<Size>) -> Option<Ref<RefreshInfoProvider>> {
        fn adapt_from_tree_recursive_helper(root: &Visual, depth: i32) -> Option<Ref<ScrollViewer>> {
            let children = root.visual_children_snapshot()?;
            if depth == 0 {
                for child in children.iter() {
                    if let Some(viewer) = child.clone().cast::<ScrollViewer>() {
                        return Some(viewer);
                    }
                }
            } else {
                for child in children.iter() {
                    let viewer = adapt_from_tree_recursive_helper(child, depth - 1);
                    if viewer.is_some() {
                        return viewer;
                    }
                }
            }

            None
        }

        if let Some(scroll_viewer) = root.to_ref().cast::<ScrollViewer>() {
            return Some(self.adapt(&scroll_viewer, refresh_visualizer_size));
        }

        let mut depth = 0;
        while depth < MAX_SEARCH_DEPTH {
            let scroll = adapt_from_tree_recursive_helper(root, depth);

            if let Some(scroll) = scroll {
                return Some(self.adapt(&scroll, refresh_visualizer_size));
            }

            depth += 1;
        }

        None
    }

    pub fn adapt(&self, adaptee: &Ref<ScrollViewer>, refresh_visualizer_size: Option<Size>) -> Ref<RefreshInfoProvider> {
        if self.scroll_viewer.borrow().is_some() {
            self.clean_up_scroll_viewer();
        }

        let previous_interaction_source = self.interaction_source.borrow().clone();

        if let Some(interaction_source) = &previous_interaction_source {
            if self.refresh_info_provider.borrow().is_some() {
                self.remove_pull_gesture_handlers(interaction_source);
            }
        }

        // Remove the previous pull gesture recognizer from the previous
        // interaction source, otherwise repeated `adapt` calls (e.g. when the
        // visual tree gets re-templated) accumulate recognizers, leading to
        // duplicate pull gesture and pull gesture ended events.
        let previous_recognizer = self.pull_gesture_recognizer.borrow().clone();
        if let (Some(recognizer), Some(interaction_source)) = (&previous_recognizer, &previous_interaction_source) {
            interaction_source.gesture_recognizers().remove(recognizer);
        }

        drop(self.pull_gesture_recognizer.replace(None));
        drop(self.interaction_source.replace(None));
        self.is_visualizer_interaction_source_attached.set(false);

        drop(self.refresh_info_provider.replace(None));
        drop(self.scroll_viewer.replace(Some(adaptee.clone())));

        let Some(content) = adaptee.content() else {
            panic!("Adaptee's content property cannot be null.");
        };

        let Some(content) = Control::from_boxed(&content) else {
            panic!("Adaptee's content property must be a Visual");
        };

        if content.get_visual_parent().is_none() {
            let weak = self.this.clone();
            let token = adaptee.loaded(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    this.scroll_viewer_loaded();
                }
            });
            self.loaded_handlers.borrow_mut().push((adaptee.downgrade(), token));
        } else {
            self.scroll_viewer_loaded();

            if content.parent().and_then(|parent| parent.cast::<InputElement>()).is_none() {
                panic!("Adaptee's content's parent must be a InputElement");
            }
        }

        let refresh_info_provider = RefreshInfoProvider::new(
            self.refresh_pull_direction.get(),
            refresh_visualizer_size,
            ElementComposition::get_element_visual(&content),
        );
        drop(self.refresh_info_provider.replace(Some(refresh_info_provider.clone())));

        let pull_gesture_recognizer =
            ScrollablePullGestureRecognizer::with_direction(self.refresh_pull_direction.get(), self.is_mouse_enabled.get());
        drop(self.pull_gesture_recognizer.replace(Some(pull_gesture_recognizer.clone())));

        let interaction_source = self.interaction_source.borrow().clone();
        if let Some(interaction_source) = interaction_source {
            interaction_source.gesture_recognizers().add(pull_gesture_recognizer);
            self.add_pull_gesture_handlers(&interaction_source, &refresh_info_provider);
            self.is_visualizer_interaction_source_attached.set(true);
        }

        let pointer_pressed = {
            let weak = self.this.clone();
            adaptee.add_handler(InputElement::pointer_pressed_event(), move |_, _| {
                if let Some(this) = weak.upgrade() {
                    this.scroll_viewer_pointer_pressed();
                }
            })
        };
        let pointer_released = {
            let weak = self.this.clone();
            adaptee.add_handler(InputElement::pointer_released_event(), move |_, _| {
                if let Some(this) = weak.upgrade() {
                    this.scroll_viewer_pointer_released();
                }
            })
        };
        let scroll_changed = {
            let weak = self.this.clone();
            adaptee.scroll_changed(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    this.scroll_viewer_scroll_changed();
                }
            })
        };
        self.scroll_viewer_handlers.borrow_mut().push([pointer_pressed, pointer_released, scroll_changed]);

        refresh_info_provider
    }

    /// Subscribes the provider to the pull gesture events of the
    /// interaction source.
    fn add_pull_gesture_handlers(&self, interaction_source: &InputElement, provider: &Ref<RefreshInfoProvider>) {
        let entered = {
            let provider = provider.clone();
            interaction_source
                .add_handler(InputElement::pull_gesture_event(), move |_, e| provider.interacting_state_entered(e))
        };
        let exited = {
            let provider = provider.clone();
            interaction_source
                .add_handler(InputElement::pull_gesture_ended_event(), move |_, e| provider.interacting_state_exited(e))
        };
        self.pull_gesture_handlers.borrow_mut().push((entered, exited));
    }

    /// Removes one subscription of the provider from the pull gesture
    /// events of the interaction source.
    fn remove_pull_gesture_handlers(&self, interaction_source: &InputElement) {
        let handlers = self.pull_gesture_handlers.borrow_mut().pop();
        if let Some((entered, exited)) = handlers {
            interaction_source.remove_handler(InputElement::pull_gesture_event(), entered);
            interaction_source.remove_handler(InputElement::pull_gesture_ended_event(), exited);
        }
    }

    fn scroll_viewer_scroll_changed(&self) {
        let provider = self.refresh_info_provider.borrow().clone();
        if let Some(provider) = provider {
            if self.is_visualizer_interaction_source_attached.get()
                && provider.is_interacting_for_refresh()
                && !self.is_within_offset_threshold()
            {
                provider.set_is_interacting_for_refresh(false);
            }
        }
    }

    pub fn set_animations(&self, refresh_visualizer: &RefreshVisualizer) {
        // COMPOSITION-SEAM: upstream `SetAnimations`
        // (ScrollViewerIRefreshInfoProviderAdapter.cs lines 172-207) gives the
        // composition visual of the visualizer and the composition visual of
        // the scroll viewer an implicit animation collection with one entry:
        // "Offset" -> a `Vector3KeyFrameAnimation` with target "Offset", the
        // expression key frame (1.0, "this.FinalValue") and a duration of
        // 150 ms (`Compositor.CreateVector3KeyFrameAnimation`,
        // `InsertExpressionKeyFrame`, `Compositor.CreateImplicitAnimationCollection`,
        // `CompositionObject.ImplicitAnimations`). Key frame animations and
        // implicit animation collections are not ported yet, so the offsets
        // the visualizer sets take effect without the 150 ms ease.
        let _ = refresh_visualizer;
    }

    fn scroll_viewer_loaded(&self) {
        let scroll_viewer = self.scroll_viewer.borrow().clone();

        let content = scroll_viewer
            .as_ref()
            .and_then(|scroll_viewer| scroll_viewer.content())
            .and_then(|content| Control::from_boxed(&content));
        let Some(content) = content else {
            panic!("Adaptee's content property must be a Visual");
        };

        let Some(parent) = content.parent().and_then(|parent| parent.cast::<InputElement>()) else {
            panic!("Adaptee's content parent must be an InputElement");
        };

        self.make_interaction_source(Some(parent));

        if let Some(scroll_viewer) = scroll_viewer {
            let token = {
                let mut handlers = self.loaded_handlers.borrow_mut();
                // Subscriptions whose scroll viewer is gone are forgotten.
                handlers.retain(|(viewer, _)| viewer.upgrade().is_some());
                let index = handlers
                    .iter()
                    .rposition(|(viewer, _)| viewer.upgrade().is_some_and(|viewer| viewer == scroll_viewer));
                index.map(|index| handlers.remove(index).1)
            };
            if let Some(token) = token {
                scroll_viewer.remove_handler(Control::loaded_event(), token);
            }
        }
    }

    fn make_interaction_source(&self, element: Option<Ref<InputElement>>) {
        drop(self.interaction_source.replace(element.clone()));

        let recognizer = self.pull_gesture_recognizer.borrow().clone();
        let provider = self.refresh_info_provider.borrow().clone();
        if let (Some(recognizer), Some(provider)) = (recognizer, provider) {
            if let Some(element) = element {
                element.gesture_recognizers().add(recognizer);
                self.add_pull_gesture_handlers(&element, &provider);
            }
            self.is_visualizer_interaction_source_attached.set(true);
        }
    }

    fn scroll_viewer_pointer_released(&self) {
        let provider = self.refresh_info_provider.borrow().clone();
        if let Some(provider) = provider {
            provider.set_is_interacting_for_refresh(false);
        }
    }

    fn scroll_viewer_pointer_pressed(&self) {
        let provider = self.refresh_info_provider.borrow().clone();
        if let Some(provider) = provider {
            provider.set_peeking_mode(!self.is_within_offset_threshold());
        }
    }

    fn is_within_offset_threshold(&self) -> bool {
        let scroll_viewer = self.scroll_viewer.borrow().clone();
        if let Some(scroll_viewer) = scroll_viewer {
            let offset = scroll_viewer.offset();

            return match self.refresh_pull_direction.get() {
                PullDirection::TopToBottom => offset.y < INITIAL_OFFSET_THRESHOLD,
                PullDirection::LeftToRight => offset.x < INITIAL_OFFSET_THRESHOLD,
                PullDirection::RightToLeft => {
                    offset.x > scroll_viewer.extent().width - scroll_viewer.viewport().width - INITIAL_OFFSET_THRESHOLD
                }
                PullDirection::BottomToTop => {
                    offset.y > scroll_viewer.extent().height - scroll_viewer.viewport().height - INITIAL_OFFSET_THRESHOLD
                }
            };
        }

        false
    }

    pub fn update_pull_direction(&self, pull_direction: PullDirection) {
        self.refresh_pull_direction.set(pull_direction);

        let provider = self.refresh_info_provider.borrow().clone();
        if let Some(provider) = provider {
            provider.set_pull_direction(pull_direction);
        }

        let recognizer = self.pull_gesture_recognizer.borrow().clone();
        if let Some(recognizer) = recognizer {
            recognizer.set_pull_direction(pull_direction);
        }
    }

    pub fn update_is_mouse_enabled(&self, is_mouse_enabled: bool) {
        self.is_mouse_enabled.set(is_mouse_enabled);

        let recognizer = self.pull_gesture_recognizer.borrow().clone();
        if let Some(recognizer) = recognizer {
            recognizer.set_is_mouse_enabled(is_mouse_enabled);
        }
    }

    pub fn update_visualizer_size(&self, refresh_visualizer_size: Option<Size>) {
        let provider = self.refresh_info_provider.borrow().clone();
        if let Some(provider) = provider {
            provider.set_refresh_visualizer_size(refresh_visualizer_size.unwrap_or_default());
        }
    }

    fn clean_up_scroll_viewer(&self) {
        let scroll_viewer = self.scroll_viewer.borrow().clone();
        if let Some(scroll_viewer) = scroll_viewer {
            let handlers = self.scroll_viewer_handlers.borrow_mut().pop();
            if let Some([pointer_pressed, pointer_released, scroll_changed]) = handlers {
                scroll_viewer.remove_handler(InputElement::pointer_pressed_event(), pointer_pressed);
                scroll_viewer.remove_handler(InputElement::pointer_released_event(), pointer_released);
                scroll_viewer.remove_handler(ScrollViewer::scroll_changed_event(), scroll_changed);
            }
        }
    }
}

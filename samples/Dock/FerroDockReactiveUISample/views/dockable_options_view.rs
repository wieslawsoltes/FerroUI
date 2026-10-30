// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferrodock_model::core::{DockingWindowState, PinnedDockDisplayMode};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, input::InputElementImpl, instantiate,
    interactivity::InteractiveImpl, layout::LayoutableImpl, BoxedValue, FerroObjectImpl, Ref,
    StyledElementImpl, VisualImpl,
};
use ferroui_controls::{
    primitives::TemplatedControlImpl, ContentControlImpl, ControlImpl, IItemsList, ItemsSource,
    UserControl,
};
use std::{any::Any, cell::RefCell, rc::Rc};

type PinnedOptions = Rc<RefCell<[Option<BoxedValue>; 3]>>;
type StateOptions = Rc<RefCell<[DockingWindowState; 11]>>;
ferroui_base::ferro_thread_local! {
    static PINNED_OPTIONS: PinnedOptions = Rc::new(RefCell::new([
        None, Some(Rc::new(PinnedDockDisplayMode::Overlay) as BoxedValue),
        Some(Rc::new(PinnedDockDisplayMode::Inline) as BoxedValue),
    ]));
    static STATE_OPTIONS: StateOptions = Rc::new(RefCell::new([
        DockingWindowState::DOCKED,
        DockingWindowState::PINNED,
        DockingWindowState::DOCUMENT,
        DockingWindowState::DOCKED | DockingWindowState::FLOATING,
        DockingWindowState::PINNED | DockingWindowState::FLOATING,
        DockingWindowState::DOCUMENT | DockingWindowState::FLOATING,
        DockingWindowState::DOCKED | DockingWindowState::HIDDEN,
        DockingWindowState::PINNED | DockingWindowState::HIDDEN,
        DockingWindowState::DOCUMENT | DockingWindowState::HIDDEN,
        DockingWindowState::DOCKED | DockingWindowState::FLOATING | DockingWindowState::HIDDEN,
        DockingWindowState::DOCUMENT | DockingWindowState::FLOATING | DockingWindowState::HIDDEN,
    ]));
}

/// Original passive docking-option view and its shared, fixed-length option arrays.
#[repr(C)]
pub struct DockableOptionsView {
    base: UserControl,
}
ferro_class!(DockableOptionsView:UserControl);
ferro_class_info!(DockableOptionsView {
    new: DockableOptionsView::new,
    markup: {
        static_properties: [
            PinnedDockDisplayModeOptions:ItemsSource {get:|| ItemsSource::new(Rc::new(OptionArray {
                values:DockableOptionsView::pinned_dock_display_mode_options(), box_item:|value|value,
            }))},
            DockingWindowStateOptions:ItemsSource {get:|| ItemsSource::new(Rc::new(OptionArray {
                values:DockableOptionsView::docking_window_state_options(), box_item:|value|Some(Rc::new(value)),
            }))},
        ],
    },
});
ferro_impl_classes!(DockableOptionsView:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl,ContentControlImpl);
impl DockableOptionsView {
    /// Gets the source nullable-object array, shared between every view instance.
    pub fn pinned_dock_display_mode_options() -> PinnedOptions {
        PINNED_OPTIONS.with(Clone::clone)
    }
    /// Gets the source flags array in its original order, without list notifications.
    pub fn docking_window_state_options() -> StateOptions {
        STATE_OPTIONS.with(Clone::clone)
    }
    /// Creates the source view and initializes its compiled markup.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self {
            base: UserControl::construct(),
        });
        crate::register_types();
        crate::compiled_dockable_options_view::populate(None, &this)
            .unwrap_or_else(|error| panic!("{error}"));
        this
    }
}
// Native enumerable view over the same fixed array: element writes remain visible,
// array identity is stable, and no observable-list behavior is invented.
struct OptionArray<T, const N: usize> {
    values: Rc<RefCell<[T; N]>>,
    box_item: fn(T) -> Option<BoxedValue>,
}
impl<T: Clone + 'static, const N: usize> IItemsList for OptionArray<T, N> {
    fn count(&self) -> usize {
        N
    }
    fn get_at(&self, index: usize) -> Option<BoxedValue> {
        (self.box_item)(self.values.borrow()[index].clone())
    }
    fn identity(&self) -> Option<usize> {
        Some(Rc::as_ptr(&self.values) as usize)
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

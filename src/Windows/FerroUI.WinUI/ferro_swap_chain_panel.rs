//! The panel that shows the framework's content inside a WinUI 3 window: a
//! `SwapChainPanel` the content renders into through a composition swap
//! chain, with the pointer, keyboard and text input, the cursor, drag and
//! drop and the automation peer of the content.
//!
//! The reference derives the panel from `SwapChainPanel` in C#. Here the
//! panel is the outer object of a composed `SwapChainPanel`
//! (`interop/composable.rs`) whose override of `OnCreateAutomationPeer` is
//! the one of the reference, and [`FerroSwapChainPanel`] is the class with
//! the members of the reference. The application owns it (`Rc`) and adds
//! its element ([`FerroSwapChainPanel::as_ui_element`]) to the tree of a
//! window; the element holds the panel weakly, so an element whose panel
//! was dropped is a plain swap chain panel (docs/porting/winui.md, section
//! 4).

use crate::interop::PointerUpdateKind;
use ferroui_base::input::raw::RawPointerEventType;

#[cfg(windows)]
pub use imp::FerroSwapChainPanel;
#[cfg(windows)]
pub(crate) use imp::tick_count;

/// The raw event of a press of a mouse or a pen, from the buttons that are
/// pressed; none when no button is.
fn pressed_event_type(left: bool, right: bool, middle: bool) -> Option<RawPointerEventType> {
    if left {
        Some(RawPointerEventType::LeftButtonDown)
    } else if right {
        Some(RawPointerEventType::RightButtonDown)
    } else if middle {
        Some(RawPointerEventType::MiddleButtonDown)
    } else {
        None
    }
}

/// The raw event of a release of a mouse or a pen, from the kind of the
/// update; none for any other update.
fn released_event_type(kind: PointerUpdateKind) -> Option<RawPointerEventType> {
    match kind {
        PointerUpdateKind::LeftButtonReleased => Some(RawPointerEventType::LeftButtonUp),
        PointerUpdateKind::RightButtonReleased => Some(RawPointerEventType::RightButtonUp),
        PointerUpdateKind::MiddleButtonReleased => Some(RawPointerEventType::MiddleButtonUp),
        _ => None,
    }
}

/// `char.IsControl` of a UTF-16 unit: the controls of C0 and C1.
fn is_control(unit: u16) -> bool {
    unit < 0x20 || (0x7F..=0x9F).contains(&unit)
}

/// What a `CharacterReceived` event gives the content: the text of the
/// unit, unless it is a control other than a line break or a tab.
///
/// The reference makes a string of the unit, a lone surrogate among them; a
/// Rust string holds no lone surrogate, so the first half of a pair is kept
/// (`pending`) and the text is sent with the second (docs/porting/winui.md,
/// section 4.2).
fn received_text(pending: &mut Option<u16>, unit: u16) -> Option<String> {
    if is_control(unit) && unit != u16::from(b'\r') && unit != u16::from(b'\n') && unit != u16::from(b'\t') {
        return None;
    }
    if (0xD800..0xDC00).contains(&unit) {
        *pending = Some(unit);
        return None;
    }
    if (0xDC00..0xE000).contains(&unit) {
        let high = pending.take()?;
        return String::from_utf16(&[high, unit]).ok();
    }
    *pending = None;
    String::from_utf16(&[unit]).ok()
}

/// Whether the keys of the virtual key are pressed: the state of the thread
/// has the bit `Down`.
#[cfg_attr(not(windows), allow(dead_code))]
fn is_down(state: crate::interop::CoreVirtualKeyStates) -> bool {
    state.contains(crate::interop::CoreVirtualKeyStates::Down)
}

#[cfg(windows)]
mod imp {
    use super::{is_down, pressed_event_type, received_text, released_event_type};
    use crate::automation::FerroSwapChainPanelAutomationPeer;
    use crate::interop::async_operation::wait;
    use crate::interop::collections::StorageItems;
    use crate::interop::composable::{compose, Composed, ComposedBase};
    use crate::interop::event_handlers::*;
    use crate::interop::win_rt::{factory, flag, hstring, take_string, AgileComPtr};
    use crate::interop::*;
    use crate::win_ui_cursor_factory::{WinUICursorFactory, WinUICursorImpl};
    use crate::win_ui_drag_source::WinUIDragSource;
    use crate::win_ui_key_interop::WinUIKeyInterop;
    use crate::win_ui_text_input_method::WinUITextInputMethod;
    use ferroui_base::input::platform::IPlatformDragSource;
    use ferroui_base::input::raw::{
        IDragDropDevice, IRawInputEventArgs, RawDragEvent, RawDragEventType, RawKeyEventArgs, RawKeyEventType,
        RawMouseWheelEventArgs, RawPointerEventArgs, RawPointerEventType, RawPointerPoint, RawTextInputEventArgs,
        RawTouchEventArgs,
    };
    use ferroui_base::input::{
        DataFormat, DataTransfer, DataTransferExtensions, DataTransferItem, DragDropEffects, FocusManager, IDataTransfer,
        IInputDevice, IInputRoot, IKeyboardDevice, Key, KeyDeviceType, KeyModifiers, KeyboardNavigationHandler, MouseDevice,
        NavigationDirection, NavigationMethod, PenDevice, PhysicalKey, RawInputModifiers, StandardCursorType, TouchDevice,
    };
    use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
    use ferroui_base::platform::storage::file_io::{BclStorageItemHandle, StorageProviderHelpers};
    use ferroui_base::platform::storage::IStorageItem as FerroStorageItem;
    use ferroui_base::platform::ICursorImpl;
    use ferroui_base::reactive::IDisposable;
    use ferroui_base::threading::Dispatcher;
    use ferroui_base::{FerroLocator, LocatorExtensions, PixelSize, Point as FerroPoint, Rect as FerroRect, Size as FerroSize, Vector};
    use ferroui_controls::embedding::EmbeddableControlRoot;
    use ferroui_controls::platform::ITopLevelImpl;
    use ferroui_controls::Control;
    use ferroui_microcom::{ComPtr, HResult};
    use ferroui_win32::open_gl::angle::SwapChainGlSurface;
    use ferroui_win32::SwapChainTopLevelImpl;
    use ferroui_base::Ref;
    use std::any::Any;
    use std::cell::{Cell, RefCell};
    use std::collections::HashMap;
    use std::rc::{Rc, Weak};
    use std::sync::atomic::{AtomicI32, AtomicU64, Ordering};
    use std::sync::Arc;

    /// `Environment.TickCount64`: milliseconds since the system started.
    pub(crate) fn tick_count() -> u64 {
        // SAFETY: takes nothing.
        unsafe { windows_sys::Win32::System::SystemInformation::GetTickCount64() }
    }

    /// The size and the scaling of the panel as the thread that renders asks
    /// for them (the reference reads two fields from that thread).
    #[derive(Default)]
    struct CachedSize {
        width: AtomicI32,
        height: AtomicI32,
        scaling: AtomicU64,
    }

    impl CachedSize {
        fn new() -> CachedSize {
            let cached = CachedSize::default();
            cached.width.store(1, Ordering::Relaxed);
            cached.height.store(1, Ordering::Relaxed);
            cached.scaling.store(1.0f64.to_bits(), Ordering::Relaxed);
            cached
        }

        fn pixel_size(&self) -> PixelSize {
            PixelSize::new(self.width.load(Ordering::Relaxed), self.height.load(Ordering::Relaxed))
        }

        fn scaling(&self) -> f64 {
            f64::from_bits(self.scaling.load(Ordering::Relaxed))
        }
    }

    /// The outer object of the composed `SwapChainPanel`.
    struct PanelOuter {
        base: ComposedBase,
        panel: RefCell<Weak<FerroSwapChainPanel>>,
    }

    impl Composed for PanelOuter {
        fn composed_base(&self) -> &ComposedBase {
            &self.base
        }
    }

    inspectable!(PanelOuter, "FerroUI.WinUI.FerroSwapChainPanel", [IUIElementOverrides]);

    impl IUIElementOverridesImpl for PanelOuter {
        fn on_create_automation_peer(&self) -> Result<Option<ComPtr<IAutomationPeer>>, HResult> {
            let panel = self.panel.borrow().upgrade();
            match panel {
                Some(panel) => panel.on_create_automation_peer().map(Some),
                None => self.base.base::<IUIElementOverrides>()?.on_create_automation_peer(),
            }
        }

        fn on_disconnect_visual_children(&self) -> Result<(), HResult> {
            self.base.base::<IUIElementOverrides>()?.on_disconnect_visual_children()
        }

        fn find_sub_elements_for_touch_targeting(&self, point: Point, bounding_rect: Rect) -> Result<*mut std::ffi::c_void, HResult> {
            self.base.base::<IUIElementOverrides>()?.find_sub_elements_for_touch_targeting(point, bounding_rect)
        }

        fn get_children_in_tab_focus_order(&self) -> Result<*mut std::ffi::c_void, HResult> {
            self.base.base::<IUIElementOverrides>()?.get_children_in_tab_focus_order()
        }

        fn on_keyboard_accelerator_invoked(&self, args: Option<&IInspectable>) -> Result<(), HResult> {
            self.base.base::<IUIElementOverrides>()?.on_keyboard_accelerator_invoked(args)
        }

        fn on_process_keyboard_accelerators(&self, args: Option<&IInspectable>) -> Result<(), HResult> {
            self.base.base::<IUIElementOverrides>()?.on_process_keyboard_accelerators(args)
        }

        fn on_bring_into_view_requested(&self, e: Option<&IInspectable>) -> Result<(), HResult> {
            self.base.base::<IUIElementOverrides>()?.on_bring_into_view_requested(e)
        }

        fn populate_property_info_override(&self, property_name: isize, animation_property_info: Option<&IInspectable>) -> Result<(), HResult> {
            self.base.base::<IUIElementOverrides>()?.populate_property_info_override(property_name, animation_property_info)
        }
    }

    /// `CursorOverlay` of the reference: `ProtectedCursor` is protected on
    /// `UIElement`; the reference derives a grid to expose it. The grid here
    /// is composed the same way, and its outer object asks the inner object
    /// for the protected interface.
    struct CursorOverlay {
        grid: ComPtr<IInspectable>,
        base: Rc<CursorOverlayOuterHandle>,
    }

    /// What the overlay keeps of its outer object: the inner object, through
    /// which the protected members are reached.
    struct CursorOverlayOuterHandle {
        protected: ComPtr<IUIElementProtected>,
    }

    struct CursorOverlayOuter {
        base: ComposedBase,
    }

    impl Composed for CursorOverlayOuter {
        fn composed_base(&self) -> &ComposedBase {
            &self.base
        }
    }

    inspectable!(CursorOverlayOuter, "FerroUI.WinUI.FerroSwapChainPanel.CursorOverlay", [IInspectable]);

    impl CursorOverlay {
        fn new() -> Result<CursorOverlay, HResult> {
            let grid_factory = factory::<IGridFactory>("Microsoft.UI.Xaml.Controls.Grid")?;
            let outer: ComPtr<IInspectable> = compose(CursorOverlayOuter { base: ComposedBase::default() }, [], |outer, inner| {
                // SAFETY: the outer object and a place of the caller's frame.
                unsafe { grid_factory.create_instance(Some(outer), inner) }.map(|_| ())
            })?;
            // SAFETY: an interface of an object made by `compose` with a
            // `CursorOverlayOuter`, alive through `outer`.
            let value = unsafe { ferroui_microcom::ComObject::<CursorOverlayOuter>::value(outer.as_ptr().cast()) };
            let protected = value.base.base::<IUIElementProtected>()?;
            Ok(CursorOverlay { grid: outer, base: Rc::new(CursorOverlayOuterHandle { protected }) })
        }

        fn set_cursor(&self, cursor: Option<ComPtr<IInputCursor>>) {
            let _ = self.base.protected.set_protected_cursor(cursor.as_deref());
        }

        fn element(&self) -> Result<ComPtr<IUIElement>, HResult> {
            self.grid.cast::<IUIElement>()
        }
    }

    ferroui_base::ferro_thread_local! {
        /// The static constructor of the reference: the first panel binds
        /// the drag source.
        static DRAG_SOURCE_BOUND: Cell<bool> = const { Cell::new(false) };

        /// Looking up panels from the drag source: keyed by the
        /// top-level platform implementation so multi-panel hosts work.
        static S_PANELS_BY_IMPL: RefCell<HashMap<usize, Weak<FerroSwapChainPanel>>> = RefCell::new(HashMap::new());

        /// The panels of the thread by an identifier that another thread may
        /// hold (the swap chain arrives on the thread that renders).
        static PANELS_BY_ID: RefCell<HashMap<u64, Weak<FerroSwapChainPanel>>> = RefCell::new(HashMap::new());
        static NEXT_ID: Cell<u64> = const { Cell::new(1) };
    }

    fn impl_key(top_level_impl: &Rc<dyn ITopLevelImpl>) -> usize {
        Rc::as_ptr(top_level_impl) as *const () as usize
    }

    /// A pointer to a swap chain on its way to the UI thread: counted, and
    /// DXGI objects are free-threaded.
    struct SwapChainReference(usize);

    // SAFETY: a counted reference to a DXGI swap chain, an object of no
    // thread.
    unsafe impl Send for SwapChainReference {}

    impl Drop for SwapChainReference {
        fn drop(&mut self) {
            // SAFETY: the reference this value holds (`OnSwapChainCreated`).
            unsafe { (*(self.0 as *const ferroui_microcom::IUnknown)).release() };
        }
    }

    /// The device of a pointer event.
    #[derive(Clone)]
    enum PointerDevice {
        Mouse(Rc<MouseDevice>),
        Touch(Rc<TouchDevice>),
        Pen(Rc<PenDevice>),
    }

    impl PointerDevice {
        fn input_device(&self) -> Rc<dyn IInputDevice> {
            match self {
                PointerDevice::Mouse(device) => device.clone(),
                PointerDevice::Touch(device) => device.clone(),
                PointerDevice::Pen(device) => device.clone(),
            }
        }

        fn is_touch(&self) -> bool {
            matches!(self, PointerDevice::Touch(_))
        }
    }

    /// The raw event of a pointer: a touch event for a touch device.
    enum PointerArgs {
        Pointer(Rc<RawPointerEventArgs>),
        Touch(Rc<RawTouchEventArgs>),
    }

    impl PointerArgs {
        fn pointer(&self) -> &RawPointerEventArgs {
            match self {
                PointerArgs::Pointer(args) => args,
                PointerArgs::Touch(args) => args,
            }
        }

        fn into_input(self) -> Rc<dyn IRawInputEventArgs> {
            match self {
                PointerArgs::Pointer(args) => args,
                PointerArgs::Touch(args) => args,
            }
        }
    }

    /// The panel. See the module.
    pub struct FerroSwapChainPanel {
        this: Weak<FerroSwapChainPanel>,
        id: u64,
        outer: ComPtr<IUIElementOverrides>,
        element: ComPtr<IUIElement>,
        framework_element: ComPtr<IFrameworkElement>,
        swap_chain_panel: ComPtr<ISwapChainPanel>,

        gl_surface: RefCell<Option<Arc<SwapChainGlSurface>>>,
        top_level_impl: RefCell<Option<Rc<SwapChainTopLevelImpl>>>,
        root: RefCell<Option<Ref<EmbeddableControlRoot>>>,
        content: RefCell<Option<Ref<Control>>>,
        mouse_device: Rc<MouseDevice>,
        touch_device: Rc<TouchDevice>,
        pen_device: Rc<PenDevice>,
        cached_size: Arc<CachedSize>,

        text_input_method: RefCell<Option<Rc<WinUITextInputMethod>>>,
        ignore_character_received: Cell<bool>,
        /// The first half of a surrogate pair `CharacterReceived` gave.
        pending_high_surrogate: Cell<Option<u16>>,

        cursor_overlay: CursorOverlay,

        // Most recent WinUI PointerPoint observed by the panel — needed by
        // StartDragAsync. The framework's pointer id is assigned
        // sequentially and does not correspond to WinUI's PointerId, so we
        // can't look up per pointer; the latest is good enough for the
        // dominant mouse-drag case.
        last_pointer_point: RefCell<Option<ComPtr<IPointerPoint>>>,

        // Cleared in DragStarting and OnUnloaded.
        outgoing_drag_data: RefCell<Option<Rc<dyn IDataTransfer>>>,
        outgoing_drag_allowed: Cell<DragDropEffects>,

        // Incoming drag-and-drop
        active_drag: RefCell<Option<Rc<DataTransfer>>>,

        /// Set when the swap chain was set on the panel.
        swap_chain_attached: Cell<bool>,
    }

    impl FerroSwapChainPanel {
        /// The static constructor of the reference: the drag source of the
        /// platform is the one of the panel.
        fn static_init() {
            if DRAG_SOURCE_BOUND.replace(true) {
                return;
            }
            let drag_source: Rc<dyn IPlatformDragSource> = Rc::new(WinUIDragSource);
            FerroLocator::current_mutable().bind::<dyn IPlatformDragSource>().to_constant(drag_source);
        }

        pub(crate) fn get_panel_for(top_level_impl: &Rc<dyn ITopLevelImpl>) -> Option<Rc<FerroSwapChainPanel>> {
            S_PANELS_BY_IMPL.with(|panels| panels.borrow().get(&impl_key(top_level_impl)).and_then(Weak::upgrade))
        }

        pub fn new() -> Result<Rc<FerroSwapChainPanel>, HResult> {
            Self::static_init();

            let panel_factory = factory::<ISwapChainPanelFactory>("Microsoft.UI.Xaml.Controls.SwapChainPanel")?;
            let cursor_overlay = CursorOverlay::new()?;
            let outer = compose::<IUIElementOverrides, PanelOuter, 0>(
                PanelOuter { base: ComposedBase::default(), panel: RefCell::new(Weak::new()) },
                [],
                |outer, inner| {
                    // SAFETY: the outer object and a place of the caller's
                    // frame.
                    unsafe { panel_factory.create_instance(Some(outer), inner) }.map(|_| ())
                },
            )?;
            let element = outer.cast::<IUIElement>()?;
            let framework_element = outer.cast::<IFrameworkElement>()?;
            let swap_chain_panel = outer.cast::<ISwapChainPanel>()?;
            let id = NEXT_ID.with(|next| next.replace(next.get() + 1));
            let panel = Rc::new_cyclic(|this: &Weak<FerroSwapChainPanel>| FerroSwapChainPanel {
                this: this.clone(),
                id,
                outer: outer.clone(),
                element,
                framework_element,
                swap_chain_panel,
                gl_surface: RefCell::new(None),
                top_level_impl: RefCell::new(None),
                root: RefCell::new(None),
                content: RefCell::new(None),
                mouse_device: MouseDevice::new(),
                touch_device: TouchDevice::new(),
                pen_device: PenDevice::new(false),
                cached_size: Arc::new(CachedSize::new()),
                text_input_method: RefCell::new(None),
                ignore_character_received: Cell::new(false),
                pending_high_surrogate: Cell::new(None),
                cursor_overlay,
                last_pointer_point: RefCell::new(None),
                outgoing_drag_data: RefCell::new(None),
                outgoing_drag_allowed: Cell::new(DragDropEffects::NONE),
                active_drag: RefCell::new(None),
                swap_chain_attached: Cell::new(false),
            });
            // SAFETY: an interface of the object `compose` made with a
            // `PanelOuter`, alive through `outer`.
            let value = unsafe { ferroui_microcom::ComObject::<PanelOuter>::value(outer.as_ptr().cast()) };
            *value.panel.borrow_mut() = Rc::downgrade(&panel);
            panel.initialize()?;
            PANELS_BY_ID.with(|panels| panels.borrow_mut().insert(panel.id, Rc::downgrade(&panel)));
            Ok(panel)
        }

        /// The body of the constructor of the reference: the overlay and the
        /// subscriptions.
        fn initialize(&self) -> Result<(), HResult> {
            self.element.set_is_tab_stop(1)?;
            // SwapChainPanel disallows Background, but without a hit-testable
            // surface WinUI won't resolve ProtectedCursor for the panel. Add a
            // transparent overlay that fills the panel — it costs nothing
            // visually but participates in cursor hit-testing.
            let overlay = self.cursor_overlay.element()?;
            let brush_factory = factory::<ISolidColorBrushFactory>("Microsoft.UI.Xaml.Media.SolidColorBrush")?;
            // Microsoft.UI.Colors.Transparent
            let transparent = Color { a: 0, r: 0xFF, g: 0xFF, b: 0xFF };
            let brush = brush_factory.create_instance_with_color(transparent)?.ok_or(HResult::POINTER)?.cast::<IBrush>()?;
            self.cursor_overlay.grid.cast::<IPanel>()?.set_background(Some(&brush))?;
            overlay.set_is_hit_test_visible(1)?;
            overlay.set_allow_drop(1)?;
            overlay.set_can_drag(1)?;

            let this = self.this.clone();
            let weak = move || this.clone();
            overlay.add_drag_enter(Some(&drag_event_handler({
                let this = weak();
                move |_, e| with(&this, |panel| panel.on_drag_enter(e))
            })))?;
            overlay.add_drag_over(Some(&drag_event_handler({
                let this = weak();
                move |_, e| with(&this, |panel| panel.on_drag_over(e))
            })))?;
            overlay.add_drag_leave(Some(&drag_event_handler({
                let this = weak();
                move |_, e| with(&this, |panel| panel.on_drag_leave(e))
            })))?;
            overlay.add_drop(Some(&drag_event_handler({
                let this = weak();
                move |_, e| with(&this, |panel| panel.on_drop(e))
            })))?;
            overlay.add_drag_starting(Some(&drag_starting_handler({
                let this = weak();
                move |_, e| with(&this, |panel| panel.on_drag_starting(e))
            })))?;
            self.outer.cast::<IPanel>()?.children()?.ok_or(HResult::POINTER)?.append(Some(&overlay))?;

            self.framework_element.add_loaded(Some(&routed_event_handler({
                let this = weak();
                move |_, _| with(&this, |panel| panel.on_loaded())
            })))?;
            self.framework_element.add_unloaded(Some(&routed_event_handler({
                let this = weak();
                move |_, _| with(&this, |panel| panel.on_unloaded())
            })))?;
            self.framework_element.add_size_changed(Some(&size_changed_event_handler({
                let this = weak();
                move |_, e| with(&this, |panel| panel.on_size_changed(e))
            })))?;
            self.swap_chain_panel.add_composition_scale_changed(Some(&composition_scale_changed_handler({
                let this = weak();
                move |_, _| with(&this, |panel| panel.on_composition_scale_changed())
            })))?;

            macro_rules! pointer {
                ($add:ident, $handler:ident) => {
                    self.element.$add(Some(&pointer_event_handler({
                        let this = weak();
                        move |_, e| with(&this, |panel| panel.$handler(e))
                    })))?;
                };
            }
            pointer!(add_pointer_pressed, on_pointer_pressed);
            pointer!(add_pointer_moved, on_pointer_moved);
            pointer!(add_pointer_released, on_pointer_released);
            pointer!(add_pointer_wheel_changed, on_pointer_wheel_changed);
            pointer!(add_pointer_canceled, on_pointer_canceled);
            pointer!(add_pointer_entered, on_pointer_entered);
            pointer!(add_pointer_exited, on_pointer_exited);
            self.element.add_got_focus(Some(&routed_event_handler({
                let this = weak();
                move |_, _| with(&this, |panel| panel.on_got_focus())
            })))?;
            self.element.add_lost_focus(Some(&routed_event_handler({
                let this = weak();
                move |_, _| with(&this, |panel| panel.on_lost_focus())
            })))?;

            let statics = factory::<IUIElementStatics>("Microsoft.UI.Xaml.UIElement")?;
            let key_down = box_key_event_handler(key_event_handler({
                let this = weak();
                move |_, e| with(&this, |panel| panel.on_key_down(e))
            }))?;
            self.element.add_handler(statics.key_down_event()?.as_deref(), Some(&key_down), 1)?;
            let key_up = box_key_event_handler(key_event_handler({
                let this = weak();
                move |_, e| with(&this, |panel| panel.on_key_up(e))
            }))?;
            self.element.add_handler(statics.key_up_event()?.as_deref(), Some(&key_up), 1)?;
            let character_received = box_character_received_handler(character_received_handler({
                let this = weak();
                move |_, e| with(&this, |panel| panel.on_character_received(e))
            }))?;
            self.element.add_handler(statics.character_received_event()?.as_deref(), Some(&character_received), 1)?;
            Ok(())
        }

        /// The element of the panel, to add to the tree of a window (the
        /// reference's panel is the element).
        pub fn as_ui_element(&self) -> ComPtr<IUIElement> {
            self.element.clone()
        }

        pub fn content(&self) -> Option<Ref<Control>> {
            self.content.borrow().clone()
        }

        pub fn set_content(&self, value: Option<Ref<Control>>) {
            *self.content.borrow_mut() = value.clone();
            let root = self.root.borrow().clone();
            if let Some(root) = root {
                root.set_content(value.map(Control::boxed));
            }
        }

        fn on_create_automation_peer(&self) -> Result<ComPtr<IAutomationPeer>, HResult> {
            FerroSwapChainPanelAutomationPeer::new(self.this.upgrade().ok_or(HResult::POINTER)?)
        }

        // ---- Diagnostics / internal automation hooks ----

        /// Returns the embedded control root, or none if the panel has not
        /// loaded yet. Used by the automation peer to enumerate the
        /// framework's children; also exposed as a diagnostic surface for
        /// the sample to verify peer lifecycle.
        pub fn get_embedded_root_for_automation(&self) -> Option<Ref<EmbeddableControlRoot>> {
            self.root.borrow().clone()
        }

        /// Whether a swap chain has been set on the panel: the first frame
        /// was given a render target. A diagnostic of the port, which the
        /// smoke run of the sample reads.
        pub fn is_swap_chain_attached(&self) -> bool {
            self.swap_chain_attached.get()
        }

        fn update_cached_size(&self) {
            let scale_x = f64::from(self.swap_chain_panel.composition_scale_x().unwrap_or(1.0));
            let scale_y = f64::from(self.swap_chain_panel.composition_scale_y().unwrap_or(1.0));
            let w = 1.max((self.framework_element.actual_width().unwrap_or(0.0) * scale_x) as i32);
            let h = 1.max((self.framework_element.actual_height().unwrap_or(0.0) * scale_y) as i32);
            self.cached_size.width.store(w, Ordering::Relaxed);
            self.cached_size.height.store(h, Ordering::Relaxed);
            self.cached_size.scaling.store(scale_x.to_bits(), Ordering::Relaxed);
        }

        fn get_pixel_size(cached_size: &CachedSize) -> PixelSize {
            cached_size.pixel_size()
        }

        fn get_scaling(cached_size: &CachedSize) -> f64 {
            cached_size.scaling()
        }

        fn on_loaded(&self) {
            if self.gl_surface.borrow().is_some() {
                return;
            }

            self.update_cached_size();

            // Create the GL surface — swap chain creation is deferred to
            // CreateGlRenderTarget where we have the actual rendering
            // context's D3D device.
            let size = self.cached_size.clone();
            let scaling = self.cached_size.clone();
            let dispatcher_queue = self.dispatcher_queue();
            let id = self.id;
            let gl_surface = SwapChainGlSurface::new(
                Arc::new(move || Self::get_pixel_size(&size)),
                Arc::new(move || Self::get_scaling(&scaling)),
                Arc::new(move |swap_chain| Self::on_swap_chain_created(dispatcher_queue.as_ref(), id, swap_chain)),
            );
            let top_level_impl = SwapChainTopLevelImpl::new(gl_surface.clone());
            top_level_impl.set_client_size(FerroSize::new(
                self.framework_element.actual_width().unwrap_or(0.0),
                self.framework_element.actual_height().unwrap_or(0.0),
            ));
            top_level_impl.set_render_scaling(f64::from(self.swap_chain_panel.composition_scale_x().unwrap_or(1.0)));
            *self.gl_surface.borrow_mut() = Some(gl_surface);
            *self.top_level_impl.borrow_mut() = Some(top_level_impl.clone());

            let text_input_method = {
                let input_impl = Rc::downgrade(&top_level_impl);
                let root_impl = Rc::downgrade(&top_level_impl);
                WinUITextInputMethod::new(
                    self.this.clone(),
                    Box::new(move || input_impl.upgrade().and_then(|top_level| top_level.input())),
                    Box::new(move || root_impl.upgrade().and_then(|top_level| top_level.input_root())),
                    Box::new(|| FerroLocator::current().get_service::<dyn IKeyboardDevice>()),
                )
            };
            *self.text_input_method.borrow_mut() = Some(text_input_method.clone());
            top_level_impl.set_text_input_method(Some(text_input_method));
            let this = self.this.clone();
            top_level_impl.set_cursor_changed(Some(Rc::new(move |cursor| {
                if let Some(panel) = this.upgrade() {
                    panel.on_ferro_cursor_changed(cursor);
                }
            })));

            // Create and start the EmbeddableControlRoot
            let platform_impl: Rc<dyn ITopLevelImpl> = top_level_impl.clone();
            let root = EmbeddableControlRoot::with_impl(platform_impl.clone());
            root.set_content(self.content.borrow().clone().map(Control::boxed));
            *self.root.borrow_mut() = Some(root.clone());
            root.prepare();
            root.start_rendering();

            S_PANELS_BY_IMPL.with(|panels| panels.borrow_mut().insert(impl_key(&platform_impl), self.this.clone()));
        }

        /// The dispatcher queue of the panel (`DependencyObject.DispatcherQueue`).
        fn dispatcher_queue(&self) -> Option<AgileComPtr<IDispatcherQueue>> {
            let queue = self.outer.cast::<IDependencyObject>().ok()?.dispatcher_queue().ok().flatten()?;
            // SAFETY: a dispatcher queue is agile.
            Some(unsafe { AgileComPtr::new(queue) })
        }

        /// Called from the render thread when the swap chain is first
        /// created. Set it on the panel via ISwapChainPanelNative COM interop.
        fn on_swap_chain_created(dispatcher_queue: Option<&AgileComPtr<IDispatcherQueue>>, id: u64, swap_chain_ptr: isize) {
            let Some(dispatcher_queue) = dispatcher_queue else {
                return;
            };
            if swap_chain_ptr == 0 {
                return;
            }
            // The swap chain is counted while it is on its way: the reference
            // hands the pointer over uncounted.
            // SAFETY: the swap chain the surface holds, live during the call.
            unsafe { (*(swap_chain_ptr as *const ferroui_microcom::IUnknown)).add_ref() };
            let swap_chain = SwapChainReference(swap_chain_ptr as usize);
            let work = dispatcher_queue_handler(move || {
                let swap_chain = swap_chain;
                let Some(panel) = PANELS_BY_ID.with(|panels| panels.borrow().get(&id).and_then(Weak::upgrade)) else {
                    return;
                };
                let native = match panel.outer.cast::<ISwapChainPanelNative>() {
                    Ok(native) => native,
                    Err(_) => return,
                };
                // SAFETY: the swap chain, counted by `swap_chain` through
                // the call; the panel counts its own reference.
                let hr = unsafe { native.set_swap_chain(swap_chain.0 as *mut std::ffi::c_void) };
                if let Err(error) = hr {
                    // Marshal.ThrowExceptionForHR
                    panic!("ISwapChainPanelNative::SetSwapChain failed: {error}");
                }
                panel.swap_chain_attached.set(true);
            });
            let _ = dispatcher_queue.get().try_enqueue(Some(&work));
        }

        fn on_unloaded(&self) {
            let top_level_impl = self.top_level_impl.borrow_mut().take();
            if let Some(top_level_impl) = &top_level_impl {
                let platform_impl: Rc<dyn ITopLevelImpl> = top_level_impl.clone();
                S_PANELS_BY_IMPL.with(|panels| panels.borrow_mut().remove(&impl_key(&platform_impl)));
            }
            let root = self.root.borrow_mut().take();
            if let Some(root) = root {
                root.stop_rendering();
                root.dispose();
            }
            drop(top_level_impl);
            if let Some(gl_surface) = self.gl_surface.borrow_mut().take() {
                gl_surface.dispose_swap_chain();
            }
            self.mouse_device.dispose();
            self.touch_device.dispose();
            self.pen_device.dispose();
        }

        fn on_size_changed(&self, e: Option<&ISizeChangedEventArgs>) {
            self.update_cached_size();
            let top_level_impl = self.top_level_impl.borrow().clone();
            if let (Some(top_level_impl), Some(e)) = (top_level_impl, e) {
                if let Ok(size) = e.new_size() {
                    top_level_impl.set_client_size(FerroSize::new(f64::from(size.width), f64::from(size.height)));
                }
            }
        }

        fn on_composition_scale_changed(&self) {
            self.update_cached_size();
            let top_level_impl = self.top_level_impl.borrow().clone();
            if let Some(top_level_impl) = top_level_impl {
                top_level_impl.set_render_scaling(f64::from(self.swap_chain_panel.composition_scale_x().unwrap_or(1.0)));
            }
        }

        // Input forwarding

        /// The input callback and the input root of the top-level, while the
        /// panel is loaded.
        fn input(&self) -> Option<(Rc<dyn Fn(Rc<dyn IRawInputEventArgs>)>, Rc<dyn IInputRoot>)> {
            let top_level_impl = self.top_level_impl.borrow().clone()?;
            Some((top_level_impl.input()?, top_level_impl.input_root()?))
        }

        fn get_pointer_device(&self, e: &IPointerRoutedEventArgs) -> PointerDevice {
            match device_type(e) {
                PointerDeviceType::Touch => PointerDevice::Touch(self.touch_device.clone()),
                PointerDeviceType::Pen => PointerDevice::Pen(self.pen_device.clone()),
                _ => PointerDevice::Mouse(self.mouse_device.clone()),
            }
        }

        fn create_raw_pointer_point(point: &IPointerPoint) -> RawPointerPoint {
            let props = point.properties().ok().flatten();
            let pos = point.position().unwrap_or(Point { x: 0.0, y: 0.0 });

            let mut raw_point = RawPointerPoint::new();
            raw_point.position = FerroPoint::new(f64::from(pos.x), f64::from(pos.y));
            if let Some(props) = &props {
                raw_point.pressure = props.pressure().unwrap_or(0.5);
                raw_point.twist = props.twist().unwrap_or(0.0);
                raw_point.x_tilt = props.x_tilt().unwrap_or(0.0);
                raw_point.y_tilt = props.y_tilt().unwrap_or(0.0);

                if let Ok(cr) = props.contact_rect() {
                    if cr.width > 0.0 || cr.height > 0.0 {
                        raw_point.set_contact_rect(FerroRect::new(
                            f64::from(cr.x),
                            f64::from(cr.y),
                            f64::from(cr.width),
                            f64::from(cr.height),
                        ));
                    }
                }
            }

            raw_point
        }

        #[allow(clippy::too_many_arguments)]
        fn create_pointer_args(
            &self,
            device: &PointerDevice,
            timestamp: u64,
            input_root: Rc<dyn IInputRoot>,
            type_: RawPointerEventType,
            point: RawPointerPoint,
            modifiers: RawInputModifiers,
            pointer_id: u32,
        ) -> PointerArgs {
            if device.is_touch() {
                PointerArgs::Touch(Rc::new(RawTouchEventArgs::with_point(
                    device.input_device(),
                    timestamp,
                    input_root,
                    type_,
                    point,
                    modifiers,
                    i64::from(pointer_id),
                )))
            } else {
                let args = RawPointerEventArgs::with_point(device.input_device(), timestamp, input_root, type_, point, modifiers);
                args.set_raw_pointer_id(i64::from(pointer_id));
                PointerArgs::Pointer(Rc::new(args))
            }
        }

        fn get_timestamp(&self, e: &IPointerRoutedEventArgs) -> u64 {
            // WinUI PointerPoint.Timestamp is in microseconds; the framework
            // expects milliseconds.
            self.current_point(e).and_then(|point| point.timestamp().ok()).unwrap_or(0) / 1000
        }

        fn current_point(&self, e: &IPointerRoutedEventArgs) -> Option<ComPtr<IPointerPoint>> {
            e.get_current_point(Some(&self.element)).ok().flatten()
        }

        fn get_pointer_modifiers(&self, e: &IPointerRoutedEventArgs) -> RawInputModifiers {
            let point = self.current_point(e);
            let props = point.as_ref().and_then(|point| point.properties().ok().flatten());
            let mut mods = WinUIKeyInterop::modifiers_from_virtual_key_modifiers(
                e.key_modifiers().unwrap_or(VirtualKeyModifiers::None),
            );
            let Some(props) = props else {
                return mods;
            };
            if flag(props.is_left_button_pressed()) {
                mods |= RawInputModifiers::LEFT_MOUSE_BUTTON;
            }
            if flag(props.is_right_button_pressed()) {
                mods |= RawInputModifiers::RIGHT_MOUSE_BUTTON;
            }
            if flag(props.is_middle_button_pressed()) {
                mods |= RawInputModifiers::MIDDLE_MOUSE_BUTTON;
            }

            if device_type(e) == PointerDeviceType::Pen {
                if flag(props.is_barrel_button_pressed()) {
                    mods |= RawInputModifiers::PEN_BARREL_BUTTON;
                }
                if flag(props.is_eraser()) {
                    mods |= RawInputModifiers::PEN_ERASER;
                }
                if flag(props.is_inverted()) {
                    mods |= RawInputModifiers::PEN_INVERTED;
                }
            }

            mods
        }

        fn on_pointer_pressed(&self, e: Option<&IPointerRoutedEventArgs>) {
            let Some(e) = e else { return };
            *self.last_pointer_point.borrow_mut() = self.current_point(e);
            let Some((input, input_root)) = self.input() else { return };

            let device = self.get_pointer_device(e);
            let timestamp = self.get_timestamp(e);
            let Some(point) = self.current_point(e) else { return };
            let raw_point = Self::create_raw_pointer_point(&point);
            let modifiers = self.get_pointer_modifiers(e);
            let pointer = e.pointer().ok().flatten();
            let pointer_id = pointer.as_ref().and_then(|pointer| pointer.pointer_id().ok()).unwrap_or(0);

            let type_ = if device.is_touch() {
                RawPointerEventType::TouchBegin
            } else {
                let props = point.properties().ok().flatten();
                let pressed = |get: fn(&IPointerPointProperties) -> Result<u8, HResult>| {
                    props.as_ref().is_some_and(|props| flag(get(props)))
                };
                match pressed_event_type(
                    pressed(IPointerPointProperties::is_left_button_pressed),
                    pressed(IPointerPointProperties::is_right_button_pressed),
                    pressed(IPointerPointProperties::is_middle_button_pressed),
                ) {
                    Some(type_) => type_,
                    None => return,
                }
            };

            let _ = self.element.focus(FocusState::Pointer);
            let _ = self.element.capture_pointer(pointer.as_deref());

            input(self.create_pointer_args(&device, timestamp, input_root, type_, raw_point, modifiers, pointer_id).into_input());
            let _ = e.set_handled(1);
        }

        fn on_pointer_moved(&self, e: Option<&IPointerRoutedEventArgs>) {
            let Some(e) = e else { return };
            *self.last_pointer_point.borrow_mut() = self.current_point(e);
            let Some((input, input_root)) = self.input() else { return };

            let device = self.get_pointer_device(e);
            let timestamp = self.get_timestamp(e);
            let Some(point) = self.current_point(e) else { return };
            let raw_point = Self::create_raw_pointer_point(&point);
            let modifiers = self.get_pointer_modifiers(e);
            let pointer_id = pointer_id(e);

            let type_ = if device.is_touch() { RawPointerEventType::TouchUpdate } else { RawPointerEventType::Move };

            let args = self.create_pointer_args(&device, timestamp, input_root, type_, raw_point, modifiers, pointer_id);
            let event = ComPtr::from_ref(e);
            let element = self.element.clone();
            let points: Box<dyn FnOnce() -> Option<Vec<RawPointerPoint>>> =
                Box::new(move || Self::get_intermediate_points(&event, &element));
            args.pointer().set_intermediate_points(Some(Rc::new(std::cell::LazyCell::new(points))));
            input(args.into_input());
        }

        fn get_intermediate_points(e: &IPointerRoutedEventArgs, relative_to: &IUIElement) -> Option<Vec<RawPointerPoint>> {
            // WinUI returns the points oldest-first and includes the current
            // point as the last entry; drop that last entry (it is the one we
            // already dispatched).
            let coalesced = e.get_intermediate_points(Some(relative_to)).ok().flatten()?;
            let count = coalesced.size().ok()?;
            if count <= 1 {
                return None;
            }

            let mut points = Vec::with_capacity(count as usize - 1);
            for i in 0..count - 1 {
                if let Ok(Some(point)) = coalesced.get_at(i) {
                    points.push(Self::create_raw_pointer_point(&point));
                }
            }

            Some(points)
        }

        fn on_pointer_released(&self, e: Option<&IPointerRoutedEventArgs>) {
            let Some(e) = e else { return };
            let Some((input, input_root)) = self.input() else { return };

            let device = self.get_pointer_device(e);
            let timestamp = self.get_timestamp(e);
            let Some(point) = self.current_point(e) else { return };
            let raw_point = Self::create_raw_pointer_point(&point);
            let modifiers = self.get_pointer_modifiers(e);
            let pointer = e.pointer().ok().flatten();
            let pointer_id = pointer.as_ref().and_then(|pointer| pointer.pointer_id().ok()).unwrap_or(0);

            let type_ = if device.is_touch() {
                RawPointerEventType::TouchEnd
            } else {
                let kind = point
                    .properties()
                    .ok()
                    .flatten()
                    .and_then(|props| props.pointer_update_kind().ok())
                    .unwrap_or(PointerUpdateKind::Other);
                match released_event_type(kind) {
                    Some(type_) => type_,
                    None => return,
                }
            };

            let _ = self.element.release_pointer_capture(pointer.as_deref());
            input(self.create_pointer_args(&device, timestamp, input_root, type_, raw_point, modifiers, pointer_id).into_input());
            let _ = e.set_handled(1);
        }

        fn on_pointer_wheel_changed(&self, e: Option<&IPointerRoutedEventArgs>) {
            let Some(e) = e else { return };
            let Some((input, input_root)) = self.input() else { return };

            let device = match self.get_pointer_device(e) {
                PointerDevice::Mouse(device) => device,
                _ => self.mouse_device.clone(),
            };
            let timestamp = self.get_timestamp(e);
            let Some(point) = self.current_point(e) else { return };
            let delta = point.properties().ok().flatten().and_then(|props| props.mouse_wheel_delta().ok()).unwrap_or(0);
            let pos = point.position().unwrap_or(Point { x: 0.0, y: 0.0 });

            input(Rc::new(RawMouseWheelEventArgs::new(
                device,
                timestamp,
                input_root,
                FerroPoint::new(f64::from(pos.x), f64::from(pos.y)),
                Vector::new(0.0, f64::from(delta)),
                self.get_pointer_modifiers(e),
            )));
            let _ = e.set_handled(1);
        }

        fn on_ferro_cursor_changed(&self, cursor: Option<Rc<dyn ICursorImpl>>) {
            let Some(cursor) = cursor else {
                self.cursor_overlay.set_cursor(None);
                return;
            };
            if let Some(win_ui_cursor) = cursor.as_any().downcast_ref::<WinUICursorImpl>() {
                self.cursor_overlay.set_cursor(win_ui_cursor.cursor());
            } else if let Some(win32_cursor) = cursor.as_any().downcast_ref::<ferroui_win32::CursorImpl>() {
                self.cursor_overlay.set_cursor(Self::map_win32_cursor_to_win_ui(win32_cursor));
            } else {
                self.cursor_overlay.set_cursor(None);
            }
        }

        fn map_win32_cursor_to_win_ui(cursor: &ferroui_win32::CursorImpl) -> Option<ComPtr<IInputCursor>> {
            use ferroui_base::platform::ICursorFactory;

            let handle = cursor.handle();
            if handle == 0 {
                return None;
            }

            for type_ in ALL_CURSOR_TYPES {
                let cached = ferroui_win32::CursorFactory::instance().get_cursor(type_);
                if let Some(c) = cached.as_any().downcast_ref::<ferroui_win32::CursorImpl>() {
                    if c.handle() == handle {
                        let shape = WinUICursorFactory::instance().get_cursor(type_);
                        return shape.as_any().downcast_ref::<WinUICursorImpl>().and_then(WinUICursorImpl::cursor);
                    }
                }
            }

            factory::<IInputSystemCursorStatics>("Microsoft.UI.Input.InputSystemCursor")
                .ok()
                .and_then(|statics| statics.create(InputSystemCursorShape::Arrow).ok().flatten())
                .and_then(|cursor| cursor.cast::<IInputCursor>().ok())
        }

        fn on_got_focus(&self) {
            let text_input_method = self.text_input_method.borrow().clone();
            if let Some(text_input_method) = text_input_method {
                text_input_method.on_panel_focus_changed(true);
            }
        }

        fn on_lost_focus(&self) {
            let text_input_method = self.text_input_method.borrow().clone();
            if let Some(text_input_method) = text_input_method {
                text_input_method.on_panel_focus_changed(false);
            }
            // Clear the framework's internal focus so the previously focused
            // control doesn't keep its :focus visual once keyboard focus moves
            // to a native WinUI element.
            let root = self.root.borrow().clone();
            if let Some(root) = root {
                if let Some(focus_manager) = FocusManager::get_focus_manager(&root) {
                    focus_manager.focus(None, NavigationMethod::Unspecified, KeyModifiers::NONE);
                }
            }
        }

        fn on_pointer_entered(&self, e: Option<&IPointerRoutedEventArgs>) {
            let Some(e) = e else { return };
            // Touch never hovers — enter is implied by the press that produced it.
            if device_type(e) == PointerDeviceType::Touch {
                return;
            }
            let Some((input, input_root)) = self.input() else { return };

            let device = self.get_pointer_device(e);
            let timestamp = self.get_timestamp(e);
            let Some(point) = self.current_point(e) else { return };
            let raw_point = Self::create_raw_pointer_point(&point);
            let modifiers = self.get_pointer_modifiers(e);
            let pointer_id = pointer_id(e);

            input(
                self.create_pointer_args(&device, timestamp, input_root, RawPointerEventType::Move, raw_point, modifiers, pointer_id)
                    .into_input(),
            );
        }

        fn on_pointer_exited(&self, e: Option<&IPointerRoutedEventArgs>) {
            let Some(e) = e else { return };
            if device_type(e) == PointerDeviceType::Touch {
                return;
            }
            let Some((input, input_root)) = self.input() else { return };

            let device = self.get_pointer_device(e);
            let timestamp = self.get_timestamp(e);
            let Some(point) = self.current_point(e) else { return };
            let raw_point = Self::create_raw_pointer_point(&point);
            let modifiers = self.get_pointer_modifiers(e);
            let pointer_id = pointer_id(e);

            input(
                self.create_pointer_args(
                    &device,
                    timestamp,
                    input_root,
                    RawPointerEventType::LeaveWindow,
                    raw_point,
                    modifiers,
                    pointer_id,
                )
                .into_input(),
            );
        }

        fn on_pointer_canceled(&self, e: Option<&IPointerRoutedEventArgs>) {
            let Some(e) = e else { return };
            let Some((input, input_root)) = self.input() else { return };

            let device = self.get_pointer_device(e);
            let timestamp = self.get_timestamp(e);
            let Some(point) = self.current_point(e) else { return };
            let raw_point = Self::create_raw_pointer_point(&point);
            let modifiers = self.get_pointer_modifiers(e);
            let pointer_id = pointer_id(e);

            let type_ = if device.is_touch() { RawPointerEventType::TouchCancel } else { RawPointerEventType::LeaveWindow };

            input(self.create_pointer_args(&device, timestamp, input_root, type_, raw_point, modifiers, pointer_id).into_input());
            let _ = e.set_handled(1);
        }

        fn on_key_down(&self, e: Option<&IKeyRoutedEventArgs>) {
            if let Some(e) = e {
                self.dispatch_key(e, RawKeyEventType::KeyDown);
            }
        }

        fn on_key_up(&self, e: Option<&IKeyRoutedEventArgs>) {
            if let Some(e) = e {
                self.dispatch_key(e, RawKeyEventType::KeyUp);
            }
        }

        fn dispatch_key(&self, e: &IKeyRoutedEventArgs, type_: RawKeyEventType) {
            let Some((input, input_root)) = self.input() else { return };

            let (Ok(virtual_key), Ok(key_status)) = (e.key(), e.key_status()) else { return };
            let (key, physical_key, key_symbol) = WinUIKeyInterop::resolve(virtual_key, key_status);
            if key == Key::None && physical_key == PhysicalKey::None {
                return;
            }

            // Tab boundary: when the framework would wrap focus inside its
            // top-level (TopLevelHost.TabNavigation = Cycle), forward to WinUI
            // so focus escapes to the next/previous native control instead.
            if type_ == RawKeyEventType::KeyDown && key == Key::Tab && self.try_escape_tab_boundary(e) {
                return;
            }

            let Some(keyboard) = Self::get_keyboard_device() else { return };

            let args = Rc::new(RawKeyEventArgs::new(
                keyboard,
                tick_count(),
                input_root,
                type_,
                key,
                Self::get_current_modifiers(),
                physical_key,
                key_symbol,
                KeyDeviceType::Keyboard,
            ));
            input(args.clone());

            if type_ == RawKeyEventType::KeyDown {
                self.ignore_character_received.set(key == Key::ImeProcessed || args.handled());
            } else if type_ == RawKeyEventType::KeyUp {
                self.ignore_character_received.set(false);
            }

            // Only mark handled if the framework consumed the event — marking a
            // KeyDown handled suppresses the matching CharacterReceived and
            // breaks text input.
            if args.handled() {
                let _ = e.set_handled(1);
            }
        }

        /// If the current focus of the framework is at the first or last tab
        /// stop in the embedded content, handle Tab / Shift+Tab here by moving
        /// WinUI focus to the next or previous native element. Returns true if
        /// the event was consumed (the caller must not forward it to the
        /// framework).
        fn try_escape_tab_boundary(&self, e: &IKeyRoutedEventArgs) -> bool {
            let Some(root) = self.root.borrow().clone() else {
                return false;
            };

            let focus_manager = FocusManager::get_focus_manager(&root);
            let Some(current) = focus_manager.and_then(|focus_manager| focus_manager.get_focused_element()) else {
                return false;
            };

            let shift = key_state_for_current_thread(VirtualKey::Shift).is_some_and(is_down);

            let root_element = root.clone().upcast::<ferroui_base::input::InputElement>();
            let Some(first) = KeyboardNavigationHandler::get_next(&root_element, NavigationDirection::Next) else {
                return false;
            };

            let at_boundary = if shift {
                // At the first tab stop — backward tab would wrap.
                current.ptr_eq(&first)
            } else {
                // At the last tab stop — forward tab would wrap. Cycle mode means
                // GetNext from the last returns the first; detect that.
                let next = KeyboardNavigationHandler::get_next(&current, NavigationDirection::Next);
                next.is_none_or(|next| next.ptr_eq(&first))
            };

            if !at_boundary {
                return false;
            }

            let direction = if shift { FocusNavigationDirection::Previous } else { FocusNavigationDirection::Next };

            // WinUI Desktop requires SearchRoot — the parameterless TryMoveFocus
            // overload only works in UWP. Use the panel's XamlRoot.Content as
            // the search scope; that covers the whole window's visual tree.
            let search_root = self
                .element
                .xaml_root()
                .ok()
                .flatten()
                .and_then(|xaml_root| xaml_root.content().ok().flatten())
                .and_then(|content| content.cast::<IDependencyObject>().ok());
            let Some(search_root) = search_root else {
                log(LogEventLevel::Verbose, "Tab boundary: no XamlRoot.Content available; letting the framework handle.");
                return false;
            };

            let moved = (|| -> Result<bool, HResult> {
                let options = crate::interop::win_rt::activate::<IFindNextElementOptions>("Microsoft.UI.Xaml.Input.FindNextElementOptions")?;
                options.set_search_root(Some(&search_root))?;
                let statics = factory::<IFocusManagerStatics>("Microsoft.UI.Xaml.Input.FocusManager")?;
                Ok(flag(statics.try_move_focus_with_options(direction, Some(&options))))
            })()
            .unwrap_or(false);
            if moved {
                let _ = e.set_handled(1);
                log(LogEventLevel::Verbose, &format!("Tab boundary: moved WinUI focus {direction:?}."));
                return true;
            }

            log(LogEventLevel::Verbose, "Tab boundary detected but TryMoveFocus returned false; letting the framework handle.");
            false
        }

        fn on_character_received(&self, e: Option<&ICharacterReceivedRoutedEventArgs>) {
            let Some(e) = e else { return };
            if self.ignore_character_received.get() {
                return;
            }

            let Some((input, input_root)) = self.input() else { return };

            let Some(keyboard) = Self::get_keyboard_device() else { return };

            let Ok(ch) = e.character() else { return };
            let mut pending = self.pending_high_surrogate.get();
            let text = received_text(&mut pending, ch);
            self.pending_high_surrogate.set(pending);
            if let Some(text) = text {
                input(Rc::new(RawTextInputEventArgs::new(keyboard, tick_count(), input_root, text)));
                let _ = e.set_handled(1);
            }
        }

        fn on_drag_enter(&self, e: Option<&IDragEventArgs>) {
            let Some(e) = e else { return };
            let Ok(Some(deferral)) = e.get_deferral() else { return };
            let e = ComPtr::from_ref(e);
            let this = self.this.clone();
            // `async void` of the reference: the task runs on the dispatcher
            // of the UI thread up to its first await before this returns.
            Dispatcher::ui_thread().start_local(async move {
                let previous = this.upgrade().and_then(|panel| panel.active_drag.borrow_mut().take());
                if let Some(previous) = previous {
                    previous.dispose();
                }
                let data_view = e.data_view().ok().flatten();
                if let Some(data_view) = data_view {
                    let data_transfer = Self::build_data_transfer_async(data_view).await;
                    if let Some(panel) = this.upgrade() {
                        *panel.active_drag.borrow_mut() = Some(data_transfer);
                        panel.update_drag_ui(&e, RawDragEventType::DragEnter);
                    }
                }
                let _ = deferral.complete();
            });
        }

        fn on_drag_over(&self, e: Option<&IDragEventArgs>) {
            if let Some(e) = e {
                self.update_drag_ui(e, RawDragEventType::DragOver);
            }
        }

        fn on_drag_leave(&self, e: Option<&IDragEventArgs>) {
            if let Some(e) = e {
                self.update_drag_ui(e, RawDragEventType::DragLeave);
            }
            if let Some(active_drag) = self.active_drag.borrow_mut().take() {
                active_drag.dispose();
            }
        }

        fn on_drop(&self, e: Option<&IDragEventArgs>) {
            if let Some(e) = e {
                self.update_drag_ui(e, RawDragEventType::Drop);
            }
            if let Some(active_drag) = self.active_drag.borrow_mut().take() {
                active_drag.dispose();
            }
        }

        fn update_drag_ui(&self, e: &IDragEventArgs, type_: RawDragEventType) {
            let Some((input, input_root)) = self.input() else { return };
            let Some(active_drag) = self.active_drag.borrow().clone() else { return };

            let Some(device) = FerroLocator::current().get_service::<dyn IDragDropDevice>() else {
                return;
            };

            let pt = e.get_position(Some(&self.element)).unwrap_or(Point { x: 0.0, y: 0.0 });
            let mut allowed = DragDropEffects::from_bits_truncate(
                e.allowed_operations().unwrap_or(DataPackageOperation::None).0,
            );
            if allowed == DragDropEffects::NONE {
                allowed = DragDropEffects::COPY | DragDropEffects::MOVE | DragDropEffects::LINK;
            }
            let modifiers = Self::get_current_modifiers();

            let data_transfer: Rc<dyn IDataTransfer> = active_drag;
            let args = Rc::new(RawDragEvent::new(
                device,
                type_,
                input_root,
                FerroPoint::new(f64::from(pt.x), f64::from(pt.y)),
                data_transfer,
                allowed,
                modifiers,
            ));
            input(args.clone());

            let _ = e.set_accepted_operation(DataPackageOperation(args.effects().bits()));
            let _ = e.set_handled(1);
        }

        async fn build_data_transfer_async(view: ComPtr<IDataPackageView>) -> Rc<DataTransfer> {
            let dt = DataTransfer::new();
            let formats = factory::<IStandardDataFormatsStatics>("Windows.ApplicationModel.DataTransfer.StandardDataFormats").ok();

            let storage_items = formats.as_ref().and_then(|formats| formats.storage_items().ok()).map(take_string);
            if let Some(storage_items) = storage_items {
                if contains(&view, &storage_items) {
                    let items = match view.get_storage_items_async() {
                        Ok(Some(operation)) => wait(operation).await,
                        Ok(None) => Err(HResult::POINTER),
                        Err(error) => Err(error),
                    };
                    match items {
                        Ok(Some(items)) => {
                            let count = items.size().unwrap_or(0);
                            for index in 0..count {
                                let Ok(Some(item)) = items.get_at(index) else { continue };
                                let path = item.path().map(take_string).unwrap_or_default();
                                if path.is_empty() {
                                    continue;
                                }
                                // Reuse the framework's BclStorage* via the same
                                // helper Win32 uses.
                                if let Some(storage) = StorageProviderHelpers::try_create_bcl_storage_item(Some(&path)) {
                                    let storage: Rc<dyn FerroStorageItem> = match storage {
                                        BclStorageItemHandle::Folder(folder) => folder,
                                        BclStorageItemHandle::File(file) => file,
                                    };
                                    dt.add(DataTransferItem::create_file(Some(storage)));
                                }
                            }
                        }
                        Ok(None) => {}
                        Err(error) => warn(None, &format!("Failed to resolve dragged storage items: {error}")),
                    }
                }
            }

            let text_format = formats.as_ref().and_then(|formats| formats.text().ok()).map(take_string);
            if let Some(text_format) = text_format {
                if contains(&view, &text_format) {
                    let text = match view.get_text_async() {
                        Ok(Some(operation)) => wait(operation).await.map(take_string),
                        Ok(None) => Err(HResult::POINTER),
                        Err(error) => Err(error),
                    };
                    match text {
                        Ok(text) if !text.is_empty() => dt.add(DataTransferItem::create_text(Some(&text))),
                        Ok(_) => {}
                        Err(error) => warn(None, &format!("Failed to resolve dragged text: {error}")),
                    }
                }
            }

            dt
        }

        // Outgoing drag — invoked by WinUIDragSource on the UI thread.
        pub(crate) async fn start_outgoing_drag_async(&self, data: Rc<dyn IDataTransfer>, allowed: DragDropEffects) -> DragDropEffects {
            let Some(pp) = self.last_pointer_point.borrow().clone() else {
                warn(Some(self), "StartOutgoingDragAsync called with no cached PointerPoint; drag suppressed.");
                return DragDropEffects::NONE;
            };

            *self.outgoing_drag_data.borrow_mut() = Some(data);
            self.outgoing_drag_allowed.set(allowed);
            let result = match self.cursor_overlay.element().and_then(|overlay| overlay.start_drag_async(Some(&pp))) {
                Ok(Some(operation)) => wait(operation).await,
                Ok(None) => Err(HResult::POINTER),
                Err(error) => Err(error),
            };
            *self.outgoing_drag_data.borrow_mut() = None;
            self.outgoing_drag_allowed.set(DragDropEffects::NONE);
            match result {
                Ok(op) => DragDropEffects::from_bits_truncate(op.0),
                Err(error) => {
                    warn(Some(self), &format!("StartDragAsync threw: {error}"));
                    DragDropEffects::NONE
                }
            }
        }

        fn on_drag_starting(&self, e: Option<&IDragStartingEventArgs>) {
            let Some(e) = e else { return };
            let Some(outgoing_drag_data) = self.outgoing_drag_data.borrow().clone() else {
                // The overlay was the drag origin (CanDrag=true) but no drag of
                // the framework was in flight — most likely a user grab on
                // empty panel space. Suppress the native drag rather than emit
                // an empty one.
                log(LogEventLevel::Verbose, "DragStarting cancelled: no active drag of the framework.");
                let _ = e.set_cancel(1);
                return;
            };

            let _ = e.set_allowed_operations(DataPackageOperation(self.outgoing_drag_allowed.get().bits()));
            let Ok(Some(deferral)) = e.get_deferral() else { return };
            let Ok(Some(package)) = e.data() else {
                let _ = deferral.complete();
                return;
            };
            let this = self.this.clone();
            Dispatcher::ui_thread().start_local(async move {
                if let Err(error) = Self::populate_data_package_async(package, outgoing_drag_data).await {
                    let panel = this.upgrade();
                    warn(panel.as_deref(), &format!("Populating outgoing DataPackage threw: {error}"));
                }
                let _ = deferral.complete();
            });
        }

        async fn populate_data_package_async(package: ComPtr<IDataPackage>, source: Rc<dyn IDataTransfer>) -> Result<(), HResult> {
            // Text — direct copy.
            let text = source.try_get_value(&DataFormat::text());
            if let Some(text) = text.filter(|text| !text.is_empty()) {
                package.set_text(hstring(&text)?.handle())?;
            }

            // Files — convert the framework's storage item paths to native
            // Windows.Storage.IStorageItem instances via the OS APIs.
            let ferro_files = source.try_get_values(&DataFormat::file());
            if let Some(ferro_files) = ferro_files {
                let mut win_ui_items = Vec::new();
                for ferro_file in ferro_files {
                    let uri = ferro_file.path();
                    let path = if uri.is_absolute_uri() && uri.scheme() == "file" { uri.local_path() } else { String::new() };
                    if path.is_empty() {
                        continue;
                    }
                    let item: Result<ComPtr<IStorageItem>, HResult> = async {
                        if std::path::Path::new(&path).is_dir() {
                            let statics = factory::<IStorageFolderStatics>("Windows.Storage.StorageFolder")?;
                            let operation = statics.get_folder_from_path_async(hstring(&path)?.handle())?.ok_or(HResult::POINTER)?;
                            wait(operation).await?.ok_or(HResult::POINTER)?.cast::<IStorageItem>()
                        } else {
                            let statics = factory::<IStorageFileStatics>("Windows.Storage.StorageFile")?;
                            let operation = statics.get_file_from_path_async(hstring(&path)?.handle())?.ok_or(HResult::POINTER)?;
                            wait(operation).await?.ok_or(HResult::POINTER)?.cast::<IStorageItem>()
                        }
                    }
                    .await;
                    match item {
                        Ok(item) => win_ui_items.push(item),
                        Err(error) => warn(
                            None,
                            &format!("Failed to expose dragged path '{path}' as Windows.Storage item: {error}"),
                        ),
                    }
                }
                if !win_ui_items.is_empty() {
                    package.set_storage_items_read_only(Some(&StorageItems::new(win_ui_items)))?;
                }
            }
            Ok(())
        }

        fn get_keyboard_device() -> Option<Rc<dyn IInputDevice>> {
            let keyboard: Rc<dyn IKeyboardDevice> = FerroLocator::current().get_service::<dyn IKeyboardDevice>()?;
            Some(keyboard)
        }

        fn get_current_modifiers() -> RawInputModifiers {
            let mut mods = RawInputModifiers::NONE;
            if key_state_for_current_thread(VirtualKey::Control).is_some_and(is_down) {
                mods |= RawInputModifiers::CONTROL;
            }
            if key_state_for_current_thread(VirtualKey::Shift).is_some_and(is_down) {
                mods |= RawInputModifiers::SHIFT;
            }
            if key_state_for_current_thread(VirtualKey::Menu).is_some_and(is_down) {
                mods |= RawInputModifiers::ALT;
            }
            mods
        }
    }

    impl Drop for FerroSwapChainPanel {
        /// The application let go of the panel: what `Unloaded` does, since
        /// nothing will render to the element any more.
        fn drop(&mut self) {
            PANELS_BY_ID.with(|panels| panels.borrow_mut().remove(&self.id));
            self.on_unloaded();
        }
    }

    /// Runs `body` with the panel while it is alive.
    fn with(this: &Weak<FerroSwapChainPanel>, body: impl FnOnce(&FerroSwapChainPanel)) {
        if let Some(panel) = this.upgrade() {
            body(&panel);
        }
    }

    fn device_type(e: &IPointerRoutedEventArgs) -> PointerDeviceType {
        e.pointer()
            .ok()
            .flatten()
            .and_then(|pointer| pointer.pointer_device_type().ok())
            .unwrap_or(PointerDeviceType::Mouse)
    }

    fn pointer_id(e: &IPointerRoutedEventArgs) -> u32 {
        e.pointer().ok().flatten().and_then(|pointer| pointer.pointer_id().ok()).unwrap_or(0)
    }

    fn contains(view: &IDataPackageView, format: &str) -> bool {
        hstring(format).map(|format| flag(view.contains(format.handle()))).unwrap_or(false)
    }

    /// `InputKeyboardSource.GetKeyStateForCurrentThread`.
    pub(crate) fn key_state_for_current_thread(virtual_key: VirtualKey) -> Option<CoreVirtualKeyStates> {
        let statics = factory::<IInputKeyboardSourceStatics>("Microsoft.UI.Input.InputKeyboardSource").ok()?;
        statics.get_key_state_for_current_thread(virtual_key).ok()
    }

    fn log(level: LogEventLevel, message: &str) {
        if let Some(logger) = Logger::try_get(level, LogArea::WINUI_PLATFORM) {
            logger.log(None, message);
        }
    }

    fn warn(source: Option<&FerroSwapChainPanel>, message: &str) {
        if let Some(logger) = Logger::try_get(LogEventLevel::Warning, LogArea::WINUI_PLATFORM) {
            logger.log(source.map(|source| source as &dyn Any), message);
        }
    }

    /// `Enum.GetValues<StandardCursorType>()`, in the order of the enumeration.
    const ALL_CURSOR_TYPES: [StandardCursorType; 24] = [
        StandardCursorType::Arrow,
        StandardCursorType::Ibeam,
        StandardCursorType::Wait,
        StandardCursorType::Cross,
        StandardCursorType::UpArrow,
        StandardCursorType::SizeWestEast,
        StandardCursorType::SizeNorthSouth,
        StandardCursorType::SizeAll,
        StandardCursorType::No,
        StandardCursorType::Hand,
        StandardCursorType::AppStarting,
        StandardCursorType::Help,
        StandardCursorType::TopSide,
        StandardCursorType::BottomSide,
        StandardCursorType::LeftSide,
        StandardCursorType::RightSide,
        StandardCursorType::TopLeftCorner,
        StandardCursorType::TopRightCorner,
        StandardCursorType::BottomLeftCorner,
        StandardCursorType::BottomRightCorner,
        StandardCursorType::DragMove,
        StandardCursorType::DragCopy,
        StandardCursorType::DragLink,
        StandardCursorType::None,
    ];
}

#[cfg(test)]
mod tests {
    // Not from upstream: the reference has no tests of the integration.
    use super::*;

    #[test]
    fn the_raw_event_of_a_press_is_the_first_button_pressed() {
        assert_eq!(Some(RawPointerEventType::LeftButtonDown), pressed_event_type(true, true, true));
        assert_eq!(Some(RawPointerEventType::RightButtonDown), pressed_event_type(false, true, true));
        assert_eq!(Some(RawPointerEventType::MiddleButtonDown), pressed_event_type(false, false, true));
        assert_eq!(None, pressed_event_type(false, false, false));
    }

    #[test]
    fn the_raw_event_of_a_release_is_the_kind_of_the_update() {
        assert_eq!(Some(RawPointerEventType::LeftButtonUp), released_event_type(PointerUpdateKind::LeftButtonReleased));
        assert_eq!(Some(RawPointerEventType::RightButtonUp), released_event_type(PointerUpdateKind::RightButtonReleased));
        assert_eq!(Some(RawPointerEventType::MiddleButtonUp), released_event_type(PointerUpdateKind::MiddleButtonReleased));
        assert_eq!(None, released_event_type(PointerUpdateKind::XButton1Released));
        assert_eq!(None, released_event_type(PointerUpdateKind::Other));
    }

    #[test]
    fn a_received_character_is_text_unless_it_is_a_control() {
        let mut pending = None;
        assert_eq!(Some("a".to_string()), received_text(&mut pending, u16::from(b'a')));
        assert_eq!(Some("\r".to_string()), received_text(&mut pending, 0x0D));
        assert_eq!(Some("\n".to_string()), received_text(&mut pending, 0x0A));
        assert_eq!(Some("\t".to_string()), received_text(&mut pending, 0x09));
        assert_eq!(None, received_text(&mut pending, 0x08));
        assert_eq!(None, received_text(&mut pending, 0x1B));
        assert_eq!(None, received_text(&mut pending, 0x7F));
        assert_eq!(None, received_text(&mut pending, 0x85));
        assert_eq!(Some("\u{e9}".to_string()), received_text(&mut pending, 0xE9));
    }

    #[test]
    fn a_surrogate_pair_is_one_text() {
        let mut pending = None;
        // U+1F600 is D83D DE00.
        assert_eq!(None, received_text(&mut pending, 0xD83D));
        assert_eq!(Some(0xD83D), pending);
        assert_eq!(Some("\u{1F600}".to_string()), received_text(&mut pending, 0xDE00));
        assert_eq!(None, pending);
        // A second half alone is nothing.
        assert_eq!(None, received_text(&mut pending, 0xDE00));
    }
}

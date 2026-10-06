use super::PresentationSource;
use ferroui_base::layout::{ILayoutManager, ILayoutRoot, LayoutManager, Layoutable};
use ferroui_base::reactive::IDisposable;
use ferroui_base::rendering::{RendererDebugOverlays, RendererDiagnostics};
use ferroui_base::Ref;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

impl PresentationSource {
    /// The scaling factor to use in layout.
    pub fn layout_scaling(&self) -> f64 {
        self.render_scaling()
    }

    /// The layout manager of the tree.
    pub fn layout_manager(&self) -> &Rc<LayoutManager> {
        &self.layout_manager
    }

    pub(super) fn create_layout_manager(owner: Weak<dyn ILayoutRoot>) -> Rc<LayoutManager> {
        LayoutManager::new(owner)
    }

    /// The second half of C# `CreateLayoutManager`: the bridge from the
    /// layout manager to the diagnostics of the renderer. The layout manager
    /// is created with the source, before the renderer exists, so the bridge
    /// is created once the renderer has been.
    pub(super) fn create_layout_diagnostic_bridge(&self) {
        let bridge = LayoutDiagnosticBridge::new(self.typed_renderer().diagnostics(), &self.layout_manager);
        bridge.setup_bridge();
        *self.layout_diagnostic_bridge.borrow_mut() = Some(bridge);
    }
}

impl ILayoutRoot for PresentationSource {
    fn layout_scaling(&self) -> f64 {
        PresentationSource::layout_scaling(self)
    }

    fn layout_manager(&self) -> Rc<dyn ILayoutManager> {
        self.layout_manager.clone()
    }

    fn root_visual(&self) -> Ref<Layoutable> {
        self.root_element().upcast()
    }
}

/// Provides layout pass timing from the layout manager to the renderer, for
/// diagnostics purposes.
pub(super) struct LayoutDiagnosticBridge {
    state: Rc<BridgeState>,
    property_changed_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
}

struct BridgeState {
    diagnostics: Rc<RendererDiagnostics>,
    layout_manager: Weak<LayoutManager>,
    is_handling: Cell<bool>,
}

impl LayoutDiagnosticBridge {
    pub fn new(diagnostics: Rc<RendererDiagnostics>, layout_manager: &Rc<LayoutManager>) -> Self {
        let state = Rc::new(BridgeState {
            diagnostics: diagnostics.clone(),
            layout_manager: Rc::downgrade(layout_manager),
            is_handling: Cell::new(false),
        });

        let weak = Rc::downgrade(&state);
        let subscription = diagnostics.property_changed(move |_, property_name| {
            if let Some(state) = weak.upgrade() {
                state.on_diagnostics_property_changed(property_name);
            }
        });

        Self { state, property_changed_subscription: RefCell::new(Some(subscription)) }
    }

    pub fn setup_bridge(&self) {
        self.state.setup_bridge();
    }

    pub fn dispose(&self) {
        if let Some(subscription) = self.property_changed_subscription.borrow_mut().take() {
            subscription.dispose();
        }
        if let Some(layout_manager) = self.state.layout_manager.upgrade() {
            layout_manager.set_layout_pass_timed(None);
        }
    }
}

impl BridgeState {
    fn setup_bridge(&self) {
        let needs_handling = self.diagnostics.debug_overlays().contains(RendererDebugOverlays::LAYOUT_TIME_GRAPH);
        if needs_handling != self.is_handling.get() {
            self.is_handling.set(needs_handling);
            let Some(layout_manager) = self.layout_manager.upgrade() else { return };
            layout_manager.set_layout_pass_timed(if needs_handling {
                let diagnostics = Rc::downgrade(&self.diagnostics);
                Some(Rc::new(move |timing| {
                    if let Some(diagnostics) = diagnostics.upgrade() {
                        diagnostics.set_last_layout_pass_timing(timing);
                    }
                }))
            } else {
                None
            });
        }
    }

    fn on_diagnostics_property_changed(&self, property_name: &str) {
        if property_name == RendererDiagnostics::DEBUG_OVERLAYS_PROPERTY_NAME {
            self.setup_bridge();
        }
    }
}

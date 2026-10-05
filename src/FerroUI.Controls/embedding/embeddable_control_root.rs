use crate::platform::{ITopLevelImpl, PlatformManager};
use crate::primitives::TemplatedControlImpl;
use crate::{ContentControlImpl, ControlImpl, TopLevel, TopLevelImpl};
use ferroui_base::input::{IFocusScope, InputElementImpl};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{ILayoutManager, LayoutableImpl, LayoutableImplExt};
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Ref, Size, StaticType, StyledElementImpl, TypeInfo,
    VisualImpl,
};
use std::cell::Cell;
use std::rc::Rc;

/// A top-level that hosts a control tree inside a surface provided by
/// someone else: a native control of another toolkit, a single-view
/// platform, an offscreen buffer.
#[repr(C)]
pub struct EmbeddableControlRoot {
    base: TopLevel,
    enforce_client_size: Cell<bool>,
}

ferro_class! {
    EmbeddableControlRoot: TopLevel, virtuals EmbeddableControlRootImpl: TopLevelImpl {
        /// Releases the platform implementation and tears the root down.
        fn dispose(this);
    }
}
ferroui_base::ferro_class_info!(EmbeddableControlRoot { new: EmbeddableControlRoot::new });

ferro_impl_classes!(
    EmbeddableControlRoot: FerroObjectImpl,
    VisualImpl,
    InteractiveImpl,
    ContentControlImpl,
    TopLevelImpl
);

impl ControlImpl for EmbeddableControlRoot {
    fn on_create_automation_peer(this: &Self) -> Ref<crate::automation::peers::AutomationPeer> {
        crate::automation::peers::EmbeddableControlRootAutomationPeer::new(this).upcast()
    }
}

impl TemplatedControlImpl for EmbeddableControlRoot {
    fn on_apply_template(this: &Self, e: &crate::primitives::TemplateAppliedEventArgs) {
        use crate::primitives::TemplatedControlImplExt;
        Self::parent_on_apply_template(this, e);
        this.enable_visual_layer_manager_layers();
    }
}

impl IFocusScope for EmbeddableControlRoot {}

impl InputElementImpl for EmbeddableControlRoot {
    fn is_focus_scope(_this: &Self) -> bool {
        true
    }
}

impl StyledElementImpl for EmbeddableControlRoot {
    fn style_key_override(_this: &Self) -> &'static TypeInfo {
        <EmbeddableControlRoot as StaticType>::TYPE
    }
}

impl LayoutableImpl for EmbeddableControlRoot {
    fn measure_override(this: &Self, mut available_size: Size) -> Size {
        if this.enforce_client_size.get() {
            available_size = this.platform_impl().map(|platform_impl| platform_impl.client_size()).unwrap_or_default();
        }
        let rv = Self::parent_measure_override(this, available_size);
        if this.enforce_client_size.get() {
            return available_size;
        }
        rv
    }
}

impl EmbeddableControlRootImpl for EmbeddableControlRoot {
    fn dispose(this: &Self) {
        if let Some(platform_impl) = this.platform_impl() {
            platform_impl.dispose();
        }
        this.ensure_closed();
    }
}

impl EmbeddableControlRoot {
    /// Field initialisation of a root over a platform implementation.
    pub fn construct(platform_impl: Rc<dyn ITopLevelImpl>) -> Self {
        Self { base: TopLevel::construct(platform_impl), enforce_client_size: Cell::new(true) }
    }

    /// Creates a root over a platform implementation.
    pub fn with_impl(platform_impl: Rc<dyn ITopLevelImpl>) -> Ref<Self> {
        instantiate(Self::construct(platform_impl))
    }

    /// Creates a root over an embeddable top-level of the windowing
    /// platform.
    pub fn new() -> Ref<Self> {
        Self::with_impl(PlatformManager::create_embeddable_top_level())
    }

    /// Whether the root is always measured with, and sized to, the client
    /// size of the platform implementation (protected upstream: for
    /// derived classes).
    pub fn enforce_client_size(&self) -> bool {
        self.enforce_client_size.get()
    }

    pub fn set_enforce_client_size(&self, value: bool) {
        self.enforce_client_size.set(value)
    }

    /// Initializes the root, applies its template and runs the initial
    /// layout pass.
    pub fn prepare(&self) {
        self.ensure_initialized();
        self.apply_template();
        self.layout_manager().execute_initial_layout_pass();
    }

    fn ensure_initialized(&self) {
        if !self.is_initialized() {
            self.begin_init();
            self.end_init();
        }
    }
}

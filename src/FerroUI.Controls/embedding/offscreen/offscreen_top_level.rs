use super::OffscreenTopLevelImplBase;
use crate::embedding::EmbeddableControlRoot;
use crate::platform::ITopLevelImpl;
use crate::primitives::TemplatedControlImpl;
use crate::{ContentControlImpl, ControlImpl, TopLevel, TopLevelImpl};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{ILayoutManager, LayoutableImpl};
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, FerroObjectImplExt, Ref, StaticType, StyledElementImpl, TypeInfo,
    VisualImpl,
};
use std::rc::Rc;

/// A top-level over an offscreen implementation: laid out and rendered
/// without a window of the platform.
#[repr(C)]
pub struct OffscreenTopLevel {
    base: TopLevel,
    platform_impl: Rc<OffscreenTopLevelImplBase>,
}

ferro_class!(OffscreenTopLevel: TopLevel);
ferroui_base::ferro_class_info!(OffscreenTopLevel {});

ferro_impl_classes!(
    OffscreenTopLevel: VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl,
    TopLevelImpl
);

impl FerroObjectImpl for OffscreenTopLevel {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.prepare();
    }
}

impl StyledElementImpl for OffscreenTopLevel {
    fn style_key_override(_this: &Self) -> &'static TypeInfo {
        <EmbeddableControlRoot as StaticType>::TYPE
    }
}

impl OffscreenTopLevel {
    /// Field initialisation of a top-level over an offscreen implementation.
    pub fn construct(platform_impl: Rc<OffscreenTopLevelImplBase>) -> Self {
        let top_level_impl: Rc<dyn ITopLevelImpl> = platform_impl.clone();
        Self { base: TopLevel::construct(top_level_impl), platform_impl }
    }

    /// Creates a top-level over an offscreen implementation and prepares
    /// it.
    pub fn new(platform_impl: Rc<OffscreenTopLevelImplBase>) -> Ref<Self> {
        instantiate(Self::construct(platform_impl))
    }

    /// The offscreen implementation of the top-level (C# `Impl`).
    pub fn offscreen_impl(&self) -> &Rc<OffscreenTopLevelImplBase> {
        &self.platform_impl
    }

    /// Initializes the top-level, applies its template and runs the
    /// initial layout pass.
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

    /// Releases the platform implementation and tears the top-level down.
    pub fn dispose(&self) {
        if let Some(platform_impl) = self.platform_impl() {
            platform_impl.dispose();
        }
        self.ensure_closed();
    }
}

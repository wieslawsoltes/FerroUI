//! WPF's `Viewport3D` (`System.Windows.Controls.Viewport3D`): the control that shows a 3D
//! scene in the 2D layout.

use crate::{Camera, Scene3DChange, Viewport3DVisual, Visual3DCollection};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::DrawingContext;
use ferroui_base::reactive::Subscription;
use ferroui_base::rendering::scene3d::Scene3DSnapshot;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, FerroObjectImpl,
    FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Rect, Ref, Size, StyledElementImpl,
    StyledProperty, Visual, VisualImpl,
};
use ferroui_controls::{Control, ControlImpl};
use std::cell::RefCell;
use std::sync::Arc;

/// WPF's `Viewport3D`: the scene of its `Camera` and `Children` (the visuals of a
/// [`Viewport3DVisual`], which this control hosts), drawn over its arranged bounds.
///
/// It draws nothing where no model is (it has no background), clips to its bounds by default
/// (`ClipToBounds` is true, as in WPF), and composes like any element under its transform,
/// clip, opacity and effects: it records one `DrawScene3D` operation with the snapshot of its
/// scene. Its desired size is zero unless a size is set, as WPF's.
#[repr(C)]
pub struct Viewport3D {
    base: Control,
    visual: Ref<Viewport3DVisual>,
    visual_subscription: RefCell<Option<Subscription>>,
}

ferro_class!(Viewport3D: Control);
ferro_class_info!(Viewport3D {
    new: Viewport3D::new,
    markup: {
        namespace: "FerroUI.Controls",
        content: Children,
        properties: [
            Children: Visual3DCollection { get: Viewport3D::children },
        ],
    },
});
ferro_impl_classes!(Viewport3D: StyledElementImpl, InteractiveImpl, InputElementImpl, ControlImpl);

impl FerroObjectImpl for Viewport3D {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        let weak = this.to_ref().downgrade();
        let subscription = this.visual.changed(move |_| {
            if let Some(this) = weak.upgrade() {
                this.invalidate_visual();
            }
        });
        *this.visual_subscription.borrow_mut() = Some(subscription);
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if change.property() == Self::camera_property().as_property() {
            this.visual.set_camera(this.camera());
        }
    }
}

impl LayoutableImpl for Viewport3D {
    fn arrange_override(this: &Self, final_size: Size) -> Size {
        // The viewport of the scene is the arranged area: the camera takes its aspect ratio.
        this.visual.set_viewport(Rect::from_size(final_size));
        final_size
    }
}

impl VisualImpl for Viewport3D {
    fn render(this: &Self, context: &mut DrawingContext) {
        let size = this.bounds().size();
        let scene = this.visual.snapshot();
        context.draw_scene_3d(&scene, Rect::from_size(size));
    }
}

ferro_properties! { impl Viewport3D {
    pub fn camera_property() -> StyledProperty<Option<Ref<Camera>>> {
        FerroProperty::register::<Viewport3D, _>("Camera", None)
    }
} }

impl Viewport3D {
    fn static_constructor() {
        Visual::clip_to_bounds_property().override_default_value::<Viewport3D>(true);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: Control::construct(), visual: Viewport3DVisual::new(), visual_subscription: RefCell::new(None) }
    }

    /// An empty viewport without a camera.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The camera of the scene.
    pub fn camera(&self) -> Option<Ref<Camera>> {
        self.get_value(Self::camera_property())
    }

    pub fn set_camera(&self, value: Option<Ref<Camera>>) {
        self.set_value(Self::camera_property(), value)
    }

    /// The visuals of the scene (a shared handle to the collection).
    pub fn children(&self) -> Visual3DCollection {
        self.visual.children()
    }

    /// The scene this control hosts (WPF keeps it private; it is the way to its generations and
    /// to the 3D hit testing).
    pub fn viewport_3d_visual(&self) -> &Ref<Viewport3DVisual> {
        &self.visual
    }

    /// The scene as it is drawn now.
    pub fn snapshot(&self) -> Arc<Scene3DSnapshot> {
        self.visual.snapshot()
    }

    /// Calls `handler` on every change of the scene, until the returned guard is dropped.
    pub fn scene_changed(&self, handler: impl Fn(Scene3DChange) + 'static) -> Subscription {
        self.visual.changed(handler)
    }
}

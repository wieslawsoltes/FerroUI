//! WPF's `Viewport3D` (`System.Windows.Controls.Viewport3D`): the control that shows a 3D
//! scene in the 2D layout.

use super::viewport_3d_input::Viewport3DInput;
use crate::ui_element_3d::PointerEvent3D;
use crate::{Camera, Scene3DChange, UIElement3D, Viewport3DVisual, Visual3DCollection, VisualTreeHelper};
use ferroui_base::input::{
    IPointer, InputElementImpl, InputElementImplExt, PointerCaptureLostEventArgs, PointerEventArgs,
    PointerPressedEventArgs, PointerReleasedEventArgs,
};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::DrawingContext;
use ferroui_base::reactive::Subscription;
use ferroui_base::rendering::scene3d::Scene3DSnapshot;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, FerroObjectImpl,
    FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Rect, Ref, Size, StyledElementImpl,
    Point, StyledProperty, Visual, VisualImpl,
};
use ferroui_controls::{Control, ControlImpl};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

/// WPF's `Viewport3D`: the scene of its `Camera` and `Children` (the visuals of a
/// [`Viewport3DVisual`], which this control hosts), drawn over its arranged bounds.
///
/// It draws nothing where no model is (it has no background), clips to its bounds by default
/// (`ClipToBounds` is true, as in WPF), and composes like any element under its transform,
/// clip, opacity and effects: it records one `DrawScene3D` operation with the snapshot of its
/// scene. Its desired size is zero unless a size is set, as WPF's.
///
/// The 2D hit testing hits it only where one of its models is (its custom hit test is the 3D hit
/// test of [`VisualTreeHelper::hit_test`]), and it routes the pointer events it receives to the
/// [`UIElement3D`]s of its scene (see the documentation of `UIElement3D`).
#[repr(C)]
pub struct Viewport3D {
    base: Control,
    visual: Ref<Viewport3DVisual>,
    visual_subscription: RefCell<Option<Subscription>>,
    input: Viewport3DInput,
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
ferro_impl_classes!(Viewport3D: StyledElementImpl, InteractiveImpl, ControlImpl);

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

    /// Hit where a model of the scene is under the point, as WPF's `Viewport3DVisual`.
    fn custom_hit_test(this: &Self, point: Point) -> Option<bool> {
        Some(VisualTreeHelper::hit_test(this, point).is_some())
    }
}

impl InputElementImpl for Viewport3D {
    fn on_pointer_entered(this: &Self, e: &PointerEventArgs) {
        Self::parent_on_pointer_entered(this, e);
        this.input.route(this, e, None);
    }

    fn on_pointer_exited(this: &Self, e: &PointerEventArgs) {
        Self::parent_on_pointer_exited(this, e);
        this.input.exited(this, e);
    }

    fn on_pointer_moved(this: &Self, e: &PointerEventArgs) {
        Self::parent_on_pointer_moved(this, e);
        this.input.route(this, e, Some(PointerEvent3D::Moved));
    }

    fn on_pointer_pressed(this: &Self, e: &PointerPressedEventArgs) {
        Self::parent_on_pointer_pressed(this, e);
        this.input.route(this, e, Some(PointerEvent3D::Pressed));
    }

    fn on_pointer_released(this: &Self, e: &PointerReleasedEventArgs) {
        Self::parent_on_pointer_released(this, e);
        this.input.route(this, e, Some(PointerEvent3D::Released));
    }

    fn on_pointer_capture_lost(this: &Self, e: &PointerCaptureLostEventArgs) {
        Self::parent_on_pointer_capture_lost(this, e);
        this.input.capture_lost(this, e.pointer());
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
        Self {
            base: Control::construct(),
            visual: Viewport3DVisual::new(),
            visual_subscription: RefCell::new(None),
            input: Viewport3DInput::default(),
        }
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

    /// The element of the scene that captures a pointer through this viewport.
    pub(crate) fn captured_element_3d(&self) -> Option<Ref<UIElement3D>> {
        self.input.captured_element()
    }

    pub(crate) fn capture_3d(&self, element: &Ref<UIElement3D>, pointer: &Rc<dyn IPointer>) -> bool {
        self.input.capture(self, element, pointer)
    }

    pub(crate) fn release_capture_3d(&self, element: &Ref<UIElement3D>, pointer: &Rc<dyn IPointer>) {
        self.input.release(self, element, pointer)
    }
}

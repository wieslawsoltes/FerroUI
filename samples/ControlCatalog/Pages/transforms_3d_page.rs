//! Not from upstream: the page of the 3D transforms of FerroUI's 3D API (`ferroui-media3d`,
//! docs/porting/media3d.md), the class of `Pages/Transforms3DPage.xaml`.
//!
//! The scene is the markup of the document: a large cube scaled, turned and moved by a
//! `Transform3DGroup`, and a small cube moved out and then turned about the centre. The rotations
//! are `AxisAngleRotation3D`s, which are `Animatable`: the page runs an animation of the framework
//! on the angle of each (0 to 360 degrees, repeated) while it is in the visual tree and the box
//! "Animate the rotations" is checked. The other controls set the axis of the large cube, its
//! scale and its offsets.

use crate::markup::{content_page_class, xaml_class};
use ferroui_base::animation::{Animation, Cue, IterationCount, KeyFrame, SignedDuration};
use ferroui_base::interactivity::IRoutedEventArgs;
use ferroui_base::reactive::{Observable, Subscription};
use ferroui_base::styling::Setter;
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::{CheckBox, ComboBox, ContentPage, Slider};
use ferroui_media3d::{
    AxisAngleRotation3D, ModelVisual3D, RotateTransform3D, ScaleTransform3D, Transform3DGroup, TranslateTransform3D,
    Vector3D, Viewport3D,
};
use std::cell::RefCell;
use std::rc::Rc;

/// The axes of the combo box "Axis of the large cube", in its order.
pub const SPIN_AXES: [Vector3D; 4] = [
    Vector3D::new(0.0, 1.0, 0.0),
    Vector3D::new(1.0, 0.0, 0.0),
    Vector3D::new(0.0, 0.0, 1.0),
    Vector3D::new(1.0, 1.0, 1.0),
];

/// A turn of the small cube takes this many turns of the large one.
const ORBIT_TURNS: f64 = 1.5;

/// The objects of the scene the page changes, found in the document once it is loaded.
pub struct Transforms3DScene {
    pub viewport: Ref<Viewport3D>,
    pub scale: Ref<ScaleTransform3D>,
    pub spin: Ref<AxisAngleRotation3D>,
    pub offset: Ref<TranslateTransform3D>,
    pub orbit: Ref<AxisAngleRotation3D>,
}

impl Transforms3DScene {
    /// The objects of the scene of `viewport`, laid out as the document lays them out: the lights
    /// in the first visual, the large cube in the second (scale, rotation, translation), the small
    /// cube in the third (scale, translation, rotation).
    fn find(viewport: Ref<Viewport3D>) -> Option<Self> {
        let visuals = viewport.children().to_vec();
        let transforms = |index: usize| {
            visuals
                .get(index)?
                .cast::<ModelVisual3D>()?
                .transform()?
                .cast::<Transform3DGroup>()
                .map(|group| group.children().to_vec())
        };
        let large = transforms(1)?;
        let small = transforms(2)?;
        let rotation = |transform: Option<&Ref<ferroui_media3d::Transform3D>>| {
            transform?.cast::<RotateTransform3D>()?.rotation()?.cast::<AxisAngleRotation3D>()
        };
        Some(Self {
            scale: large.first()?.cast::<ScaleTransform3D>()?,
            spin: rotation(large.get(1))?,
            offset: large.get(2)?.cast::<TranslateTransform3D>()?,
            orbit: rotation(small.get(2))?,
            viewport,
        })
    }
}

/// An animation of the angle of `rotation` from 0 to 360 degrees in `seconds`, repeated, applied
/// to it: it runs until the returned guard is dropped, which returns the angle to its own value.
pub fn turn(rotation: &Ref<AxisAngleRotation3D>, seconds: f64) -> Subscription {
    let animation = Animation::new();
    animation.set_duration(SignedDuration::from_seconds(seconds));
    animation.set_iteration_count(IterationCount::INFINITE);
    for (cue, angle) in [(0.0, 0.0), (1.0, 360.0)] {
        let key_frame = KeyFrame::new();
        key_frame.set_cue(Cue::new(cue));
        key_frame.setters().add(Setter::new(AxisAngleRotation3D::angle_property(), angle));
        animation.children().add(key_frame);
    }
    Subscription::new(animation.apply(rotation, None, Observable::return_(true), None, true))
}

#[repr(C)]
pub struct Transforms3DPage {
    base: ContentPage,
    // Assigned once the document is loaded, so that the handlers that run while it loads (the
    // initial values of the controls) change nothing.
    scene: RefCell<Option<Rc<Transforms3DScene>>>,
    // The running animations of the two rotations, while the page is in the visual tree.
    animations: RefCell<Vec<Subscription>>,
    // The handlers of the attachment of the page to the visual tree.
    attachment: RefCell<Vec<Subscription>>,
}

content_page_class!(Transforms3DPage);
ferro_class_info!(Transforms3DPage {
    new: Transforms3DPage::new,
    markup: {
        methods: [
            fn AnimationChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<Transforms3DPage>, _sender: Option<BoxedValue>, _e: Rc<dyn IRoutedEventArgs>| {
                    this.restart_animations()
                },
            fn TransformChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<Transforms3DPage>, _sender: Option<BoxedValue>, _e: Rc<dyn IRoutedEventArgs>| {
                    this.apply_transforms()
                },
        ],
    },
});
xaml_class!(Transforms3DPage, "/Pages/Transforms3DPage.xaml");

impl Transforms3DPage {
    pub fn construct() -> Self {
        Self {
            base: ContentPage::construct(),
            scene: RefCell::new(None),
            animations: RefCell::new(Vec::new()),
            attachment: RefCell::new(Vec::new()),
        }
    }

    /// # Panics
    /// Panics if the document does not have the scene and the controls the page uses.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        let viewport = this.get_control::<Viewport3D>("Viewport");
        let scene = Transforms3DScene::find(viewport).expect("the scene of Pages/Transforms3DPage.xaml");
        *this.scene.borrow_mut() = Some(Rc::new(scene));
        this.apply_transforms();

        // The animations run while the page is shown: a page that is not in the visual tree keeps
        // no subscription to the clock. The handlers hold the page weakly.
        let weak = this.downgrade();
        let attached = this.attached_to_visual_tree(move |_| {
            if let Some(this) = weak.upgrade() {
                this.restart_animations();
            }
        });
        let weak = this.downgrade();
        let detached = this.detached_from_visual_tree(move |_| {
            if let Some(this) = weak.upgrade() {
                this.animations.borrow_mut().clear();
            }
        });
        this.attachment.borrow_mut().extend([attached, detached]);
        this
    }

    /// The objects of the scene the page changes.
    pub fn scene(&self) -> Rc<Transforms3DScene> {
        self.scene.borrow().clone().expect("the scene of the page is found when the page is created")
    }

    /// Whether the animations of the rotations run.
    pub fn is_animating(&self) -> bool {
        !self.animations.borrow().is_empty()
    }

    fn slider(&self, name: &str) -> f64 {
        self.get_control::<Slider>(name).value()
    }

    /// Sets the axis, the scale and the offsets of the large cube from the controls.
    fn apply_transforms(&self) {
        let Some(scene) = self.scene.borrow().clone() else { return };
        let index = self.get_control::<ComboBox>("SpinAxis").selected_index();
        let axis = usize::try_from(index).ok().and_then(|index| SPIN_AXES.get(index)).copied();
        scene.spin.set_axis(axis.unwrap_or(SPIN_AXES[0]));
        let scale = self.slider("Scale");
        scene.scale.set_scale_x(scale);
        scene.scale.set_scale_y(scale);
        scene.scale.set_scale_z(scale);
        scene.offset.set_offset_x(self.slider("OffsetX"));
        scene.offset.set_offset_y(self.slider("OffsetY"));
    }

    /// Starts the animations again with the duration of the controls, when the page is in the
    /// visual tree and "Animate the rotations" is checked; stops them otherwise.
    fn restart_animations(&self) {
        let mut animations = self.animations.borrow_mut();
        animations.clear();
        let Some(scene) = self.scene.borrow().clone() else { return };
        let animate = self.get_control::<CheckBox>("Animate").is_checked() == Some(true);
        if !animate || !self.is_attached_to_visual_tree() {
            return;
        }
        let seconds = self.slider("SecondsPerTurn");
        animations.push(turn(&scene.spin, seconds));
        animations.push(turn(&scene.orbit, seconds * ORBIT_TURNS));
    }
}

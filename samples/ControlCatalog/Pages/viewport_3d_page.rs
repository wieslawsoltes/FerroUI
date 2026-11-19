//! Not from upstream: the page of the `Viewport3D` of FerroUI's 3D API (`ferroui-media3d`,
//! docs/porting/media3d.md), the class of `Pages/Viewport3DPage.xaml`.
//!
//! The scene is the markup of the document. The code-behind sets what the controls beside the
//! viewport choose: the angles of the three rotations of the cube, its colour and its specular
//! highlight, the distance and the field of view of the camera, and the colours of the lights.
//! Each is a property of an object of the scene: a change raises a change of the scene, and the
//! viewport draws it again.

use crate::markup::{content_page_class, xaml_class};
use ferroui_base::interactivity::IRoutedEventArgs;
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::{Color, Colors, IBrush};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::{CheckBox, ComboBox, ContentPage, Slider};
use ferroui_media3d::{
    AmbientLight, AxisAngleRotation3D, DiffuseMaterial, DirectionalLight, GeometryModel3D, MaterialGroup, Model3DGroup,
    ModelVisual3D, PerspectiveCamera, Point3D, RotateTransform3D, SpecularMaterial, Transform3DGroup, Viewport3D,
};
use std::cell::RefCell;
use std::rc::Rc;

/// The colours of the combo box "Material colour", in its order.
pub const MATERIAL_COLOURS: [Color; 6] = [
    Color::from_rgb(0xD1, 0x34, 0x38),
    Color::from_rgb(0xF7, 0x63, 0x0C),
    Color::from_rgb(0x10, 0x7C, 0x10),
    Color::from_rgb(0x00, 0x78, 0xD4),
    Color::from_rgb(0x88, 0x17, 0x98),
    Color::from_rgb(0xF0, 0xF0, 0xF0),
];

/// The colour of the ambient light when it is on (as the document has it).
const AMBIENT: Color = Color::from_rgb(0x40, 0x40, 0x40);

/// The direction of the camera (as the document has it): it looks at the centre of the cube from
/// above, and the distance moves it along that line.
const CAMERA_RISE: f64 = 0.25;

/// The objects of the scene the controls change, found in the document once it is loaded.
pub struct Viewport3DScene {
    pub viewport: Ref<Viewport3D>,
    pub camera: Ref<PerspectiveCamera>,
    pub ambient: Ref<AmbientLight>,
    pub directional: Ref<DirectionalLight>,
    pub roll: Ref<AxisAngleRotation3D>,
    pub pitch: Ref<AxisAngleRotation3D>,
    pub yaw: Ref<AxisAngleRotation3D>,
    pub diffuse: Ref<DiffuseMaterial>,
    pub specular: Ref<SpecularMaterial>,
}

impl Viewport3DScene {
    /// The objects of the scene of `viewport`, laid out as the document lays them out: the lights
    /// in the first visual, the floor in the second, the cube in the third.
    fn find(viewport: Ref<Viewport3D>) -> Option<Self> {
        let camera = viewport.camera()?.cast::<PerspectiveCamera>()?;
        let visuals = viewport.children().to_vec();
        let lights = visuals.first()?.cast::<ModelVisual3D>()?.content()?.cast::<Model3DGroup>()?.children().to_vec();
        let ambient = lights.iter().find_map(|light| light.cast::<AmbientLight>())?;
        let directional = lights.iter().find_map(|light| light.cast::<DirectionalLight>())?;
        let cube = visuals.get(2)?.cast::<ModelVisual3D>()?;
        let rotations = cube.transform()?.cast::<Transform3DGroup>()?.children().to_vec();
        let rotation =
            |index: usize| rotations.get(index)?.cast::<RotateTransform3D>()?.rotation()?.cast::<AxisAngleRotation3D>();
        let (roll, pitch, yaw) = (rotation(0)?, rotation(1)?, rotation(2)?);
        let model = cube.content()?.cast::<GeometryModel3D>()?;
        let layers = model.material()?.cast::<MaterialGroup>()?.children().to_vec();
        let diffuse = layers.iter().find_map(|layer| layer.cast::<DiffuseMaterial>())?;
        let specular = layers.iter().find_map(|layer| layer.cast::<SpecularMaterial>())?;
        Some(Self { viewport, camera, ambient, directional, roll, pitch, yaw, diffuse, specular })
    }
}

#[repr(C)]
pub struct Viewport3DPage {
    base: ContentPage,
    // Assigned once the document is loaded, so that the handlers that run while it loads (the
    // initial values of the controls) change nothing.
    scene: RefCell<Option<Rc<Viewport3DScene>>>,
}

content_page_class!(Viewport3DPage);
ferro_class_info!(Viewport3DPage {
    new: Viewport3DPage::new,
    markup: {
        methods: [
            fn SceneChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<Viewport3DPage>, _sender: Option<BoxedValue>, _e: Rc<dyn IRoutedEventArgs>| {
                    this.apply_controls()
                },
        ],
    },
});
xaml_class!(Viewport3DPage, "/Pages/Viewport3DPage.xaml");

fn solid(color: Color) -> Rc<dyn IBrush> {
    Rc::new(ImmutableSolidColorBrush::new(color))
}

impl Viewport3DPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct(), scene: RefCell::new(None) }
    }

    /// # Panics
    /// Panics if the document does not have the scene and the controls the page uses.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        let viewport = this.get_control::<Viewport3D>("Viewport");
        let scene = Viewport3DScene::find(viewport).expect("the scene of Pages/Viewport3DPage.xaml");
        *this.scene.borrow_mut() = Some(Rc::new(scene));
        this.apply_controls();
        this
    }

    /// The objects of the scene the controls change.
    pub fn scene(&self) -> Rc<Viewport3DScene> {
        self.scene.borrow().clone().expect("the scene of the page is found when the page is created")
    }

    fn slider(&self, name: &str) -> f64 {
        self.get_control::<Slider>(name).value()
    }

    fn checked(&self, name: &str) -> bool {
        self.get_control::<CheckBox>(name).is_checked() == Some(true)
    }

    /// Sets the scene from the controls.
    fn apply_controls(&self) {
        let Some(scene) = self.scene.borrow().clone() else { return };
        scene.yaw.set_angle(self.slider("Yaw"));
        scene.pitch.set_angle(self.slider("Pitch"));
        scene.roll.set_angle(self.slider("Roll"));

        let distance = self.slider("Distance");
        scene.camera.set_position(Point3D::new(0.0, distance * CAMERA_RISE, distance));
        scene.camera.set_field_of_view(self.slider("FieldOfView"));

        let index = self.get_control::<ComboBox>("MaterialColour").selected_index();
        let colour = usize::try_from(index).ok().and_then(|index| MATERIAL_COLOURS.get(index)).copied();
        scene.diffuse.set_brush(Some(solid(colour.unwrap_or(MATERIAL_COLOURS[0]))));
        // A material without a brush draws nothing: the highlight is off.
        scene.specular.set_brush(self.checked("Specular").then(|| solid(Colors::WHITE)));

        scene.ambient.set_color(if self.checked("Ambient") { AMBIENT } else { Colors::BLACK });
        scene.directional.set_color(if self.checked("Directional") { Colors::WHITE } else { Colors::BLACK });
    }
}

//! Not from upstream: the page of the interactive 3D elements of FerroUI's 3D API
//! (`ferroui-media3d`, docs/porting/media3d.md, sections 3.16 and 3.17), the class of
//! `Pages/Interaction3DPage.xaml`.
//!
//! The three cubes of the document are `ModelUIElement3D`s in a `ContainerUIElement3D`. The
//! viewport routes its pointer events to them through the 3D hit testing: a cube is lit while the
//! pointer is over it (entered and exited), a press selects it (pressed, with the hit of the
//! press), and the presses bubble to the container, which counts them. Beside that, the 2D pointer
//! moves of the viewport ask `VisualTreeHelper::hit_test` for the nearest model under the pointer
//! and show the hit: the point in the coordinates of the mesh, the distance from the camera, the
//! triangle and the side.

use crate::markup::{content_page_class, xaml_class};
use ferroui_base::input::PointerEventArgs;
use ferroui_base::interactivity::IRoutedEventArgs;
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::{Color, IBrush};
use ferroui_base::reactive::Subscription;
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Point, Ref, Visual};
use ferroui_controls::{ContentPage, TextBlock};
use ferroui_media3d::{
    ContainerUIElement3D, DiffuseMaterial, GeometryModel3D, ModelUIElement3D, RayMeshGeometry3DHitTestResult,
    Viewport3D, VisualTreeHelper,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The names of the cubes, in the order of the document (left to right).
pub const CUBE_NAMES: [&str; 3] = ["Red cube", "Green cube", "Blue cube"];

/// The colours the document gives the cubes, in its order.
pub const CUBE_COLOURS: [Color; 3] =
    [Color::from_rgb(0xE7, 0x48, 0x56), Color::from_rgb(0x16, 0xC6, 0x0C), Color::from_rgb(0x00, 0x78, 0xD4)];

/// The colour of the selected cube.
pub const SELECTED_COLOUR: Color = Color::from_rgb(0xFF, 0xB9, 0x00);

/// A cube of the scene: its element, its model, the material the page colours and the colour the
/// document gives it.
pub struct Cube3D {
    pub name: &'static str,
    pub element: Ref<ModelUIElement3D>,
    pub model: Ref<GeometryModel3D>,
    pub material: Ref<DiffuseMaterial>,
    pub colour: Color,
}

/// `colour` mixed with white, for a cube under the pointer.
pub fn lit(colour: Color) -> Color {
    let mix = |channel: u8| (f64::from(channel) + (255.0 - f64::from(channel)) * 0.45).round() as u8;
    Color::from_rgb(mix(colour.r), mix(colour.g), mix(colour.b))
}

fn solid(color: Color) -> Rc<dyn IBrush> {
    Rc::new(ImmutableSolidColorBrush::new(color))
}

#[repr(C)]
pub struct Interaction3DPage {
    base: ContentPage,
    viewport: RefCell<Option<Ref<Viewport3D>>>,
    cubes: RefCell<Rc<Vec<Cube3D>>>,
    over: Cell<Option<usize>>,
    selected: Cell<Option<usize>>,
    presses: Cell<usize>,
    // The handlers of the elements, removed with the page.
    subscriptions: RefCell<Vec<Subscription>>,
}

content_page_class!(Interaction3DPage);
ferro_class_info!(Interaction3DPage {
    new: Interaction3DPage::new,
    markup: {
        methods: [
            fn ViewportPointerMoved(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<Interaction3DPage>, _sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.viewport_pointer_moved(&e)
                },
            fn ViewportPointerExited(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<Interaction3DPage>, _sender: Option<BoxedValue>, _e: Rc<dyn IRoutedEventArgs>| {
                    this.report_hit(None)
                },
        ],
    },
});
xaml_class!(Interaction3DPage, "/Pages/Interaction3DPage.xaml");

impl Interaction3DPage {
    pub fn construct() -> Self {
        Self {
            base: ContentPage::construct(),
            viewport: RefCell::new(None),
            cubes: RefCell::new(Rc::new(Vec::new())),
            over: Cell::new(None),
            selected: Cell::new(None),
            presses: Cell::new(0),
            subscriptions: RefCell::new(Vec::new()),
        }
    }

    /// # Panics
    /// Panics if the document does not have the scene the page uses.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        let viewport = this.get_control::<Viewport3D>("Viewport");
        let container = viewport
            .children()
            .iter()
            .find_map(|visual| visual.cast::<ContainerUIElement3D>())
            .expect("the container of the cubes of Pages/Interaction3DPage.xaml");
        let cubes: Vec<Cube3D> = container
            .children()
            .iter()
            .filter_map(|visual| visual.cast::<ModelUIElement3D>())
            .zip(CUBE_NAMES.into_iter().zip(CUBE_COLOURS))
            .map(|(element, (name, colour))| {
                let model =
                    element.model().and_then(|model| model.cast::<GeometryModel3D>()).expect("the model of a cube");
                let material = model
                    .material()
                    .and_then(|material| material.cast::<DiffuseMaterial>())
                    .expect("the material of a cube");
                Cube3D { name, element, model, material, colour }
            })
            .collect();
        assert_eq!(cubes.len(), CUBE_NAMES.len(), "the cubes of Pages/Interaction3DPage.xaml");
        this.subscribe(&container, &cubes);
        *this.viewport.borrow_mut() = Some(viewport);
        *this.cubes.borrow_mut() = Rc::new(cubes);
        this
    }

    /// The pointer handlers of the cubes and of the container. They hold the page weakly.
    fn subscribe(&self, container: &Ref<ContainerUIElement3D>, cubes: &[Cube3D]) {
        let mut subscriptions = self.subscriptions.borrow_mut();
        for (index, cube) in cubes.iter().enumerate() {
            let page = self.to_ref().downgrade();
            subscriptions.push(cube.element.pointer_entered(move |_, _| {
                if let Some(page) = page.upgrade() {
                    page.over.set(Some(index));
                    page.refresh();
                }
            }));
            let page = self.to_ref().downgrade();
            subscriptions.push(cube.element.pointer_exited(move |_, _| {
                if let Some(page) = page.upgrade() {
                    if page.over.get() == Some(index) {
                        page.over.set(None);
                    }
                    page.refresh();
                }
            }));
            let page = self.to_ref().downgrade();
            subscriptions.push(cube.element.pointer_pressed(move |_, args| {
                if let Some(page) = page.upgrade() {
                    page.select(index, args.hit());
                }
            }));
        }
        // The presses are not handled by the cubes: they bubble to the container.
        let page = self.to_ref().downgrade();
        subscriptions.push(container.pointer_pressed(move |_, _| {
            if let Some(page) = page.upgrade() {
                page.presses.set(page.presses.get() + 1);
                page.text("PressText")
                    .set_text(Some(&format!("Presses that reached the container: {}", page.presses.get())));
            }
        }));
    }

    /// The cubes of the scene, in the order of the document.
    pub fn cubes(&self) -> Rc<Vec<Cube3D>> {
        self.cubes.borrow().clone()
    }

    /// The viewport of the page.
    pub fn viewport(&self) -> Ref<Viewport3D> {
        self.viewport.borrow().clone().expect("the viewport of the page")
    }

    /// The index of the cube under the pointer, if one is.
    pub fn over(&self) -> Option<usize> {
        self.over.get()
    }

    /// The index of the selected cube, if one is.
    pub fn selected(&self) -> Option<usize> {
        self.selected.get()
    }

    fn text(&self, name: &str) -> Ref<TextBlock> {
        self.get_control::<TextBlock>(name)
    }

    /// Selects the cube `index` (or no cube, when it is the selected one) after a press on it.
    pub fn select(&self, index: usize, hit: Option<&RayMeshGeometry3DHitTestResult>) {
        let cubes = self.cubes();
        let Some(cube) = cubes.get(index) else { return };
        if self.selected.get() == Some(index) {
            self.selected.set(None);
            self.text("SelectedText").set_text(Some("Selected: nothing"));
        } else {
            self.selected.set(Some(index));
            let at = hit.map(|hit| {
                let point = hit.point_hit();
                format!(", pressed at ({:.2}, {:.2}, {:.2}) of its mesh", point.x, point.y, point.z)
            });
            self.text("SelectedText").set_text(Some(&format!("Selected: {}{}", cube.name, at.unwrap_or_default())));
        }
        self.refresh();
    }

    /// Colours the cubes after what the pointer is over and what is selected, and says what the
    /// pointer is over.
    fn refresh(&self) {
        let cubes = self.cubes();
        for (index, cube) in cubes.iter().enumerate() {
            let colour = if self.selected.get() == Some(index) {
                SELECTED_COLOUR
            } else if self.over.get() == Some(index) {
                lit(cube.colour)
            } else {
                cube.colour
            };
            cube.material.set_brush(Some(solid(colour)));
        }
        let over = self.over.get().and_then(|index| cubes.get(index)).map_or("nothing", |cube| cube.name);
        self.text("OverText").set_text(Some(&format!("Over: {over}")));
    }

    fn viewport_pointer_moved(&self, e: &Rc<dyn IRoutedEventArgs>) {
        let Some(e) = e.downcast_ref::<PointerEventArgs>() else { return };
        let viewport = self.viewport();
        let relative_to: &Visual = &viewport;
        self.report_hit_at(e.get_position(Some(relative_to)));
    }

    /// Hit tests the scene at `point` (in the coordinates of the viewport) with
    /// `VisualTreeHelper::hit_test`, shows the nearest hit, and returns the index of the cube hit.
    pub fn report_hit_at(&self, point: Point) -> Option<usize> {
        let hit = VisualTreeHelper::hit_test(&*self.viewport(), point);
        self.report_hit(hit.as_ref())
    }

    /// Shows `hit` (nothing under the pointer when `None`) and returns the index of the cube hit.
    fn report_hit(&self, hit: Option<&RayMeshGeometry3DHitTestResult>) -> Option<usize> {
        let cubes = self.cubes();
        let Some(hit) = hit else {
            self.text("HitText").set_text(Some("Nothing under the pointer"));
            return None;
        };
        let index = cubes.iter().position(|cube| cube.model.ptr_eq(hit.model_hit()));
        let name = index.and_then(|index| cubes.get(index)).map_or("A model", |cube| cube.name);
        let point = hit.point_hit();
        let side = if hit.is_front_face() { "front" } else { "back" };
        self.text("HitText").set_text(Some(&format!(
            "{name}: point ({:.2}, {:.2}, {:.2}) of its mesh, {:.2} from the camera, triangle {} {} {}, {side} face",
            point.x,
            point.y,
            point.z,
            hit.distance_to_ray_origin(),
            hit.vertex_index_1(),
            hit.vertex_index_2(),
            hit.vertex_index_3(),
        )));
        index
    }
}

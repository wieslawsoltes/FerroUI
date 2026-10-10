//! Port of `Pages/GeometryHitTestingPage.xaml.cs`: the class of the document
//! `Pages/GeometryHitTestingPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::input::{InputElement, PointerEventArgs};
use ferroui_base::interactivity::Interactive;
use ferroui_base::media::{Brushes, Geometry, IBrush, IntersectionResult, PolylineGeometry, TranslateTransform};
use ferroui_base::{ferro_class_info, instantiate, Point, Ref};
use ferroui_controls::shapes::{Path, Shape};
use ferroui_controls::{Panel, TextBlock, UserControl};
use std::cell::{OnceCell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

#[repr(C)]
pub struct GeometryHitTestingPage {
    base: UserControl,
    hit_stroke: Rc<dyn IBrush>,
    probe_position: Ref<TranslateTransform>,
    strokes: RefCell<HashMap<Ref<Shape>, Option<Rc<dyn IBrush>>>>,
    hits: RefCell<Vec<(Ref<Shape>, IntersectionResult)>>,
    probe_geometry: OnceCell<Ref<Geometry>>,
    scene: OnceCell<Ref<Panel>>,
    probe: OnceCell<Ref<Path>>,
    status: OnceCell<Ref<TextBlock>>,
}

user_control_class!(GeometryHitTestingPage);
ferro_class_info!(GeometryHitTestingPage { new: GeometryHitTestingPage::new });
xaml_class!(GeometryHitTestingPage, "/Pages/GeometryHitTestingPage.xaml");

impl GeometryHitTestingPage {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
            hit_stroke: Brushes::yellow(),
            probe_position: TranslateTransform::new(),
            strokes: RefCell::new(HashMap::new()),
            hits: RefCell::new(Vec::new()),
            probe_geometry: OnceCell::new(),
            scene: OnceCell::new(),
            probe: OnceCell::new(),
            status: OnceCell::new(),
        }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();

        let scene = this.get_control::<Panel>("Scene");
        let probe = this.get_control::<Path>("Probe");
        let status = this.get_control::<TextBlock>("Status");
        let _ = this.scene.set(scene.clone());
        let _ = this.probe.set(probe.clone());
        let _ = this.status.set(status);

        let probe_geometry = this.create_probe_geometry();
        probe_geometry.set_transform(&this.probe_position);
        probe.set_data(&probe_geometry);
        let _ = this.probe_geometry.set(probe_geometry);

        for child in scene.children().to_vec() {
            if let Some(shape) = child.cast::<Shape>() {
                if shape != probe {
                    let stroke = shape.stroke();
                    this.strokes.borrow_mut().insert(shape, stroke);
                }
            }
        }

        // The handlers belong to a child of the page: they hold the page weakly.
        let weak = this.downgrade();
        scene.add_handler(InputElement::pointer_entered_event(), move |sender, e| {
            if let Some(this) = weak.upgrade() {
                this.scene_on_pointer_moved(sender, e);
            }
        });
        let weak = this.downgrade();
        scene.add_handler(InputElement::pointer_moved_event(), move |sender, e| {
            if let Some(this) = weak.upgrade() {
                this.scene_on_pointer_moved(sender, e);
            }
        });
        let weak = this.downgrade();
        scene.add_handler(InputElement::pointer_exited_event(), move |sender, e| {
            if let Some(this) = weak.upgrade() {
                this.scene_on_pointer_exited(sender, e);
            }
        });

        this.update_status();
        this
    }

    fn probe_geometry(&self) -> &Ref<Geometry> {
        self.probe_geometry.get().expect("the probe geometry is created by the constructor")
    }

    fn scene(&self) -> &Ref<Panel> {
        self.scene.get().expect("the scene is found by the constructor")
    }

    fn probe(&self) -> &Ref<Path> {
        self.probe.get().expect("the probe is found by the constructor")
    }

    fn status(&self) -> &Ref<TextBlock> {
        self.status.get().expect("the status is found by the constructor")
    }

    fn create_probe_geometry(&self) -> Ref<Geometry> {
        PolylineGeometry::with_points([Point::new(0.0, -42.0), Point::new(38.0, 26.0), Point::new(-38.0, 26.0)], true)
            .upcast()
    }

    fn scene_on_pointer_moved(&self, _sender: &Interactive, e: &PointerEventArgs) {
        self.probe().set_is_visible(true);
        self.move_probe(e.get_position(Some(self.scene())));
    }

    fn move_probe(&self, position: Point) {
        self.probe_position.set_x(position.x);
        self.probe_position.set_y(position.y);

        self.clear_hits();

        for result in self.scene().get_input_elements_at_geometry(self.probe_geometry(), true) {
            if let Some(shape) = result.visual_hit.cast::<Shape>() {
                if self.strokes.borrow().contains_key(&shape) {
                    shape.set_stroke(Some(self.hit_stroke.clone()));
                    self.hits.borrow_mut().push((shape, result.intersection_result));
                }
            }
        }

        self.update_status();
    }

    fn scene_on_pointer_exited(&self, _sender: &Interactive, _e: &PointerEventArgs) {
        self.probe().set_is_visible(false);
        self.clear_hits();
        self.update_status();
    }

    fn clear_hits(&self) {
        let hits = std::mem::take(&mut *self.hits.borrow_mut());
        for (shape, _) in hits {
            let stroke = self.strokes.borrow()[&shape].clone();
            shape.set_stroke(stroke);
        }
    }

    fn update_status(&self) {
        if self.hits.borrow().is_empty() {
            self.status().set_text(Some("No intersection"));
            return;
        }

        let hits: Vec<String> = self
            .hits
            .borrow()
            .iter()
            .map(|(shape, intersection)| format!("{} ({:?})", shape.name().unwrap_or_default(), intersection))
            .collect();
        self.status().set_text(Some(&format!("Intersecting, {}", hits.join(", "))));
    }
}

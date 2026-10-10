//! Port of `ViewModels/Transform3DPageViewModel.cs`.

use ferroui_base::data::model::{Event, INotifyPropertyChanged};
use ferroui_base::ferro_markup_type;
use mini_mvvm::ViewModelBase;
use std::cell::Cell;
use std::rc::Rc;

pub struct Transform3DPageViewModel {
    base: ViewModelBase,
    depth: Cell<f64>,

    center_x: Cell<f64>,
    center_y: Cell<f64>,
    center_z: Cell<f64>,
    angle_x: Cell<f64>,
    angle_y: Cell<f64>,
    angle_z: Cell<f64>,
}

impl PartialEq for Transform3DPageViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl INotifyPropertyChanged for Transform3DPageViewModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}

impl Transform3DPageViewModel {
    pub fn new() -> Rc<Transform3DPageViewModel> {
        Rc::new(Self {
            base: ViewModelBase::new(),
            depth: Cell::new(200.0),
            center_x: Cell::new(0.0),
            center_y: Cell::new(0.0),
            center_z: Cell::new(0.0),
            angle_x: Cell::new(0.0),
            angle_y: Cell::new(0.0),
            angle_z: Cell::new(0.0),
        })
    }

    pub fn depth(&self) -> f64 {
        self.depth.get()
    }

    pub fn set_depth(&self, value: f64) {
        self.base.raise_and_set_if_changed_cell(&self.depth, value, "Depth");
    }

    pub fn center_x(&self) -> f64 {
        self.center_x.get()
    }

    pub fn set_center_x(&self, value: f64) {
        self.base.raise_and_set_if_changed_cell(&self.center_x, value, "CenterX");
    }

    pub fn center_y(&self) -> f64 {
        self.center_y.get()
    }

    pub fn set_center_y(&self, value: f64) {
        self.base.raise_and_set_if_changed_cell(&self.center_y, value, "CenterY");
    }

    pub fn center_z(&self) -> f64 {
        self.center_z.get()
    }

    pub fn set_center_z(&self, value: f64) {
        self.base.raise_and_set_if_changed_cell(&self.center_z, value, "CenterZ");
    }

    pub fn angle_x(&self) -> f64 {
        self.angle_x.get()
    }

    pub fn set_angle_x(&self, value: f64) {
        self.base.raise_and_set_if_changed_cell(&self.angle_x, value, "AngleX");
    }

    pub fn angle_y(&self) -> f64 {
        self.angle_y.get()
    }

    pub fn set_angle_y(&self, value: f64) {
        self.base.raise_and_set_if_changed_cell(&self.angle_y, value, "AngleY");
    }

    pub fn angle_z(&self) -> f64 {
        self.angle_z.get()
    }

    pub fn set_angle_z(&self, value: f64) {
        self.base.raise_and_set_if_changed_cell(&self.angle_z, value, "AngleZ");
    }
}

ferro_markup_type!(class Transform3DPageViewModel {
    this: Rc<Transform3DPageViewModel>,
    handles: [Transform3DPageViewModel, Rc<Transform3DPageViewModel>, Option<Rc<Transform3DPageViewModel>>],
    constructors: [() => Transform3DPageViewModel::new],
    properties: [
        Depth: f64 {
            get: |this: &Rc<Transform3DPageViewModel>| this.depth(),
            set: |this: &Rc<Transform3DPageViewModel>, value: f64| this.set_depth(value)
        },
        CenterX: f64 {
            get: |this: &Rc<Transform3DPageViewModel>| this.center_x(),
            set: |this: &Rc<Transform3DPageViewModel>, value: f64| this.set_center_x(value)
        },
        CenterY: f64 {
            get: |this: &Rc<Transform3DPageViewModel>| this.center_y(),
            set: |this: &Rc<Transform3DPageViewModel>, value: f64| this.set_center_y(value)
        },
        CenterZ: f64 {
            get: |this: &Rc<Transform3DPageViewModel>| this.center_z(),
            set: |this: &Rc<Transform3DPageViewModel>, value: f64| this.set_center_z(value)
        },
        AngleX: f64 {
            get: |this: &Rc<Transform3DPageViewModel>| this.angle_x(),
            set: |this: &Rc<Transform3DPageViewModel>, value: f64| this.set_angle_x(value)
        },
        AngleY: f64 {
            get: |this: &Rc<Transform3DPageViewModel>| this.angle_y(),
            set: |this: &Rc<Transform3DPageViewModel>, value: f64| this.set_angle_y(value)
        },
        AngleZ: f64 {
            get: |this: &Rc<Transform3DPageViewModel>| this.angle_z(),
            set: |this: &Rc<Transform3DPageViewModel>, value: f64| this.set_angle_z(value)
        },
    ],
    notify_property_changed: Transform3DPageViewModel,
});

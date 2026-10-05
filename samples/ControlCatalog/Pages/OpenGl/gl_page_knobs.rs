//! Port of `Pages/OpenGl/GlPageKnobs.xaml.cs`: the class of the document
//! `Pages/OpenGl/GlPageKnobs.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::{ferro_class_info, ferro_properties, instantiate, DirectProperty, FerroProperty, Ref};
use ferroui_controls::UserControl;
use std::cell::{Cell, RefCell};

#[repr(C)]
pub struct GlPageKnobs {
    base: UserControl,
    yaw: Cell<f32>,
    pitch: Cell<f32>,
    roll: Cell<f32>,
    disco: Cell<f32>,
    info: RefCell<String>,
}

user_control_class!(GlPageKnobs);
ferro_class_info!(GlPageKnobs {
    new: GlPageKnobs::new,
    markup: {
        namespace: "ControlCatalog.Pages.OpenGl",
    },
});
xaml_class!(GlPageKnobs, "/Pages/OpenGl/GlPageKnobs.xaml");

ferro_properties! {
    impl GlPageKnobs {
        pub fn yaw_property() -> DirectProperty<GlPageKnobs, f32> {
            FerroProperty::register_direct::<GlPageKnobs, _>(
                "Yaw",
                |o| o.yaw(),
                Some(|o: &GlPageKnobs, v| o.set_yaw(v)),
                0.0,
            )
        }

        pub fn pitch_property() -> DirectProperty<GlPageKnobs, f32> {
            FerroProperty::register_direct::<GlPageKnobs, _>(
                "Pitch",
                |o| o.pitch(),
                Some(|o: &GlPageKnobs, v| o.set_pitch(v)),
                0.0,
            )
        }

        pub fn roll_property() -> DirectProperty<GlPageKnobs, f32> {
            FerroProperty::register_direct::<GlPageKnobs, _>(
                "Roll",
                |o| o.roll(),
                Some(|o: &GlPageKnobs, v| o.set_roll(v)),
                0.0,
            )
        }

        pub fn disco_property() -> DirectProperty<GlPageKnobs, f32> {
            FerroProperty::register_direct::<GlPageKnobs, _>(
                "Disco",
                |o| o.disco(),
                Some(|o: &GlPageKnobs, v| o.set_disco(v)),
                0.0,
            )
        }

        pub fn info_property() -> DirectProperty<GlPageKnobs, String> {
            FerroProperty::register_direct::<GlPageKnobs, _>(
                "Info",
                |o| o.info(),
                Some(|o: &GlPageKnobs, v| o.set_info(v)),
                String::new(),
            )
        }
    }
}

impl GlPageKnobs {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
            yaw: Cell::new(0.0),
            pitch: Cell::new(0.0),
            roll: Cell::new(0.0),
            disco: Cell::new(0.0),
            info: RefCell::new(String::new()),
        }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }

    pub fn yaw(&self) -> f32 {
        self.yaw.get()
    }

    pub fn set_yaw(&self, value: f32) {
        self.set_and_raise_cell(Self::yaw_property(), &self.yaw, value);
    }

    pub fn pitch(&self) -> f32 {
        self.pitch.get()
    }

    pub fn set_pitch(&self, value: f32) {
        self.set_and_raise_cell(Self::pitch_property(), &self.pitch, value);
    }

    pub fn roll(&self) -> f32 {
        self.roll.get()
    }

    pub fn set_roll(&self, value: f32) {
        self.set_and_raise_cell(Self::roll_property(), &self.roll, value);
    }

    pub fn disco(&self) -> f32 {
        self.disco.get()
    }

    pub fn set_disco(&self, value: f32) {
        self.set_and_raise_cell(Self::disco_property(), &self.disco, value);
    }

    pub fn info(&self) -> String {
        self.info.borrow().clone()
    }

    pub fn set_info(&self, value: String) {
        self.set_and_raise(Self::info_property(), &self.info, value);
    }
}

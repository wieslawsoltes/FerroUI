//! PlayingCard control: the original white Border/Image composition and vector art.
use crate::view_models::presentation::CARD_TRANSITION_MS;
use crate::{controls::tray_container::brush, models};
use ferroui::base::animation::{
    DoubleTransition, IntegerTransition, TimeSpan, Transitions,
    easings::{CubicEaseInOut, Easing, SplineEasing},
};
use ferroui::controls::Canvas;
use ferroui::{
    base::{
        controls::ResourceDictionary,
        media::{DrawingImage, IImage},
    },
    controls::{Border, Image},
    prelude::*,
};
use ferroui_base::{
    StyledElementImpl, Visual, VisualImpl, input::InputElementImpl, interactivity::InteractiveImpl,
    layout::LayoutableImpl,
};
use ferroui_controls::ControlImpl;
use std::{cell::Cell, rc::Rc};

#[repr(C)]
pub struct PlayingCard {
    base: Border,
    card: Cell<models::PlayingCard>,
    image: Ref<Image>,
    motion: Transitions,
}
ferro_class!(PlayingCard: Border);
ferro_impl_classes!(PlayingCard: FerroObjectImpl, StyledElementImpl, VisualImpl, LayoutableImpl, InteractiveImpl, InputElementImpl, ControlImpl);
impl PlayingCard {
    pub fn new(card: models::PlayingCard, artwork: &ResourceDictionary) -> Ref<Self> {
        let motion = Transitions::new();
        let duration = TimeSpan::from_milliseconds(CARD_TRANSITION_MS as f64);
        for property in [
            Canvas::left_property().as_property(),
            Canvas::top_property().as_property(),
        ] {
            let transition = DoubleTransition::with_property(property, duration);
            transition.set_easing(Easing::new(CubicEaseInOut::new()));
            motion.add(transition.into());
        }
        let depth =
            IntegerTransition::with_property(Visual::z_index_property().as_property(), duration);
        depth.set_easing(Easing::new(SplineEasing::with_points(1.0, 0.0, 1.0, 0.0)));
        motion.add(depth.into());
        let image = Image::new();
        image.set_is_hit_test_visible(false);
        let this = instantiate(Self {
            base: Border::construct(),
            card: Cell::new(card),
            image: image.clone(),
            motion,
        });
        this.set_width(135.0);
        this.set_height(196.0);
        this.set_padding(Thickness::uniform(4.0));
        this.set_background(Some(brush("White")));
        this.set_border_brush(Some(brush("#87968C")));
        this.set_border_thickness(Thickness::uniform(1.0));
        this.set_corner_radius(CornerRadius::uniform(6.0));
        this.set_child(image);
        this.update_artwork(card, artwork);
        this
    }
    pub fn update_card(&self, card: models::PlayingCard, artwork: &ResourceDictionary) {
        if self.card.replace(card) != card {
            self.update_artwork(card, artwork);
        }
    }
    fn update_artwork(&self, card: models::PlayingCard, artwork: &ResourceDictionary) {
        let key = if card.is_face_down {
            "CardBack".to_string()
        } else {
            card.resource_key()
        };
        let value = artwork.get(&key.into()).expect("card artwork resource");
        let art = value
            .downcast_ref::<Ref<DrawingImage>>()
            .expect("DrawingImage resource");
        let art: Rc<dyn IImage> = art.clone().into();
        self.image.set_source(Some(art));
        ferroui::controls::automation::AutomationProperties::set_name(
            self,
            Some(&if card.is_face_down {
                "Face down card".into()
            } else {
                card.label()
            }),
        );
    }
    pub fn card_id(&self) -> u8 {
        self.card.get().id
    }
    pub fn set_motion_enabled(&self, enabled: bool) {
        if enabled {
            if self.transitions().is_none() {
                self.set_transitions(Some(self.motion.clone()));
            }
        } else {
            self.set_transitions(None);
        }
    }
    pub fn set_selected(&self, selected: bool) {
        self.set_border_brush(Some(brush(if selected { "#FFD447" } else { "#87968C" })));
        self.set_border_thickness(Thickness::uniform(if selected { 4.0 } else { 1.0 }));
    }
}

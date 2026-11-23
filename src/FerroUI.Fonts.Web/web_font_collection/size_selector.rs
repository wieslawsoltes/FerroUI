//! The `font-size` descriptor of CSS Fonts 5: a family with faces for different sizes of text.
//!
//! The wrapping shaper asks here for the face of each run, so `ferroui-base` and every other font
//! pay nothing. Only the typefaces of such a family are marked in the shaper.

use super::*;
use ferroui_fonts_web_core::matching::face_for_font_size;

impl Shared {
    /// Whether the family of a face has a face limited to a size.
    pub(super) fn family_has_sizes(&self, id: FontFaceId) -> bool {
        let faces = self.faces.borrow();
        let Some(own) = faces.iter().find(|slot| slot.id == id) else {
            return false;
        };
        let family = &own.descriptors.family;
        faces.iter().any(|slot| slot.descriptors.size_range.is_some() && family_matches(&slot.descriptors, family))
    }

    /// Lets the shaper ask for the face of a size, once it is installed.
    pub(super) fn connect_size_chooser(this: &Rc<Self>) {
        let Some(shaper) = this.feature_shaper.borrow().clone() else {
            return;
        };
        if shaper.has_size_chooser() {
            return;
        }
        let weak = Rc::downgrade(this);
        shaper.set_size_chooser(Rc::new(move |id, typeface, em_size| {
            weak.upgrade().and_then(|this| Shared::face_for_size(&this, id, typeface, em_size))
        }));
    }

    /// The typeface of the face for a size, when it is another face and it has loaded. Asking
    /// starts its load, and until then the run keeps the face it has.
    fn face_for_size(
        this: &Rc<Self>,
        id: FontFaceId,
        typeface: &Rc<GlyphTypeface>,
        em_size: f64,
    ) -> Option<Rc<GlyphTypeface>> {
        if !em_size.is_finite() || em_size <= 0.0 {
            return None;
        }
        let (chosen, request) = {
            let faces = this.faces.borrow();
            let own = faces.iter().find(|slot| slot.id == id)?;
            let family: Vec<&Slot> =
                faces.iter().filter(|slot| family_matches(&slot.descriptors, &own.descriptors.family)).collect();
            let descriptors: Vec<&FontFaceDescriptors> = family.iter().map(|slot| &slot.descriptors).collect();
            let index = family.iter().position(|slot| slot.id == id)?;
            let chosen = family[face_for_font_size(&descriptors, index, em_size as f32)].id;
            if chosen == id {
                return None;
            }
            let key = own.instances.iter().find(|(_, instance)| Rc::ptr_eq(instance, typeface))?.0;
            (chosen, key.request())
        };
        Self::request_load(this, chosen);
        this.instance(chosen, &request)
    }

    /// Marks what is already loaded in a family that just gained a face limited to a size.
    pub(super) fn refresh_sized_family(this: &Rc<Self>, family: &str) {
        let loaded: Vec<(FontFaceId, Rc<GlyphTypeface>)> = this
            .faces
            .borrow()
            .iter()
            .filter(|slot| family_matches(&slot.descriptors, family))
            .flat_map(|slot| {
                let instances = slot.instances.iter().map(|(_, instance)| (slot.id, instance.clone()));
                slot.glyph_typeface.iter().map(|typeface| (slot.id, typeface.clone())).chain(instances)
            })
            .collect();
        for (id, typeface) in loaded {
            this.register_features(id, &typeface);
        }
        Self::connect_size_chooser(this);
    }
}

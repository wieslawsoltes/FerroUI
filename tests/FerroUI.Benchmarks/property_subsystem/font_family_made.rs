//! A font family made from its name, as an application makes it for an element: the family of a
//! text block written where the block is built (`FontFamily::from("Inter")`, or the
//! `FontFamily="Inter"` of markup compiled for a template, once for every element made from it).
//!
//! - `named`: a family of one name;
//! - `list`: a family of two names, which has a key of its own;
//! - `sourced`: a family of a font of the application (`source#name`);
//! - `set_unchanged`: the family of a control set to a family made from the name it has: the
//!   property system compares the two before it does anything.
//!
//! Each invocation makes a family for each of a thousand elements.

use crate::harness::Registry;
use crate::trees::fixture::Fixture;
use ferroui_base::media::FontFamily;
use ferroui_base::Ref;
use ferroui_controls::documents::TextElement;
use ferroui_controls::{Border, Control};

const OBJECTS: usize = 1_000;

pub struct FontFamilyMade {
    _fixture: Fixture,
    controls: Vec<Ref<Control>>,
    made: Vec<FontFamily>,
}

impl FontFamilyMade {
    fn new() -> Self {
        let fixture = Fixture::new();
        let controls: Vec<Ref<Control>> = (0..OBJECTS).map(|_| Border::new().upcast()).collect();
        for control in &controls {
            TextElement::set_font_family_on(control, FontFamily::new("Inter"));
        }
        Self { _fixture: fixture, controls, made: Vec::with_capacity(OBJECTS) }
    }

    fn make(&mut self, name: &str) {
        self.made.clear();
        for _ in 0..OBJECTS {
            self.made.push(FontFamily::new(std::hint::black_box(name)));
        }
    }

    pub fn named(&mut self) {
        self.make("Inter");
    }

    pub fn list(&mut self) {
        self.make("Inter, Arial");
    }

    pub fn sourced(&mut self) {
        self.make("resm:Application.Fonts#Inter");
    }

    pub fn set_unchanged(&mut self) {
        for control in &self.controls {
            TextElement::set_font_family_on(control, FontFamily::new(std::hint::black_box("Inter")));
        }
    }
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("property_subsystem", "FontFamilyMade").min_iteration_time(200);
    let parameters = format!("Objects={OBJECTS}");
    class.benchmark("named", parameters.clone(), FontFamilyMade::new, FontFamilyMade::named).operations_per_invoke(OBJECTS as u32);
    class.benchmark("list", parameters.clone(), FontFamilyMade::new, FontFamilyMade::list).operations_per_invoke(OBJECTS as u32);
    class
        .benchmark("sourced", parameters.clone(), FontFamilyMade::new, FontFamilyMade::sourced)
        .operations_per_invoke(OBJECTS as u32);
    class
        .benchmark("set_unchanged", parameters, FontFamilyMade::new, FontFamilyMade::set_unchanged)
        .operations_per_invoke(OBJECTS as u32);
}

#[cfg(test)]
mod tests {
    #[test]
    fn font_family_made() {
        crate::harness::smoke_class(super::register, "FontFamilyMade");
    }
}

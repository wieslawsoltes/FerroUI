//! The instances of a glyph typeface at the `FontVariations` of texts (upstream dfee7f224b).

use std::any::Any;
use std::io::Read;
use std::rc::Rc;

use super::MAX_VARIATION_INSTANCES;
use crate::media::fonts::testing::TestPlatformTypeface;
use crate::media::fonts::OpenTypeTag;
use crate::media::{
    FontSimulations, FontStretch, FontStyle, FontVariation, FontVariationSettings, FontWeight, GlyphTypeface,
    IFontMemory, IPlatformTypeface,
};
use crate::utilities::ReadOnlyMemory;

const INTER_VARIABLE_FONT: &[u8] = include_bytes!("../test_assets/fonts/InterVariable.ttf");
const WGHT: OpenTypeTag = OpenTypeTag::from_chars('w', 'g', 'h', 't');
const OPSZ: OpenTypeTag = OpenTypeTag::from_chars('o', 'p', 's', 'z');

/// A platform typeface of the variable Inter at a position of its variation space, as a backend
/// that makes instances reports it.
struct VariableTypeface {
    inner: Rc<TestPlatformTypeface>,
    variation_settings: FontVariationSettings,
}

impl IFontMemory for VariableTypeface {
    fn try_get_table(&self, tag: OpenTypeTag) -> Option<ReadOnlyMemory<u8>> {
        self.inner.try_get_table(tag)
    }

    fn dispose(&self) {
        IFontMemory::dispose(&*self.inner);
    }
}

impl IPlatformTypeface for VariableTypeface {
    fn family_name(&self) -> String {
        self.inner.family_name()
    }

    fn weight(&self) -> FontWeight {
        self.inner.weight()
    }

    fn style(&self) -> FontStyle {
        self.inner.style()
    }

    fn stretch(&self) -> FontStretch {
        self.inner.stretch()
    }

    fn font_simulations(&self) -> FontSimulations {
        self.inner.font_simulations()
    }

    fn variation_settings(&self) -> FontVariationSettings {
        self.variation_settings.clone()
    }

    fn with_variation_settings(&self, settings: &FontVariationSettings) -> Option<Rc<dyn IPlatformTypeface>> {
        let kept = self.variation_settings.variations().iter().filter(|v| settings.try_get_value(v.tag).is_none());
        let variation_settings = FontVariationSettings::new(kept.chain(settings.variations()).copied());
        Some(Rc::new(VariableTypeface { inner: self.inner.clone(), variation_settings }))
    }

    fn try_get_stream(&self) -> Option<Box<dyn Read>> {
        self.inner.try_get_stream()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

fn variable_typeface() -> Rc<GlyphTypeface> {
    let inner =
        TestPlatformTypeface::from_bytes(INTER_VARIABLE_FONT.to_vec(), FontSimulations::None).expect("the font loads");
    let platform_typeface: Rc<dyn IPlatformTypeface> =
        Rc::new(VariableTypeface { inner, variation_settings: FontVariationSettings::empty() });
    GlyphTypeface::new(platform_typeface, FontSimulations::None).expect("the font has its tables")
}

fn weight(value: f64) -> FontVariationSettings {
    FontVariationSettings::new([FontVariation::new(WGHT, value)])
}

fn axis(typeface: &GlyphTypeface, tag: OpenTypeTag) -> Option<f64> {
    typeface.platform_typeface().variation_settings().try_get_value(tag)
}

#[test]
fn equal_settings_give_the_same_instance() {
    let variable = variable_typeface();
    let bold = variable.instance_with_variations(&weight(700.0));
    assert_eq!(axis(&bold, WGHT), Some(700.0));
    assert!(Rc::ptr_eq(&bold, &variable.instance_with_variations(&weight(700.0))));
    assert!(!Rc::ptr_eq(&bold, &variable.instance_with_variations(&weight(300.0))));
}

#[test]
fn the_instances_are_bounded_and_the_least_recent_is_dropped() {
    let variable = variable_typeface();
    let first = variable.instance_with_variations(&weight(100.0));
    for index in 0..MAX_VARIATION_INSTANCES {
        variable.instance_with_variations(&weight(200.0 + index as f64));
    }
    assert_eq!(variable.variation_instances.borrow().len(), MAX_VARIATION_INSTANCES);
    assert!(!Rc::ptr_eq(&first, &variable.instance_with_variations(&weight(100.0))));
    // A hit moves the instance to the end, so it survives the next insertions.
    let kept = variable.instance_with_variations(&weight(250.0));
    variable.instance_with_variations(&weight(800.0));
    assert!(Rc::ptr_eq(&kept, &variable.instance_with_variations(&weight(250.0))));
}

#[test]
fn an_optical_size_set_by_the_text_is_kept() {
    let variable = variable_typeface();
    let settings = FontVariationSettings::new([FontVariation::new(OPSZ, 30.0), FontVariation::new(WGHT, 500.0)]);
    let instance = variable.instance_with_variations(&settings);
    assert!(Rc::ptr_eq(&instance.at_optical_size(14.0), &instance));
    assert_eq!(axis(&instance, OPSZ), Some(30.0));

    // Without it, the instance follows the size of the text and keeps its weight.
    let bold = variable.instance_with_variations(&weight(700.0));
    let small = bold.at_optical_size(14.0);
    assert_eq!((axis(&small, OPSZ), axis(&small, WGHT)), (Some(14.0), Some(700.0)));
}

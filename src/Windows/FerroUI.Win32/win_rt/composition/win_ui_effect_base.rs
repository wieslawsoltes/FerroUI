//! The graph effects the backend hands to the compositor of the Windows
//! Runtime: an effect of Direct2D by its class identifier, with its
//! properties and its sources.
//!
//! The reference has a base class (`WinUIEffectBase`) that implements the
//! three interfaces of an effect and leaves the identifier and the
//! properties to the classes that derive from it. Here an effect is a value
//! that says what it is ([`WinUIEffect`]: the abstract members of the base
//! class), which every host compiles and tests, and the base class is the
//! COM object over such a value with its sources (`WinUIEffectBase`,
//! Windows only).

use super::d2d_effects::D2DEffects;
use ferroui_microcom::Guid;

/// The value of a property of an effect: what the reference creates a
/// `WinRTPropertyValue` from.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum EffectPropertyValue {
    Single(f32),
    UInt32(u32),
    SingleArray(Vec<f32>),
}

/// What an effect says of itself: the abstract members of the base class
/// of the reference.
pub(crate) trait WinUIEffect {
    /// The class identifier of the effect of Direct2D.
    fn effect_id(&self) -> Guid;

    fn property_count(&self) -> u32;

    /// The property of the index; `None` for an index the effect has no
    /// property for.
    fn get_property(&self, index: u32) -> Option<EffectPropertyValue>;

    /// The name the object answers as its runtime class name (the reference
    /// answers the full name of the type, found by reflection).
    fn runtime_class_name(&self) -> &'static str;
}

#[allow(dead_code)] // An effect of the reference no brush of the backend is built from.
pub(crate) struct BorderEffect {
    x: i32,
    y: i32,
}

#[allow(dead_code)]
impl BorderEffect {
    pub fn new(x: i32, y: i32) -> BorderEffect {
        BorderEffect { x, y }
    }
}

impl WinUIEffect for BorderEffect {
    fn effect_id(&self) -> Guid {
        D2DEffects::CLSID_D2D1_BORDER
    }

    fn property_count(&self) -> u32 {
        2
    }

    fn get_property(&self, index: u32) -> Option<EffectPropertyValue> {
        if index == 0 {
            return Some(EffectPropertyValue::UInt32(self.x as u32));
        }
        if index == 1 {
            return Some(EffectPropertyValue::UInt32(self.y as u32));
        }
        None
    }

    fn runtime_class_name(&self) -> &'static str {
        "FerroUI.Win32.WinRT.Composition.BorderEffect"
    }
}

pub(crate) struct BlendEffect {
    mode: i32,
}

impl BlendEffect {
    pub fn new(mode: i32) -> BlendEffect {
        BlendEffect { mode }
    }
}

impl WinUIEffect for BlendEffect {
    fn effect_id(&self) -> Guid {
        D2DEffects::CLSID_D2D1_BLEND
    }

    fn property_count(&self) -> u32 {
        1
    }

    fn get_property(&self, index: u32) -> Option<EffectPropertyValue> {
        if index == 0 {
            return Some(EffectPropertyValue::UInt32(self.mode as u32));
        }
        None
    }

    fn runtime_class_name(&self) -> &'static str {
        "FerroUI.Win32.WinRT.Composition.BlendEffect"
    }
}

#[allow(dead_code)] // An effect of the reference no brush of the backend is built from.
pub(crate) struct CompositeStepEffect {
    mode: f32,
}

#[allow(dead_code)]
impl CompositeStepEffect {
    pub fn new(mode: i32) -> CompositeStepEffect {
        CompositeStepEffect { mode: mode as f32 }
    }
}

impl WinUIEffect for CompositeStepEffect {
    fn effect_id(&self) -> Guid {
        D2DEffects::CLSID_D2D1_COMPOSITE
    }

    fn property_count(&self) -> u32 {
        1
    }

    fn get_property(&self, index: u32) -> Option<EffectPropertyValue> {
        if index == 0 {
            return Some(EffectPropertyValue::UInt32(self.mode as u32));
        }
        None
    }

    fn runtime_class_name(&self) -> &'static str {
        "FerroUI.Win32.WinRT.Composition.CompositeStepEffect"
    }
}

pub(crate) struct OpacityEffect {
    opacity: f32,
}

impl OpacityEffect {
    pub fn new(opacity: f32) -> OpacityEffect {
        OpacityEffect { opacity }
    }
}

impl WinUIEffect for OpacityEffect {
    fn effect_id(&self) -> Guid {
        D2DEffects::CLSID_D2D1_OPACITY
    }

    fn property_count(&self) -> u32 {
        1
    }

    fn get_property(&self, index: u32) -> Option<EffectPropertyValue> {
        if index == 0 {
            return Some(EffectPropertyValue::Single(self.opacity));
        }
        None
    }

    fn runtime_class_name(&self) -> &'static str {
        "FerroUI.Win32.WinRT.Composition.OpacityEffect"
    }
}

pub(crate) struct ColorSourceEffect {
    color: Vec<f32>,
}

impl ColorSourceEffect {
    pub fn new(color: Vec<f32>) -> ColorSourceEffect {
        ColorSourceEffect { color }
    }
}

impl WinUIEffect for ColorSourceEffect {
    fn effect_id(&self) -> Guid {
        D2DEffects::CLSID_D2D1_FLOOD
    }

    fn property_count(&self) -> u32 {
        1
    }

    fn get_property(&self, index: u32) -> Option<EffectPropertyValue> {
        if index == 0 {
            return Some(EffectPropertyValue::SingleArray(self.color.clone()));
        }
        None
    }

    fn runtime_class_name(&self) -> &'static str {
        "FerroUI.Win32.WinRT.Composition.ColorSourceEffect"
    }
}

#[repr(u32)]
#[allow(non_camel_case_types, dead_code, clippy::enum_variant_names)]
enum D2D1_GAUSSIANBLUR_OPTIMIZATION {
    D2D1_GAUSSIANBLUR_OPTIMIZATION_SPEED,
    D2D1_GAUSSIANBLUR_OPTIMIZATION_BALANCED,
    D2D1_GAUSSIANBLUR_OPTIMIZATION_QUALITY,
    D2D1_GAUSSIANBLUR_OPTIMIZATION_FORCE_DWORD,
}

#[repr(u32)]
#[allow(non_camel_case_types, dead_code, clippy::enum_variant_names)]
enum D2D1_BORDER_MODE {
    D2D1_BORDER_MODE_SOFT,
    D2D1_BORDER_MODE_HARD,
    D2D1_BORDER_MODE_FORCE_DWORD,
}

/// `D2D1GaussianBlurProp`.
mod d2d1_gaussian_blur_prop {
    pub const D2D1_GAUSSIANBLUR_PROP_STANDARD_DEVIATION: u32 = 0;
    pub const D2D1_GAUSSIANBLUR_PROP_OPTIMIZATION: u32 = 1;
    pub const D2D1_GAUSSIANBLUR_PROP_BORDER_MODE: u32 = 2;
    #[allow(dead_code)]
    pub const D2D1_GAUSSIANBLUR_PROP_FORCE_DWORD: u32 = 3;
}

pub(crate) struct WinUIGaussianBlurEffect;

impl WinUIEffect for WinUIGaussianBlurEffect {
    fn effect_id(&self) -> Guid {
        D2DEffects::CLSID_D2D1_GAUSSIAN_BLUR
    }

    fn property_count(&self) -> u32 {
        3
    }

    fn get_property(&self, index: u32) -> Option<EffectPropertyValue> {
        use d2d1_gaussian_blur_prop::*;
        match index {
            D2D1_GAUSSIANBLUR_PROP_STANDARD_DEVIATION => Some(EffectPropertyValue::Single(30.0)),
            D2D1_GAUSSIANBLUR_PROP_OPTIMIZATION => {
                Some(EffectPropertyValue::UInt32(D2D1_GAUSSIANBLUR_OPTIMIZATION::D2D1_GAUSSIANBLUR_OPTIMIZATION_BALANCED as u32))
            }
            D2D1_GAUSSIANBLUR_PROP_BORDER_MODE => Some(EffectPropertyValue::UInt32(D2D1_BORDER_MODE::D2D1_BORDER_MODE_HARD as u32)),
            _ => None,
        }
    }

    fn runtime_class_name(&self) -> &'static str {
        "FerroUI.Win32.WinRT.Composition.WinUIGaussianBlurEffect"
    }
}

/// `D2D1_SATURATION_PROP`.
mod d2d1_saturation_prop {
    pub const D2D1_SATURATION_PROP_SATURATION: u32 = 0;
    #[allow(dead_code)]
    pub const D2D1_SATURATION_PROP_FORCE_DWORD: u32 = 1;
}

pub(crate) struct SaturationEffect;

impl WinUIEffect for SaturationEffect {
    fn effect_id(&self) -> Guid {
        D2DEffects::CLSID_D2D1_SATURATION
    }

    fn property_count(&self) -> u32 {
        1
    }

    fn get_property(&self, index: u32) -> Option<EffectPropertyValue> {
        match index {
            d2d1_saturation_prop::D2D1_SATURATION_PROP_SATURATION => Some(EffectPropertyValue::Single(2.0)),
            _ => None,
        }
    }

    fn runtime_class_name(&self) -> &'static str {
        "FerroUI.Win32.WinRT.Composition.SaturationEffect"
    }
}

#[cfg(windows)]
pub(crate) use imp::WinUIEffectBase;

#[cfg(windows)]
mod imp {
    use super::{EffectPropertyValue, WinUIEffect};
    use crate::win_rt::{
        IGraphicsEffect, IGraphicsEffectD2D1Interop, IGraphicsEffectD2D1InteropImpl, IGraphicsEffectImpl,
        IGraphicsEffectSource, IGraphicsEffectSourceImpl, IInspectableImpl, IPropertyValue, TrustLevel, WinRTInspectable,
        WinRTPropertyValue, GRAPHICS_EFFECT_PROPERTY_MAPPING,
    };
    use ferroui_microcom::{make_com_with, ComPtr, Guid, HResult, Interface, InterfaceEntry};

    /// An effect as the object the compositor is given: it answers
    /// `IGraphicsEffect`, `IGraphicsEffectSource` and
    /// `IGraphicsEffectD2D1Interop`, and the compositor asks one for the
    /// other.
    pub(crate) struct WinUIEffectBase {
        effect: Box<dyn WinUIEffect>,
        /// Released with the object (`Destroyed` of the reference).
        sources: Vec<ComPtr<IGraphicsEffectSource>>,
    }

    impl WinUIEffectBase {
        /// The effect over its sources, as the object the system calls.
        ///
        /// The reference keeps a source that is an effect of its own as the
        /// managed object and takes a reference to any other; and because
        /// the system releases the reference `GetSource` returned and uses
        /// the object afterwards, it leases the native pointer of an effect
        /// that is the source of another until the brushes are made
        /// (`CreateAcrylicBlurBackdropBrush`). Here every source is a
        /// counted reference the effect holds until it is destroyed, so an
        /// effect that is a source lives as long as the effect it is the
        /// source of and nothing is leased.
        pub fn new(effect: impl WinUIEffect + 'static, sources: &[&IGraphicsEffectSource]) -> ComPtr<IGraphicsEffect> {
            make_com_with::<IGraphicsEffect, _, 2>(
                WinUIEffectBase {
                    effect: Box::new(effect),
                    sources: sources.iter().map(|source| ComPtr::from_ref(*source)).collect(),
                },
                [InterfaceEntry::of::<IGraphicsEffectSource>(), InterfaceEntry::of::<IGraphicsEffectD2D1Interop>()],
            )
        }
    }

    impl IInspectableImpl for WinUIEffectBase {
        fn get_iids(&self, iid_count: *mut u64, iids: *mut *mut Guid) -> Result<(), HResult> {
            // SAFETY: the out parameters of the call the generated thunk
            // received.
            unsafe {
                WinRTInspectable::get_iids(
                    &[IGraphicsEffect::IID, IGraphicsEffectSource::IID, IGraphicsEffectD2D1Interop::IID],
                    iid_count,
                    iids,
                )
            }
        }

        fn get_runtime_class_name(&self) -> Result<isize, HResult> {
            WinRTInspectable::runtime_class_name(self.effect.runtime_class_name())
        }

        fn get_trust_level(&self) -> Result<TrustLevel, HResult> {
            Ok(WinRTInspectable::trust_level())
        }
    }

    impl IGraphicsEffectImpl for WinUIEffectBase {
        fn name(&self) -> Result<isize, HResult> {
            Ok(0)
        }

        fn set_name(&self, _name: isize) -> Result<(), HResult> {
            Ok(())
        }
    }

    impl IGraphicsEffectSourceImpl for WinUIEffectBase {}

    impl IGraphicsEffectD2D1InteropImpl for WinUIEffectBase {
        fn get_effect_id(&self) -> Result<Guid, HResult> {
            Ok(self.effect.effect_id())
        }

        fn get_named_property_mapping(
            &self,
            _name: isize,
            _index: *mut u32,
            _mapping: *mut GRAPHICS_EFFECT_PROPERTY_MAPPING,
        ) -> Result<(), HResult> {
            // "Not supported"
            Err(HResult::NOTIMPL)
        }

        fn get_property_count(&self) -> Result<u32, HResult> {
            Ok(self.effect.property_count())
        }

        fn get_property(&self, index: u32) -> Result<Option<ComPtr<IPropertyValue>>, HResult> {
            Ok(self.effect.get_property(index).map(|value| {
                match value {
                    EffectPropertyValue::Single(f) => WinRTPropertyValue::from_single(f),
                    EffectPropertyValue::UInt32(u) => WinRTPropertyValue::from_u_int32(u),
                    EffectPropertyValue::SingleArray(array) => WinRTPropertyValue::from_single_array(array),
                }
                .into_com()
            }))
        }

        fn get_source(&self, index: u32) -> Result<Option<ComPtr<IGraphicsEffectSource>>, HResult> {
            // "Invalid index". The reference compares with `>` and so
            // fails for the index one past the end with the exception of
            // the array instead: a failure either way.
            self.sources.get(index as usize).cloned().map(Some).ok_or(HResult::INVALIDARG)
        }

        fn get_source_count(&self) -> Result<u32, HResult> {
            Ok(self.sources.len() as u32)
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream: the reference has no tests of the effects.
    use super::*;

    #[test]
    fn every_effect_names_its_effect_of_direct2d_and_its_properties() {
        let border = BorderEffect::new(1, 2);
        assert_eq!(D2DEffects::CLSID_D2D1_BORDER, border.effect_id());
        assert_eq!(2, border.property_count());
        assert_eq!(Some(EffectPropertyValue::UInt32(1)), border.get_property(0));
        assert_eq!(Some(EffectPropertyValue::UInt32(2)), border.get_property(1));
        assert_eq!(None, border.get_property(2));

        // 23 and 22: the luminosity and the colour blend modes of the mica
        // brush.
        let blend = BlendEffect::new(23);
        assert_eq!(D2DEffects::CLSID_D2D1_BLEND, blend.effect_id());
        assert_eq!(1, blend.property_count());
        assert_eq!(Some(EffectPropertyValue::UInt32(23)), blend.get_property(0));
        assert_eq!(None, blend.get_property(1));

        let composite = CompositeStepEffect::new(3);
        assert_eq!(D2DEffects::CLSID_D2D1_COMPOSITE, composite.effect_id());
        assert_eq!(Some(EffectPropertyValue::UInt32(3)), composite.get_property(0));
        assert_eq!(None, composite.get_property(1));

        let opacity = OpacityEffect::new(0.6);
        assert_eq!(D2DEffects::CLSID_D2D1_OPACITY, opacity.effect_id());
        assert_eq!(Some(EffectPropertyValue::Single(0.6)), opacity.get_property(0));
        assert_eq!(None, opacity.get_property(1));

        let color = ColorSourceEffect::new(vec![0.5, 0.25, 0.125, 1.0]);
        assert_eq!(D2DEffects::CLSID_D2D1_FLOOD, color.effect_id());
        assert_eq!(Some(EffectPropertyValue::SingleArray(vec![0.5, 0.25, 0.125, 1.0])), color.get_property(0));
        assert_eq!(None, color.get_property(1));

        let blur = WinUIGaussianBlurEffect;
        assert_eq!(D2DEffects::CLSID_D2D1_GAUSSIAN_BLUR, blur.effect_id());
        assert_eq!(3, blur.property_count());
        // A standard deviation of 30, the balanced optimization, the hard
        // border mode.
        assert_eq!(Some(EffectPropertyValue::Single(30.0)), blur.get_property(0));
        assert_eq!(Some(EffectPropertyValue::UInt32(1)), blur.get_property(1));
        assert_eq!(Some(EffectPropertyValue::UInt32(1)), blur.get_property(2));
        assert_eq!(None, blur.get_property(3));

        let saturation = SaturationEffect;
        assert_eq!(D2DEffects::CLSID_D2D1_SATURATION, saturation.effect_id());
        assert_eq!(1, saturation.property_count());
        assert_eq!(Some(EffectPropertyValue::Single(2.0)), saturation.get_property(0));
        assert_eq!(None, saturation.get_property(1));
    }

    #[test]
    fn the_class_identifiers_are_the_ones_of_the_system_header() {
        // d2d1effects.h.
        assert_eq!(Guid::parse("1FEB6D69-2FE6-4AC9-8C58-1D7F93E7A6A5"), Some(D2DEffects::CLSID_D2D1_GAUSSIAN_BLUR));
        assert_eq!(Guid::parse("5CB2D9CF-327D-459F-A0CE-40C0B2086BF7"), Some(D2DEffects::CLSID_D2D1_SATURATION));
        assert_eq!(Guid::parse("81C5B77B-13F8-4CDD-AD20-C890547AC65D"), Some(D2DEffects::CLSID_D2D1_BLEND));
        assert_eq!(Guid::parse("61C23C20-AE69-4D8E-94CF-50078DF638F2"), Some(D2DEffects::CLSID_D2D1_FLOOD));
        assert_eq!(Guid::parse("811D79A4-DE28-4454-8094-C64685F8BD4C"), Some(D2DEffects::CLSID_D2D1_OPACITY));
    }
}

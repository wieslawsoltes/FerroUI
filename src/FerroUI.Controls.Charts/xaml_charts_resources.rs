use ferroui_base::controls::{
    IResourceDictionary, IResourceProvider, IThemeVariantProvider, ResourceDictionary,
    ResourceProviderImpl,
};
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::IBrush;
use ferroui_base::styling::ThemeVariant;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl,
    FerroObjectImplExt, Ref,
};
use std::rc::Rc;

/// Theme dictionaries for the published Charts resource keys. Local and ancestor
/// resources can replace each brush without writing local values onto axes.
#[repr(C)]
pub struct XamlChartsResources {
    base: ResourceDictionary,
}
ferro_class!(XamlChartsResources: ResourceDictionary);
ferro_class_info!(XamlChartsResources {
    new: XamlChartsResources::new,
    interfaces: [Rc<dyn IResourceProvider>, Rc<dyn IResourceDictionary>, Rc<dyn IThemeVariantProvider>],
});
ferro_impl_classes!(XamlChartsResources: ResourceProviderImpl);
impl FerroObjectImpl for XamlChartsResources {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.add_theme_dictionary(ThemeVariant::light(), theme(false));
        this.add_theme_dictionary(ThemeVariant::dark(), theme(true));
        this.add_theme_dictionary(ThemeVariant::default(), theme(false));
    }
}
impl XamlChartsResources {
    pub fn construct() -> Self {
        Self {
            base: ResourceDictionary::construct(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}

fn theme(dark: bool) -> Ref<ResourceDictionary> {
    let resources = ResourceDictionary::new();
    for (key, color) in [
        (
            "ChartsControlBackgroundBrush",
            if dark { 0xff202020 } else { 0x00ffffff },
        ),
        (
            "ChartsControlForegroundBrush",
            if dark { 0xfff3f3f3 } else { 0xff242424 },
        ),
        (
            "ChartsGridLineMajorBrush",
            if dark { 0xff505050 } else { 0xffdedede },
        ),
        (
            "ChartsGridLineMinorBrush",
            if dark { 0xff383838 } else { 0xffeeeeee },
        ),
        (
            "ChartsTickBrush",
            if dark { 0xffbcbcbc } else { 0xff666666 },
        ),
        (
            "ChartsTickLabelBrush",
            if dark { 0xfff3f3f3 } else { 0xff242424 },
        ),
        (
            "ChartsAxisLineBrush",
            if dark { 0xff8a8a8a } else { 0xff777777 },
        ),
    ] {
        let brush: Rc<dyn IBrush> = Rc::new(ImmutableSolidColorBrush::from_uint32(color));
        resources.add_value(key, brush);
    }
    resources
}

pub(crate) fn solid(color: u32) -> Rc<dyn IBrush> {
    Rc::new(ImmutableSolidColorBrush::from_uint32(color))
}

pub(crate) const SERIES_COLORS: [u32; 8] = [
    0xff0078d4, 0xffe74856, 0xff13a10e, 0xff886ce4, 0xffc19c00, 0xff00b7c3, 0xffca5010, 0xffe3008c,
];

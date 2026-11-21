use ferroribbon::theming::{
    RibbonChromeStyle, RibbonColor, RibbonThemePalette, RibbonThemeResources, RibbonThemeStyle,
};
use ferroui_base::{
    FerroObjectImpl, FerroProperty, Ref, StyledElementImpl, StyledProperty, VisualImpl,
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate,
};
use ferroui_base::{
    input::InputElementImpl, interactivity::InteractiveImpl, layout::LayoutableImpl,
};
use ferroui_controls::{ControlImpl, PanelImpl, StackPanel, StackPanelImpl};
use std::rc::Rc;
#[repr(C)]
pub struct ThemeConsumer {
    base: StackPanel,
}
ferro_class!(ThemeConsumer:StackPanel);
ferro_impl_classes!(ThemeConsumer:FerroObjectImpl,StyledElementImpl,VisualImpl,InteractiveImpl,InputElementImpl,LayoutableImpl,ControlImpl,PanelImpl,StackPanelImpl);
ferro_class_info!(ThemeConsumer {new:ThemeConsumer::new,markup:{namespace:"FerroRibbon.Consumer"}});
ferro_properties! {impl ThemeConsumer {
    pub fn color_property()->StyledProperty<RibbonColor>{FerroProperty::register::<Self,_>("Color",RibbonColor::TRANSPARENT)}
    pub fn optional_color_property()->StyledProperty<Option<RibbonColor>>{FerroProperty::register::<Self,_>("OptionalColor",None)}
    pub fn palette_property()->StyledProperty<Option<Rc<RibbonThemePalette>>>{FerroProperty::register::<Self,_>("Palette",None)}
    pub fn chrome_property()->StyledProperty<RibbonChromeStyle>{FerroProperty::register::<Self,_>("Chrome",RibbonChromeStyle::Colorful)}
    pub fn ribbon_style_property()->StyledProperty<RibbonThemeStyle>{FerroProperty::register::<Self,_>("RibbonStyle",RibbonThemeStyle::Office)}
    pub fn current_resources_property()->StyledProperty<Option<Ref<RibbonThemeResources>>>{FerroProperty::register::<Self,_>("CurrentResources",None)}
    pub fn presets_property()->StyledProperty<Rc<[Rc<RibbonThemePalette>]>>{FerroProperty::register::<Self,_>("Presets",RibbonThemePalette::presets())}
}}
impl ThemeConsumer {
    pub fn new() -> Ref<Self> {
        instantiate(Self {
            base: StackPanel::construct(),
        })
    }
    pub fn color(&self) -> RibbonColor {
        self.get_value(Self::color_property())
    }
    pub fn set_color(&self, v: RibbonColor) {
        self.set_value(Self::color_property(), v)
    }
    pub fn optional_color(&self) -> Option<RibbonColor> {
        self.get_value(Self::optional_color_property())
    }
    pub fn set_optional_color(&self, v: Option<RibbonColor>) {
        self.set_value(Self::optional_color_property(), v)
    }
    pub fn palette(&self) -> Option<Rc<RibbonThemePalette>> {
        self.get_value(Self::palette_property())
    }
    pub fn set_palette(&self, v: Option<Rc<RibbonThemePalette>>) {
        self.set_value(Self::palette_property(), v)
    }
    pub fn chrome(&self) -> RibbonChromeStyle {
        self.get_value(Self::chrome_property())
    }
    pub fn set_chrome(&self, v: RibbonChromeStyle) {
        self.set_value(Self::chrome_property(), v)
    }
    pub fn ribbon_style(&self) -> RibbonThemeStyle {
        self.get_value(Self::ribbon_style_property())
    }
    pub fn set_ribbon_style(&self, v: RibbonThemeStyle) {
        self.set_value(Self::ribbon_style_property(), v)
    }
    pub fn current_resources(&self) -> Option<Ref<RibbonThemeResources>> {
        self.get_value(Self::current_resources_property())
    }
    pub fn set_current_resources(&self, v: Option<Ref<RibbonThemeResources>>) {
        self.set_value(Self::current_resources_property(), v)
    }
    pub fn presets(&self) -> Rc<[Rc<RibbonThemePalette>]> {
        self.get_value(Self::presets_property())
    }
    pub fn set_presets(&self, v: Rc<[Rc<RibbonThemePalette>]>) {
        self.set_value(Self::presets_property(), v)
    }
}

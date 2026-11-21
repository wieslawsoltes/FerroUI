//! Source font-picker constructors/defaults; actual combo public implementation inherited.
use super::{RibbonComboBox, RibbonComboBoxImpl, RibbonControlBaseImpl, RibbonInputBaseImpl};
use ferroribbon_core::collections::RibbonFixedItems;
use ferroui_base::{
    FerroObjectImpl, FerroObjectImplExt, Ref, StyledElementImpl, VisualImpl, ferro_class,
    ferro_class_info, ferro_impl_classes, input::InputElementImpl, instantiate,
    interactivity::InteractiveImpl, layout::LayoutableImpl,
};
use ferroui_controls::{ControlImpl, primitives::TemplatedControlImpl};
#[repr(C)]
pub struct RibbonFontComboBox {
    base: RibbonComboBox,
}
ferro_class!(RibbonFontComboBox:RibbonComboBox);
ferro_class_info!(RibbonFontComboBox{new:RibbonFontComboBox::new,markup:{namespace:"FerroRibbon.Controls",static_properties:[DefaultFonts:ferroribbon_core::collections::RibbonReadOnlyList<String>{get:RibbonFontComboBox::default_fonts}]}});
ferro_impl_classes!(RibbonFontComboBox:VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl,RibbonControlBaseImpl,RibbonInputBaseImpl,RibbonComboBoxImpl);
impl RibbonFontComboBox {
    pub fn new() -> Ref<Self> {
        instantiate(Self {
            base: RibbonComboBox::construct(),
        })
    }
    pub fn default_fonts() -> ferroribbon_core::collections::RibbonReadOnlyList<String> {
        ferroui_base::ferro_thread_local! {static VALUES:ferroribbon_core::collections::RibbonReadOnlyList<String>=RibbonFixedItems::new(vec![
            "Aptos", "Arial", "Arial Black", "Bahnschrift", "Calibri", "Cambria", "Candara", "Comic Sans MS", "Consolas", "Constantia", "Corbel",
            "Courier New", "Franklin Gothic", "Garamond", "Georgia", "Gill Sans", "Helvetica", "Impact", "Inter", "Lucida Console", "Menlo",
            "Monaco", "Open Sans", "Palatino", "Roboto", "Segoe UI", "Tahoma", "Times New Roman", "Trebuchet MS", "Verdana",
        ].into_iter().map(str::to_string).collect()) as ferroribbon_core::collections::RibbonReadOnlyList<_>;}
        VALUES.with(Clone::clone)
    }
}
impl StyledElementImpl for RibbonFontComboBox {
    fn style_key_override(_this: &Self) -> &'static ferroui_base::TypeInfo {
        <Self as ferroui_base::StaticType>::TYPE
    }
}
impl FerroObjectImpl for RibbonFontComboBox {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.set_is_editable(true);
        this.set_is_font_preview(true);
        this.set_input_width(132.);
        let key = "Font".into();
        let read = || {
            ferroribbon_core::localization::RibbonStrings::current()
                .get(Some(&key))
                .expect("Font localization")
                .map(|v| {
                    v.to_utf8()
                        .expect("Native Font Label cannot represent invalid UTF16")
                })
        };
        let label = if read().as_deref() == Some("Font") {
            Some("Font".into())
        } else {
            read()
        };
        this.set_label(label);
        for value in Self::default_fonts().iter() {
            this.items()
                .add(Some(ferroribbon_core::model::RibbonObject::value(value)));
        }
    }
}
#[repr(C)]
pub struct RibbonFontSizeComboBox {
    base: RibbonComboBox,
}
ferro_class!(RibbonFontSizeComboBox:RibbonComboBox);
ferro_class_info!(RibbonFontSizeComboBox{new:RibbonFontSizeComboBox::new,markup:{namespace:"FerroRibbon.Controls",static_properties:[DefaultSizes:ferroribbon_core::collections::RibbonReadOnlyList<f64>{get:RibbonFontSizeComboBox::default_sizes}]}});
ferro_impl_classes!(RibbonFontSizeComboBox:VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl,RibbonControlBaseImpl,RibbonInputBaseImpl,RibbonComboBoxImpl);
impl RibbonFontSizeComboBox {
    pub fn new() -> Ref<Self> {
        instantiate(Self {
            base: RibbonComboBox::construct(),
        })
    }
    pub fn default_sizes() -> ferroribbon_core::collections::RibbonReadOnlyList<f64> {
        ferroui_base::ferro_thread_local! {static VALUES:ferroribbon_core::collections::RibbonReadOnlyList<f64>=RibbonFixedItems::new(vec![8., 9., 10., 10.5, 11., 12., 14., 16., 18., 20., 22., 24., 26., 28., 36., 48., 72.]) as ferroribbon_core::collections::RibbonReadOnlyList<_>;}
        VALUES.with(Clone::clone)
    }
}
impl StyledElementImpl for RibbonFontSizeComboBox {
    fn style_key_override(_this: &Self) -> &'static ferroui_base::TypeInfo {
        <Self as ferroui_base::StaticType>::TYPE
    }
}
impl FerroObjectImpl for RibbonFontSizeComboBox {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.set_is_editable(true);
        this.set_input_width(46.);
        this.set_label(Some("Font Size".into()));
        for value in Self::default_sizes().iter() {
            this.items()
                .add(Some(ferroribbon_core::model::RibbonObject::value(value)));
        }
    }
}

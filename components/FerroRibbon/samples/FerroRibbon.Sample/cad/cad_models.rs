// Copyright (c) 2026 Wiesław Šoltés. MIT. Native Rust port of CadModels.cs.
use super::cad_icon::CadIcon;
use ferroribbon_core::model::RibbonIcon;
use ferroui_base::{
    Ref,
    media::{Color, MediaCollection, SolidColorBrush},
};
use std::{fmt, rc::Rc};
pub struct CadLayer {
    name: String,
    color: Color,
    color_brush: Ref<SolidColorBrush>,
    is_on: bool,
    is_frozen: bool,
    is_locked: bool,
}
impl CadLayer {
    pub fn new(name: impl Into<String>, color: Color) -> Rc<Self> {
        Self::with_options(name, color, true, false, false)
    }
    pub fn with_options(
        name: impl Into<String>,
        color: Color,
        is_on: bool,
        is_frozen: bool,
        is_locked: bool,
    ) -> Rc<Self> {
        Rc::new(Self {
            name: name.into(),
            color,
            color_brush: SolidColorBrush::with_color(color),
            is_on,
            is_frozen,
            is_locked,
        })
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn color(&self) -> Color {
        self.color
    }
    pub fn color_brush(&self) -> Ref<SolidColorBrush> {
        self.color_brush.clone()
    }
    pub fn is_on(&self) -> bool {
        self.is_on
    }
    pub fn is_frozen(&self) -> bool {
        self.is_frozen
    }
    pub fn is_locked(&self) -> bool {
        self.is_locked
    }
    pub fn on_icon(&self) -> Option<Rc<RibbonIcon>> {
        CadIcon::get(if self.is_on { "LayerOn" } else { "LayerOff" })
    }
    pub fn freeze_icon(&self) -> Option<Rc<RibbonIcon>> {
        CadIcon::get(if self.is_frozen {
            "LayerFreeze"
        } else {
            "LayerThaw"
        })
    }
    pub fn lock_icon(&self) -> Option<Rc<RibbonIcon>> {
        CadIcon::get(if self.is_locked {
            "LayerLock"
        } else {
            "LayerUnlock"
        })
    }
    pub fn samples() -> Rc<[Rc<Self>]> {
        LAYERS.with(Rc::clone)
    }
}
pub struct CadColorOption {
    name: String,
    swatch: Ref<SolidColorBrush>,
    special_opacity: f64,
}
impl CadColorOption {
    pub fn new(name: impl Into<String>, color: Option<Color>) -> Rc<Self> {
        Self::with_special(name, color, false)
    }
    pub fn with_special(name: impl Into<String>, color: Option<Color>, special: bool) -> Rc<Self> {
        Rc::new(Self {
            name: name.into(),
            swatch: SolidColorBrush::with_color(color.unwrap_or(Color::from_argb(0, 0, 0, 0))),
            special_opacity: if special { 1. } else { 0. },
        })
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn swatch(&self) -> Ref<SolidColorBrush> {
        self.swatch.clone()
    }
    pub fn special_opacity(&self) -> f64 {
        self.special_opacity
    }
    pub fn samples() -> Rc<[Rc<Self>]> {
        COLORS.with(Rc::clone)
    }
}
pub struct CadLinetype {
    name: String,
    description: String,
    dashes: Option<MediaCollection<f64>>,
}
impl CadLinetype {
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        dashes: Option<MediaCollection<f64>>,
    ) -> Rc<Self> {
        Rc::new(Self {
            name: name.into(),
            description: description.into(),
            dashes,
        })
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn description(&self) -> &str {
        &self.description
    }
    pub fn dashes(&self) -> Option<MediaCollection<f64>> {
        self.dashes.clone()
    }
    pub fn samples() -> Rc<[Rc<Self>]> {
        LINETYPES.with(Rc::clone)
    }
}
pub struct CadLineweight {
    name: String,
    thickness: f64,
}
impl CadLineweight {
    pub fn new(name: impl Into<String>, thickness: f64) -> Rc<Self> {
        Rc::new(Self {
            name: name.into(),
            thickness,
        })
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn thickness(&self) -> f64 {
        self.thickness
    }
    pub fn samples() -> Rc<[Rc<Self>]> {
        WEIGHTS.with(Rc::clone)
    }
}
impl fmt::Display for CadLayer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)
    }
}
impl fmt::Display for CadColorOption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)
    }
}
impl fmt::Display for CadLinetype {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)
    }
}
impl fmt::Display for CadLineweight {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)
    }
}
ferroui_base::ferro_thread_local! {
static LAYERS:Rc<[Rc<CadLayer>]>=Rc::from(vec![
CadLayer::with_options("0",Color::from_argb(255,242,242,242),true,false,false),
CadLayer::with_options("A-WALL",Color::from_argb(255,242,242,242),true,false,false),
CadLayer::with_options("A-DOOR",Color::from_argb(255,255,213,79),true,false,false),
CadLayer::with_options("A-GLAZ",Color::from_argb(255,77,208,225),true,false,false),
CadLayer::with_options("A-FURN",Color::from_argb(255,156,204,101),true,false,true),
CadLayer::with_options("A-ANNO-DIMS",Color::from_argb(255,239,83,80),true,false,false),
CadLayer::with_options("A-ANNO-TEXT",Color::from_argb(255,242,242,242),true,false,false),
CadLayer::with_options("A-HATCH",Color::from_argb(255,140,154,171),true,true,false),
CadLayer::with_options("A-AREA",Color::from_argb(255,179,136,255),false,false,false),
CadLayer::with_options("CENTER",Color::from_argb(255,179,136,255),true,false,false),
CadLayer::with_options("DEFPOINTS",Color::from_argb(255,242,242,242),false,false,true),
]);
static COLORS:Rc<[Rc<CadColorOption>]>=Rc::from(vec![
CadColorOption::with_special("ByLayer",Some(Color::from_argb(255,242,242,242)),true),
CadColorOption::with_special("ByBlock",Some(Color::from_argb(255,242,242,242)),true),
CadColorOption::with_special("Red",Some(Color::from_argb(255,255,0,0)),false),
CadColorOption::with_special("Yellow",Some(Color::from_argb(255,255,255,0)),false),
CadColorOption::with_special("Green",Some(Color::from_argb(255,0,255,0)),false),
CadColorOption::with_special("Cyan",Some(Color::from_argb(255,0,255,255)),false),
CadColorOption::with_special("Blue",Some(Color::from_argb(255,0,0,255)),false),
CadColorOption::with_special("Magenta",Some(Color::from_argb(255,255,0,255)),false),
CadColorOption::with_special("White",Some(Color::from_argb(255,255,255,255)),false),
CadColorOption::with_special("Select Colors...",Some(Color::from_argb(0,0,0,0)),false),
]);
static LINETYPES:Rc<[Rc<CadLinetype>]>=Rc::from(vec![
CadLinetype::new("ByLayer","",None),
CadLinetype::new("ByBlock","",None),
CadLinetype::new("Continuous","Solid line",None),
CadLinetype::new("CENTER","Center ____ _ ____ _",Some(MediaCollection::from_items([f64::from_bits(4620693217682128896),f64::from_bits(4611686018427387904),f64::from_bits(4611686018427387904),f64::from_bits(4611686018427387904)]))),
CadLinetype::new("DASHED","Dashed __ __ __ __",Some(MediaCollection::from_items([f64::from_bits(4616189618054758400),f64::from_bits(4611686018427387904)]))),
CadLinetype::new("HIDDEN","Hidden _ _ _ _ _ _",Some(MediaCollection::from_items([f64::from_bits(4611686018427387904),f64::from_bits(4611686018427387904)]))),
CadLinetype::new("PHANTOM","Phantom ____ _ _ ____",Some(MediaCollection::from_items([f64::from_bits(4621256167635550208),f64::from_bits(4611686018427387904),f64::from_bits(4611686018427387904),f64::from_bits(4611686018427387904),f64::from_bits(4611686018427387904),f64::from_bits(4611686018427387904)]))),
CadLinetype::new("DOT","Dot . . . . . . .",Some(MediaCollection::from_items([f64::from_bits(4602678819172646912),f64::from_bits(4611686018427387904)]))),
]);
static WEIGHTS:Rc<[Rc<CadLineweight>]>=Rc::from(vec![
CadLineweight::new("ByLayer",f64::from_bits(4607182418800017408)),
CadLineweight::new("ByBlock",f64::from_bits(4607182418800017408)),
CadLineweight::new("Default",f64::from_bits(4607182418800017408)),
CadLineweight::new("0.00 mm",f64::from_bits(4607182418800017408)),
CadLineweight::new("0.13 mm",f64::from_bits(4607182418800017408)),
CadLineweight::new("0.25 mm",f64::from_bits(4609434218613702656)),
CadLineweight::new("0.35 mm",f64::from_bits(4611686018427387904)),
CadLineweight::new("0.50 mm",f64::from_bits(4613937818241073152)),
CadLineweight::new("0.70 mm",f64::from_bits(4616189618054758400)),
CadLineweight::new("1.00 mm",f64::from_bits(4617315517961601024)),
]);
}

// A stable, read-only native items view of each shared Rust sample slice.
// This is the concrete XAML ItemsSource boundary; it does not emulate IEnumerable.
use ferroui_base::ferro_markup_type;
use ferroui_controls::ItemsSource;
ferroui_base::ferro_thread_local! {
 static LAYER_ITEMS:ItemsSource=ItemsSource::from_values(CadLayer::samples().iter().cloned());
 static COLOR_ITEMS:ItemsSource=ItemsSource::from_values(CadColorOption::samples().iter().cloned());
 static LINETYPE_ITEMS:ItemsSource=ItemsSource::from_values(CadLinetype::samples().iter().cloned());
 static WEIGHT_ITEMS:ItemsSource=ItemsSource::from_values(CadLineweight::samples().iter().cloned());
}
impl PartialEq for CadLayer {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl PartialEq for CadColorOption {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl PartialEq for CadLinetype {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl PartialEq for CadLineweight {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
ferro_markup_type!(class CadLayer {
 namespace:"FerroRibbon.Sample.Cad",this:Rc<CadLayer>,handles:[Rc<CadLayer>,Option<Rc<CadLayer>>],
 constructors:[(String,Color)=>CadLayer::new,(String,Color,bool,bool,bool)=>CadLayer::with_options],
 properties:[Name:String{get:|this:&Rc<CadLayer>|this.name().to_owned()},Color:Color{get:|this:&Rc<CadLayer>|this.color()},ColorBrush:Ref<SolidColorBrush>{get:|this:&Rc<CadLayer>|this.color_brush()},IsOn:bool{get:|this:&Rc<CadLayer>|this.is_on()},IsFrozen:bool{get:|this:&Rc<CadLayer>|this.is_frozen()},IsLocked:bool{get:|this:&Rc<CadLayer>|this.is_locked()},OnIcon:Option<Rc<RibbonIcon>>{get:|this:&Rc<CadLayer>|this.on_icon()},FreezeIcon:Option<Rc<RibbonIcon>>{get:|this:&Rc<CadLayer>|this.freeze_icon()},LockIcon:Option<Rc<RibbonIcon>>{get:|this:&Rc<CadLayer>|this.lock_icon()}],
 static_properties:[Samples:ItemsSource{get:||LAYER_ITEMS.with(Clone::clone)}],
 methods:[fn ToString()->String => |this:&Rc<CadLayer>|this.to_string()]
});
ferro_markup_type!(class CadColorOption {
 namespace:"FerroRibbon.Sample.Cad",this:Rc<CadColorOption>,handles:[Rc<CadColorOption>,Option<Rc<CadColorOption>>],
 constructors:[(String,Option<Color>)=>CadColorOption::new,(String,Option<Color>,bool)=>CadColorOption::with_special],
 properties:[Name:String{get:|this:&Rc<CadColorOption>|this.name().to_owned()},Swatch:Ref<SolidColorBrush>{get:|this:&Rc<CadColorOption>|this.swatch()},SpecialOpacity:f64{get:|this:&Rc<CadColorOption>|this.special_opacity()}],
 static_properties:[Samples:ItemsSource{get:||COLOR_ITEMS.with(Clone::clone)}],
 methods:[fn ToString()->String => |this:&Rc<CadColorOption>|this.to_string()]
});
ferro_markup_type!(class CadLinetype {
 namespace:"FerroRibbon.Sample.Cad",this:Rc<CadLinetype>,handles:[Rc<CadLinetype>,Option<Rc<CadLinetype>>],
 constructors:[(String,String,Option<MediaCollection<f64>>)=>CadLinetype::new],
 properties:[Name:String{get:|this:&Rc<CadLinetype>|this.name().to_owned()},Description:String{get:|this:&Rc<CadLinetype>|this.description().to_owned()},Dashes:Option<MediaCollection<f64>>{get:|this:&Rc<CadLinetype>|this.dashes()}],
 static_properties:[Samples:ItemsSource{get:||LINETYPE_ITEMS.with(Clone::clone)}],
 methods:[fn ToString()->String => |this:&Rc<CadLinetype>|this.to_string()]
});
ferro_markup_type!(class CadLineweight {
 namespace:"FerroRibbon.Sample.Cad",this:Rc<CadLineweight>,handles:[Rc<CadLineweight>,Option<Rc<CadLineweight>>],
 constructors:[(String,f64)=>CadLineweight::new],
 properties:[Name:String{get:|this:&Rc<CadLineweight>|this.name().to_owned()},Thickness:f64{get:|this:&Rc<CadLineweight>|this.thickness()}],
 static_properties:[Samples:ItemsSource{get:||WEIGHT_ITEMS.with(Clone::clone)}],
 methods:[fn ToString()->String => |this:&Rc<CadLineweight>|this.to_string()]
});

pub fn register_types() {
    use ferroui_base::{
        data::core::ValueTypes,
        metadata::{MarkupType, MarkupTyped},
    };
    MarkupType::register_all(&[
        CadLayer::MARKUP,
        CadColorOption::MARKUP,
        CadLinetype::MARKUP,
        CadLineweight::MARKUP,
    ]);
    ValueTypes::register_reference::<CadLayer>();
    ValueTypes::register_reference::<CadColorOption>();
    ValueTypes::register_reference::<CadLinetype>();
    ValueTypes::register_reference::<CadLineweight>();
    ValueTypes::register_display::<CadLayer>();
    ValueTypes::register_display::<CadColorOption>();
    ValueTypes::register_display::<CadLinetype>();
    ValueTypes::register_display::<CadLineweight>();
}

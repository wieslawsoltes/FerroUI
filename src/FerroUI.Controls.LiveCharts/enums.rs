/// The chart calculation selected by a native host.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ChartKind {
    #[default]
    Cartesian,
    Pie,
    Polar,
    Sankey,
    Treemap,
    GeoMap,
}
ferroui_base::ferro_markup_enum!(ChartKind {
    Cartesian,
    Pie,
    Polar,
    Sankey,
    Treemap,
    GeoMap
});

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum LegendPosition {
    #[default]
    Hidden,
    Top,
    Left,
    Right,
    Bottom,
}
ferroui_base::ferro_markup_enum!(LegendPosition {
    Hidden,
    Top,
    Left,
    Right,
    Bottom
});

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TooltipPosition {
    Hidden,
    #[default]
    Auto,
    Top,
    Bottom,
    Left,
    Right,
    Center,
}
ferroui_base::ferro_markup_enum!(TooltipPosition {
    Hidden,
    Auto,
    Top,
    Bottom,
    Left,
    Right,
    Center
});

bitflags::bitflags! {
    #[derive(Clone,Copy,Debug,Default,PartialEq,Eq,Hash)]
    pub struct ZoomAndPanMode:u32 {
        const NONE=0;
        const PAN_X=1;
        const ZOOM_X=2;
        const PAN_Y=4;
        const ZOOM_Y=8;
        const NO_FIT=16;
        const NO_ZOOM_BY_SECTION=32;
        const INVERT_PANNING_POINTER_TRIGGER=64;
        const X=Self::PAN_X.bits()|Self::ZOOM_X.bits();
        const Y=Self::PAN_Y.bits()|Self::ZOOM_Y.bits();
        const BOTH=Self::X.bits()|Self::Y.bits();
    }
}
ferroui_base::ferro_markup_enum!(flags ZoomAndPanMode {
    None=ZoomAndPanMode::NONE,PanX=ZoomAndPanMode::PAN_X,ZoomX=ZoomAndPanMode::ZOOM_X,
    PanY=ZoomAndPanMode::PAN_Y,ZoomY=ZoomAndPanMode::ZOOM_Y,
    X=ZoomAndPanMode::X,Y=ZoomAndPanMode::Y,Both=ZoomAndPanMode::BOTH,
    NoFit=ZoomAndPanMode::NO_FIT,NoZoomBySection=ZoomAndPanMode::NO_ZOOM_BY_SECTION,
    InvertPanningPointerTrigger=ZoomAndPanMode::INVERT_PANNING_POINTER_TRIGGER
});

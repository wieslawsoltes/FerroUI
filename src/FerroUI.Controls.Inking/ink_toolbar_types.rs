//! Public toolbar discriminants, matching the WinUI inking contract.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum InkToolbarButtonFlyoutPlacement {
    #[default]
    Auto,
    Top,
    Bottom,
    Left,
    Right,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum InkToolbarFlyoutItemKind {
    #[default]
    Simple,
    Radio,
    Check,
    RadioCheck,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum InkToolbarInitialControls {
    #[default]
    All,
    None,
    PensOnly,
    AllExceptPens,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum InkToolbarMenuKind {
    #[default]
    Stencil,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum InkToolbarStencilKind {
    #[default]
    Ruler,
    Protractor,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum InkToolbarToggle {
    #[default]
    Ruler,
    Custom,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum InkToolbarTool {
    #[default]
    BallpointPen,
    Pencil,
    Highlighter,
    Eraser,
    CustomPen,
    CustomTool,
}

ferroui_base::ferro_markup_enum!(InkToolbarButtonFlyoutPlacement {Auto,Top,Bottom,Left,Right},{namespace:"FerroUI.Controls"});
ferroui_base::ferro_markup_enum!(InkToolbarFlyoutItemKind {Simple,Radio,Check,RadioCheck},{namespace:"FerroUI.Controls"});
ferroui_base::ferro_markup_enum!(InkToolbarInitialControls {All,None,PensOnly,AllExceptPens},{namespace:"FerroUI.Controls"});
ferroui_base::ferro_markup_enum!(InkToolbarMenuKind {Stencil},{namespace:"FerroUI.Controls"});
ferroui_base::ferro_markup_enum!(InkToolbarStencilKind {Ruler,Protractor},{namespace:"FerroUI.Controls"});
ferroui_base::ferro_markup_enum!(InkToolbarToggle {Ruler,Custom},{namespace:"FerroUI.Controls"});
ferroui_base::ferro_markup_enum!(InkToolbarTool {BallpointPen,Pencil,Highlighter,Eraser,CustomPen,CustomTool},{namespace:"FerroUI.Controls"});

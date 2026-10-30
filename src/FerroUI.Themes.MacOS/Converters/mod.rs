//! The value converters of the WinUI theme: the converters of microsoft-ui-xaml that the theme
//! resources name and FerroUI's control set has no counterpart of (docs/porting/winui-theme.md,
//! section 5). Markup names them with `xmlns:winuiconverters="using:FerroUI.Themes.MacOS.Converters"`.

mod corner_radius_to_thickness_converter;

pub use corner_radius_to_thickness_converter::{CornerRadiusToThicknessConverter, CornerRadiusToThicknessConverterKind};

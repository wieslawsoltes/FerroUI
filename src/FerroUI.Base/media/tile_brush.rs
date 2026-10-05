use crate::media::{AlignmentX, AlignmentY, Brush, BrushImpl, BrushImplExt, Stretch, TileMode};
use crate::rendering::composition::generated::ServerCompositionSimpleTileBrushProps;
use crate::rendering::composition::transport::BatchStreamWriter;
use crate::rendering::composition::Compositor;
use crate::{ferro_class, ferro_impl_classes, ferro_property, FerroObjectImpl, FerroProperty, RelativeRect, StyledProperty};

/// Base class for brushes which display repeating images.
#[repr(C)]
pub struct TileBrush {
    base: Brush,
}

ferro_class!(TileBrush: Brush);
ferro_impl_classes!(TileBrush: FerroObjectImpl);

impl BrushImpl for TileBrush {
    fn serialize_changes(this: &Self, c: &Compositor, writer: &mut BatchStreamWriter<'_>) {
        Self::parent_serialize_changes(this, c, writer);
        ServerCompositionSimpleTileBrushProps::serialize_all_changes(
            writer,
            this.alignment_x(),
            this.alignment_y(),
            this.destination_rect(),
            this.source_rect(),
            this.stretch(),
            this.tile_mode(),
        );
    }
}

crate::ferro_properties! { impl TileBrush {
    ferro_property!(pub fn alignment_x_property() -> StyledProperty<AlignmentX> {
        FerroProperty::register::<TileBrush, _>("AlignmentX", AlignmentX::Center)
    });

    ferro_property!(pub fn alignment_y_property() -> StyledProperty<AlignmentY> {
        FerroProperty::register::<TileBrush, _>("AlignmentY", AlignmentY::Center)
    });

    ferro_property!(pub fn destination_rect_property() -> StyledProperty<RelativeRect> {
        FerroProperty::register::<TileBrush, _>("DestinationRect", RelativeRect::FILL)
    });

    ferro_property!(pub fn source_rect_property() -> StyledProperty<RelativeRect> {
        FerroProperty::register::<TileBrush, _>("SourceRect", RelativeRect::FILL)
    });

    ferro_property!(pub fn stretch_property() -> StyledProperty<Stretch> {
        FerroProperty::register::<TileBrush, _>("Stretch", Stretch::Uniform)
    });

    ferro_property!(pub fn tile_mode_property() -> StyledProperty<TileMode> {
        FerroProperty::register::<TileBrush, _>("TileMode", TileMode::None)
    });
} }

impl TileBrush {
    /// Creates the class data.
    pub fn construct() -> Self {
        Self { base: Brush::construct() }
    }

    /// The horizontal alignment of a tile in the destination.
    pub fn alignment_x(&self) -> AlignmentX {
        self.get_value(Self::alignment_x_property())
    }

    pub fn set_alignment_x(&self, value: AlignmentX) {
        self.set_value(Self::alignment_x_property(), value)
    }

    /// The vertical alignment of a tile in the destination.
    pub fn alignment_y(&self) -> AlignmentY {
        self.get_value(Self::alignment_y_property())
    }

    pub fn set_alignment_y(&self, value: AlignmentY) {
        self.set_value(Self::alignment_y_property(), value)
    }

    /// The rectangle on the destination in which to paint a tile.
    pub fn destination_rect(&self) -> RelativeRect {
        self.get_value(Self::destination_rect_property())
    }

    pub fn set_destination_rect(&self, value: RelativeRect) {
        self.set_value(Self::destination_rect_property(), value)
    }

    /// The rectangle of the source image that will be displayed.
    pub fn source_rect(&self) -> RelativeRect {
        self.get_value(Self::source_rect_property())
    }

    pub fn set_source_rect(&self, value: RelativeRect) {
        self.set_value(Self::source_rect_property(), value)
    }

    /// A value controlling how the source rectangle will be stretched to
    /// fill the destination rect.
    pub fn stretch(&self) -> Stretch {
        self.get_value(Self::stretch_property())
    }

    pub fn set_stretch(&self, value: Stretch) {
        self.set_value(Self::stretch_property(), value)
    }

    /// The brush's tile mode.
    pub fn tile_mode(&self) -> TileMode {
        self.get_value(Self::tile_mode_property())
    }

    pub fn set_tile_mode(&self, value: TileMode) {
        self.set_value(Self::tile_mode_property(), value)
    }
}

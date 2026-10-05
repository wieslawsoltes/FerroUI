use bitflags::bitflags;

bitflags! {
    /// The per-point flags of a simple glyph.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
    pub struct GlyphFlag: u8 {
        const None = 0x00;
        const OnCurvePoint = 0x01;
        const XShortVector = 0x02;
        const YShortVector = 0x04;
        const Repeat = 0x08;
        const XIsSameOrPositiveXShortVector = 0x10;
        const YIsSameOrPositiveYShortVector = 0x20;
        const OverlapSimple = 0x40;
        const Reserved = 0x80;
    }
}

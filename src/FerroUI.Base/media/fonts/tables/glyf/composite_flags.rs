use bitflags::bitflags;

bitflags! {
    /// The per-component flags of a composite glyph.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
    pub struct CompositeFlags: u16 {
        const ArgsAreWords = 0x0001;
        const ArgsAreXYValues = 0x0002;
        const RoundXYToGrid = 0x0004;
        const WeHaveAScale = 0x0008;
        const MoreComponents = 0x0020;
        const WeHaveAnXAndYScale = 0x0040;
        const WeHaveATwoByTwo = 0x0080;
        const WeHaveInstructions = 0x0100;
        const UseMyMetrics = 0x0200;
        const OverlapCompound = 0x0400;
        /// Must be ignored.
        const Reserved = 0x1000;
        const ScaledComponentOffset = 0x2000;
        const UnscaledComponentOffset = 0x4000;
    }
}

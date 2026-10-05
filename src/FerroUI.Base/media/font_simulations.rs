bitflags::bitflags! {
    /// Specifies algorithmic style simulations to be applied to a typeface.
    /// Bold and oblique simulations can be combined.
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
    pub struct FontSimulations: u8 {
        /// No simulations are performed.
        const None = 0x0000;
        /// Algorithmic emboldening is applied.
        const Bold = 0x0001;
        /// Algorithmic italicization is applied.
        const Oblique = 0x0002;
    }
}

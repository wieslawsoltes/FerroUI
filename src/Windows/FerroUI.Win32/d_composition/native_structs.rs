//! The structures of DirectComposition the interfaces name.

/// A rational number: the rate of composition.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(missing_docs)]
pub struct DXGI_RATIONAL {
    pub numerator: u32,
    pub denominator: u32,
}

/// The timing of the frames of a composition device
/// (`IDCompositionDevice::GetFrameStatistics`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(missing_docs)]
pub struct DCOMPOSITION_FRAME_STATISTICS {
    pub last_frame_time: i64,
    pub current_composition_rate: DXGI_RATIONAL,
    pub current_time: i64,
    pub time_frequency: i64,
    pub next_estimated_frame_time: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_layout_of_the_frame_statistics_is_the_one_of_the_system() {
        // dcomptypes.h: a LARGE_INTEGER, a DXGI_RATIONAL and three
        // LARGE_INTEGER, 8 bytes each.
        assert_eq!(std::mem::size_of::<DXGI_RATIONAL>(), 8);
        assert_eq!(std::mem::size_of::<DCOMPOSITION_FRAME_STATISTICS>(), 40);
        assert_eq!(std::mem::offset_of!(DCOMPOSITION_FRAME_STATISTICS, current_composition_rate), 8);
        assert_eq!(std::mem::offset_of!(DCOMPOSITION_FRAME_STATISTICS, current_time), 16);
        assert_eq!(std::mem::offset_of!(DCOMPOSITION_FRAME_STATISTICS, next_estimated_frame_time), 32);
    }
}

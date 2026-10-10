//! What a compositor says about a top-level between two
//! `xdg_surface.configure` events (the port of `XdgConfigureBatch.cs`).

use super::decoration_mode::DecorationMode;
use bitflags::bitflags;
use ferroui_base::{PixelSize, Size};

bitflags! {
    /// The states of `xdg_toplevel.configure`.
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub struct XdgToplevelStates: u32 {
        const MAXIMIZED = 1 << 0;
        const FULLSCREEN = 1 << 1;
        const RESIZING = 1 << 2;
        const ACTIVATED = 1 << 3;
        const TILED_LEFT = 1 << 4;
        const TILED_RIGHT = 1 << 5;
        const TILED_TOP = 1 << 6;
        const TILED_BOTTOM = 1 << 7;
        const SUSPENDED = 1 << 8;
    }
}

/// The events of a top-level that one `xdg_surface.configure` seals.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct XdgConfigureBatch {
    /// `xdg_toplevel.configure_bounds`.
    pub bounds: Option<PixelSize>,
    /// The size of `xdg_toplevel.configure`: the window geometry, zero where the client chooses.
    pub size: PixelSize,
    pub states: XdgToplevelStates,
    /// The serial of `xdg_surface.configure`.
    pub serial: u32,
    /// The largest size a window sized to its content may take.
    pub max_size: Size,
    /// The answer of the decoration manager that came before the first configure.
    pub initial_decoration_mode: Option<DecorationMode>,
}

impl XdgConfigureBatch {
    /// The states of the array of `xdg_toplevel.configure`: 32-bit values in the byte order of
    /// the machine. Values the port does not know, and bytes after the last whole value, are
    /// ignored.
    pub fn parse_states(data: &[u8]) -> XdgToplevelStates {
        let mut states = XdgToplevelStates::empty();
        for value in data.chunks_exact(4) {
            let value = u32::from_ne_bytes([value[0], value[1], value[2], value[3]]);
            states |= match value {
                1 => XdgToplevelStates::MAXIMIZED,
                2 => XdgToplevelStates::FULLSCREEN,
                3 => XdgToplevelStates::RESIZING,
                4 => XdgToplevelStates::ACTIVATED,
                5 => XdgToplevelStates::TILED_LEFT,
                6 => XdgToplevelStates::TILED_RIGHT,
                7 => XdgToplevelStates::TILED_TOP,
                8 => XdgToplevelStates::TILED_BOTTOM,
                9 => XdgToplevelStates::SUSPENDED,
                _ => XdgToplevelStates::empty(),
            };
        }
        states
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests for this file.
    use super::*;

    fn array(values: &[u32]) -> Vec<u8> {
        values.iter().flat_map(|value| value.to_ne_bytes()).collect()
    }

    #[test]
    fn the_states_of_a_configure_are_read_from_its_array() {
        assert_eq!(XdgConfigureBatch::parse_states(&[]), XdgToplevelStates::empty());
        assert_eq!(
            XdgConfigureBatch::parse_states(&array(&[4, 1])),
            XdgToplevelStates::ACTIVATED | XdgToplevelStates::MAXIMIZED
        );
        assert_eq!(
            XdgConfigureBatch::parse_states(&array(&[2, 5, 6, 7, 8, 9, 3])),
            XdgToplevelStates::FULLSCREEN
                | XdgToplevelStates::TILED_LEFT
                | XdgToplevelStates::TILED_RIGHT
                | XdgToplevelStates::TILED_TOP
                | XdgToplevelStates::TILED_BOTTOM
                | XdgToplevelStates::SUSPENDED
                | XdgToplevelStates::RESIZING
        );
    }

    #[test]
    fn unknown_states_and_a_broken_array_are_ignored() {
        assert_eq!(XdgConfigureBatch::parse_states(&array(&[0, 10, 4000])), XdgToplevelStates::empty());
        let mut data = array(&[4]);
        data.extend_from_slice(&[1, 0]);
        assert_eq!(XdgConfigureBatch::parse_states(&data), XdgToplevelStates::ACTIVATED);
    }
}

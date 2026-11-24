/// Specifies which bounds of a visual its backdrop effect is limited to
/// (`Visual::backdrop_effect`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum BackdropEffectBoundsSource {
    /// The layout bounds of the visual. The backdrop effect is drawn there
    /// also where the visual has no content of its own.
    #[default]
    Layout = 0,
    /// The bounds of the visual's subtree.
    Subtree = 1,
}

/// Selects whether a backdrop effect keeps a snapshot of what is behind the
/// visual, or reads it again on every draw. With `Default` the compositor
/// chooses.
///
/// FerroUI draws every backdrop effect from the tree behind the visual, on
/// the GPU of each backend, with no read-back of the target: the mode is a
/// hint that the compositor may follow (`docs/porting/DEVIATIONS.md`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum BackdropEffectCache {
    /// The compositor chooses.
    #[default]
    Default = 0,
    /// Keeps a snapshot of what is behind the visual and updates only its
    /// changed parts.
    Retained = 1,
    /// Reads what is behind the visual again on every draw.
    Volatile = 2,
}

impl BackdropEffectCache {
    /// Parses a backdrop cache mode name, as the `BackdropEffectCache="Retained"`
    /// attribute syntax does.
    pub fn parse(s: &str) -> Result<Self, String> {
        match s {
            "Default" => Ok(Self::Default),
            "Retained" => Ok(Self::Retained),
            "Volatile" => Ok(Self::Volatile),
            _ => Err(format!("Unknown BackdropEffectCache: {s}")),
        }
    }
}

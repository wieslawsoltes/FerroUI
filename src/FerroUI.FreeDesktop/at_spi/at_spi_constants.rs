//! The port of `AtSpiConstants.cs`.
// Every constant of the reference is here; it does not use all of them either.
#![allow(dead_code)]

use super::at_spi_state::AtSpiState;
use ferroui_base::utilities::CultureInfo;

// D-Bus paths
pub(crate) const ROOT_PATH: &str = "/org/a11y/atspi/accessible/root";
pub(crate) const CACHE_PATH: &str = "/org/a11y/atspi/cache";
pub(crate) const NULL_PATH: &str = "/org/a11y/atspi/null";
/// The prefix of the paths of the nodes. The reference's prefix carries
/// the name of its project (`docs/porting/DEVIATIONS.md`).
pub(crate) const APP_PATH_PREFIX: &str = "/org/ferroui/a11y";
pub(crate) const REGISTRY_PATH: &str = "/org/a11y/atspi/registry";

// Interface names
pub(crate) const IFACE_ACCESSIBLE: &str = "org.a11y.atspi.Accessible";
pub(crate) const IFACE_APPLICATION: &str = "org.a11y.atspi.Application";
pub(crate) const IFACE_COMPONENT: &str = "org.a11y.atspi.Component";
pub(crate) const IFACE_ACTION: &str = "org.a11y.atspi.Action";
pub(crate) const IFACE_VALUE: &str = "org.a11y.atspi.Value";
pub(crate) const IFACE_EVENT_OBJECT: &str = "org.a11y.atspi.Event.Object";
pub(crate) const IFACE_EVENT_WINDOW: &str = "org.a11y.atspi.Event.Window";
pub(crate) const IFACE_CACHE: &str = "org.a11y.atspi.Cache";
pub(crate) const IFACE_SELECTION: &str = "org.a11y.atspi.Selection";
pub(crate) const IFACE_IMAGE: &str = "org.a11y.atspi.Image";
pub(crate) const IFACE_TEXT: &str = "org.a11y.atspi.Text";
pub(crate) const IFACE_EDITABLE_TEXT: &str = "org.a11y.atspi.EditableText";
pub(crate) const IFACE_COLLECTION: &str = "org.a11y.atspi.Collection";

// Bus names
pub(crate) const BUS_NAME_REGISTRY: &str = "org.a11y.atspi.Registry";
pub(crate) const BUS_NAME_A11Y: &str = "org.a11y.Bus";
pub(crate) const PATH_A11Y: &str = "/org/a11y/bus";

// Interface versions
pub(crate) const ACCESSIBLE_VERSION: u32 = 1;
pub(crate) const APPLICATION_VERSION: u32 = 1;
pub(crate) const COMPONENT_VERSION: u32 = 1;
pub(crate) const ACTION_VERSION: u32 = 1;
pub(crate) const VALUE_VERSION: u32 = 1;
pub(crate) const EVENT_OBJECT_VERSION: u32 = 1;
pub(crate) const EVENT_WINDOW_VERSION: u32 = 1;
pub(crate) const CACHE_VERSION: u32 = 1;
pub(crate) const IMAGE_VERSION: u32 = 1;
pub(crate) const SELECTION_VERSION: u32 = 1;
pub(crate) const TEXT_VERSION: u32 = 1;
pub(crate) const EDITABLE_TEXT_VERSION: u32 = 1;
pub(crate) const COLLECTION_VERSION: u32 = 1;

pub(crate) const WIDGET_LAYER: u32 = 3;
pub(crate) const WINDOW_LAYER: u32 = 7;

/// The name of the toolkit, as the application interface and the
/// attributes of a node report it.
pub(crate) const TOOLKIT_NAME: &str = "FerroUI";

/// The states as the two words of a state set.
pub(crate) fn build_state_set(states: &[AtSpiState]) -> Vec<u32> {
    if states.is_empty() {
        return vec![0, 0];
    }

    let mut low = 0u32;
    let mut high = 0u32;
    for state in states {
        let bit = *state as u32;
        if bit < 32 {
            low |= 1u32 << bit;
        } else if bit < 64 {
            high |= 1u32 << (bit - 32);
        }
    }

    vec![low, high]
}

pub(crate) fn resolve_locale() -> String {
    locale_from_culture_name(CultureInfo::current_ui_culture().name())
}

pub(crate) fn locale_from_culture_name(culture: &str) -> String {
    let culture = if culture.trim().is_empty() { "en_US" } else { culture };
    culture.replace('-', "_")
}

pub(crate) fn resolve_toolkit_version() -> String {
    // The reference reports the version of its assembly.
    env!("CARGO_PKG_VERSION").to_string()
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests of this project.
    use super::*;

    #[test]
    fn a_state_set_is_two_words() {
        assert_eq!(build_state_set(&[]), vec![0, 0]);
        assert_eq!(build_state_set(&[AtSpiState::Active]), vec![0b10, 0]);
        assert_eq!(
            build_state_set(&[AtSpiState::Enabled, AtSpiState::ManagesDescendants, AtSpiState::Indeterminate]),
            vec![(1 << 8) | (1 << 31), 1]
        );
        assert_eq!(build_state_set(&[AtSpiState::ReadOnly, AtSpiState::Checkable]), vec![0, (1 << 11) | (1 << 9)]);
        // A state given twice is one bit.
        assert_eq!(build_state_set(&[AtSpiState::Focused, AtSpiState::Focused]), vec![1 << 12, 0]);
    }

    #[test]
    fn the_locale_has_an_underscore() {
        assert_eq!(locale_from_culture_name("en-US"), "en_US");
        assert_eq!(locale_from_culture_name("pl-PL"), "pl_PL");
        assert_eq!(locale_from_culture_name(""), "en_US");
        assert_eq!(locale_from_culture_name("  "), "en_US");
    }

    #[test]
    fn the_paths_of_the_port_do_not_name_the_reference() {
        assert_eq!(APP_PATH_PREFIX, "/org/ferroui/a11y");
        assert!(zbus::zvariant::ObjectPath::try_from(format!("{APP_PATH_PREFIX}/1")).is_ok());
        for path in [ROOT_PATH, CACHE_PATH, NULL_PATH, REGISTRY_PATH, PATH_A11Y] {
            assert!(zbus::zvariant::ObjectPath::try_from(path).is_ok());
        }
    }
}

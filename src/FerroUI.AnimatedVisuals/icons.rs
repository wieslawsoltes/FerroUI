//! The animated icons of WinUI (`AnimatedBackVisualSource` and the others), as Lottie documents
//! made from the generated sources by `scripts/animated-visuals/winui_icons.py`. A document is
//! linked only into an application that names its class.

use crate::source::{AnimatedVisualSource, LottieVisualSource};
use ferroui_base::{ferro_class, ferro_class_info, instantiate, FerroObjectImpl, Ref};

macro_rules! animated_icon_sources {
    ($($(#[$meta:meta])* $name:ident => $file:literal,)*) => {
        $(
            $(#[$meta])*
            #[repr(C)]
            pub struct $name {
                base: LottieVisualSource,
            }

            ferro_class!($name: LottieVisualSource);
            ferro_class_info!($name {
                new: $name::new,
                markup: { namespace: "FerroUI.AnimatedVisuals" },
            });

            impl FerroObjectImpl for $name {}

            impl $name {
                pub fn new() -> Ref<Self> {
                    let this = instantiate(Self { base: LottieVisualSource::construct() });
                    this.set_source(include_str!($file));
                    this
                }
            }
        )*

        /// The type of each icon source, for the table of markup types.
        pub(crate) const ICON_SOURCE_TYPES: &[&ferroui_base::TypeInfo] = &[$($name::TYPE),*];

        /// The documents of all icon sources, by class name.
        #[cfg(test)]
        pub(crate) const ICON_DOCUMENTS: &[(&str, &str)] = &[$((stringify!($name), include_str!($file))),*];

        /// An icon source as the base class, by class name.
        #[cfg(test)]
        pub(crate) fn icon_source(name: &str) -> Option<Ref<AnimatedVisualSource>> {
            match name {
                $(stringify!($name) => Some($name::new().upcast()),)*
                _ => None,
            }
        }
    };
}

animated_icon_sources! {
    /// The check mark of a check box.
    AnimatedAcceptVisualSource => "icons/Accept.json",
    /// The arrow of a back button.
    AnimatedBackVisualSource => "icons/Back.json",
    /// The chevron of a drop-down.
    AnimatedChevronDownSmallVisualSource => "icons/ChevronDownSmall.json",
    /// A chevron that turns from right to down.
    AnimatedChevronRightDownSmallVisualSource => "icons/ChevronRightDownSmall.json",
    /// A chevron that turns from up to down.
    AnimatedChevronUpDownSmallVisualSource => "icons/ChevronUpDownSmall.json",
    /// The magnifier of a search box.
    AnimatedFindVisualSource => "icons/Find.json",
    /// The three lines of a navigation button.
    AnimatedGlobalNavigationButtonVisualSource => "icons/GlobalNavigationButton.json",
    /// The gear of a settings button.
    AnimatedSettingsVisualSource => "icons/Settings.json",
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AnimatedVisual, FrameCommand, FrameCommands};

    #[test]
    fn every_icon_is_a_document_with_markers_that_draws_at_each_marker() {
        assert_eq!(ICON_DOCUMENTS.len(), 8);
        for (name, json) in ICON_DOCUMENTS {
            let visual = AnimatedVisual::parse(json).unwrap_or_else(|error| panic!("{name}: {error}"));
            assert_eq!((visual.width, visual.height), (48.0, 48.0), "{name}");
            let markers = visual.progress_markers();
            assert!(markers.len() >= 12, "{name}: {} markers", markers.len());
            let mut commands = FrameCommands::default();
            for (marker, progress) in markers {
                visual.evaluate(visual.frame_at(progress), &mut commands);
                let drawn = commands.commands.iter().any(|command| matches!(command, FrameCommand::Draw(_)));
                // A check mark is there when it is on, and gone when it is off.
                if *name == "AnimatedAcceptVisualSource" {
                    if marker.ends_with("On_End") {
                        assert!(drawn, "{name}: nothing is drawn at {marker}");
                    } else if marker.ends_with("Off_End") {
                        assert!(!drawn, "{name}: a mark is drawn at {marker}");
                    }
                } else {
                    assert!(drawn, "{name}: nothing is drawn at {marker}");
                }
            }
        }
    }

    #[test]
    fn the_paints_of_every_icon_are_bound_to_the_foreground() {
        for (name, json) in ICON_DOCUMENTS {
            let visual = AnimatedVisual::parse(json).unwrap();
            let mut commands = FrameCommands::default();
            visual.evaluate(visual.frame_at(0.0), &mut commands);
            for command in &commands.commands {
                if let FrameCommand::Draw(draw) = command {
                    assert_eq!(draw.binding.as_deref(), Some("Foreground"), "{name}");
                }
            }
        }
    }

    #[test]
    fn a_source_class_gives_its_document() {
        let source = icon_source("AnimatedBackVisualSource").and_then(|source| source.source()).unwrap();
        assert!(source.try_create_animated_visual().is_ok());
    }
}

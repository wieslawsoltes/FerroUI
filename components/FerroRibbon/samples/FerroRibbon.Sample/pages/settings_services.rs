// Copyright (c) 2026 Wiesław Šoltés. MIT. Native services for the source SettingsPage.
use super::SettingsPage;
use crate::{
    demo_settings::DemoSettings,
    settings_view_model::{SettingsSelection, SettingsServices, SettingsWordAction},
};
use ferroribbon::{
    controls::{RibbonCommandPalette, RibbonCustomizePage},
    localization::RibbonFrameworkCulture,
    theming::RibbonTheme,
};
use ferroribbon_core::{
    RibbonDensity, RibbonDisplayMode,
    localization::RibbonStrings,
    theming::{RibbonChromeStyle, RibbonThemePalette, RibbonThemeStyle},
};
use ferroui_base::{
    WeakRef, data::core::ValueTypes, styling::ThemeVariant, utilities::CultureInfo,
};
use ferroui_controls::{TopLevel, primitives::PopupRoot};
use std::rc::Rc;
pub(super) struct NativeSettingsServices {
    pub page: WeakRef<SettingsPage>,
    pub settings: Rc<DemoSettings>,
}
impl SettingsServices for NativeSettingsServices {
    fn select(&self, selection: SettingsSelection) -> Result<(), String> {
        match selection {
            SettingsSelection::Theme(index) => {
                if let Some(page) = self.page.upgrade()
                    && let Some(mut top) = TopLevel::get_top_level(Some(&page))
                {
                    // Native popup roots belong to a source XamlRoot through
                    // their existing parent TopLevel chain. Keep the root content
                    // identity; do not theme only a popup or recreate controls.
                    while let Some(popup) = top.cast::<PopupRoot>() {
                        top = popup
                            .parent_top_level()
                            .ok_or("Settings popup has no parent root")?;
                    }
                    if let Some(content) = top.content()
                        && let Some(root) = ValueTypes::as_object(&*content)
                            .and_then(|v| v.cast::<ferroui_controls::Control>())
                    {
                        root.set_value(
                            ThemeVariant::requested_theme_variant_property(),
                            match index {
                                1 => Some(ThemeVariant::light()),
                                2 => Some(ThemeVariant::dark()),
                                _ => None,
                            },
                        );
                    }
                }
            }
            SettingsSelection::Palette(index) => {
                let palette = if index <= 0 {
                    None
                } else {
                    Some(
                        RibbonThemePalette::presets()
                            .get((index - 1) as usize)
                            .ok_or_else(|| format!("Palette index {index} is out of range"))?
                            .clone(),
                    )
                };
                self.settings.set_palette_override(palette.clone());
                RibbonTheme::apply_palette(Some(palette.unwrap_or_else(RibbonThemePalette::word)))
                    .map_err(|e| e.to_string())?;
            }
            SettingsSelection::Chrome(index) => RibbonTheme::apply_chrome_style(if index == 1 {
                RibbonChromeStyle::Colorful
            } else {
                RibbonChromeStyle::Neutral
            })
            .map_err(|e| e.to_string())?,
            SettingsSelection::Surface(index) => self
                .settings
                .set_surface_style(if index == 1 {
                    RibbonThemeStyle::Cad
                } else {
                    RibbonThemeStyle::Office
                })
                .map_err(|e| e.to_string())?,
            SettingsSelection::Layout {
                density_index,
                layout_index,
            } => self.settings.apply(
                RibbonDensity::from_raw(density_index.max(0)),
                match layout_index {
                    1 => Some(RibbonDisplayMode::Classic),
                    2 => Some(RibbonDisplayMode::Simplified),
                    _ => None,
                },
            ),
            SettingsSelection::Language(language) => {
                let name = language.ok_or("Language has no selected item")?;
                let native = CultureInfo::get_culture_info(&name);
                // The actual picker contains these five source languages. The
                // SDK's unknown-name path does not implement CultureInfo's name
                // validation; report that boundary rather than accept invalid
                // names as an apparently successful English fallback.
                if native.lcid() == 0x1000
                    && !["en", "de", "fr", "es", "pl"]
                        .iter()
                        .any(|x| name.eq_ignore_ascii_case(x))
                {
                    return Err(format!(
                        "Source culture construction is unverified for {name:?}"
                    ));
                }
                let culture = RibbonFrameworkCulture::new(native);
                RibbonStrings::set_current(Some(
                    RibbonStrings::for_culture(Some(culture)).map_err(|e| e.to_string())?,
                ))
                .map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    }
    fn word_action(&self, action: SettingsWordAction) -> Result<Option<String>, String> {
        let ribbon = self.settings.word_ribbon();
        match action {
            SettingsWordAction::ShowState => {
                return Ok(Some(match ribbon {
                    Some(ribbon) => ribbon
                        .save_state_to_json()?
                        .to_utf8()
                        .map_err(|_| "Saved state cannot be represented as native text")?,
                    None => "Open the Word page first.".into(),
                }));
            }
            SettingsWordAction::ResetCustomization => {
                if let Some(ribbon) = ribbon {
                    ribbon.reset_customization()?;
                }
            }
            SettingsWordAction::Customize => {
                if let Some(ribbon) = ribbon {
                    ribbon
                        .show_customize_dialog(RibbonCustomizePage::Ribbon)
                        .map_err(|e| e.to_string())?;
                }
            }
            SettingsWordAction::CommandPalette => {
                if let Some(ribbon) = ribbon {
                    RibbonCommandPalette::show(&ribbon);
                }
            }
        }
        Ok(None)
    }
    fn log(&self) -> Vec<String> {
        self.settings.log().borrow().clone()
    }
}

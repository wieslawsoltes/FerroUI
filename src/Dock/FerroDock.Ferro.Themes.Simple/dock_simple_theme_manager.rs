// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::DockSimpleTheme;
use ferrodock_ferro::themes::{DockPresetThemeManagerBase, DockPresetThemeManagerHooks};
use ferrodock_model::collections::ReadOnlyListHandle;
use ferroui_base::{controls::IResourceDictionary, utilities::Uri};
use ferroui_controls::Application;
use std::rc::Rc;
const PRESET_URI_PREFIX: &str = "ferres://FerroDock.Ferro.Themes.Simple/Presets/Ide/";
ferroui_base::ferro_thread_local! {
    static PRESET_NAMES:ReadOnlyListHandle<String>=ReadOnlyListHandle::from_items(vec!["Default".into(),"VS Code Light".into(),"VS Code Dark".into(),"Rider Light".into(),"Rider Dark".into()]);
    static PRESET_URIS:ReadOnlyListHandle<Uri>=ReadOnlyListHandle::from_items(["Default","VsCodeLight","VsCodeDark","RiderLight","RiderDark"].into_iter().map(|name|Uri::absolute(&format!("{PRESET_URI_PREFIX}{name}.xaml")).expect("a source constant preset URI")).collect());
}
/// Provides the original light/dark and preset switching for Dock Simple resources.
pub struct DockSimpleThemeManager {
    base: DockPresetThemeManagerBase,
}
impl DockSimpleThemeManager {
    /// Creates the original manager with its shared source name/URI arrays.
    pub fn new() -> Rc<Self> {
        Rc::new(Self {
            base: DockPresetThemeManagerBase::new(
                Some(PRESET_NAMES.with(Clone::clone)),
                Some(PRESET_URIS.with(Clone::clone)),
                Some(PRESET_URI_PREFIX.into()),
            )
            .expect("the source preset arrays have matching nonzero counts"),
        })
    }
}
impl DockPresetThemeManagerHooks for DockSimpleThemeManager {
    fn base(&self) -> &DockPresetThemeManagerBase {
        &self.base
    }
    fn try_get_default_preset_owner(
        &self,
        application: &Application,
    ) -> Option<Rc<dyn IResourceDictionary>> {
        for style in application.styles().snapshot().iter() {
            if let Some(theme) = style
                .as_object()
                .and_then(|object| object.downcast_ref::<DockSimpleTheme>())
            {
                return Some(theme.resources().into());
            }
        }
        None
    }
}

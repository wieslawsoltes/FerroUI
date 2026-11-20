//! `Strings/InvariantResources.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Not ported from a document of microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml,
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0): the labels of the commands of the context
//! flyout of the text controls, which WinUI takes from the localized resources of the framework
//! (the labels of the StandardUICommand kinds Cut, Copy, Paste, Undo, Redo and SelectAll that
//! controls/dev/CommandBarFlyout/TextCommandBarFlyout.cpp gives its buttons), not from the theme.
//! FerroUI has no localized resources of its framework, so the labels are string resources of the
//! theme with the keys of the Fluent theme of FerroUI (src/FerroUI.Themes.Fluent/Strings/
//! InvariantResources.xaml), in English: an application replaces them by defining the same keys in
//! its resources, for every theme. The keys of Undo, Redo and Select All are added by this theme.

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::controls::ResourceDictionary;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    // TextBox, PasswordBox and SelectableTextBlock context flyout
    resources.text("StringTextFlyoutCutText", "Cut");
    resources.text("StringTextFlyoutCopyText", "Copy");
    resources.text("StringTextFlyoutPasteText", "Paste");
    resources.text("StringTextFlyoutUndoText", "Undo");
    resources.text("StringTextFlyoutRedoText", "Redo");
    resources.text("StringTextFlyoutSelectAllText", "Select All");
    Ok(())
}

/// The resource dictionary of the document, declared in `scope`.
pub(crate) fn build(scope: &Scope) -> Result<Ref<ResourceDictionary>> {
    let dictionary = ResourceDictionary::new();
    merge(&Resources::new(&dictionary, scope))?;
    Ok(dictionary)
}

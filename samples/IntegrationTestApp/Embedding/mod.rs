//! The native text box the embedding page hosts (namespace `IntegrationTestApp.Embedding`):
//! one module per upstream file.
//!
//! Ported: the control ([`NativeTextBox`]), the contract of its platform specific part
//! ([`INativeTextBoxFactory`], [`INativeTextBoxImpl`]) and the AppKit part
//! (`MacOSTextBoxFactory`, `MacOSViewHandle`, `MacHelper`). The managed original reaches AppKit
//! through a binding package; here the few Objective-C runtime calls the text box needs are in
//! `objc.rs`, which is not a port.
//!
//! Not ported, because their platform is not a platform of the port: `Win32TextBoxFactory.cs`,
//! `Win32WindowControlHandle.cs` and `WinApi.cs`.

use crate::markup::XamlClass;
use ferroui_base::metadata::MarkupType;
use ferroui_base::TypeInfo;

mod i_native_text_box_factory;
mod native_text_box;

#[cfg(target_os = "macos")]
mod mac_helper;
#[cfg(target_os = "macos")]
mod mac_os_text_box_factory;
#[cfg(target_os = "macos")]
mod mac_os_view_handle;
#[cfg(target_os = "macos")]
pub(crate) mod objc;

pub use i_native_text_box_factory::{INativeTextBoxFactory, INativeTextBoxImpl};
pub use native_text_box::NativeTextBox;

#[cfg(target_os = "macos")]
pub use mac_helper::MacHelper;
#[cfg(target_os = "macos")]
pub use mac_os_text_box_factory::MacOSTextBoxFactory;
#[cfg(target_os = "macos")]
pub use mac_os_view_handle::MacOSViewHandle;

/// The classes of this namespace (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[NativeTextBox::TYPE];

/// The classes of this namespace that have a document (`&X::XAML_CLASS`).
pub(crate) const CLASSES: &[&XamlClass] = &[];

/// The types of this namespace declared with `ferro_markup_type!` / `ferro_markup_enum!`
/// (`<X as MarkupTyped>::MARKUP`).
pub(crate) const MARKUP_TYPES: &[&MarkupType] = &[];

/// What the untyped value conversions must know about the types of this namespace
/// (`ValueTypes::register_reference::<X>()`, nullable forms, casts to contracts).
pub(crate) fn register_value_types() {}

/// Makes the typed lists of this namespace known to markup (`ferro_markup_list!`).
pub(crate) fn register_lists() {}

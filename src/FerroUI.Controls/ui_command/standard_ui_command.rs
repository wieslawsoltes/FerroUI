//! WinUI's `StandardUICommand` and `StandardUICommandKind`
//! (`dxaml/xcp/dxaml/lib/StandardUICommand_Partial.cpp` with the English texts of
//! `dxaml/xcp/dxaml/dllsrv/winrt/Microsoft.UI.Xaml.Common.rc`, microsoft-ui-xaml commit
//! `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c) Microsoft Corporation, see
//! NOTICE.md).

use super::{KeyboardAccelerator, XamlUICommand};
use crate::{Symbol, SymbolIconSource};
use ferroui_base::input::{Key, KeyModifiers};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, Ref, StyledProperty,
};
use std::cell::Cell;

/// The platform commands a [`StandardUICommand`] describes (WinUI's `StandardUICommandKind`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum StandardUICommandKind {
    /// No command: the command describes nothing (WinUI reports an error when such a command
    /// enters the tree).
    #[default]
    None = 0,
    Cut = 1,
    Copy = 2,
    Paste = 3,
    SelectAll = 4,
    Delete = 5,
    Share = 6,
    Save = 7,
    Open = 8,
    Close = 9,
    Pause = 10,
    Play = 11,
    Stop = 12,
    Forward = 13,
    Backward = 14,
    Undo = 15,
    Redo = 16,
}

ferroui_base::ferro_markup_enum!(
    StandardUICommandKind {
        None, Cut, Copy, Paste, SelectAll, Delete, Share, Save, Open, Close, Pause, Play, Stop, Forward, Backward,
        Undo, Redo,
    },
    { namespace: "FerroUI.Controls" }
);

/// What a kind sets: the label, the symbol of the icon, the key and modifiers of the accelerator
/// and the description (`PopulateForKind` with the English resource strings).
struct KindProperties {
    label: &'static str,
    symbol: Symbol,
    key: Key,
    modifiers: KeyModifiers,
    description: &'static str,
}

impl StandardUICommandKind {
    fn properties(self) -> Option<KindProperties> {
        use StandardUICommandKind as K;
        let control = KeyModifiers::CONTROL;
        let none = KeyModifiers::NONE;
        let (label, symbol, key, modifiers, description) = match self {
            K::None => return None,
            K::Cut => ("Cut", Symbol::Cut, Key::X, control, "Remove the selected content and put it on the clipboard"),
            K::Copy => ("Copy", Symbol::Copy, Key::C, control, "Copy the selected content to the clipboard"),
            K::Paste => (
                "Paste",
                Symbol::Paste,
                Key::V,
                control,
                "Insert the contents of the clipboard at the current location",
            ),
            K::SelectAll => ("Select All", Symbol::SelectAll, Key::A, control, "Select all content"),
            K::Delete => ("Delete", Symbol::Delete, Key::Delete, none, "Delete the selected content"),
            K::Share => ("Share", Symbol::Share, Key::None, none, "Share the selected content"),
            K::Save => ("Save", Symbol::Save, Key::S, control, "Save your changes"),
            K::Open => ("Open", Symbol::OpenFile, Key::O, control, "Open"),
            K::Close => ("Close", Symbol::Cancel, Key::W, control, "Close"),
            K::Pause => ("Pause", Symbol::Pause, Key::None, none, "Pause"),
            K::Play => ("Play", Symbol::Play, Key::None, none, "Play"),
            K::Stop => ("Stop", Symbol::Stop, Key::None, none, "Stop"),
            K::Forward => ("Forward", Symbol::Forward, Key::None, none, "Go to the next item"),
            K::Backward => ("Backward", Symbol::Back, Key::None, none, "Go to the previous item"),
            K::Undo => ("Undo", Symbol::Undo, Key::Z, control, "Reverse the most recent action"),
            K::Redo => ("Redo", Symbol::Redo, Key::Y, control, "Repeat the most recently undone action"),
        };
        Some(KindProperties { label, symbol, key, modifiers, description })
    }
}

/// An [`XamlUICommand`] of a platform command (WinUI's `StandardUICommand`): its
/// [`kind`](Self::kind) sets the label, the icon (a [`SymbolIconSource`]), the keyboard
/// accelerator and the description of the command.
///
/// A property the application sets itself is the application's from then on: a later change of
/// the kind leaves it as it is.
#[repr(C)]
pub struct StandardUICommand {
    base: XamlUICommand,
    owns_label: Cell<bool>,
    owns_icon_source: Cell<bool>,
    owns_description: Cell<bool>,
    owns_keyboard_accelerator: Cell<bool>,
    setting_property_internally: Cell<bool>,
    previous_accelerator: Cell<(Key, KeyModifiers)>,
}

ferro_class!(StandardUICommand: XamlUICommand);
ferro_class_info!(StandardUICommand {
    new: StandardUICommand::new,
    markup: { namespace: "FerroUI.Controls" },
});

ferro_properties! { impl StandardUICommand {
    /// Defines the `Kind` property. Default [`StandardUICommandKind::None`].
    pub fn kind_property() -> StyledProperty<StandardUICommandKind> {
        FerroProperty::register::<StandardUICommand, _>("Kind", StandardUICommandKind::None)
    }
} }

impl FerroObjectImpl for StandardUICommand {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if this.setting_property_internally.get() || !change.is_effective_value_change() {
            return;
        }
        let property = change.property();
        if property == Self::kind_property().as_property() {
            this.populate_for_kind(this.kind());
        } else if property == XamlUICommand::label_property().as_property() {
            this.owns_label.set(false);
        } else if property == XamlUICommand::icon_source_property().as_property() {
            this.owns_icon_source.set(false);
        } else if property == XamlUICommand::description_property().as_property() {
            this.owns_description.set(false);
        }
    }
}

impl StandardUICommand {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: XamlUICommand::construct(),
            owns_label: Cell::new(true),
            owns_icon_source: Cell::new(true),
            owns_description: Cell::new(true),
            owns_keyboard_accelerator: Cell::new(true),
            setting_property_internally: Cell::new(false),
            previous_accelerator: Cell::new((Key::None, KeyModifiers::NONE)),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Creates the command of a kind (WinUI's `StandardUICommand(StandardUICommandKind)`).
    pub fn with_kind(kind: StandardUICommandKind) -> Ref<Self> {
        let command = Self::new();
        command.set_kind(kind);
        command
    }

    pub fn kind(&self) -> StandardUICommandKind {
        self.get_value(Self::kind_property())
    }

    pub fn set_kind(&self, value: StandardUICommandKind) {
        self.set_value(Self::kind_property(), value);
    }

    fn set_internally(&self, set: impl FnOnce()) {
        self.setting_property_internally.set(true);
        set();
        self.setting_property_internally.set(false);
    }

    fn populate_for_kind(&self, kind: StandardUICommandKind) {
        let Some(properties) = kind.properties() else {
            return;
        };
        if self.owns_label.get() {
            self.set_internally(|| self.set_label(properties.label));
        }
        if self.owns_icon_source.get() {
            let icon = SymbolIconSource::new();
            icon.set_symbol(properties.symbol);
            self.set_internally(|| self.set_icon_source(Some(icon.upcast())));
        }
        self.set_keyboard_accelerator_if_unset(properties.key, properties.modifiers);
        if self.owns_description.get() {
            self.set_internally(|| self.set_description(properties.description));
        }
    }

    /// `SetKeyboardAcceleratorIfUnset`: the accelerators stay the command's own while they are
    /// the one the previous kind set (or none, before any kind set one).
    fn set_keyboard_accelerator_if_unset(&self, key: Key, modifiers: KeyModifiers) {
        if !self.owns_keyboard_accelerator.get() {
            return;
        }
        let accelerators = self.keyboard_accelerators();
        let current = accelerators.snapshot();
        let previous = self.previous_accelerator.get();
        let owned = match current.len() {
            0 => previous.0 == Key::None,
            1 => (current[0].key(), current[0].modifiers()) == previous,
            _ => false,
        };
        if !owned {
            self.owns_keyboard_accelerator.set(false);
            return;
        }
        accelerators.clear();
        if key != Key::None {
            accelerators.add(KeyboardAccelerator::with_key(key, modifiers));
        }
        self.previous_accelerator.set((key, modifiers));
    }
}

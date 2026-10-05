// (c) Copyright Microsoft Corporation.
// This source is subject to the Microsoft Public License (Ms-PL).
// Please see https://go.microsoft.com/fwlink/?LinkID=131993 for details.
// All other rights reserved.
//
// Ported from the Silverlight Toolkit sources as adapted by the upstream
// project; the license text is in the `NOTICE.md` of this crate.

use ferroui_base::input::KeyModifiers;

/// Helpers of the calendar for the state of the modifier keys.
pub(crate) struct CalendarExtensions;

impl CalendarExtensions {
    /// Whether the control and the shift modifiers are part of `modifiers`,
    /// in that order.
    pub(crate) fn get_meta_key_state(modifiers: KeyModifiers) -> (bool, bool) {
        let ctrl = modifiers.contains(KeyModifiers::CONTROL);
        let shift = modifiers.contains(KeyModifiers::SHIFT);
        (ctrl, shift)
    }
}

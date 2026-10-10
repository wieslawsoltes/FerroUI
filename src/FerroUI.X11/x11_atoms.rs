// Permission is hereby granted, free of charge, to any person obtaining
// a copy of this software and associated documentation files (the
// "Software"), to deal in the Software without restriction, including
// without limitation the rights to use, copy, modify, merge, publish,
// distribute, sublicense, and/or sell copies of the Software, and to
// permit persons to whom the Software is furnished to do so, subject to
// the following conditions:
//
// The above copyright notice and this permission notice shall be
// included in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
// MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
// NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE
// LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION
// WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
//
// Copyright (c) 2006 Novell, Inc. (https://www.novell.com)
//
//

//! The atoms of the connection (the port of `X11Atoms.cs`).
//!
//! The reference declares one field per atom and generates, at build time,
//! the method that interns them all in one round trip (`PopulateAtoms`,
//! from the names of the fields). Here the list is written out: the fields
//! of the structure, and the two tables `populate_atoms` reads.
//!
//! The file this is ported from carries the licence of the project it came
//! from, reproduced in the `NOTICE.md` of the crate.

use crate::selections::data_format_helper::{MIME_TYPE_TEXT_PLAIN, MIME_TYPE_TEXT_PLAIN_UTF8};
use crate::xlib::{self, Atom, XDisplay};
use std::cell::{OnceCell, RefCell};
use std::collections::HashMap;

/// The atoms the backend names, interned on the connection it was made
/// for.
#[allow(non_snake_case)]
pub struct X11Atoms {
    display: Option<XDisplay>,

    // Our atoms
    pub PRIMARY: Atom,
    pub SECONDARY: Atom,
    pub ARC: Atom,
    pub ATOM: Atom,
    pub BITMAP: Atom,
    pub CARDINAL: Atom,
    pub COLORMAP: Atom,
    pub CURSOR: Atom,
    pub CUT_BUFFER0: Atom,
    pub CUT_BUFFER1: Atom,
    pub CUT_BUFFER2: Atom,
    pub CUT_BUFFER3: Atom,
    pub CUT_BUFFER4: Atom,
    pub CUT_BUFFER5: Atom,
    pub CUT_BUFFER6: Atom,
    pub CUT_BUFFER7: Atom,
    pub DRAWABLE: Atom,
    pub FONT: Atom,
    pub INTEGER: Atom,
    pub PIXMAP: Atom,
    pub POINT: Atom,
    pub RECTANGLE: Atom,
    pub RESOURCE_MANAGER: Atom,
    pub RGB_COLOR_MAP: Atom,
    pub RGB_BEST_MAP: Atom,
    pub RGB_BLUE_MAP: Atom,
    pub RGB_DEFAULT_MAP: Atom,
    pub RGB_GRAY_MAP: Atom,
    pub RGB_GREEN_MAP: Atom,
    pub RGB_RED_MAP: Atom,
    pub STRING: Atom,
    pub VISUALID: Atom,
    pub WINDOW: Atom,
    pub WM_COMMAND: Atom,
    pub WM_HINTS: Atom,
    pub WM_CLIENT_MACHINE: Atom,
    pub WM_ICON_NAME: Atom,
    pub WM_ICON_SIZE: Atom,
    pub WM_NAME: Atom,
    pub WM_NORMAL_HINTS: Atom,
    pub WM_SIZE_HINTS: Atom,
    pub WM_ZOOM_HINTS: Atom,
    pub MIN_SPACE: Atom,
    pub NORM_SPACE: Atom,
    pub MAX_SPACE: Atom,
    pub END_SPACE: Atom,
    pub SUPERSCRIPT_X: Atom,
    pub SUPERSCRIPT_Y: Atom,
    pub SUBSCRIPT_X: Atom,
    pub SUBSCRIPT_Y: Atom,
    pub UNDERLINE_POSITION: Atom,
    pub UNDERLINE_THICKNESS: Atom,
    pub STRIKEOUT_ASCENT: Atom,
    pub STRIKEOUT_DESCENT: Atom,
    pub ITALIC_ANGLE: Atom,
    pub X_HEIGHT: Atom,
    pub QUAD_WIDTH: Atom,
    pub WEIGHT: Atom,
    pub POINT_SIZE: Atom,
    pub RESOLUTION: Atom,
    pub COPYRIGHT: Atom,
    pub NOTICE: Atom,
    pub FONT_NAME: Atom,
    pub FAMILY_NAME: Atom,
    pub FULL_NAME: Atom,
    pub CAP_HEIGHT: Atom,
    pub WM_CLASS: Atom,
    pub WM_TRANSIENT_FOR: Atom,
    pub EDID: Atom,
    pub WM_PROTOCOLS: Atom,
    pub WM_DELETE_WINDOW: Atom,
    pub WM_TAKE_FOCUS: Atom,
    pub _NET_SUPPORTED: Atom,
    pub _NET_CLIENT_LIST: Atom,
    pub _NET_NUMBER_OF_DESKTOPS: Atom,
    pub _NET_DESKTOP_GEOMETRY: Atom,
    pub _NET_DESKTOP_VIEWPORT: Atom,
    pub _NET_CURRENT_DESKTOP: Atom,
    pub _NET_DESKTOP_NAMES: Atom,
    pub _NET_ACTIVE_WINDOW: Atom,
    pub _NET_WORKAREA: Atom,
    pub _NET_SUPPORTING_WM_CHECK: Atom,
    pub _NET_VIRTUAL_ROOTS: Atom,
    pub _NET_DESKTOP_LAYOUT: Atom,
    pub _NET_SHOWING_DESKTOP: Atom,
    pub _NET_CLOSE_WINDOW: Atom,
    pub _NET_MOVERESIZE_WINDOW: Atom,
    pub _NET_WM_MOVERESIZE: Atom,
    pub _NET_RESTACK_WINDOW: Atom,
    pub _NET_REQUEST_FRAME_EXTENTS: Atom,
    pub _NET_WM_NAME: Atom,
    pub _NET_WM_VISIBLE_NAME: Atom,
    pub _NET_WM_ICON_NAME: Atom,
    pub _NET_WM_VISIBLE_ICON_NAME: Atom,
    pub _NET_WM_DESKTOP: Atom,
    pub _NET_WM_WINDOW_TYPE: Atom,
    pub _NET_WM_STATE: Atom,
    pub _NET_WM_ALLOWED_ACTIONS: Atom,
    pub _NET_WM_ACTION_MAXIMIZE_VERT: Atom,
    pub _NET_WM_ACTION_MAXIMIZE_HORZ: Atom,
    pub _NET_WM_ACTION_FULLSCREEN: Atom,
    pub _NET_WM_ACTION_MINIMIZE: Atom,
    pub _NET_WM_STRUT: Atom,
    pub _NET_WM_STRUT_PARTIAL: Atom,
    pub _NET_WM_ICON_GEOMETRY: Atom,
    pub _NET_WM_ICON: Atom,
    pub _NET_WM_PID: Atom,
    pub _NET_WM_HANDLED_ICONS: Atom,
    pub _NET_WM_USER_TIME: Atom,
    pub _NET_FRAME_EXTENTS: Atom,
    pub _NET_WM_PING: Atom,
    pub _NET_WM_SYNC_REQUEST: Atom,
    pub _NET_WM_SYNC_REQUEST_COUNTER: Atom,
    pub _NET_SYSTEM_TRAY_S: Atom,
    pub _NET_SYSTEM_TRAY_ORIENTATION: Atom,
    pub _NET_SYSTEM_TRAY_OPCODE: Atom,
    pub _NET_WM_STATE_MAXIMIZED_HORZ: Atom,
    pub _NET_WM_STATE_MAXIMIZED_VERT: Atom,
    pub _NET_WM_STATE_FULLSCREEN: Atom,
    pub _XEMBED: Atom,
    pub _XEMBED_INFO: Atom,
    pub _MOTIF_WM_HINTS: Atom,
    pub _NET_WM_STATE_SKIP_TASKBAR: Atom,
    pub _NET_WM_STATE_ABOVE: Atom,
    pub _NET_WM_STATE_MODAL: Atom,
    pub _NET_WM_STATE_HIDDEN: Atom,
    pub _NET_WM_CONTEXT_HELP: Atom,
    pub _NET_WM_WINDOW_OPACITY: Atom,
    pub _NET_WM_WINDOW_TYPE_DESKTOP: Atom,
    pub _NET_WM_WINDOW_TYPE_DOCK: Atom,
    pub _NET_WM_WINDOW_TYPE_TOOLBAR: Atom,
    pub _NET_WM_WINDOW_TYPE_MENU: Atom,
    pub _NET_WM_WINDOW_TYPE_UTILITY: Atom,
    pub _NET_WM_WINDOW_TYPE_SPLASH: Atom,
    pub _NET_WM_WINDOW_TYPE_DIALOG: Atom,
    pub _NET_WM_WINDOW_TYPE_NORMAL: Atom,
    pub CLIPBOARD: Atom,
    pub CLIPBOARD_MANAGER: Atom,
    pub SAVE_TARGETS: Atom,
    pub MULTIPLE: Atom,
    pub TARGETS: Atom,
    pub UTF8_STRING: Atom,
    pub UTF16_STRING: Atom,
    pub ATOM_PAIR: Atom,
    pub MANAGER: Atom,
    pub _KDE_NET_WM_BLUR_BEHIND_REGION: Atom,
    pub INCR: Atom,
    pub _NET_WM_STATE_FOCUSED: Atom,
    pub FERRO_SAVE_TARGETS_PROPERTY_ATOM: Atom,
    pub XdndActionCopy: Atom,
    pub XdndActionLink: Atom,
    pub XdndActionMove: Atom,
    pub XdndAware: Atom,
    pub XdndDrop: Atom,
    pub XdndEnter: Atom,
    pub XdndFinished: Atom,
    pub XdndLeave: Atom,
    pub XdndPosition: Atom,
    pub XdndProxy: Atom,
    pub XdndSelection: Atom,
    pub XdndStatus: Atom,
    pub XdndTypeList: Atom,

    names_to_atoms: RefCell<HashMap<String, Atom>>,
    atoms_to_names: RefCell<HashMap<Atom, String>>,
    text_formats: OnceCell<[Atom; 5]>,
}

/// The atoms the protocol predefines, with their values.
const PREDEFINED_ATOMS: &[(&str, Atom)] = &[
    ("PRIMARY", 1),
    ("SECONDARY", 2),
    ("ARC", 3),
    ("ATOM", 4),
    ("BITMAP", 5),
    ("CARDINAL", 6),
    ("COLORMAP", 7),
    ("CURSOR", 8),
    ("CUT_BUFFER0", 9),
    ("CUT_BUFFER1", 10),
    ("CUT_BUFFER2", 11),
    ("CUT_BUFFER3", 12),
    ("CUT_BUFFER4", 13),
    ("CUT_BUFFER5", 14),
    ("CUT_BUFFER6", 15),
    ("CUT_BUFFER7", 16),
    ("DRAWABLE", 17),
    ("FONT", 18),
    ("INTEGER", 19),
    ("PIXMAP", 20),
    ("POINT", 21),
    ("RECTANGLE", 22),
    ("RESOURCE_MANAGER", 23),
    ("RGB_COLOR_MAP", 24),
    ("RGB_BEST_MAP", 25),
    ("RGB_BLUE_MAP", 26),
    ("RGB_DEFAULT_MAP", 27),
    ("RGB_GRAY_MAP", 28),
    ("RGB_GREEN_MAP", 29),
    ("RGB_RED_MAP", 30),
    ("STRING", 31),
    ("VISUALID", 32),
    ("WINDOW", 33),
    ("WM_COMMAND", 34),
    ("WM_HINTS", 35),
    ("WM_CLIENT_MACHINE", 36),
    ("WM_ICON_NAME", 37),
    ("WM_ICON_SIZE", 38),
    ("WM_NAME", 39),
    ("WM_NORMAL_HINTS", 40),
    ("WM_SIZE_HINTS", 41),
    ("WM_ZOOM_HINTS", 42),
    ("MIN_SPACE", 43),
    ("NORM_SPACE", 44),
    ("MAX_SPACE", 45),
    ("END_SPACE", 46),
    ("SUPERSCRIPT_X", 47),
    ("SUPERSCRIPT_Y", 48),
    ("SUBSCRIPT_X", 49),
    ("SUBSCRIPT_Y", 50),
    ("UNDERLINE_POSITION", 51),
    ("UNDERLINE_THICKNESS", 52),
    ("STRIKEOUT_ASCENT", 53),
    ("STRIKEOUT_DESCENT", 54),
    ("ITALIC_ANGLE", 55),
    ("X_HEIGHT", 56),
    ("QUAD_WIDTH", 57),
    ("WEIGHT", 58),
    ("POINT_SIZE", 59),
    ("RESOLUTION", 60),
    ("COPYRIGHT", 61),
    ("NOTICE", 62),
    ("FONT_NAME", 63),
    ("FAMILY_NAME", 64),
    ("FULL_NAME", 65),
    ("CAP_HEIGHT", 66),
    ("WM_CLASS", 67),
    ("WM_TRANSIENT_FOR", 68),
];

/// The names of the atoms that are interned, in the order of the fields.
const INTERNED_ATOM_NAMES: &[&str] = &[
    "EDID",
    "WM_PROTOCOLS",
    "WM_DELETE_WINDOW",
    "WM_TAKE_FOCUS",
    "_NET_SUPPORTED",
    "_NET_CLIENT_LIST",
    "_NET_NUMBER_OF_DESKTOPS",
    "_NET_DESKTOP_GEOMETRY",
    "_NET_DESKTOP_VIEWPORT",
    "_NET_CURRENT_DESKTOP",
    "_NET_DESKTOP_NAMES",
    "_NET_ACTIVE_WINDOW",
    "_NET_WORKAREA",
    "_NET_SUPPORTING_WM_CHECK",
    "_NET_VIRTUAL_ROOTS",
    "_NET_DESKTOP_LAYOUT",
    "_NET_SHOWING_DESKTOP",
    "_NET_CLOSE_WINDOW",
    "_NET_MOVERESIZE_WINDOW",
    "_NET_WM_MOVERESIZE",
    "_NET_RESTACK_WINDOW",
    "_NET_REQUEST_FRAME_EXTENTS",
    "_NET_WM_NAME",
    "_NET_WM_VISIBLE_NAME",
    "_NET_WM_ICON_NAME",
    "_NET_WM_VISIBLE_ICON_NAME",
    "_NET_WM_DESKTOP",
    "_NET_WM_WINDOW_TYPE",
    "_NET_WM_STATE",
    "_NET_WM_ALLOWED_ACTIONS",
    "_NET_WM_ACTION_MAXIMIZE_VERT",
    "_NET_WM_ACTION_MAXIMIZE_HORZ",
    "_NET_WM_ACTION_FULLSCREEN",
    "_NET_WM_ACTION_MINIMIZE",
    "_NET_WM_STRUT",
    "_NET_WM_STRUT_PARTIAL",
    "_NET_WM_ICON_GEOMETRY",
    "_NET_WM_ICON",
    "_NET_WM_PID",
    "_NET_WM_HANDLED_ICONS",
    "_NET_WM_USER_TIME",
    "_NET_FRAME_EXTENTS",
    "_NET_WM_PING",
    "_NET_WM_SYNC_REQUEST",
    "_NET_WM_SYNC_REQUEST_COUNTER",
    "_NET_SYSTEM_TRAY_S",
    "_NET_SYSTEM_TRAY_ORIENTATION",
    "_NET_SYSTEM_TRAY_OPCODE",
    "_NET_WM_STATE_MAXIMIZED_HORZ",
    "_NET_WM_STATE_MAXIMIZED_VERT",
    "_NET_WM_STATE_FULLSCREEN",
    "_XEMBED",
    "_XEMBED_INFO",
    "_MOTIF_WM_HINTS",
    "_NET_WM_STATE_SKIP_TASKBAR",
    "_NET_WM_STATE_ABOVE",
    "_NET_WM_STATE_MODAL",
    "_NET_WM_STATE_HIDDEN",
    "_NET_WM_CONTEXT_HELP",
    "_NET_WM_WINDOW_OPACITY",
    "_NET_WM_WINDOW_TYPE_DESKTOP",
    "_NET_WM_WINDOW_TYPE_DOCK",
    "_NET_WM_WINDOW_TYPE_TOOLBAR",
    "_NET_WM_WINDOW_TYPE_MENU",
    "_NET_WM_WINDOW_TYPE_UTILITY",
    "_NET_WM_WINDOW_TYPE_SPLASH",
    "_NET_WM_WINDOW_TYPE_DIALOG",
    "_NET_WM_WINDOW_TYPE_NORMAL",
    "CLIPBOARD",
    "CLIPBOARD_MANAGER",
    "SAVE_TARGETS",
    "MULTIPLE",
    "TARGETS",
    "UTF8_STRING",
    "UTF16_STRING",
    "ATOM_PAIR",
    "MANAGER",
    "_KDE_NET_WM_BLUR_BEHIND_REGION",
    "INCR",
    "_NET_WM_STATE_FOCUSED",
    "FERRO_SAVE_TARGETS_PROPERTY_ATOM",
    "XdndActionCopy",
    "XdndActionLink",
    "XdndActionMove",
    "XdndAware",
    "XdndDrop",
    "XdndEnter",
    "XdndFinished",
    "XdndLeave",
    "XdndPosition",
    "XdndProxy",
    "XdndSelection",
    "XdndStatus",
    "XdndTypeList",
];

impl X11Atoms {
    /// Interns the atoms on a connection.
    pub fn new(display: XDisplay) -> Self {
        let atoms = xlib::x_intern_atoms(display, INTERNED_ATOM_NAMES, false);
        Self::populate_atoms(Some(display), &atoms)
    }

    /// The atoms with the values `intern` gives for the names that are
    /// interned, without a connection: [`get_atom`](Self::get_atom) and
    /// [`get_atom_name`](Self::get_atom_name) then only know these atoms.
    /// For code that works on atoms without a server (the tests of the
    /// backend).
    pub fn with_interned(mut intern: impl FnMut(&str) -> Atom) -> Self {
        let atoms: Vec<Atom> = INTERNED_ATOM_NAMES.iter().map(|name| intern(name)).collect();
        Self::populate_atoms(None, &atoms)
    }

    /// The generated `PopulateAtoms` of the reference: names the
    /// predefined atoms and stores the interned ones (an atom the server
    /// answered with zero stays zero and unnamed).
    fn populate_atoms(display: Option<XDisplay>, atoms: &[Atom]) -> Self {
        assert_eq!(atoms.len(), INTERNED_ATOM_NAMES.len(), "one atom per name");
        let this = Self {
            display,
            PRIMARY: 1,
            SECONDARY: 2,
            ARC: 3,
            ATOM: 4,
            BITMAP: 5,
            CARDINAL: 6,
            COLORMAP: 7,
            CURSOR: 8,
            CUT_BUFFER0: 9,
            CUT_BUFFER1: 10,
            CUT_BUFFER2: 11,
            CUT_BUFFER3: 12,
            CUT_BUFFER4: 13,
            CUT_BUFFER5: 14,
            CUT_BUFFER6: 15,
            CUT_BUFFER7: 16,
            DRAWABLE: 17,
            FONT: 18,
            INTEGER: 19,
            PIXMAP: 20,
            POINT: 21,
            RECTANGLE: 22,
            RESOURCE_MANAGER: 23,
            RGB_COLOR_MAP: 24,
            RGB_BEST_MAP: 25,
            RGB_BLUE_MAP: 26,
            RGB_DEFAULT_MAP: 27,
            RGB_GRAY_MAP: 28,
            RGB_GREEN_MAP: 29,
            RGB_RED_MAP: 30,
            STRING: 31,
            VISUALID: 32,
            WINDOW: 33,
            WM_COMMAND: 34,
            WM_HINTS: 35,
            WM_CLIENT_MACHINE: 36,
            WM_ICON_NAME: 37,
            WM_ICON_SIZE: 38,
            WM_NAME: 39,
            WM_NORMAL_HINTS: 40,
            WM_SIZE_HINTS: 41,
            WM_ZOOM_HINTS: 42,
            MIN_SPACE: 43,
            NORM_SPACE: 44,
            MAX_SPACE: 45,
            END_SPACE: 46,
            SUPERSCRIPT_X: 47,
            SUPERSCRIPT_Y: 48,
            SUBSCRIPT_X: 49,
            SUBSCRIPT_Y: 50,
            UNDERLINE_POSITION: 51,
            UNDERLINE_THICKNESS: 52,
            STRIKEOUT_ASCENT: 53,
            STRIKEOUT_DESCENT: 54,
            ITALIC_ANGLE: 55,
            X_HEIGHT: 56,
            QUAD_WIDTH: 57,
            WEIGHT: 58,
            POINT_SIZE: 59,
            RESOLUTION: 60,
            COPYRIGHT: 61,
            NOTICE: 62,
            FONT_NAME: 63,
            FAMILY_NAME: 64,
            FULL_NAME: 65,
            CAP_HEIGHT: 66,
            WM_CLASS: 67,
            WM_TRANSIENT_FOR: 68,
            EDID: atoms[0],
            WM_PROTOCOLS: atoms[1],
            WM_DELETE_WINDOW: atoms[2],
            WM_TAKE_FOCUS: atoms[3],
            _NET_SUPPORTED: atoms[4],
            _NET_CLIENT_LIST: atoms[5],
            _NET_NUMBER_OF_DESKTOPS: atoms[6],
            _NET_DESKTOP_GEOMETRY: atoms[7],
            _NET_DESKTOP_VIEWPORT: atoms[8],
            _NET_CURRENT_DESKTOP: atoms[9],
            _NET_DESKTOP_NAMES: atoms[10],
            _NET_ACTIVE_WINDOW: atoms[11],
            _NET_WORKAREA: atoms[12],
            _NET_SUPPORTING_WM_CHECK: atoms[13],
            _NET_VIRTUAL_ROOTS: atoms[14],
            _NET_DESKTOP_LAYOUT: atoms[15],
            _NET_SHOWING_DESKTOP: atoms[16],
            _NET_CLOSE_WINDOW: atoms[17],
            _NET_MOVERESIZE_WINDOW: atoms[18],
            _NET_WM_MOVERESIZE: atoms[19],
            _NET_RESTACK_WINDOW: atoms[20],
            _NET_REQUEST_FRAME_EXTENTS: atoms[21],
            _NET_WM_NAME: atoms[22],
            _NET_WM_VISIBLE_NAME: atoms[23],
            _NET_WM_ICON_NAME: atoms[24],
            _NET_WM_VISIBLE_ICON_NAME: atoms[25],
            _NET_WM_DESKTOP: atoms[26],
            _NET_WM_WINDOW_TYPE: atoms[27],
            _NET_WM_STATE: atoms[28],
            _NET_WM_ALLOWED_ACTIONS: atoms[29],
            _NET_WM_ACTION_MAXIMIZE_VERT: atoms[30],
            _NET_WM_ACTION_MAXIMIZE_HORZ: atoms[31],
            _NET_WM_ACTION_FULLSCREEN: atoms[32],
            _NET_WM_ACTION_MINIMIZE: atoms[33],
            _NET_WM_STRUT: atoms[34],
            _NET_WM_STRUT_PARTIAL: atoms[35],
            _NET_WM_ICON_GEOMETRY: atoms[36],
            _NET_WM_ICON: atoms[37],
            _NET_WM_PID: atoms[38],
            _NET_WM_HANDLED_ICONS: atoms[39],
            _NET_WM_USER_TIME: atoms[40],
            _NET_FRAME_EXTENTS: atoms[41],
            _NET_WM_PING: atoms[42],
            _NET_WM_SYNC_REQUEST: atoms[43],
            _NET_WM_SYNC_REQUEST_COUNTER: atoms[44],
            _NET_SYSTEM_TRAY_S: atoms[45],
            _NET_SYSTEM_TRAY_ORIENTATION: atoms[46],
            _NET_SYSTEM_TRAY_OPCODE: atoms[47],
            _NET_WM_STATE_MAXIMIZED_HORZ: atoms[48],
            _NET_WM_STATE_MAXIMIZED_VERT: atoms[49],
            _NET_WM_STATE_FULLSCREEN: atoms[50],
            _XEMBED: atoms[51],
            _XEMBED_INFO: atoms[52],
            _MOTIF_WM_HINTS: atoms[53],
            _NET_WM_STATE_SKIP_TASKBAR: atoms[54],
            _NET_WM_STATE_ABOVE: atoms[55],
            _NET_WM_STATE_MODAL: atoms[56],
            _NET_WM_STATE_HIDDEN: atoms[57],
            _NET_WM_CONTEXT_HELP: atoms[58],
            _NET_WM_WINDOW_OPACITY: atoms[59],
            _NET_WM_WINDOW_TYPE_DESKTOP: atoms[60],
            _NET_WM_WINDOW_TYPE_DOCK: atoms[61],
            _NET_WM_WINDOW_TYPE_TOOLBAR: atoms[62],
            _NET_WM_WINDOW_TYPE_MENU: atoms[63],
            _NET_WM_WINDOW_TYPE_UTILITY: atoms[64],
            _NET_WM_WINDOW_TYPE_SPLASH: atoms[65],
            _NET_WM_WINDOW_TYPE_DIALOG: atoms[66],
            _NET_WM_WINDOW_TYPE_NORMAL: atoms[67],
            CLIPBOARD: atoms[68],
            CLIPBOARD_MANAGER: atoms[69],
            SAVE_TARGETS: atoms[70],
            MULTIPLE: atoms[71],
            TARGETS: atoms[72],
            UTF8_STRING: atoms[73],
            UTF16_STRING: atoms[74],
            ATOM_PAIR: atoms[75],
            MANAGER: atoms[76],
            _KDE_NET_WM_BLUR_BEHIND_REGION: atoms[77],
            INCR: atoms[78],
            _NET_WM_STATE_FOCUSED: atoms[79],
            FERRO_SAVE_TARGETS_PROPERTY_ATOM: atoms[80],
            XdndActionCopy: atoms[81],
            XdndActionLink: atoms[82],
            XdndActionMove: atoms[83],
            XdndAware: atoms[84],
            XdndDrop: atoms[85],
            XdndEnter: atoms[86],
            XdndFinished: atoms[87],
            XdndLeave: atoms[88],
            XdndPosition: atoms[89],
            XdndProxy: atoms[90],
            XdndSelection: atoms[91],
            XdndStatus: atoms[92],
            XdndTypeList: atoms[93],
            names_to_atoms: RefCell::new(HashMap::new()),
            atoms_to_names: RefCell::new(HashMap::new()),
            text_formats: OnceCell::new(),
        };
        for (name, value) in PREDEFINED_ATOMS {
            this.set_name(name, *value);
        }
        for (name, value) in INTERNED_ATOM_NAMES.iter().zip(atoms) {
            // `InitAtom`
            if *value != 0 {
                this.set_name(name, *value);
            }
        }
        this
    }

    /// The atoms corresponding to text formats, by order of preference.
    pub fn text_formats(&self) -> &[Atom] {
        self.text_formats.get_or_init(|| {
            [
                self.UTF16_STRING,
                self.UTF8_STRING,
                self.get_atom(MIME_TYPE_TEXT_PLAIN),
                self.get_atom(MIME_TYPE_TEXT_PLAIN_UTF8),
                self.STRING,
            ]
        })
    }

    fn set_name(&self, name: &str, value: Atom) {
        self.names_to_atoms.borrow_mut().insert(name.to_string(), value);
        self.atoms_to_names.borrow_mut().insert(value, name.to_string());
    }

    /// The atom of a name, interned when it is not known yet.
    pub fn get_atom(&self, name: &str) -> Atom {
        if let Some(rv) = self.names_to_atoms.borrow().get(name) {
            return *rv;
        }
        let Some(display) = self.display else {
            return 0;
        };
        let atom = xlib::x_intern_atom(display, name, false);
        self.set_name(name, atom);
        atom
    }

    /// The name of an atom, asked from the server when it is not known
    /// yet.
    pub fn get_atom_name(&self, atom: Atom) -> Option<String> {
        if atom == 0 {
            return None;
        }

        if let Some(rv) = self.atoms_to_names.borrow().get(&atom) {
            return Some(rv.clone());
        }
        let name = xlib::get_atom_name(self.display?, atom)?;
        self.atoms_to_names.borrow_mut().insert(atom, name.clone());
        self.names_to_atoms.borrow_mut().insert(name.clone(), atom);
        Some(name)
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests for this file.
    use super::*;

    fn atoms() -> X11Atoms {
        let mut next = 100;
        X11Atoms::with_interned(|_| {
            next += 1;
            next
        })
    }

    #[test]
    fn predefined_atoms_have_the_values_of_the_protocol() {
        let atoms = atoms();
        assert_eq!(atoms.PRIMARY, 1);
        assert_eq!(atoms.ATOM, 4);
        assert_eq!(atoms.CARDINAL, 6);
        assert_eq!(atoms.STRING, 31);
        assert_eq!(atoms.WINDOW, 33);
        assert_eq!(atoms.WM_NAME, 39);
        assert_eq!(atoms.WM_CLASS, 67);
        assert_eq!(atoms.WM_TRANSIENT_FOR, 68);
        assert_eq!(PREDEFINED_ATOMS.len(), 68);
        for (index, (_, value)) in PREDEFINED_ATOMS.iter().enumerate() {
            assert_eq!(*value, index as Atom + 1);
        }
    }

    #[test]
    fn interned_atoms_are_stored_in_the_order_of_their_names() {
        let atoms = atoms();
        assert_eq!(atoms.get_atom(INTERNED_ATOM_NAMES[0]), 101);
        assert_eq!(atoms.get_atom("WM_PROTOCOLS"), atoms.WM_PROTOCOLS);
        assert_eq!(atoms.get_atom("_NET_WM_STATE"), atoms._NET_WM_STATE);
        assert_eq!(atoms.get_atom_name(atoms._NET_WM_STATE_FULLSCREEN).as_deref(), Some("_NET_WM_STATE_FULLSCREEN"));
        assert_eq!(atoms.get_atom_name(atoms.WM_DELETE_WINDOW).as_deref(), Some("WM_DELETE_WINDOW"));
        assert_eq!(atoms.get_atom_name(31).as_deref(), Some("STRING"));
        assert_eq!(atoms.get_atom_name(0), None);
        // Without a connection an unknown atom has no name and an unknown name no atom.
        assert_eq!(atoms.get_atom_name(99_999), None);
        assert_eq!(atoms.get_atom("NOT_AN_ATOM_OF_THE_TABLE"), 0);
    }

    #[test]
    fn every_interned_name_is_distinct() {
        let mut names: Vec<&str> = INTERNED_ATOM_NAMES.to_vec();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), INTERNED_ATOM_NAMES.len());
    }

    #[test]
    fn an_atom_the_server_does_not_give_stays_unnamed() {
        let atoms = X11Atoms::with_interned(|name| if name == "CLIPBOARD" { 0 } else { 500 });
        assert_eq!(atoms.CLIPBOARD, 0);
        assert!(!atoms.names_to_atoms.borrow().contains_key("CLIPBOARD"));
    }
}

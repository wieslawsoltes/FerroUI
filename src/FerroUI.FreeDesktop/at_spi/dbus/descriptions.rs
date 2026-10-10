//! The interfaces the accessibility server answers for, as the
//! descriptions in the reference's `DBusXml/` have them at the tracked
//! commit (`Accessible.xml`, `Action.xml`, `Application.xml`,
//! `Cache.xml`, `Collection.xml`, `Component.xml`, `EditableText.xml`,
//! `Event.xml`, `Image.xml`, `Selection.xml`, `Text.xml`, `Value.xml`):
//! the members, their signatures, the properties and the signals.

use super::interface::InterfaceDescription;

/// The arguments of every event signal: `siiva{sv}`.
const EVENT: &str = "s i i v a{sv}";

pub(crate) static ACCESSIBLE: InterfaceDescription = InterfaceDescription {
    name: "org.a11y.atspi.Accessible",
    methods: &[
        ("GetChildAtIndex", "i", "(so)"),
        ("GetChildren", "", "a(so)"),
        ("GetIndexInParent", "", "i"),
        ("GetRelationSet", "", "a(ua(so))"),
        ("GetRole", "", "u"),
        ("GetRoleName", "", "s"),
        ("GetLocalizedRoleName", "", "s"),
        ("GetState", "", "au"),
        ("GetAttributes", "", "a{ss}"),
        ("GetApplication", "", "(so)"),
        ("GetInterfaces", "", "as"),
    ],
    properties: &[
        ("version", "u", false),
        ("Name", "s", false),
        ("Description", "s", false),
        ("Parent", "(so)", false),
        ("ChildCount", "i", false),
        ("Locale", "s", false),
        ("AccessibleId", "s", false),
        ("HelpText", "s", false),
    ],
    signals: &[],
};

pub(crate) static ACTION: InterfaceDescription = InterfaceDescription {
    name: "org.a11y.atspi.Action",
    methods: &[
        ("GetDescription", "i", "s"),
        ("GetName", "i", "s"),
        ("GetLocalizedName", "i", "s"),
        ("GetKeyBinding", "i", "s"),
        ("GetActions", "", "a(sss)"),
        ("DoAction", "i", "b"),
    ],
    properties: &[("version", "u", false), ("NActions", "i", false)],
    signals: &[],
};

pub(crate) static APPLICATION: InterfaceDescription = InterfaceDescription {
    name: "org.a11y.atspi.Application",
    methods: &[("GetLocale", "u", "s"), ("GetApplicationBusAddress", "", "s")],
    properties: &[
        ("ToolkitName", "s", false),
        ("Version", "s", false),
        ("ToolkitVersion", "s", false),
        ("AtspiVersion", "s", false),
        ("InterfaceVersion", "u", false),
        ("Id", "i", true),
    ],
    signals: &[],
};

pub(crate) static CACHE: InterfaceDescription = InterfaceDescription {
    name: "org.a11y.atspi.Cache",
    methods: &[("GetItems", "", "a((so)(so)(so)iiassusau)")],
    properties: &[("version", "u", false)],
    signals: &[("AddAccessible", "((so)(so)(so)iiassusau)"), ("RemoveAccessible", "(so)")],
};

pub(crate) static COLLECTION: InterfaceDescription = InterfaceDescription {
    name: "org.a11y.atspi.Collection",
    methods: &[
        ("GetMatches", "(aiia{ss}iaiiasib) u i b", "a(so)"),
        ("GetMatchesTo", "o (aiia{ss}iaiiasib) u u b i b", "a(so)"),
        ("GetMatchesFrom", "o (aiia{ss}iaiiasib) u u i b", "a(so)"),
        ("GetActiveDescendant", "", "(so)"),
    ],
    properties: &[("version", "u", false)],
    signals: &[],
};

pub(crate) static COMPONENT: InterfaceDescription = InterfaceDescription {
    name: "org.a11y.atspi.Component",
    methods: &[
        ("Contains", "i i u", "b"),
        ("GetAccessibleAtPoint", "i i u", "(so)"),
        ("GetExtents", "u", "(iiii)"),
        ("GetPosition", "u", "i i"),
        ("GetSize", "", "i i"),
        ("GetLayer", "", "u"),
        ("GetMDIZOrder", "", "n"),
        ("GrabFocus", "", "b"),
        ("GetAlpha", "", "d"),
        ("SetExtents", "i i i i u", "b"),
        ("SetPosition", "i i u", "b"),
        ("SetSize", "i i", "b"),
        ("ScrollTo", "u", "b"),
        ("ScrollToPoint", "u i i", "b"),
    ],
    properties: &[("version", "u", false)],
    signals: &[],
};

pub(crate) static EDITABLE_TEXT: InterfaceDescription = InterfaceDescription {
    name: "org.a11y.atspi.EditableText",
    methods: &[
        ("SetTextContents", "s", "b"),
        ("InsertText", "i s i", "b"),
        ("CopyText", "i i", ""),
        ("CutText", "i i", "b"),
        ("DeleteText", "i i", "b"),
        ("PasteText", "i", "b"),
    ],
    properties: &[("version", "u", false)],
    signals: &[],
};

pub(crate) static IMAGE: InterfaceDescription = InterfaceDescription {
    name: "org.a11y.atspi.Image",
    methods: &[("GetImageExtents", "u", "(iiii)"), ("GetImagePosition", "u", "i i"), ("GetImageSize", "", "i i")],
    properties: &[("version", "u", false), ("ImageDescription", "s", false), ("ImageLocale", "s", false)],
    signals: &[],
};

pub(crate) static SELECTION: InterfaceDescription = InterfaceDescription {
    name: "org.a11y.atspi.Selection",
    methods: &[
        ("GetSelectedChild", "i", "(so)"),
        ("SelectChild", "i", "b"),
        ("DeselectSelectedChild", "i", "b"),
        ("IsChildSelected", "i", "b"),
        ("SelectAll", "", "b"),
        ("ClearSelection", "", "b"),
        ("DeselectChild", "i", "b"),
    ],
    properties: &[("version", "u", false), ("NSelectedChildren", "i", false)],
    signals: &[],
};

pub(crate) static TEXT: InterfaceDescription = InterfaceDescription {
    name: "org.a11y.atspi.Text",
    methods: &[
        ("GetStringAtOffset", "i u", "s i i"),
        ("GetText", "i i", "s"),
        ("SetCaretOffset", "i", "b"),
        ("GetTextBeforeOffset", "i u", "s i i"),
        ("GetTextAtOffset", "i u", "s i i"),
        ("GetTextAfterOffset", "i u", "s i i"),
        ("GetCharacterAtOffset", "i", "i"),
        ("GetAttributeValue", "i s", "s"),
        ("GetAttributes", "i", "a{ss} i i"),
        ("GetDefaultAttributes", "", "a{ss}"),
        ("GetCharacterExtents", "i u", "i i i i"),
        ("GetOffsetAtPoint", "i i u", "i"),
        ("GetNSelections", "", "i"),
        ("GetSelection", "i", "i i"),
        ("AddSelection", "i i", "b"),
        ("RemoveSelection", "i", "b"),
        ("SetSelection", "i i i", "b"),
        ("GetRangeExtents", "i i u", "i i i i"),
        ("GetBoundedRanges", "i i i i u u u", "a(iisv)"),
        ("GetAttributeRun", "i b", "a{ss} i i"),
        ("GetDefaultAttributeSet", "", "a{ss}"),
        ("ScrollSubstringTo", "i i u", "b"),
        ("ScrollSubstringToPoint", "i i u i i", "b"),
    ],
    properties: &[("version", "u", false), ("CharacterCount", "i", false), ("CaretOffset", "i", false)],
    signals: &[],
};

pub(crate) static VALUE: InterfaceDescription = InterfaceDescription {
    name: "org.a11y.atspi.Value",
    methods: &[],
    properties: &[
        ("version", "u", false),
        ("MinimumValue", "d", false),
        ("MaximumValue", "d", false),
        ("MinimumIncrement", "d", false),
        ("CurrentValue", "d", true),
        ("Text", "s", false),
    ],
    signals: &[],
};

pub(crate) static EVENT_OBJECT: InterfaceDescription = InterfaceDescription {
    name: "org.a11y.atspi.Event.Object",
    methods: &[],
    properties: &[("version", "u", false)],
    signals: &[
        ("PropertyChange", EVENT),
        ("BoundsChanged", EVENT),
        ("LinkSelected", EVENT),
        ("StateChanged", EVENT),
        ("ChildrenChanged", EVENT),
        ("VisibleDataChanged", EVENT),
        ("SelectionChanged", EVENT),
        ("ModelChanged", EVENT),
        ("ActiveDescendantChanged", EVENT),
        ("Announcement", EVENT),
        ("AttributesChanged", EVENT),
        ("RowInserted", EVENT),
        ("RowReordered", EVENT),
        ("RowDeleted", EVENT),
        ("ColumnInserted", EVENT),
        ("ColumnReordered", EVENT),
        ("ColumnDeleted", EVENT),
        ("TextBoundsChanged", EVENT),
        ("TextSelectionChanged", EVENT),
        ("TextChanged", EVENT),
        ("TextAttributesChanged", EVENT),
        ("TextCaretMoved", EVENT),
    ],
};

pub(crate) static EVENT_WINDOW: InterfaceDescription = InterfaceDescription {
    name: "org.a11y.atspi.Event.Window",
    methods: &[],
    properties: &[],
    signals: &[
        ("PropertyChange", EVENT),
        ("Minimize", EVENT),
        ("Maximize", EVENT),
        ("Restore", EVENT),
        ("Close", EVENT),
        ("Create", EVENT),
        ("Reparent", EVENT),
        ("DesktopCreate", EVENT),
        ("DesktopDestroy", EVENT),
        ("Destroy", EVENT),
        ("Activate", EVENT),
        ("Deactivate", EVENT),
        ("Raise", EVENT),
        ("Lower", EVENT),
        ("Move", EVENT),
        ("Resize", EVENT),
        ("Shade", EVENT),
        ("uUshade", EVENT),
        ("Restyle", EVENT),
    ],
};

// Derived from Keys/KeyIcon.cs; see ../NOTICE.md.
#[derive(Clone, Copy, Default, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct KeyIcon(pub i32);
#[allow(non_upper_case_globals)]
impl KeyIcon {
    pub const None: Self = Self(0);
    pub const Backspace: Self = Self(1);
    pub const Shift: Self = Self(2);
    pub const ShiftActive: Self = Self(3);
    pub const CapsLock: Self = Self(4);
    pub const Enter: Self = Self(5);
    pub const Search: Self = Self(6);
    pub const Send: Self = Self(7);
    pub const Go: Self = Self(8);
    pub const Next: Self = Self(9);
    pub const Previous: Self = Self(10);
    pub const Done: Self = Self(11);
    pub const Space: Self = Self(12);
    pub const Language: Self = Self(13);
    pub const Emoji: Self = Self(14);
    pub const Clipboard: Self = Self(15);
    pub const Settings: Self = Self(16);
    pub const HideKeyboard: Self = Self(17);
    pub const ArrowLeft: Self = Self(18);
    pub const ArrowRight: Self = Self(19);
    pub const ArrowUp: Self = Self(20);
    pub const ArrowDown: Self = Self(21);
    pub const Undo: Self = Self(22);
    pub const Redo: Self = Self(23);
    pub const Copy: Self = Self(24);
    pub const Cut: Self = Self(25);
    pub const Paste: Self = Self(26);
    pub const SelectAll: Self = Self(27);
    pub const Select: Self = Self(28);
    pub const Microphone: Self = Self(29);
    pub const OneHandedLeft: Self = Self(30);
    pub const OneHandedRight: Self = Self(31);
    pub const Keyboard: Self = Self(32);
    pub const Tab: Self = Self(33);
    pub const Incognito: Self = Self(34);
    pub const TextEditing: Self = Self(35);
    pub const Home: Self = Self(36);
    pub const End: Self = Self(37);
    pub const DeleteForward: Self = Self(38);
    pub const SplitKeyboard: Self = Self(39);
    pub const Close: Self = Self(40);
    pub const Pin: Self = Self(41);
    pub const Unpin: Self = Self(42);
    pub const Delete: Self = Self(43);
    pub const Recent: Self = Self(44);
    pub const Smiley: Self = Self(45);
    pub const Person: Self = Self(46);
    pub const Nature: Self = Self(47);
    pub const Food: Self = Self(48);
    pub const Travel: Self = Self(49);
    pub const Activity: Self = Self(50);
    pub const Objects: Self = Self(51);
    pub const Symbols: Self = Self(52);
    pub const Flag: Self = Self(53);
    pub const Emoticon: Self = Self(54);
    pub const ChevronLeft: Self = Self(55);
    pub const ChevronRight: Self = Self(56);
    pub const ChevronUp: Self = Self(57);
    pub const ChevronDown: Self = Self(58);
    pub const MoreHorizontal: Self = Self(59);
    pub const Resize: Self = Self(60);
    pub const Autocorrect: Self = Self(61);
    pub const Numpad: Self = Self(62);
    pub const DocumentStart: Self = Self(63);
    pub const DocumentEnd: Self = Self(64);
    pub const ResizeHeight: Self = Self(65);
    pub const Floating: Self = Self(66);
    pub const DragHandle: Self = Self(67);
}
impl std::fmt::Display for KeyIcon {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::None => f.write_str("None"),
            Self::Backspace => f.write_str("Backspace"),
            Self::Shift => f.write_str("Shift"),
            Self::ShiftActive => f.write_str("ShiftActive"),
            Self::CapsLock => f.write_str("CapsLock"),
            Self::Enter => f.write_str("Enter"),
            Self::Search => f.write_str("Search"),
            Self::Send => f.write_str("Send"),
            Self::Go => f.write_str("Go"),
            Self::Next => f.write_str("Next"),
            Self::Previous => f.write_str("Previous"),
            Self::Done => f.write_str("Done"),
            Self::Space => f.write_str("Space"),
            Self::Language => f.write_str("Language"),
            Self::Emoji => f.write_str("Emoji"),
            Self::Clipboard => f.write_str("Clipboard"),
            Self::Settings => f.write_str("Settings"),
            Self::HideKeyboard => f.write_str("HideKeyboard"),
            Self::ArrowLeft => f.write_str("ArrowLeft"),
            Self::ArrowRight => f.write_str("ArrowRight"),
            Self::ArrowUp => f.write_str("ArrowUp"),
            Self::ArrowDown => f.write_str("ArrowDown"),
            Self::Undo => f.write_str("Undo"),
            Self::Redo => f.write_str("Redo"),
            Self::Copy => f.write_str("Copy"),
            Self::Cut => f.write_str("Cut"),
            Self::Paste => f.write_str("Paste"),
            Self::SelectAll => f.write_str("SelectAll"),
            Self::Select => f.write_str("Select"),
            Self::Microphone => f.write_str("Microphone"),
            Self::OneHandedLeft => f.write_str("OneHandedLeft"),
            Self::OneHandedRight => f.write_str("OneHandedRight"),
            Self::Keyboard => f.write_str("Keyboard"),
            Self::Tab => f.write_str("Tab"),
            Self::Incognito => f.write_str("Incognito"),
            Self::TextEditing => f.write_str("TextEditing"),
            Self::Home => f.write_str("Home"),
            Self::End => f.write_str("End"),
            Self::DeleteForward => f.write_str("DeleteForward"),
            Self::SplitKeyboard => f.write_str("SplitKeyboard"),
            Self::Close => f.write_str("Close"),
            Self::Pin => f.write_str("Pin"),
            Self::Unpin => f.write_str("Unpin"),
            Self::Delete => f.write_str("Delete"),
            Self::Recent => f.write_str("Recent"),
            Self::Smiley => f.write_str("Smiley"),
            Self::Person => f.write_str("Person"),
            Self::Nature => f.write_str("Nature"),
            Self::Food => f.write_str("Food"),
            Self::Travel => f.write_str("Travel"),
            Self::Activity => f.write_str("Activity"),
            Self::Objects => f.write_str("Objects"),
            Self::Symbols => f.write_str("Symbols"),
            Self::Flag => f.write_str("Flag"),
            Self::Emoticon => f.write_str("Emoticon"),
            Self::ChevronLeft => f.write_str("ChevronLeft"),
            Self::ChevronRight => f.write_str("ChevronRight"),
            Self::ChevronUp => f.write_str("ChevronUp"),
            Self::ChevronDown => f.write_str("ChevronDown"),
            Self::MoreHorizontal => f.write_str("MoreHorizontal"),
            Self::Resize => f.write_str("Resize"),
            Self::Autocorrect => f.write_str("Autocorrect"),
            Self::Numpad => f.write_str("Numpad"),
            Self::DocumentStart => f.write_str("DocumentStart"),
            Self::DocumentEnd => f.write_str("DocumentEnd"),
            Self::ResizeHeight => f.write_str("ResizeHeight"),
            Self::Floating => f.write_str("Floating"),
            Self::DragHandle => f.write_str("DragHandle"),
            _ => std::fmt::Display::fmt(&self.0, f),
        }
    }
}
impl std::fmt::Debug for KeyIcon {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}

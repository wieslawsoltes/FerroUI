//! WinUI's `Symbol` and `SymbolIcon` (`dxaml/xcp/core/core/elements/icon.cpp` and the type model
//! `Microsoft.UI.Xaml.Controls.cs` of microsoft-ui-xaml, commit
//! `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c) Microsoft Corporation, see
//! NOTICE.md). The members, values and glyphs are checked against the source by
//! `scripts/winui-controls/symbol_map.py`.

use crate::primitives::{TemplateAppliedEventArgs, TemplatedControl, TemplatedControlImpl, TemplatedControlImplExt};
use crate::{ControlImpl, IconElement, TextBlock};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{Layoutable, LayoutableImpl};
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, Ref, StyledElementImpl, StyledProperty, VisualImpl,
};
use std::cell::RefCell;

/// The symbols of [`SymbolIcon`], with the values of WinUI's `Symbol`: the code points of the
/// Segoe UI Symbol font. The glyph an icon draws is [`Symbol::glyph`], the code point of the same
/// picture in Segoe Fluent Icons.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum Symbol {
    Previous = 0xE100,
    Next = 0xE101,
    Play = 0xE102,
    Pause = 0xE103,
    Edit = 0xE104,
    Save = 0xE105,
    Clear = 0xE106,
    Delete = 0xE107,
    Remove = 0xE108,
    Add = 0xE109,
    Cancel = 0xE10A,
    Accept = 0xE10B,
    More = 0xE10C,
    Redo = 0xE10D,
    Undo = 0xE10E,
    Home = 0xE10F,
    Up = 0xE110,
    Forward = 0xE111,
    Back = 0xE112,
    Favorite = 0xE113,
    Camera = 0xE114,
    Setting = 0xE115,
    Video = 0xE116,
    Sync = 0xE117,
    Download = 0xE118,
    Mail = 0xE119,
    Find = 0xE11A,
    Help = 0xE11B,
    Upload = 0xE11C,
    #[default]
    Emoji = 0xE11D,
    TwoPage = 0xE11E,
    LeaveChat = 0xE11F,
    MailForward = 0xE120,
    Clock = 0xE121,
    Send = 0xE122,
    Crop = 0xE123,
    RotateCamera = 0xE124,
    People = 0xE125,
    OpenPane = 0xE126,
    ClosePane = 0xE127,
    World = 0xE128,
    Flag = 0xE129,
    PreviewLink = 0xE12A,
    Globe = 0xE12B,
    Trim = 0xE12C,
    AttachCamera = 0xE12D,
    ZoomIn = 0xE12E,
    Bookmarks = 0xE12F,
    Document = 0xE130,
    ProtectedDocument = 0xE131,
    Page = 0xE132,
    Bullets = 0xE133,
    Comment = 0xE134,
    MailFilled = 0xE135,
    ContactInfo = 0xE136,
    HangUp = 0xE137,
    ViewAll = 0xE138,
    MapPin = 0xE139,
    Phone = 0xE13A,
    VideoChat = 0xE13B,
    Switch = 0xE13C,
    Contact = 0xE13D,
    Rename = 0xE13E,
    Pin = 0xE141,
    MusicInfo = 0xE142,
    Go = 0xE143,
    Keyboard = 0xE144,
    DockLeft = 0xE145,
    DockRight = 0xE146,
    DockBottom = 0xE147,
    Remote = 0xE148,
    Refresh = 0xE149,
    Rotate = 0xE14A,
    Shuffle = 0xE14B,
    List = 0xE14C,
    Shop = 0xE14D,
    SelectAll = 0xE14E,
    Orientation = 0xE14F,
    Import = 0xE150,
    ImportAll = 0xE151,
    BrowsePhotos = 0xE155,
    WebCam = 0xE156,
    Pictures = 0xE158,
    SaveLocal = 0xE159,
    Caption = 0xE15A,
    Stop = 0xE15B,
    ShowResults = 0xE15C,
    Volume = 0xE15D,
    Repair = 0xE15E,
    Message = 0xE15F,
    Page2 = 0xE160,
    CalendarDay = 0xE161,
    CalendarWeek = 0xE162,
    Calendar = 0xE163,
    Character = 0xE164,
    MailReplyAll = 0xE165,
    Read = 0xE166,
    Link = 0xE167,
    Account = 0xE168,
    ShowBcc = 0xE169,
    HideBcc = 0xE16A,
    Cut = 0xE16B,
    Attach = 0xE16C,
    Paste = 0xE16D,
    Filter = 0xE16E,
    Copy = 0xE16F,
    Emoji2 = 0xE170,
    Important = 0xE171,
    MailReply = 0xE172,
    SlideShow = 0xE173,
    Sort = 0xE174,
    Manage = 0xE178,
    AllApps = 0xE179,
    DisconnectDrive = 0xE17A,
    MapDrive = 0xE17B,
    NewWindow = 0xE17C,
    OpenWith = 0xE17D,
    ContactPresence = 0xE181,
    Priority = 0xE182,
    GoToToday = 0xE184,
    Font = 0xE185,
    FontColor = 0xE186,
    Contact2 = 0xE187,
    Folder = 0xE188,
    Audio = 0xE189,
    Placeholder = 0xE18A,
    View = 0xE18B,
    SetLockScreen = 0xE18C,
    SetTile = 0xE18D,
    ClosedCaption = 0xE190,
    StopSlideShow = 0xE191,
    Permissions = 0xE192,
    Highlight = 0xE193,
    DisableUpdates = 0xE194,
    UnFavorite = 0xE195,
    UnPin = 0xE196,
    OpenLocal = 0xE197,
    Mute = 0xE198,
    Italic = 0xE199,
    Underline = 0xE19A,
    Bold = 0xE19B,
    MoveToFolder = 0xE19C,
    LikeDislike = 0xE19D,
    Dislike = 0xE19E,
    Like = 0xE19F,
    AlignRight = 0xE1A0,
    AlignCenter = 0xE1A1,
    AlignLeft = 0xE1A2,
    Zoom = 0xE1A3,
    ZoomOut = 0xE1A4,
    OpenFile = 0xE1A5,
    OtherUser = 0xE1A6,
    Admin = 0xE1A7,
    Street = 0xE1C3,
    Map = 0xE1C4,
    ClearSelection = 0xE1C5,
    FontDecrease = 0xE1C6,
    FontIncrease = 0xE1C7,
    FontSize = 0xE1C8,
    CellPhone = 0xE1C9,
    ReShare = 0xE1CA,
    Tag = 0xE1CB,
    RepeatOne = 0xE1CC,
    RepeatAll = 0xE1CD,
    OutlineStar = 0xE1CE,
    SolidStar = 0xE1CF,
    Calculator = 0xE1D0,
    Directions = 0xE1D1,
    Target = 0xE1D2,
    Library = 0xE1D3,
    PhoneBook = 0xE1D4,
    Memo = 0xE1D5,
    Microphone = 0xE1D6,
    PostUpdate = 0xE1D7,
    BackToWindow = 0xE1D8,
    FullScreen = 0xE1D9,
    NewFolder = 0xE1DA,
    CalendarReply = 0xE1DB,
    UnSyncFolder = 0xE1DD,
    ReportHacked = 0xE1DE,
    SyncFolder = 0xE1DF,
    BlockContact = 0xE1E0,
    SwitchApps = 0xE1E1,
    AddFriend = 0xE1E2,
    TouchPointer = 0xE1E3,
    GoToStart = 0xE1E4,
    ZeroBars = 0xE1E5,
    OneBar = 0xE1E6,
    TwoBars = 0xE1E7,
    ThreeBars = 0xE1E8,
    FourBars = 0xE1E9,
    Scan = 0xE294,
    Preview = 0xE295,
    GlobalNavigationButton = 0xE700,
    Share = 0xE72D,
    Print = 0xE749,
    XboxOneConsole = 0xE990,
}

ferroui_base::ferro_markup_enum!(Symbol {
    Previous, Next, Play, Pause, Edit, Save, Clear, Delete,
    Remove, Add, Cancel, Accept, More, Redo, Undo, Home,
    Up, Forward, Back, Favorite, Camera, Setting, Video, Sync,
    Download, Mail, Find, Help, Upload, Emoji, TwoPage, LeaveChat,
    MailForward, Clock, Send, Crop, RotateCamera, People, OpenPane, ClosePane,
    World, Flag, PreviewLink, Globe, Trim, AttachCamera, ZoomIn, Bookmarks,
    Document, ProtectedDocument, Page, Bullets, Comment, MailFilled, ContactInfo, HangUp,
    ViewAll, MapPin, Phone, VideoChat, Switch, Contact, Rename, Pin,
    MusicInfo, Go, Keyboard, DockLeft, DockRight, DockBottom, Remote, Refresh,
    Rotate, Shuffle, List, Shop, SelectAll, Orientation, Import, ImportAll,
    BrowsePhotos, WebCam, Pictures, SaveLocal, Caption, Stop, ShowResults, Volume,
    Repair, Message, Page2, CalendarDay, CalendarWeek, Calendar, Character, MailReplyAll,
    Read, Link, Account, ShowBcc, HideBcc, Cut, Attach, Paste,
    Filter, Copy, Emoji2, Important, MailReply, SlideShow, Sort, Manage,
    AllApps, DisconnectDrive, MapDrive, NewWindow, OpenWith, ContactPresence, Priority, GoToToday,
    Font, FontColor, Contact2, Folder, Audio, Placeholder, View, SetLockScreen,
    SetTile, ClosedCaption, StopSlideShow, Permissions, Highlight, DisableUpdates, UnFavorite, UnPin,
    OpenLocal, Mute, Italic, Underline, Bold, MoveToFolder, LikeDislike, Dislike,
    Like, AlignRight, AlignCenter, AlignLeft, Zoom, ZoomOut, OpenFile, OtherUser,
    Admin, Street, Map, ClearSelection, FontDecrease, FontIncrease, FontSize, CellPhone,
    ReShare, Tag, RepeatOne, RepeatAll, OutlineStar, SolidStar, Calculator, Directions,
    Target, Library, PhoneBook, Memo, Microphone, PostUpdate, BackToWindow, FullScreen,
    NewFolder, CalendarReply, UnSyncFolder, ReportHacked, SyncFolder, BlockContact, SwitchApps, AddFriend,
    TouchPointer, GoToStart, ZeroBars, OneBar, TwoBars, ThreeBars, FourBars, Scan,
    Preview, GlobalNavigationButton, Share, Print, XboxOneConsole,
}, { namespace: "FerroUI.Controls" });

impl Symbol {
    /// Every member, in the order WinUI declares them.
    pub const ALL: &'static [Self] = &[
        Self::Previous, Self::Next, Self::Play, Self::Pause, Self::Edit, Self::Save,
        Self::Clear, Self::Delete, Self::Remove, Self::Add, Self::Cancel, Self::Accept,
        Self::More, Self::Redo, Self::Undo, Self::Home, Self::Up, Self::Forward,
        Self::Back, Self::Favorite, Self::Camera, Self::Setting, Self::Video, Self::Sync,
        Self::Download, Self::Mail, Self::Find, Self::Help, Self::Upload, Self::Emoji,
        Self::TwoPage, Self::LeaveChat, Self::MailForward, Self::Clock, Self::Send, Self::Crop,
        Self::RotateCamera, Self::People, Self::OpenPane, Self::ClosePane, Self::World, Self::Flag,
        Self::PreviewLink, Self::Globe, Self::Trim, Self::AttachCamera, Self::ZoomIn, Self::Bookmarks,
        Self::Document, Self::ProtectedDocument, Self::Page, Self::Bullets, Self::Comment, Self::MailFilled,
        Self::ContactInfo, Self::HangUp, Self::ViewAll, Self::MapPin, Self::Phone, Self::VideoChat,
        Self::Switch, Self::Contact, Self::Rename, Self::Pin, Self::MusicInfo, Self::Go,
        Self::Keyboard, Self::DockLeft, Self::DockRight, Self::DockBottom, Self::Remote, Self::Refresh,
        Self::Rotate, Self::Shuffle, Self::List, Self::Shop, Self::SelectAll, Self::Orientation,
        Self::Import, Self::ImportAll, Self::BrowsePhotos, Self::WebCam, Self::Pictures, Self::SaveLocal,
        Self::Caption, Self::Stop, Self::ShowResults, Self::Volume, Self::Repair, Self::Message,
        Self::Page2, Self::CalendarDay, Self::CalendarWeek, Self::Calendar, Self::Character, Self::MailReplyAll,
        Self::Read, Self::Link, Self::Account, Self::ShowBcc, Self::HideBcc, Self::Cut,
        Self::Attach, Self::Paste, Self::Filter, Self::Copy, Self::Emoji2, Self::Important,
        Self::MailReply, Self::SlideShow, Self::Sort, Self::Manage, Self::AllApps, Self::DisconnectDrive,
        Self::MapDrive, Self::NewWindow, Self::OpenWith, Self::ContactPresence, Self::Priority, Self::GoToToday,
        Self::Font, Self::FontColor, Self::Contact2, Self::Folder, Self::Audio, Self::Placeholder,
        Self::View, Self::SetLockScreen, Self::SetTile, Self::ClosedCaption, Self::StopSlideShow, Self::Permissions,
        Self::Highlight, Self::DisableUpdates, Self::UnFavorite, Self::UnPin, Self::OpenLocal, Self::Mute,
        Self::Italic, Self::Underline, Self::Bold, Self::MoveToFolder, Self::LikeDislike, Self::Dislike,
        Self::Like, Self::AlignRight, Self::AlignCenter, Self::AlignLeft, Self::Zoom, Self::ZoomOut,
        Self::OpenFile, Self::OtherUser, Self::Admin, Self::Street, Self::Map, Self::ClearSelection,
        Self::FontDecrease, Self::FontIncrease, Self::FontSize, Self::CellPhone, Self::ReShare, Self::Tag,
        Self::RepeatOne, Self::RepeatAll, Self::OutlineStar, Self::SolidStar, Self::Calculator, Self::Directions,
        Self::Target, Self::Library, Self::PhoneBook, Self::Memo, Self::Microphone, Self::PostUpdate,
        Self::BackToWindow, Self::FullScreen, Self::NewFolder, Self::CalendarReply, Self::UnSyncFolder, Self::ReportHacked,
        Self::SyncFolder, Self::BlockContact, Self::SwitchApps, Self::AddFriend, Self::TouchPointer, Self::GoToStart,
        Self::ZeroBars, Self::OneBar, Self::TwoBars, Self::ThreeBars, Self::FourBars, Self::Scan,
        Self::Preview, Self::GlobalNavigationButton, Self::Share, Self::Print, Self::XboxOneConsole,
    ];

    /// The glyph `SymbolIcon` draws for the symbol: `CSymbolIcon::ConvertSymbolValueToGlyph`, which
    /// maps most values to the code point of the same picture in Segoe Fluent Icons and keeps the
    /// others.
    pub const fn glyph(self) -> char {
        match self {
            Self::Previous => '\u{e892}',
            Self::Next => '\u{e893}',
            Self::Play => '\u{e768}',
            Self::Pause => '\u{e769}',
            Self::Edit => '\u{e70f}',
            Self::Save => '\u{e74e}',
            Self::Clear => '\u{e894}',
            Self::Delete => '\u{e74d}',
            Self::Remove => '\u{e738}',
            Self::Add => '\u{e710}',
            Self::Cancel => '\u{e711}',
            Self::Accept => '\u{e8fb}',
            Self::More => '\u{e712}',
            Self::Redo => '\u{e7a6}',
            Self::Undo => '\u{e7a7}',
            Self::Home => '\u{e80f}',
            Self::Up => '\u{e74a}',
            Self::Forward => '\u{e72a}',
            Self::Back => '\u{e72b}',
            Self::Favorite => '\u{e734}',
            Self::Camera => '\u{e722}',
            Self::Setting => '\u{e713}',
            Self::Video => '\u{e714}',
            Self::Sync => '\u{e895}',
            Self::Download => '\u{e896}',
            Self::Mail => '\u{e715}',
            Self::Find => '\u{e721}',
            Self::Help => '\u{e897}',
            Self::Upload => '\u{e898}',
            Self::Emoji => '\u{e899}',
            Self::TwoPage => '\u{e89a}',
            Self::LeaveChat => '\u{e89b}',
            Self::MailForward => '\u{e89c}',
            Self::Clock => '\u{e823}',
            Self::Send => '\u{e724}',
            Self::Crop => '\u{e7a8}',
            Self::RotateCamera => '\u{e89e}',
            Self::People => '\u{e716}',
            Self::OpenPane => '\u{e8a0}',
            Self::ClosePane => '\u{e89f}',
            Self::World => '\u{e909}',
            Self::Flag => '\u{e7c1}',
            Self::PreviewLink => '\u{e8a1}',
            Self::Globe => '\u{e774}',
            Self::Trim => '\u{e78a}',
            Self::AttachCamera => '\u{e8a2}',
            Self::ZoomIn => '\u{e8a3}',
            Self::Bookmarks => '\u{e8a4}',
            Self::Document => '\u{e8a5}',
            Self::ProtectedDocument => '\u{e8a6}',
            Self::Page => '\u{e729}',
            Self::Bullets => '\u{e8fd}',
            Self::Comment => '\u{e90a}',
            Self::MailFilled => '\u{e8a8}',
            Self::ContactInfo => '\u{e779}',
            Self::HangUp => '\u{e778}',
            Self::ViewAll => '\u{e8a9}',
            Self::MapPin => '\u{e7b7}',
            Self::Phone => '\u{e717}',
            Self::VideoChat => '\u{e8aa}',
            Self::Switch => '\u{e8ab}',
            Self::Contact => '\u{e77b}',
            Self::Rename => '\u{e8ac}',
            Self::Pin => '\u{e718}',
            Self::MusicInfo => '\u{e90b}',
            Self::Go => '\u{e8ad}',
            Self::Keyboard => '\u{e765}',
            Self::DockLeft => '\u{e90c}',
            Self::DockRight => '\u{e90d}',
            Self::DockBottom => '\u{e90e}',
            Self::Remote => '\u{e8af}',
            Self::Refresh => '\u{e72c}',
            Self::Rotate => '\u{e7ad}',
            Self::Shuffle => '\u{e8b1}',
            Self::List => '\u{ea37}',
            Self::Shop => '\u{e719}',
            Self::SelectAll => '\u{e8b3}',
            Self::Orientation => '\u{e8b4}',
            Self::Import => '\u{e8b5}',
            Self::ImportAll => '\u{e8b6}',
            Self::BrowsePhotos => '\u{e7c5}',
            Self::WebCam => '\u{e8b8}',
            Self::Pictures => '\u{e8b9}',
            Self::SaveLocal => '\u{e78c}',
            Self::Caption => '\u{e8ba}',
            Self::Stop => '\u{e71a}',
            Self::ShowResults => '\u{e8bc}',
            Self::Volume => '\u{e767}',
            Self::Repair => '\u{e90f}',
            Self::Message => '\u{e8bd}',
            Self::Page2 => '\u{e7c3}',
            Self::CalendarDay => '\u{e8bf}',
            Self::CalendarWeek => '\u{e8c0}',
            Self::Calendar => '\u{e787}',
            Self::Character => '\u{e8c1}',
            Self::MailReplyAll => '\u{e8c2}',
            Self::Read => '\u{e8c3}',
            Self::Link => '\u{e71b}',
            Self::Account => '\u{e910}',
            Self::ShowBcc => '\u{e8c4}',
            Self::HideBcc => '\u{e8c5}',
            Self::Cut => '\u{e8c6}',
            Self::Attach => '\u{e723}',
            Self::Paste => '\u{e77f}',
            Self::Filter => '\u{e71c}',
            Self::Copy => '\u{e8c8}',
            Self::Emoji2 => '\u{e76e}',
            Self::Important => '\u{e8c9}',
            Self::MailReply => '\u{e8ca}',
            Self::SlideShow => '\u{e786}',
            Self::Sort => '\u{e8cb}',
            Self::Manage => '\u{e912}',
            Self::AllApps => '\u{e71d}',
            Self::DisconnectDrive => '\u{e8cd}',
            Self::MapDrive => '\u{e8ce}',
            Self::NewWindow => '\u{e78b}',
            Self::OpenWith => '\u{e7ac}',
            Self::ContactPresence => '\u{e8cf}',
            Self::Priority => '\u{e8d0}',
            Self::GoToToday => '\u{e8d1}',
            Self::Font => '\u{e8d2}',
            Self::FontColor => '\u{e8d3}',
            Self::Contact2 => '\u{e8d4}',
            Self::Folder => '\u{e8b7}',
            Self::Audio => '\u{e8d6}',
            Self::Placeholder => '\u{e18a}',
            Self::View => '\u{e890}',
            Self::SetLockScreen => '\u{e7b5}',
            Self::SetTile => '\u{e97b}',
            Self::ClosedCaption => '\u{e7f0}',
            Self::StopSlideShow => '\u{e620}',
            Self::Permissions => '\u{e8d7}',
            Self::Highlight => '\u{e7e6}',
            Self::DisableUpdates => '\u{e8d8}',
            Self::UnFavorite => '\u{e8d9}',
            Self::UnPin => '\u{e77a}',
            Self::OpenLocal => '\u{e8da}',
            Self::Mute => '\u{e74f}',
            Self::Italic => '\u{e8db}',
            Self::Underline => '\u{e8dc}',
            Self::Bold => '\u{e8dd}',
            Self::MoveToFolder => '\u{e8de}',
            Self::LikeDislike => '\u{e8df}',
            Self::Dislike => '\u{e8e0}',
            Self::Like => '\u{e8e1}',
            Self::AlignRight => '\u{e8e2}',
            Self::AlignCenter => '\u{e8e3}',
            Self::AlignLeft => '\u{e8e4}',
            Self::Zoom => '\u{e71e}',
            Self::ZoomOut => '\u{e71f}',
            Self::OpenFile => '\u{e8e5}',
            Self::OtherUser => '\u{e7ee}',
            Self::Admin => '\u{e7ef}',
            Self::Street => '\u{e913}',
            Self::Map => '\u{e707}',
            Self::ClearSelection => '\u{e8e6}',
            Self::FontDecrease => '\u{e8e7}',
            Self::FontIncrease => '\u{e8e8}',
            Self::FontSize => '\u{e8e9}',
            Self::CellPhone => '\u{e8ea}',
            Self::ReShare => '\u{e8eb}',
            Self::Tag => '\u{e8ec}',
            Self::RepeatOne => '\u{e8ed}',
            Self::RepeatAll => '\u{e8ee}',
            Self::OutlineStar => '\u{e734}',
            Self::SolidStar => '\u{e735}',
            Self::Calculator => '\u{e8ef}',
            Self::Directions => '\u{e8f0}',
            Self::Target => '\u{f5f0}',
            Self::Library => '\u{e8f1}',
            Self::PhoneBook => '\u{e780}',
            Self::Memo => '\u{e77c}',
            Self::Microphone => '\u{e720}',
            Self::PostUpdate => '\u{e8f3}',
            Self::BackToWindow => '\u{e73f}',
            Self::FullScreen => '\u{e740}',
            Self::NewFolder => '\u{e8f4}',
            Self::CalendarReply => '\u{e8f5}',
            Self::UnSyncFolder => '\u{e8f6}',
            Self::ReportHacked => '\u{e730}',
            Self::SyncFolder => '\u{e8f7}',
            Self::BlockContact => '\u{e8f8}',
            Self::SwitchApps => '\u{e8f9}',
            Self::AddFriend => '\u{e8fa}',
            Self::TouchPointer => '\u{e7c9}',
            Self::GoToStart => '\u{e8fc}',
            Self::ZeroBars => '\u{e904}',
            Self::OneBar => '\u{e905}',
            Self::TwoBars => '\u{e906}',
            Self::ThreeBars => '\u{e907}',
            Self::FourBars => '\u{e908}',
            Self::Scan => '\u{e8fe}',
            Self::Preview => '\u{e8ff}',
            Self::GlobalNavigationButton => '\u{e700}',
            Self::Share => '\u{e72d}',
            Self::Print => '\u{e749}',
            Self::XboxOneConsole => '\u{e990}',
        }
    }
}

/// Represents an icon that uses a glyph of the symbol font of the theme as its content (WinUI's
/// `SymbolIcon`).
///
/// The template of the theme draws the glyph of [`symbol`](Self::symbol) ([`Symbol::glyph`]) in
/// its text part `PART_Text`, with the font of the theme resource `SymbolThemeFontFamily` (Segoe
/// Fluent Icons in the WinUI theme), 20 pixels, normal style and weight; the foreground is the
/// icon's. The font is not bundled: off Windows an application supplies a symbol font with the
/// same code points and sets `SymbolThemeFontFamily` to it (docs/porting/winui-controls.md,
/// section 5.2). The WinUI theme has the template; the Fluent and Simple themes have none.
///
/// Deviation (docs/porting/winui-controls.md, SymbolIcon): WinUI builds the text block in code
/// (`CSymbolIcon::ApplyTemplate`) with the font `Segoe Fluent Icons` written in; here the
/// template is the theme's and the font its resource.
#[repr(C)]
pub struct SymbolIcon {
    base: IconElement,
    text: RefCell<Option<Ref<TextBlock>>>,
}

ferro_class!(SymbolIcon: IconElement);
ferroui_base::ferro_class_info!(SymbolIcon {
    new: SymbolIcon::new,
    markup: {
        namespace: "FerroUI.Controls",
        attributes: [TemplatePart("PART_Text", type(Ref<TextBlock>))],
    },
});
ferro_impl_classes!(SymbolIcon: StyledElementImpl, LayoutableImpl, InteractiveImpl, InputElementImpl, ControlImpl);

impl FerroObjectImpl for SymbolIcon {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        // CSymbolIcon::OnPropertyChanged: the symbol and the foreground go to the text block.
        if change.property() == Self::symbol_property().as_property()
            || change.property() == TemplatedControl::foreground_property().as_property()
        {
            this.update_child_text_block();
        }
    }
}

impl VisualImpl for SymbolIcon {
    // WinUI's icons draw their glyph as it is in a right-to-left flow (a FontIcon mirrors it only
    // with MirroredWhenRightToLeft).
    fn bypass_flow_direction_policies(_this: &Self) -> bool {
        true
    }
}

impl TemplatedControlImpl for SymbolIcon {
    fn on_apply_template(this: &Self, args: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, args);
        *this.text.borrow_mut() = args.name_scope().find_as::<TextBlock>("PART_Text");
        this.update_child_text_block();
    }
}

ferroui_base::ferro_properties! { impl SymbolIcon {
    ferro_property!(
        /// Defines the `Symbol` property: the symbol the icon shows. `Emoji` by default, as in
        /// WinUI (`CSymbolIcon::m_nSymbol`).
        pub fn symbol_property() -> StyledProperty<Symbol> {
            FerroProperty::register::<SymbolIcon, _>("Symbol", Symbol::Emoji)
        }
    );
} }

impl SymbolIcon {
    fn static_constructor() {
        Layoutable::affects_measure::<SymbolIcon>(&[Self::symbol_property().as_property()]);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: IconElement::construct(), text: RefCell::new(None) }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Creates an icon that shows `symbol` (WinUI's `SymbolIcon(Symbol)`).
    pub fn with_symbol(symbol: Symbol) -> Ref<Self> {
        let icon = Self::new();
        icon.set_symbol(symbol);
        icon
    }

    /// Gets the symbol the icon shows.
    pub fn symbol(&self) -> Symbol {
        self.get_value(Self::symbol_property())
    }

    /// Sets the symbol the icon shows.
    pub fn set_symbol(&self, value: Symbol) {
        self.set_value(Self::symbol_property(), value);
    }

    /// `CSymbolIcon::UpdateChildTextBlockText`, with the foreground the text block gets in
    /// `OnPropertyChanged`.
    fn update_child_text_block(&self) {
        let text = self.text.borrow().clone();
        if let Some(text) = text {
            text.set_text(Some(&self.symbol().glyph().to_string()));
            text.set_foreground(self.foreground());
        }
    }
}

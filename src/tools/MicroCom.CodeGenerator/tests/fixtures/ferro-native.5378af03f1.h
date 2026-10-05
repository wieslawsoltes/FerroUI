#pragma once
#include "com.h"
#include "stddef.h"
struct FrnSize;
struct FrnPixelSize;
struct FrnRect;
struct FrnVector;
struct FrnPoint;
struct FrnScreen;
struct FrnFramebuffer;
struct FrnColor;
struct IFerroNativeFactory;
struct IFrnString;
struct IFrnTopLevel;
struct IFrnWindowBase;
struct IFrnPopup;
struct IFrnWindow;
struct IFrnTopLevelEvents;
struct IFrnWindowBaseEvents;
struct IFrnWindowEvents;
struct IFrnTextInputMethodClient;
struct IFrnTextInputMethod;
struct IFrnMacOptions;
struct IFrnActionCallback;
struct IFrnPlatformThreadingInterfaceEvents;
struct IFrnLoopCancellation;
struct IFrnPlatformThreadingInterface;
struct IFrnSystemDialogEvents;
struct IFrnStorageProvider;
struct IFrnFilePickerFileTypes;
struct IFrnScreenEvents;
struct IFrnScreens;
struct IFrnClipboard;
struct IFrnClipboardDataSource;
struct IFrnClipboardDataItem;
struct IFrnClipboardDataValue;
struct IFrnCursor;
struct IFrnCursorFactory;
struct IFrnSoftwareRenderTarget;
struct IFrnGlDisplay;
struct IFrnGlContext;
struct IFrnGlSurfaceRenderTarget;
struct IFrnGlSurfaceRenderingSession;
struct IFrnMetalDisplay;
struct IFrnMetalDevice;
struct IFrnMetalRenderTarget;
struct IFrnMTLSharedEvent;
struct IFrnMetalTexture;
struct IFrnNativeObjectsMemoryManagement;
struct IFrnMetalRenderingSession;
struct IFrnTrayIcon;
struct IFrnMenu;
struct IFrnPredicateCallback;
struct IFrnMenuItem;
struct IFrnMenuEvents;
struct IFrnStringArray;
struct IFrnDndResultCallback;
struct IFrnGCHandleDeallocatorCallback;
struct IFrnDispatcher;
struct IFrnNativeControlHost;
struct IFrnNativeControlHostTopLevelAttachment;
struct IFrnApplicationEvents;
struct IFrnApplicationCommands;
struct IFrnAutomationPeer;
struct IFrnAutomationPeerArray;
struct IFrnAutomationNode;
struct IFrnPlatformSettings;
struct IFrnPlatformBehaviorInhibition;
struct IFrnPlatformRenderTimer;
enum FrnKey
{
    FrnKeyNone = 0,
    FrnKeyCancel = 1,
    FrnKeyBack = 2,
    FrnKeyTab = 3,
    FrnKeyLineFeed = 4,
    FrnKeyClear = 5,
    FrnKeyReturn = 6,
    FrnKeyEnter = 6,
    FrnKeyPause = 7,
    FrnKeyCapsLock = 8,
    FrnKeyCapital = 8,
    FrnKeyHangulMode = 9,
    FrnKeyKanaMode = 9,
    FrnKeyJunjaMode = 10,
    FrnKeyFinalMode = 11,
    FrnKeyKanjiMode = 12,
    FrnKeyHanjaMode = 12,
    FrnKeyEscape = 13,
    FrnKeyImeConvert = 14,
    FrnKeyImeNonConvert = 15,
    FrnKeyImeAccept = 16,
    FrnKeyImeModeChange = 17,
    FrnKeySpace = 18,
    FrnKeyPageUp = 19,
    FrnKeyPrior = 19,
    FrnKeyPageDown = 20,
    FrnKeyNext = 20,
    FrnKeyEnd = 21,
    FrnKeyHome = 22,
    FrnKeyLeft = 23,
    FrnKeyUp = 24,
    FrnKeyRight = 25,
    FrnKeyDown = 26,
    FrnKeySelect = 27,
    FrnKeyPrint = 28,
    FrnKeyExecute = 29,
    FrnKeySnapshot = 30,
    FrnKeyPrintScreen = 30,
    FrnKeyInsert = 31,
    FrnKeyDelete = 32,
    FrnKeyHelp = 33,
    FrnKeyD0 = 34,
    FrnKeyD1 = 35,
    FrnKeyD2 = 36,
    FrnKeyD3 = 37,
    FrnKeyD4 = 38,
    FrnKeyD5 = 39,
    FrnKeyD6 = 40,
    FrnKeyD7 = 41,
    FrnKeyD8 = 42,
    FrnKeyD9 = 43,
    FrnKeyA = 44,
    FrnKeyB = 45,
    FrnKeyC = 46,
    FrnKeyD = 47,
    FrnKeyE = 48,
    FrnKeyF = 49,
    FrnKeyG = 50,
    FrnKeyH = 51,
    FrnKeyI = 52,
    FrnKeyJ = 53,
    FrnKeyK = 54,
    FrnKeyL = 55,
    FrnKeyM = 56,
    FrnKeyN = 57,
    FrnKeyO = 58,
    FrnKeyP = 59,
    FrnKeyQ = 60,
    FrnKeyR = 61,
    FrnKeyS = 62,
    FrnKeyT = 63,
    FrnKeyU = 64,
    FrnKeyV = 65,
    FrnKeyW = 66,
    FrnKeyX = 67,
    FrnKeyY = 68,
    FrnKeyZ = 69,
    FrnKeyLWin = 70,
    FrnKeyRWin = 71,
    FrnKeyApps = 72,
    FrnKeySleep = 73,
    FrnKeyNumPad0 = 74,
    FrnKeyNumPad1 = 75,
    FrnKeyNumPad2 = 76,
    FrnKeyNumPad3 = 77,
    FrnKeyNumPad4 = 78,
    FrnKeyNumPad5 = 79,
    FrnKeyNumPad6 = 80,
    FrnKeyNumPad7 = 81,
    FrnKeyNumPad8 = 82,
    FrnKeyNumPad9 = 83,
    FrnKeyMultiply = 84,
    FrnKeyAdd = 85,
    FrnKeySeparator = 86,
    FrnKeySubtract = 87,
    FrnKeyDecimal = 88,
    FrnKeyDivide = 89,
    FrnKeyF1 = 90,
    FrnKeyF2 = 91,
    FrnKeyF3 = 92,
    FrnKeyF4 = 93,
    FrnKeyF5 = 94,
    FrnKeyF6 = 95,
    FrnKeyF7 = 96,
    FrnKeyF8 = 97,
    FrnKeyF9 = 98,
    FrnKeyF10 = 99,
    FrnKeyF11 = 100,
    FrnKeyF12 = 101,
    FrnKeyF13 = 102,
    FrnKeyF14 = 103,
    FrnKeyF15 = 104,
    FrnKeyF16 = 105,
    FrnKeyF17 = 106,
    FrnKeyF18 = 107,
    FrnKeyF19 = 108,
    FrnKeyF20 = 109,
    FrnKeyF21 = 110,
    FrnKeyF22 = 111,
    FrnKeyF23 = 112,
    FrnKeyF24 = 113,
    FrnKeyNumLock = 114,
    FrnKeyScroll = 115,
    FrnKeyLeftShift = 116,
    FrnKeyRightShift = 117,
    FrnKeyLeftCtrl = 118,
    FrnKeyRightCtrl = 119,
    FrnKeyLeftAlt = 120,
    FrnKeyRightAlt = 121,
    FrnKeyBrowserBack = 122,
    FrnKeyBrowserForward = 123,
    FrnKeyBrowserRefresh = 124,
    FrnKeyBrowserStop = 125,
    FrnKeyBrowserSearch = 126,
    FrnKeyBrowserFavorites = 127,
    FrnKeyBrowserHome = 128,
    FrnKeyVolumeMute = 129,
    FrnKeyVolumeDown = 130,
    FrnKeyVolumeUp = 131,
    FrnKeyMediaNextTrack = 132,
    FrnKeyMediaPreviousTrack = 133,
    FrnKeyMediaStop = 134,
    FrnKeyMediaPlayPause = 135,
    FrnKeyLaunchMail = 136,
    FrnKeySelectMedia = 137,
    FrnKeyLaunchApplication1 = 138,
    FrnKeyLaunchApplication2 = 139,
    FrnKeyOemSemicolon = 140,
    FrnKeyOem1 = 140,
    FrnKeyOemPlus = 141,
    FrnKeyOemComma = 142,
    FrnKeyOemMinus = 143,
    FrnKeyOemPeriod = 144,
    FrnKeyOemQuestion = 145,
    FrnKeyOem2 = 145,
    FrnKeyOemTilde = 146,
    FrnKeyOem3 = 146,
    FrnKeyAbntC1 = 147,
    FrnKeyAbntC2 = 148,
    FrnKeyOemOpenBrackets = 149,
    FrnKeyOem4 = 149,
    FrnKeyOemPipe = 150,
    FrnKeyOem5 = 150,
    FrnKeyOemCloseBrackets = 151,
    FrnKeyOem6 = 151,
    FrnKeyOemQuotes = 152,
    FrnKeyOem7 = 152,
    FrnKeyOem8 = 153,
    FrnKeyOemBackslash = 154,
    FrnKeyOem102 = 154,
    FrnKeyImeProcessed = 155,
    FrnKeySystem = 156,
    FrnKeyOemAttn = 157,
    FrnKeyDbeAlphanumeric = 157,
    FrnKeyOemFinish = 158,
    FrnKeyDbeKatakana = 158,
    FrnKeyDbeHiragana = 159,
    FrnKeyOemCopy = 159,
    FrnKeyDbeSbcsChar = 160,
    FrnKeyOemAuto = 160,
    FrnKeyDbeDbcsChar = 161,
    FrnKeyOemEnlw = 161,
    FrnKeyOemBackTab = 162,
    FrnKeyDbeRoman = 162,
    FrnKeyDbeNoRoman = 163,
    FrnKeyAttn = 163,
    FrnKeyCrSel = 164,
    FrnKeyDbeEnterWordRegisterMode = 164,
    FrnKeyExSel = 165,
    FrnKeyDbeEnterImeConfigureMode = 165,
    FrnKeyEraseEof = 166,
    FrnKeyDbeFlushString = 166,
    FrnKeyPlay = 167,
    FrnKeyDbeCodeInput = 167,
    FrnKeyDbeNoCodeInput = 168,
    FrnKeyZoom = 168,
    FrnKeyNoName = 169,
    FrnKeyDbeDetermineString = 169,
    FrnKeyDbeEnterDialogConversionMode = 170,
    FrnKeyPa1 = 170,
    FrnKeyOemClear = 171,
    FrnKeyDeadCharProcessed = 172,
    FrnKeyFnLeftArrow = 10001,
    FrnKeyFnRightArrow = 10002,
    FrnKeyFnUpArrow = 10003,
    FrnKeyFnDownArrow = 10004,
};
enum FrnPhysicalKey
{
    FrnPhysicalKeyNone = 0,
    FrnPhysicalKeyBackquote = 1,
    FrnPhysicalKeyBackslash = 2,
    FrnPhysicalKeyBracketLeft = 3,
    FrnPhysicalKeyBracketRight = 4,
    FrnPhysicalKeyComma = 5,
    FrnPhysicalKeyDigit0 = 6,
    FrnPhysicalKeyDigit1 = 7,
    FrnPhysicalKeyDigit2 = 8,
    FrnPhysicalKeyDigit3 = 9,
    FrnPhysicalKeyDigit4 = 10,
    FrnPhysicalKeyDigit5 = 11,
    FrnPhysicalKeyDigit6 = 12,
    FrnPhysicalKeyDigit7 = 13,
    FrnPhysicalKeyDigit8 = 14,
    FrnPhysicalKeyDigit9 = 15,
    FrnPhysicalKeyEqual = 16,
    FrnPhysicalKeyIntlBackslash = 17,
    FrnPhysicalKeyIntlRo = 18,
    FrnPhysicalKeyIntlYen = 19,
    FrnPhysicalKeyA = 20,
    FrnPhysicalKeyB = 21,
    FrnPhysicalKeyC = 22,
    FrnPhysicalKeyD = 23,
    FrnPhysicalKeyE = 24,
    FrnPhysicalKeyF = 25,
    FrnPhysicalKeyG = 26,
    FrnPhysicalKeyH = 27,
    FrnPhysicalKeyI = 28,
    FrnPhysicalKeyJ = 29,
    FrnPhysicalKeyK = 30,
    FrnPhysicalKeyL = 31,
    FrnPhysicalKeyM = 32,
    FrnPhysicalKeyN = 33,
    FrnPhysicalKeyO = 34,
    FrnPhysicalKeyP = 35,
    FrnPhysicalKeyQ = 36,
    FrnPhysicalKeyR = 37,
    FrnPhysicalKeyS = 38,
    FrnPhysicalKeyT = 39,
    FrnPhysicalKeyU = 40,
    FrnPhysicalKeyV = 41,
    FrnPhysicalKeyW = 42,
    FrnPhysicalKeyX = 43,
    FrnPhysicalKeyY = 44,
    FrnPhysicalKeyZ = 45,
    FrnPhysicalKeyMinus = 46,
    FrnPhysicalKeyPeriod = 47,
    FrnPhysicalKeyQuote = 48,
    FrnPhysicalKeySemicolon = 49,
    FrnPhysicalKeySlash = 50,
    FrnPhysicalKeyAltLeft = 51,
    FrnPhysicalKeyAltRight = 52,
    FrnPhysicalKeyBackspace = 53,
    FrnPhysicalKeyCapsLock = 54,
    FrnPhysicalKeyContextMenu = 55,
    FrnPhysicalKeyControlLeft = 56,
    FrnPhysicalKeyControlRight = 57,
    FrnPhysicalKeyEnter = 58,
    FrnPhysicalKeyMetaLeft = 59,
    FrnPhysicalKeyMetaRight = 60,
    FrnPhysicalKeyShiftLeft = 61,
    FrnPhysicalKeyShiftRight = 62,
    FrnPhysicalKeySpace = 63,
    FrnPhysicalKeyTab = 64,
    FrnPhysicalKeyConvert = 65,
    FrnPhysicalKeyKanaMode = 66,
    FrnPhysicalKeyLang1 = 67,
    FrnPhysicalKeyLang2 = 68,
    FrnPhysicalKeyLang3 = 69,
    FrnPhysicalKeyLang4 = 70,
    FrnPhysicalKeyLang5 = 71,
    FrnPhysicalKeyNonConvert = 72,
    FrnPhysicalKeyDelete = 73,
    FrnPhysicalKeyEnd = 74,
    FrnPhysicalKeyHelp = 75,
    FrnPhysicalKeyHome = 76,
    FrnPhysicalKeyInsert = 77,
    FrnPhysicalKeyPageDown = 78,
    FrnPhysicalKeyPageUp = 79,
    FrnPhysicalKeyArrowDown = 80,
    FrnPhysicalKeyArrowLeft = 81,
    FrnPhysicalKeyArrowRight = 82,
    FrnPhysicalKeyArrowUp = 83,
    FrnPhysicalKeyNumLock = 84,
    FrnPhysicalKeyNumPad0 = 85,
    FrnPhysicalKeyNumPad1 = 86,
    FrnPhysicalKeyNumPad2 = 87,
    FrnPhysicalKeyNumPad3 = 88,
    FrnPhysicalKeyNumPad4 = 89,
    FrnPhysicalKeyNumPad5 = 90,
    FrnPhysicalKeyNumPad6 = 91,
    FrnPhysicalKeyNumPad7 = 92,
    FrnPhysicalKeyNumPad8 = 93,
    FrnPhysicalKeyNumPad9 = 94,
    FrnPhysicalKeyNumPadAdd = 95,
    FrnPhysicalKeyNumPadClear = 96,
    FrnPhysicalKeyNumPadComma = 97,
    FrnPhysicalKeyNumPadDecimal = 98,
    FrnPhysicalKeyNumPadDivide = 99,
    FrnPhysicalKeyNumPadEnter = 100,
    FrnPhysicalKeyNumPadEqual = 101,
    FrnPhysicalKeyNumPadMultiply = 102,
    FrnPhysicalKeyNumPadParenLeft = 103,
    FrnPhysicalKeyNumPadParenRight = 104,
    FrnPhysicalKeyNumPadSubtract = 105,
    FrnPhysicalKeyEscape = 106,
    FrnPhysicalKeyF1 = 107,
    FrnPhysicalKeyF2 = 108,
    FrnPhysicalKeyF3 = 109,
    FrnPhysicalKeyF4 = 110,
    FrnPhysicalKeyF5 = 111,
    FrnPhysicalKeyF6 = 112,
    FrnPhysicalKeyF7 = 113,
    FrnPhysicalKeyF8 = 114,
    FrnPhysicalKeyF9 = 115,
    FrnPhysicalKeyF10 = 116,
    FrnPhysicalKeyF11 = 117,
    FrnPhysicalKeyF12 = 118,
    FrnPhysicalKeyF13 = 119,
    FrnPhysicalKeyF14 = 120,
    FrnPhysicalKeyF15 = 121,
    FrnPhysicalKeyF16 = 122,
    FrnPhysicalKeyF17 = 123,
    FrnPhysicalKeyF18 = 124,
    FrnPhysicalKeyF19 = 125,
    FrnPhysicalKeyF20 = 126,
    FrnPhysicalKeyF21 = 127,
    FrnPhysicalKeyF22 = 128,
    FrnPhysicalKeyF23 = 129,
    FrnPhysicalKeyF24 = 130,
    FrnPhysicalKeyPrintScreen = 131,
    FrnPhysicalKeyScrollLock = 132,
    FrnPhysicalKeyPause = 133,
    FrnPhysicalKeyBrowserBack = 134,
    FrnPhysicalKeyBrowserFavorites = 135,
    FrnPhysicalKeyBrowserForward = 136,
    FrnPhysicalKeyBrowserHome = 137,
    FrnPhysicalKeyBrowserRefresh = 138,
    FrnPhysicalKeyBrowserSearch = 139,
    FrnPhysicalKeyBrowserStop = 140,
    FrnPhysicalKeyEject = 141,
    FrnPhysicalKeyLaunchApp1 = 142,
    FrnPhysicalKeyLaunchApp2 = 143,
    FrnPhysicalKeyLaunchMail = 144,
    FrnPhysicalKeyMediaPlayPause = 145,
    FrnPhysicalKeyMediaSelect = 146,
    FrnPhysicalKeyMediaStop = 147,
    FrnPhysicalKeyMediaTrackNext = 148,
    FrnPhysicalKeyMediaTrackPrevious = 149,
    FrnPhysicalKeyPower = 150,
    FrnPhysicalKeySleep = 151,
    FrnPhysicalKeyAudioVolumeDown = 152,
    FrnPhysicalKeyAudioVolumeMute = 153,
    FrnPhysicalKeyAudioVolumeUp = 154,
    FrnPhysicalKeyWakeUp = 155,
    FrnPhysicalKeyAgain = 156,
    FrnPhysicalKeyCopy = 157,
    FrnPhysicalKeyCut = 158,
    FrnPhysicalKeyFind = 159,
    FrnPhysicalKeyOpen = 160,
    FrnPhysicalKeyPaste = 161,
    FrnPhysicalKeyProps = 162,
    FrnPhysicalKeySelect = 163,
    FrnPhysicalKeyUndo = 164,
};
enum SystemDecorations
{
    SystemDecorationsNone = 0,
    SystemDecorationsBorderOnly = 1,
    SystemDecorationsFull = 2,
};
enum FrnAutomationProperty
{
    AutomationPeer_AutomationId,
    AutomationPeer_BoundingRectangle,
    AutomationPeer_ClassName,
    AutomationPeer_Name,
    RangeValueProvider_Value,
    ValueProvider_Value,
    ToggleProvider_ToggleState,
    ExpandCollapseProvider_ExpandCollapseState,
    SelectionItemProvider_IsSelected,
    SelectionProvider_Selection,
};
enum FrnScreenOrientation
{
    UnknownOrientation,
    Landscape,
    Portrait,
    LandscapeFlipped,
    PortraitFlipped,
};
enum FrnPixelFormat
{
    kFrnRgb565,
    kFrnRgba8888,
    kFrnBgra8888,
};
enum FrnRawMouseEventType
{
    LeaveWindow,
    LeftButtonDown,
    LeftButtonUp,
    RightButtonDown,
    RightButtonUp,
    MiddleButtonDown,
    MiddleButtonUp,
    XButton1Down,
    XButton1Up,
    XButton2Down,
    XButton2Up,
    Move,
    Wheel,
    NonClientLeftButtonDown,
    TouchBegin,
    TouchUpdate,
    TouchEnd,
    TouchCancel,
    Magnify,
    Rotate,
    Swipe,
};
enum FrnRawKeyEventType
{
    KeyDown,
    KeyUp,
};
enum FrnInputModifiers
{
    FrnInputModifiersNone = 0,
    Alt = 1,
    Control = 2,
    Shift = 4,
    Windows = 8,
    LeftMouseButton = 16,
    RightMouseButton = 32,
    MiddleMouseButton = 64,
    XButton1MouseButton = 128,
    XButton2MouseButton = 256,
};
enum class FrnDragDropEffects
{
    None = 0,
    Copy = 1,
    Move = 2,
    Link = 4,
};
enum class FrnDragEventType
{
    Enter,
    Over,
    Leave,
    Drop,
};
enum FrnWindowState
{
    Normal,
    Minimized,
    Maximized,
    FullScreen,
};
enum FrnStandardCursorType
{
    CursorArrow,
    CursorIbeam,
    CursorWait,
    CursorCross,
    CursorUpArrow,
    CursorSizeWestEast,
    CursorSizeNorthSouth,
    CursorSizeAll,
    CursorNo,
    CursorHand,
    CursorAppStarting,
    CursorHelp,
    CursorTopSide,
    CursorBottomSize,
    CursorLeftSide,
    CursorRightSide,
    CursorTopLeftCorner,
    CursorTopRightCorner,
    CursorBottomLeftCorner,
    CursorBottomRightCorner,
    CursorDragMove,
    CursorDragCopy,
    CursorDragLink,
    CursorNone,
};
enum FrnWindowEdge
{
    WindowEdgeNorthWest,
    WindowEdgeNorth,
    WindowEdgeNorthEast,
    WindowEdgeWest,
    WindowEdgeEast,
    WindowEdgeSouthWest,
    WindowEdgeSouth,
    WindowEdgeSouthEast,
};
enum FrnMenuItemToggleType
{
    None,
    CheckMark,
    Radio,
};
enum FrnPlatformResizeReason
{
    ResizeUnspecified,
    ResizeUser,
    ResizeApplication,
    ResizeLayout,
    ResizeDpiChange,
};
enum FrnAutomationControlType
{
    AutomationNone,
    AutomationButton,
    AutomationCalendar,
    AutomationCheckBox,
    AutomationComboBox,
    AutomationComboBoxItem,
    AutomationEdit,
    AutomationHyperlink,
    AutomationImage,
    AutomationListItem,
    AutomationList,
    AutomationMenu,
    AutomationMenuBar,
    AutomationMenuItem,
    AutomationProgressBar,
    AutomationRadioButton,
    AutomationScrollBar,
    AutomationSlider,
    AutomationSpinner,
    AutomationStatusBar,
    AutomationTab,
    AutomationTabItem,
    AutomationText,
    AutomationToolBar,
    AutomationToolTip,
    AutomationTree,
    AutomationTreeItem,
    AutomationCustom,
    AutomationGroup,
    AutomationThumb,
    AutomationDataGrid,
    AutomationDataItem,
    AutomationDocument,
    AutomationSplitButton,
    AutomationWindow,
    AutomationPane,
    AutomationHeader,
    AutomationHeaderItem,
    AutomationTable,
    AutomationTitleBar,
    AutomationSeparator,
    AutomationExpander,
};
enum FrnLandmarkType
{
    LandmarkNone = -1,
    LandmarkBanner,
    LandmarkComplementary,
    LandmarkContentInfo,
    LandmarkRegion,
    LandmarkForm,
    LandmarkMain,
    LandmarkNavigation,
    LandmarkSearch,
};
enum FrnWindowTransparencyMode
{
    Opaque,
    Transparent,
    Blur,
};
enum FrnPlatformThemeVariant
{
    Light,
    Dark,
    HighContrastLight,
    HighContrastDark,
};
enum FrnPointerDeviceType
{
    Mouse,
    Pen,
};
enum FrnLiveSetting
{
    LiveSettingOff,
    LiveSettingPolite,
    LiveSettingAssertive,
};
struct FrnSize
{
    double Width;
    double Height;
};
struct FrnPixelSize
{
    int Width;
    int Height;
};
struct FrnRect
{
    double X;
    double Y;
    double Width;
    double Height;
};
struct FrnVector
{
    double X;
    double Y;
};
struct FrnPoint
{
    double X;
    double Y;
};
struct FrnScreen
{
    FrnRect Bounds;
    FrnRect WorkingArea;
    float Scaling;
    bool IsPrimary;
    FrnScreenOrientation Orientation;
};
struct FrnFramebuffer
{
    void* Data;
    int Width;
    int Height;
    int Stride;
    FrnVector Dpi;
    FrnPixelFormat PixelFormat;
};
struct FrnColor
{
    unsigned char Alpha;
    unsigned char Red;
    unsigned char Green;
    unsigned char Blue;
};
COMINTERFACE(IFerroNativeFactory, 809c652e, 7396, 11d2, 97, 71, 00, a0, c9, b4, d5, 0c) : IUnknown
{
    virtual HRESULT Initialize (
        IFrnGCHandleDeallocatorCallback* deallocator, 
        IFrnApplicationEvents* appCb, 
        IFrnDispatcher* dispatcher
    ) = 0;
    virtual IFrnMacOptions* GetMacOptions () = 0;
    virtual HRESULT CreateTopLevel (
        IFrnTopLevelEvents* cb, 
        IFrnTopLevel** ppv
    ) = 0;
    virtual HRESULT CreateWindow (
        IFrnWindowEvents* cb, 
        IFrnWindow** ppv
    ) = 0;
    virtual HRESULT CreatePopup (
        IFrnWindowEvents* cb, 
        IFrnPopup** ppv
    ) = 0;
    virtual HRESULT CreatePlatformThreadingInterface (
        IFrnPlatformThreadingInterface** ppv
    ) = 0;
    virtual HRESULT CreateStorageProvider (
        IFrnStorageProvider** ppv
    ) = 0;
    virtual HRESULT CreateScreens (
        IFrnScreenEvents* cb, 
        IFrnScreens** ppv
    ) = 0;
    virtual HRESULT CreateClipboard (
        IFrnClipboard** ppv
    ) = 0;
    virtual HRESULT CreateCursorFactory (
        IFrnCursorFactory** ppv
    ) = 0;
    virtual HRESULT ObtainGlDisplay (
        IFrnGlDisplay** ppv
    ) = 0;
    virtual HRESULT ObtainMetalDisplay (
        IFrnMetalDisplay** ppv
    ) = 0;
    virtual HRESULT SetAppMenu (
        IFrnMenu* menu
    ) = 0;
    virtual HRESULT SetServicesMenu (
        IFrnMenu* menu
    ) = 0;
    virtual HRESULT CreateMenu (
        IFrnMenuEvents* cb, 
        IFrnMenu** ppv
    ) = 0;
    virtual HRESULT CreateMenuItem (
        IFrnMenuItem** ppv
    ) = 0;
    virtual HRESULT CreateMenuItemSeparator (
        IFrnMenuItem** ppv
    ) = 0;
    virtual HRESULT CreateTrayIcon (
        IFrnTrayIcon** ppv
    ) = 0;
    virtual HRESULT CreateApplicationCommands (
        IFrnApplicationCommands** ppv
    ) = 0;
    virtual HRESULT CreatePlatformSettings (
        IFrnPlatformSettings** ppv
    ) = 0;
    virtual HRESULT CreatePlatformBehaviorInhibition (
        IFrnPlatformBehaviorInhibition** ppv
    ) = 0;
    virtual HRESULT CreatePlatformRenderTimer (
        IFrnPlatformRenderTimer** ppv
    ) = 0;
    virtual HRESULT ImportMTLSharedEvent (
        void* idMtlSharedEvent, 
        IFrnMTLSharedEvent** ppv
    ) = 0;
    virtual HRESULT CreateMemoryManagementHelper (
        IFrnNativeObjectsMemoryManagement** ppv
    ) = 0;
    virtual HRESULT SetDockMenu (
        IFrnMenu* menu
    ) = 0;
};
COMINTERFACE(IFrnString, 233e094f, 9b9f, 44a3, 9a, 6e, 69, 48, bb, dd, 9f, b1) : IUnknown
{
    virtual HRESULT Pointer (
        void** retOut
    ) = 0;
    virtual HRESULT Length (
        int* ret
    ) = 0;
};
COMINTERFACE(IFrnTopLevel, e8cccd3e, e6dc, 430a, a0, b9, 2c, e7, d7, 92, 2d, e6) : IUnknown
{
    virtual HRESULT GetClientSize (
        FrnSize* ret
    ) = 0;
    virtual HRESULT GetScaling (
        double* ret
    ) = 0;
    virtual HRESULT Invalidate () = 0;
    virtual HRESULT PointToClient (
        FrnPoint point, 
        FrnPoint* ret
    ) = 0;
    virtual HRESULT PointToScreen (
        FrnPoint point, 
        FrnPoint* ret
    ) = 0;
    virtual HRESULT SetCursor (
        IFrnCursor* cursor
    ) = 0;
    virtual HRESULT CreateGlRenderTarget (
        IFrnGlContext* context, 
        IFrnGlSurfaceRenderTarget** ret
    ) = 0;
    virtual HRESULT CreateSoftwareRenderTarget (
        IFrnSoftwareRenderTarget** ret
    ) = 0;
    virtual HRESULT CreateMetalRenderTarget (
        IFrnMetalDevice* device, 
        IFrnMetalRenderTarget** ret
    ) = 0;
    virtual HRESULT ObtainNSViewHandle (
        void** retOut
    ) = 0;
    virtual HRESULT ObtainNSViewHandleRetained (
        void** retOut
    ) = 0;
    virtual HRESULT CreateNativeControlHost (
        IFrnNativeControlHost** retOut
    ) = 0;
    virtual HRESULT GetInputMethod (
        IFrnTextInputMethod** ppv
    ) = 0;
    virtual HRESULT SetTransparencyMode (
        FrnWindowTransparencyMode mode
    ) = 0;
    virtual HRESULT GetCurrentDisplayId (
        unsigned int* ret
    ) = 0;
    virtual HRESULT BeginDragAndDropOperation (
        FrnDragDropEffects effects, 
        FrnPoint point, 
        IFrnClipboardDataSource* source, 
        IFrnDndResultCallback* callback, 
        void* sourceHandle
    ) = 0;
};
COMINTERFACE(IFrnWindowBase, e5aca675, 02b7, 4129, aa, 79, d6, e4, 17, 21, 0b, da) : virtual IFrnTopLevel
{
    virtual HRESULT GetFrameSize (
        FrnSize* result
    ) = 0;
    virtual HRESULT SetFrameThemeVariant (
        FrnPlatformThemeVariant mode
    ) = 0;
    virtual HRESULT SetParent (
        IFrnWindowBase* parent
    ) = 0;
    virtual HRESULT Show (
        bool activate, 
        bool isDialog
    ) = 0;
    virtual HRESULT Hide () = 0;
    virtual HRESULT Close () = 0;
    virtual HRESULT Activate () = 0;
    virtual HRESULT SetMinMaxSize (
        FrnSize minSize, 
        FrnSize maxSize
    ) = 0;
    virtual HRESULT Resize (
        double width, 
        double height, 
        FrnPlatformResizeReason reason
    ) = 0;
    virtual HRESULT BeginMoveDrag () = 0;
    virtual HRESULT BeginResizeDrag (
        FrnWindowEdge edge
    ) = 0;
    virtual HRESULT GetPosition (
        FrnPoint* ret
    ) = 0;
    virtual HRESULT SetPosition (
        FrnPoint point
    ) = 0;
    virtual HRESULT SetTopMost (
        bool value
    ) = 0;
    virtual HRESULT SetMainMenu (
        IFrnMenu* menu
    ) = 0;
    virtual HRESULT ObtainNSWindowHandle (
        void** retOut
    ) = 0;
    virtual HRESULT ObtainNSWindowHandleRetained (
        void** retOut
    ) = 0;
};
COMINTERFACE(IFrnPopup, 83e588f3, 6981, 4e48, 9e, a0, e1, e5, 69, f7, 9a, 91) : virtual IFrnWindowBase
{
};
COMINTERFACE(IFrnWindow, cab661de, 49d6, 4ead, b5, 9c, ea, c9, b2, b6, c2, 8d) : virtual IFrnWindowBase
{
    virtual HRESULT SetEnabled (
        bool enable
    ) = 0;
    virtual HRESULT SetCanResize (
        bool value
    ) = 0;
    virtual HRESULT SetCanMinimize (
        bool value
    ) = 0;
    virtual HRESULT SetCanMaximize (
        bool value
    ) = 0;
    virtual HRESULT SetDecorations (
        SystemDecorations value
    ) = 0;
    virtual HRESULT SetTitle (
        char* utf8Title
    ) = 0;
    virtual HRESULT SetTitleBarColor (
        FrnColor color
    ) = 0;
    virtual HRESULT SetWindowState (
        FrnWindowState state
    ) = 0;
    virtual HRESULT GetWindowState (
        FrnWindowState* ret
    ) = 0;
    virtual HRESULT TakeFocusFromChildren () = 0;
    virtual HRESULT SetExtendClientArea (
        bool enable
    ) = 0;
    virtual HRESULT GetExtendTitleBarHeight (
        double* ret
    ) = 0;
    virtual HRESULT SetExtendTitleBarHeight (
        double value
    ) = 0;
    virtual HRESULT GetWindowZOrder (
        long* ret
    ) = 0;
};
COMINTERFACE(IFrnTopLevelEvents, fda9c1b3, 69e0, 43d7, 94, 59, 8c, c9, 7c, b4, 1f, 6a) : IUnknown
{
    virtual void Closed () = 0;
    virtual HRESULT Paint () = 0;
    virtual void Resized (
        const FrnSize& size, 
        FrnPlatformResizeReason reason
    ) = 0;
    virtual void RawMouseEvent (
        FrnRawMouseEventType type, 
        FrnPointerDeviceType deviceType, 
        u_int64_t timeStamp, 
        FrnInputModifiers modifiers, 
        FrnPoint point, 
        FrnVector delta, 
        float pressure, 
        float xTilt, 
        float yTilt
    ) = 0;
    virtual bool RawKeyEvent (
        FrnRawKeyEventType type, 
        u_int64_t timeStamp, 
        FrnInputModifiers modifiers, 
        FrnKey key, 
        FrnPhysicalKey physicalKey, 
        const char* keySymbol
    ) = 0;
    virtual bool RawTextInputEvent (
        u_int64_t timeStamp, 
        const char* text
    ) = 0;
    virtual void ScalingChanged (
        double scaling
    ) = 0;
    virtual void RunRenderPriorityJobs () = 0;
    virtual void LostFocus () = 0;
    virtual IFrnAutomationPeer* GetAutomationPeer () = 0;
    virtual FrnDragDropEffects DragEvent (
        FrnDragEventType type, 
        FrnPoint position, 
        FrnInputModifiers modifiers, 
        FrnDragDropEffects effects, 
        IFrnClipboard* clipboard, 
        void* dataTransferHandle
    ) = 0;
};
COMINTERFACE(IFrnWindowBaseEvents, 939b6599, 40a8, 4710, a4, c8, 5d, 72, d8, f1, 74, fb) : IFrnTopLevelEvents
{
    virtual void Activated () = 0;
    virtual void Deactivated () = 0;
    virtual void PositionChanged (
        FrnPoint position
    ) = 0;
};
COMINTERFACE(IFrnWindowEvents, 1ae178ee, 1fcc, 447f, b6, dd, b7, bb, 72, 7f, 93, 4c) : IFrnWindowBaseEvents
{
    virtual bool Closing () = 0;
    virtual void WindowStateChanged (
        FrnWindowState state
    ) = 0;
    virtual void GotInputWhenDisabled () = 0;
};
COMINTERFACE(IFrnTextInputMethodClient, f2079145, a2d9, 42b8, a8, 5e, 27, 32, e3, c2, b0, 55) : IUnknown
{
    virtual void SetPreeditText (
        char* preeditText
    ) = 0;
    virtual void SelectInSurroundingText (
        int start, 
        int length
    ) = 0;
};
COMINTERFACE(IFrnTextInputMethod, 1382a29f, e260, 4c7a, b8, 3f, c9, 9f, c7, 2e, 27, c2) : IUnknown
{
    virtual HRESULT SetClient (
        IFrnTextInputMethodClient* client
    ) = 0;
    virtual void Reset () = 0;
    virtual void SetCursorRect (
        FrnRect rect
    ) = 0;
    virtual void SetSurroundingText (
        char* text, 
        int anchorOffset, 
        int cursorOffset
    ) = 0;
};
COMINTERFACE(IFrnMacOptions, e34ae0f8, 18b4, 48a3, b0, 9d, 2e, 6b, 19, a3, cf, 5e) : IUnknown
{
    virtual HRESULT SetShowInDock (
        int show
    ) = 0;
    virtual HRESULT SetApplicationTitle (
        char* utf8string
    ) = 0;
    virtual HRESULT SetDisableSetProcessName (
        int disable
    ) = 0;
    virtual HRESULT SetDisableAppDelegate (
        int disable
    ) = 0;
};
COMINTERFACE(IFrnActionCallback, 04c1b049, 1f43, 418a, 91, 59, ca, e6, 27, ec, 13, 67) : IUnknown
{
    virtual void Run () = 0;
};
COMINTERFACE(IFrnPlatformThreadingInterfaceEvents, 6df4d2db, 0b80, 4f59, ad, 88, 0b, aa, 5e, 21, eb, 14) : IUnknown
{
    virtual void Signaled () = 0;
    virtual void Timer () = 0;
    virtual void ReadyForBackgroundProcessing () = 0;
};
COMINTERFACE(IFrnLoopCancellation, 97330f88, c22b, 4a8e, a1, 30, 20, 15, 20, 09, 1b, 01) : IUnknown
{
    virtual void Cancel () = 0;
};
COMINTERFACE(IFrnPlatformThreadingInterface, fbc06f3d, 7860, 42df, 83, fd, 53, c4, b0, 2d, d9, c3) : IUnknown
{
    virtual bool GetCurrentThreadIsLoopThread () = 0;
    virtual void SetEvents (
        IFrnPlatformThreadingInterfaceEvents* cb
    ) = 0;
    virtual IFrnLoopCancellation* CreateLoopCancellation () = 0;
    virtual void RunLoop (
        IFrnLoopCancellation* cancel
    ) = 0;
    virtual void Signal () = 0;
    virtual void UpdateTimer (
        int ms
    ) = 0;
    virtual void RequestBackgroundProcessing () = 0;
};
COMINTERFACE(IFrnSystemDialogEvents, 6c621a6e, e4c1, 4ae3, 97, 49, 83, ee, ef, fa, 09, b6) : IUnknown
{
    virtual void OnCompleted (
        IFrnStringArray* array
    ) = 0;
    virtual void OnCompletedWithFilter (
        IFrnStringArray* array, 
        int selectedFilterIndex
    ) = 0;
};
COMINTERFACE(IFrnStorageProvider, 4d7a47db, a944, 4061, ab, e7, 62, cb, 6a, a0, ff, d5) : IUnknown
{
    virtual void SelectFolderDialog (
        IFrnTopLevel* parentTopLevel, 
        IFrnSystemDialogEvents* events, 
        bool allowMultiple, 
        const char* title, 
        const char* initialPath
    ) = 0;
    virtual void OpenFileDialog (
        IFrnTopLevel* parentTopLevel, 
        IFrnSystemDialogEvents* events, 
        bool allowMultiple, 
        const char* title, 
        const char* initialDirectory, 
        const char* initialFile, 
        IFrnFilePickerFileTypes* filters
    ) = 0;
    virtual void SaveFileDialog (
        IFrnTopLevel* parentTopLevel, 
        IFrnSystemDialogEvents* events, 
        const char* title, 
        const char* initialDirectory, 
        const char* initialFile, 
        IFrnFilePickerFileTypes* filters
    ) = 0;
    virtual HRESULT SaveBookmarkToBytes (
        IFrnString* fileUri, 
        void** err, 
        IFrnString** ppv
    ) = 0;
    virtual HRESULT ReadBookmarkFromBytes (
        void* ptr, 
        int len, 
        IFrnString** ppv
    ) = 0;
    virtual void ReleaseBookmark (
        IFrnString* fileUri
    ) = 0;
    virtual bool OpenSecurityScope (
        IFrnString* fileUri
    ) = 0;
    virtual void CloseSecurityScope (
        IFrnString* fileUri
    ) = 0;
    virtual HRESULT TryResolveFileReferenceUri (
        IFrnString* fileUri, 
        IFrnString** ret
    ) = 0;
};
COMINTERFACE(IFrnFilePickerFileTypes, 4d7ab7db, a111, 406f, ab, eb, 11, cb, 6a, a0, 33, d5) : IUnknown
{
    virtual int GetCount () = 0;
    virtual bool IsDefaultType (
        int index
    ) = 0;
    virtual bool IsAnyType (
        int index
    ) = 0;
    virtual IFrnString* GetName (
        int index
    ) = 0;
    virtual HRESULT GetPatterns (
        int index, 
        IFrnStringArray** ppv
    ) = 0;
    virtual HRESULT GetExtensions (
        int index, 
        IFrnStringArray** ppv
    ) = 0;
    virtual HRESULT GetMimeTypes (
        int index, 
        IFrnStringArray** ppv
    ) = 0;
    virtual HRESULT GetAppleUniformTypeIdentifiers (
        int index, 
        IFrnStringArray** ppv
    ) = 0;
};
COMINTERFACE(IFrnScreenEvents, 424b1bd4, a111, 4987, bf, d0, 9d, 64, 21, 54, b1, b3) : IUnknown
{
    virtual HRESULT OnChanged () = 0;
};
COMINTERFACE(IFrnScreens, 9a52bc7a, d8c7, 4230, 8d, 34, 70, 4a, 0b, 70, a9, 33) : IUnknown
{
    virtual HRESULT GetScreenIds (
        unsigned int* ptrFirstResult, 
        int* ret
    ) = 0;
    virtual HRESULT GetScreen (
        unsigned int screenId, 
        void** localizedName, 
        FrnScreen* ret
    ) = 0;
};
COMINTERFACE(IFrnClipboard, 792b1bd4, 76cc, 46ea, bf, d0, 9d, 64, 21, 54, b1, b3) : IUnknown
{
    virtual HRESULT GetFormats (
        int64_t changeCount, 
        IFrnStringArray** ret
    ) = 0;
    virtual HRESULT GetItemCount (
        int64_t changeCount, 
        int* ret
    ) = 0;
    virtual HRESULT GetItemFormats (
        int index, 
        int64_t changeCount, 
        IFrnStringArray** ret
    ) = 0;
    virtual HRESULT GetItemValueAsString (
        int index, 
        int64_t changeCount, 
        const char* format, 
        IFrnString** ret
    ) = 0;
    virtual HRESULT GetItemValueAsBytes (
        int index, 
        int64_t changeCount, 
        const char* format, 
        IFrnString** ret
    ) = 0;
    virtual HRESULT Clear (
        int64_t* ret
    ) = 0;
    virtual HRESULT GetChangeCount (
        int64_t* ret
    ) = 0;
    virtual HRESULT SetData (
        IFrnClipboardDataSource* dataSource
    ) = 0;
    virtual bool IsTextFormat (
        const char* format
    ) = 0;
};
COMINTERFACE(IFrnClipboardDataSource, 10b39f02, efcb, 428b, be, e5, a0, b0, 12, c1, fb, 7d) : IUnknown
{
    virtual int GetItemCount () = 0;
    virtual HRESULT GetItem (
        int index, 
        IFrnClipboardDataItem** ppv
    ) = 0;
};
COMINTERFACE(IFrnClipboardDataItem, e40f36d9, 69f4, 45fd, 9c, a2, 6e, 64, e8, 0f, eb, 6d) : IUnknown
{
    virtual HRESULT ProvideFormats (
        IFrnStringArray** ppv
    ) = 0;
    virtual HRESULT GetValue (
        const char* format, 
        IFrnClipboardDataValue** ppv
    ) = 0;
};
COMINTERFACE(IFrnClipboardDataValue, e97f24f6, 1c84, 4d95, 8f, fe, 5b, 2c, 72, e0, 16, ed) : IUnknown
{
    virtual bool IsString () = 0;
    virtual IFrnString* AsString () = 0;
    virtual long GetByteLength () = 0;
    virtual void CopyBytesTo (
        void* buffer
    ) = 0;
};
COMINTERFACE(IFrnCursor, 3f998545, f027, 4d4d, bd, 2a, 1a, 80, 92, 6d, 98, 4e) : IUnknown
{
};
COMINTERFACE(IFrnCursorFactory, 51ecfb12, c427, 4757, a2, c9, 15, 96, bf, ce, 53, ef) : IUnknown
{
    virtual HRESULT GetCursor (
        FrnStandardCursorType cursorType, 
        IFrnCursor** retOut
    ) = 0;
    virtual HRESULT CreateCustomCursor (
        void* bitmapData, 
        size_t length, 
        FrnPixelSize hotPixel, 
        IFrnCursor** retOut
    ) = 0;
};
COMINTERFACE(IFrnSoftwareRenderTarget, 931062d2, 5bc8, 4062, 85, 88, 83, dd, 8d, eb, 99, c2) : IUnknown
{
    virtual HRESULT SetFrame (
        FrnFramebuffer* fb
    ) = 0;
};
COMINTERFACE(IFrnGlDisplay, 60452465, 8616, 40af, bc, 00, 04, 2e, 69, 82, 8c, e7) : IUnknown
{
    virtual HRESULT CreateContext (
        IFrnGlContext* share, 
        IFrnGlContext** ppv
    ) = 0;
    virtual void LegacyClearCurrentContext () = 0;
    virtual HRESULT WrapContext (
        void* native, 
        IFrnGlContext** ppv
    ) = 0;
    virtual void* GetProcAddress (
        char* proc
    ) = 0;
};
COMINTERFACE(IFrnGlContext, 78c5711e, 2a98, 40d2, ba, c4, 0c, c9, a4, 9d, c4, f3) : IUnknown
{
    virtual HRESULT MakeCurrent (
        IUnknown** ppv
    ) = 0;
    virtual HRESULT LegacyMakeCurrent () = 0;
    virtual int GetSampleCount () = 0;
    virtual int GetStencilSize () = 0;
    virtual void* GetNativeHandle () = 0;
    virtual int texImageIOSurface2D (
        int target, 
        int internal_format, 
        int width, 
        int height, 
        int format, 
        int type, 
        void* ioSurface, 
        int plane
    ) = 0;
    virtual bool GetIOKitRegistryId (
        uint64_t* value
    ) = 0;
};
COMINTERFACE(IFrnGlSurfaceRenderTarget, 931062d2, 5bc8, 4062, 85, 88, 83, dd, 8d, eb, 99, c2) : IUnknown
{
    virtual HRESULT BeginDrawing (
        IFrnGlSurfaceRenderingSession** ret
    ) = 0;
};
COMINTERFACE(IFrnGlSurfaceRenderingSession, e625b406, f04c, 484e, 94, 6a, 4a, bd, 2c, 60, 15, ad) : IUnknown
{
    virtual HRESULT GetPixelSize (
        FrnPixelSize* ret
    ) = 0;
    virtual HRESULT GetScaling (
        double* ret
    ) = 0;
};
COMINTERFACE(IFrnMetalDisplay, da291767, 4db3, 4598, 89, 3d, 09, ec, aa, 23, 89, 3f) : IUnknown
{
    virtual HRESULT CreateDevice (
        IFrnMetalDevice** ret
    ) = 0;
};
COMINTERFACE(IFrnMetalDevice, 969fa914, b74a, 4c9f, 87, 25, 51, 60, dc, 63, 57, 9e) : IUnknown
{
    virtual void* GetDevice () = 0;
    virtual void* GetQueue () = 0;
    virtual bool GetIOKitRegistryId (
        uint64_t* value
    ) = 0;
    virtual HRESULT ImportIOSurface (
        void* handle, 
        FrnPixelFormat pixelFormat, 
        IFrnMetalTexture** ppv
    ) = 0;
    virtual HRESULT ImportSharedEvent (
        void* mtlSharedEventInstance, 
        IFrnMTLSharedEvent** ppv
    ) = 0;
    virtual HRESULT SubmitWait (
        IFrnMTLSharedEvent* ev, 
        uint64_t value
    ) = 0;
    virtual HRESULT SubmitSignal (
        IFrnMTLSharedEvent* ev, 
        uint64_t value
    ) = 0;
};
COMINTERFACE(IFrnMetalRenderTarget, f1306b71, eca0, 426e, 87, 00, 10, 51, 92, 69, 3b, 1a) : IUnknown
{
    virtual HRESULT BeginDrawing (
        IFrnMetalRenderingSession** ret
    ) = 0;
};
COMINTERFACE(IFrnMTLSharedEvent, a1f4fcde, 9152, 48bd, bf, 8a, b1, b6, 51, 13, 4a, 69) : IUnknown
{
    virtual void* GetNativeHandle () = 0;
    virtual bool Wait (
        uint64_t value, 
        uint64_t timeoutMS
    ) = 0;
    virtual void SetSignaledValue (
        uint64_t value
    ) = 0;
    virtual uint64_t GetSignaledValue () = 0;
};
COMINTERFACE(IFrnMetalTexture, 722aad20, a87b, 4ce5, b5, 0f, f0, 5c, fa, 4c, da, 39) : IUnknown
{
    virtual void* GetNativeHandle () = 0;
    virtual int GetWidth () = 0;
    virtual int GetHeight () = 0;
    virtual int GetSampleCount () = 0;
};
COMINTERFACE(IFrnNativeObjectsMemoryManagement, 74027aa2, 5262, 45a5, a7, 4a, 5a, 53, 37, 3d, cc, 17) : IUnknown
{
    virtual void RetainNSObject (
        void* obj
    ) = 0;
    virtual void ReleaseNSObject (
        void* obj
    ) = 0;
    virtual uint64_t GetRetainCountForNSObject (
        void* obj
    ) = 0;
    virtual void RetainCFObject (
        void* obj
    ) = 0;
    virtual void ReleaseCFObject (
        void* obj
    ) = 0;
    virtual int64_t GetRetainCountForCFObject (
        void* obj
    ) = 0;
};
COMINTERFACE(IFrnMetalRenderingSession, e625b406, f04c, 484e, 94, 6a, 4a, bd, 2c, 60, 15, ad) : IUnknown
{
    virtual HRESULT GetPixelSize (
        FrnPixelSize* ret
    ) = 0;
    virtual double GetScaling () = 0;
    virtual void* GetTexture () = 0;
};
COMINTERFACE(IFrnTrayIcon, 60992d19, 38f0, 4141, a0, a9, 76, ac, 30, 38, 01, f3) : IUnknown
{
    virtual HRESULT SetIcon (
        void* data, 
        size_t length
    ) = 0;
    virtual HRESULT SetMenu (
        IFrnMenu* menu
    ) = 0;
    virtual HRESULT SetIsVisible (
        bool isVisible
    ) = 0;
    virtual HRESULT SetToolTipText (
        char* text
    ) = 0;
    virtual HRESULT SetIsTemplateIcon (
        bool text
    ) = 0;
};
COMINTERFACE(IFrnMenu, a7724dc1, cf6b, 4fa8, 9d, 23, 22, 8b, f2, 59, 3e, dc) : IUnknown
{
    virtual HRESULT InsertItem (
        int index, 
        IFrnMenuItem* item
    ) = 0;
    virtual HRESULT RemoveItem (
        IFrnMenuItem* item
    ) = 0;
    virtual HRESULT SetTitle (
        char* utf8String
    ) = 0;
    virtual HRESULT Clear () = 0;
};
COMINTERFACE(IFrnPredicateCallback, 59e0586d, bd1c, 4b85, 98, 82, 80, d4, 48, b0, fe, d9) : IUnknown
{
    virtual bool Evaluate () = 0;
};
COMINTERFACE(IFrnMenuItem, f890219a, 1720, 4cd5, 9a, 26, cd, 95, fc, cb, f5, 3c) : IUnknown
{
    virtual HRESULT SetSubMenu (
        IFrnMenu* menu
    ) = 0;
    virtual HRESULT SetTitle (
        char* utf8String
    ) = 0;
    virtual HRESULT SetToolTip (
        char* utf8String
    ) = 0;
    virtual HRESULT SetGesture (
        FrnKey key, 
        FrnInputModifiers modifiers
    ) = 0;
    virtual HRESULT SetAction (
        IFrnPredicateCallback* predicate, 
        IFrnActionCallback* callback
    ) = 0;
    virtual HRESULT SetIsChecked (
        bool isChecked
    ) = 0;
    virtual HRESULT SetIsVisible (
        bool isVisible
    ) = 0;
    virtual HRESULT SetToggleType (
        FrnMenuItemToggleType toggleType
    ) = 0;
    virtual HRESULT SetIcon (
        void* data, 
        size_t length
    ) = 0;
};
COMINTERFACE(IFrnMenuEvents, 0af7df53, 7632, 42f4, a6, 50, 09, 92, c3, 61, b4, 77) : IUnknown
{
    virtual void NeedsUpdate () = 0;
    virtual void Opening () = 0;
    virtual void Closed () = 0;
};
COMINTERFACE(IFrnStringArray, 5142bb41, 66ab, 49e7, bb, 37, cd, 07, 9c, 00, 0f, 27) : IUnknown
{
    virtual unsigned int GetCount () = 0;
    virtual HRESULT Get (
        unsigned int index, 
        IFrnString** ppv
    ) = 0;
};
COMINTERFACE(IFrnDndResultCallback, a13d2382, 3b3a, 4d1c, 9b, 27, 8f, 34, 65, 3d, 3f, 01) : IUnknown
{
    virtual void OnDragAndDropComplete (
        FrnDragDropEffects effecct
    ) = 0;
};
COMINTERFACE(IFrnGCHandleDeallocatorCallback, f07c608e, 52e9, 422d, 83, 6e, c7, 0f, 6e, 9b, 80, f5) : IUnknown
{
    virtual void FreeGCHandle (
        void* handle
    ) = 0;
};
COMINTERFACE(IFrnDispatcher, 96688589, 5dc7, 41ec, 9c, e3, d4, 81, 94, 24, 54, ee) : IUnknown
{
    virtual void Post (
        IFrnActionCallback* cb
    ) = 0;
};
COMINTERFACE(IFrnNativeControlHost, 91c7f677, f26b, 4ff3, 93, cc, cf, 15, aa, 96, 6f, fa) : IUnknown
{
    virtual HRESULT CreateDefaultChild (
        void* parent, 
        void** retOut
    ) = 0;
    virtual IFrnNativeControlHostTopLevelAttachment* CreateAttachment () = 0;
    virtual void DestroyDefaultChild (
        void* child
    ) = 0;
};
COMINTERFACE(IFrnNativeControlHostTopLevelAttachment, 14a9e164, 1aae, 4271, bb, 78, 7b, 52, 30, 99, 9b, 52) : IUnknown
{
    virtual void* GetParentHandle () = 0;
    virtual HRESULT InitializeWithChildHandle (
        void* child
    ) = 0;
    virtual HRESULT AttachTo (
        IFrnNativeControlHost* host
    ) = 0;
    virtual void ShowInBounds (
        float x, 
        float y, 
        float width, 
        float height
    ) = 0;
    virtual void HideWithSize (
        float width, 
        float height
    ) = 0;
    virtual void ReleaseChild () = 0;
};
COMINTERFACE(IFrnApplicationEvents, 6575b5af, f27a, 4609, 86, 6c, f1, f0, 14, c2, 0f, 79) : IUnknown
{
    virtual void FilesOpened (
        IFrnStringArray* args
    ) = 0;
    virtual void UrlsOpened (
        IFrnStringArray* urls
    ) = 0;
    virtual bool TryShutdown () = 0;
    virtual void OnReopen () = 0;
    virtual void OnHide () = 0;
    virtual void OnUnhide () = 0;
    virtual void OnActivate () = 0;
    virtual void OnDeactivate () = 0;
};
COMINTERFACE(IFrnApplicationCommands, b4284791, 055b, 4313, 8c, 2e, 50, f0, a8, c7, 2c, e9) : IUnknown
{
    virtual HRESULT UnhideApp () = 0;
    virtual HRESULT HideApp () = 0;
    virtual HRESULT ShowAll () = 0;
    virtual HRESULT HideOthers () = 0;
};
COMINTERFACE(IFrnAutomationPeer, b87016f3, 7eec, 41de, b3, 85, 07, 84, 4c, 26, 8d, c4) : IUnknown
{
    virtual IFrnAutomationNode* GetNode () = 0;
    virtual void SetNode (
        IFrnAutomationNode* node
    ) = 0;
    virtual IFrnString* GetAcceleratorKey () = 0;
    virtual IFrnString* GetAccessKey () = 0;
    virtual FrnAutomationControlType GetAutomationControlType () = 0;
    virtual IFrnString* GetAutomationId () = 0;
    virtual FrnRect GetBoundingRectangle () = 0;
    virtual IFrnAutomationPeerArray* GetChildren () = 0;
    virtual IFrnString* GetClassName () = 0;
    virtual IFrnAutomationPeer* GetLabeledBy () = 0;
    virtual IFrnString* GetName () = 0;
    virtual IFrnAutomationPeer* GetParent () = 0;
    virtual IFrnAutomationPeer* GetVisualRoot () = 0;
    virtual bool HasKeyboardFocus () = 0;
    virtual bool IsContentElement () = 0;
    virtual bool IsControlElement () = 0;
    virtual bool IsEnabled () = 0;
    virtual bool IsKeyboardFocusable () = 0;
    virtual void SetFocus () = 0;
    virtual bool ShowContextMenu () = 0;
    virtual IFrnAutomationPeer* GetRootPeer () = 0;
    virtual bool IsInteropPeer () = 0;
    virtual void* InteropPeer_GetNativeControlHandle () = 0;
    virtual bool IsRootProvider () = 0;
    virtual IFrnWindowBase* RootProvider_GetWindow () = 0;
    virtual IFrnAutomationPeer* RootProvider_GetFocus () = 0;
    virtual IFrnAutomationPeer* RootProvider_GetPeerFromPoint (
        FrnPoint point
    ) = 0;
    virtual bool IsEmbeddedRootProvider () = 0;
    virtual IFrnAutomationPeer* EmbeddedRootProvider_GetFocus () = 0;
    virtual IFrnAutomationPeer* EmbeddedRootProvider_GetPeerFromPoint (
        FrnPoint point
    ) = 0;
    virtual bool IsExpandCollapseProvider () = 0;
    virtual bool ExpandCollapseProvider_GetIsExpanded () = 0;
    virtual bool ExpandCollapseProvider_GetShowsMenu () = 0;
    virtual void ExpandCollapseProvider_Expand () = 0;
    virtual void ExpandCollapseProvider_Collapse () = 0;
    virtual bool IsInvokeProvider () = 0;
    virtual void InvokeProvider_Invoke () = 0;
    virtual bool IsRangeValueProvider () = 0;
    virtual double RangeValueProvider_GetValue () = 0;
    virtual double RangeValueProvider_GetMinimum () = 0;
    virtual double RangeValueProvider_GetMaximum () = 0;
    virtual double RangeValueProvider_GetSmallChange () = 0;
    virtual double RangeValueProvider_GetLargeChange () = 0;
    virtual void RangeValueProvider_SetValue (
        double value
    ) = 0;
    virtual bool IsSelectionItemProvider () = 0;
    virtual bool SelectionItemProvider_IsSelected () = 0;
    virtual bool IsToggleProvider () = 0;
    virtual int ToggleProvider_GetToggleState () = 0;
    virtual void ToggleProvider_Toggle () = 0;
    virtual bool IsValueProvider () = 0;
    virtual IFrnString* ValueProvider_GetValue () = 0;
    virtual void ValueProvider_SetValue (
        const char* value
    ) = 0;
    virtual IFrnString* GetHelpText () = 0;
    virtual IFrnString* GetPlaceholderText () = 0;
    virtual FrnLandmarkType GetLandmarkType () = 0;
    virtual int GetHeadingLevel () = 0;
    virtual FrnLiveSetting GetLiveSetting () = 0;
};
COMINTERFACE(IFrnAutomationPeerArray, b00af5da, 78af, 4b33, bf, ff, 4c, e1, 3a, 62, 39, a9) : IUnknown
{
    virtual unsigned int GetCount () = 0;
    virtual HRESULT Get (
        unsigned int index, 
        IFrnAutomationPeer** ppv
    ) = 0;
};
COMINTERFACE(IFrnAutomationNode, 004dc40b, e435, 49dc, ba, c5, 62, 72, ee, 35, 38, 2a) : IUnknown
{
    virtual void Dispose () = 0;
    virtual void ChildrenChanged () = 0;
    virtual void PropertyChanged (
        FrnAutomationProperty property
    ) = 0;
    virtual void FocusChanged () = 0;
};
COMINTERFACE(IFrnPlatformSettings, d1f009cc, 9d2d, 493b, 84, 5d, 90, d2, c1, 04, ba, ae) : IUnknown
{
    virtual FrnPlatformThemeVariant GetPlatformTheme () = 0;
    virtual unsigned int GetAccentColor () = 0;
    virtual void RegisterColorsChange (
        IFrnActionCallback* callback
    ) = 0;
};
COMINTERFACE(IFrnPlatformBehaviorInhibition, 12edf00d, 5803, 4d3f, 99, 47, b4, 84, 0e, 5e, 93, 72) : IUnknown
{
    virtual void SetInhibitAppSleep (
        bool inhibitAppSleep, 
        char* reason
    ) = 0;
};
COMINTERFACE(IFrnPlatformRenderTimer, 22edf20d, 5803, 2d3f, 92, 47, b4, 84, 2e, 5e, 93, 22) : IUnknown
{
    virtual int RegisterTick (
        IFrnActionCallback* callback
    ) = 0;
    virtual void Start () = 0;
    virtual void Stop () = 0;
    virtual bool RunsInBackground () = 0;
};

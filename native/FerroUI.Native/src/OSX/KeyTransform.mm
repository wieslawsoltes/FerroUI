#include "KeyTransform.h"

#import <Carbon/Carbon.h>
#include <array>
#include <unordered_map>

struct KeyInfo
{
    uint16_t scanCode;
    FrnPhysicalKey physicalKey;
    FrnKey qwertyKey;
    uint16_t menuChar;
};

// ScanCode - PhysicalKey - Key mapping (the virtual key is mapped as in a standard QWERTY keyboard)
// https://github.com/chromium/chromium/blob/main/ui/events/keycodes/dom/dom_code_data.inc
// This list has the same order as the PhysicalKey enum.
const KeyInfo keyInfos[] =
{
    // Writing System Keys
    { 0x32, FrnPhysicalKeyBackquote, FrnKeyOem3, '`' },
    { 0x2A, FrnPhysicalKeyBackslash, FrnKeyOem5, '\\' },
    { 0x21, FrnPhysicalKeyBracketLeft,FrnKeyOem4, '[' },
    { 0x1E, FrnPhysicalKeyBracketRight, FrnKeyOem6, ']' },
    { 0x2B, FrnPhysicalKeyComma, FrnKeyOemComma, ',' },
    { 0x1D, FrnPhysicalKeyDigit0, FrnKeyD0, '0' },
    { 0x12, FrnPhysicalKeyDigit1, FrnKeyD1, '1' },
    { 0x13, FrnPhysicalKeyDigit2, FrnKeyD2, '2' },
    { 0x14, FrnPhysicalKeyDigit3, FrnKeyD3, '3' },
    { 0x15, FrnPhysicalKeyDigit4, FrnKeyD4, '4' },
    { 0x17, FrnPhysicalKeyDigit5, FrnKeyD5, '5' },
    { 0x16, FrnPhysicalKeyDigit6, FrnKeyD6, '6' },
    { 0x1A, FrnPhysicalKeyDigit7, FrnKeyD7, '7' },
    { 0x1C, FrnPhysicalKeyDigit8, FrnKeyD8, '8' },
    { 0x19, FrnPhysicalKeyDigit9, FrnKeyD9, '9' },
    { 0x18, FrnPhysicalKeyEqual, FrnKeyOemPlus, '=' },
    { 0x0A, FrnPhysicalKeyIntlBackslash, FrnKeyOem102, 0 },
    { 0x5E, FrnPhysicalKeyIntlRo, FrnKeyOem102, 0 },
    { 0x5D, FrnPhysicalKeyIntlYen, FrnKeyOem5, 0 },
    { 0x00, FrnPhysicalKeyA, FrnKeyA, 'a' },
    { 0x0B, FrnPhysicalKeyB, FrnKeyB, 'b' },
    { 0x08, FrnPhysicalKeyC, FrnKeyC, 'c' },
    { 0x02, FrnPhysicalKeyD, FrnKeyD, 'd' },
    { 0x0E, FrnPhysicalKeyE, FrnKeyE, 'e' },
    { 0x03, FrnPhysicalKeyF, FrnKeyF, 'f' },
    { 0x05, FrnPhysicalKeyG, FrnKeyG, 'g' },
    { 0x04, FrnPhysicalKeyH, FrnKeyH, 'h' },
    { 0x22, FrnPhysicalKeyI, FrnKeyI, 'i' },
    { 0x26, FrnPhysicalKeyJ, FrnKeyJ, 'j' },
    { 0x28, FrnPhysicalKeyK, FrnKeyK, 'k' },
    { 0x25, FrnPhysicalKeyL, FrnKeyL, 'l' },
    { 0x2E, FrnPhysicalKeyM, FrnKeyM, 'm' },
    { 0x2D, FrnPhysicalKeyN, FrnKeyN, 'n' },
    { 0x1F, FrnPhysicalKeyO, FrnKeyO, 'o' },
    { 0x23, FrnPhysicalKeyP, FrnKeyP, 'p' },
    { 0x0C, FrnPhysicalKeyQ, FrnKeyQ, 'q' },
    { 0x0F, FrnPhysicalKeyR, FrnKeyR, 'r' },
    { 0x01, FrnPhysicalKeyS, FrnKeyS, 's' },
    { 0x11, FrnPhysicalKeyT, FrnKeyT, 't' },
    { 0x20, FrnPhysicalKeyU, FrnKeyU, 'u' },
    { 0x09, FrnPhysicalKeyV, FrnKeyV, 'v' },
    { 0x0D, FrnPhysicalKeyW, FrnKeyW, 'w' },
    { 0x07, FrnPhysicalKeyX, FrnKeyX, 'x' },
    { 0x10, FrnPhysicalKeyY, FrnKeyY, 'y' },
    { 0x06, FrnPhysicalKeyZ, FrnKeyZ, 'z' },
    { 0x1B, FrnPhysicalKeyMinus, FrnKeyOemMinus, '-' },
    { 0x2F, FrnPhysicalKeyPeriod, FrnKeyOemPeriod, '.' },
    { 0x27, FrnPhysicalKeyQuote, FrnKeyOem7, '\'' },
    { 0x29, FrnPhysicalKeySemicolon, FrnKeyOem1, ';' },
    { 0x2C, FrnPhysicalKeySlash, FrnKeyOem2, '/' },

    // Functional Keys
    { 0x3A, FrnPhysicalKeyAltLeft, FrnKeyLeftAlt, 0 },
    { 0x3D, FrnPhysicalKeyAltRight, FrnKeyRightAlt, 0 },
    { 0x33, FrnPhysicalKeyBackspace, FrnKeyBack, kBackspaceCharCode },
    { 0x39, FrnPhysicalKeyCapsLock, FrnKeyCapsLock, 0 },
    { 0x6E, FrnPhysicalKeyContextMenu, FrnKeyApps, 0 },
    { 0x3B, FrnPhysicalKeyControlLeft, FrnKeyLeftCtrl, 0 },
    { 0x3E, FrnPhysicalKeyControlRight, FrnKeyRightCtrl, 0 },
    { 0x24, FrnPhysicalKeyEnter, FrnKeyEnter, kReturnCharCode },
    { 0x37, FrnPhysicalKeyMetaLeft, FrnKeyLWin, 0 },
    { 0x36, FrnPhysicalKeyMetaRight, FrnKeyRWin, 0 },
    { 0x38, FrnPhysicalKeyShiftLeft, FrnKeyLeftShift, 0 },
    { 0x3C, FrnPhysicalKeyShiftRight, FrnKeyRightShift, 0 },
    { 0x31, FrnPhysicalKeySpace, FrnKeySpace, kSpaceCharCode },
    { 0x30, FrnPhysicalKeyTab, FrnKeyTab, kTabCharCode },
    //{   , FrnPhysicalKeyConvert, 0 },
    //{   , FrnPhysicalKeyKanaMode, 0 },
    { 0x68, FrnPhysicalKeyLang1, FrnKeyKanaMode, 0 },
    { 0x66, FrnPhysicalKeyLang2, FrnKeyHanjaMode, 0 },
    //{   , FrnPhysicalKeyLang3, 0 },
    //{   , FrnPhysicalKeyLang4, 0 },
    //{   , FrnPhysicalKeyLang5, 0 },
    //{   , FrnPhysicalKeyNonConvert, 0 },

    // Control Pad Section
    { 0x75, FrnPhysicalKeyDelete, FrnKeyDelete, NSDeleteFunctionKey },
    { 0x77, FrnPhysicalKeyEnd, FrnKeyEnd, NSEndFunctionKey },
    //{   , FrnPhysicalKeyHelp, 0 },
    { 0x73, FrnPhysicalKeyHome, FrnKeyHome, NSHomeFunctionKey },
    { 0x72, FrnPhysicalKeyInsert, FrnKeyInsert, NSInsertFunctionKey },
    { 0x79, FrnPhysicalKeyPageDown, FrnKeyPageDown, NSPageDownFunctionKey },
    { 0x74, FrnPhysicalKeyPageUp, FrnKeyPageUp, NSPageUpFunctionKey },

    // Arrow Pad Section
    { 0x7D, FrnPhysicalKeyArrowDown, FrnKeyDown, NSDownArrowFunctionKey },
    { 0x7B, FrnPhysicalKeyArrowLeft, FrnKeyLeft, NSLeftArrowFunctionKey },
    { 0x7C, FrnPhysicalKeyArrowRight, FrnKeyRight, NSRightArrowFunctionKey },
    { 0x7E, FrnPhysicalKeyArrowUp, FrnKeyUp, NSUpArrowFunctionKey },

    // Numpad Section
    { 0x47, FrnPhysicalKeyNumLock, FrnKeyClear, kClearCharCode },
    { 0x52, FrnPhysicalKeyNumPad0, FrnKeyNumPad0, '0' },
    { 0x53, FrnPhysicalKeyNumPad1, FrnKeyNumPad1, '1' },
    { 0x54, FrnPhysicalKeyNumPad2, FrnKeyNumPad2, '2' },
    { 0x55, FrnPhysicalKeyNumPad3, FrnKeyNumPad3, '3' },
    { 0x56, FrnPhysicalKeyNumPad4, FrnKeyNumPad4, '4' },
    { 0x57, FrnPhysicalKeyNumPad5, FrnKeyNumPad5, '5' },
    { 0x58, FrnPhysicalKeyNumPad6, FrnKeyNumPad6, '6' },
    { 0x59, FrnPhysicalKeyNumPad7, FrnKeyNumPad7, '7' },
    { 0x5B, FrnPhysicalKeyNumPad8, FrnKeyNumPad8, '8' },
    { 0x5C, FrnPhysicalKeyNumPad9, FrnKeyNumPad9, '9' },
    { 0x45, FrnPhysicalKeyNumPadAdd, FrnKeyAdd, '+' },
    //{   , FrnPhysicalKeyNumPadClear, 0 },
    { 0x5F, FrnPhysicalKeyNumPadComma, FrnKeyAbntC2, 0 },
    { 0x41, FrnPhysicalKeyNumPadDecimal, FrnKeyDecimal, '.' },
    { 0x4B, FrnPhysicalKeyNumPadDivide, FrnKeyDivide, '/' },
    { 0x4C, FrnPhysicalKeyNumPadEnter, FrnKeyEnter, kReturnCharCode },
    { 0x51, FrnPhysicalKeyNumPadEqual, FrnKeyOemPlus, '=' },
    { 0x43, FrnPhysicalKeyNumPadMultiply, FrnKeyMultiply, '*' },
    //{   , FrnPhysicalKeyNumPadParenLeft, 0 },
    //{   , FrnPhysicalKeyNumPadParenRight, 0 },
    { 0x4E, FrnPhysicalKeyNumPadSubtract, FrnKeySubtract, '-' },

    // Function Section
    { 0x35, FrnPhysicalKeyEscape, FrnKeyEscape, kEscapeCharCode },
    { 0x7A, FrnPhysicalKeyF1, FrnKeyF1, NSF1FunctionKey },
    { 0x78, FrnPhysicalKeyF2, FrnKeyF2, NSF2FunctionKey },
    { 0x63, FrnPhysicalKeyF3, FrnKeyF3, NSF3FunctionKey },
    { 0x76, FrnPhysicalKeyF4, FrnKeyF4, NSF4FunctionKey },
    { 0x60, FrnPhysicalKeyF5, FrnKeyF5, NSF5FunctionKey },
    { 0x61, FrnPhysicalKeyF6, FrnKeyF6, NSF6FunctionKey },
    { 0x62, FrnPhysicalKeyF7, FrnKeyF7, NSF7FunctionKey },
    { 0x64, FrnPhysicalKeyF8, FrnKeyF8, NSF8FunctionKey },
    { 0x65, FrnPhysicalKeyF9, FrnKeyF9, NSF9FunctionKey },
    { 0x6D, FrnPhysicalKeyF10, FrnKeyF10, NSF10FunctionKey },
    { 0x67, FrnPhysicalKeyF11, FrnKeyF11, NSF11FunctionKey },
    { 0x6F, FrnPhysicalKeyF12, FrnKeyF12, NSF12FunctionKey },
    { 0x69, FrnPhysicalKeyF13, FrnKeyF13, NSF13FunctionKey },
    { 0x6B, FrnPhysicalKeyF14, FrnKeyF14, NSF14FunctionKey },
    { 0x71, FrnPhysicalKeyF15, FrnKeyF15, NSF15FunctionKey },
    { 0x6A, FrnPhysicalKeyF16, FrnKeyF16, NSF16FunctionKey },
    { 0x40, FrnPhysicalKeyF17, FrnKeyF17, NSF17FunctionKey },
    { 0x4F, FrnPhysicalKeyF18, FrnKeyF18, NSF18FunctionKey },
    { 0x50, FrnPhysicalKeyF19, FrnKeyF19, NSF19FunctionKey },
    { 0x5A, FrnPhysicalKeyF20, FrnKeyF20, NSF20FunctionKey },
    //{   , FrnPhysicalKeyF21, 0 },
    //{   , FrnPhysicalKeyF22, 0 },
    //{   , FrnPhysicalKeyF23, 0 },
    //{   , FrnPhysicalKeyF24, 0 },
    //{   , FrnPhysicalKeyPrintScreen, 0 },
    //{   , FrnPhysicalKeyScrollLock, 0 },
    //{   , FrnPhysicalKeyPause, 0 },

    // Media Keys
    //{   , FrnPhysicalKeyBrowserBack, 0 },
    //{   , FrnPhysicalKeyBrowserFavorites, 0 },
    //{   , FrnPhysicalKeyBrowserForward, 0 },
    //{   , FrnPhysicalKeyBrowserHome, 0 },
    //{   , FrnPhysicalKeyBrowserRefresh, 0 },
    //{   , FrnPhysicalKeyBrowserSearch, 0 },
    //{   , FrnPhysicalKeyBrowserStop, 0 },
    //{   , FrnPhysicalKeyEject, 0 },
    //{   , FrnPhysicalKeyLaunchApp1, 0 },
    //{   , FrnPhysicalKeyLaunchApp2, 0 },
    //{   , FrnPhysicalKeyLaunchMail, 0 },
    //{   , FrnPhysicalKeyMediaPlayPause, 0 },
    //{   , FrnPhysicalKeyMediaSelect, 0 },
    //{   , FrnPhysicalKeyMediaStop, 0 },
    //{   , FrnPhysicalKeyMediaTrackNext, 0 },
    //{   , FrnPhysicalKeyMediaTrackPrevious, 0 },
    //{   , FrnPhysicalKeyPower, 0 },
    //{   , FrnPhysicalKeySleep, 0 },
    { 0x49, FrnPhysicalKeyAudioVolumeDown, FrnKeyVolumeDown, 0 },
    { 0x4A, FrnPhysicalKeyAudioVolumeMute, FrnKeyVolumeMute, 0 },
    { 0x48, FrnPhysicalKeyAudioVolumeUp, FrnKeyVolumeUp, 0 },
    //{   , FrnPhysicalKeyWakeUp, 0 },

    // Legacy Keys
    //{   , FrnPhysicalKeyAgain, 0 },
    //{   , FrnPhysicalKeyCopy, 0 },
    //{   , FrnPhysicalKeyCut, 0 },
    //{   , FrnPhysicalKeyFind, 0 },
    //{   , FrnPhysicalKeyOpen, 0 },
    //{   , FrnPhysicalKeyPaste, 0 },
    //{   , FrnPhysicalKeyProps, 0 },
    //{   , FrnPhysicalKeySelect, 0 },
    //{   , FrnPhysicalKeyUndo, 0 }
};

std::unordered_map<uint16_t, FrnKey> virtualKeyFromChar =
{
    // Alphabetic keys
    { 'A', FrnKeyA },
    { 'B', FrnKeyB },
    { 'C', FrnKeyC },
    { 'D', FrnKeyD },
    { 'E', FrnKeyE },
    { 'F', FrnKeyF },
    { 'G', FrnKeyG },
    { 'H', FrnKeyH },
    { 'I', FrnKeyI },
    { 'J', FrnKeyJ },
    { 'K', FrnKeyK },
    { 'L', FrnKeyL },
    { 'M', FrnKeyM },
    { 'N', FrnKeyN },
    { 'O', FrnKeyO },
    { 'P', FrnKeyP },
    { 'Q', FrnKeyQ },
    { 'R', FrnKeyR },
    { 'S', FrnKeyS },
    { 'T', FrnKeyT },
    { 'U', FrnKeyU },
    { 'V', FrnKeyV },
    { 'W', FrnKeyW },
    { 'X', FrnKeyX },
    { 'Y', FrnKeyY },
    { 'Z', FrnKeyZ },
    { 'a', FrnKeyA },
    { 'b', FrnKeyB },
    { 'c', FrnKeyC },
    { 'd', FrnKeyD },
    { 'e', FrnKeyE },
    { 'f', FrnKeyF },
    { 'g', FrnKeyG },
    { 'h', FrnKeyH },
    { 'i', FrnKeyI },
    { 'j', FrnKeyJ },
    { 'k', FrnKeyK },
    { 'l', FrnKeyL },
    { 'm', FrnKeyM },
    { 'n', FrnKeyN },
    { 'o', FrnKeyO },
    { 'p', FrnKeyP },
    { 'q', FrnKeyQ },
    { 'r', FrnKeyR },
    { 's', FrnKeyS },
    { 't', FrnKeyT },
    { 'u', FrnKeyU },
    { 'v', FrnKeyV },
    { 'w', FrnKeyW },
    { 'x', FrnKeyX },
    { 'y', FrnKeyY },
    { 'z', FrnKeyZ },

    // Punctuation: US specific mappings (same as Chromium)
    { ';', FrnKeyOem1 },
    { ':', FrnKeyOem1 },
    { '=', FrnKeyOemPlus },
    { '+', FrnKeyOemPlus },
    { ',', FrnKeyOemComma },
    { '<', FrnKeyOemComma },
    { '-', FrnKeyOemMinus },
    { '_', FrnKeyOemMinus },
    { '.', FrnKeyOemPeriod },
    { '>', FrnKeyOemPeriod },
    { '/', FrnKeyOem2 },
    { '?', FrnKeyOem2 },
    { '`', FrnKeyOem3 },
    { '~', FrnKeyOem3 },
    { '[', FrnKeyOem4 },
    { '{', FrnKeyOem4 },
    { '\\', FrnKeyOem5 },
    { '|', FrnKeyOem5 },
    { ']', FrnKeyOem6 },
    { '}', FrnKeyOem6 },
    { '\'', FrnKeyOem7 },
    { '"', FrnKeyOem7 },

    // Apple function keys
    // https://developer.apple.com/documentation/appkit/1535851-function-key_unicode_values
    { NSDeleteFunctionKey, FrnKeyDelete },
    { NSUpArrowFunctionKey, FrnKeyUp },
    { NSLeftArrowFunctionKey, FrnKeyLeft },
    { NSRightArrowFunctionKey, FrnKeyRight },
    { NSPageUpFunctionKey, FrnKeyPageUp },
    { NSPageDownFunctionKey, FrnKeyPageDown },
    { NSHomeFunctionKey, FrnKeyHome },
    { NSEndFunctionKey, FrnKeyEnd },
    { NSClearLineFunctionKey, FrnKeyClear },
    { NSExecuteFunctionKey, FrnKeyExecute },
    { NSHelpFunctionKey, FrnKeyHelp },
    { NSInsertFunctionKey, FrnKeyInsert },
    { NSMenuFunctionKey, FrnKeyApps },
    { NSPauseFunctionKey, FrnKeyPause },
    { NSPrintFunctionKey, FrnKeyPrint },
    { NSPrintScreenFunctionKey, FrnKeyPrintScreen },
    { NSScrollLockFunctionKey, FrnKeyScroll },
    { NSF1FunctionKey, FrnKeyF1 },
    { NSF2FunctionKey, FrnKeyF2 },
    { NSF3FunctionKey, FrnKeyF3 },
    { NSF4FunctionKey, FrnKeyF4 },
    { NSF5FunctionKey, FrnKeyF5 },
    { NSF6FunctionKey, FrnKeyF6 },
    { NSF7FunctionKey, FrnKeyF7 },
    { NSF8FunctionKey, FrnKeyF8 },
    { NSF9FunctionKey, FrnKeyF9 },
    { NSF10FunctionKey, FrnKeyF10 },
    { NSF11FunctionKey, FrnKeyF11 },
    { NSF12FunctionKey, FrnKeyF12 },
    { NSF13FunctionKey, FrnKeyF13 },
    { NSF14FunctionKey, FrnKeyF14 },
    { NSF15FunctionKey, FrnKeyF15 },
    { NSF16FunctionKey, FrnKeyF16 },
    { NSF17FunctionKey, FrnKeyF17 },
    { NSF18FunctionKey, FrnKeyF18 },
    { NSF19FunctionKey, FrnKeyF19 },
    { NSF20FunctionKey, FrnKeyF20 },
    { NSF21FunctionKey, FrnKeyF21 },
    { NSF22FunctionKey, FrnKeyF22 },
    { NSF23FunctionKey, FrnKeyF23 },
    { NSF24FunctionKey, FrnKeyF24 }
};

typedef std::array<FrnPhysicalKey, 0x7F> PhysicalKeyArray;

static PhysicalKeyArray BuildPhysicalKeyFromScanCode()
{
    PhysicalKeyArray result {};

    for (auto& keyInfo : keyInfos)
    {
        result[keyInfo.scanCode] = keyInfo.physicalKey;
    }

    return result;
}

PhysicalKeyArray physicalKeyFromScanCode = BuildPhysicalKeyFromScanCode();

static std::unordered_map<FrnPhysicalKey, FrnKey, std::hash<int>> BuildQwertyVirtualKeyFromPhysicalKey()
{
    std::unordered_map<FrnPhysicalKey, FrnKey, std::hash<int>> result;
    result.reserve(sizeof(keyInfos) / sizeof(keyInfos[0]));

    for (auto& keyInfo : keyInfos)
    {
        result[keyInfo.physicalKey] = keyInfo.qwertyKey;
    }

    return result;
}

std::unordered_map<FrnPhysicalKey, FrnKey, std::hash<int>> qwertyVirtualKeyFromPhysicalKey = BuildQwertyVirtualKeyFromPhysicalKey();

static std::unordered_map<FrnKey, uint16_t, std::hash<int>> BuildMenuCharFromVirtualKey()
{
    std::unordered_map<FrnKey, uint16_t, std::hash<int>> result;
    result.reserve(100);
    
    for (auto& keyInfo : keyInfos)
    {
        if (keyInfo.menuChar != 0)
            result[keyInfo.qwertyKey] = keyInfo.menuChar;
    }

    return result;
}

std::unordered_map<FrnKey, uint16_t, std::hash<int>> menuCharFromVirtualKey = BuildMenuCharFromVirtualKey();

static bool IsNumpadOrNumericKey(FrnPhysicalKey physicalKey)
{
    return (physicalKey >= FrnPhysicalKeyDigit0 && physicalKey <= FrnPhysicalKeyDigit9)
        || (physicalKey >= FrnPhysicalKeyNumLock && physicalKey <= FrnPhysicalKeyNumPadSubtract);
}

FrnPhysicalKey PhysicalKeyFromScanCode(uint16_t scanCode)
{
    return scanCode < physicalKeyFromScanCode.size() ? physicalKeyFromScanCode[scanCode] : FrnPhysicalKeyNone;
}

static bool IsAllowedAsciiChar(UniChar c)
{
    if (c < 0x20)
    {
        switch (c)
        {
            case kBackspaceCharCode:
            case kReturnCharCode:
            case kTabCharCode:
            case kEscapeCharCode:
                return true;
            default:
                return false;
        }
    }

    if (c == kDeleteCharCode)
        return false;

    return true;
}

static UniCharCount CharsFromScanCode(UInt16 scanCode, NSEventModifierFlags modifierFlags, UInt16 keyAction, UniChar* buffer, UniCharCount bufferSize)
{
    auto currentKeyboard = TISCopyCurrentKeyboardInputSource();
    if (!currentKeyboard)
        return 0;

    auto layoutData = static_cast<CFDataRef>(TISGetInputSourceProperty(currentKeyboard, kTISPropertyUnicodeKeyLayoutData));
    if (!layoutData)
        return 0;

    auto* keyboardLayout = reinterpret_cast<const UCKeyboardLayout*>(CFDataGetBytePtr(layoutData));

    UInt32 deadKeyState = 0;
    UniCharCount length = 0;
    
    int glyphModifiers = 0;
    if (modifierFlags & NSEventModifierFlagShift)
        glyphModifiers |= shiftKey;
    if (modifierFlags & NSEventModifierFlagCapsLock)
        glyphModifiers |= alphaLock;
    if (modifierFlags & NSEventModifierFlagOption)
        glyphModifiers |= optionKey;

    auto result = UCKeyTranslate(
        keyboardLayout,
        scanCode,
        keyAction,
        (glyphModifiers >> 8) & 0xFF,
        LMGetKbdType(),
        kUCKeyTranslateNoDeadKeysBit,
        &deadKeyState,
        bufferSize,
        &length,
        buffer);

    if (result != noErr)
        return 0;

    if (deadKeyState)
    {
        // translate a space with dead key state to get the dead key itself
        result = UCKeyTranslate(
            keyboardLayout,
            kVK_Space,
            keyAction,
            0,
            LMGetKbdType(),
            kUCKeyTranslateNoDeadKeysBit,
            &deadKeyState,
            bufferSize,
            &length,
            buffer);

        if (result != noErr)
            return 0;
    }

    if (length == 1 && buffer[0] <= 0x7F && !IsAllowedAsciiChar(buffer[0]))
        return 0;

    return length;
}

FrnKey VirtualKeyFromScanCode(uint16_t scanCode, NSEventModifierFlags modifierFlags)
{
    auto physicalKey = PhysicalKeyFromScanCode(scanCode);
    if (!IsNumpadOrNumericKey(physicalKey))
    {
        const UniCharCount charCount = 4;
        UniChar chars[charCount];
        auto length = CharsFromScanCode(scanCode, modifierFlags, kUCKeyActionDown, chars, charCount);
        if (length > 0)
        {
            auto it = virtualKeyFromChar.find(chars[0]);
            if (it != virtualKeyFromChar.end())
                return it->second;
        }
    }

    auto it = qwertyVirtualKeyFromPhysicalKey.find(physicalKey);
    return it == qwertyVirtualKeyFromPhysicalKey.end() ? FrnKeyNone : it->second;
}

NSString* KeySymbolFromScanCode(uint16_t scanCode, NSEventModifierFlags modifierFlags)
{
    auto physicalKey = PhysicalKeyFromScanCode(scanCode);

    const UniCharCount charCount = 4;
    UniChar chars[charCount];
    auto length = CharsFromScanCode(scanCode, modifierFlags, kUCKeyActionDisplay, chars, charCount);
    if (length > 0)
        return [NSString stringWithCharacters:chars length:length];

    auto it = qwertyVirtualKeyFromPhysicalKey.find(physicalKey);
    if (it == qwertyVirtualKeyFromPhysicalKey.end())
        return nullptr;

    auto menuChar = MenuCharFromVirtualKey(it->second);
    return menuChar == 0 || menuChar > 0x7E ? nullptr : [NSString stringWithCharacters:&menuChar length:1];
}

uint16_t MenuCharFromVirtualKey(FrnKey key)
{
    auto it = menuCharFromVirtualKey.find(key);
    return it == menuCharFromVirtualKey.end() ? 0 : it->second;
}

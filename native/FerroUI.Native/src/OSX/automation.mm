#include "common.h"
#include "automation.h"
#include "FrnAutomationNode.h"
#include "FrnString.h"
#include "INSWindowHolder.h"
#include "FrnView.h"
#include "WindowInterfaces.h"

static NSRange ToNSRange(FrnTextSpan span)
{
    if (span.Start < 0 || span.End < span.Start || span.End >= NSNotFound)
        return NSMakeRange(NSNotFound, 0);
    return NSMakeRange((NSUInteger)span.Start, (NSUInteger)(span.End - span.Start));
}

static FrnTextSpan ToTextSpan(NSRange range)
{
    if (range.location >= NSNotFound || range.length >= NSNotFound - range.location)
        return {-1, -1};
    return {(int64_t)range.location, (int64_t)(range.location + range.length)};
}

static NSString* TextFromUtf8(IFrnString* value)
{
    ComPtr<IFrnString> text(value, true);
    int length;
    if (!text.getRaw() || text->Length(&length) != S_OK || length < 0)
        return nil;
    if (length == 0)
        return @"";
    void* bytes;
    if (text->Pointer(&bytes) != S_OK || bytes == nullptr)
        return nil;
    return [[NSString alloc] initWithBytes:bytes length:(NSUInteger)length encoding:NSUTF8StringEncoding];
}

static NSColor* TextColor(FrnColor color)
{
    return [NSColor colorWithSRGBRed:color.Red / 255.0
                              green:color.Green / 255.0
                               blue:color.Blue / 255.0
                              alpha:color.Alpha / 255.0];
}

static NSFont* TextFont(NSString* family, FrnTextStyle style)
{
    auto manager = [NSFontManager sharedFontManager];
    const auto traits = style.Italic ? NSItalicFontMask : NSUnitalicFontMask;
    // AppKit's family lookup uses its own weight scale, with 5 for regular and 9 for bold.
    constexpr NSInteger weights[] = {2, 2, 3, 4, 5, 6, 8, 9, 10, 12, 14};
    const auto weight = weights[(style.FontWeight + 50) / 100];
    NSFont* font = nil;
    if (family.length > 0)
        font = [manager fontWithFamily:family traits:traits weight:weight size:style.FontSize];
    if (font == nil)
    {
        font = [NSFont systemFontOfSize:style.FontSize
                                weight:style.FontWeight >= 700 ? NSFontWeightBold : NSFontWeightRegular];
        if (style.Italic)
            font = [manager convertFont:font toHaveTrait:NSItalicFontMask];
    }
    return font;
}

static void AddAccessibleTextStyle(NSMutableAttributedString* text, NSRange range,
                                   FrnTextStyle style, NSString* family)
{
    if (style.FontSize > 0.0)
    {
        NSFont* font = TextFont(family, style);
        if (font != nil)
        {
            auto keys = @{
                NSAccessibilityFontNameKey: font.fontName,
                NSAccessibilityFontFamilyKey: font.familyName ?: @"",
                NSAccessibilityVisibleNameKey: font.displayName ?: @"",
                NSAccessibilityFontSizeKey: @(font.pointSize),
            };
            [text addAttribute:NSAccessibilityFontTextAttribute value:keys range:range];
        }
    }
    if (style.HasForeground)
        [text addAttribute:NSAccessibilityForegroundColorTextAttribute
                    value:(__bridge id)TextColor(style.Foreground).CGColor range:range];
    if (style.HasBackground)
        [text addAttribute:NSAccessibilityBackgroundColorTextAttribute
                    value:(__bridge id)TextColor(style.Background).CGColor range:range];
    if (style.Underline)
        [text addAttribute:NSAccessibilityUnderlineTextAttribute value:@(NSUnderlineStyleSingle) range:range];
    if (style.Strikethrough)
        [text addAttribute:NSAccessibilityStrikethroughTextAttribute value:@YES range:range];
    if (style.Script != 0)
        [text addAttribute:NSAccessibilitySuperscriptTextAttribute value:@(style.Script) range:range];
}

@implementation FrnAccessibilityElement
{
    ComPtr<IFrnAutomationPeer> _peer;
    FrnAutomationNode* _node;
    NSMutableArray* _children;
    NSArray<NSString*>* _attributeNames;
}

+ (NSAccessibilityElement *)acquire:(IFrnAutomationPeer *)peer
{
    if (peer == nullptr)
        return nil;
    
    auto instance = peer->GetNode();
    
    if (instance != nullptr)
        return dynamic_cast<FrnAutomationNode*>(instance)->GetOwner();
    
    if (peer->IsInteropPeer())
    {
        auto view = (__bridge NSAccessibilityElement*)peer->InteropPeer_GetNativeControlHandle();
        return view;
    }
    else if (peer->IsRootProvider())
    {
        auto window = peer->RootProvider_GetWindow();
        
        if (window == nullptr)
        {
            NSLog(@"IRootProvider.PlatformImpl returned null or a non-WindowBaseImpl.");
            return nil;
        }
        
        auto holder = dynamic_cast<INSViewHolder*>(window);
        auto view = holder->GetNSView();
        return (NSAccessibilityElement*)[view window];
    }
    else
    {
        return [[FrnAccessibilityElement alloc] initWithPeer:peer];
    }
}

- (FrnAccessibilityElement *)initWithPeer:(IFrnAutomationPeer *)peer
{
    self = [super init];
    _peer = peer;
    _node = new FrnAutomationNode(self);
    _peer->SetNode(_node);
    return self;
}

- (void)dealloc
{
    if (_node)
        delete _node;
    _node = nullptr;
}

- (NSString *)description
{
    return [NSString stringWithFormat:@"%@ '%@' (%p)",
        GetNSStringAndRelease(_peer->GetClassName()),
        GetNSStringAndRelease(_peer->GetName()),
        _peer.getRaw()];
}

- (IFrnAutomationPeer *)peer
{
    return _peer;
}

- (BOOL)isAccessibilityElement
{
    return _peer->IsControlElement();
}

- (NSAccessibilityRole)accessibilityRole
{
    auto controlType = _peer->GetAutomationControlType();

    switch (controlType) {
        case AutomationButton: return NSAccessibilityButtonRole;
        case AutomationCalendar: return NSAccessibilityGridRole;
        case AutomationCheckBox: return NSAccessibilityCheckBoxRole;
        case AutomationComboBox: return NSAccessibilityPopUpButtonRole;
        case AutomationComboBoxItem: return NSAccessibilityMenuItemRole;
        case AutomationEdit:
            return _peer->TextProvider_HasRanges() && _peer->TextProvider_IsMultiline()
                ? NSAccessibilityTextAreaRole : NSAccessibilityTextFieldRole;
        case AutomationHyperlink: return NSAccessibilityLinkRole;
        case AutomationImage: return NSAccessibilityImageRole;
        case AutomationListItem: return NSAccessibilityRowRole;
        case AutomationList: return NSAccessibilityListRole;
        case AutomationMenu: return NSAccessibilityMenuBarRole;
        case AutomationMenuBar: return NSAccessibilityMenuBarRole;
        case AutomationMenuItem: return NSAccessibilityMenuItemRole;
        case AutomationProgressBar: return NSAccessibilityProgressIndicatorRole;
        case AutomationRadioButton: return NSAccessibilityRadioButtonRole;
        case AutomationScrollBar: return NSAccessibilityScrollBarRole;
        case AutomationSlider: return NSAccessibilitySliderRole;
        case AutomationSpinner: return NSAccessibilityIncrementorRole;
        case AutomationStatusBar: return NSAccessibilityGroupRole;
        case AutomationTab: return NSAccessibilityTabGroupRole;
        case AutomationTabItem: return NSAccessibilityRadioButtonRole;
        case AutomationText: return NSAccessibilityStaticTextRole;
        case AutomationToolBar: return NSAccessibilityToolbarRole;
        case AutomationToolTip: return NSAccessibilityPopoverRole;
        case AutomationTree: return NSAccessibilityOutlineRole;
        case AutomationTreeItem: return NSAccessibilityCellRole;
        case AutomationCustom: return NSAccessibilityUnknownRole;
        case AutomationGroup: return NSAccessibilityGroupRole;
        case AutomationThumb: return NSAccessibilityHandleRole;
        case AutomationDataGrid: return NSAccessibilityGridRole;
        case AutomationDataItem: return NSAccessibilityCellRole;
        case AutomationDocument: return NSAccessibilityStaticTextRole;
        case AutomationSplitButton: return NSAccessibilityPopUpButtonRole;
        case AutomationWindow: return NSAccessibilityWindowRole;
        case AutomationPane: return NSAccessibilityGroupRole;
        case AutomationScrollViewer: return NSAccessibilityScrollAreaRole;
        case AutomationHeader: return @"AXHeading";
        case AutomationHeaderItem:  return NSAccessibilityButtonRole;
        case AutomationTable: return NSAccessibilityTableRole;
        case AutomationTitleBar: return NSAccessibilityGroupRole;
        case AutomationExpander: return NSAccessibilityDisclosureTriangleRole;
        // Treat unknown roles as generic group container items. Returning
        // NSAccessibilityUnknownRole is also possible but makes the screen
        // reader focus on the item instead of passing focus to child items.
        default: return NSAccessibilityGroupRole;
    }
}

- (NSAccessibilitySubrole)accessibilitySubrole
{
    auto controlType = _peer->GetAutomationControlType();
    switch (controlType) {
        case AutomationList: return @"AXContentList";
        case AutomationListItem: return NSAccessibilityTableRowSubrole;
    }

    auto landmarkType = _peer->GetLandmarkType();
    switch (landmarkType) {
        case LandmarkBanner: return @"AXLandmarkBanner";
        case LandmarkComplementary: return @"AXLandmarkComplementary";
        case LandmarkContentInfo: return @"AXLandmarkContentInfo";
        case LandmarkRegion: return @"AXLandmarkRegion";
        case LandmarkForm: return @"AXLandmarkForm";
        case LandmarkMain: return @"AXLandmarkMain";
        case LandmarkNavigation: return @"AXLandmarkNavigation";
        case LandmarkSearch: return @"AXLandmarkSearch";
    }

    return NSAccessibilityUnknownSubrole;
}

- (NSString *)accessibilityRoleDescription
{
    auto landmarkType = _peer->GetLandmarkType();
    switch (landmarkType) {
        case LandmarkBanner: return @"banner";
        case LandmarkComplementary: return @"complementary";
        case LandmarkContentInfo: return @"content";
        case LandmarkRegion: return @"region";
        case LandmarkForm: return @"form";
        case LandmarkMain: return @"main";
        case LandmarkNavigation: return @"navigation";
        case LandmarkSearch: return @"search";
    }
    return NSAccessibilityRoleDescription([self accessibilityRole], [self accessibilitySubrole]);
}

// Note: Apple has deprecated this API, but it's still used to set attributes not supported by NSAccessibility
- (NSArray<NSString *> *)accessibilityAttributeNames
{
    if (_attributeNames == nil)
    {
        _attributeNames = @[
            @"AXARIALive", // kAXARIALiveAttribute
        ];
    }
    return _attributeNames;
}

- (id)accessibilityAttributeValue:(NSAccessibilityAttributeName)attribute
{
    if ([attribute isEqualToString:@"AXARIALive" /* kAXARIALiveAttribute */])
    {
        switch (_peer->GetLiveSetting())
        {
            case LiveSettingOff: return nil;
            case LiveSettingPolite: return @"polite";
            case LiveSettingAssertive: return @"assertive";
        }
        return nil;
    }
    return nil;
}

- (NSString *)accessibilityIdentifier
{
    return GetNSStringAndRelease(_peer->GetAutomationId());
}

- (NSString *)accessibilityTitle
{
    // StaticText exposes its text via the value property.
    if (_peer->GetAutomationControlType() != AutomationText)
    {
        return GetNSStringAndRelease(_peer->GetName());
    }
    
    return [super accessibilityTitle];
}

- (NSString *)accessibilityHelp
{
    return GetNSStringAndRelease(_peer->GetHelpText()); 
}

- (NSString *)accessibilityPlaceholderValue
{
    return GetNSStringAndRelease(_peer->GetPlaceholderText());
}

- (id)accessibilityValue
{
    if (_peer->IsRangeValueProvider())
    {
        return [NSNumber numberWithDouble:_peer->RangeValueProvider_GetValue()];
    }
    else if (_peer->IsToggleProvider())
    {
        switch (_peer->ToggleProvider_GetToggleState()) {
            case 0: return [NSNumber numberWithBool:NO];
            case 1: return [NSNumber numberWithBool:YES];
            default: return [NSNumber numberWithInt:2];
        }
    }
    else if (_peer->IsValueProvider())
    {
        if (_peer->TextProvider_HasRanges())
        {
            const auto span = _peer->TextProvider_GetDocumentSpan();
            return [self accessibilityStringForRange:ToNSRange(span)];
        }
        return GetNSStringAndRelease(_peer->ValueProvider_GetValue());
    }
    else if (_peer->GetAutomationControlType() == AutomationText)
    {
        return GetNSStringAndRelease(_peer->GetName());
    }
    else if (_peer->GetAutomationControlType() == AutomationHeader)
    {
        return [NSNumber numberWithInt:_peer->GetHeadingLevel()];
    }

    return [super accessibilityValue];
}

- (void)setAccessibilityValue:(id)newValue
{
    if (!_peer->IsEnabled())
        return;
    if (_peer->IsValueProvider() && !_peer->ValueProvider_IsReadOnly())
    {
        if (newValue == nil)
            _peer->ValueProvider_SetValue(nullptr, 0);
        else if ([newValue isKindOfClass:[NSString class]])
        {
            __attribute__((objc_precise_lifetime)) NSData* utf8 =
                [(NSString*)newValue dataUsingEncoding:NSUTF8StringEncoding];
            if (utf8 != nil)
                _peer->ValueProvider_SetValue(utf8.length != 0 ? utf8.bytes : "", utf8.length);
        }
    }
    else if (_peer->IsRangeValueProvider() && !_peer->RangeValueProvider_IsReadOnly())
    {
        if ([newValue isKindOfClass:[NSNumber class]])
            _peer->RangeValueProvider_SetValue([(NSNumber*)newValue doubleValue]);
    }
}

- (NSInteger)accessibilityNumberOfCharacters
{
    return ToNSRange(_peer->TextProvider_GetDocumentSpan()).length;
}

- (NSRange)accessibilitySelectedTextRange
{
    ComPtr<IFrnTextSpanArray> spans(_peer->TextProvider_GetSelection(), true);
    return spans.getRaw() ? ToNSRange(spans->GetPrimary()) : NSMakeRange(NSNotFound, 0);
}

- (NSArray<NSValue *> *)accessibilitySelectedTextRanges
{
    ComPtr<IFrnTextSpanArray> spans(_peer->TextProvider_GetSelection(), true);
    if (!spans.getRaw())
        return nil;

    NSMutableArray<NSValue *> *result = [NSMutableArray array];
    const auto count = spans->GetCount();
    for (uint32_t i = 0; i < count; ++i)
    {
        const auto range = ToNSRange(spans->Get(i));
        if (range.location != NSNotFound)
            [result addObject:[NSValue valueWithRange:range]];
    }
    if (result.count == 0)
    {
        const auto caret = ToNSRange(spans->GetPrimary());
        if (caret.location != NSNotFound)
            [result addObject:[NSValue valueWithRange:caret]];
    }
    return result;
}

- (void)setAccessibilitySelectedTextRange:(NSRange)range
{
    if (_peer->IsEnabled())
        _peer->TextProvider_Select(ToTextSpan(range));
}

- (NSString *)accessibilitySelectedText
{
    return [self accessibilityStringForRange:[self accessibilitySelectedTextRange]];
}

- (NSRange)accessibilityVisibleCharacterRange
{
    return ToNSRange(_peer->TextProvider_GetVisibleSpan());
}

- (NSString *)accessibilityStringForRange:(NSRange)range
{
    return TextFromUtf8(_peer->TextProvider_GetString(ToTextSpan(range)));
}

- (NSAttributedString *)accessibilityAttributedStringForRange:(NSRange)range
{
    ComPtr<IFrnStyledText> styled(_peer->TextProvider_GetStyledText(ToTextSpan(range)), true);
    if (!styled.getRaw())
        return nil;
    NSString* content = TextFromUtf8(styled->GetText());
    if (content == nil || content.length != range.length)
        return nil;

    auto text = [[NSMutableAttributedString alloc] initWithString:content];
    for (unsigned int index = 0, count = styled->GetRunCount(); index < count; ++index)
    {
        const auto style = styled->GetStyle(index);
        const auto run = ToNSRange(style.Span);
        if (run.location == NSNotFound || NSMaxRange(run) > content.length)
            return nil;
        AddAccessibleTextStyle(text, run, style, TextFromUtf8(styled->GetFontName(index)));
    }
    return text;
}

- (NSRange)accessibilityStyleRangeForIndex:(NSInteger)index
{
    return ToNSRange(_peer->TextProvider_GetStyleSpan(index));
}

- (NSRect)accessibilityFrameForRange:(NSRange)range
{
    const auto bounds = _peer->TextProvider_GetBounds(ToTextSpan(range));
    return bounds.Width == 0 && bounds.Height == 0 ? NSZeroRect : [self rectToScreen:bounds];
}

- (NSRange)accessibilityRangeForPosition:(NSPoint)point
{
    const auto topLevel = [self accessibilityTopLevelUIElement];
    if (![topLevel isKindOfClass:[FrnWindow class]])
        return NSMakeRange(NSNotFound, 0);

    auto window = (FrnWindow*)topLevel;
    auto view = [window view];
    if (view == nil)
        return NSMakeRange(NSNotFound, 0);

    const auto windowPoint = [window convertPointFromScreen:point];
    const auto localPoint = [view convertPoint:windowPoint fromView:nil];
    return ToNSRange(_peer->TextProvider_GetSpanFromPoint({localPoint.x, localPoint.y}));
}

- (NSRange)accessibilityRangeForIndex:(NSInteger)index
{
    return ToNSRange(_peer->TextProvider_GetCharacterSpan(index));
}

- (NSInteger)accessibilityLineForIndex:(NSInteger)index
{
    const auto line = _peer->TextProvider_GetLineForIndex(index);
    return line < 0 ? NSNotFound : line;
}

- (NSRange)accessibilityRangeForLine:(NSInteger)line
{
    return ToNSRange(_peer->TextProvider_GetRangeForLine(line));
}

- (NSInteger)accessibilityInsertionPointLineNumber
{
    const auto line = _peer->TextProvider_GetInsertionPointLine();
    return line < 0 ? NSNotFound : line;
}

- (BOOL)accessibilityIsAttributeSettable:(NSAccessibilityAttributeName)attribute
{
    if ([attribute isEqualToString:NSAccessibilityValueAttribute])
    {
        if (_peer->IsValueProvider())
            return !_peer->ValueProvider_IsReadOnly();
        if (_peer->IsRangeValueProvider())
            return !_peer->RangeValueProvider_IsReadOnly();
        return NO;
    }

    if ([attribute isEqualToString:NSAccessibilitySelectedTextRangeAttribute])
        return _peer->IsEnabled() && _peer->TextProvider_HasRanges() && _peer->TextProvider_CanSelect();

    return [super accessibilityIsAttributeSettable:attribute];
}

- (id)accessibilityMinValue
{
    if (_peer->IsRangeValueProvider())
    {
        return [NSNumber numberWithDouble:_peer->RangeValueProvider_GetMinimum()];
    }
    
    return [super accessibilityMinValue];
}

- (id)accessibilityMaxValue
{
    if (_peer->IsRangeValueProvider())
    {
        return [NSNumber numberWithDouble:_peer->RangeValueProvider_GetMaximum()];
    }
    
    return [super accessibilityMaxValue];
}

- (BOOL)isAccessibilityEnabled
{
    return _peer->IsEnabled();
}

- (BOOL)isAccessibilityFocused
{
    return _peer->HasKeyboardFocus();
}

- (NSArray *)accessibilityChildren
{
    if (_children == nullptr && _peer != nullptr)
        [self recalculateChildren];
    return _children;
}

- (NSRect)accessibilityFrame
{
    auto bounds = _peer->GetBoundingRectangle();
    return [self rectToScreen:bounds];
}

- (id)accessibilityParent
{
    auto parentPeer = _peer->GetParent();

    if (parentPeer == nullptr)
        return [NSApplication sharedApplication];

    // When the parent is a root provider, return the FrnView (content view)
    // rather than the FrnWindow. macOS accessibility requires that the parent
    // chain is consistent with the children chain: FrnView exposes these
    // elements as its accessibilityChildren, so the elements must report
    // FrnView as their accessibilityParent.  A mismatch causes macOS to be
    // unable to resolve AXUIElementRefs back to the correct object, which
    // makes setter calls like AXUIElementSetAttributeValue silently land on
    // FrnView instead of the target FrnAccessibilityElement.
    if (parentPeer->IsRootProvider())
    {
        auto window = parentPeer->RootProvider_GetWindow();
        if (window != nullptr)
        {
            auto holder = dynamic_cast<INSViewHolder*>(window);
            if (holder != nullptr)
                return holder->GetNSView();
        }
    }

    return [FrnAccessibilityElement acquire:parentPeer];
}

- (id)accessibilityTopLevelUIElement
{
    auto rootPeer = _peer->GetRootPeer();
    return [FrnAccessibilityElement acquire:rootPeer];
}

- (id)accessibilityWindow
{
    auto rootPeer = _peer->GetVisualRoot();
    return [FrnAccessibilityElement acquire:rootPeer];
}

- (id)accessibilityHorizontalScrollBar
{
    if (_peer == nullptr)
        return nil;
    return [FrnAccessibilityElement acquire:_peer->ScrollProvider_GetHorizontalScrollBar()];
}

- (id)accessibilityVerticalScrollBar
{
    if (_peer == nullptr)
        return nil;
    return [FrnAccessibilityElement acquire:_peer->ScrollProvider_GetVerticalScrollBar()];
}

- (BOOL)isAccessibilityExpanded
{
    if (!_peer->IsExpandCollapseProvider())
        return NO;
    return _peer->ExpandCollapseProvider_GetIsExpanded();
}

- (void)setAccessibilityExpanded:(BOOL)accessibilityExpanded
{
    if (!_peer->IsExpandCollapseProvider() || !_peer->IsEnabled())
        return;
    if (accessibilityExpanded)
        _peer->ExpandCollapseProvider_Expand();
    else
        _peer->ExpandCollapseProvider_Collapse();
}

- (BOOL)accessibilityPerformPress
{
    if (!_peer->IsEnabled())
        return NO;
    if (_peer->IsInvokeProvider())
    {
        _peer->InvokeProvider_Invoke();
    }
    else if (_peer->IsExpandCollapseProvider())
    {
        if (_peer->ExpandCollapseProvider_GetIsExpanded())
            _peer->ExpandCollapseProvider_Collapse();
        else
            _peer->ExpandCollapseProvider_Expand();
    }
    else if (_peer->IsToggleProvider())
    {
        _peer->ToggleProvider_Toggle();
    }
    return YES;
}

- (BOOL)accessibilityPerformIncrement
{
    if (!_peer->IsRangeValueProvider() || _peer->RangeValueProvider_IsReadOnly() || !_peer->IsEnabled())
        return NO;
    auto value = _peer->RangeValueProvider_GetValue();
    value += _peer->RangeValueProvider_GetSmallChange();
    _peer->RangeValueProvider_SetValue(value);
    return YES;
}

- (BOOL)accessibilityPerformDecrement
{
    if (!_peer->IsRangeValueProvider() || _peer->RangeValueProvider_IsReadOnly() || !_peer->IsEnabled())
        return NO;
    auto value = _peer->RangeValueProvider_GetValue();
    value -= _peer->RangeValueProvider_GetSmallChange();
    _peer->RangeValueProvider_SetValue(value);
    return YES;
}

- (BOOL)accessibilityPerformShowMenu
{
    if (!_peer->IsExpandCollapseProvider() || !_peer->IsEnabled())
        return NO;
    _peer->ExpandCollapseProvider_Expand();
    return YES;
}

- (NSArray<NSAccessibilityActionName> *)accessibilityActionNames
{
    NSAccessibilityActionName scrollToVisible = @"AXScrollToVisible";
    NSArray<NSAccessibilityActionName> *base = [super accessibilityActionNames];
    if (base == nil)
        base = @[];
    if ([base containsObject:scrollToVisible])
        return base;
    return [base arrayByAddingObject:scrollToVisible];
}

- (void)accessibilityPerformAction:(NSAccessibilityActionName)action
{
    if ([action isEqualToString:@"AXScrollToVisible"])
        _peer->BringIntoView();
    else
        [super accessibilityPerformAction:action];
}

- (BOOL)isAccessibilitySelected
{
    if (_peer->IsSelectionItemProvider())
        return _peer->SelectionItemProvider_IsSelected();
    return NO;
}

- (void)setAccessibilitySelected:(BOOL)accessibilitySelected
{
    if (!_peer->IsSelectionItemProvider() || !_peer->IsEnabled())
        return;
    if (accessibilitySelected)
        _peer->SelectionItemProvider_AddToSelection();
    else
        _peer->SelectionItemProvider_RemoveFromSelection();
}

- (BOOL)isAccessibilitySelectorAllowed:(SEL)selector
{
    if (selector == @selector(setAccessibilitySelectedTextRange:))
        return _peer->IsEnabled() && _peer->TextProvider_HasRanges() && _peer->TextProvider_CanSelect();

    if (selector == @selector(accessibilityNumberOfCharacters) ||
        selector == @selector(accessibilitySelectedText) ||
        selector == @selector(accessibilitySelectedTextRange) ||
        selector == @selector(accessibilitySelectedTextRanges) ||
        selector == @selector(accessibilityVisibleCharacterRange) ||
        selector == @selector(accessibilityStringForRange:) ||
        selector == @selector(accessibilityFrameForRange:) ||
        selector == @selector(accessibilityRangeForIndex:) ||
        selector == @selector(accessibilityRangeForPosition:))
        return _peer->TextProvider_HasRanges();

    if (selector == @selector(accessibilityLineForIndex:) ||
        selector == @selector(accessibilityRangeForLine:) ||
        selector == @selector(accessibilityInsertionPointLineNumber))
        return _peer->TextProvider_HasLogicalLines();

    if (selector == @selector(accessibilityAttributedStringForRange:) ||
        selector == @selector(accessibilityStyleRangeForIndex:))
        return _peer->TextProvider_HasStyles();

    if (selector == @selector(setAccessibilityValue:))
    {
        return (_peer->IsValueProvider() && !_peer->ValueProvider_IsReadOnly()) ||
               (_peer->IsRangeValueProvider() && !_peer->RangeValueProvider_IsReadOnly());
    }
    else if (selector == @selector(accessibilityPerformShowMenu))
    {
        return _peer->IsExpandCollapseProvider() && _peer->ExpandCollapseProvider_GetShowsMenu();
    }
    else if (selector == @selector(isAccessibilityExpanded))
    {
        return _peer->IsExpandCollapseProvider();
    }
    else if (selector == @selector(accessibilityPerformPress))
    {
        return _peer->IsInvokeProvider() || _peer->IsExpandCollapseProvider() || _peer->IsToggleProvider();
    }
    else if (selector == @selector(setAccessibilitySelected:))
    {
        return _peer->IsSelectionItemProvider();
    }
    else if (selector == @selector(accessibilityPerformIncrement) ||
             selector == @selector(accessibilityPerformDecrement))
    {
        return _peer->IsRangeValueProvider() && !_peer->RangeValueProvider_IsReadOnly();
    }
    else if (selector == @selector(accessibilityMinValue) ||
             selector == @selector(accessibilityMaxValue))
    {
        return _peer->IsRangeValueProvider();
    }

    return [super isAccessibilitySelectorAllowed:selector];
}

- (NSRect)rectToScreen:(FrnRect)rect
{
    id topLevel = [self accessibilityTopLevelUIElement];

    if (![topLevel isKindOfClass:[FrnWindow class]])
        return NSZeroRect;

    auto window = (FrnWindow*)topLevel;
    auto view = [window view];

    if (view == nil)
        return NSZeroRect;

    auto nsRect = ToNSRect(rect);
    auto windowRect = [view convertRect:nsRect toView:nil];
    return [window convertRectToScreen:windowRect];
}

- (void)raiseChildrenChanged
{
    auto changed = _children ? [NSMutableSet setWithArray:_children] : [NSMutableSet set];

    [self recalculateChildren];
    
    if (_children)
        [changed addObjectsFromArray:_children];

	/*
	For future reference, upon testing with a sample SwiftUI app:

    1) Containers vanish. VStack/HStack don't appear in the accessibility tree at all, 
      only real elements (Text, Button, List) do, parented to one root AXHostingView.

    2) Changes post on the nearest real element, not the container. 
       Toggling a child two VStack/HStack levels deep fired AXLayoutChanged on the AXHostingView 
       (the nearest real element), never on the hidden containers.

    3) Real controls get their own notification. The List (AXOutline) posted AXRowCountChanged on itself.

	Apple never posts a structural notification on a non-accessibility container.
    It targets the nearest element that's actually in the tree.
    That's exactly what the code below does (templated parent, else walk to the nearest exposed ancestor).
	*/

    id target = [FrnAccessibilityElement acquire:_peer->GetTemplatedParent()];
    if (target == nil)
        target = self;
    while ([target isKindOfClass:[FrnAccessibilityElement class]] && ![(FrnAccessibilityElement*)target isAccessibilityElement])
        target = [(FrnAccessibilityElement*)target accessibilityParent];

    NSAccessibilityPostNotificationWithUserInfo(
        target,
        NSAccessibilityLayoutChangedNotification,
        @{ NSAccessibilityUIElementsKey: [changed allObjects]});
}

- (void)raisePropertyChanged:(FrnAutomationProperty)property
{
    switch (property)
    {
        case AutomationPeer_AutomationId:
            // accessibilityIdentifier is read on-demand; no VoiceOver announcement required.
            break;
        case AutomationPeer_Name:
            NSAccessibilityPostNotification(self, NSAccessibilityTitleChangedNotification);
            if (_peer->GetLiveSetting() != LiveSettingOff)
                [self raiseLiveRegionChanged];
            break;
        case ValueProvider_Value:
        case RangeValueProvider_Value:
            NSAccessibilityPostNotification(self, NSAccessibilityValueChangedNotification);
            break;
        case AutomationPeer_BoundingRectangle:
            NSAccessibilityPostNotification(self, NSAccessibilityMovedNotification);
            NSAccessibilityPostNotification(self, NSAccessibilityResizedNotification);
            break;
        case SelectionItemProvider_IsSelected:
        case SelectionProvider_Selection:
            NSAccessibilityPostNotification(self, NSAccessibilitySelectedChildrenChangedNotification);
            break;
        case TextProvider_Selection:
            NSAccessibilityPostNotification(self, NSAccessibilitySelectedTextChangedNotification);
            break;
        case ToggleProvider_ToggleState:
            NSAccessibilityPostNotification(self, NSAccessibilityValueChangedNotification);
            break;
        case ExpandCollapseProvider_ExpandCollapseState:
            NSAccessibilityPostNotification(self, NSAccessibilityValueChangedNotification);
            if (_peer->ExpandCollapseProvider_GetIsExpanded())
                NSAccessibilityPostNotification(self, (__bridge NSString *)kAXRowExpandedNotification);
            else
                NSAccessibilityPostNotification(self, (__bridge NSString *)kAXRowCollapsedNotification);
            break;
        default:
            break;
    }
}

- (void)raiseLiveRegionChanged
{
    NSAccessibilityPostNotification(self, @"AXLiveRegionChanged" /* kAXLiveRegionChangedNotification */);

    // Announce the new string
    auto name = _peer->GetName();
    if (name != nullptr)
    {
        NSAccessibilityPriorityLevel priority = NSAccessibilityPriorityLow;
        switch (_peer->GetLiveSetting())
        {
            case LiveSettingOff:
                return;
            case LiveSettingPolite:
                priority = NSAccessibilityPriorityMedium;
                break;
            case LiveSettingAssertive:
                priority = NSAccessibilityPriorityHigh;
                break;
        }

        NSDictionary <NSString *, id> *userInfo = @{
            NSAccessibilityAnnouncementKey: GetNSStringAndRelease(name),
            NSAccessibilityPriorityKey: @(priority)
        };

        id topLevel = [self accessibilityTopLevelUIElement];
        if ([topLevel isKindOfClass:[FrnWindow class]])
        {
            NSAccessibilityPostNotificationWithUserInfo(topLevel, NSAccessibilityAnnouncementRequestedNotification, userInfo);
        }
    }
}

- (void)setAccessibilityFocused:(BOOL)accessibilityFocused
{
    if (accessibilityFocused)
        _peer->SetFocus();
}

- (void)recalculateChildren
{
    auto childPeers = _peer->GetChildren();
    auto childCount = childPeers != nullptr ? childPeers->GetCount() : 0;

    if (childCount > 0)
    {
        _children = [[NSMutableArray alloc] initWithCapacity:childCount];
        
        for (int i = 0; i < childCount; ++i)
        {
            IFrnAutomationPeer* child;
            
            if (childPeers->Get(i, &child) == S_OK)
            {
                id element = [FrnAccessibilityElement acquire:child];
                [_children addObject:element];
            }
        }
    }
    else
    {
        _children = nil;
    }
}

@end

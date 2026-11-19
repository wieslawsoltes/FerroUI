//! Reviewed native adaptations. Original snippets remain in the catalog; these
//! markup documents state the replacement API and their behavior in Rust.

use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::Color;
use ferroui_base::{ObjectType, Ref, Thickness};
use ferroui_controls::primitives::ToggleButton;
use ferroui_controls::{Border, Button, Control, RadioButtons, TextBlock, TextBox};
use ferroui_markup_xaml::markup_extensions::XBindings;
use ferroui_markup_xaml::XamlLoadError;
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;
use std::cell::Cell;
use std::rc::Rc;

#[derive(Clone, Copy, Debug)]
pub struct PageAdaptation {
    pub page_id: &'static str,
    pub xaml: &'static str,
    pub notes: &'static str,
    /// Whether the page is `ported` in `port-status.json`: every catalog example of the page has
    /// a native adaptation with its events and bindings, or a recorded reason why it cannot.
    /// Otherwise the page is `partial`.
    pub ported: bool,
}

macro_rules! page {
    ($id:literal, $notes:literal) => {
        PageAdaptation {
            page_id: $id,
            xaml: include_str!(concat!("Pages/", $id, ".xaml")),
            notes: $notes,
            ported: false,
        }
    };
    (ported $id:literal, $notes:literal) => {
        PageAdaptation {
            page_id: $id,
            xaml: include_str!(concat!("Pages/", $id, ".xaml")),
            notes: $notes,
            ported: true,
        }
    };
}

pub static ADAPTATIONS: &[PageAdaptation] = &[
    page!(ported "MediaPlayerElement", "Both compositions use the existing portable MediaPlayerElement. The shared bundled H.264/AAC Sintel excerpt replaces unbundled WMV assets. Open File uses the host storage picker; AutoPlay, poster, stretch, transport options and full-window use native controls. Platform acceptance is recorded separately in docs/media/ISSUE-561.md."),
    page!(ported "CaptureElementPreview", "Explicit camera start/stop uses CameraPreview and the existing native camera providers, independently of video playback. CameraCaptureUI demonstrates owned photos and optional native silent video recording. Record is enabled only when the provider advertises it; no microphone is requested."),
    page!(ported "Sound", "The two-dimensional sounds are short application-owned PCM cues played through the portable native MediaPlayer, with sound/volume controls and seven named sound actions. WinUI ElementSoundPlayer.SpatialAudioMode is Windows-specific and unavailable on the portable provider contract; the spatial example is explicitly not applicable and disabled."),
    page!("Image", "Original bitmaps, four stretch modes, decode-to-height and both NineGrid examples use native Image, Bitmap, CroppedBitmap and DrawingImage. SVG decoding and animated GIF playback remain explicit unsupported examples with visible diagnostics; exact native WinUI decoding and pixel parity remain pending."),
    page!("FlipView", "All three source compositions use native Carousel, PageSlide and PipsPager with original images, labels, horizontal/vertical layouts and the first three source data groups. WinUI container APIs, flip chrome, XY-focus, RTL and exact transition parity remain pending."),
    page!("ConnectedAnimation", "All four source compositions use the native ConnectedAnimationService with Frame/FramePage navigation, original keys, configuration choices and source image assets. ItemsControl/WrapPanel compositions explicitly adapt ListView/ItemsRepeater containers; virtualization, XY-focus, entrance transitions and exact rendering acceptance remain pending."),
    page!("ColorPicker", "The original options use native ColorView, ColorSlider and explicit text/More controls. WinUI's inline ColorPicker maps to ColorView while FerroUI ColorPicker retains its dropdown API. Color, alpha, input visibility and Box/Ring handlers are preserved; exact source layout, localization and host input/accessibility acceptance remain pending."),
    page!("CompactSizing", "The source standard/compact page navigation copies all four text/password values and the selected date. The existing native compact dictionary is scoped to each compact page; global theme density remains unchanged. Header and PasswordBox are explicit compositions/adaptations; full WinUI control contracts and input/visual acceptance remain pending."),
    page!("InfoBadge", "All four source compositions use native InfoBadge: NavigationView item badges, severity styles, an embedded button badge and dynamic NumberBox values. Visibility and pane options retain the original handlers. Exact upstream rendering and host accessibility acceptance remain pending."),
    page!("NavigationView", "All eight source compositions use native NavigationView, items, hierarchy, footer, settings and Frame navigation. Model templates and C# handlers are explicitly adapted to Rust. Top overflow, selectors, transition effects and platform visual/accessibility acceptance remain pending."),
    page!("ProgressRing", "Both source examples use the native ProgressRing, active state, background options and original NumberBox empty-value recovery. Native arc animation is an explicit adaptation; exact Lottie motion, custom animated sources and platform accessibility acceptance remain pending."),
    page!("RatingControl", "Both source examples use native RatingControl input, caption changes, clearing/read-only options and half-step placeholder binding. Programmatic updates retain the original caption; user ValueChanged selects Your rating. Exact source animation and host accessibility acceptance remain pending."),
    page!(ported "InfoBar", "All three source examples use native InfoBar with the code behind of InfoBarPage.xaml.cs in Rust (control_samples.rs): SeverityComboBox_SelectionChanged (Informational, Success, Warning, Error), MessageComboBox_SelectionChanged (the short and the long message, long at load) and ActionButtonComboBox_SelectionChanged (none, the Action button, the Informational link to the Microsoft home page). The Is Open, Is Icon Visible and Is Closable options keep the source's TwoWay x:Bind, compiled by the x:Bind core. The ComboBoxes' Headers are composed; the DisplayMessage and DisplayButton values feed the source viewer's substitutions only."),
    page!(ported "Popup", "The source example uses native Popup with ShowPopupOffsetClicked, ClosePopupClicked and PopupClosed of PopupPage.xaml.cs in Rust (the light-dismiss option is disabled while the popup is open). The offsets and IsLightDismissEnabled keep the source's OneWay x:Bind of the NumberBoxes and the ToggleSwitch (IsOn is IsChecked, the Header its Content). As WinUI's Popup without a placement target, the popup's top left corner is at its parent's, moved by the offsets (AnchorAndGravity, TopLeft, BottomRight)."),
    page!("Clipboard", "Text copy/paste, available formats and clear use the host clipboard service asynchronously. Bitmap/file operations, history/roaming, content-change notifications and original accessibility announcements remain pending."),
    page!("PullToRefresh", "The source RefreshContainer request/deferral behavior inserts NewControl on the UI dispatcher. ListView is explicitly adapted to ListBox; the custom visualizer and full gesture/animation parity remain pending."),
    page!("PageTransition", "The original alternating sample-page layouts use the native Frame/FramePage types, back/forward journals and Rust handlers. Navigation transition effects remain pending."),
    page!("AppBarButton", "All six source examples use native CommandBarButton (WinUI's AppBarButton) with AppBarButton_Click of AppBarButtonPage.xaml.cs in Rust: the symbol, font and path icons (the source's Content of the path icon is the Icon, which the template presents), the KeyboardAccelerator Ctrl+S as the HotKey, and the flyout with its text box; the outputs are polite live regions for UIHelper.AnnounceActionForAccessibility. FerroUI has no BitmapIcon: the bitmap icon is an Image of the source's Slices2.png, not drawn in the foreground's single color, so the page is partial. The symbol glyphs need the platform's symbol font."),
    page!("AppBarToggleButton", "All four source examples use native CommandBarToggleButton (WinUI's AppBarToggleButton) with AppBarButton_Click of AppBarToggleButtonPage.xaml.cs in Rust (IsChecked = True, False or empty for the third state, as C#'s bool?.ToString()): the symbol, font and three-state path icons (the source's Content of the path icon is the Icon). FerroUI has no BitmapIcon: the bitmap icon is an Image of the source's Slices2.png, not drawn in the foreground's single color, so the page is partial. The symbol glyph needs the platform's symbol font."),
    page!(ported "AppBarSeparator", "The source example uses native CommandBarSeparator (WinUI's AppBarSeparator) between the source's CommandBarButtons with their symbol icons in a CommandBar, in a scroll viewer that scrolls horizontally without a bar and not vertically (the scroll modes are the bar visibilities Hidden and Disabled). The page has no code-behind upstream. The symbol glyphs need the platform's symbol font."),
    page!(ported "CommandBar", "The source example uses native CommandBar with the code behind of CommandBarPage.xaml.cs in Rust: OnElementClicked (the clicked command's label), Open and Close (IsOpen with IsSticky), AddSecondaryCommands_Click (four buttons and a separator while Settings is alone) and RemoveSecondaryCommands, and the KeyboardAccelerators of the constructor and the markup as HotKeys (Ctrl+A, Ctrl+E, F4, Ctrl+I; Ctrl+N, Delete, Ctrl+Subtract, Ctrl+Add). The MultipleButtons property feeds the source viewer's substitutions only, and the gallery creates the page anew instead of OnNavigatingFrom. FerroUI's CommandBar has no closed display mode: its buttons keep their labels while it is closed. The symbol glyphs need the platform's symbol font."),
    page!("AccessibilityColorContrast", "The original luminance/contrast calculation and pass thresholds run in Rust. Opaque RGB text inputs replace the Gallery's custom inline color pickers; those picker controls remain pending."),
    page!(ported "AutoSuggestBox", "Both source examples use native AutoSuggestBox with the handlers of AutoSuggestBoxPage.xaml.cs in Rust (control_samples.rs): AutoSuggestBox_TextChanged filters the source's 97 cats by every word of the text a user types (No results found when none matches) and AutoSuggestBox_SuggestionChosen writes the chosen cat; Control2_TextChanged, Control2_SuggestionChosen and Control2_QuerySubmitted search the gallery's controls by every word of their title, prefix matches first, and show the chosen or the first control's image, title and subtitle. The suggestions are the controls' titles (the ToString of ControlInfoDataItem), mapped back to their catalog entries, and case is ignored by Unicode lowercase instead of CurrentCultureIgnoreCase. The search box's UpdateTextOnSelect is False: as in the source, only Control2_SuggestionChosen puts a control's title in the box, so a chosen No results found keeps the text. The output is the gallery's OutputTextBlockStyle, the title its BaseTextBlockStyle (semibold) and WrapWholeWords is WrapWithOverflow."),
    page!(ported "NumberBox", "All three source examples use native NumberBox with the code behind of NumberBoxPage.xaml.cs in Rust: the expression box, the spin button box with SpinButtonPlacementGroup_SelectionChanged (Inline, Compact; as in the source the box keeps its markup placement until the selection changes), and SetNumberBoxNumberFormatter, a formatter of two fraction digits that rounds half up to 0.25 (DecimalFormatter with an IncrementNumberRounder). The formatter writes and reads the invariant decimal separator instead of the region's."),
    page!(ported "PersonPicture", "The source example uses native PersonPicture (WinUI's control in ferroui-controls) with its RadioButtons handler: the profile image, the display name Jane Doe and the initials SB. The profile image is the gallery's PersonPicture image instead of the remote image of the source."),
    page!(ported "PagerControl", "All three source examples use native PagerControl (WinUI's control in ferroui-controls): the basic pager, the number box and button panel display modes, and unbounded number box and button panel pagers. The page has no code-behind upstream."),
    page!("PipsPager", "Both source examples use native PipsPager with the code behind of PipsPagerPage.xaml.cs in Rust: the pager of the source's eight Pictures, whose SelectedPageIndex keeps the source's TwoWay x:Bind of the gallery's SelectedIndex, and the pager whose TestPipsPager2_SelectedIndexChanged announces the page (a polite live region) with the orientation and button options. FerroUI has no FlipView: the gallery is a Carousel, as on the FlipView page (partial). FerroUI's PipsPager has IsPreviousButtonVisible and IsNextButtonVisible instead of PreviousButtonVisibility and NextButtonVisibility (no VisibleOnPointerOver) and no WrapMode, so those options are left out and the page is partial."),
    page!("TabView", "Native TabControl displays tab content; WinUI add/close/drag contracts remain pending."),
    page!("Pivot", "Native TabControl adapts the basic Pivot sample; Pivot-specific input and animation remain pending."),
    page!(ported "RadialGradientBrush", "The source example uses native RadialGradientBrush (yellow to blue) as the fill of the rectangle, with the code behind of RadialGradientBrushPage.xaml.cs in Rust (brush_samples.rs): OnPageLoaded with InitializeSliders (the ranges, values, steps and small changes of the six sliders for the mapping mode, in pixels of the rectangle for Absolute, fractions of it otherwise), OnMappingModeChanged, OnSpreadMethodChanged (Pad, Reflect, Repeat) and OnSliderValueChanged (Center, RadiusX, RadiusY and GradientOrigin). FerroUI's RadialGradientBrush has no MappingMode: each point and radius has a unit, Relative for RelativeToBoundingBox and Absolute for Absolute, which the handlers set. FerroUI's Slider has no Header (composed) and no StepFrequency: the steps are TickFrequency with IsSnapToTickEnabled. SelectedValue is the index of the source's value."),
    page!("Geometry", "The source example with the code behind of GeometryPage.xaml.cs in Rust (shape_samples.rs): the header image of the page's theme (the source's Geometry.dark.png for the Default theme and Geometry.light.png for Light, its ThemeDictionaries), the three tip buttons of ShowGeometryButtonClick1 to 3, and CopyOverlayResourceToClipboardButton_Click and CopyControlResourceToClipboardButton_Click, which put the resource's name on the window's clipboard (the gallery's CopyButton is a Button with its glyph, its feedback a polite live region). The corner radius table keeps the source's rows; the radii are their values (8 and 4), CaptionTextBlockStyle is 12, the card styles are Borders with the card brushes and the selectable style names are SelectableTextBlocks (no IsTextSelectionEnabled on TextBlock). FerroUI has no TeachingTip: each tip is the Flyout of its button with the tip's title and subtitle, which the click opens and closes, so the page is partial. The glyphs need the platform's symbol font."),
    page!("IconElement", "All six source examples with MonochromeButton_CheckedChanged of IconElementPage.xaml.cs in Rust (brush_samples.rs): the FontIcon with the source's font family and glyph, the PathIcon and the SymbolIcon in their buttons use the native icons. FerroUI has no BitmapIcon or ImageIcon: the bitmap icon is an Image of the source's Slices.png whose ShowAsMonochrome is a rectangle of the foreground brush masked by the bitmap (the check box toggles it), and the PNG image icon is an Image in its button. FerroUI decodes no SVG and the remote SVG of the source is not loaded: its button says so. Those substitutes make the page partial. The glyphs need the source's icon fonts."),
    page!("Color", "Native brushes display the color swatches; the full upstream color tools are not claimed."),
    page!(ported "Spacing", "The source page with its text, the two layout images of the theme (the source's Cards and Dialog PNGs of the Default and Light ThemeDictionaries, which the gallery's SampleThemeListener follows, set in Rust by brush_samples.rs) and the table of the seven spacing values with their bars and usages. The page has no code-behind upstream and no x:Bind (the earlier slider adaptation is gone). The keyed styles are style classes, the card and tile styles Borders with the card brushes, ScrollView is a ScrollViewer, ControlCornerRadius is 4, CaptionTextBlockStyle 12 and SubtitleTextBlockStyle 20 semibold; HorizontalScrollMode is the bar's visibility."),
    page!("XamlStyles", "The explicit WinUI style example is adapted to an opt-in ControlTheme; implicit WinUI style semantics are not changed globally."),
    page!(ported "Templates", "All four source examples with the code behind of TemplatesPage.xaml.cs in Rust (binding_samples.rs): the TextBox's ControlTemplate with the source's header, bordered box and Edit symbol (FerroUI's TextBox has no Header, so the template's header is the box's Tag, and the source's ContentElement is the PART_ScrollViewer with the PART_TextPresenter of FerroUI's TextBox template), the ComboBox's ItemTemplate DataTemplate, the constructor's DataTemplate created in code (FuncDataTemplate: the E734 glyph and the item's text by a Binding), and LayoutSelector_SelectionChanged between the page's two ItemsPanelTemplates (styles of the list that a class switches). FerroUI has no ListView or WrapGrid: the list is a ListBox and WrapGrid a WrapPanel. The Headers of the ComboBoxes are composed; ScrollView is a ScrollViewer. The glyphs need the platform's symbol font."),
    page!("CustomUserControls", "A native UserControl hosts the counter behavior; the original additional custom controls remain pending."),
    page!("AccessibilityScreenReader", "Native automation names and decorative accessibility views are assigned; screen-reader execution remains unverified."),
    page!(ported "Button", "All four source examples use native Button with Button_Click of ButtonPage.xaml.cs in Rust: the text button and its Disable option (the source's OneWay x:Bind of IsChecked, negated by BoolConverters.Not instead of IsChecked.Value.Equals(x:False)), the graphical button with the source's Slices.png, the WinUI theme's AccentButtonStyle and SubtleButtonStyle, and the wrapping layouts (WrapWholeWords is WrapWithOverflow)."),
    page!(ported "RepeatButton", "The source example uses native RepeatButton with RepeatButton_Click of RepeatButtonPage.xaml.cs in Rust: the static click count and the polite live-region output. The Disable option is the source's OneWay x:Bind of IsChecked, negated by BoolConverters.Not instead of IsChecked.Value.Equals(x:False)."),
    page!(ported "ToggleButton", "The source example uses native ToggleButton with the constructor and the Checked/Unchecked handlers of ToggleButtonPage.xaml.cs in Rust (On and Off, over IsCheckedChanged). The Disable option is the source's OneWay x:Bind of IsChecked, negated by BoolConverters.Not instead of IsChecked.Value.Equals(x:False)."),
    page!(ported "ToggleSwitch", "Both source examples use native ToggleSwitch: the simple switch, and the switch with its header, OnContent and OffContent and the ProgressRing bound to it with the source's OneWay x:Bind. WinUI's IsOn is IsChecked and its Header is the Content of FerroUI's ToggleSwitch, which the WinUI theme presents as the header. The page has no code-behind upstream."),
    page!(ported "CheckBox", "All three source examples use native CheckBox with the handlers of CheckBoxPage.xaml.cs in Rust: the two-state and three-state outputs, and Select all with SetCheckedState (the aggregate of the options at load and after each option) and the user cycle from checked to unchecked. The source's automation names and output automation ids are kept; rendering parity against Windows remains pending."),
    page!(ported "RadioButton", "Both source examples use native RadioButtons with the handlers of RadioButtonPage.xaml.cs in Rust: RadioButton_Checked writes the selected option, and the Background and Border groups color the border (Green, Yellow, White; DarkGreen, Gold, White). As in the source, the border keeps its markup colors until a selection changes. Rendering and screen-reader parity remain pending."),
    page!("TextBox", "All four source examples use native TextBox: the simple box, the box with its header (composed above the box, whose automation name it is) and placeholder, the read-only box with the source's font, foreground and character spacing (CharacterSpacing 200, thousandths of an em, is a LetterSpacing of 4.8 at 24), and the multi-line box with AcceptsReturn, wrapping and the green SelectionBrush (SelectionHighlightColor). The page has no code-behind upstream. FerroUI's TextBox has no IsSpellCheckEnabled, so the multi-line box does not check spelling and the page is partial."),
    page!("PasswordBox", "All three source examples with PasswordBox_PasswordChanged and RevealModeCheckbox_Changed of PasswordBoxPage.xaml.cs in Rust (input_samples.rs): the validation of an empty or 'Password' value, the header (composed), placeholder and custom character, and the reveal check box. FerroUI has no PasswordBox: each box is a TextBox with a PasswordChar whose Text, text changes and RevealPassword stand for Password, PasswordChanged and PasswordRevealMode, so the page is partial."),
    page!("TextBlock", "All five source examples: the simple text, the keyed style (a keyed ControlTheme set with Theme), the source's font, foreground, wrapping (WrapWholeWords is WrapWithOverflow) and character spacing (CharacterSpacing 200, thousandths of an em, is a LetterSpacing of 4.8 at 24), and the inline Run, LineBreak, Span, Bold, Italic and Underline. The page has no code-behind upstream. FerroUI's TextBlock has no IsTextSelectionEnabled or SelectionHighlightColor (SelectableTextBlock's selection cannot be switched off): the selectable text is a RichTextBlock, which has both, with the source's OneWay x:Bind of the switch, so the page is partial."),
    page!("Slider", "All four source examples use native Slider with the source's OneWay x:Bind of the outputs and of the range options, and SnapsToRadioButtons_SelectionChanged in Rust. FerroUI's Slider has no Header (composed), no StepFrequency (TickFrequency with IsSnapToTickEnabled) and no SnapsTo: Ticks is IsSnapToTickEnabled, and StepValues, the integer steps of the source, has no counterpart (the slider does not snap), so the page is partial. The source's Value.ToString() is the bound Value."),
    page!("ProgressBar", "Both source examples use native ProgressBar: the indeterminate bar, and the determinate bar set by the NumberBox with ProgressValue_ValueChanged of ProgressBarPage.xaml.cs in Rust (an empty box goes back to 0). FerroUI's ProgressBar has no ShowPaused or ShowError, so the indeterminate bar's Progress state options (Running, Paused, Error) and the source's x:Bind of IsChecked.Value are not ported and the page is partial."),
    page!("ComboBox", "All three source examples use native ComboBox with ColorComboBox_SelectionChanged and Combo3_Loaded of ComboBoxPage.xaml.cs in Rust: the color rectangle, the font list (FontHelper.Fonts as names; the output's font follows the selection instead of the x:Bind of FontItem.Font), and the editable font sizes with the source's OneWay x:Bind of SelectedValue to the output's font size. Header is composed. FerroUI's ComboBox has no TextSubmitted event and there is no ContentDialog, so Combo3_TextSubmitted (the check of a typed size and its dialog) is not ported and the page is partial."),
    page!(ported "Border", "The source example uses native Border with the handlers of BorderPage.xaml.cs in Rust: the thickness slider and the Background (Green, Yellow, Blue, White) and BorderBrush (DarkGreen, Gold, DarkBlue, White) options. FerroUI's Slider has no Header (composed) and no StepFrequency or SnapsTo: the steps of 1 are TickFrequency with IsSnapToTickEnabled. IsFocusEngagementEnabled is the gamepad focus engagement of Xbox, which FerroUI does not have."),
    page!(ported "Canvas", "The source example uses native Canvas: the red rectangle keeps the source's OneWay x:Bind of Canvas.Left, Canvas.Top and the z-index to the sliders (the binding converts the value instead of the source's (x:Int32) cast, which the x:Bind core makes a type test), Canvas.ZIndex is the shared ZIndex, and UpdateZSliderAutomationName of CanvasPage.xaml.cs names the z-index slider in Rust. FerroUI's Slider has no Header (composed) and no StepFrequency or SnapsTo: the steps of 1 are TickFrequency with IsSnapToTickEnabled."),
    page!(ported "Grid", "The source example uses native Grid with the source's OneWay x:Bind of ColumnSpacing, RowSpacing and the red rectangle's Grid.Column and Grid.Row to the sliders (the binding converts the value instead of the source's (x:Int32) cast, which the x:Bind core makes a type test). The page has no code-behind upstream. FerroUI's Slider has no Header (composed) and no StepFrequency or SnapsTo: the steps of 1 are TickFrequency with IsSnapToTickEnabled."),
    page!(ported "StackPanel", "The source example uses native StackPanel with the source's OneWay x:Bind of Spacing to the slider and OrientationGroup_SelectionChanged of StackPanelPage.xaml.cs in Rust (the Tag of the checked option is the orientation). FerroUI's Slider has no Header (composed) and no StepFrequency or SnapsTo: the steps of 1 are TickFrequency with IsSnapToTickEnabled."),
    page!(ported "RelativePanel", "The source example uses native RelativePanel with the source's relations: RightOf, AlignRightWithPanel, Below and AlignHorizontalCenterWith. The page has no code-behind upstream."),
    page!(ported "Expander", "Both source examples use native Expander: the text header and content with ExpandDirectionComboBox_SelectionChanged of ExpanderPage.xaml.cs in Rust (Down from the top, Up from the bottom), and the centered header over left-aligned content (HorizontalContentAlignment Left, Padding 0). The ComboBox's Header is composed."),
    page!("Viewbox", "The Viewbox scales native vector content; the upstream bitmap asset is a separate pending sample."),
    page!("WrapPanel", "The source wrapping layout uses shared WrapPanel; ItemsStretch and Padding require explicit owner adaptation."),
    page!(ported "ScrollViewer", "The source example uses native ScrollViewer with the source's cliff.jpg and the code behind of ScrollViewerPage.xaml.cs in Rust (scroll_samples.rs): the constructor's ZoomToFactor(4) (once the scroll viewer is first loaded, when it has its presenter), ZoomModeComboBox_SelectionChanged (the zoom mode, the slider enabled only with zooming, the factor 2 otherwise), ZoomSlider_ValueChanged (ChangeView), ScrollViewerControl_ViewChanged (the slider follows a change of ZoomFactor) and the four scroll mode and bar visibility handlers; the slider keeps the source's OneWay x:Bind of MinZoomFactor and MaxZoomFactor (its Header is composed). FerroUI's ScrollViewer has no HorizontalScrollMode or VerticalScrollMode: a Disabled scroll mode is the bar's Disabled visibility, which turns the scrolling of the axis off, and Enabled and Auto keep the visibility of the bar's box. IsVerticalScrollChainingEnabled is IsScrollChainingEnabled."),
    page!(ported "Flyout", "The source example uses native Button.Flyout with DeleteConfirmation_Click of FlyoutPage.xaml.cs in Rust: the confirmation hides the flyout. The message's BaseTextBlockStyle is its semibold weight; the page's SharedFlyout resource is used by no example and is left out."),
    page!(ported "DropDownButton", "Both source examples use native DropDownButton with a MenuFlyout placed BottomEdgeAlignedLeft: the text button and the icon button with the source's FontIcon glyphs on the button and the items. WinUI's MenuFlyoutItem is MenuItem (Text is Header). The page has no code-behind upstream. The glyphs need the symbol font of the platform (Segoe Fluent Icons on Windows)."),
    page!(ported "SplitButton", "Both source examples use native SplitButton with the constructor and handlers of SplitButtonPage.xaml.cs in Rust: the primary part colors the RichEditBox text with its swatch, a flyout swatch colors the text and the primary part and closes the flyout, new text takes the current color (TextChanged), and the text button's swatches close its flyout. FerroUI has no GridView and no VariableSizedWrapGrid: both flyouts are a UniformGrid of three columns whose swatches are buttons, and GridView_ItemClick is their Click."),
    page!("ToggleSplitButton", "The source example uses native ToggleSplitButton with BulletButton_Click of ToggleSplitButtonPage.xaml.cs in Rust: the clicked bullet button sets the symbol and the automation name, checks the button, closes the flyout and focuses the RichEditBox. FerroUI's RichEditBox has no paragraph list type (ParagraphFormat.ListType, MarkerType), so neither the bullet buttons nor MyListButton_IsCheckedChanged set the list of the text, and the page stays partial."),
    page!(ported "ToolTip", "All three source examples use native ToolTip (the page has no code-behind upstream; status_samples.rs sets the source's cliff.jpg on the image): the button's simple ToolTip, the TextBlock's ToolTip with the source's VerticalOffset of -80, and the image's non-occluding ToolTip placed to its right. WinUI's ToolTipService.ToolTip is the attached ToolTip.Tip, and the ToolTip's own VerticalOffset and Placement are the attached ToolTip.VerticalOffset and ToolTip.Placement of the element. FerroUI's ToolTip has no PlacementRect: the source's rectangle (0,0,400,266) is the bounds of the 400 by 266 image, which is the target of the placement, so Right places the tip beside the image as the source does. AutomationProperties.FullDescription is HelpText, the description FerroUI's automation has."),
    page!(ported "HyperlinkButton", "Both source examples use native HyperlinkButton: NavigateUri opens the Microsoft home page through the top level's launcher, with the Disable option as the source's OneWay x:Bind of IsChecked negated by BoolConverters.Not; GoToHyperlinkButton_Click navigates the gallery to the ToggleButton page, as the source's App.MainWindow.Navigate does."),
    page!(ported "CalendarDatePicker", "The source example uses native CalendarDatePicker with its header (composed above it, whose automation name it is) and the placeholder Pick a date. The page has no code-behind upstream."),
    page!("CalendarView", "The source example uses native Calendar (WinUI's CalendarView) with SelectionMode_SelectionChanged of CalendarViewPage.xaml.cs in Rust (None, Single as SingleDate, Multiple as MultipleRange). FerroUI's Calendar has no IsGroupLabelVisible, IsOutOfScopeEnabled or CalendarIdentifier (only the Gregorian calendar), and its names follow the current culture instead of a Language of the calendar, so those options and their x:Bind are left out and the page is partial."),
    page!(ported "DatePicker", "Both source examples use native DatePicker: the picker with its header (composed above it), and the picker with the formatted day and hidden year with OnNavigatedTo of DatePickerPage.xaml.cs in Rust (the date two months from now, within the years from now to five years from now; WinUI's Date is the SelectedDate). FerroUI's DayFormat is a .NET date format: WinUI's {day.integer} ({dayofweek.abbreviated}) is d (ddd)."),
    page!(ported "TimePicker", "All three source examples use native TimePicker: the picker with the clock of the current culture, the picker with its header (composed) and MinuteIncrement 15, and the 12-hour and 24-hour ClockIdentifiers. The page has no code-behind upstream; the x:Bind of the static path sys:DateTime.Now.TimeOfDay (a member of a .NET type, which the x:Bind core does not compile) is the time of day set once in Rust, as that OneTime binding."),
    page!(ported "Shape", "All three source examples use native Ellipse, Rectangle and Polygon with the source's TwoWay x:Bind of the sliders (sizes, radii and stroke thickness) and the OneWay x:Bind of the Show points switch to the point labels (Visibility is IsVisible, IsOn is IsChecked, the switch's Header its Content). The page has no code-behind upstream. FerroUI's Slider has no Header (composed) and no StepFrequency or SnapsTo: the steps of 0.5 are TickFrequency with IsSnapToTickEnabled. Canvas.ZIndex is the shared ZIndex."),
    page!(ported "Line", "All four source examples use native Line, Polyline, Path and a GeometryGroup of LineGeometry, EllipseGeometry and RectangleGeometry (FillRule EvenOdd), with the source's TwoWay x:Bind of the stroke thicknesses and of the ellipse's radii, and the OneWay x:Bind of the Show points switches to the point labels (Visibility is IsVisible, IsOn is IsChecked, the switch's Header its Content). The page has no code-behind upstream. WinUI's X1, Y1, X2 and Y2 are FerroUI's StartPoint and EndPoint: the four point sliders set them in Rust (shape_samples.rs) for the source's TwoWay x:Bind of each coordinate. FerroUI's Slider has no Header (composed) and no StepFrequency or SnapsTo: the steps of 0.5 are TickFrequency with IsSnapToTickEnabled. Canvas.ZIndex is the shared ZIndex; the Rect of RectangleGeometry is written x,y,width,height."),
    page!("Typography", "The WinUI type ramp is expressed with shared TextBlock font sizes and weights."),
    page!("ListView", "All ten source examples use native ListBox (FerroUI has no ListView) with the code behind of ListViewPage.xaml.cs in Rust (list_samples.rs): the basic list, which takes the focus when it is loaded; SelectionModeComboBox_SelectionChanged (Single; Multiple as Multiple with Toggle; Extended as Multiple; None as items that are not selected, hit or focused, a class of the page, because FerroUI's SelectionMode has no None); ListView_Drop between and within the two lists with the source's insertion index, the contacts in an in-process data format, and a drag that starts when the pointer pressed on an item moves (DragItemsStarting); the grouped contacts after their group headers in one list, the headers not selectable; OnFilterChanged with Remove_NonMatching, AddBack_Contacts and the count in a polite live region (UIHelper.AnnounceActionForAccessibility); the inverted list with AddItemToEnd and MessageReceived, which keeps its last item in view; the images of CustomDataObject; ContactDeleteMenuItem_Click of the items' context menu; the scroll position of ListViewPersistenceHelper (the key of the first visible item and the offset in it), saved and restored; and ScrollIntoView with the Default and Leading alignments. The contacts of Contacts.txt are a table, x:Bind in the templates is {Binding} over markup types and WinUI's StackPanel with a background, padding and corners is a Border. FerroUI has no grouping (CollectionViewSource, GroupStyle) and no sticky group headers (the Sticky Headers switch changes nothing), no IsTextTrimmed on TextBlock (TextBlock_IsTextTrimmedChanged does not set the description's tooltip), no ItemsUpdatingScrollMode and no check boxes of the Multiple mode, so the page is partial."),
    page!("TreeView", "All four source examples use native TreeView with the code behind of TreeViewPage.xaml.cs in Rust (tree_samples.rs): the nodes of InitializeSampleTreeView as TreeViewItems with their content as the header (FerroUI has no TreeViewNode), the multi-selection tree (Multiple with Toggle: a click toggles an item, without check boxes), the tree bound to GetData with the names and children of its ExplorerItems, and ExplorerItemTemplateSelector as the folder and file data templates of the tree, with the source's folder.png and the E8A5 glyph. The source's item templates are TreeViewItems with the x:Bind of Children: here hierarchical data templates (FuncTreeDataTemplate) whose children are the item's Children, and the templates' IsExpanded is a style of the tree. FerroUI's TreeView has no drag and drop of its items (CanDragItems, AllowDrop), so the first tree does not move items and the page is partial. The glyph needs the platform's symbol font."),
    page!(ported "MenuBar", "All three source examples use native Menu (WinUI's MenuBar; MenuBarItem.Title is the Header of a top MenuItem) with OnElementClicked of MenuBarPage.xaml.cs in Rust (the first letter of the item's name picks the output): the simple bar, the bar whose KeyboardAccelerators are HotKeys shown as InputGestures, and the bar with a submenu, separators and two radio groups (RadioMenuFlyoutItem is a MenuItem with the ToggleType Radio)."),
    page!("MenuFlyout", "All seven source examples use native MenuFlyout with MenuFlyoutItem_Click and SplitMenuFlyoutItem_Click of MenuFlyoutPage.xaml.cs in Rust: the sort menu of a compact CommandBarButton (the Tag of the item), the check items and separator, the cascading menus, the icons (FontIcon and SymbolIcon), the KeyboardAccelerators as HotKeys shown as InputGestures with the source's fonts, and the radio groups. WinUI's MenuFlyoutItem is MenuItem (Text is Header); ToggleMenuFlyoutItem and RadioMenuFlyoutItem are MenuItems with the ToggleType CheckBox and Radio. FerroUI has no SplitMenuFlyoutItem: Save and Share are MenuItems with the source's items, whose primary parts cannot be clicked, so the page is partial. The glyphs need the platform's symbol font."),
    page!(ported "SplitView", "The source example uses native SplitView with the code behind of SplitViewPage.xaml.cs in Rust (split_view_samples.rs): NavLinksList_ItemClick (the link's page in the content), PanePlacement_Toggled, togglePaneButton_CheckedChanged and SplitViewPage_Loaded with UpdateNavLinkItemLayout (the item template with the icon on the side of the pane), displayModeCombobox_SelectionChanged and paneBackgroundCombobox_SelectionChanged; the pane lengths keep the source's OneWay x:Bind of the sliders and IsPaneOpen its TwoWay x:Bind of the toggle button (the binding converts the nullable IsChecked, so no converter is needed). The NavLinks are a markup type for the {Binding} of the templates. FerroUI has no ListView: the links are a ListBox whose ItemClick is a tap on an item. The page's visual states are classes set by the handlers: Red, Blue and Green of the split view (the PaneBackground states) and RightIconLayout of the list. FerroUI's Slider has no Header (composed) and no StepFrequency or SnapsTo: the steps of 8 are TickFrequency with IsSnapToTickEnabled; the switch's IsOn is IsChecked and its Header its Content. As in the source, the split view keeps its markup DisplayMode until the box's selection changes. The symbol glyphs need the platform's symbol font."),
    page!(ported "Binding", "All seven source examples with the code behind of BindingPage.xaml.cs in Rust (binding_samples.rs): the OneWay and TwoWay x:Bind between the text boxes (UpdateSourceTrigger PropertyChanged), the x:Bind of a text box's text through a converter (emptyStringToVisibilityConverter is StringConverters.IsNotNullOrEmpty, Visibility is IsVisible), and the detail panel's DataContext x:Bind of the list's SelectedItem, all compiled by the x:Bind core. A native page has no x:Class, so the x:Bind of the page's own members is its code behind in Rust: GreetingMessage is set once (OneTime), FormatDate(DatePickerControl.SelectedDate) follows the picker, and ExampleViewModel is the page's DataContext, read with {Binding} over the markup types ExampleViewModel and ListDetailItem (whose values do not change, so they raise no property changes; TargetNullValue as the source's, whose NullString is empty, not null). FerroUI has no ListView: the list is a ListBox. The Headers are composed and AppBarSeparator is CommandBarSeparator."),
    page!("XamlResources", "Static and dynamic resource examples use the shared resource dictionary; broader WinUI lookup rules remain tracked."),
    page!("AccessibilityKeyboard", "Tab order examples use native enabled/focusable controls and explicit TabIndex."),
    // G1 (#418): pages of controls that exist on main.
    page!(ported "ThemeShadow", "The source example uses the native ThemeShadow as the Border's Shadow with its Loaded and ValueChanged handlers in Rust: the grid is a receiver of the shadow and the slider sets the Z of the Border's Translation, the elevation the shadow is drawn at. As in WinUI 3 the theme shadow is a drop shadow; the receivers are kept but draw nothing. Rendering parity against Windows remains pending."),
    page!(ported "Acrylic", "All three source examples use the native AcrylicBrush: the theme's AcrylicInAppFillColorDefaultBrush, and the page's two brushes with the source's values, changed by the Loaded, Slider and ComboBox handlers in Rust (tint opacity, tint colour, fallback colour, tint luminosity opacity). The SolidColorBrush items and their ColorTemplate are composed swatches; the adaptive VisualStateManager sizes are the wide ones. Rendering parity against Windows remains pending."),
    page!(ported "SystemBackdrops", "All three source examples open the source's sample windows: Window.SystemBackdrop with MicaBackdrop (Base, BaseAlt) and DesktopAcrylicBackdrop, and MicaController and DesktopAcrylicController (Base, Thin) with a SystemBackdropConfiguration that follows activation, the theme and high contrast; SetBackdrop falls back as the C# does where the platform has no material, and the theme box sets the window's theme. The announcement is a live region; the badge logo image of the window title is not shown."),
    page!(ported "TitleBar", "All three source examples use the native TitleBar: title, subtitle and button visibility bound to the options, and the two sample windows (TitleBar.IsDragRegion on a status badge, RecomputeDragRegions after adding a button; the end-to-end window with the back button bound to Frame.CanGoBack, BackRequested, PaneToggleRequested and NavigationView navigation). ImageIconSource is a PathIconSource and PersonPicture a composed circle with the initials (neither exists in FerroUI)."),
    page!("RichEditBox", "The simple editor, the custom context menu command and the formatting editor use the native RichEditBox and its text object model: bold/italic toggles, font colors with the TextChanged rule, FindText highlighting and RTF open/save through the window's storage provider. The context menu is the theme's MenuFlyout (no CommandBarFlyout or SelectionFlyout). Both math examples need the Windows RichEdit math mode and are recorded as not applicable with visible diagnostics."),
    page!(ported "RichTextBlock", "All four source examples use the native RichTextBlock: formatted runs and a hyperlink, SelectionHighlightColor, OverflowContentTarget through two RichTextBlockOverflow columns, and a TextHighlighter that follows the ComboBox color over the source range. The overflow targets keep the source x:Bind of the elements; exact text layout and host accessibility remain pending."),
    page!("ItemsRepeater", "All six source examples, and the nested repeaters of the source page, use the native ItemsRepeater of ferroui-controls-repeater: observable sources, the stack, uniform grid and flow layouts, ItemsRepeaterScrollHost, native IDataTemplateSelector selectors and the Gallery ActivityFeedLayout and VariedImageSizeLayout ported to Rust VirtualizingLayouts. x:Bind becomes {Binding} over markup types; the composition scale animation becomes a scale set after each layout pass. The accessibility announcements and the key-index mapping of the recipe source remain pending."),
    page!(ported "LayoutPanel", "The source example uses the native LayoutPanel with the FeaturedTileLayout of the source ported to a Rust NonVirtualizingLayout; the options switch to a vertical StackLayout and set the spacing, as the C# handlers do."),
    page!(ported "InkCanvas", "All three source examples use the native InkCanvas, InkPresenter and InkToolbar with the handlers of InkCanvasPage.xaml.cs in Rust: input devices, ink colour, stroke width, inking enabled and clear; the toolbar's available tools, active tool in both directions, flyout placement and clear; save, clear and restore with the source's messages. Header is composed, TargetInkCanvas is set in Rust and the saved strokes are the bytes of InkStrokeContainer::save in memory. Routed window input and rendering acceptance remain pending."),
    page!("Chart", "The seven source examples map onto the native CartesianChart with the data and option handlers of ChartPage.xaml.cs in Rust: category labels, legend, bar orientation and category sort, grid lines and axis visibility, series visibility, dash style, thickness, markers and data labels, a day axis for the DateTimeAxis with its intervals and label formats, and the live series. LegendTitle, tick marks, axis brushes, marker shapes and per-point overrides are not available; rendering parity remains pending."),
];

/// The buttons of the native examples that navigate the gallery, as the source's
/// `App.MainWindow.Navigate(typeof(ItemPage), id)`: the page, the name of its button and the page
/// the button opens. The gallery wires them when it shows the page (`gallery.rs`).
pub static GALLERY_LINKS: &[(&str, &str, &str)] = &[("HyperlinkButton", "GoToToggleButton", "ToggleButton")];

pub fn find(page_id: &str) -> Option<&'static PageAdaptation> {
    ADAPTATIONS.iter().find(|sample| sample.page_id == page_id)
}

/// Creates real framework controls from the reviewed markup, then wires each
/// behavior in Rust. No alternate drawing or input system is used.
pub fn create(sample: &PageAdaptation) -> Result<Ref<Control>, XamlLoadError> {
    crate::assets::register();
    ferroui_controls_color_picker::register_types();
    ferroui_controls_repeater::register_types();
    ferroui_controls_inking::register_types();
    ferroui_controls_livecharts::register_types();
    ferroui_media_video_controls::register_types();
    ferroui_media_camera_controls::register_types();
    let root = FerroRuntimeXamlLoader::parse_as::<Ref<Control>>(sample.xaml, None)?;
    crate::frame_sample::wire(&root)?;
    crate::navigation_view_samples::wire(&root)?;
    crate::sizing_sample::wire(&root)?;
    crate::platform_samples::wire(&root);
    crate::input_samples::wire(&root);
    crate::control_samples::wire(&root);
    crate::feedback_samples::wire(&root);
    crate::color_picker_samples::wire(&root);
    crate::connected_animation_samples::wire(&root)?;
    crate::media_samples::wire(&root);
    crate::playback_samples::wire(&root);
    crate::materials_samples::wire(&root);
    crate::window_samples::wire(&root);
    crate::text_samples::wire(&root);
    crate::repeater_samples::wire(&root);
    crate::ink_samples::wire(&root);
    crate::chart_samples::wire(&root);
    crate::button_samples::wire(sample.page_id, &root);
    crate::layout_samples::wire(sample.page_id, &root);
    crate::text_input_samples::wire(sample.page_id, &root);
    crate::status_samples::wire(sample.page_id, &root);
    crate::command_samples::wire(sample.page_id, &root);
    crate::menu_samples::wire(sample.page_id, &root);
    crate::date_samples::wire(sample.page_id, &root);
    crate::list_samples::wire(sample.page_id, &root);
    crate::tree_samples::wire(sample.page_id, &root);
    crate::split_view_samples::wire(sample.page_id, &root);
    crate::shape_samples::wire(sample.page_id, &root);
    crate::scroll_samples::wire(sample.page_id, &root);
    crate::brush_samples::wire(sample.page_id, &root);
    crate::binding_samples::wire(sample.page_id, &root);
    wire(&root);
    initialize_x_bind(&root)?;
    Ok(root)
}

/// The `{x:Bind}` bindings of a page (#525): initialized once the page and its Rust code-behind
/// are wired, as WinUI's `Bindings.Initialize` runs on `Loading` after the constructor. A page that
/// is attached to a window later is not read again (the attach finds them initialized). A
/// binding that cannot be read is an error of the page.
fn initialize_x_bind(root: &Ref<Control>) -> Result<(), XamlLoadError> {
    match XBindings::get_for_root(root) {
        Some(bindings) => bindings
            .initialize()
            .map_err(|errors| XamlLoadError::with_message(format!("x:Bind: {errors}"))),
        None => Ok(()),
    }
}

fn wire(root: &Ref<Control>) {
    wire_contrast(root);
    wire_radio_groups(root);
    let Some(output) = find_control::<TextBlock>(root, "Output") else {
        return;
    };
    if let Some(primary) = find_control::<Button>(root, "Primary") {
        let count = Rc::new(Cell::new(0u64));
        let weak_output = output.downgrade();
        primary.click(move |_, _| {
            count.set(count.get().saturating_add(1));
            if let Some(output) = weak_output.upgrade() {
                output.set_text(Some(&format!("Clicked {} times", count.get())));
            }
        });
    }
    if let (Some(disable), Some(primary)) = (
        find_control::<ToggleButton>(root, "Disable"),
        find_control::<Control>(root, "Primary"),
    ) {
        let weak_primary = primary.downgrade();
        let weak_disable = disable.downgrade();
        disable.property_changed(move |_| {
            if let (Some(primary), Some(disable)) = (weak_primary.upgrade(), weak_disable.upgrade())
            {
                primary.set_is_enabled(disable.is_checked() != Some(true));
            }
        });
    }
    output.set_margin(Thickness::new(0.0, 4.0, 0.0, 0.0));
}

fn wire_radio_groups(root: &Ref<Control>) {
    let Some(output) = find_control::<Border>(root, "ControlOutput") else {
        return;
    };
    for name in ["BackgroundRadioButtons", "BorderRadioButtons"] {
        let radio = root.get_control::<RadioButtons>(name);
        let weak_output = output.downgrade();
        let weak_radio = radio.downgrade();
        radio.property_changed(move |change| {
            if change.property()
                != ferroui_controls::primitives::SelectingItemsControl::selected_index_property()
                    .as_property()
            {
                return;
            }
            if let (Some(output), Some(radio)) = (weak_output.upgrade(), weak_radio.upgrade()) {
                let color = match (name, radio.selected_index()) {
                    ("BackgroundRadioButtons", 0) => Color::from_rgb(0, 128, 0),
                    ("BackgroundRadioButtons", 1) => Color::from_rgb(255, 255, 0),
                    ("BorderRadioButtons", 0) => Color::from_rgb(0, 100, 0),
                    ("BorderRadioButtons", 1) => Color::from_rgb(255, 215, 0),
                    (_, 2) => Color::from_rgb(255, 255, 255),
                    _ => return,
                };
                let brush = Some(Rc::new(ImmutableSolidColorBrush::new(color))
                    as Rc<dyn ferroui_base::media::IBrush>);
                if name == "BackgroundRadioButtons" {
                    output.set_background(brush);
                } else {
                    output.set_border_brush(brush);
                }
            }
        });
    }
    let text = root.get_control::<TextBlock>("Control1Output");
    for (name, label) in [
        ("Option1", "Option 1"),
        ("Option2", "Option 2"),
        ("Option3", "Option 3"),
    ] {
        let radio = root.get_control::<ToggleButton>(name);
        let weak_radio = radio.downgrade();
        let weak_text = text.downgrade();
        radio.property_changed(move |change| {
            if change.property() == ToggleButton::is_checked_property().as_property() {
                if let (Some(radio), Some(text)) = (weak_radio.upgrade(), weak_text.upgrade()) {
                    if radio.is_checked() == Some(true) {
                        text.set_text(Some(&format!("You selected {label}")));
                    }
                }
            }
        });
    }
}

fn wire_contrast(root: &Ref<Control>) {
    let Some(text_color) = find_control::<TextBox>(root, "TextColor") else {
        return;
    };
    let background_color = root.get_control::<TextBox>("BackgroundColor");
    let weak_root = root.downgrade();
    let update = Rc::new(move || {
        let Some(root) = weak_root.upgrade() else {
            return;
        };
        let first = crate::contrast::parse_rgb(
            &root
                .get_control::<TextBox>("TextColor")
                .text()
                .unwrap_or_default(),
        );
        let second = crate::contrast::parse_rgb(
            &root
                .get_control::<TextBox>("BackgroundColor")
                .text()
                .unwrap_or_default(),
        );
        let ratio = root.get_control::<TextBlock>("ContrastRatio");
        let checks = root.get_control::<TextBlock>("ContrastChecks");
        let (Some(first), Some(second)) = (first, second) else {
            ratio.set_text(Some("Enter two six-digit RGB colors."));
            checks.set_text(Some("Contrast is unavailable while an input is invalid."));
            return;
        };
        let result = crate::contrast::ContrastResult::calculate(first, second);
        ratio.set_text(Some(&format!("{:.2}:1", result.ratio)));
        let state = |passed| if passed { "Pass" } else { "Fail" };
        checks.set_text(Some(&format!(
            "Regular text (4.5:1): {} · Large text (3:1): {} · Components (3:1): {}",
            state(result.normal_text),
            state(result.large_text),
            state(result.components),
        )));
        root.get_control::<Border>("ContrastPreview")
            .set_background(Some(Rc::new(ImmutableSolidColorBrush::new(second))));
        root.get_control::<TextBlock>("ContrastText")
            .set_foreground(Some(Rc::new(ImmutableSolidColorBrush::new(first))));
    });
    for input in [text_color, background_color] {
        let update = update.clone();
        input.property_changed(move |change| {
            if change.property() == TextBox::text_property().as_property() {
                update();
            }
        });
    }
    update();
}

fn find_control<T: ObjectType>(root: &Ref<Control>, name: &str) -> Option<Ref<T>> {
    root.find_control::<Control>(name)
        .and_then(|control| control.cast::<T>())
}

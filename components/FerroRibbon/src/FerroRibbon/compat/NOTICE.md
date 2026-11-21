# Input compatibility attribution

RibbonInputTextBox adapts selected null/first-line TextBox behavior from Uno commit
da217df2312e9e39676880f9cb4b8d519cc297e0,
src/Uno.UI/UI/Xaml/Controls/TextBox/TextBox.cs, licensed under Apache-2.0. Visibility
retains the original enum values from the same revision. The same full Apache-2.0
license is also retained in docs/licenses/Uno-FontIcon-Apache-2.0.md for distribution.

Changes: use native FerroUI inheritance, typed properties and coercion; normalize
nullable UTF8 strings and return the prefix before CR/LF through native TextBox
editing services. This is a partial source-platform mapping, not the Uno runtime.

Slider numerical and keyboard adaptations (slider_math.rs, ribbon_input_slider.rs)
are based on original Microsoft Slider.mux.cs, SliderKeyProcess.cs and DirectUI
DoubleUtil.cs at the same pinned Uno revision. Copyright (c) Microsoft Corporation.
Licensed under MIT; the full unchanged notice is retained for distribution in
docs/licenses/Uno-Controls-Microsoft-MIT.txt.
Changes use Rust, native FerroUI class/property/input services and explicit source
algorithm adapters.

Range coercion/retained raw Value in ribbon_input_slider.rs additionally adapts
RangeBase.cs, RangeBase.mux.cs, RangeBase.mux.core.cs and RangeBase.Properties.cs at
that same Uno revision (Apache-2.0). Changes integrate source coercion with native
FerroUI property metadata and routed value notifications.

The frozen thumb drag-coordinate adaptation also follows original Thumb.mux.cs
(Copyright (c) Microsoft Corporation, MIT) at the same pinned Uno revision. Original
bytes/hash/blob provenance and full MIT notice remain retained as above. The port
uses a native matrix snapshot and weak native root, preserving fractional pointer
positions. This does not claim full source Thumb/touch/pen/platform parity.

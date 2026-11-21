# Control attribution

Ribbon controls derive from RibbonSpace, Copyright (c) 2026 Wiesław Šoltés, MIT.
The original notice/license are preserved at the repository root.

The push-button click/command/flyout order is adapted from the pinned Uno
ButtonBase.mux.cs and Button.mux.cs (source commit
 da217df2312e9e39676880f9cb4b8d519cc297e0), which retain these notices:

The native menu adapters also retain the selected menu command/toggle/radio behavior.

Copyright (c) Microsoft Corporation. All rights reserved.
Licensed under the MIT License.

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

RibbonInputBase/RibbonTextBox and their template derive from the original RibbonSpace
RibbonInputs.cs/Inputs.xaml under the original MIT notice.

The native color dialog visibility/focus adaptation follows the pinned Uno ContentDialog
source (Microsoft Corporation, MIT, with Uno Apache-2.0 project notices). Reference
methods and XAML setters compile only in the C# test harness. They are not runtime
reflection or a linked Uno platform dependency.

Additional pinned Uno development references: ICommandExtensions.cs
(src/Uno.Foundation/Extensions) and DeferralManager.cs (src/Uno.UWP/Helpers), revision
da217df2312e9e39676880f9cb4b8d519cc297e0. These helper sources use the Uno repository
Apache-2.0 license, retained alongside Microsoft MIT ContentDialog notices. They are
development oracles, not new Rust runtime/BCL dependencies; the native code uses FerroUI
ICommand/Dispatcher/controls and Rust closures/ownership. No Cargo lockfile or SDK
revision changed.

The counted native closing-deferral algorithm in ribbon_dialog_lifecycle.rs is
adapted to Rust from Uno's DeferralManager.cs under the retained Uno Apache-2.0
license. Changes use a concrete per-dialog Rust state/handle and Result for
duplicate completion; no generic DeferralManager or .NET task runtime is shipped.
The ContentDialog lifecycle adaptation retains the Microsoft MIT attribution
above. The full Apache-2.0 text is retained at ../LICENSE-APACHE-2.0.md.

MIT License

Copyright (c) 2026 Wiesław Šoltés

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

The FontIcon capability adapter in compat/font_icon.rs is adapted from the Uno FontIcon
child architecture, Copyright (c) Uno Platform Inc., Apache-2.0. Changes use Rust and
native FerroUI inheritance, properties, layout, child ownership and metadata.
LICENSE-APACHE-2.0.md retains its license. The remaining RibbonSpace port code retains
the MIT license above.

Uno.Fonts.Fluent 2.6.1 supplies the unchanged embedded
Assets/Fonts/uno-fluentui-assets.ttf font. Copyright (C) 2015-2023 Uno Platform inc. -
all rights reserved. Licensed under Apache-2.0; the full license is retained in
LICENSE-APACHE-2.0.md and docs/licenses/Uno-Fonts-Fluent-Apache-2.0.md. No Uno runtime
service is included by this asset.

The DataTemplateSelector selection adapter in compat/data_template_selector.rs follows
Uno.WinUI6.7.135 at da217df2312e9e39676880f9cb4b8d519cc297e0, Copyright (c) Uno Platform
Inc., Apache-2.0. The full license is retained in LICENSE-APACHE-2.0.md. Changes use
Rust trait dispatch, native IDataTemplate and static reusable-library metadata. No Uno
binary dependency is included.

The native counted closing-deferral algorithm in controls/ribbon_dialog_lifecycle.rs is
also adapted from the pinned Uno repository under Apache-2.0. It uses concrete Rust
state/ownership and native FerroUI dispatch. These attribution exceptions accompany this
crate's existing MIT AND Apache-2.0 license expression.

The ImageIcon capability adapter in compat/image_icon.rs adapts ImageIcon.cs and
ImageIcon.Properties.cs, Copyright (c) Microsoft Corporation, MIT, and Uno's
ImageIcon.Uno.cs, Copyright (c) Uno Platform Inc., Apache-2.0, at Uno.WinUI6.7.135
commit da217df2312e9e39676880f9cb4b8d519cc297e0. The MIT permission and warranty terms
above also apply to the Microsoft portions; LICENSE-APACHE-2.0.md retains Uno's license.
Changes use native Rust/FerroUI image, ownership, property, layout and Loaded services.
No Uno binary or general runtime emulation is included.

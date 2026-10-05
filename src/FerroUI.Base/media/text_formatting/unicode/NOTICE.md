# Third-party notices for `media/text_formatting/unicode`

The code in this directory is a Rust port of the Unicode support of the upstream
UI framework this project is ported from (MIT License, see the notice at the
repository root). Parts of that code were themselves derived from the projects
below, whose notices are kept in the corresponding source files.

## RichTextKit

`unicode_trie.rs`, `unicode_trie_builder.rs`, `unicode_trie_builder_constants.rs`,
`binary_reader_extensions.rs`, `line_break.rs` (and the line break rules built on it)

    RichTextKit
    Copyright © 2019 Topten Software. All Rights Reserved.

    Licensed under the Apache License, Version 2.0 (the "License"); you may
    not use this product except in compliance with the License. You may obtain
    a copy of the License at

    https://www.apache.org/licenses/LICENSE-2.0

    Unless required by applicable law or agreed to in writing, software
    distributed under the License is distributed on an "AS IS" BASIS, WITHOUT
    WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the
    License for the specific language governing permissions and limitations
    under the License.

The trie is in turn a port of <https://github.com/foliojs/unicode-trie> and the
line breaker of <https://github.com/foliojs/linebreak> (both MIT License,
Copyright (c) Devon Govett), which follow the UTrie2 design of ICU.

## SixLabors.Fonts

`bidi_algorithm.rs`, `bidi_data.rs`

    Copyright (c) Six Labors.
    Licensed under the Apache License, Version 2.0.
    Ported from: https://github.com/SixLabors/Fonts/

## .NET runtime

`grapheme_enumerator.rs`

    This source file is adapted from the .NET cross-platform runtime project.
    (https://github.com/dotnet/runtime/)
    Copyright (c) .NET Foundation and Contributors. MIT License.

## Unicode data

`unicode_data_trie.rs`, `bidi_trie.rs`, `segmentation_trie.rs`,
`east_asian_width_trie.rs`, `script_extensions_data.rs` and the property enums
are derived from the Unicode Character Database, version 17.0.0.

    Copyright © 1991-2025 Unicode, Inc. All rights reserved.
    Distributed under the Terms of Use in https://www.unicode.org/copyright.html
    and the Unicode License v3 (https://www.unicode.org/license.txt).

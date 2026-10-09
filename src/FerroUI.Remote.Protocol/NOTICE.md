# NOTICE

## Source code

The messages, the resolver and the transports of this crate are ported from
the `Avalonia.Remote.Protocol` project of Avalonia (MIT; see the `NOTICE.md`
at the root of the repository). The tests in `tests/remote_protocol_tests.rs`
are ported from `RemoteProtocolTests.cs` of the `Avalonia.DesignerSupport.Tests`
project of the same repository.

`lib.rs` compiles `key.rs` and `physical_key.rs` of the base library
(`src/FerroUI.Base/input/`) into this crate, as the upstream project file
compiles `Input/Key.cs` and `Input/PhysicalKey.cs` of `Avalonia.Base` into
its assembly.

## Metsys.Bson

`metsys_bson.rs` is ported from `MetsysBson.cs` of that project, which is the
source of the Metsys.Bson library (<https://github.com/elaberge/Metsys.Bson>)
imported there in one file. The file carries the notice of its origin:

```
Copyright (c) 2010, Karl Seguin - http://www.openmymind.net/
All rights reserved.

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions are met:
    * Redistributions of source code must retain the above copyright
      notice, this list of conditions and the following disclaimer.
    * Redistributions in binary form must reproduce the above copyright
      notice, this list of conditions and the following disclaimer in the
      documentation and/or other materials provided with the distribution.
    * Neither the name of the <organization> nor the
      names of its contributors may be used to endorse or promote products
      derived from this software without specific prior written permission.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS" AND
ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED
WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
DISCLAIMED. IN NO EVENT SHALL <COPYRIGHT HOLDER> BE LIABLE FOR ANY
DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES
(INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES;
LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND
ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
(INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS
SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

Code imported from https://github.com/elaberge/Metsys.Bson without any changes
```

The last line is the upstream project's note on its import. The port is a
translation to Rust: the reflection of the original is replaced by
declarations (`bson_class!`, `bson_enum!`), and the differences are recorded
in `docs/porting/DEVIATIONS.md`.

The date of a day count in `DateTime::to_sortable_string` is computed with
the algorithm `civil_from_days` published by Howard Hinnant
(<https://howardhinnant.github.io/date_algorithms.html>, public domain).

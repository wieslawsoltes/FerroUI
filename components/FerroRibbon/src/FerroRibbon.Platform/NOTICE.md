FerroRibbon native command collation uses a fixed case-insensitive ICU profile for
ribbon-search label ordering and customization catalog sorting/filtering. The profile
data is adapted from .NET Foundation pal_collation.c v10.0.13, MIT license
(LICENSE-DOTNET-MIT). No CLR/BCL/runtime implementation or option-dispatch API is
included.

The macOS backend dynamically links the operating system libicucore. ICU code or
binaries are not bundled. The ICU Unicode license is retained in LICENSE-ICU.

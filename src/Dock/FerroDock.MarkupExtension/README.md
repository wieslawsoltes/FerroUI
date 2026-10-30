# FerroDock.MarkupExtension

The original Dock URI-loading and named-reference extensions, using FerroUI's shared
markup loader, services and namescope. Add ferrodock-markup-extension and register its
types with the normal application metadata registration. Its Cargo build exports
both extensions for independent compiled consumers.

The original framework namespace maps to https://github.com/ferroui, with the CLR
namespace FerroUI.MarkupExtension and assembly FerroDock.MarkupExtension. Both default
and explicit namespace forms are checked by the downstream consumer:

```xml
<ContentControl Content="{Load Layout.xaml}" />
<Border Tag="{Reference ExistingControl}" />
```

Load preserves its optional Source, both constructors and actual caller URI context.
It delegates loading and errors to FerroXamlLoader and does not cache loaded objects.
Reference preserves its optional Name, both constructors and immediate name lookup.
It returns the same named object or model and does not defer missing references.

Shared namescopes now support plain model objects as well as controls, with original
reference identity, mixed duplicate-name and nested-scope behavior. A named plain
application object backed by the real Document engine is checked through compiled
Reference and a live compiled title binding. The existing shared namescope/compiler
packages carry documented support changes, without another registry or UI model.
Standalone declared plain model roots now compile/load, retaining one constructor
instance and completing the native scope. Interface and inherited class views share
the enclosing engine identity through the common value metadata. External constructor descriptor carriers now return their actual model types; public
value paths come from the declared model syntax/shared registry. Actual policy/override
roots drive the original engine resolver. All concrete model constructor exports,
logical assembly ownership, plain-model x:Class fields, every generic/cross-package root,
CLR metadata/inheritance and all loader/platform/lifetime cases remain partial.

Validation lives in the separate FerroDock.Markup.Tests consumer. The source-only
scripts/dock/factory-markup-reference project uses unchanged original C# assemblies and
records the source behavior independently. Ordinary Cargo tests require no .NET.

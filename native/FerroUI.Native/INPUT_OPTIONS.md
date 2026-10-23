# macOS input-scope hints

The input-options extension delivers supported semantic content hints to the
existing `FrnView` through AppKit's public `NSTextContent` protocol. Policy and
delivery state live in Rust; the Objective-C++ bridge only selects public
AppKit constants and assigns `contentType` on that same view.

This is a bounded input-scope implementation. AppKit does not expose 43 keyboard
types corresponding to WinUI's 43 `InputScopeNameValue` values. Native numeric
keypads, script/width switching, return-key labels, locale selection, filtering
and masking are not supplied by `NSTextContent`.

## Source and ABI

The WinUI value contract remains pinned to
[`fe244ca6ac4d482712b234a9de3cf771cc6481de`](https://github.com/microsoft/microsoft-ui-xaml/blob/fe244ca6ac4d482712b234a9de3cf771cc6481de/dxaml/xcp/tools/XCPTypesAutoGen/XamlOM/Model/Microsoft.UI.Xaml.Input.cs).
The native source import remains pinned by `UPSTREAM_REVISION`; the local
`0003-text-input-content-hints.patch` records the additional native/IDL changes
for `scripts/sync-native.sh`.

The existing `IFrnTextInputMethod` IID and five method slots are unchanged. An
optional `IFrnTextInputMethodOptions` interface, IID
`8b73203e-0e92-493d-b290-fe3fd801fdb8`, supplies `SetContentType` and is obtained
through `QueryInterface`. A separately distributed older native library can
continue to supply the old interface without supporting these hints.

Apple documents
[`NSTextContent.contentType`](https://developer.apple.com/documentation/appkit/nstextcontent/contenttype)
as the semantic meaning of a text input area and identifies its use by the
system. Its property is available from macOS 11. The
[`URL`](https://developer.apple.com/documentation/appkit/nstextcontenttype/url),
[`name`](https://developer.apple.com/documentation/appkit/nstextcontenttype/name),
[`emailAddress`](https://developer.apple.com/documentation/appkit/nstextcontenttype/emailaddress)
and
[`telephoneNumber`](https://developer.apple.com/documentation/appkit/nstextcontenttype/telephonenumber)
constants are available from macOS 14. The bridge guards those constants with
`@available(macOS 14.0, *)`; an earlier supported system receives `nil`.
It does not invent private NSString values or change the deployment target.
The reviewed documentation URLs, availability and downloaded-source hashes are
recorded in `input-options-api-evidence.json`.

## Mapping and privacy

The first explicitly supplied name is authoritative. Empty or absent Names
fall back to legacy `TextInputContentType`; explicit `Default` clears a legacy
semantic hint. Supported legacy `Name`, `Email` and `Url` receive the matching
content hints. Legacy `Digits` is not declared to be a telephone number.

| Input scope | AppKit content type |
| --- | --- |
| `Url` | `NSTextContentTypeURL` |
| `EmailSmtpAddress`, `EmailNameOrAddress` | `NSTextContentTypeEmailAddress` |
| `PersonalFullName` | `NSTextContentTypeName` |
| `TelephoneNumber` | `NSTextContentTypeTelephoneNumber` |
| `Default`, `Text`, `Chat`, `ChatWithoutEmoji`, `NativeScript` | `nil` |
| `CurrencyAmountAndSymbol`, `CurrencyAmount`, `Digits`, `Number`, `NumberFullWidth`, `FormulaNumber`, `Formula` | `nil` |
| `DateMonthNumber`, `DateDayNumber`, `DateYear`, `TimeHour`, `TimeMinutesOrSeconds` | `nil`; these do not describe a birthdate or complete date/time |
| `TelephoneCountryCode`, `TelephoneAreaCode`, `TelephoneLocalNumber`, `NameOrPhoneNumber` | `nil`; these do not describe an unambiguous whole telephone number |
| `AlphanumericHalfWidth`, `AlphanumericFullWidth`, `Hiragana`, `KatakanaHalfWidth`, `KatakanaFullWidth`, `Hanja`, `HangulHalfWidth`, `HangulFullWidth`, `ChineseHalfWidth`, `ChineseFullWidth` | `nil`; semantic content types cannot request a script or width |
| `Search`, `SearchIncremental`, `Maps` | `nil`; no equivalent content meaning |
| `Password`, `NumericPassword`, `NumericPin`, `AlphanumericPin` | `nil`; semantic completion hints are suppressed |

`IsSensitive`, legacy `Password`/`Pin` and the four explicit secret scopes each
suppress semantic completion/AutoFill hints independently. A non-secret scope
does not clear an independently configured sensitive policy. Explicit
`ShowSuggestions = false` also suppresses these hints. This suppression only
covers the new semantic hint channel. It does not claim to disable every IME's
own candidate service, enable secure event input, obscure document text, or
restrict the user's input source.

## Lifetime and remaining native service boundary

Options can arrive before the active client. Rust retains the requested policy
and delivers it when a client is attached. Detaching the last client and
disposing the adapter clear the view's content hint. Repeated identical policy
does not call the native setter again. A guarded delivery loop observes newer
policy or client state installed synchronously by a callback. Updating a hint
does not recreate the view/input context, reset the input method, or discard
marked text.

This change does **not** enable spelling correction, inline prediction or
Writing Tools. Apple's
[`NSTextCheckingClient`](https://developer.apple.com/documentation/appkit/nstextcheckingclient)
and
[`NSTextCheckingController`](https://developer.apple.com/documentation/appkit/nstextcheckingcontroller)
require annotated-range edits and associated notifications that this view
does not implement. The existing native text-input callbacks still route
replacement through selection/preedit/raw input. An exact, generation-bound
replacement/composition bridge is needed before those services can preserve
rich formatting, intercepted edits and one-transaction undo. Returning a
semantic hint is not evidence that those services are implemented.

## Validation

The portable `native_text_input_options` unit tests cover all 43 values,
ordered/empty/default fallback, privacy precedence, active-client lifetime,
deduplicated delivery and reentrant policy/client changes. The native ABI
layout test checks the unchanged old vtable and the additive interface.

The dedicated main-thread `text_input_options` integration executable creates
a real FerroUI window and RichEditBox, queries the actual AppKit view's
`NSTextContent` conformance and getter, compares it with the public system
constants, exercises all 43 names and a live NameValue mutation, checks
sensitive/suggestion changes, and checks focus loss/refocus. It invokes
`setMarkedText:selectedRange:replacementRange:` on that view, changes options,
and asserts that the native marked range, input-context identity, rich
document snapshot and undo state remain intact. On pre-14 systems it checks
the `nil` availability fallback.

Commands on macOS:

```sh
cargo test --locked -p ferroui-native --lib native_input
cargo test --locked -p ferroui-native --lib vtable_sizes
cargo test --locked -p ferroui-native --test text_input_options
```

The new tests require fresh execution after this source tranche. The AppKit
harness checks the concrete protocol/bridge, not the contents of third-party
IME candidate windows or the behavior of a real password/AutoFill session.

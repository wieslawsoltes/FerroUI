#!/usr/bin/env python3
"""Reproduce the pinned WinUI rich text source/member map using only Git/Python.

Source evidence is read from immutable Git objects. This scanner does not pretend
to be a C++ compiler: include resolution is source-relative or unambiguous by
path suffix, and callable candidates are lexical. Exact public member IDs come
from the upstream IntelliSense XML. Reviewed status is never inferred from names.
"""

from __future__ import annotations

import argparse
import collections
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import subprocess
import sys
import xml.etree.ElementTree as ET


ROOT = Path(__file__).resolve().parents[2]
DESTINATION = ROOT / "docs/porting/winui-rich-text"
PRIMARY_ROOTS = {
    "dxaml/xcp/core/text/": 230,
    "dxaml/xcp/core/native/text/": 62,
    "dxaml/xcp/components/text/": 23,
}
TEXT_API = "packaging/Intellisense/generated/Microsoft.UI.Text.xml"
XAML_API = "packaging/Intellisense/generated/Microsoft.UI.Xaml.xml"
CODE_SUFFIXES = {".h", ".hpp", ".hxx", ".cpp", ".cxx", ".cc", ".inl", ".c", ".cs", ".idl"}
HEADER_SUFFIXES = {".h", ".hpp", ".hxx", ".inl"}
STATES = {"pending", "partial", "complete", "reuse_candidate", "reference"}
INCLUDE = re.compile(r'^\s*#\s*include\s*([<"])([^>"\r\n]+)[>"]', re.MULTILINE)
LEXICAL_IGNORED = re.compile(r'//[^\n]*|/\*[\s\S]*?\*/|"(?:\\.|[^"\\])*"|\'(?:\\.|[^\'\\])*\'')
SCOPED_CALLABLE = re.compile(r"\b((?:[A-Za-z_]\w*::)+(?:~?[A-Za-z_]\w*|operator\s*[^\s(]+))\s*\(")
XAML_CONTROL_TYPES = {
    "RichTextBlock", "RichTextBlockOverflow", "RichEditBox", "RichEditBoxAutomationPeer",
    "RichTextBlockAutomationPeer", "RichTextBlockOverflowAutomationPeer", "RichEditClipboardFormat",
    "RichEditBoxTextChangingEventArgs", "RichEditBoxSelectionChangingEventArgs",
    "TextControlCopyingToClipboardEventArgs", "TextControlCuttingToClipboardEventArgs",
    "TextControlPasteEventArgs", "TextCompositionStartedEventArgs", "TextCompositionChangedEventArgs",
    "TextCompositionEndedEventArgs", "TextChangedEventArgs", "TextSelectionChangedEventArgs",
    "CandidateWindowBoundsChangedEventArgs", "TextReadingOrder", "TextLineBounds", "TextWrapping",
    "TextTrimming", "TextAlignment", "LineStackingStrategy", "OpticalMarginAlignment",
    "CandidateWindowAlignment", "ContextMenuEventArgs", "ContextMenuOpeningEventHandler",
    "InputScope", "InputScopeName", "InputScopeNameValue",
    "FontCapitals", "FontEastAsianLanguage", "FontEastAsianWidths", "FontFraction",
    "FontNumeralAlignment", "FontNumeralStyle", "FontVariants",
}
XAML_TEXT_AUTOMATION_TYPES = {
    "ITextProvider", "ITextProvider2", "ITextRangeProvider", "ITextRangeProvider2",
    "ITextEditProvider", "ITextChildProvider", "SupportedTextSelection", "TextUnit",
    "TextPatternRangeEndpoint", "AutomationActiveEnd", "AutomationAnimationStyle",
    "AutomationBulletStyle", "AutomationCaretBidiMode", "AutomationCaretPosition",
    "AutomationFlowDirections", "AutomationOutlineStyles", "AutomationStyleId",
    "AutomationTextDecorationLineStyle", "AutomationTextEditChangeType",
    "AnnotationType", "AutomationAnnotation", "AnnotationPatternIdentifiers",
}
XAML_TEXT_AUTOMATION_MEMBERS = {
    "F:Microsoft.UI.Xaml.Automation.Peers.PatternInterface.Text",
    "F:Microsoft.UI.Xaml.Automation.Peers.PatternInterface.Text2",
    "F:Microsoft.UI.Xaml.Automation.Peers.PatternInterface.TextRange",
    "F:Microsoft.UI.Xaml.Automation.Peers.PatternInterface.TextEdit",
    "F:Microsoft.UI.Xaml.Automation.Peers.PatternInterface.TextChild",
    "F:Microsoft.UI.Xaml.Automation.Peers.AutomationEvents.TextPatternOnTextChanged",
    "F:Microsoft.UI.Xaml.Automation.Peers.AutomationEvents.TextPatternOnTextSelectionChanged",
    "F:Microsoft.UI.Xaml.Automation.Peers.AutomationEvents.TextEditTextChanged",
    "M:Microsoft.UI.Xaml.Automation.Peers.AutomationPeer.RaiseTextEditTextChangedEvent(Microsoft.UI.Xaml.Automation.AutomationTextEditChangeType,Windows.Foundation.Collections.IVectorView{System.String})",
}
RELATED_TYPES = XAML_CONTROL_TYPES | {
    "TextElement", "TextElementCollection", "TextElementTreeCollection", "Block", "Paragraph",
    "BlockCollection", "Inline", "InlineCollection", "InlineUIContainer", "Run", "CRun", "Span",
    "Bold", "Italic", "Underline", "Hyperlink", "HyperlinkClickEventArgs", "LineBreak", "Glyphs",
    "Typography", "TextPointer", "TextPointerWrapper", "TextHighlighter", "TextHighlighterBase",
    "TextHighlighterCollection", "TextRange", "TextRangeCollection", "TextBlock", "TextBlockView",
    "TextBox", "TextBoxBase", "TextBoxView", "TextSelection", "TextSelectionNotify", "TextContainer",
    "TextView", "TextAdapter", "TextRangeAdapter", "TextProvider", "TextRangeProvider",
    "TextNavigationHelper", "TextSelectionGripper", "TextBoxAutomationPeer", "TextBlockAutomationPeer",
    "TextBoxBaseAutomationPeer", "InputScope", "InputScopeName", "InputScopeNameFactory", "FontFamily",
    "FontAndScriptServices", "InheritedProperties", "RichTextServicesHelper", "GripperPopup",
    "WindowLessSiteHost", "TextControlFlyoutHelper", "TextControlHelper", "TextBoxPlaceholderTextHelper",
    "TextAlternativesAsyncOperation", "CaretBrowsingGlobal", "BlockTextElement",
    "CaretBrowsingCaret", "CaretBrowsingDialog", "HyperLinkAutomationPeer", "TextOptions",
    "TextCompositionEventArgs",
}
RELATED_NAMES = {name.lower() for name in RELATED_TYPES}
EXPLICIT_REFERENCES = {
    "LICENSE", "NOTICE.md", TEXT_API, XAML_API,
    "eng/productmetadata.props", "eng/Versions.props", "eng/winuidetails.targets", "eng/folderpaths.props",
    "eng/adhocapp.targets", "eng/consumebinaries.targets", "eng/externalbinaries.targets",
    "dxaml/xcp/pal/inc/paltext.h",
    "dxaml/xcp/tools/XCPTypesAutoGen/XamlOM/Model/Microsoft.UI.Xaml.Documents.cs",
    "dxaml/xcp/tools/XCPTypesAutoGen/XamlOM/Model/Microsoft.UI.Xaml.Controls.cs",
    "dxaml/xcp/tools/XCPTypesAutoGen/XamlOM/Model/Microsoft.UI.Xaml.Input.cs",
    "dxaml/xcp/tools/XCPTypesAutoGen/XamlOM/Model/Microsoft.UI.Xaml.cs",
    "dxaml/xcp/tools/XCPTypesAutoGen/XamlOM/Model/Microsoft.UI.Xaml.Automation.Peers.cs",
    "dxaml/xcp/tools/XCPTypesAutoGen/XamlOM/Model/Microsoft.UI.Xaml.Automation.Provider.cs",
    "dxaml/xcp/tools/XCPTypesAutoGen/XamlOM/Model/Microsoft.UI.Xaml.Automation.Text.cs",
    "dxaml/xcp/tools/XCPTypesAutoGen/XamlOM/Model/Microsoft.UI.Xaml.Automation.cs",
    "dxaml/xcp/tools/XCPTypesAutoGen/XamlOM/Model/Microsoft.UI.Xaml.Internal.cs",
    "dxaml/xcp/win/shared/UIAPatternProviderWrapper.cpp",
    "dxaml/xcp/win/inc/UIAPatternProviderWrapper.h",
    "dxaml/xcp/win/shared/UIAWindow.cpp",
    "dxaml/xcp/win/inc/UIAWindow.h",
    "dxaml/xcp/dxaml/lib/AutomationPeer_Partial.cpp",
    "dxaml/xcp/dxaml/lib/AutomationPeer_Partial.h",
    "dxaml/xcp/core/core/elements/uielement.cpp",
    "dxaml/xcp/core/dll/CoreImports.cpp",
    "dxaml/xcp/components/DependencyObject/DependencyProperty.cpp",
    "dxaml/xcp/components/DependencyObject/PropertySystem.cpp",
    "dxaml/xcp/components/metadata/inc/EnumDefs.g.h",
    "dxaml/xcp/dxaml/idl/winrt/main/microsoft.ui.xaml.idl",
    "dxaml/xcp/dxaml/themes/generic.xaml",
    "controls/dev/CommonStyles/RichEditBox_themeresources.xaml",
    "controls/dev/CommandBarFlyout/TextCommandBarFlyout.cpp",
    "controls/dev/CommandBarFlyout/TextCommandBarFlyout.h",
    "controls/dev/CommonStyles/TextBlock_themeresources.xaml",
    "controls/dev/CommonStyles/TextBox_themeresources.xaml",
    "controls/dev/CommonStyles/Hyperlink_themeresources.xaml",
}
FORMAT_ROOT = "src/FerroUI.Base/media/text_formatting/"
DOCUMENT_ROOT = "src/FerroUI.Controls/documents/"


class GitObjects:
    def __init__(self, reference: Path, commit: str):
        self.reference = reference
        tree = subprocess.check_output(
            ["git", "-C", str(reference), "ls-tree", "-r", "-z", commit], stderr=subprocess.PIPE
        )
        self.blobs = {}
        for entry in tree.split(b"\0"):
            if not entry:
                continue
            attributes, path = entry.split(b"\t", 1)
            _mode, kind, object_id = attributes.decode().split()
            if kind == "blob":
                self.blobs[path.decode()] = object_id
        self.reader = subprocess.Popen(
            ["git", "-C", str(reference), "cat-file", "--batch"],
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
        )
        self.cache: dict[str, bytes] = {}

    def read(self, path: str) -> bytes:
        if path not in self.cache:
            assert self.reader.stdin and self.reader.stdout
            self.reader.stdin.write((self.blobs[path] + "\n").encode())
            self.reader.stdin.flush()
            header = self.reader.stdout.readline().decode().split()
            if len(header) != 3 or header[1] != "blob":
                raise ValueError(f"Cannot read pinned Git blob for {path}: {header}")
            self.cache[path] = self.reader.stdout.read(int(header[2]))
            if self.reader.stdout.read(1) != b"\n":
                raise ValueError(f"Invalid Git object framing for {path}")
        return self.cache[path]

    def close(self):
        if self.reader.stdin:
            self.reader.stdin.close()
        self.reader.wait()
        if self.reader.stdout:
            self.reader.stdout.close()
        if self.reader.stderr:
            self.reader.stderr.close()


def type_stem(path: str) -> str:
    name = PurePosixPath(path).stem.lower()
    name = re.sub(r"(?:_partial|\.g)$", "", name)
    return name


def selected_reason(path: str) -> str | None:
    for prefix in PRIMARY_ROOTS:
        if path.startswith(prefix):
            return prefix
    if path in EXPLICIT_REFERENCES:
        return "explicit dependency/reference"
    if path.startswith("dxaml/xcp/pal/win/") and PurePosixPath(path).name.startswith("DWrite"):
        return "platform font and script service"
    if path.startswith("dxaml/xcp/") and type_stem(path) in RELATED_NAMES:
        return "related document/control/service declaration or projection"
    if path.startswith("dxaml/test/native/external/foundation/text/"):
        return "upstream text behavior test family"
    if path.startswith("dxaml/test/managed/controls/RichTextBlock/"):
        return "upstream rich document managed tests"
    if path.startswith("dxaml/test/managed/controls/TextBox/") and "TextEditingTests" in path:
        return "upstream editing tests"
    if path.startswith("dxaml/test/native/external/foundation/graphics/rendering/") and re.search(
        r"(RichTextBlock|TextHighlighter|TextBlock|CaretBrowsing)", PurePosixPath(path).name
    ):
        return "upstream text rendering tests"
    return None


def classify(path: str) -> tuple[str, str, list[str]]:
    lower = path.lower()
    stem = type_stem(path)
    if "/test/" in lower or "/unittests/" in lower:
        return "upstream tests", "reference", []
    if path in {TEXT_API, XAML_API} or "/xamlom/model/" in lower or lower.endswith(".idl") or lower.endswith("/enumdefs.g.h"):
        return "public API metadata", "reference", []
    if "/winrtgeneratedclasses/" in lower:
        return "generated WinRT projection", "reference", []
    if lower.endswith((".vcxproj", ".props", ".targets", ".pl")) or path in {"LICENSE", "NOTICE.md"}:
        return "build, tooling and attribution", "reference", []
    if lower.endswith(".xaml"):
        return "theme reference", "reference", ["src/FerroUI.Themes.WinUI/"]
    if path.startswith("controls/dev/CommandBarFlyout/TextCommandBarFlyout."):
        return "text command flyout", "pending", ["src/FerroUI.Controls/rich_edit_box_format_commands.rs"]
    if "automationpeer" in lower or stem in {"textprovider", "textrangeprovider", "uiapatternproviderwrapper"}:
        return "text automation", "pending", ["src/FerroUI.Controls/automation/"]
    if "/pal/" in lower or "/fonts/" in lower or stem in {"fontfamily", "fontandscriptservices", "fonts", "typefacecollection"}:
        return "font and script services", "reuse_candidate", ["src/FerroUI.Base/media/font_manager.rs", FORMAT_ROOT + "text_shaper.rs"]
    if "/classification/" in lower or "/unicodedata/" in lower or "/shaping/" in lower:
        return "Unicode and shaping", "reuse_candidate", [FORMAT_ROOT + "unicode/"]
    if "/richtextservices/textformatter/ls" in lower or "/richtextservices/inci/ls.h" in lower or "lineservices" in lower:
        return "LineServices adapters", "pending", [FORMAT_ROOT + "text_formatter_impl.rs", FORMAT_ROOT + "text_line_impl.rs"]
    if "/richtextservices/" in lower:
        return "rich formatting contracts and structures", "reuse_candidate", [FORMAT_ROOT]
    if "/blocklayout/" in lower:
        return "block layout", "pending", ["src/FerroUI.Controls/rich_text_block.rs", FORMAT_ROOT]
    if "/core/native/text/common/" in lower or stem.startswith(("hw", "d2d")) or stem == "dwritetextrenderer":
        return "native rendering adapters", "reuse_candidate", ["src/FerroUI.Base/media/drawing_context.rs"]
    if "/core/native/text/controls/" in lower:
        return "editable controls and text host", "pending", ["src/FerroUI.Controls/rich_edit_box.rs", "src/FerroUI.Controls/rich_text/"]
    if "/components/text/" in lower or "highlighter" in lower or stem == "textrangecollection":
        return "highlighters and text components", "pending", ["src/FerroUI.Controls/rich_text/"]
    if any(word in lower for word in ("selection", "gripper", "textposition", "textpointer", "inputprocessor", "navigation", "textadapter", "textrangeadapter", "textcontainer", "textview")):
        return "selection, positions and input", "pending", [DOCUMENT_ROOT, "src/FerroUI.Base/input/text_input/"]
    if "richtextblock" in lower:
        return "rich text controls and overflow", "pending", ["src/FerroUI.Controls/rich_text_block.rs", "src/FerroUI.Controls/rich_text_block_overflow.rs"]
    if "/dxaml/lib/" in lower:
        return "control projection and services", "pending", ["src/FerroUI.Controls/"]
    if stem in {"block", "blocktextelement", "paragraph", "blockcollection", "textschema", "textelementcollection", "richtextserviceshelper"}:
        return "block document model", "pending", [DOCUMENT_ROOT]
    if stem in {"inline", "run", "crun", "span", "bold", "italic", "underline", "linebreak", "inlinecollection", "inlineuicontainer", "textelement", "hyperlink", "typography"}:
        return "document inlines and formatting", "reuse_candidate", [DOCUMENT_ROOT]
    if stem in {"textblock", "textblockview"}:
        return "plain text control and view", "reuse_candidate", ["src/FerroUI.Controls/text_block.rs"]
    return "shared text infrastructure", "pending", [FORMAT_ROOT, DOCUMENT_ROOT]


def callable_candidates(source: str) -> list[dict]:
    # Keep offsets/newlines stable when removing non-code. This deliberately
    # does not claim to enumerate unqualified, macro-generated or inherited APIs.
    clean = LEXICAL_IGNORED.sub(lambda m: "".join("\n" if c == "\n" else " " for c in m.group()), source)
    candidates = []
    for match in SCOPED_CALLABLE.finditer(clean):
        start = match.end() - 1
        depth = 1
        index = start + 1
        while index < len(clean) and depth:
            depth += (clean[index] == "(") - (clean[index] == ")")
            index += 1
        if depth:
            continue
        suffix = clean[index:index + 160]
        if not re.match(r"\s*(?:(?:const|noexcept|override|final)\b\s*)*(?:\{|:)", suffix):
            continue
        candidates.append({
            "name": match.group(1), "line": source.count("\n", 0, match.start()) + 1,
            "kind": "scoped_callable_definition_candidate", "state": "unreviewed",
        })
    return candidates


def validate_review(entry: dict, label: str):
    if entry.get("state", "pending") not in STATES:
        raise ValueError(f"Invalid state for {label}")
    for target in entry.get("targets", []):
        target_path = target["path"] if isinstance(target, dict) else target
        candidate = (ROOT / target_path).resolve()
        if not candidate.is_relative_to(ROOT) or not candidate.exists():
            raise ValueError(f"Missing/invalid reviewed Rust target {target_path} for {label}")
        if isinstance(target, dict) and target.get("symbol"):
            if not candidate.is_file() or target["symbol"] not in candidate.read_text():
                raise ValueError(f"Missing target symbol {target['symbol']} for {label}")
    for test in entry.get("tests", []):
        path = (ROOT / test["path"]).resolve()
        if not path.is_relative_to(ROOT) or not path.is_file():
            raise ValueError(f"Missing test path {test['path']} for {label}")
        if test.get("symbol") and test["symbol"] not in path.read_text():
            raise ValueError(f"Missing test symbol {test['symbol']} for {label}")
    if entry.get("state") == "complete":
        if not entry.get("targets") or not entry.get("tests") or not entry.get("reason"):
            raise ValueError(f"Complete mapping lacks Rust target, tests or review reason: {label}")


def validate_mapping(mapping: dict):
    deviations = set(re.findall(r"\|\s*(RT-[A-Z]\d+)\s*\|", (DESTINATION / "DEVIATIONS.md").read_text()))
    for group in ("files", "api_members"):
        for label, entry in mapping[group].items():
            unknown = set(entry.get("deviations", [])) - deviations
            if unknown:
                raise ValueError(f"Undefined deviations for {label}: {sorted(unknown)}")
    for name, capability in mapping["capabilities"].items():
        if capability.get("state") not in STATES or not capability.get("description"):
            raise ValueError(f"Invalid capability record: {name}")
    for run in mapping["validation"]:
        if not run.get("command") or not run.get("result"):
            raise ValueError("A validation record requires the actual command and result")


def api_type_plan(owner: str) -> dict:
    name = owner.rsplit(".", 1)[-1]
    if ".Automation." in owner:
        return {
            "batch": "R7",
            "candidate_paths": ["src/FerroUI.Controls/automation/", "src/Windows/FerroUI.Win32/automation/"],
            "remaining_review": "Review text provider/range/peer members individually, including attributes, units, platform marshalling, weak lifetime, events and native clients. Text2 caret, TextChild, accepted-composition TextEdit and public peer/EventsSource events have mapped implementations; conversion-target backend data, annotations, unsupported attributes and native client conformance remain explicit pending contracts.",
        }
    if name in {"TextHighlighter", "TextHighlighterBase", "TextRange"} and ".Documents." in owner:
        return {
            "batch": "R6",
            "candidate_paths": [DOCUMENT_ROOT + "text_highlighter.rs", DOCUMENT_ROOT + "text_highlight_range.rs", DOCUMENT_ROOT + "text_highlight_merge.rs"],
            "remaining_review": "Review range value semantics, defaults, mutable brush invalidation and layering. The projection-only base can map to FerroObject; TextRange needs a Rust name distinct from the live document range.",
        }
    if name == "Typography":
        return {
            "batch": "R6",
            "candidate_paths": [DOCUMENT_ROOT + "typography.rs", DOCUMENT_ROOT + "typography_format.rs", "src/FerroUI.Base/media/font_typography.rs", FORMAT_ROOT + "text_shaper.rs"],
            "remaining_review": "All 43 named attached properties, 86 accessors, seven enum types and 38 values now map to inherited FerroUI properties and ordered OpenType features. Fresh tests cover inheritance, snapshots, mutable raw overrides, every text consumer, real HarfBuzz shaping and XAML. Keep native font/script raster conformance, exact native feature collection order and unsupported RTF typography outside completed claims; see TYPOGRAPHY.md and RT-I25.",
        }
    if name in {"FontCapitals", "FontEastAsianLanguage", "FontEastAsianWidths", "FontFraction", "FontNumeralAlignment", "FontNumeralStyle", "FontVariants"}:
        return {
            "batch": "R6",
            "candidate_paths": ["src/FerroUI.Base/media/font_typography.rs", DOCUMENT_ROOT + "typography_format.rs"],
            "remaining_review": "The seven Rust enums and all 38 pinned numeric constants have exact value and feature-tag tests. Constant/type identity is complete; engine behavior remains part of the partial Typography contract, not an implication of enum coverage.",
        }
    if name == "Glyphs":
        return {"batch": "R8", "candidate_paths": ["src/FerroUI.Base/media/glyph_run.rs"], "remaining_review": "The existing glyph run is a rendering contract; the document Glyphs control's parsing, indices, font URI, properties and layout need their own review."}
    if name in {"TextPointer", "LogicalDirection"}:
        return {"batch": "R1/R2", "candidate_paths": [DOCUMENT_ROOT + "text_position.rs"], "remaining_review": "Review UTF-16 positions, gravity, element/visual parents and geometry; public text offsets exclude native synthetic element markers."}
    if name in {"UnderlineStyle", "HyperlinkClickEventArgs"}:
        return {"batch": "R1/R5", "candidate_paths": [DOCUMENT_ROOT + "hyperlink.rs"], "remaining_review": "Review hyperlink presentation, shared handled arguments, activation and navigation behavior."}
    if ".Documents." in owner:
        stem = re.sub(r"(?<!^)(?=[A-Z])", "_", name).lower()
        stem = {"InlineUIContainer": "inline_ui_container"}.get(name, stem)
        return {"batch": "R1/R2", "candidate_paths": [DOCUMENT_ROOT + stem + ".rs"], "remaining_review": "Review each document property, collection identity, inherited formatting, invalidation, markup and observable edge case against the pinned source."}
    if name.startswith("RichTextBlock"):
        return {"batch": "R2/R7", "candidate_paths": ["src/FerroUI.Controls/rich_text_block.rs", "src/FerroUI.Controls/rich_text_block_overflow.rs", "src/FerroUI.Controls/rich_text_layout.rs"], "remaining_review": "Review document ownership, line/overflow continuation, selection, rendering, input, property notifications and accessibility. Existing formatter output is not exact native engine or pixel parity."}
    if owner.startswith("Microsoft.UI.Xaml.Input.") and name in {"InputScope", "InputScopeName", "InputScopeNameValue"}:
        return {
            "batch": "R5/R8",
            "candidate_paths": ["src/FerroUI.Base/input/input_scope.rs", "src/FerroUI.Base/input/input_scope_name_value.rs", "src/FerroUI.Base/input/text_input/text_input_options.rs", "src/FerroUI.Controls/rich_edit_box.rs"],
            "remaining_review": "The 43 exact enum constants have individual mappings. Review object defaults and constructors, ordered shared Names, live mutation and weak subscription lifetime, runtime/compiled markup and immutable scope transport separately from host behavior. RichEditBox projects the full ordered snapshot. IMM32 forwards the complete equal-weight array; single-purpose backends choose the first name or an explicit fallback. Browser exact edits, IBus ContentType and optional AppKit semantic hints have separate mappings. Real OS keyboard/prediction sessions, platform-specific unsupported hints and WinRT ABI remain separate contracts.",
        }
    if ".Controls." in owner:
        return {"batch": "R5/R7", "candidate_paths": ["src/FerroUI.Controls/rich_edit_box.rs", "src/FerroUI.Controls/rich_edit_box_text_input_method_client.rs"], "remaining_review": "Review editable control members, native event timing, IME/clipboard capabilities, templates, accelerators and automation. Missing members are omitted, not stubbed."}
    if name in {"RichEditTextDocument", "RichEditTextRange", "ITextRange", "ITextSelection"}:
        return {"batch": "R3/R8", "candidate_paths": ["src/FerroUI.Controls/rich_text/rich_edit_text_document.rs"], "remaining_review": "Audit the external TOM contract member by member. The Rust facade uses typed errors, UTF-16 text ranges and shared editor services; remaining option behavior, ABI and native units are not inferred from similarly named methods. Eight native-valued units including shaped Cluster, explicit Rust Read/Write adapters, paragraph tabs and structured Unicode case conversion have individual mappings; locale/engine conformance and unmapped members remain pending."}
    if name in {"ITextCharacterFormat", "ITextParagraphFormat"}:
        return {"batch": "R3/R8", "candidate_paths": [DOCUMENT_ROOT + "character_format.rs"], "remaining_review": "Audit native formatting units, mixed values, font/brush representation, tabs, lists, language resolution and formatting flags. Inherited language and supported RTF locale IDs have explicit mappings; Rust patches cover only reviewed fields."}
    if name in {"TextGetOptions", "TextSetOptions"}:
        return {"batch": "R4/R8", "candidate_paths": ["src/FerroUI.Controls/rich_text/rich_edit_text_document.rs", "src/FerroUI.Controls/rich_text/rtf.rs", "src/FerroUI.Controls/rich_text/rich_text_transfer.rs"], "remaining_review": "Review every import/export option and supported content feature against its mapped implementation. Typed get/set flags reject unsupported modes explicitly; these errors do not implement omitted content semantics or native stream APIs."}
    if name == "FontWeights":
        return {"batch": "R6", "candidate_paths": ["src/FerroUI.Base/media/font_weight.rs", "src/FerroUI.Base/markup_types/values.rs"], "remaining_review": "The eleven pinned predefined weights reuse existing FontWeight associated constants, typed markup fields and named parsing. The independent pinned-source audit compares every value; existing runtime tests cover representative conversion and metadata behavior. FontWeights static-class/namespace ABI, arbitrary integer validation and native font selection are not implied by completed named-value mappings; see TYPOGRAPHY.md."}
    if owner.startswith("Microsoft.UI.Text."):
        return {"batch": "R3/R8", "candidate_paths": ["src/FerroUI.Controls/rich_text/", DOCUMENT_ROOT], "remaining_review": "The native implementation is external to the pinned repository. Review this exact enum/format/service contract and its consumers before claiming equivalent behavior."}
    return {"batch": "R2/R6", "candidate_paths": ["src/FerroUI.Base/media/", "src/FerroUI.Controls/rich_text_layout.rs"], "remaining_review": "Audit shared layout/rendering enum values and all consumers, including unsupported values and native defaults."}


def public_api(reader: GitObjects, mapping: dict) -> dict:
    members = []
    for source in (TEXT_API, XAML_API):
        root = ET.fromstring(reader.read(source))
        for node in root.findall("./members/member"):
            identifier = node.attrib["name"]
            name = identifier[2:].split("(", 1)[0]
            owner = name if identifier.startswith("T:") else name.rsplit(".", 1)[0]
            if source == XAML_API:
                if not name.startswith("Microsoft.UI.Xaml.Documents.") and owner.split(".")[-1] not in XAML_CONTROL_TYPES | XAML_TEXT_AUTOMATION_TYPES and identifier not in XAML_TEXT_AUTOMATION_MEMBERS:
                    continue
            entry = {
                "id": identifier, "source": source, "owner": owner,
                "kind": identifier[0], "state": "pending", "targets": [], "tests": [],
            }
            if source == TEXT_API:
                entry["implementation_source"] = "external WinUIEdit/Microsoft.UI.Text implementation"
            if identifier in mapping["api_members"]:
                entry.update(mapping["api_members"][identifier])
                validate_review(entry, identifier)
            members.append(entry)
    members.sort(key=lambda member: member["id"])
    identifiers = [member["id"] for member in members]
    if len(set(identifiers)) != len(identifiers):
        raise ValueError("Upstream public member identifiers are not unique")
    extra = set(mapping["api_members"]) - set(identifiers)
    if extra:
        raise ValueError(f"Reviewed API members outside the source inventory: {sorted(extra)}")
    return {
        "schema_version": 1, "reference": mapping["reference"],
        "coverage": "Exact entries of the selected upstream IntelliSense XML; not undocumented/internal or inherited members.",
        "type_plans": {owner: api_type_plan(owner) for owner in sorted({member["owner"] for member in members})},
        "members": members,
    }


def build_inventory(reader: GitObjects, mapping: dict) -> dict:
    selected = {path: selected_reason(path) for path in reader.blobs if selected_reason(path)}
    for prefix, count in PRIMARY_ROOTS.items():
        actual = sum(path.startswith(prefix) for path in selected)
        if actual != count:
            raise ValueError(f"Pinned primary scope changed: {prefix}: expected {count}, found {actual}; review the source pin and scope")
    extra = set(mapping["files"]) - set(selected)
    if extra:
        raise ValueError(f"Reviewed files outside source scope: {sorted(extra)}")
    by_name: dict[str, list[str]] = collections.defaultdict(list)
    by_lower = {path.lower(): path for path in reader.blobs}
    for path in reader.blobs:
        if PurePosixPath(path).suffix.lower() in HEADER_SUFFIXES:
            by_name[PurePosixPath(path).name.lower()].append(path)
    for candidates in by_name.values():
        candidates.sort()

    def resolve(owner: str, header: str) -> tuple[str, list[str]]:
        header = header.replace("\\", "/")
        relative = os.path.normpath(str(PurePosixPath(owner).parent / header)).replace("\\", "/").lower()
        if relative in by_lower:
            return "relative_path", [by_lower[relative]]
        if header.lower() in by_lower:
            return "repository_path", [by_lower[header.lower()]]
        candidates = by_name.get(PurePosixPath(header).name.lower(), [])
        suffix = [p for p in candidates if p.lower().endswith("/" + header.lower())]
        if len(suffix) == 1:
            return "unique_path_suffix", suffix
        return ("ambiguous" if suffix else "external_or_generated"), suffix

    edges: dict[str, list[dict]] = {}

    def includes(path: str) -> list[dict]:
        if path in edges:
            return edges[path]
        source = reader.read(path).decode("utf-8-sig", errors="replace")
        result = []
        for match in INCLUDE.finditer(source):
            resolution, candidates = resolve(path, match.group(2))
            result.append({
                "header": match.group(2), "line": source.count("\n", 0, match.start()) + 1,
                "style": "system" if match.group(1) == "<" else "quoted",
                "resolution": resolution, "candidates": candidates,
            })
        edges[path] = result
        return result

    entries = []
    for path, reason in sorted(selected.items()):
        data = reader.read(path)
        component, state, candidates = classify(path)
        entry = {
            "path": path, "git_blob": reader.blobs[path], "sha256": hashlib.sha256(data).hexdigest(),
            "bytes": len(data), "lines": data.count(b"\n") + bool(data and not data.endswith(b"\n")),
            "scope_reason": reason, "component": component, "state": state,
            "reuse_or_destination_candidates": candidates, "targets": [], "tests": [],
        }
        if PurePosixPath(path).suffix.lower() in CODE_SUFFIXES:
            entry["includes"] = includes(path)
            entry["callable_candidates"] = callable_candidates(data.decode("utf-8-sig", errors="replace"))
        if path in mapping["files"]:
            entry.update(mapping["files"][path])
            validate_review(entry, path)
        entries.append(entry)

    # Traverse only unambiguous source-level includes. Ambiguity and missing SDK
    # headers remain explicit boundary edges rather than guessed resolutions.
    queue = collections.deque(sorted(path for path in selected if PurePosixPath(path).suffix.lower() in CODE_SUFFIXES))
    seen = set(queue)
    while queue:
        for edge in includes(queue.popleft()):
            if edge["resolution"] in {"ambiguous", "external_or_generated"}:
                continue
            for candidate in edge["candidates"]:
                if candidate not in seen:
                    seen.add(candidate)
                    queue.append(candidate)
    dependencies = []
    for path in sorted(seen - set(selected)):
        data = reader.read(path)
        dependencies.append({
            "path": path, "git_blob": reader.blobs[path], "sha256": hashlib.sha256(data).hexdigest(),
            "state": "framework_dependency_review", "includes": edges.get(path, []),
        })
    boundaries = []
    for path in sorted(seen):
        for edge in edges.get(path, []):
            if edge["resolution"] in {"ambiguous", "external_or_generated"}:
                boundaries.append({"from": path, **edge})
    return {
        "schema_version": 1, "reference": mapping["reference"],
        "primary_roots": PRIMARY_ROOTS,
        "scope_policy": "Complete primary roots; explicit platform/build/metadata dependencies; related text type definitions and projections; selected upstream text test families.",
        "member_policy": "Public API exact XML IDs are in api-members.json. Internal callable candidates are lexical, unreviewed and non-exhaustive. Pending files mean all members unverified.",
        "dependency_policy": "Transitive source-relative/unambiguous-suffix include graph. No preprocessor, include-directory, generated-header or ABI resolution is claimed. Boundary edges stop traversal.",
        "files": entries, "framework_dependencies": dependencies, "include_boundaries": boundaries,
    }


def render_tracking(inventory: dict) -> str:
    lines = [
        "# Rich text source-file tracking", "",
        "Generated by `scripts/porting/winui-rich-text-inventory.py`; edit `mapping.json`, then regenerate.", "",
        "Every file in the three primary roots is retained, including build and data files. A reuse/destination candidate is a plan, not an implementation claim. `pending` means all members remain unverified; `reuse_candidate` means an existing contract still needs a WinUI behavior audit. Exact public member IDs are in [api-members.json](api-members.json).", "",
    ]
    groups: dict[str, list[dict]] = collections.defaultdict(list)
    for entry in inventory["files"]:
        groups[entry["component"]].append(entry)
    for component, entries in sorted(groups.items()):
        lines += [f"## {component.capitalize()}", "", "| Upstream file | State | Reviewed Rust target |", "|---|---|---|"]
        for entry in entries:
            targets = [target["path"] if isinstance(target, dict) else target for target in entry["targets"]]
            display = ", ".join(f"`{target}`" for target in targets) or "—"
            lines.append(f"| `{entry['path']}` | {entry['state']} | {display} |")
        lines.append("")
    return "\n".join(lines)


def render_progress(inventory: dict, api: dict, mapping: dict) -> str:
    states = collections.Counter(entry["state"] for entry in inventory["files"])
    member_states = collections.Counter(entry["state"] for entry in api["members"])
    lines = [
        "# Rich text port progress", "", "Generated from the pinned source inventory and reviewed `mapping.json`.", "",
        "## Inventory coverage", "", "| Scope | Files |", "|---|---:|",
    ]
    for prefix, count in PRIMARY_ROOTS.items():
        lines.append(f"| `{prefix}` | {count} |")
    lines += [
        f"| Additional explicit dependencies, projections, metadata and test source | {len(inventory['files']) - sum(PRIMARY_ROOTS.values())} |",
        f"| Total selected source/reference files | {len(inventory['files'])} |",
        f"| Additional framework headers reached through unambiguous includes | {len(inventory['framework_dependencies'])} |",
        f"| Include boundary edges requiring SDK/generated-header/include-order review | {len(inventory['include_boundaries'])} |", "",
        "## Reviewed implementation status", "", "| State | Files | Public API entries |", "|---|---:|---:|",
    ]
    for state in sorted(STATES):
        lines.append(f"| {state} | {states[state]} | {member_states[state]} |")
    lines += [
        "", f"Public API inventory: **{len(api['members'])} exact documented member identifiers**. Type and enum entries are retained alongside properties, methods and events; this denominator is not a count of methods.", "",
        "`reference` entries are build/generated/test/metadata inputs, not completed runtime functionality. `reuse_candidate` is not verified parity. A `partial` file may contain working behavior while still having unmapped members. All claims of completion require target and test evidence. Internal lexical callable candidates do not establish complete member enumeration.", "",
        "## Implemented capability evidence", "",
    ]
    if mapping["capabilities"]:
        lines += ["| Capability | State | Evidence / remaining scope |", "|---|---|---|"]
        for name, value in mapping["capabilities"].items():
            lines.append(f"| {name} | {value['state']} | {value['description'].replace('|', '/')} |")
    else:
        lines.append("No runtime capability has been marked complete in the initial source-map batch.")
    lines += ["", "## Verification records", ""]
    if mapping["validation"]:
        for run in mapping["validation"]:
            lines.append(f"- `{run['command']}` — {run['result']}")
    else:
        lines.append("Runtime tests have not yet been recorded in this mapping. See [TESTS.md](TESTS.md) for the verification plan.")
    if not any("cargo test" in run["command"] for run in mapping["validation"]):
        lines += ["", "No successful Rust runtime-test command has been recorded yet; source-map checks and linked test definitions are traceability evidence only."]
    lines += ["", "Full WinUI rich text parity remains open until the pending member mappings and [DEVIATIONS.md](DEVIATIONS.md) boundaries are closed with evidence.", ""]
    return "\n".join(lines)


def render_api_map(api: dict) -> str:
    grouped = collections.defaultdict(list)
    for member in api["members"]:
        grouped[member["owner"]].append(member)
    lines = [
        "# Rich text public API mapping", "",
        "Generated from the exact pinned XML identifiers and reviewed `mapping.json`; do not edit this file.", "",
        "The type plan locates all retained public types, including entirely pending types. Candidate paths describe work allocation and do not assert that a Rust type exists or that it is equivalent. Every pending member remains listed in [api-members.json](api-members.json). A partial mapping records only the behavior described with its target, tests and deviation; it does not close the whole native contract.", "",
        "## Type and contract plan", "",
        "| Public owner | Exact entries | Partial / complete | Batch | Candidate implementation |", "|---|---:|---:|---|---|",
    ]
    for owner, members in sorted(grouped.items()):
        plan = api["type_plans"][owner]
        counts = collections.Counter(member["state"] for member in members)
        paths = ", ".join(f"`{path}`" for path in plan["candidate_paths"])
        lines.append(f"| `{owner}` | {len(members)} | {counts['partial']} / {counts['complete']} | {plan['batch']} | {paths} |")
    lines += ["", "## Reviewed member evidence", ""]
    for owner, members in sorted(grouped.items()):
        lines += [f"### `{owner}`", "", api["type_plans"][owner]["remaining_review"], ""]
        reviewed = [member for member in members if member["state"] != "pending"]
        if not reviewed:
            lines += ["All retained members remain pending.", ""]
            continue
        for member in reviewed:
            lines += [f"#### `{member['id']}`", "", f"State: **{member['state']}**. {member.get('reason', '')}", ""]
            for target in member["targets"]:
                path = target["path"] if isinstance(target, dict) else target
                symbol = " — `" + target["symbol"] + "`" if isinstance(target, dict) and target.get("symbol") else ""
                lines.append(f"- Target: [{path}](../../../{path}){symbol}.")
            for test in member["tests"]:
                symbol = " — `" + test["symbol"] + "`" if test.get("symbol") else ""
                lines.append(f"- Test: [{test['path']}](../../../{test['path']}){symbol}.")
            if member.get("deviations"):
                lines.append("- Differences: " + ", ".join(f"[{item}](DEVIATIONS.md)" for item in member["deviations"]) + ".")
            lines.append("")
    return "\n".join(lines)


def render_dependencies(inventory: dict) -> str:
    commit = inventory["reference"]["commit"]
    base = inventory["reference"]["repository"] + "/blob/" + commit + "/"
    def link(path: str) -> str:
        return f"[`{path}`]({base}{path})"
    lines = [
        "# Rich text source and dependency boundaries", "",
        "Generated alongside the inventory. Source links are pinned to the same commit as the implementation map.", "",
        "## Available source and external engines", "",
        "| Dependency | Evidence at the pinned revision | Disposition |", "|---|---|---|",
        f"| Editable RichEdit engine | {link('dxaml/xcp/core/native/text/Controls/TextBoxBase.cpp')}, `CTextBoxBase::InitializeTextServices`, loads `WinUIEdit.dll` and obtains `CreateTextServices`/`ShutdownTextServices`. | The public repository supplies the control and host; its engine implementation is external. Implement the observable editing contracts in Rust. |",
        f"| Rich text object model metadata | {link('eng/productmetadata.props')} names `Microsoft.UI.Text.winmd` from `WinUIDetailsLibPath` with `ImplementationDll` `WinUIEdit.dll`. {link(TEXT_API)} documents its public members. | Metadata/docs can define a semantic target; they do not provide implementation source. All documented members are retained in `api-members.json`. |",
        f"| Internal dependency package | {link('eng/Versions.props')} selects `Microsoft.Internal.WinUIDetails`; {link('eng/winuidetails.targets')} imports its targets. | Do not label externally delivered source/headers as present in this repository or redistributable under the repository's MIT license. |",
        f"| LineServices | {link('dxaml/xcp/core/text/RichTextServices/inci/Ls.h')} includes `Ptls-min.h`; {link('dxaml/xcp/core/text/RichTextServices/TextFormatter/LsTextFormatter.cpp')} constructs the callback table and calls the external line engine. | The header and engine implementation are absent from the tracked tree. Reuse FerroUI formatting and audit the semantic role of every visible host callback. |",
        f"| DirectWrite | {link('dxaml/xcp/pal/win/DWriteFontAndScriptServices.cpp')}, `DWriteFontFace`, `DWriteTextAnalyzer` and their PAL contracts wrap platform APIs. | Existing FerroUI font/shaper/backend APIs own platform integration; no second font/rasterization subsystem is introduced by the rich control. |",
        f"| Direct2D, WIC and text bitmap composition | {link('dxaml/xcp/core/native/text/Common/D2DTextDrawingContext.cpp')} and native `HWRenderTarget`/brush/bitmap classes. | Map rendering semantics onto `DrawingContext`, existing glyph runs, brushes, clipping and render invalidation. Backend rasterization is separate from document behavior. |",
        f"| Text input, linguistics and windowless host | {link('dxaml/xcp/core/native/text/Controls/TextServicesHost.cpp')}, `LinguisticsHelper`, `PrivateTextInputSettings`, `TextBoxUIManagerEventSink`, `WindowLessSiteHost`. | Reuse FerroUI input/IME/clipboard/dispatcher contracts. Proofing, candidate, handwriting and Windows UI-manager features remain explicit service boundaries. |",
        f"| Accessibility | Native rich-edit and rich-text automation peers plus {link('dxaml/xcp/core/inc/TextProvider.h')} / {link('dxaml/xcp/core/inc/TextRangeProvider.h')}. | Existing automation peers are integration points, but full text/range patterns and embedded-object behavior require implementations and platform verification. |", "",
        "## Include graph", "",
        f"The machine-readable graph records **{len(inventory['framework_dependencies'])} additional framework files** reached from the selected scope and **{len(inventory['include_boundaries'])} boundary edges**. It reads source-relative paths first and otherwise follows only unique repository path suffixes. Windows case-insensitive lookup is applied to source paths.", "",
        "This is a source-dependency audit, not a compiler dependency dump. Conditional compilation, build include-path precedence, SDK/private-package headers, generated headers, runtime interface acquisition and linker inputs cannot be inferred from filename matching. Ambiguous and absent includes are preserved with their referring file and line; traversal stops at those boundaries. `external_or_generated` means absent from this Git tree, not proof that an API is proprietary or unavailable elsewhere.", "",
        "General dependency-object, rendering, input, UI Automation and DXAML framework headers appear in the transitive graph because the text host uses the framework. They are not automatically added as independent source ports: their behavior must map to an existing FerroUI API or receive a separately reviewed implementation. A matching basename is insufficient evidence of such a mapping.", "",
        "## Frequent boundary includes", "", "| Include | Resolution | Referring edges |", "|---|---|---:|",
    ]
    counts = collections.Counter((entry["header"], entry["resolution"]) for entry in inventory["include_boundaries"])
    for (header, resolution), count in sorted(counts.items(), key=lambda item: (-item[1], item[0]))[:80]:
        lines.append(f"| `{header}` | {resolution} | {count} |")
    lines += ["", "The full boundary list and all candidate matches are retained in `inventory.json`; this summary is intentionally bounded.", ""]
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--reference", type=Path, default=os.environ.get("WINUI_XAML_ROOT"))
    parser.add_argument("--check", action="store_true", help="Compare generated records without writing")
    arguments = parser.parse_args()
    if not arguments.reference:
        parser.error("provide --reference or WINUI_XAML_ROOT (a local Git repository containing the pinned commit)")
    mapping = json.loads((DESTINATION / "mapping.json").read_text())
    if mapping.get("schema_version") != 1:
        raise ValueError("Unsupported mapping schema")
    validate_mapping(mapping)
    reader = GitObjects(arguments.reference, mapping["reference"]["commit"])
    try:
        inventory = build_inventory(reader, mapping)
        api = public_api(reader, mapping)
    finally:
        reader.close()
    outputs = {
        "inventory.json": json.dumps(inventory, indent=2, ensure_ascii=False) + "\n",
        "api-members.json": json.dumps(api, indent=2, ensure_ascii=False) + "\n",
        "API-MAP.md": render_api_map(api),
        "TRACKING.md": render_tracking(inventory),
        "PROGRESS.md": render_progress(inventory, api, mapping),
        "DEPENDENCIES.md": render_dependencies(inventory),
    }
    changed = []
    for name, content in outputs.items():
        path = DESTINATION / name
        if not path.is_file() or path.read_text() != content:
            changed.append(name)
            if not arguments.check:
                path.write_text(content)
    if arguments.check and changed:
        print("Stale rich text tracking: " + ", ".join(changed), file=sys.stderr)
        return 1
    print(
        f"Rich text inventory {'checked' if arguments.check else 'generated'}: "
        f"{len(inventory['files'])} source/reference files "
        f"(230 core + 62 native + 23 components), {len(api['members'])} public API entries, "
        f"{len(inventory['framework_dependencies'])} framework dependencies, "
        f"{len(inventory['include_boundaries'])} include boundary edges."
    )
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (ValueError, OSError, subprocess.CalledProcessError, ET.ParseError) as error:
        print(f"rich-text-inventory: {error}", file=sys.stderr)
        sys.exit(2)

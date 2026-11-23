"""Compile the complete ToggleButton example panel from the catalog page."""
import json
from pathlib import Path
import struct
import sys
import xml.etree.ElementTree as ET

def lower(panel):
    code = bytearray(b"FXCP\x01")
    native = []

    def emit(op, payload, statement):
        code.append(op)
        code.extend(payload)
        native.append(statement+";")

    emit(1, b"", "let p = panel()")
    for prop, value in panel.attrib.items():
        op, method = {"ItemSpacing": (2, "set_item_spacing"), "LineSpacing": (3, "set_line_spacing")}[prop]
        emit(op, struct.pack("<d",float(value)), f"p.{method}({float(value)})")
    for child in panel:
        assert child.tag.endswith("}ToggleButton") and len(child) == 0
        emit(4, b"", "let c = toggle()")
        for key, value in child.attrib.items():
            if key in ("Name","Content"):
                data = value.encode()
                assert value.isascii()
                if key == "Name":
                    emit(5, bytes([len(data)])+data, f"name(&c, &scope, {json.dumps(value)})")
                else:
                    emit(6, bytes([len(data)])+data, f"content(&c, {json.dumps(value)})")
            else:
                op, method = {"IsThreeState":(7,"set_is_three_state"),"IsEnabled":(8,"set_is_enabled"),"IsChecked":(9,"set_is_checked")}[key]
                assert value in ("True","False")
                v = value.lower() if key != "IsChecked" else f"Some({value.lower()})"
                emit(op, bytes([value=="True"]), f"c.{method}({v})")
        emit(10, b"", "attach(&p, c)")
    emit(0, b"", "p.end_init()")
    return bytes(code), "{\n"+"\n".join(native)+"\n(p,scope)\n}"


if __name__ == "__main__":
    out = Path(sys.argv[1])
    root = ET.parse("samples/ControlCatalog/Pages/ButtonsPage.xaml")
    section, = [s for s in root.findall(".//{*}SampleSection") if s.get("Header") == "ToggleButton"]
    panel, = section.findall("./{*}StackPanel/{*}WrapPanel")
    data, body = lower(panel)
    (out/"template.bin").write_bytes(data)
    (out/"native.rs").write_text(body)

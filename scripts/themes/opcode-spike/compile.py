"""Lower the default WinUI Button template to bytes or straight-line calls of one host ABI."""
from decimal import Decimal
from pathlib import Path
import re
import struct
import sys
import xml.etree.ElementTree as ET

OUTPUT = Path(sys.argv[1])
X = "{http://schemas.microsoft.com/winfx/2006/xaml}"
F = "{https://github.com/ferroui}"
root = ET.parse("src/FerroUI.Themes.WinUI/Controls/Button.xaml").getroot()
theme = next(t for t in root.findall(F+"ControlTheme") if t.get(X+"Key") == "DefaultButtonStyle")
template = next(s for s in theme.findall(F+"Setter") if s.get("Property") == "Template")[0]
presenter, = template
assert presenter.tag == F+"ContentPresenter" and not template.attrib
properties = "Background Foreground BackgroundSizing BorderBrush BorderThickness Content ContentTemplate CornerRadius Padding HorizontalContentAlignment VerticalContentAlignment".split()
payload = bytearray(b"FXOP\x01")
calls = []

def emit(op, data, native):
    payload.append(op)
    payload.extend(data)
    calls.append(native+";")

emit(1, b"", "let p = presenter()")
for key, value in presenter.attrib.items():
    if key == X+"Name":
        name = value.encode()
        assert value.isidentifier()
        emit(2, bytes([len(name)])+name, f'name(&p, &scope, "{value}")')
    elif key == "RecognizesAccessKey":
        assert value in ("True", "False")
        emit(4, bytes([value == "True"]), f"access(&p, {value.lower()})")
    else:
        source = re.fullmatch(r"\{TemplateBinding (\w+)\}", value)[1]
        a, b = properties.index(key), properties.index(source)
        emit(3, bytes([a,b]), f"bind(&p, {a}, {b})")
wrapper, = presenter
collection, = wrapper
transition, = collection
assert (wrapper.tag, collection.tag, transition.tag) == tuple(F+n for n in ("ContentPresenter.Transitions","Transitions","SolidColorBrushTransition"))
assert not wrapper.attrib and not collection.attrib and len(transition) == 0
assert set(transition.attrib) == {"Property","Duration"}
hours, minutes, seconds = map(Decimal, transition.get("Duration").split(":"))
ticks = int((hours * 3600 + minutes * 60 + seconds) * 10_000_000)
prop = properties.index(transition.get("Property"))
emit(5, b"", "let list = Transitions::new()")
emit(6, b"", "let item = SolidColorBrushTransition::new()")
emit(7, bytes([prop]), f"transition_property(&item, {prop})")
emit(8, struct.pack("<q", ticks), f"duration(&item, {ticks})")
emit(9, b"", "append(&list, &item)")
emit(10, b"", "install(&p, list)")
emit(0, b"", "p.try_end_init().unwrap()")
(OUTPUT/"template.bin").write_bytes(payload)
(OUTPUT/"native.rs").write_text("{\n"+"\n".join(calls)+"\n(p,scope)\n}")

#!/usr/bin/env python3
"""Turns the generated animated icons of WinUI into Lottie documents.

    winui_icons.py <controls/dev/AnimatedIcon/AnimatedVisuals of WinUI> <output folder>

WinUI ships these icons as generated C++ that builds a composition tree. The tree has few kinds of
node, so each file is read back into that tree and written as a document the animated-visual
runtime plays. A colour bound to the theme keeps its binding as `{color:Foreground}` in the name of
the paint. A key whose value is a Bezier expression is sampled, four times per frame.
"""
import json
import math
import pathlib
import re
import sys

FLOAT = r"-?\d+(?:\.\d+)?(?:[eE][-+]?\d+)?F"
SAMPLES_PER_FRAME = 4
STEP = 1e-4
EDGE = 1e-3
HANDLED = {
    "visual": {"IsVisible", "Opacity", "Offset", "Scale", "Scale.X", "Scale.Y", "RotationAngleInDegrees"},
    "shape": {"Offset", "Scale", "Scale.X", "Scale.Y", "RotationAngleInDegrees", "StrokeThickness"},
    "geometry": {"TrimStart", "TrimEnd"},
}


def floats(text):
    return [float(item[:-1]) for item in re.findall(FLOAT, text)]


def rounded(value):
    return round(value, 4) + 0.0


class Source:
    def __init__(self, path):
        self.text = path.read_text(encoding="utf-8-sig")
        self.name = path.stem
        start = self.text.index(f"class {self.name}_AnimatedVisual")
        end = self.text.index("public:", start)
        body = self.text[start:end]
        self.methods = {
            match.group(1): match.group(2)
            for match in re.finditer(r"^    [\w:<>]+ (\w+)\(\)\n    \{\n(.*?)^    \}\n", body, re.M | re.S)
        }
        self.members = {}
        for name, method in self.methods.items():
            for member in re.findall(r"(?:result|return) = (_\w+) =|return (_\w+) =", method):
                self.members[member[0] or member[1]] = name
        self.frames = float(re.search(rf"{self.name}::FrameCount\(\)\s*\{{\s*return ([\d.]+);", self.text).group(1))
        self.rate = float(re.search(rf"{self.name}::Framerate\(\)\s*\{{\s*return ([\d.]+);", self.text).group(1))
        self.markers = [
            (name, float(value)) for name, value in re.findall(r'\{ L"(\w+)", ([\d.]+) \}', self.text)
        ]
        self.next_index = 0
        self.assets = []
        self.parameters = {}
        root = self.methods["Root"]
        for parameter, animation in re.findall(r'StartProgressBoundAnimation\(propertySet, L"(\w+)", (\w+)\(\)', root):
            self.parameters[parameter] = self.animation(animation, 0.0)

    def method_of(self, token):
        token = token.strip()
        if token.endswith("()"):
            return token[:-2]
        return self.members[token]

    def easing(self, token):
        token = token.strip()
        if token.startswith("_c.CreateCubicBezierEasingFunction"):
            return tuple(floats(token))
        if token.startswith("_c.CreateLinearEasingFunction"):
            return None
        body = self.methods[self.method_of(token)]
        if "IsFinalStepSingleFrame(true)" in body:
            return "hold"
        if "IsInitialStepSingleFrame(true)" in body:
            return "step"
        if "CreateCubicBezierEasingFunction" in body:
            return tuple(floats(body))
        if "CreateLinearEasingFunction" in body:
            return None
        raise ValueError(f"{self.name}: an easing that is not known: {token}")

    def animation(self, name, initial):
        body = self.methods[name]
        keys = []
        for line in body.splitlines():
            line = line.strip()
            first = re.match(r"const auto result = Create\w+KeyFrameAnimation\((.*)\);$", line)
            insert = re.match(r"result\.InsertKeyFrame\((.*)\);$", line)
            expression = re.match(r'result\.InsertExpressionKeyFrame\(([^,]+), L"([^"]+)", (.*)\);$', line)
            if expression:
                keys.append((floats(expression.group(1))[0], expression.group(2), self.easing(expression.group(3))))
            elif first or insert:
                keys.append(self.key((first or insert).group(1)))
        return Animation(self, keys, initial)

    def key(self, text):
        progress = floats(text)[0]
        rest = text[text.index(",") + 1:].strip()
        if rest.startswith("{"):
            close = rest.index("}")
            value = tuple(floats(rest[:close]))
            rest = rest[close + 1:].lstrip(", ")
        else:
            head, _, tail = rest.partition(",")
            head = head.strip()
            value = head == "true" if head in ("true", "false") else floats(head)[0]
            rest = tail.strip()
        return progress, value, self.easing(rest) if rest else None

    def animated(self, body):
        return {
            prop: animation
            for prop, animation in re.findall(r'StartProgressBoundAnimation\(result, L"([\w.]+)", (\w+)\(\)', body)
        }

    def index(self):
        self.next_index += 1
        return self.next_index


def bezier_y(easing, x):
    x1, y1, x2, y2 = easing
    low, high = 0.0, 1.0
    for _ in range(40):
        t = (low + high) / 2
        at = 3 * (1 - t) ** 2 * t * x1 + 3 * (1 - t) * t * t * x2 + t**3
        if at < x:
            low = t
        else:
            high = t
    t = (low + high) / 2
    return 3 * (1 - t) ** 2 * t * y1 + 3 * (1 - t) * t * t * y2 + t**3


class Animation:
    def __init__(self, source, keys, initial):
        self.source = source
        self.keys = sorted(keys, key=lambda key: key[0])
        if self.keys and self.keys[0][0] > 0.0:
            self.keys.insert(0, (0.0, initial, self.keys[0][2]))

    def value(self, key):
        if isinstance(key[1], str):
            return self.expression(key[1], key[0])
        return key[1]

    def expression(self, text, progress):
        match = re.search(r"_\.(t\d+)", text)
        t = self.source.parameters[match.group(1)].at(progress)
        points = [tuple(float(part) for part in item.split(",")) for item in re.findall(r"Vector2\(([^)]+)\)", text)]
        if len(points) != 4:
            raise ValueError(f"{self.source.name}: an expression that is not a cubic: {text}")
        weights = ((1 - t) ** 3, 3 * (1 - t) ** 2 * t, 3 * (1 - t) * t * t, t**3)
        return tuple(sum(weight * point[axis] for weight, point in zip(weights, points)) for axis in range(2))

    def at(self, progress):
        if progress <= self.keys[0][0]:
            return self.value(self.keys[0])
        for before, after in zip(self.keys, self.keys[1:]):
            if progress <= after[0]:
                if isinstance(after[1], str):
                    return self.expression(after[1], progress)
                span = after[0] - before[0]
                x = (progress - before[0]) / span if span > 0 else 1.0
                easing = after[2]
                if easing == "hold":
                    return self.value(after) if x >= 1.0 else self.value(before)
                if easing == "step":
                    return self.value(after) if x > 0.0 else self.value(before)
                y = bezier_y(easing, x) if easing else x
                a, b = self.value(before), self.value(after)
                if isinstance(a, tuple):
                    return tuple(p + (q - p) * y for p, q in zip(a, b))
                return a + (b - a) * y
        return self.value(self.keys[-1])

    def lottie(self, scale=1.0):
        frames = self.source.frames
        out = []

        def components(value):
            items = value if isinstance(value, tuple) else (value,)
            return [rounded(item * scale) for item in items]

        def push(time, value, **extra):
            frame = rounded(time * frames)
            if out and frame <= out[-1]["t"]:
                out.pop()
            out.append({"t": frame, "s": components(value), **extra})

        for before, after in zip(self.keys, self.keys[1:]):
            start = self.value(before)
            easing = after[2]
            if isinstance(after[1], str):
                count = max(1, math.ceil((after[0] - before[0]) * frames * SAMPLES_PER_FRAME))
                for step in range(count):
                    time = before[0] + (after[0] - before[0]) * step / count
                    push(time, self.at(time if step else time + STEP / frames))
            elif easing == "hold":
                push(before[0], start, h=1)
            elif easing == "step":
                push(before[0], start, h=1)
                push(before[0] + STEP / frames, self.value(after), h=1)
            elif easing is None:
                push(before[0], start, o={"x": [0.0], "y": [0.0]}, i={"x": [1.0], "y": [1.0]})
            else:
                x1, y1, x2, y2 = easing
                push(
                    before[0],
                    start,
                    o={"x": [rounded(x1)], "y": [rounded(y1)]},
                    i={"x": [rounded(x2)], "y": [rounded(y2)]},
                )
        push(self.keys[-1][0], self.value(self.keys[-1]))
        return {"a": 1, "k": out}


def combined_scale(source, x, y):
    """Scale.X and Scale.Y as one value: linear keys on a fine grid, with both sides of each edge."""
    frames = source.frames
    count = math.ceil(frames * SAMPLES_PER_FRAME)
    times = {step / count for step in range(count + 1)}
    for animation in (x, y):
        for key in animation.keys:
            times.add(key[0])
            if key[0] > 0.0:
                times.add(max(0.0, key[0] - EDGE / frames))
    keys = []
    for time in sorted(times):
        frame = round(time * frames, 3)
        if keys and frame <= keys[-1]["t"]:
            continue
        keys.append({"t": frame, "s": [rounded(x.at(time) * 100.0), rounded(y.at(time) * 100.0)]})
    return {"a": 1, "k": keys}


def check_handled(source, name, kind, animated):
    unknown = set(animated) - HANDLED[kind]
    if unknown:
        raise ValueError(f"{source.name}.{name}: animated properties not converted: {sorted(unknown)}")


def constant(value, scale=1.0):
    if isinstance(value, tuple):
        return {"a": 0, "k": [rounded(item * scale) for item in value]}
    return {"a": 0, "k": rounded(value * scale)}


def matrix_parts(a, b, c, d, tx, ty, where):
    sx = math.hypot(a, b)
    rotation = math.atan2(b, a) if sx > 0 else 0.0
    sy = (a * d - b * c) / sx if sx > 0 else d
    if sx > 0 and abs(a * c + b * d) > 1e-4:
        raise ValueError(f"{where}: a matrix with skew")
    return (sx, sy), math.degrees(rotation), (tx, ty)


def one(pattern, body):
    match = re.search(pattern, body)
    return floats(match.group(1)) if match else None


def transform(source, body, kind, name):
    """The transform of a node: `-centre, scale, rotation, centre, matrix, offset`."""
    animated = source.animated(body)
    check_handled(source, name, kind, animated)
    matrix = None
    if kind == "visual":
        values = one(r"result\.TransformMatrix\(\{(.*?)\}\)", body)
        if values:
            matrix = (values[0], values[1], values[4], values[5], values[12], values[13])
    else:
        sprite = re.search(r"CreateSpriteShape\(\w+, \{(.*?)\}", body)
        values = floats(sprite.group(1)) if sprite else one(r"result\.TransformMatrix\(\{(.*?)\}\)", body)
        if values:
            matrix = tuple(values[:6])
    center = tuple(one(r"result\.CenterPoint\(\{(.*?)\}\)", body) or (0.0, 0.0))[:2]
    scale = tuple(one(r"result\.Scale\(\{(.*?)\}\)", body) or (1.0, 1.0))[:2]
    offset = tuple(one(r"result\.Offset\(\{(.*?)\}\)", body) or (0.0, 0.0))[:2]
    rotation = (one(r"result\.RotationAngleInDegrees\((.*?)\);", body) or [0.0])[0]
    plain = (
        scale == (1.0, 1.0) and rotation == 0.0 and center == (0.0, 0.0)
        and not {"Scale", "Scale.X", "Scale.Y", "RotationAngleInDegrees"} & set(animated)
    )
    if matrix and matrix != (1.0, 0.0, 0.0, 1.0, 0.0, 0.0):
        if not plain:
            raise ValueError(f"{source.name}.{name}: a matrix with other transforms")
        scale, rotation, position = matrix_parts(*matrix, f"{source.name}.{name}")
        if "Offset" in animated:
            raise ValueError(f"{source.name}.{name}: a matrix with an animated offset")
        result = {"a": constant((0.0, 0.0)), "p": constant((position[0] + offset[0], position[1] + offset[1]))}
        result["s"] = constant(scale, 100.0)
        result["r"] = constant(rotation)
        return result
    result = {"a": constant(center)}
    if "Offset" in animated:
        animation = source.animation(animated["Offset"], offset)
        property_ = animation.lottie()
        for key in property_["k"]:
            key["s"] = [rounded(key["s"][0] + center[0]), rounded(key["s"][1] + center[1])]
        result["p"] = property_
    else:
        result["p"] = constant((offset[0] + center[0], offset[1] + center[1]))
    if "Scale.X" in animated or "Scale.Y" in animated:
        x = source.animation(animated["Scale.X"], scale[0]) if "Scale.X" in animated else Animation(source, [(0.0, scale[0], None)], scale[0])
        y = source.animation(animated["Scale.Y"], scale[1]) if "Scale.Y" in animated else Animation(source, [(0.0, scale[1], None)], scale[1])
        result["s"] = combined_scale(source, x, y)
    elif "Scale" in animated:
        result["s"] = source.animation(animated["Scale"], scale).lottie(100.0)
    else:
        result["s"] = constant(scale, 100.0)
    if "RotationAngleInDegrees" in animated:
        result["r"] = source.animation(animated["RotationAngleInDegrees"], rotation).lottie()
    else:
        result["r"] = constant(rotation)
    return result


def figures(source, geometry):
    body = source.methods[geometry]
    if "CreateGeometryGroup" in body:
        if "D2D1_FILL_MODE_WINDING" not in body:
            raise ValueError(f"{source.name}.{geometry}: a group that is not filled by winding")
        return [item for part in re.findall(r"(Geometry\w*)\(\)\.get\(\)", body) for item in figures(source, part)]
    out = []
    current = None
    for line in body.splitlines():
        line = line.strip()
        if line.startswith("sink->BeginFigure"):
            x, y = floats(line)[:2]
            current = {"v": [[x, y]], "i": [[0.0, 0.0]], "o": [[0.0, 0.0]], "c": False}
        elif line.startswith("sink->AddLine"):
            x, y = floats(line)[:2]
            current["v"].append([x, y])
            current["i"].append([0.0, 0.0])
            current["o"].append([0.0, 0.0])
        elif line.startswith("sink->AddBezier"):
            a, b, c, d, x, y = floats(line)[:6]
            last = current["v"][-1]
            current["o"][-1] = [a - last[0], b - last[1]]
            current["v"].append([x, y])
            current["i"].append([c - x, d - y])
            current["o"].append([0.0, 0.0])
        elif line.startswith("sink->EndFigure"):
            current["c"] = "D2D1_FIGURE_END_CLOSED" in line
            first, last = current["v"][0], current["v"][-1]
            if current["c"] and len(current["v"]) > 1 and abs(first[0] - last[0]) < 1e-6 and abs(first[1] - last[1]) < 1e-6:
                current["i"][0] = current["i"].pop()
                current["v"].pop()
                current["o"].pop()
            for key in ("v", "i", "o"):
                current[key] = [[rounded(p[0]), rounded(p[1])] for p in current[key]]
            out.append({"ty": "sh", "ks": {"a": 0, "k": current}})
            current = None
    if not out:
        raise ValueError(f"{source.name}.{geometry}: a geometry without figures")
    return out


def geometry_items(source, token):
    name = source.method_of(token)
    body = source.methods[name]
    items = []
    if "CreateEllipseGeometry" in body:
        center = one(r"result\.Center\(\{(.*?)\}\)", body) or [0.0, 0.0]
        radius = one(r"result\.Radius\(\{(.*?)\}\)", body)
        items.append({"ty": "el", "p": constant(tuple(center)), "s": constant((radius[0] * 2, radius[1] * 2))})
    else:
        inner = re.search(r"CreatePathGeometry\((.*)\);", body).group(1)
        direct = re.search(r"(Geometry\w*)\(\)\)\)", inner)
        if direct:
            geometry = direct.group(1)
        else:
            path = source.methods[source.method_of(inner)]
            geometry = re.search(r"(Geometry\w*)\(\)\)", path).group(1)
        items.extend(figures(source, geometry))
    animated = source.animated(body)
    check_handled(source, name, "geometry", animated)
    start = (one(r"result\.TrimStart\((.*?)\);", body) or [0.0])[0]
    end = (one(r"result\.TrimEnd\((.*?)\);", body) or [1.0])[0]
    if animated.keys() & {"TrimStart", "TrimEnd"} or start != 0.0 or end != 1.0:
        trim = {"ty": "tm", "o": constant(0.0), "m": 1}
        trim["s"] = source.animation(animated["TrimStart"], start).lottie(100.0) if "TrimStart" in animated else constant(start, 100.0)
        trim["e"] = source.animation(animated["TrimEnd"], end).lottie(100.0) if "TrimEnd" in animated else constant(end, 100.0)
        items.append(trim)
    return items


FOREGROUND = {"a": 0, "k": [0.0, 0.0, 0.0, 1.0]}
CAPS = {"Flat": 1, "Round": 2, "Square": 3, "Triangle": 2}
JOINS = {"Miter": 1, "Round": 2, "Bevel": 3, "MiterOrBevel": 1}


def sprite(source, name):
    body = source.methods[name]
    created = re.search(r"CreateSpriteShape\((\w+\(\)|\w+)", body).group(1)
    if created == "geometry":
        created = re.search(r"const auto geometry = (\w+\(\)|_\w+);", body).group(1)
    items = geometry_items(source, created)
    stroke = "StrokeBrush(" in body
    fill = "FillBrush(" in body or re.search(r"CreateSpriteShape\(\w+, \{.*?\}, (?!nullptr)", body) is not None
    animated = source.animated(body)
    if stroke:
        thickness = (one(r"StrokeThickness\((.*?)\)", body) or [1.0])[0]
        width = source.animation(animated["StrokeThickness"], thickness).lottie() if "StrokeThickness" in animated else constant(thickness)
        cap = re.search(r"StrokeStartCap\(CompositionStrokeCap::(\w+)\)", body)
        join = re.search(r"StrokeLineJoin\(CompositionStrokeLineJoin::(\w+)\)", body)
        items.append({
            "ty": "st",
            "nm": "{color:Foreground}",
            "c": FOREGROUND,
            "o": constant(100.0),
            "w": width,
            "lc": CAPS[cap.group(1)] if cap else 1,
            "lj": JOINS[join.group(1)] if join else 1,
            "ml": (one(r"StrokeMiterLimit\((.*?)\)", body) or [4.0])[0],
        })
    if fill:
        items.append({"ty": "fl", "nm": "{color:Foreground}", "c": FOREGROUND, "o": constant(100.0), "r": 1})
    items.append({"ty": "tr", **transform(source, body, "shape", name)})
    return {"ty": "gr", "it": items}


def shape(source, token):
    name = source.method_of(token)
    body = source.methods[name]
    if "CreateContainerShape" not in body:
        return sprite(source, name)
    children = [shape(source, child) for child in re.findall(r"(?:shapes|result\.Shapes\(\))\.Append\((\w+\(\)|_\w+)\)", body)]
    return {"ty": "gr", "it": [*reversed(children), {"ty": "tr", **transform(source, body, "shape", name)}]}


def visibility(source, body):
    animated = source.animated(body)
    initial = "result.IsVisible(false)" not in body
    if "IsVisible" not in animated:
        return None if initial else {"a": 0, "k": 0.0}
    animation = source.animation(animated["IsVisible"], initial)
    keys = []
    for index, key in enumerate(animation.keys):
        keys.append((key[0], 100.0 if animation.value(key) else 0.0))
    out = []
    for progress, value in keys:
        frame = rounded(progress * source.frames)
        if out and out[-1]["t"] >= frame:
            out.pop()
        out.append({"t": frame, "s": [value], "h": 1})
    return {"a": 1, "k": out}


def opacity(source, body):
    animated = source.animated(body)
    fixed = (one(r"result\.Opacity\((.*?)\);", body) or [1.0])[0]
    if "Opacity" in animated:
        return source.animation(animated["Opacity"], fixed).lottie(100.0)
    return constant(fixed, 100.0) if fixed != 1.0 else None


def visual(source, token):
    name = source.method_of(token)
    body = source.methods[name]
    size = one(r"result\.Size\(\{(.*?)\}\)", body) or [0.0, 0.0]
    clipped = "result.Clip(" in body
    ks = transform(source, body, "visual", name)
    seen, faded = visibility(source, body), opacity(source, body)
    frames = source.frames
    if "CreateShapeVisual" in body:
        shapes = [shape(source, child) for child in re.findall(r"(?:shapes|result\.Shapes\(\))\.Append\((\w+\(\)|_\w+)\)", body)]
        layer = {"ty": 4, "ind": source.index(), "ip": 0, "op": frames + 1, "st": 0, "shapes": list(reversed(shapes))}
        if clipped:
            raise ValueError(f"{source.name}.{name}: a clipped shape visual")
    else:
        children = re.findall(r"(?:children|result\.Children\(\))\.InsertAtTop\((\w+\(\)|_\w+)\)", body)
        reference = f"comp_{len(source.assets)}"
        asset = {"id": reference, "layers": []}
        source.assets.append(asset)
        asset["layers"] = [visual(source, child) for child in reversed(children)]
        layer = {"ty": 0, "ind": source.index(), "refId": reference, "ip": 0, "op": frames + 1, "st": 0}
        if clipped:
            layer["w"], layer["h"] = size[0], size[1]
    if seen and faded:
        # The two multiply: the fade goes on a layer inside the one that shows and hides.
        inner = {**layer, "ks": {**ks, "o": faded}}
        reference = f"comp_{len(source.assets)}"
        source.assets.append({"id": reference, "layers": [inner]})
        return {"ty": 0, "ind": source.index(), "refId": reference, "ip": 0, "op": frames + 1, "st": 0, "ks": {"o": seen}}
    layer["ks"] = {**ks, "o": seen or faded or constant(100.0)}
    return layer


def convert(path):
    source = Source(path)
    root = source.methods["Root"]
    children = re.findall(r"(?:children|result\.Children\(\))\.InsertAtTop\((\w+\(\)|_\w+)\)", root)
    layers = [visual(source, child) for child in reversed(children)]
    size = re.search(r"Size\(\) const\s*\{\s*return \{ (.*?) \};", source.text)
    width, height = floats(size.group(1))
    markers = [{"cm": name, "tm": round(value * source.frames, 6), "dr": 0} for name, value in source.markers]
    return {
        "v": "5.7.0",
        "fr": source.rate,
        "ip": 0,
        "op": source.frames,
        "w": width,
        "h": height,
        "assets": source.assets,
        "layers": layers,
        "markers": markers,
    }


def main():
    folder, out = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2])
    out.mkdir(parents=True, exist_ok=True)
    for path in sorted(folder.glob("Animated*VisualSource.cpp")):
        document = convert(path)
        target = out / (path.stem.removeprefix("Animated").removesuffix("VisualSource") + ".json")
        target.write_text(json.dumps(document, separators=(",", ":")) + "\n", encoding="utf-8")
        print(target.name, target.stat().st_size, "bytes", len(document["layers"]), "layers", len(document["markers"]), "markers")


if __name__ == "__main__":
    main()

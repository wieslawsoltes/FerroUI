#!/usr/bin/env python3
"""Writes the symbol font of the ControlCatalog, `FerroUI Catalog Symbols`
(`samples/ControlCatalog/Assets/Fonts/FerroUICatalogSymbols.ttf`).

The WinUI controls draw their glyphs with the font of the theme resource `SymbolThemeFontFamily`,
Segoe Fluent Icons on Windows (docs/porting/winui-controls.md, section 5.2). That font is not
redistributable, so off Windows a glyph is a missing-glyph box. The catalog names this font after
Segoe Fluent Icons in `SymbolThemeFontFamily` on its page of the WinUI controls: on Windows the
system font draws, elsewhere this one.

The outlines are licence-clean, written for FerroUI: the geometries of the WinUI theme of the port
(`src/FerroUI.Themes.WinUI/Resources/Glyphs.xaml`, the glyphs its templates draw as paths), and the
outlines of `EXTRA` below, drawn for this font. Each glyph has the code point of the Segoe Fluent
Icons glyph it stands for, so the controls need no mapping; its ink is centred in the em.

    catalog_symbol_font.py           write the font
    catalog_symbol_font.py --check   fail if the font is not what the script writes
    catalog_symbol_font.py --list    print the code points

Standard library only; the output is deterministic.
"""

import math
import os
import re
import struct
import sys

REPOSITORY = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
GLYPHS = os.path.join(REPOSITORY, "src/FerroUI.Themes.WinUI/Resources/Glyphs.xaml")
OUTPUT = os.path.join(REPOSITORY, "samples/ControlCatalog/Assets/Fonts/FerroUICatalogSymbols.ttf")
FAMILY = "FerroUI Catalog Symbols"
UPEM = 2048
ASCENT, DESCENT = 1792, 256
CENTER = (UPEM / 2, (ASCENT - DESCENT) / 2)


def star(cx, cy, outer, inner, rotate=-90.0):
    points = []
    for i in range(10):
        r = outer if i % 2 == 0 else inner
        a = math.radians(rotate + i * 36)
        points.append((cx + r * math.cos(a), cy + r * math.sin(a)))
    return "M" + " L".join(f"{x:.3f},{y:.3f}" for x, y in points) + " Z"


def gear(cx, cy, teeth, outer, root, hole):
    points = []
    step = 360.0 / teeth
    for i in range(teeth):
        base = i * step
        for angle, r in ((base - step * 0.18, outer), (base + step * 0.18, outer), (base + step * 0.32, root), (base + step * 0.68, root)):
            a = math.radians(angle)
            points.append((cx + r * math.cos(a), cy + r * math.sin(a)))
    ring = "M" + " L".join(f"{x:.3f},{y:.3f}" for x, y in points) + " Z"
    return ring + f" M{cx - hole:.3f},{cy:.3f} A{hole},{hole} 0 1 0 {cx + hole:.3f},{cy:.3f} A{hole},{hole} 0 1 0 {cx - hole:.3f},{cy:.3f} Z"


# Outlines drawn for this font (16-unit design, y down): the glyphs the controls and the catalog page
# use that the WinUI theme draws nowhere as a path. The comment names the Segoe Fluent Icons glyph.
EXTRA = {
    0xE734: star(8, 8.6, 7.8, 3.3) + " " + star(8, 8.6, 5.6, 2.37),  # FavoriteStar / OutlineStar (RatingControl)
    0xE735: star(8, 8.6, 7.8, 3.3),  # FavoriteStarFill / SolidStar (RatingControl)
    0xE713: gear(8, 8, 8, 7.8, 6.1, 2.9),  # Setting (NavigationView settings item)
    0xE715: "M0,2.5 H16 V13.5 H0 Z M1,3.5 V12.5 H15 V3.5 Z M1.6,3.6 L8,8.4 L14.4,3.6 L15,4.4 L8,9.6 L1,4.4 Z",  # Mail
    0xE80F: "M8,0.6 L16,7.6 L14.6,7.6 L14.6,15.2 L1.4,15.2 L1.4,7.6 L0,7.6 Z M8,2.5 L2.7,7.1 L2.7,14 L13.3,14 L13.3,7.1 Z",  # Home
    0xE721: "M0.5,6.5 A6,6 0 1 1 12.5,6.5 A6,6 0 1 1 0.5,6.5 Z M1.7,6.5 A4.8,4.8 0 1 0 11.3,6.5 A4.8,4.8 0 1 0 1.7,6.5 Z M11.2,10.4 L16,15.2 L15.2,16 L10.4,11.2 Z",  # Search / Find
    0xE710: "M7.25,1 H8.75 V7.25 H15 V8.75 H8.75 V15 H7.25 V8.75 H1 V7.25 H7.25 Z",  # Add
    0xE72D: "M10,1 L15.5,5.5 L10,10 L10,7.2 C6.5,7.2 4,8.6 2,12 C2.6,7.2 5.4,4.6 10,4.1 Z",  # Share
    0xE8C9: "M7,1 H9 V10 H7 Z M8,12 A1.25,1.25 0 1 1 8,14.5 A1.25,1.25 0 1 1 8,12 Z",  # Important
    0xEA38: "M7,1 H9 V10 H7 Z M8,12 A1.25,1.25 0 1 1 8,14.5 A1.25,1.25 0 1 1 8,12 Z",  # the attention glyph of InfoBadge
}

# The symbols InfoBar draws over its circle (F136) are drawn at a fraction of the em, centred on it,
# as WinUI's glyphs are.
SCALE = {0xF13E: 8.0 / 15.0, 0xF13D: 0.6}


# --- path data -----------------------------------------------------------------------------------


def tokens(data):
    for match in re.finditer(r"[MmLlHhVvCcSsQqTtAaZzF]|[-+]?(?:\d+\.?\d*|\.\d+)(?:[eE][-+]?\d+)?", data):
        yield match.group(0)


def arc_to_cubics(x1, y1, rx, ry, phi, large, sweep, x2, y2):
    if (x1, y1) == (x2, y2):
        return []
    if rx == 0 or ry == 0:
        return [((x1, y1), (x2, y2), (x2, y2))]
    rx, ry = abs(rx), abs(ry)
    cp, sp = math.cos(math.radians(phi)), math.sin(math.radians(phi))
    dx, dy = (x1 - x2) / 2, (y1 - y2) / 2
    x1p, y1p = cp * dx + sp * dy, -sp * dx + cp * dy
    lam = (x1p * x1p) / (rx * rx) + (y1p * y1p) / (ry * ry)
    if lam > 1:
        rx, ry = rx * math.sqrt(lam), ry * math.sqrt(lam)
    num = rx * rx * ry * ry - rx * rx * y1p * y1p - ry * ry * x1p * x1p
    den = rx * rx * y1p * y1p + ry * ry * x1p * x1p
    coef = math.sqrt(max(0.0, num / den)) if den else 0.0
    if large == sweep:
        coef = -coef
    cxp, cyp = coef * rx * y1p / ry, -coef * ry * x1p / rx
    cx = cp * cxp - sp * cyp + (x1 + x2) / 2
    cy = sp * cxp + cp * cyp + (y1 + y2) / 2

    def angle(ux, uy, vx, vy):
        a = math.atan2(ux * vy - uy * vx, ux * vx + uy * vy)
        return a

    t1 = angle(1, 0, (x1p - cxp) / rx, (y1p - cyp) / ry)
    dt = angle((x1p - cxp) / rx, (y1p - cyp) / ry, (-x1p - cxp) / rx, (-y1p - cyp) / ry)
    if not sweep and dt > 0:
        dt -= 2 * math.pi
    elif sweep and dt < 0:
        dt += 2 * math.pi
    n = max(1, int(math.ceil(abs(dt) / (math.pi / 2) - 1e-9)))
    d = dt / n
    k = 4 / 3 * math.tan(d / 4)
    out = []
    for i in range(n):
        a1, a2 = t1 + i * d, t1 + (i + 1) * d
        e1 = (math.cos(a1) - k * math.sin(a1), math.sin(a1) + k * math.cos(a1))
        e2 = (math.cos(a2) + k * math.sin(a2), math.sin(a2) - k * math.cos(a2))
        e3 = (math.cos(a2), math.sin(a2))

        def tr(p):
            return (cx + rx * p[0] * cp - ry * p[1] * sp, cy + rx * p[0] * sp + ry * p[1] * cp)

        out.append((tr(e1), tr(e2), tr(e3)))
    return out


def parse(data):
    """Contours of segments: [(kind, points)] with kind 'L' (end), 'Q' (c, end), 'C' (c1, c2, end);
    and the fill rule (True for nonzero, F1)."""
    toks = list(tokens(data))
    i, cmd, contours, current = 0, None, [], None
    x = y = sx = sy = 0.0
    last_c = last_q = None
    nonzero = False

    def num():
        nonlocal i
        v = float(toks[i])
        i += 1
        return v

    while i < len(toks):
        t = toks[i]
        if re.match(r"[A-Za-z]", t):
            cmd = t
            i += 1
            if cmd == "F":
                nonzero = num() == 1
                continue
            if cmd in "Zz":
                if current:
                    contours.append(current)
                current = None
                x, y = sx, sy
                last_c = last_q = None
                continue
        rel = cmd.islower()
        c = cmd.upper()
        if c == "M":
            nx, ny = num(), num()
            if rel:
                nx, ny = nx + x, ny + y
            if current:
                contours.append(current)
            x, y = sx, sy = nx, ny
            current = {"start": (x, y), "segments": []}
            cmd = "l" if rel else "L"
            last_c = last_q = None
            continue
        if current is None:
            current = {"start": (x, y), "segments": []}
        if c in "LHV":
            if c == "L":
                nx, ny = num(), num()
                if rel:
                    nx, ny = nx + x, ny + y
            elif c == "H":
                nx, ny = num() + (x if rel else 0), y
            else:
                nx, ny = x, num() + (y if rel else 0)
            current["segments"].append(("L", [(nx, ny)]))
            x, y = nx, ny
            last_c = last_q = None
        elif c in "CS":
            if c == "C":
                c1 = (num(), num())
            else:
                c1 = (2 * x - last_c[0], 2 * y - last_c[1]) if last_c else (x, y)
            c2 = (num(), num())
            e = (num(), num())
            if rel:
                if c == "C":
                    c1 = (c1[0] + x, c1[1] + y)
                c2 = (c2[0] + x, c2[1] + y)
                e = (e[0] + x, e[1] + y)
            current["segments"].append(("C", [c1, c2, e]))
            last_c, last_q = c2, None
            x, y = e
        elif c in "QT":
            if c == "Q":
                q = (num(), num())
                if rel:
                    q = (q[0] + x, q[1] + y)
            else:
                q = (2 * x - last_q[0], 2 * y - last_q[1]) if last_q else (x, y)
            e = (num(), num())
            if rel:
                e = (e[0] + x, e[1] + y)
            current["segments"].append(("Q", [q, e]))
            last_q, last_c = q, None
            x, y = e
        elif c == "A":
            rx, ry, phi = num(), num(), num()
            large, sweep = int(num()), int(num())
            e = (num(), num())
            if rel:
                e = (e[0] + x, e[1] + y)
            for c1, c2, end in arc_to_cubics(x, y, rx, ry, phi, large, sweep, e[0], e[1]):
                current["segments"].append(("C", [c1, c2, end]))
            x, y = e
            last_c = last_q = None
        else:
            raise SystemExit(f"unsupported path command {cmd!r}")
    if current:
        contours.append(current)
    return contours, nonzero


def cubic_to_quads(p0, c1, c2, p3, pieces=4):
    def lerp(a, b, t):
        return (a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t)

    def split(p0, p1, p2, p3, t):
        a, b, c = lerp(p0, p1, t), lerp(p1, p2, t), lerp(p2, p3, t)
        d, e = lerp(a, b, t), lerp(b, c, t)
        f = lerp(d, e, t)
        return (p0, a, d, f), (f, e, c, p3)

    out, rest = [], (p0, c1, c2, p3)
    for i in range(pieces, 0, -1):
        piece, rest = split(*rest, 1.0 / i) if i > 1 else (rest, None)
        q0, q1, q2, q3 = piece
        control = ((3 * (q1[0] + q2[0]) - q0[0] - q3[0]) / 4, (3 * (q1[1] + q2[1]) - q0[1] - q3[1]) / 4)
        out.append((control, q3))
    return out


def quad_contour(contour):
    """[(x, y, on_curve)] of a contour, in design units, y down."""
    points = [(contour["start"][0], contour["start"][1], True)]
    cur = contour["start"]
    for kind, pts in contour["segments"]:
        if kind == "L":
            points.append((pts[0][0], pts[0][1], True))
            cur = pts[0]
        elif kind == "Q":
            points.append((pts[0][0], pts[0][1], False))
            points.append((pts[1][0], pts[1][1], True))
            cur = pts[1]
        else:
            for control, end in cubic_to_quads(cur, pts[0], pts[1], pts[2]):
                points.append((control[0], control[1], False))
                points.append((end[0], end[1], True))
            cur = pts[2]
    if len(points) > 1 and points[-1][2] and abs(points[-1][0] - points[0][0]) < 1e-9 and abs(points[-1][1] - points[0][1]) < 1e-9:
        points.pop()
    return points


def area(poly):
    return sum(poly[i][0] * poly[(i + 1) % len(poly)][1] - poly[(i + 1) % len(poly)][0] * poly[i][1] for i in range(len(poly))) / 2


def inside(pt, poly):
    x, y = pt
    result = False
    for i in range(len(poly)):
        (x1, y1), (x2, y2) = poly[i], poly[(i + 1) % len(poly)]
        if (y1 > y) != (y2 > y) and x < (x2 - x1) * (y - y1) / (y2 - y1) + x1:
            result = not result
    return result


def glyph_outline(code, data):
    """Contours in font units: [[(x, y, on)]]."""
    contours, nonzero = parse(data)
    quads = [quad_contour(c) for c in contours]
    quads = [q for q in quads if len(q) >= 2]
    xs = [p[0] for q in quads for p in q]
    ys = [p[1] for q in quads for p in q]
    extent = max(max(abs(v) for v in xs), max(abs(v) for v in ys))
    design = next((d for d in (16, 20, 24) if extent <= d + 0.5), 2048 if extent > 100 else math.ceil(extent))
    scale = UPEM / design * SCALE.get(code, 1.0)
    # Every outline is path data, y down (the templates draw them as Paths): the font turns them
    # over. (The 2048-unit outlines were taken for y-up font outlines, which drew the chevrons of
    # NavigationView upside down.)
    y_up = False
    cx, cy = (min(xs) + max(xs)) / 2, (min(ys) + max(ys)) / 2
    out = []
    for q in quads:
        pts = []
        for x, y, on in q:
            fx = CENTER[0] + (x - cx) * scale
            fy = CENTER[1] + ((y - cy) if y_up else (cy - y)) * scale
            pts.append((int(round(fx)), int(round(fy)), on))
        out.append(pts)
    polys = [[(p[0], p[1]) for p in c] for c in out]
    if not nonzero:
        # Even-odd source: TrueType fills by non-zero winding, so a contour inside an odd number of
        # others is a hole and turns the other way (outer contours clockwise, y up).
        for idx, poly in enumerate(polys):
            depth = sum(1 for j, other in enumerate(polys) if j != idx and inside(poly[0], other))
            clockwise = area(poly) < 0
            if clockwise != (depth % 2 == 0):
                out[idx] = [out[idx][0]] + list(reversed(out[idx][1:]))
    return out


def glyph_sources():
    sources = {}
    text = open(GLYPHS, encoding="utf-8").read()
    for code, data in re.findall(r'x:Key="GlyphGeometry([0-9A-F]{4})">([^<]*)<', text):
        sources[int(code, 16)] = data
    sources.update(EXTRA)
    return dict(sorted(sources.items()))


# --- the font file -------------------------------------------------------------------------------


def checksum(data):
    data += b"\0" * (-len(data) % 4)
    return sum(struct.unpack(f">{len(data) // 4}I", data)) & 0xFFFFFFFF


def glyf_entry(contours):
    if not contours:
        return b"", (0, 0, 0, 0), 0, 0
    xs = [p[0] for c in contours for p in c]
    ys = [p[1] for c in contours for p in c]
    bbox = (min(xs), min(ys), max(xs), max(ys))
    data = struct.pack(">hhhhh", len(contours), *bbox)
    ends, n = [], 0
    for c in contours:
        n += len(c)
        ends.append(n - 1)
    data += struct.pack(f">{len(ends)}H", *ends) + struct.pack(">H", 0)
    flags, dx, dy = b"", b"", b""
    px = py = 0
    for c in contours:
        for x, y, on in c:
            flags += bytes([1 if on else 0])
            dx += struct.pack(">h", x - px)
            dy += struct.pack(">h", y - py)
            px, py = x, y
    data += flags + dx + dy
    data += b"\0" * (-len(data) % 4)
    return data, bbox, sum(len(c) for c in contours), len(contours)


def name_table():
    records = [(0, "Outlines written for FerroUI (MIT); see samples/ControlCatalog/NOTICE.md."), (1, FAMILY), (2, "Regular"),
               (3, f"{FAMILY} Regular 1.0"), (4, f"{FAMILY} Regular"), (5, "Version 1.0"), (6, FAMILY.replace(" ", "") + "-Regular")]
    strings, entries, offset = b"", b"", 0
    for name_id, value in records:
        encoded = value.encode("utf-16-be")
        entries += struct.pack(">HHHHHH", 3, 1, 0x409, name_id, len(encoded), offset)
        strings += encoded
        offset += len(encoded)
    return struct.pack(">HHH", 0, len(records), 6 + 12 * len(records)) + entries + strings


def cmap_table(codes):
    # Format 4: one segment per code point, then the end segment.
    segments = [(c, c, gid) for gid, c in codes] + [(0xFFFF, 0xFFFF, 0)]
    n = len(segments)
    search = 2 * (1 << int(math.log2(n)))
    ends = struct.pack(f">{n}H", *[s[1] for s in segments])
    starts = struct.pack(f">{n}H", *[s[0] for s in segments])
    deltas = struct.pack(f">{n}h", *[((gid - start) + 0x8000) % 0x10000 - 0x8000 if start != 0xFFFF else 1 for start, _, gid in segments])
    offsets = struct.pack(f">{n}H", *[0] * n)
    body = struct.pack(">HHHH", 2 * n, search, int(math.log2(search // 2)), 2 * n - search) + ends + b"\0\0" + starts + deltas + offsets
    sub = struct.pack(">HHH", 4, 14 + len(body), 0) + body
    return struct.pack(">HHHHI", 0, 2, 0, 3, 20) + struct.pack(">HHI", 3, 1, 20) + sub


def build():
    sources = glyph_sources()
    glyphs = [[]]  # .notdef, empty
    codes = []
    for code, data in sources.items():
        codes.append((len(glyphs), code))
        glyphs.append(glyph_outline(code, data))
    glyf, loca, bboxes, max_points, max_contours = b"", [0], [], 0, 0
    for contours in glyphs:
        data, bbox, points, count = glyf_entry(contours)
        glyf += data
        loca.append(len(glyf))
        bboxes.append(bbox)
        max_points, max_contours = max(max_points, points), max(max_contours, count)
    num = len(glyphs)
    inked = [b for b in bboxes if b != (0, 0, 0, 0)]
    xmin, ymin = min(b[0] for b in inked), min(b[1] for b in inked)
    xmax, ymax = max(b[2] for b in inked), max(b[3] for b in inked)
    tables = {
        b"cmap": cmap_table(codes),
        b"glyf": glyf,
        b"head": struct.pack(">IIIIHHqqhhhhHHhhh", 0x00010000, 0x00010000, 0, 0x5F0F3CF5, 0x000B, UPEM, 0, 0,
                             xmin, ymin, xmax, ymax, 0, 8, 2, 1, 0),
        b"hhea": struct.pack(">IhhhHhhhhhhhhhhhH", 0x00010000, ASCENT, -DESCENT, 0, UPEM, min(b[0] for b in bboxes),
                             min(UPEM - b[2] for b in bboxes), xmax, 1, 0, 0, 0, 0, 0, 0, 0, num),
        b"hmtx": b"".join(struct.pack(">Hh", UPEM, b[0]) for b in bboxes),
        b"loca": struct.pack(f">{len(loca)}I", *loca),
        b"maxp": struct.pack(">IHHHHHHHHHHHHHH", 0x00010000, num, max_points, max_contours, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0),
        b"name": name_table(),
        b"OS/2": struct.pack(">HhHHHhhhhhhhhhhh10sIIIIBBBBHHHhhhHHIIhhHHH", 4, UPEM, 400, 5, 0, 1331, 1433, 1331, 0,
                             1331, 1433, 1331, 491, 102, 512, 0, b"\0" * 10, 0, 0x10000000, 0, 0, b"NONE"[0], b"NONE"[1],
                             b"NONE"[2], b"NONE"[3], 0x0040, min(c for _, c in codes), max(c for _, c in codes), ASCENT,
                             -DESCENT, 0, ASCENT, DESCENT, 1, 0, 1024, 1434, 0, 32, 0),
        b"post": struct.pack(">IIhhIIIII", 0x00030000, 0, -205, 102, 0, 0, 0, 0, 0),
    }
    tags = sorted(tables)
    n = len(tags)
    search = 16 * (1 << int(math.log2(n)))
    header = struct.pack(">IHHHH", 0x00010000, n, search, int(math.log2(search // 16)), n * 16 - search)
    offset = 12 + 16 * n
    directory, body = b"", b""
    for tag in tags:
        data = tables[tag]
        directory += struct.pack(">4sIII", tag, checksum(data), offset + len(body), len(data))
        body += data + b"\0" * (-len(data) % 4)
    font = bytearray(header + directory + body)
    adjust = (0xB1B0AFBA - checksum(bytes(font))) & 0xFFFFFFFF
    head_offset = 12 + 16 * n + sum(len(tables[t]) + (-len(tables[t]) % 4) for t in tags[: tags.index(b"head")])
    font[head_offset + 8: head_offset + 12] = struct.pack(">I", adjust)
    return bytes(font), [c for _, c in codes]


def main(argv):
    font, codes = build()
    if "--list" in argv:
        print(" ".join(f"{c:04X}" for c in codes))
        return 0
    if "--check" in argv:
        current = open(OUTPUT, "rb").read() if os.path.exists(OUTPUT) else None
        if current != font:
            print(f"error: {os.path.relpath(OUTPUT, REPOSITORY)} is stale; run scripts/winui-controls/catalog_symbol_font.py", file=sys.stderr)
            return 1
        print(f"{FAMILY}: {len(codes)} glyphs, up to date")
        return 0
    with open(OUTPUT, "wb") as file:
        file.write(font)
    print(f"wrote {os.path.relpath(OUTPUT, REPOSITORY)}: {len(codes)} glyphs")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))

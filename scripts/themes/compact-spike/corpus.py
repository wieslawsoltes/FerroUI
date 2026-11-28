"""Measure a lossless XAML element/attribute/text stream before semantic lowering."""
import argparse
import json
import struct
from collections import Counter
from pathlib import Path
from xml.parsers import expat


def events(source):
    result = []
    parser = expat.ParserCreate()
    parser.ordered_attributes = True
    parser.buffer_text = True
    parser.StartElementHandler = lambda name, attrs: result.append(('open', name, *attrs))
    parser.EndElementHandler = lambda name: result.append(('close',))
    parser.CharacterDataHandler = lambda value: result.append(('text', value))
    parser.Parse(source, True)
    return result


def put(n):
    data = bytearray()
    while n >= 128:
        data.append(128 | (n & 127))
        n >>= 7
    data.append(n)
    return data


def get(stream, cursor):
    value = shift = 0
    while True:
        byte = stream[cursor]
        cursor += 1
        value |= (byte & 127) << shift
        if byte < 128:
            return value, cursor
        shift += 7


def encode(documents):
    # Token counts also include namespace declarations and all inter-element whitespace.
    counts = Counter({name: 1 for name in documents})
    for rows in documents.values():
        for event in rows:
            counts.update(event[1:])
    ids = {s: i for i, (s, _) in enumerate(counts.most_common())}
    out = bytearray(b'FXB1') + put(len(ids))
    for s in ids:
        raw = s.encode()
        out += put(len(raw)) + raw
    pool_bytes = len(out)
    out += put(len(documents))
    directory = bytearray()
    bodies = bytearray()
    for name, rows in documents.items():
        directory += put(ids[name]) + struct.pack('<I', len(bodies))
        for op, *args in rows:
            if op == 'open':
                bodies += b'\x01' + put(ids[args[0]]) + put((len(args)-1)//2)
                for token in args[1:]:
                    bodies += put(ids[token])
            elif op == 'text':
                bodies += b'\x02' + put(ids[args[0]])
            else:
                bodies.append(0)
    return out + directory + bodies, dict(pool_and_header=pool_bytes, program_bytes=len(bodies),
        directory_and_count=len(directory)+len(put(len(documents))), symbols=len(ids))


def decode(blob):
    assert blob[:4] == b'FXB1'
    strings = []
    count, p = get(blob, 4)
    for _ in range(count):
        length, p = get(blob, p)
        strings.append(blob[p:p+length].decode())
        p += length
    directories = []
    count, p = get(blob, p)
    for _ in range(count):
        name, p = get(blob, p)
        directories.append((strings[name], struct.unpack_from('<I', blob, p)[0]))
        p += 4
    base = p
    documents = {}
    for i, (name, offset) in enumerate(directories):
        p = base + offset
        end = base + directories[i+1][1] if i+1 < len(directories) else len(blob)
        rows = []
        while p < end:
            op = blob[p]
            p += 1
            if op == 0:
                rows.append(('close',))
            elif op == 1:
                tag, p = get(blob, p)
                count, p = get(blob, p)
                row = ['open', strings[tag]]
                for _ in range(count*2):
                    token, p = get(blob, p)
                    row.append(strings[token])
                rows.append(tuple(row))
            else:
                assert op == 2
                token, p = get(blob, p)
                rows.append(('text', strings[token]))
        assert p == end
        documents[name] = rows
    return documents


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    output = parser.parse_args().output
    output.mkdir(exist_ok=True, parents=True)
    root = Path(__file__).resolve().parents[3]
    stats = {}
    for name, folder in [('hello', 'samples/HelloWorld'), ('winui', 'src/FerroUI.Themes.WinUI'),
                         ('catalog', 'samples/ControlCatalog')]:
        folder = root / folder
        paths = sorted(folder.rglob('*.xaml'))
        corpus = {p.relative_to(folder).as_posix(): events(p.read_bytes()) for p in paths}
        blob, parts = encode(corpus)
        assert decode(blob) == corpus
        (output / (name + '.fxb')).write_bytes(blob)
        stats[name] = dict(documents=len(paths), source_bytes=sum(p.stat().st_size for p in paths),
                           encoded_bytes=len(blob), **parts)
    (output / 'corpus.json').write_text(json.dumps(stats, indent=2)+'\n')
    print(json.dumps(stats, indent=2))

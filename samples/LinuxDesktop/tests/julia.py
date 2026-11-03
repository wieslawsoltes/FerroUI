#!/usr/bin/env python3
"""Generate the native image-viewer smoke fixture as a portable pixmap on stdout."""
import math
import sys

WIDTH, HEIGHT = 540, 328
RAMP = [(12, 28, 45), (33, 77, 103), (68, 139, 153), (186, 224, 197),
        (255, 218, 161), (225, 153, 115), (67, 61, 93), (12, 28, 45)]


def color_at(x, y):
    z = complex((x - WIDTH / 2) * 3.30 / WIDTH, (y - HEIGHT / 2) * 3.30 / WIDTH)
    for iteration in range(130):
        z = z * z + complex(-0.745, 0.113)
        radius = abs(z)
        if radius > 8:
            level = iteration + 1 - math.log2(math.log(radius))
            position = (level * 0.12) % (len(RAMP) - 1)
            index = int(position)
            t = position - index
            return bytes(round((1 - t) * a + t * b) for a, b in zip(RAMP[index], RAMP[index + 1]))
    return bytes((11, 23, 36))


if __name__ == "__main__":
    sys.stdout.buffer.write(f"P6\n{WIDTH} {HEIGHT}\n255\n".encode("ascii"))
    for row in range(HEIGHT):
        sys.stdout.buffer.write(b"".join(color_at(column, row) for column in range(WIDTH)))

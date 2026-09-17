#!/usr/bin/env python3
"""Render icons/panel.png — the module tile Basecamp shows in the App Manager.

The mark is a count that rises: three capsules stepping up, and above the
tallest a separate dot — the increment that has not landed yet. It reads at
sidebar size, where anything with interior detail would not.

Basecamp's icon contract wants a 256x256 PNG for a ui_qml package, so this
renders at 256 with 4x supersampling. No image library — a PNG is a zlib stream
of filtered scanlines, and drawing four shapes does not justify a dependency.

    python3 tools/make-icon.py
"""
import struct, zlib

SIZE, SS = 256, 4                      # output size, supersampling factor

BAR_W = 0.12                           # bar width in unit space
BAR_GAP = 0.07
BASE_Y = 0.30                          # baseline, y grows downward
HEIGHTS = (0.17, 0.29, 0.41)

INK = (0x3B, 0x82, 0xF6)               # the count so far
NEXT_INK = (0x4E, 0xC9, 0xA5)          # AppTheme.good — the increment to come

CAP_R = BAR_W / 2
XS = (-(BAR_W + BAR_GAP), 0.0, BAR_W + BAR_GAP)

# A bar is a capsule: its spine runs between the two cap centres, so the drawn
# height matches HEIGHTS exactly rather than overshooting by a radius at each end.
SPINES = [(x, BASE_Y - CAP_R, x, BASE_Y - h + CAP_R) for x, h in zip(XS, HEIGHTS)]

DOT_R = 0.055
DOT = (XS[2], BASE_Y - HEIGHTS[2] - DOT_R - 0.065)


def in_capsule(px, py, x0, y0, x1, y1, r):
    vx, vy = x1 - x0, y1 - y0
    wx, wy = px - x0, py - y0
    t = (wx * vx + wy * vy) / (vx * vx + vy * vy)
    t = max(0.0, min(1.0, t))
    dx, dy = wx - t * vx, wy - t * vy
    return dx * dx + dy * dy <= r * r


def sample(px, py):
    """Colour at a unit-space point, or None where nothing is drawn."""
    dx, dy = px - DOT[0], py - DOT[1]
    if dx * dx + dy * dy <= DOT_R * DOT_R:
        return NEXT_INK
    for s in SPINES:
        if in_capsule(px, py, *s, CAP_R):
            return INK
    return None


rows = []
for y in range(SIZE):
    row = bytearray()
    for x in range(SIZE):
        r = g = b = hits = 0
        for sy in range(SS):
            for sx in range(SS):
                px = (x + (sx + 0.5) / SS) / SIZE - 0.5
                py = (y + (sy + 0.5) / SS) / SIZE - 0.5
                c = sample(px, py)
                if c:
                    r += c[0]; g += c[1]; b += c[2]; hits += 1
        n = SS * SS
        if hits:
            # Colour is the mean over COVERED samples; alpha is what fraction
            # of the pixel they cover. Averaging colour over all n instead
            # would darken every edge pixel toward black.
            row += bytes((r // hits, g // hits, b // hits, 255 * hits // n))
        else:
            row += b"\0\0\0\0"
    rows.append(bytes(row))

raw = b"".join(b"\0" + r for r in rows)  # filter type 0 per scanline


def chunk(tag, data):
    return (struct.pack(">I", len(data)) + tag + data
            + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF))


png = (b"\x89PNG\r\n\x1a\n"
       + chunk(b"IHDR", struct.pack(">IIBBBBB", SIZE, SIZE, 8, 6, 0, 0, 0))
       + chunk(b"IDAT", zlib.compress(raw, 9))
       + chunk(b"IEND", b""))

with open("icons/panel.png", "wb") as f:
    f.write(png)
print(f"icons/panel.png — {SIZE}x{SIZE}, {len(png)} bytes")

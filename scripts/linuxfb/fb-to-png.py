#!/usr/bin/env python3
"""Writes the content of a framebuffer as a PNG file, with the standard library only.

  fb-to-png.py /dev/fb1 out.png                  a framebuffer device: its geometry is read from sysfs
  fb-to-png.py frame.raw out.png 640x400 32 bgra a raw file: size, bits per pixel, bgra|rgba|rgb565 [line length]

Reading a framebuffer device reads the memory of that device; nothing is captured from a screen.
"""
import os
import struct
import sys
import zlib


def geometry_of_device(node):
    name = os.path.basename(node)
    base = f"/sys/class/graphics/{name}"
    with open(f"{base}/virtual_size") as f:
        width, height = (int(v) for v in f.read().strip().split(","))
    with open(f"{base}/bits_per_pixel") as f:
        bits = int(f.read().strip())
    with open(f"{base}/stride") as f:
        stride = int(f.read().strip())
    return width, height, bits, stride


def main():
    if len(sys.argv) < 3:
        print(__doc__, file=sys.stderr)
        return 2
    source, target = sys.argv[1], sys.argv[2]
    order = "bgra"
    if len(sys.argv) >= 5:
        width, height = (int(v) for v in sys.argv[3].lower().split("x"))
        bits = int(sys.argv[4])
        order = sys.argv[5] if len(sys.argv) > 5 else ("rgb565" if bits == 16 else "bgra")
        stride = int(sys.argv[6]) if len(sys.argv) > 6 else width * bits // 8
    else:
        width, height, bits, stride = geometry_of_device(source)
        if bits == 16:
            order = "rgb565"
    with open(source, "rb") as f:
        data = f.read(stride * height)
    if len(data) < stride * height:
        print(f"{source}: {len(data)} bytes, {stride * height} expected", file=sys.stderr)
        return 1

    rows = bytearray()
    for y in range(height):
        row = data[y * stride:y * stride + width * bits // 8]
        rows.append(0)
        if bits == 32:
            if order == "rgba":
                rgb = bytearray(width * 3)
                rgb[0::3], rgb[1::3], rgb[2::3] = row[0::4], row[1::4], row[2::4]
            else:
                rgb = bytearray(width * 3)
                rgb[0::3], rgb[1::3], rgb[2::3] = row[2::4], row[1::4], row[0::4]
            rows += rgb
        elif bits == 16:
            for x in range(width):
                (v,) = struct.unpack_from("<H", row, x * 2)
                rows += bytes(((v >> 11) * 255 // 31, ((v >> 5) & 0x3F) * 255 // 63, (v & 0x1F) * 255 // 31))
        else:
            print(f"{bits} bits per pixel are not handled", file=sys.stderr)
            return 1

    def chunk(kind, payload):
        body = kind + payload
        return struct.pack(">I", len(payload)) + body + struct.pack(">I", zlib.crc32(body) & 0xFFFFFFFF)

    png = b"\x89PNG\r\n\x1a\n"
    png += chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 2, 0, 0, 0))
    png += chunk(b"IDAT", zlib.compress(bytes(rows), 6))
    png += chunk(b"IEND", b"")
    with open(target, "wb") as f:
        f.write(png)
    distinct = len(set(bytes(rows[i:i + 3]) for i in range(1, len(rows) - 3, 97)))
    print(f"{target}: {width}x{height}, {bits} bits per pixel, {distinct} distinct colours in a sample")
    return 0


if __name__ == "__main__":
    sys.exit(main())

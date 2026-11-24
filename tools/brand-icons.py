# SPDX-License-Identifier: MIT OR Apache-2.0
# Usage: python3 tools/brand-icons.py assets/brand 32 256 512
# Derive the smaller app icons from the brand's icon-1024.png by averaging pixel blocks (each size
# must divide 1024); stdlib only.
import struct, sys, zlib

def read_png(path):
    data = open(path, 'rb').read()
    assert data[:8] == b'\x89PNG\r\n\x1a\n', 'not a PNG'
    pos, idat, head = 8, b'', None
    while pos < len(data):
        n, kind = struct.unpack('>I4s', data[pos:pos + 8])
        body = data[pos + 8:pos + 8 + n]
        if kind == b'IHDR': head = struct.unpack('>IIBBBBB', body)
        elif kind == b'IDAT': idat += body
        pos += 12 + n
    w, h, depth, ctype, _, _, interlace = head
    assert depth == 8 and ctype == 6 and interlace == 0, 'expects 8-bit RGBA, not interlaced'
    raw, bpp, stride = zlib.decompress(idat), 4, w * 4
    rows, prev = [], bytearray(stride)
    for y in range(h):
        f, line = raw[y * (stride + 1)], bytearray(raw[y * (stride + 1) + 1:(y + 1) * (stride + 1)])
        for i in range(stride):
            a = line[i - bpp] if i >= bpp else 0
            b, c = prev[i], prev[i - bpp] if i >= bpp else 0
            if f == 1: line[i] = (line[i] + a) & 255
            elif f == 2: line[i] = (line[i] + b) & 255
            elif f == 3: line[i] = (line[i] + (a + b) // 2) & 255
            elif f == 4:
                p = a + b - c; pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
                line[i] = (line[i] + (a if pa <= pb and pa <= pc else b if pb <= pc else c)) & 255
        rows.append(line); prev = line
    return w, rows

def write_png(path, n, rows):
    def chunk(t, d): return struct.pack('>I', len(d)) + t + d + struct.pack('>I', zlib.crc32(t + d) & 0xffffffff)
    body = b''.join(b'\x00' + bytes(r) for r in rows)
    open(path, 'wb').write(b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', n, n, 8, 6, 0, 0, 0))
                           + chunk(b'IDAT', zlib.compress(body, 9)) + chunk(b'IEND', b''))

def shrink(w, rows, n):
    k = w // n
    out = []
    for j in range(n):
        line = bytearray()
        for i in range(n):
            acc = [0, 0, 0, 0]
            for y in range(j * k, (j + 1) * k):
                r = rows[y]
                for x in range(i * k, (i + 1) * k):
                    al = r[4 * x + 3]
                    for c in range(3): acc[c] += r[4 * x + c] * al
                    acc[3] += al
            al = acc[3]
            line += bytes([round(acc[c] / al) if al else 0 for c in range(3)] + [round(al / (k * k))])
        out.append(line)
    return out

folder = sys.argv[1]
w, rows = read_png(f'{folder}/icon-1024.png')
for n in map(int, sys.argv[2:]):
    assert w % n == 0, f'{n} does not divide {w}'
    write_png(f'{folder}/icon-{n}.png', n, shrink(w, rows, n))

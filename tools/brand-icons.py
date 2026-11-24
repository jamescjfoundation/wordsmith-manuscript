# SPDX-License-Identifier: MIT OR Apache-2.0
# Usage: python3 tools/brand-icons.py assets/brand 32 256 512
#        python3 tools/brand-icons.py assets/brand --ico
# Derive the smaller app icons from the brand's icon-1024.png by averaging pixel blocks (each size
# must divide 1024); stdlib only. With --ico, write icon.ico for the Windows program: 16, 24, 32 and
# 48 px drawn from favicon.svg (the version tuned for small sizes), 64, 128 and 256 px shrunk from
# icon-1024.png.
import re, struct, sys, zlib

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

def png_bytes(n, rows):
    def chunk(t, d): return struct.pack('>I', len(d)) + t + d + struct.pack('>I', zlib.crc32(t + d) & 0xffffffff)
    body = b''.join(b'\x00' + bytes(r) for r in rows)
    return (b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', n, n, 8, 6, 0, 0, 0))
            + chunk(b'IDAT', zlib.compress(body, 9)) + chunk(b'IEND', b''))

def write_png(path, n, rows):
    open(path, 'wb').write(png_bytes(n, rows))

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

# --- favicon.svg: a small rasterizer for the shapes it uses (rect, rounded rect, a path of M, L, H,
# V, Q, C and Z in absolute coordinates, solid fills), filled by the nonzero rule with exact
# horizontal coverage and 16 rows of samples per pixel.

def parse_color(c):
    c = c.lstrip('#')
    return tuple(int(c[i:i + 2], 16) for i in (0, 2, 4))

def attrs(tag):
    return dict(re.findall(r'([\w:-]+)="([^"]*)"', tag))

def rect_polygon(a):
    x, y, w, h = (float(a.get(k, 0)) for k in ('x', 'y', 'width', 'height'))
    r = min(float(a.get('rx', a.get('ry', 0))), w / 2, h / 2)
    if r <= 0:
        return [(x, y), (x + w, y), (x + w, y + h), (x, y + h)]
    import math
    pts, steps = [], 24
    for cx, cy, start in ((x + w - r, y + r, -90), (x + w - r, y + h - r, 0), (x + r, y + h - r, 90), (x + r, y + r, 180)):
        for i in range(steps + 1):
            t = math.radians(start + 90 * i / steps)
            pts.append((cx + r * math.cos(t), cy + r * math.sin(t)))
    return pts

def path_polygons(d):
    toks = re.findall(r'[MLHVQCZmlhvqcz]|-?(?:\d+\.?\d*|\.\d+)(?:[eE][-+]?\d+)?', d)
    polys, cur, pos, i, cmd = [], [], (0.0, 0.0), 0, None
    def num():
        nonlocal i
        v = float(toks[i]); i += 1
        return v
    while i < len(toks):
        if toks[i].isalpha():
            cmd = toks[i]; i += 1
            assert cmd.isupper(), f'relative path command {cmd} is not supported'
        if cmd == 'Z':
            if cur: polys.append(cur)
            cur = []
            continue
        if cmd == 'M':
            if cur: polys.append(cur)
            pos = (num(), num()); cur = [pos]; cmd = 'L'
        elif cmd == 'L': pos = (num(), num()); cur.append(pos)
        elif cmd == 'H': pos = (num(), pos[1]); cur.append(pos)
        elif cmd == 'V': pos = (pos[0], num()); cur.append(pos)
        elif cmd == 'Q':
            (x0, y0), (x1, y1), (x2, y2) = pos, (num(), num()), (num(), num())
            for k in range(1, 9):
                t = k / 8
                cur.append(((1 - t) ** 2 * x0 + 2 * (1 - t) * t * x1 + t * t * x2, (1 - t) ** 2 * y0 + 2 * (1 - t) * t * y1 + t * t * y2))
            pos = (x2, y2)
        elif cmd == 'C':
            (x0, y0), (x1, y1), (x2, y2), (x3, y3) = pos, (num(), num()), (num(), num()), (num(), num())
            for k in range(1, 13):
                t, u = k / 12, 1 - k / 12
                cur.append((u**3 * x0 + 3 * u * u * t * x1 + 3 * u * t * t * x2 + t**3 * x3, u**3 * y0 + 3 * u * u * t * y1 + 3 * u * t * t * y2 + t**3 * y3))
            pos = (x3, y3)
        else:
            raise ValueError(f'path command {cmd} is not supported')
    if cur: polys.append(cur)
    return polys

def coverage(polys, n, scale, sub=16):
    cov = [[0.0] * n for _ in range(n)]
    edges = []
    for poly in polys:
        for (xa, ya), (xb, yb) in zip(poly, poly[1:] + poly[:1]):
            if ya != yb: edges.append((xa * scale, ya * scale, xb * scale, yb * scale))
    for row in range(n * sub):
        y = (row + 0.5) / sub
        hits = []
        for xa, ya, xb, yb in edges:
            if (ya <= y < yb) or (yb <= y < ya):
                hits.append((xa + (y - ya) * (xb - xa) / (yb - ya), 1 if yb > ya else -1))
        hits.sort()
        wind, line = 0, cov[row // sub]
        for (x, w), nxt in zip(hits, hits[1:] + [(None, 0)]):
            wind += w
            if wind != 0 and nxt[0] is not None:
                x0, x1 = max(0.0, x), min(float(n), nxt[0])
                px = int(x0)
                while x0 < x1 and px < n:
                    right = min(x1, px + 1.0)
                    line[px] += (right - x0) / sub
                    x0, px = right, px + 1
    return cov

def render_svg(path, n):
    svg = open(path).read()
    vb = [float(v) for v in re.search(r'viewBox="([^"]+)"', svg).group(1).split()]
    scale = n / vb[2]
    px = [[[0.0, 0.0, 0.0, 0.0] for _ in range(n)] for _ in range(n)]  # premultiplied
    for tag in re.findall(r'<(?:rect|path)\b[^>]*>', svg):
        a = attrs(tag)
        polys = path_polygons(a['d']) if tag.startswith('<path') else [rect_polygon(a)]
        r, g, b = parse_color(a.get('fill', '#000000'))
        for y, line in enumerate(coverage(polys, n, scale)):
            for x, c in enumerate(line):
                c = min(1.0, c)
                if c <= 0: continue
                p = px[y][x]
                p[0] = r * c + p[0] * (1 - c); p[1] = g * c + p[1] * (1 - c); p[2] = b * c + p[2] * (1 - c)
                p[3] = c + p[3] * (1 - c)
    rows = []
    for line in px:
        out = bytearray()
        for r, g, b, al in line:
            out += bytes([round(r / al) if al else 0, round(g / al) if al else 0, round(b / al) if al else 0, round(al * 255)])
        rows.append(out)
    return rows

# --- icon.ico: 32-bit DIB entries below 256 px (read by every Windows version and resource
# compiler), a PNG entry at 256 px.

def dib(n, rows):
    head = struct.pack('<IiiHHIIiiII', 40, n, 2 * n, 1, 32, 0, 0, 0, 0, 0, 0)
    pixels = b''.join(bytes(v for x in range(n) for v in (r[4 * x + 2], r[4 * x + 1], r[4 * x], r[4 * x + 3])) for r in reversed(rows))
    stride = (n + 31) // 32 * 4
    mask = b''
    for r in reversed(rows):
        bits = bytearray(stride)
        for x in range(n):
            if r[4 * x + 3] == 0: bits[x // 8] |= 0x80 >> (x % 8)
        mask += bytes(bits)
    return head + pixels + mask

def write_ico(path, images):
    out, offset = [], 6 + 16 * len(images)
    entries = b''
    for n, rows in images:
        data = png_bytes(n, rows) if n >= 256 else dib(n, rows)
        entries += struct.pack('<BBBBHHII', n % 256, n % 256, 0, 0, 1, 32, len(data), offset)
        out.append(data); offset += len(data)
    open(path, 'wb').write(struct.pack('<HHH', 0, 1, len(images)) + entries + b''.join(out))

ICO_FROM_FAVICON = (16, 24, 32, 48)
ICO_FROM_ICON = (64, 128, 256)

if __name__ == '__main__':
    folder = sys.argv[1]
    w, rows = read_png(f'{folder}/icon-1024.png')
    if sys.argv[2:] == ['--ico']:
        images = [(n, render_svg(f'{folder}/favicon.svg', n)) for n in ICO_FROM_FAVICON]
        images += [(n, shrink(w, rows, n)) for n in ICO_FROM_ICON]
        write_ico(f'{folder}/icon.ico', images)
    else:
        for n in map(int, sys.argv[2:]):
            assert w % n == 0, f'{n} does not divide {w}'
            write_png(f'{folder}/icon-{n}.png', n, shrink(w, rows, n))

"""Asset decoders: Texture2D pixels, Mesh, Material, AudioClip, Font, and a tiny PNG writer."""
import struct
import zlib

from sfile import Reader

# --- PNG -------------------------------------------------------------------


def write_png(path, w, h, rgba_top_down):
    def chunk(tag, data):
        c = struct.pack(">I", len(data)) + tag + data
        return c + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)
    rows = b"".join(b"\0" + rgba_top_down[y * w * 4:(y + 1) * w * 4] for y in range(h))
    png = b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 6, 0, 0, 0))
    png += chunk(b"IDAT", zlib.compress(rows, 9)) + chunk(b"IEND", b"")
    with open(path, "wb") as fh:
        fh.write(png)


# --- ETC1 ------------------------------------------------------------------

_ETC1_TABLES = [(2, 8), (5, 17), (9, 29), (13, 42), (18, 60), (24, 80), (33, 106), (47, 183)]


def _clamp(v):
    return 0 if v < 0 else 255 if v > 255 else v


def _etc1_block(block):
    hi, lo = struct.unpack(">II", block)
    diff = (hi >> 1) & 1
    flip = hi & 1
    if diff:
        def ext5(v): return (v << 3) | (v >> 2)
        r1, g1, b1 = (hi >> 27) & 31, (hi >> 19) & 31, (hi >> 11) & 31
        dr, dg, db = (hi >> 24) & 7, (hi >> 16) & 7, (hi >> 8) & 7
        dr, dg, db = [d - 8 if d >= 4 else d for d in (dr, dg, db)]
        base = [(ext5(r1), ext5(g1), ext5(b1)), (ext5(r1 + dr), ext5(g1 + dg), ext5(b1 + db))]
    else:
        def ext4(v): return (v << 4) | v
        base = [(ext4((hi >> 28) & 15), ext4((hi >> 20) & 15), ext4((hi >> 12) & 15)),
                (ext4((hi >> 24) & 15), ext4((hi >> 16) & 15), ext4((hi >> 8) & 15))]
    tables = [(hi >> 5) & 7, (hi >> 2) & 7]
    out = [[None] * 4 for _ in range(4)]  # out[y][x]
    for x in range(4):
        for y in range(4):
            i = x * 4 + y
            sub = (y >= 2) if flip else (x >= 2)
            msb = (lo >> (16 + i)) & 1
            lsb = (lo >> i) & 1
            small, big = _ETC1_TABLES[tables[sub]]
            mod = [small, big, -small, -big][(msb << 1) | lsb]
            r, g, b = base[sub]
            out[y][x] = (_clamp(r + mod), _clamp(g + mod), _clamp(b + mod), 255)
    return out


def decode_pixels(fmt, w, h, data):
    """Returns RGBA bytes in Unity's row order (bottom row first)."""
    px = bytearray(w * h * 4)
    if fmt == 13:  # RGBA4444
        for i in range(w * h):
            v = data[2 * i] | (data[2 * i + 1] << 8)
            px[4 * i:4 * i + 4] = bytes((((v >> 12) & 15) * 17, ((v >> 8) & 15) * 17,
                                         ((v >> 4) & 15) * 17, (v & 15) * 17))
    elif fmt == 3:  # RGB24
        for i in range(w * h):
            px[4 * i:4 * i + 4] = data[3 * i:3 * i + 3] + b"\xff"
    elif fmt == 4:  # RGBA32
        px[:] = data[:w * h * 4]
    elif fmt == 34:  # ETC1, blocks in raster order
        bw = (w + 3) // 4
        for bi in range((w // 4) * (h // 4)):
            bx, by = bi % bw, bi // bw
            blk = _etc1_block(data[bi * 8:bi * 8 + 8])
            for y in range(4):
                for x in range(4):
                    o = ((by * 4 + y) * w + bx * 4 + x) * 4
                    px[o:o + 4] = bytes(blk[y][x])
    else:
        raise ValueError(f"texture format {fmt} not supported")
    return bytes(px)


def flip_rows(px, w, h):
    stride = w * 4
    return b"".join(px[(h - 1 - y) * stride:(h - y) * stride] for y in range(h))


# --- object layouts (Unity 4.3, stripped type trees) -------------------------


def texture2d(r: Reader):
    name = r.string()
    w, h, size, fmt = r.i32(), r.i32(), r.i32(), r.i32()
    r.bytes_(4)  # mipmap, readable, readAllowed, pad
    for _ in range(8):
        r.i32()  # imageCount, dim, filter, aniso, mipBias, wrap, lightmapFmt, colorSpace
    n = r.i32()
    data = r.bytes_(n)
    return dict(name=name, w=w, h=h, fmt=fmt, data=data)


def mesh(r: Reader):
    name = r.string()
    subs = []
    for _ in range(r.i32()):
        first_byte, index_count, strip = r.u32(), r.u32(), r.i32()
        first_vertex, vertex_count = r.u32(), r.u32()
        r.vec3(); r.vec3()
        subs.append((first_byte, index_count, strip))
    for _ in range(4):  # blend shape vectors
        assert r.i32() == 0
    assert r.i32() == 0  # bind poses
    assert r.i32() == 0  # bone name hashes
    r.u32()  # root bone hash
    r.bytes_(5); r.align()
    ib = r.bytes_(r.i32()); r.align()
    assert r.i32() == 0  # skin
    r.u32()  # current channels
    vcount = r.u32()
    channels = [tuple(r.bytes_(4)) for _ in range(r.i32())]
    streams = []
    for _ in range(r.i32()):
        streams.append((r.u32(), r.u32(), r.u8(), r.u8(), r.u16()))
    vdata = r.bytes_(r.i32())
    pos, uv = [], []
    for v in range(vcount):
        for ci, (stream, offset, fmt, dim) in enumerate(channels):
            if dim == 0 or ci not in (0, 3):
                continue
            mask, soff, stride, _, _ = streams[stream]
            o = soff + v * stride + offset
            vals = struct.unpack_from("<%df" % dim, vdata, o)
            (pos if ci == 0 else uv).append(vals)
    indices = []
    for first_byte, count, strip in subs:
        idx = list(struct.unpack_from("<%dH" % count, ib, first_byte))
        if strip:
            tri = []
            for i in range(len(idx) - 2):
                a, b, c = idx[i], idx[i + 1], idx[i + 2]
                if a == b or b == c or a == c:
                    continue
                tri += [a, b, c] if i % 2 == 0 else [b, a, c]
            idx = tri
        indices += idx
    return dict(name=name, pos=pos, uv=uv or [(0, 0)] * len(pos), indices=indices)


def material(r: Reader):
    name = r.string()
    shader = r.pptr()
    for _ in range(r.i32()):
        r.string()
    r.i32()  # custom render queue
    texenvs = {}
    for _ in range(r.i32()):
        key = r.string()
        tex = r.pptr()
        texenvs[key] = dict(tex=tex, scale=r.vec2(), offset=r.vec2())
    floats = {}
    for _ in range(r.i32()):
        key = r.string()
        floats[key] = r.f32()
    colors = {}
    for _ in range(r.i32()):
        key = r.string()
        colors[key] = r.color()
    return dict(name=name, shader=shader, tex=texenvs, floats=floats, colors=colors)


def audioclip(r: Reader):
    name = r.string()
    fmt, typ = r.i32(), r.i32()
    r.bytes_(2); r.align()
    r.i32()  # stream mode
    data = r.bytes_(r.i32())
    return dict(name=name, format=fmt, type=typ, data=data)


def sniff_audio(data):
    if data[:4] == b"OggS":
        return "ogg"
    if data[:3] == b"ID3" or (data[0] == 0xFF and (data[1] & 0xE0) == 0xE0):
        return "mp3"
    if data[:4] == b"RIFF":
        return "wav"
    return "bin"


def font_ttf(raw):
    for sig in (b"\x00\x01\x00\x00", b"OTTO", b"true"):
        i = raw.find(sig)
        while i >= 0:
            # TrueType offset table: numTables follows, sane range
            nt = struct.unpack_from(">H", raw, i + 4)[0]
            if 4 <= nt <= 40 and i >= 4:
                n = struct.unpack_from("<i", raw, i - 4)[0]
                if 0 < n <= len(raw) - i:
                    return raw[i:i + n]
            i = raw.find(sig, i + 1)
    return None

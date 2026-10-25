"""Extract Square Lights assets + scene graphs into web-friendly files.

usage: python3 -I extract.py <unity Data dir with merged .assets> <out dir>
"""
import json
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from sfile import SerializedFile  # noqa: E402
from scene import Scene, load_scripts  # noqa: E402
import decode  # noqa: E402

SRC, OUT = sys.argv[1], sys.argv[2]
FILES = {"sharedassets0.assets": "sa0", "sharedassets1.assets": "sa1"}


def key_of(sf, pptr):
    fid, pid = pptr
    if pid == 0:
        return None
    fname = sf.name if fid == 0 else sf.externals[fid - 1]
    short = FILES.get(fname)
    return f"{short}:{pid}" if short else None


def main():
    os.makedirs(f"{OUT}/tex", exist_ok=True)
    os.makedirs(f"{OUT}/audio", exist_ok=True)
    os.makedirs(f"{OUT}/fonts", exist_ok=True)
    shared = {n: SerializedFile(f"{SRC}/{n}") for n in FILES}
    scripts = load_scripts(shared["sharedassets0.assets"])
    textures, meshes, materials, audio, fonts = {}, {}, {}, {}, {}

    for fname, sf in shared.items():
        for pid, (_, _, cid, _) in sf.objects.items():
            key = f"{FILES[fname]}:{pid}"
            r = sf.reader(pid)
            if cid == 28:
                t = decode.texture2d(r)
                if t["w"] == 0:
                    continue
                px = decode.decode_pixels(t["fmt"], t["w"], t["h"], t["data"])
                fn = f"tex/{t['name']}.png"
                decode.write_png(f"{OUT}/{fn}", t["w"], t["h"], decode.flip_rows(px, t["w"], t["h"]))
                textures[key] = dict(name=t["name"], file=fn, w=t["w"], h=t["h"])
            elif cid == 43:
                meshes[key] = decode.mesh(r)
            elif cid == 21:
                m = decode.material(r)
                shader = key_of(sf, m["shader"])
                shader_name = None
                if shader:
                    s_sf = shared[[n for n, s in FILES.items() if s == shader.split(":")[0]][0]]
                    shader_name = s_sf.reader(int(shader.split(":")[1])).string()
                main = m["tex"].get("_MainTex", {})
                materials[key] = dict(
                    name=m["name"], shader=shader_name,
                    tex=key_of(sf, main["tex"]) if main else None,
                    scale=main.get("scale", (1, 1)), offset=main.get("offset", (0, 0)),
                    color=m["colors"].get("_Color") or m["colors"].get("_TintColor"),
                    colors=m["colors"])
            elif cid == 83:
                a = decode.audioclip(r)
                ext = decode.sniff_audio(a["data"])
                fn = f"audio/{a['name'].replace(' ', '_')}.{ext}"
                with open(f"{OUT}/{fn}", "wb") as fh:
                    fh.write(a["data"])
                audio[key] = dict(name=a["name"], file=fn, type=a["type"])
            elif cid == 128:
                name = r.string()
                ttf = decode.font_ttf(sf.raw(pid))
                if ttf:
                    fn = f"fonts/{name}.ttf"
                    with open(f"{OUT}/{fn}", "wb") as fh:
                        fh.write(ttf)
                    fonts[key] = dict(name=name, file=fn)

    scenes = {}
    for level, label in (("level0", "game"), ("mainData", "menu")):
        sf = SerializedFile(f"{SRC}/{level}")
        sc = Scene(sf, scripts)
        nodes = []
        index = {n.go: i for i, n in enumerate(sc.nodes.values())}
        for n in sc.nodes.values():
            d = dict(name=n.name, active=n.active,
                     parent=index[n.parent.go] if n.parent else -1,
                     pos=n.lpos, rot=n.lrot, scale=n.lscale,
                     scripts=[s for s, _ in sc.behaviours(n)])
            for mf in sc.comp(n, 33):
                r = sf.reader(mf); r.pptr()
                d["mesh"] = key_of(sf, r.pptr())
            for mr in sc.comp(n, 23):
                r = sf.reader(mr); r.pptr()
                d["visible"] = r.u8() == 1
                r.bytes_(3); r.vec2(); r.vec2()
                d["materials"] = [key_of(sf, r.pptr()) for _ in range(r.i32())]
            for tm in sc.comp(n, 102):
                r = sf.reader(tm); r.pptr()
                text = r.string()
                offz, csize, lsp = r.f32(), r.f32(), r.f32()
                anchor, align = r.i16(), r.i16()
                tab, fsize, fstyle = r.f32(), r.i32(), r.i32()
                r.u8(); r.align()
                font = key_of(sf, r.pptr())
                d["text"] = dict(text=text, charSize=csize, lineSpacing=lsp, anchor=anchor,
                                 align=align, fontSize=fsize, style=fstyle, font=font)
            for gt in sc.comp(n, 131):
                r = sf.reader(gt); r.pptr(); r.u8(); r.align()
                tex = key_of(sf, r.pptr())
                color = r.color()
                inset = (r.f32(), r.f32(), r.f32(), r.f32())
                d["guiTexture"] = dict(tex=tex, color=color, inset=inset)
            for bc in sc.comp(n, 65):
                r = sf.reader(bc); r.pptr(); r.pptr()
                r.u8(); enabled = r.u8() == 1; r.align()
                if enabled:
                    d["box"] = dict(size=r.vec3(), center=r.vec3())
            for cam in sc.comp(n, 20):
                r = sf.reader(cam); r.pptr(); r.u8(); r.align()
                r.u32()
                bg = r.color()
                r.f32(); r.f32(); r.f32(); r.f32()
                near, far, fov = r.f32(), r.f32(), r.f32()
                ortho = r.u8() == 1; r.align()
                d["camera"] = dict(bg=bg, near=near, far=far, fov=fov, ortho=ortho, orthoSize=r.f32())
            nodes.append(d)
        scenes[label] = nodes

    with open(f"{OUT}/assets.json", "w") as fh:
        json.dump(dict(textures=textures, meshes=meshes, materials=materials,
                       audio=audio, fonts=fonts), fh)
    for label, nodes in scenes.items():
        with open(f"{OUT}/scene_{label}.json", "w") as fh:
            json.dump(nodes, fh)
    print(f"textures={len(textures)} meshes={len(meshes)} materials={len(materials)} "
          f"audio={len(audio)} fonts={len(fonts)} " +
          " ".join(f"{k}={len(v)} nodes" for k, v in scenes.items()))


main()

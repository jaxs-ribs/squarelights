// Square Lights host: loads the WASM simulation, feeds it input at a fixed
// 50 Hz, renders its instance records with WebGPU and plays its sounds.
//
// URL flags: ?debug (overlay), ?start=game (skip menu), ?seed=N, ?autoplay (bot holds jump)

import init, { Engine } from "./pkg/squarelights.js";

const params = new URLSearchParams(location.search);
const TICK = 1 / 50;
const VIRTUAL_W = 800, VIRTUAL_H = 480, ASPECT = VIRTUAL_W / VIRTUAL_H;
const RECORD = 28;
const PREFS_KEY = "squarelights.prefs";

const $ = (id) => document.getElementById(id);
const statusEl = $("status"), overlay = $("overlay"), debugEl = $("debug"), canvas = $("screen");

// Everything a test harness needs to observe the running game.
const sq = (window.__sq = {
  ready: false, errors: [], fps: 0, ticks: 0, instances: 0, drawCalls: 0, textures: 0,
  state: () => (engine ? JSON.parse(engine.state()) : null),
  key: (code, down) => engine && engine.key(code, down),
  tap: (x, y) => { if (!engine) return; engine.pointer(77, x, y, 0); pendingUp.push([77, x, y]); },
});
window.addEventListener("error", (e) => sq.errors.push(String(e.message || e)));
window.addEventListener("unhandledrejection", (e) => sq.errors.push(String(e.reason)));

let engine = null;
const pendingUp = [];

const freshPrefs = params.has("fresh"); // ignore and don't touch saved prefs
function loadPrefs() {
  if (freshPrefs) return "{}";
  try { return localStorage.getItem(PREFS_KEY) || "{}"; } catch { return "{}"; }
}
function savePrefs(json) {
  if (freshPrefs) return;
  try { localStorage.setItem(PREFS_KEY, json); } catch { /* private mode: prefs live for the session */ }
}

async function fetchJSON(url) {
  const r = await fetch(url);
  if (!r.ok) throw new Error(`${url}: ${r.status}`);
  return r.text();
}

// ---------------------------------------------------------------- audio ---
class Audio {
  constructor(assets) {
    this.ctx = new (window.AudioContext || window.webkitAudioContext)();
    this.buffers = new Map();
    this.music = null; // {clip, node, gain, state}
    this.files = Object.values(assets.audio).map((a) => a.file);
  }
  async load() {
    await Promise.all(this.files.map(async (file) => {
      const name = file.split("/").pop().replace(/\.[^.]+$/, "");
      try {
        const data = await (await fetch(`assets/${file}`)).arrayBuffer();
        this.buffers.set(name, await this.ctx.decodeAudioData(data));
      } catch (e) { sq.errors.push(`audio ${file}: ${e}`); }
    }));
  }
  unlock() { if (this.ctx.state !== "running") this.ctx.resume(); }
  shot(name) {
    const b = this.buffers.get(name);
    if (!b || this.ctx.state !== "running") return;
    const s = this.ctx.createBufferSource();
    s.buffer = b;
    s.connect(this.ctx.destination);
    s.start();
  }
  sync(m) {
    const clip = m.clip;
    if (this.music && this.music.state === "stopped" && m.state === "playing") this.reset();
    if (!this.music || this.music.clip !== clip) {
      if (this.music) this.music.node.stop();
      const b = this.buffers.get(clip);
      if (!b) return;
      const node = this.ctx.createBufferSource();
      node.buffer = b; node.loop = true;
      const gain = this.ctx.createGain();
      node.connect(gain).connect(this.ctx.destination);
      node.start();
      this.music = { clip, node, gain, state: "playing" };
    }
    const audible = m.state === "playing" && !m.mute;
    this.music.gain.gain.value = audible ? m.volume : 0;
    if (m.state === "stopped" && this.music.state !== "stopped") { this.music.node.stop(); this.music.state = "stopped"; }
    else if (m.state !== "stopped") this.music.state = m.state;
  }
  // a stopped source can't restart; a scene reload changes the clip name back
  reset() { if (this.music) { try { this.music.node.stop(); } catch {} this.music = null; } }
}

// ------------------------------------------------------------- renderer ---
const WGSL = /* wgsl */ `
struct Cam { viewProj: mat4x4f };
struct Inst { head: vec4f, m0: vec4f, m1: vec4f, m2: vec4f, m3: vec4f, uv: vec4f, color: vec4f };
@group(0) @binding(0) var<uniform> cam: Cam;
@group(0) @binding(1) var<storage, read> insts: array<Inst>;
@group(1) @binding(0) var tex: texture_2d<f32>;
@group(1) @binding(1) var samp: sampler;

struct VOut { @builtin(position) pos: vec4f, @location(0) uv: vec2f, @location(1) color: vec4f };

@vertex fn vs(@location(0) p: vec3f, @location(1) uv: vec2f, @builtin(instance_index) ii: u32) -> VOut {
  let inst = insts[ii];
  let model = mat4x4f(inst.m0, inst.m1, inst.m2, inst.m3);
  var o: VOut;
  o.pos = cam.viewProj * model * vec4f(p, 1.0);
  let t = uv * inst.uv.xy + inst.uv.zw;
  o.uv = vec2f(t.x, 1.0 - t.y);   // Unity UV origin is bottom-left; textures are stored top-down
  o.color = inst.color;
  return o;
}
// Mobile/Unlit (Supports Lightmap) without a lightmap: plain texture
@fragment fn fs_opaque(i: VOut) -> @location(0) vec4f {
  return vec4f(textureSample(tex, samp, i.uv).rgb, 1.0);
}
// Mobile/Particles/*: texture * vertex color (meshes carry none -> white)
@fragment fn fs_particle(i: VOut) -> @location(0) vec4f {
  return textureSample(tex, samp, i.uv);
}
// GUI/Text Shader: glyph alpha tinted by material color
@fragment fn fs_text(i: VOut) -> @location(0) vec4f {
  return vec4f(i.color.rgb, i.color.a * textureSample(tex, samp, i.uv).a);
}`;

class Renderer {
  static async create(canvas) {
    if (!navigator.gpu) throw new Error("WebGPU is not available in this browser.");
    const adapter = await navigator.gpu.requestAdapter({ powerPreference: "high-performance" });
    if (!adapter) throw new Error("No WebGPU adapter found.");
    const device = await adapter.requestDevice();
    device.lost.then((i) => sq.errors.push(`device lost: ${i.message}`));
    device.addEventListener?.("uncapturederror", (e) => sq.errors.push(`gpu: ${e.error.message}`));
    return new Renderer(canvas, adapter, device);
  }

  constructor(canvas, adapter, device) {
    this.canvas = canvas;
    this.device = device;
    this.adapterInfo = adapter.info ? `${adapter.info.vendor} ${adapter.info.architecture}`.trim() : "";
    this.ctx = canvas.getContext("webgpu");
    this.format = navigator.gpu.getPreferredCanvasFormat();
    this.ctx.configure({ device, format: this.format, alphaMode: "opaque" });
    const module = device.createShaderModule({ code: WGSL });
    this.camBuf = device.createBuffer({ size: 64, usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST });
    this.instBuf = null;
    this.instCap = 0;
    this.layout0 = device.createBindGroupLayout({ entries: [
      { binding: 0, visibility: GPUShaderStage.VERTEX, buffer: { type: "uniform" } },
      { binding: 1, visibility: GPUShaderStage.VERTEX, buffer: { type: "read-only-storage" } },
    ] });
    this.layout1 = device.createBindGroupLayout({ entries: [
      { binding: 0, visibility: GPUShaderStage.FRAGMENT, texture: {} },
      { binding: 1, visibility: GPUShaderStage.FRAGMENT, sampler: {} },
    ] });
    const layout = device.createPipelineLayout({ bindGroupLayouts: [this.layout0, this.layout1] });
    const vertex = { module, entryPoint: "vs", buffers: [{ arrayStride: 20, attributes: [
      { shaderLocation: 0, offset: 0, format: "float32x3" },
      { shaderLocation: 1, offset: 12, format: "float32x2" },
    ] }] };
    const blend = (src, dst) => ({ color: { srcFactor: src, dstFactor: dst, operation: "add" },
      alpha: { srcFactor: "one", dstFactor: "one-minus-src-alpha", operation: "add" } });
    const mk = (entry, blendState, depthWrite, depthCompare = "less-equal") => device.createRenderPipeline({
      layout, vertex,
      fragment: { module, entryPoint: entry, targets: [{ format: this.format, blend: blendState }] },
      primitive: { topology: "triangle-list", cullMode: "none" },
      depthStencil: { format: "depth24plus", depthWriteEnabled: depthWrite, depthCompare },
    });
    this.pipelines = {
      opaque: mk("fs_opaque", undefined, true),
      alpha: mk("fs_particle", blend("src-alpha", "one-minus-src-alpha"), false),
      additive: mk("fs_particle", blend("src-alpha", "one"), false),
      // GUI/Text Shader is ZTest Always: 3D text draws over everything
      text: mk("fs_text", blend("src-alpha", "one-minus-src-alpha"), false, "always"),
    };
    this.sampler = device.createSampler({ magFilter: "linear", minFilter: "linear", addressModeU: "clamp-to-edge", addressModeV: "clamp-to-edge" });
    this.white = this.textureFromPixels(new Uint8Array([255, 255, 255, 255]), 1, 1);
    this.atlases = new Map();
    this.out = new Float32Array(RECORD * 1024);
    this.ops = [];
    this.depth = null;
  }

  textureFromPixels(px, w, h) {
    const t = this.device.createTexture({ size: [w, h], format: "rgba8unorm",
      usage: GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.COPY_DST });
    this.device.queue.writeTexture({ texture: t }, px, { bytesPerRow: w * 4 }, [w, h]);
    return t;
  }

  textureFromSource(src) {
    const t = this.device.createTexture({ size: [src.width, src.height], format: "rgba8unorm",
      usage: GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.COPY_DST | GPUTextureUsage.RENDER_ATTACHMENT });
    this.device.queue.copyExternalImageToTexture({ source: src }, { texture: t }, [src.width, src.height]);
    return t;
  }

  bindTexture(t) {
    return this.device.createBindGroup({ layout: this.layout1, entries: [
      { binding: 0, resource: t.createView() }, { binding: 1, resource: this.sampler },
    ] });
  }

  async loadAssets(assets, tables) {
    const d = this.device;
    this.meshes = tables.meshes.map((key) => {
      const m = assets.meshes[key];
      const v = new Float32Array(m.pos.length * 5);
      m.pos.forEach((p, i) => v.set([p[0], p[1], p[2], m.uv[i][0], m.uv[i][1]], i * 5));
      const idx = new Uint32Array(m.indices);
      const vb = d.createBuffer({ size: v.byteLength, usage: GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_DST });
      const ib = d.createBuffer({ size: idx.byteLength, usage: GPUBufferUsage.INDEX | GPUBufferUsage.COPY_DST });
      d.queue.writeBuffer(vb, 0, v);
      d.queue.writeBuffer(ib, 0, idx);
      return { vb, ib, count: idx.length };
    });
    // unit quad for text, origin at the upper-left corner (TextMesh anchor 0)
    const q = new Float32Array([0, 0, 0, 0, 1, 1, 0, 0, 1, 1, 1, -1, 0, 1, 0, 0, -1, 0, 0, 0]);
    const qi = new Uint32Array([0, 1, 2, 0, 2, 3]);
    const qvb = d.createBuffer({ size: q.byteLength, usage: GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_DST });
    const qib = d.createBuffer({ size: qi.byteLength, usage: GPUBufferUsage.INDEX | GPUBufferUsage.COPY_DST });
    d.queue.writeBuffer(qvb, 0, q);
    d.queue.writeBuffer(qib, 0, qi);
    this.quad = { vb: qvb, ib: qib, count: 6 };

    const texByKey = new Map();
    await Promise.all(Object.entries(assets.textures).map(async ([key, t]) => {
      const img = await createImageBitmap(await (await fetch(`assets/${t.file}`)).blob(), { premultiplyAlpha: "none", colorSpaceConversion: "none" });
      texByKey.set(key, this.textureFromSource(img));
    }));
    this.materials = tables.materials.map((key) => {
      const m = assets.materials[key];
      const shader = m.shader || "";
      const kind = shader.includes("Particle-Add") ? "additive" : shader.includes("Particle") ? "alpha"
        : m.name.includes("Font") ? "text" : "opaque";
      const tex = (m.tex && texByKey.get(m.tex)) || this.white;
      return { name: m.name, kind, group: this.bindTexture(tex) };
    });
    sq.textures = texByKey.size;
  }

  // One glyph atlas per (font, size, style). Unity draws dynamic-font glyphs
  // at fontSize pixels with 0.1 * characterSize world units per pixel; text is
  // drawn as one instanced quad per glyph, so changing strings (the score
  // ticks every frame) never allocates GPU memory.
  atlas(font, fontSize, style, fontFamilies) {
    const key = `${font}|${fontSize}|${style}`;
    let a = this.atlases.get(key);
    if (a) return a;
    const scale = 2, px = fontSize * scale;
    const css = `${style & 1 ? "bold " : ""}${style & 2 ? "italic " : ""}${px}px "${fontFamilies[font] || "sans-serif"}"`;
    const c = document.createElement("canvas");
    const g = c.getContext("2d");
    g.font = css;
    const lineH = Math.ceil(px * 1.15), W = 2048, glyphs = new Map();
    let x = 0, y = 0;
    const cells = [];
    for (let code = 32; code < 127; code++) {
      const ch = String.fromCharCode(code);
      const adv = g.measureText(ch).width, w = Math.ceil(adv) + 4;
      if (x + w > W) { x = 0; y += lineH; }
      cells.push([code, ch, x, y, w, adv]);
      x += w;
    }
    c.width = W;
    c.height = y + lineH;
    g.font = css;
    g.fillStyle = "#fff";
    g.textBaseline = "top";
    for (const [code, ch, cx, cy, w, adv] of cells) {
      g.fillText(ch, cx + 2, cy + Math.round(px * 0.08));
      glyphs.set(code, { u: cx / W, v: cy / c.height, w: w / W, h: lineH / c.height, pw: w, adv });
    }
    // pen positions measured on whole prefixes so the font's kerning/spacing applies
    const mg = document.createElement("canvas").getContext("2d");
    mg.font = css;
    const layouts = new Map();
    const layout = (text) => {
      let l = layouts.get(text);
      if (!l) {
        l = new Float32Array(text.length);
        for (let i = 1; i < text.length; i++) l[i] = mg.measureText(text.slice(0, i)).width;
        if (layouts.size > 256) layouts.clear();
        layouts.set(text, l);
      }
      return l;
    };
    a = { group: this.bindTexture(this.textureFromSource(c)), glyphs, lineH, scale, layout };
    this.atlases.set(key, a);
    return a;
  }

  resize() {
    const dpr = Math.min(window.devicePixelRatio || 1, 3);
    const w = Math.max(1, Math.round(this.canvas.clientWidth * dpr));
    const h = Math.max(1, Math.round(this.canvas.clientHeight * dpr));
    if (this.canvas.width !== w || this.canvas.height !== h || !this.depth) {
      this.canvas.width = w;
      this.canvas.height = h;
      this.depth?.destroy();
      this.depth = this.device.createTexture({ size: [w, h], format: "depth24plus", usage: GPUTextureUsage.RENDER_ATTACHMENT });
      this.depthView = this.depth.createView();
    }
    // letterbox to the original 800x480 (5:3) frame
    let vw = w, vh = Math.round(w / ASPECT);
    if (vh > h) { vh = h; vw = Math.round(h * ASPECT); }
    this.vp = { x: Math.floor((w - vw) / 2), y: Math.floor((h - vh) / 2), w: vw, h: vh, dpr };
  }

  draw(records, texts, cam, fontFamilies) {
    this.resize();
    const d = this.device;
    const [, , , half, near, far, r, g, b] = cam;
    const v = cam.slice(9, 25);
    // ortho projection * camera view (Unity cameras look down +Z in view space)
    const sx = 1 / (half * ASPECT), sy = 1 / half, sz = 1 / (far - near), tz = -near * sz;
    const vp = (this.vpMat ||= new Float32Array(16));
    for (let c = 0; c < 4; c++) {
      vp[c * 4 + 0] = sx * v[c * 4 + 0];
      vp[c * 4 + 1] = sy * v[c * 4 + 1];
      vp[c * 4 + 2] = sz * v[c * 4 + 2] + tz * v[c * 4 + 3];
      vp[c * 4 + 3] = v[c * 4 + 3];
    }
    d.queue.writeBuffer(this.camBuf, 0, vp);

    // Copy mesh records and expand text records into glyph records, building
    // the draw list as we go (consecutive records with the same state batch).
    const n = records.length / RECORD;
    let out = this.out, w = 0;
    const ops = this.ops;
    let nops = 0;
    const need = (k) => {
      if (w + k * RECORD > out.length) {
        const grown = new Float32Array(Math.max(out.length * 2, w + k * RECORD + 4096));
        grown.set(out.subarray(0, w));
        out = this.out = grown;
      }
    };
    const op = (type, a, b, first) => {
      const last = nops ? ops[nops - 1] : null;
      if (last && last.type === type && last.a === a && last.b === b) { last.count++; return; }
      if (!ops[nops]) ops[nops] = {};
      const o = ops[nops++];
      o.type = type; o.a = a; o.b = b; o.first = first; o.count = 1;
    };
    for (let i = 0; i < n; i++) {
      const o = i * RECORD;
      if (records[o] === 0) {
        need(1);
        out.set(records.subarray(o, o + RECORD), w);
        op(0, records[o + 1], records[o + 2], w / RECORD);
        w += RECORD;
        continue;
      }
      const t = texts[records[o + 1]];
      if (!t) continue;
      const [text, font, fontSize, charSize, , , style] = t;
      const at = this.atlas(font, fontSize, style, fontFamilies);
      const k = (0.1 * charSize) / at.scale;
      need(text.length);
      const pens = at.layout(text);
      for (let ci = 0; ci < text.length; ci++) {
        const pen = pens[ci];
        const gl = at.glyphs.get(text.charCodeAt(ci)) || at.glyphs.get(63);
        out.set(records.subarray(o, o + RECORD), w);
        const gw = gl.pw * k, gh = at.lineH * k, px = (pen - 2) * k;
        for (let r = 0; r < 4; r++) {
          const c0 = records[o + 4 + r], c1 = records[o + 8 + r];
          out[w + 4 + r] = c0 * gw;
          out[w + 8 + r] = c1 * gh;
          out[w + 16 + r] = records[o + 16 + r] + c0 * px;
        }
        out[w + 20] = gl.w; out[w + 21] = gl.h; out[w + 22] = gl.u; out[w + 23] = 1 - gl.h - gl.v;
        op(1, at, 0, w / RECORD);
        w += RECORD;
      }
    }
    const bytes = Math.max(256, w * 4);
    if (bytes > this.instCap) {
      this.instCap = Math.max(bytes, this.instCap * 2);
      this.instBuf?.destroy();
      this.instBuf = d.createBuffer({ size: this.instCap, usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST });
      this.group0 = d.createBindGroup({ layout: this.layout0, entries: [
        { binding: 0, resource: { buffer: this.camBuf } }, { binding: 1, resource: { buffer: this.instBuf } }] });
    }
    if (w) d.queue.writeBuffer(this.instBuf, 0, out, 0, w);

    const enc = d.createCommandEncoder();
    const pass = enc.beginRenderPass({
      colorAttachments: [{ view: this.ctx.getCurrentTexture().createView(), loadOp: "clear", storeOp: "store", clearValue: { r: 0, g: 0, b: 0, a: 1 } }],
      depthStencilAttachment: { view: this.depthView, depthLoadOp: "clear", depthClearValue: 1, depthStoreOp: "discard" },
    });
    // camera clear color inside the 5:3 frame
    pass.setViewport(this.vp.x, this.vp.y, this.vp.w, this.vp.h, 0, 1);
    pass.setScissorRect(this.vp.x, this.vp.y, this.vp.w, this.vp.h);
    pass.setBindGroup(0, this.group0);
    this.clearRect(pass, r, g, b);

    let pipe = null, group = null, geo = null;
    for (let q = 0; q < nops; q++) {
      const o = ops[q];
      let p, gr, ge;
      if (o.type === 0) {
        const m = this.materials[o.b];
        p = this.pipelines[m.kind]; gr = m.group; ge = this.meshes[o.a];
      } else {
        p = this.pipelines.text; gr = o.a.group; ge = this.quad;
      }
      if (p !== pipe) { pass.setPipeline(p); pipe = p; }
      if (gr !== group) { pass.setBindGroup(1, gr); group = gr; }
      if (ge !== geo) { pass.setVertexBuffer(0, ge.vb); pass.setIndexBuffer(ge.ib, "uint32"); geo = ge; }
      pass.drawIndexed(ge.count, o.count, 0, 0, o.first);
    }
    const calls = nops;
    pass.end();
    d.queue.submit([enc.finish()]);
    sq.instances = n;
    sq.drawCalls = calls;
  }

  clearRect(pass, r, g, b) {
    // a full-frame quad behind everything in the camera's background color
    if (!this.bgPipe) {
      const m = this.device.createShaderModule({ code: `
        @group(0) @binding(0) var<uniform> c: vec4f;
        @vertex fn vs(@builtin(vertex_index) i: u32) -> @builtin(position) vec4f {
          let p = array(vec2f(-1, -1), vec2f(3, -1), vec2f(-1, 3));
          return vec4f(p[i], 1.0, 1.0);
        }
        @fragment fn fs() -> @location(0) vec4f { return c; }` });
      this.bgBuf = this.device.createBuffer({ size: 16, usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST });
      this.bgPipe = this.device.createRenderPipeline({ layout: "auto",
        vertex: { module: m, entryPoint: "vs" },
        fragment: { module: m, entryPoint: "fs", targets: [{ format: this.format }] },
        depthStencil: { format: "depth24plus", depthWriteEnabled: false, depthCompare: "always" } });
      this.bgGroup = this.device.createBindGroup({ layout: this.bgPipe.getBindGroupLayout(0),
        entries: [{ binding: 0, resource: { buffer: this.bgBuf } }] });
    }
    this.device.queue.writeBuffer(this.bgBuf, 0, new Float32Array([r, g, b, 1]));
    pass.setPipeline(this.bgPipe);
    pass.setBindGroup(0, this.bgGroup);
    pass.draw(3);
    pass.setBindGroup(0, this.group0);
  }

  // canvas client coords -> 800x480 virtual screen, origin bottom-left
  toVirtual(clientX, clientY) {
    const rect = this.canvas.getBoundingClientRect();
    const dpr = this.vp.dpr;
    const px = (clientX - rect.left) * dpr - this.vp.x;
    const py = (clientY - rect.top) * dpr - this.vp.y;
    return [(px / this.vp.w) * VIRTUAL_W, (1 - py / this.vp.h) * VIRTUAL_H];
  }
}

// ---------------------------------------------------------------- input ---
const KEYS = { ArrowLeft: 0, ArrowRight: 1, ArrowUp: 2, Space: 2, KeyW: 2, KeyP: 3, KeyR: 4, KeyE: 5,
  KeyS: 6, KeyM: 7, KeyC: 8, KeyO: 9, KeyT: 10, KeyN: 11 };

function bindInput(renderer, audio) {
  const unlock = () => audio && audio.unlock();
  addEventListener("keydown", (e) => {
    unlock();
    const k = KEYS[e.code];
    if (k === undefined || e.metaKey || e.ctrlKey) return;
    e.preventDefault();
    if (!e.repeat) engine.key(k, true);
  });
  addEventListener("keyup", (e) => { const k = KEYS[e.code]; if (k !== undefined) engine.key(k, false); });
  addEventListener("blur", () => { for (const k of new Set(Object.values(KEYS))) engine.key(k, false); });
  const ptr = (kind) => (e) => {
    unlock();
    if (kind === 0) canvas.setPointerCapture?.(e.pointerId);
    const [x, y] = renderer.toVirtual(e.clientX, e.clientY);
    engine.pointer(e.pointerId, x, y, kind);
    e.preventDefault();
  };
  canvas.addEventListener("pointerdown", ptr(0));
  canvas.addEventListener("pointermove", (e) => { if (e.buttons || e.pointerType === "touch") ptr(1)(e); });
  canvas.addEventListener("pointerup", ptr(2));
  canvas.addEventListener("pointercancel", ptr(2));
  canvas.addEventListener("contextmenu", (e) => e.preventDefault());
}

// ----------------------------------------------------------------- main ---
async function main() {
  const [wasm, menu, game, assetsText] = await Promise.all([
    init(), fetchJSON("assets/scene_menu.json"), fetchJSON("assets/scene_game.json"), fetchJSON("assets/assets.json"),
  ]);
  const assets = JSON.parse(assetsText);
  const seed = BigInt(params.get("seed") || Math.floor(Math.random() * 2 ** 31));
  const start = params.get("start") === "game" ? "game" : "menu";
  let prefs = loadPrefs();
  if (start === "game") {
    // skipping the menu also skips its fake loading screen
    try { prefs = JSON.stringify({ ...JSON.parse(prefs), loading: 0 }); } catch { prefs = '{"loading":0}'; }
  }
  engine = new Engine(menu, game, assetsText, prefs, seed, start);
  engine.set_aspect(ASPECT);
  sq.engine = engine;
  const tables = JSON.parse(engine.tables());
  const renderRecords = (alpha) => {
    const len = engine.render(alpha);
    return new Float32Array(wasm.memory.buffer, engine.records_ptr(), len);
  };
  let textsVersion = -1, texts = {};
  const currentTexts = () => {
    const v = engine.texts_version();
    if (v !== textsVersion) { texts = JSON.parse(engine.texts()); textsVersion = v; }
    return texts;
  };

  const fontFamilies = [];
  await Promise.all(tables.fonts.map(async (key, i) => {
    const f = assets.fonts[key];
    const face = new FontFace(f.name, `url(assets/${f.file})`);
    try { await face.load(); document.fonts.add(face); fontFamilies[i] = f.name; } catch (e) { sq.errors.push(`font ${f.name}: ${e}`); }
  }));

  const renderer = await Renderer.create(canvas);
  await renderer.loadAssets(assets, tables);
  sq.gpu = renderer.adapterInfo;

  let audio = null;
  if (!params.has("mute")) {
    try { audio = new Audio(assets); audio.load(); } catch (e) { sq.errors.push(`audio: ${e}`); }
  }
  bindInput(renderer, audio);

  overlay.hidden = true;
  const debug = params.has("debug");
  debugEl.hidden = !debug;
  if (params.has("autoplay")) engine.key(2, true);

  // Render one frame, read the pixels back and check every opaque mesh instance
  // actually covered its own center pixel (catches culling/depth/batching bugs).
  sq.audit = async () => {
    const records = renderRecords(1).slice();
    const cam = engine.camera(1);
    renderer.draw(records, currentTexts(), cam, fontFamilies);
    const bmp = await createImageBitmap(canvas);
    const c2 = new OffscreenCanvas(bmp.width, bmp.height);
    const g = c2.getContext("2d");
    g.drawImage(bmp, 0, 0);
    const vp = renderer.vp, [, , , half] = cam, v = cam.slice(9, 25);
    const missing = [];
    let checked = 0;
    for (let i = 0; i < records.length / RECORD; i++) {
      const o = i * RECORD;
      if (records[o] !== 0 || renderer.materials[records[o + 2]].kind !== "opaque") continue;
      if (renderer.materials[records[o + 2]].name !== "blockColored") continue;
      const wx = records[o + 16], wy = records[o + 17], wz = records[o + 18];
      const vx = v[0] * wx + v[4] * wy + v[8] * wz + v[12], vy = v[1] * wx + v[5] * wy + v[9] * wz + v[13];
      const sx = vp.x + (vx / (half * ASPECT) * 0.5 + 0.5) * vp.w, sy = vp.y + (0.5 - vy / half * 0.5) * vp.h;
      if (sx < vp.x + 4 || sx > vp.x + vp.w - 4 || sy < vp.y + 4 || sy > vp.y + vp.h - 4) continue;
      checked++;
      const [r, gg, b] = g.getImageData(Math.round(sx), Math.round(sy), 1, 1).data;
      if (Math.max(r, gg, b) < 70) missing.push({ x: +wx.toFixed(1), y: +wy.toFixed(1), z: +wz.toFixed(1), px: [r, gg, b] });
    }
    return { checked, missing };
  };

  let last = performance.now(), acc = 0, frames = 0, fpsT = last, lastClip = null, lastMusic = "", music = null;
  // frame timing observables: work = JS+WASM time per frame, gap = rAF interval
  const work = [], gaps = [];
  sq.frameStats = () => {
    const pct = (a, p) => { const s = [...a].sort((x, y) => x - y); return +(s[Math.min(s.length - 1, Math.floor(s.length * p))] || 0).toFixed(2); };
    return { n: work.length, workP50: pct(work, 0.5), workP99: pct(work, 0.99), workMax: pct(work, 1), gapP50: pct(gaps, 0.5), gapP99: pct(gaps, 0.99), over9ms: gaps.filter((g) => g > 9).length };
  };
  sq.resetStats = () => { work.length = 0; gaps.length = 0; };
  sq.loadMs = Math.round(performance.now());
  const frame = (now) => {
    gaps.push(now - last);
    if (gaps.length > 2000) { gaps.shift(); work.shift(); }
    const t0 = performance.now();
    acc += Math.min(0.25, (now - last) / 1000);
    last = now;
    let steps = 0;
    while (acc >= TICK && steps < 8) {
      engine.frame();
      acc -= TICK;
      steps++;
      sq.ticks++;
      while (pendingUp.length) { const [id, x, y] = pendingUp.shift(); engine.pointer(id, x, y, 2); }
    }
    if (audio) {
      const shots = engine.drain_sounds();
      if (shots !== "[]") for (const s of JSON.parse(shots)) audio.shot(s);
      const ms = engine.music();
      if (ms !== lastMusic) { lastMusic = ms; music = JSON.parse(ms); }
      const m = music;
      if (m.clip !== lastClip) { audio.reset(); lastClip = m.clip; }
      audio.sync(m);
    }
    const p = engine.take_prefs();
    if (p) savePrefs(p);
    const alpha = acc / TICK;
    renderer.draw(renderRecords(alpha), currentTexts(), engine.camera(alpha), fontFamilies);
    frames++;
    if (now - fpsT >= 500) { sq.fps = Math.round((frames * 1000) / (now - fpsT)); frames = 0; fpsT = now; }
    if (debug) {
      const s = sq.state();
      debugEl.textContent = `fps ${sq.fps}  inst ${sq.instances}  draws ${sq.drawCalls}  ${sq.gpu}\n` +
        Object.entries(s).filter(([k]) => k !== "rows").map(([k, v]) => `${k}: ${JSON.stringify(v)}`).join("\n");
    }
    work.push(performance.now() - t0);
    requestAnimationFrame(frame);
  };
  requestAnimationFrame(frame);
  sq.ready = true;
}

main().catch((e) => {
  console.error(e);
  sq.errors.push(String(e));
  statusEl.textContent = String(e.message || e);
});

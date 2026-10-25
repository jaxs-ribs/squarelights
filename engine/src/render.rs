//! Flattens a scene into GPU instance records for the host renderer.
//!
//! Record layout (28 f32):
//!   [kind, mesh_or_node, material, depth, m00..m33, uv_scale.xy, uv_offset.xy, color.rgba]
//! kind 0 = mesh instance, 1 = TextMesh (mesh_or_node is the node index).
//! Opaque instances come first (any order), then transparent ones back to front.

use crate::math::{v3, Mat4, V3};
use crate::world::World;

pub const RECORD: usize = 28;

pub struct View {
    pub center: V3,
    pub half_w: f32,
    pub half_h: f32,
    /// camera forward axis; depth = distance along it
    pub forward: V3,
}

/// Previous-frame positions so the host can interpolate between 50 Hz ticks.
pub struct Interp {
    pub prev: Vec<V3>,
    pub cur: Vec<V3>,
}

impl Interp {
    pub fn new() -> Interp {
        Interp { prev: Vec::new(), cur: Vec::new() }
    }

    pub fn capture(&mut self, w: &World) {
        std::mem::swap(&mut self.prev, &mut self.cur);
        self.cur.clear();
        self.cur.extend((0..w.nodes.len()).map(|i| w.position(i)));
        if self.prev.len() != self.cur.len() {
            self.prev = self.cur.clone();
        }
    }

    /// Offset to add to node i's current world position for blend factor `alpha`.
    pub fn offset(&self, i: usize, alpha: f32) -> V3 {
        if i >= self.cur.len() {
            return V3::ZERO;
        }
        let d = self.cur[i] - self.prev[i];
        // Teleports (wrap-around, respawns) must not smear across the screen.
        if d.length() > 40.0 {
            return V3::ZERO;
        }
        d * (alpha - 1.0)
    }
}

pub fn build(w: &World, view: &View, interp: &Interp, alpha: f32, out: &mut Vec<f32>) {
    out.clear();
    let mut transparent: Vec<(f32, [f32; RECORD])> = Vec::new();
    // Interpolation offsets compose down the hierarchy: offsetting a child by
    // its own world delta already includes the parent's motion.
    for i in 0..w.nodes.len() {
        let n = &w.nodes[i];
        if !n.visible {
            continue;
        }
        let is_text = n.text.as_ref().map(|t| !t.text.is_empty()).unwrap_or(false);
        let (Some(mesh), Some(mat)) = (n.mesh, n.material) else {
            if !is_text {
                continue;
            }
            let m = translate(w.matrix(i), interp.offset(i, alpha));
            let p = m.point(V3::ZERO);
            if !near_view(view, p, v3(400.0, 200.0, 0.0)) {
                continue;
            }
            let mat = n.material.unwrap_or(0) as f32;
            let depth = dot(view.forward, p - view.center);
            let rec = record(1.0, i as f32, mat, depth, &m, [1.0, 1.0, 0.0, 0.0], n.color);
            transparent.push((depth, rec));
            continue;
        };
        let m = translate(w.matrix(i), interp.offset(i, alpha));
        let center = m.point(V3::ZERO);
        let ext = m.abs_vector(w.mesh_extent[mesh as usize]);
        if !near_view(view, center, ext) {
            continue;
        }
        let info = &w.materials[mat as usize];
        let off = n.uv_offset.unwrap_or(info.offset);
        let uv = [info.scale[0], info.scale[1], off[0], off[1]];
        let depth = dot(view.forward, center - view.center);
        let rec = record(0.0, mesh as f32, mat as f32, depth, &m, uv, n.color);
        if info.transparent {
            transparent.push((depth, rec));
        } else {
            out.extend_from_slice(&rec);
        }
    }
    // Unity sorts transparent renderers far-to-near by view distance.
    transparent.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    for (_, r) in transparent {
        out.extend_from_slice(&r);
    }
}

fn translate(mut m: Mat4, d: V3) -> Mat4 {
    m.0[12] += d.x;
    m.0[13] += d.y;
    m.0[14] += d.z;
    m
}

fn near_view(v: &View, c: V3, ext: V3) -> bool {
    (c.x - v.center.x).abs() <= v.half_w + ext.x + 2.0 && (c.y - v.center.y).abs() <= v.half_h + ext.y + 2.0
}

fn record(kind: f32, id: f32, mat: f32, depth: f32, m: &Mat4, uv: [f32; 4], color: [f32; 4]) -> [f32; RECORD] {
    let mut r = [0.0f32; RECORD];
    r[0] = kind;
    r[1] = id;
    r[2] = mat;
    r[3] = depth;
    r[4..20].copy_from_slice(&m.0);
    r[20..24].copy_from_slice(&uv);
    r[24..28].copy_from_slice(&color);
    r
}

fn dot(a: V3, b: V3) -> f32 {
    a.x * b.x + a.y * b.y + a.z * b.z
}

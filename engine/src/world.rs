//! A tiny Unity-like scene graph: named nodes with local TRS, parenting, meshes,
//! materials, text meshes and box colliders, loaded from the extracted scene JSON.

use std::collections::HashMap;

use serde::Deserialize;

use crate::math::{v3, Mat4, Quat, V3};

#[derive(Deserialize)]
struct NodeJson {
    name: String,
    parent: i32,
    pos: [f32; 3],
    rot: [f32; 4],
    scale: [f32; 3],
    #[serde(default)]
    scripts: Vec<String>,
    mesh: Option<String>,
    #[serde(default)]
    materials: Vec<Option<String>>,
    #[serde(default = "yes")]
    visible: bool,
    text: Option<TextJson>,
    #[serde(rename = "box")]
    collider: Option<BoxJson>,
    #[serde(rename = "guiTexture")]
    gui: Option<GuiJson>,
    camera: Option<CameraJson>,
}

#[derive(Deserialize)]
struct CameraJson {
    bg: [f32; 4],
    near: f32,
    far: f32,
    #[serde(rename = "orthoSize")]
    ortho_size: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct Camera {
    pub node: usize,
    pub bg: [f32; 4],
    pub near: f32,
    pub far: f32,
    pub ortho_size: f32,
}

#[derive(Deserialize)]
struct GuiJson {
    tex: Option<String>,
    color: [f32; 4],
    inset: [f32; 4],
}

/// A GUITexture: screen-space rect anchored at the node's (viewport) position.
#[derive(Clone, Debug)]
pub struct Gui {
    pub tex: Option<String>,
    pub color: [f32; 4],
    pub inset: [f32; 4],
}

fn yes() -> bool {
    true
}

#[derive(Deserialize)]
struct TextJson {
    text: String,
    #[serde(rename = "charSize")]
    char_size: f32,
    anchor: i32,
    align: i32,
    #[serde(rename = "fontSize")]
    font_size: i32,
    style: i32,
    font: Option<String>,
}

#[derive(Deserialize)]
struct BoxJson {
    size: [f32; 3],
    center: [f32; 3],
}

#[derive(Deserialize)]
struct MaterialJson {
    name: String,
    shader: Option<String>,
    scale: [f32; 2],
    offset: [f32; 2],
    color: Option<[f32; 4]>,
}

#[derive(Deserialize)]
struct MeshJson {
    pos: Vec<[f32; 3]>,
}

#[derive(Deserialize)]
struct AssetsJson {
    meshes: HashMap<String, MeshJson>,
    materials: HashMap<String, MaterialJson>,
    fonts: HashMap<String, serde_json::Value>,
}

#[derive(Clone, Debug)]
pub struct Text {
    pub text: String,
    pub font: u32,
    pub font_size: f32,
    pub char_size: f32,
    pub anchor: i32,
    pub align: i32,
    pub style: i32,
}

#[derive(Clone, Debug)]
pub struct Node {
    pub name: String,
    pub parent: Option<usize>,
    pub children: Vec<usize>,
    pub lpos: V3,
    pub lrot: Quat,
    pub lscale: V3,
    pub scripts: Vec<String>,
    pub mesh: Option<u32>,
    pub material: Option<u32>,
    pub visible: bool,
    /// Per-renderer `material.mainTextureOffset` override.
    pub uv_offset: Option<[f32; 2]>,
    /// `renderer.material.color` (only text shaders actually use it).
    pub color: [f32; 4],
    pub text: Option<Text>,
    pub collider: Option<(V3, V3)>,
    pub gui: Option<Gui>,
}

pub struct MaterialInfo {
    /// Unity render queue: opaque (2000) draws before transparent (3000).
    pub transparent: bool,
    pub scale: [f32; 2],
    pub offset: [f32; 2],
    pub color: [f32; 4],
}

pub struct World {
    pub nodes: Vec<Node>,
    by_name: HashMap<String, Vec<usize>>,
    pub mesh_keys: Vec<String>,
    pub mesh_extent: Vec<V3>,
    pub material_keys: Vec<String>,
    pub materials: Vec<MaterialInfo>,
    pub font_keys: Vec<String>,
    pub camera: Camera,
    /// Bumped whenever any TextMesh string changes (host re-reads texts()).
    pub text_version: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Space {
    World,
    Self_,
}

impl World {
    pub fn load(scene_json: &str, assets_json: &str) -> World {
        let assets: AssetsJson = serde_json::from_str(assets_json).expect("assets json");
        let mut mesh_keys: Vec<String> = assets.meshes.keys().cloned().collect();
        mesh_keys.sort();
        let mesh_extent = mesh_keys
            .iter()
            .map(|k| {
                let p = &assets.meshes[k].pos;
                let mut e = V3::ZERO;
                for q in p {
                    e.x = e.x.max(q[0].abs());
                    e.y = e.y.max(q[1].abs());
                    e.z = e.z.max(q[2].abs());
                }
                e
            })
            .collect();
        let mut material_keys: Vec<String> = assets.materials.keys().cloned().collect();
        material_keys.sort();
        let materials = material_keys
            .iter()
            .map(|k| {
                let m = &assets.materials[k];
                MaterialInfo {
                    transparent: m.shader.as_deref().unwrap_or("").contains("Particle") || m.name.contains("Font"),
                    scale: m.scale,
                    offset: m.offset,
                    color: m.color.unwrap_or([1.0; 4]),
                }
            })
            .collect();
        let mut font_keys: Vec<String> = assets.fonts.keys().cloned().collect();
        font_keys.sort();

        let raw: Vec<NodeJson> = serde_json::from_str(scene_json).expect("scene json");
        let mut nodes: Vec<Node> = raw
            .iter()
            .map(|n| {
                let material = n
                    .materials
                    .first()
                    .and_then(|m| m.as_ref())
                    .and_then(|k| material_keys.iter().position(|x| x == k))
                    .map(|i| i as u32);
                let color = material.map(|m| materials_color(&assets, &material_keys, m)).unwrap_or([1.0; 4]);
                Node {
                    name: n.name.clone(),
                    parent: if n.parent < 0 { None } else { Some(n.parent as usize) },
                    children: Vec::new(),
                    lpos: V3::from_slice(&n.pos),
                    lrot: Quat::from_slice(&n.rot),
                    lscale: V3::from_slice(&n.scale),
                    scripts: n.scripts.clone(),
                    mesh: n
                        .mesh
                        .as_ref()
                        .and_then(|k| mesh_keys.iter().position(|x| x == k))
                        .map(|i| i as u32),
                    material,
                    visible: n.visible,
                    uv_offset: None,
                    color,
                    text: n.text.as_ref().map(|t| Text {
                        text: t.text.clone(),
                        font: t
                            .font
                            .as_ref()
                            .and_then(|k| font_keys.iter().position(|x| x == k))
                            .unwrap_or(0) as u32,
                        font_size: t.font_size as f32,
                        char_size: t.char_size,
                        anchor: t.anchor,
                        align: t.align,
                        style: t.style,
                    }),
                    collider: n.collider.as_ref().map(|b| (V3::from_slice(&b.size), V3::from_slice(&b.center))),
                    gui: n.gui.as_ref().map(|g| Gui { tex: g.tex.clone(), color: g.color, inset: g.inset }),
                }
            })
            .collect();
        for i in 0..nodes.len() {
            if let Some(p) = nodes[i].parent {
                nodes[p].children.push(i);
            }
        }
        let mut by_name: HashMap<String, Vec<usize>> = HashMap::new();
        for (i, n) in nodes.iter().enumerate() {
            by_name.entry(n.name.clone()).or_default().push(i);
        }
        let camera = raw
            .iter()
            .enumerate()
            .find_map(|(i, n)| {
                n.camera.as_ref().map(|c| Camera { node: i, bg: c.bg, near: c.near, far: c.far, ortho_size: c.ortho_size })
            })
            .expect("scene has a camera");
        World { nodes, by_name, mesh_keys, mesh_extent, material_keys, materials, font_keys, camera, text_version: 0 }
    }

    /// GameObject.Find: first object with that name.
    pub fn find(&self, name: &str) -> usize {
        match self.by_name.get(name) {
            Some(v) => v[0],
            None => panic!("GameObject.Find(\"{name}\") found nothing"),
        }
    }

    pub fn try_find(&self, name: &str) -> Option<usize> {
        self.by_name.get(name).map(|v| v[0])
    }

    pub fn with_script(&self, script: &str) -> Vec<usize> {
        (0..self.nodes.len()).filter(|&i| self.nodes[i].scripts.iter().any(|s| s == script)).collect()
    }

    /// Depth-first descendants (BroadcastMessage order).
    pub fn descendants(&self, i: usize, out: &mut Vec<usize>) {
        out.push(i);
        for &c in &self.nodes[i].children {
            self.descendants(c, out);
        }
    }

    pub fn local_matrix(&self, i: usize) -> Mat4 {
        let n = &self.nodes[i];
        Mat4::trs(n.lpos, n.lrot, n.lscale)
    }

    pub fn matrix(&self, i: usize) -> Mat4 {
        let local = self.local_matrix(i);
        match self.nodes[i].parent {
            Some(p) => self.matrix(p).mul(&local),
            None => local,
        }
    }

    pub fn position(&self, i: usize) -> V3 {
        match self.nodes[i].parent {
            Some(p) => self.matrix(p).point(self.nodes[i].lpos),
            None => self.nodes[i].lpos,
        }
    }

    pub fn rotation(&self, i: usize) -> Quat {
        match self.nodes[i].parent {
            Some(p) => self.rotation(p) * self.nodes[i].lrot,
            None => self.nodes[i].lrot,
        }
    }

    pub fn set_position(&mut self, i: usize, p: V3) {
        let local = match self.nodes[i].parent {
            Some(par) => self.matrix(par).inverse_affine().point(p),
            None => p,
        };
        self.nodes[i].lpos = local;
    }

    pub fn set_rotation(&mut self, i: usize, q: Quat) {
        let local = match self.nodes[i].parent {
            Some(par) => self.rotation(par).inverse() * q,
            None => q,
        };
        self.nodes[i].lrot = local.normalized();
    }

    pub fn translate(&mut self, i: usize, x: f32, y: f32, z: f32, space: Space) {
        let d = match space {
            Space::World => v3(x, y, z),
            Space::Self_ => self.rotation(i).rotate(v3(x, y, z)),
        };
        let p = self.position(i) + d;
        self.set_position(i, p);
    }

    pub fn rotate(&mut self, i: usize, x: f32, y: f32, z: f32, space: Space) {
        let e = Quat::euler(x, y, z);
        match space {
            Space::Self_ => {
                let n = &mut self.nodes[i];
                n.lrot = (n.lrot * e).normalized();
            }
            Space::World => {
                let r = self.rotation(i);
                self.set_rotation(i, e * r);
            }
        }
    }

    pub fn scale(&self, i: usize) -> V3 {
        self.nodes[i].lscale
    }

    pub fn set_scale(&mut self, i: usize, s: V3) {
        self.nodes[i].lscale = s;
    }

    pub fn add_scale(&mut self, i: usize, d: V3) {
        self.nodes[i].lscale += d;
    }

    pub fn set_text(&mut self, i: usize, s: &str) {
        if let Some(t) = self.nodes[i].text.as_mut() {
            if t.text != s {
                t.text = s.to_string();
                self.text_version += 1;
            }
        }
    }

    pub fn text(&self, i: usize) -> &str {
        self.nodes[i].text.as_ref().map(|t| t.text.as_str()).unwrap_or("")
    }

    /// GUITexture screen rect (x, y, w, h) in the 800x480 virtual screen, origin bottom-left.
    pub fn gui_rect(&self, i: usize) -> [f32; 4] {
        let g = self.nodes[i].gui.as_ref().expect("node has no GUITexture");
        let p = self.position(i);
        [p.x * crate::input::SCREEN_W + g.inset[0], p.y * crate::input::SCREEN_H + g.inset[1], g.inset[2], g.inset[3]]
    }

    /// GUITexture.HitTest.
    pub fn gui_hit(&self, i: usize, x: f32, y: f32) -> bool {
        let r = self.gui_rect(i);
        x >= r[0] && x < r[0] + r[2] && y >= r[1] && y < r[1] + r[3]
    }

    /// World-space AABB of a node's box collider (center, half extents).
    pub fn collider_aabb(&self, i: usize) -> Option<(V3, V3)> {
        let (size, center) = self.nodes[i].collider?;
        let m = self.matrix(i);
        Some((m.point(center), m.abs_vector(size * 0.5)))
    }
}

fn materials_color(assets: &AssetsJson, keys: &[String], m: u32) -> [f32; 4] {
    assets.materials[&keys[m as usize]].color.unwrap_or([1.0; 4])
}

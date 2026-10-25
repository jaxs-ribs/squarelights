//! Square Lights, ported from Unity 4.3 to Rust/WASM.
//!
//! The host (web/main.js) feeds input events, calls `frame()` at 50 Hz,
//! and draws the instance records from `render()` with WebGPU.

pub mod game;
pub mod input;
pub mod math;
pub mod menu;
pub mod render;
pub mod runtime;
pub mod world;

use wasm_bindgen::prelude::*;

use game::GameScene;
use input::InputQueue;
use menu::MenuScene;
use render::Interp;
use runtime::{Ctx, Music, MusicState, Prefs, Rng, SceneId};
use world::World;

pub enum Scene {
    Menu(MenuScene),
    Game(GameScene),
}

pub fn build_scene(menu_json: &str, game_json: &str, assets_json: &str, rng: Rng, id: SceneId, prefs: Prefs) -> Scene {
    match id {
        SceneId::Game => {
            let w = World::load(game_json, assets_json);
            let music = Music { clip: "InGame_Theme", volume: 0.69, state: MusicState::Stopped, mute: false };
            Scene::Game(GameScene::new(Ctx::new(w, rng, prefs, music)))
        }
        SceneId::Menu => {
            let w = World::load(menu_json, assets_json);
            let music = Music { clip: "Menu_theme", volume: 1.0, state: MusicState::Stopped, mute: false };
            Scene::Menu(MenuScene::new(Ctx::new(w, rng, prefs, music)))
        }
    }
}

pub fn scene_world(scene: &Scene) -> &World {
    match scene {
        Scene::Game(g) => &g.ctx.w,
        Scene::Menu(m) => &m.ctx.w,
    }
}

#[wasm_bindgen]
pub struct Engine {
    game_json: String,
    menu_json: String,
    assets_json: String,
    seed: u64,
    loads: u64,
    pub(crate) scene: Scene,
    input: InputQueue,
    interp: Interp,
    records: Vec<f32>,
    sounds: Vec<&'static str>,
    aspect: f32,
}

#[wasm_bindgen]
impl Engine {
    /// `prefs_json` is the saved PlayerPrefs object ({} on first run).
    #[wasm_bindgen(constructor)]
    pub fn new(menu_json: String, game_json: String, assets_json: String, prefs_json: &str, seed: u64, start: &str) -> Engine {
        let mut prefs = Prefs::default();
        if let Ok(map) = serde_json::from_str(prefs_json) {
            prefs.map = map;
        }
        let first = if start == "game" { SceneId::Game } else { SceneId::Menu };
        if first == SceneId::Game && !prefs.has_key("loading") {
            prefs.set_int("loading", 0);
        }
        let scene = build_scene(&menu_json, &game_json, &assets_json, Rng::new(seed), first, prefs);
        let mut e = Engine {
            game_json,
            menu_json,
            assets_json,
            seed,
            loads: 0,
            scene,
            input: InputQueue::default(),
            interp: Interp::new(),
            records: Vec::new(),
            sounds: Vec::new(),
            aspect: 800.0 / 480.0,
        };
        e.interp.capture(scene_world(&e.scene));
        e
    }

    fn load(&mut self, id: SceneId, prefs: Prefs) {
        self.loads += 1;
        let rng = Rng::new(self.seed ^ self.loads.wrapping_mul(0x5851_F42D_4C95_7F2D));
        self.scene = build_scene(&self.menu_json, &self.game_json, &self.assets_json, rng, id, prefs);
        self.interp = Interp::new();
        self.interp.capture(scene_world(&self.scene));
    }

    fn world(&self) -> &World {
        scene_world(&self.scene)
    }

    /// Host input: key codes are input::Key values.
    pub fn key(&mut self, code: usize, down: bool) {
        self.input.key(code, down);
    }

    /// Pointer in 800x480 virtual-screen coordinates, origin bottom-left. kind: 0 down, 1 move, 2 up.
    pub fn pointer(&mut self, id: i32, x: f32, y: f32, kind: u8) {
        self.input.pointer(id, x, y, kind);
    }

    /// Advance one 50 Hz tick.
    pub fn frame(&mut self) {
        let input = self.input.frame();
        let request = match &mut self.scene {
            Scene::Game(g) => {
                g.ctx.input = input;
                g.frame();
                self.sounds.append(&mut g.ctx.sounds);
                g.ctx.load_request.take().map(|r| (r, g.ctx.prefs.clone()))
            }
            Scene::Menu(m) => {
                m.ctx.input = input;
                m.frame();
                self.sounds.append(&mut m.ctx.sounds);
                m.ctx.load_request.take().map(|r| (r, m.ctx.prefs.clone()))
            }
        };
        if let Some((id, prefs)) = request {
            self.load(id, prefs);
        } else {
            self.interp.capture(scene_world(&self.scene));
        }
    }

    pub fn set_aspect(&mut self, aspect: f32) {
        self.aspect = aspect;
    }

    /// Camera as [x, y, z, ortho_half_height, near, far, bg_r, bg_g, bg_b, view matrix (16, column-major)].
    pub fn camera(&self, alpha: f32) -> Vec<f32> {
        let w = self.world();
        let c = w.camera;
        let p = w.position(c.node) + self.interp.offset(c.node, alpha);
        let view = math::Mat4::trs(p, w.rotation(c.node), math::V3::ONE).inverse_affine();
        let mut v = vec![p.x, p.y, p.z, c.ortho_size, c.near, c.far, c.bg[0], c.bg[1], c.bg[2]];
        v.extend_from_slice(&view.0);
        v
    }

    /// Build instance records (see render.rs) into an internal buffer and
    /// return their f32 count; read them in place via `records_ptr()`.
    pub fn render(&mut self, alpha: f32) -> usize {
        let cam = self.camera(alpha);
        let w = self.world();
        let forward = w.rotation(w.camera.node).rotate(math::v3(0.0, 0.0, 1.0));
        let view = render::View { center: math::v3(cam[0], cam[1], cam[2]), half_w: cam[3] * self.aspect, half_h: cam[3], forward };
        let mut out = std::mem::take(&mut self.records);
        render::build(scene_world(&self.scene), &view, &self.interp, alpha, &mut out);
        let n = out.len();
        self.records = out;
        n
    }

    /// Pointer to the records written by `render()` (valid until the next render/frame).
    pub fn records_ptr(&self) -> *const f32 {
        self.records.as_ptr()
    }

    /// Changes whenever a TextMesh string changes or a scene loads.
    pub fn texts_version(&self) -> u32 {
        self.world().text_version.wrapping_add((self.loads as u32).wrapping_mul(1_000_003))
    }

    /// Text meshes referenced by kind-1 records: JSON {node: [text, font, fontSize, charSize, anchor, align, style]}.
    pub fn texts(&self) -> String {
        let w = self.world();
        let mut m = serde_json::Map::new();
        for (i, n) in w.nodes.iter().enumerate() {
            if let Some(t) = &n.text {
                if !t.text.is_empty() {
                    m.insert(
                        i.to_string(),
                        serde_json::json!([t.text, t.font, t.font_size, t.char_size, t.anchor, t.align, t.style]),
                    );
                }
            }
        }
        serde_json::Value::Object(m).to_string()
    }

    /// Asset index tables the records refer to: {meshes:[key], materials:[key], fonts:[key]}.
    pub fn tables(&self) -> String {
        let w = self.world();
        let transparent: Vec<bool> = w.materials.iter().map(|m| m.transparent).collect();
        serde_json::json!({"meshes": w.mesh_keys, "materials": w.material_keys, "transparent": transparent, "fonts": w.font_keys})
            .to_string()
    }

    /// One-shot sounds requested since the last call (clip names).
    pub fn drain_sounds(&mut self) -> String {
        let s = serde_json::to_string(&self.sounds).unwrap();
        self.sounds.clear();
        s
    }

    /// {clip, volume, state: "playing"|"paused"|"stopped", mute}
    pub fn music(&self) -> String {
        let m = match &self.scene {
            Scene::Game(g) => &g.ctx.music,
            Scene::Menu(s) => &s.ctx.music,
        };
        let st = match m.state {
            MusicState::Playing => "playing",
            MusicState::Paused => "paused",
            MusicState::Stopped => "stopped",
        };
        serde_json::json!({"clip": m.clip, "volume": m.volume, "state": st, "mute": m.mute}).to_string()
    }

    /// PlayerPrefs to persist, or "" if unchanged since the last call.
    pub fn take_prefs(&mut self) -> String {
        let p = match &mut self.scene {
            Scene::Game(g) => &mut g.ctx.prefs,
            Scene::Menu(m) => &mut m.ctx.prefs,
        };
        if !p.dirty {
            return String::new();
        }
        p.dirty = false;
        serde_json::to_string(&p.map).unwrap()
    }

    /// Observable game state for tests and the debug overlay.
    pub fn state(&self) -> String {
        match &self.scene {
            Scene::Game(g) => g.debug_state().to_string(),
            Scene::Menu(m) => m.debug_state().to_string(),
        }
    }
}

impl Engine {
    /// Native helper for tests: render and copy the records out.
    pub fn render_vec(&mut self, alpha: f32) -> Vec<f32> {
        let n = self.render(alpha);
        self.records[..n].to_vec()
    }
}

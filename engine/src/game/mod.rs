//! The "SquareLights" scene: a line-by-line port of the original MonoBehaviours.
//! Each script's state lives in a struct here; behaviour is split by area:
//! `character` (player, input, camera), `platforms` (generator + falling blocks),
//! `pickups` (coins, hearts, obstacles), `hud` (score, lives, end screen).

mod character;
mod hud;
mod pickups;
mod platforms;

use crate::math::{Quat, V3};
use crate::runtime::{Ctx, FIXED_DT};

pub use character::{CharacterMovement, CharacterTexture};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Call {
    GivePosition,
    PgIsInside,
    PgProcess,
    StopFalling,
    CoinAvailable,
    AvoidObjects,
    PickupReturn(u16),
    BarricadeDeath(u8),
    BarricadeFall(u8),
    BlockadeDeath(u8),
    BlockadeFall(u8),
    TrampAnim(u8, u8),
    RetryReturnAnim,
    RestartGame,
    ExitReturnAnim,
    BackToMenu,
    CoinCounterSlow,
}

pub type GameCtx = Ctx<Call>;

#[derive(Default)]
pub struct InputManager {
    pub node: usize,
    pub jump_ok: i32,
    pub side_to_go: i32,
    pub jump_ok_value: f32,
    pub side_to_go_value: f32,
    pub present_finger: (f32, f32),
    pub data_finger: (f32, f32),
    pub derivative: (f32, f32),
    pub d_time: i32,
    pub hit: i32,
    pub tick: i32,
}

#[derive(Default)]
pub struct Tbl {
    pub node: usize,
    pub hit: i32,
    pub tick: i32,
}

#[derive(Default)]
pub struct CameraRig {
    pub cam: usize,
    pub target: usize,
    pub speed: f32,
    pub min_y: f32,
    pub max_y: f32,
    pub min_y_coin: f32,
    pub max_y_coin: f32,
    pub min_y_obstacle: f32,
    pub max_y_obstacle: f32,
}

#[derive(Default)]
pub struct PlatformGenerator {
    /// Spawn z per chunk ([singles, doubles]); see platforms_start.
    pub spawn_z: [[f32; 10]; 2],
    pub switcher: i32,
    pub singles: [usize; 10],
    pub doubles: [usize; 10],
    pub platform_number: i32,
    pub platform_number2: i32,
    pub a: i32,
    pub b: i32,
    pub e: i32,
    pub spawn_intel: f32,
    pub spawn_intel_divider: i32,
    pub x_min: f32,
    pub x_max: f32,
    pub x_min2: f32,
    pub x_max2: f32,
    pub chunk_type_for_coin: i32,
    pub height: f32,
    pub startloc: i32,
    pub num: i32,
    pub startloc2: i32,
    pub num2: i32,
    pub zoner: i32,
    pub phase: i32,
    pub inside_single: [i32; 10],
    pub inside_double: [i32; 10],
    pub global_inside: i32,
    pub chosen_triangle: i32,
    pub global_triangle_inside: i32,
    pub chosen_obstacle: i32,
    pub global_obstacle_inside: i32,
    pub tramp_inside: [i32; 5],
    pub global_tramp_inside: i32,
    pub super_coin_inside: i32,
    pub heart_inside: i32,
}

#[derive(Default, Clone)]
pub struct BlockController {
    pub node: usize,
    pub initial: bool,
    pub get_started: i32,
    pub out: i32,
    pub difficulty: f32,
    pub rand_time: f32,
    pub timer: i32,
    pub ypos: f32,
    pub evaluated: i32,
    pub initial_pos: V3,
    pub height: f32,
    pub increaser: i32,
    pub max_height_bool: i32,
}

#[derive(Default, Clone)]
pub struct Block {
    pub node: usize,
    pub bumped: i32,
    pub bump_count: i32,
    pub bump_return: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PickupKind {
    Coin,
    SuperCoin,
    Life,
}

#[derive(Clone)]
pub struct Pickup {
    pub kind: PickupKind,
    pub node: usize,
    pub time: f32,
    pub speed: f32,
    pub turnspeed: f32,
    pub dead: i32,
    pub spin: i32,
    pub initial_position: V3,
}

#[derive(Clone, Default)]
pub struct CoinDrop {
    pub node: usize,
    pub immortal: i32,
    pub initial_position: V3,
    pub allow_movement: i32,
    pub timer: i32,
}

#[derive(Clone)]
pub struct Barricade {
    pub node: usize,
    pub dead: i32,
    pub gravity: f32,
    pub time: i32,
    pub out: i32,
    pub initial_position: V3,
    pub initial_rotation: Quat,
    pub parts: [usize; 5],
    pub part_pos: [V3; 5],
    pub part_rot: [Quat; 5],
}

#[derive(Clone)]
pub struct Blockade {
    pub node: usize,
    pub dead: i32,
    pub gravity: i32,
    pub anim: i32,
    pub time: i32,
    pub out: i32,
    pub initial_position: V3,
    pub spikes: [usize; 4],
    pub spike_pos: [V3; 4],
}

#[derive(Clone)]
pub struct Trampoline {
    pub node: usize,
    pub time: i32,
    pub out: i32,
    pub old_movement: f32,
    pub tension: f32,
    pub blocks: [usize; 9],
}

#[derive(Default)]
pub struct GuiButton {
    pub node: usize,
    pub hit: i32,
    pub option: i32,
}

#[derive(Default)]
pub struct ScoreManager {
    pub node: usize,
    pub points: i32,
    pub difficulty: f32,
    pub time: i32,
    pub action: i32,
    pub teleport_timer: i32,
    pub pause_timer: i32,
    pub new_high: bool,
    pub pause_bool: i32,
    pub pause_pos_shift: V3,
    pub offset1: V3,
    pub offset2: V3,
    pub init_pause_gui_pos: V3,
    pub init_bottom_left: V3,
    pub init_bottom_right: V3,
    pub init_top_left: V3,
    pub renderer_hidden: bool,
    pub n: HudNodes,
}

#[derive(Default)]
pub struct HudNodes {
    pub background: usize,
    pub button: usize,
    pub bottom_left: usize,
    pub bottom_right: usize,
    pub top_left: usize,
    pub loading_screen: usize,
    pub pause_symbol: usize,
    pub new_highscore: usize,
    pub highscores: usize,
    pub game_paused: usize,
    pub your_score: usize,
    pub coins_collected: usize,
    pub score: usize,
    pub coin_counter: usize,
    pub triangle_esthetic: usize,
    pub life: [usize; 3],
    pub button_retry: usize,
    pub button_exit: usize,
    pub ranks: [usize; 5],
}

pub struct GameScene {
    pub ctx: GameCtx,
    pub cm: CharacterMovement,
    pub ct: CharacterTexture,
    pub im: InputManager,
    pub top_left: Tbl,
    pub bottom_left: Tbl,
    pub bottom_right: Tbl,
    pub cam: CameraRig,
    pub pg: PlatformGenerator,
    pub controllers: Vec<BlockController>,
    pub blocks: Vec<Block>,
    pub pickups: Vec<Pickup>,
    pub coindrops: Vec<CoinDrop>,
    pub coin_av: [i32; 5],
    pub barricades: Vec<Barricade>,
    pub blockades: Vec<Blockade>,
    pub trampolines: Vec<Trampoline>,
    pub score: ScoreManager,
    pub retry: GuiButton,
    pub exit: GuiButton,
    pub pause: GuiButton,
    pub coins: i32,
    pub life_counts: [(usize, i32); 3],
    pub rotators: Vec<usize>,
    /// Box colliders the player capsule can sweep against.
    pub colliders: Vec<usize>,
    /// Nodes of the 20 coin triangles, PowerTriangle, STLife (spawn targets).
    pub triangles: [usize; 20],
    pub super_coin: usize,
    pub st_life: usize,
    pub barricade_nodes: [usize; 5],
    pub blockade_nodes: [usize; 5],
    pub trampoline_nodes: [usize; 5],
}

impl GameScene {
    pub fn new(ctx: GameCtx) -> GameScene {
        let w = &ctx.w;
        let find = |n: &str| w.find(n);
        let names = |prefix: &str, n: usize| -> Vec<usize> { (1..=n).map(|i| w.find(&format!("{prefix}{i}"))).collect() };
        let triangles: [usize; 20] = names("Triangle", 20).try_into().unwrap();
        let barricade_nodes: [usize; 5] = names("Barricade", 5).try_into().unwrap();
        let blockade_nodes: [usize; 5] = names("Blockade", 5).try_into().unwrap();
        let trampoline_nodes: [usize; 5] = names("Trampoline", 5).try_into().unwrap();
        let cc = find("CharacterCollider");
        let colliders = (0..w.nodes.len()).filter(|&i| i != cc && w.nodes[i].collider.is_some()).collect();
        let rotators = w.with_script("Rotation");
        let mut controllers: Vec<BlockController> = Vec::new();
        for (script, initial) in [("BlockControllerInitial", true), ("BlockController", false)] {
            for node in w.with_script(script) {
                controllers.push(BlockController { node, initial, ..Default::default() });
            }
        }
        let blocks = w.with_script("Block").into_iter().map(|node| Block { node, ..Default::default() }).collect();
        let mut pickups = Vec::new();
        for (kind, script) in [(PickupKind::Coin, "Coin"), (PickupKind::SuperCoin, "SuperCoin"), (PickupKind::Life, "Life")] {
            for node in w.with_script(script) {
                pickups.push(Pickup {
                    kind,
                    node,
                    time: 0.0,
                    speed: 0.0,
                    turnspeed: 0.0,
                    dead: 0,
                    spin: 0,
                    initial_position: V3::ZERO,
                });
            }
        }
        let coindrops = (1..=5).map(|i| CoinDrop { node: w.find(&format!("Coindrop{i}")), ..Default::default() }).collect();
        let life_counts = [(find("1Life"), 1), (find("2Life"), 1), (find("3Life"), 1)];
        let super_coin = find("PowerTriangle");
        let st_life = find("STLife");
        let im_node = find("GUI_Global");
        let tl = find("TopLeft");
        let bl = find("BottomLeft");
        let br = find("BottomRight");
        let retry = find("GUI_Retry");
        let exit = find("GUI_Exit");
        let pause = find("GUI_Pause");
        let score_node = find("Score");

        let mut s = GameScene {
            cm: CharacterMovement::new(cc),
            ct: CharacterTexture { node: find("Character"), ..Default::default() },
            im: InputManager { node: im_node, ..Default::default() },
            top_left: Tbl { node: tl, ..Default::default() },
            bottom_left: Tbl { node: bl, ..Default::default() },
            bottom_right: Tbl { node: br, ..Default::default() },
            cam: CameraRig { cam: find("MainCamera"), target: cc, ..Default::default() },
            pg: PlatformGenerator::default(),
            controllers,
            blocks,
            pickups,
            coindrops,
            coin_av: [0; 5],
            barricades: Vec::new(),
            blockades: Vec::new(),
            trampolines: Vec::new(),
            score: ScoreManager { node: score_node, ..Default::default() },
            retry: GuiButton { node: retry, ..Default::default() },
            exit: GuiButton { node: exit, ..Default::default() },
            pause: GuiButton { node: pause, ..Default::default() },
            coins: 0,
            life_counts,
            rotators,
            colliders,
            triangles,
            super_coin,
            st_life,
            barricade_nodes,
            blockade_nodes,
            trampoline_nodes,
            ctx,
        };
        s.start();
        s
    }

    /// Unity calls every Start() before the first frame. Order matters in a
    /// few places (CoinDrop before the scripts that read its initialPosition).
    fn start(&mut self) {
        // The menu sets "loading" = 1, which makes the original hold a fake
        // 6-second "Loading..." screen (300 ticks). Nothing actually loads, so
        // start the run immediately, exactly like a Retry does.
        if self.ctx.prefs.get_int("loading") == 1 {
            self.ctx.prefs.set_int("loading", 0);
        }
        self.ctx.music.state = crate::runtime::MusicState::Playing;
        for i in 0..self.coindrops.len() {
            self.coindrop_start(i);
        }
        self.character_start();
        self.camera_start();
        self.platforms_start();
        self.pickups_start();
        self.hud_start();
    }

    /// One Unity frame at the fixed 50 Hz step: FixedUpdate, Update, Invokes.
    pub fn frame(&mut self) {
        self.ctx.frame += 1;
        let running = self.ctx.time_scale > 0.0;
        if running {
            self.ctx.time += FIXED_DT as f64;
            self.fixed_update();
        }
        self.update(if running { FIXED_DT } else { 0.0 });
        if running {
            for call in self.ctx.due_invokes() {
                self.dispatch(call);
            }
        }
    }

    fn fixed_update(&mut self) {
        self.character_texture_fixed();
        self.character_fixed();
        self.camera_fixed();
        self.score_fixed();
        self.platforms_fixed();
        self.pickups_fixed();
        self.hud_fixed();
    }

    fn update(&mut self, dt: f32) {
        self.input_update();
        self.character_update(dt);
        self.score_update();
        self.music_update();
    }

    fn dispatch(&mut self, call: Call) {
        match call {
            Call::GivePosition => self.give_position(),
            Call::PgIsInside => self.pg_is_inside(),
            Call::PgProcess => self.pg_process(),
            Call::StopFalling => self.stop_falling(),
            Call::CoinAvailable => self.coin_is_available(),
            Call::AvoidObjects => self.avoid_objects(),
            Call::PickupReturn(i) => self.pickup_return(i as usize),
            Call::BarricadeDeath(i) => self.barricade_death(i as usize),
            Call::BarricadeFall(i) => self.barricade_fall(i as usize),
            Call::BlockadeDeath(i) => self.blockade_death(i as usize),
            Call::BlockadeFall(i) => self.blockades[i as usize].gravity = -1,
            Call::TrampAnim(i, stage) => self.trampoline_anim(i as usize, stage),
            Call::RetryReturnAnim => self.button_anim(self.score.n.button_retry, -0.06),
            Call::ExitReturnAnim => self.button_anim(self.score.n.button_exit, -0.2),
            Call::RestartGame => self.ctx.load_request = Some(crate::runtime::SceneId::Game),
            Call::BackToMenu => self.ctx.load_request = Some(crate::runtime::SceneId::Menu),
            Call::CoinCounterSlow => {
                let t = self.coins.to_string();
                self.ctx.w.set_text(self.score.n.coin_counter, &t);
            }
        }
    }
}

impl GameScene {
    /// Observable state for tests, the ?debug overlay and browser automation.
    pub fn debug_state(&self) -> serde_json::Value {
        let p = self.player_position();
        let cam = self.ctx.w.position(self.cam.cam);
        let c = &self.cm;
        serde_json::json!({
            "scene": "game",
            "frame": self.ctx.frame,
            "time": self.ctx.time,
            "paused": self.ctx.time_scale == 0.0,
            "player": [p.x, p.y],
            "camera": [cam.x, cam.y],
            "height": c.height,
            "heightMax": c.height_max,
            "jump": c.jump,
            "fallTime": c.fall_time,
            "movement": c.movement,
            "speed": c.movement_speed,
            "lives": c.lives,
            "dead": self.ct.death,
            "points": self.score.points,
            "coins": self.coins,
            "rows": (0..6).map(|k| [c.x_min[k], c.x_max[k], c.x_min2[k], c.x_max2[k], c.y_min[k], c.y_max[k]]).collect::<Vec<_>>(),
            "divider": self.pg.spawn_intel_divider,
            "teleport": self.score.teleport_timer,
            "loading": self.ctx.prefs.get_int("loading"),
            "frameUV": [self.ct.frame, self.ct.annim],
        })
    }
}
